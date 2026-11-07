//! The pockets by key, the pocket order and the queue's position counter.
//!
//! Each method is one rule's effect on the pockets, called once the queue has
//! checked that rule's guards on the parties and the leases.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{PocketStatus, Publication, PublishOutcome, ShedReason, SupersededRecord};
use crate::clock::Timestamp;
use crate::error::PublishError;
use crate::ids::{Payload, PocketKey, Position, Rank, RecordKey};
use crate::records::{HeldRecord, PocketRecords};

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
/// queued or leased pocket is untouched. The idle arm is `ReviveIdlePocket` (owned
/// by T25) and the parked arm `ReviveParkedPocket` (owned by T26): either way the
/// pocket rejoins the order at the back with its expiry count reset.
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
