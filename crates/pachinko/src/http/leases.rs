//! The four lease routes: ack, nack, renew and release, each naming the lease it
//! acts under in its path. The lease id is parsed here, before anything reaches the
//! owner; every other guard is the library's, in the order of the rules' `requires`.

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use libpachinko::{ConsumerId, LeaseId, PocketFate, RecordKey, SettleOutcome};
use serde::{Deserialize, Serialize};

use super::{AppState, BoundedJson, Refusal, unmapped};

/// The body of `POST /leases/{lease_id}/ack` and `.../nack`.
#[derive(Debug, Deserialize)]
pub(super) struct SettleBody {
    consumer_id: String,
    record_key: String,
}

/// The body of `POST /leases/{lease_id}/renew` and `.../release`.
#[derive(Debug, Deserialize)]
pub(super) struct LeaseBody {
    consumer_id: String,
}

/// An ack's or nack's reply: the rule that took it.
#[derive(Debug, Serialize)]
pub(super) struct SettleReply {
    outcome: &'static str,
}

/// A renew's reply: the lease's new `expires_at`, in milliseconds since the Unix
/// epoch.
#[derive(Debug, Serialize)]
pub(super) struct RenewReply {
    expires_at: u64,
}

/// A release's reply: where the pocket went.
#[derive(Debug, Serialize)]
pub(super) struct ReleaseReply {
    pocket: &'static str,
}

/// The settle outcome's name on the wire.
fn settle_outcome(outcome: SettleOutcome) -> Result<&'static str, Refusal> {
    match outcome {
        SettleOutcome::Settled => Ok("settled"),
        SettleOutcome::Deferred => Ok("deferred"),
        other => Err(unmapped(&other)),
    }
}

/// The pocket fate's name on the wire.
fn pocket_fate(fate: PocketFate) -> Result<&'static str, Refusal> {
    match fate {
        PocketFate::Queued => Ok("queued"),
        PocketFate::Idle => Ok("idle"),
        other => Err(unmapped(&other)),
    }
}

/// The path's lease id: exactly 32 lower-case hexadecimal digits, or 400
/// `bad_request` before anything reaches the owner.
fn lease_id(path: Result<Path<String>, PathRejection>) -> Result<LeaseId, Refusal> {
    let Path(text) = path?;
    text.parse()
        .map_err(|error| Refusal::BadRequest(format!("lease id {text:?}: {error}")))
}

/// `POST /leases/{lease_id}/ack`: 200 with `settled` or `deferred`.
pub(super) async fn ack(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    BoundedJson(body): BoundedJson<SettleBody>,
) -> Result<Json<SettleReply>, Refusal> {
    let lease = lease_id(path)?;
    let consumer = ConsumerId::new(body.consumer_id);
    let key = RecordKey::new(body.record_key);
    let outcome = state.queue.ack(consumer, lease, key).await??;
    Ok(Json(SettleReply {
        outcome: settle_outcome(outcome)?,
    }))
}

/// `POST /leases/{lease_id}/nack`: 200 with `settled` or `deferred`.
pub(super) async fn nack(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    BoundedJson(body): BoundedJson<SettleBody>,
) -> Result<Json<SettleReply>, Refusal> {
    let lease = lease_id(path)?;
    let consumer = ConsumerId::new(body.consumer_id);
    let key = RecordKey::new(body.record_key);
    let outcome = state.queue.nack(consumer, lease, key).await??;
    Ok(Json(SettleReply {
        outcome: settle_outcome(outcome)?,
    }))
}

/// `POST /leases/{lease_id}/renew`: 200 with the new `expires_at`.
pub(super) async fn renew(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    BoundedJson(body): BoundedJson<LeaseBody>,
) -> Result<Json<RenewReply>, Refusal> {
    let lease = lease_id(path)?;
    let consumer = ConsumerId::new(body.consumer_id);
    let expires_at = state.queue.renew(consumer, lease).await??;
    Ok(Json(RenewReply {
        expires_at: expires_at.as_millis(),
    }))
}

/// `POST /leases/{lease_id}/release`: 200 with `queued` or `idle`.
pub(super) async fn release(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    BoundedJson(body): BoundedJson<LeaseBody>,
) -> Result<Json<ReleaseReply>, Refusal> {
    let lease = lease_id(path)?;
    let consumer = ConsumerId::new(body.consumer_id);
    let (fate, _ended) = state.queue.release(consumer, lease).await??;
    Ok(Json(ReleaseReply {
        pocket: pocket_fate(fate)?,
    }))
}
