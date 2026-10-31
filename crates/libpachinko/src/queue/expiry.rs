//! Lease expiry: `LeaseLapses`, which the clock triggers, and the three rules that
//! process each `LeaseLapsed`: `LeaseExpiryParksPocket`,
//! `LeaseExpiryKeepsPocketAtFront` and `LeaseExpiryIdlesEmptyPocket`, with the
//! `Freed` chain's requeue rules.

use super::outcome::{Lapsed, PocketFate};
use super::state::{State, free_records};
use super::view::{LeaseStatus, PocketStatus};
use crate::{LeaseId, Timestamp};

impl State {
    /// `LeaseLapses` for every active lease whose `expires_at` is at or before
    /// `now`, ordered by `expires_at` and then id. The order changes no outcome:
    /// each lease has its own pocket, and no lapse rule assigns a position.
    pub(super) fn lapse(&mut self, now: Timestamp) -> Vec<Lapsed> {
        let due = due_leases(self, now);
        due.into_iter()
            .filter_map(|id| lease_lapsed(self, now, id))
            .collect()
    }
}

/// The active leases whose `expires_at` has passed, in lapse order.
fn due_leases(state: &State, now: Timestamp) -> Vec<LeaseId> {
    let mut due: Vec<(Timestamp, LeaseId)> = state
        .active
        .values()
        .filter_map(|id| state.leases.get(id).map(|lease| (lease.expires_at, *id)))
        .filter(|(expires_at, _)| *expires_at <= now)
        .collect();
    due.sort_unstable();
    due.into_iter().map(|(_, id)| id).collect()
}

/// `LeaseLapsed`: the lease expires at `now`. At the expiry limit the pocket parks;
/// below it, a pocket with records goes back to queued in its old place and one
/// without goes idle. Held and deferred records are freed either way.
fn lease_lapsed(state: &mut State, now: Timestamp, id: LeaseId) -> Option<Lapsed> {
    let lease = state.leases.get_mut(&id)?;
    lease.status = LeaseStatus::Expired { ended_at: now };
    state.active.remove(&lease.consumer);
    let pocket = state.pockets.get_mut(&lease.pocket_key)?;
    let fate = if pocket.consecutive_expiries + 1 >= state.config.max_consecutive_expiries() {
        PocketFate::Parked
    } else if pocket.records.is_empty() {
        PocketFate::Idle
    } else {
        PocketFate::Queued
    };
    pocket.consecutive_expiries += 1;
    free_records(pocket);
    pocket.status = match fate {
        PocketFate::Parked => PocketStatus::Parked,
        PocketFate::Idle => PocketStatus::Idle,
        PocketFate::Queued => PocketStatus::Queued,
    };
    if fate == PocketFate::Queued {
        state
            .order
            .insert(pocket.position, lease.pocket_key.clone());
    }
    Some(Lapsed {
        lease_id: id,
        consumer: lease.consumer.clone(),
        pocket_key: lease.pocket_key.clone(),
        fate,
    })
}
