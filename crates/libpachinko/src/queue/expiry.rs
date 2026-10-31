//! The clock's triggers: `LeaseLapses` with the three rules that process
//! `LeaseLapsed`, and the two decays.

use super::{Lease, LeaseId, LeaseStatus, Queue, TickReport};
use crate::clock::{Clock, Timestamp};
use crate::records::PocketRecords;

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// Processes what the clock has made due: every active lease whose expiry is
    /// at or before now lapses (`LeaseLapses`), earliest first, and the pocket
    /// parks at the expiry limit (`LeaseExpiryParksPocket`), keeps its place at the
    /// front with records (`LeaseExpiryKeepsPocketAtFront`) or goes idle without
    /// (`LeaseExpiryIdlesEmptyPocket`), its held and deferred records freed. Then
    /// every superseded record and dead letter whose expiry is at or before now
    /// decays (`SupersededRecordDecays`, `DeadLetterDecays`).
    ///
    /// The owner calls it in the same stream as every other trigger, so an action
    /// processed first stops a lapse at its guards.
    ///
    /// ```
    /// use libpachinko::clock::{FakeClock, Timestamp};
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::{PocketStatus, Publication, Queue};
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let clock = FakeClock::new(Timestamp::from_millis(0));
    /// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), clock.clone()).unwrap();
    /// queue.publisher_joins("p").unwrap();
    /// queue.consumer_joins("c").unwrap();
    /// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
    /// queue.pull("c", 10).unwrap();
    /// assert!(queue.tick().lapsed.is_empty());
    /// clock.advance(Config::default().lease_ttl);
    /// assert_eq!(queue.tick().lapsed.len(), 1);
    /// ```
    pub fn tick(&mut self) -> TickReport {
        let now = self.clock.now();
        let lapsed = self
            .leases
            .due(now)
            .into_iter()
            .filter_map(|lease| self.lapse(lease, now))
            .collect();
        let (decayed_superseded, decayed_dead_letters) = self.shed.decay(now);
        TickReport {
            lapsed,
            decayed_superseded,
            decayed_dead_letters,
        }
    }

    /// One `LeaseLapsed`: the lease ends as expired and its pocket takes the
    /// expiry rule its count and records select.
    fn lapse(&mut self, lease: LeaseId, now: Timestamp) -> Option<Lease> {
        let ended = self.leases.end(lease, LeaseStatus::Expired, now)?;
        self.consumers.set_lease(&ended.consumer_id, None);
        self.pockets
            .lapse(&ended.pocket_key, self.config.max_consecutive_expiries);
        Some(ended)
    }
}
