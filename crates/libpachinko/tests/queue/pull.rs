//! Pull: `PullClaimsFrontPocket`, `PullWithinLease`, `HoldRecord` and
//! `PullFindsNoPocket`, the lease view, and the lease invariants.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use libpachinko::clock::FakeClock;
use libpachinko::lease_ids::LeaseIds;
use libpachinko::records::{MemoryRecords, RecordStatus};
use libpachinko::testing::PocketStatus;
use libpachinko::{
    LeaseId, LeaseIdError, LeaseStatus, Payload, PocketKey, Publication, PullError, PullOutcome,
    Queue, Rank, RecordKey,
};

use crate::fixture::{Fixture, LEASE_TTL, T0, config, consumer, invariants, keys, publisher};

/// A fixture with `count` records `r0`, `r1`, ... published into pocket `pk`.
fn pocket_of(count: usize) -> Fixture {
    let mut fixture = Fixture::new();
    for index in 0..count {
        let key = format!("r{index}");
        fixture.publish("pk", &key, 1, &key).unwrap();
    }
    fixture
}

// scenario: S027
// obligation: rule-success.PullClaimsFrontPocket
// obligation: rule-entity-creation.PullClaimsFrontPocket.1
// obligation: rule-success.HoldRecord
// obligation: entity-fields.Lease
// obligation: transition-edge.Pocket.queued.leased
// obligation: transition-edge.Record.queued.held
// obligation: when-set.HoldRecord.Record.held_rank
// obligation: when-set.HoldRecord.Record.held_payload
// obligation: when-presence.Record.held_rank
// obligation: when-presence.Record.held_payload
// obligation: projection.Pocket.held_records
// obligation: entity-relationship.Consumer.leases
// obligation: projection.Consumer.active_leases
// obligation: entity-relationship.Pocket.leases
// obligation: projection.Pocket.active_leases
#[test]
fn a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch() {
    let mut fixture = pocket_of(2);
    fixture.advance(5);
    let batch = fixture.pull("c1", 10);
    let now = T0.saturating_add_millis(5);
    assert_eq!(batch.lease, LeaseId::from_u128(1));
    assert_eq!(batch.pocket_key.as_str(), "pk");
    assert_eq!(batch.expires_at, now + LEASE_TTL);
    assert_eq!(keys(&batch.records), ["r0", "r1"]);
    assert_eq!(batch.records[0].payload, Payload::from(b"r0".as_slice()));
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
    assert_eq!(fixture.order().len(), 0);
    let record = fixture.record("pk", "r0");
    assert_eq!(
        record.status,
        RecordStatus::Held {
            rank: Rank::new(1),
            payload: Payload::from(b"r0".as_slice())
        }
    );
    let snapshot = fixture.snapshot();
    let [lease] = snapshot.leases.as_slice() else {
        panic!("one lease")
    };
    assert_eq!(
        (
            lease.id,
            lease.consumer_id.as_str(),
            lease.pocket_key.as_str()
        ),
        (batch.lease, "c1", "pk")
    );
    assert_eq!(
        (lease.status, lease.granted_at, lease.expires_at),
        (LeaseStatus::Active, now, now + LEASE_TTL)
    );
    assert!(
        snapshot
            .consumers
            .iter()
            .any(|(id, l)| id.as_str() == "c1" && *l == Some(batch.lease))
    );
}

// scenario: S028
// obligation: guarantee.OldestFirst
#[test]
fn a_positive_limit_returns_at_most_that_many_oldest_first_and_later_pulls_continue() {
    let mut fixture = pocket_of(5);
    assert_eq!(keys(&fixture.pull("c1", 2).records), ["r0", "r1"]);
    assert_eq!(keys(&fixture.pull("c1", 2).records), ["r2", "r3"]);
    assert_eq!(keys(&fixture.pull("c1", 2).records), ["r4"]);
}

// scenario: S029
// obligation: guarantee.OldestFirst
#[test]
fn a_limit_above_max_pull_batch_is_capped() {
    let mut config = config();
    config.max_pull_batch = 3;
    let mut fixture = Fixture::with_config(config);
    for index in 0..5 {
        let key = format!("r{index}");
        fixture.publish("pk", &key, 1, &key).unwrap();
    }
    assert_eq!(keys(&fixture.pull("c1", 1_000).records), ["r0", "r1", "r2"]);
}

// scenario: S030
#[test]
fn a_limit_above_the_queued_count_returns_every_queued_record() {
    let mut fixture = pocket_of(3);
    assert_eq!(keys(&fixture.pull("c1", 50).records), ["r0", "r1", "r2"]);
    assert_eq!(fixture.pull("c1", 50).records.len(), 0);
}

// scenario: S031
#[test]
fn a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch() {
    let mut fixture = pocket_of(2);
    let batch = fixture.pull("c1", 0);
    assert_eq!(batch.records.len(), 0);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
    assert_eq!(fixture.record("pk", "r0").status, RecordStatus::Queued);
    assert_eq!(fixture.snapshot().leases.len(), 1);
}

// scenario: S032
#[test]
fn a_negative_limit_behaves_like_zero() {
    let mut fixture = pocket_of(2);
    let batch = fixture.pull("c1", -1);
    assert_eq!(batch.records.len(), 0);
    assert_eq!(fixture.pocket("pk").status, PocketStatus::Leased);
    assert_eq!(fixture.pull("c1", i64::MIN).records.len(), 0);
    assert_eq!(keys(&fixture.pull("c1", 10).records), ["r0", "r1"]);
}

// scenario: S033
// obligation: rule-success.PullFindsNoPocket
// obligation: rule-failure.PullClaimsFrontPocket.2
// obligation: rule-failure.PullFindsNoPocket.1
#[test]
fn a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error() {
    let mut fixture = Fixture::new();
    assert_eq!(
        fixture.try_pull("c1", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(fixture.snapshot().leases.len(), 0);
    fixture.publish("pk", "a", 1, "a").unwrap();
    fixture.pull("c1", 10);
    assert_eq!(
        fixture.try_pull("c2", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(fixture.lease_view("c2"), None);
}

// scenario: S036
// obligation: rule-success.PullWithinLease
// obligation: rule-failure.PullClaimsFrontPocket.1
// obligation: rule-failure.PullWithinLease.1
// obligation: surface-exposure.Consuming
#[test]
fn a_pull_under_a_lease_extends_the_lease() {
    let mut fixture = pocket_of(3);
    assert_eq!(fixture.lease_view("c1"), None);
    let first = fixture.pull("c1", 1);
    let before = fixture.lease_view("c1").unwrap();
    assert_eq!(before.expires_at, T0 + LEASE_TTL);
    assert_eq!(before.pocket_key.as_str(), "pk");
    assert_eq!(keys(&before.records), ["r0"]);
    fixture.advance(400);
    let second = fixture.pull("c1", 1);
    let after = fixture.lease_view("c1").unwrap();
    let extended = T0.saturating_add_millis(400) + LEASE_TTL;
    assert_eq!(after.expires_at, extended);
    assert_eq!(second.expires_at, extended);
    assert_eq!(second.lease, first.lease);
    assert_eq!(keys(&after.records), ["r0", "r1"]);
    let lease = &fixture.snapshot().leases[0];
    assert_eq!((lease.granted_at, lease.expires_at), (T0, extended));
}

// scenario: S037
// obligation: rule-failure.HoldRecord.2
// obligation: rule-failure.HoldRecord.1
#[test]
fn a_held_record_is_not_handed_out_again_under_the_same_lease() {
    let mut fixture = pocket_of(2);
    let first = fixture.pull("c1", 1);
    fixture.publish("pk", "r0", 2, "r0-v2").unwrap();
    let second = fixture.pull("c1", 10);
    assert_eq!(second.lease, first.lease);
    assert_eq!(keys(&second.records), ["r1"]);
    assert_eq!(fixture.pull("c1", 10).records.len(), 0);
    let view = fixture.lease_view("c1").unwrap();
    assert_eq!(view.records[0].rank, Rank::new(1));
}

// scenario: S038
// obligation: guarantee.OldestFirst
#[test]
fn pockets_are_claimed_in_the_order_they_entered() {
    let mut fixture = Fixture::new();
    for pocket in ["b", "c", "a"] {
        fixture.publish(pocket, "k", 1, "v").unwrap();
    }
    assert_eq!(fixture.pull("c1", 1).pocket_key.as_str(), "b");
    assert_eq!(fixture.pull("c2", 1).pocket_key.as_str(), "c");
    assert_eq!(fixture.pull("c3", 1).pocket_key.as_str(), "a");
}

// scenario: S039
// obligation: guarantee.ExclusivePocket
// obligation: rule-failure.PullFindsNoPocket.2
#[test]
fn a_second_consumer_never_receives_records_from_a_leased_pocket() {
    let mut fixture = pocket_of(3);
    fixture.publish("other", "o", 1, "o").unwrap();
    fixture.pull("c1", 1);
    let second = fixture.pull("c2", 10);
    assert_eq!(second.pocket_key.as_str(), "other");
    assert_eq!(keys(&second.records), ["o"]);
    assert_eq!(
        fixture.try_pull("c3", 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(keys(&fixture.pull("c2", 10).records), Vec::<&str>::new());
    assert_eq!(keys(&fixture.pull("c1", 10).records), ["r1", "r2"]);
}

/// Three pockets, three consumers: two leased, one pocket left queued, records
/// held, superseded while held, and pulled again under the lease.
fn leases_in_play() -> Fixture {
    let mut fixture = Fixture::new();
    for pocket in ["x", "y", "z"] {
        for key in ["a", "b"] {
            fixture.publish(pocket, key, 1, "v").unwrap();
        }
    }
    fixture.pull("c1", 1);
    fixture.advance(10);
    fixture.pull("c2", 2);
    fixture.publish("x", "a", 5, "v5").unwrap();
    fixture.publish("y", "b", 5, "v5").unwrap();
    fixture.pull("c1", 5);
    fixture
}

// obligation: invariant.Pocket.LeasedHasOneActiveLease
// obligation: invariant.Pocket.UnleasedHasNoActiveLease
// obligation: rule-failure.PullWithinLease.2
// obligation: rule-failure.PullClaimsFrontPocket.3
#[test]
fn a_leased_pocket_has_exactly_one_active_lease_and_no_other_pocket_has_one() {
    let snapshot = leases_in_play().snapshot();
    assert_eq!(snapshot.leases.len(), 2);
    invariants::leased_has_one_active_lease(&snapshot);
    invariants::unleased_has_no_active_lease(&snapshot);
    invariants::order_is_the_queued_pockets(&snapshot);
}

// obligation: invariant.OneActiveLeasePerPocket
// obligation: invariant.OneActiveLeasePerConsumer
#[test]
fn each_pocket_and_each_consumer_has_at_most_one_active_lease() {
    let snapshot = leases_in_play().snapshot();
    invariants::one_active_lease_per_pocket(&snapshot);
    invariants::one_active_lease_per_consumer(&snapshot);
}

// obligation: invariant.Lease.GrantedBeforeExpiry
#[test]
fn every_lease_is_granted_before_it_expires() {
    let snapshot = leases_in_play().snapshot();
    invariants::granted_before_expiry(&snapshot);
}

// obligation: invariant.Record.HeldVersionNotNewer
#[test]
fn a_held_version_is_never_newer_than_the_record() {
    let fixture = leases_in_play();
    let record = fixture.record("x", "a");
    assert_eq!(record.rank, Rank::new(5));
    assert!(matches!(record.status, RecordStatus::Held { rank, .. } if rank == Rank::new(1)));
    invariants::held_version_not_newer(&fixture.snapshot());
}

// obligation: invariant.OutstandingOnlyWhileLeased
#[test]
fn held_records_exist_only_in_leased_pockets() {
    let snapshot = leases_in_play().snapshot();
    assert!(
        snapshot
            .pockets
            .iter()
            .any(|p| p.status == PocketStatus::Queued)
    );
    invariants::outstanding_only_while_leased(&snapshot);
}

/// A lease-id source with one id, 1, to give; every draw after it is
/// [`LeaseIdError::Exhausted`]. `spent()` is a source with none to give. It
/// counts every draw, refused ones included, in `draws()`.
#[derive(Clone, Debug, Default)]
struct OneId {
    spent: Arc<AtomicBool>,
    draws: Arc<AtomicUsize>,
}

impl OneId {
    fn spent() -> Self {
        Self {
            spent: Arc::new(AtomicBool::new(true)),
            draws: Arc::default(),
        }
    }

    fn draws(&self) -> usize {
        self.draws.load(Ordering::SeqCst)
    }
}

impl LeaseIds for OneId {
    fn next(&self) -> Result<LeaseId, LeaseIdError> {
        self.draws.fetch_add(1, Ordering::SeqCst);
        if self.spent.swap(true, Ordering::SeqCst) {
            Err(LeaseIdError::Exhausted)
        } else {
            Ok(LeaseId::from_u128(1))
        }
    }
}

/// A faulty lease-id source that gives id 1 on every draw.
#[derive(Clone, Copy, Debug, Default)]
struct RepeatsOne;

impl LeaseIds for RepeatsOne {
    fn next(&self) -> Result<LeaseId, LeaseIdError> {
        Ok(LeaseId::from_u128(1))
    }
}

/// A queue over `ids` with the fixture's publisher and consumers joined.
fn queue_over<L: LeaseIds>(ids: L) -> Queue<MemoryRecords, FakeClock, L> {
    let mut queue = Queue::new(config(), FakeClock::new(T0), ids).unwrap();
    queue.publisher_joins(publisher()).unwrap();
    for id in ["c1", "c2"] {
        queue.consumer_joins(consumer(id)).unwrap();
    }
    queue
}

fn publish_into<L: LeaseIds>(
    queue: &mut Queue<MemoryRecords, FakeClock, L>,
    pocket_key: &str,
    key: &str,
) {
    let publication = Publication::new(
        PocketKey::new(pocket_key),
        RecordKey::new(key),
        Rank::new(1),
        Payload::default(),
    );
    queue.publish(&publisher(), publication).unwrap();
}

// obligation: rule-success.PullWithinLease
#[test]
fn an_exhausted_source_refuses_only_a_claim_and_the_refusal_changes_nothing() {
    let source = OneId::default();
    let mut queue = queue_over(source.clone());
    for (pocket_key, key) in [("a", "r0"), ("a", "r1"), ("b", "r0")] {
        publish_into(&mut queue, pocket_key, key);
    }
    let Ok(PullOutcome::Batch {
        lease, pocket_key, ..
    }) = queue.pull(&consumer("c1"), 1)
    else {
        panic!("c1 claims a under the source's one id");
    };
    assert_eq!((lease, pocket_key.as_str()), (LeaseId::from_u128(1), "a"));
    let Ok(PullOutcome::Batch {
        lease: within,
        records,
        ..
    }) = queue.pull(&consumer("c1"), 1)
    else {
        panic!("a pull within the lease draws no id");
    };
    assert_eq!((within, keys(&records)), (lease, vec!["r1"]));
    assert_eq!(source.draws(), 1, "the pull within the lease drew no id");
    let before = queue.snapshot();
    assert_eq!(
        queue.pull(&consumer("c2"), 1),
        Err(PullError::LeaseId(LeaseIdError::Exhausted))
    );
    assert_eq!(queue.snapshot(), before, "a refused claim changes nothing");
}

// obligation: rule-success.PullFindsNoPocket
#[test]
fn a_pull_that_finds_no_pocket_draws_no_lease_id() {
    let source = OneId::spent();
    let mut queue = queue_over(source.clone());
    assert_eq!(
        queue.pull(&consumer("c1"), 10),
        Ok(PullOutcome::NoPocketAvailable)
    );
    assert_eq!(source.draws(), 0, "a pull that finds no pocket draws no id");
}

#[test]
fn a_claim_refused_for_a_still_active_id_changes_nothing() {
    let mut queue = queue_over(RepeatsOne);
    for pocket_key in ["a", "b"] {
        publish_into(&mut queue, pocket_key, "r0");
    }
    let Ok(PullOutcome::Batch {
        lease, pocket_key, ..
    }) = queue.pull(&consumer("c1"), 1)
    else {
        panic!("c1 claims a under id 1");
    };
    assert_eq!((lease, pocket_key.as_str()), (LeaseId::from_u128(1), "a"));
    let before = queue.snapshot();
    assert_eq!(
        queue.pull(&consumer("c2"), 1),
        Err(PullError::LeaseId(LeaseIdError::StillActive(lease)))
    );
    assert_eq!(queue.snapshot(), before, "a refused claim changes nothing");
}
