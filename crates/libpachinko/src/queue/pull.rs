//! Pull: `PullWithinLease`, `PullClaimsFrontPocket` and `PullFindsNoPocket`, with
//! `HoldRecord` for each record handed out.

use super::leases::Term;
use super::{LeaseId, PullOutcome, PulledBatch, Queue};
use crate::clock::Clock;
use crate::error::ConsumerError;
use crate::records::PocketRecords;

/// `min(limit, max_pull_batch)`, and 0 for a limit that is not positive.
fn batch_size(limit: i64, max_pull_batch: u32) -> usize {
    let capped = limit.clamp(0, i64::from(max_pull_batch));
    usize::try_from(capped).unwrap_or(0)
}

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// `Pull`: with an active lease, the next records of its pocket
    /// (`PullWithinLease`); without one, a new lease on the queued pocket with the
    /// smallest position (`PullClaimsFrontPocket`); without one and with no queued
    /// pocket, [`PullOutcome::NoPocketAvailable`] (`PullFindsNoPocket`). A batch
    /// holds the `min(limit, max_pull_batch)` oldest queued records, none when
    /// `limit` is not positive, and the lease's expiry moves to now plus
    /// `lease_ttl`.
    ///
    /// # Errors
    ///
    /// [`ConsumerError::UnknownConsumer`] when no consumer with this id has joined.
    /// Finding no pocket is an outcome, never an error.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, PullOutcome, Queue};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// assert_eq!(queue.pull("c", 10).unwrap(), PullOutcome::NoPocketAvailable);
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// let PullOutcome::Batch(batch) = queue.pull("c", 0).unwrap() else { panic!() };
    /// assert!(batch.records.is_empty());
    /// ```
    pub fn pull(
        &mut self,
        consumer_id: &str,
        limit: i64,
    ) -> Result<PullOutcome<R::Batch>, ConsumerError> {
        let active = self.consumers.active_lease(consumer_id)?;
        let size = batch_size(limit, self.config.max_pull_batch);
        let term = self.lease_term();
        Ok(match active {
            Some(lease) => self.pull_within_lease(lease, size, term),
            None => self.claim_front_pocket(consumer_id, size, term),
        })
    }

    /// `PullWithinLease`.
    fn pull_within_lease(
        &mut self,
        lease: LeaseId,
        size: usize,
        term: Term,
    ) -> PullOutcome<R::Batch> {
        self.leases
            .extend(lease, term.expires_at)
            .and_then(|pocket_key| {
                self.pockets
                    .hold(&pocket_key, size)
                    .map(|records| PulledBatch {
                        lease,
                        pocket_key,
                        expires_at: term.expires_at,
                        records,
                    })
            })
            .map_or(PullOutcome::NoPocketAvailable, PullOutcome::Batch)
    }

    /// `PullClaimsFrontPocket`, or `PullFindsNoPocket` when nothing is queued.
    fn claim_front_pocket(
        &mut self,
        consumer_id: &str,
        size: usize,
        term: Term,
    ) -> PullOutcome<R::Batch> {
        self.pockets
            .claim_front()
            .and_then(|pocket_key| {
                let lease = self.leases.grant(&pocket_key, consumer_id, term);
                self.consumers.set_lease(consumer_id, Some(lease));
                self.pockets
                    .hold(&pocket_key, size)
                    .map(|records| PulledBatch {
                        lease,
                        pocket_key,
                        expires_at: term.expires_at,
                        records,
                    })
            })
            .map_or(PullOutcome::NoPocketAvailable, PullOutcome::Batch)
    }
}
