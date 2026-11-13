//! The batch publish route (T27): `POST /publishers/{publisher_id}/publish`, its
//! bodies and its whole-request checks. Every check runs over every row before the
//! rows reach the owner, which applies them in order as one command.

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use libpachinko::{PocketKey, Publication, PublisherId, Rank, RecordKey};
use serde::{Deserialize, Serialize};

use super::{AppState, BoundedJson, Limits, Refusal, payload, publish_outcome, within};
use crate::actor::RowOutcome;

/// The body of `POST /publishers/{publisher_id}/publish`.
#[derive(Debug, Deserialize)]
pub(super) struct BatchBody {
    records: Vec<BatchRow>,
}

/// One row of a batch publish.
#[derive(Debug, Deserialize)]
struct BatchRow {
    pocket_key: String,
    record_key: String,
    rank: i64,
    /// The payload bytes, base64 with the standard alphabet and padding.
    payload: String,
}

/// A batch publish's reply: one outcome per row, in row order.
#[derive(Debug, Serialize)]
pub(super) struct BatchReply {
    outcomes: Vec<&'static str>,
}

/// A batch row's outcome on the wire.
fn row_outcome(outcome: RowOutcome) -> Result<&'static str, Refusal> {
    match outcome {
        RowOutcome::Applied(outcome) => publish_outcome(outcome),
        RowOutcome::EqualRank => Ok("equal_rank"),
    }
}

/// The rows of a batch publish, checked in the order `http-api.md` gives, every
/// check over every row before any row reaches the owner: at least one row, at most
/// `--max-batch-rows`, every payload base64, then every payload within
/// `--max-payload-bytes`.
fn batch_rows(records: Vec<BatchRow>, limits: Limits) -> Result<Vec<Publication>, Refusal> {
    if records.is_empty() {
        return Err(Refusal::BadRequest(String::from(
            "records is empty: a batch carries at least one row",
        )));
    }
    if records.len() > limits.batch_rows {
        return Err(Refusal::TooManyRows {
            rows: records.len(),
            max: limits.batch_rows,
        });
    }
    let payloads = records
        .iter()
        .zip(1..)
        .map(|(record, row)| {
            payload(&record.payload)
                .map_err(|refusal| Refusal::BadRequest(format!("row {row}: {refusal}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    records
        .into_iter()
        .zip(payloads)
        .zip(1..)
        .map(|((record, payload), row)| {
            Ok(Publication::new(
                PocketKey::new(record.pocket_key),
                RecordKey::new(record.record_key),
                Rank::new(record.rank),
                within(row, payload, limits.payload_bytes)?,
            ))
        })
        .collect()
}

/// `POST /publishers/{publisher_id}/publish`: 200 with one outcome per row, in row
/// order. Every whole-request refusal is decided before the rows reach the owner,
/// which applies them as one command.
pub(super) async fn publish_batch(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    BoundedJson(body): BoundedJson<BatchBody>,
) -> Result<Json<BatchReply>, Refusal> {
    let Path(id) = path?;
    let rows = batch_rows(body.records, state.limits)?;
    // `batch_rows` refuses an empty batch, so the first row exists.
    let first_pocket = rows[0].pocket_key.clone();
    let outcomes = state
        .queue
        .publish_batch(PublisherId::new(id), rows)
        .await?
        .map_err(|error| Refusal::from_publish(error, &first_pocket))?;
    Ok(Json(BatchReply {
        outcomes: outcomes
            .into_iter()
            .map(row_outcome)
            .collect::<Result<_, _>>()?,
    }))
}
