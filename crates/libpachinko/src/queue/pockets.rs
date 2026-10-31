//! The pockets by key, the pocket order and the position counter.
//!
//! Each method is one rule's effect on the pockets, called once the queue has
//! checked that rule's guards on the parties and the leases.

use std::collections::BTreeMap;

use super::shed::Stamp;
use super::{PocketStatus, Publication, SettleOutcome, ShedReason, SupersededRecord};
use crate::error::{ActionError, PublishError};
use crate::records::{HeldRecord, PocketRecords, RecordState, RecordStatus, Version};

/// One pocket.
#[derive(Clone, Debug)]
pub(super) struct Pocket<R> {
    pub(super) key: String,
    pub(super) status: PocketStatus,
    pub(super) position: u64,
    pub(super) consecutive_expiries: u32,
    pub(super) records: R,
}

/// Which publish rule a publish meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    NewPocket,
    NewKey,
    Supersede,
    Stale,
}

/// The pockets, the order of the queued ones and the queue's position counter,
/// which both pockets and records take positions from.
#[derive(Clone, Debug)]
pub(super) struct Pockets<R> {
    by_key: BTreeMap<String, Pocket<R>>,
    order: BTreeMap<u64, String>,
    next_position: u64,
}

impl<R> Default for Pockets<R> {
    fn default() -> Self {
        Self {
            by_key: BTreeMap::new(),
            order: BTreeMap::new(),
            next_position: 0,
        }
    }
}

/// `ReviveIdlePocket` and `ReviveParkedPocket`, the answer to `PocketTouched`: an
/// idle or parked pocket rejoins the order at the next position.
fn touch<R>(pocket: &mut Pocket<R>, order: &mut BTreeMap<u64, String>, next_position: &mut u64) {
    if matches!(pocket.status, PocketStatus::Idle | PocketStatus::Parked) {
        pocket.status = PocketStatus::Queued;
        pocket.position = *next_position;
        *next_position += 1;
        pocket.consecutive_expiries = 0;
        order.insert(pocket.position, pocket.key.clone());
    }
}

impl<R: PocketRecords> Pockets<R> {
    /// The queue's `next_position`.
    #[cfg(feature = "testing")]
    pub(super) const fn next_position(&self) -> u64 {
        self.next_position
    }

    /// Which of the four publish rules takes `publication`, or the equal-rank
    /// placeholder's rejection.
    pub(super) fn target(&self, publication: &Publication) -> Result<Target, PublishError> {
        let Some(pocket) = self.by_key.get(&publication.pocket_key) else {
            return Ok(Target::NewPocket);
        };
        let Some(state) = pocket.records.lookup(&publication.record_key) else {
            return Ok(Target::NewKey);
        };
        match publication.rank.cmp(&state.rank) {
            std::cmp::Ordering::Greater => Ok(Target::Supersede),
            std::cmp::Ordering::Less => Ok(Target::Stale),
            std::cmp::Ordering::Equal => Err(PublishError::EqualRank {
                pocket_key: publication.pocket_key.clone(),
                record_key: publication.record_key.clone(),
                rank: publication.rank,
            }),
        }
    }

    /// `PublishIntoNewPocket`: the pocket takes the next position, its one record
    /// the one after.
    pub(super) fn create(&mut self, publication: Publication) {
        let pocket_position = self.next_position;
        let record_position = pocket_position + 1;
        self.next_position = record_position + 1;
        let mut records = R::default();
        records.insert_queued(
            &publication.record_key,
            Version {
                rank: publication.rank,
                payload: publication.payload,
            },
            record_position,
        );
        let pocket = Pocket {
            key: publication.pocket_key.clone(),
            status: PocketStatus::Queued,
            position: pocket_position,
            consecutive_expiries: 0,
            records,
        };
        self.order
            .insert(pocket_position, publication.pocket_key.clone());
        self.by_key.insert(publication.pocket_key, pocket);
    }

    /// `PublishNewKey`: the record takes the next position, then `PocketTouched`.
    pub(super) fn add_key(&mut self, publication: Publication) {
        let Some(pocket) = self.by_key.get_mut(&publication.pocket_key) else {
            return;
        };
        let version = Version {
            rank: publication.rank,
            payload: publication.payload,
        };
        pocket
            .records
            .insert_queued(&publication.record_key, version, self.next_position);
        self.next_position += 1;
        touch(pocket, &mut self.order, &mut self.next_position);
    }

    /// `Supersede`: the record keeps its position and takes the new rank and
    /// payload, then `PocketTouched`. Returns the old version, shed.
    pub(super) fn supersede(
        &mut self,
        publication: Publication,
        stamp: Stamp,
    ) -> Option<SupersededRecord> {
        let pocket = self.by_key.get_mut(&publication.pocket_key)?;
        let version = Version::new(publication.rank, publication.payload);
        let old = pocket.records.supersede(&publication.record_key, version)?;
        touch(pocket, &mut self.order, &mut self.next_position);
        Some(SupersededRecord {
            pocket_key: publication.pocket_key,
            key: publication.record_key,
            rank: old.rank,
            payload: old.payload,
            reason: ShedReason::Superseded,
            shed_at: stamp.at,
            expires_at: stamp.expires_at,
        })
    }

    /// `DropStalePublish`: the record is untouched, then `PocketTouched`. Returns the
    /// stale version, shed.
    pub(super) fn drop_stale(
        &mut self,
        publication: Publication,
        stamp: Stamp,
    ) -> SupersededRecord {
        self.by_key
            .get_mut(&publication.pocket_key)
            .into_iter()
            .for_each(|pocket| touch(pocket, &mut self.order, &mut self.next_position));
        SupersededRecord {
            pocket_key: publication.pocket_key,
            key: publication.record_key,
            rank: publication.rank,
            payload: publication.payload,
            reason: ShedReason::Stale,
            shed_at: stamp.at,
            expires_at: stamp.expires_at,
        }
    }

    /// `PullClaimsFrontPocket`'s pocket step: the queued pocket with the smallest
    /// position leaves the order, leased. Returns its key.
    pub(super) fn claim_front(&mut self) -> Option<String> {
        let (_, key) = self.order.pop_first()?;
        let pocket = self.by_key.get_mut(&key)?;
        pocket.status = PocketStatus::Leased;
        Some(key)
    }

    /// `HoldRecord` for each of the `limit` oldest queued records of the pocket.
    pub(super) fn hold(&mut self, pocket_key: &str, limit: usize) -> Option<R::Batch> {
        self.by_key
            .get_mut(pocket_key)
            .map(|pocket| pocket.records.hold_oldest(limit))
    }

    /// The record must exist and be held; returns its state.
    pub(super) fn require_held(
        &self,
        pocket_key: &str,
        record_key: &str,
    ) -> Result<RecordState, ActionError> {
        self.by_key
            .get(pocket_key)
            .and_then(|pocket| pocket.records.lookup(record_key))
            .filter(|state| state.status == RecordStatus::Held)
            .ok_or_else(|| ActionError::RecordNotHeld(record_key.to_owned()))
    }

    /// The settle step of an ack or nack on a held record: removed when the held
    /// version is the latest (`AckSettlesRecord`, `NackSettlesRecord`), deferred at
    /// the next position when a newer one waits (`AckDefersNewerVersion`,
    /// `NackDefersNewerVersion`). Returns which, and the held version.
    pub(super) fn settle(
        &mut self,
        pocket_key: &str,
        state: RecordState,
        record_key: &str,
    ) -> (SettleOutcome, Option<Version>) {
        let Some(pocket) = self.by_key.get_mut(pocket_key) else {
            return (SettleOutcome::Settled, None);
        };
        if state.held_rank == Some(state.rank) {
            return (SettleOutcome::Settled, pocket.records.settle(record_key));
        }
        let held = pocket.records.defer(record_key, self.next_position);
        self.next_position += 1;
        (SettleOutcome::Deferred, held)
    }

    /// The pocket step of a release: with records it frees them and rejoins the
    /// order at the next position (`ReleaseRequeuesPocket`); without, it goes idle
    /// (`ReleaseIdlesEmptyPocket`). Either way its expiry count resets.
    pub(super) fn release(&mut self, pocket_key: &str) {
        let Some(pocket) = self.by_key.get_mut(pocket_key) else {
            return;
        };
        pocket.consecutive_expiries = 0;
        if pocket.records.record_count() == 0 {
            pocket.status = PocketStatus::Idle;
            return;
        }
        pocket.records.free_all();
        pocket.status = PocketStatus::Queued;
        pocket.position = self.next_position;
        self.next_position += 1;
        self.order.insert(pocket.position, pocket.key.clone());
    }

    /// The pocket step of a lapse: at the limit it parks
    /// (`LeaseExpiryParksPocket`); below it, with records it rejoins the order at
    /// its old position (`LeaseExpiryKeepsPocketAtFront`) and without it goes idle
    /// (`LeaseExpiryIdlesEmptyPocket`). Held and deferred records are freed.
    pub(super) fn lapse(&mut self, pocket_key: &str, max_consecutive_expiries: u32) {
        let Some(pocket) = self.by_key.get_mut(pocket_key) else {
            return;
        };
        pocket.consecutive_expiries += 1;
        pocket.records.free_all();
        if pocket.consecutive_expiries >= max_consecutive_expiries {
            pocket.status = PocketStatus::Parked;
        } else if pocket.records.record_count() == 0 {
            pocket.status = PocketStatus::Idle;
        } else {
            pocket.status = PocketStatus::Queued;
            self.order.insert(pocket.position, pocket.key.clone());
        }
    }

    /// The pocket's held records, by position.
    pub(super) fn held(&self, pocket_key: &str) -> Vec<HeldRecord> {
        self.by_key
            .get(pocket_key)
            .map(|pocket| pocket.records.held())
            .unwrap_or_default()
    }

    /// Every pocket, by key.
    #[cfg(feature = "testing")]
    pub(super) fn all(&self) -> impl Iterator<Item = &Pocket<R>> {
        self.by_key.values()
    }
}
