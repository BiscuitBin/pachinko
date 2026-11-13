//! The pockets by key, the pocket order and the queue's position counter.
//!
//! Each method is one rule's effect on the pockets, called once the queue has
//! checked that rule's guards on the parties and the leases.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    PocketFate, PocketStatus, Publication, PublishOutcome, SettleOutcome, ShedReason,
    SupersededRecord,
};
use crate::error::PublishError;
use crate::ids::{HeldRecord, Payload, PocketKey, Rank, RecordKey};
use crate::records::{PocketRecords, Position, RecordStatus};
use crate::timestamp::Timestamp;

/// One pocket.
#[derive(Clone, Debug)]
pub(super) struct Pocket<R> {
    pub(super) status: PocketStatus,
    pub(super) position: Position,
    pub(super) consecutive_expiries: u32,
    pub(super) records: R,
}

/// When a payload is shed and when it decays.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stamp {
    pub(super) shed_at: Timestamp,
    pub(super) expires_at: Timestamp,
}

/// The pockets, the order of the queued ones and the specification's
/// `Queue.next_position`, which pockets and records both take positions from.
#[derive(Clone, Debug)]
pub(super) struct Pockets<R> {
    by_key: BTreeMap<PocketKey, Pocket<R>>,
    order: BTreeSet<(Position, PocketKey)>,
    next_position: Position,
}

impl<R> Default for Pockets<R> {
    fn default() -> Self {
        Self {
            by_key: BTreeMap::new(),
            order: BTreeSet::new(),
            next_position: Position::default(),
        }
    }
}

/// Takes `next_position` and advances it.
const fn take_position(next_position: &mut Position) -> Position {
    let position = *next_position;
    *next_position = position.next();
    position
}

/// The `PocketTouched` dispatch every publish into an existing pocket ends in. A
/// queued or leased pocket is untouched. The idle arm is `ReviveIdlePocket`, which
/// a release or a lapse that idles a pocket makes reachable, and the parked arm
/// `ReviveParkedPocket`, which a lapse that parks one makes reachable: either way the pocket rejoins the order at
/// the back with its expiry count reset.
fn touch<R>(
    key: &PocketKey,
    pocket: &mut Pocket<R>,
    order: &mut BTreeSet<(Position, PocketKey)>,
    next_position: &mut Position,
) {
    match pocket.status {
        PocketStatus::Queued | PocketStatus::Leased => {}
        PocketStatus::Idle | PocketStatus::Parked => {
            pocket.status = PocketStatus::Queued;
            pocket.position = take_position(next_position);
            pocket.consecutive_expiries = 0;
            order.insert((pocket.position, key.clone()));
        }
    }
}

impl<R: PocketRecords> Pockets<R> {
    /// Applies one publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede` or
    /// `DropStalePublish`, by their disjoint guards, each but the first ending in
    /// the `PocketTouched` dispatch. Returns the rule taken and what it shed.
    ///
    /// # Errors
    ///
    /// [`PublishError::EqualRank`] when the rank equals the stored one, which no
    /// rule covers; nothing changes.
    pub(super) fn publish(
        &mut self,
        publication: Publication,
        stamp: Stamp,
    ) -> Result<(PublishOutcome, Option<SupersededRecord>), PublishError> {
        let Publication {
            pocket_key,
            record_key,
            rank,
            payload,
        } = publication;
        let Some(pocket) = self.by_key.get_mut(&pocket_key) else {
            self.create(pocket_key, record_key, rank, payload);
            return Ok((PublishOutcome::Created, None));
        };
        let (outcome, shed) = match pocket.records.lookup(&record_key) {
            None => {
                let position = take_position(&mut self.next_position);
                pocket
                    .records
                    .insert_queued(record_key, rank, payload, position);
                (PublishOutcome::NewKey, None)
            }
            Some(stored) if rank > stored.rank => {
                let loser = pocket.records.supersede(&record_key, rank, payload);
                let shed = loser.map(|(rank, payload)| {
                    shed(
                        &pocket_key,
                        (record_key, rank, payload),
                        ShedReason::Superseded,
                        stamp,
                    )
                });
                (PublishOutcome::Superseded, shed)
            }
            Some(stored) if rank < stored.rank => {
                let shed = shed(
                    &pocket_key,
                    (record_key, rank, payload),
                    ShedReason::Stale,
                    stamp,
                );
                (PublishOutcome::Stale, Some(shed))
            }
            Some(_) => {
                return Err(PublishError::EqualRank {
                    key: record_key,
                    rank,
                });
            }
        };
        touch(
            &pocket_key,
            pocket,
            &mut self.order,
            &mut self.next_position,
        );
        Ok((outcome, shed))
    }

    /// `PublishIntoNewPocket`: the pocket takes `next_position` and its record the
    /// one after.
    fn create(
        &mut self,
        pocket_key: PocketKey,
        record_key: RecordKey,
        rank: Rank,
        payload: Payload,
    ) {
        let pocket_position = take_position(&mut self.next_position);
        let record_position = take_position(&mut self.next_position);
        let mut records = R::default();
        records.insert_queued(record_key, rank, payload, record_position);
        self.order.insert((pocket_position, pocket_key.clone()));
        let pocket = Pocket {
            status: PocketStatus::Queued,
            position: pocket_position,
            consecutive_expiries: 0,
            records,
        };
        self.by_key.insert(pocket_key, pocket);
    }

    /// The queued pocket with the smallest position, the one [`Self::claim_front`]
    /// would claim, left queued; `None` when no pocket is queued. A claim peeks
    /// first so it can draw its lease id before it changes anything.
    pub(super) fn front(&self) -> Option<&PocketKey> {
        self.order.first().map(|(_, key)| key)
    }

    /// `PullClaimsFrontPocket`'s pocket step: the queued pocket with the smallest
    /// position leaves the order, leased. Returns its key, or `None` when no pocket
    /// is queued (`PullFindsNoPocket`).
    pub(super) fn claim_front(&mut self) -> Option<PocketKey> {
        let (_, key) = self.order.pop_first()?;
        let pocket = self.by_key.get_mut(&key)?;
        pocket.status = PocketStatus::Leased;
        Some(key)
    }

    /// `HoldRecord` for each of the `limit` oldest queued records of the pocket.
    pub(super) fn hold(&mut self, key: &PocketKey, limit: usize) -> Vec<HeldRecord> {
        self.by_key
            .get_mut(key)
            .map(|pocket| pocket.records.hold_oldest(limit).into_iter().collect())
            .unwrap_or_default()
    }

    /// The pocket's held records, smallest position first.
    pub(super) fn held(&self, key: &PocketKey) -> Vec<HeldRecord> {
        self.by_key
            .get(key)
            .map(|pocket| pocket.records.held())
            .unwrap_or_default()
    }

    /// Whether the pocket holds the record `key`: the ack and nack rules' last two
    /// guards, `exists record` and `record.status = held`.
    pub(super) fn is_held(&self, pocket_key: &PocketKey, key: &RecordKey) -> bool {
        self.by_key
            .get(pocket_key)
            .and_then(|pocket| pocket.records.lookup(key))
            .is_some_and(|record| matches!(record.status, RecordStatus::Held { .. }))
    }

    /// The ack and nack rules' split on a held record: with `held_rank = rank` the
    /// record is removed (`AckSettlesRecord`, `NackSettlesRecord`); with
    /// `held_rank < rank` it is deferred at a fresh position from `next_position`
    /// (`AckDefersNewerVersion`, `NackDefersNewerVersion`). Returns the rule taken
    /// and the held version, or `None`, changing nothing, when the record is not
    /// held.
    pub(super) fn settle(
        &mut self,
        pocket_key: &PocketKey,
        key: &RecordKey,
    ) -> Option<(SettleOutcome, HeldRecord)> {
        let pocket = self.by_key.get_mut(pocket_key)?;
        let record = pocket.records.lookup(key)?;
        let RecordStatus::Held {
            rank: held_rank, ..
        } = record.status
        else {
            return None;
        };
        if held_rank == record.rank {
            let held = pocket.records.settle(key)?;
            return Some((SettleOutcome::Settled, held));
        }
        let position = take_position(&mut self.next_position);
        let held = pocket.records.defer(key, position)?;
        Some((SettleOutcome::Deferred, held))
    }

    /// A release's pocket step: every held and deferred record back to queued at
    /// its own position in one `free_held` (`RequeueHeldRecord`,
    /// `RequeueDeferredRecord`); then, with records left, the pocket rejoins the
    /// order at the back (`ReleaseRequeuesPocket`), and with none it goes idle
    /// (`ReleaseIdlesEmptyPocket`). Either way its expiry count resets. `None`
    /// when no such pocket exists.
    pub(super) fn release(&mut self, pocket_key: &PocketKey) -> Option<PocketFate> {
        let pocket = self.by_key.get_mut(pocket_key)?;
        pocket.records.free_held();
        pocket.consecutive_expiries = 0;
        if pocket.records.record_count() == 0 {
            pocket.status = PocketStatus::Idle;
            return Some(PocketFate::Idle);
        }
        pocket.status = PocketStatus::Queued;
        pocket.position = take_position(&mut self.next_position);
        self.order.insert((pocket.position, pocket_key.clone()));
        Some(PocketFate::Queued)
    }

    /// A lapse's pocket step, the three rules that process `LeaseLapsed` in their
    /// order: the expiry count rises by one; every held and deferred record goes
    /// back to queued at its own position in one `free_held` (`RequeueHeldRecord`,
    /// `RequeueDeferredRecord`); then a pocket whose count has reached `limit`
    /// parks (`LeaseExpiryParksPocket`), checked before its records are counted,
    /// so an empty one parks too; else one with records rejoins the order at the
    /// position it was claimed from, which keeps it at the front
    /// (`LeaseExpiryKeepsPocketAtFront`); else it goes idle
    /// (`LeaseExpiryIdlesEmptyPocket`). A pocket that is not leased is left alone.
    pub(super) fn lapse(&mut self, pocket_key: &PocketKey, limit: u32) {
        let Some(pocket) = self.by_key.get_mut(pocket_key) else {
            return;
        };
        if pocket.status != PocketStatus::Leased {
            return;
        }
        pocket.consecutive_expiries = pocket.consecutive_expiries.saturating_add(1);
        pocket.records.free_held();
        if pocket.consecutive_expiries >= limit {
            pocket.status = PocketStatus::Parked;
        } else if pocket.records.record_count() > 0 {
            pocket.status = PocketStatus::Queued;
            self.order.insert((pocket.position, pocket_key.clone()));
        } else {
            pocket.status = PocketStatus::Idle;
        }
    }

    /// The specification's `Queue.next_position`.
    #[cfg(feature = "testing")]
    pub(super) const fn next_position(&self) -> Position {
        self.next_position
    }

    /// Every pocket, by key.
    #[cfg(feature = "testing")]
    pub(super) fn all(&self) -> impl Iterator<Item = (&PocketKey, &Pocket<R>)> {
        self.by_key.iter()
    }

    /// The pocket order as the queue holds it, front first.
    #[cfg(feature = "testing")]
    pub(super) fn order(&self) -> impl Iterator<Item = &(Position, PocketKey)> {
        self.order.iter()
    }
}

/// The superseded record a `Supersede` or `DropStalePublish` sheds.
fn shed(
    pocket_key: &PocketKey,
    (key, rank, payload): (RecordKey, Rank, Payload),
    reason: ShedReason,
    stamp: Stamp,
) -> SupersededRecord {
    SupersededRecord {
        pocket_key: pocket_key.clone(),
        key,
        rank,
        payload,
        reason,
        shed_at: stamp.shed_at,
        expires_at: stamp.expires_at,
    }
}
