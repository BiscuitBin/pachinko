//! The lease read, `GET /consumers/{consumer_id}/lease`: the consumer's active
//! lease with its held records, each carrying its mark.

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use libpachinko::{ConsumerId, LeaseView, Mark, MarkedRecord};
use serde::Serialize;

use super::{AppState, Refusal, unmapped};

/// One held record on the lease read: a batch's record with its mark.
#[derive(Debug, Serialize)]
struct LeaseRecordReply {
    record_key: String,
    rank: i64,
    payload: String,
    mark: &'static str,
}

/// The mark's name on the wire.
fn mark_name(mark: Mark) -> Result<&'static str, Refusal> {
    match mark {
        Mark::Unsettled => Ok("unsettled"),
        Mark::Acked => Ok("acked"),
        Mark::Nacked => Ok("nacked"),
        other => Err(unmapped(&other)),
    }
}

impl TryFrom<MarkedRecord> for LeaseRecordReply {
    type Error = Refusal;

    fn try_from(record: MarkedRecord) -> Result<Self, Refusal> {
        Ok(Self {
            record_key: record.key.as_str().to_owned(),
            rank: record.rank.get(),
            payload: STANDARD.encode(record.payload.as_ref()),
            mark: mark_name(record.mark)?,
        })
    }
}

/// The lease read's reply: the lease's pocket, `expires_at` and held records.
#[derive(Debug, Serialize)]
struct LeaseReply {
    pocket_key: String,
    expires_at: u64,
    records: Vec<LeaseRecordReply>,
}

impl TryFrom<LeaseView> for LeaseReply {
    type Error = Refusal;

    fn try_from(view: LeaseView) -> Result<Self, Refusal> {
        Ok(Self {
            pocket_key: view.pocket_key.as_str().to_owned(),
            expires_at: view.expires_at.as_millis(),
            records: view
                .records
                .into_iter()
                .map(LeaseRecordReply::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// `GET /consumers/{consumer_id}/lease`: 200 with the lease, or 204 when the
/// consumer holds none.
pub(super) async fn lease(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, Refusal> {
    let Path(id) = path?;
    let view = state.queue.lease_view(ConsumerId::new(id)).await??;
    match view {
        Some(view) => Ok(Json(LeaseReply::try_from(view)?).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

#[cfg(test)]
mod tests {
    use libpachinko::Mark;

    #[test]
    fn every_mark_has_its_name() {
        let names = [Mark::Unsettled, Mark::Acked, Mark::Nacked].map(super::mark_name);
        assert_eq!(names, [Ok("unsettled"), Ok("acked"), Ok("nacked")]);
    }
}
