//! The record store: one pocket's records, behind the trait the queue drives.
//!
//! [`PocketRecords`] is the record-level steps the module's rules take and nothing
//! more. The [`Queue`](crate::queue::Queue) decides every rule and then asks the
//! store to carry out one step; the store never decides anything itself.
//! [`MemoryRecords`] is the trait's reference implementation and the store the
//! library's tests run on; the service supplies another (decision 0020).

#[cfg(feature = "testing")]
pub mod contract;
mod memory;

use std::fmt::Debug;

pub use memory::MemoryRecords;

/// A record's state: `queued`, `held` or `deferred`.
///
/// ```
/// use libpachinko::records::RecordStatus;
///
/// assert_ne!(RecordStatus::Queued, RecordStatus::Held);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum RecordStatus {
    /// Waiting in the pocket's order to be pulled.
    Queued,
    /// Handed out under the pocket's active lease.
    Held,
    /// Its held version was acked or nacked while a newer version waited; it
    /// is pullable again once the lease ends.
    Deferred,
}

/// One version of a key: a rank and the payload published with it.
///
/// ```
/// use libpachinko::records::Version;
///
/// let version = Version::new(3, b"three".to_vec());
/// assert_eq!((version.rank, version.payload.as_slice()), (3, &b"three"[..]));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Version {
    /// The publisher's rank; higher wins.
    pub rank: i64,
    /// The opaque bytes, never read by pachinko.
    pub payload: Vec<u8>,
}

impl Version {
    /// A version of `rank` carrying `payload`.
    ///
    /// ```
    /// use libpachinko::records::Version;
    ///
    /// assert_eq!(Version::new(1, Vec::new()).rank, 1);
    /// ```
    #[must_use]
    pub const fn new(rank: i64, payload: Vec<u8>) -> Self {
        Self { rank, payload }
    }
}

/// What a rule needs to know about a key before it acts: its latest rank, its
/// status and, while held, the rank of the version handed out.
///
/// ```
/// use libpachinko::records::{RecordState, RecordStatus};
///
/// let state = RecordState::new(5, RecordStatus::Held, Some(4));
/// assert_eq!(state.held_rank, Some(4));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RecordState {
    /// The latest rank.
    pub rank: i64,
    /// Queued, held or deferred.
    pub status: RecordStatus,
    /// The held version's rank, present exactly while the record is held.
    pub held_rank: Option<i64>,
}

impl RecordState {
    /// A state with these three parts.
    ///
    /// ```
    /// use libpachinko::records::{RecordState, RecordStatus};
    ///
    /// assert_eq!(RecordState::new(1, RecordStatus::Queued, None).rank, 1);
    /// ```
    #[must_use]
    pub const fn new(rank: i64, status: RecordStatus, held_rank: Option<i64>) -> Self {
        Self {
            rank,
            status,
            held_rank,
        }
    }
}

/// A held record as the `Consuming` surface exposes it, and as a batch carries it:
/// the key, the held rank and the held payload.
///
/// ```
/// use libpachinko::records::HeldRecord;
///
/// let held = HeldRecord::new("k".to_owned(), 2, b"v".to_vec());
/// assert_eq!(held.key, "k");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HeldRecord {
    /// The record key.
    pub key: String,
    /// The rank of the version handed out.
    pub rank: i64,
    /// The payload of the version handed out.
    pub payload: Vec<u8>,
}

impl HeldRecord {
    /// A held record with these parts.
    ///
    /// ```
    /// use libpachinko::records::HeldRecord;
    ///
    /// assert_eq!(HeldRecord::new("k".to_owned(), 7, Vec::new()).rank, 7);
    /// ```
    #[must_use]
    pub const fn new(key: String, rank: i64, payload: Vec<u8>) -> Self {
        Self { key, rank, payload }
    }
}

/// Every field of one record, for inspection: what a test reads to check the
/// module's invariants and transitions. No rule reads it.
///
/// ```
/// use libpachinko::records::{RecordStatus, RecordView, Version};
///
/// let view = RecordView::new("k".to_owned(), Version::new(1, Vec::new()), 4, RecordStatus::Queued, None);
/// assert_eq!(view.position, 4);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RecordView {
    /// The record key.
    pub key: String,
    /// The latest rank and payload.
    pub latest: Version,
    /// Where the record stands among its pocket's records.
    pub position: u64,
    /// Queued, held or deferred.
    pub status: RecordStatus,
    /// The version handed out, present exactly while the record is held.
    pub held: Option<Version>,
}

impl RecordView {
    /// A view with these parts.
    ///
    /// ```
    /// use libpachinko::records::{RecordStatus, RecordView, Version};
    ///
    /// let held = Some(Version::new(1, Vec::new()));
    /// let view = RecordView::new("k".to_owned(), Version::new(2, Vec::new()), 0, RecordStatus::Held, held);
    /// assert_eq!(view.held.map(|version| version.rank), Some(1));
    /// ```
    #[must_use]
    pub const fn new(
        key: String,
        latest: Version,
        position: u64,
        status: RecordStatus,
        held: Option<Version>,
    ) -> Self {
        Self {
            key,
            latest,
            position,
            status,
            held,
        }
    }
}

/// One pocket's records: the steps the module's rules take on them.
///
/// The queue calls each step only once its rule's guards hold, and a step whose
/// precondition does not hold changes nothing and returns `None`. The behaviour
/// every implementation must show is written once, as
/// `records::contract::check_contract` behind the `testing` feature.
///
/// ```
/// use libpachinko::records::{MemoryRecords, PocketRecords, RecordStatus, Version};
///
/// let mut records = MemoryRecords::default();
/// records.insert_queued("k", Version::new(1, b"a".to_vec()), 10);
/// let batch = records.hold_oldest(5);
/// assert_eq!(batch.len(), 1);
/// assert_eq!(records.lookup("k").map(|state| state.status), Some(RecordStatus::Held));
/// ```
pub trait PocketRecords: Clone + Debug + Default + Send + Sync + 'static {
    /// What a pull hands out: the held versions of the records it held, oldest
    /// first.
    type Batch: Clone + Debug + Send + Sync + 'static;

    /// The key's latest rank, status and held rank, or `None` when the pocket has
    /// no record with that key.
    fn lookup(&self, key: &str) -> Option<RecordState>;

    /// Adds a queued record at `position` (`PublishIntoNewPocket`,
    /// `PublishNewKey`). The queue calls it only for a key the pocket lacks.
    fn insert_queued(&mut self, key: &str, version: Version, position: u64);

    /// Replaces the key's latest rank and payload in place, leaving its position,
    /// status and held version alone (`Supersede`). Returns the replaced version,
    /// or `None`, changing nothing, when the key is absent.
    fn supersede(&mut self, key: &str, version: Version) -> Option<Version>;

    /// Holds the `limit` queued records with the smallest positions, smallest
    /// first, each holding its latest version (`HoldRecord`), and returns them as
    /// a batch. All of them when `limit` exceeds their count; none when it is 0.
    fn hold_oldest(&mut self, limit: usize) -> Self::Batch;

    /// Removes a held record (`AckSettlesRecord`, `NackSettlesRecord`) and returns
    /// its held version, or `None`, changing nothing, when the key is not held.
    fn settle(&mut self, key: &str) -> Option<Version>;

    /// Moves a held record to deferred at `position`, dropping its held version
    /// (`AckDefersNewerVersion`, `NackDefersNewerVersion`), and returns that
    /// version, or `None`, changing nothing, when the key is not held.
    fn defer(&mut self, key: &str, position: u64) -> Option<Version>;

    /// Returns every held and deferred record to queued, dropping each held
    /// version and keeping each position (`RequeueHeldRecord`,
    /// `RequeueDeferredRecord`).
    fn free_all(&mut self);

    /// How many records the pocket has, in any state.
    fn record_count(&self) -> usize;

    /// The held records, as the `Consuming` surface exposes them, by position.
    fn held(&self) -> Vec<HeldRecord>;

    /// Every record, by key: the inspection seam tests read invariants through.
    fn snapshot(&self) -> Vec<RecordView>;
}
