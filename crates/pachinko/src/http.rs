//! The REST API: one route per surface action, JSON bodies with base64 payloads, and
//! one mapping from the library's errors to HTTP statuses. `docs/reference/http-api.md`
//! owns the route table.
//!
//! Every handler takes the queue's mutex, runs one synchronous queue operation and
//! releases it before it returns; nothing awaits while the lock is held.

use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use libpachinko::{
    Batch, ConsumerId, Error, HeldLease, HeldRecord, LeaseId, Payload, PocketFate, PocketKey,
    Publication, PublishOutcome, PublisherId, PullOutcome, Queue, Rank, RecordKey, SettleOutcome,
};
use serde::{Deserialize, Serialize};

use crate::{RandomLeaseIds, SystemClock};

/// The queue every handler and the sweep share.
pub(crate) type SharedQueue = Arc<Mutex<Queue<SystemClock, RandomLeaseIds>>>;

/// Runs one queue operation under the lock. A lock poisoned by a panicking holder is
/// taken anyway: every operation leaves the queue whole or returns before changing it.
pub(crate) fn with_queue<T>(
    queue: &SharedQueue,
    operation: impl FnOnce(&mut Queue<SystemClock, RandomLeaseIds>) -> T,
) -> T {
    let mut guard = queue.lock().unwrap_or_else(PoisonError::into_inner);
    operation(&mut guard)
}

/// The router: the ten routes of `docs/reference/http-api.md`, each logged.
pub(crate) fn router(queue: SharedQueue) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/publishers/{publisher_id}", post(publisher_joins))
        .route("/consumers/{consumer_id}", post(consumer_joins))
        .route("/pockets/{pocket_key}/records/{record_key}", put(publish))
        .route("/consumers/{consumer_id}/pull", post(pull))
        .route("/consumers/{consumer_id}/lease", get(lease_of))
        .route("/leases/{lease_id}/ack", post(ack))
        .route("/leases/{lease_id}/nack", post(nack))
        .route("/leases/{lease_id}/renew", post(renew))
        .route("/leases/{lease_id}/release", post(release))
        .layer(middleware::from_fn(log_request))
        .with_state(queue)
}

/// Why a request was refused.
#[derive(Debug)]
enum Problem {
    /// A rule's `requires` failed.
    Refused(Error),
    /// The request could not be read: a malformed body, lease id or payload.
    BadRequest(String),
}

impl From<Error> for Problem {
    fn from(error: Error) -> Self {
        Self::Refused(error)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let (status, error, message) = match self {
            Self::Refused(error) => (status(&error), code(&error), error.to_string()),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
        };
        (status, Json(ErrorBody { error, message })).into_response()
    }
}

/// The one mapping from a refusal to its status.
pub(crate) const fn status(error: &Error) -> StatusCode {
    match error {
        Error::PublisherExists(_)
        | Error::ConsumerExists(_)
        | Error::EqualRank { .. }
        | Error::LeaseNotActive(_) => StatusCode::CONFLICT,
        Error::UnknownPublisher(_) | Error::UnknownConsumer(_) | Error::RecordNotHeld(_) => {
            StatusCode::NOT_FOUND
        }
        Error::LeaseNotYours(_) => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// The refusal's name on the wire: its variant in snake case.
pub(crate) const fn code(error: &Error) -> &'static str {
    match error {
        Error::PublisherExists(_) => "publisher_exists",
        Error::ConsumerExists(_) => "consumer_exists",
        Error::UnknownPublisher(_) => "unknown_publisher",
        Error::UnknownConsumer(_) => "unknown_consumer",
        Error::EqualRank { .. } => "equal_rank",
        Error::LeaseNotActive(_) => "lease_not_active",
        Error::LeaseNotYours(_) => "lease_not_yours",
        Error::RecordNotHeld(_) => "record_not_held",
        Error::InvalidConfig(_) => "invalid_config",
        _ => "internal",
    }
}

/// A JSON body, or 400 `bad_request` in place of axum's plain-text rejection.
fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, Problem> {
    body.map(|Json(body)| body)
        .map_err(|rejection| Problem::BadRequest(rejection.body_text()))
}

fn lease_id(text: &str) -> Result<LeaseId, Problem> {
    text.parse()
        .map_err(|error| Problem::BadRequest(format!("{error}: {text:?}")))
}

fn payload(text: &str) -> Result<Payload, Problem> {
    STANDARD
        .decode(text)
        .map(Payload::new)
        .map_err(|error| Problem::BadRequest(format!("payload is not base64: {error}")))
}

fn reply<T: Serialize>(result: Result<T, Problem>) -> Response {
    result.map_or_else(IntoResponse::into_response, |body| {
        Json(body).into_response()
    })
}

fn created(result: Result<(), Error>) -> Response {
    result.map_or_else(
        |error| Problem::Refused(error).into_response(),
        |()| StatusCode::CREATED.into_response(),
    )
}

#[derive(Deserialize)]
struct PublishBody {
    publisher_id: String,
    rank: i64,
    payload: String,
}

#[derive(Deserialize)]
struct PullBody {
    limit: i64,
}

#[derive(Deserialize)]
struct SettleBody {
    consumer_id: String,
    record_key: String,
}

#[derive(Deserialize)]
struct LeaseBody {
    consumer_id: String,
}

#[derive(Serialize)]
struct OutcomeReply {
    outcome: &'static str,
}

#[derive(Serialize)]
struct RecordReply {
    record_key: String,
    rank: i64,
    payload: String,
}

impl From<HeldRecord> for RecordReply {
    fn from(record: HeldRecord) -> Self {
        Self {
            record_key: record.record_key.to_string(),
            rank: record.rank.get(),
            payload: STANDARD.encode(record.payload.as_bytes()),
        }
    }
}

#[derive(Serialize)]
struct LeaseReply {
    lease_id: String,
    pocket_key: String,
    expires_at: u64,
    records: Vec<RecordReply>,
}

impl From<Batch> for LeaseReply {
    fn from(batch: Batch) -> Self {
        Self {
            lease_id: batch.lease_id.to_string(),
            pocket_key: batch.pocket_key.to_string(),
            expires_at: batch.expires_at.as_millis(),
            records: batch.records.into_iter().map(RecordReply::from).collect(),
        }
    }
}

impl From<HeldLease> for LeaseReply {
    fn from(lease: HeldLease) -> Self {
        Self {
            lease_id: lease.lease_id.to_string(),
            pocket_key: lease.pocket_key.to_string(),
            expires_at: lease.expires_at.as_millis(),
            records: lease.records.into_iter().map(RecordReply::from).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum PullReply {
    Batch(LeaseReply),
    NoPocket { no_pocket_available: bool },
}

#[derive(Serialize)]
struct RenewReply {
    expires_at: u64,
}

#[derive(Serialize)]
struct ReleaseReply {
    pocket: &'static str,
}

const fn publish_outcome(outcome: PublishOutcome) -> &'static str {
    match outcome {
        PublishOutcome::Created => "created",
        PublishOutcome::NewKey => "new_key",
        PublishOutcome::Superseded => "superseded",
        PublishOutcome::Stale => "stale",
        _ => "unknown",
    }
}

const fn settle_outcome(outcome: SettleOutcome) -> &'static str {
    match outcome {
        SettleOutcome::Settled => "settled",
        SettleOutcome::Deferred => "deferred",
        _ => "unknown",
    }
}

const fn pocket_fate(fate: PocketFate) -> &'static str {
    match fate {
        PocketFate::Queued => "queued",
        PocketFate::Idle => "idle",
        PocketFate::Parked => "parked",
        _ => "unknown",
    }
}

fn pull_reply(outcome: PullOutcome) -> PullReply {
    match outcome {
        PullOutcome::Batch(batch) => PullReply::Batch(LeaseReply::from(batch)),
        _ => PullReply::NoPocket {
            no_pocket_available: true,
        },
    }
}

async fn healthz() -> &'static str {
    "ok"
}

async fn publisher_joins(State(queue): State<SharedQueue>, Path(id): Path<String>) -> Response {
    created(with_queue(&queue, |q| {
        q.publisher_joins(PublisherId::new(id))
    }))
}

async fn consumer_joins(State(queue): State<SharedQueue>, Path(id): Path<String>) -> Response {
    created(with_queue(&queue, |q| {
        q.consumer_joins(ConsumerId::new(id))
    }))
}

async fn publish(
    State(queue): State<SharedQueue>,
    Path((pocket_key, record_key)): Path<(String, String)>,
    request: Result<Json<PublishBody>, JsonRejection>,
) -> Response {
    reply(body(request).and_then(|request| {
        let publication = Publication::new(
            PublisherId::new(request.publisher_id),
            PocketKey::new(pocket_key),
            RecordKey::new(record_key),
            Rank::new(request.rank),
            payload(&request.payload)?,
        );
        let outcome = with_queue(&queue, |q| q.publish(publication))?;
        Ok(OutcomeReply {
            outcome: publish_outcome(outcome),
        })
    }))
}

async fn pull(
    State(queue): State<SharedQueue>,
    Path(consumer): Path<String>,
    request: Result<Json<PullBody>, JsonRejection>,
) -> Response {
    reply(body(request).and_then(|request| {
        let consumer = ConsumerId::new(consumer);
        let outcome = with_queue(&queue, |q| q.pull(&consumer, request.limit))?;
        Ok(pull_reply(outcome))
    }))
}

async fn lease_of(State(queue): State<SharedQueue>, Path(consumer): Path<String>) -> Response {
    let consumer = ConsumerId::new(consumer);
    with_queue(&queue, |q| q.lease_of(&consumer)).map_or_else(
        |error| Problem::Refused(error).into_response(),
        |lease| {
            lease.map_or_else(
                || StatusCode::NO_CONTENT.into_response(),
                |lease| Json(LeaseReply::from(lease)).into_response(),
            )
        },
    )
}

/// Which of the two settle actions a request names.
#[derive(Clone, Copy)]
enum Settle {
    Ack,
    Nack,
}

fn settle(
    queue: &SharedQueue,
    lease: &str,
    request: Result<Json<SettleBody>, JsonRejection>,
    action: Settle,
) -> Response {
    reply(lease_id(lease).and_then(|lease| {
        let request = body(request)?;
        let (consumer, key) = (
            ConsumerId::new(request.consumer_id),
            RecordKey::new(request.record_key),
        );
        let outcome = with_queue(queue, |q| match action {
            Settle::Ack => q.ack(&consumer, lease, &key),
            Settle::Nack => q.nack(&consumer, lease, &key),
        })?;
        Ok(OutcomeReply {
            outcome: settle_outcome(outcome),
        })
    }))
}

async fn ack(
    State(queue): State<SharedQueue>,
    Path(lease): Path<String>,
    request: Result<Json<SettleBody>, JsonRejection>,
) -> Response {
    settle(&queue, &lease, request, Settle::Ack)
}

async fn nack(
    State(queue): State<SharedQueue>,
    Path(lease): Path<String>,
    request: Result<Json<SettleBody>, JsonRejection>,
) -> Response {
    settle(&queue, &lease, request, Settle::Nack)
}

async fn renew(
    State(queue): State<SharedQueue>,
    Path(lease): Path<String>,
    request: Result<Json<LeaseBody>, JsonRejection>,
) -> Response {
    reply(lease_id(&lease).and_then(|lease| {
        let consumer = ConsumerId::new(body(request)?.consumer_id);
        let expires_at = with_queue(&queue, |q| q.renew(&consumer, lease))?;
        Ok(RenewReply {
            expires_at: expires_at.as_millis(),
        })
    }))
}

async fn release(
    State(queue): State<SharedQueue>,
    Path(lease): Path<String>,
    request: Result<Json<LeaseBody>, JsonRejection>,
) -> Response {
    reply(lease_id(&lease).and_then(|lease| {
        let consumer = ConsumerId::new(body(request)?.consumer_id);
        let fate = with_queue(&queue, |q| q.release(&consumer, lease))?;
        Ok(ReleaseReply {
            pocket: pocket_fate(fate),
        })
    }))
}

/// One log line per request: method, path and status.
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    tracing::info!(%method, path, status = response.status().as_u16(), "request");
    response
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use libpachinko::{ConsumerId, Error, LeaseId, PocketKey, PublisherId, Rank, RecordKey};

    use super::{code, status};

    #[test]
    fn every_refusal_has_its_status_and_name() {
        let lease = LeaseId::from_u128(1);
        let cases = [
            (
                Error::PublisherExists(PublisherId::new("p")),
                StatusCode::CONFLICT,
                "publisher_exists",
            ),
            (
                Error::ConsumerExists(ConsumerId::new("c")),
                StatusCode::CONFLICT,
                "consumer_exists",
            ),
            (
                Error::UnknownPublisher(PublisherId::new("p")),
                StatusCode::NOT_FOUND,
                "unknown_publisher",
            ),
            (
                Error::UnknownConsumer(ConsumerId::new("c")),
                StatusCode::NOT_FOUND,
                "unknown_consumer",
            ),
            (
                Error::EqualRank {
                    pocket_key: PocketKey::new("a"),
                    record_key: RecordKey::new("k"),
                    rank: Rank::new(1),
                },
                StatusCode::CONFLICT,
                "equal_rank",
            ),
            (
                Error::LeaseNotActive(lease),
                StatusCode::CONFLICT,
                "lease_not_active",
            ),
            (
                Error::LeaseNotYours(lease),
                StatusCode::FORBIDDEN,
                "lease_not_yours",
            ),
            (
                Error::RecordNotHeld(RecordKey::new("k")),
                StatusCode::NOT_FOUND,
                "record_not_held",
            ),
            (
                Error::InvalidConfig("x"),
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_config",
            ),
        ];
        for (error, want_status, want_code) in cases {
            assert_eq!(
                (status(&error), code(&error)),
                (want_status, want_code),
                "{error:?}"
            );
        }
    }
}
