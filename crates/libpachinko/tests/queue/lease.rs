//! Renew, release, revive, fencing and the consumer's view: `Renew`,
//! `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `ReviveIdlePocket`, and the
//! guard order every lease action shares; and the lease's end, by release or by
//! lapse, through the six rules that answer `Freed`, one per row of the
//! reconciliation table, with the pocket's fate and the order of the entries one
//! end creates.

use std::time::Duration;

use libpachinko::records::{Position, RecordStatus};
use libpachinko::testing::{PocketStatus, ShedReason};
use libpachinko::{
    LeaseActionError, LeaseId, LeaseStatus, Mark, Payload, PocketFate, PublishOutcome, PullOutcome,
    Rank, SettleOutcome, Timestamp,
};

use crate::fixture::{
    DEAD_LETTER_TTL, Fixture, LEASE_TTL, SUPERSEDED_TTL, T0, config, consumer, held_as, invariants,
    keys, letter_rows, marked_keys, shed_rows, text,
};

/// `T0` plus `millis`.
const fn at(millis: u64) -> Timestamp {
    T0.saturating_add_millis(millis)
}

/// Pocket `a` with `a1` and `a2`, then pocket `b` with `b1`.
fn two_pockets() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    fixture.publish("b", "b1", 1, "b1").unwrap();
    fixture
}

/// A fixture with a 10 ms lease and one held record under `c1`'s lease, pulled at
/// `T0`.
fn short_lease() -> (Fixture, LeaseId) {
    let mut short = config();
    short.lease_ttl = Duration::from_millis(10);
    let mut fixture = Fixture::with_config(short);
    fixture.publish("pk", "k", 1, "v").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    (fixture, lease)
}

// scenario: S023
// obligation: rule-success.ReviveIdlePocket
// obligation: transition-edge.Pocket.idle.queued
// obligation: rule-failure.ReviveIdlePocket.2
#[test]
fn a_publish_revives_an_idle_pocket_at_the_back() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 10).lease;
    fixture.ack("c1", lease, "a1").unwrap();
    fixture.ack("c1", lease, "a2").unwrap();
    fixture.release("c1", lease).unwrap();
    assert_eq!(fixture.pocket("a").status, PocketStatus::Idle);
    fixture.publish("c", "c1", 1, "c").unwrap();
    let next = fixture.snapshot().next_position;
    assert_eq!(
        fixture.publish("a", "a3", 1, "a3"),
        Ok(PublishOutcome::NewKey)
    );
    let pocket = fixture.pocket("a");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(
        fixture.record("a", "a3").position,
        next,
        "PublishNewKey takes its position first"
    );
    assert_eq!(pocket.position, next.next(), "then the revive: the back");
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(fixture.order(), ["b", "c", "a"]);
    fixture.publish("a", "a4", 1, "a4").unwrap();
    assert_eq!(
        fixture.pocket("a").position,
        next.next(),
        "a queued pocket stays put"
    );
    invariants::all(&fixture.snapshot());
}

// scenario: S035
// obligation: rule-success.PullWithinLease
// obligation: rule-failure.PullClaimsFrontPocket.1
// obligation: rule-failure.PullFindsNoPocket.1
#[test]
fn a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 1).lease;
    let again = fixture.pull("c1", 1);
    assert_eq!((again.lease, again.pocket_key.as_str()), (lease, "a"));
    let theirs = fixture.pull("c2", 10).lease;
    assert!(fixture.order().is_empty(), "no pocket is queued");
    let empty = fixture.pull("c1", 10);
    assert_eq!((empty.lease, empty.pocket_key.as_str()), (lease, "a"));
    assert_eq!(empty.records.len(), 0);
    fixture.release("c2", theirs).unwrap();
    let still = fixture.pull("c1", 10);
    assert_eq!((still.lease, still.pocket_key.as_str()), (lease, "a"));
    fixture.release("c1", lease).unwrap();
    let other = fixture.pull("c1", 10);
    assert_ne!(other.lease, lease);
    assert_eq!(other.pocket_key.as_str(), "b");
}

// scenario: S050
// obligation: rule-success.Renew
#[test]
fn a_renew_extends_the_lease_to_now_plus_lease_ttl() {
    let (mut fixture, lease) = short_lease();
    fixture.advance(6);
    assert_eq!(fixture.renew("c1", lease), Ok(T0.saturating_add_millis(16)));
    let snapshot = fixture.snapshot();
    let [active] = snapshot.leases.as_slice() else {
        panic!("one lease")
    };
    assert_eq!(active.expires_at, T0.saturating_add_millis(16));
    assert_eq!(active.granted_at, T0, "a renew is not a new grant");
    invariants::granted_before_expiry(&snapshot);
}

// scenario: S051
// obligation: rule-success.Renew
#[test]
fn renewals_do_not_stack() {
    let (mut fixture, lease) = short_lease();
    fixture.advance(4);
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.lease_view("c1").unwrap().expires_at,
        T0.saturating_add_millis(14)
    );
    fixture.advance(2);
    let first = fixture.renew("c1", lease).unwrap();
    let second = fixture.renew("c1", lease).unwrap();
    assert_eq!((first, second), (T0.saturating_add_millis(16), first));
    assert_eq!(fixture.snapshot().leases.len(), 1);
}

// scenario: S052
// obligation: rule-failure.Renew.1
// obligation: transition-terminal.Lease.status
#[test]
fn a_renew_of_an_ended_lease_is_rejected_and_moves_nothing() {
    let (mut fixture, lease) = short_lease();
    fixture.release("c1", lease).unwrap();
    fixture.advance(3);
    let ended = LeaseActionError::LeaseNotActive(lease);
    fixture.refused(&ended, |f| f.renew("c1", lease));
    fixture.pull("c1", 10);
    fixture.refused(&ended, |f| f.renew("c1", lease));
}

// scenario: S053
// obligation: rule-success.ReleaseRequeuesPocket
// obligation: rule-success.RequeueUnsettledRecord
// obligation: rule-success.ConsumeAckedRecord
// obligation: rule-failure.ReleaseIdlesEmptyPocket.4
// obligation: transition-edge.Pocket.leased.queued
// obligation: transition-edge.Record.held.queued
// obligation: transition-edge.Lease.active.released
// obligation: when-set.ReleaseRequeuesPocket.Lease.ended_at
// obligation: when-presence.Lease.ended_at
#[test]
fn a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place() {
    let mut fixture = two_pockets();
    let a2 = fixture.record("a", "a2").position;
    let lease = fixture.pull("c1", 10).lease;
    fixture.ack("c1", lease, "a1").unwrap();
    assert_eq!(
        fixture.record("a", "a1").status,
        held_as(1, "a1", Mark::Acked, None, None),
        "an acked record stays until the lease ends"
    );
    fixture.advance(7);
    let next = fixture.snapshot().next_position;
    let (fate, ended) = fixture.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Queued);
    assert_eq!(
        ended.status,
        LeaseStatus::Released {
            ended_at: T0.saturating_add_millis(7)
        }
    );
    assert_eq!(
        (ended.id, ended.consumer_id.clone()),
        (lease, consumer("c1"))
    );
    invariants::ended_after_grant(&ended);
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.position, pocket.consecutive_expiries),
        (PocketStatus::Queued, next, 0)
    );
    let record = fixture.record("a", "a2");
    assert_eq!((record.status, record.position), (RecordStatus::Queued, a2));
    assert_eq!(pocket.records.len(), 1, "the acked a1 is consumed");
    assert_eq!(fixture.order(), ["b", "a"]);
    let snapshot = fixture.snapshot();
    assert!(
        snapshot.leases.is_empty(),
        "an ended lease leaves the registry"
    );
    assert!(snapshot.consumers.iter().all(|(_, lease)| lease.is_none()));
    invariants::all(&snapshot);
    assert_eq!(fixture.pull("c1", 10).pocket_key.as_str(), "b");
}

// scenario: S054
// obligation: rule-success.ReleaseRequeuesPocket
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
// obligation: rule-failure.ReleaseIdlesEmptyPocket.4
// obligation: projection.Pocket.ending_records
#[test]
fn a_release_with_only_an_acked_record_and_its_deferred_version_requeues_the_pocket() {
    let (mut fixture, lease) = short_lease();
    fixture.publish("pk", "k", 2, "v2").unwrap();
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.record("pk", "k").status,
        held_as(1, "v", Mark::Acked, None, Some(2)),
        "acked, with v2 deferred: not one of the ending records"
    );
    let (fate, _) = fixture.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Queued);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Queued);
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank, record.position),
        (RecordStatus::Queued, Rank::new(2), Position::new(2))
    );
    invariants::all(&fixture.snapshot());
}

// scenario: S055
// obligation: rule-success.ReleaseIdlesEmptyPocket
// obligation: rule-failure.ReleaseRequeuesPocket.4
// obligation: transition-edge.Pocket.leased.idle
// obligation: when-set.ReleaseIdlesEmptyPocket.Lease.ended_at
// obligation: invariant.Pocket.IdleHasNoRecords
#[test]
fn a_release_idles_an_empty_pocket() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 10).lease;
    for key in ["a1", "a2"] {
        fixture.ack("c1", lease, key).unwrap();
    }
    let position = fixture.pocket("a").position;
    let (fate, ended) = fixture.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Idle);
    assert_eq!(ended.status, LeaseStatus::Released { ended_at: T0 });
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.consecutive_expiries, pocket.position),
        (PocketStatus::Idle, 0, position)
    );
    let snapshot = fixture.snapshot();
    invariants::idle_has_no_records(&snapshot);
    invariants::all(&snapshot);
    assert_eq!(fixture.order(), ["b"]);
    assert_eq!(fixture.pull("c1", 10).pocket_key.as_str(), "b");
    assert_eq!(
        fixture.try_pull("c2", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
}

// scenario: S057
// obligation: invariant.Lease.EndedAfterGrant
// obligation: rule-failure.ReleaseRequeuesPocket.1
// obligation: rule-failure.ReleaseIdlesEmptyPocket.1
// obligation: transition-terminal.Lease.status
#[test]
fn a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal() {
    let (mut fixture, lease) = short_lease();
    let (_, ended) = fixture.release("c1", lease).unwrap();
    assert_eq!(ended.granted_at, T0);
    assert_eq!(ended.status, LeaseStatus::Released { ended_at: T0 });
    invariants::ended_after_grant(&ended);
    let again = LeaseActionError::LeaseNotActive(lease);
    fixture.refused(&again, |f| f.release("c1", lease));
}

// scenario: S068
// obligation: guarantee.ActionsNameTheirLease
// obligation: rule-failure.AckMarksRecord.1
// obligation: rule-failure.Renew.1
// obligation: rule-failure.ReleaseRequeuesPocket.1
#[test]
fn a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    let old = fixture.pull("c1", 10).lease;
    fixture.release("c1", old).unwrap();
    fixture.publish("pk", "k", 2, "v2").unwrap();
    let new = fixture.pull("c1", 10);
    assert_ne!(new.lease, old);
    assert_eq!(new.pocket_key.as_str(), "pk");
    assert_eq!(new.records[0].rank, Rank::new(2));
    fixture.advance(5);
    let ended = LeaseActionError::LeaseNotActive(old);
    fixture.refused(&ended, |f| f.renew("c1", old));
    fixture.refused(&ended, |f| f.ack("c1", old, "k"));
    fixture.refused(&ended, |f| f.nack("c1", old, "k"));
    fixture.refused(&ended, |f| f.release("c1", old));
    assert_eq!(fixture.ack("c1", new.lease, "k"), Ok(SettleOutcome::Acked));
}

// scenario: S070
// obligation: rule-failure.AckMarksRecord.2
// obligation: rule-failure.NackMarksRecord.2
// obligation: rule-failure.Renew.2
// obligation: rule-failure.ReleaseRequeuesPocket.2
// obligation: rule-failure.ReleaseIdlesEmptyPocket.2
// obligation: guarantee.ExclusivePocket
#[test]
fn a_consumer_cannot_act_on_another_consumers_lease() {
    let mut fixture = two_pockets();
    let mine = fixture.pull("c1", 10).lease;
    fixture.pull("c2", 10);
    let not_yours = LeaseActionError::LeaseNotYours(mine);
    fixture.refused(&not_yours, |f| f.ack("c2", mine, "a1"));
    fixture.refused(&not_yours, |f| f.nack("c2", mine, "a1"));
    fixture.refused(&not_yours, |f| f.renew("c2", mine));
    fixture.refused(&not_yours, |f| f.release("c2", mine));
    fixture.refused(&not_yours, |f| f.ack("c3", mine, "absent"));
    fixture.release("c1", mine).unwrap();
    let ended = LeaseActionError::LeaseNotActive(mine);
    fixture.refused(&ended, |f| f.ack("c2", mine, "a1"));
    fixture.refused(&ended, |f| f.nack("c2", mine, "a1"));
    fixture.refused(&ended, |f| f.renew("c2", mine));
    fixture.refused(&ended, |f| f.release("c2", mine));
}

// obligation: guarantee.ActionsNameTheirLease
#[test]
fn an_unknown_consumer_is_refused_before_any_lease_guard() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    let unknown = LeaseActionError::UnknownConsumer(consumer("nobody"));
    for id in [lease, LeaseId::from_u128(42)] {
        fixture.refused(&unknown, |f| f.ack("nobody", id, "absent"));
        fixture.refused(&unknown, |f| f.nack("nobody", id, "absent"));
        fixture.refused(&unknown, |f| f.renew("nobody", id));
        fixture.refused(&unknown, |f| f.release("nobody", id));
    }
}

// scenario: S076
// obligation: surface-exposure.Consuming
// obligation: projection.Pocket.held_records
#[test]
fn the_consumer_view_exposes_only_the_active_lease_and_its_held_versions() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    fixture.publish("pk", "queued", 1, "q").unwrap();
    fixture.publish("other", "x", 1, "x").unwrap();
    let lease = fixture.pull("c1", 1).lease;
    fixture.pull("c2", 10);
    fixture.publish("pk", "k", 3, "v3").unwrap();
    assert_eq!(
        fixture.snapshot().superseded.len(),
        0,
        "v1 is held, not shed"
    );
    let view = fixture.lease_view("c1").unwrap();
    assert_eq!(view.pocket_key.as_str(), "pk");
    assert_eq!(view.expires_at, T0 + LEASE_TTL);
    assert_eq!(marked_keys(&view.records), ["k"], "only the held records");
    assert_eq!(
        (view.records[0].rank, &view.records[0].payload),
        (Rank::new(1), &Payload::from(b"v1".as_slice())),
        "the held version, not the latest"
    );
    assert_eq!(view.records[0].mark, Mark::Unsettled);
    fixture.release("c1", lease).unwrap();
    assert_eq!(fixture.lease_view("c1"), None);
    assert_eq!(
        marked_keys(&fixture.lease_view("c2").unwrap().records),
        ["x"]
    );
}

/// How a test ends `c1`'s lease: released at 50 ms, or lapsed by a tick at
/// 2,000 ms, past every expiry these tests reach and before any entry they shed
/// decays.
#[derive(Clone, Copy, Debug)]
enum Ending {
    Release,
    Lapse,
}

const ENDINGS: [Ending; 2] = [Ending::Release, Ending::Lapse];

impl Ending {
    /// Ends `lease` this way and returns the instant it ended.
    fn end(self, fixture: &mut Fixture, lease: LeaseId) -> Timestamp {
        match self {
            Self::Release => {
                fixture.at(50);
                let (_, ended) = fixture.release("c1", lease).unwrap();
                assert_eq!(ended.status, LeaseStatus::Released { ended_at: at(50) });
                at(50)
            }
            Self::Lapse => {
                fixture.at(2_000);
                let report = fixture.tick();
                let lapsed: Vec<LeaseId> = report.lapsed.iter().map(|l| l.id).collect();
                assert_eq!(lapsed, [lease], "the lease lapsed");
                at(2_000)
            }
        }
    }

    /// The pocket's consecutive-expiry count after this end of a lease that
    /// started at 0: a release resets it and a lapse counts one.
    const fn count(self) -> u32 {
        match self {
            Self::Release => 0,
            Self::Lapse => 1,
        }
    }
}

/// `pk` with `k` at rank 1, payload `v1`, at position 1 and `early` at 2; `c1`
/// pulls `k` alone at `T0`, and `v3` arrives at 10 ms, deferred behind `v1` at
/// arrival position 3.
fn v3_deferred_behind_v1() -> (Fixture, LeaseId) {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    fixture.publish("pk", "early", 1, "e").unwrap();
    let lease = fixture.pull("c1", 1).lease;
    fixture.at(10);
    assert_eq!(
        fixture.publish("pk", "k", 3, "v3"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank),
        (
            held_as(1, "v1", Mark::Unsettled, None, Some(3)),
            Rank::new(3)
        ),
        "v3 is deferred behind the held v1"
    );
    (fixture, lease)
}

/// Checks that `pk` is queued with the count `ending` leaves, that `k` is queued
/// at `rank` and `position`, and that `c2` is handed `order`.
fn after_the_end(
    fixture: &mut Fixture,
    ending: Ending,
    (rank, position): (i64, u64),
    order: &[&str],
) {
    let pocket = fixture.pocket("pk");
    assert_eq!(
        (pocket.status, pocket.consecutive_expiries),
        (PocketStatus::Queued, ending.count()),
        "{ending:?}"
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank, record.position),
        (
            RecordStatus::Queued,
            Rank::new(rank),
            Position::new(position)
        ),
        "{ending:?}"
    );
    assert_eq!(keys(&fixture.pull("c2", 10).records), order, "{ending:?}");
}

// scenario: S078
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
// obligation: rule-failure.ConsumeAckedRecord.4
// obligation: guarantee.MarksStandHoweverLeaseEnds
// obligation: guarantee.EveryDropIsVisible
// obligation: guarantee.OldestSlotNewestPayload
#[test]
fn scenario_a_an_acked_version_is_consumed_and_the_deferred_one_queued_at_its_arrival() {
    for ending in ENDINGS {
        let (mut fixture, lease) = v3_deferred_behind_v1();
        fixture.at(20);
        fixture.ack("c1", lease, "k").unwrap();
        ending.end(&mut fixture, lease);
        let snapshot = fixture.snapshot();
        assert_eq!(
            shed_rows(&snapshot),
            [],
            "{ending:?}: v1 is recorded nowhere"
        );
        assert_eq!(letter_rows(&snapshot), [], "{ending:?}");
        after_the_end(&mut fixture, ending, (3, 3), &["early", "k"]);
    }
}

// scenario: S079
// obligation: rule-success.DeadLetterNackedVersionQueueDeferred
// obligation: rule-entity-creation.DeadLetterNackedVersionQueueDeferred.1
// obligation: rule-failure.DeadLetterNackedRecord.4
// obligation: guarantee.MarksStandHoweverLeaseEnds
#[test]
fn scenario_b_a_nacked_version_is_dead_lettered_and_the_deferred_one_queued_at_its_arrival() {
    for ending in ENDINGS {
        let (mut fixture, lease) = v3_deferred_behind_v1();
        fixture.at(20);
        fixture.nack("c1", lease, "k").unwrap();
        assert_eq!(fixture.snapshot().dead_letters.len(), 0, "{ending:?}");
        let ended = ending.end(&mut fixture, lease);
        let snapshot = fixture.snapshot();
        let letter = ("pk", "k", 1, "v1", "c1", at(20), ended + DEAD_LETTER_TTL);
        assert_eq!(letter_rows(&snapshot), [letter], "{ending:?}");
        assert_eq!(shed_rows(&snapshot), [], "{ending:?}");
        after_the_end(&mut fixture, ending, (3, 3), &["early", "k"]);
    }
}

// scenario: S080
// obligation: rule-success.Supersede
// obligation: rule-success.DeadLetterNackedVersionQueueDeferred
// obligation: guarantee.NothingLostWhileHeld
// obligation: guarantee.OldestSlotNewestPayload
#[test]
fn scenario_c_v3_is_shed_when_v5_arrives_and_v5_takes_v3s_arrival_position() {
    for ending in ENDINGS {
        let (mut fixture, lease) = v3_deferred_behind_v1();
        fixture.at(15);
        fixture.publish("pk", "between", 1, "b").unwrap();
        fixture.at(18);
        fixture.publish("pk", "k", 5, "v5").unwrap();
        let v3 = (
            "pk",
            "k",
            3,
            "v3",
            ShedReason::Superseded,
            at(18),
            at(18) + SUPERSEDED_TTL,
        );
        assert_eq!(shed_rows(&fixture.snapshot()), [v3], "{ending:?}: at once");
        fixture.at(20);
        fixture.nack("c1", lease, "k").unwrap();
        let ended = ending.end(&mut fixture, lease);
        let snapshot = fixture.snapshot();
        assert_eq!(shed_rows(&snapshot), [v3], "{ending:?}");
        let letter = ("pk", "k", 1, "v1", "c1", at(20), ended + DEAD_LETTER_TTL);
        assert_eq!(letter_rows(&snapshot), [letter], "{ending:?}");
        after_the_end(&mut fixture, ending, (5, 3), &["early", "k", "between"]);
    }
}

// scenario: S081
// obligation: rule-success.SupersedeUnsettledHeldVersion
// obligation: rule-entity-creation.SupersedeUnsettledHeldVersion.1
// obligation: rule-failure.RequeueUnsettledRecord.4
// obligation: guarantee.EveryDropIsVisible
// obligation: guarantee.HeldSetStableUntilLeaseEnds
// obligation: guarantee.ExpiryRecoversWithinTtl
#[test]
fn scenario_d_an_unsettled_version_is_superseded_at_the_end_and_v3_takes_its_position() {
    for ending in ENDINGS {
        let (mut fixture, lease) = v3_deferred_behind_v1();
        assert_eq!(fixture.snapshot().superseded.len(), 0, "{ending:?}");
        let ended = ending.end(&mut fixture, lease);
        let snapshot = fixture.snapshot();
        let v1 = (
            "pk",
            "k",
            1,
            "v1",
            ShedReason::Superseded,
            ended,
            ended + SUPERSEDED_TTL,
        );
        assert_eq!(shed_rows(&snapshot), [v1], "{ending:?}: shed at the end");
        assert_eq!(letter_rows(&snapshot), [], "{ending:?}");
        after_the_end(&mut fixture, ending, (3, 1), &["k", "early"]);
    }
}

// scenario: S087
// obligation: rule-success.ReleaseIdlesEmptyPocket
// obligation: rule-success.LeaseExpiryIdlesEmptyPocket
// obligation: rule-success.ConsumeAckedRecord
// obligation: rule-success.DeadLetterNackedRecord
// obligation: rule-failure.ReleaseRequeuesPocket.4
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.5
// obligation: projection.Pocket.ending_records
#[test]
fn a_pocket_whose_records_were_all_acked_or_nacked_idles_on_release_and_on_lapse() {
    for ending in ENDINGS {
        let mut fixture = two_pockets();
        let lease = fixture.pull("c1", 10).lease;
        fixture.at(20);
        fixture.ack("c1", lease, "a1").unwrap();
        fixture.nack("c1", lease, "a2").unwrap();
        assert_eq!(fixture.snapshot().dead_letters.len(), 0, "{ending:?}");
        let ended = ending.end(&mut fixture, lease);
        let pocket = fixture.pocket("a");
        assert_eq!(
            (
                pocket.status,
                pocket.consecutive_expiries,
                pocket.records.len()
            ),
            (PocketStatus::Idle, ending.count(), 0),
            "{ending:?}"
        );
        let letter = ("a", "a2", 1, "a2", "c1", at(20), ended + DEAD_LETTER_TTL);
        assert_eq!(letter_rows(&fixture.snapshot()), [letter], "{ending:?}");
        assert_eq!(fixture.order(), ["b"], "{ending:?}");
    }
}

// scenario: S088
// obligation: rule-success.ReleaseRequeuesPocket
// obligation: rule-success.LeaseExpiryKeepsPocketAtFront
// obligation: rule-success.ConsumeAckedRecord
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
// obligation: rule-failure.ReleaseIdlesEmptyPocket.4
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.5
#[test]
fn a_pocket_whose_records_were_all_acked_with_one_deferred_version_is_queued() {
    for ending in ENDINGS {
        let mut fixture = two_pockets();
        let lease = fixture.pull("c1", 10).lease;
        fixture.at(10);
        fixture.publish("a", "a2", 2, "newer").unwrap();
        assert_eq!(
            fixture.record("a", "a2").status,
            held_as(1, "a2", Mark::Unsettled, None, Some(5)),
            "{ending:?}"
        );
        fixture.at(20);
        for key in ["a1", "a2"] {
            fixture.ack("c1", lease, key).unwrap();
        }
        ending.end(&mut fixture, lease);
        let pocket = fixture.pocket("a");
        assert_eq!(
            (pocket.status, pocket.consecutive_expiries),
            (PocketStatus::Queued, ending.count()),
            "{ending:?}"
        );
        let [record] = pocket.records.as_slice() else {
            panic!("{ending:?}: only a2's newer version is left")
        };
        assert_eq!(
            (record.key.as_str(), record.status.clone(), record.rank),
            ("a2", RecordStatus::Queued, Rank::new(2)),
            "{ending:?}"
        );
        assert_eq!(record.position, Position::new(5), "{ending:?}");
        let snapshot = fixture.snapshot();
        assert_eq!(
            (shed_rows(&snapshot), letter_rows(&snapshot)),
            (vec![], vec![])
        );
    }
}

// scenario: S089
// obligation: rule-success.LeaseExpiryParksPocket
// obligation: rule-success.DeadLetterNackedRecord
// obligation: rule-entity-creation.DeadLetterNackedRecord.1
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.4
#[test]
fn at_an_expiry_limit_of_one_a_pocket_of_nacks_parks_with_no_records_and_its_dead_letters() {
    let mut limit_one = config();
    limit_one.max_consecutive_expiries = 1;
    let mut fixture = Fixture::with_config(limit_one);
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    fixture.at(20);
    for key in ["a1", "a2"] {
        fixture.nack("c1", lease, key).unwrap();
    }
    assert_eq!(
        fixture.snapshot().dead_letters.len(),
        0,
        "not before the lapse"
    );
    let ended = Ending::Lapse.end(&mut fixture, lease);
    let pocket = fixture.pocket("a");
    assert_eq!(
        (
            pocket.status,
            pocket.consecutive_expiries,
            pocket.records.len()
        ),
        (PocketStatus::Parked, 1, 0)
    );
    let expires_at = ended + DEAD_LETTER_TTL;
    assert_eq!(
        letter_rows(&fixture.snapshot()),
        [
            ("a", "a1", 1, "a1", "c1", at(20), expires_at),
            ("a", "a2", 1, "a2", "c1", at(20), expires_at)
        ]
    );
}

// scenario: S093
// obligation: rule-success.LeaseExpiryParksPocket
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
// obligation: rule-success.DeadLetterNackedVersionQueueDeferred
// obligation: rule-entity-creation.DeadLetterNackedVersionQueueDeferred.1
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.4
#[test]
fn at_an_expiry_limit_of_one_a_marked_version_with_a_deferred_one_parks_with_it_queued() {
    for mark in [Mark::Acked, Mark::Nacked] {
        let mut limit_one = config();
        limit_one.max_consecutive_expiries = 1;
        let mut fixture = Fixture::with_config(limit_one);
        fixture.publish("pk", "k", 1, "v1").unwrap();
        fixture.publish("pk", "early", 1, "e").unwrap();
        fixture.publish("other", "o", 1, "o").unwrap();
        let lease = fixture.pull("c1", 1).lease;
        fixture.at(10);
        fixture.publish("pk", "k", 3, "v3").unwrap();
        let held = held_as(1, "v1", Mark::Unsettled, None, Some(5));
        assert_eq!(fixture.record("pk", "k").status, held, "{mark:?}");
        fixture.at(20);
        fixture.mark_as("c1", lease, "k", mark);
        let ended = Ending::Lapse.end(&mut fixture, lease);
        let pocket = fixture.pocket("pk");
        assert_eq!(
            (pocket.status, pocket.consecutive_expiries),
            (PocketStatus::Parked, 1),
            "{mark:?}"
        );
        assert_eq!(fixture.order(), ["other"], "{mark:?}");
        let records: Vec<_> = (pocket.records.iter())
            .map(|r| {
                (
                    r.key.as_str(),
                    r.status.clone(),
                    r.rank.get(),
                    text(&r.payload),
                )
            })
            .collect();
        let queued = RecordStatus::Queued;
        let expected = [("early", queued.clone(), 1, "e"), ("k", queued, 3, "v3")];
        assert_eq!(records, expected, "{mark:?}: v3 is kept, queued");
        let positions: Vec<u64> = (pocket.records.iter())
            .map(|record| record.position.get())
            .collect();
        assert_eq!(positions, [2, 5], "{mark:?}: v3 at its arrival position");
        let snapshot = fixture.snapshot();
        assert_eq!(shed_rows(&snapshot), [], "{mark:?}");
        let letters = match mark {
            Mark::Nacked => vec![("pk", "k", 1, "v1", "c1", at(20), ended + DEAD_LETTER_TTL)],
            _ => vec![],
        };
        assert_eq!(letter_rows(&snapshot), letters, "{mark:?}");
    }
}

// scenario: S091
// obligation: rule-success.SupersedeUnsettledHeldVersion
// obligation: rule-success.DeadLetterNackedRecord
// obligation: guarantee.HeldSetStableUntilLeaseEnds
// obligation: guarantee.HeldRecordsOldestFirst
#[test]
fn the_entries_one_lease_end_creates_come_out_smallest_held_position_first() {
    let mut fixture = Fixture::new();
    // Positions 1 to 4; the key order is the reverse of the position order.
    for key in ["z", "y", "b", "a"] {
        fixture.publish("pk", key, 1, key).unwrap();
    }
    let lease = fixture.pull("c1", 10).lease;
    fixture.at(10);
    for key in ["b", "z"] {
        fixture.publish("pk", key, 2, "newer").unwrap();
    }
    assert_eq!(
        fixture.snapshot().superseded.len(),
        0,
        "nothing is shed while held"
    );
    fixture.at(20);
    for key in ["a", "y"] {
        fixture.nack("c1", lease, key).unwrap();
    }
    fixture.at(50);
    fixture.release("c1", lease).unwrap();
    let snapshot = fixture.snapshot();
    let (shed_at, shed_expiry) = (at(50), at(50) + SUPERSEDED_TTL);
    assert_eq!(
        shed_rows(&snapshot),
        [
            (
                "pk",
                "z",
                1,
                "z",
                ShedReason::Superseded,
                shed_at,
                shed_expiry
            ),
            (
                "pk",
                "b",
                1,
                "b",
                ShedReason::Superseded,
                shed_at,
                shed_expiry
            )
        ]
    );
    let letter_expiry = at(50) + DEAD_LETTER_TTL;
    assert_eq!(
        letter_rows(&snapshot),
        [
            ("pk", "y", 1, "y", "c1", at(20), letter_expiry),
            ("pk", "a", 1, "a", "c1", at(20), letter_expiry)
        ]
    );
    let positions: Vec<(&str, u64)> = (snapshot.pockets[0].records.iter())
        .map(|record| (record.key.as_str(), record.position.get()))
        .collect();
    assert_eq!(positions, [("b", 3), ("z", 1)], "held positions kept");
}
