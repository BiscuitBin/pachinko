//! The store seam: one pocket's records, behind the trait the queue drives.
//!
//! [`PocketRecords`] is the record-level steps the specification's rules take and
//! nothing more. The [`Queue`](crate::queue::Queue) decides every rule and then
//! asks the store to carry out one step; the store decides nothing itself.
//! [`MemoryRecords`] is the one implementation (decision 0019), and
//! `contract::check_contract`, behind the `testing` feature, is the one contract
//! every implementation passes.

#[cfg(feature = "testing")]
pub mod contract;
pub mod memory;

use std::fmt::Debug;

use crate::ids::{Payload, Position, Rank, RecordKey};

pub use memory::MemoryRecords;

/// A record's state. The held version, `held_rank` and `held_payload` in the
/// specification, exists only while the record is held, so it lives in the
/// [`Held`](RecordStatus::Held) variant and nowhere else.
///
/// ```
/// use libpachinko::ids::{Payload, Rank};
/// use libpachinko::records::RecordStatus;
///
/// let held = RecordStatus::Held { rank: Rank::new(1), payload: Payload::default() };
/// assert_ne!(held, RecordStatus::Queued);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecordStatus {
    /// Waiting in its pocket to be pulled.
    Queued,
    /// Handed out under the pocket's active lease.
    Held {
        /// The `held_rank`: the rank of the version handed out.
        rank: Rank,
        /// The `held_payload`: the payload of the version handed out.
        payload: Payload,
    },
    /// A newer version arrived while the key was held and the held version was
    /// acked or nacked; pullable again once the lease ends.
    Deferred,
}

/// A held record as a batch carries it and the `Consuming` surface exposes it: the
/// key, the held rank and the held payload.
///
/// ```
/// use libpachinko::ids::{Payload, Rank, RecordKey};
/// use libpachinko::records::HeldRecord;
///
/// let held = HeldRecord::new(RecordKey::new("k"), Rank::new(2), Payload::default());
/// assert_eq!(held.key.as_str(), "k");
/// assert_eq!(held.rank, Rank::new(2));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HeldRecord {
    /// The record key.
    pub key: RecordKey,
    /// The rank of the version handed out.
    pub rank: Rank,
    /// The payload of the version handed out.
    pub payload: Payload,
}

impl HeldRecord {
    /// A held record with these parts.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Rank, RecordKey};
    /// use libpachinko::records::HeldRecord;
    ///
    /// assert_eq!(HeldRecord::new(RecordKey::new("k"), Rank::new(7), Payload::default()).rank.get(), 7);
    /// ```
    #[must_use]
    pub const fn new(key: RecordKey, rank: Rank, payload: Payload) -> Self {
        Self { key, rank, payload }
    }
}

/// Every field of one record: its key, latest rank and payload, position and
/// status.
///
/// ```
/// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
/// use libpachinko::records::{RecordStatus, RecordView};
///
/// let view = RecordView::new(
///     RecordKey::new("k"),
///     Rank::new(1),
///     Payload::default(),
///     Position::new(4),
///     RecordStatus::Queued,
/// );
/// assert_eq!(view.position, Position::new(4));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RecordView {
    /// The record key.
    pub key: RecordKey,
    /// The latest rank.
    pub rank: Rank,
    /// The latest payload.
    pub payload: Payload,
    /// Where the record stands among its pocket's records.
    pub position: Position,
    /// Queued, held with its held version, or deferred.
    pub status: RecordStatus,
}

impl RecordView {
    /// A view with these parts.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{RecordStatus, RecordView};
    ///
    /// let view = RecordView::new(
    ///     RecordKey::new("k"),
    ///     Rank::new(2),
    ///     Payload::default(),
    ///     Position::new(0),
    ///     RecordStatus::Deferred,
    /// );
    /// assert_eq!(view.status, RecordStatus::Deferred);
    /// ```
    #[must_use]
    pub const fn new(
        key: RecordKey,
        rank: Rank,
        payload: Payload,
        position: Position,
        status: RecordStatus,
    ) -> Self {
        Self {
            key,
            rank,
            payload,
            position,
            status,
        }
    }
}

/// One pocket's records: the steps the specification's rules take on them.
///
/// The queue calls each step once its rule's guards hold. A step whose
/// precondition does not hold changes nothing. Every implementation passes
/// `contract::check_contract`, behind the `testing` feature.
///
/// `Clone` must be a deep, independent copy: a clone shares no record and no
/// interior mutability with the original, so a change to one never shows in the
/// other. The contract checks it, and a cloned `Queue` forks on it.
///
/// ```
/// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
/// use libpachinko::records::{MemoryRecords, PocketRecords};
///
/// let mut records = MemoryRecords::default();
/// records.insert_queued(RecordKey::new("k"), Rank::new(1), Payload::default(), Position::new(0));
/// let batch: Vec<_> = records.hold_oldest(5).into_iter().collect();
/// assert_eq!(batch.len(), 1);
/// assert_eq!(records.held(), batch);
/// ```
pub trait PocketRecords: Clone + Debug + Default + Send + Sync + 'static {
    /// What [`hold_oldest`](Self::hold_oldest) returns: the records it held, at
    /// their held version, smallest position first.
    type Batch: Clone + Debug + Send + Sync + 'static + IntoIterator<Item = HeldRecord>;

    /// The record with this key, or `None` when the pocket has none.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords, RecordStatus};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("k");
    /// assert_eq!(records.lookup(&key), None);
    /// records.insert_queued(key.clone(), Rank::new(3), Payload::default(), Position::new(0));
    /// assert_eq!(records.lookup(&key).map(|view| view.status), Some(RecordStatus::Queued));
    /// ```
    fn lookup(&self, key: &RecordKey) -> Option<RecordView>;

    /// Adds a queued record at `position` (`PublishIntoNewPocket`,
    /// `PublishNewKey`). A key the pocket already has, in any state, is refused and
    /// nothing changes (`OneRecordPerKeyPerPocket`).
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("k");
    /// records.insert_queued(key.clone(), Rank::new(1), Payload::default(), Position::new(0));
    /// records.insert_queued(key.clone(), Rank::new(9), Payload::default(), Position::new(1));
    /// assert_eq!(records.record_count(), 1);
    /// assert_eq!(records.lookup(&key).map(|view| view.rank), Some(Rank::new(1)));
    /// ```
    fn insert_queued(&mut self, key: RecordKey, rank: Rank, payload: Payload, position: Position);

    /// Replaces the key's latest rank and payload in place, leaving its position,
    /// status and held version alone (`Supersede`). Returns the loser, or `None`,
    /// changing nothing, when the key is absent.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("k");
    /// records.insert_queued(key.clone(), Rank::new(1), Payload::from(b"a".as_slice()), Position::new(4));
    /// let loser = records.supersede(&key, Rank::new(2), Payload::from(b"b".as_slice()));
    /// assert_eq!(loser, Some((Rank::new(1), Payload::from(b"a".as_slice()))));
    /// assert_eq!(records.lookup(&key).map(|view| view.position), Some(Position::new(4)));
    /// ```
    fn supersede(
        &mut self,
        key: &RecordKey,
        rank: Rank,
        payload: Payload,
    ) -> Option<(Rank, Payload)>;

    /// Holds the `limit` queued records with the smallest positions, each at its
    /// latest version (`HoldRecord`), and returns them in that order: all of them
    /// when `limit` exceeds their count, none when it is 0. A held or deferred
    /// record is never taken.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// records.insert_queued(RecordKey::new("later"), Rank::new(1), Payload::default(), Position::new(7));
    /// records.insert_queued(RecordKey::new("sooner"), Rank::new(1), Payload::default(), Position::new(3));
    /// assert!(records.hold_oldest(0).is_empty());
    /// assert_eq!(records.hold_oldest(1)[0].key.as_str(), "sooner");
    /// ```
    fn hold_oldest(&mut self, limit: usize) -> Self::Batch;

    /// Removes a held record and returns it at its held version, or `None`,
    /// changing nothing, when the key is not held.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("k");
    /// records.insert_queued(key.clone(), Rank::new(1), Payload::default(), Position::new(0));
    /// assert_eq!(records.settle(&key), None);
    /// records.hold_oldest(1);
    /// assert_eq!(records.settle(&key).map(|held| held.rank), Some(Rank::new(1)));
    /// assert_eq!(records.record_count(), 0);
    /// ```
    fn settle(&mut self, key: &RecordKey) -> Option<HeldRecord>;

    /// Moves a held record to deferred at `position`, dropping its held version,
    /// and returns that version, or `None`, changing nothing, when the key is not
    /// held.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords, RecordStatus};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("k");
    /// records.insert_queued(key.clone(), Rank::new(1), Payload::default(), Position::new(0));
    /// records.hold_oldest(1);
    /// assert!(records.defer(&key, Position::new(5)).is_some());
    /// let view = records.lookup(&key).unwrap();
    /// assert_eq!((view.status, view.position), (RecordStatus::Deferred, Position::new(5)));
    /// ```
    fn defer(&mut self, key: &RecordKey, position: Position) -> Option<HeldRecord>;

    /// Returns every held and deferred record to queued at its own position,
    /// dropping each held version (`RequeueHeldRecord`, `RequeueDeferredRecord`),
    /// and returns how many. It costs O(outstanding), never O(records).
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// records.insert_queued(RecordKey::new("a"), Rank::new(1), Payload::default(), Position::new(0));
    /// records.insert_queued(RecordKey::new("b"), Rank::new(1), Payload::default(), Position::new(1));
    /// records.hold_oldest(1);
    /// assert_eq!(records.free_held(), 1);
    /// assert!(records.held().is_empty());
    /// ```
    fn free_held(&mut self) -> usize;

    /// How many records the pocket has, in any state.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// records.insert_queued(RecordKey::new("a"), Rank::new(1), Payload::default(), Position::new(0));
    /// records.hold_oldest(1);
    /// assert_eq!(records.record_count(), 1);
    /// ```
    fn record_count(&self) -> usize;

    /// The held records at their held version, smallest position first.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// let key = RecordKey::new("a");
    /// records.insert_queued(key.clone(), Rank::new(1), Payload::default(), Position::new(0));
    /// records.hold_oldest(1);
    /// records.supersede(&key, Rank::new(2), Payload::default());
    /// assert_eq!(records.held()[0].rank, Rank::new(1));
    /// ```
    fn held(&self) -> Vec<HeldRecord>;

    /// Every record, by key: what a test reads the invariants through.
    ///
    /// ```
    /// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
    /// use libpachinko::records::{MemoryRecords, PocketRecords};
    ///
    /// let mut records = MemoryRecords::default();
    /// records.insert_queued(RecordKey::new("b"), Rank::new(1), Payload::default(), Position::new(0));
    /// records.insert_queued(RecordKey::new("a"), Rank::new(1), Payload::default(), Position::new(1));
    /// let keys: Vec<_> = records.snapshot().into_iter().map(|view| view.key).collect();
    /// assert_eq!(keys, [RecordKey::new("a"), RecordKey::new("b")]);
    /// ```
    fn snapshot(&self) -> Vec<RecordView>;
}
