//! Identity and publish: `PublisherJoins`, `ConsumerJoins`, the four publish rules
//! and the two revive rules, driven through the public API with the fakes.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Config, ConsumerId, CountingLeaseIds, Error, FakeClock, Payload, PocketFate, PocketKey,
    PocketStatus, PocketView, Position, Publication, PublishOutcome, PublisherId, PullOutcome,
    Queue, Rank, RecordKey, RecordStatus, ShedReason, Snapshot, SupersededRecord, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const SUPERSEDED_TTL: Duration = Duration::from_millis(5_000);

fn queue() -> (TestQueue, FakeClock) {
    let config = Config::new(
        Duration::from_millis(1_000),
        2,
        10,
        SUPERSEDED_TTL,
        Duration::from_millis(7_000),
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    queue.consumer_joins(ConsumerId::new("c")).unwrap();
    (queue, clock)
}

fn publication(pocket: &str, key: &str, rank: i64, payload: &str) -> Publication {
    Publication::new(
        PublisherId::new("p"),
        PocketKey::new(pocket),
        RecordKey::new(key),
        Rank::new(rank),
        Payload::from(payload.as_bytes()),
    )
}

fn pocket<'a>(snapshot: &'a Snapshot, key: &str) -> &'a PocketView {
    snapshot
        .pockets
        .iter()
        .find(|p| p.key.as_str() == key)
        .unwrap()
}

fn pull(queue: &mut TestQueue) -> libpachinko::Batch {
    match queue.pull(&ConsumerId::new("c"), 10).unwrap() {
        PullOutcome::Batch(batch) => batch,
        other => panic!("expected a batch, got {other:?}"),
    }
}

// rule-success.PublisherJoins, rule-entity-creation.PublisherJoins.1,
// rule-failure.PublisherJoins.1, invariant.UniquePublisherId, entity-fields.Publisher
#[test]
fn a_publisher_joins_once() {
    let (mut queue, _) = queue();
    assert_eq!(queue.publisher_joins(PublisherId::new("q")), Ok(()));
    assert_eq!(
        queue.snapshot().publishers,
        [PublisherId::new("p"), PublisherId::new("q")]
    );
    let before = queue.snapshot();
    assert_eq!(
        queue.publisher_joins(PublisherId::new("q")),
        Err(Error::PublisherExists(PublisherId::new("q")))
    );
    assert_eq!(queue.snapshot(), before);
}

// rule-success.ConsumerJoins, rule-entity-creation.ConsumerJoins.1,
// rule-failure.ConsumerJoins.1, invariant.UniqueConsumerId, entity-fields.Consumer
#[test]
fn a_consumer_joins_once() {
    let (mut queue, _) = queue();
    assert_eq!(queue.consumer_joins(ConsumerId::new("d")), Ok(()));
    assert_eq!(
        queue.snapshot().consumers,
        [ConsumerId::new("c"), ConsumerId::new("d")]
    );
    let before = queue.snapshot();
    assert_eq!(
        queue.consumer_joins(ConsumerId::new("d")),
        Err(Error::ConsumerExists(ConsumerId::new("d")))
    );
    assert_eq!(queue.snapshot(), before);
}

// surface-actor.Publishing: only a joined publisher publishes.
#[test]
fn a_publish_from_a_publisher_that_has_not_joined_is_refused() {
    let (mut queue, _) = queue();
    let before = queue.snapshot();
    let mut stranger = publication("a", "k1", 1, "x");
    stranger.publisher = PublisherId::new("stranger");
    assert_eq!(
        queue.publish(stranger),
        Err(Error::UnknownPublisher(PublisherId::new("stranger")))
    );
    assert_eq!(queue.snapshot(), before);
}

// rule-success.PublishIntoNewPocket, rule-entity-creation.PublishIntoNewPocket.1,
// rule-entity-creation.PublishIntoNewPocket.2, surface-provides.Publishing
#[test]
fn a_publish_into_a_new_pocket_creates_it_queued_with_its_record_behind_it() {
    let (mut queue, _) = queue();
    assert_eq!(
        queue.publish(publication("a", "k1", 4, "hello")),
        Ok(PublishOutcome::Created)
    );
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Queued);
    assert_eq!(a.position, Position::new(0));
    assert_eq!(a.consecutive_expiries, 0);
    assert_eq!(a.records.len(), 1);
    let record = &a.records[0];
    assert_eq!(record.key, RecordKey::new("k1"));
    assert_eq!(record.rank, Rank::new(4));
    assert_eq!(record.payload.as_bytes(), b"hello");
    assert_eq!(record.position, Position::new(1));
    assert_eq!(record.status, RecordStatus::Queued);
    assert_eq!(snapshot.next_position, Position::new(2));
    assert_eq!(snapshot.pocket_order, [PocketKey::new("a")]);
}

// rule-failure.PublishIntoNewPocket.1: the pocket exists, so another rule takes it.
#[test]
fn a_publish_into_an_existing_pocket_does_not_create_one() {
    let (mut queue, _) = queue();
    queue.publish(publication("a", "k1", 1, "x")).unwrap();
    assert_eq!(
        queue.publish(publication("a", "k2", 1, "y")),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(queue.snapshot().pockets.len(), 1);
}

// rule-success.PublishNewKey, rule-entity-creation.PublishNewKey.1
#[test]
fn a_new_key_joins_its_pocket_at_the_next_position() {
    let (mut queue, _) = queue();
    queue.publish(publication("a", "k1", 1, "x")).unwrap();
    queue.publish(publication("b", "k1", 1, "x")).unwrap();
    assert_eq!(
        queue.publish(publication("a", "k2", 9, "y")),
        Ok(PublishOutcome::NewKey)
    );
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    let k2 = a.records.iter().find(|r| r.key.as_str() == "k2").unwrap();
    assert_eq!(k2.position, Position::new(4));
    assert_eq!(k2.rank, Rank::new(9));
    assert_eq!(k2.payload.as_bytes(), b"y");
    assert_eq!(k2.status, RecordStatus::Queued);
    assert_eq!(snapshot.next_position, Position::new(5));
    assert_eq!(
        a.position,
        Position::new(0),
        "a queued pocket keeps its place"
    );
}

// rule-failure.PublishNewKey.1 (no pocket) and rule-failure.PublishNewKey.2 (key exists)
#[test]
fn a_new_key_needs_an_existing_pocket_and_an_unused_key() {
    let (mut queue, _) = queue();
    assert_eq!(
        queue.publish(publication("a", "k1", 1, "x")),
        Ok(PublishOutcome::Created)
    );
    assert_eq!(
        queue.publish(publication("a", "k1", 2, "y")),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(pocket(&queue.snapshot(), "a").records.len(), 1);
}

// rule-success.Supersede, rule-entity-creation.Supersede.1, invariant.ShedBeforeExpiry,
// entity-fields.SupersededRecord, and the OldestSlotNewestPayload guarantee
#[test]
fn a_higher_rank_supersedes_in_place_and_sheds_the_old_version() {
    let (mut queue, clock) = queue();
    queue.publish(publication("a", "k1", 1, "old")).unwrap();
    queue.publish(publication("a", "k2", 1, "x")).unwrap();
    clock.advance(Duration::from_millis(250));
    assert_eq!(
        queue.publish(publication("a", "k1", 3, "new")),
        Ok(PublishOutcome::Superseded)
    );
    clock.advance(Duration::from_millis(250));
    assert_eq!(
        queue.publish(publication("a", "k1", 8, "newest")),
        Ok(PublishOutcome::Superseded)
    );
    let snapshot = queue.snapshot();
    let k1 = &pocket(&snapshot, "a").records[0];
    assert_eq!(k1.rank, Rank::new(8));
    assert_eq!(k1.payload.as_bytes(), b"newest");
    assert_eq!(
        k1.position,
        Position::new(1),
        "the slot of its first publish"
    );
    assert_eq!(snapshot.next_position, Position::new(3));
    assert_eq!(snapshot.superseded.len(), 2);
    assert_first_shed(&snapshot.superseded[0]);
    assert_eq!(snapshot.superseded[1].rank, Rank::new(3));
}

/// The version the first supersede shed: k1 at rank 1, shed at 1250ms.
fn assert_first_shed(shed: &SupersededRecord) {
    assert_eq!(shed.pocket_key, PocketKey::new("a"));
    assert_eq!(shed.record_key, RecordKey::new("k1"));
    assert_eq!(shed.rank, Rank::new(1));
    assert_eq!(shed.payload.as_bytes(), b"old");
    assert_eq!(shed.reason, ShedReason::Superseded);
    assert_eq!(shed.shed_at, Timestamp::from_millis(1_250));
    assert_eq!(
        shed.expires_at,
        Timestamp::from_millis(1_250) + SUPERSEDED_TTL
    );
    assert!(shed.shed_at <= shed.expires_at);
}

// rule-failure.Supersede.1 (no pocket), rule-failure.Supersede.2 (no record),
// rule-failure.Supersede.3 (rank not higher)
#[test]
fn a_supersede_needs_the_pocket_the_record_and_a_higher_rank() {
    let (mut queue, _) = queue();
    assert_eq!(
        queue.publish(publication("a", "k1", 5, "x")),
        Ok(PublishOutcome::Created)
    );
    assert_eq!(
        queue.publish(publication("a", "k2", 5, "x")),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(
        queue.publish(publication("a", "k1", 4, "y")),
        Ok(PublishOutcome::Stale)
    );
    let snapshot = queue.snapshot();
    assert_eq!(pocket(&snapshot, "a").records[0].rank, Rank::new(5));
    assert!(
        snapshot
            .superseded
            .iter()
            .all(|s| s.reason == ShedReason::Stale)
    );
}

// rule-success.DropStalePublish, rule-entity-creation.DropStalePublish.1, and the
// EveryDropIsVisible guarantee
#[test]
fn a_lower_rank_is_shed_as_stale_and_the_record_is_untouched() {
    let (mut queue, clock) = queue();
    queue.publish(publication("a", "k1", 5, "kept")).unwrap();
    clock.advance(Duration::from_millis(40));
    assert_eq!(
        queue.publish(publication("a", "k1", 2, "late")),
        Ok(PublishOutcome::Stale)
    );
    let snapshot = queue.snapshot();
    let k1 = &pocket(&snapshot, "a").records[0];
    assert_eq!(
        (k1.rank, k1.payload.as_bytes()),
        (Rank::new(5), b"kept".as_slice())
    );
    assert_eq!(snapshot.next_position, Position::new(2));
    let shed = &snapshot.superseded[0];
    assert_eq!(shed.reason, ShedReason::Stale);
    assert_eq!(shed.rank, Rank::new(2));
    assert_eq!(shed.payload.as_bytes(), b"late");
    assert_eq!(shed.shed_at, Timestamp::from_millis(1_040));
    assert_eq!(
        shed.expires_at,
        Timestamp::from_millis(1_040) + SUPERSEDED_TTL
    );
}

// rule-failure.DropStalePublish.1 (no pocket), rule-failure.DropStalePublish.2 (no
// record), rule-failure.DropStalePublish.3 (rank not lower)
#[test]
fn a_stale_drop_needs_the_pocket_the_record_and_a_lower_rank() {
    let (mut queue, _) = queue();
    assert_eq!(
        queue.publish(publication("a", "k1", 1, "x")),
        Ok(PublishOutcome::Created)
    );
    assert_eq!(
        queue.publish(publication("a", "k2", -7, "x")),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(
        queue.publish(publication("a", "k1", 2, "y")),
        Ok(PublishOutcome::Superseded)
    );
    assert!(
        queue
            .snapshot()
            .superseded
            .iter()
            .all(|s| s.reason == ShedReason::Superseded)
    );
}

// The first open question's placeholder: no rule covers an equal rank.
#[test]
fn an_equal_rank_is_rejected_as_the_placeholder_says() {
    let (mut queue, _) = queue();
    queue.publish(publication("a", "k1", 3, "x")).unwrap();
    let before = queue.snapshot();
    let refused = queue.publish(publication("a", "k1", 3, "y"));
    assert_eq!(
        refused,
        Err(Error::EqualRank {
            pocket_key: PocketKey::new("a"),
            record_key: RecordKey::new("k1"),
            rank: Rank::new(3),
        })
    );
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("first open question")
    );
    assert_eq!(queue.snapshot(), before);
}

/// Publishes k1 into `a`, pulls it, acks it and releases, leaving `a` idle.
fn idle_pocket(queue: &mut TestQueue) {
    queue.publish(publication("a", "k1", 1, "x")).unwrap();
    let batch = pull(queue);
    queue
        .ack(&ConsumerId::new("c"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
    assert_eq!(
        queue.release(&ConsumerId::new("c"), batch.lease_id),
        Ok(PocketFate::Idle)
    );
}

// rule-success.ReviveIdlePocket, transition-edge.Pocket.idle.queued
#[test]
fn a_publish_into_an_idle_pocket_revives_it_at_the_back() {
    let (mut queue, _) = queue();
    idle_pocket(&mut queue);
    queue.publish(publication("b", "k1", 1, "x")).unwrap();
    let next = queue.snapshot().next_position.get();
    assert_eq!(
        queue.publish(publication("a", "k9", 1, "y")),
        Ok(PublishOutcome::NewKey)
    );
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Queued);
    assert_eq!(a.records[0].position, Position::new(next));
    assert_eq!(a.position, Position::new(next + 1));
    assert_eq!(a.consecutive_expiries, 0);
    assert_eq!(snapshot.next_position, Position::new(next + 2));
    assert_eq!(
        snapshot.pocket_order,
        [PocketKey::new("b"), PocketKey::new("a")]
    );
}

// rule-failure.ReviveIdlePocket.1 (no pocket: a new pocket is created, not revived),
// rule-failure.ReviveIdlePocket.2 and rule-failure.ReviveParkedPocket.2 (the pocket is
// neither idle nor parked: it keeps its status and position)
#[test]
fn a_publish_into_a_queued_or_leased_pocket_revives_nothing() {
    let (mut queue, _) = queue();
    queue.publish(publication("a", "k1", 1, "x")).unwrap();
    queue.publish(publication("a", "k2", 1, "x")).unwrap();
    assert_eq!(pocket(&queue.snapshot(), "a").position, Position::new(0));
    pull(&mut queue);
    queue.publish(publication("a", "k3", 1, "x")).unwrap();
    queue.publish(publication("a", "k1", 2, "x")).unwrap();
    queue.publish(publication("a", "k2", 0, "x")).unwrap();
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Leased);
    assert_eq!(a.position, Position::new(0));
    assert_eq!(snapshot.pocket_order, []);
}

/// Lets `a`'s lease lapse twice, which parks it at `max_consecutive_expiries = 2`.
fn parked_pocket(queue: &mut TestQueue, clock: &FakeClock) {
    queue.publish(publication("a", "k1", 1, "x")).unwrap();
    for _ in 0..2 {
        pull(queue);
        clock.advance(Duration::from_millis(1_000));
        queue.lapse();
    }
    assert_eq!(pocket(&queue.snapshot(), "a").status, PocketStatus::Parked);
}

// rule-success.ReviveParkedPocket, transition-edge.Pocket.parked.queued: every kind of
// publish into a parked pocket revives it.
#[test]
fn any_publish_into_a_parked_pocket_revives_it_at_the_back() {
    for (key, rank, outcome) in [
        ("k1", 2, PublishOutcome::Superseded),
        ("k1", 0, PublishOutcome::Stale),
        ("k2", 1, PublishOutcome::NewKey),
    ] {
        let (mut queue, clock) = queue();
        parked_pocket(&mut queue, &clock);
        assert_eq!(queue.snapshot().pocket_order, []);
        let next = queue.snapshot().next_position.get();
        assert_eq!(queue.publish(publication("a", key, rank, "y")), Ok(outcome));
        let snapshot = queue.snapshot();
        let a = pocket(&snapshot, "a");
        assert_eq!(a.status, PocketStatus::Queued, "{outcome:?}");
        assert_eq!(a.consecutive_expiries, 0, "{outcome:?}");
        assert_eq!(
            a.position.get(),
            snapshot.next_position.get() - 1,
            "{outcome:?}"
        );
        assert!(a.position.get() >= next, "{outcome:?}");
        assert_eq!(snapshot.pocket_order, [PocketKey::new("a")]);
    }
}

// rule-failure.ReviveParkedPocket.1: a publish naming no existing pocket creates one.
#[test]
fn a_publish_into_a_missing_pocket_revives_nothing_and_creates_it() {
    let (mut queue, clock) = queue();
    parked_pocket(&mut queue, &clock);
    assert_eq!(
        queue.publish(publication("b", "k1", 1, "x")),
        Ok(PublishOutcome::Created)
    );
    let snapshot = queue.snapshot();
    assert_eq!(pocket(&snapshot, "a").status, PocketStatus::Parked);
    assert_eq!(snapshot.pocket_order, [PocketKey::new("b")]);
}

// The NeverRefusedForCapacity guarantee: the prototype holds no bound, so no publish is
// refused for want of room.
#[test]
fn many_publishes_are_never_refused() {
    let (mut queue, _) = queue();
    for n in 0..2_000 {
        let key = format!("k{n}");
        assert!(queue.publish(publication("a", &key, 1, "x")).is_ok());
    }
    assert_eq!(pocket(&queue.snapshot(), "a").records.len(), 2_000);
}
