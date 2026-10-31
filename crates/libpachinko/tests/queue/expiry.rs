//! Lease expiry: `LeaseLapses` and the three rules that process `LeaseLapsed`,
//! driven by `tick` under the fake clock.

use std::time::Duration;

use libpachinko::queue::{LeaseStatus, PocketStatus};
use libpachinko::records::RecordStatus;

use crate::fixture::{Fixture, LEASE_TTL, T0, at, config, keys};

const ONE_MS: Duration = Duration::from_millis(1);

/// A fixture whose consumer `c1` holds `a` of pocket `pk`, with `b` queued behind
/// it and `later` queued after `pk`.
fn leased(max_consecutive_expiries: u32) -> (Fixture, libpachinko::queue::LeaseId) {
    let mut config = config();
    config.max_consecutive_expiries = max_consecutive_expiries;
    let mut fixture = Fixture::with_config(config);
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.publish("pk", "b", 1, "b").unwrap();
    fixture.publish("later", "x", 1, "x").unwrap();
    let batch = fixture.pull("c1", 1);
    (fixture, batch.lease)
}

// obligation: rule-success.LeaseLapses
// obligation: temporal.LeaseLapses
#[test]
fn a_lease_lapses_at_its_expiry_not_before_and_only_once() {
    let (mut fixture, lease) = leased(3);
    fixture.advance(LEASE_TTL.checked_sub(ONE_MS).unwrap());
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
    fixture.advance(ONE_MS);
    let report = fixture.queue.tick();
    assert_eq!(report.lapsed.len(), 1);
    assert_eq!(report.lapsed[0].id, lease);
    fixture.advance(LEASE_TTL);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
}

// obligation: rule-failure.LeaseLapses.1
// obligation: rule-failure.LeaseExpiryParksPocket.1
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.1
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.1
#[test]
fn a_released_lease_never_lapses() {
    let (mut fixture, lease) = leased(1);
    fixture.queue.release("c1", lease).unwrap();
    let before = fixture.snapshot();
    fixture.advance(LEASE_TTL * 2);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    assert_eq!(fixture.snapshot(), before);
}

// obligation: rule-failure.LeaseExpiryParksPocket.2
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.2
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.2
// A renewal, ack, nack or pull processed before the lapse moves `expires_at`, and
// the lapse stops at its guard.
#[test]
fn an_action_processed_first_moves_the_expiry_past_now() {
    let (mut fixture, lease) = leased(1);
    fixture.advance(LEASE_TTL.checked_sub(ONE_MS).unwrap());
    fixture.queue.renew("c1", lease).unwrap();
    fixture.advance(ONE_MS);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    fixture.advance(LEASE_TTL.checked_sub(ONE_MS * 2).unwrap());
    fixture.queue.ack("c1", lease, "a").unwrap();
    fixture.advance(ONE_MS * 2);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    fixture.advance(LEASE_TTL.checked_sub(ONE_MS * 3).unwrap());
    fixture.pull("c1", 0);
    fixture.advance(ONE_MS * 3);
    assert_eq!(fixture.queue.tick().lapsed.len(), 0);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
}

// obligation: rule-success.LeaseExpiryParksPocket
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.4
// obligation: transition-edge.Pocket.leased.parked
// obligation: transition-edge.Lease.active.expired
// obligation: when-set.LeaseExpiryParksPocket.Lease.ended_at
// obligation: invariant.Pocket.ExpiriesWithinLimit
#[test]
fn the_lapse_that_reaches_the_limit_parks_the_pocket_and_frees_its_records() {
    let (mut fixture, _) = leased(3);
    for lapse in 1..=3 {
        fixture.advance(LEASE_TTL);
        let report = fixture.queue.tick();
        assert_eq!(report.lapsed.len(), 1);
        let ended = &report.lapsed[0];
        assert_eq!(ended.status, LeaseStatus::Expired);
        assert_eq!(ended.ended_at, Some(fixture.now()));
        assert!(ended.ended_at >= Some(ended.granted_at));
        let pocket = fixture.pocket("pk");
        assert_eq!(pocket.consecutive_expiries, lapse);
        assert!(pocket.consecutive_expiries <= 3);
        if lapse < 3 {
            assert_eq!(pocket.status, PocketStatus::Queued);
            assert_eq!(fixture.pull("c1", 1).pocket_key, "pk");
        } else {
            assert_eq!(pocket.status, PocketStatus::Parked);
        }
    }
    let a = fixture.record("pk", "a").unwrap();
    assert_eq!((a.status, a.held), (RecordStatus::Queued, None));
    assert_eq!(fixture.active_lease("c1"), None);
    assert_eq!(fixture.pull("c2", 10).pocket_key, "later");
}

// The parking rule has no records guard: an empty pocket parks at the limit.
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.4
#[test]
fn an_empty_pocket_parks_at_the_limit() {
    let (mut fixture, lease) = leased(1);
    fixture.queue.ack("c1", lease, "a").unwrap();
    let rest = fixture.pull("c1", 10);
    fixture.queue.ack("c1", rest.lease, "b").unwrap();
    fixture.advance(LEASE_TTL);
    fixture.queue.tick();
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Parked);
}

// obligation: rule-success.LeaseExpiryKeepsPocketAtFront
// obligation: rule-failure.LeaseExpiryParksPocket.4
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.5
// obligation: when-set.LeaseExpiryKeepsPocketAtFront.Lease.ended_at
#[test]
fn a_lapse_below_the_limit_keeps_the_pocket_at_the_front() {
    let (mut fixture, lease) = leased(3);
    let position = fixture.pocket("pk").position;
    fixture.publish("pk", "a", 2, "a2").unwrap();
    fixture.queue.ack("c1", lease, "a").unwrap();
    fixture.pull("c1", 1);
    fixture.advance(LEASE_TTL);
    let report = fixture.queue.tick();
    assert_eq!(report.lapsed[0].ended_at, Some(fixture.now()));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Queued);
    assert_eq!(pocket.position, position);
    assert_eq!(pocket.consecutive_expiries, 1);
    assert!(
        pocket
            .records
            .iter()
            .all(|record| record.status == RecordStatus::Queued && record.held.is_none())
    );
    let again = fixture.pull("c2", 10);
    assert_eq!(again.pocket_key, "pk");
    assert_eq!(keys(&again), ["b", "a"]);
}

// obligation: rule-success.LeaseExpiryIdlesEmptyPocket
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.5
// obligation: when-set.LeaseExpiryIdlesEmptyPocket.Lease.ended_at
#[test]
fn a_lapse_below_the_limit_idles_an_empty_pocket() {
    let (mut fixture, lease) = leased(3);
    fixture.queue.ack("c1", lease, "a").unwrap();
    fixture.pull("c1", 10);
    fixture.queue.ack("c1", lease, "b").unwrap();
    fixture.advance(LEASE_TTL);
    let report = fixture.queue.tick();
    assert_eq!(report.lapsed[0].status, LeaseStatus::Expired);
    assert_eq!(report.lapsed[0].ended_at, Some(fixture.now()));
    let pocket = fixture.pocket("pk");
    assert_eq!(pocket.status, PocketStatus::Idle);
    assert_eq!(pocket.consecutive_expiries, 1);
    assert_eq!(pocket.records.len(), 0);
}

// One tick processes every lease that has lapsed, earliest first.
#[test]
fn one_tick_lapses_every_due_lease_in_expiry_order() {
    let (mut fixture, first) = leased(3);
    fixture.advance(ONE_MS);
    let second = fixture.pull("c2", 1).lease;
    fixture.advance(LEASE_TTL * 2);
    let report = fixture.queue.tick();
    let ids: Vec<_> = report.lapsed.iter().map(|lease| lease.id).collect();
    assert_eq!(ids, [first, second]);
    assert_eq!(report.lapsed[0].expires_at, at(T0, LEASE_TTL));
}

// A zero TTL admits a lease that lapses at the next tick.
#[test]
fn a_zero_lease_ttl_lapses_at_the_next_tick() {
    let mut config = config();
    config.lease_ttl = Duration::ZERO;
    let mut fixture = Fixture::with_config(config);
    fixture.publish("pk", "a", 1, "a").unwrap();
    let batch = fixture.pull("c1", 10);
    assert_eq!(batch.expires_at, T0);
    assert_eq!(fixture.queue.tick().lapsed.len(), 1);
}
