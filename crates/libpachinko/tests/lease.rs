//! Renew and release: `Renew`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`,
//! `RequeueHeldRecord` and `RequeueDeferredRecord`, and the `ActionsNameTheirLease`
//! guarantee.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Batch, Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, LeaseStatus, Payload,
    PocketFate, PocketKey, PocketStatus, PocketView, Position, Publication, PublisherId,
    PullOutcome, Queue, Rank, RecordKey, RecordStatus, Snapshot, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const LEASE_TTL: Duration = Duration::from_millis(1_000);

fn queue() -> (TestQueue, FakeClock) {
    let config = Config::new(
        LEASE_TTL,
        2,
        10,
        Duration::from_millis(5_000),
        Duration::from_millis(7_000),
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    queue.consumer_joins(ConsumerId::new("c1")).unwrap();
    queue.consumer_joins(ConsumerId::new("c2")).unwrap();
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

fn pull(queue: &mut TestQueue, consumer: &str, limit: i64) -> Batch {
    match queue.pull(&ConsumerId::new(consumer), limit).unwrap() {
        PullOutcome::Batch(batch) => batch,
        other => panic!("expected a batch, got {other:?}"),
    }
}

fn c(id: &str) -> ConsumerId {
    ConsumerId::new(id)
}

fn pocket<'a>(snapshot: &'a Snapshot, key: &str) -> &'a PocketView {
    snapshot
        .pockets
        .iter()
        .find(|p| p.key.as_str() == key)
        .unwrap()
}

fn record_status<'a>(snapshot: &'a Snapshot, key: &str) -> &'a RecordStatus {
    &pocket(snapshot, "a")
        .records
        .iter()
        .find(|r| r.key.as_str() == key)
        .unwrap()
        .status
}

// rule-success.Renew
#[test]
fn a_renewal_moves_expires_at_to_now_plus_the_ttl() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    clock.advance(Duration::from_millis(700));
    let renewed = Timestamp::from_millis(1_700) + LEASE_TTL;
    assert_eq!(queue.renew(&c("c1"), batch.lease_id), Ok(renewed));
    assert_eq!(queue.snapshot().leases[0].expires_at, renewed);
}

// rule-failure.Renew.1 (not active) and rule-failure.Renew.2 (not the consumer's)
#[test]
fn a_renewal_needs_an_active_lease_of_the_consumers() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    assert_eq!(
        queue.renew(&c("c2"), batch.lease_id),
        Err(Error::LeaseNotYours(batch.lease_id))
    );
    queue.release(&c("c1"), batch.lease_id).unwrap();
    let before = queue.snapshot();
    assert_eq!(
        queue.renew(&c("c1"), batch.lease_id),
        Err(Error::LeaseNotActive(batch.lease_id))
    );
    assert_eq!(queue.snapshot(), before);
}

// rule-success.ReleaseRequeuesPocket, when-set.ReleaseRequeuesPocket.Lease.ended_at,
// transition-edge.Lease.active.released, transition-edge.Pocket.leased.queued,
// rule-success.RequeueHeldRecord, transition-edge.Record.held.queued,
// rule-success.RequeueDeferredRecord, transition-edge.Record.deferred.queued,
// invariant.Lease.EndedAfterGrant, when-presence.Lease.ended_at
#[test]
fn a_release_with_records_requeues_the_pocket_at_the_back_and_frees_its_records() {
    let (mut queue, clock) = queue();
    for key in ["k1", "k2", "k3"] {
        publish(&mut queue, "a", key, 1);
    }
    publish(&mut queue, "b", "k1", 1);
    let batch = pull(&mut queue, "c1", 2);
    publish(&mut queue, "a", "k1", 2);
    queue
        .ack(&c("c1"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
    let deferred_at = pocket(&queue.snapshot(), "a").records[0].position;
    clock.advance(Duration::from_millis(100));
    let before = queue.snapshot();
    assert_eq!(
        queue.release(&c("c1"), batch.lease_id),
        Ok(PocketFate::Queued)
    );
    let after = queue.snapshot();
    let a = pocket(&after, "a");
    assert_eq!(a.status, PocketStatus::Queued);
    assert_eq!(a.position, before.next_position);
    assert_eq!(a.consecutive_expiries, 0);
    assert_eq!(
        after.next_position,
        Position::new(before.next_position.get() + 1)
    );
    assert_eq!(
        after.pocket_order,
        [PocketKey::new("b"), PocketKey::new("a")]
    );
    assert_eq!(
        a.records[0].status,
        RecordStatus::Queued,
        "deferred k1 is queued"
    );
    assert_eq!(a.records[0].position, deferred_at);
    assert_eq!(
        a.records[1].status,
        RecordStatus::Queued,
        "held k2 is queued"
    );
    assert_eq!(
        a.records[1].position,
        pocket(&before, "a").records[1].position
    );
    let lease = &after.leases[0];
    let ended_at = Timestamp::from_millis(1_100);
    assert_eq!(lease.status, LeaseStatus::Released { ended_at });
    assert!(ended_at >= lease.granted_at);
    assert_eq!(queue.lease_of(&c("c1")), Ok(None));
}

// rule-failure.RequeueHeldRecord.1 and rule-failure.RequeueDeferredRecord.1 (a settled
// record no longer exists and is not requeued), rule-failure.RequeueHeldRecord.2 and
// rule-failure.RequeueDeferredRecord.2 (a queued record is left as it was).
#[test]
fn a_release_frees_only_held_and_deferred_records() {
    let (mut queue, _) = queue();
    for key in ["k1", "k2", "k3"] {
        publish(&mut queue, "a", key, 1);
    }
    let batch = pull(&mut queue, "c1", 2);
    queue
        .ack(&c("c1"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
    let before = queue.snapshot();
    queue.release(&c("c1"), batch.lease_id).unwrap();
    let after = queue.snapshot();
    let keys: Vec<_> = pocket(&after, "a")
        .records
        .iter()
        .map(|r| r.key.as_str())
        .collect();
    assert_eq!(keys, ["k2", "k3"]);
    assert_eq!(
        pocket(&after, "a").records[1],
        pocket(&before, "a").records[1],
        "k3 untouched"
    );
}

// rule-success.ReleaseIdlesEmptyPocket, when-set.ReleaseIdlesEmptyPocket.Lease.ended_at,
// transition-edge.Pocket.leased.idle, invariant.Pocket.IdleHasNoRecords
#[test]
fn a_release_of_an_empty_pocket_idles_it() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    queue
        .nack(&c("c1"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
    clock.advance(Duration::from_millis(5));
    let next = queue.snapshot().next_position;
    assert_eq!(
        queue.release(&c("c1"), batch.lease_id),
        Ok(PocketFate::Idle)
    );
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Idle);
    assert_eq!(a.consecutive_expiries, 0);
    assert_eq!(a.records, []);
    assert_eq!(
        snapshot.next_position, next,
        "an idle pocket takes no position"
    );
    assert_eq!(snapshot.pocket_order, []);
    assert_eq!(
        snapshot.leases[0].status,
        LeaseStatus::Released {
            ended_at: Timestamp::from_millis(1_005)
        }
    );
}

// rule-failure.ReleaseRequeuesPocket.1 and rule-failure.ReleaseIdlesEmptyPocket.1 (not
// active), rule-failure.ReleaseRequeuesPocket.2 and
// rule-failure.ReleaseIdlesEmptyPocket.2 (not the consumer's),
// transition-terminal.Lease.status (a released lease stays released)
#[test]
fn a_release_needs_an_active_lease_of_the_consumers() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    assert_eq!(
        queue.release(&c("c2"), batch.lease_id),
        Err(Error::LeaseNotYours(batch.lease_id))
    );
    queue.release(&c("c1"), batch.lease_id).unwrap();
    clock.advance(LEASE_TTL * 3);
    let before = queue.snapshot();
    assert_eq!(
        queue.release(&c("c1"), batch.lease_id),
        Err(Error::LeaseNotActive(batch.lease_id))
    );
    assert_eq!(queue.lapse(), [], "an ended lease never lapses");
    assert_eq!(queue.snapshot(), before);
}

// rule-failure.ReleaseRequeuesPocket.4 (no records: idled instead) and
// rule-failure.ReleaseIdlesEmptyPocket.4 (records: requeued instead)
#[test]
fn the_record_count_picks_requeue_or_idle() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    let a = pull(&mut queue, "c1", 0);
    let b = pull(&mut queue, "c2", 1);
    queue
        .ack(&c("c2"), b.lease_id, &RecordKey::new("k1"))
        .unwrap();
    assert_eq!(queue.release(&c("c1"), a.lease_id), Ok(PocketFate::Queued));
    assert_eq!(queue.release(&c("c2"), b.lease_id), Ok(PocketFate::Idle));
}

// The ActionsNameTheirLease guarantee: an action naming an ended lease is rejected even
// when the same consumer holds the same pocket again under a new lease, with the same
// key held at a newer rank.
#[test]
fn an_action_under_an_ended_lease_never_lands_on_its_successor() {
    let (mut queue, _) = queue();
    publish(&mut queue, "a", "k1", 1);
    let old = pull(&mut queue, "c1", 1);
    queue.release(&c("c1"), old.lease_id).unwrap();
    publish(&mut queue, "a", "k1", 2);
    let new = pull(&mut queue, "c1", 1);
    assert_eq!(new.pocket_key, PocketKey::new("a"));
    assert_ne!(new.lease_id, old.lease_id);
    let before = queue.snapshot();
    let k1 = RecordKey::new("k1");
    assert_eq!(
        queue.ack(&c("c1"), old.lease_id, &k1),
        Err(Error::LeaseNotActive(old.lease_id))
    );
    assert_eq!(
        queue.nack(&c("c1"), old.lease_id, &k1),
        Err(Error::LeaseNotActive(old.lease_id))
    );
    assert_eq!(
        queue.renew(&c("c1"), old.lease_id),
        Err(Error::LeaseNotActive(old.lease_id))
    );
    assert_eq!(
        queue.release(&c("c1"), old.lease_id),
        Err(Error::LeaseNotActive(old.lease_id))
    );
    assert_eq!(queue.snapshot(), before);
    assert_eq!(
        record_status(&before, "k1"),
        &RecordStatus::Held {
            rank: Rank::new(2),
            payload: Payload::from(b"k1@2".as_slice())
        }
    );
    assert_eq!(
        queue.ack(&c("c1"), new.lease_id, &k1),
        Ok(libpachinko::SettleOutcome::Settled)
    );
}

// surface-provides.Consuming: ack, nack, renew and release are offered only for the
// consumer's active lease; a lease id never handed out is not one.
#[test]
fn the_lease_actions_are_offered_only_for_the_consumers_active_lease() {
    let (mut queue, _) = queue();
    let never = LeaseId::from_u128(77);
    assert_eq!(
        queue.renew(&c("c1"), never),
        Err(Error::LeaseNotActive(never))
    );
    assert_eq!(
        queue.release(&c("c1"), never),
        Err(Error::LeaseNotActive(never))
    );
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    assert_eq!(
        queue
            .lease_of(&c("c1"))
            .unwrap()
            .map(|lease| lease.lease_id),
        Some(batch.lease_id)
    );
    assert!(queue.renew(&c("c1"), batch.lease_id).is_ok());
}

// invariant.Lease.GrantedBeforeExpiry and invariant.Lease.EndedAfterGrant when the
// clock steps backwards: the queue's `now` never goes back, so a renew, pull, ack or
// release after the step stamps no time before the lease's grant.
#[test]
fn a_clock_that_steps_back_never_moves_a_lease_before_its_grant() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    let batch = pull(&mut queue, "c1", 1);
    clock.set(Timestamp::from_millis(0));
    let renewed = queue.renew(&c("c1"), batch.lease_id).unwrap();
    assert_eq!(renewed, START + LEASE_TTL);
    assert_eq!(pull(&mut queue, "c1", 1).expires_at, START + LEASE_TTL);
    queue
        .ack(&c("c1"), batch.lease_id, &RecordKey::new("k1"))
        .unwrap();
    queue.release(&c("c1"), batch.lease_id).unwrap();
    let lease = &queue.snapshot().leases[0];
    assert_eq!(lease.granted_at, START);
    assert_eq!(lease.expires_at, START + LEASE_TTL);
    assert_eq!(
        lease.status,
        LeaseStatus::Released { ended_at: START },
        "EndedAfterGrant"
    );
}
