//! Pull: `PullWithinLease`, `PullClaimsFrontPocket` and `PullFindsNoPocket`, with
//! `HoldRecord` for each record handed out.

use super::Queue;
use super::leases::Active;
use super::types::PullOutcome;
use crate::clock::Clock;
use crate::error::PullError;
use crate::ids::{ConsumerId, LeaseId};
use crate::lease_ids::LeaseIds;
use crate::records::PocketRecords;

/// `min(limit, max_pull_batch)`, and 0 for a limit that is not positive.
fn batch_size(limit: i64, max_pull_batch: u32) -> usize {
    let capped = limit.clamp(0, i64::from(max_pull_batch));
    usize::try_from(capped).unwrap_or(0)
}

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Pull`. With an active lease, the lease is extended to `now + lease_ttl` and
    /// its pocket's oldest queued records are held (`PullWithinLease`). Without
    /// one, the queued pocket with the smallest position is leased under a new
    /// lease granted now and expiring at `now + lease_ttl`, and its oldest queued
    /// records held (`PullClaimsFrontPocket`). Without one and with no queued
    /// pocket, [`PullOutcome::NoPocketAvailable`] (`PullFindsNoPocket`). A batch
    /// holds `min(limit, max_pull_batch)` records, none when `limit` is not
    /// positive, and carries the lease's `expires_at` as this pull left it.
    ///
    /// # Errors
    ///
    /// [`PullError::UnknownConsumer`] when the consumer has not joined, checked
    /// first. Finding no pocket is an outcome, never an error.
    ///
    /// ```
    /// use libpachinko::PullOutcome;
    /// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
    /// # use libpachinko::{
    /// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank,
    /// #     RecordKey,
    /// # };
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// let c = ConsumerId::new("c");
    /// queue.consumer_joins(c.clone()).unwrap();
    /// assert_eq!(queue.pull(&c, 10), Ok(PullOutcome::NoPocketAvailable));
    /// let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
    /// queue.publish(&p, publication).unwrap();
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 0) else { panic!() };
    /// assert!(records.is_empty());
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 10) else { panic!() };
    /// assert_eq!(records[0].key.as_str(), "rk");
    /// ```
    pub fn pull(&mut self, consumer_id: &ConsumerId, limit: i64) -> Result<PullOutcome, PullError> {
        let active = self.active_lease(consumer_id)?;
        let size = batch_size(limit, self.config.max_pull_batch);
        Ok(match active {
            Some(lease) => self.pull_within_lease(lease, size),
            None => self.claim_front_pocket(consumer_id, size),
        })
    }

    /// `PullWithinLease`.
    fn pull_within_lease(&mut self, lease: LeaseId, size: usize) -> PullOutcome {
        let expires_at = self.now() + self.config.lease_ttl;
        let Some(pocket_key) = self.leases.extend(lease, expires_at) else {
            return PullOutcome::NoPocketAvailable;
        };
        let records = self.pockets.hold(&pocket_key, size);
        PullOutcome::Batch {
            lease,
            pocket_key,
            expires_at,
            records,
        }
    }

    /// `PullClaimsFrontPocket`, or `PullFindsNoPocket` when nothing is queued.
    fn claim_front_pocket(&mut self, consumer_id: &ConsumerId, size: usize) -> PullOutcome {
        let Some(pocket_key) = self.pockets.claim_front() else {
            return PullOutcome::NoPocketAvailable;
        };
        let granted_at = self.now();
        let expires_at = granted_at + self.config.lease_ttl;
        let lease = self.fresh_lease_id();
        self.leases.grant(
            lease,
            Active {
                pocket_key: pocket_key.clone(),
                consumer_id: consumer_id.clone(),
                granted_at,
                expires_at,
            },
        );
        self.consumers.insert(consumer_id.clone(), Some(lease));
        let records = self.pockets.hold(&pocket_key, size);
        PullOutcome::Batch {
            lease,
            pocket_key,
            expires_at,
            records,
        }
    }

    /// A lease id from the source. The source never repeats one; should it return
    /// an id that is still active, the queue draws again rather than let two
    /// leases share it.
    fn fresh_lease_id(&self) -> LeaseId {
        let mut id = self.lease_ids.next();
        while self.leases.is_active(id) {
            id = self.lease_ids.next();
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::batch_size;
    use crate::clock::FakeClock;
    use crate::config::Config;
    use crate::ids::{ConsumerId, LeaseId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use crate::lease_ids::LeaseIds;
    use crate::queue::{Publication, PullOutcome, Queue};
    use crate::records::MemoryRecords;

    #[test]
    fn the_batch_size_is_the_limit_capped_and_floored_at_zero() {
        assert_eq!(batch_size(5, 100), 5);
        assert_eq!(batch_size(500, 100), 100);
        assert_eq!(batch_size(0, 100), 0);
        assert_eq!(batch_size(-3, 100), 0);
        assert_eq!(batch_size(i64::MIN, 1), 0);
        assert_eq!(batch_size(i64::MAX, u32::MAX), u32::MAX as usize);
    }

    /// A faulty source that hands out 1, 1, 2: it repeats an id.
    #[derive(Clone, Debug, Default)]
    struct Repeating(Arc<AtomicUsize>);

    impl LeaseIds for Repeating {
        fn next(&self) -> LeaseId {
            let ids = [1, 1, 2];
            LeaseId::from_u128(ids[self.0.fetch_add(1, Ordering::SeqCst).min(2)])
        }
    }

    #[test]
    fn a_repeated_lease_id_that_is_still_active_is_drawn_again() {
        let mut queue: Queue<MemoryRecords, _, _> = Queue::new(
            Config::default(),
            FakeClock::default(),
            Repeating::default(),
        )
        .unwrap();
        let publisher = PublisherId::new("p");
        queue.publisher_joins(publisher.clone()).unwrap();
        for pocket in ["a", "b"] {
            let publication = Publication::new(
                PocketKey::new(pocket),
                RecordKey::new("k"),
                Rank::new(1),
                Payload::default(),
            );
            queue.publish(&publisher, publication).unwrap();
        }
        let mut leases = Vec::new();
        for consumer in ["c1", "c2"] {
            queue.consumer_joins(ConsumerId::new(consumer)).unwrap();
            let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&ConsumerId::new(consumer), 1)
            else {
                panic!("expected a batch");
            };
            leases.push(lease);
        }
        assert_eq!(leases, [LeaseId::from_u128(1), LeaseId::from_u128(2)]);
    }
}
