//! The pockets by key, the pocket order and the queue's position counter.
//!
//! Each method is one rule's effect on the pockets, called once the queue has
//! checked that rule's guards on the parties and the leases.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    PocketFate, PocketStatus, Publication, PublishOutcome, ShedReason, SupersededRecord,
};
use crate::error::PublishError;
use crate::ids::{HeldRecord, Mark, MarkedRecord, Payload, PocketKey, Rank, RecordKey};
use crate::records::{EndedVersion, PocketRecords, Position, RecordStatus};
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

/// The `PocketTouched` dispatch a publish into an existing pocket that adds or
/// raises a record ends in (`PublishNewKey`, `Supersede`,
/// `DeferBehindHeldVersion`); a stale publish never reaches it. A queued or leased
/// pocket is untouched. The idle arm is `ReviveIdlePocket`, which a release or a
/// lapse that idles a pocket makes reachable, and the parked arm
/// `ReviveParkedPocket`, which a lapse that parks one makes reachable: either way
/// the pocket rejoins the order at the back with its expiry count reset.
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
    /// Applies one publish: `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`,
    /// `DeferBehindHeldVersion` or `DropStalePublish`, by their disjoint guards,
    /// each of the three that add or raise a record in an existing pocket ending in
    /// the `PocketTouched` dispatch; a stale publish leaves the pocket as it is.
    /// Returns the rule taken and what it shed.
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
        let arrival = (record_key, rank, payload);
        let (outcome, shed) = raise(pocket, &mut self.next_position, &pocket_key, arrival, stamp)?;
        // Exhaustive, so a new outcome must say whether it touches its pocket: only
        // a rule that adds or raises a record does (a stale publish does not).
        let touches = match outcome {
            PublishOutcome::Created | PublishOutcome::NewKey | PublishOutcome::Superseded => true,
            PublishOutcome::Stale => false,
        };
        if touches {
            touch(
                &pocket_key,
                pocket,
                &mut self.order,
                &mut self.next_position,
            );
        }
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

    /// The pocket's held records with their marks, smallest position first.
    pub(super) fn held(&self, key: &PocketKey) -> Vec<MarkedRecord> {
        self.by_key
            .get(key)
            .map(|pocket| pocket.records.held())
            .unwrap_or_default()
    }

    /// The mark of the record `key` if the pocket holds it: the ack and nack
    /// rules' guards `exists record` and `record.status = held`, and the mark their
    /// last guard reads. `None` when the record is absent or queued.
    pub(super) fn mark_of(&self, pocket_key: &PocketKey, key: &RecordKey) -> Option<Mark> {
        let record = self.by_key.get(pocket_key)?.records.lookup(key)?;
        match record.status {
            RecordStatus::Held { mark, .. } => Some(mark),
            RecordStatus::Queued => None,
        }
    }

    /// The first mark of a held record (`AckMarksRecord`, `NackMarksRecord`): the
    /// record is marked, with `at` as its `nacked_at` for a nack, and the pocket's
    /// consecutive-expiry count resets to 0. A repeat never calls it, so the first
    /// mark and its time stand.
    pub(super) fn mark(
        &mut self,
        pocket_key: &PocketKey,
        key: &RecordKey,
        mark: Mark,
        at: Timestamp,
    ) {
        if let Some(pocket) = self.by_key.get_mut(pocket_key) {
            pocket.records.mark(key, mark, at);
            pocket.consecutive_expiries = 0;
        }
    }

    /// A release's pocket step: the lease's end over every held record, by the six
    /// `Freed` rules, in one `end_lease`; then, with records left, the pocket
    /// rejoins the order at the back (`ReleaseRequeuesPocket`), and with none it
    /// goes idle (`ReleaseIdlesEmptyPocket`). Either way its expiry count resets.
    /// Returns the fate and the held versions the lease's end took out, for the
    /// shed queues, or `None` when no such pocket exists.
    pub(super) fn release(
        &mut self,
        pocket_key: &PocketKey,
    ) -> Option<(PocketFate, Vec<EndedVersion>)> {
        let pocket = self.by_key.get_mut(pocket_key)?;
        let ended = pocket.records.end_lease();
        pocket.consecutive_expiries = 0;
        if pocket.records.record_count() == 0 {
            pocket.status = PocketStatus::Idle;
            return Some((PocketFate::Idle, ended));
        }
        pocket.status = PocketStatus::Queued;
        pocket.position = take_position(&mut self.next_position);
        self.order.insert((pocket.position, pocket_key.clone()));
        Some((PocketFate::Queued, ended))
    }

    /// A lapse's pocket step, the three rules that process `LeaseLapsed` in their
    /// order: the expiry count rises by one; the lease's end runs over every held
    /// record, by the six `Freed` rules, in one `end_lease`; then a pocket whose
    /// count has reached `limit` parks (`LeaseExpiryParksPocket`), checked before
    /// its records are counted, so an empty one parks too; else one with records
    /// rejoins the order at the position it was claimed from, which keeps it at the
    /// front (`LeaseExpiryKeepsPocketAtFront`); else it goes idle
    /// (`LeaseExpiryIdlesEmptyPocket`). Returns the held versions the lease's end
    /// took out, for the shed queues. A pocket that is not leased is left alone.
    pub(super) fn lapse(&mut self, pocket_key: &PocketKey, limit: u32) -> Vec<EndedVersion> {
        let Some(pocket) = self.by_key.get_mut(pocket_key) else {
            return Vec::new();
        };
        if pocket.status != PocketStatus::Leased {
            return Vec::new();
        }
        pocket.consecutive_expiries = pocket.consecutive_expiries.saturating_add(1);
        let ended = pocket.records.end_lease();
        if pocket.consecutive_expiries >= limit {
            pocket.status = PocketStatus::Parked;
        } else if pocket.records.record_count() > 0 {
            pocket.status = PocketStatus::Queued;
            self.order.insert((pocket.position, pocket_key.clone()));
        } else {
            pocket.status = PocketStatus::Idle;
        }
        ended
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

/// A publish into an existing pocket, by the four rules' disjoint guards on the
/// stored rank and the record's state: a new key takes `next_position`
/// (`PublishNewKey`); a higher rank supersedes a version nobody holds in place and
/// sheds it (`Supersede`), or, over a held record with nothing deferred, is
/// deferred behind the held version at a fresh position from `next_position`,
/// shedding nothing (`DeferBehindHeldVersion`); a lower rank is shed as stale
/// (`DropStalePublish`). The pocket's touch is the caller's.
///
/// # Errors
///
/// [`PublishError::EqualRank`] when the rank equals the stored one; nothing
/// changes.
fn raise<R: PocketRecords>(
    pocket: &mut Pocket<R>,
    next_position: &mut Position,
    pocket_key: &PocketKey,
    (record_key, rank, payload): (RecordKey, Rank, Payload),
    stamp: Stamp,
) -> Result<(PublishOutcome, Option<SupersededRecord>), PublishError> {
    let Some(stored) = pocket.records.lookup(&record_key) else {
        let position = take_position(next_position);
        pocket
            .records
            .insert_queued(record_key, rank, payload, position);
        return Ok((PublishOutcome::NewKey, None));
    };
    if rank == stored.rank {
        return Err(PublishError::EqualRank {
            key: record_key,
            rank,
        });
    }
    if rank < stored.rank {
        let stale = (record_key, rank, payload);
        let shed = shed(pocket_key, stale, ShedReason::Stale, stamp);
        return Ok((PublishOutcome::Stale, Some(shed)));
    }
    if let RecordStatus::Held {
        deferred_position: None,
        ..
    } = stored.status
    {
        let position = take_position(next_position);
        pocket
            .records
            .defer_behind_held(&record_key, rank, payload, position);
        return Ok((PublishOutcome::Superseded, None));
    }
    let loser = pocket.records.supersede(&record_key, rank, payload);
    let shed = loser.map(|(rank, payload)| {
        shed(
            pocket_key,
            (record_key, rank, payload),
            ShedReason::Superseded,
            stamp,
        )
    });
    Ok((PublishOutcome::Superseded, shed))
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
