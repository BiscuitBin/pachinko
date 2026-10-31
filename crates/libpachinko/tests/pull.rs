//! Pull: `PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord` and
//! `PullFindsNoPocket`, the `OldestFirst` and `ExclusivePocket` guarantees, and the
//! `Consuming` surface's `exposes`.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Batch, Config, ConsumerId, CountingLeaseIds, Error, FakeClock, HeldLease, LeaseId, LeaseStatus,
    Payload, PocketKey, PocketStatus, PocketView, Publication, PublisherId, PullOutcome, Queue,
    Rank, RecordKey, RecordStatus, Snapshot, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const LEASE_TTL: Duration = Duration::from_millis(1_000);
const MAX_PULL_BATCH: u32 = 3;

fn queue() -> (TestQueue, FakeClock) {
    let config = Config::new(
        LEASE_TTL,
        2,
        MAX_PULL_BATCH,
        Duration::from_millis(5_000),
        Duration::from_millis(7_000),
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    for c in ["c1", "c2", "c3"] {
        queue.consumer_joins(ConsumerId::new(c)).unwrap();
    }
    (queue, clock)
}

fn publish(queue: &mut TestQueue, pocket: &str, key: &str, rank: i64) {
    queue
        .publish(Publication::new(
            PublisherId::new("p"),
            PocketKey::new(pocket),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::from(format!("{key}@{rank}").as_bytes()),
        ))
        .unwrap();
}

fn batch(queue: &mut TestQueue, consumer: &str, limit: i64) -> Batch {
    match queue.pull(&ConsumerId::new(consumer), limit).unwrap() {
        PullOutcome::Batch(batch) => batch,
        other => panic!("expected a batch, got {other:?}"),
    }
}

fn keys(batch: &Batch) -> Vec<&str> {
    batch
        .records
        .iter()
        .map(|r| r.record_key.as_str())
        .collect()
}

fn pocket<'a>(snapshot: &'a Snapshot, key: &str) -> &'a PocketView {
    snapshot
        .pockets
        .iter()
        .find(|p| p.key.as_str() == key)
        .unwrap()
}

// rule-success.PullFindsNoPocket
#[test]
fn a_consumer_without_a_lease_finds_no_pocket_when_none_is_queued() {
    let (mut queue, _) = queue();
    let before = queue.snapshot();
    assert_eq!(
        queue.pull(&ConsumerId::new("c1"), 5),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(queue.snapshot(), before);
}

// rule-failure.PullFindsNoPocket.1 (the consumer holds a lease) and
// rule-failure.PullFindsNoPocket.2 (a pocket is queued)
#[test]
fn a_consumer_finds_a_pocket_when_one_is_queued_or_it_holds_a_lease() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    let claimed = batch(&mut queue, "c1", 5);
    let again = batch(&mut queue, "c1", 5);
    assert_eq!(again.lease_id, claimed.lease_id);
    assert_eq!(again.records, []);
}

// surface-actor.Consuming: a pull from a consumer that has not joined is refused.
#[test]
fn a_pull_from_a_consumer_that_has_not_joined_is_refused() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    let before = queue.snapshot();
    assert_eq!(
        queue.pull(&ConsumerId::new("stranger"), 1),
        Err(Error::UnknownConsumer(ConsumerId::new("stranger")))
    );
    assert_eq!(queue.snapshot(), before);
}

// rule-success.PullClaimsFrontPocket, rule-entity-creation.PullClaimsFrontPocket.1,
// transition-edge.Pocket.queued.leased, invariant.Lease.GrantedBeforeExpiry,
// entity-fields.Lease
#[test]
fn a_pull_claims_the_front_pocket_under_a_new_lease() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "b", "k1", 1);
    publish(&mut queue, "a", "k1", 1);
    clock.advance(Duration::from_millis(30));
    let claimed = batch(&mut queue, "c1", 5);
    assert_eq!(claimed.pocket_key, PocketKey::new("b"));
    assert_eq!(claimed.lease_id, LeaseId::from_u128(1));
    assert_eq!(
        claimed.expires_at,
        Timestamp::from_millis(1_030) + LEASE_TTL
    );
    let snapshot = queue.snapshot();
    assert_eq!(pocket(&snapshot, "b").status, PocketStatus::Leased);
    assert_eq!(snapshot.pocket_order, [PocketKey::new("a")]);
    let lease = &snapshot.leases[0];
    assert_eq!(lease.id, claimed.lease_id);
    assert_eq!(lease.pocket_key, PocketKey::new("b"));
    assert_eq!(lease.consumer, ConsumerId::new("c1"));
    assert_eq!(lease.status, LeaseStatus::Active);
    assert_eq!(lease.granted_at, Timestamp::from_millis(1_030));
    assert_eq!(lease.expires_at, claimed.expires_at);
    assert!(lease.granted_at <= lease.expires_at);
}

// rule-failure.PullClaimsFrontPocket.1: a consumer with an active lease claims nothing.
#[test]
fn a_consumer_holding_a_lease_claims_no_second_pocket() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    batch(&mut queue, "c1", 5);
    let again = batch(&mut queue, "c1", 5);
    assert_eq!(again.pocket_key, PocketKey::new("a"));
    let snapshot = queue.snapshot();
    assert_eq!(snapshot.leases.len(), 1);
    assert_eq!(pocket(&snapshot, "b").status, PocketStatus::Queued);
}

// rule-failure.PullClaimsFrontPocket.2 (no queued pocket) and
// rule-failure.PullClaimsFrontPocket.3 (only a queued pocket is claimed: leased, idle
// and parked pockets are passed over)
#[test]
fn only_a_queued_pocket_is_claimed() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    batch(&mut queue, "c1", 5);
    assert_eq!(
        queue.pull(&ConsumerId::new("c2"), 5),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(queue.snapshot().leases.len(), 1);
}

// rule-success.PullWithinLease: the lease is extended and the next oldest records
// handed out.
#[test]
fn a_pull_within_a_lease_extends_it_and_hands_out_the_next_records() {
    let (mut queue, clock) = queue();
    for key in ["k1", "k2", "k3", "k4"] {
        publish(&mut queue, "a", key, 1);
    }
    let first = batch(&mut queue, "c1", 2);
    assert_eq!(keys(&first), ["k1", "k2"]);
    clock.advance(Duration::from_millis(400));
    let second = batch(&mut queue, "c1", 2);
    assert_eq!(second.lease_id, first.lease_id);
    assert_eq!(keys(&second), ["k3", "k4"]);
    assert_eq!(second.expires_at, Timestamp::from_millis(1_400) + LEASE_TTL);
    assert_eq!(queue.snapshot().leases[0].expires_at, second.expires_at);
}

// rule-failure.PullWithinLease.1: without an active lease the pull claims instead.
#[test]
fn a_pull_without_a_lease_is_not_a_pull_within_one() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    let first = batch(&mut queue, "c1", 5);
    queue
        .release(&ConsumerId::new("c1"), first.lease_id)
        .unwrap();
    let second = batch(&mut queue, "c1", 5);
    assert_ne!(second.lease_id, first.lease_id);
    assert_eq!(keys(&second), ["k1"]);
}

// rule-success.HoldRecord, when-set.HoldRecord.Record.held_rank,
// when-set.HoldRecord.Record.held_payload, transition-edge.Record.queued.held,
// when-presence.Record.held_rank, when-presence.Record.held_payload
#[test]
fn every_record_a_batch_hands_out_is_held_at_the_version_handed() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 3);
    publish(&mut queue, "a", "k2", 5);
    publish(&mut queue, "a", "k3", 1);
    let handed = batch(&mut queue, "c1", 2);
    let versions: Vec<(&str, i64, &[u8])> = handed
        .records
        .iter()
        .map(|r| (r.record_key.as_str(), r.rank.get(), r.payload.as_bytes()))
        .collect();
    assert_eq!(
        versions,
        [("k1", 3, b"k1@3".as_slice()), ("k2", 5, b"k2@5".as_slice())]
    );
    let snapshot = queue.snapshot();
    let records = &pocket(&snapshot, "a").records;
    assert_eq!(
        records[0].status,
        RecordStatus::Held {
            rank: Rank::new(3),
            payload: Payload::from(b"k1@3".as_slice())
        }
    );
    assert_eq!(
        records[1].status,
        RecordStatus::Held {
            rank: Rank::new(5),
            payload: Payload::from(b"k2@5".as_slice())
        }
    );
    assert_eq!(records[2].status, RecordStatus::Queued);
}

// rule-failure.HoldRecord.2: a held record is not handed out, nor held, again;
// rule-failure.HoldRecord.1: an acked record, which no longer exists, is not handed out.
#[test]
fn a_record_already_held_or_settled_is_not_handed_out_again() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    let first = batch(&mut queue, "c1", 1);
    assert_eq!(keys(&first), ["k1"]);
    queue
        .ack(
            &ConsumerId::new("c1"),
            first.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    let second = batch(&mut queue, "c1", 5);
    assert_eq!(keys(&second), ["k2"]);
    assert_eq!(batch(&mut queue, "c1", 5).records, []);
}

// The OldestFirst guarantee: smallest positions first, at most limit, at most
// max_pull_batch, and a limit that is not positive yields an empty batch.
#[test]
fn a_batch_is_the_oldest_records_bounded_by_the_limit_and_the_maximum() {
    let (mut queue, _) = queue();
    for key in ["k5", "k1", "k4", "k2", "k3"] {
        publish(&mut queue, "a", key, 1);
    }
    publish(&mut queue, "a", "k1", 2);
    assert_eq!(keys(&batch(&mut queue, "c1", 1)), ["k5"]);
    assert_eq!(keys(&batch(&mut queue, "c1", 100)), ["k1", "k4", "k2"]);
    assert_eq!(keys(&batch(&mut queue, "c1", i64::MAX)), ["k3"]);
}

// The OldestFirst guarantee's last clause, for a claim and for a pull within a lease.
#[test]
fn a_limit_that_is_not_positive_yields_an_empty_batch_not_an_error() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let claimed = batch(&mut queue, "c1", 0);
    assert_eq!(claimed.records, []);
    assert_eq!(pocket(&queue.snapshot(), "a").status, PocketStatus::Leased);
    clock.advance(Duration::from_millis(10));
    let within = batch(&mut queue, "c1", -5);
    assert_eq!(within.records, []);
    assert_eq!(within.expires_at, Timestamp::from_millis(1_010) + LEASE_TTL);
    assert_eq!(batch(&mut queue, "c1", i64::MIN + 1).records, []);
}

// The ExclusivePocket guarantee: a leased pocket is handed to no other consumer.
#[test]
fn a_leased_pocket_is_handed_to_no_other_consumer() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    let first = batch(&mut queue, "c1", 1);
    let second = batch(&mut queue, "c2", 5);
    assert_eq!(first.pocket_key, PocketKey::new("a"));
    assert_eq!(second.pocket_key, PocketKey::new("b"));
    assert_eq!(
        queue.pull(&ConsumerId::new("c3"), 5),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(keys(&batch(&mut queue, "c1", 5)), ["k2"]);
}

// surface-exposure.Consuming: the lease's pocket key, expires_at and each held
// record's key, held rank and held payload.
#[test]
fn a_consumer_sees_its_lease_and_the_versions_it_holds() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    publish(&mut queue, "a", "k3", 1);
    let handed = batch(&mut queue, "c1", 2);
    publish(&mut queue, "a", "k1", 9);
    let lease: HeldLease = queue.lease_of(&ConsumerId::new("c1")).unwrap().unwrap();
    assert_eq!(lease.lease_id, handed.lease_id);
    assert_eq!(lease.pocket_key, PocketKey::new("a"));
    assert_eq!(lease.expires_at, handed.expires_at);
    assert_eq!(
        lease.records, handed.records,
        "the held versions, not k1 at rank 9"
    );
    assert_eq!(queue.lease_of(&ConsumerId::new("c2")), Ok(None));
    assert_eq!(
        queue.lease_of(&ConsumerId::new("stranger")),
        Err(Error::UnknownConsumer(ConsumerId::new("stranger")))
    );
}
