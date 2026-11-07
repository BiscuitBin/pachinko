//! The invariant walk: every trigger in any order, refused ones included, with
//! each step's outcome and post-state predicted from the snapshot before it (in
//! `walk`), and after every step all 25 invariants, every declared transition and
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
//! Three existence guards, whose trigger is only ever emitted for an entity the
//! guard then finds, so the rejection is unreachable; the walk's post-state
//! prediction is what shows the guard holds:
//!
//! - `rule-failure.ReviveIdlePocket.1`: `PocketTouched` names the pocket a publish
//!   found, and the predicted pocket is the one revived;
//! - `rule-failure.RequeueHeldRecord.1` and `rule-failure.RequeueDeferredRecord.1`:
//!   `Freed` names only a record of the pocket, and every release and lapse is
//!   predicted to keep each record's key, rank, payload and position, queue the
//!   held and the deferred, and leave a queued record as it was.
//!
//! One more is reached but cannot be observed:
//!
//! - `rule-failure.RequeueDeferredRecord.2`: reached whenever a release or lapse
//!   frees a held record, because `Freed` is emitted for held records too. A
//!   wrongly accepted `RequeueDeferredRecord` would also leave the record queued,
//!   as `RequeueHeldRecord` does, so the rejection has no effect a test can see.
//!   Only `RequeueHeldRecord` moves the record, and the walk shows the outcome the
//!   spec requires: each freed held record ends queued at its own key, rank,
//!   payload and position, with its held version dropped.
//!
//! The walk runs over `MemoryRecords`, `FakeClock` and `CountingLeaseIds`; its
//! functions are generic over all three, so another store runs it unchanged.

use std::collections::{BTreeMap, BTreeSet};

use libpachinko::clock::{FakeClock, Timestamp};
use libpachinko::config::Config;
use libpachinko::ids::{ConsumerId, PocketKey, RecordKey};
use libpachinko::lease_ids::{CountingLeaseIds, LeaseId};
use libpachinko::queue::{Lease, LeaseStatus, PocketStatus, PullOutcome, Queue, Snapshot};
use libpachinko::records::{HeldRecord, MemoryRecords, RecordStatus, RecordView};

use super::fixture::invariants;
use proptest::strategy::{Strategy, ValueTree};

use super::walk::{self, Walk, WalkStep};

pub(crate) use invariants::ended_after_grant;

/// The five invariants over config, read from the queue under test. The three
/// TTLs are `Duration`s, non-negative by type, so each is witnessed by what it
/// guarantees and a wrong TTL would break: every stamp lands exactly one TTL on.
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
        assert_eq!(
            letter.expires_at,
            letter.nacked_at + config.dead_letter_ttl,
            "DeadLetterTtlNonNegative: {}",
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
    invariants::unique_pocket_position(s);
    invariants::unique_record_position(s);
    invariants::pocket_positions_below_next(s);
    invariants::record_positions_below_next(s);
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

const RECORD_EDGES: [(&str, &str); 4] = [
    ("queued", "held"),
    ("held", "queued"),
    ("held", "deferred"),
    ("deferred", "queued"),
];

const LEASE_EDGES: [(&str, &str); 2] = [("active", "released"), ("active", "expired")];

fn record_state(status: &RecordStatus) -> &'static str {
    match status {
        RecordStatus::Held { .. } => "held",
        RecordStatus::Deferred => "deferred",
        _ => "queued",
    }
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

/// One pocket's records across a step: each change of status is a declared edge,
/// a held version does not change while held, a new record is queued, and a
/// record leaves only by the settle of its held version.
fn check_records(
    key: &PocketKey,
    before: &[RecordView],
    after: &[RecordView],
    settled: Option<&(PocketKey, RecordKey)>,
) {
    for record in before {
        let Some(now) = find(after, |r| r.key == record.key) else {
            let leaves = (key.clone(), record.key.clone());
            assert!(
                record_state(&record.status) == "held" && settled == Some(&leaves),
                "transition-rejected.Record.status: {} left without a settle",
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
        if edge == ("held", "held") {
            assert_eq!(
                record.status, now.status,
                "a held version does not change while held"
            );
        }
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

/// Every change of status across a step is an edge its entity declares. A pocket
/// is created queued and never removed; a lease is created active and ends only
/// as released or expired, both terminal: an ended lease never shows as active
/// again. `ended` is what this step ended, `before` what earlier steps did.
pub(crate) fn check_transitions(
    pre: &Snapshot,
    post: &Snapshot,
    settled: Option<&(PocketKey, RecordKey)>,
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
        check_records(&pocket.key, &pocket.records, &after.records, settled);
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
/// none for a limit that is not positive. A key leaves `handed` when its record
/// is settled, so a key published again under the same lease is a new record
/// (open question 9, as the rules stand).
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
// obligation: invariant.UniquePocketPosition
// obligation: invariant.UniqueRecordPosition
// obligation: invariant.PocketPositionsBelowNext
// obligation: invariant.RecordPositionsBelowNext
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
// obligation: rule-failure.RequeueHeldRecord.1
// obligation: rule-failure.RequeueDeferredRecord.1
// obligation: rule-failure.RequeueDeferredRecord.2
// obligation: surface-provides.Consuming
// obligation: guarantee.OldestFirst
// obligation: guarantee.ExclusivePocket
// obligation: guarantee.OneVisitPerKeyPerLease
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
    "publish Stale",
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
    "ack Settled",
    "ack Deferred",
    "ack UnknownConsumer",
    "ack LeaseNotActive",
    "ack LeaseNotYours",
    "ack RecordNotHeld",
    "nack Settled",
    "nack Deferred",
    "nack UnknownConsumer",
    "nack LeaseNotActive",
    "nack LeaseNotYours",
    "nack RecordNotHeld",
    "renew accepted",
    "renew UnknownConsumer",
    "renew LeaseNotActive",
    "renew LeaseNotYours",
    "release Queued",
    "release Idle",
    "release UnknownConsumer",
    "release LeaseNotActive",
    "release LeaseNotYours",
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
    "record Held in Leased",
    "record Deferred in Leased",
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
