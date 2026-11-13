//! Lease expiry: `LeaseLapses` and the three rules that process `LeaseLapsed`,
//! parking and `ReviveParkedPocket`, fencing after a lapse, the branch-A order of a
//! lapse and a late action in both directions, the clock's clamp, the cost of a
//! tick, and the one strict order. Every tick goes through the fixture, which
//! checks every state invariant and `ExpiriesWithinLimit` after it.

use libpachinko::clock::FakeClock;
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::records::RecordStatus;
use libpachinko::testing::PocketStatus;
use libpachinko::{
    ConsumerId, LeaseActionError, LeaseId, LeaseStatus, Payload, PocketFate, PocketKey,
    Publication, PublishError, PublishOutcome, PublisherId, PullOutcome, Queue, Rank, RecordKey,
    SettleOutcome, TickReport, Timestamp,
};

use crate::counting::{Counting, recording};
use crate::fixture::{Fixture, T0, consumer, invariants, keys, short_lease};

/// The lease every test here uses, in milliseconds.
const TTL: u64 = 10;

/// `T0` plus `millis`.
const fn at(millis: u64) -> Timestamp {
    T0.saturating_add_millis(millis)
}

/// A fixture with a 10 ms lease.
fn fixture() -> Fixture {
    Fixture::with_config(short_lease(TTL))
}

/// A fixture with a 10 ms lease and pocket `a` holding `a1` and `a2`, then pocket
/// `b` holding `b1`.
fn two_pockets() -> Fixture {
    let mut fixture = fixture();
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.publish("a", "a2", 1, "a2").unwrap();
    fixture.publish("b", "b1", 1, "b1").unwrap();
    fixture
}

impl Fixture {
    /// Moves the clock one lease on and ticks: the lease pulled at the previous
    /// instant lapses, and nothing else is due.
    fn lapse(&mut self) -> TickReport {
        self.advance(TTL);
        let report = self.tick();
        assert_eq!(report.lapsed.len(), 1, "one lease lapsed");
        report
    }

    /// `c1` claims the front pocket, which must be `pocket`, and lets the lease
    /// lapse.
    fn claim_and_lapse(&mut self, pocket: &str) -> TickReport {
        let batch = self.pull("c1", 10);
        assert_eq!(batch.pocket_key.as_str(), pocket);
        self.lapse()
    }

    /// A second queue forked from this one; the clock is shared.
    fn fork(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            clock: self.clock.clone(),
        }
    }
}

/// Pocket `a` parked by three lapses in a row, with `b` queued behind it.
fn parked() -> Fixture {
    let mut fixture = two_pockets();
    for _ in 0..3 {
        fixture.claim_and_lapse("a");
    }
    assert_eq!(fixture.pocket("a").status, PocketStatus::Parked);
    fixture
}

// scenario: S006
// obligation: rule-success.LeaseLapses
#[test]
fn a_zero_lease_ttl_lapses_at_the_instant_of_grant() {
    let mut fixture = Fixture::with_config(short_lease(0));
    fixture.publish("a", "a1", 1, "a1").unwrap();
    let batch = fixture.pull("c1", 10);
    assert_eq!(batch.expires_at, T0);
    let report = fixture.tick();
    assert_eq!(report.lapsed.len(), 1);
    assert_eq!(report.lapsed[0].id, batch.lease);
    assert_eq!(
        report.lapsed[0].status,
        LeaseStatus::Expired { ended_at: T0 }
    );
    assert_eq!(fixture.lease_view("c1"), None);
}

// scenario: S059
// obligation: temporal.LeaseLapses
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.2
// obligation: rule-failure.LeaseExpiryParksPocket.2
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.2
// obligation: transition-edge.Lease.active.expired
// obligation: when-set.LeaseExpiryKeepsPocketAtFront.Lease.ended_at
#[test]
fn a_lease_lapses_exactly_at_expires_at() {
    let mut fixture = two_pockets();
    let batch = fixture.pull("c1", 10);
    assert_eq!(batch.expires_at, at(10));
    fixture.at(9);
    let before = fixture.snapshot();
    assert_eq!(fixture.tick().lapsed.len(), 0);
    assert_eq!(
        fixture.snapshot(),
        before,
        "a tick before expiry changed the queue"
    );
    fixture.at(10);
    let report = fixture.tick();
    assert_eq!(report.lapsed.len(), 1);
    let lapsed = &report.lapsed[0];
    assert_eq!(lapsed.id, batch.lease);
    assert_eq!(lapsed.status, LeaseStatus::Expired { ended_at: at(10) });
    assert_eq!(lapsed.expires_at, at(10));
    invariants::ended_after_grant(lapsed);
}

// scenario: S065
// obligation: rule-failure.LeaseLapses.1
// obligation: rule-failure.LeaseExpiryParksPocket.1
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.1
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.1
// obligation: transition-terminal.Lease.status
#[test]
fn a_lapse_fires_once_per_lease() {
    let mut fixture = two_pockets();
    fixture.pull("c1", 10);
    fixture.lapse();
    let after = fixture.snapshot();
    for later in [10, 20, 1_000] {
        fixture.at(later);
        assert!(fixture.tick().lapsed.is_empty(), "a lease lapsed twice");
    }
    assert_eq!(fixture.snapshot(), after);
    assert_eq!(fixture.pocket("a").consecutive_expiries, 1);
}

// scenario: S058
// obligation: transition-rejected.Lease.status
#[test]
fn a_released_lease_never_lapses_afterwards() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 10).lease;
    fixture.release("c1", lease).unwrap();
    let after = fixture.snapshot();
    for later in [10, 11, 1_000] {
        fixture.at(later);
        assert!(fixture.tick().lapsed.is_empty(), "a released lease lapsed");
    }
    assert_eq!(fixture.snapshot(), after);
    assert_eq!(fixture.pocket("a").consecutive_expiries, 0);
}

// scenario: S060
// obligation: rule-success.LeaseExpiryKeepsPocketAtFront
// obligation: rule-failure.LeaseExpiryParksPocket.4
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.5
// obligation: rule-success.RequeueHeldRecord
// obligation: guarantee.ExpiryRecoversWithinTtl
#[test]
fn a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records() {
    let mut fixture = two_pockets();
    let positions: Vec<_> = ["a1", "a2"]
        .map(|key| fixture.record("a", key).position)
        .to_vec();
    let position = fixture.pocket("a").position;
    let first = fixture.pull("c1", 10);
    assert_eq!(keys(&first.records), ["a1", "a2"]);
    fixture.lapse();
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.position, pocket.consecutive_expiries),
        (PocketStatus::Queued, position, 1)
    );
    assert_eq!(fixture.order(), ["a", "b"]);
    for (key, position) in ["a1", "a2"].into_iter().zip(positions) {
        let record = fixture.record("a", key);
        assert_eq!(
            (record.status, record.position),
            (RecordStatus::Queued, position)
        );
    }
    let second = fixture.pull("c2", 10);
    assert_eq!(
        second.pocket_key.as_str(),
        "a",
        "the lapsed pocket kept the front"
    );
    assert_ne!(second.lease, first.lease);
    assert_eq!(keys(&second.records), ["a1", "a2"]);
}

// scenario: S061
// obligation: rule-success.LeaseExpiryIdlesEmptyPocket
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.5
// obligation: when-set.LeaseExpiryIdlesEmptyPocket.Lease.ended_at
#[test]
fn a_lapse_with_no_records_idles_the_pocket() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 10).lease;
    fixture.ack("c1", lease, "a1").unwrap();
    fixture.ack("c1", lease, "a2").unwrap();
    let report = fixture.lapse();
    assert!(matches!(
        report.lapsed[0].status,
        LeaseStatus::Expired { ended_at } if ended_at == at(10)
    ));
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.consecutive_expiries),
        (PocketStatus::Idle, 1)
    );
    assert_eq!(fixture.order(), ["b"]);
}

// scenario: S062
// obligation: rule-success.LeaseExpiryParksPocket
// obligation: rule-failure.LeaseExpiryKeepsPocketAtFront.4
// obligation: transition-edge.Pocket.leased.parked
// obligation: when-set.LeaseExpiryParksPocket.Lease.ended_at
// obligation: invariant.Pocket.ExpiriesWithinLimit
// obligation: rule-success.ReviveParkedPocket
// obligation: transition-edge.Pocket.parked.queued
#[test]
fn three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it() {
    let mut fixture = two_pockets();
    for count in 1..=2 {
        fixture.claim_and_lapse("a");
        let pocket = fixture.pocket("a");
        assert_eq!(
            (pocket.status, pocket.consecutive_expiries),
            (PocketStatus::Queued, count)
        );
    }
    let report = fixture.claim_and_lapse("a");
    assert!(matches!(
        report.lapsed[0].status,
        LeaseStatus::Expired { .. }
    ));
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.consecutive_expiries),
        (PocketStatus::Parked, 3)
    );
    assert_eq!(fixture.record("a", "a1").status, RecordStatus::Queued);
    assert_eq!(fixture.pull("c2", 10).pocket_key.as_str(), "b");
    let next = fixture.snapshot().next_position;
    assert_eq!(
        fixture.publish("a", "a3", 1, "a3"),
        Ok(PublishOutcome::NewKey)
    );
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.position, pocket.consecutive_expiries),
        (PocketStatus::Queued, next.next(), 0),
        "revived at the back, after a3's own position, with the count reset"
    );
    fixture.check();
}

// scenario: S024 (open question 11: the stale half, as the rules stand)
// obligation: rule-failure.ReviveParkedPocket.1
#[test]
fn a_supersede_or_a_stale_publish_revives_a_parked_pocket() {
    let fixture = parked();
    for (rank, outcome) in [(2, PublishOutcome::Superseded), (0, PublishOutcome::Stale)] {
        let mut fork = fixture.fork();
        let next = fork.snapshot().next_position;
        assert_eq!(fork.publish("a", "a1", rank, "x"), Ok(outcome));
        let pocket = fork.pocket("a");
        assert_eq!(
            (pocket.status, pocket.position, pocket.consecutive_expiries),
            (PocketStatus::Queued, next, 0),
            "{outcome:?} revives at the back"
        );
        assert_eq!(fork.order(), ["b", "a"]);
        fork.check();
    }
    // A publish into a pocket that does not exist creates it; nothing is revived.
    let mut fork = fixture.fork();
    assert_eq!(fork.publish("z", "z1", 1, "z"), Ok(PublishOutcome::Created));
    assert_eq!(fork.pocket("a").status, PocketStatus::Parked);
}

// scenario: S025
// obligation: rule-failure.Supersede.3
// obligation: rule-failure.DropStalePublish.3
#[test]
fn an_equal_rank_publish_does_not_revive_a_parked_pocket() {
    let mut fixture = parked();
    let before = fixture.snapshot();
    assert!(matches!(
        fixture.publish("a", "a1", 1, "x"),
        Err(PublishError::EqualRank { .. })
    ));
    assert_eq!(fixture.snapshot(), before);
    assert_eq!(fixture.pocket("a").status, PocketStatus::Parked);
}

// scenario: S026 (open question 7: only a publish revives a parked pocket, as the
// rules stand; an operator action or a cooldown would be a new rule)
// obligation: rule-failure.ReviveParkedPocket.2
#[test]
fn only_a_publish_revives_a_parked_pocket() {
    let mut fixture = parked();
    let lease = fixture.pull("c2", 10).lease;
    assert_eq!(fixture.pocket("b").status, PocketStatus::Leased);
    // A publish into a pocket that is not parked leaves it where it is.
    assert_eq!(
        fixture.publish("b", "b2", 1, "b2"),
        Ok(PublishOutcome::NewKey)
    );
    assert_eq!(fixture.pocket("b").status, PocketStatus::Leased);
    fixture.release("c2", lease).unwrap();
    for later in [1_000, 1_000_000] {
        fixture.at(later);
        fixture.tick();
    }
    let lease = fixture.pull("c3", 10).lease;
    fixture.release("c3", lease).unwrap();
    assert_eq!(fixture.pull("c1", 10).pocket_key.as_str(), "b");
    assert_eq!(
        fixture.try_pull("c2", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    let pocket = fixture.pocket("a");
    assert_eq!(
        (pocket.status, pocket.consecutive_expiries),
        (PocketStatus::Parked, 3)
    );
}

// scenario: S034
// obligation: rule-failure.PullClaimsFrontPocket.2
#[test]
fn idle_parked_and_leased_pockets_are_never_claimed() {
    let mut fixture = parked();
    let lease = fixture.pull("c1", 10).lease;
    fixture.ack("c1", lease, "b1").unwrap();
    fixture.release("c1", lease).unwrap();
    assert_eq!(fixture.pocket("b").status, PocketStatus::Idle);
    fixture.publish("c", "c1", 1, "c1").unwrap();
    fixture.publish("d", "d1", 1, "d1").unwrap();
    assert_eq!(fixture.pull("c2", 10).pocket_key.as_str(), "c");
    // Now `a` is parked, `b` idle and `c` leased: only `d` is claimable.
    assert_eq!(fixture.pull("c3", 10).pocket_key.as_str(), "d");
    fixture.queue.consumer_joins(consumer("c4")).unwrap();
    assert_eq!(
        fixture.try_pull("c4", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    fixture.check();
}

// scenario: S048
// obligation: rule-success.RequeueDeferredRecord
#[test]
fn a_supersede_while_held_is_deferred_on_nack_and_the_lapse_frees_it() {
    let mut fixture = two_pockets();
    let lease = fixture.pull("c1", 10).lease;
    fixture.publish("a", "a1", 2, "newer").unwrap();
    assert_eq!(fixture.nack("c1", lease, "a1"), Ok(SettleOutcome::Deferred));
    let deferred = fixture.record("a", "a1");
    assert_eq!(deferred.status, RecordStatus::Deferred);
    fixture.lapse();
    let record = fixture.record("a", "a1");
    assert_eq!(
        (record.status, record.position),
        (RecordStatus::Queued, deferred.position),
        "queued again at its deferred position"
    );
    let batch = fixture.pull("c2", 10);
    assert_eq!(batch.pocket_key.as_str(), "a");
    assert_eq!(keys(&batch.records), ["a2", "a1"]);
    assert_eq!(batch.records[1].rank, Rank::new(2));
    assert_eq!(batch.records[1].payload, Payload::from(&b"newer"[..]));
}

/// Pocket `a` with three records, lapsed once.
fn lapsed_once() -> Fixture {
    let mut fixture = two_pockets();
    fixture.publish("a", "a3", 1, "a3").unwrap();
    fixture.claim_and_lapse("a");
    fixture
}

// scenario: S064
// obligation: invariant.Pocket.ExpiriesWithinLimit
#[test]
fn a_pull_a_nack_or_a_renew_under_a_lease_does_not_reset_the_expiry_count() {
    let mut fixture = lapsed_once();
    let count = |fixture: &Fixture| fixture.pocket("a").consecutive_expiries;
    assert_eq!(count(&fixture), 1);
    let lease = fixture.pull("c1", 10).lease;
    assert_eq!(count(&fixture), 1, "after the pull");
    assert_eq!(fixture.nack("c1", lease, "a1"), Ok(SettleOutcome::Settled));
    assert_eq!(count(&fixture), 1, "after the nack");
    fixture.renew("c1", lease).unwrap();
    assert_eq!(count(&fixture), 1, "after the renew");
    let again = fixture.pull("c1", 10).lease;
    assert_eq!(again, lease, "a pull within the lease");
    assert_eq!(count(&fixture), 1, "after that pull");
    fixture.lapse();
    assert_eq!(count(&fixture), 2);
}

// scenario: S077
// obligation: rule-success.AckSettlesRecord
// obligation: rule-success.AckDefersNewerVersion
// obligation: invariant.Pocket.ExpiriesWithinLimit
#[test]
fn an_ack_resets_the_consecutive_expiry_count() {
    let mut fixture = lapsed_once();
    let lease = fixture.pull("c1", 10).lease;
    let count = |fixture: &Fixture| {
        let pocket = fixture.pocket("a");
        (pocket.status, pocket.consecutive_expiries)
    };
    assert_eq!(count(&fixture), (PocketStatus::Leased, 1));
    let not_held = LeaseActionError::RecordNotHeld(RecordKey::new("b1"));
    fixture.refused(&not_held, |f| f.ack("c1", lease, "b1"));
    assert_eq!(count(&fixture), (PocketStatus::Leased, 1), "a refused ack");
    assert_eq!(fixture.ack("c1", lease, "a1"), Ok(SettleOutcome::Settled));
    assert_eq!(count(&fixture), (PocketStatus::Leased, 0), "a settling ack");
    fixture.lapse();
    assert_eq!(count(&fixture), (PocketStatus::Queued, 1));
    assert_eq!(fixture.order(), ["a", "b"], "kept at the front");
    // The deferring ack resets the count too.
    let lease = fixture.pull("c1", 10).lease;
    assert_eq!(
        fixture.publish("a", "a2", 2, "newer"),
        Ok(PublishOutcome::Superseded)
    );
    assert_eq!(fixture.ack("c1", lease, "a2"), Ok(SettleOutcome::Deferred));
    assert_eq!(
        count(&fixture),
        (PocketStatus::Leased, 0),
        "a deferring ack"
    );
    // From there it parks on the third lapse, not the second.
    fixture.lapse();
    assert_eq!(count(&fixture), (PocketStatus::Queued, 1));
    fixture.claim_and_lapse("a");
    assert_eq!(count(&fixture), (PocketStatus::Queued, 2));
    fixture.claim_and_lapse("a");
    assert_eq!(count(&fixture), (PocketStatus::Parked, 3));
    fixture.check();
}

// scenario: S056
// obligation: rule-success.ReleaseRequeuesPocket
#[test]
fn a_release_resets_the_consecutive_expiry_count() {
    let mut fixture = lapsed_once();
    fixture.claim_and_lapse("a");
    assert_eq!(fixture.pocket("a").consecutive_expiries, 2);
    let lease = fixture.pull("c1", 10).lease;
    let (fate, _) = fixture.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Queued);
    assert_eq!(fixture.pocket("a").consecutive_expiries, 0);
    // `a` went to the back on release: `b` is claimed first, and released.
    let lease = fixture.pull("c1", 10).lease;
    fixture.release("c1", lease).unwrap();
    for count in 1..=2 {
        fixture.claim_and_lapse("a");
        let pocket = fixture.pocket("a");
        assert_eq!(
            (pocket.status, pocket.consecutive_expiries),
            (PocketStatus::Queued, count)
        );
    }
    fixture.claim_and_lapse("a");
    assert_eq!(fixture.pocket("a").status, PocketStatus::Parked);
}

// scenario: S063
// obligation: rule-failure.LeaseExpiryIdlesEmptyPocket.4
#[test]
fn at_the_threshold_an_empty_pocket_parks_rather_than_idling() {
    let mut fixture = two_pockets();
    for _ in 0..2 {
        fixture.claim_and_lapse("a");
    }
    let lease = fixture.pull("c1", 10).lease;
    fixture.nack("c1", lease, "a1").unwrap();
    fixture.nack("c1", lease, "a2").unwrap();
    fixture.lapse();
    let pocket = fixture.pocket("a");
    assert_eq!(
        (
            pocket.status,
            pocket.consecutive_expiries,
            pocket.records.len()
        ),
        (PocketStatus::Parked, 3, 0)
    );
}

/// The branch-A fork: `k` published and pulled at `T0` under a 10 ms lease by
/// `c1`, the queue forked, and the clock set to 11, past `expires_at` but before
/// any tick. The first queue is ticked before the action, the second after it.
fn late() -> (Fixture, Fixture, LeaseId) {
    let mut fixture = fixture();
    fixture.publish("a", "k", 1, "k").unwrap();
    fixture.publish("b", "b1", 1, "b1").unwrap();
    let lease = fixture.pull("c1", 10).lease;
    let fork = fixture.fork();
    fixture.at(11);
    (fixture, fork, lease)
}

/// Ticks `first`, which lapses `lease` and keeps `a` at the front with `k`
/// requeued, then checks that `action` naming the old lease is refused.
fn tick_then_refused<T: std::fmt::Debug>(
    first: &mut Fixture,
    lease: LeaseId,
    action: impl FnOnce(&mut Fixture) -> Result<T, LeaseActionError>,
) {
    let report = first.tick();
    assert_eq!(report.lapsed[0].id, lease);
    assert_eq!(first.pocket("a").status, PocketStatus::Queued);
    assert_eq!(first.order(), ["a", "b"]);
    assert_eq!(first.record("a", "k").status, RecordStatus::Queued);
    first.refused(&LeaseActionError::LeaseNotActive(lease), action);
}

/// Checks that `second`'s lease is active, expiring at 21, with `a` leased, and
/// that a tick now lapses nothing.
fn extended_and_survives(second: &mut Fixture, lease: LeaseId) {
    let view = second.lease_view("c1").unwrap();
    assert_eq!(view.expires_at, at(21));
    assert_eq!(second.snapshot().leases[0].id, lease);
    assert_eq!(second.pocket("a").status, PocketStatus::Leased);
    assert_eq!(second.tick().lapsed.len(), 0);
    assert_eq!(second.pocket("a").status, PocketStatus::Leased);
}

// scenario: S066
// (the renew half; a_release_after_expires_at_but_before_the_lapse_succeeds is the release half)
// obligation: guarantee.ExpiryRecoversWithinTtl
#[test]
fn a_renew_after_expires_at_but_before_the_lapse_succeeds() {
    let (mut first, mut second, lease) = late();
    tick_then_refused(&mut first, lease, |f| f.renew("c1", lease));
    assert_eq!(second.renew("c1", lease), Ok(at(21)));
    extended_and_survives(&mut second, lease);
}

// S066's release half; the tag is on a_renew_after_expires_at_but_before_the_lapse_succeeds
// obligation: transition-edge.Lease.active.released
#[test]
fn a_release_after_expires_at_but_before_the_lapse_succeeds() {
    let (mut first, mut second, lease) = late();
    tick_then_refused(&mut first, lease, |f| f.release("c1", lease));
    let next = second.snapshot().next_position;
    let (fate, ended) = second.release("c1", lease).unwrap();
    assert_eq!(fate, PocketFate::Queued);
    assert_eq!(ended.status, LeaseStatus::Released { ended_at: at(11) });
    assert_eq!(second.snapshot().leases.len(), 0);
    assert_eq!(second.tick().lapsed.len(), 0);
    assert_eq!(second.pocket("a").position, next, "a fresh position");
    assert_eq!(second.order(), ["b", "a"]);
}

// scenario: S067
// (the ack half; a_pull_after_expires_at_but_before_the_lapse_extends_the_lease is the pull half)
// obligation: rule-success.AckSettlesRecord
#[test]
fn an_ack_after_expires_at_but_before_the_lapse_succeeds() {
    let (mut first, mut second, lease) = late();
    tick_then_refused(&mut first, lease, |f| f.ack("c1", lease, "k"));
    assert_eq!(second.ack("c1", lease, "k"), Ok(SettleOutcome::Settled));
    extended_and_survives(&mut second, lease);
}

// S067's pull half; the tag is on an_ack_after_expires_at_but_before_the_lapse_succeeds
// obligation: rule-success.PullWithinLease
#[test]
fn a_pull_after_expires_at_but_before_the_lapse_extends_the_lease() {
    let (mut first, mut second, lease) = late();
    first.tick();
    let again = first.pull("c1", 10);
    assert_eq!(again.pocket_key.as_str(), "a");
    assert_ne!(again.lease, lease, "a new lease");
    assert_eq!(
        keys(&again.records),
        ["k"],
        "the record is handed out again"
    );
    first.check();
    let within = second.pull("c1", 10);
    assert_eq!(within.lease, lease);
    assert!(within.records.is_empty(), "the record is still held");
    assert_eq!(within.expires_at, at(21));
    extended_and_survives(&mut second, lease);
}

// scenario: S069
// obligation: rule-failure.Renew.2
#[test]
fn after_a_lapse_the_old_lease_is_fenced_even_when_the_key_has_a_newer_rank() {
    let mut fixture = two_pockets();
    let old = fixture.pull("c1", 10).lease;
    fixture.lapse();
    fixture.publish("a", "a1", 2, "newer").unwrap();
    let new = fixture.pull("c2", 10);
    assert_eq!(new.pocket_key.as_str(), "a");
    assert_ne!(new.lease, old);
    let fenced = LeaseActionError::LeaseNotActive(old);
    fixture.refused(&fenced, |f| f.ack("c1", old, "a1"));
    fixture.refused(&fenced, |f| f.renew("c1", old));
    fixture.refused(&fenced, |f| f.release("c1", old));
}

#[test]
fn a_backward_clock_step_never_lapses_a_lease_early_nor_stamps_backward() {
    let mut fixture = fixture();
    fixture.publish("a", "a1", 1, "a1").unwrap();
    fixture.at(100);
    let lease = fixture.pull("c1", 10).lease;
    fixture.at(0);
    assert_eq!(fixture.tick().lapsed.len(), 0);
    assert_eq!(fixture.renew("c1", lease), Ok(at(100 + TTL)));
    fixture.publish("a", "a1", 2, "newer").unwrap();
    assert_eq!(fixture.snapshot().superseded[0].shed_at, at(100));
    fixture.at(109);
    assert_eq!(fixture.tick().lapsed.len(), 0);
    fixture.at(110);
    assert_eq!(fixture.tick().lapsed.len(), 1);
}

// A tick costs what is due: three due leases among many active, and a shed
// backlog none of which is due.
#[test]
fn a_tick_examines_only_what_is_due() {
    const POCKETS: usize = 200;
    const SHED: usize = 500;
    let clock = FakeClock::new(T0);
    let config = short_lease(TTL);
    let mut queue: Queue<Counting, _, _> =
        Queue::new(config, clock.clone(), CountingLeaseIds::default()).unwrap();
    let publisher = PublisherId::new("p");
    queue.publisher_joins(publisher.clone()).unwrap();
    let publish = |queue: &mut Queue<Counting, _, _>, pocket: usize, rank| {
        let publication = Publication::new(
            PocketKey::new(format!("p{pocket:03}")),
            RecordKey::new(format!("r{pocket:03}")),
            Rank::new(rank),
            Payload::default(),
        );
        queue.publish(&publisher, publication).unwrap();
    };
    for pocket in 0..POCKETS {
        publish(&mut queue, pocket, 1);
    }
    for rank in 2..SHED + 2 {
        publish(&mut queue, POCKETS - 1, rank.try_into().unwrap());
    }
    for consumer in 0..POCKETS {
        if consumer == 3 {
            clock.set(at(5));
        }
        let id = ConsumerId::new(format!("c{consumer}"));
        queue.consumer_joins(id.clone()).unwrap();
        queue.pull(&id, 10).unwrap();
    }
    clock.set(at(TTL));
    let ((report, cost), calls) = recording(|| queue.tick_with_cost());
    assert_eq!(report.lapsed.len(), 3);
    assert_eq!(
        (report.decayed_superseded, report.decayed_dead_letters),
        (0, 0)
    );
    assert_eq!(
        cost.leases_examined, 4,
        "the three due and the one that stopped the walk"
    );
    assert_eq!(
        cost.shed_examined, 1,
        "the superseded backlog's one stopper; the dead letters are empty"
    );
    let due = ["r000", "r001", "r002"];
    let freed = calls
        .iter()
        .filter(|(step, _)| *step == "free_held")
        .count();
    assert_eq!(freed, 3);
    for (step, keys) in &calls {
        assert!(
            keys.len() == 1 && due.contains(&keys[0].as_str()),
            "{step} ran on a pocket that was not due: {keys:?}"
        );
    }
}

// scenario: S075
// obligation: invariant.UniquePocketPosition
// obligation: invariant.UniqueRecordPosition
// obligation: invariant.PocketPositionsBelowNext
// obligation: invariant.RecordPositionsBelowNext
#[test]
fn pockets_and_records_form_one_strict_deterministic_order() {
    let mut fixture = two_pockets();
    let step = |fixture: &mut Fixture| invariants::one_strict_order(&fixture.snapshot());
    let lease = fixture.pull("c1", 1).lease;
    step(&mut fixture);
    fixture.ack("c1", lease, "a1").unwrap();
    fixture.publish("a", "a1", 1, "again").unwrap();
    let batch = fixture.pull("c1", 10);
    step(&mut fixture);
    fixture.publish("a", "a2", 2, "newer").unwrap();
    assert_eq!(keys(&batch.records), ["a2", "a1"]);
    fixture.nack("c1", lease, "a2").unwrap();
    step(&mut fixture);
    fixture.release("c1", lease).unwrap();
    step(&mut fixture);
    // `b` is now the front; it lapses, then `a` lapses three times and parks.
    fixture.claim_and_lapse("b");
    step(&mut fixture);
    for _ in 0..3 {
        let front = fixture.order()[0].clone();
        fixture.claim_and_lapse(&front);
        step(&mut fixture);
    }
    fixture.publish("a", "a9", 1, "revive").unwrap();
    fixture.publish("b", "b1", 2, "revive").unwrap();
    step(&mut fixture);
    let mut sorted = fixture.snapshot().pocket_order;
    sorted.sort();
    assert_eq!(fixture.snapshot().pocket_order, sorted);
    fixture.check();
}
