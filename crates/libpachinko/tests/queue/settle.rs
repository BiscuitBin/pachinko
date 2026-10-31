//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`,
//! `NackSettlesRecord` and `NackDefersNewerVersion`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use libpachinko::clock::{Clock, Timestamp};
use libpachinko::error::ActionError;
use libpachinko::queue::{LeaseId, Publication, PullOutcome, Queue, SettleOutcome};
use libpachinko::records::{MemoryRecords, RecordStatus, Version};

use crate::fixture::{Batch, DECAY_TTL, Fixture, LEASE_TTL, at, config};

/// Which of the two actions a shared test drives.
#[derive(Clone, Copy, Debug)]
enum Action {
    Ack,
    Nack,
}

fn act(
    fixture: &mut Fixture,
    action: Action,
    consumer_id: &str,
    lease: LeaseId,
    record_key: &str,
) -> Result<SettleOutcome, ActionError> {
    match action {
        Action::Ack => fixture.queue.ack(consumer_id, lease, record_key),
        Action::Nack => fixture.queue.nack(consumer_id, lease, record_key),
    }
}

/// `a` and `b` published into `pk` and pulled by `c1`; `c` published after.
fn held_pocket() -> (Fixture, Batch) {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "b", 1, "b1").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.publish("pk", "c", 1, "c1").unwrap();
    (fixture, batch)
}

// obligation: rule-success.AckSettlesRecord
#[test]
fn an_ack_of_the_latest_version_removes_the_record_and_extends_the_lease() {
    let (mut fixture, batch) = held_pocket();
    fixture.advance(Duration::from_millis(300));
    assert_eq!(
        fixture.queue.ack("c1", batch.lease, "a"),
        Ok(SettleOutcome::Settled)
    );
    assert_eq!(fixture.record("pk", "a"), None);
    assert_eq!(
        fixture.snapshot().leases[0].expires_at,
        at(fixture.now(), LEASE_TTL)
    );
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
}

// obligation: rule-success.AckDefersNewerVersion
// obligation: transition-edge.Record.held.deferred
// obligation: projection.Pocket.deferred_records
#[test]
fn an_ack_with_a_newer_version_waiting_defers_the_record_to_the_back() {
    let (mut fixture, batch) = held_pocket();
    fixture.publish("pk", "a", 2, "a2").unwrap();
    let next = fixture.snapshot().next_position;
    fixture.advance(Duration::from_millis(300));
    assert_eq!(
        fixture.queue.ack("c1", batch.lease, "a"),
        Ok(SettleOutcome::Deferred)
    );
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(record.status, RecordStatus::Deferred);
    assert_eq!(record.position, next);
    assert_eq!(record.held, None);
    assert_eq!(record.latest, Version::new(2, b"a2".to_vec()));
    let deferred: Vec<String> = fixture
        .pocket("pk")
        .records
        .into_iter()
        .filter(|record| record.status == RecordStatus::Deferred)
        .map(|record| record.key)
        .collect();
    assert_eq!(deferred, ["a"]);
    assert_eq!(fixture.snapshot().next_position, next + 1);
    assert_eq!(
        fixture.snapshot().leases[0].expires_at,
        at(fixture.now(), LEASE_TTL)
    );
}

// obligation: rule-success.NackSettlesRecord
// obligation: rule-entity-creation.NackSettlesRecord.1
// obligation: entity-fields.DeadLetter
// obligation: invariant.DeadLetter.NackedBeforeExpiry
#[test]
fn a_nack_of_the_latest_version_dead_letters_it_and_removes_the_record() {
    let (mut fixture, batch) = held_pocket();
    fixture.advance(Duration::from_millis(300));
    assert_eq!(
        fixture.queue.nack("c1", batch.lease, "a"),
        Ok(SettleOutcome::Settled)
    );
    assert_eq!(fixture.record("pk", "a"), None);
    let letter = &fixture.snapshot().dead_letters[0];
    assert_eq!(
        (letter.pocket_key.as_str(), letter.key.as_str(), letter.rank),
        ("pk", "a", 1)
    );
    assert_eq!(letter.payload, b"a1");
    assert_eq!(letter.nacked_by, "c1");
    assert_eq!(letter.nacked_at, fixture.now());
    assert_eq!(letter.expires_at, at(fixture.now(), DECAY_TTL));
    assert!(letter.nacked_at <= letter.expires_at);
    assert_eq!(
        fixture.snapshot().leases[0].expires_at,
        at(fixture.now(), LEASE_TTL)
    );
}

/// A clock one millisecond later on every read, so a rule that reads it twice
/// stamps two different instants.
#[derive(Clone, Debug, Default)]
struct SteppingClock(Arc<AtomicU64>);

impl Clock for SteppingClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

// A nack reads the clock once: the lease's new expiry, the dead letter's
// `nacked_at` and its `expires_at` all come from the same `now`.
#[test]
fn a_nack_stamps_the_lease_and_the_dead_letter_from_one_clock_read() {
    let mut queue: Queue<MemoryRecords, _> =
        Queue::new(config(), SteppingClock::default()).unwrap();
    queue.publisher_joins("p").unwrap();
    queue.consumer_joins("c1").unwrap();
    queue
        .publish("p", Publication::new("pk", "a", 1, b"a1".to_vec()))
        .unwrap();
    let PullOutcome::Batch(batch) = queue.pull("c1", 10).unwrap() else {
        panic!("expected a batch");
    };
    assert_eq!(
        queue.nack("c1", batch.lease, "a"),
        Ok(SettleOutcome::Settled)
    );
    let snapshot = queue.snapshot();
    let letter = &snapshot.dead_letters[0];
    assert_eq!(
        snapshot.leases[0].expires_at,
        at(letter.nacked_at, LEASE_TTL)
    );
    assert_eq!(letter.expires_at, at(letter.nacked_at, DECAY_TTL));
}

// obligation: rule-success.NackDefersNewerVersion
// obligation: rule-entity-creation.NackDefersNewerVersion.1
#[test]
fn a_nack_with_a_newer_version_waiting_dead_letters_the_held_one_and_defers() {
    let (mut fixture, batch) = held_pocket();
    fixture.publish("pk", "a", 2, "a2").unwrap();
    let next = fixture.snapshot().next_position;
    assert_eq!(
        fixture.queue.nack("c1", batch.lease, "a"),
        Ok(SettleOutcome::Deferred)
    );
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(
        (record.status, record.position, record.held),
        (RecordStatus::Deferred, next, None)
    );
    let letter = &fixture.snapshot().dead_letters[0];
    assert_eq!((letter.rank, letter.payload.as_slice()), (1, &b"a1"[..]));
    assert_eq!(letter.nacked_by, "c1");
}

// obligation: rule-failure.AckSettlesRecord.1
// obligation: rule-failure.AckDefersNewerVersion.1
// obligation: rule-failure.NackSettlesRecord.1
// obligation: rule-failure.NackDefersNewerVersion.1
#[test]
fn an_action_naming_an_ended_lease_is_rejected_even_on_a_successor_lease() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, first) = held_pocket();
        fixture.queue.release("c1", first.lease).unwrap();
        fixture.publish("pk", "a", 2, "a2").unwrap();
        let second = fixture.pull("c1", 10);
        assert_eq!(second.pocket_key, "pk");
        let before = fixture.snapshot();
        assert_eq!(
            act(&mut fixture, action, "c1", first.lease, "a"),
            Err(ActionError::LeaseNotActive(first.lease))
        );
        assert_eq!(fixture.snapshot(), before);
        assert!(act(&mut fixture, action, "c1", second.lease, "a").is_ok());
    }
}

// obligation: rule-failure.AckSettlesRecord.2
// obligation: rule-failure.AckDefersNewerVersion.2
// obligation: rule-failure.NackSettlesRecord.2
// obligation: rule-failure.NackDefersNewerVersion.2
#[test]
fn an_action_on_another_consumers_lease_is_rejected() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        let before = fixture.snapshot();
        assert_eq!(
            act(&mut fixture, action, "c2", batch.lease, "a"),
            Err(ActionError::LeaseNotYours(batch.lease))
        );
        assert_eq!(fixture.snapshot(), before);
    }
}

// obligation: rule-failure.AckSettlesRecord.3
// obligation: rule-failure.AckDefersNewerVersion.3
// obligation: rule-failure.NackSettlesRecord.3
// obligation: rule-failure.NackDefersNewerVersion.3
#[test]
fn an_action_on_a_key_the_pocket_lacks_is_rejected() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        fixture.queue.ack("c1", batch.lease, "b").unwrap();
        let before = fixture.snapshot();
        for key in ["b", "never"] {
            assert_eq!(
                act(&mut fixture, action, "c1", batch.lease, key),
                Err(ActionError::RecordNotHeld(key.to_owned()))
            );
        }
        assert_eq!(fixture.snapshot(), before);
    }
}

// obligation: rule-failure.AckSettlesRecord.4
// obligation: rule-failure.AckDefersNewerVersion.4
// obligation: rule-failure.NackSettlesRecord.4
// obligation: rule-failure.NackDefersNewerVersion.4
#[test]
fn an_action_on_a_queued_or_deferred_record_is_rejected() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        fixture.publish("pk", "a", 2, "a2").unwrap();
        fixture.queue.ack("c1", batch.lease, "a").unwrap();
        let before = fixture.snapshot();
        for key in ["c", "a"] {
            assert_eq!(
                act(&mut fixture, action, "c1", batch.lease, key),
                Err(ActionError::RecordNotHeld(key.to_owned()))
            );
        }
        assert_eq!(fixture.snapshot(), before);
    }
}

// obligation: rule-failure.AckSettlesRecord.5
// obligation: rule-failure.NackSettlesRecord.5
#[test]
fn a_held_version_older_than_the_latest_is_not_settled() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        fixture.publish("pk", "a", 2, "a2").unwrap();
        assert_eq!(
            act(&mut fixture, action, "c1", batch.lease, "a"),
            Ok(SettleOutcome::Deferred)
        );
        assert!(fixture.record("pk", "a").is_some());
    }
}

// obligation: rule-failure.AckDefersNewerVersion.5
// obligation: rule-failure.NackDefersNewerVersion.5
#[test]
fn a_held_version_that_is_the_latest_is_not_deferred() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        let next = fixture.snapshot().next_position;
        assert_eq!(
            act(&mut fixture, action, "c1", batch.lease, "a"),
            Ok(SettleOutcome::Settled)
        );
        assert_eq!(fixture.snapshot().next_position, next);
    }
}

#[test]
fn an_unknown_consumer_cannot_act() {
    for action in [Action::Ack, Action::Nack] {
        let (mut fixture, batch) = held_pocket();
        assert_eq!(
            act(&mut fixture, action, "stranger", batch.lease, "a"),
            Err(ActionError::UnknownConsumer("stranger".to_owned()))
        );
    }
}

// A key acked and published again under the same lease is a new record, queued
// and pullable under that lease (the module's ninth open question; the rules
// allow it today).
#[test]
fn a_key_published_again_after_its_ack_is_a_new_queued_record() {
    let (mut fixture, batch) = held_pocket();
    fixture.queue.ack("c1", batch.lease, "a").unwrap();
    fixture.publish("pk", "a", 1, "again").unwrap();
    let record = fixture.record("pk", "a").unwrap();
    assert_eq!(record.status, RecordStatus::Queued);
    let next = fixture.pull("c1", 10);
    assert_eq!(crate::fixture::keys(&next), ["c", "a"]);
}
