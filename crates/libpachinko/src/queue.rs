//! The queue: the module's aggregate and the one place its rules run.
//!
//! [`Queue`] holds every entity of `docs/specs/pachinko.allium` and offers one
//! operation per rule group, each documented with the rules it covers. Every
//! operation reads `now` from the [`Clock`] the queue was built with, once, at its
//! start, and never lets it go back: a reading earlier than one an operation has
//! already seen counts as that one. Nothing inside the library calls
//! [`Queue::lapse`] or [`Queue::decay`] on its own, so the clock-triggered rules run
//! only when the caller sweeps.

mod decay;
mod expiry;
mod lease;
mod outcome;
mod publish;
mod pull;
mod settle;
mod state;
mod view;

pub use outcome::{
    Batch, Decayed, HeldLease, HeldRecord, Lapsed, PocketFate, Publication, PublishOutcome,
    PullOutcome, SettleOutcome,
};
pub use view::{
    DeadLetter, LeaseStatus, LeaseView, PocketStatus, PocketView, RecordStatus, RecordView,
    ShedReason, Snapshot, SupersededRecord,
};

use std::fmt;

use state::State;

use crate::{
    Clock, Config, ConsumerId, Error, LeaseId, LeaseIds, PublisherId, RecordKey, Timestamp,
};

/// The deployment's one queue of pockets.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublishOutcome, PublisherId, PullOutcome, Queue, Rank, RecordKey,
///     SettleOutcome};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// let (p, c) = (PublisherId::new("p1"), ConsumerId::new("c1"));
/// queue.publisher_joins(p.clone()).unwrap();
/// queue.consumer_joins(c.clone()).unwrap();
/// let publish = |rank| Publication::new(p.clone(), PocketKey::new("tenant-a"),
///     RecordKey::new("k1"), Rank::new(rank), Payload::from(b"hello".as_slice()));
/// assert_eq!(queue.publish(publish(1)), Ok(PublishOutcome::Created));
/// let PullOutcome::Batch(batch) = queue.pull(&c, 10).unwrap() else { panic!() };
/// assert_eq!(queue.publish(publish(2)), Ok(PublishOutcome::Superseded));
/// assert_eq!(queue.ack(&c, batch.lease_id, &RecordKey::new("k1")), Ok(SettleOutcome::Deferred));
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Queue<C, L> {
    state: State,
    clock: C,
    lease_ids: L,
}

impl<C, L> Queue<C, L>
where
    C: Clock + Clone + fmt::Debug + 'static,
    L: LeaseIds + Clone + fmt::Debug + 'static,
{
    /// An empty queue: no publishers, consumers, pockets or leases, and
    /// `next_position` at 0.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, Position, Queue};
    /// let queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.snapshot().next_position, Position::new(0));
    /// assert!(queue.snapshot().pockets.is_empty() && queue.snapshot().leases.is_empty());
    /// ```
    ///
    /// The clock and the lease-id source are `Clone`, `Debug` and `'static`, so the
    /// queue is too (invariant 3); a clock that is not is refused:
    ///
    /// ```compile_fail
    /// use libpachinko::{Clock, Config, CountingLeaseIds, Queue, Timestamp};
    /// struct Still;
    /// impl Clock for Still {
    ///     fn now(&self) -> Timestamp { Timestamp::from_millis(0) }
    /// }
    /// let _queue = Queue::new(Config::default(), Still, CountingLeaseIds::default());
    /// ```
    #[must_use]
    pub const fn new(config: Config, clock: C, lease_ids: L) -> Self {
        Self {
            state: State::new(config),
            clock,
            lease_ids,
        }
    }

    /// The configuration the queue was built with.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, Queue};
    /// let queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.config(), &Config::default());
    /// ```
    #[must_use]
    pub const fn config(&self) -> &Config {
        self.state.config()
    }

    /// `PublisherJoins`.
    ///
    /// # Errors
    ///
    /// [`Error::PublisherExists`] when a publisher already has the id.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, Error, FakeClock, PublisherId, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.publisher_joins(PublisherId::new("p")), Ok(()));
    /// assert_eq!(queue.publisher_joins(PublisherId::new("p")),
    ///     Err(Error::PublisherExists(PublisherId::new("p"))));
    /// ```
    pub fn publisher_joins(&mut self, id: PublisherId) -> Result<(), Error> {
        self.state.publisher_joins(id)
    }

    /// `ConsumerJoins`.
    ///
    /// # Errors
    ///
    /// [`Error::ConsumerExists`] when a consumer already has the id.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, Error, FakeClock, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.consumer_joins(ConsumerId::new("c")), Ok(()));
    /// assert_eq!(queue.consumer_joins(ConsumerId::new("c")),
    ///     Err(Error::ConsumerExists(ConsumerId::new("c"))));
    /// ```
    pub fn consumer_joins(&mut self, id: ConsumerId) -> Result<(), Error> {
        self.state.consumer_joins(id)
    }

    /// `Publish`: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede` or
    /// `DropStalePublish`, and then, for a publish into an existing pocket,
    /// `ReviveIdlePocket` or `ReviveParkedPocket` when the pocket is idle or parked.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownPublisher`] when the publisher has not joined, and
    /// [`Error::EqualRank`] when the rank equals the stored record's, which no rule
    /// covers (the module's first open question).
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, Payload, PocketKey, Publication,
    ///     PublishOutcome, PublisherId, Queue, Rank, RecordKey};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// queue.publisher_joins(PublisherId::new("p")).unwrap();
    /// let publish = |key: &str, rank| Publication::new(PublisherId::new("p"),
    ///     PocketKey::new("a"), RecordKey::new(key), Rank::new(rank), Payload::default());
    /// assert_eq!(queue.publish(publish("k1", 2)), Ok(PublishOutcome::Created));
    /// assert_eq!(queue.publish(publish("k2", 1)), Ok(PublishOutcome::NewKey));
    /// assert_eq!(queue.publish(publish("k1", 3)), Ok(PublishOutcome::Superseded));
    /// assert_eq!(queue.publish(publish("k1", 1)), Ok(PublishOutcome::Stale));
    /// assert!(queue.publish(publish("k1", 3)).is_err());
    /// ```
    pub fn publish(&mut self, publication: Publication) -> Result<PublishOutcome, Error> {
        let now = self.state.now(self.clock.now());
        self.state.publish(now, publication)
    }

    /// `Pull`: `PullWithinLease` when the consumer holds a lease,
    /// `PullClaimsFrontPocket` when it holds none and a pocket is queued, and
    /// `PullFindsNoPocket` otherwise; `HoldRecord` for each record of the batch.
    ///
    /// A batch holds the smallest-position queued records, at most `limit` and at
    /// most `max_pull_batch`; a `limit` that is not positive yields an empty batch.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownConsumer`] when the consumer has not joined.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, PullOutcome, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// queue.consumer_joins(ConsumerId::new("c")).unwrap();
    /// assert_eq!(queue.pull(&ConsumerId::new("c"), 5), Ok(PullOutcome::NoPocketAvailable));
    /// assert!(queue.pull(&ConsumerId::new("x"), 5).is_err());
    /// ```
    pub fn pull(&mut self, consumer: &ConsumerId, limit: i64) -> Result<PullOutcome, Error> {
        let now = self.state.now(self.clock.now());
        self.state.pull(now, &self.lease_ids, consumer, limit)
    }

    /// `Ack`: `AckSettlesRecord` when the held version is the latest, and
    /// `AckDefersNewerVersion` when a newer one arrived while it was held. Either way
    /// the lease is extended.
    ///
    /// # Errors
    ///
    /// [`Error::LeaseNotActive`], [`Error::LeaseNotYours`] or
    /// [`Error::RecordNotHeld`], checked in that order.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, Queue,
    ///     RecordKey};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// let lease = LeaseId::from_u128(1);
    /// assert_eq!(queue.ack(&ConsumerId::new("c"), lease, &RecordKey::new("k")),
    ///     Err(Error::LeaseNotActive(lease)));
    /// ```
    pub fn ack(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, Error> {
        let now = self.state.now(self.clock.now());
        self.state.ack(now, consumer, lease, record_key)
    }

    /// `Nack`: `NackSettlesRecord` or `NackDefersNewerVersion`, which also make the
    /// held version a dead letter. Either way the lease is extended.
    ///
    /// # Errors
    ///
    /// As [`Queue::ack`].
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, Queue,
    ///     RecordKey};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// let lease = LeaseId::from_u128(1);
    /// assert_eq!(queue.nack(&ConsumerId::new("c"), lease, &RecordKey::new("k")),
    ///     Err(Error::LeaseNotActive(lease)));
    /// ```
    pub fn nack(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, Error> {
        let now = self.state.now(self.clock.now());
        self.state.nack(now, consumer, lease, record_key)
    }

    /// `Renew`: the lease's `expires_at` moves to `now + lease_ttl`, which this
    /// returns.
    ///
    /// # Errors
    ///
    /// [`Error::LeaseNotActive`] or [`Error::LeaseNotYours`], in that order.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// let lease = LeaseId::from_u128(1);
    /// assert_eq!(queue.renew(&ConsumerId::new("c"), lease), Err(Error::LeaseNotActive(lease)));
    /// ```
    pub fn renew(&mut self, consumer: &ConsumerId, lease: LeaseId) -> Result<Timestamp, Error> {
        let now = self.state.now(self.clock.now());
        self.state.renew(now, consumer, lease)
    }

    /// `Release`: `ReleaseRequeuesPocket`, with `RequeueHeldRecord` and
    /// `RequeueDeferredRecord` for its records, when the pocket has records, and
    /// `ReleaseIdlesEmptyPocket` when it has none.
    ///
    /// # Errors
    ///
    /// [`Error::LeaseNotActive`] or [`Error::LeaseNotYours`], in that order.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// let lease = LeaseId::from_u128(1);
    /// assert_eq!(queue.release(&ConsumerId::new("c"), lease), Err(Error::LeaseNotActive(lease)));
    /// ```
    pub fn release(&mut self, consumer: &ConsumerId, lease: LeaseId) -> Result<PocketFate, Error> {
        let now = self.state.now(self.clock.now());
        self.state.release(now, consumer, lease)
    }

    /// `LeaseLapses`: every active lease whose `expires_at` is at or before `now`,
    /// ordered by `expires_at` and then id, goes through `LeaseExpiryParksPocket`,
    /// `LeaseExpiryKeepsPocketAtFront` or `LeaseExpiryIdlesEmptyPocket`, with
    /// `RequeueHeldRecord` and `RequeueDeferredRecord` for its records.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert!(queue.lapse().is_empty());
    /// ```
    pub fn lapse(&mut self) -> Vec<Lapsed> {
        let now = self.state.now(self.clock.now());
        self.state.lapse(now)
    }

    /// `SupersededRecordDecays` and `DeadLetterDecays`: every entry whose
    /// `expires_at` is at or before `now` is removed.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// assert_eq!(queue.decay().superseded, 0);
    /// ```
    pub fn decay(&mut self) -> Decayed {
        let now = self.state.now(self.clock.now());
        self.state.decay(now)
    }

    /// The `Consuming` surface's `exposes`: the consumer's active lease, or `None`.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownConsumer`] when the consumer has not joined.
    ///
    /// ```
    /// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// queue.consumer_joins(ConsumerId::new("c")).unwrap();
    /// assert_eq!(queue.lease_of(&ConsumerId::new("c")), Ok(None));
    /// assert!(queue.lease_of(&ConsumerId::new("x")).is_err());
    /// ```
    pub fn lease_of(&self, consumer: &ConsumerId) -> Result<Option<HeldLease>, Error> {
        self.state.lease_of(consumer)
    }

    /// Every entity, cloned: the read model the invariant checks read.
    ///
    /// ```
    /// use libpachinko::{Config, CountingLeaseIds, FakeClock, PublisherId, Queue};
    /// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
    /// queue.publisher_joins(PublisherId::new("p")).unwrap();
    /// assert_eq!(queue.snapshot().publishers, [PublisherId::new("p")]);
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.state.snapshot()
    }
}
