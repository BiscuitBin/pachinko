//! The queue: the module's `Queue` entity, its pockets, records, leases and the
//! two decaying queues, with one method per trigger.
//!
//! `Queue` is plain data with `&mut self` methods; it performs no effect, and reads
//! the time only through the [`Clock`] it was built with. Whoever owns it processes
//! one trigger at a time, so a lapse and a consumer's action meet in one stream, as
//! the module's expiry comment requires.

mod expiry;
mod leases;
mod parties;
mod pockets;
mod publish;
mod pull;
mod renew;
mod settle;
mod shed;
mod types;
#[cfg(feature = "testing")]
mod view;

use crate::clock::{Clock, Timestamp};
use crate::config::Config;
use crate::error::{ConfigError, ConsumerError, JoinError};
use crate::records::PocketRecords;

use leases::Leases;
use parties::{Consumers, Publishers};
use pockets::Pockets;
use shed::Shed;

pub use crate::error::fencing::LeaseId;
pub use types::{
    DeadLetter, Lease, LeaseStatus, LeaseView, PocketStatus, Publication, PublishOutcome,
    PullOutcome, PulledBatch, SettleOutcome, ShedReason, SupersededRecord, TickReport,
};
#[cfg(feature = "testing")]
pub use view::{PocketView, QueueSnapshot};

/// The deployment's one queue, over a record store `R` and a clock `C`.
///
/// ```
/// use libpachinko::clock::{FakeClock, Timestamp};
/// use libpachinko::config::Config;
/// use libpachinko::queue::{Publication, PublishOutcome, PullOutcome, Queue, SettleOutcome};
/// use libpachinko::records::MemoryRecords;
///
/// let clock = FakeClock::new(Timestamp::from_millis(0));
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), clock).unwrap();
/// queue.publisher_joins("publisher").unwrap();
/// queue.consumer_joins("consumer").unwrap();
/// let publication = Publication::new("orders", "order-1", 1, b"{}".to_vec());
/// assert_eq!(queue.publish("publisher", publication).unwrap(), PublishOutcome::New);
/// let PullOutcome::Batch(batch) = queue.pull("consumer", 10).unwrap() else { panic!() };
/// assert_eq!(batch.records.len(), 1);
/// let outcome = queue.ack("consumer", batch.lease, "order-1").unwrap();
/// assert_eq!(outcome, SettleOutcome::Settled);
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Queue<R, C> {
    config: Config,
    clock: C,
    publishers: Publishers,
    consumers: Consumers,
    leases: Leases,
    pockets: Pockets<R>,
    shed: Shed,
    /// The latest instant the queue has read: its `now` never goes back, even
    /// when the clock does.
    latest: Timestamp,
}

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// An empty queue with `config`, reading the time from `clock`.
    ///
    /// # Errors
    ///
    /// The [`ConfigError`] [`Config::validate`] returns.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut config = Config::default();
    /// config.max_pull_batch = 0;
    /// assert!(Queue::<MemoryRecords, _>::new(config, FakeClock::default()).is_err());
    /// ```
    pub fn new(config: Config, clock: C) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            clock,
            publishers: Publishers::default(),
            consumers: Consumers::default(),
            leases: Leases::default(),
            pockets: Pockets::default(),
            shed: Shed::default(),
            latest: Timestamp::default(),
        })
    }

    /// The configuration the queue was built with.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// assert_eq!(*queue.config(), Config::default());
    /// ```
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// `PublisherJoins`.
    ///
    /// # Errors
    ///
    /// [`JoinError::AlreadyJoined`] when a publisher with this id has joined.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// assert!(queue.publisher_joins("p").is_ok());
    /// assert!(queue.publisher_joins("p").is_err());
    /// ```
    pub fn publisher_joins(&mut self, publisher_id: &str) -> Result<(), JoinError> {
        self.publishers.join(publisher_id)
    }

    /// `ConsumerJoins`.
    ///
    /// # Errors
    ///
    /// [`JoinError::AlreadyJoined`] when a consumer with this id has joined.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// assert!(queue.consumer_joins("c").is_ok());
    /// assert!(queue.consumer_joins("c").is_err());
    /// ```
    pub fn consumer_joins(&mut self, consumer_id: &str) -> Result<(), JoinError> {
        self.consumers.join(consumer_id)
    }

    /// The `Consuming` surface's `exposes` for `consumer_id`: its active lease's
    /// pocket key, expiry and held records, or `None` without an active lease.
    ///
    /// # Errors
    ///
    /// [`ConsumerError::UnknownConsumer`] when no consumer with this id has joined.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, Queue};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, b"x".to_vec())).unwrap();
    /// queue.pull("c", 10).unwrap();
    /// let view = queue.lease_view("c").unwrap().unwrap();
    /// assert_eq!(view.pocket_key, "pk");
    /// assert_eq!(view.held[0].key, "rk");
    /// ```
    pub fn lease_view(&self, consumer_id: &str) -> Result<Option<LeaseView>, ConsumerError> {
        let lease = self.consumers.active_lease(consumer_id)?;
        Ok(lease.and_then(|id| {
            self.leases
                .pocket_of(id)
                .map(|(pocket_key, expires_at)| LeaseView {
                    lease: id,
                    held: self.pockets.held(&pocket_key),
                    pocket_key,
                    expires_at,
                })
        }))
    }

    /// The module's `now`: the clock's reading, held at the latest instant the
    /// queue has read so far, so a clock that steps back cannot stamp a lease
    /// end before its grant or an expiry before its grant.
    fn now(&mut self) -> Timestamp {
        let now = self.clock.now().max(self.latest);
        self.latest = now;
        now
    }

    /// Now, and when a lease granted or extended now lapses.
    fn lease_term(&mut self) -> leases::Term {
        let now = self.now();
        leases::Term {
            granted_at: now,
            expires_at: now.saturating_add(self.config.lease_ttl),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Queue;
    use crate::clock::{Clock, FakeClock};
    use crate::records::{MemoryRecords, PocketRecords};
    use std::fmt::Debug;

    fn assert_invariant_3<T: Clone + Debug + Send + Sync + 'static>() {}

    /// Generic over every store and clock `Queue::new` accepts, so it compiles
    /// only when the trait bounds alone carry invariant 3 to the queue.
    fn queue_meets_invariant_3<R: PocketRecords, C: Clock>() {
        assert_invariant_3::<Queue<R, C>>();
    }

    #[test]
    fn every_queue_is_clone_debug_send_sync_and_static() {
        queue_meets_invariant_3::<MemoryRecords, FakeClock>();
    }
}
