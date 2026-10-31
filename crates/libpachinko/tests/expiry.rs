//! Lease expiry: `LeaseLapses`, `LeaseExpiryParksPocket`,
//! `LeaseExpiryKeepsPocketAtFront` and `LeaseExpiryIdlesEmptyPocket`, parking after
//! `max_consecutive_expiries`, revival, and the `ExpiryRecoversWithinTtl` guarantee.
//! The fake clock stands before, at and after each `expires_at`; nothing sleeps.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::time::Duration;

use libpachinko::{
    Batch, Clock, Config, ConsumerId, CountingLeaseIds, FakeClock, Lapsed, LeaseId, LeaseStatus,
    Payload, PocketFate, PocketKey, PocketStatus, PocketView, Publication, PublishOutcome,
    PublisherId, PullOutcome, Queue, Rank, RecordKey, RecordStatus, Snapshot, Timestamp,
};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const START: Timestamp = Timestamp::from_millis(1_000);
const LEASE_TTL: Duration = Duration::from_millis(1_000);
const ONE_MS: Duration = Duration::from_millis(1);

fn queue_with(max_consecutive_expiries: u32) -> (TestQueue, FakeClock) {
    let config = Config::new(
        LEASE_TTL,
        max_consecutive_expiries,
        10,
        Duration::from_millis(5_000),
        Duration::from_millis(7_000),
    )
    .unwrap();
    let clock = FakeClock::new(START);
    let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p")).unwrap();
    for c in ["c1", "c2"] {
        queue.consumer_joins(ConsumerId::new(c)).unwrap();
    }
    (queue, clock)
}

fn queue() -> (TestQueue, FakeClock) {
    queue_with(3)
}

fn publish(queue: &mut TestQueue, pocket: &str, key: &str, rank: i64) -> PublishOutcome {
    queue
        .publish(Publication::new(
            PublisherId::new("p"),
            PocketKey::new(pocket),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::from(format!("{key}@{rank}").as_bytes()),
        ))
        .unwrap()
}

fn pull(queue: &mut TestQueue, consumer: &str, limit: i64) -> Batch {
    match queue.pull(&ConsumerId::new(consumer), limit).unwrap() {
        PullOutcome::Batch(batch) => batch,
        other => panic!("expected a batch, got {other:?}"),
    }
}

fn pocket<'a>(snapshot: &'a Snapshot, key: &str) -> &'a PocketView {
    snapshot
        .pockets
        .iter()
        .find(|p| p.key.as_str() == key)
        .unwrap()
}

/// What a lapse pass reported, as comparable tuples.
fn summary(lapsed: &[Lapsed]) -> Vec<(LeaseId, &str, &str, PocketFate)> {
    lapsed
        .iter()
        .map(|l| {
            (
                l.lease_id,
                l.consumer.as_str(),
                l.pocket_key.as_str(),
                l.fate,
            )
        })
        .collect()
}

/// One millisecond before `t`.
fn just_before(t: Timestamp) -> Timestamp {
    Timestamp::from_millis(t.as_millis() - 1)
}

// temporal.LeaseLapses and rule-success.LeaseLapses: not before expires_at, at it, and
// not again.
#[test]
fn a_lease_lapses_at_its_expires_at_and_only_once() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    clock.set(just_before(batch.expires_at));
    assert_eq!(queue.lapse(), [], "one millisecond before");
    assert_eq!(queue.snapshot().leases[0].status, LeaseStatus::Active);
    clock.set(batch.expires_at);
    assert_eq!(
        summary(&queue.lapse()),
        [(batch.lease_id, "c1", "a", PocketFate::Queued)]
    );
    clock.advance(LEASE_TTL * 10);
    assert_eq!(queue.lapse(), [], "an expired lease does not lapse again");
}

// rule-failure.LeaseLapses.1, rule-failure.LeaseExpiryParksPocket.1,
// rule-failure.LeaseExpiryKeepsPocketAtFront.1 and
// rule-failure.LeaseExpiryIdlesEmptyPocket.1: an ended lease does not lapse.
// rule-failure.LeaseExpiryParksPocket.2, rule-failure.LeaseExpiryKeepsPocketAtFront.2
// and rule-failure.LeaseExpiryIdlesEmptyPocket.2: a lease renewed before the sweep
// reaches it has moved its expires_at, and does not lapse (the eighth open question's
// processing order).
#[test]
fn an_ended_or_renewed_lease_does_not_lapse() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    let released = pull(&mut queue, "c1", 1);
    let renewed = pull(&mut queue, "c2", 1);
    queue
        .release(&ConsumerId::new("c1"), released.lease_id)
        .unwrap();
    clock.set(renewed.expires_at + ONE_MS);
    let late = queue
        .renew(&ConsumerId::new("c2"), renewed.lease_id)
        .unwrap();
    assert_eq!(late, renewed.expires_at + ONE_MS + LEASE_TTL);
    assert_eq!(queue.lapse(), []);
    clock.set(late);
    assert_eq!(
        summary(&queue.lapse()),
        [(renewed.lease_id, "c2", "b", PocketFate::Queued)]
    );
}

// rule-success.LeaseExpiryKeepsPocketAtFront, transition-edge.Lease.active.expired,
// when-set.LeaseExpiryKeepsPocketAtFront.Lease.ended_at,
// transition-edge.Pocket.leased.queued (by expiry), and RequeueHeldRecord and
// RequeueDeferredRecord on the expiry path.
#[test]
fn a_lapse_below_the_limit_keeps_the_pocket_at_the_front_with_its_records_freed() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    publish(&mut queue, "b", "k1", 1);
    let batch = pull(&mut queue, "c1", 2);
    publish(&mut queue, "a", "k1", 5);
    queue
        .ack(
            &ConsumerId::new("c1"),
            batch.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    let before = queue.snapshot();
    clock.set(before.leases[0].expires_at);
    assert_eq!(
        summary(&queue.lapse()),
        [(batch.lease_id, "c1", "a", PocketFate::Queued)]
    );
    let after = queue.snapshot();
    let a = pocket(&after, "a");
    assert_eq!(a.status, PocketStatus::Queued);
    assert_eq!(
        a.position,
        pocket(&before, "a").position,
        "it keeps its place"
    );
    assert_eq!(a.consecutive_expiries, 1);
    assert!(a.records.iter().all(|r| r.status == RecordStatus::Queued));
    assert_eq!(
        after.pocket_order,
        [PocketKey::new("a"), PocketKey::new("b")]
    );
    assert_eq!(after.next_position, before.next_position);
    let ended_at = before.leases[0].expires_at;
    assert_eq!(after.leases[0].status, LeaseStatus::Expired { ended_at });
    let next = pull(&mut queue, "c2", 10);
    assert_eq!(
        next.pocket_key,
        PocketKey::new("a"),
        "pullable again at once"
    );
}

// rule-success.LeaseExpiryParksPocket, transition-edge.Pocket.leased.parked,
// when-set.LeaseExpiryParksPocket.Lease.ended_at, invariant.Pocket.ExpiriesWithinLimit,
// rule-failure.PullClaimsFrontPocket.3 (a parked pocket is never claimed)
#[test]
fn the_limiting_lapse_parks_the_pocket_until_a_publish_revives_it() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    for expiries in 1..=3 {
        let batch = pull(&mut queue, "c1", 1);
        clock.set(batch.expires_at);
        let fate = if expiries < 3 {
            PocketFate::Queued
        } else {
            PocketFate::Parked
        };
        assert_eq!(summary(&queue.lapse()), [(batch.lease_id, "c1", "a", fate)]);
        let snapshot = queue.snapshot();
        assert_eq!(pocket(&snapshot, "a").consecutive_expiries, expiries);
        assert!(expiries <= queue.config().max_consecutive_expiries());
        assert_eq!(
            snapshot.leases.last().unwrap().status,
            LeaseStatus::Expired {
                ended_at: batch.expires_at
            }
        );
    }
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Parked);
    assert_eq!(a.records[0].status, RecordStatus::Queued);
    assert_eq!(snapshot.pocket_order, []);
    assert_eq!(
        queue.pull(&ConsumerId::new("c2"), 1),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(publish(&mut queue, "a", "k1", 0), PublishOutcome::Stale);
    assert_eq!(pocket(&queue.snapshot(), "a").status, PocketStatus::Queued);
    assert_eq!(pocket(&queue.snapshot(), "a").consecutive_expiries, 0);
    assert_eq!(pull(&mut queue, "c2", 1).pocket_key, PocketKey::new("a"));
}

// rule-success.LeaseExpiryIdlesEmptyPocket,
// when-set.LeaseExpiryIdlesEmptyPocket.Lease.ended_at
#[test]
fn a_lapse_below_the_limit_idles_a_pocket_with_no_records() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    let batch = pull(&mut queue, "c1", 1);
    queue
        .ack(
            &ConsumerId::new("c1"),
            batch.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    let expires_at = queue.snapshot().leases[0].expires_at;
    clock.set(expires_at + Duration::from_millis(50));
    assert_eq!(
        summary(&queue.lapse()),
        [(batch.lease_id, "c1", "a", PocketFate::Idle)]
    );
    let snapshot = queue.snapshot();
    let a = pocket(&snapshot, "a");
    assert_eq!(a.status, PocketStatus::Idle);
    assert_eq!(a.consecutive_expiries, 1);
    assert_eq!(
        snapshot.leases[0].status,
        LeaseStatus::Expired {
            ended_at: expires_at + Duration::from_millis(50)
        }
    );
}

// rule-failure.LeaseExpiryParksPocket.4 (below the limit: kept or idled),
// rule-failure.LeaseExpiryKeepsPocketAtFront.4 and
// rule-failure.LeaseExpiryIdlesEmptyPocket.4 (at the limit: parked, with records or
// none), rule-failure.LeaseExpiryKeepsPocketAtFront.5 (no records: idled) and
// rule-failure.LeaseExpiryIdlesEmptyPocket.5 (records: kept at the front)
#[test]
fn the_expiry_count_and_the_record_count_pick_the_lapse_rule() {
    for (limit, records, fate) in [
        (2, true, PocketFate::Queued),
        (2, false, PocketFate::Idle),
        (1, true, PocketFate::Parked),
        (1, false, PocketFate::Parked),
    ] {
        let (mut queue, clock) = queue_with(limit);
        publish(&mut queue, "a", "k1", 1);
        let batch = pull(&mut queue, "c1", 1);
        if !records {
            queue
                .ack(
                    &ConsumerId::new("c1"),
                    batch.lease_id,
                    &RecordKey::new("k1"),
                )
                .unwrap();
        }
        clock.advance(LEASE_TTL);
        let got = queue.lapse();
        assert_eq!(
            summary(&got),
            [(batch.lease_id, "c1", "a", fate)],
            "{limit} {records}"
        );
    }
}

// rule-failure.LeaseExpiryParksPocket.3, rule-failure.LeaseExpiryKeepsPocketAtFront.3
// and rule-failure.LeaseExpiryIdlesEmptyPocket.3 (the pocket is leased) hold by the
// LeasedHasOneActiveLease invariant, which the walk in invariants.rs asserts after
// every step. Several due leases lapse in (expires_at, lease id) order.
#[test]
fn due_leases_lapse_in_expiry_then_id_order() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    let first = pull(&mut queue, "c1", 1);
    clock.advance(Duration::from_millis(10));
    let second = pull(&mut queue, "c2", 1);
    clock.advance(Duration::from_millis(5));
    queue.renew(&ConsumerId::new("c1"), first.lease_id).unwrap();
    clock.advance(LEASE_TTL * 2);
    let ids: Vec<LeaseId> = queue.lapse().into_iter().map(|l| l.lease_id).collect();
    assert_eq!(
        ids,
        [second.lease_id, first.lease_id],
        "the earlier expires_at first"
    );

    let (mut queue, clock) = self::queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "b", "k1", 1);
    let first = pull(&mut queue, "c1", 1);
    clock.advance(Duration::from_millis(10));
    let second = pull(&mut queue, "c2", 1);
    queue.renew(&ConsumerId::new("c1"), first.lease_id).unwrap();
    clock.advance(LEASE_TTL * 2);
    let ids: Vec<LeaseId> = queue.lapse().into_iter().map(|l| l.lease_id).collect();
    assert_eq!(
        ids,
        [first.lease_id, second.lease_id],
        "a tie goes by lease id"
    );
}

// The ExpiryRecoversWithinTtl guarantee: a consumer that stops loses its lease
// lease_ttl after its last action, and its records are pullable again at once.
#[test]
fn a_silent_consumer_loses_its_lease_a_ttl_after_its_last_action() {
    let (mut queue, clock) = queue();
    publish(&mut queue, "a", "k1", 1);
    publish(&mut queue, "a", "k2", 1);
    let batch = pull(&mut queue, "c1", 1);
    clock.advance(Duration::from_millis(400));
    queue
        .ack(
            &ConsumerId::new("c1"),
            batch.lease_id,
            &RecordKey::new("k1"),
        )
        .unwrap();
    let last_action = clock.now();
    clock.set(just_before(last_action + LEASE_TTL));
    assert_eq!(queue.lapse(), []);
    clock.set(last_action + LEASE_TTL);
    assert_eq!(queue.lapse().len(), 1);
    assert_eq!(queue.lease_of(&ConsumerId::new("c1")), Ok(None));
    let next = pull(&mut queue, "c2", 5);
    assert_eq!(next.records[0].record_key, RecordKey::new("k2"));
}
