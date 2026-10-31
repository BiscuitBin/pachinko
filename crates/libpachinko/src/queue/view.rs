//! The read model: a cloned picture of every entity in the module, for tests and
//! for checking the invariants. No surface exposes it (the module's second open
//! question), so the service does not serve it.

use crate::{
    ConsumerId, LeaseId, Payload, PocketKey, Position, PublisherId, Rank, RecordKey, Timestamp,
};

/// A pocket's state.
///
/// ```
/// use libpachinko::PocketStatus;
/// assert_ne!(PocketStatus::Queued, PocketStatus::Parked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// A record's state. `held_rank` and `held_payload` exist only while it is held.
///
/// ```
/// use libpachinko::{Payload, Rank, RecordStatus};
/// let held = RecordStatus::Held { rank: Rank::new(1), payload: Payload::default() };
/// assert_ne!(held, RecordStatus::Queued);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecordStatus {
    /// Waiting in its pocket to be pulled.
    Queued,
    /// Handed out under the pocket's lease.
    Held {
        /// The `held_rank`: the rank of the version handed out.
        rank: Rank,
        /// The `held_payload`: the payload of the version handed out.
        payload: Payload,
    },
    /// A newer version arrived while the key was held, and the held version was
    /// acked or nacked; pullable again once the lease ends.
    Deferred,
}

/// A lease's state. `ended_at` exists only once it has ended.
///
/// ```
/// use libpachinko::{LeaseStatus, Timestamp};
/// let ended = LeaseStatus::Released { ended_at: Timestamp::from_millis(1) };
/// assert_ne!(ended, LeaseStatus::Active);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Why a payload became a superseded record.
///
/// ```
/// use libpachinko::ShedReason;
/// assert_ne!(ShedReason::Superseded, ShedReason::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShedReason {
    /// It lost a supersede.
    Superseded,
    /// It arrived with a lower rank than stored.
    Stale,
}

/// A record as the read model shows it.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublisherId, Queue, Rank, RecordKey};
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// let publish = |rank| Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(rank), Payload::from(rank.to_string().into_bytes()));
/// queue.publish(publish(1)).unwrap();
/// let record = &queue.snapshot().pockets[0].records[0];
/// assert_eq!((record.key.as_str(), record.rank), ("k", Rank::new(1)));
/// assert_eq!(record.payload, Payload::from(b"1".as_slice()));
/// assert_eq!(record.status, libpachinko::RecordStatus::Queued);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecordView {
    /// The record's key.
    pub key: RecordKey,
    /// The latest rank.
    pub rank: Rank,
    /// The latest payload.
    pub payload: Payload,
    /// Where it stands in its pocket.
    pub position: Position,
    /// Its state.
    pub status: RecordStatus,
}

/// A pocket as the read model shows it, with its records by key.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublisherId, Queue, Rank, RecordKey};
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// let publish = |rank| Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(rank), Payload::from(rank.to_string().into_bytes()));
/// queue.publish(publish(1)).unwrap();
/// let pocket = &queue.snapshot().pockets[0];
/// assert_eq!(pocket.key, PocketKey::new("a"));
/// assert_eq!(pocket.status, libpachinko::PocketStatus::Queued);
/// assert_eq!((pocket.consecutive_expiries, pocket.records.len()), (0, 1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PocketView {
    /// The pocket's key.
    pub key: PocketKey,
    /// Its state.
    pub status: PocketStatus,
    /// Where it stands in the pocket order.
    pub position: Position,
    /// Lease expiries since it was last released, revived or created.
    pub consecutive_expiries: u32,
    /// Its records, ordered by key.
    pub records: Vec<RecordView>,
}

/// A lease as the read model shows it, active or ended.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublisherId, Queue, Rank, RecordKey};
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// let publish = |rank| Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(rank), Payload::from(rank.to_string().into_bytes()));
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// queue.publish(publish(1)).unwrap();
/// queue.pull(&ConsumerId::new("c"), 1).unwrap();
/// let lease = &queue.snapshot().leases[0];
/// assert_eq!(lease.pocket_key, PocketKey::new("a"));
/// assert_eq!(lease.consumer, ConsumerId::new("c"));
/// assert_eq!(lease.status, libpachinko::LeaseStatus::Active);
/// assert!(lease.granted_at < lease.expires_at);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LeaseView {
    /// The lease's id.
    pub id: LeaseId,
    /// The pocket it claims.
    pub pocket_key: PocketKey,
    /// The consumer it belongs to.
    pub consumer: ConsumerId,
    /// Its state.
    pub status: LeaseStatus,
    /// When it was claimed.
    pub granted_at: Timestamp,
    /// When it lapses, or lapsed, unless its consumer acts first.
    pub expires_at: Timestamp,
}

/// A payload that lost a supersede or arrived stale, kept until it decays.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublisherId, Queue, Rank, RecordKey};
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// let publish = |rank| Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(rank), Payload::from(rank.to_string().into_bytes()));
/// queue.publish(publish(2)).unwrap();
/// queue.publish(publish(1)).unwrap();
/// let shed = &queue.snapshot().superseded[0];
/// assert_eq!((shed.rank, shed.reason), (Rank::new(1), libpachinko::ShedReason::Stale));
/// assert_eq!(shed.payload, Payload::from(b"1".as_slice()));
/// assert!(shed.shed_at <= shed.expires_at);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SupersededRecord {
    /// The pocket it was published into.
    pub pocket_key: PocketKey,
    /// Its key.
    pub record_key: RecordKey,
    /// Its rank.
    pub rank: Rank,
    /// Its payload.
    pub payload: Payload,
    /// Why it was shed.
    pub reason: ShedReason,
    /// When it was shed.
    pub shed_at: Timestamp,
    /// When it decays.
    pub expires_at: Timestamp,
}

/// A nacked payload, kept until it decays.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
///     Publication, PublisherId, Queue, Rank, RecordKey};
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// let publish = |rank| Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(rank), Payload::from(rank.to_string().into_bytes()));
/// use libpachinko::PullOutcome;
/// let c = ConsumerId::new("c");
/// queue.consumer_joins(c.clone()).unwrap();
/// queue.publish(publish(1)).unwrap();
/// let Ok(PullOutcome::Batch(batch)) = queue.pull(&c, 1) else { panic!() };
/// queue.nack(&c, batch.lease_id, &RecordKey::new("k")).unwrap();
/// let letter = &queue.snapshot().dead_letters[0];
/// assert_eq!((&letter.nacked_by, letter.rank), (&c, Rank::new(1)));
/// assert!(letter.nacked_at <= letter.expires_at);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeadLetter {
    /// The pocket it was held in.
    pub pocket_key: PocketKey,
    /// Its key.
    pub record_key: RecordKey,
    /// The held rank that was nacked.
    pub rank: Rank,
    /// The held payload that was nacked.
    pub payload: Payload,
    /// The consumer that nacked it.
    pub nacked_by: ConsumerId,
    /// When it was nacked.
    pub nacked_at: Timestamp,
    /// When it decays.
    pub expires_at: Timestamp,
}

/// Every entity in the queue at one moment, cloned out of it.
///
/// ```
/// use libpachinko::{Config, CountingLeaseIds, FakeClock, Position, Queue};
///
/// let queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// let snapshot = queue.snapshot();
/// assert_eq!(snapshot.next_position, Position::new(0));
/// assert!(snapshot.pockets.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Snapshot {
    /// The position the next pocket or record to enter an order takes.
    pub next_position: Position,
    /// The publishers, by id.
    pub publishers: Vec<PublisherId>,
    /// The consumers, by id.
    pub consumers: Vec<ConsumerId>,
    /// The pockets, by key.
    pub pockets: Vec<PocketView>,
    /// The pocket order: the queued pockets' keys, smallest position first.
    pub pocket_order: Vec<PocketKey>,
    /// Every lease, active or ended, by id.
    pub leases: Vec<LeaseView>,
    /// The superseded records, oldest first.
    pub superseded: Vec<SupersededRecord>,
    /// The dead letters, oldest first.
    pub dead_letters: Vec<DeadLetter>,
}
