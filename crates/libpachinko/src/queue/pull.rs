//! Pull: `PullWithinLease`, `PullClaimsFrontPocket`, the `Handed` chain's
//! `HoldRecord`, and `PullFindsNoPocket`; and the `Consuming` surface's `exposes`.

use std::collections::BTreeMap;

use super::outcome::{Batch, HeldLease, HeldRecord, PullOutcome};
use super::state::{Lease, Record, State};
use super::view::{LeaseStatus, PocketStatus, RecordStatus};
use crate::{ConsumerId, Error, LeaseId, LeaseIds, Position, RecordKey, Timestamp};

impl State {
    /// `Pull`: within the consumer's lease if it holds one, else a claim of the
    /// front pocket if one is queued, else no pocket.
    pub(super) fn pull(
        &mut self,
        now: Timestamp,
        lease_ids: &impl LeaseIds,
        consumer: &ConsumerId,
        limit: i64,
    ) -> Result<PullOutcome, Error> {
        known_consumer(self, consumer)?;
        let lease = self
            .active
            .get(consumer)
            .copied()
            .or_else(|| claim_front_pocket(self, now, lease_ids, consumer));
        Ok(lease
            .and_then(|lease| hand_out(self, now, lease, limit))
            .map_or(PullOutcome::NoPocketAvailable, PullOutcome::Batch))
    }

    /// The `Consuming` surface's `exposes`: the consumer's active lease, its pocket,
    /// its `expires_at` and the held records' keys, held ranks and held payloads.
    pub(super) fn lease_of(&self, consumer: &ConsumerId) -> Result<Option<HeldLease>, Error> {
        known_consumer(self, consumer)?;
        Ok(self.active.get(consumer).and_then(|id| {
            let lease = self.leases.get(id)?;
            let pocket = self.pockets.get(&lease.pocket_key)?;
            let mut held: Vec<(Position, HeldRecord)> =
                pocket.records.iter().filter_map(held_version).collect();
            held.sort_by_key(|(position, _)| *position);
            Some(HeldLease {
                lease_id: *id,
                pocket_key: lease.pocket_key.clone(),
                expires_at: lease.expires_at,
                records: held.into_iter().map(|(_, record)| record).collect(),
            })
        }))
    }
}

/// The `Consuming` surface faces a consumer that has joined.
fn known_consumer(state: &State, consumer: &ConsumerId) -> Result<(), Error> {
    if state.consumers.contains(consumer) {
        Ok(())
    } else {
        Err(Error::UnknownConsumer(consumer.clone()))
    }
}

/// `PullClaimsFrontPocket`'s claim: the queued pocket with the smallest position
/// leaves the pocket order under a new active lease. `None` is `PullFindsNoPocket`.
fn claim_front_pocket(
    state: &mut State,
    now: Timestamp,
    lease_ids: &impl LeaseIds,
    consumer: &ConsumerId,
) -> Option<LeaseId> {
    let (_, pocket_key) = state.order.pop_first()?;
    let pocket = state.pockets.get_mut(&pocket_key)?;
    let id =
        std::iter::repeat_with(|| lease_ids.next()).find(|id| !state.leases.contains_key(id))?;
    pocket.status = PocketStatus::Leased;
    let lease = Lease {
        pocket_key,
        consumer: consumer.clone(),
        status: LeaseStatus::Active,
        granted_at: now,
        expires_at: now + state.config.lease_ttl(),
    };
    state.leases.insert(id, lease);
    state.active.insert(consumer.clone(), id);
    Some(id)
}

/// The batch of `PullWithinLease` and `PullClaimsFrontPocket`: the lease runs from
/// now, and the oldest queued records, at most `min(limit, max_pull_batch)` and none
/// for a limit that is not positive, are handed out and held. They are popped off the
/// front of the pocket's queued index, so a pull reads only the records it hands out
/// and a limit that is not positive reads none.
fn hand_out(state: &mut State, now: Timestamp, id: LeaseId, limit: i64) -> Option<Batch> {
    let lease = state.leases.get_mut(&id)?;
    let pocket = state.pockets.get_mut(&lease.pocket_key)?;
    lease.expires_at = now + state.config.lease_ttl();
    let most = limit.clamp(0, i64::from(state.config.max_pull_batch()));
    let records = std::iter::from_fn(|| pocket.queued.pop_first())
        .take(usize::try_from(most).unwrap_or_default())
        .filter_map(|(_, key)| hold_queued(&mut pocket.records, &key))
        .collect();
    Some(Batch {
        lease_id: id,
        pocket_key: lease.pocket_key.clone(),
        expires_at: lease.expires_at,
        records,
    })
}

/// `HoldRecord` for a key popped off the queued index. The index names only queued
/// records, so a key it names that is missing or not queued is a rule that forgot the
/// index: a debug build fails on it, and a release build hands nothing out for it
/// rather than holding a record twice.
fn hold_queued(records: &mut BTreeMap<RecordKey, Record>, key: &RecordKey) -> Option<HeldRecord> {
    let record = records.get_mut(key);
    debug_assert!(
        record
            .as_ref()
            .is_some_and(|record| record.status == RecordStatus::Queued),
        "the queued index names {key:?}, which is not a queued record"
    );
    record
        .filter(|record| record.status == RecordStatus::Queued)
        .map(|record| hold_record(key, record))
}

/// `HoldRecord`: the handed record is held at its current rank and payload.
fn hold_record(key: &RecordKey, record: &mut Record) -> HeldRecord {
    record.status = RecordStatus::Held {
        rank: record.rank,
        payload: record.payload.clone(),
    };
    HeldRecord {
        record_key: key.clone(),
        rank: record.rank,
        payload: record.payload.clone(),
    }
}

/// A held record's position and held version, as the batch handed it out; `None`
/// for a record that is not held.
fn held_version((key, record): (&RecordKey, &Record)) -> Option<(Position, HeldRecord)> {
    match &record.status {
        RecordStatus::Held { rank, payload } => Some((
            record.position,
            HeldRecord {
                record_key: key.clone(),
                rank: *rank,
                payload: payload.clone(),
            },
        )),
        RecordStatus::Queued | RecordStatus::Deferred => None,
    }
}
