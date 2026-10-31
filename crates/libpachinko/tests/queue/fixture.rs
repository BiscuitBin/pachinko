//! A queue over `MemoryRecords` and a `FakeClock` the test keeps a handle on, with
//! one publisher, `p`, and two consumers, `c1` and `c2`, already joined.

use std::time::Duration;

use libpachinko::clock::{FakeClock, Timestamp};
use libpachinko::config::Config;
use libpachinko::error::PublishError;
use libpachinko::queue::{
    LeaseId, PocketView, Publication, PublishOutcome, PullOutcome, PulledBatch, Queue,
    QueueSnapshot,
};
use libpachinko::records::{HeldRecord, MemoryRecords, RecordView};

/// The instant every fixture starts at.
pub(crate) const T0: Timestamp = Timestamp::from_millis(1_000_000);

/// A short lease, so temporal tests count in milliseconds.
pub(crate) const LEASE_TTL: Duration = Duration::from_millis(1_000);

/// The decay TTL for both shed queues.
pub(crate) const DECAY_TTL: Duration = Duration::from_millis(5_000);

pub(crate) type TestQueue = Queue<MemoryRecords, FakeClock>;
pub(crate) type Batch = PulledBatch<Vec<HeldRecord>>;

pub(crate) struct Fixture {
    pub(crate) queue: TestQueue,
    pub(crate) clock: FakeClock,
}

/// The fixture's configuration: short TTLs, the module's limits.
pub(crate) fn config() -> Config {
    let mut config = Config::default();
    config.lease_ttl = LEASE_TTL;
    config.superseded_ttl = DECAY_TTL;
    config.dead_letter_ttl = DECAY_TTL;
    config
}

/// `instant` moved `duration` later.
pub(crate) fn at(instant: Timestamp, duration: Duration) -> Timestamp {
    instant.saturating_add(duration)
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::with_config(config())
    }

    pub(crate) fn with_config(config: Config) -> Self {
        let clock = FakeClock::new(T0);
        let mut queue = Queue::new(config, clock.clone()).unwrap();
        queue.publisher_joins("p").unwrap();
        queue.consumer_joins("c1").unwrap();
        queue.consumer_joins("c2").unwrap();
        Self { queue, clock }
    }

    pub(crate) fn now(&self) -> Timestamp {
        libpachinko::clock::Clock::now(&self.clock)
    }

    pub(crate) fn advance(&self, duration: Duration) {
        self.clock.advance(duration);
    }

    pub(crate) fn publish(
        &mut self,
        pocket_key: &str,
        record_key: &str,
        rank: i64,
        payload: &str,
    ) -> Result<PublishOutcome, PublishError> {
        let publication =
            Publication::new(pocket_key, record_key, rank, payload.as_bytes().to_vec());
        self.queue.publish("p", publication)
    }

    /// A pull that must return a batch.
    pub(crate) fn pull(&mut self, consumer_id: &str, limit: i64) -> Batch {
        match self.queue.pull(consumer_id, limit).unwrap() {
            PullOutcome::Batch(batch) => batch,
            other => panic!("expected a batch, got {other:?}"),
        }
    }

    pub(crate) fn snapshot(&self) -> QueueSnapshot {
        self.queue.snapshot()
    }

    pub(crate) fn pocket(&self, pocket_key: &str) -> PocketView {
        self.snapshot()
            .pockets
            .into_iter()
            .find(|pocket| pocket.key == pocket_key)
            .unwrap_or_else(|| panic!("no pocket {pocket_key}"))
    }

    pub(crate) fn record(&self, pocket_key: &str, record_key: &str) -> Option<RecordView> {
        self.pocket(pocket_key)
            .records
            .into_iter()
            .find(|record| record.key == record_key)
    }

    /// The consumer's active lease, from the inspection seam.
    pub(crate) fn active_lease(&self, consumer_id: &str) -> Option<LeaseId> {
        self.snapshot()
            .consumers
            .into_iter()
            .find(|(id, _)| id == consumer_id)
            .and_then(|(_, lease)| lease)
    }
}

/// The held records' keys, in batch order.
pub(crate) fn keys(batch: &Batch) -> Vec<&str> {
    batch
        .records
        .iter()
        .map(|record| record.key.as_str())
        .collect()
}
