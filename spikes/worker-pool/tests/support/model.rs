//! The oracle: a model of the module written from the rules' `requires` clauses in
//! `docs/specs/pachinko.allium`, never from the machines. It reads the machines' state
//! into plain data and predicts, for one client input, which rule applies or which
//! guard refuses it. The test then compares the machines' reply with the prediction.
//!
//! Where the machines decide by routing a `Pull` through the order machine's lease
//! table and a `Claim`, the model decides from the leases the pocket machines hold and
//! the positions the order machine has assigned: the same facts, read as state.

use std::collections::{BTreeMap, BTreeSet};

use worker_pool::{
    Config, ConsumerId, Direct, Input, LeaseId, OrderStatus, Payload, PocketKey, PocketStatus,
    Published, PublisherId, Rank, Reason, RecordKey, RecordStatus, Response, Timestamp,
};

/// A record as the model sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecordSnap {
    pub(crate) status: RecordStatus,
    pub(crate) rank: Rank,
    pub(crate) held_rank: Option<Rank>,
    pub(crate) position: u64,
}

/// A pocket as the model sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PocketSnap {
    pub(crate) status: PocketStatus,
    pub(crate) records: BTreeMap<RecordKey, RecordSnap>,
}

/// A lease as the model sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LeaseSnap {
    pub(crate) pocket: PocketKey,
    pub(crate) consumer: ConsumerId,
    pub(crate) active: bool,
}

/// The aggregate state, as plain data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) config: Config,
    pub(crate) publishers: BTreeSet<PublisherId>,
    pub(crate) consumers: BTreeSet<ConsumerId>,
    pub(crate) pockets: BTreeMap<PocketKey, PocketSnap>,
    pub(crate) leases: BTreeMap<LeaseId, LeaseSnap>,
    /// The queued pockets by position, the module's `Pockets where status = queued`
    /// ordered by `position`.
    pub(crate) queued: BTreeMap<u64, PocketKey>,
}

/// Which accepted outcome a rule produces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Joined,
    Published(Published),
    /// `Batch` on this pocket handing exactly these keys, smallest position first.
    Batch(PocketKey, Vec<RecordKey>),
    NoPocketAvailable,
    Acked,
    Nacked,
    Renewed(Timestamp),
    Released,
}

/// What the model predicts for one client input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Prediction {
    Accept(Kind),
    Reject(Reason),
}

impl Snapshot {
    pub(crate) fn of(direct: &Direct) -> Self {
        let mut pockets = BTreeMap::new();
        let mut leases = BTreeMap::new();
        for (key, pocket) in direct
            .pockets()
            .iter()
            .flat_map(worker_pool::PocketMachine::pockets)
        {
            let records = pocket
                .records
                .iter()
                .map(|(key, record)| {
                    let snap = RecordSnap {
                        status: record.status,
                        rank: record.rank,
                        held_rank: record.held_rank,
                        position: record.position,
                    };
                    (key.clone(), snap)
                })
                .collect();
            pockets.insert(
                key.clone(),
                PocketSnap {
                    status: pocket.status,
                    records,
                },
            );
            for lease in &pocket.leases {
                leases.insert(
                    lease.id,
                    LeaseSnap {
                        pocket: key.clone(),
                        consumer: lease.consumer.clone(),
                        active: lease.status == worker_pool::LeaseStatus::Active,
                    },
                );
            }
        }
        let queued = direct
            .order()
            .views()
            .filter_map(|(key, view)| match view.status {
                OrderStatus::Queued { position } => Some((position, key.clone())),
                _ => None,
            })
            .collect();
        Self {
            config: *direct.config(),
            publishers: direct.order().publishers().cloned().collect(),
            consumers: direct.order().consumers().cloned().collect(),
            pockets,
            leases,
            queued,
        }
    }

    /// `Lease{consumer: consumer, status: active}`.
    fn active_lease_of(&self, consumer: &ConsumerId) -> Option<(LeaseId, &LeaseSnap)> {
        self.leases
            .iter()
            .find(|(_, lease)| lease.active && lease.consumer == *consumer)
            .map(|(id, lease)| (*id, lease))
    }

    /// `oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))`.
    fn oldest(&self, pocket: &PocketKey, limit: u64) -> Vec<RecordKey> {
        let n = usize::try_from(limit.min(self.config.max_pull_batch)).unwrap_or(usize::MAX);
        let mut queued: Vec<(u64, &RecordKey)> = self.pockets[pocket]
            .records
            .iter()
            .filter(|(_, record)| record.status == RecordStatus::Queued)
            .map(|(key, record)| (record.position, key))
            .collect();
        queued.sort_unstable();
        queued
            .into_iter()
            .take(n)
            .map(|(_, key)| key.clone())
            .collect()
    }

    /// The guards every lease action shares, in the rules' order: the lease exists, is
    /// active and is the consumer's.
    fn lease_guards(&self, consumer: &ConsumerId, lease: LeaseId) -> Result<&LeaseSnap, Reason> {
        let snap = self.leases.get(&lease).ok_or(Reason::UnknownLease)?;
        if !snap.active {
            return Err(Reason::LeaseNotActive);
        }
        if snap.consumer != *consumer {
            return Err(Reason::NotYourLease);
        }
        Ok(snap)
    }

    /// `AckSettlesRecord`/`AckDefersNewerVersion` and the nack pair: the record exists in
    /// the lease's pocket and is held; which of the two applies does not change the reply.
    fn settle_guards(
        &self,
        consumer: &ConsumerId,
        lease: LeaseId,
        record: &RecordKey,
    ) -> Result<(), Reason> {
        let snap = self.lease_guards(consumer, lease)?;
        let record = self.pockets[&snap.pocket]
            .records
            .get(record)
            .ok_or(Reason::NoSuchRecord)?;
        if record.status != RecordStatus::Held {
            return Err(Reason::RecordNotHeld);
        }
        Ok(())
    }
}

/// What the rules say happens to `input` at `now`, from the snapshot alone.
pub(crate) fn predict(snapshot: &Snapshot, input: &Input, now: Timestamp) -> Prediction {
    use Prediction::{Accept, Reject};
    match input {
        Input::PublisherJoins { publisher } if snapshot.publishers.contains(publisher) => {
            Reject(Reason::DuplicatePublisher)
        }
        Input::ConsumerJoins { consumer } if snapshot.consumers.contains(consumer) => {
            Reject(Reason::DuplicateConsumer)
        }
        Input::PublisherJoins { .. } | Input::ConsumerJoins { .. } => Accept(Kind::Joined),
        Input::Publish {
            pocket_key,
            record_key,
            rank,
            ..
        } => predict_publish(snapshot, pocket_key, record_key, *rank),
        Input::Pull { consumer, limit } => predict_pull(snapshot, consumer, *limit),
        Input::Ack {
            consumer,
            lease,
            record_key,
        } => match snapshot.settle_guards(consumer, *lease, record_key) {
            Ok(()) => Accept(Kind::Acked),
            Err(reason) => Reject(reason),
        },
        Input::Nack {
            consumer,
            lease,
            record_key,
        } => match snapshot.settle_guards(consumer, *lease, record_key) {
            Ok(()) => Accept(Kind::Nacked),
            Err(reason) => Reject(reason),
        },
        Input::Renew { consumer, lease } => match snapshot.lease_guards(consumer, *lease) {
            Ok(_) => Accept(Kind::Renewed(now.plus(snapshot.config.lease_ttl))),
            Err(reason) => Reject(reason),
        },
        Input::Release { consumer, lease } => match snapshot.lease_guards(consumer, *lease) {
            Ok(snap) if snapshot.pockets[&snap.pocket].status != PocketStatus::Leased => {
                Reject(Reason::PocketNotLeased)
            }
            Ok(_) => Accept(Kind::Released),
            Err(reason) => Reject(reason),
        },
        _ => Reject(Reason::Protocol),
    }
}

/// The four publish rules' disjoint guards, and the uncovered equal rank.
fn predict_publish(
    snapshot: &Snapshot,
    pocket: &PocketKey,
    record: &RecordKey,
    rank: Rank,
) -> Prediction {
    let Some(pocket) = snapshot.pockets.get(pocket) else {
        return Prediction::Accept(Kind::Published(Published::NewPocket));
    };
    let Some(stored) = pocket.records.get(record) else {
        return Prediction::Accept(Kind::Published(Published::NewKey));
    };
    match rank.cmp(&stored.rank) {
        std::cmp::Ordering::Greater => Prediction::Accept(Kind::Published(Published::Superseded)),
        std::cmp::Ordering::Less => Prediction::Accept(Kind::Published(Published::Stale)),
        std::cmp::Ordering::Equal => Prediction::Reject(Reason::EqualRank),
    }
}

/// `PullWithinLease`, `PullClaimsFrontPocket` or `PullFindsNoPocket`.
fn predict_pull(snapshot: &Snapshot, consumer: &ConsumerId, limit: u64) -> Prediction {
    if let Some((_, lease)) = snapshot.active_lease_of(consumer) {
        // `requires: pocket.status = leased` holds by LeasedHasOneActiveLease; were it
        // not to, no rule would cover the pull.
        if snapshot.pockets[&lease.pocket].status != PocketStatus::Leased {
            return Prediction::Reject(Reason::Protocol);
        }
        let keys = snapshot.oldest(&lease.pocket, limit);
        return Prediction::Accept(Kind::Batch(lease.pocket.clone(), keys));
    }
    match snapshot.queued.iter().next() {
        Some((_, front)) => {
            let keys = snapshot.oldest(front, limit);
            Prediction::Accept(Kind::Batch(front.clone(), keys))
        }
        None => Prediction::Accept(Kind::NoPocketAvailable),
    }
}

/// Whether the machines' reply is the one the prediction named.
pub(crate) fn matches(prediction: &Prediction, response: &Response) -> bool {
    match (prediction, response) {
        (Prediction::Reject(expected), Response::Rejected(actual)) => expected == actual,
        (Prediction::Accept(Kind::Joined), Response::Joined)
        | (Prediction::Accept(Kind::NoPocketAvailable), Response::NoPocketAvailable)
        | (Prediction::Accept(Kind::Acked), Response::Acked)
        | (Prediction::Accept(Kind::Nacked), Response::Nacked)
        | (Prediction::Accept(Kind::Released), Response::Released) => true,
        (Prediction::Accept(Kind::Published(expected)), Response::Published(actual)) => {
            expected == actual
        }
        (Prediction::Accept(Kind::Renewed(expected)), Response::Renewed { expires_at }) => {
            expected == expires_at
        }
        (
            Prediction::Accept(Kind::Batch(pocket, keys)),
            Response::Batch {
                pocket_key,
                records,
                ..
            },
        ) => pocket == pocket_key && records.iter().map(|record| &record.key).eq(keys.iter()),
        _ => false,
    }
}

// The client inputs, built by name so a script reads as the module's triggers do.

pub(crate) fn publisher_joins(publisher: &str) -> Input {
    Input::PublisherJoins {
        publisher: PublisherId::new(publisher),
    }
}

pub(crate) fn consumer_joins(consumer: &str) -> Input {
    Input::ConsumerJoins {
        consumer: ConsumerId::new(consumer),
    }
}

pub(crate) fn publish(publisher: &str, pocket: &str, record: &str, rank: u64) -> Input {
    Input::Publish {
        publisher: PublisherId::new(publisher),
        pocket_key: PocketKey::new(pocket),
        record_key: RecordKey::new(record),
        rank: Rank::new(rank),
        payload: Payload::new(format!("{record}@{rank}")),
    }
}

pub(crate) fn pull(consumer: &str, limit: u64) -> Input {
    Input::Pull {
        consumer: ConsumerId::new(consumer),
        limit,
    }
}

pub(crate) fn ack(consumer: &str, lease: LeaseId, record: &str) -> Input {
    Input::Ack {
        consumer: ConsumerId::new(consumer),
        lease,
        record_key: RecordKey::new(record),
    }
}

pub(crate) fn nack(consumer: &str, lease: LeaseId, record: &str) -> Input {
    Input::Nack {
        consumer: ConsumerId::new(consumer),
        lease,
        record_key: RecordKey::new(record),
    }
}

pub(crate) fn renew(consumer: &str, lease: LeaseId) -> Input {
    Input::Renew {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

pub(crate) fn release(consumer: &str, lease: LeaseId) -> Input {
    Input::Release {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}
