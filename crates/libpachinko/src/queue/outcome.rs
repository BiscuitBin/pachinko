//! What each queue operation answers: the publish and pull outcomes, the batch, the
//! settle and release outcomes, the lapses and the decay count, and the consumer's
//! view of its lease.

use crate::{ConsumerId, LeaseId, Payload, PocketKey, PublisherId, Rank, RecordKey, Timestamp};

/// One publish: the `Publish(publisher, pocket_key, record_key, rank, payload)`
/// trigger's arguments.
///
/// ```
/// use libpachinko::{Payload, PocketKey, Publication, PublisherId, Rank, RecordKey};
///
/// let publication = Publication::new(
///     PublisherId::new("p1"),
///     PocketKey::new("tenant-a"),
///     RecordKey::new("k1"),
///     Rank::new(1),
///     Payload::from(b"hello".as_slice()),
/// );
/// assert_eq!(publication.rank, Rank::new(1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Publication {
    /// Who publishes; must have joined.
    pub publisher: PublisherId,
    /// The pocket the record goes into.
    pub pocket_key: PocketKey,
    /// The record's key within the pocket.
    pub record_key: RecordKey,
    /// The version's rank; higher wins.
    pub rank: Rank,
    /// The version's bytes.
    pub payload: Payload,
}

impl Publication {
    /// A publication with the given arguments.
    ///
    /// ```
    /// use libpachinko::{Payload, PocketKey, Publication, PublisherId, Rank, RecordKey};
    /// let publication = Publication::new(PublisherId::new("p"), PocketKey::new("a"),
    ///     RecordKey::new("k"), Rank::new(2), Payload::new(vec![1]));
    /// assert_eq!(publication.publisher, PublisherId::new("p"));
    /// assert_eq!(publication.pocket_key, PocketKey::new("a"));
    /// assert_eq!(publication.record_key, RecordKey::new("k"));
    /// assert_eq!(publication.payload, Payload::new(vec![1]));
    /// ```
    #[must_use]
    pub const fn new(
        publisher: PublisherId,
        pocket_key: PocketKey,
        record_key: RecordKey,
        rank: Rank,
        payload: Payload,
    ) -> Self {
        Self {
            publisher,
            pocket_key,
            record_key,
            rank,
            payload,
        }
    }
}

/// Which publish rule took the publish.
///
/// ```
/// use libpachinko::PublishOutcome;
/// assert_ne!(PublishOutcome::Created, PublishOutcome::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublishOutcome {
    /// `PublishIntoNewPocket`: the pocket was created with the record.
    Created,
    /// `PublishNewKey`: the pocket gained a record for a new key.
    NewKey,
    /// `Supersede`: the higher rank replaced the record's rank and payload in place,
    /// and the old version became a superseded record.
    Superseded,
    /// `DropStalePublish`: the lower rank became a superseded record unread.
    Stale,
}

/// One record a batch hands out or a lease holds: its key and the version handed.
///
/// ```
/// use libpachinko::{FakeClock, CountingLeaseIds, Config, ConsumerId, Payload, PocketKey,
///     Publication, PublisherId, PullOutcome, Queue, Rank, RecordKey};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// queue.publish(Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(1), Payload::from(b"x".as_slice()))).unwrap();
/// let PullOutcome::Batch(batch) = queue.pull(&ConsumerId::new("c"), 1).unwrap() else { panic!() };
/// assert_eq!(batch.records[0].record_key, RecordKey::new("k"));
/// assert_eq!(batch.records[0].rank, Rank::new(1));
/// assert_eq!(batch.records[0].payload.as_bytes(), b"x");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HeldRecord {
    /// The record's key.
    pub record_key: RecordKey,
    /// The rank of the version handed out, the record's `held_rank`.
    pub rank: Rank,
    /// The payload of the version handed out, the record's `held_payload`.
    pub payload: Payload,
}

/// What a pull hands back: the lease, the pocket key and zero or more records,
/// oldest first.
///
/// ```
/// use libpachinko::{FakeClock, CountingLeaseIds, Config, ConsumerId, LeaseId, Payload,
///     PocketKey, Publication, PublisherId, PullOutcome, Queue, Rank, RecordKey, Timestamp};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// queue.publish(Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(1), Payload::default())).unwrap();
/// let PullOutcome::Batch(batch) = queue.pull(&ConsumerId::new("c"), 10).unwrap() else { panic!() };
/// assert_eq!(batch.lease_id, LeaseId::from_u128(1));
/// assert_eq!(batch.pocket_key, PocketKey::new("a"));
/// assert_eq!(batch.expires_at, Timestamp::from_millis(300_000));
/// assert_eq!(batch.records.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Batch {
    /// The lease the records were handed under; every ack, nack, renew and release
    /// names it.
    pub lease_id: LeaseId,
    /// The leased pocket.
    pub pocket_key: PocketKey,
    /// When the lease lapses unless the consumer acts first.
    pub expires_at: Timestamp,
    /// The records handed out, smallest position first.
    pub records: Vec<HeldRecord>,
}

/// What a pull answers. A pull is never refused for want of a pocket.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, PullOutcome, Queue};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// assert_eq!(queue.pull(&ConsumerId::new("c"), 1), Ok(PullOutcome::NoPocketAvailable));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PullOutcome {
    /// `PullWithinLease` or `PullClaimsFrontPocket`: a batch under the consumer's
    /// lease, which may hold no records.
    Batch(Batch),
    /// `PullFindsNoPocket`: the consumer holds no lease and no pocket is queued.
    NoPocketAvailable,
}

/// Which ack or nack rule took the action.
///
/// ```
/// use libpachinko::SettleOutcome;
/// assert_ne!(SettleOutcome::Settled, SettleOutcome::Deferred);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SettleOutcome {
    /// `AckSettlesRecord` or `NackSettlesRecord`: the held version was the latest,
    /// and the record is gone.
    Settled,
    /// `AckDefersNewerVersion` or `NackDefersNewerVersion`: a newer version arrived
    /// while the key was held; it restarts at the back, deferred until the lease ends.
    Deferred,
}

/// Where a release or a lapse left the pocket.
///
/// ```
/// use libpachinko::PocketFate;
/// assert_ne!(PocketFate::Queued, PocketFate::Idle);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PocketFate {
    /// Back in the pocket order: at the back after `ReleaseRequeuesPocket`, in its
    /// old place after `LeaseExpiryKeepsPocketAtFront`.
    Queued,
    /// No records left: `ReleaseIdlesEmptyPocket` or `LeaseExpiryIdlesEmptyPocket`.
    Idle,
    /// Set aside after consecutive expiries: `LeaseExpiryParksPocket`.
    Parked,
}

/// One lease the sweep found past its `expires_at`, and where its pocket went.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketFate,
///     PocketKey, Publication, PublisherId, Queue, Rank, RecordKey};
///
/// let clock = FakeClock::default();
/// let mut queue = Queue::new(Config::default(), clock.clone(), CountingLeaseIds::default());
/// queue.publisher_joins(PublisherId::new("p")).unwrap();
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// queue.publish(Publication::new(PublisherId::new("p"), PocketKey::new("a"),
///     RecordKey::new("k"), Rank::new(1), Payload::default())).unwrap();
/// queue.pull(&ConsumerId::new("c"), 1).unwrap();
/// clock.advance(Duration::from_secs(300));
/// let lapsed = queue.lapse();
/// assert_eq!(lapsed.len(), 1);
/// assert_eq!(lapsed[0].fate, PocketFate::Queued);
/// assert_eq!(lapsed[0].consumer, ConsumerId::new("c"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Lapsed {
    /// The lease that expired.
    pub lease_id: LeaseId,
    /// Its consumer.
    pub consumer: ConsumerId,
    /// Its pocket.
    pub pocket_key: PocketKey,
    /// Which lapse rule took it.
    pub fate: PocketFate,
}

/// How many entries one decay pass removed.
///
/// ```
/// use libpachinko::{Config, CountingLeaseIds, Decayed, FakeClock, Queue};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// assert_eq!(queue.decay(), Decayed::default());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Decayed {
    /// `SupersededRecordDecays`: superseded records whose `expires_at` had passed.
    pub superseded: usize,
    /// `DeadLetterDecays`: dead letters whose `expires_at` had passed.
    pub dead_letters: usize,
}

/// The `Consuming` surface's `exposes`: the consumer's active lease, its pocket's
/// key, its `expires_at`, and each held record's key, held rank and held payload.
///
/// ```
/// use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Queue};
///
/// let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
/// queue.consumer_joins(ConsumerId::new("c")).unwrap();
/// assert_eq!(queue.lease_of(&ConsumerId::new("c")), Ok(None));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HeldLease {
    /// The lease, which the consumer's actions name.
    pub lease_id: LeaseId,
    /// The leased pocket's key.
    pub pocket_key: PocketKey,
    /// When the lease lapses unless the consumer acts first.
    pub expires_at: Timestamp,
    /// The pocket's held records, smallest position first.
    pub records: Vec<HeldRecord>,
}
