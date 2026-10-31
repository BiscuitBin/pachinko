//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`,
//! `NackSettlesRecord` and `NackDefersNewerVersion`.

use super::{DeadLetter, LeaseId, Queue, SettleOutcome};
use crate::clock::Clock;
use crate::error::ActionError;
use crate::records::{PocketRecords, Version};

/// What a settle step did, for the nack that records its dead letter.
struct Settled {
    outcome: SettleOutcome,
    pocket_key: String,
    held: Option<Version>,
}

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// `Ack` on a held record under `lease`: removed when the held version is the
    /// latest (`AckSettlesRecord`), deferred to the back when a newer version
    /// waits (`AckDefersNewerVersion`). Either way the lease's expiry moves to now
    /// plus `lease_ttl`.
    ///
    /// # Errors
    ///
    /// [`ActionError::UnknownConsumer`], [`ActionError::LeaseNotActive`] for a lease
    /// that has ended or never existed, [`ActionError::LeaseNotYours`], or
    /// [`ActionError::RecordNotHeld`]; the queue is then unchanged.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, PullOutcome, Queue, SettleOutcome};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
    /// queue.publish("p", Publication::new("pk", "rk", 2, Vec::new())).unwrap();
    /// assert_eq!(queue.ack("c", batch.lease, "rk").unwrap(), SettleOutcome::Deferred);
    /// ```
    pub fn ack(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
        record_key: &str,
    ) -> Result<SettleOutcome, ActionError> {
        self.settle_held(consumer_id, lease, record_key)
            .map(|settled| settled.outcome)
    }

    /// `Nack` on a held record under `lease`: the held version becomes a dead
    /// letter, and the record is removed (`NackSettlesRecord`) or, when a newer
    /// version waits, deferred to the back (`NackDefersNewerVersion`). Either way
    /// the lease's expiry moves to now plus `lease_ttl`.
    ///
    /// # Errors
    ///
    /// As [`Queue::ack`].
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, PullOutcome, Queue, SettleOutcome};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
    /// assert_eq!(queue.nack("c", batch.lease, "rk").unwrap(), SettleOutcome::Settled);
    /// assert!(queue.nack("c", batch.lease, "rk").is_err());
    /// ```
    pub fn nack(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
        record_key: &str,
    ) -> Result<SettleOutcome, ActionError> {
        let settled = self.settle_held(consumer_id, lease, record_key)?;
        let now = self.clock.now();
        let expires_at = now.saturating_add(self.config.dead_letter_ttl);
        settled
            .held
            .map(|held| DeadLetter {
                pocket_key: settled.pocket_key,
                key: record_key.to_owned(),
                rank: held.rank,
                payload: held.payload,
                nacked_by: consumer_id.to_owned(),
                nacked_at: now,
                expires_at,
            })
            .into_iter()
            .for_each(|letter| self.shed.keep_dead_letter(letter));
        Ok(settled.outcome)
    }

    /// The guards and effects ack and nack share: the lease is active and the
    /// consumer's, the record is held; the lease is extended and the record
    /// settled or deferred.
    fn settle_held(
        &mut self,
        consumer_id: &str,
        lease: LeaseId,
        record_key: &str,
    ) -> Result<Settled, ActionError> {
        let pocket_key = self.authorise(consumer_id, lease)?;
        let state = self.pockets.require_held(&pocket_key, record_key)?;
        let term = self.lease_term();
        self.leases.extend(lease, term.expires_at);
        let (outcome, held) = self.pockets.settle(&pocket_key, state, record_key);
        Ok(Settled {
            outcome,
            pocket_key,
            held,
        })
    }
}
