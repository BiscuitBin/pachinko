//! The module's entities as the machines hold them: the pocket machine's pockets,
//! records, leases, superseded records and dead letters, and the order machine's
//! view of each pocket.
//!
//! Every field is public and read-only from outside the machines: the invariant check
//! and the test oracle read them, and only a machine's step writes them.

use std::collections::BTreeMap;

use crate::protocol::{ConsumerId, LeaseId, Partition, Payload, PocketKey, Rank, RecordKey};
use crate::time::Timestamp;

/// `Pocket.status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PocketStatus {
    /// Waiting in the pocket order.
    Queued,
    /// Held by one consumer under an active lease.
    Leased,
    /// No records.
    Idle,
    /// Set aside after consecutive lease expiries until a publish revives it.
    Parked,
}

/// `Record.status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecordStatus {
    /// Waiting in its pocket.
    Queued,
    /// Handed to the pocket's consumer.
    Held,
    /// A newer version arrived while held and the held version was settled; pullable
    /// again once the lease ends.
    Deferred,
}

/// `Record`: the one slot a record key has in its pocket.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Record {
    /// The key.
    pub key: RecordKey,
    /// The latest rank.
    pub rank: Rank,
    /// The latest payload.
    pub payload: Payload,
    /// Where the record stands among its pocket's records, smallest first. Assigned
    /// from the pocket machine's counter tagged with its partition.
    pub position: u64,
    /// The status.
    pub status: RecordStatus,
    /// `held_rank`, present exactly while held.
    pub held_rank: Option<Rank>,
    /// `held_payload`, present exactly while held.
    pub held_payload: Option<Payload>,
}

/// `Lease.status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeaseStatus {
    /// Active.
    Active,
    /// Ended by a release.
    Released,
    /// Ended by a lapse.
    Expired,
}

/// `Lease`: a consumer's exclusive, renewable claim on one pocket. Ended leases are
/// kept, so that an action naming one is answered `LeaseNotActive` and never matched to
/// a successor.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Lease {
    /// The identity and fencing token.
    pub id: LeaseId,
    /// The consumer.
    pub consumer: ConsumerId,
    /// The status.
    pub status: LeaseStatus,
    /// `granted_at`.
    pub granted_at: Timestamp,
    /// `expires_at`.
    pub expires_at: Timestamp,
    /// `ended_at`, present exactly once released or expired.
    pub ended_at: Option<Timestamp>,
}

/// `SupersededRecord.reason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShedReason {
    /// Lost a supersede.
    Superseded,
    /// Arrived with a lower rank than stored.
    Stale,
}

/// `SupersededRecord`: a payload that lost a supersede or arrived stale, visible
/// until it decays.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SupersededRecord {
    /// The key.
    pub key: RecordKey,
    /// The rank that lost.
    pub rank: Rank,
    /// The payload that lost.
    pub payload: Payload,
    /// Why.
    pub reason: ShedReason,
    /// `shed_at`.
    pub shed_at: Timestamp,
    /// `expires_at`.
    pub expires_at: Timestamp,
}

/// `DeadLetter`: a nacked payload, visible until it decays.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeadLetter {
    /// The key.
    pub key: RecordKey,
    /// The rank nacked.
    pub rank: Rank,
    /// The payload nacked.
    pub payload: Payload,
    /// `nacked_by`.
    pub nacked_by: ConsumerId,
    /// `nacked_at`.
    pub nacked_at: Timestamp,
    /// `expires_at`.
    pub expires_at: Timestamp,
}

/// `Pocket` as its pocket machine holds it: the status, the expiry count, the records
/// and everything shed from them, and every lease ever granted on it. The position is
/// not here: the order machine owns it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pocket {
    /// The key.
    pub key: PocketKey,
    /// The status.
    pub status: PocketStatus,
    /// `consecutive_expiries`.
    pub consecutive_expiries: u32,
    /// The records, one per key.
    pub records: BTreeMap<RecordKey, Record>,
    /// Every lease granted on the pocket, oldest first; at most one is active.
    pub leases: Vec<Lease>,
    /// The superseded records not yet decayed.
    pub superseded: Vec<SupersededRecord>,
    /// The dead letters not yet decayed.
    pub dead_letters: Vec<DeadLetter>,
}

impl Pocket {
    /// A pocket as `PublishIntoNewPocket` creates it: queued, no expiries, nothing in it.
    #[must_use]
    pub const fn new(key: PocketKey) -> Self {
        Self {
            key,
            status: PocketStatus::Queued,
            consecutive_expiries: 0,
            records: BTreeMap::new(),
            leases: Vec::new(),
            superseded: Vec::new(),
            dead_letters: Vec::new(),
        }
    }

    /// The active lease, if the pocket is leased.
    #[must_use]
    pub fn active_lease(&self) -> Option<&Lease> {
        self.leases
            .iter()
            .find(|lease| lease.status == LeaseStatus::Active)
    }

    /// The lease with this identity, active or ended.
    #[must_use]
    pub fn lease(&self, id: LeaseId) -> Option<&Lease> {
        self.leases.iter().find(|lease| lease.id == id)
    }
}

/// The order machine's view of one pocket's status: authoritative for the position,
/// a copy of the pocket machine's status that lags it by the messages in flight.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OrderStatus {
    /// In the pocket order at this position.
    Queued {
        /// The position.
        position: u64,
    },
    /// Claimed: off the pocket order, under this consumer's lease.
    Leased {
        /// The position it held when claimed; kept if a lapse requeues it at the front.
        position: u64,
        /// The consumer.
        consumer: ConsumerId,
        /// The lease the order machine minted.
        lease: LeaseId,
    },
    /// Idle, as the pocket machine last reported.
    Idle,
    /// Parked, as the pocket machine last reported.
    Parked,
}

/// What the order machine knows of one pocket.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrderView {
    /// The partition that owns the pocket.
    pub partition: Partition,
    /// The status as the order machine sees it.
    pub status: OrderStatus,
}

impl OrderView {
    /// A view.
    #[must_use]
    pub const fn new(partition: Partition, status: OrderStatus) -> Self {
        Self { partition, status }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lease, LeaseStatus, Pocket, PocketStatus};
    use crate::protocol::{ConsumerId, LeaseId, Partition, PocketKey};
    use crate::time::Timestamp;

    #[test]
    fn a_new_pocket_is_queued_and_empty() {
        let pocket = Pocket::new(PocketKey::new("k"));
        assert_eq!(pocket.status, PocketStatus::Queued);
        assert_eq!(pocket.consecutive_expiries, 0);
        assert!(pocket.records.is_empty());
        assert!(pocket.active_lease().is_none());
    }

    #[test]
    fn lease_lookups_find_ended_leases_too() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        let id = LeaseId::new(Partition::new(0), 1);
        pocket.leases.push(Lease {
            id,
            consumer: ConsumerId::new("c"),
            status: LeaseStatus::Expired,
            granted_at: Timestamp::ZERO,
            expires_at: Timestamp::ZERO,
            ended_at: Some(Timestamp::ZERO),
        });
        assert!(pocket.active_lease().is_none());
        assert_eq!(
            pocket.lease(id).map(|l| l.status),
            Some(LeaseStatus::Expired)
        );
        assert!(pocket.lease(LeaseId::new(Partition::new(0), 2)).is_none());
    }
}
