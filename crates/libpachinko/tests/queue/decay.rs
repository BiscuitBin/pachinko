//! Decay: `SupersededRecordDecays` and `DeadLetterDecays`, each at its own
//! `expires_at`, not before and once. A dead letter is created when its lease
//! ends, and its `expires_at` is measured from that end. The entries a lease's end
//! creates are stamped from the one `ended_at` the release or the lapse wrote, and
//! a lapse's entries can decay in the tick that made them.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use libpachinko::clock::Clock;
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::records::MemoryRecords;
use libpachinko::testing::ShedReason;
use libpachinko::{
    Lease, LeaseId, LeaseStatus, Payload, PocketKey, Publication, PullOutcome, Queue, Rank,
    RecordKey, Timestamp,
};

use crate::fixture::{
    DEAD_LETTER_TTL, Fixture, SUPERSEDED_TTL, T0, config, consumer, letter_rows, publisher,
};

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

// A lapse runs before the decay in one tick, with one `now`, so with both TTLs at
// zero the entries the lapse creates are due at once and decay in that same tick.
// obligation: rule-success.SupersedeUnsettledHeldVersion
// obligation: rule-success.DeadLetterNackedRecord
// obligation: rule-success.SupersededRecordDecays
// obligation: rule-success.DeadLetterDecays
#[test]
fn with_zero_ttls_a_lapses_entries_decay_in_the_tick_that_made_them() {
    let mut zero = config();
    zero.superseded_ttl = Duration::ZERO;
    zero.dead_letter_ttl = Duration::ZERO;
    let mut fixture = Fixture::with_config(zero);
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    fixture.publish("a", "a1", 2, "newer").unwrap();
    fixture.nack("c1", lease, "a2").unwrap();
    let snapshot = fixture.snapshot();
    assert!(snapshot.superseded.is_empty() && snapshot.dead_letters.is_empty());
    fixture.advance(u64::try_from(config().lease_ttl.as_millis()).unwrap());
    let report = fixture.tick();
    assert_eq!(report.lapsed.len(), 1, "the lease lapsed");
    assert_eq!(
        (report.decayed_superseded, report.decayed_dead_letters),
        (1, 1),
        "the shed held version and the dead letter decay in the lapse's tick"
    );
    let snapshot = fixture.snapshot();
    assert!(snapshot.superseded.is_empty() && snapshot.dead_letters.is_empty());
    assert_eq!(fixture.record("a", "a1").rank, Rank::new(2));
    assert_eq!(fixture.tick().decayed_superseded, 0, "a decay fires once");
}

/// A clock that moves one millisecond on every read, so two reads never agree.
#[derive(Clone, Debug)]
struct SteppingClock(Arc<AtomicU64>);

impl Clock for SteppingClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

type SteppingQueue = Queue<MemoryRecords, SteppingClock, CountingLeaseIds>;

/// A queue over a stepping clock whose pocket `a` is leased by `c1`, holding `a1`
/// unsettled with a newer version deferred behind it and `a2` nacked, so its
/// lease's end makes one superseded record and one dead letter.
fn stepping(lease_ttl: Duration) -> (SteppingQueue, LeaseId) {
    let mut settings = config();
    settings.lease_ttl = lease_ttl;
    let clock = SteppingClock(Arc::new(AtomicU64::new(1_000)));
    let mut queue = Queue::new(settings, clock, CountingLeaseIds::default()).unwrap();
    queue.publisher_joins(publisher()).unwrap();
    queue.consumer_joins(consumer("c1")).unwrap();
    let publish = |key: &str, rank| {
        Publication::new(
            PocketKey::new("a"),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::from(key.as_bytes()),
        )
    };
    queue.publish(&publisher(), publish("a1", 1)).unwrap();
    queue.publish(&publisher(), publish("a2", 1)).unwrap();
    let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&consumer("c1"), 10) else {
        panic!("expected a batch")
    };
    queue.publish(&publisher(), publish("a1", 2)).unwrap();
    queue
        .nack(&consumer("c1"), lease, &RecordKey::new("a2"))
        .unwrap();
    (queue, lease)
}

/// Every stamp the lease's end made is measured from the lease's `ended_at`.
fn stamped_from_the_end(queue: &SteppingQueue, ended: &Lease) {
    let (LeaseStatus::Released { ended_at } | LeaseStatus::Expired { ended_at }) = ended.status
    else {
        panic!("the lease has not ended: {ended:?}")
    };
    let snapshot = queue.snapshot();
    let shed = &snapshot.superseded;
    assert_eq!(shed.len(), 1);
    assert_eq!(
        (shed[0].shed_at, shed[0].expires_at),
        (ended_at, ended_at + SUPERSEDED_TTL),
        "the superseded record is stamped from ended_at"
    );
    let letters = &snapshot.dead_letters;
    assert_eq!(letters.len(), 1);
    assert_eq!(
        letters[0].expires_at,
        ended_at + DEAD_LETTER_TTL,
        "the dead letter expires from ended_at"
    );
    assert!(
        letters[0].nacked_at < ended_at,
        "it keeps its nack's own time"
    );
}

// With a clock that moves on every read, a release and a lapse each stamp every
// entry their lease's end makes from the one `now` they wrote as `ended_at`.
// obligation: rule-success.SupersedeUnsettledHeldVersion
// obligation: rule-success.DeadLetterNackedRecord
#[test]
fn a_lease_ends_entries_are_stamped_from_its_one_ended_at() {
    let (mut released, lease) = stepping(Duration::from_millis(1_000));
    let (_, ended) = released.release(&consumer("c1"), lease).unwrap();
    stamped_from_the_end(&released, &ended);
    let (mut lapsed, _) = stepping(Duration::ZERO);
    let report = lapsed.tick();
    assert_eq!(report.lapsed.len(), 1, "the lease lapsed");
    stamped_from_the_end(&lapsed, &report.lapsed[0]);
}
