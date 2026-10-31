//! Publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede` and
//! `DropStalePublish`, each with the `PocketTouched` revive it emits.

use super::pockets::{Pockets, Target};
use super::shed::{Shed, Stamp};
use super::{Publication, PublishOutcome, Queue};
use crate::clock::Clock;
use crate::error::PublishError;
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// `Publish`: the four publish rules, with disjoint guards on the pocket and
    /// the record key's stored rank. A publish into an existing pocket also
    /// revives it if it is idle or parked (`ReviveIdlePocket`,
    /// `ReviveParkedPocket`), at the next position.
    ///
    /// # Errors
    ///
    /// [`PublishError::UnknownPublisher`] when no publisher with this id has
    /// joined, and [`PublishError::EqualRank`] when the rank equals the stored
    /// one, which no rule covers (the module's first open question). Either way
    /// the queue is unchanged.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{Publication, PublishOutcome, Queue};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// let publish = |rank| Publication::new("pk", "rk", rank, Vec::new());
    /// assert_eq!(queue.publish("p", publish(2)).unwrap(), PublishOutcome::New);
    /// assert_eq!(queue.publish("p", publish(3)).unwrap(), PublishOutcome::Superseded);
    /// assert_eq!(queue.publish("p", publish(1)).unwrap(), PublishOutcome::Stale);
    /// assert!(queue.publish("p", publish(3)).is_err());
    /// ```
    pub fn publish(
        &mut self,
        publisher_id: &str,
        publication: Publication,
    ) -> Result<PublishOutcome, PublishError> {
        self.publishers.require(publisher_id)?;
        let target = self.pockets.target(&publication)?;
        let now = self.now();
        let stamp = Stamp {
            at: now,
            expires_at: now.saturating_add(self.config.superseded_ttl),
        };
        Ok(match target {
            Target::NewPocket => into_new_pocket(&mut self.pockets, publication),
            Target::NewKey => new_key(&mut self.pockets, publication),
            Target::Supersede => supersede(&mut self.pockets, &mut self.shed, publication, stamp),
            Target::Stale => drop_stale(&mut self.pockets, &mut self.shed, publication, stamp),
        })
    }
}

/// `PublishIntoNewPocket`.
fn into_new_pocket<R: PocketRecords>(
    pockets: &mut Pockets<R>,
    publication: Publication,
) -> PublishOutcome {
    pockets.create(publication);
    PublishOutcome::New
}

/// `PublishNewKey`, then `PocketTouched`.
fn new_key<R: PocketRecords>(pockets: &mut Pockets<R>, publication: Publication) -> PublishOutcome {
    pockets.add_key(publication);
    PublishOutcome::New
}

/// `Supersede`, then `PocketTouched`; the replaced version is shed.
fn supersede<R: PocketRecords>(
    pockets: &mut Pockets<R>,
    shed: &mut Shed,
    publication: Publication,
    stamp: Stamp,
) -> PublishOutcome {
    pockets
        .supersede(publication, stamp)
        .into_iter()
        .for_each(|record| shed.keep_superseded(record));
    PublishOutcome::Superseded
}

/// `DropStalePublish`, then `PocketTouched`; the stale version is shed.
fn drop_stale<R: PocketRecords>(
    pockets: &mut Pockets<R>,
    shed: &mut Shed,
    publication: Publication,
    stamp: Stamp,
) -> PublishOutcome {
    shed.keep_superseded(pockets.drop_stale(publication, stamp));
    PublishOutcome::Stale
}
