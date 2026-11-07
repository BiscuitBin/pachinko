//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord`
//! and `NackDefersNewerVersion`, their refusals, and the two open-question-9
//! scenarios as the rules stand.

use std::time::Duration;

use libpachinko::error::{LeaseActionError, PublishError};
use libpachinko::ids::{Payload, Rank, RecordKey};
use libpachinko::lease_ids::LeaseId;
use libpachinko::queue::{PublishOutcome, SettleOutcome, ShedReason};
use libpachinko::records::RecordStatus;

use crate::fixture::{DEAD_LETTER_TTL, Fixture, LEASE_TTL, T0, config, consumer, invariants, keys};

/// A fixture whose pocket `pk` holds `k` at rank 1, payload `v1`, pulled by `c1`.
fn held() -> (Fixture, LeaseId) {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "k", 1, "v1").unwrap();
    let batch = fixture.pull("c1", 10);
    (fixture, batch.lease)
}

// scenario: S040
// obligation: rule-success.AckSettlesRecord
// obligation: rule-failure.AckDefersNewerVersion.5
#[test]
fn an_ack_settles_a_held_record() {
    let (mut fixture, lease) = held();
    fixture.publish("pk", "other", 1, "o").unwrap();
    assert_eq!(fixture.ack("c1", lease, "k"), Ok(SettleOutcome::Settled));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.records.len(), 1, "only the other key is left");
    assert_eq!(pocket.records[0].key.as_str(), "other");
    assert_eq!(fixture.lease_view("c1").unwrap().records.len(), 0);
    assert_eq!(fixture.snapshot().superseded.len(), 0);
    assert_eq!(fixture.snapshot().dead_letters.len(), 0);
    invariants::all(&fixture.snapshot());
}

// scenario: S041
// obligation: rule-success.AckSettlesRecord
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
// obligation: rule-failure.AckSettlesRecord.1
// obligation: rule-failure.AckSettlesRecord.2
// obligation: rule-failure.AckSettlesRecord.3
// obligation: rule-failure.AckSettlesRecord.4
// obligation: rule-failure.AckDefersNewerVersion.1
// obligation: rule-failure.AckDefersNewerVersion.2
// obligation: rule-failure.AckDefersNewerVersion.3
// obligation: rule-failure.AckDefersNewerVersion.4
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
    let not_held = LeaseActionError::RecordNotHeld(RecordKey::new("k"));
    fixture.refused(&not_held, |f| f.ack("c1", lease, "k"));
}

// scenario: S043
// obligation: rule-success.AckDefersNewerVersion
// obligation: rule-failure.AckSettlesRecord.5
// obligation: transition-edge.Record.held.deferred
// obligation: projection.Pocket.deferred_records
// obligation: guarantee.NothingLostWhileHeld
#[test]
fn a_supersede_while_the_key_is_held_defers_it_on_ack() {
    let (mut fixture, lease) = held();
    assert_eq!(
        fixture.publish("pk", "k", 3, "v3"),
        Ok(PublishOutcome::Superseded)
    );
    let next = fixture.snapshot().next_position;
    assert_eq!(fixture.ack("c1", lease, "k"), Ok(SettleOutcome::Deferred));
    let record = fixture.record("pk", "k");
    assert_eq!(record.status, RecordStatus::Deferred);
    assert_eq!(
        (record.rank, record.payload),
        (Rank::new(3), Payload::from(b"v3".as_slice()))
    );
    assert_eq!(record.position, next, "a fresh position");
    assert_eq!(fixture.snapshot().next_position, next.next());
    assert_eq!(fixture.lease_view("c1").unwrap().records.len(), 0);
    invariants::all(&fixture.snapshot());
}

// scenario: S044
// obligation: rule-success.RequeueDeferredRecord
// obligation: transition-edge.Record.deferred.queued
// obligation: guarantee.OneVisitPerKeyPerLease
#[test]
fn a_deferred_record_restarts_at_the_back_of_its_pocket() {
    let (mut fixture, lease) = held();
    fixture.publish("pk", "k", 3, "v3").unwrap();
    fixture.publish("pk", "between", 1, "b").unwrap();
    let between = fixture.record("pk", "between").position;
    fixture.ack("c1", lease, "k").unwrap();
    assert!(fixture.record("pk", "k").position > between);
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert_eq!(
        keys(&again.records),
        ["between"],
        "the deferred key is not handed out again under this lease"
    );
    fixture.release("c1", lease).unwrap();
    assert_eq!(fixture.record("pk", "k").status, RecordStatus::Queued);
    let next = fixture.pull("c1", 10);
    assert_ne!(next.lease, lease);
    assert_eq!(keys(&next.records), ["between", "k"]);
    assert_eq!(next.records[1].rank, Rank::new(3));
}

// scenario: S045
// obligation: rule-success.Supersede
// obligation: guarantee.NothingLostWhileHeld
#[test]
fn two_supersedes_while_held_shed_both_losers_and_leave_the_held_version() {
    let (mut fixture, lease) = held();
    fixture.publish("pk", "k", 3, "v3").unwrap();
    fixture.publish("pk", "k", 5, "v5").unwrap();
    let shed: Vec<_> = fixture
        .snapshot()
        .superseded
        .into_iter()
        .map(|s| (s.rank.get(), s.reason))
        .collect();
    assert_eq!(
        shed,
        [(1, ShedReason::Superseded), (3, ShedReason::Superseded)]
    );
    let held = &fixture.lease_view("c1").unwrap().records[0];
    assert_eq!(
        (held.rank, &held.payload),
        (Rank::new(1), &Payload::from(b"v1".as_slice()))
    );
    assert_eq!(fixture.ack("c1", lease, "k"), Ok(SettleOutcome::Deferred));
    assert_eq!(fixture.record("pk", "k").rank, Rank::new(5));
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
        "the OQ1 placeholder, against the latest rank"
    );
    let record = fixture.record("pk", "k");
    assert_eq!(record.rank, Rank::new(3));
    assert_eq!(
        record.status,
        RecordStatus::Held {
            rank: Rank::new(1),
            payload: Payload::from(b"v1".as_slice())
        }
    );
}

// scenario: S047
// obligation: rule-success.NackSettlesRecord
// obligation: rule-entity-creation.NackSettlesRecord.1
// obligation: rule-failure.NackDefersNewerVersion.5
// obligation: entity-fields.DeadLetter
// obligation: invariant.DeadLetter.NackedBeforeExpiry
#[test]
fn a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery() {
    let (mut fixture, lease) = held();
    fixture.advance(20);
    assert_eq!(fixture.nack("c1", lease, "k"), Ok(SettleOutcome::Settled));
    let now = T0.saturating_add_millis(20);
    assert_eq!(
        fixture.lease_view("c1").unwrap().expires_at,
        now + LEASE_TTL,
        "the nack extended it"
    );
    let snapshot = fixture.snapshot();
    let [letter] = snapshot.dead_letters.as_slice() else {
        panic!("one dead letter")
    };
    assert_eq!(
        (letter.pocket_key.as_str(), letter.key.as_str(), letter.rank),
        ("pk", "k", Rank::new(1))
    );
    assert_eq!(letter.payload, Payload::from(b"v1".as_slice()));
    assert_eq!(letter.nacked_by, consumer("c1"));
    assert_eq!(
        (letter.nacked_at, letter.expires_at),
        (now, now + DEAD_LETTER_TTL)
    );
    invariants::nacked_before_expiry(&snapshot);
    assert_eq!(fixture.pocket("pk").records.len(), 0);
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert!(again.records.is_empty(), "no redelivery");
    assert_eq!(
        again.expires_at,
        now + LEASE_TTL,
        "the pull within the lease extends it"
    );
}

// obligation: rule-success.NackDefersNewerVersion
// obligation: rule-entity-creation.NackDefersNewerVersion.1
// obligation: rule-failure.NackSettlesRecord.5
#[test]
fn a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer() {
    let (mut fixture, lease) = held();
    fixture.publish("pk", "k", 3, "v3").unwrap();
    let next = fixture.snapshot().next_position;
    fixture.advance(20);
    assert_eq!(fixture.nack("c1", lease, "k"), Ok(SettleOutcome::Deferred));
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.leases[0].expires_at,
        T0.saturating_add_millis(20) + LEASE_TTL,
        "the nack extended it"
    );
    let [letter] = snapshot.dead_letters.as_slice() else {
        panic!("one dead letter")
    };
    assert_eq!(
        (letter.rank, &letter.payload),
        (Rank::new(1), &Payload::from(b"v1".as_slice()))
    );
    let record = fixture.record("pk", "k");
    assert_eq!(
        (record.status, record.position, record.rank),
        (RecordStatus::Deferred, next, Rank::new(3))
    );
    invariants::all(&snapshot);
}

// scenario: S049
// obligation: rule-failure.NackSettlesRecord.1
// obligation: rule-failure.NackSettlesRecord.2
// obligation: rule-failure.NackSettlesRecord.3
// obligation: rule-failure.NackSettlesRecord.4
// obligation: rule-failure.NackDefersNewerVersion.1
// obligation: rule-failure.NackDefersNewerVersion.2
// obligation: rule-failure.NackDefersNewerVersion.3
// obligation: rule-failure.NackDefersNewerVersion.4
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

// scenario: S071
// obligation: guarantee.OneVisitPerKeyPerLease
#[test]
fn oq9_a_key_acked_then_published_again_within_the_lease_is_a_new_queued_record() {
    // OQ9, as the rules stand: publish creates the record queued, so it is
    // pullable under the same lease. If the maintainer chooses one visit per key,
    // this test flips.
    let (mut fixture, lease) = held();
    fixture.ack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.publish("pk", "k", 2, "v2"),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(fixture.record("pk", "k").status, RecordStatus::Queued);
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert_eq!(keys(&again.records), ["k"]);
    assert_eq!(again.records[0].rank, Rank::new(2));
}

// scenario: S072
// obligation: guarantee.OneVisitPerKeyPerLease
#[test]
fn oq9_a_key_nacked_then_published_again_within_the_lease_is_a_new_queued_record() {
    // OQ9, as the rules stand: see the ack test above.
    let (mut fixture, lease) = held();
    fixture.nack("c1", lease, "k").unwrap();
    assert_eq!(
        fixture.publish("pk", "k", 2, "v2"),
        Ok(PublishOutcome::NewKey)
    );
    let again = fixture.pull("c1", 10);
    assert_eq!(again.lease, lease);
    assert_eq!(keys(&again.records), ["k"]);
    assert_eq!(fixture.snapshot().dead_letters.len(), 1);
}
