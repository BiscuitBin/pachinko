//! Decay: `SupersededRecordDecays` and `DeadLetterDecays`. The fake clock stands
//! before, at and after each `expires_at`; nothing sleeps.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Config, ConsumerId, CountingLeaseIds, Decayed, FakeClock, Payload, PocketKey, Publication,
    PublisherId, PullOutcome, Queue, Rank, RecordKey, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const SUPERSEDED_TTL: Duration = Duration::from_millis(5_000);
const DEAD_LETTER_TTL: Duration = Duration::from_millis(7_000);

fn queue_with(superseded_ttl: Duration, dead_letter_ttl: Duration) -> (TestQueue, FakeClock) {
    let config = Config::new(
        Duration::from_secs(100),
        2,
        10,
        superseded_ttl,
        dead_letter_ttl,
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    queue.consumer_joins(ConsumerId::new("c")).unwrap();
    (queue, clock)
}

fn publish(queue: &mut TestQueue, key: &str, rank: i64) {
    queue
        .publish(Publication::new(
            PublisherId::new("p"),
            PocketKey::new("a"),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::default(),
        ))
        .unwrap();
}

fn decayed(superseded: usize, dead_letters: usize) -> (usize, usize) {
    (superseded, dead_letters)
}

fn counts(decayed: Decayed) -> (usize, usize) {
    (decayed.superseded, decayed.dead_letters)
}

fn just_before(t: Timestamp) -> Timestamp {
    Timestamp::from_millis(t.as_millis() - 1)
}

/// Sheds one superseded record at START and nacks one record at START + 10ms.
fn one_of_each(queue: &mut TestQueue, clock: &FakeClock) {
    publish(queue, "k1", 1);
    publish(queue, "k1", 2);
    clock.advance(Duration::from_millis(10));
    let PullOutcome::Batch(batch) = queue.pull(&ConsumerId::new("c"), 1).unwrap() else {
        panic!("a pocket is queued");
    };
    queue
        .nack(&ConsumerId::new("c"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
}

// rule-success.SupersededRecordDecays, temporal.SupersededRecordDecays,
// rule-success.DeadLetterDecays, temporal.DeadLetterDecays: not before expires_at, at
// it, and not again.
#[test]
fn each_entry_decays_at_its_expires_at_and_only_once() {
    let (mut queue, clock) = queue_with(SUPERSEDED_TTL, DEAD_LETTER_TTL);
    one_of_each(&mut queue, &clock);
    let shed_expires = START + SUPERSEDED_TTL;
    let letter_expires = START + Duration::from_millis(10) + DEAD_LETTER_TTL;
    assert_eq!(queue.snapshot().superseded[0].expires_at, shed_expires);
    assert_eq!(queue.snapshot().dead_letters[0].expires_at, letter_expires);

    clock.set(just_before(shed_expires));
    assert_eq!(counts(queue.decay()), decayed(0, 0));
    clock.set(shed_expires);
    assert_eq!(counts(queue.decay()), decayed(1, 0));
    assert_eq!(queue.snapshot().superseded, []);
    assert_eq!(queue.snapshot().dead_letters.len(), 1);

    clock.set(just_before(letter_expires));
    assert_eq!(counts(queue.decay()), decayed(0, 0));
    clock.set(letter_expires);
    assert_eq!(counts(queue.decay()), decayed(0, 1));
    assert_eq!(queue.snapshot().dead_letters, []);

    clock.advance(Duration::from_secs(3_600));
    assert_eq!(counts(queue.decay()), decayed(0, 0));
}

// A zero TTL is admitted: expires_at equals shed_at and nacked_at, the
// ShedBeforeExpiry and NackedBeforeExpiry invariants hold with equality, and the
// entry decays on the next pass at the same instant.
#[test]
fn a_zero_ttl_decays_at_the_instant_of_shedding() {
    let (mut queue, clock) = queue_with(Duration::ZERO, Duration::ZERO);
    one_of_each(&mut queue, &clock);
    let snapshot = queue.snapshot();
    assert_eq!(
        snapshot.superseded[0].shed_at,
        snapshot.superseded[0].expires_at
    );
    assert_eq!(
        snapshot.dead_letters[0].nacked_at,
        snapshot.dead_letters[0].expires_at
    );
    assert_eq!(counts(queue.decay()), decayed(1, 1));
}

// Decay touches only what has expired, oldest and newest alike, and leaves the
// queue's records alone.
#[test]
fn decay_removes_only_expired_entries() {
    let (mut queue, clock) = queue_with(SUPERSEDED_TTL, DEAD_LETTER_TTL);
    publish(&mut queue, "k1", 1);
    publish(&mut queue, "k1", 2);
    clock.advance(Duration::from_millis(100));
    publish(&mut queue, "k1", 0);
    clock.set(START + SUPERSEDED_TTL);
    assert_eq!(counts(queue.decay()), decayed(1, 0));
    let snapshot = queue.snapshot();
    assert_eq!(snapshot.superseded.len(), 1);
    assert_eq!(snapshot.superseded[0].rank, Rank::new(0));
    assert_eq!(snapshot.pockets[0].records.len(), 1);
}
