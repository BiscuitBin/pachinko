//! The two surfaces: who each faces, what each provides and exposes, and their
//! guarantees.

use libpachinko::error::{ActionError, ConsumerError, PublishError};
use libpachinko::queue::{LeaseId, PocketStatus, Publication, PublishOutcome, PullOutcome};
use libpachinko::records::{HeldRecord, RecordStatus};

use crate::fixture::{Fixture, LEASE_TTL, T0, at, keys};

fn held(key: &str, rank: i64, payload: &str) -> HeldRecord {
    HeldRecord::new(key.to_owned(), rank, payload.as_bytes().to_vec())
}

// obligation: surface-actor.Publishing
#[test]
fn only_a_joined_publisher_publishes() {
    let mut fixture = Fixture::new();
    let publication = Publication::new("pk", "a", 1, Vec::new());
    assert_eq!(
        fixture.queue.publish("c1", publication.clone()),
        Err(PublishError::UnknownPublisher("c1".to_owned()))
    );
    assert_eq!(
        fixture.queue.publish("p", publication),
        Ok(PublishOutcome::New)
    );
}

// obligation: surface-provides.Publishing
#[test]
fn publishing_provides_joining_and_publishing() {
    let mut fixture = Fixture::new();
    fixture.queue.publisher_joins("fresh").unwrap();
    let publication = Publication::new("pk", "a", 1, Vec::new());
    assert_eq!(
        fixture.queue.publish("fresh", publication),
        Ok(PublishOutcome::New)
    );
}

// obligation: surface-actor.Consuming
#[test]
fn only_a_joined_consumer_pulls_reads_or_acts() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    let unknown = ConsumerError::UnknownConsumer("p".to_owned());
    assert_eq!(fixture.queue.pull("p", 1).err(), Some(unknown.clone()));
    assert_eq!(fixture.queue.lease_view("p").err(), Some(unknown));
    assert_eq!(
        fixture.queue.ack("p", batch.lease, "a").err(),
        Some(ActionError::UnknownConsumer("p".to_owned()))
    );
}

// obligation: surface-provides.Consuming
// Joining and pulling are always provided; ack, nack, renew and release only for
// the consumer's active lease.
#[test]
fn consuming_provides_lease_actions_only_for_an_active_lease() {
    let mut fixture = Fixture::new();
    fixture.queue.consumer_joins("fresh").unwrap();
    assert_eq!(
        fixture.queue.pull("fresh", 1),
        Ok(PullOutcome::NoPocketAvailable)
    );
    let none = LeaseId::from_raw(999);
    let inactive = Err(ActionError::LeaseNotActive(none));
    assert_eq!(
        fixture.queue.ack("fresh", none, "a").map(|_| ()),
        inactive.clone()
    );
    assert_eq!(
        fixture.queue.nack("fresh", none, "a").map(|_| ()),
        inactive.clone()
    );
    assert_eq!(
        fixture.queue.renew("fresh", none).map(|_| ()),
        inactive.clone()
    );
    assert_eq!(fixture.queue.release("fresh", none).map(|_| ()), inactive);
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("fresh", 10);
    assert!(fixture.queue.renew("fresh", batch.lease).is_ok());
    assert!(fixture.queue.ack("fresh", batch.lease, "a").is_ok());
    assert!(fixture.queue.release("fresh", batch.lease).is_ok());
}

// obligation: surface-exposure.Consuming
#[test]
fn consuming_exposes_the_lease_pocket_expiry_and_held_versions() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.queue.lease_view("c1"), Ok(None));
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "b", 1, "b1").unwrap();
    fixture.publish("pk", "c", 1, "c1").unwrap();
    let batch = fixture.pull("c1", 2);
    fixture.publish("pk", "a", 2, "a2").unwrap();
    let view = fixture.queue.lease_view("c1").unwrap().unwrap();
    assert_eq!(view.lease, batch.lease);
    assert_eq!(view.pocket_key, "pk");
    assert_eq!(view.expires_at, at(T0, LEASE_TTL));
    assert_eq!(view.held, [held("a", 1, "a1"), held("b", 1, "b1")]);
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    let view = fixture.queue.lease_view("c1").unwrap().unwrap();
    assert_eq!(view.held, [held("b", 1, "b1")]);
    assert_eq!(fixture.queue.lease_view("c2"), Ok(None));
}

// @guarantee ExclusivePocket
#[test]
fn no_other_consumer_is_handed_a_record_from_a_leased_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    fixture.pull("c1", 1);
    assert_eq!(
        fixture.queue.pull("c2", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(
        fixture.record("pk", "b").unwrap().status,
        RecordStatus::Queued
    );
}

// @guarantee OldestFirst
#[test]
fn a_batch_holds_the_smallest_positions_in_order() {
    let mut fixture = Fixture::new();
    for key in ["d", "b", "a", "c"] {
        fixture.publish("pk", key, 1, key).unwrap();
    }
    fixture.publish("pk", "d", 2, "d2").unwrap();
    assert_eq!(keys(&fixture.pull("c1", 3)), ["d", "b", "a"]);
}

// @guarantee OneVisitPerKeyPerLease and NothingLostWhileHeld
#[test]
fn a_newer_version_while_held_waits_for_the_lease_to_end() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.publish("pk", "a", 2, "a2").unwrap();
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    assert_eq!(fixture.pull("c1", 10).records.len(), 0);
    fixture.queue.release("c1", batch.lease).unwrap();
    let next = fixture.pull("c2", 10);
    assert_eq!(next.records, [held("a", 2, "a2")]);
}

// @guarantee ExpiryRecoversWithinTtl
#[test]
fn a_silent_consumer_loses_its_lease_lease_ttl_after_its_last_action() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.advance(LEASE_TTL / 2);
    fixture.queue.renew("c1", batch.lease).unwrap();
    let last = fixture.now();
    fixture.advance(LEASE_TTL);
    assert_eq!(
        fixture.queue.tick().lapsed[0].expires_at,
        at(last, LEASE_TTL)
    );
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Queued);
    assert_eq!(fixture.pull("c2", 10).records, [held("a", 1, "a")]);
}

// @guarantee NeverRefusedForCapacity
#[test]
fn a_publish_is_never_refused_for_capacity() {
    let mut fixture = Fixture::new();
    for index in 0..2_000 {
        let key = format!("k{index}");
        assert!(fixture.publish("pk", &key, 1, "x").is_ok());
    }
    assert_eq!(fixture.pocket("pk").records.len(), 2_000);
}

// @guarantee EveryDropIsVisible
#[test]
fn every_lost_payload_becomes_a_superseded_record() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 2, "two").unwrap();
    fixture.publish("pk", "a", 3, "three").unwrap();
    fixture.publish("pk", "a", 1, "one").unwrap();
    let shed: Vec<Vec<u8>> = fixture
        .snapshot()
        .superseded
        .into_iter()
        .map(|record| record.payload)
        .collect();
    assert_eq!(shed, [b"two".to_vec(), b"one".to_vec()]);
}

// @guarantee OldestSlotNewestPayload
#[test]
fn a_superseded_record_keeps_the_position_of_its_first_publish() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "b", 1, "b1").unwrap();
    for rank in 2..10 {
        fixture.publish("pk", "a", rank, "later").unwrap();
    }
    let batch = fixture.pull("c1", 1);
    assert_eq!(batch.records, [held("a", 9, "later")]);
}
