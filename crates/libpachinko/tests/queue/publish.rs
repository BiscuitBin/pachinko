//! Publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`,
//! `DropStalePublish`, the equal-rank placeholder, and the `PocketTouched` dispatch
//! on a queued or leased pocket.

use libpachinko::error::PublishError;
use libpachinko::ids::{Payload, Position, Rank, RecordKey};
use libpachinko::queue::{PocketStatus, PublishOutcome, ShedReason};
use libpachinko::records::RecordStatus;

use crate::fixture::{Fixture, SUPERSEDED_TTL, T0, invariants, keys};

fn rank_and_payload(fixture: &Fixture, pocket: &str, key: &str) -> (i64, Vec<u8>) {
    let record = fixture.record(pocket, key);
    (record.rank.get(), record.payload.into_bytes())
}

// scenario: S011
// obligation: rule-success.PublishIntoNewPocket
// obligation: rule-entity-creation.PublishIntoNewPocket.1
// obligation: rule-entity-creation.PublishIntoNewPocket.2
// obligation: entity-fields.Queue
// obligation: entity-fields.Pocket
// obligation: entity-fields.Record
// obligation: entity-relationship.Pocket.records
// obligation: surface-provides.Publishing
#[test]
fn a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.snapshot().next_position, Position::new(0));
    assert_eq!(
        fixture.publish("pk", "a", 3, "x"),
        Ok(PublishOutcome::Created)
    );
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(pocket.position, Position::new(0));
    assert_eq!(pocket.consecutive_expiries, 0);
    let record = fixture.record("pk", "a");
    assert_eq!(record.position, Position::new(1));
    assert_eq!(record.status, RecordStatus::Queued);
    assert_eq!(rank_and_payload(&fixture, "pk", "a"), (3, b"x".to_vec()));
    assert_eq!(fixture.snapshot().next_position, Position::new(2));
    assert_eq!(fixture.order(), ["pk"]);
}

// scenario: S012
// obligation: rule-success.PublishNewKey
// obligation: rule-entity-creation.PublishNewKey.1
// obligation: rule-failure.PublishIntoNewPocket.1
// obligation: projection.Pocket.queued_records
#[test]
fn a_publish_of_a_new_key_appends_to_an_existing_pocket() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "x").unwrap();
    assert_eq!(
        fixture.publish("pk", "b", 7, "y"),
        Ok(PublishOutcome::NewKey)
    );
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.pockets.len(), 1);
    assert_eq!(fixture.pocket("pk").position, Position::new(0));
    let record = fixture.record("pk", "b");
    assert_eq!(
        (record.position, record.status),
        (Position::new(2), RecordStatus::Queued)
    );
    assert_eq!(snapshot.next_position, Position::new(3));
    assert_eq!(keys(&fixture.pull("c1", 10).records), ["a", "b"]);
}

// scenario: S013
// obligation: rule-failure.PublishIntoNewPocket.1
// obligation: rule-failure.PublishNewKey.1
#[test]
fn the_same_record_key_in_two_pockets_is_two_records() {
    let mut fixture = Fixture::new();
    assert_eq!(
        fixture.publish("one", "k", 1, "x"),
        Ok(PublishOutcome::Created)
    );
    assert_eq!(
        fixture.publish("two", "k", 1, "y"),
        Ok(PublishOutcome::Created)
    );
    assert_eq!(rank_and_payload(&fixture, "one", "k"), (1, b"x".to_vec()));
    assert_eq!(rank_and_payload(&fixture, "two", "k"), (1, b"y".to_vec()));
    assert_ne!(
        fixture.record("one", "k").position,
        fixture.record("two", "k").position
    );
    assert_eq!(fixture.snapshot().superseded.len(), 0);
}

// scenario: S014
// obligation: rule-success.Supersede
// obligation: rule-entity-creation.Supersede.1
// obligation: rule-failure.PublishNewKey.2
// obligation: entity-fields.SupersededRecord
// obligation: invariant.SupersededRecord.ShedBeforeExpiry
// obligation: guarantee.EveryDropIsVisible
#[test]
fn a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "old").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    fixture.advance(7);
    assert_eq!(
        fixture.publish("pk", "a", 2, "new"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "a");
    assert_eq!(
        (record.position, record.status),
        (Position::new(1), RecordStatus::Queued)
    );
    assert_eq!(rank_and_payload(&fixture, "pk", "a"), (2, b"new".to_vec()));
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.next_position, Position::new(3));
    let [shed] = snapshot.superseded.as_slice() else {
        panic!("one shed entry")
    };
    assert_eq!(shed.pocket_key.as_str(), "pk");
    assert_eq!(shed.key, RecordKey::new("a"));
    assert_eq!(
        (shed.rank, shed.payload.clone()),
        (Rank::new(1), Payload::from(b"old".as_slice()))
    );
    assert_eq!(shed.reason, ShedReason::Superseded);
    let now = T0.saturating_add_millis(7);
    assert_eq!((shed.shed_at, shed.expires_at), (now, now + SUPERSEDED_TTL));
    invariants::shed_before_expiry(&snapshot);
}

// scenario: S015
// obligation: guarantee.OldestSlotNewestPayload
// obligation: guarantee.EveryDropIsVisible
#[test]
fn repeated_supersede_keeps_the_first_slot_and_sheds_every_loser() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "v1").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    for (rank, payload) in [(2, "v2"), (3, "v3"), (4, "v4")] {
        assert_eq!(
            fixture.publish("pk", "a", rank, payload),
            Ok(PublishOutcome::Superseded)
        );
    }
    assert_eq!(fixture.record("pk", "a").position, Position::new(1));
    assert_eq!(rank_and_payload(&fixture, "pk", "a"), (4, b"v4".to_vec()));
    let shed: Vec<(i64, Vec<u8>)> = fixture
        .snapshot()
        .superseded
        .into_iter()
        .map(|s| (s.rank.get(), s.payload.into_bytes()))
        .collect();
    assert_eq!(
        shed,
        [
            (1, b"v1".to_vec()),
            (2, b"v2".to_vec()),
            (3, b"v3".to_vec())
        ]
    );
    let batch = fixture.pull("c1", 10);
    assert_eq!(keys(&batch.records), ["a", "b"]);
    assert_eq!(batch.records[0].rank, Rank::new(4));
}

// scenario: S016
// obligation: rule-success.DropStalePublish
// obligation: rule-entity-creation.DropStalePublish.1
// obligation: rule-failure.Supersede.3
// obligation: guarantee.EveryDropIsVisible
#[test]
fn a_stale_publish_is_shed_not_applied_and_not_an_error() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 5, "kept").unwrap();
    fixture.advance(3);
    assert_eq!(
        fixture.publish("pk", "a", 4, "late"),
        Ok(PublishOutcome::Stale)
    );
    assert_eq!(rank_and_payload(&fixture, "pk", "a"), (5, b"kept".to_vec()));
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.next_position, Position::new(2));
    let [shed] = snapshot.superseded.as_slice() else {
        panic!("one shed entry")
    };
    assert_eq!((shed.rank, shed.reason), (Rank::new(4), ShedReason::Stale));
    assert_eq!(shed.payload, Payload::from(b"late".as_slice()));
    let now = T0.saturating_add_millis(3);
    assert_eq!((shed.shed_at, shed.expires_at), (now, now + SUPERSEDED_TTL));
}

// scenario: S017
// obligation: rule-failure.DropStalePublish.3
// Open question 1 placeholder: no rule covers an equal rank, so the publish is
// refused with `EqualRank`; this pins the placeholder, not a decision.
#[test]
fn an_equal_rank_publish_is_rejected_as_the_open_question_1_placeholder() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 2, "first").unwrap();
    let before = fixture.snapshot();
    assert_eq!(
        fixture.publish("pk", "a", 2, "second"),
        Err(PublishError::EqualRank {
            key: RecordKey::new("a"),
            rank: Rank::new(2)
        })
    );
    assert_eq!(fixture.snapshot(), before);
}

// scenario: S018
// obligation: rule-failure.DropStalePublish.3
#[test]
fn negative_and_zero_ranks_order_like_integers() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", -5, "m5").unwrap();
    assert_eq!(
        fixture.publish("pk", "a", -1, "m1"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", 0, "z"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", -2, "m2"),
        Ok(PublishOutcome::Stale)
    );
    assert_eq!(rank_and_payload(&fixture, "pk", "a"), (0, b"z".to_vec()));
}

// scenario: S019
#[test]
fn large_ranks_keep_full_precision_across_supersede_and_stale() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", i64::MIN, "min").unwrap();
    assert_eq!(
        fixture.publish("pk", "a", i64::MIN + 1, "min+1"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", i64::MAX - 1, "max-1"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", i64::MAX, "max"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", i64::MAX - 1, "late"),
        Ok(PublishOutcome::Stale)
    );
    assert_eq!(
        rank_and_payload(&fixture, "pk", "a"),
        (i64::MAX, b"max".to_vec())
    );
    let shed: Vec<i64> = fixture
        .snapshot()
        .superseded
        .iter()
        .map(|s| s.rank.get())
        .collect();
    assert_eq!(shed, [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX - 1]);
    assert_eq!(fixture.pull("c1", 1).records[0].rank, Rank::new(i64::MAX));
}

// scenario: S020
#[test]
fn payload_bytes_and_keys_round_trip_unchanged() {
    let mut fixture = Fixture::new();
    let every_byte: Vec<u8> = (0..=u8::MAX).collect();
    let (pocket, key) = ("pocket-\u{e9}\u{4e2d}\u{1f600}", "key-\u{3b1}\u{3b2}\u{0}");
    let publication = libpachinko::queue::Publication::new(
        libpachinko::ids::PocketKey::new(pocket),
        RecordKey::new(key),
        Rank::new(1),
        Payload::new(every_byte.clone()),
    );
    fixture
        .queue
        .publish(&crate::fixture::publisher(), publication)
        .unwrap();
    let batch = fixture.pull("c1", 1);
    assert_eq!(batch.pocket_key.as_str(), pocket);
    assert_eq!(batch.records[0].key.as_str(), key);
    assert_eq!(batch.records[0].payload.clone().into_bytes(), every_byte);
}

// scenario: S021
#[test]
fn publishing_into_a_leased_pocket_neither_requeues_it_nor_escapes_the_lease() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    let pocket_before = fixture.pocket("pk");
    let lease_before = fixture.snapshot().leases;
    assert_eq!(
        fixture.publish("pk", "b", 1, "b"),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(
        fixture.publish("pk", "a", 2, "a2"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(
        fixture.publish("pk", "a", 0, "a0"),
        Ok(PublishOutcome::Stale)
    );
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Leased);
    assert_eq!(pocket.position, pocket_before.position);
    assert_eq!(fixture.order().len(), 0);
    assert_eq!(fixture.snapshot().leases, lease_before);
    assert!(matches!(
        fixture.try_pull("c2", 10),
        Ok(libpachinko::queue::PullOutcome::NoPocketAvailable)
    ));
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, batch.lease);
    assert_eq!(keys(&again.records), ["b"]);
}

// scenario: S022
#[test]
fn publishing_into_a_queued_pocket_does_not_move_it_in_the_pocket_order() {
    let mut fixture = Fixture::new();
    fixture.publish("first", "a", 1, "a").unwrap();
    fixture.publish("second", "a", 1, "a").unwrap();
    fixture.publish("first", "b", 1, "b").unwrap();
    fixture.publish("first", "a", 2, "a2").unwrap();
    fixture.publish("first", "a", 0, "a0").unwrap();
    assert_eq!(fixture.pocket("first").position, Position::new(0));
    assert_eq!(fixture.pocket("first").status, PocketStatus::Queued);
    assert_eq!(fixture.order(), ["first", "second"]);
    invariants::order_is_the_queued_pockets(&fixture.snapshot());
    assert_eq!(fixture.pull("c1", 0).pocket_key.as_str(), "first");
}

/// A publish storm over three pockets and three keys, every publish rule taken.
fn publish_storm() -> Fixture {
    let mut fixture = Fixture::new();
    // Ranks 0 (new), -1 (stale), 2 (supersede), 1 (stale), round by round.
    for rank in [0, -1, 2, 1] {
        for (pocket, key) in ["x", "y", "z"]
            .into_iter()
            .flat_map(|pocket| ["k1", "k2", "k3"].map(|key| (pocket, key)))
        {
            fixture.publish(pocket, key, rank, "v").unwrap();
        }
        fixture.advance(1);
    }
    fixture
}

// obligation: invariant.OnePocketPerKey
// obligation: rule-failure.Supersede.1
// obligation: rule-failure.DropStalePublish.1
#[test]
fn one_pocket_per_key_holds_after_publishes_into_new_and_existing_pockets() {
    let snapshot = publish_storm().snapshot();
    assert_eq!(snapshot.pockets.len(), 3);
    invariants::one_pocket_per_key(&snapshot);
}

// obligation: invariant.OneRecordPerKeyPerPocket
// obligation: rule-failure.Supersede.2
// obligation: rule-failure.DropStalePublish.2
#[test]
fn one_record_per_key_per_pocket_holds_after_new_keys_supersedes_and_stale_publishes() {
    let snapshot = publish_storm().snapshot();
    assert!(snapshot.pockets.iter().all(|p| p.records.len() == 3));
    invariants::one_record_per_key_per_pocket(&snapshot);
}

// obligation: invariant.UniquePocketPosition
// obligation: invariant.PocketPositionsBelowNext
#[test]
fn pocket_positions_are_unique_and_below_next_after_publishes() {
    let snapshot = publish_storm().snapshot();
    invariants::unique_pocket_position(&snapshot);
    invariants::pocket_positions_below_next(&snapshot);
    invariants::order_is_the_queued_pockets(&snapshot);
}

// obligation: invariant.UniqueRecordPosition
// obligation: invariant.RecordPositionsBelowNext
#[test]
fn record_positions_are_unique_and_below_next_after_publishes() {
    let snapshot = publish_storm().snapshot();
    invariants::unique_record_position(&snapshot);
    invariants::record_positions_below_next(&snapshot);
    assert_eq!(snapshot.next_position, Position::new(12));
}

// obligation: invariant.SupersededRecord.ShedBeforeExpiry
#[test]
fn every_shed_entry_is_shed_before_it_expires() {
    let snapshot = publish_storm().snapshot();
    assert_ne!(snapshot.superseded.len(), 0);
    invariants::shed_before_expiry(&snapshot);
}
