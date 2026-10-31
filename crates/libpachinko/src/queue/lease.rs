//! Renew and release: `Renew`, `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket`
//! with the `Freed` chain's requeue rules; and the check every lease action shares.

use super::outcome::PocketFate;
use super::state::{Lease, State, free_records, take_position};
use super::view::{LeaseStatus, PocketStatus};
use crate::{ConsumerId, Error, LeaseId, PocketKey, Timestamp};

impl State {
    /// `Renew`: the lease runs `lease_ttl` from now.
    pub(super) fn renew(
        &mut self,
        now: Timestamp,
        consumer: &ConsumerId,
        lease: LeaseId,
    ) -> Result<Timestamp, Error> {
        let expires_at = now + self.config.lease_ttl();
        let lease = consumers_active_lease(self, consumer, lease)?;
        lease.expires_at = expires_at;
        Ok(expires_at)
    }

    /// `Release`: the lease ends, and its pocket is requeued at the back with its
    /// records freed, or idled when it has none.
    pub(super) fn release(
        &mut self,
        now: Timestamp,
        consumer: &ConsumerId,
        lease: LeaseId,
    ) -> Result<PocketFate, Error> {
        let pocket_key = end_lease(self, consumer, lease, now)?;
        Ok(release_pocket(self, &pocket_key))
    }
}

/// The lease an ack, nack, renew or release names, checked as each rule's `requires`
/// reads: it is active, and then it is the consumer's. An id that names no lease
/// names no active one (`ActionsNameTheirLease`).
pub(super) fn consumers_active_lease<'a>(
    state: &'a mut State,
    consumer: &ConsumerId,
    id: LeaseId,
) -> Result<&'a mut Lease, Error> {
    match state.leases.get_mut(&id) {
        Some(lease) if lease.status == LeaseStatus::Active && &lease.consumer == consumer => {
            Ok(lease)
        }
        Some(lease) if lease.status == LeaseStatus::Active => Err(Error::LeaseNotYours(id)),
        _ => Err(Error::LeaseNotActive(id)),
    }
}

/// Ends the consumer's active lease as released at `now`, returning its pocket.
fn end_lease(
    state: &mut State,
    consumer: &ConsumerId,
    id: LeaseId,
    now: Timestamp,
) -> Result<PocketKey, Error> {
    let lease = consumers_active_lease(state, consumer, id)?;
    lease.status = LeaseStatus::Released { ended_at: now };
    let pocket_key = lease.pocket_key.clone();
    state.active.remove(consumer);
    Ok(pocket_key)
}

/// `ReleaseRequeuesPocket` when the pocket has records, with `RequeueHeldRecord` and
/// `RequeueDeferredRecord` for them; `ReleaseIdlesEmptyPocket` when it has none.
/// Either way its expiries are forgotten.
fn release_pocket(state: &mut State, pocket_key: &PocketKey) -> PocketFate {
    let Some(pocket) = state.pockets.get_mut(pocket_key) else {
        return PocketFate::Idle;
    };
    pocket.consecutive_expiries = 0;
    if pocket.records.is_empty() {
        pocket.status = PocketStatus::Idle;
        return PocketFate::Idle;
    }
    free_records(pocket);
    let position = take_position(&mut state.next_position);
    pocket.status = PocketStatus::Queued;
    pocket.position = position;
    state.order.insert(position, pocket_key.clone());
    PocketFate::Queued
}
