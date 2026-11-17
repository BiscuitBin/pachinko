//! Ack and nack: `AckMarksRecord`, `AckRepeats`, `NackMarksRecord` and
//! `NackRepeats`, their refusals and the refusal of a changed mark, a newer
//! version deferred behind a held one (`DeferBehindHeldVersion`), a key published
//! again under the lease that acked or nacked it, and the lease read's marks.

use std::time::Duration;

use libpachinko::records::{Position, RecordStatus};
use libpachinko::testing::ShedReason;
use libpachinko::{
    LeaseActionError, LeaseId, Mark, MarkedRecord, Payload, PublishError, PublishOutcome, Rank,
    RecordKey, SettleOutcome, Timestamp,
};

use crate::fixture::{
    DEAD_LETTER_TTL, Fixture, LEASE_TTL, SUPERSEDED_TTL, T0, config, consumer, held_as, invariants,
    keys, letter_rows, shed_rows,
};

/// `T0` plus `millis`.
const fn at(millis: u64) -> Timestamp {
    T0.saturating_add_millis(millis)
}

/// A fixture whose pocket `pk` holds `k` at rank 1, payload `v1`, pulled by `c1`.
/// `pk` stands at 0 and `k` at 1, so the next position is 2.
fn held() -> (Fixture, LeaseId) {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    let batch = fixture.pull("c1", 10);
    (fixture, batch.lease)
}

/// `MarkedRecord` for `key` at rank 1 with its key as payload, as `k1` and `k2`
/// are published below.
fn marked(key: &str, payload: &str, mark: Mark) -> MarkedRecord {
    MarkedRecord::new(
        RecordKey::new(key),
        Rank::new(1),
        Payload::from(payload.as_bytes()),
        mark,
    )
}

// scenario: S040
// obligation: rule-success.AckMarksRecord
// obligation: rule-failure.AckRepeats.5
// obligation: guarantee.HeldSetStableUntilLeaseEnds
#[test]
fn an_ack_marks_a_held_record_and_removes_nothing() {
    let (mut fixture, lease) = held();
    fixture.publish("pk", "other", 1, "o").unwrap();
    assert_eq!(fixture.ack("c1", lease, "k"), Ok(SettleOutcome::Acked));
    let record = fixture.record("pk", "k");
    assert_eq!(record.status, held_as(1, "v1", Mark::Acked, None, None));
    assert_eq!(record.position, Position::new(1));
    assert_eq!(keys_of(&fixture), ["k", "other"], "nothing is removed");
    assert_eq!(
        fixture.lease_view("c1").unwrap().records,
        [marked("k", "v1", Mark::Acked)]
    );
    assert_eq!(fixture.snapshot().superseded.len(), 0);
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
    invariants::all(&fixture.snapshot());
}

/// The record keys of pocket `pk`, by key.
fn keys_of(fixture: &Fixture) -> Vec<String> {
    (fixture.pocket("pk").records.iter())
        .map(|record| record.key.as_str().to_owned())
        .collect()
}

// scenario: S041
// obligation: rule-success.AckMarksRecord
#[test]
fn an_ack_extends_the_lease() {
    let mut short = config();
    short.lease_ttl = Duration::from_millis(10);
    let mut fixture = Fixture::with_config(short);
    fixture.publish("pk", "k", 1, "v").unwrap();
    let batch = fixture.pull("c1", 10);
    assert_eq!(batch.expires_at, T0.saturating_add_millis(10));
    fixture.advance(4);
    fixture.ack("c1", batch.lease, "k").unwrap();
    let snapshot = fixture.snapshot();
    let [lease] = snapshot.leases.as_slice() else {
        panic!("one lease")
    };
    assert_eq!(lease.expires_at, T0.saturating_add_millis(14));
    assert_eq!(
        fixture.lease_view("c1").unwrap().expires_at,
        T0.saturating_add_millis(14)
    );
}

// scenario: S042
// obligation: rule-failure.AckMarksRecord.1
// obligation: rule-failure.AckMarksRecord.2
// obligation: rule-failure.AckMarksRecord.3
// obligation: rule-failure.AckMarksRecord.4
// obligation: rule-failure.AckRepeats.1
// obligation: rule-failure.AckRepeats.2
// obligation: rule-failure.AckRepeats.3
// obligation: rule-failure.AckRepeats.4
#[test]
fn rejected_acks_change_nothing_not_even_the_leases_expiry() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v").unwrap();
    fixture.publish("pk", "queued", 1, "q").unwrap();
    fixture.publish("other", "x", 1, "x").unwrap();
    let lease = fixture.pull("c1", 1).lease;
    let theirs = fixture.pull("c2", 1).lease;
    fixture.advance(500);
    let unknown = LeaseActionError::UnknownConsumer(consumer("nobody"));
    fixture.refused(&unknown, |f| f.ack("nobody", lease, "k"));
    let never = LeaseId::from_u128(99);
    let not_active = LeaseActionError::LeaseNotActive(never);
    fixture.refused(&not_active, |f| f.ack("c1", never, "k"));
    let not_yours = LeaseActionError::LeaseNotYours(theirs);
    fixture.refused(&not_yours, |f| f.ack("c1", theirs, "x"));
    for key in ["absent", "queued"] {
        let not_held = LeaseActionError::RecordNotHeld(RecordKey::new(key));
        fixture.refused(&not_held, |f| f.ack("c1", lease, key));
    }
    let not_held = LeaseActionError::RecordNotHeld(RecordKey::new("x"));
    fixture.refused(&not_held, |f| f.ack("c1", lease, "x"));
    assert_eq!(
        fixture.lease_view("c1").unwrap().expires_at,
        T0 + LEASE_TTL,
        "no refusal moved the expiry"
    );
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.ack("c1", lease, "k"),
        Ok(SettleOutcome::Acked),
        "a repeat is not a refusal"
    );
}

// scenario: S043
// obligation: rule-success.DeferBehindHeldVersion
// obligation: rule-failure.Supersede.4
// obligation: when-presence.Record.deferred_position
// obligation: entity-optional.Record.deferred_position
// obligation: guarantee.NothingLostWhileHeld
#[test]
fn a_supersede_while_the_key_is_held_is_deferred_behind_the_held_version() {
    let (mut fixture, lease) = held();
    let next = fixture.snapshot().next_position;
    assert_eq!(
        fixture.publish("pk", "k", 3, "v3"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        record.status,
        held_as(1, "v1", Mark::Unsettled, None, Some(next.get())),
        "the newer version is deferred at the counter's value"
    );
    assert_eq!(
        (record.rank, record.payload, record.position),
        (
            Rank::new(3),
            Payload::from(b"v3".as_slice()),
            Position::new(1)
        )
    );
    assert_eq!(fixture.snapshot().next_position, next.next());
    assert_eq!(fixture.snapshot().superseded.len(), 0, "nothing is shed");
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.record("pk", "k").status,
        held_as(1, "v1", Mark::Acked, None, Some(next.get()))
    );
    assert_eq!(
        fixture.lease_view("c1").unwrap().records,
        [marked("k", "v1", Mark::Acked)],
        "the lease read shows the held version"
    );
    invariants::all(&fixture.snapshot());
}

// scenario: S044
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
// obligation: guarantee.OneVisitPerKeyPerLease
// obligation: guarantee.OldestSlotNewestPayload
#[test]
fn a_version_deferred_behind_an_acked_one_is_queued_at_its_arrival_position() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    fixture.publish("pk", "early", 1, "e").unwrap();
    let lease = fixture.pull("c1", 1).lease;
    fixture.publish("pk", "k", 3, "v3").unwrap();
    let arrival = Position::new(3);
    fixture.publish("pk", "between", 1, "b").unwrap();
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.record("pk", "k").position,
        Position::new(1),
        "a held record keeps its position until the lease ends"
    );
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert_eq!(
        keys(&again.records),
        ["early", "between"],
        "the deferred key is not handed out again under this lease"
    );
    fixture.release("c1", lease).unwrap();
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.position, record.rank),
        (RecordStatus::Queued, arrival, Rank::new(3))
    );
    let next = fixture.pull("c1", 10);
    assert_ne!(next.lease, lease);
    assert_eq!(keys(&next.records), ["early", "k", "between"]);
}

// scenario: S045
// obligation: rule-success.Supersede
// obligation: rule-failure.DeferBehindHeldVersion.5
// obligation: guarantee.NothingLostWhileHeld
// obligation: guarantee.EveryDropIsVisible
#[test]
fn two_supersedes_while_held_shed_only_the_deferred_version_and_never_the_held_one() {
    let (mut fixture, lease) = held();
    fixture.at(10);
    fixture.publish("pk", "k", 3, "v3").unwrap();
    assert_eq!(fixture.snapshot().superseded.len(), 0, "v1 is held");
    fixture.at(20);
    assert_eq!(
        fixture.publish("pk", "k", 5, "v5"),
        Ok(PublishOutcome::Superseded)
    );
    let v3 = (
        "pk",
        "k",
        3,
        "v3",
        ShedReason::Superseded,
        at(20),
        at(20) + SUPERSEDED_TTL,
    );
    assert_eq!(shed_rows(&fixture.snapshot()), [v3], "v3 is shed at once");
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank),
        (
            held_as(1, "v1", Mark::Unsettled, None, Some(2)),
            Rank::new(5)
        ),
        "v5 keeps v3's arrival position"
    );
    fixture.ack("c1", lease, "k").unwrap();
    fixture.at(30);
    fixture.release("c1", lease).unwrap();
    let snapshot = fixture.snapshot();
    assert_eq!(shed_rows(&snapshot), [v3], "the acked v1 is consumed");
    assert_eq!(letter_rows(&snapshot), []);
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.position, record.rank),
        (RecordStatus::Queued, Position::new(2), Rank::new(5))
    );
}

// scenario: S046
// obligation: rule-success.DropStalePublish
#[test]
fn a_stale_check_while_held_compares_with_the_latest_rank_not_the_held_one() {
    let (mut fixture, _) = held();
    fixture.publish("pk", "k", 3, "v3").unwrap();
    assert_eq!(
        fixture.publish("pk", "k", 2, "v2"),
        Ok(PublishOutcome::Stale)
    );
    assert_eq!(
        fixture.publish("pk", "k", 3, "again"),
        Err(PublishError::EqualRank {
            key: RecordKey::new("k"),
            rank: Rank::new(3)
        }),
        "an equal rank is rejected, against the latest rank"
    );
    let record = fixture.record("pk", "k");
    assert_eq!(record.rank, Rank::new(3));
    assert_eq!(
        record.status,
        held_as(1, "v1", Mark::Unsettled, None, Some(2))
    );
}

// scenario: S047
// obligation: rule-success.NackMarksRecord
// obligation: rule-failure.NackRepeats.5
// obligation: when-presence.Record.nacked_at
// obligation: entity-optional.Record.nacked_at
// obligation: rule-success.DeadLetterNackedRecord
// obligation: rule-entity-creation.DeadLetterNackedRecord.1
// obligation: entity-fields.DeadLetter
// obligation: invariant.DeadLetter.NackedBeforeExpiry
#[test]
fn a_nack_marks_the_held_version_without_redelivery_and_dead_letters_it_at_the_lease_end() {
    let (mut fixture, lease) = held();
    fixture.at(20);
    assert_eq!(fixture.nack("c1", lease, "k"), Ok(SettleOutcome::Nacked));
    assert_eq!(
        fixture.lease_view("c1").unwrap().expires_at,
        at(20) + LEASE_TTL,
        "the nack extended it"
    );
    assert_eq!(
        fixture.record("pk", "k").status,
        held_as(1, "v1", Mark::Nacked, Some(at(20)), None)
    );
    assert_eq!(
        fixture.snapshot().dead_letters.len(),
        0,
        "not before the end"
    );
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert!(again.records.is_empty(), "no redelivery");
    assert_eq!(
        again.expires_at,
        at(20) + LEASE_TTL,
        "the pull within the lease extends it"
    );
    fixture.at(30);
    fixture.release("c1", lease).unwrap();
    let snapshot = fixture.snapshot();
    let letter = ("pk", "k", 1, "v1", "c1", at(20), at(30) + DEAD_LETTER_TTL);
    assert_eq!(letter_rows(&snapshot), [letter]);
    invariants::nacked_before_expiry(&snapshot);
    assert_eq!(fixture.pocket("pk").records.len(), 0);
}

// scenario: S049
// obligation: rule-failure.NackMarksRecord.1
// obligation: rule-failure.NackMarksRecord.2
// obligation: rule-failure.NackMarksRecord.3
// obligation: rule-failure.NackMarksRecord.4
// obligation: rule-failure.NackRepeats.1
// obligation: rule-failure.NackRepeats.2
// obligation: rule-failure.NackRepeats.3
// obligation: rule-failure.NackRepeats.4
#[test]
fn rejected_nacks_change_nothing_and_create_no_dead_letter() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v").unwrap();
    fixture.publish("pk", "queued", 1, "q").unwrap();
    fixture.publish("other", "x", 1, "x").unwrap();
    let lease = fixture.pull("c1", 1).lease;
    let theirs = fixture.pull("c2", 1).lease;
    fixture.advance(500);
    let unknown = LeaseActionError::UnknownConsumer(consumer("nobody"));
    fixture.refused(&unknown, |f| f.nack("nobody", lease, "k"));
    let never = LeaseId::from_u128(99);
    let not_active = LeaseActionError::LeaseNotActive(never);
    fixture.refused(&not_active, |f| f.nack("c1", never, "k"));
    let not_yours = LeaseActionError::LeaseNotYours(theirs);
    fixture.refused(&not_yours, |f| f.nack("c1", theirs, "x"));
    for key in ["absent", "queued", "x"] {
        let not_held = LeaseActionError::RecordNotHeld(RecordKey::new(key));
        fixture.refused(&not_held, |f| f.nack("c1", lease, key));
    }
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
    assert_eq!(fixture.lease_view("c1").unwrap().expires_at, T0 + LEASE_TTL);
}

/// `pk` with `k1` and `k2`, both held by `c1` from `T0`, then `k1` nacked and
/// `k2` acked at 10 ms.
fn marked_both_ways() -> (Fixture, LeaseId) {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k1", 1, "k1").unwrap();
    fixture.publish("pk", "k2", 1, "k2").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    fixture.at(10);
    fixture.nack("c1", lease, "k1").unwrap();
    fixture.ack("c1", lease, "k2").unwrap();
    (fixture, lease)
}

// scenario: S084
// obligation: rule-success.AckRepeats
// obligation: rule-success.NackRepeats
// obligation: rule-failure.AckMarksRecord.5
// obligation: rule-failure.NackMarksRecord.5
// obligation: guarantee.MarksStandHoweverLeaseEnds
#[test]
fn a_repeat_ack_or_nack_answers_like_the_first_extends_the_lease_and_changes_nothing_else() {
    let (mut fixture, lease) = marked_both_ways();
    fixture.at(30);
    let mut expected = fixture.snapshot();
    expected.leases[0].expires_at = at(30) + LEASE_TTL;
    assert_eq!(fixture.ack("c1", lease, "k2"), Ok(SettleOutcome::Acked));
    assert_eq!(fixture.nack("c1", lease, "k1"), Ok(SettleOutcome::Nacked));
    assert_eq!(
        fixture.snapshot(),
        expected,
        "a repeat moves the lease's expiry and nothing else"
    );
    assert_eq!(
        fixture.record("pk", "k1").status,
        held_as(1, "k1", Mark::Nacked, Some(at(10)), None),
        "the first nack's time stands"
    );
}

// scenario: S085
// obligation: rule-failure.AckMarksRecord.5
// obligation: rule-failure.AckRepeats.5
// obligation: rule-failure.NackMarksRecord.5
// obligation: rule-failure.NackRepeats.5
// obligation: guarantee.MarksStandHoweverLeaseEnds
#[test]
fn a_changed_mark_is_refused_as_a_mark_conflict_and_changes_nothing() {
    let (mut fixture, lease) = marked_both_ways();
    fixture.at(500);
    let conflict = |key: &str| LeaseActionError::MarkConflict(RecordKey::new(key));
    fixture.refused(&conflict("k1"), |f| f.ack("c1", lease, "k1"));
    fixture.refused(&conflict("k2"), |f| f.nack("c1", lease, "k2"));
    assert_eq!(
        fixture.lease_view("c1").unwrap().records,
        [
            marked("k1", "k1", Mark::Nacked),
            marked("k2", "k2", Mark::Acked)
        ],
        "the first marks stand"
    );
    // The queue's clock never runs back: had a refusal read the clock at 500, a
    // renew at 100 would extend from 500.
    fixture.at(100);
    assert_eq!(
        fixture.renew("c1", lease),
        Ok(at(100) + LEASE_TTL),
        "a refused mark reads no clock"
    );
}

// scenario: S086
// obligation: rule-success.NackRepeats
// obligation: rule-success.DeadLetterNackedRecord
#[test]
fn a_repeat_nack_keeps_the_first_nacks_time() {
    let (mut fixture, lease) = held();
    fixture.at(10);
    fixture.nack("c1", lease, "k").unwrap();
    fixture.at(30);
    assert_eq!(fixture.nack("c1", lease, "k"), Ok(SettleOutcome::Nacked));
    fixture.at(40);
    fixture.release("c1", lease).unwrap();
    let letter = ("pk", "k", 1, "v1", "c1", at(10), at(40) + DEAD_LETTER_TTL);
    assert_eq!(letter_rows(&fixture.snapshot()), [letter]);
}

// scenario: S090
// obligation: surface-exposure.Consuming
// obligation: projection.Pocket.held_records
// obligation: when-presence.Record.mark
#[test]
fn the_lease_read_shows_every_held_record_with_its_mark_smallest_position_first() {
    let mut fixture = Fixture::new();
    for key in ["z", "a", "m"] {
        fixture.publish("pk", key, 1, key).unwrap();
    }
    let lease = fixture.pull("c1", 10).lease;
    fixture.ack("c1", lease, "a").unwrap();
    fixture.nack("c1", lease, "m").unwrap();
    assert_eq!(
        fixture.lease_view("c1").unwrap().records,
        [
            marked("z", "z", Mark::Unsettled),
            marked("a", "a", Mark::Acked),
            marked("m", "m", Mark::Nacked)
        ]
    );
}

// scenario: S071
// obligation: guarantee.OneVisitPerKeyPerLease
// obligation: rule-success.DeferBehindHeldVersion
// obligation: rule-success.ConsumeAckedVersionQueueDeferred
#[test]
fn a_key_acked_then_published_again_within_the_lease_waits_until_the_lease_ends() {
    let (mut fixture, lease) = held();
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.publish("pk", "k", 2, "v2"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank),
        (held_as(1, "v1", Mark::Acked, None, Some(2)), Rank::new(2)),
        "the record stays held and acked, with v2 deferred"
    );
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert!(again.records.is_empty(), "not pullable again in this lease");
    fixture.release("c1", lease).unwrap();
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.position),
        (RecordStatus::Queued, Position::new(2))
    );
    let next = fixture.pull("c1", 10);
    assert_ne!(next.lease, lease);
    assert_eq!(keys(&next.records), ["k"]);
    assert_eq!(next.records[0].rank, Rank::new(2));
}

// scenario: S072
// obligation: guarantee.OneVisitPerKeyPerLease
// obligation: rule-success.DeadLetterNackedVersionQueueDeferred
#[test]
fn a_key_nacked_then_published_again_within_the_lease_waits_until_the_lease_ends() {
    let (mut fixture, lease) = held();
    fixture.nack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.publish("pk", "k", 2, "v2"),
        Ok(PublishOutcome::Superseded)
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.rank),
        (
            held_as(1, "v1", Mark::Nacked, Some(T0), Some(2)),
            Rank::new(2)
        )
    );
    let again = fixture.pull("c1", 10);
    assert!(again.records.is_empty(), "not pullable again in this lease");
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
    fixture.at(5);
    fixture.release("c1", lease).unwrap();
    let letter = ("pk", "k", 1, "v1", "c1", T0, at(5) + DEAD_LETTER_TTL);
    assert_eq!(
        letter_rows(&fixture.snapshot()),
        [letter],
        "the dead letter appears at the release"
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.position, record.rank),
        (RecordStatus::Queued, Position::new(2), Rank::new(2)),
        "v2 is queued at its arrival position"
    );
    assert_eq!(keys(&fixture.pull("c1", 10).records), ["k"]);
}
