//! The values the queue's operations take and return, and the entities its
//! inspection seam shows.

use crate::clock::Timestamp;
use crate::ids::{ConsumerId, Payload, PocketKey, Rank, RecordKey};
use crate::lease_ids::LeaseId;
use crate::records::HeldRecord;

/// One publish: the pocket, the record key, its rank and its payload.
///
/// ```
/// use libpachinko::ids::{Payload, PocketKey, Rank, RecordKey};
/// use libpachinko::queue::Publication;
///
/// let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
/// assert_eq!(publication.rank, Rank::new(1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Publication {
    /// The pocket published into.
    pub pocket_key: PocketKey,
    /// The record key.
    pub record_key: RecordKey,
    /// The publisher's rank; higher wins.
    pub rank: Rank,
    /// The opaque bytes.
    pub payload: Payload,
}

impl Publication {
    /// A publication with these parts.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, PocketKey, Rank, RecordKey};
    /// use libpachinko::queue::Publication;
    ///
    /// let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
    /// assert_eq!(publication.pocket_key.as_str(), "pk");
    /// ```
    #[must_use]
    pub const fn new(
        pocket_key: PocketKey,
        record_key: RecordKey,
        rank: Rank,
        payload: Payload,
    ) -> Self {
        Self {
            pocket_key,
            record_key,
            rank,
            payload,
        }
    }
}

/// Which publish rule took a publish.
///
/// ```
/// use libpachinko::queue::PublishOutcome;
/// assert_ne!(PublishOutcome::Created, PublishOutcome::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PublishOutcome {
    /// `PublishIntoNewPocket`: a new pocket with its first record.
    Created,
    /// `PublishNewKey`: a new record in an existing pocket.
    NewKey,
    /// `Supersede`: the record took the higher rank and its payload in place; the
    /// loser is a superseded record.
    Superseded,
    /// `DropStalePublish`: the rank was lower than stored; the arrival is a
    /// superseded record marked stale, and the record is unchanged.
    Stale,
}

/// What a pull hands back.
///
/// ```
/// use libpachinko::queue::PullOutcome;
/// assert_eq!(PullOutcome::NoPocketAvailable, PullOutcome::NoPocketAvailable);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PullOutcome {
    /// A batch: the lease it was pulled under, the pocket, the lease's `expires_at`
    /// after this pull, and the records now held, oldest first, possibly none.
    Batch {
        /// The lease the records are held under.
        lease: LeaseId,
        /// The leased pocket.
        pocket_key: PocketKey,
        /// When the lease lapses, as this pull left it.
        expires_at: Timestamp,
        /// The records handed out, at their held version, smallest position first.
        records: Vec<HeldRecord>,
    },
    /// `PullFindsNoPocket`: the consumer holds no lease and no pocket is queued.
    /// An outcome, never an error.
    NoPocketAvailable,
}

/// The `Consuming` surface's `exposes` for one consumer's active lease: the pocket
/// key, `expires_at` and the held records at their held rank and payload.
///
/// ```
/// use libpachinko::clock::Timestamp;
/// use libpachinko::ids::PocketKey;
/// use libpachinko::queue::LeaseView;
///
/// let view = LeaseView::new(PocketKey::new("pk"), Timestamp::from_millis(5), Vec::new());
/// assert_eq!(view.expires_at, Timestamp::from_millis(5));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct LeaseView {
    /// The leased pocket.
    pub pocket_key: PocketKey,
    /// When the lease lapses.
    pub expires_at: Timestamp,
    /// The held records, smallest position first.
    pub records: Vec<HeldRecord>,
}

impl LeaseView {
    /// A lease view with these parts.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
    /// use libpachinko::ids::PocketKey;
    /// use libpachinko::queue::LeaseView;
    ///
    /// assert!(LeaseView::new(PocketKey::new("pk"), Timestamp::default(), Vec::new()).records.is_empty());
    /// ```
    #[must_use]
    pub const fn new(
        pocket_key: PocketKey,
        expires_at: Timestamp,
        records: Vec<HeldRecord>,
    ) -> Self {
        Self {
            pocket_key,
            expires_at,
            records,
        }
    }
}

/// A pocket's state.
///
/// ```
/// use libpachinko::queue::PocketStatus;
/// assert_ne!(PocketStatus::Queued, PocketStatus::Parked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PocketStatus {
    /// In the pocket order, waiting for a consumer.
    Queued,
    /// Under one active lease.
    Leased,
    /// No records.
    Idle,
    /// Set aside after consecutive lease expiries, until a publish revives it.
    Parked,
}

/// A lease's state. `ended_at` exists only once it has ended.
///
/// ```
/// use libpachinko::clock::Timestamp;
/// use libpachinko::queue::LeaseStatus;
///
/// let ended = LeaseStatus::Released { ended_at: Timestamp::from_millis(1) };
/// assert_ne!(ended, LeaseStatus::Active);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LeaseStatus {
    /// Its consumer may pull, ack, nack, renew and release under it.
    Active,
    /// Its consumer released it.
    Released {
        /// When it ended.
        ended_at: Timestamp,
    },
    /// It lapsed.
    Expired {
        /// When it ended.
        ended_at: Timestamp,
    },
}

/// One lease, as the inspection seam and a lapse report show it.
///
/// ```
/// use libpachinko::queue::LeaseStatus;
/// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
/// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let consumer = ConsumerId::new("c");
/// # queue.consumer_joins(consumer.clone()).unwrap();
/// # let publication = |rank| Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(rank), Payload::default());
/// queue.publish(&publisher, publication(1)).unwrap();
/// queue.pull(&consumer, 1).unwrap();
/// # #[cfg(feature = "testing")] {
/// let lease = &queue.snapshot().leases[0];
/// assert_eq!(lease.status, LeaseStatus::Active);
/// assert!(lease.granted_at <= lease.expires_at);
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Lease {
    /// Its id.
    pub id: LeaseId,
    /// The leased pocket.
    pub pocket_key: PocketKey,
    /// The consumer holding it.
    pub consumer_id: ConsumerId,
    /// Active, released or expired.
    pub status: LeaseStatus,
    /// When it was granted.
    pub granted_at: Timestamp,
    /// When it lapses, or lapsed.
    pub expires_at: Timestamp,
}

/// Why a payload became a superseded record.
///
/// ```
/// use libpachinko::queue::ShedReason;
/// assert_ne!(ShedReason::Superseded, ShedReason::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShedReason {
    /// It lost a supersede.
    Superseded,
    /// It arrived with a lower rank than stored.
    Stale,
}

/// A payload kept visible until it decays: the loser of a supersede, or a stale
/// arrival (`EveryDropIsVisible`).
///
/// ```
/// use libpachinko::queue::ShedReason;
/// # use libpachinko::{clock::FakeClock, config::Config, ids::*, lease_ids::CountingLeaseIds};
/// # use libpachinko::{queue::{Publication, Queue}, records::MemoryRecords};
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let consumer = ConsumerId::new("c");
/// # queue.consumer_joins(consumer.clone()).unwrap();
/// # let publication = |rank| Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(rank), Payload::default());
/// queue.publish(&publisher, publication(2)).unwrap();
/// queue.publish(&publisher, publication(1)).unwrap();
/// # #[cfg(feature = "testing")] {
/// let shed = &queue.snapshot().superseded[0];
/// assert_eq!(shed.reason, ShedReason::Stale);
/// assert!(shed.shed_at <= shed.expires_at);
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SupersededRecord {
    /// The pocket it was published into.
    pub pocket_key: PocketKey,
    /// The record key.
    pub key: RecordKey,
    /// Its rank.
    pub rank: Rank,
    /// Its payload.
    pub payload: Payload,
    /// Superseded or stale.
    pub reason: ShedReason,
    /// When it was shed.
    pub shed_at: Timestamp,
    /// When it decays: `shed_at` plus `superseded_ttl`.
    pub expires_at: Timestamp,
}
