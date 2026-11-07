//! Renew, release, revive, fencing and the consumer's view: `Renew`,
//! `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `RequeueHeldRecord`,
//! `ReviveIdlePocket`, and the guard order every lease action shares.

use std::time::Duration;

use libpachinko::error::LeaseActionError;
use libpachinko::ids::{Payload, Rank};
use libpachinko::lease_ids::LeaseId;
use libpachinko::queue::{
    LeaseStatus, PocketFate, PocketStatus, PublishOutcome, PullOutcome, SettleOutcome,
};
use libpachinko::records::RecordStatus;

use crate::fixture::{Fixture, LEASE_TTL, T0, config, consumer, invariants, keys};

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
// obligation: rule-success.RequeueHeldRecord
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
// obligation: rule-success.RequeueDeferredRecord
// obligation: rule-failure.RequeueHeldRecord.2
#[test]
fn a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it() {
    let (mut fixture, lease) = short_lease();
    fixture.publish("pk", "k", 2, "v2").unwrap();
    assert_eq!(fixture.ack("c1", lease, "k"), Ok(SettleOutcome::Deferred));
    let (fate, _) = fixture.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Queued);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Queued);
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank),
        (RecordStatus::Queued, Rank::new(2))
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
// obligation: rule-failure.AckSettlesRecord.1
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
    assert_eq!(
        fixture.ack("c1", new.lease, "k"),
        Ok(SettleOutcome::Settled)
    );
}

// scenario: S070
// obligation: rule-failure.AckSettlesRecord.2
// obligation: rule-failure.NackSettlesRecord.2
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
    let view = fixture.lease_view("c1").unwrap();
    assert_eq!(view.pocket_key.as_str(), "pk");
    assert_eq!(view.expires_at, T0 + LEASE_TTL);
    assert_eq!(keys(&view.records), ["k"], "only the held records");
    assert_eq!(
        (view.records[0].rank, &view.records[0].payload),
        (Rank::new(1), &Payload::from(b"v1".as_slice())),
        "the held version, not the latest"
    );
    fixture.release("c1", lease).unwrap();
    assert_eq!(fixture.lease_view("c1"), None);
    assert_eq!(keys(&fixture.lease_view("c2").unwrap().records), ["x"]);
}
