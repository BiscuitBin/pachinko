//! Decay: `SupersededRecordDecays` and `DeadLetterDecays`, each at its own
//! `expires_at`, not before and once. A dead letter is created when its lease
//! ends, and its `expires_at` is measured from that end.

use std::time::Duration;

use libpachinko::testing::ShedReason;

use crate::fixture::{DEAD_LETTER_TTL, Fixture, SUPERSEDED_TTL, T0, config, letter_rows};

/// Milliseconds of a TTL.
fn millis(ttl: Duration) -> u64 {
    u64::try_from(ttl.as_millis()).unwrap()
}

// scenario: S007
// obligation: rule-success.SupersededRecordDecays
// obligation: rule-success.DeadLetterDecays
#[test]
fn zero_superseded_and_dead_letter_ttls_decay_at_once() {
    let mut zero = config();
    zero.superseded_ttl = Duration::ZERO;
    zero.dead_letter_ttl = Duration::ZERO;
    let mut fixture = Fixture::with_config(zero);
    fixture.publish("a", "a1", 1, "old").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    fixture.publish("a", "a1", 2, "new").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    fixture.nack("c1", lease, "a2").unwrap();
    assert_eq!(
        fixture.snapshot().dead_letters.len(),
        0,
        "not before the end"
    );
    fixture.release("c1", lease).unwrap();
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.superseded[0].expires_at, T0);
    assert_eq!(snapshot.dead_letters[0].expires_at, T0);
    let report = fixture.tick();
    assert_eq!(
        (report.decayed_superseded, report.decayed_dead_letters),
        (1, 1)
    );
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.superseded.len(), 0);
    assert_eq!(snapshot.dead_letters.len(), 0);
}

// scenario: S073
// obligation: rule-success.SupersededRecordDecays
// obligation: temporal.SupersededRecordDecays
#[test]
fn superseded_entries_decay_at_shed_at_plus_superseded_ttl() {
    let mut fixture = Fixture::new();
    fixture.publish("a", "a1", 2, "kept").unwrap();
    fixture.advance(3);
    fixture.publish("a", "a1", 1, "stale").unwrap();
    fixture.advance(1);
    fixture.publish("a", "a1", 3, "newer").unwrap();
    let shed = fixture.snapshot().superseded;
    assert_eq!(shed[0].reason, ShedReason::Stale);
    assert_eq!(shed[0].shed_at, T0.saturating_add_millis(3));
    let ttl = millis(SUPERSEDED_TTL);
    fixture.at(3 + ttl - 1);
    assert_eq!(fixture.tick().decayed_superseded, 0);
    assert_eq!(fixture.snapshot().superseded.len(), 2);
    fixture.at(3 + ttl);
    assert_eq!(fixture.tick().decayed_superseded, 1);
    let left = fixture.snapshot().superseded;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].reason, ShedReason::Superseded);
    fixture.at(4 + ttl);
    assert_eq!(fixture.tick().decayed_superseded, 1);
    assert_eq!(fixture.tick().decayed_superseded, 0, "a decay fires once");
    assert_eq!(fixture.snapshot().superseded.len(), 0);
}

// scenario: S074
// obligation: rule-success.DeadLetterDecays
// obligation: temporal.DeadLetterDecays
// obligation: rule-success.DeadLetterNackedRecord
// obligation: invariant.DeadLetter.NackedBeforeExpiry
#[test]
fn dead_letters_decay_at_the_lease_end_plus_dead_letter_ttl() {
    let mut fixture = Fixture::new();
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    fixture.at(2);
    fixture.nack("c1", lease, "a1").unwrap();
    fixture.at(3);
    fixture.nack("c1", lease, "a2").unwrap();
    assert_eq!(
        fixture.snapshot().dead_letters.len(),
        0,
        "not before the end"
    );
    fixture.at(10);
    fixture.release("c1", lease).unwrap();
    let ttl = millis(DEAD_LETTER_TTL);
    let expires_at = T0.saturating_add_millis(10 + ttl);
    let (two, three) = (T0.saturating_add_millis(2), T0.saturating_add_millis(3));
    assert_eq!(
        letter_rows(&fixture.snapshot()),
        [
            ("a", "a1", 1, "a1", "c1", two, expires_at),
            ("a", "a2", 1, "a2", "c1", three, expires_at)
        ],
        "each keeps its nack's time and expires from the lease's end"
    );
    fixture.at(3 + ttl);
    assert_eq!(
        fixture.tick().decayed_dead_letters,
        0,
        "not at nacked_at plus the TTL"
    );
    fixture.at(10 + ttl - 1);
    assert_eq!(fixture.tick().decayed_dead_letters, 0);
    fixture.at(10 + ttl);
    let report = fixture.tick();
    assert_eq!(
        (report.decayed_dead_letters, report.decayed_superseded),
        (2, 0)
    );
    assert_eq!(fixture.tick().decayed_dead_letters, 0, "a decay fires once");
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
}
