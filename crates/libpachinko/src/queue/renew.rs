//! Renew and release: `Renew`, `ReleaseRequeuesPocket` and
//! `ReleaseIdlesEmptyPocket`, with the `Freed` rules `RequeueHeldRecord` and
//! `RequeueDeferredRecord`.

use super::{Lease, LeaseId, LeaseStatus, Queue};
use crate::clock::{Clock, Timestamp};
use crate::error::ActionError;
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// `Renew`: the lease's expiry moves to now plus `lease_ttl`. Returns the new
    /// expiry.
    ///
    /// # Errors
    ///
    /// [`ActionError::UnknownConsumer`], [`ActionError::LeaseNotActive`] or
    /// [`ActionError::LeaseNotYours`]; the queue is then unchanged.
    ///
    /// ```
    /// use libpachinko::clock::{FakeClock, Timestamp};
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, PullOutcome, Queue};
    /// use libpachinko::records::MemoryRecords;
    /// use std::time::Duration;
    ///
    /// let clock = FakeClock::new(Timestamp::from_millis(0));
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), clock.clone()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
    /// clock.advance(Duration::from_secs(60));
    /// assert_eq!(queue.renew("c", batch.lease).unwrap(), Timestamp::from_millis(360_000));
    /// ```
    pub fn renew(&mut self, consumer_id: &str, lease: LeaseId) -> Result<Timestamp, ActionError> {
        self.authorise(consumer_id, lease)?;
        let term = self.lease_term();
        self.leases.extend(lease, term.expires_at);
        Ok(term.expires_at)
    }

    /// `Release`: the lease ends as released. A pocket with records frees its held
    /// and deferred records and rejoins the order at the next position
    /// (`ReleaseRequeuesPocket`); an empty one goes idle
    /// (`ReleaseIdlesEmptyPocket`). Either way its expiry count resets. Returns
    /// the lease as it ended.
    ///
    /// # Errors
    ///
    /// As [`Queue::renew`].
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{LeaseStatus, Publication, PullOutcome, Queue};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
    /// assert_eq!(queue.release("c", batch.lease).unwrap().status, LeaseStatus::Released);
    /// assert!(queue.release("c", batch.lease).is_err());
    /// ```
    pub fn release(&mut self, consumer_id: &str, lease: LeaseId) -> Result<Lease, ActionError> {
        self.authorise(consumer_id, lease)?;
        let now = self.now();
        let ended = self
            .leases
            .end(lease, LeaseStatus::Released, now)
            .ok_or(ActionError::LeaseNotActive(lease))?;
        self.consumers.set_lease(consumer_id, None);
        self.pockets.release(&ended.pocket_key);
        Ok(ended)
    }

    /// The guards every lease action shares: the consumer has joined, and the
    /// lease is active and the consumer's. Returns the lease's pocket key.
    pub(super) fn authorise(
        &self,
        consumer_id: &str,
        lease: LeaseId,
    ) -> Result<String, ActionError> {
        self.consumers.active_lease(consumer_id)?;
        self.leases.check(lease, consumer_id)
    }
}
