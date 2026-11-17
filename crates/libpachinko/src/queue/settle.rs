//! Ack and nack: `AckMarksRecord`, `AckRepeats`, `NackMarksRecord` and
//! `NackRepeats`. An ack or a nack marks the held record and removes nothing: the
//! record stays held until its lease ends, and the lease's end acts on the mark.
//! The first mark also resets its pocket's consecutive-expiry count; a repeat
//! only extends the lease.

use super::Queue;
use super::types::SettleOutcome;
use crate::clock::Clock;
use crate::error::LeaseActionError;
use crate::ids::{ConsumerId, LeaseId, Mark, RecordKey};
use crate::lease_ids::LeaseIds;
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Ack` of `record_key` under `lease`: the held record is marked acked and
    /// stays held, at the version it was handed out with, until the lease ends,
    /// which consumes it (`AckMarksRecord`). A repeat ack succeeds and changes
    /// nothing but the lease's expiry (`AckRepeats`). Either way the lease's
    /// `expires_at` moves to `now + lease_ttl` and the answer is
    /// [`SettleOutcome::Acked`]; the first ack also resets the pocket's
    /// consecutive-expiry count to 0.
    ///
    /// # Errors
    ///
    /// In this order: [`LeaseActionError::UnknownConsumer`],
    /// [`LeaseActionError::LeaseNotActive`], [`LeaseActionError::LeaseNotYours`],
    /// [`LeaseActionError::RecordNotHeld`] when the lease's pocket does not hold
    /// the key, and [`LeaseActionError::MarkConflict`] when the record is already
    /// nacked. A refused ack changes nothing, `expires_at` included, and reads no
    /// clock.
    ///
    /// ```
    /// use libpachinko::{LeaseActionError, PullOutcome, SettleOutcome};
    /// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
    /// # use libpachinko::{
    /// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank,
    /// #     RecordKey,
    /// # };
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// let publish = |key| Publication::new(PocketKey::new("pk"), RecordKey::new(key), Rank::new(1), Payload::default());
    /// queue.publish(&p, publish("a")).unwrap();
    /// queue.publish(&p, publish("b")).unwrap();
    /// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 2) else { panic!() };
    /// let (a, b) = (RecordKey::new("a"), RecordKey::new("b"));
    /// assert_eq!(queue.ack(&c, lease, &a), Ok(SettleOutcome::Acked));
    /// assert_eq!(queue.ack(&c, lease, &a), Ok(SettleOutcome::Acked));
    /// assert_eq!(queue.nack(&c, lease, &b), Ok(SettleOutcome::Nacked));
    /// assert_eq!(queue.ack(&c, lease, &b), Err(LeaseActionError::MarkConflict(b)));
    /// let other = RecordKey::new("other");
    /// assert_eq!(queue.ack(&c, lease, &other), Err(LeaseActionError::RecordNotHeld(other)));
    /// ```
    pub fn ack(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, LeaseActionError> {
        self.mark_held(consumer_id, lease, record_key, Mark::Acked)?;
        Ok(SettleOutcome::Acked)
    }

    /// `Nack` of `record_key` under `lease`: as [`Queue::ack`], but the held record
    /// is marked nacked with `nacked_at = now` (`NackMarksRecord`), and a repeat
    /// nack keeps the first nack's time (`NackRepeats`). The answer is
    /// [`SettleOutcome::Nacked`]. The record is never handed out again under this
    /// lease, and no dead letter exists yet: the lease's end makes one, keeping
    /// this `nacked_at`.
    ///
    /// # Errors
    ///
    /// As [`Queue::ack`], with [`LeaseActionError::MarkConflict`] when the record
    /// is already acked. A refused nack changes nothing.
    ///
    /// ```
    /// use libpachinko::{PullOutcome, SettleOutcome};
    /// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
    /// # use libpachinko::{
    /// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank,
    /// #     RecordKey,
    /// # };
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// # queue.publish(&p, Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default())).unwrap();
    /// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert_eq!(queue.nack(&c, lease, &RecordKey::new("rk")), Ok(SettleOutcome::Nacked));
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert!(records.is_empty());
    /// assert_eq!(queue.lease_view(&c).unwrap().unwrap().records.len(), 1);
    /// ```
    pub fn nack(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, LeaseActionError> {
        self.mark_held(consumer_id, lease, record_key, Mark::Nacked)?;
        Ok(SettleOutcome::Nacked)
    }

    /// What ack and nack share: the guards in rule order, all read before the
    /// clock, so a refusal reads none; then the lease's extension; then, on the
    /// first mark only, the mark and the pocket's count reset.
    fn mark_held(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
        mark: Mark,
    ) -> Result<(), LeaseActionError> {
        let pocket_key = self.authorise(consumer_id, lease)?;
        let stored = self
            .pockets
            .mark_of(&pocket_key, record_key)
            .ok_or_else(|| LeaseActionError::RecordNotHeld(record_key.clone()))?;
        let first = stored == Mark::Unsettled;
        if !first && stored != mark {
            return Err(LeaseActionError::MarkConflict(record_key.clone()));
        }
        let at = self.now();
        self.leases.extend(lease, at + self.config.lease_ttl);
        if first {
            self.pockets.mark(&pocket_key, record_key, mark, at);
        }
        Ok(())
    }
}
