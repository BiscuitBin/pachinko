//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord`
//! and `NackDefersNewerVersion`.

use super::Queue;
use super::types::{DeadLetter, SettleOutcome};
use crate::clock::Clock;
use crate::clock::Timestamp;
use crate::error::LeaseActionError;
use crate::ids::{ConsumerId, PocketKey, RecordKey};
use crate::lease_ids::{LeaseId, LeaseIds};
use crate::records::{HeldRecord, PocketRecords};

/// What a settle step leaves for a nack's dead letter.
struct Settled {
    /// The lease's pocket.
    pocket_key: PocketKey,
    /// The held version, as it was handed out.
    held: HeldRecord,
    /// The one `now` the step read: the lease's extension and the dead letter's
    /// stamps are all measured from it.
    at: Timestamp,
}

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Ack` of `record_key` under `lease`. When the held version is the latest the
    /// record is removed (`AckSettlesRecord`, [`SettleOutcome::Settled`]); when a
    /// newer version arrived while it was held, the record is deferred at a fresh
    /// position and pulled again only once the lease ends
    /// (`AckDefersNewerVersion`, [`SettleOutcome::Deferred`]). Either way the
    /// lease's `expires_at` moves to `now + lease_ttl`.
    ///
    /// # Errors
    ///
    /// In this order: [`LeaseActionError::UnknownConsumer`],
    /// [`LeaseActionError::LeaseNotActive`], [`LeaseActionError::LeaseNotYours`],
    /// and [`LeaseActionError::RecordNotHeld`] when the lease's pocket does not
    /// hold the key. A refused ack changes nothing, `expires_at` included.
    ///
    /// ```
    /// use libpachinko::error::LeaseActionError;
    /// use libpachinko::queue::{PullOutcome, SettleOutcome};
    /// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// let publish = |rank| Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(rank), Payload::default());
    /// queue.publish(&p, publish(1)).unwrap();
    /// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 1) else { panic!() };
    /// queue.publish(&p, publish(2)).unwrap();
    /// let key = RecordKey::new("rk");
    /// assert_eq!(queue.ack(&c, lease, &key), Ok(SettleOutcome::Deferred));
    /// assert_eq!(queue.ack(&c, lease, &key), Err(LeaseActionError::RecordNotHeld(key)));
    /// ```
    pub fn ack(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, LeaseActionError> {
        self.settle_held(consumer_id, lease, record_key)
            .map(|(outcome, _)| outcome)
    }

    /// `Nack` of `record_key` under `lease`: as [`Queue::ack`], and the held
    /// version becomes a dead letter stamped `nacked_at = now` and
    /// `expires_at = now + dead_letter_ttl`, kept beside the superseded records
    /// (`NackSettlesRecord`, `NackDefersNewerVersion`). The record is never handed
    /// out again under this lease.
    ///
    /// # Errors
    ///
    /// As [`Queue::ack`]. A refused nack changes nothing and creates no dead
    /// letter.
    ///
    /// ```
    /// use libpachinko::queue::{PullOutcome, SettleOutcome};
    /// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// # queue.publish(&p, Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default())).unwrap();
    /// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert_eq!(queue.nack(&c, lease, &RecordKey::new("rk")), Ok(SettleOutcome::Settled));
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert!(records.is_empty());
    /// ```
    pub fn nack(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, LeaseActionError> {
        let (outcome, settled) = self.settle_held(consumer_id, lease, record_key)?;
        let Settled {
            pocket_key,
            held,
            at: nacked_at,
        } = settled;
        self.shed.keep_dead_letter(DeadLetter {
            pocket_key,
            key: held.key,
            rank: held.rank,
            payload: held.payload,
            nacked_by: consumer_id.clone(),
            nacked_at,
            expires_at: nacked_at + self.config.dead_letter_ttl,
        });
        Ok(outcome)
    }

    /// What ack and nack share: the four guards in rule order, then the lease's
    /// extension, then the settle or defer split. Returns the rule taken and what
    /// a nack's dead letter is made from.
    fn settle_held(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<(SettleOutcome, Settled), LeaseActionError> {
        let pocket_key = self.authorise(consumer_id, lease)?;
        if !self.pockets.is_held(&pocket_key, record_key) {
            return Err(LeaseActionError::RecordNotHeld(record_key.clone()));
        }
        let at = self.now();
        self.leases.extend(lease, at + self.config.lease_ttl);
        // Unreachable: `is_held` held above, and nothing between changed the record.
        let (outcome, held) = self
            .pockets
            .settle(&pocket_key, record_key)
            .ok_or_else(|| LeaseActionError::RecordNotHeld(record_key.clone()))?;
        Ok((
            outcome,
            Settled {
                pocket_key,
                held,
                at,
            },
        ))
    }
}
