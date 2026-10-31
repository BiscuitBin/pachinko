//! Renew and release: `Renew`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`,
//! and the `Freed` rules `RequeueHeldRecord` and `RequeueDeferredRecord`.

use std::time::Duration;

use libpachinko::error::ActionError;
use libpachinko::queue::{LeaseStatus, PocketStatus};
use libpachinko::records::{RecordStatus, Version};

use crate::fixture::{Fixture, LEASE_TTL, T0, at};

// obligation: rule-success.Renew
#[test]
fn a_renew_moves_the_expiry_to_now_plus_the_ttl() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.advance(Duration::from_millis(700));
    let expires_at = fixture.queue.renew("c1", batch.lease).unwrap();
    assert_eq!(expires_at, at(fixture.now(), LEASE_TTL));
    assert_eq!(fixture.snapshot().leases[0].expires_at, expires_at);
    assert_eq!(fixture.snapshot().leases[0].granted_at, T0);
}

// obligation: rule-failure.Renew.1
// obligation: rule-failure.ReleaseRequeuesPocket.1
// obligation: rule-failure.ReleaseIdlesEmptyPocket.1
// obligation: transition-terminal.Lease.status
#[test]
fn an_ended_lease_cannot_be_renewed_or_released() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.queue.release("c1", batch.lease).unwrap();
    let before = fixture.snapshot();
    let ended = Err(ActionError::LeaseNotActive(batch.lease));
    assert_eq!(
        fixture.queue.renew("c1", batch.lease),
        ended.clone().map(|()| T0)
    );
    assert_eq!(
        fixture.queue.release("c1", batch.lease).map(|_| ()),
        ended.clone()
    );
    assert_eq!(fixture.queue.ack("c1", batch.lease, "a").map(|_| ()), ended);
    fixture.advance(LEASE_TTL * 10);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    assert_eq!(fixture.snapshot().pockets, before.pockets);
}

// obligation: rule-failure.Renew.2
// obligation: rule-failure.ReleaseRequeuesPocket.2
// obligation: rule-failure.ReleaseIdlesEmptyPocket.2
#[test]
fn another_consumers_lease_cannot_be_renewed_or_released() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    let before = fixture.snapshot();
    let theirs = Err(ActionError::LeaseNotYours(batch.lease));
    assert_eq!(
        fixture.queue.renew("c2", batch.lease).map(|_| ()),
        theirs.clone()
    );
    assert_eq!(fixture.queue.release("c2", batch.lease).map(|_| ()), theirs);
    assert_eq!(fixture.snapshot(), before);
}

// obligation: rule-success.ReleaseRequeuesPocket
// obligation: rule-success.RequeueHeldRecord
// obligation: rule-success.RequeueDeferredRecord
// obligation: rule-failure.ReleaseIdlesEmptyPocket.4
// obligation: transition-edge.Pocket.leased.queued
// obligation: transition-edge.Record.held.queued
// obligation: transition-edge.Record.deferred.queued
// obligation: transition-edge.Lease.active.released
// obligation: when-set.ReleaseRequeuesPocket.Lease.ended_at
// obligation: when-presence.Lease.ended_at
// obligation: invariant.Lease.EndedAfterGrant
#[test]
fn a_release_with_records_frees_them_and_requeues_the_pocket_at_the_back() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "b", 1, "b1").unwrap();
    fixture.publish("pk", "c", 1, "c1").unwrap();
    let batch = fixture.pull("c1", 2);
    fixture.publish("pk", "a", 2, "a2").unwrap();
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    let deferred_at = fixture.record("pk", "a").unwrap().position;
    let next = fixture.snapshot().next_position;
    fixture.advance(Duration::from_millis(250));
    let ended = fixture.queue.release("c1", batch.lease).unwrap();
    assert_eq!(ended.id, batch.lease);
    assert_eq!(ended.status, LeaseStatus::Released);
    assert_eq!(ended.ended_at, Some(fixture.now()));
    assert!(ended.ended_at >= Some(ended.granted_at));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(pocket.position, next);
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(fixture.snapshot().next_position, next + 1);
    let b = fixture.record("pk", "b").unwrap();
    assert_eq!(
        (b.status, b.held, b.position),
        (RecordStatus::Queued, None, 2)
    );
    let a = fixture.record("pk", "a").unwrap();
    assert_eq!((a.status, a.position), (RecordStatus::Queued, deferred_at));
    assert_eq!(a.latest, Version::new(2, b"a2".to_vec()));
    assert_eq!(fixture.snapshot().leases.len(), 0);
    assert_eq!(fixture.active_lease("c1"), None);
}

// obligation: rule-failure.RequeueHeldRecord.1
// obligation: rule-failure.RequeueHeldRecord.2
// obligation: rule-failure.RequeueDeferredRecord.1
// obligation: rule-failure.RequeueDeferredRecord.2
#[test]
fn a_release_leaves_queued_records_and_settled_keys_alone() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    fixture.publish("pk", "c", 1, "c").unwrap();
    let batch = fixture.pull("c1", 2);
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    let c_before = fixture.record("pk", "c").unwrap();
    fixture.queue.release("c1", batch.lease).unwrap();
    assert_eq!(fixture.record("pk", "a"), None);
    assert_eq!(fixture.record("pk", "c").unwrap(), c_before);
}

// obligation: rule-success.ReleaseIdlesEmptyPocket
// obligation: rule-failure.ReleaseRequeuesPocket.4
// obligation: transition-edge.Pocket.leased.idle
// obligation: when-set.ReleaseIdlesEmptyPocket.Lease.ended_at
// obligation: invariant.Pocket.IdleHasNoRecords
#[test]
fn a_release_of_an_empty_pocket_idles_it() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    let next = fixture.snapshot().next_position;
    let ended = fixture.queue.release("c1", batch.lease).unwrap();
    assert_eq!(ended.status, LeaseStatus::Released);
    assert_eq!(ended.ended_at, Some(fixture.now()));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Idle);
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(pocket.records.len(), 0);
    assert_eq!(fixture.snapshot().next_position, next);
    assert_eq!(
        fixture.queue.pull("c2", 10).unwrap(),
        libpachinko::queue::PullOutcome::NoPocketAvailable
    );
}

// A release resets the pocket's expiry count, with records or without.
#[test]
fn a_release_resets_the_expiry_count() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    fixture.advance(LEASE_TTL);
    fixture.queue.tick();
    assert_eq!(fixture.pocket("pk").consecutive_expiries, 1);
    let batch = fixture.pull("c1", 10);
    fixture.queue.release("c1", batch.lease).unwrap();
    assert_eq!(fixture.pocket("pk").consecutive_expiries, 0);
}

#[test]
fn an_unknown_consumer_cannot_renew_or_release() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    let unknown = ActionError::UnknownConsumer("stranger".to_owned());
    assert_eq!(
        fixture.queue.renew("stranger", batch.lease).err(),
        Some(unknown.clone())
    );
    assert_eq!(
        fixture.queue.release("stranger", batch.lease).err(),
        Some(unknown)
    );
}

// Lease ids are never reused: each grant takes a larger one.
#[test]
fn every_grant_takes_a_new_larger_lease_id() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let mut seen = Vec::new();
    for _ in 0..3 {
        let batch = fixture.pull("c1", 0);
        seen.push(batch.lease);
        fixture.queue.release("c1", batch.lease).unwrap();
    }
    assert!(seen.windows(2).all(|pair| pair[0] < pair[1]));
}
