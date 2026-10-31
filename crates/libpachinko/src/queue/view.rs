//! The inspection seam, behind the `testing` feature: every entity of the queue
//! as a value, for tests that check the module's invariants and transitions.
//!
//! It is the only reader of the superseded records and dead letters, and it is
//! not part of the default API: who reads those queues is the module's second open
//! question.

use super::{DeadLetter, Lease, LeaseId, PocketStatus, Queue, SupersededRecord};
use crate::clock::Clock;
use crate::records::{PocketRecords, RecordView};

/// One pocket and its records.
///
/// ```
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::{PocketStatus, Publication, Queue};
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.publisher_joins("p").unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
/// let pocket = &queue.snapshot().pockets[0];
/// assert_eq!((pocket.key.as_str(), pocket.status, pocket.position), ("pk", PocketStatus::Queued, 0));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PocketView {
    /// The pocket key.
    pub key: String,
    /// Queued, leased, idle or parked.
    pub status: PocketStatus,
    /// Its position in the pocket order, or where it last stood there.
    pub position: u64,
    /// How many leases in a row have lapsed on it.
    pub consecutive_expiries: u32,
    /// Its records, by key.
    pub records: Vec<RecordView>,
}

/// Every entity of the queue, by key or id.
///
/// ```
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::Queue;
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.consumer_joins("c").unwrap();
/// let snapshot = queue.snapshot();
/// assert_eq!(snapshot.next_position, 0);
/// assert_eq!(snapshot.consumers, vec![("c".to_owned(), None)]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct QueueSnapshot {
    /// The queue's `next_position`.
    pub next_position: u64,
    /// The publishers' ids.
    pub publishers: Vec<String>,
    /// The consumers' ids, each with its active lease.
    pub consumers: Vec<(String, Option<LeaseId>)>,
    /// The pockets, by key.
    pub pockets: Vec<PocketView>,
    /// The active leases, by id.
    pub leases: Vec<Lease>,
    /// The superseded and stale records, by expiry.
    pub superseded: Vec<SupersededRecord>,
    /// The dead letters, by expiry.
    pub dead_letters: Vec<DeadLetter>,
}

impl<R: PocketRecords, C: Clock> Queue<R, C> {
    /// Every entity of the queue, as values.
    ///
    /// ```
    /// use libpachinko::clock::FakeClock;
    /// use libpachinko::config::Config;
    /// use libpachinko::queue::Queue;
    /// use libpachinko::records::MemoryRecords;
    ///
    /// let queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
    /// assert!(queue.snapshot().pockets.is_empty());
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            next_position: self.pockets.next_position(),
            publishers: self.publishers.snapshot(),
            consumers: self.consumers.snapshot(),
            pockets: self
                .pockets
                .all()
                .map(|pocket| PocketView {
                    key: pocket.key.clone(),
                    status: pocket.status,
                    position: pocket.position,
                    consecutive_expiries: pocket.consecutive_expiries,
                    records: pocket.records.snapshot(),
                })
                .collect(),
            leases: self.leases.snapshot(),
            superseded: self.shed.superseded(),
            dead_letters: self.shed.dead_letters(),
        }
    }
}
