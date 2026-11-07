//! The HTTP API: one route per action, JSON bodies with base64 payloads, and one
//! exhaustive map from a refusal to its status and code. `docs/reference/http-api.md`
//! owns the route table.
//!
//! Every handler reads its request, sends one command through the
//! [`QueueHandle`] and turns the answer into a reply. A handler holds nothing the
//! owner needs across an `.await`: the handle is a channel sender.

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use libpachinko::error::{JoinError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
use libpachinko::queue::{LeaseView, Publication, PublishOutcome, PullOutcome};
use libpachinko::records::HeldRecord;
use serde::{Deserialize, Serialize};

use crate::actor::{QueueHandle, Stopped};

/// What every handler shares: the handle to the queue's owner.
#[derive(Clone, Debug)]
struct AppState {
    queue: QueueHandle,
}

/// The router over `queue`, each request logged at `debug`.
pub(crate) fn router(queue: QueueHandle) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/publishers/{publisher_id}", post(publisher_joins))
        .route("/consumers/{consumer_id}", post(consumer_joins))
        .route("/pockets/{pocket_key}/records/{record_key}", put(publish))
        .route("/consumers/{consumer_id}/pull", post(pull))
        .route("/consumers/{consumer_id}/lease", get(lease))
        .layer(middleware::from_fn(log_request))
        .with_state(AppState { queue })
}

/// Why a request was refused: one variant per code on the wire.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Refusal {
    /// `PublisherJoins` named a publisher that has already joined.
    #[error("publisher {0} has already joined")]
    PublisherExists(PublisherId),
    /// `ConsumerJoins` named a consumer that has already joined.
    #[error("consumer {0} has already joined")]
    ConsumerExists(ConsumerId),
    /// A publish named a publisher that has not joined.
    #[error("publisher {0} has not joined")]
    UnknownPublisher(PublisherId),
    /// A pull or lease read named a consumer that has not joined.
    #[error("consumer {0} has not joined")]
    UnknownConsumer(ConsumerId),
    /// A publish carried the stored record's rank: the placeholder for the
    /// specification's first open question, not a decision.
    #[error(
        "rank {rank} equals the stored rank of {record_key} in pocket {pocket_key}; no rule \
        covers an equal rank (open question 1), so this refusal is a placeholder"
    )]
    EqualRank {
        /// The pocket published into.
        pocket_key: PocketKey,
        /// The record key published.
        record_key: RecordKey,
        /// The rank both the stored record and the publish carry.
        rank: Rank,
    },
    /// The request could not be read: a malformed body, path or payload.
    #[error("{0}")]
    BadRequest(String),
    /// The queue's owner has stopped, or the library refused with a variant this map
    /// does not know.
    #[error("{0}")]
    NotReady(String),
}

impl Refusal {
    /// The status the refusal is answered with.
    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::PublisherExists(_) | Self::ConsumerExists(_) | Self::EqualRank { .. } => {
                StatusCode::CONFLICT
            }
            Self::UnknownPublisher(_) | Self::UnknownConsumer(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotReady(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// The refusal's code on the wire.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::PublisherExists(_) => "publisher_exists",
            Self::ConsumerExists(_) => "consumer_exists",
            Self::UnknownPublisher(_) => "unknown_publisher",
            Self::UnknownConsumer(_) => "unknown_consumer",
            Self::EqualRank { .. } => "equal_rank",
            Self::BadRequest(_) => "bad_request",
            Self::NotReady(_) => "not_ready",
        }
    }
}

/// The arm `#[non_exhaustive]` forces on every conversion from a library error:
/// a variant this map does not know answers 503 `not_ready` and logs an error.
fn unmapped(error: &impl std::fmt::Debug) -> Refusal {
    tracing::error!(?error, "a library refusal has no mapping");
    Refusal::NotReady(format!("unmapped library refusal {error:?}"))
}

impl From<JoinError> for Refusal {
    fn from(error: JoinError) -> Self {
        match error {
            JoinError::PublisherExists(id) => Self::PublisherExists(id),
            JoinError::ConsumerExists(id) => Self::ConsumerExists(id),
            other => unmapped(&other),
        }
    }
}

impl Refusal {
    /// The refusal for a publish into `pocket_key`: the library's error does not
    /// carry the pocket, so a publish error is converted only with it.
    pub(crate) fn from_publish(error: PublishError, pocket_key: &PocketKey) -> Self {
        match error {
            PublishError::UnknownPublisher(id) => Self::UnknownPublisher(id),
            PublishError::EqualRank { key, rank } => Self::EqualRank {
                pocket_key: pocket_key.clone(),
                record_key: key,
                rank,
            },
            other => unmapped(&other),
        }
    }
}

impl From<PullError> for Refusal {
    fn from(error: PullError) -> Self {
        match error {
            PullError::UnknownConsumer(id) => Self::UnknownConsumer(id),
            other => unmapped(&other),
        }
    }
}

impl From<Stopped> for Refusal {
    fn from(stopped: Stopped) -> Self {
        Self::NotReady(stopped.to_string())
    }
}

impl From<JsonRejection> for Refusal {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<PathRejection> for Refusal {
    fn from(rejection: PathRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

/// The error body every refusal is answered with.
#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: self.code(),
            message: self.to_string(),
        };
        (self.status(), Json(body)).into_response()
    }
}

/// The body of `PUT /pockets/{pocket_key}/records/{record_key}`.
#[derive(Debug, Deserialize)]
struct PublishBody {
    publisher_id: String,
    rank: i64,
    /// The payload bytes, base64 with the standard alphabet and padding.
    payload: String,
}

/// The body of `POST /consumers/{consumer_id}/pull`.
#[derive(Debug, Deserialize)]
struct PullBody {
    limit: i64,
}

/// A publish's reply: the rule that took it.
#[derive(Debug, Serialize)]
struct PublishReply {
    outcome: &'static str,
}

/// One held record on the wire.
#[derive(Debug, Serialize)]
struct RecordReply {
    record_key: String,
    rank: i64,
    payload: String,
}

impl From<HeldRecord> for RecordReply {
    fn from(record: HeldRecord) -> Self {
        Self {
            record_key: record.key.as_str().to_owned(),
            rank: record.rank.get(),
            payload: STANDARD.encode(record.payload.as_ref()),
        }
    }
}

/// A pull's reply: a batch under a lease, or no pocket.
#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum PullReply {
    /// The records handed out, oldest first, possibly none.
    Batch {
        lease_id: String,
        pocket_key: String,
        expires_at: u64,
        records: Vec<RecordReply>,
    },
    /// No pocket was queued: data, not an error.
    NoPocket,
}

/// The lease read's reply: the lease's pocket, `expires_at` and held records.
#[derive(Debug, Serialize)]
struct LeaseReply {
    pocket_key: String,
    expires_at: u64,
    records: Vec<RecordReply>,
}

impl From<LeaseView> for LeaseReply {
    fn from(view: LeaseView) -> Self {
        Self {
            pocket_key: view.pocket_key.as_str().to_owned(),
            expires_at: view.expires_at.as_millis(),
            records: view.records.into_iter().map(RecordReply::from).collect(),
        }
    }
}

/// The publish outcome's name on the wire.
fn publish_outcome(outcome: PublishOutcome) -> Result<&'static str, Refusal> {
    match outcome {
        PublishOutcome::Created => Ok("created"),
        PublishOutcome::NewKey => Ok("new_key"),
        PublishOutcome::Superseded => Ok("superseded"),
        PublishOutcome::Stale => Ok("stale"),
        other => Err(unmapped(&other)),
    }
}

/// The pull reply for `outcome`, built from the one outcome the owner returned.
fn pull_reply(outcome: PullOutcome) -> Result<PullReply, Refusal> {
    match outcome {
        PullOutcome::Batch {
            lease,
            pocket_key,
            expires_at,
            records,
        } => Ok(PullReply::Batch {
            lease_id: lease.to_string(),
            pocket_key: pocket_key.as_str().to_owned(),
            expires_at: expires_at.as_millis(),
            records: records.into_iter().map(RecordReply::from).collect(),
        }),
        PullOutcome::NoPocketAvailable => Ok(PullReply::NoPocket),
        other => Err(unmapped(&other)),
    }
}

/// `text` decoded as a base64 payload.
fn payload(text: &str) -> Result<Payload, Refusal> {
    STANDARD
        .decode(text)
        .map(Payload::new)
        .map_err(|error| Refusal::BadRequest(format!("payload is not base64: {error}")))
}

/// `GET /healthz`: 200 `ok`.
async fn healthz() -> &'static str {
    "ok"
}

/// `POST /publishers/{publisher_id}`: 201 with no body.
async fn publisher_joins(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, Refusal> {
    let Path(id) = path?;
    state.queue.publisher_joins(PublisherId::new(id)).await??;
    Ok(StatusCode::CREATED)
}

/// `POST /consumers/{consumer_id}`: 201 with no body.
async fn consumer_joins(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, Refusal> {
    let Path(id) = path?;
    state.queue.consumer_joins(ConsumerId::new(id)).await??;
    Ok(StatusCode::CREATED)
}

/// `PUT /pockets/{pocket_key}/records/{record_key}`: 200 with the outcome.
async fn publish(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    body: Result<Json<PublishBody>, JsonRejection>,
) -> Result<Json<PublishReply>, Refusal> {
    let Path((pocket, record)) = path?;
    let Json(body) = body?;
    let pocket_key = PocketKey::new(pocket);
    let publication = Publication::new(
        pocket_key.clone(),
        RecordKey::new(record),
        Rank::new(body.rank),
        payload(&body.payload)?,
    );
    let outcome = state
        .queue
        .publish(PublisherId::new(body.publisher_id), publication)
        .await?
        .map_err(|error| Refusal::from_publish(error, &pocket_key))?;
    Ok(Json(PublishReply {
        outcome: publish_outcome(outcome)?,
    }))
}

/// `POST /consumers/{consumer_id}/pull`: 200 with a batch or `no_pocket`.
async fn pull(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<Json<PullBody>, JsonRejection>,
) -> Result<Json<PullReply>, Refusal> {
    let Path(id) = path?;
    let Json(body) = body?;
    let outcome = state.queue.pull(ConsumerId::new(id), body.limit).await??;
    Ok(Json(pull_reply(outcome)?))
}

/// `GET /consumers/{consumer_id}/lease`: 200 with the lease, or 204 when the
/// consumer holds none.
async fn lease(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, Refusal> {
    let Path(id) = path?;
    let view = state.queue.lease_view(ConsumerId::new(id)).await??;
    Ok(view.map_or_else(
        || StatusCode::NO_CONTENT.into_response(),
        |view| Json(LeaseReply::from(view)).into_response(),
    ))
}

/// One `debug` line per request: method, path and status. At the default `info`
/// level none is written: a per-request `info` line cost PR 39 a third of its
/// publish rate.
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    tracing::debug!(%method, path, status = response.status().as_u16(), "request");
    response
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use libpachinko::error::{JoinError, PublishError, PullError};
    use libpachinko::ids::{ConsumerId, PocketKey, PublisherId, Rank, RecordKey};
    use serde_json::{Value, json};

    use super::Refusal;
    use crate::actor::Stopped;

    /// The status and the parsed body `refusal` is answered with.
    async fn answered(refusal: Refusal) -> (StatusCode, Value) {
        let response = refusal.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    async fn assert_answered(refusal: Refusal, status: StatusCode, code: &str) {
        let message = refusal.to_string();
        assert_eq!(
            answered(refusal).await,
            (status, json!({"error": code, "message": message}))
        );
    }

    #[tokio::test]
    async fn publisher_exists_is_409() {
        let refusal = Refusal::PublisherExists(PublisherId::new("p"));
        assert_answered(refusal, StatusCode::CONFLICT, "publisher_exists").await;
    }

    #[tokio::test]
    async fn consumer_exists_is_409() {
        let refusal = Refusal::ConsumerExists(ConsumerId::new("c"));
        assert_answered(refusal, StatusCode::CONFLICT, "consumer_exists").await;
    }

    #[tokio::test]
    async fn unknown_publisher_is_404() {
        let refusal = Refusal::UnknownPublisher(PublisherId::new("p"));
        assert_answered(refusal, StatusCode::NOT_FOUND, "unknown_publisher").await;
    }

    #[tokio::test]
    async fn unknown_consumer_is_404() {
        let refusal = Refusal::UnknownConsumer(ConsumerId::new("c"));
        assert_answered(refusal, StatusCode::NOT_FOUND, "unknown_consumer").await;
    }

    #[tokio::test]
    async fn equal_rank_is_409_and_says_it_is_a_placeholder() {
        let refusal = Refusal::EqualRank {
            pocket_key: PocketKey::new("pocket"),
            record_key: RecordKey::new("k"),
            rank: Rank::new(3),
        };
        assert!(refusal.to_string().contains("placeholder"));
        assert!(refusal.to_string().contains("pocket pocket"));
        assert_answered(refusal, StatusCode::CONFLICT, "equal_rank").await;
    }

    #[tokio::test]
    async fn bad_request_is_400() {
        let refusal = Refusal::BadRequest(String::from("not JSON"));
        assert_answered(refusal, StatusCode::BAD_REQUEST, "bad_request").await;
    }

    #[tokio::test]
    async fn not_ready_is_503() {
        let refusal = Refusal::from(Stopped::Owner);
        assert_eq!(refusal.to_string(), "the queue's owner has stopped");
        assert_answered(refusal, StatusCode::SERVICE_UNAVAILABLE, "not_ready").await;
    }

    #[test]
    fn every_library_refusal_has_its_own_mapping() {
        let (p, c) = (PublisherId::new("p"), ConsumerId::new("c"));
        let (pocket, key, rank) = (PocketKey::new("q"), RecordKey::new("k"), Rank::new(1));
        let cases = [
            (
                Refusal::from(JoinError::PublisherExists(p.clone())),
                Refusal::PublisherExists(p.clone()),
            ),
            (
                Refusal::from(JoinError::ConsumerExists(c.clone())),
                Refusal::ConsumerExists(c.clone()),
            ),
            (
                Refusal::from_publish(PublishError::UnknownPublisher(p.clone()), &pocket),
                Refusal::UnknownPublisher(p),
            ),
            (
                Refusal::from_publish(
                    PublishError::EqualRank {
                        key: key.clone(),
                        rank,
                    },
                    &pocket,
                ),
                Refusal::EqualRank {
                    pocket_key: pocket.clone(),
                    record_key: key,
                    rank,
                },
            ),
            (
                Refusal::from(PullError::UnknownConsumer(c.clone())),
                Refusal::UnknownConsumer(c),
            ),
        ];
        for (converted, expected) in cases {
            assert_eq!(converted, expected);
            assert_ne!(
                converted.code(),
                "not_ready",
                "{converted:?} hit the forced arm"
            );
        }
    }

    #[test]
    fn the_forced_arm_is_not_ready() {
        let refusal = super::unmapped(&"SomeNewVariant");
        assert_eq!(refusal.code(), "not_ready");
        assert!(refusal.to_string().contains("SomeNewVariant"));
    }
}
