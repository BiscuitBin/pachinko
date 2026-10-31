//! The seven action types `do_action` carries, each a one-row IPC body in and a
//! one-batch IPC reply out.

use arrow_flight::ActionType;
use bytes::Bytes;
use tonic::Status;

use super::codec::{self, CONSUMER_ID, PUBLISHER_ID, RECORD_KEY};
use super::{action_status, consumer_status, join_status, unavailable};
use crate::actor::QueueHandle;

/// `PublisherJoins`: body `publisher_id`.
const PUBLISHER_JOINS: &str = "publisher_joins";
/// `ConsumerJoins`: body `consumer_id`.
const CONSUMER_JOINS: &str = "consumer_joins";
/// `Ack`: body `consumer_id`, `lease_id`, `record_key`.
const ACK: &str = "ack";
/// `Nack`: body `consumer_id`, `lease_id`, `record_key`.
const NACK: &str = "nack";
/// `RenewLease`: body `consumer_id`, `lease_id`.
const RENEW: &str = "renew";
/// `ReleaseLease`: body `consumer_id`, `lease_id`.
const RELEASE: &str = "release";
/// The `Consuming` surface's `exposes`: body `consumer_id`.
const STATUS: &str = "status";

/// Each action type and its one-line description, as `list_actions` gives them.
const ACTIONS: [(&str, &str); 7] = [
    (
        PUBLISHER_JOINS,
        "Join as a publisher. Body: publisher_id Utf8. Reply: outcome Utf8 = joined.",
    ),
    (
        CONSUMER_JOINS,
        "Join as a consumer. Body: consumer_id Utf8. Reply: outcome Utf8 = joined.",
    ),
    (
        ACK,
        "Ack a held record. Body: consumer_id Utf8, lease_id UInt64, record_key Utf8. \
            Reply: outcome Utf8 = settled or deferred.",
    ),
    (
        NACK,
        "Nack a held record to a dead letter. Body: consumer_id Utf8, lease_id UInt64, \
            record_key Utf8. Reply: outcome Utf8 = settled or deferred.",
    ),
    (
        RENEW,
        "Renew a lease. Body: consumer_id Utf8, lease_id UInt64. Reply: expires_at UInt64.",
    ),
    (
        RELEASE,
        "Release a lease. Body: consumer_id Utf8, lease_id UInt64. \
            Reply: lease_id UInt64, ended_at UInt64.",
    ),
    (
        STATUS,
        "Read a consumer's active lease. Body: consumer_id Utf8. Reply: metadata lease = \
            active or none, with lease_id, pocket_key and expires_at; rows record_key, rank, payload.",
    ),
];

/// Every action type, with its description.
pub(super) fn listing() -> Vec<ActionType> {
    ACTIONS
        .iter()
        .map(|(kind, description)| ActionType {
            r#type: (*kind).to_owned(),
            description: (*description).to_owned(),
        })
        .collect()
}

/// Performs the action `kind` with `body` and returns its reply.
pub(super) async fn perform(queue: &QueueHandle, kind: &str, body: &[u8]) -> Result<Bytes, Status> {
    match kind {
        PUBLISHER_JOINS => publisher_joins_action(queue, body).await,
        CONSUMER_JOINS => consumer_joins_action(queue, body).await,
        ACK => ack_action(queue, body).await,
        NACK => nack_action(queue, body).await,
        RENEW => renew_action(queue, body).await,
        RELEASE => release_action(queue, body).await,
        STATUS => status_action(queue, body).await,
        other => Err(Status::invalid_argument(format!(
            "unknown action type {other}"
        ))),
    }
}

/// `publisher_joins`.
async fn publisher_joins_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let publisher_id = codec::utf8_value(&request, PUBLISHER_ID)?;
    queue
        .join_publisher(publisher_id)
        .await
        .map_err(unavailable)?
        .map_err(|error| join_status(&error))?;
    codec::joined_reply()
}

/// `consumer_joins`.
async fn consumer_joins_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    queue
        .join_consumer(consumer_id)
        .await
        .map_err(unavailable)?
        .map_err(|error| join_status(&error))?;
    codec::joined_reply()
}

/// `ack`.
async fn ack_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    let lease = codec::lease_value(&request)?;
    let record_key = codec::utf8_value(&request, RECORD_KEY)?;
    let outcome = queue
        .ack_record(consumer_id, lease, record_key)
        .await
        .map_err(unavailable)?
        .map_err(|error| action_status(&error))?;
    codec::settle_reply(outcome)
}

/// `nack`.
async fn nack_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    let lease = codec::lease_value(&request)?;
    let record_key = codec::utf8_value(&request, RECORD_KEY)?;
    let outcome = queue
        .nack_record(consumer_id, lease, record_key)
        .await
        .map_err(unavailable)?
        .map_err(|error| action_status(&error))?;
    codec::settle_reply(outcome)
}

/// `renew`.
async fn renew_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    let lease = codec::lease_value(&request)?;
    let expires_at = queue
        .renew_lease(consumer_id, lease)
        .await
        .map_err(unavailable)?
        .map_err(|error| action_status(&error))?;
    codec::renew_reply(expires_at)
}

/// `release`.
async fn release_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    let lease = codec::lease_value(&request)?;
    let ended = queue
        .release_lease(consumer_id, lease)
        .await
        .map_err(unavailable)?
        .map_err(|error| action_status(&error))?;
    codec::release_reply(&ended)
}

/// `status`.
async fn status_action(queue: &QueueHandle, body: &[u8]) -> Result<Bytes, Status> {
    let request = codec::read_one_row(body)?;
    let consumer_id = codec::utf8_value(&request, CONSUMER_ID)?;
    let view = queue
        .lease_status(consumer_id)
        .await
        .map_err(unavailable)?
        .map_err(|error| consumer_status(&error))?;
    codec::status_reply(view)
}

#[cfg(test)]
mod tests {
    use super::{ACTIONS, listing, perform};
    use crate::actor::QueueHandle;
    use tonic::Code;

    #[test]
    fn the_listing_names_the_seven_actions_in_order() {
        let kinds: Vec<String> = listing().into_iter().map(|action| action.r#type).collect();
        assert_eq!(
            kinds,
            [
                "publisher_joins",
                "consumer_joins",
                "ack",
                "nack",
                "renew",
                "release",
                "status"
            ]
        );
        assert_eq!(ACTIONS.len(), 7);
    }

    #[tokio::test]
    async fn an_unknown_action_type_is_an_invalid_argument() {
        let status = perform(&QueueHandle::stopped(), "drop_table", b"")
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
        assert_eq!(status.message(), "unknown action type drop_table");
    }

    #[tokio::test]
    async fn a_malformed_body_is_an_invalid_argument() {
        for kind in ["publisher_joins", "ack", "status"] {
            let status = perform(&QueueHandle::stopped(), kind, b"not ipc")
                .await
                .unwrap_err();
            assert_eq!(status.code(), Code::InvalidArgument, "{kind}");
        }
    }
}
