//! The protocol between the machines and their clients: the identifiers, the one
//! enum of inputs, the one enum of outputs and the responses a client sees.
//!
//! Every message a machine receives is an [`Input`], whoever sent it: a client, another
//! machine or the clock. Every effect a machine asks for is an [`Output`]: a message to
//! another machine, with or without the client's reply handle, or the reply itself.

use crate::time::Timestamp;

/// A publisher's identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct PublisherId(String);

/// A consumer's identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ConsumerId(String);

/// The key that names a pocket.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct PocketKey(String);

/// The key that names a record within its pocket.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct RecordKey(String);

macro_rules! string_id {
    ($name:ident) => {
        impl $name {
            /// Wraps the identity.
            #[must_use]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identity as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(PublisherId);
string_id!(ConsumerId);
string_id!(PocketKey);
string_id!(RecordKey);

/// The publisher-supplied order between the versions of one key; higher wins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Rank(u64);

impl Rank {
    /// A rank.
    #[must_use]
    pub const fn new(rank: u64) -> Self {
        Self(rank)
    }

    /// The rank as a number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The opaque bytes a record carries; nothing here reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Payload(Vec<u8>);

impl Payload {
    /// Wraps the bytes.
    ///
    /// ```
    /// use worker_pool::Payload;
    /// assert_eq!(Payload::new("hi").bytes(), b"hi");
    /// ```
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// The bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A partition: a disjoint set of pockets, chosen by a stable function of the pocket
/// key, whose state is touched by exactly one thread. Never a pocket.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Partition(u32);

impl Partition {
    /// The partition with this index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Which machine a message is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MachineId {
    /// The order machine: the pocket order, the pocket position counter and the
    /// consumer to active lease routing.
    Order,
    /// The pocket machine owning one partition's pockets, records and leases.
    Partition(Partition),
}

/// A lease's identity and the fencing token: minted by the order machine, strictly
/// increasing in `serial`, and carrying the partition that owns the pocket so that an
/// action naming it routes with no table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct LeaseId {
    /// The partition of the pocket the lease is on.
    pub partition: Partition,
    /// The order machine's counter at the grant.
    pub serial: u64,
}

impl LeaseId {
    /// A lease identity.
    #[must_use]
    pub const fn new(partition: Partition, serial: u64) -> Self {
        Self { partition, serial }
    }
}

/// What a pocket became when a lease on it ended, as the pocket machine tells the
/// order machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Became {
    /// `ReleaseRequeuesPocket`: queued again with a fresh position at the back.
    RequeuedAtBack,
    /// `LeaseExpiryKeepsPocketAtFront`: queued again at the position it had.
    RequeuedKeepingPosition,
    /// `ReleaseIdlesEmptyPocket` or `LeaseExpiryIdlesEmptyPocket`: no records left.
    Idle,
    /// `LeaseExpiryParksPocket`: set aside until a publish revives it.
    Parked,
}

/// Every message a machine can receive: a client's action, a message chained from
/// another machine, or the clock.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// `PublisherJoins(publisher_id)`; to the order machine.
    PublisherJoins {
        /// The joining publisher.
        publisher: PublisherId,
    },
    /// `ConsumerJoins(consumer_id)`; to the order machine.
    ConsumerJoins {
        /// The joining consumer.
        consumer: ConsumerId,
    },
    /// `Publish(publisher, pocket_key, record_key, rank, payload)`; to the pocket
    /// machine of the key's partition.
    Publish {
        /// The publisher.
        publisher: PublisherId,
        /// The pocket.
        pocket_key: PocketKey,
        /// The record's key within the pocket.
        record_key: RecordKey,
        /// The version's rank.
        rank: Rank,
        /// The bytes.
        payload: Payload,
    },
    /// `Pull(consumer, limit)`; to the order machine. A limit of zero yields an empty
    /// batch, which stands for the module's "not positive".
    Pull {
        /// The consumer.
        consumer: ConsumerId,
        /// The most records wanted.
        limit: u64,
    },
    /// `Ack(consumer, lease, record_key)`; to the lease's partition.
    Ack {
        /// The consumer.
        consumer: ConsumerId,
        /// The lease the action is under.
        lease: LeaseId,
        /// The held record.
        record_key: RecordKey,
    },
    /// `Nack(consumer, lease, record_key)`; to the lease's partition.
    Nack {
        /// The consumer.
        consumer: ConsumerId,
        /// The lease the action is under.
        lease: LeaseId,
        /// The held record.
        record_key: RecordKey,
    },
    /// `Renew(consumer, lease)`; to the lease's partition.
    Renew {
        /// The consumer.
        consumer: ConsumerId,
        /// The lease.
        lease: LeaseId,
    },
    /// `Release(consumer, lease)`; to the lease's partition.
    Release {
        /// The consumer.
        consumer: ConsumerId,
        /// The lease.
        lease: LeaseId,
    },
    /// Order machine to pocket machine, forwarded under the client's reply handle: the
    /// order machine took the pocket off the front and minted the lease; the pocket
    /// machine creates the lease with that identity, holds the batch and replies.
    Claim {
        /// The pocket claimed.
        pocket_key: PocketKey,
        /// The consumer the lease is granted to.
        consumer: ConsumerId,
        /// The identity the lease takes.
        lease: LeaseId,
        /// The pull's limit.
        limit: u64,
    },
    /// Order machine to pocket machine, forwarded under the client's reply handle: a
    /// `Pull` from a consumer the order machine believes holds this lease.
    PullUnderLease {
        /// The consumer.
        consumer: ConsumerId,
        /// The lease the order machine believes active.
        lease: LeaseId,
        /// The pull's limit.
        limit: u64,
    },
    /// Pocket machine to order machine, no reply handle: a new, idle or parked pocket
    /// became queued and wants a position.
    Enqueue {
        /// The pocket.
        pocket_key: PocketKey,
        /// The partition that owns it.
        partition: Partition,
    },
    /// Pocket machine to order machine, no reply handle: a lease ended and the pocket
    /// left `leased`.
    PocketBecame {
        /// The pocket.
        pocket_key: PocketKey,
        /// What it became.
        became: Became,
        /// The consumer whose lease ended.
        consumer: ConsumerId,
        /// The lease that ended.
        lease: LeaseId,
    },
    /// Pocket machine to order machine, forwarded under the client's reply handle: a
    /// `PullUnderLease` named a lease the pocket machine had already ended, so the
    /// order machine's view was stale; it handles this exactly as a `Pull`, after the
    /// `PocketBecame` that precedes it in the same queue.
    RetryPull {
        /// The consumer.
        consumer: ConsumerId,
        /// The pull's limit.
        limit: u64,
    },
    /// The clock, injected by the pool at a fixed interval into every machine; the only
    /// input on which a lease lapses or a decay happens.
    Tick,
}

/// What a client gets back: exactly one per client input, whichever machine sends it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Response {
    /// `PublisherJoins` or `ConsumerJoins` created the party.
    Joined,
    /// One of the four publish rules applied.
    Published(Published),
    /// `PullWithinLease` or `PullClaimsFrontPocket` handed these records.
    Batch {
        /// The lease the records are held under.
        lease: LeaseId,
        /// The pocket.
        pocket_key: PocketKey,
        /// The records, smallest position first.
        records: Vec<HandedRecord>,
    },
    /// `PullFindsNoPocket`.
    NoPocketAvailable,
    /// `AckSettlesRecord` or `AckDefersNewerVersion`.
    Acked,
    /// `NackSettlesRecord` or `NackDefersNewerVersion`.
    Nacked,
    /// `Renew`.
    Renewed {
        /// The lease's new expiry.
        expires_at: Timestamp,
    },
    /// `ReleaseRequeuesPocket` or `ReleaseIdlesEmptyPocket`.
    Released,
    /// No rule's guards held; the reason names the guard that failed.
    Rejected(Reason),
}

/// Which publish rule applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Published {
    /// `PublishIntoNewPocket`.
    NewPocket,
    /// `PublishNewKey`.
    NewKey,
    /// `Supersede`.
    Superseded,
    /// `DropStalePublish`: the publish itself became a superseded record.
    Stale,
}

/// A record as a batch hands it out: `record.key`, `held_rank`, `held_payload`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HandedRecord {
    /// The record's key.
    pub key: RecordKey,
    /// The rank handed out, the record's rank at the hold.
    pub rank: Rank,
    /// The payload handed out.
    pub payload: Payload,
}

impl HandedRecord {
    /// A handed record.
    #[must_use]
    pub const fn new(key: RecordKey, rank: Rank, payload: Payload) -> Self {
        Self { key, rank, payload }
    }
}

/// The guard that failed, when no rule's guards held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// `PublisherJoins`: a publisher with that id exists.
    DuplicatePublisher,
    /// `ConsumerJoins`: a consumer with that id exists.
    DuplicateConsumer,
    /// A publish whose rank equals the stored rank: no rule covers it, and the checker
    /// treats it as rejected; the module's first open question.
    EqualRank,
    /// The named lease is not active: released or expired.
    LeaseNotActive,
    /// The named lease is another consumer's.
    NotYourLease,
    /// No lease with that identity was ever granted.
    UnknownLease,
    /// No record with that key is in the lease's pocket.
    NoSuchRecord,
    /// The record is not held.
    RecordNotHeld,
    /// The lease is active but its pocket is not leased; unreachable by the invariants,
    /// kept as a guard because the release rules state it.
    PocketNotLeased,
    /// An input the protocol never sends this machine in this state. Counted as a
    /// protocol violation by the machine that saw it.
    Protocol,
}

/// What a step asks the pool to do, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Output {
    /// Send this input to that machine under the reply handle of the input being
    /// processed, so that the reply, wherever it is produced, reaches the client.
    Forward(MachineId, Input),
    /// Send this input to that machine with no reply handle: a notification nobody
    /// waits for.
    Notify(MachineId, Input),
    /// Answer the client whose input is being processed.
    Reply(Response),
}

#[cfg(test)]
mod tests {
    use super::{ConsumerId, LeaseId, Partition, Payload, PocketKey, PublisherId, Rank, RecordKey};

    #[test]
    fn identities_round_trip_their_text() {
        assert_eq!(PublisherId::new("p").as_str(), "p");
        assert_eq!(ConsumerId::new("c").as_str(), "c");
        assert_eq!(PocketKey::new("k").as_str(), "k");
        assert_eq!(RecordKey::new("r").as_str(), "r");
        assert_eq!(Rank::new(3).get(), 3);
        assert_eq!(Payload::new(vec![1, 2]).bytes(), &[1, 2]);
        assert_eq!(Partition::new(4).index(), 4);
    }

    #[test]
    fn lease_ids_order_by_partition_then_serial() {
        let a = LeaseId::new(Partition::new(0), 9);
        let b = LeaseId::new(Partition::new(1), 1);
        assert!(a < b);
        assert!(LeaseId::new(Partition::new(0), 1) < a);
    }
}
