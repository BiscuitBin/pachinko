//! The queue's public value types: ids, states, the trigger's input and the rules'
//! outcomes.

use super::LeaseId;
use crate::clock::Timestamp;
use crate::records::HeldRecord;

/// A pocket's state: `queued`, `leased`, `idle` or `parked`.
///
/// ```
/// use libpachinko::queue::PocketStatus;
///
/// assert_ne!(PocketStatus::Idle, PocketStatus::Parked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PocketStatus {
    /// In the pocket order, waiting for a consumer.
    Queued,
    /// Under one active lease.
    Leased,
    /// No records; out of the pocket order until a publish revives it.
    Idle,
    /// Set aside after consecutive lease expiries until a publish revives it.
    Parked,
}

/// A lease's state: `active`, or one of the two terminal states.
///
/// ```
/// use libpachinko::queue::LeaseStatus;
///
/// assert_ne!(LeaseStatus::Released, LeaseStatus::Expired);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LeaseStatus {
    /// The consumer's claim on its pocket.
    Active,
    /// Ended by the consumer's release.
    Released,
    /// Ended by a lapse.
    Expired,
}

/// A lease, as the queue reports one: active in an inspection, ended when a
/// release or a lapse returns it.
///
/// ```
/// use libpachinko::clock::{FakeClock, Timestamp};
/// use libpachinko::config::Config;
/// use libpachinko::queue::{LeaseStatus, Publication, PullOutcome, Queue};
/// use libpachinko::records::MemoryRecords;
///
/// let clock = FakeClock::new(Timestamp::from_millis(0));
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), clock).unwrap();
/// queue.publisher_joins("p").unwrap();
/// queue.consumer_joins("c").unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 1, Vec::new())).unwrap();
/// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
/// let ended = queue.release("c", batch.lease).unwrap();
/// assert_eq!(ended.status, LeaseStatus::Released);
/// assert_eq!(ended.ended_at, Some(Timestamp::from_millis(0)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Lease {
    /// The fencing token.
    pub id: LeaseId,
    /// The leased pocket.
    pub pocket_key: String,
    /// The consumer holding it.
    pub consumer_id: String,
    /// Active, released or expired.
    pub status: LeaseStatus,
    /// When it was granted.
    pub granted_at: Timestamp,
    /// When it lapses unless extended.
    pub expires_at: Timestamp,
    /// When it ended, present exactly once it is released or expired.
    pub ended_at: Option<Timestamp>,
}

/// One publish: the `Publish` trigger's pocket key, record key, rank and payload.
///
/// ```
/// use libpachinko::queue::Publication;
///
/// let publication = Publication::new("pocket", "record", 2, b"payload".to_vec());
/// assert_eq!(publication.rank, 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Publication {
    /// The pocket to publish into.
    pub pocket_key: String,
    /// The record key within the pocket.
    pub record_key: String,
    /// The version's rank; higher wins.
    pub rank: i64,
    /// The opaque bytes.
    pub payload: Vec<u8>,
}

impl Publication {
    /// A publication of `payload` at `rank` for `record_key` in `pocket_key`.
    ///
    /// ```
    /// use libpachinko::queue::Publication;
    ///
    /// assert_eq!(Publication::new("p", "r", 0, Vec::new()).pocket_key, "p");
    /// ```
    #[must_use]
    pub fn new(pocket_key: &str, record_key: &str, rank: i64, payload: Vec<u8>) -> Self {
        Self {
            pocket_key: pocket_key.to_owned(),
            record_key: record_key.to_owned(),
            rank,
            payload,
        }
    }
}

/// Which publish rule took a publish.
///
/// ```
/// use libpachinko::queue::PublishOutcome;
///
/// assert_ne!(PublishOutcome::New, PublishOutcome::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PublishOutcome {
    /// A new record, in a new pocket (`PublishIntoNewPocket`) or an existing one
    /// (`PublishNewKey`).
    New,
    /// A higher rank replaced the record's (`Supersede`); the old version is shed.
    Superseded,
    /// A lower rank than the record's (`DropStalePublish`); this version is shed.
    Stale,
}

/// What a pull returns: a batch under the consumer's lease, or no pocket.
///
/// ```
/// use libpachinko::queue::PullOutcome;
///
/// let outcome: PullOutcome<Vec<u8>> = PullOutcome::NoPocketAvailable;
/// assert_eq!(outcome, PullOutcome::NoPocketAvailable);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PullOutcome<B> {
    /// The consumer's lease, its pocket and the records handed out
    /// (`PullWithinLease`, `PullClaimsFrontPocket`).
    Batch(PulledBatch<B>),
    /// The consumer has no lease and no pocket is queued (`PullFindsNoPocket`).
    NoPocketAvailable,
}

/// The module's `Batch`: the lease, the pocket key, the lease's new expiry and the
/// records handed out, oldest first.
///
/// ```
/// use libpachinko::clock::{FakeClock, Timestamp};
/// use libpachinko::config::Config;
/// use libpachinko::queue::{Publication, PullOutcome, Queue};
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.publisher_joins("p").unwrap();
/// queue.consumer_joins("c").unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 1, b"x".to_vec())).unwrap();
/// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
/// assert_eq!(batch.pocket_key, "pk");
/// assert_eq!(batch.records[0].payload, b"x");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PulledBatch<B> {
    /// The lease the records were handed out under.
    pub lease: LeaseId,
    /// The leased pocket.
    pub pocket_key: String,
    /// When the lease lapses unless extended.
    pub expires_at: Timestamp,
    /// The records handed out, oldest first.
    pub records: B,
}

/// Which settle rule took an ack or nack.
///
/// ```
/// use libpachinko::queue::SettleOutcome;
///
/// assert_ne!(SettleOutcome::Settled, SettleOutcome::Deferred);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SettleOutcome {
    /// The held version was the latest; the record is gone.
    Settled,
    /// A newer version waited; the record is deferred to the back.
    Deferred,
}

/// What one tick processed: the leases that lapsed, and how many shed records and
/// dead letters decayed.
///
/// ```
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::Queue;
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// let report = queue.tick();
/// assert!(report.lapsed.is_empty());
/// assert_eq!((report.decayed_superseded, report.decayed_dead_letters), (0, 0));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TickReport {
    /// The leases that lapsed, each ended as expired, in expiry order.
    pub lapsed: Vec<Lease>,
    /// How many superseded and stale records decayed.
    pub decayed_superseded: usize,
    /// How many dead letters decayed.
    pub decayed_dead_letters: usize,
}

/// The `Consuming` surface's `exposes` for one consumer's active lease: the pocket
/// key, the expiry and each held record's key, held rank and held payload.
///
/// ```
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::Queue;
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.consumer_joins("c").unwrap();
/// assert_eq!(queue.lease_view("c").unwrap(), None);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct LeaseView {
    /// The active lease.
    pub lease: LeaseId,
    /// Its pocket.
    pub pocket_key: String,
    /// When it lapses unless extended.
    pub expires_at: Timestamp,
    /// The pocket's held records, by position.
    pub held: Vec<HeldRecord>,
}

/// Why a payload was shed.
///
/// ```
/// use libpachinko::queue::ShedReason;
///
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

/// A payload that lost a supersede or arrived stale, kept until it decays. No
/// public reader exists (the module's second open question); tests read it through
/// the `testing` inspection seam.
///
/// ```
/// # #[cfg(feature = "testing")] {
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::{Publication, Queue, ShedReason};
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.publisher_joins("p").unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 2, b"kept".to_vec())).unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 1, b"late".to_vec())).unwrap();
/// let shed = &queue.snapshot().superseded[0];
/// assert_eq!((shed.rank, shed.reason), (1, ShedReason::Stale));
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SupersededRecord {
    /// The pocket it was published into.
    pub pocket_key: String,
    /// Its record key.
    pub key: String,
    /// Its rank.
    pub rank: i64,
    /// Its payload.
    pub payload: Vec<u8>,
    /// Superseded or stale.
    pub reason: ShedReason,
    /// When it was shed.
    pub shed_at: Timestamp,
    /// When it decays.
    pub expires_at: Timestamp,
}

/// A nacked payload, kept until it decays. No public reader exists (the module's
/// second open question); tests read it through the `testing` inspection seam.
///
/// ```
/// # #[cfg(feature = "testing")] {
/// use libpachinko::clock::FakeClock;
/// use libpachinko::config::Config;
/// use libpachinko::queue::{Publication, PullOutcome, Queue};
/// use libpachinko::records::MemoryRecords;
///
/// let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), FakeClock::default()).unwrap();
/// queue.publisher_joins("p").unwrap();
/// queue.consumer_joins("c").unwrap();
/// queue.publish("p", Publication::new("pk", "rk", 1, b"x".to_vec())).unwrap();
/// let PullOutcome::Batch(batch) = queue.pull("c", 10).unwrap() else { panic!() };
/// queue.nack("c", batch.lease, "rk").unwrap();
/// assert_eq!(queue.snapshot().dead_letters[0].nacked_by, "c");
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DeadLetter {
    /// The pocket it was held in.
    pub pocket_key: String,
    /// Its record key.
    pub key: String,
    /// The held rank.
    pub rank: i64,
    /// The held payload.
    pub payload: Vec<u8>,
    /// The consumer that nacked it.
    pub nacked_by: String,
    /// When it was nacked.
    pub nacked_at: Timestamp,
    /// When it decays.
    pub expires_at: Timestamp,
}
