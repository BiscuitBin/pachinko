//! The queue: the specification's `Queue` entity with its publishers, consumers,
//! pockets, records, leases and shed queue, and one method per trigger.
//!
//! `Queue` is plain data with `&mut self` methods. It performs no effect: it reads
//! the time only through its [`Clock`] and draws lease ids only from its
//! [`LeaseIds`]. Whoever owns it applies one operation at a time, so a lapse, which
//! [`Queue::tick`] processes, and a consumer's action meet in one stream, as
//! decision 0019 requires.

mod expiry;
mod leases;
mod pockets;
mod publish;
mod pull;
mod renew;
mod settle;
mod shed;
#[cfg(feature = "testing")]
mod snapshot;
mod types;

use std::collections::{BTreeMap, BTreeSet};

use crate::clock::{Clock, Timestamp};
use crate::config::Config;
use crate::error::{ConfigError, JoinError, PullError};
use crate::ids::{ConsumerId, PublisherId};
use crate::lease_ids::{LeaseId, LeaseIds};
use crate::records::PocketRecords;

use leases::Leases;
use pockets::Pockets;
use shed::Shed;

pub use expiry::TickReport;
#[cfg(feature = "testing")]
pub use snapshot::{PocketView, Snapshot};
pub use types::{
    DeadLetter, Lease, LeaseStatus, LeaseView, PocketFate, PocketStatus, Publication,
    PublishOutcome, PullOutcome, SettleOutcome, ShedReason, SupersededRecord,
};

/// The deployment's one queue, over a record store `R`, a clock `C` and a lease-id
/// source `L`.
///
/// `Queue` is `Clone`, as invariant 3 asks of every public type, and a clone forks
/// the state: the two never share a pocket, record, lease or shed entry afterwards,
/// because [`PocketRecords`] requires its `Clone` to be a deep, independent copy,
/// which `check_contract` proves, and the queue's other stores are plain values.
/// What they do share is whatever the clock
/// and the lease-id source share between clones: a shared [`FakeClock`] keeps
/// ticking for both, and a shared [`CountingLeaseIds`] keeps counting for both.
///
/// [`FakeClock`]: crate::clock::FakeClock
/// [`CountingLeaseIds`]: crate::lease_ids::CountingLeaseIds
///
/// ```
/// use libpachinko::clock::{FakeClock, Timestamp};
/// use libpachinko::config::Config;
/// use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
/// use libpachinko::lease_ids::CountingLeaseIds;
/// use libpachinko::queue::{Publication, PublishOutcome, PullOutcome, Queue};
/// use libpachinko::records::MemoryRecords;
///
/// let clock = FakeClock::new(Timestamp::from_millis(0));
/// let mut queue: Queue<MemoryRecords, _, _> =
///     Queue::new(Config::default(), clock, CountingLeaseIds::default()).unwrap();
/// let (publisher, consumer) = (PublisherId::new("p"), ConsumerId::new("c"));
/// queue.publisher_joins(publisher.clone()).unwrap();
/// queue.consumer_joins(consumer.clone()).unwrap();
/// let publication = Publication::new(PocketKey::new("orders"), RecordKey::new("o-1"), Rank::new(1), Payload::default());
/// assert_eq!(queue.publish(&publisher, publication), Ok(PublishOutcome::Created));
/// let forked = queue.clone();
/// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&consumer, 10) else { panic!() };
/// assert_eq!(records.len(), 1);
/// assert_eq!(forked.lease_view(&consumer), Ok(None));
/// ```
///
/// A store that is not `Send + Sync` cannot be a [`PocketRecords`], so no queue can
/// be built over one (invariant 3):
///
/// ```compile_fail,E0277
/// use std::rc::Rc;
/// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
/// use libpachinko::records::{HeldRecord, PocketRecords, RecordView};
///
/// #[derive(Clone, Debug, Default)]
/// struct RcRecords(Rc<Vec<RecordView>>);
///
/// impl PocketRecords for RcRecords {
///     type Batch = Vec<HeldRecord>;
///     fn lookup(&self, _: &RecordKey) -> Option<RecordView> { None }
///     fn insert_queued(&mut self, _: RecordKey, _: Rank, _: Payload, _: Position) {}
///     fn supersede(&mut self, _: &RecordKey, _: Rank, _: Payload) -> Option<(Rank, Payload)> { None }
///     fn hold_oldest(&mut self, _: usize) -> Vec<HeldRecord> { Vec::new() }
///     fn settle(&mut self, _: &RecordKey) -> Option<HeldRecord> { None }
///     fn defer(&mut self, _: &RecordKey, _: Position) -> Option<HeldRecord> { None }
///     fn free_held(&mut self) -> usize { 0 }
///     fn record_count(&self) -> usize { 0 }
///     fn held(&self) -> Vec<HeldRecord> { Vec::new() }
///     fn snapshot(&self) -> Vec<RecordView> { Vec::new() }
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Queue<R: PocketRecords, C: Clock, L: LeaseIds> {
    config: Config,
    clock: C,
    lease_ids: L,
    publishers: BTreeSet<PublisherId>,
    /// Each consumer with its active lease, if any (`OneActiveLeasePerConsumer`).
    consumers: BTreeMap<ConsumerId, Option<LeaseId>>,
    pockets: Pockets<R>,
    leases: Leases,
    shed: Shed,
    /// The latest instant the queue has read: its `now` never goes back, even when
    /// the clock does.
    latest: Timestamp,
}

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// An empty queue with `config`, reading the time from `clock` and lease ids
    /// from `lease_ids`.
    ///
    /// # Errors
    ///
    /// The [`ConfigError`] [`Config::check`] returns.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::error::ConfigError;
    /// use libpachinko::lease_ids::CountingLeaseIds;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut config = Config::default();
    /// config.max_consecutive_expiries = 0;
    /// let queue = Queue::<MemoryRecords, _, _>::new(config, FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.err(), Some(ConfigError::MaxConsecutiveExpiriesZero));
    /// ```
    pub fn new(config: Config, clock: C, lease_ids: L) -> Result<Self, ConfigError> {
        config.check()?;
        Ok(Self {
            config,
            clock,
            lease_ids,
            publishers: BTreeSet::new(),
            consumers: BTreeMap::new(),
            pockets: Pockets::default(),
            leases: Leases::default(),
            shed: Shed::default(),
            latest: Timestamp::default(),
        })
    }

    /// The configuration the queue was built with.
    ///
    /// ```
    /// # use libpachinko::{clock::FakeClock, config::Config, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::Queue, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// let queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// assert_eq!(queue.config(), &Config::default());
    /// ```
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// `PublisherJoins`.
    ///
    /// # Errors
    ///
    /// [`JoinError::PublisherExists`] when a publisher with this id has joined;
    /// nothing changes.
    ///
    /// ```
    /// use libpachinko::error::JoinError;
    /// use libpachinko::ids::PublisherId;
    /// # use libpachinko::{clock::FakeClock, config::Config, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::Queue, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// assert_eq!(queue.publisher_joins(PublisherId::new("p")), Ok(()));
    /// assert_eq!(
    ///     queue.publisher_joins(PublisherId::new("p")),
    ///     Err(JoinError::PublisherExists(PublisherId::new("p")))
    /// );
    /// ```
    pub fn publisher_joins(&mut self, publisher_id: PublisherId) -> Result<(), JoinError> {
        if self.publishers.contains(&publisher_id) {
            return Err(JoinError::PublisherExists(publisher_id));
        }
        self.publishers.insert(publisher_id);
        Ok(())
    }

    /// `ConsumerJoins`.
    ///
    /// # Errors
    ///
    /// [`JoinError::ConsumerExists`] when a consumer with this id has joined;
    /// nothing changes.
    ///
    /// ```
    /// use libpachinko::error::JoinError;
    /// use libpachinko::ids::ConsumerId;
    /// # use libpachinko::{clock::FakeClock, config::Config, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::Queue, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// assert_eq!(queue.consumer_joins(ConsumerId::new("c")), Ok(()));
    /// assert_eq!(
    ///     queue.consumer_joins(ConsumerId::new("c")),
    ///     Err(JoinError::ConsumerExists(ConsumerId::new("c")))
    /// );
    /// ```
    pub fn consumer_joins(&mut self, consumer_id: ConsumerId) -> Result<(), JoinError> {
        if self.consumers.contains_key(&consumer_id) {
            return Err(JoinError::ConsumerExists(consumer_id));
        }
        self.consumers.insert(consumer_id, None);
        Ok(())
    }

    /// The `Consuming` surface's `exposes` for `consumer_id`: its active lease's
    /// pocket key, `expires_at` and held records, or `None` for a joined consumer
    /// with no active lease.
    ///
    /// # Errors
    ///
    /// [`PullError::UnknownConsumer`] when no consumer with this id has joined.
    ///
    /// ```
    /// use libpachinko::ids::{ConsumerId, PocketKey};
    /// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let publisher = PublisherId::new("p");
    /// # queue.publisher_joins(publisher.clone()).unwrap();
    /// # let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
    /// # queue.publish(&publisher, publication).unwrap();
    /// let consumer = ConsumerId::new("c");
    /// assert!(queue.lease_view(&consumer).is_err());
    /// queue.consumer_joins(consumer.clone()).unwrap();
    /// assert_eq!(queue.lease_view(&consumer), Ok(None));
    /// queue.pull(&consumer, 10).unwrap();
    /// let view = queue.lease_view(&consumer).unwrap().unwrap();
    /// assert_eq!(view.pocket_key, PocketKey::new("pk"));
    /// assert_eq!(view.records[0].key.as_str(), "rk");
    /// ```
    pub fn lease_view(&self, consumer_id: &ConsumerId) -> Result<Option<LeaseView>, PullError> {
        let active = self.active_lease(consumer_id)?;
        Ok(active.and_then(|id| {
            let lease = self.leases.get(id)?;
            Some(LeaseView::new(
                lease.pocket_key.clone(),
                lease.expires_at,
                self.pockets.held(&lease.pocket_key),
            ))
        }))
    }

    /// The consumer's active lease, if any.
    fn active_lease(&self, consumer_id: &ConsumerId) -> Result<Option<LeaseId>, PullError> {
        self.consumers
            .get(consumer_id)
            .copied()
            .ok_or_else(|| PullError::UnknownConsumer(consumer_id.clone()))
    }

    /// The specification's `now`: the clock's reading, held at the latest instant
    /// the queue has read, so a clock that steps back cannot stamp anything before
    /// an earlier stamp.
    fn now(&mut self) -> Timestamp {
        self.latest = self.latest.max(self.clock.now());
        self.latest
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::time::Duration;

    use super::Queue;
    use crate::clock::{Clock, FakeClock, Timestamp};
    use crate::config::Config;
    use crate::lease_ids::{CountingLeaseIds, LeaseIds};
    use crate::records::{MemoryRecords, PocketRecords};

    fn assert_invariant_3<T: Clone + Debug + Send + Sync + 'static>() {}

    /// Compiles only when the trait bounds alone carry invariant 3 to the queue.
    fn queue_meets_invariant_3<R: PocketRecords, C: Clock, L: LeaseIds>() {
        assert_invariant_3::<Queue<R, C, L>>();
    }

    #[test]
    fn every_queue_is_clone_debug_send_sync_and_static() {
        queue_meets_invariant_3::<MemoryRecords, FakeClock, CountingLeaseIds>();
    }

    #[test]
    fn now_never_goes_back_when_the_clock_does() {
        let clock = FakeClock::new(Timestamp::from_millis(100));
        let mut queue: Queue<MemoryRecords, _, _> = Queue::new(
            Config::default(),
            clock.clone(),
            CountingLeaseIds::default(),
        )
        .unwrap();
        assert_eq!(queue.now(), Timestamp::from_millis(100));
        clock.set(Timestamp::from_millis(50));
        assert_eq!(queue.now(), Timestamp::from_millis(100));
        clock.advance(Duration::from_millis(60));
        assert_eq!(queue.now(), Timestamp::from_millis(110));
    }
}
