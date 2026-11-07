//! Publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede` and
//! `DropStalePublish`, each but the first ending in the `PocketTouched` dispatch.

use super::Queue;
use super::pockets::Stamp;
use super::types::{Publication, PublishOutcome};
use crate::clock::Clock;
use crate::error::PublishError;
use crate::ids::PublisherId;
use crate::lease_ids::LeaseIds;
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// `Publish`: the four publish rules, by their disjoint guards on the pocket and
    /// the stored rank of the record key. A new pocket takes `next_position` and its
    /// record the next; a new key takes `next_position`; a higher rank supersedes in
    /// place and sheds the loser; a lower rank is shed as stale. A publish into an
    /// existing pocket leaves a queued or leased pocket where it is, and revives an
    /// idle or parked one at the back (`PocketTouched`). A shed payload is stamped
    /// `shed_at = now` and `expires_at = now + superseded_ttl`.
    ///
    /// # Errors
    ///
    /// [`PublishError::UnknownPublisher`] when the publisher has not joined, checked
    /// first, and [`PublishError::EqualRank`] when the rank equals the stored one:
    /// no rule covers it, a placeholder for open question 1. Either way nothing
    /// changes.
    ///
    /// ```
    /// use libpachinko::error::PublishError;
    /// use libpachinko::queue::PublishOutcome;
    /// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
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
