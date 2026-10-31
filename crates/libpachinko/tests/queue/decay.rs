//! Decay: `SupersededRecordDecays` and `DeadLetterDecays`, driven by `tick` under
//! the fake clock.

use std::time::Duration;

use crate::fixture::{DECAY_TTL, Fixture};

const ONE_MS: Duration = Duration::from_millis(1);

// obligation: rule-success.SupersededRecordDecays
// obligation: temporal.SupersededRecordDecays
#[test]
fn a_superseded_record_decays_at_its_expiry_not_before_and_only_once() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "old").unwrap();
    fixture.publish("pk", "a", 2, "new").unwrap();
    fixture.publish("pk", "a", 0, "stale").unwrap();
    fixture.advance(DECAY_TTL.checked_sub(ONE_MS).unwrap());
    assert_eq!(fixture.queue.tick().decayed_superseded, 0);
    assert_eq!(fixture.snapshot().superseded.len(), 2);
    fixture.advance(ONE_MS);
    assert_eq!(fixture.queue.tick().decayed_superseded, 2);
    assert_eq!(fixture.snapshot().superseded.len(), 0);
    assert_eq!(fixture.queue.tick().decayed_superseded, 0);
    // The record itself never decays.
    assert!(fixture.record("pk", "a").is_some());
}

// obligation: rule-success.DeadLetterDecays
// obligation: temporal.DeadLetterDecays
#[test]
fn a_dead_letter_decays_at_its_expiry_not_before_and_only_once() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    fixture.queue.nack("c1", batch.lease, "a").unwrap();
    fixture.advance(DECAY_TTL.checked_sub(ONE_MS).unwrap());
    assert_eq!(fixture.queue.tick().decayed_dead_letters, 0);
    assert_eq!(fixture.snapshot().dead_letters.len(), 1);
    fixture.advance(ONE_MS);
    assert_eq!(fixture.queue.tick().decayed_dead_letters, 1);
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
    assert_eq!(fixture.queue.tick().decayed_dead_letters, 0);
}

// Each entry decays on its own expiry: a later one outlives an earlier one.
#[test]
fn entries_decay_in_expiry_order() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a1").unwrap();
    fixture.publish("pk", "a", 2, "a2").unwrap();
    fixture.advance(DECAY_TTL / 2);
    fixture.publish("pk", "a", 3, "a3").unwrap();
    fixture.advance(DECAY_TTL / 2);
    assert_eq!(fixture.queue.tick().decayed_superseded, 1);
    let left = fixture.snapshot().superseded;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].rank, 2);
}
