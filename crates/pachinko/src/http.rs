//! The HTTP API: one route per action, JSON bodies with base64 payloads, and one
//! exhaustive map from a refusal to its status and code. `docs/reference/http-api.md`
//! owns the route table.
//!
//! Every handler reads its request, sends one command through the
//! [`QueueHandle`] and turns the answer into a reply. A handler holds nothing the
//! owner needs across an `.await`: the handle is a channel sender.
//!
//! Every route but `/healthz` and `/readyz` runs under [`bounded`], which answers
//! 408 `request_timeout` once its handler has run for `--request-timeout-ms`.

use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::rejection::{BytesRejection, FailedToBufferBody, JsonRejection, PathRejection};
use axum::extract::{DefaultBodyLimit, FromRequest, Path, Request, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_LENGTH;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use libpachinko::error::{JoinError, LeaseActionError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
use libpachinko::lease_ids::LeaseId;
use libpachinko::queue::{LeaseView, Publication, PublishOutcome, PullOutcome};
use libpachinko::records::HeldRecord;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::ServeConfig;
use crate::actor::{QueueHandle, Readiness, Stopped};

mod batch;
mod leases;

/// What every handler shares: the handle to the queue's owner, the readiness
/// `GET /readyz` reports and the request-shape limits.
#[derive(Clone, Debug)]
struct AppState {
    queue: QueueHandle,
    readiness: watch::Receiver<Readiness>,
    limits: Limits,
}
/// The request-shape limits (T27): a request over one is refused for its shape
/// before any row applies, never for want of room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    /// `--max-batch-rows`: the most rows one batch publish carries.
    batch_rows: usize,
    /// `--max-payload-bytes`: the largest decoded payload of one row or one publish.
    payload_bytes: usize,
    /// `--max-body-bytes`: the largest request body any route reads.
    body_bytes: usize,
}

impl From<&ServeConfig> for Limits {
    fn from(config: &ServeConfig) -> Self {
        Self {
            batch_rows: config.max_batch_rows(),
            payload_bytes: config.max_payload_bytes(),
            body_bytes: config.max_body_bytes(),
        }
    }
}

/// The router over `queue` under `limits`, each request logged at `debug`. Every
/// route but `/healthz` and `/readyz` is bounded at `request_timeout`. The body limit
/// replaces axum's default of 2 MiB on every route.
pub(crate) fn router(
    queue: QueueHandle,
    readiness: watch::Receiver<Readiness>,
    request_timeout: Duration,
    limits: Limits,
) -> Router {
    bounded(api(), request_timeout)
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(DefaultBodyLimit::max(limits.body_bytes))
        .layer(middleware::from_fn(log_request))
        .with_state(AppState {
            queue,
            readiness,
            limits,
        })
}

/// The API's routes, each one action. Every route added here runs under the request
/// timeout; `router` adds the unbounded probes after the bound is applied.
fn api() -> Router<AppState> {
    Router::new()
        .route("/publishers/{publisher_id}", post(publisher_joins))
        .route("/consumers/{consumer_id}", post(consumer_joins))
        .route("/pockets/{pocket_key}/records/{record_key}", put(publish))
        .route(
            "/publishers/{publisher_id}/publish",
            post(batch::publish_batch),
        )
        .route("/consumers/{consumer_id}/pull", post(pull))
        .route("/consumers/{consumer_id}/lease", get(lease))
        .route("/leases/{lease_id}/ack", post(leases::ack))
        .route("/leases/{lease_id}/nack", post(leases::nack))
        .route("/leases/{lease_id}/renew", post(leases::renew))
        .route("/leases/{lease_id}/release", post(leases::release))
}

/// `routes` with each handler bounded at `bound`: one that runs longer is dropped
/// and answered 408 `request_timeout`. A command it already sent to the owner is
/// not withdrawn; the owner still applies it.
fn bounded<S>(routes: Router<S>, bound: Duration) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    routes.layer(middleware::from_fn(move |request: Request, next: Next| {
        run_bounded(bound, request, next)
    }))
}

/// Runs `next` on `request` for at most `bound`.
async fn run_bounded(bound: Duration, request: Request, next: Next) -> Response {
    tokio::time::timeout(bound, next.run(request))
        .await
        .unwrap_or_else(|_| Refusal::RequestTimeout(bound).into_response())
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
    /// A lease action named a lease that has ended or never existed.
    #[error("lease {0} is not active")]
    LeaseNotActive(LeaseId),
    /// A lease action named another consumer's active lease.
    #[error("lease {0} is not this consumer's")]
    LeaseNotYours(LeaseId),
    /// An ack or nack named a record the lease's pocket does not hold.
    #[error("record {0} is not held under this lease")]
    RecordNotHeld(RecordKey),
    /// The request could not be read: a malformed body, path or payload.
    #[error("{0}")]
    BadRequest(String),
    /// The service is starting or stopping, the queue's owner has stopped, or the
    /// library refused with a variant this map does not know.
    #[error("{0}")]
    NotReady(String),
    /// A batch publish carried more rows than `--max-batch-rows`.
    #[error("the batch has {rows} rows, over the limit of {max} (--max-batch-rows)")]
    TooManyRows {
        /// The rows the batch carried.
        rows: usize,
        /// `--max-batch-rows`.
        max: usize,
    },
    /// A payload decoded to more bytes than `--max-payload-bytes`. The single `PUT`
    /// is row 1.
    #[error(
        "the payload of row {row} is {bytes} bytes, over the limit of {max} \
        (--max-payload-bytes)"
    )]
    PayloadTooLarge {
        /// The row, counted from 1.
        row: usize,
        /// The decoded payload's length.
        bytes: usize,
        /// `--max-payload-bytes`.
        max: usize,
    },
    /// The request body was longer than `--max-body-bytes`, by its `Content-Length`
    /// or as read.
    #[error("the request body is over the limit of {max} bytes (--max-body-bytes)")]
    BodyTooLarge {
        /// `--max-body-bytes`.
        max: usize,
    },
    /// The handler ran past `--request-timeout-ms`; a command it sent may still be
    /// applied.
    #[error(
        "the request took longer than {}ms; a command it sent may still be applied",
        .0.as_millis()
    )]
    RequestTimeout(Duration),
}

impl Refusal {
    /// The status the refusal is answered with.
    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::PublisherExists(_)
            | Self::ConsumerExists(_)
            | Self::EqualRank { .. }
            | Self::LeaseNotActive(_) => StatusCode::CONFLICT,
            Self::UnknownPublisher(_) | Self::UnknownConsumer(_) | Self::RecordNotHeld(_) => {
                StatusCode::NOT_FOUND
            }
            Self::LeaseNotYours(_) => StatusCode::FORBIDDEN,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotReady(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::TooManyRows { .. } | Self::PayloadTooLarge { .. } | Self::BodyTooLarge { .. } => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Self::RequestTimeout(_) => StatusCode::REQUEST_TIMEOUT,
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
            Self::LeaseNotActive(_) => "lease_not_active",
            Self::LeaseNotYours(_) => "lease_not_yours",
            Self::RecordNotHeld(_) => "record_not_held",
            Self::BadRequest(_) => "bad_request",
            Self::NotReady(_) => "not_ready",
            Self::TooManyRows { .. } => "too_many_rows",
            Self::PayloadTooLarge { .. } => "payload_too_large",
            Self::BodyTooLarge { .. } => "body_too_large",
            Self::RequestTimeout(_) => "request_timeout",
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

impl From<LeaseActionError> for Refusal {
    fn from(error: LeaseActionError) -> Self {
        match error {
            LeaseActionError::UnknownConsumer(id) => Self::UnknownConsumer(id),
            LeaseActionError::LeaseNotActive(id) => Self::LeaseNotActive(id),
            LeaseActionError::LeaseNotYours(id) => Self::LeaseNotYours(id),
            LeaseActionError::RecordNotHeld(key) => Self::RecordNotHeld(key),
            other => unmapped(&other),
        }
    }
}

impl From<Stopped> for Refusal {
    fn from(stopped: Stopped) -> Self {
        Self::NotReady(stopped.to_string())
    }
}

impl Refusal {
    /// The refusal for a body the `Json` extractor rejected: over `max` bytes as
    /// read, told apart by the rejection's type, is 413 `body_too_large`; any other
    /// rejection is 400 `bad_request`.
    fn from_json(rejection: JsonRejection, max: usize) -> Self {
        match rejection {
            JsonRejection::BytesRejection(BytesRejection::FailedToBufferBody(
                FailedToBufferBody::LengthLimitError(_),
            )) => Self::BodyTooLarge { max },
            other => Self::BadRequest(other.body_text()),
        }
    }
}

impl From<PathRejection> for Refusal {
    fn from(rejection: PathRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

/// A JSON body no longer than `--max-body-bytes`. A `Content-Length` over the limit
/// is refused before the body is read; a body without one is read up to the limit.
#[derive(Debug)]
struct BoundedJson<T>(T);

impl<T> FromRequest<AppState> for BoundedJson<T>
where
    T: DeserializeOwned + Send,
{
    type Rejection = Refusal;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, Refusal> {
        let max = state.limits.body_bytes;
        let declared = request
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|length| length.to_str().ok()?.parse::<u64>().ok());
        if declared.is_some_and(|length| length > u64::try_from(max).unwrap_or(u64::MAX)) {
            return Err(Refusal::BodyTooLarge { max });
        }
        // The body is read before its content type is checked, so a body over the
        // limit is 413 whatever it claims to be. The content type is then axum's own
        // decision, made on the request rebuilt around the bytes already read.
        let (parts, body) = request.into_parts();
        let mut body_only = Request::new(body);
        *body_only.extensions_mut() = parts.extensions.clone();
        let bytes = Bytes::from_request(body_only, state)
            .await
            .map_err(|rejection| Refusal::from_json(JsonRejection::from(rejection), max))?;
        Json::<T>::from_request(Request::from_parts(parts, Body::from(bytes)), state)
            .await
            .map(|Json(body)| Self(body))
            .map_err(|rejection| Refusal::from_json(rejection, max))
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

/// `payload`, unless it is longer than `max` bytes: row `row`'s
/// `payload_too_large`.
fn within(row: usize, payload: Payload, max: usize) -> Result<Payload, Refusal> {
    let bytes = payload.as_ref().len();
    if bytes > max {
        return Err(Refusal::PayloadTooLarge { row, bytes, max });
    }
    Ok(payload)
}

/// `GET /healthz`: 200 `ok`.
async fn healthz() -> &'static str {
    "ok"
}

/// `GET /readyz`: 204 with no body while the owner is serving, 503 `not_ready`
/// naming the state before it has begun and from the moment shutdown starts.
async fn readyz(State(state): State<AppState>) -> Result<StatusCode, Refusal> {
    let current = *state.readiness.borrow();
    match current {
        Readiness::Serving => Ok(StatusCode::NO_CONTENT),
        other => Err(Refusal::NotReady(other.to_string())),
    }
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
    BoundedJson(body): BoundedJson<PublishBody>,
) -> Result<Json<PublishReply>, Refusal> {
    let Path((pocket, record)) = path?;
    let pocket_key = PocketKey::new(pocket);
    let publication = Publication::new(
        pocket_key.clone(),
        RecordKey::new(record),
        Rank::new(body.rank),
        within(1, payload(&body.payload)?, state.limits.payload_bytes)?,
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
    BoundedJson(body): BoundedJson<PullBody>,
) -> Result<Json<PullReply>, Refusal> {
    let Path(id) = path?;
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

/// One `debug` line per request, after the reply: method, path, status and elapsed
/// milliseconds. At the default `info` level none is written: a per-request `info`
/// line cost PR 39 a third of its publish rate.
async fn log_request(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    tracing::debug!(
        %method,
        path,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "request"
    );
    response
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::Router;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use libpachinko::config::Config;
    use libpachinko::error::{JoinError, LeaseActionError, PublishError, PullError};
    use libpachinko::ids::{ConsumerId, PocketKey, PublisherId, Rank, RecordKey};
    use libpachinko::lease_ids::LeaseId;
    use libpachinko::queue::Queue;
    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tokio::sync::watch;

    use axum::Json;
    use axum::body::Body;
    use axum::extract::rejection::JsonRejection;
    use axum::extract::{FromRequest, Request};
    use axum::http::header::CONTENT_TYPE;

    use super::{Limits, Refusal, bounded, router};
    use crate::actor::{self, Readiness, Stopped};
    use crate::{EpochSerialLeaseIds, ServeConfig, SystemClock};

    /// How long one request may take before the test fails.
    const BOUND: Duration = Duration::from_secs(5);

    /// Serves `routes` on port 0 while `drive` runs against its base URL with a
    /// blocking client whose every request is bounded, then stops serving.
    async fn serve_while<T: Send>(routes: Router, drive: impl FnOnce(&Probe) -> T + Send) -> T {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, routes).await });
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(BOUND))
            .build()
            .into();
        let probe = Probe { agent, base };
        let driven = tokio::task::block_in_place(|| drive(&probe));
        server.abort();
        driven
    }

    /// A blocking client against one test server.
    struct Probe {
        agent: ureq::Agent,
        base: String,
    }

    impl Probe {
        /// The status and body text of `method` on `path`.
        fn send(&self, method: &str, path: &str) -> (u16, String) {
            let url = format!("{}{path}", self.base);
            let response = match method {
                "GET" => self.agent.get(&url).call(),
                "POST" => self.agent.post(&url).send_empty(),
                other => panic!("no such request in this test: {other}"),
            };
            Self::read(response)
        }

        /// The status and body text of `method` on `path` with the JSON `body`.
        fn send_json(&self, method: &str, path: &str, body: &str) -> (u16, String) {
            let url = format!("{}{path}", self.base);
            let response = match method {
                "POST" => self.agent.post(&url),
                "PUT" => self.agent.put(&url),
                other => panic!("no such request in this test: {other}"),
            }
            .header("content-type", "application/json")
            .send(body);
            Self::read(response)
        }

        /// The status and body text of `response`.
        fn read(response: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> (u16, String) {
            let mut response = response.unwrap();
            let status = response.status().as_u16();
            (status, response.body_mut().read_to_string().unwrap())
        }
    }

    /// The real router over an owner that is never run, so a command is sent and
    /// never answered, with the readiness `readiness` holds; and that owner, which
    /// the caller keeps so that its inbox stays open.
    fn unanswered_router(
        readiness: &watch::Sender<Readiness>,
        bound: Duration,
    ) -> (Router, impl Future + use<>) {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let queue = Queue::new(
            Config::default(),
            SystemClock::new().unwrap(),
            lease_ids.clone(),
        )
        .unwrap();
        let (_stop, stopping) = watch::channel(false);
        let ticks = actor::tick_lane().1;
        let (handle, owner) = actor::start(queue, lease_ids, readiness.clone(), stopping, ticks);
        let limits = Limits::from(&ServeConfig::parse(Vec::<&str>::new()).unwrap());
        (router(handle, readiness.subscribe(), bound, limits), owner)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn readyz_is_503_until_serving_204_while_serving_and_503_once_stopping() {
        let (readiness, _) = watch::channel(Readiness::Starting);
        let (routes, _owner) = unanswered_router(&readiness, BOUND);
        let not_ready = |state: &str| {
            (
                503,
                json!({"error": "not_ready", "message": state}).to_string(),
            )
        };
        let seen = serve_while(routes, |probe| {
            let starting = probe.send("GET", "/readyz");
            readiness.send_replace(Readiness::Serving);
            let serving = probe.send("GET", "/readyz");
            readiness.send_replace(Readiness::Stopping);
            let stopping = probe.send("GET", "/readyz");
            [starting, serving, stopping]
        })
        .await;
        assert_eq!(
            seen,
            [
                not_ready("starting"),
                (204, String::new()),
                not_ready("stopping")
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_handler_that_never_completes_answers_408_with_the_body() {
        let routes = bounded(
            Router::new().route("/never", get(std::future::pending::<()>)),
            Duration::from_millis(50),
        );
        let (status, body) = serve_while(routes, |probe| probe.send("GET", "/never")).await;
        assert_eq!(status, 408);
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            body,
            json!({
                "error": "request_timeout",
                "message": "the request took longer than 50ms; a command it sent may still be applied"
            })
        );
    }

    /// One batch row, the bodies of the lease actions, and a lease action's path.
    const BATCH: &str =
        r#"{"records": [{"pocket_key": "a", "record_key": "k", "rank": 1, "payload": ""}]}"#;
    const SETTLE: &str = r#"{"consumer_id": "c", "record_key": "k"}"#;
    const CONSUMER: &str = r#"{"consumer_id": "c"}"#;
    fn lease(action: &str) -> String {
        format!("/leases/00000000000000000000000000000001/{action}")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_api_routes_are_bounded_and_the_probes_answer_beside_them() {
        let (readiness, _) = watch::channel(Readiness::Serving);
        let (routes, _owner) = unanswered_router(&readiness, Duration::from_millis(50));
        let seen = serve_while(routes, |probe| {
            // One line per route in `api()`: a route added there gets a line here.
            [
                probe.send("POST", "/publishers/p").0,
                probe.send("POST", "/consumers/c").0,
                probe
                    .send_json(
                        "PUT",
                        "/pockets/a/records/k",
                        r#"{"publisher_id": "p", "rank": 1, "payload": ""}"#,
                    )
                    .0,
                probe
                    .send_json("POST", "/consumers/c/pull", r#"{"limit": 1}"#)
                    .0,
                probe.send("GET", "/consumers/c/lease").0,
                probe.send_json("POST", "/publishers/p/publish", BATCH).0,
                probe.send_json("POST", &lease("ack"), SETTLE).0,
                probe.send_json("POST", &lease("nack"), SETTLE).0,
                probe.send_json("POST", &lease("renew"), CONSUMER).0,
                probe.send_json("POST", &lease("release"), CONSUMER).0,
                probe.send("GET", "/healthz").0,
                probe.send("GET", "/readyz").0,
            ]
        })
        .await;
        let mut bounded = [408; 12];
        bounded[10] = 200;
        bounded[11] = 204;
        assert_eq!(seen, bounded);
    }

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
    async fn lease_not_active_is_409() {
        let refusal = Refusal::LeaseNotActive(LeaseId::from_u128(1));
        assert_answered(refusal, StatusCode::CONFLICT, "lease_not_active").await;
    }

    #[tokio::test]
    async fn lease_not_yours_is_403() {
        let refusal = Refusal::LeaseNotYours(LeaseId::from_u128(1));
        assert_answered(refusal, StatusCode::FORBIDDEN, "lease_not_yours").await;
    }

    #[tokio::test]
    async fn record_not_held_is_404() {
        let refusal = Refusal::RecordNotHeld(RecordKey::new("k"));
        assert_answered(refusal, StatusCode::NOT_FOUND, "record_not_held").await;
    }

    #[tokio::test]
    async fn bad_request_is_400() {
        let refusal = Refusal::BadRequest(String::from("not JSON"));
        assert_answered(refusal, StatusCode::BAD_REQUEST, "bad_request").await;
    }

    #[tokio::test]
    async fn too_many_rows_is_413() {
        let refusal = Refusal::TooManyRows { rows: 5, max: 4 };
        assert_eq!(
            refusal.to_string(),
            "the batch has 5 rows, over the limit of 4 (--max-batch-rows)"
        );
        assert_answered(refusal, StatusCode::PAYLOAD_TOO_LARGE, "too_many_rows").await;
    }

    #[tokio::test]
    async fn payload_too_large_is_413_and_names_the_row() {
        let refusal = Refusal::PayloadTooLarge {
            row: 2,
            bytes: 17,
            max: 16,
        };
        assert_eq!(
            refusal.to_string(),
            "the payload of row 2 is 17 bytes, over the limit of 16 (--max-payload-bytes)"
        );
        assert_answered(refusal, StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large").await;
    }

    #[tokio::test]
    async fn body_too_large_is_413() {
        let refusal = Refusal::BodyTooLarge { max: 128 };
        assert_eq!(
            refusal.to_string(),
            "the request body is over the limit of 128 bytes (--max-body-bytes)"
        );
        assert_answered(refusal, StatusCode::PAYLOAD_TOO_LARGE, "body_too_large").await;
    }

    /// The `Json` extractor's rejection of `body`, under axum's default limit of
    /// 2 MiB.
    async fn rejection(body: Vec<u8>) -> JsonRejection {
        let request = Request::builder()
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        Json::<Value>::from_request(request, &()).await.unwrap_err()
    }

    #[tokio::test]
    async fn the_length_limit_rejection_is_body_too_large_and_a_parse_one_bad_request() {
        let too_long = rejection(vec![b' '; 2 * 1024 * 1024 + 1]).await;
        assert_eq!(
            Refusal::from_json(too_long, 7),
            Refusal::BodyTooLarge { max: 7 }
        );
        let malformed = rejection(b"{".to_vec()).await;
        assert_eq!(Refusal::from_json(malformed, 7).code(), "bad_request");
    }

    #[tokio::test]
    async fn not_ready_is_503() {
        let refusal = Refusal::from(Stopped::Owner);
        assert_eq!(refusal.to_string(), "the queue's owner has stopped");
        assert_answered(refusal, StatusCode::SERVICE_UNAVAILABLE, "not_ready").await;
    }

    #[tokio::test]
    async fn request_timeout_is_408() {
        let refusal = Refusal::RequestTimeout(Duration::from_secs(30));
        assert_eq!(
            refusal.to_string(),
            "the request took longer than 30000ms; a command it sent may still be applied"
        );
        assert_answered(refusal, StatusCode::REQUEST_TIMEOUT, "request_timeout").await;
    }

    #[tokio::test]
    async fn a_state_other_than_serving_is_503_not_ready_naming_it() {
        let refusal = Refusal::NotReady(Readiness::Starting.to_string());
        assert_answered(refusal, StatusCode::SERVICE_UNAVAILABLE, "not_ready").await;
    }

    #[test]
    fn every_library_refusal_has_its_own_mapping() {
        let (p, c) = (PublisherId::new("p"), ConsumerId::new("c"));
        let (pocket, key, rank) = (PocketKey::new("q"), RecordKey::new("k"), Rank::new(1));
        let lease = LeaseId::from_u128(7);
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
                Refusal::UnknownConsumer(c.clone()),
            ),
            (
                Refusal::from(LeaseActionError::UnknownConsumer(c.clone())),
                Refusal::UnknownConsumer(c),
            ),
            (
                Refusal::from(LeaseActionError::LeaseNotActive(lease)),
                Refusal::LeaseNotActive(lease),
            ),
            (
                Refusal::from(LeaseActionError::LeaseNotYours(lease)),
                Refusal::LeaseNotYours(lease),
            ),
            (
                Refusal::from(LeaseActionError::RecordNotHeld(RecordKey::new("h"))),
                Refusal::RecordNotHeld(RecordKey::new("h")),
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

    /// Sends `request` to `address` and reads the reply until the server closes.
    fn exchange(address: std::net::SocketAddr, request: &[u8]) -> String {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request).unwrap();
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        reply
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_streamed_body_over_the_cap_is_413_before_its_content_type_is_checked() {
        let config = ServeConfig::parse([
            "--listen",
            "127.0.0.1:0",
            "--max-payload-bytes",
            "16",
            "--max-body-bytes",
            "128",
        ])
        .unwrap();
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = watch::channel(());
        let server = tokio::spawn(async move { crate::serve(listener, &config, stopped).await });
        // Chunked, so no `Content-Length`, and no content type: 200 bytes over a
        // 128-byte cap.
        let mut request = b"POST /consumers/c/pull HTTP/1.1\r\nhost: test\r\n\
transfer-encoding: chunked\r\nconnection: close\r\n\r\nc8\r\n"
            .to_vec();
        request.extend([b'x'; 200]);
        request.extend(b"\r\n0\r\n\r\n");
        let reply = tokio::task::block_in_place(|| exchange(address, &request));
        drop(stop);
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("serve did not stop in time")
            .unwrap()
            .unwrap();
        assert!(reply.starts_with("HTTP/1.1 413"), "{reply}");
        assert!(reply.contains(r#""error":"body_too_large""#), "{reply}");
    }

    #[test]
    fn the_forced_arm_is_not_ready() {
        let refusal = super::unmapped(&"SomeNewVariant");
        assert_eq!(refusal.code(), "not_ready");
        assert!(refusal.to_string().contains("SomeNewVariant"));
    }
}
