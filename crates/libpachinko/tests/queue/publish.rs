//! Publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`,
//! `DropStalePublish`, the equal-rank placeholder, and the revive rules.

use libpachinko::error::PublishError;
use libpachinko::queue::{PocketStatus, Publication, PublishOutcome, ShedReason};
use libpachinko::records::{RecordStatus, Version};

use crate::fixture::{DECAY_TTL, Fixture, LEASE_TTL, T0, at, config};

fn version(rank: i64, payload: &str) -> Version {
    Version::new(rank, payload.as_bytes().to_vec())
}

/// A fixture whose pocket `pk` is idle: published, pulled, acked and released.
fn idle_pocket() -> Fixture {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    fixture.queue.release("c1", batch.lease).unwrap();
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Idle);
    fixture
}

/// A fixture whose pocket `pk` is parked after one lapse, holding `a` at rank 1.
fn parked_pocket() -> Fixture {
    let mut config = config();
    config.max_consecutive_expiries = 1;
    let mut fixture = Fixture::with_config(config);
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.pull("c1", 10);
    fixture.advance(LEASE_TTL);
    fixture.queue.tick();
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Parked);
    fixture
}

// obligation: rule-success.PublishIntoNewPocket
// obligation: rule-entity-creation.PublishIntoNewPocket.1
// obligation: rule-entity-creation.PublishIntoNewPocket.2
// obligation: entity-fields.Pocket
// obligation: entity-fields.Record
// obligation: entity-relationship.Pocket.records
#[test]
fn a_publish_into_a_new_pocket_creates_the_pocket_then_its_record() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.publish("pk", "a", 3, "x"), Ok(PublishOutcome::New));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(pocket.position, 0);
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(pocket.records.len(), 1);
    let record = &pocket.records[0];
    assert_eq!(record.key, "a");
    assert_eq!(record.latest, version(3, "x"));
    assert_eq!(record.position, 1);
    assert_eq!(record.status, RecordStatus::Queued);
    assert_eq!(record.held, None);
    assert_eq!(fixture.snapshot().next_position, 2);
}

// obligation: rule-failure.PublishIntoNewPocket.1
#[test]
fn a_publish_into_an_existing_pocket_creates_no_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    fixture.publish("pk", "b", 1, "y").unwrap();
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.pockets.len(), 1);
    assert_eq!(snapshot.pockets[0].position, 0);
}

// obligation: rule-success.PublishNewKey
// obligation: rule-entity-creation.PublishNewKey.1
// obligation: projection.Pocket.queued_records
#[test]
fn a_new_key_in_an_existing_pocket_takes_the_next_position() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    assert_eq!(fixture.publish("pk", "b", 7, "y"), Ok(PublishOutcome::New));
    let record = fixture.record("pk", "b").unwrap();
    assert_eq!(record.latest, version(7, "y"));
    assert_eq!(record.position, 2);
    assert_eq!(record.status, RecordStatus::Queued);
    assert_eq!(fixture.snapshot().next_position, 3);
    let queued: Vec<String> = fixture
        .pocket("pk")
        .records
        .into_iter()
        .filter(|record| record.status == RecordStatus::Queued)
        .map(|record| record.key)
        .collect();
    assert_eq!(queued, ["a", "b"]);
    // A queued pocket is not revived: its position stays.
    assert_eq!(fixture.pocket("pk").position, 0);
}

// obligation: rule-failure.PublishNewKey.1
#[test]
fn a_new_key_without_a_pocket_is_the_new_pocket_rule() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    assert_eq!(
        fixture.publish("other", "a", 1, "x"),
        Ok(PublishOutcome::New)
    );
    assert_eq!(fixture.pocket("other").position, 2);
    assert_eq!(fixture.record("other", "a").unwrap().position, 3);
}

// obligation: rule-failure.PublishNewKey.2
#[test]
fn an_existing_key_is_not_added_again() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    fixture.publish("pk", "a", 2, "y").unwrap();
    assert_eq!(fixture.pocket("pk").records.len(), 1);
    assert_eq!(fixture.snapshot().next_position, 2);
}

// obligation: rule-success.Supersede
// obligation: rule-entity-creation.Supersede.1
// obligation: entity-fields.SupersededRecord
// obligation: invariant.SupersededRecord.ShedBeforeExpiry
#[test]
fn a_higher_rank_supersedes_in_place_and_sheds_the_old_version() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "old").unwrap();
    assert_eq!(
        fixture.publish("pk", "a", 2, "new"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(record.latest, version(2, "new"));
    assert_eq!(record.position, 1);
    assert_eq!(fixture.snapshot().next_position, 2);
    let shed = &fixture.snapshot().superseded[0];
    assert_eq!(
        (shed.pocket_key.as_str(), shed.key.as_str(), shed.rank),
        ("pk", "a", 1)
    );
    assert_eq!(shed.payload, b"old");
    assert_eq!(shed.reason, ShedReason::Superseded);
    assert_eq!(shed.shed_at, T0);
    assert_eq!(shed.expires_at, at(T0, DECAY_TTL));
    assert!(shed.shed_at <= shed.expires_at);
}

// A supersede of a held record changes its rank and payload only: the held
// version, the status and the position stay.
#[test]
fn a_supersede_of_a_held_record_keeps_the_held_version() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "old").unwrap();
    fixture.pull("c1", 10);
    fixture.publish("pk", "a", 2, "new").unwrap();
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(record.status, RecordStatus::Held);
    assert_eq!(record.held, Some(version(1, "old")));
    assert_eq!(record.latest, version(2, "new"));
    assert_eq!(record.position, 1);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
}

// obligation: rule-failure.Supersede.1
// obligation: rule-failure.DropStalePublish.1
#[test]
fn without_a_pocket_nothing_is_superseded_or_dropped() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.publish("pk", "a", 5, "x"), Ok(PublishOutcome::New));
    assert_eq!(fixture.snapshot().superseded.len(), 0);
}

// obligation: rule-failure.Supersede.2
// obligation: rule-failure.DropStalePublish.2
#[test]
fn without_a_record_nothing_is_superseded_or_dropped() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 5, "x").unwrap();
    assert_eq!(fixture.publish("pk", "b", 1, "y"), Ok(PublishOutcome::New));
    assert_eq!(fixture.snapshot().superseded.len(), 0);
}

// obligation: rule-failure.Supersede.3
// obligation: rule-success.DropStalePublish
// obligation: rule-entity-creation.DropStalePublish.1
#[test]
fn a_lower_rank_is_dropped_as_stale_and_the_record_is_untouched() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 5, "kept").unwrap();
    assert_eq!(
        fixture.publish("pk", "a", 4, "late"),
        Ok(PublishOutcome::Stale)
    );
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(record.latest, version(5, "kept"));
    assert_eq!(record.position, 1);
    let shed = &fixture.snapshot().superseded[0];
    assert_eq!((shed.key.as_str(), shed.rank), ("a", 4));
    assert_eq!(shed.payload, b"late");
    assert_eq!(shed.reason, ShedReason::Stale);
    assert_eq!(shed.shed_at, T0);
    assert_eq!(shed.expires_at, at(T0, DECAY_TTL));
}

// obligation: rule-failure.DropStalePublish.3
#[test]
fn a_higher_rank_is_not_dropped_as_stale() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 5, "x").unwrap();
    fixture.publish("pk", "a", 6, "y").unwrap();
    let shed = &fixture.snapshot().superseded[0];
    assert_eq!(shed.reason, ShedReason::Superseded);
    assert_eq!(shed.rank, 5);
}

// No rule covers rank = stored rank: the module's placeholder rejects it.
#[test]
fn an_equal_rank_is_rejected_and_changes_nothing() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 5, "x").unwrap();
    let before = fixture.snapshot();
    assert_eq!(
        fixture.publish("pk", "a", 5, "y"),
        Err(PublishError::EqualRank {
            pocket_key: "pk".to_owned(),
            record_key: "a".to_owned(),
            rank: 5,
        })
    );
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn an_unknown_publisher_is_rejected_and_changes_nothing() {
    let mut fixture = Fixture::new();
    let before = fixture.snapshot();
    let publication = Publication::new("pk", "a", 1, Vec::new());
    assert_eq!(
        fixture.queue.publish("stranger", publication),
        Err(PublishError::UnknownPublisher("stranger".to_owned()))
    );
    assert_eq!(fixture.snapshot(), before);
}

// obligation: rule-success.ReviveIdlePocket
// obligation: transition-edge.Pocket.idle.queued
#[test]
fn a_publish_into_an_idle_pocket_revives_it_at_the_back() {
    let mut fixture = idle_pocket();
    let next = fixture.snapshot().next_position;
    fixture.publish("pk", "b", 1, "b1").unwrap();
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    // The record takes `next`, then the revive takes the one after.
    assert_eq!(fixture.record("pk", "b").unwrap().position, next);
    assert_eq!(pocket.position, next + 1);
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(fixture.snapshot().next_position, next + 2);
}

// obligation: rule-failure.ReviveIdlePocket.1
// obligation: rule-failure.ReviveParkedPocket.1
#[test]
fn a_publish_that_creates_a_pocket_revives_nothing() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    // Positions 0 and 1 only: no revive took a third.
    assert_eq!(fixture.snapshot().next_position, 2);
}

// obligation: rule-failure.ReviveIdlePocket.2
// obligation: rule-failure.ReviveParkedPocket.2
#[test]
fn a_touch_on_a_queued_or_leased_pocket_revives_nothing() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    fixture.publish("pk", "a", 2, "y").unwrap();
    assert_eq!(fixture.pocket("pk").position, 0);
    fixture.pull("c1", 10);
    fixture.publish("pk", "a", 3, "z").unwrap();
    fixture.publish("pk", "a", 1, "stale").unwrap();
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Leased);
    assert_eq!(pocket.position, 0);
    assert_eq!(fixture.snapshot().next_position, 2);
}

// obligation: rule-success.ReviveParkedPocket
// obligation: transition-edge.Pocket.parked.queued
#[test]
fn a_supersede_into_a_parked_pocket_revives_it_at_the_back() {
    let mut fixture = parked_pocket();
    let next = fixture.snapshot().next_position;
    fixture.publish("pk", "a", 2, "a2").unwrap();
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(pocket.position, next);
    assert_eq!(pocket.consecutive_expiries, 0);
    assert_eq!(fixture.snapshot().next_position, next + 1);
}

// Every publish into an existing pocket touches it, a stale one included.
#[test]
fn a_stale_publish_into_a_parked_pocket_revives_it() {
    let mut fixture = parked_pocket();
    assert_eq!(
        fixture.publish("pk", "a", 0, "old"),
        Ok(PublishOutcome::Stale)
    );
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Queued);
}

// The equal-rank placeholder matches no rule, so it emits no `PocketTouched`.
#[test]
fn an_equal_rank_publish_into_a_parked_pocket_leaves_it_parked() {
    let mut fixture = parked_pocket();
    assert!(fixture.publish("pk", "a", 1, "same").is_err());
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Parked);
}
