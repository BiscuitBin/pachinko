//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord` and
//! `NackDefersNewerVersion`, and the `NothingLostWhileHeld` and
//! `OneVisitPerKeyPerLease` guarantees.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Batch, Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, Payload, PocketKey,
    PocketView, Position, Publication, PublisherId, PullOutcome, Queue, Rank, RecordKey,
    RecordStatus, SettleOutcome, Snapshot, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const LEASE_TTL: Duration = Duration::from_millis(1_000);
const DEAD_LETTER_TTL: Duration = Duration::from_millis(7_000);

/// Which action a table-driven case takes.
#[derive(Clone, Copy, Debug)]
enum Action {
    Ack,
    Nack,
}

const BOTH: [Action; 2] = [Action::Ack, Action::Nack];

fn queue() -> (TestQueue, FakeClock) {
    let config = Config::new(
        LEASE_TTL,
        2,
        10,
        Duration::from_millis(5_000),
        DEAD_LETTER_TTL,
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    queue.consumer_joins(ConsumerId::new("c1")).unwrap();
    queue.consumer_joins(ConsumerId::new("c2")).unwrap();
    (queue, clock)
}

fn publish(queue: &mut TestQueue, key: &str, rank: i64) {
    queue
        .publish(Publication::new(
            PublisherId::new("p"),
            PocketKey::new("a"),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::from(format!("{key}@{rank}").as_bytes()),
        ))
        .unwrap();
}

fn pull(queue: &mut TestQueue, consumer: &str, limit: i64) -> Batch {
    match queue.pull(&ConsumerId::new(consumer), limit).unwrap() {
        PullOutcome::Batch(batch) => batch,
        other => panic!("expected a batch, got {other:?}"),
    }
}

fn act(
    queue: &mut TestQueue,
    action: Action,
    consumer: &str,
    lease: LeaseId,
    key: &str,
) -> Result<SettleOutcome, Error> {
    let (consumer, key) = (ConsumerId::new(consumer), RecordKey::new(key));
    match action {
        Action::Ack => queue.ack(&consumer, lease, &key),
        Action::Nack => queue.nack(&consumer, lease, &key),
    }
}

fn pocket(snapshot: &Snapshot) -> &PocketView {
    &snapshot.pockets[0]
}

/// Publishes k1 and k2 at rank 1 into `a` and pulls both under c1's lease.
fn holding(queue: &mut TestQueue) -> Batch {
    publish(queue, "k1", 1);
    publish(queue, "k2", 1);
    pull(queue, "c1", 10)
}

// rule-success.AckSettlesRecord and rule-success.NackSettlesRecord: the record is gone
// and the lease extended; a nack also makes the held version a dead letter.
// rule-entity-creation.NackSettlesRecord.1, invariant.DeadLetter.NackedBeforeExpiry,
// entity-fields.DeadLetter
#[test]
fn settling_the_latest_version_removes_the_record_and_extends_the_lease() {
    for action in BOTH {
        let (mut queue, clock) = queue();
        let batch = holding(&mut queue);
        clock.advance(Duration::from_millis(600));
        assert_eq!(
            act(&mut queue, action, "c1", batch.lease_id, "k1"),
            Ok(SettleOutcome::Settled)
        );
        let snapshot = queue.snapshot();
        let keys: Vec<_> = pocket(&snapshot)
            .records
            .iter()
            .map(|r| r.key.as_str())
            .collect();
        assert_eq!(keys, ["k2"], "{action:?}");
        assert_eq!(
            snapshot.leases[0].expires_at,
            Timestamp::from_millis(1_600) + LEASE_TTL
        );
        match action {
            Action::Ack => assert_eq!(snapshot.dead_letters, []),
            Action::Nack => {
                let letter = &snapshot.dead_letters[0];
                assert_eq!(letter.pocket_key, PocketKey::new("a"));
                assert_eq!(letter.record_key, RecordKey::new("k1"));
                assert_eq!(letter.rank, Rank::new(1));
                assert_eq!(letter.payload.as_bytes(), b"k1@1");
                assert_eq!(letter.nacked_by, ConsumerId::new("c1"));
                assert_eq!(letter.nacked_at, Timestamp::from_millis(1_600));
                assert_eq!(
                    letter.expires_at,
                    Timestamp::from_millis(1_600) + DEAD_LETTER_TTL
                );
                assert!(letter.nacked_at <= letter.expires_at);
            }
        }
    }
}

// rule-success.AckDefersNewerVersion and rule-success.NackDefersNewerVersion,
// transition-edge.Record.held.deferred, rule-entity-creation.NackDefersNewerVersion.1,
// when-presence.Record.held_rank and held_payload (cleared on deferral)
#[test]
fn settling_a_superseded_hold_defers_the_newer_version_to_the_back() {
    for action in BOTH {
        let (mut queue, clock) = queue();
        let batch = holding(&mut queue);
        publish(&mut queue, "k1", 4);
        clock.advance(Duration::from_millis(300));
        let next = queue.snapshot().next_position;
        assert_eq!(
            act(&mut queue, action, "c1", batch.lease_id, "k1"),
            Ok(SettleOutcome::Deferred)
        );
        let snapshot = queue.snapshot();
        let k1 = &pocket(&snapshot).records[0];
        assert_eq!(k1.status, RecordStatus::Deferred, "{action:?}");
        assert_eq!(k1.position, next);
        assert_eq!(
            (k1.rank, k1.payload.as_bytes()),
            (Rank::new(4), b"k1@4".as_slice())
        );
        assert_eq!(snapshot.next_position, Position::new(next.get() + 1));
        assert_eq!(
            snapshot.leases[0].expires_at,
            Timestamp::from_millis(1_300) + LEASE_TTL
        );
        match action {
            Action::Ack => assert_eq!(snapshot.dead_letters, []),
            Action::Nack => {
                let letter = &snapshot.dead_letters[0];
                assert_eq!(
                    (letter.rank, letter.payload.as_bytes()),
                    (Rank::new(1), b"k1@1".as_slice())
                );
                assert_eq!(letter.nacked_by, ConsumerId::new("c1"));
                assert_eq!(letter.nacked_at, Timestamp::from_millis(1_300));
                assert_eq!(
                    letter.expires_at,
                    Timestamp::from_millis(1_300) + DEAD_LETTER_TTL
                );
            }
        }
    }
}

// rule-failure.AckSettlesRecord.1, rule-failure.AckDefersNewerVersion.1,
// rule-failure.NackSettlesRecord.1, rule-failure.NackDefersNewerVersion.1: the lease
// is not active.
#[test]
fn an_action_under_an_ended_or_unknown_lease_is_refused() {
    for action in BOTH {
        let (mut queue, _) = queue();
        let batch = holding(&mut queue);
        queue
            .release(&ConsumerId::new("c1"), batch.lease_id)
            .unwrap();
        let before = queue.snapshot();
        assert_eq!(
            act(&mut queue, action, "c1", batch.lease_id, "k1"),
            Err(Error::LeaseNotActive(batch.lease_id))
        );
        let unknown = LeaseId::from_u128(999);
        assert_eq!(
            act(&mut queue, action, "c1", unknown, "k1"),
            Err(Error::LeaseNotActive(unknown))
        );
        assert_eq!(queue.snapshot(), before, "{action:?}");
    }
}

// rule-failure.AckSettlesRecord.2, rule-failure.AckDefersNewerVersion.2,
// rule-failure.NackSettlesRecord.2, rule-failure.NackDefersNewerVersion.2: the lease
// is not the consumer's. surface-actor.Consuming.
#[test]
fn an_action_under_another_consumers_lease_is_refused() {
    for action in BOTH {
        let (mut queue, _) = queue();
        let batch = holding(&mut queue);
        let before = queue.snapshot();
        assert_eq!(
            act(&mut queue, action, "c2", batch.lease_id, "k1"),
            Err(Error::LeaseNotYours(batch.lease_id))
        );
        assert_eq!(
            act(&mut queue, action, "stranger", batch.lease_id, "nothing"),
            Err(Error::LeaseNotYours(batch.lease_id))
        );
        assert_eq!(queue.snapshot(), before, "{action:?}");
    }
}

// rule-failure.AckSettlesRecord.3, rule-failure.AckDefersNewerVersion.3,
// rule-failure.NackSettlesRecord.3, rule-failure.NackDefersNewerVersion.3 (no such
// record) and rule-failure.AckSettlesRecord.4, rule-failure.AckDefersNewerVersion.4,
// rule-failure.NackSettlesRecord.4, rule-failure.NackDefersNewerVersion.4 (the record
// is not held: queued, or deferred).
#[test]
fn an_action_on_a_record_the_lease_does_not_hold_is_refused() {
    for action in BOTH {
        let (mut queue, _) = queue();
        publish(&mut queue, "k1", 1);
        publish(&mut queue, "k2", 1);
        let batch = pull(&mut queue, "c1", 1);
        publish(&mut queue, "k1", 2);
        act(&mut queue, Action::Ack, "c1", batch.lease_id, "k1").unwrap();
        let before = queue.snapshot();
        for key in ["missing", "k2", "k1"] {
            assert_eq!(
                act(&mut queue, action, "c1", batch.lease_id, key),
                Err(Error::RecordNotHeld(RecordKey::new(key))),
                "{action:?} {key}"
            );
        }
        assert_eq!(queue.snapshot(), before);
    }
}

// rule-failure.AckSettlesRecord.5, rule-failure.NackSettlesRecord.5 (the held version
// is older: deferred instead), rule-failure.AckDefersNewerVersion.5,
// rule-failure.NackDefersNewerVersion.5 (the held version is the latest: settled
// instead), side by side.
#[test]
fn the_held_rank_against_the_latest_rank_picks_settle_or_defer() {
    for action in BOTH {
        let (mut queue, _) = queue();
        let batch = holding(&mut queue);
        publish(&mut queue, "k2", 3);
        assert_eq!(
            act(&mut queue, action, "c1", batch.lease_id, "k1"),
            Ok(SettleOutcome::Settled)
        );
        assert_eq!(
            act(&mut queue, action, "c1", batch.lease_id, "k2"),
            Ok(SettleOutcome::Deferred)
        );
    }
}

// The order of the checks follows each rule's requires: active, then the consumer's,
// then the record.
#[test]
fn the_lease_is_checked_before_the_consumer_and_the_consumer_before_the_record() {
    let (mut queue, _) = queue();
    let batch = holding(&mut queue);
    assert_eq!(
        act(&mut queue, Action::Ack, "c2", batch.lease_id, "missing"),
        Err(Error::LeaseNotYours(batch.lease_id))
    );
    queue
        .release(&ConsumerId::new("c1"), batch.lease_id)
        .unwrap();
    assert_eq!(
        act(&mut queue, Action::Ack, "c2", batch.lease_id, "missing"),
        Err(Error::LeaseNotActive(batch.lease_id))
    );
}

// The NothingLostWhileHeld and OneVisitPerKeyPerLease guarantees: the newer version is
// kept, not handed again under the lease, and pulled once the lease has ended.
#[test]
fn a_deferred_version_waits_for_the_lease_to_end_and_is_then_pulled() {
    let (mut queue, _) = queue();
    let batch = holding(&mut queue);
    publish(&mut queue, "k1", 2);
    queue
        .ack(
            &ConsumerId::new("c1"),
            batch.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    assert_eq!(pull(&mut queue, "c1", 10).records, []);
    queue
        .release(&ConsumerId::new("c1"), batch.lease_id)
        .unwrap();
    let next = pull(&mut queue, "c2", 10);
    let handed: Vec<_> = next
        .records
        .iter()
        .map(|r| (r.record_key.as_str(), r.rank.get()))
        .collect();
    assert_eq!(handed, [("k2", 1), ("k1", 2)]);
}

// Open question 9 as the rules stand: a key settled and published afresh under the same
// lease is a new record, queued, and pullable under that lease.
#[test]
fn a_key_settled_and_published_again_is_a_new_record() {
    let (mut queue, _) = queue();
    let batch = holding(&mut queue);
    queue
        .ack(
            &ConsumerId::new("c1"),
            batch.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    publish(&mut queue, "k1", 1);
    let again = pull(&mut queue, "c1", 10);
    assert_eq!(again.records[0].record_key, RecordKey::new("k1"));
}
