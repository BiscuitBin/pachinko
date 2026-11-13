//! A queue over `MemoryRecords`, a `FakeClock` the test keeps a handle on and
//! `CountingLeaseIds`, with one publisher, `p`, and three consumers, `c1`, `c2` and
//! `c3`, already joined; helpers that publish and pull by literal keys; and the
//! state invariants this ticket owns, each checked by name over a `Snapshot`.

use std::collections::BTreeSet;
use std::time::Duration;

use libpachinko::clock::FakeClock;
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::records::{MemoryRecords, Position, RecordStatus, RecordView};
use libpachinko::testing::{PocketStatus, PocketView, Snapshot};
use libpachinko::{
    Config, ConsumerId, HeldRecord, Lease, LeaseActionError, LeaseId, LeaseStatus, LeaseView,
    Payload, PocketFate, PocketKey, Publication, PublishError, PublishOutcome, PublisherId,
    PullError, PullOutcome, Queue, Rank, RecordKey, SettleOutcome, TickReport, Timestamp,
};

/// The instant every fixture starts at.
pub(crate) const T0: Timestamp = Timestamp::from_millis(1_000_000);

/// A short lease, so temporal tests count in milliseconds.
pub(crate) const LEASE_TTL: Duration = Duration::from_millis(1_000);

/// A short superseded TTL.
pub(crate) const SUPERSEDED_TTL: Duration = Duration::from_millis(5_000);

/// A short dead-letter TTL, unlike the other two so a mix-up shows.
pub(crate) const DEAD_LETTER_TTL: Duration = Duration::from_millis(7_000);

pub(crate) type TestQueue = Queue<MemoryRecords, FakeClock, CountingLeaseIds>;

/// A pulled batch, unpacked.
#[derive(Debug)]
pub(crate) struct Batch {
    pub(crate) lease: LeaseId,
    pub(crate) pocket_key: PocketKey,
    pub(crate) expires_at: Timestamp,
    pub(crate) records: Vec<HeldRecord>,
}

pub(crate) struct Fixture {
    pub(crate) queue: TestQueue,
    pub(crate) clock: FakeClock,
}

/// The fixture's configuration: short TTLs, the specification's limits.
pub(crate) fn config() -> Config {
    let mut config = Config::default();
    config.lease_ttl = LEASE_TTL;
    config.superseded_ttl = SUPERSEDED_TTL;
    config.dead_letter_ttl = DEAD_LETTER_TTL;
    config
}

/// The fixture's configuration with `lease_ttl` of `millis`, for the lapse tests
/// that count in single milliseconds.
pub(crate) fn short_lease(millis: u64) -> Config {
    let mut config = config();
    config.lease_ttl = Duration::from_millis(millis);
    config
}

pub(crate) fn publisher() -> PublisherId {
    PublisherId::new("p")
}

pub(crate) fn consumer(id: &str) -> ConsumerId {
    ConsumerId::new(id)
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::with_config(config())
    }

    pub(crate) fn with_config(config: Config) -> Self {
        let clock = FakeClock::new(T0);
        let mut queue = Queue::new(config, clock.clone(), CountingLeaseIds::default()).unwrap();
        queue.publisher_joins(publisher()).unwrap();
        for id in ["c1", "c2", "c3"] {
            queue.consumer_joins(consumer(id)).unwrap();
        }
        Self { queue, clock }
    }

    pub(crate) fn advance(&self, millis: u64) {
        self.clock.advance(Duration::from_millis(millis));
    }

    pub(crate) fn publish(
        &mut self,
        pocket_key: &str,
        record_key: &str,
        rank: i64,
        payload: &str,
    ) -> Result<PublishOutcome, PublishError> {
        let publication = Publication::new(
            PocketKey::new(pocket_key),
            RecordKey::new(record_key),
            Rank::new(rank),
            Payload::from(payload.as_bytes()),
        );
        self.queue.publish(&publisher(), publication)
    }

    pub(crate) fn try_pull(
        &mut self,
        consumer_id: &str,
        limit: i64,
    ) -> Result<PullOutcome, PullError> {
        self.queue.pull(&consumer(consumer_id), limit)
    }

    /// A pull that must return a batch.
    pub(crate) fn pull(&mut self, consumer_id: &str, limit: i64) -> Batch {
        match self.try_pull(consumer_id, limit).unwrap() {
            PullOutcome::Batch {
                lease,
                pocket_key,
                expires_at,
                records,
            } => Batch {
                lease,
                pocket_key,
                expires_at,
                records,
            },
            other => panic!("expected a batch, got {other:?}"),
        }
    }

    pub(crate) fn lease_view(&self, consumer_id: &str) -> Option<LeaseView> {
        self.queue.lease_view(&consumer(consumer_id)).unwrap()
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        self.queue.snapshot()
    }

    pub(crate) fn pocket(&self, pocket_key: &str) -> PocketView {
        self.snapshot()
            .pockets
            .into_iter()
            .find(|pocket| pocket.key.as_str() == pocket_key)
            .unwrap_or_else(|| panic!("no pocket {pocket_key}"))
    }

    pub(crate) fn record(&self, pocket_key: &str, record_key: &str) -> RecordView {
        self.pocket(pocket_key)
            .records
            .into_iter()
            .find(|record| record.key.as_str() == record_key)
            .unwrap_or_else(|| panic!("no record {record_key} in {pocket_key}"))
    }

    pub(crate) fn ack(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
        record_key: &str,
    ) -> Result<SettleOutcome, LeaseActionError> {
        self.queue
            .ack(&consumer(consumer_id), lease, &RecordKey::new(record_key))
    }

    pub(crate) fn nack(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
        record_key: &str,
    ) -> Result<SettleOutcome, LeaseActionError> {
        self.queue
            .nack(&consumer(consumer_id), lease, &RecordKey::new(record_key))
    }

    pub(crate) fn renew(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
    ) -> Result<Timestamp, LeaseActionError> {
        self.queue.renew(&consumer(consumer_id), lease)
    }

    pub(crate) fn release(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
    ) -> Result<(PocketFate, Lease), LeaseActionError> {
        self.queue.release(&consumer(consumer_id), lease)
    }

    /// Runs `action`, which must be refused with `expected`, and checks that the
    /// queue's snapshot is the same after as before.
    pub(crate) fn refused<T: std::fmt::Debug>(
        &mut self,
        expected: &LeaseActionError,
        action: impl FnOnce(&mut Self) -> Result<T, LeaseActionError>,
    ) {
        let before = self.snapshot();
        let outcome = action(self);
        assert_eq!(outcome.unwrap_err(), *expected);
        assert_eq!(
            self.snapshot(),
            before,
            "a refused action changed the queue"
        );
    }

    /// Sets the clock to `millis` after [`T0`].
    pub(crate) fn at(&self, millis: u64) {
        self.clock.set(T0.saturating_add_millis(millis));
    }

    /// One tick, after which every state invariant and `ExpiriesWithinLimit`
    /// are checked.
    pub(crate) fn tick(&mut self) -> TickReport {
        let report = self.queue.tick();
        self.check();
        report
    }

    /// Every state invariant and `ExpiriesWithinLimit` at the queue's limit, over
    /// a fresh snapshot.
    pub(crate) fn check(&self) {
        let snapshot = self.snapshot();
        invariants::all(&snapshot);
        invariants::expiries_within_limit(&snapshot, self.queue.config().max_consecutive_expiries);
    }

    /// The pocket keys in the pocket order, front first.
    pub(crate) fn order(&self) -> Vec<String> {
        self.snapshot()
            .pocket_order
            .into_iter()
            .map(|(_, key)| key.as_str().to_owned())
            .collect()
    }
}

/// The records' keys, in order.
pub(crate) fn keys(records: &[HeldRecord]) -> Vec<&str> {
    records.iter().map(|record| record.key.as_str()).collect()
}

/// `true` when `items` holds no value twice. A `Vec` is checked, so a duplicate
/// the invariant forbids shows.
fn distinct<T: Ord>(items: impl IntoIterator<Item = T>) -> bool {
    let items: Vec<T> = items.into_iter().collect();
    let count = items.len();
    items.into_iter().collect::<BTreeSet<T>>().len() == count
}

/// Every state invariant this ticket owns, each by its specification name.
pub(crate) mod invariants {
    use super::{
        BTreeSet, Lease, LeaseStatus, PocketStatus, Position, RecordStatus, Snapshot, distinct,
    };

    pub(crate) fn unique_publisher_id(s: &Snapshot) {
        assert!(distinct(s.publishers.iter()), "UniquePublisherId");
    }

    pub(crate) fn unique_consumer_id(s: &Snapshot) {
        assert!(
            distinct(s.consumers.iter().map(|(id, _)| id)),
            "UniqueConsumerId"
        );
    }

    pub(crate) fn one_pocket_per_key(s: &Snapshot) {
        assert!(
            distinct(s.pockets.iter().map(|p| &p.key)),
            "OnePocketPerKey"
        );
    }

    pub(crate) fn one_record_per_key_per_pocket(s: &Snapshot) {
        for pocket in &s.pockets {
            assert!(
                distinct(pocket.records.iter().map(|r| &r.key)),
                "OneRecordPerKeyPerPocket in {}",
                pocket.key
            );
        }
    }

    pub(crate) fn unique_pocket_position(s: &Snapshot) {
        assert!(
            distinct(s.pockets.iter().map(|p| p.position)),
            "UniquePocketPosition"
        );
    }

    pub(crate) fn unique_record_position(s: &Snapshot) {
        let positions = s
            .pockets
            .iter()
            .flat_map(|p| p.records.iter().map(|r| r.position));
        assert!(distinct(positions), "UniqueRecordPosition");
    }

    pub(crate) fn pocket_positions_below_next(s: &Snapshot) {
        for pocket in &s.pockets {
            assert!(
                pocket.position < s.next_position,
                "PocketPositionsBelowNext: {}",
                pocket.key
            );
        }
    }

    pub(crate) fn record_positions_below_next(s: &Snapshot) {
        for record in s.pockets.iter().flat_map(|p| &p.records) {
            assert!(
                record.position < s.next_position,
                "RecordPositionsBelowNext: {}",
                record.key
            );
        }
    }

    /// The active leases on the pocket.
    fn active_on(s: &Snapshot, key: &libpachinko::PocketKey) -> usize {
        s.leases
            .iter()
            .filter(|l| &l.pocket_key == key && l.status == LeaseStatus::Active)
            .count()
    }

    pub(crate) fn leased_has_one_active_lease(s: &Snapshot) {
        for pocket in s
            .pockets
            .iter()
            .filter(|p| p.status == PocketStatus::Leased)
        {
            assert_eq!(
                active_on(s, &pocket.key),
                1,
                "LeasedHasOneActiveLease: {}",
                pocket.key
            );
        }
    }

    pub(crate) fn unleased_has_no_active_lease(s: &Snapshot) {
        for pocket in s
            .pockets
            .iter()
            .filter(|p| p.status != PocketStatus::Leased)
        {
            assert_eq!(
                active_on(s, &pocket.key),
                0,
                "UnleasedHasNoActiveLease: {}",
                pocket.key
            );
        }
    }

    pub(crate) fn one_active_lease_per_pocket(s: &Snapshot) {
        let active = s.leases.iter().filter(|l| l.status == LeaseStatus::Active);
        assert!(
            distinct(active.map(|l| &l.pocket_key)),
            "OneActiveLeasePerPocket"
        );
    }

    pub(crate) fn one_active_lease_per_consumer(s: &Snapshot) {
        let active = s.leases.iter().filter(|l| l.status == LeaseStatus::Active);
        assert!(
            distinct(active.map(|l| &l.consumer_id)),
            "OneActiveLeasePerConsumer"
        );
        for (id, lease) in &s.consumers {
            let registered: BTreeSet<_> = s
                .leases
                .iter()
                .filter(|l| &l.consumer_id == id && l.status == LeaseStatus::Active)
                .map(|l| l.id)
                .collect();
            assert_eq!(
                registered,
                lease.iter().copied().collect(),
                "OneActiveLeasePerConsumer: {id}'s lease and the registry agree"
            );
        }
    }

    pub(crate) fn granted_before_expiry(s: &Snapshot) {
        for lease in &s.leases {
            assert!(
                lease.granted_at <= lease.expires_at,
                "GrantedBeforeExpiry: {}",
                lease.id
            );
        }
    }

    pub(crate) fn held_version_not_newer(s: &Snapshot) {
        for record in s.pockets.iter().flat_map(|p| &p.records) {
            if let RecordStatus::Held { rank, .. } = &record.status {
                assert!(*rank <= record.rank, "HeldVersionNotNewer: {}", record.key);
            }
        }
    }

    pub(crate) fn outstanding_only_while_leased(s: &Snapshot) {
        let outstanding = s.pockets.iter().flat_map(|pocket| {
            pocket
                .records
                .iter()
                .filter(|record| record.status != RecordStatus::Queued)
                .map(move |record| (pocket, record))
        });
        for (pocket, record) in outstanding {
            assert_eq!(
                pocket.status,
                PocketStatus::Leased,
                "OutstandingOnlyWhileLeased: {}",
                record.key
            );
        }
    }

    pub(crate) fn shed_before_expiry(s: &Snapshot) {
        for shed in &s.superseded {
            assert!(
                shed.shed_at <= shed.expires_at,
                "ShedBeforeExpiry: {}",
                shed.key
            );
        }
    }

    /// The pocket order is exactly the queued pockets at their positions.
    pub(crate) fn order_is_the_queued_pockets(s: &Snapshot) {
        let mut queued: Vec<(Position, _)> = s
            .pockets
            .iter()
            .filter(|p| p.status == PocketStatus::Queued)
            .map(|p| (p.position, p.key.clone()))
            .collect();
        queued.sort();
        assert_eq!(
            s.pocket_order, queued,
            "the pocket order is the queued pockets"
        );
    }

    pub(crate) fn idle_has_no_records(s: &Snapshot) {
        for pocket in s.pockets.iter().filter(|p| p.status == PocketStatus::Idle) {
            assert!(
                pocket.records.is_empty(),
                "IdleHasNoRecords: {}",
                pocket.key
            );
        }
    }

    pub(crate) fn nacked_before_expiry(s: &Snapshot) {
        for letter in &s.dead_letters {
            assert!(
                letter.nacked_at <= letter.expires_at,
                "NackedBeforeExpiry: {}",
                letter.key
            );
        }
    }

    /// `EndedAfterGrant` over a lease as it ended: the registry keeps no ended
    /// lease, so the check reads the one an end returned.
    pub(crate) fn ended_after_grant(lease: &Lease) {
        match lease.status {
            LeaseStatus::Released { ended_at } | LeaseStatus::Expired { ended_at } => {
                assert!(
                    ended_at >= lease.granted_at,
                    "EndedAfterGrant: {}",
                    lease.id
                );
            }
            _ => panic!("EndedAfterGrant: {} has not ended", lease.id),
        }
    }

    /// `ExpiriesWithinLimit`, at the queue's `max_consecutive_expiries`, which the
    /// snapshot does not carry.
    pub(crate) fn expiries_within_limit(s: &Snapshot, limit: u32) {
        for pocket in &s.pockets {
            assert!(
                pocket.consecutive_expiries <= limit,
                "ExpiriesWithinLimit: {}",
                pocket.key
            );
        }
    }

    /// One strict, deterministic order: the pocket positions and the record
    /// positions are each distinct and below `next_position`, and the pocket order
    /// is the queued pockets sorted by position.
    pub(crate) fn one_strict_order(s: &Snapshot) {
        unique_pocket_position(s);
        pocket_positions_below_next(s);
        for pocket in &s.pockets {
            assert!(
                distinct(pocket.records.iter().map(|r| r.position)),
                "record positions in {}",
                pocket.key
            );
        }
        unique_record_position(s);
        record_positions_below_next(s);
        order_is_the_queued_pockets(s);
    }

    /// Every state invariant, for a test that wants them all after its steps.
    pub(crate) fn all(s: &Snapshot) {
        unique_publisher_id(s);
        unique_consumer_id(s);
        one_pocket_per_key(s);
        one_record_per_key_per_pocket(s);
        unique_pocket_position(s);
        unique_record_position(s);
        pocket_positions_below_next(s);
        record_positions_below_next(s);
        leased_has_one_active_lease(s);
        unleased_has_no_active_lease(s);
        one_active_lease_per_pocket(s);
        one_active_lease_per_consumer(s);
        granted_before_expiry(s);
        held_version_not_newer(s);
        outstanding_only_while_leased(s);
        shed_before_expiry(s);
        order_is_the_queued_pockets(s);
        idle_has_no_records(s);
        nacked_before_expiry(s);
    }
}
