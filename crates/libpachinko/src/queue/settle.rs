//! Ack and nack: `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord` and
//! `NackDefersNewerVersion`.

use super::lease::consumers_active_lease;
use super::outcome::SettleOutcome;
use super::state::{State, take_position};
use super::view::{DeadLetter, RecordStatus};
use crate::{ConsumerId, Error, LeaseId, Payload, PocketKey, Rank, RecordKey, Timestamp};

/// The record an ack or nack names, once every guard but the rank comparison holds.
struct Held {
    pocket_key: PocketKey,
    record_key: RecordKey,
    /// The held version's rank.
    rank: Rank,
    /// The held version's payload.
    payload: Payload,
    /// A newer version arrived while it was held: `held_rank < rank`.
    newer: bool,
}

impl State {
    /// `AckSettlesRecord` or `AckDefersNewerVersion`.
    pub(super) fn ack(
        &mut self,
        now: Timestamp,
        consumer: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, Error> {
        let held = held_record(self, now, consumer, lease, record_key)?;
        Ok(settle_or_defer(self, &held))
    }

    /// `NackSettlesRecord` or `NackDefersNewerVersion`: as ack, and the held version
    /// becomes a dead letter.
    pub(super) fn nack(
        &mut self,
        now: Timestamp,
        consumer: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
    ) -> Result<SettleOutcome, Error> {
        let held = held_record(self, now, consumer, lease, record_key)?;
        dead_letter(self, now, consumer, &held);
        Ok(settle_or_defer(self, &held))
    }
}

/// Checks the guards the four rules share, in their order (the lease is active, it
/// is the consumer's, the record exists, it is held), and on success extends the
/// lease, which all four ensure.
fn held_record(
    state: &mut State,
    now: Timestamp,
    consumer: &ConsumerId,
    id: LeaseId,
    record_key: &RecordKey,
) -> Result<Held, Error> {
    let expires_at = now + state.config.lease_ttl();
    let lease = consumers_active_lease(state, consumer, id)?;
    let pocket_key = lease.pocket_key.clone();
    let record = state
        .pockets
        .get(&pocket_key)
        .and_then(|pocket| pocket.records.get(record_key));
    let Some((
        rank,
        RecordStatus::Held {
            rank: held_rank,
            payload,
        },
    )) = record.map(|record| (record.rank, &record.status))
    else {
        return Err(Error::RecordNotHeld(record_key.clone()));
    };
    let held = Held {
        pocket_key,
        record_key: record_key.clone(),
        rank: *held_rank,
        payload: payload.clone(),
        newer: *held_rank < rank,
    };
    if let Some(lease) = state.leases.get_mut(&id) {
        lease.expires_at = expires_at;
    }
    Ok(held)
}

/// The settle rules remove the record; the defer rules send it to the back,
/// deferred, with its held version dropped.
fn settle_or_defer(state: &mut State, held: &Held) -> SettleOutcome {
    let Some(pocket) = state.pockets.get_mut(&held.pocket_key) else {
        return SettleOutcome::Settled;
    };
    if !held.newer {
        pocket.records.remove(&held.record_key);
        return SettleOutcome::Settled;
    }
    if let Some(record) = pocket.records.get_mut(&held.record_key) {
        record.status = RecordStatus::Deferred;
        record.position = take_position(&mut state.next_position);
    }
    SettleOutcome::Deferred
}

/// The nack rules' dead letter: the held version, who nacked it and when.
fn dead_letter(state: &mut State, now: Timestamp, consumer: &ConsumerId, held: &Held) {
    state.dead_letters.push(DeadLetter {
        pocket_key: held.pocket_key.clone(),
        record_key: held.record_key.clone(),
        rank: held.rank,
        payload: held.payload.clone(),
        nacked_by: consumer.clone(),
        nacked_at: now,
        expires_at: now + state.config.dead_letter_ttl(),
    });
}
