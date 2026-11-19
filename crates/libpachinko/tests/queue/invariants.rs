//! The invariant walk: every trigger in any order, refused ones included, with
//! each step's outcome and post-state predicted from the snapshot before it (in
//! `walk`), and after every step all 35 invariants, every declared transition and
//! the `Consuming` guarantees checked by name.
//!
//! Six guards no direct test can make fail, because an invariant the walk checks
//! after every step implies each. Every one requires an active lease's pocket to
//! be leased, and `LeasedHasOneActiveLease` with `UnleasedHasNoActiveLease` make
//! that true of every active lease:
//!
//! - `rule-failure.PullWithinLease.2`;
//! - `rule-failure.ReleaseRequeuesPocket.3` and `rule-failure.ReleaseIdlesEmptyPocket.3`;
//! - `rule-failure.LeaseExpiryParksPocket.3`,
//!   `rule-failure.LeaseExpiryKeepsPocketAtFront.3` and
//!   `rule-failure.LeaseExpiryIdlesEmptyPocket.3`.
//!
//! Guards whose trigger is only ever emitted for an entity the guard then finds,
//! so the rejection is unreachable; the walk's post-state prediction is what
//! shows the guard holds:
//!
//! - `rule-failure.ReviveIdlePocket.1` and `rule-failure.ReviveParkedPocket.1`,
//!   each rule's `exists pocket`: `PocketTouched` names the pocket a publish
//!   found, and the predicted pocket is the one revived;
//! - the `.1` guard, `exists record`, and the `.2` guard, `record.status = held`,
//!   of each of the six rules that answer `Freed`: `RequeueUnsettledRecord`,
//!   `SupersedeUnsettledHeldVersion`, `ConsumeAckedRecord`,
//!   `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedRecord` and
//!   `DeadLetterNackedVersionQueueDeferred`. A lease's end sends `Freed` only for
//!   the records of its pocket's held set, so the record exists and is held. Every
//!   release and lapse is predicted record by record, and a queued record, which
//!   is sent no `Freed`, is predicted to stay as it was.
//!
//! The six rules' `.3` guards, on the mark, and `.4` guards, on a deferred
//! version, are disjoint and complete over the held records: each `Freed` passes
//! one rule's and fails the other five's. The walk reaches every row of the
//! reconciliation table on a release and on a lapse (`WANTED`) and predicts the
//! post-state, the superseded records and the dead letters exactly, so a rule
//! wrongly accepted shows as an entry too many, a record kept or removed, or a
//! position moved, in at least one row where its guard fails. One is reached but
//! cannot be observed:
//!
//! - `rule-failure.RequeueUnsettledRecord.4`: it fails only where a version is
//!   deferred, and in each of those rows the rule that does answer leaves the
//!   record queued, as `RequeueUnsettledRecord` would, which adds no entry and
//!   moves nothing, so the rejection has no effect a test can see. The walk shows
//!   the outcome the spec requires: the deferred version queued, at the held
//!   version's position for an unsettled one and at its arrival position for a
//!   marked one, and the held version superseded or dead-lettered by its mark.
//!
//! The walk runs over `MemoryRecords`, `FakeClock` and `CountingLeaseIds`; its
//! functions are generic over all three, so another store runs it unchanged.

use std::collections::{BTreeMap, BTreeSet};

use libpachinko::clock::FakeClock;
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::records::{MemoryRecords, RecordStatus, RecordView};
use libpachinko::testing::{PocketStatus, Snapshot};
use libpachinko::{
    Config, ConsumerId, HeldRecord, Lease, LeaseId, LeaseStatus, Mark, PocketKey, PullOutcome,
    Queue, RecordKey, Timestamp,
};

use super::fixture::invariants;
use proptest::strategy::{Strategy, ValueTree};

use super::walk::{self, Walk, WalkStep};

pub(crate) use invariants::ended_after_grant;

/// The five invariants over config, read from the queue under test. The three
/// TTLs are `Duration`s, non-negative by type, so each is witnessed by what it
/// guarantees and a wrong TTL would break: a superseded record expires exactly
/// one TTL after it was shed, and a lease and a dead letter at least one TTL after
/// their grant and their nack, since a lease's `expires_at` moves on with each
/// extension and a dead letter's runs from its lease's end, at or after the nack.
/// The walk's model predicts every stamp exactly.
fn check_config(s: &Snapshot, config: &Config) {
    assert!(config.max_consecutive_expiries >= 1, "ExpiryLimitPositive");
    assert!(config.max_pull_batch >= 1, "PullBatchPositive");
    for lease in &s.leases {
        assert!(
            lease.expires_at >= lease.granted_at + config.lease_ttl,
            "LeaseTtlNonNegative: {} runs at least lease_ttl from its grant",
            lease.id
        );
    }
    for shed in &s.superseded {
        assert_eq!(
            shed.expires_at,
            shed.shed_at + config.superseded_ttl,
            "SupersededTtlNonNegative: {}",
            shed.key
        );
    }
    for letter in &s.dead_letters {
        assert!(
            letter.expires_at >= letter.nacked_at + config.dead_letter_ttl,
            "DeadLetterTtlNonNegative: {} expires at least dead_letter_ttl after its nack",
            letter.key
        );
    }
}

/// Every invariant but `EndedAfterGrant`, which reads the leases a step ended
/// ([`ended_after_grant`]), over one snapshot, each assertion named after its
/// invariant; the uniqueness checks count over the snapshot's `Vec`s.
pub(crate) fn check_invariants(s: &Snapshot, config: &Config) {
    check_config(s, config);
    invariants::leased_has_one_active_lease(s);
    invariants::unleased_has_no_active_lease(s);
    invariants::idle_has_no_records(s);
    invariants::expiries_within_limit(s, config.max_consecutive_expiries);
    invariants::held_version_not_newer(s);
    invariants::nacked_has_its_time(s);
    invariants::only_nacked_has_nack_time(s);
    invariants::deferred_has_arrival_position(s);
    invariants::only_deferred_has_arrival_position(s);
    invariants::deferred_arrived_after_held(s);
    invariants::held_copies_agree(s);
    invariants::shed_before_expiry(s);
    invariants::nacked_before_expiry(s);
    invariants::granted_before_expiry(s);
    invariants::unique_publisher_id(s);
    invariants::unique_consumer_id(s);
    invariants::one_pocket_per_key(s);
    invariants::one_record_per_key_per_pocket(s);
    invariants::one_active_lease_per_pocket(s);
    invariants::one_active_lease_per_consumer(s);
    invariants::outstanding_only_while_leased(s);
    invariants::marked_pocket_has_no_expiries(s);
    invariants::unique_pocket_position(s);
    invariants::unique_record_position(s);
    invariants::pocket_positions_below_next(s);
    invariants::record_positions_below_next(s);
    invariants::arrival_position_unoccupied(s);
    invariants::unique_arrival_position(s);
    invariants::arrival_positions_below_next(s);
    invariants::order_is_the_queued_pockets(s);
}

const POCKET_EDGES: [(PocketStatus, PocketStatus); 6] = [
    (PocketStatus::Queued, PocketStatus::Leased),
    (PocketStatus::Leased, PocketStatus::Queued),
    (PocketStatus::Leased, PocketStatus::Idle),
    (PocketStatus::Leased, PocketStatus::Parked),
    (PocketStatus::Idle, PocketStatus::Queued),
    (PocketStatus::Parked, PocketStatus::Queued),
];

const RECORD_EDGES: [(&str, &str); 2] = [("queued", "held"), ("held", "queued")];

const LEASE_EDGES: [(&str, &str); 2] = [("active", "released"), ("active", "expired")];

/// A record's state as the module names it; any other is no state of the module,
/// and no declared edge leads to or from it.
fn record_state(status: &RecordStatus) -> &'static str {
    match status {
        RecordStatus::Queued => "queued",
        RecordStatus::Held { .. } => "held",
        _ => "undeclared",
    }
}

/// Whether the record is one of its pocket's `ending_records`: held, acked or
/// nacked, with no newer version deferred behind it.
fn ending(status: &RecordStatus) -> bool {
    matches!(
        status,
        RecordStatus::Held {
            mark,
            deferred_position: None,
            ..
        } if *mark != Mark::Unsettled
    )
}

fn lease_state(status: LeaseStatus) -> &'static str {
    match status {
        LeaseStatus::Active => "active",
        LeaseStatus::Released { .. } => "released",
        LeaseStatus::Expired { .. } => "expired",
        _ => "unknown",
    }
}

fn find<T>(items: &[T], matches: impl Fn(&T) -> bool) -> Option<&T> {
    items.iter().find(|item| matches(item))
}

/// One pocket's records across a step: each change of status is a declared edge
/// and a new record is queued. While held, a record keeps its held version, its
/// mark once set (`MarksStandHoweverLeaseEnds`) and its arrival position once
/// given; it is queued again or leaves the pocket only in a step that ended the
/// pocket's lease (`HeldSetStableUntilLeaseEnds`), and it leaves only as one of
/// the pocket's `ending_records`. `lease_ended` says whether this step ended it.
fn check_records(before: &[RecordView], after: &[RecordView], lease_ended: bool) {
    for record in before {
        let Some(now) = find(after, |r| r.key == record.key) else {
            assert!(
                lease_ended && ending(&record.status),
                "transition-rejected.Record.status: {} left other than as an ending record \
                    at its lease's end",
                record.key
            );
            continue;
        };
        let edge = (record_state(&record.status), record_state(&now.status));
        assert!(
            edge.0 == edge.1 || RECORD_EDGES.contains(&edge),
            "transition-rejected.Record.status: {} {edge:?}",
            record.key
        );
        assert!(
            edge != ("held", "queued") || lease_ended,
            "HeldSetStableUntilLeaseEnds: {} queued again while its lease is active",
            record.key
        );
        check_held(record, now);
    }
    for record in after
        .iter()
        .filter(|r| find(before, |b| b.key == r.key).is_none())
    {
        assert_eq!(
            record.status,
            RecordStatus::Queued,
            "a record is created queued"
        );
    }
}

/// A record held before and after a step keeps its held version, its mark once
/// set and its arrival position once given.
fn check_held(before: &RecordView, after: &RecordView) {
    let (
        RecordStatus::Held {
            rank,
            payload,
            mark,
            nacked_at,
            deferred_position,
        },
        RecordStatus::Held {
            rank: rank_now,
            payload: payload_now,
            mark: mark_now,
            nacked_at: nacked_at_now,
            deferred_position: deferred_now,
        },
    ) = (&before.status, &after.status)
    else {
        return;
    };
    assert_eq!(
        (rank, payload),
        (rank_now, payload_now),
        "HeldSetStableUntilLeaseEnds: {}'s held version does not change while held",
        before.key
    );
    if *mark != Mark::Unsettled {
        assert_eq!(
            (mark, nacked_at),
            (mark_now, nacked_at_now),
            "MarksStandHoweverLeaseEnds: {}'s first mark stands",
            before.key
        );
    }
    if deferred_position.is_some() {
        assert_eq!(
            deferred_position, deferred_now,
            "{}: a version that supersedes a deferred one inherits its arrival position",
            before.key
        );
    }
}

/// Every change of status across a step is an edge its entity declares. A pocket
/// is created queued and never removed; a lease is created active and ends only
/// as released or expired, both terminal: an ended lease never shows as active
/// again. `ended` is what this step ended, `before` what earlier steps did.
pub(crate) fn check_transitions(
    pre: &Snapshot,
    post: &Snapshot,
    ended: &[Lease],
    before: &BTreeMap<LeaseId, Lease>,
) {
    for pocket in &pre.pockets {
        let after =
            find(&post.pockets, |p| p.key == pocket.key).expect("a pocket is never removed");
        let edge = (pocket.status, after.status);
        assert!(
            edge.0 == edge.1 || POCKET_EDGES.contains(&edge),
            "transition-rejected.Pocket.status: {} {edge:?}",
            pocket.key
        );
        let lease_ended = ended.iter().any(|lease| lease.pocket_key == pocket.key);
        check_records(&pocket.records, &after.records, lease_ended);
    }
    for pocket in post
        .pockets
        .iter()
        .filter(|p| find(&pre.pockets, |b| b.key == p.key).is_none())
    {
        assert_eq!(
            pocket.status,
            PocketStatus::Queued,
            "a pocket is created queued"
        );
    }
    for lease in &post.leases {
        assert_eq!(
            lease.status,
            LeaseStatus::Active,
            "the registry holds only active leases"
        );
        assert!(
            !before.contains_key(&lease.id) && find(ended, |e| e.id == lease.id).is_none(),
            "transition-terminal.Lease.status: ended lease {} is active again",
            lease.id
        );
    }
    for lease in &pre.leases {
        if find(&post.leases, |l| l.id == lease.id).is_some() {
            continue;
        }
        let end = find(ended, |e| e.id == lease.id).expect("an active lease leaves only by ending");
        let edge = (lease_state(lease.status), lease_state(end.status));
        assert!(
            LEASE_EDGES.contains(&edge),
            "transition-rejected.Lease.status: {} {edge:?}",
            lease.id
        );
    }
}

/// `OldestFirst`, `ExclusivePocket` and `OneVisitPerKeyPerLease` for one batch,
/// against the snapshot before the pull. `size` is `min(limit, max_pull_batch)`,
/// none for a limit that is not positive. A key stays in `handed` until its lease
/// ends, acked, nacked or neither, so a key handed out twice under one lease fails
/// `OneVisitPerKeyPerLease`.
pub(crate) fn check_batch(
    pre: &Snapshot,
    handed: &mut BTreeMap<LeaseId, BTreeSet<(PocketKey, RecordKey)>>,
    consumer: &ConsumerId,
    batch: &PullOutcome,
    size: usize,
) {
    let PullOutcome::Batch {
        lease,
        pocket_key,
        records,
        ..
    } = batch
    else {
        return;
    };
    let pocket = find(&pre.pockets, |p| &p.key == pocket_key).expect("a batch's pocket exists");
    let mut queued: Vec<&RecordView> = pocket
        .records
        .iter()
        .filter(|r| r.status == RecordStatus::Queued)
        .collect();
    queued.sort_by_key(|r| r.position);
    let oldest: Vec<HeldRecord> = queued
        .into_iter()
        .take(size)
        .map(|r| HeldRecord::new(r.key.clone(), r.rank, r.payload.clone()))
        .collect();
    assert_eq!(records, &oldest, "OldestFirst");
    assert!(
        pre.leases
            .iter()
            .all(|l| &l.pocket_key != pocket_key || &l.consumer_id == consumer),
        "ExclusivePocket"
    );
    let visits = handed.entry(*lease).or_default();
    for record in records {
        let visit = (pocket_key.clone(), record.key.clone());
        assert!(
            visits.insert(visit),
            "OneVisitPerKeyPerLease: {}",
            record.key
        );
    }
}

/// One walk over the store, clock and lease-id source the wave ships.
fn memory_walk(steps: &[WalkStep]) -> Walk {
    let clock = FakeClock::new(Timestamp::from_millis(0));
    let mut queue: Queue<MemoryRecords, FakeClock, CountingLeaseIds> =
        Queue::new(walk::config(), clock.clone(), CountingLeaseIds::default()).unwrap();
    walk::walk(&mut queue, &|to| clock.set(to), steps)
}

// obligation: invariant.Pocket.LeasedHasOneActiveLease
// obligation: invariant.Pocket.UnleasedHasNoActiveLease
// obligation: invariant.Pocket.IdleHasNoRecords
// obligation: invariant.Pocket.ExpiriesWithinLimit
// obligation: invariant.Record.HeldVersionNotNewer
// obligation: invariant.Record.NackedHasItsTime
// obligation: invariant.Record.OnlyNackedHasNackTime
// obligation: invariant.Record.DeferredHasArrivalPosition
// obligation: invariant.Record.OnlyDeferredHasArrivalPosition
// obligation: invariant.Record.DeferredArrivedAfterHeld
// obligation: invariant.Record.HeldCopiesAgree
// obligation: invariant.SupersededRecord.ShedBeforeExpiry
// obligation: invariant.DeadLetter.NackedBeforeExpiry
// obligation: invariant.Lease.GrantedBeforeExpiry
// obligation: invariant.Lease.EndedAfterGrant
// obligation: invariant.UniquePublisherId
// obligation: invariant.UniqueConsumerId
// obligation: invariant.OnePocketPerKey
// obligation: invariant.OneRecordPerKeyPerPocket
// obligation: invariant.OneActiveLeasePerPocket
// obligation: invariant.OneActiveLeasePerConsumer
// obligation: invariant.OutstandingOnlyWhileLeased
// obligation: invariant.MarkedPocketHasNoExpiries
// obligation: invariant.UniquePocketPosition
// obligation: invariant.UniqueRecordPosition
// obligation: invariant.PocketPositionsBelowNext
// obligation: invariant.RecordPositionsBelowNext
// obligation: invariant.ArrivalPositionUnoccupied
// obligation: invariant.UniqueArrivalPosition
// obligation: invariant.ArrivalPositionsBelowNext
// obligation: invariant.ExpiryLimitPositive
// obligation: invariant.PullBatchPositive
// obligation: invariant.LeaseTtlNonNegative
// obligation: invariant.SupersededTtlNonNegative
// obligation: invariant.DeadLetterTtlNonNegative
// obligation: transition-rejected.Pocket.status
// obligation: transition-rejected.Record.status
// obligation: transition-rejected.Lease.status
// obligation: transition-terminal.Lease.status
// obligation: rule-failure.PullWithinLease.2
// obligation: rule-failure.ReleaseRequeuesPocket.3
// obligation: rule-failure.ReleaseIdlesEmptyPocket.3
// obligation: rule-failure.LeaseExpiryParksPocket.3
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.3
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.3
// obligation: rule-failure.ReviveIdlePocket.1
// obligation: rule-failure.ReviveParkedPocket.1
// obligation: rule-failure.RequeueUnsettledRecord.1
// obligation: rule-failure.RequeueUnsettledRecord.2
// obligation: rule-failure.RequeueUnsettledRecord.3
// obligation: rule-failure.RequeueUnsettledRecord.4
// obligation: rule-failure.SupersedeUnsettledHeldVersion.1
// obligation: rule-failure.SupersedeUnsettledHeldVersion.2
// obligation: rule-failure.SupersedeUnsettledHeldVersion.3
// obligation: rule-failure.SupersedeUnsettledHeldVersion.4
// obligation: rule-failure.ConsumeAckedRecord.1
// obligation: rule-failure.ConsumeAckedRecord.2
// obligation: rule-failure.ConsumeAckedRecord.3
// obligation: rule-failure.ConsumeAckedRecord.4
// obligation: rule-failure.ConsumeAckedVersionQueueDeferred.1
// obligation: rule-failure.ConsumeAckedVersionQueueDeferred.2
// obligation: rule-failure.ConsumeAckedVersionQueueDeferred.3
// obligation: rule-failure.ConsumeAckedVersionQueueDeferred.4
// obligation: rule-failure.DeadLetterNackedRecord.1
// obligation: rule-failure.DeadLetterNackedRecord.2
// obligation: rule-failure.DeadLetterNackedRecord.3
// obligation: rule-failure.DeadLetterNackedRecord.4
// obligation: rule-failure.DeadLetterNackedVersionQueueDeferred.1
// obligation: rule-failure.DeadLetterNackedVersionQueueDeferred.2
// obligation: rule-failure.DeadLetterNackedVersionQueueDeferred.3
// obligation: rule-failure.DeadLetterNackedVersionQueueDeferred.4
// obligation: surface-provides.Consuming
// obligation: guarantee.OldestFirst
// obligation: guarantee.ExclusivePocket
// obligation: guarantee.OneVisitPerKeyPerLease
// obligation: guarantee.HeldSetStableUntilLeaseEnds
// obligation: guarantee.MarksStandHoweverLeaseEnds
#[test]
fn every_invariant_holds_after_every_step_of_any_walk() {
    walk::runner(512)
        .run(&walk::steps(), |steps| {
            memory_walk(&steps);
            Ok(())
        })
        .unwrap();
}

/// What a blind generator would miss: every state the module names and every
/// outcome the rules give, refusals included.
const WANTED: &[&str] = &[
    "publisher joins true",
    "publisher joins false",
    "consumer joins true",
    "consumer joins false",
    "publish Created",
    "publish NewKey",
    "publish Superseded",
    "publish defers behind a held version",
    "publish supersedes a deferred version",
    "publish Stale",
    "stale publish leaves a parked pocket parked",
    "publish EqualRank",
    "publish UnknownPublisher",
    "revive Idle",
    "revive Parked",
    "pull NoPocketAvailable",
    "pull UnknownConsumer",
    "pull claims",
    "pull within lease",
    "empty batch",
    "batch of several",
    "ack marks",
    "ack repeats",
    "ack MarkConflict",
    "ack UnknownConsumer",
    "ack LeaseNotActive",
    "ack LeaseNotYours",
    "ack RecordNotHeld",
    "ack resets a counted pocket",
    "nack marks",
    "nack repeats",
    "nack MarkConflict",
    "nack UnknownConsumer",
    "nack LeaseNotActive",
    "nack LeaseNotYours",
    "nack RecordNotHeld",
    "nack resets a counted pocket",
    "renew accepted",
    "renew UnknownConsumer",
    "renew LeaseNotActive",
    "renew LeaseNotYours",
    "release Queued",
    "release Idle",
    "release UnknownConsumer",
    "release LeaseNotActive",
    "release LeaseNotYours",
    "release RequeueUnsettledRecord",
    "release SupersedeUnsettledHeldVersion",
    "release ConsumeAckedRecord",
    "release ConsumeAckedVersionQueueDeferred",
    "release DeadLetterNackedRecord",
    "release DeadLetterNackedVersionQueueDeferred",
    "lapse RequeueUnsettledRecord",
    "lapse SupersedeUnsettledHeldVersion",
    "lapse ConsumeAckedRecord",
    "lapse ConsumeAckedVersionQueueDeferred",
    "lapse DeadLetterNackedRecord",
    "lapse DeadLetterNackedVersionQueueDeferred",
    "lapse Queued",
    "lapse Idle",
    "lapse Parked",
    "decay superseded",
    "decay dead letter",
    "pocket Queued",
    "pocket Leased",
    "pocket Idle",
    "pocket Parked",
    "record Queued in Queued",
    "record Queued in Leased",
    "record Queued in Parked",
    "record Held Unsettled in Leased",
    "record Held Acked in Leased",
    "record Held Nacked in Leased",
    "branch A pull",
    "branch A ack",
    "branch A nack",
    "branch A renew",
    "branch A release",
    "renew with the clock behind",
    "release with the clock behind",
    "batch of one",
];

/// The labels `walks` seeded walks reach, drawn from the same generator and
/// runner seed as the walk test.
fn reached(walks: usize) -> BTreeSet<String> {
    let mut runner = walk::runner(1);
    let strategy = walk::steps();
    let mut reached = BTreeSet::new();
    for _ in 0..walks {
        let steps = strategy.new_tree(&mut runner).unwrap().current();
        reached.extend(memory_walk(&steps).reached);
    }
    reached
}

// The generator is not blind: the same seeded walks reach every state the module
// names and every outcome the rules give. A label nothing produces is a defect in
// the generator's weights.
#[test]
fn the_walk_reaches_every_state_and_every_outcome() {
    let reached = reached(512);
    let missing: Vec<&str> = WANTED
        .iter()
        .copied()
        .filter(|label| !reached.contains(*label))
        .collect();
    assert_eq!(missing, Vec::<&str>::new(), "never reached");
}
