//! Publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`,
//! `DeferBehindHeldVersion` and `DropStalePublish`. The three that add or raise a
//! record in an existing pocket end in the `PocketTouched` dispatch; a stale
//! publish does not touch its pocket.

use super::Queue;
use super::pockets::Stamp;
use super::types::{Publication, PublishOutcome};
use crate::clock::Clock;
use crate::error::PublishError;
use crate::ids::PublisherId;
use crate::lease_ids::LeaseIds;
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Publish`: the five publish rules, by their disjoint guards on the pocket,
    /// the stored rank of the record key and the record's state. A new pocket takes
    /// `next_position` and its record the next; a new key takes `next_position`. A
    /// higher rank supersedes a version nobody holds in place and sheds the loser
    /// (`Supersede`); over a held record with nothing deferred it is deferred behind
    /// the held version at `next_position`, its arrival position, and nothing is
    /// shed (`DeferBehindHeldVersion`); both answer [`PublishOutcome::Superseded`].
    /// The stored rank is the latest version's whatever the record's state, so a
    /// lower rank is shed as stale. A publish that adds or raises a record leaves a
    /// queued or leased pocket where it is and revives an idle or parked one at the
    /// back (`PocketTouched`); a stale publish leaves its pocket alone, so a parked
    /// pocket stays parked with its count. A shed payload is stamped `shed_at = now`
    /// and `expires_at = now + superseded_ttl`.
    ///
    /// # Errors
    ///
    /// [`PublishError::UnknownPublisher`] when the publisher has not joined, checked
    /// first, and [`PublishError::EqualRank`] when the rank equals the stored one:
    /// no rule covers it, so the specification rejects it. Either way nothing
    /// changes.
    ///
    /// ```
    /// use libpachinko::{PublishError, PublishOutcome, PullOutcome};
    /// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
    /// # use libpachinko::{
    /// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank,
    /// #     RecordKey,
    /// # };
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// # let p = PublisherId::new("p");
    /// # queue.publisher_joins(p.clone()).unwrap();
    /// let publish = |key: &str, rank| Publication::new(PocketKey::new("pk"), RecordKey::new(key), Rank::new(rank), Payload::default());
    /// assert_eq!(queue.publish(&p, publish("a", 2)), Ok(PublishOutcome::Created));
    /// assert_eq!(queue.publish(&p, publish("b", 1)), Ok(PublishOutcome::NewKey));
    /// assert_eq!(queue.publish(&p, publish("a", 3)), Ok(PublishOutcome::Superseded));
    /// assert_eq!(queue.publish(&p, publish("a", 1)), Ok(PublishOutcome::Stale));
    /// assert!(matches!(queue.publish(&p, publish("a", 3)), Err(PublishError::EqualRank { .. })));
    /// # let c = ConsumerId::new("c");
    /// # queue.consumer_joins(c.clone()).unwrap();
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert_eq!((records[0].key.as_str(), records[0].rank), ("a", Rank::new(3)));
    /// assert_eq!(queue.publish(&p, publish("a", 4)), Ok(PublishOutcome::Superseded));
    /// let held = &queue.lease_view(&c).unwrap().unwrap().records[0];
    /// assert_eq!(held.rank, Rank::new(3), "the held version stays until the lease ends");
    /// ```
    pub fn publish(
        &mut self,
        publisher_id: &PublisherId,
        publication: Publication,
    ) -> Result<PublishOutcome, PublishError> {
        if !self.publishers.contains(publisher_id) {
            return Err(PublishError::UnknownPublisher(publisher_id.clone()));
        }
        let now = self.now();
        let stamp = Stamp {
            shed_at: now,
            expires_at: now + self.config.superseded_ttl,
        };
        let (outcome, shed) = self.pockets.publish(publication, stamp)?;
        if let Some(record) = shed {
            self.shed.keep(record);
        }
        Ok(outcome)
    }
}
