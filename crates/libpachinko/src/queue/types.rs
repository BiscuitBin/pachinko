//! The values the queue's operations take and return, and the entities its
//! inspection seam shows. [`PocketStatus`], [`ShedReason`], [`SupersededRecord`] and
//! [`DeadLetter`] are the second kind: the queue keeps them in every build, and the
//! `testing` module alone exports them.

use crate::ids::{ConsumerId, HeldRecord, LeaseId, Payload, PocketKey, Rank, RecordKey};
use crate::timestamp::Timestamp;

/// One publish: the pocket, the record key, its rank and its payload.
///
/// ```
/// use libpachinko::{Payload, PocketKey, Publication, Rank, RecordKey};
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
    /// use libpachinko::{Payload, PocketKey, Publication, Rank, RecordKey};
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
/// use libpachinko::PublishOutcome;
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
/// use libpachinko::PullOutcome;
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
/// use libpachinko::{LeaseView, PocketKey, Timestamp};
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
    /// use libpachinko::{LeaseView, PocketKey, Timestamp};
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
/// # #[cfg(feature = "testing")] {
/// use libpachinko::testing::PocketStatus;
/// assert_ne!(PocketStatus::Queued, PocketStatus::Parked);
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(
    not(feature = "testing"),
    expect(
        unreachable_pub,
        reason = "exported by the testing module alone until open question 2 is answered"
    )
)]
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
/// use libpachinko::{LeaseStatus, Timestamp};
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

/// One lease, as a release and a lapse report hand it back, ended, and as the
/// inspection seam shows it, active.
///
/// ```
/// use libpachinko::{LeaseStatus, PullOutcome};
/// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
/// # use libpachinko::{
/// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank, RecordKey,
/// # };
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let consumer = ConsumerId::new("c");
/// # queue.consumer_joins(consumer.clone()).unwrap();
/// # let publication = |rank| Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(rank), Payload::default());
/// queue.publish(&publisher, publication(1)).unwrap();
/// let Ok(PullOutcome::Batch { lease: id, .. }) = queue.pull(&consumer, 1) else { panic!() };
/// let (_, lease) = queue.release(&consumer, id).unwrap();
/// assert_eq!((lease.id, &lease.consumer_id), (id, &consumer));
/// assert_eq!(lease.status, LeaseStatus::Released { ended_at: lease.granted_at });
/// assert!(lease.granted_at <= lease.expires_at);
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
/// # #[cfg(feature = "testing")] {
/// use libpachinko::testing::ShedReason;
/// assert_ne!(ShedReason::Superseded, ShedReason::Stale);
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(
    not(feature = "testing"),
    expect(
        unreachable_pub,
        reason = "exported by the testing module alone until open question 2 is answered"
    )
)]
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
/// # #[cfg(feature = "testing")] {
/// use libpachinko::testing::ShedReason;
/// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
/// # use libpachinko::{
/// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank, RecordKey,
/// # };
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let consumer = ConsumerId::new("c");
/// # queue.consumer_joins(consumer.clone()).unwrap();
/// # let publication = |rank| Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(rank), Payload::default());
/// queue.publish(&publisher, publication(2)).unwrap();
/// queue.publish(&publisher, publication(1)).unwrap();
/// let shed = &queue.snapshot().superseded[0];
/// assert_eq!(shed.reason, ShedReason::Stale);
/// assert!(shed.shed_at <= shed.expires_at);
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(
    not(feature = "testing"),
    expect(
        unreachable_pub,
        reason = "exported by the testing module alone until open question 2 is answered"
    )
)]
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

/// Which rule took an ack or nack.
///
/// ```
/// use libpachinko::SettleOutcome;
/// assert_ne!(SettleOutcome::Settled, SettleOutcome::Deferred);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SettleOutcome {
    /// `AckSettlesRecord` or `NackSettlesRecord`: the held version was the latest,
    /// and the record is gone.
    Settled,
    /// `AckDefersNewerVersion` or `NackDefersNewerVersion`: a newer version arrived
    /// while the key was held, so the record is deferred at a fresh position until
    /// the lease ends.
    Deferred,
}

/// Where a release left the pocket.
///
/// ```
/// use libpachinko::PocketFate;
/// assert_ne!(PocketFate::Queued, PocketFate::Idle);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PocketFate {
    /// `ReleaseRequeuesPocket`: it had records, held, deferred or queued, and is
    /// back in the pocket order at the back.
    Queued,
    /// `ReleaseIdlesEmptyPocket`: it had no records and is idle until a publish
    /// revives it.
    Idle,
}

/// A nacked payload, kept visible until it decays.
///
/// ```
/// # #[cfg(feature = "testing")] {
/// use libpachinko::{PullOutcome, SettleOutcome};
/// # use libpachinko::{clock::FakeClock, lease_ids::CountingLeaseIds, records::MemoryRecords};
/// # use libpachinko::{
/// #     Config, ConsumerId, Payload, PocketKey, Publication, PublisherId, Queue, Rank, RecordKey,
/// # };
/// # let (clock, ids) = (FakeClock::default(), CountingLeaseIds::default());
/// # let mut queue: Queue<MemoryRecords, _, _> = Queue::new(Config::default(), clock, ids).unwrap();
/// # let publisher = PublisherId::new("p");
/// # queue.publisher_joins(publisher.clone()).unwrap();
/// # let consumer = ConsumerId::new("c");
/// # queue.consumer_joins(consumer.clone()).unwrap();
/// # let publication = Publication::new(PocketKey::new("pk"), RecordKey::new("rk"), Rank::new(1), Payload::default());
/// queue.publish(&publisher, publication).unwrap();
/// let Ok(PullOutcome::Batch { lease, .. }) = queue.pull(&consumer, 1) else { panic!() };
/// assert_eq!(queue.nack(&consumer, lease, &RecordKey::new("rk")), Ok(SettleOutcome::Settled));
/// let letter = &queue.snapshot().dead_letters[0];
/// assert_eq!(letter.nacked_by, consumer);
/// assert!(letter.nacked_at <= letter.expires_at);
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(
    not(feature = "testing"),
    expect(
        unreachable_pub,
        reason = "exported by the testing module alone until open question 2 is answered"
    )
)]
pub struct DeadLetter {
    /// The pocket the record was in.
    pub pocket_key: PocketKey,
    /// The record key.
    pub key: RecordKey,
    /// The held rank that was nacked.
    pub rank: Rank,
    /// The held payload that was nacked.
    pub payload: Payload,
    /// The consumer that nacked it.
    pub nacked_by: ConsumerId,
    /// When it was nacked.
    pub nacked_at: Timestamp,
    /// When it decays: `nacked_at` plus `dead_letter_ttl`.
    pub expires_at: Timestamp,
}
