//! The inspection seam, behind the `testing` feature: every entity of the queue as
//! a value, enough for a test to check every invariant without a second seam.
//!
//! No surface exposes it, and the service does not serve it: who reads the shed
//! queue is the specification's second open question. Each collection is a `Vec`,
//! so a duplicate an invariant forbids would show rather than be merged away.

use super::Queue;
use super::types::{Lease, LeaseStatus, PocketStatus, SupersededRecord};
use crate::clock::Clock;
use crate::ids::{ConsumerId, PocketKey, Position, PublisherId};
use crate::lease_ids::{LeaseId, LeaseIds};
use crate::records::{PocketRecords, RecordView};

/// One pocket and its records.
///
/// ```
/// use libpachinko::ids::Position;
/// use libpachinko::queue::PocketStatus;
/// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
/// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
/// queue.publish(&publisher, publication).unwrap();
/// let pocket = &queue.snapshot().pockets[0];
/// assert_eq!((pocket.status, pocket.position), (PocketStatus::Queued, Position::new(0)));
/// assert_eq!(pocket.records[0].position, Position::new(1));
/// assert_eq!(queue.snapshot().pocket_order, vec![(Position::new(0), PocketKey::new("pk"))]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PocketView {
    /// The pocket key.
    pub key: PocketKey,
    /// Queued, leased, idle or parked.
    pub status: PocketStatus,
    /// Its position in the pocket order, or where it last stood there.
    pub position: Position,
    /// How many of its leases in a row have lapsed.
    pub consecutive_expiries: u32,
    /// Its records, by key.
    pub records: Vec<RecordView>,
}

/// Every entity of the queue.
///
/// ```
/// use libpachinko::ids::{ConsumerId, Position};
/// # use libpachinko::{clock::FakeClock, config::Config, lease_ids::CountingLeaseIds};
/// # use libpachinko::{queue::Queue, records::MemoryRecords};
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// let snapshot = queue.snapshot();
/// assert_eq!(snapshot.next_position, Position::new(0));
/// assert_eq!(snapshot.consumers, vec![(ConsumerId::new("c"), None)]);
/// assert!(snapshot.pocket_order.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Snapshot {
    /// The specification's `Queue.next_position`.
    pub next_position: Position,
    /// The publishers' ids, in order.
    pub publishers: Vec<PublisherId>,
    /// The consumers' ids, in order, each with its active lease.
    pub consumers: Vec<(ConsumerId, Option<LeaseId>)>,
    /// The pockets, by key.
    pub pockets: Vec<PocketView>,
    /// The pocket order as the queue holds it, front first: the index a claim
    /// takes from, kept apart from `pockets` so a check can compare the two.
    pub pocket_order: Vec<(Position, PocketKey)>,
    /// The active leases, by id. An ended lease leaves the registry.
    pub leases: Vec<Lease>,
    /// The superseded and stale records, by expiry and arrival.
    pub superseded: Vec<SupersededRecord>,
}

impl<R: PocketRecords, C: Clock, L: LeaseIds> Queue<R, C, L> {
    /// Every entity of the queue, as values.
    ///
    /// ```
    /// # use libpachinko::{clock::FakeClock, config::Config, lease_ids::CountingLeaseIds};
    /// # use libpachinko::{queue::Queue, records::MemoryRecords};
    /// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
    /// let queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
    /// assert!(queue.snapshot().pockets.is_empty());
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            next_position: self.pockets.next_position(),
            publishers: self.publishers.iter().cloned().collect(),
            consumers: self
                .consumers
                .iter()
                .map(|(id, lease)| (id.clone(), *lease))
                .collect(),
            pockets: self
                .pockets
                .all()
                .map(|(key, pocket)| PocketView {
                    key: key.clone(),
                    status: pocket.status,
                    position: pocket.position,
                    consecutive_expiries: pocket.consecutive_expiries,
                    records: pocket.records.snapshot(),
                })
                .collect(),
            pocket_order: self.pockets.order().cloned().collect(),
            leases: self
                .leases
                .all()
                .map(|(id, lease)| Lease {
                    id: *id,
                    pocket_key: lease.pocket_key.clone(),
                    consumer_id: lease.consumer_id.clone(),
                    status: LeaseStatus::Active,
                    granted_at: lease.granted_at,
                    expires_at: lease.expires_at,
                })
                .collect(),
            superseded: self.shed.superseded(),
        }
    }
}
