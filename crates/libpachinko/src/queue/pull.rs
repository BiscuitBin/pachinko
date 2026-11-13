//! Pull: `PullWithinLease`, `PullClaimsFrontPocket` and `PullFindsNoPocket`, with
//! `HoldRecord` for each record handed out.

use super::Queue;
use super::leases::Active;
use super::types::PullOutcome;
use crate::clock::Clock;
use crate::error::{LeaseIdError, PullError};
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
    /// Only a claim draws a lease id, once, and only after it finds a queued
    /// pocket: a pull within a lease and a pull that finds no pocket never draw.
    ///
    /// # Errors
    ///
    /// [`PullError::UnknownConsumer`] when the consumer has not joined, checked
    /// first. [`PullError::LeaseId`] when a claim needs a lease id and the source
    /// cannot give one ([`LeaseIdError::Exhausted`]), or gives one that is still
    /// active ([`LeaseIdError::StillActive`]); the source is not drawn again, and
    /// the refused pull changes nothing. Finding no pocket is an outcome, never an
    /// error.
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
        match active {
            Some(lease) => Ok(self.pull_within_lease(lease, size)),
            None => self.claim_front_pocket(consumer_id, size),
        }
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
    ///
    /// It peeks at the front pocket, draws one lease id and checks it is not active
    /// before it claims the pocket, so a refused claim changes nothing and the
    /// source is drawn at most once.
    fn claim_front_pocket(
        &mut self,
        consumer_id: &ConsumerId,
        size: usize,
    ) -> Result<PullOutcome, PullError> {
        if self.pockets.front().is_none() {
            return Ok(PullOutcome::NoPocketAvailable);
        }
        let lease = self.lease_ids.next()?;
        if self.leases.is_active(lease) {
            return Err(LeaseIdError::StillActive(lease).into());
        }
        // The front pocket was just seen, so the claim takes it; the `else` mirrors
        // `pull_within_lease` rather than panic on a state that cannot occur.
        let Some(pocket_key) = self.pockets.claim_front() else {
            return Ok(PullOutcome::NoPocketAvailable);
        };
        let granted_at = self.now();
        let expires_at = granted_at + self.config.lease_ttl;
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
        Ok(PullOutcome::Batch {
            lease,
            pocket_key,
            expires_at,
            records,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::batch_size;
    use crate::clock::FakeClock;
    use crate::config::Config;
    use crate::error::{LeaseIdError, PullError};
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
        fn next(&self) -> Result<LeaseId, LeaseIdError> {
            let ids = [1, 1, 2];
            Ok(LeaseId::from_u128(
                ids[self.0.fetch_add(1, Ordering::SeqCst).min(2)],
            ))
        }
    }

    #[test]
    fn a_repeated_lease_id_that_is_still_active_refuses_the_claim_and_changes_nothing() {
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
        let (c1, c2) = (ConsumerId::new("c1"), ConsumerId::new("c2"));
        queue.consumer_joins(c1.clone()).unwrap();
        queue.consumer_joins(c2.clone()).unwrap();
        let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c1, 1) else {
            panic!("expected a batch");
        };
        assert_eq!(lease, LeaseId::from_u128(1));
        assert_eq!(
            queue.pull(&c2, 1),
            Err(PullError::LeaseId(LeaseIdError::StillActive(
                LeaseId::from_u128(1)
            )))
        );
        let leased: Vec<LeaseId> = queue
            .leases
            .by_expiry()
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        assert_eq!(leased, [LeaseId::from_u128(1)], "one lease is active");
        let only = queue.leases.get(LeaseId::from_u128(1)).unwrap();
        assert_eq!(only.pocket_key.as_str(), "a", "a is the only pocket leased");
        assert_eq!(queue.pockets.front().map(PocketKey::as_str), Some("b"));
        assert_eq!(queue.consumers.get(&c2), Some(&None));
    }

    /// A faulty source that returns id 1 on every draw and counts its draws. It
    /// panics past the second, so a queue that draws again while the id is active
    /// fails here rather than spin.
    #[derive(Clone, Debug, Default)]
    struct AlwaysOne(Arc<AtomicUsize>);

    impl LeaseIds for AlwaysOne {
        fn next(&self) -> Result<LeaseId, LeaseIdError> {
            let drawn = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            assert!(drawn <= 2, "the source was drawn {drawn} times");
            Ok(LeaseId::from_u128(1))
        }
    }

    #[test]
    fn a_source_that_repeats_an_active_id_is_drawn_twice_and_refused() {
        let source = AlwaysOne::default();
        let mut queue: Queue<MemoryRecords, _, _> =
            Queue::new(Config::default(), FakeClock::default(), source.clone()).unwrap();
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
        let (c1, c2) = (ConsumerId::new("c1"), ConsumerId::new("c2"));
        queue.consumer_joins(c1.clone()).unwrap();
        queue.consumer_joins(c2.clone()).unwrap();
        let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c1, 1) else {
            panic!("expected a batch");
        };
        assert_eq!(lease, LeaseId::from_u128(1));
        let refused = queue.pull(&c2, 1);
        assert_eq!(
            refused,
            Err(PullError::LeaseId(LeaseIdError::StillActive(
                LeaseId::from_u128(1)
            )))
        );
        assert_eq!(source.0.load(Ordering::SeqCst), 2);
    }
}
