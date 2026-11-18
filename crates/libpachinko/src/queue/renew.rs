//! Renew and release: `Renew`, `ReleaseRequeuesPocket` and
//! `ReleaseIdlesEmptyPocket`; the guard chain every lease action shares; and what
//! a release and a lapse share once the store has applied the six `Freed` rules:
//! the held versions the lease's end took out kept in the shed queues, stamped
//! from the one `ended_at`.
//!
//! No action compares `expires_at` with `now`: a lease the registry still holds is
//! active whatever the clock says, so a late action processed before the lapse
//! succeeds (decision 0019, the specification's Lease expiry comment).

use super::Queue;
use super::types::{DeadLetter, Lease, LeaseStatus, PocketFate, ShedReason, SupersededRecord};
use crate::clock::Clock;
use crate::error::LeaseActionError;
use crate::ids::{ConsumerId, LeaseId, PocketKey};
use crate::lease_ids::LeaseIds;
use crate::records::{EndedVersion, PocketRecords};
use crate::timestamp::Timestamp;

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Renew`: the lease's `expires_at` moves to `now + lease_ttl`, replacing the
    /// old one rather than adding to it. Returns the new `expires_at`.
    ///
    /// # Errors
    ///
    /// In this order: [`LeaseActionError::UnknownConsumer`],
    /// [`LeaseActionError::LeaseNotActive`] for a lease that has ended or never
    /// existed, and [`LeaseActionError::LeaseNotYours`]. A refused renew changes
    /// nothing.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::{PullOutcome, Timestamp};
    /// # use libpachinko::{lease_ids::CountingLeaseIds, records::MemoryRecords};
    /// # use libpachinko::{
    /// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank,
    /// #     RecordKey,
    /// # };
    /// let clock = FakeClock::new(Timestamp::from_millis(0));
    /// let mut config = Config::default();
    /// config.lease_ttl = Duration::from_millis(10);
    /// # let ids = CountingLeaseIds::default();
    /// let mut queue: Queue<MemoryRecords, _, _> = Queue::new(config, clock.clone(), ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// # queue.publish(&p, Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default())).unwrap();
    /// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 1) else { panic!() };
    /// clock.advance(Duration::from_millis(6));
    /// assert_eq!(queue.renew(&c, lease), Ok(Timestamp::from_millis(16)));
    /// assert_eq!(queue.renew(&c, lease), Ok(Timestamp::from_millis(16)));
    /// ```
    pub fn renew(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
    ) -> Result<Timestamp, LeaseActionError> {
        self.authorise(consumer_id, lease)?;
        Ok(self.extend_lease(lease))
    }

    /// `Release`: the lease ends as released with `ended_at = now`, leaving the
    /// registry, and every held record is freed by the six `Freed` rules, which
    /// read its mark: an unsettled record is queued again at its position, an
    /// acked one consumed and a nacked one dead-lettered, and a version deferred
    /// behind a held one is queued (the unsettled held version then superseded at
    /// the lease's end). Then a pocket with records rejoins the order at the back
    /// ([`PocketFate::Queued`], `ReleaseRequeuesPocket`) and one without goes idle
    /// ([`PocketFate::Idle`], `ReleaseIdlesEmptyPocket`), its expiry count reset
    /// either way. Returns the fate and the lease as it ended.
    ///
    /// # Errors
    ///
    /// As [`Queue::renew`]: a second release of one lease is
    /// [`LeaseActionError::LeaseNotActive`]. A refused release changes nothing.
    ///
    /// ```
    /// use libpachinko::{LeaseActionError, LeaseStatus, PocketFate, PullOutcome};
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
    /// let (fate, ended) = queue.release(&c, lease).unwrap();
    /// assert_eq!(fate, PocketFate::Queued);
    /// assert!(matches!(ended.status, LeaseStatus::Released { ended_at } if ended_at >= ended.granted_at));
    /// assert_eq!(queue.release(&c, lease), Err(LeaseActionError::LeaseNotActive(lease)));
    /// ```
    pub fn release(
        &mut self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
    ) -> Result<(PocketFate, Lease), LeaseActionError> {
        let pocket_key = self.authorise(consumer_id, lease)?;
        let now = self.now();
        let ended = self
            .leases
            .end(lease, LeaseStatus::Released { ended_at: now })
            .ok_or(LeaseActionError::LeaseNotActive(lease))?;
        self.consumers.insert(consumer_id.clone(), None);
        // A lease's pocket always exists: no rule removes a pocket.
        let (fate, versions) = self
            .pockets
            .release(&pocket_key)
            .unwrap_or((PocketFate::Idle, Vec::new()));
        self.keep_ended(&ended, now, versions);
        Ok((fate, ended))
    }

    /// Keeps the held versions one lease's end took out, in the order the store
    /// returned them: a superseded held version (`SupersedeUnsettledHeldVersion`)
    /// with `shed_at = ended_at` and `expires_at = ended_at + superseded_ttl`, and
    /// a nacked one (`DeadLetterNackedRecord`, `DeadLetterNackedVersionQueueDeferred`)
    /// with `nacked_by` the lease's consumer, the `nacked_at` its nack stored and
    /// `expires_at = ended_at + dead_letter_ttl`.
    pub(super) fn keep_ended(
        &mut self,
        lease: &Lease,
        ended_at: Timestamp,
        versions: Vec<EndedVersion>,
    ) {
        for version in versions {
            match version {
                EndedVersion::Superseded(held) => self.shed.keep(SupersededRecord {
                    pocket_key: lease.pocket_key.clone(),
                    key: held.key,
                    rank: held.rank,
                    payload: held.payload,
                    reason: ShedReason::Superseded,
                    shed_at: ended_at,
                    expires_at: ended_at + self.config.superseded_ttl,
                }),
                EndedVersion::DeadLettered { held, nacked_at } => {
                    self.shed.keep_dead_letter(DeadLetter {
                        pocket_key: lease.pocket_key.clone(),
                        key: held.key,
                        rank: held.rank,
                        payload: held.payload,
                        nacked_by: lease.consumer_id.clone(),
                        nacked_at,
                        expires_at: ended_at + self.config.dead_letter_ttl,
                    });
                }
            }
        }
    }

    /// The guards every lease action shares, in the order of the rules'
    /// `requires`: the consumer has joined, the lease is active, and it is the
    /// consumer's. Reads nothing but the registries, never the clock. Returns the
    /// lease's pocket key.
    pub(super) fn authorise(
        &self,
        consumer_id: &ConsumerId,
        lease: LeaseId,
    ) -> Result<PocketKey, LeaseActionError> {
        if !self.consumers.contains_key(consumer_id) {
            return Err(LeaseActionError::UnknownConsumer(consumer_id.clone()));
        }
        self.leases.check(lease, consumer_id)
    }

    /// Moves the active lease's `expires_at` to `now + lease_ttl`, in both
    /// indexes, and returns it. Called only after every guard has held.
    pub(super) fn extend_lease(&mut self, lease: LeaseId) -> Timestamp {
        let expires_at = self.now() + self.config.lease_ttl;
        self.leases.extend(lease, expires_at);
        expires_at
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::clock::FakeClock;
    use crate::config::Config;
    use crate::ids::{ConsumerId, LeaseId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use crate::lease_ids::CountingLeaseIds;
    use crate::queue::{Publication, PullOutcome, Queue};
    use crate::records::MemoryRecords;
    use crate::timestamp::Timestamp;

    #[test]
    fn the_expiry_index_holds_one_entry_per_lease_after_each_extension() {
        let clock = FakeClock::new(Timestamp::from_millis(0));
        let config = Config {
            lease_ttl: Duration::from_millis(10),
            ..Config::default()
        };
        let mut queue: Queue<MemoryRecords, _, _> =
            Queue::new(config, clock.clone(), CountingLeaseIds::default()).unwrap();
        let (p, c) = (PublisherId::new("p"), ConsumerId::new("c"));
        queue.publisher_joins(p.clone()).unwrap();
        queue.consumer_joins(c.clone()).unwrap();
        let publication = Publication::new(
            PocketKey::new("pk"),
            RecordKey::new("k"),
            Rank::new(1),
            Payload::default(),
        );
        queue.publish(&p, publication).unwrap();
        let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 10) else {
            panic!("expected a batch")
        };
        let at = |millis| vec![(Timestamp::from_millis(millis), lease)];
        clock.advance(Duration::from_millis(4));
        queue.ack(&c, lease, &RecordKey::new("k")).unwrap();
        assert_eq!(queue.leases.by_expiry(), at(14));
        clock.advance(Duration::from_millis(2));
        queue.renew(&c, lease).unwrap();
        assert_eq!(queue.leases.by_expiry(), at(16));
        queue.renew(&c, lease).unwrap();
        assert_eq!(queue.leases.by_expiry(), at(16));
        assert!(queue.renew(&c, LeaseId::from_u128(9)).is_err());
        assert_eq!(queue.leases.by_expiry(), at(16));
        queue.release(&c, lease).unwrap();
        assert_eq!(queue.leases.by_expiry().len(), 0);
    }
}
