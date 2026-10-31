//! Pull: `PullClaimsFrontPocket`, `PullWithinLease`, `HoldRecord` and
//! `PullFindsNoPocket`.

use std::time::Duration;

use libpachinko::error::ConsumerError;
use libpachinko::queue::{LeaseStatus, PocketStatus, PullOutcome};
use libpachinko::records::{RecordStatus, Version};

use crate::fixture::{Fixture, LEASE_TTL, T0, at, config, keys};

// obligation: rule-success.PullClaimsFrontPocket
// obligation: rule-entity-creation.PullClaimsFrontPocket.1
// obligation: entity-fields.Lease
// obligation: entity-relationship.Consumer.leases
// obligation: projection.Consumer.active_leases
// obligation: entity-relationship.Pocket.leases
// obligation: projection.Pocket.active_leases
// obligation: transition-edge.Pocket.queued.leased
// obligation: invariant.Lease.GrantedBeforeExpiry
#[test]
fn a_pull_without_a_lease_claims_the_front_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("first", "a", 1, "a").unwrap();
    fixture.publish("second", "b", 1, "b").unwrap();
    let batch = fixture.pull("c1", 10);
    assert_eq!(batch.pocket_key, "first");
    assert_eq!(batch.expires_at, at(T0, LEASE_TTL));
    assert_eq!(fixture.pocket("first").status, PocketStatus::Leased);
    assert_eq!(fixture.pocket("second").status, PocketStatus::Queued);
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.leases.len(), 1);
    let lease = &snapshot.leases[0];
    assert_eq!(lease.id, batch.lease);
    assert_eq!(lease.pocket_key, "first");
    assert_eq!(lease.consumer_id, "c1");
    assert_eq!(lease.status, LeaseStatus::Active);
    assert_eq!(lease.granted_at, T0);
    assert_eq!(lease.expires_at, at(T0, LEASE_TTL));
    assert_eq!(lease.ended_at, None);
    assert!(lease.granted_at <= lease.expires_at);
    assert_eq!(fixture.active_lease("c1"), Some(batch.lease));
}

// obligation: rule-success.HoldRecord
// obligation: transition-edge.Record.queued.held
// obligation: when-set.HoldRecord.Record.held_rank
// obligation: when-set.HoldRecord.Record.held_payload
// obligation: when-presence.Record.held_rank
// obligation: when-presence.Record.held_payload
// obligation: projection.Pocket.held_records
#[test]
fn a_pull_holds_the_oldest_records_with_their_latest_versions() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "b", 1, "b1").unwrap();
    fixture.publish("pk", "c", 1, "c1").unwrap();
    fixture.publish("pk", "a", 2, "a2").unwrap();
    assert_eq!(fixture.record("pk", "a").unwrap().held, None);
    let batch = fixture.pull("c1", 2);
    assert_eq!(keys(&batch), ["a", "b"]);
    assert_eq!(batch.records[0].rank, 2);
    assert_eq!(batch.records[0].payload, b"a2");
    let a = fixture.record("pk", "a").unwrap();
    assert_eq!(a.status, RecordStatus::Held);
    assert_eq!(a.held, Some(Version::new(2, b"a2".to_vec())));
    let c = fixture.record("pk", "c").unwrap();
    assert_eq!((c.status, c.held), (RecordStatus::Queued, None));
}

// obligation: rule-failure.PullClaimsFrontPocket.1
// obligation: rule-success.PullWithinLease
#[test]
fn a_pull_under_a_lease_continues_in_its_pocket_and_extends_it() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    fixture.publish("other", "x", 1, "x").unwrap();
    let first = fixture.pull("c1", 1);
    fixture.advance(Duration::from_millis(400));
    let second = fixture.pull("c1", 10);
    assert_eq!(second.lease, first.lease);
    assert_eq!(second.pocket_key, "pk");
    assert_eq!(keys(&second), ["b"]);
    assert_eq!(second.expires_at, at(fixture.now(), LEASE_TTL));
    assert_eq!(fixture.snapshot().leases.len(), 1);
    assert_eq!(fixture.pocket("other").status, PocketStatus::Queued);
}

// obligation: rule-failure.HoldRecord.2
#[test]
fn a_held_record_is_not_handed_out_again_under_its_lease() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    fixture.publish("pk", "a", 2, "a2").unwrap();
    let again = fixture.pull("c1", 10);
    assert_eq!(again.records.len(), 0);
    assert_eq!(
        fixture.record("pk", "a").unwrap().held,
        Some(Version::new(1, b"a".to_vec()))
    );
}

// obligation: rule-failure.HoldRecord.1
// `Handed` names only records the batch selected, which exist by construction; a
// key the pocket lacks is never handed, so a pull holds nothing beyond its pocket.
#[test]
fn a_pull_holds_only_records_of_its_own_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("other", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    assert_eq!(
        fixture.record("other", "a").unwrap().status,
        RecordStatus::Queued
    );
}

// obligation: rule-failure.PullWithinLease.1
#[test]
fn a_pull_without_a_lease_does_not_continue_an_ended_one() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    let first = fixture.pull("c1", 1);
    fixture.queue.release("c1", first.lease).unwrap();
    let second = fixture.pull("c1", 10);
    assert_ne!(second.lease, first.lease);
    assert_eq!(keys(&second), ["a", "b"]);
}

// obligation: rule-failure.PullClaimsFrontPocket.3
// The claimed pocket is the queued one with the smallest position: a pocket
// another consumer holds is never claimed.
#[test]
fn a_pull_never_claims_a_leased_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("first", "a", 1, "a").unwrap();
    fixture.publish("second", "b", 1, "b").unwrap();
    fixture.pull("c1", 10);
    let batch = fixture.pull("c2", 10);
    assert_eq!(batch.pocket_key, "second");
}

// obligation: rule-success.PullFindsNoPocket
// obligation: rule-failure.PullClaimsFrontPocket.2
#[test]
fn a_pull_without_a_lease_or_a_queued_pocket_finds_no_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    let before = fixture.snapshot();
    assert_eq!(
        fixture.queue.pull("c2", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(fixture.snapshot(), before);
}

// obligation: rule-failure.PullFindsNoPocket.1
#[test]
fn a_pull_under_a_lease_is_a_batch_even_when_nothing_is_queued() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    let again = fixture.pull("c1", 10);
    assert_eq!(again.records.len(), 0);
}

// obligation: rule-failure.PullFindsNoPocket.2
#[test]
fn a_pull_with_a_queued_pocket_finds_one() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    assert!(matches!(
        fixture.queue.pull("c1", 10),
        Ok(PullOutcome::Batch(_))
    ));
}

// A limit that is not positive still claims and yields an empty batch.
#[test]
fn a_non_positive_limit_claims_and_holds_nothing() {
    for limit in [0, -1, i64::MIN] {
        let mut fixture = Fixture::new();
        fixture.publish("pk", "a", 1, "a").unwrap();
        let batch = fixture.pull("c1", limit);
        assert_eq!(batch.records.len(), 0);
        assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
        assert_eq!(
            fixture.record("pk", "a").unwrap().status,
            RecordStatus::Queued
        );
    }
}

// The batch never exceeds `max_pull_batch`, whatever the limit.
#[test]
fn a_batch_is_capped_at_max_pull_batch() {
    let mut config = config();
    config.max_pull_batch = 2;
    let mut fixture = Fixture::with_config(config);
    for key in ["a", "b", "c"] {
        fixture.publish("pk", key, 1, key).unwrap();
    }
    assert_eq!(keys(&fixture.pull("c1", i64::MAX)), ["a", "b"]);
    assert_eq!(keys(&fixture.pull("c1", 1)), ["c"]);
}

#[test]
fn an_unknown_consumer_cannot_pull() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    assert_eq!(
        fixture.queue.pull("stranger", 1),
        Err(ConsumerError::UnknownConsumer("stranger".to_owned()))
    );
}

// The pocket order is by position: a revived pocket rejoins at the back.
#[test]
fn a_revived_pocket_queues_behind_older_pockets() {
    let mut fixture = Fixture::new();
    fixture.publish("revived", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    fixture.queue.release("c1", batch.lease).unwrap();
    fixture.publish("older", "x", 1, "x").unwrap();
    fixture.publish("revived", "b", 1, "b").unwrap();
    assert_eq!(fixture.pull("c2", 10).pocket_key, "older");
    assert_eq!(fixture.pull("c1", 10).pocket_key, "revived");
}
