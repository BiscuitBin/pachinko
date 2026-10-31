//! The queue's entities, as the rules in the sibling modules read and change them,
//! and the read model cloned out of them.

use std::collections::{BTreeMap, BTreeSet};

use super::view::{
    DeadLetter, LeaseStatus, LeaseView, PocketStatus, PocketView, RecordStatus, RecordView,
    Snapshot, SupersededRecord,
};
use crate::{
    Config, ConsumerId, LeaseId, Payload, PocketKey, Position, PublisherId, Rank, RecordKey,
    Timestamp,
};

/// A record in its pocket. Its key is the pocket's map key.
#[derive(Clone, Debug)]
pub(super) struct Record {
    pub(super) rank: Rank,
    pub(super) payload: Payload,
    pub(super) position: Position,
    pub(super) status: RecordStatus,
}

/// A pocket and its records, by key. Its key is the queue's map key.
#[derive(Clone, Debug)]
pub(super) struct Pocket {
    pub(super) status: PocketStatus,
    pub(super) position: Position,
    pub(super) consecutive_expiries: u32,
    pub(super) records: BTreeMap<RecordKey, Record>,
}

/// A lease, active or ended. Its id is the queue's map key.
#[derive(Clone, Debug)]
pub(super) struct Lease {
    pub(super) pocket_key: PocketKey,
    pub(super) consumer: ConsumerId,
    pub(super) status: LeaseStatus,
    pub(super) granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
}

/// Every entity of the module. Ended leases are kept, as the module keeps them, so
/// an id is never reissued.
#[derive(Clone, Debug)]
pub(super) struct State {
    pub(super) config: Config,
    pub(super) next_position: u64,
    pub(super) publishers: BTreeSet<PublisherId>,
    pub(super) consumers: BTreeSet<ConsumerId>,
    pub(super) pockets: BTreeMap<PocketKey, Pocket>,
    /// The pocket order: every queued pocket, by position.
    pub(super) order: BTreeMap<Position, PocketKey>,
    pub(super) leases: BTreeMap<LeaseId, Lease>,
    /// Each consumer's active lease, the `Consumer.active_leases` projection.
    pub(super) active: BTreeMap<ConsumerId, LeaseId>,
    pub(super) superseded: Vec<SupersededRecord>,
    pub(super) dead_letters: Vec<DeadLetter>,
    /// The latest time any operation has read: the queue's `now`, which never goes
    /// back however the clock moves.
    latest: Timestamp,
}

/// Takes `next_position` and advances it, as every rule that assigns one does.
pub(super) const fn take_position(next_position: &mut u64) -> Position {
    let position = Position::new(*next_position);
    *next_position += 1;
    position
}

/// `RequeueHeldRecord` and `RequeueDeferredRecord`, for every record of a pocket whose
/// lease has ended: held and deferred records return to queued; a held record's held
/// version is dropped, and each keeps its position.
pub(super) fn free_records(pocket: &mut Pocket) {
    for record in pocket.records.values_mut() {
        record.status = RecordStatus::Queued;
    }
}

impl State {
    pub(super) const fn new(config: Config) -> Self {
        Self {
            config,
            next_position: 0,
            publishers: BTreeSet::new(),
            consumers: BTreeSet::new(),
            pockets: BTreeMap::new(),
            order: BTreeMap::new(),
            leases: BTreeMap::new(),
            active: BTreeMap::new(),
            superseded: Vec::new(),
            dead_letters: Vec::new(),
            latest: Timestamp::from_millis(0),
        }
    }

    /// The queue's `now` for an operation whose clock reads `read`: the reading, or
    /// the latest earlier one if the clock has stepped back since. The module's time
    /// only moves forwards, and every `granted_at <= expires_at` and
    /// `ended_at >= granted_at` rests on it.
    pub(super) fn now(&mut self, read: Timestamp) -> Timestamp {
        self.latest = self.latest.max(read);
        self.latest
    }

    pub(super) const fn config(&self) -> &Config {
        &self.config
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            next_position: Position::new(self.next_position),
            publishers: self.publishers.iter().cloned().collect(),
            consumers: self.consumers.iter().cloned().collect(),
            pockets: self.pockets.iter().map(pocket_view).collect(),
            pocket_order: self.order.values().cloned().collect(),
            leases: self.leases.iter().map(lease_view).collect(),
            superseded: self.superseded.clone(),
            dead_letters: self.dead_letters.clone(),
        }
    }
}

fn pocket_view((key, pocket): (&PocketKey, &Pocket)) -> PocketView {
    PocketView {
        key: key.clone(),
        status: pocket.status,
        position: pocket.position,
        consecutive_expiries: pocket.consecutive_expiries,
        records: pocket
            .records
            .iter()
            .map(|(key, record)| RecordView {
                key: key.clone(),
                rank: record.rank,
                payload: record.payload.clone(),
                position: record.position,
                status: record.status.clone(),
            })
            .collect(),
    }
}

fn lease_view((id, lease): (&LeaseId, &Lease)) -> LeaseView {
    LeaseView {
        id: *id,
        pocket_key: lease.pocket_key.clone(),
        consumer: lease.consumer.clone(),
        status: lease.status,
        granted_at: lease.granted_at,
        expires_at: lease.expires_at,
    }
}
