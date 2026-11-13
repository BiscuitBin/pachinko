//! The clock's triggers: `LeaseLapses`, with the three rules that process
//! `LeaseLapsed` (`LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` and
//! `LeaseExpiryIdlesEmptyPocket`), and the two decays, `SupersededRecordDecays` and
//! `DeadLetterDecays`.
//!
//! A tick is one operation like any other: whoever owns the queue applies it
//! between two actions, never inside one, so an action processed first stops a
//! lapse at its guards and a lapse processed first refuses the action (decision
//! 0019, the specification's Lease expiry comment).

use super::Queue;
use super::types::{Lease, LeaseStatus};
use crate::clock::Clock;
use crate::ids::LeaseId;
use crate::lease_ids::LeaseIds;
use crate::records::PocketRecords;
use crate::timestamp::Timestamp;

/// What one [`Queue::tick`] processed.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::clock::FakeClock;
/// use libpachinko::{LeaseStatus, TickReport, Timestamp};
/// # use libpachinko::{lease_ids::CountingLeaseIds, records::MemoryRecords};
/// # use libpachinko::{
/// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank, RecordKey,
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
/// queue.pull(&c, 1).unwrap();
/// clock.advance(Duration::from_millis(10));
/// let report: TickReport = queue.tick();
/// assert_eq!(report.lapsed[0].status, LeaseStatus::Expired { ended_at: Timestamp::from_millis(10) });
/// assert_eq!((report.decayed_superseded, report.decayed_dead_letters), (0, 0));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TickReport {
    /// The leases that lapsed, earliest `expires_at` first, each as it ended.
    pub lapsed: Vec<Lease>,
    /// How many superseded records decayed.
    pub decayed_superseded: usize,
    /// How many dead letters decayed.
    pub decayed_dead_letters: usize,
}

impl TickReport {
    /// Whether the tick lapsed or decayed anything.
    ///
    /// ```
    /// use libpachinko::TickReport;
    /// assert!(TickReport::default().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        [
            self.lapsed.len(),
            self.decayed_superseded,
            self.decayed_dead_letters,
        ]
        .iter()
        .all(|&count| count == 0)
    }
}

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// Processes everything the clock has made due, reading `now` once: every
    /// active lease whose `expires_at` is at or before `now` lapses, earliest
    /// first (`LeaseLapses`), and then every superseded record and dead letter
    /// whose `expires_at` is at or before `now` decays. The cost is what is due,
    /// never what is active or kept.
    ///
    /// A lapsed lease ends as [`LeaseStatus::Expired`] with `ended_at = now` and
    /// leaves the registry, so a second tick finds nothing, and its consumer holds
    /// no lease. Its pocket's expiry count rises by one and its held and deferred
    /// records return to queued; then the pocket parks once the count reaches
    /// `max_consecutive_expiries`, keeps its place at the front with records, or
    /// goes idle without. A lease expiring while a tick runs waits for the next.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::{LeaseActionError, PullOutcome, Timestamp};
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
    /// clock.advance(Duration::from_millis(9));
    /// assert!(queue.tick().lapsed.is_empty());
    /// clock.advance(Duration::from_millis(1));
    /// assert_eq!(queue.tick().lapsed[0].id, lease);
    /// assert!(queue.tick().is_empty());
    /// assert_eq!(queue.renew(&c, lease), Err(LeaseActionError::LeaseNotActive(lease)));
    /// let Ok(PullOutcome::Batch { records, .. }) = queue.pull(&c, 1) else { panic!() };
    /// assert_eq!(records[0].key.as_str(), "rk");
    /// ```
    pub fn tick(&mut self) -> TickReport {
        self.tick_counting().0
    }

    /// The tick itself, with what it read to find what was due beside what it
    /// processed: the report, then the expiry-index entries the lapse walk read,
    /// then the shed entries the decay read. [`Queue::tick`] drops the two counts;
    /// the `testing` module's `Queue::tick_with_cost` returns them.
    pub(super) fn tick_counting(&mut self) -> (TickReport, usize, usize) {
        let now = self.now();
        let due = self.leases.due(now);
        let lapsed = due
            .ids
            .into_iter()
            .filter_map(|lease| self.lapse(lease, now))
            .collect();
        let decay = self.shed.decay(now);
        let report = TickReport {
            lapsed,
            decayed_superseded: decay.superseded,
            decayed_dead_letters: decay.dead_letters,
        };
        (report, due.examined, decay.examined)
    }

    /// One `LeaseLapsed`: the lease ends as expired at `now`, its consumer holds
    /// no lease, and its pocket takes the expiry rule its count and records
    /// select. `None` when the lease is no longer active.
    fn lapse(&mut self, lease: LeaseId, now: Timestamp) -> Option<Lease> {
        let ended = self
            .leases
            .end(lease, LeaseStatus::Expired { ended_at: now })?;
        if let Some(active) = self.consumers.get_mut(&ended.consumer_id) {
            *active = None;
        }
        self.pockets
            .lapse(&ended.pocket_key, self.config.max_consecutive_expiries);
        Some(ended)
    }
}

// The pockets' iterator is the inspection seam's, behind `testing`.
#[cfg(test)]
#[cfg(feature = "testing")]
mod tests {
    use crate::clock::FakeClock;
    use crate::config::Config;
    use crate::ids::{ConsumerId, LeaseId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use crate::lease_ids::CountingLeaseIds;
    use crate::queue::types::PocketStatus;
    use crate::queue::{Publication, PullOutcome, Queue};
    use crate::records::MemoryRecords;
    use crate::timestamp::Timestamp;

    /// A queue with pocket `pk` leased by `c` under the returned lease.
    fn leased() -> (Queue<MemoryRecords, FakeClock, CountingLeaseIds>, LeaseId) {
        let clock = FakeClock::new(Timestamp::from_millis(0));
        let mut queue = Queue::new(Config::default(), clock, CountingLeaseIds::default()).unwrap();
        let (p, c) = (PublisherId::new("p"), ConsumerId::new("c"));
        queue.publisher_joins(p.clone()).unwrap();
        queue.consumer_joins(c.clone()).unwrap();
        let publication = Publication::new(
            PocketKey::new("pk"),
            RecordKey::new("rk"),
            Rank::new(1),
            Payload::default(),
        );
        queue.publish(&p, publication).unwrap();
        let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&c, 1) else {
            panic!("expected a batch")
        };
        (queue, lease)
    }

    #[test]
    fn a_lapse_of_a_lease_no_longer_active_changes_nothing() {
        let (mut queue, lease) = leased();
        let now = Timestamp::from_millis(0);
        assert!(queue.lapse(lease, now).is_some());
        let count = |queue: &Queue<_, _, _>| {
            queue
                .pockets
                .all()
                .map(|(_, pocket)| (pocket.status, pocket.consecutive_expiries))
                .collect::<Vec<_>>()
        };
        let before = count(&queue);
        assert_eq!(before, [(PocketStatus::Queued, 1)]);
        assert_eq!(queue.lapse(lease, now), None);
        assert_eq!(count(&queue), before);
    }

    #[test]
    fn the_pocket_step_of_a_lapse_leaves_a_pocket_that_is_not_leased_alone() {
        let (mut queue, lease) = leased();
        queue.release(&ConsumerId::new("c"), lease).unwrap();
        let key = PocketKey::new("pk");
        queue.pockets.lapse(&key, 1);
        queue.pockets.lapse(&PocketKey::new("absent"), 1);
        let (_, pocket) = queue.pockets.all().next().unwrap();
        assert_eq!(
            (pocket.status, pocket.consecutive_expiries),
            (PocketStatus::Queued, 0)
        );
    }
}
