//! Joining, and `do_put` as `Publish`: each row's outcome read from its batch's
//! `PutResult`.

use arrow_array::{ArrayRef, RecordBatch, StringArray};
use arrow_flight::FlightDescriptor;
use arrow_flight::encode::FlightDataEncoderBuilder;
use pachinko::ServeOptions;
use std::sync::Arc;
use tonic::Code;

use crate::fixture::{config, with_server};

#[tokio::test]
async fn both_parties_join_and_a_second_join_is_a_failed_precondition() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        assert_eq!(client.join_publisher("p").await.unwrap(), "joined");
        assert_eq!(client.join_consumer("c").await.unwrap(), "joined");
        let again = client.join_publisher("p").await.unwrap_err();
        assert_eq!(again.code(), Code::FailedPrecondition);
        assert_eq!(again.message(), "p has already joined");
        let again = client.join_consumer("c").await.unwrap_err();
        assert_eq!(again.code(), Code::FailedPrecondition);
        assert_eq!(again.message(), "c has already joined");
    })
    .await;
}

#[tokio::test]
async fn each_publish_rule_answers_its_row() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_publisher("p").await.unwrap();
        let rows = [
            ("orders", "a", 1, &b"a1"[..]),
            ("orders", "b", 1, b"b1"),
            ("orders", "a", 2, b"a2"),
            ("orders", "a", 1, b"late"),
            ("orders", "a", 2, b"same"),
        ];
        let outcomes = client.publish("p", &rows).await.unwrap();
        // A new pocket, a new key, a supersede, a stale rank, an equal rank.
        assert_eq!(outcomes, ["new", "new", "superseded", "stale", "rejected"]);
    })
    .await;
}

#[tokio::test]
async fn each_input_batch_is_answered_by_its_own_put_result() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_publisher("p").await.unwrap();
        let first = [("orders", "a", 1, &b"a1"[..]), ("orders", "b", 1, b"b1")];
        let second = [("orders", "a", 3, &b"a3"[..])];
        let outcomes = client
            .publish_batches("p", &[&first, &second])
            .await
            .unwrap();
        assert_eq!(outcomes, [vec!["new", "new"], vec!["superseded"]]);
    })
    .await;
}

#[tokio::test]
async fn an_unknown_publisher_is_not_found_and_publishes_nothing() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_consumer("c").await.unwrap();
        let status = client
            .publish("nobody", &[("orders", "a", 1, b"a1")])
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no publisher nobody has joined");
        assert_eq!(client.pull("c", 10).await.unwrap().outcome, "no_pocket");
    })
    .await;
}

#[tokio::test]
async fn a_malformed_batch_or_descriptor_is_an_invalid_argument() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_publisher("p").await.unwrap();
        client.join_consumer("c").await.unwrap();
        let keys: ArrayRef = Arc::new(StringArray::from(vec!["a"]));
        let missing_columns = RecordBatch::try_from_iter([("record_key", keys)]).unwrap();
        let data = FlightDataEncoderBuilder::new()
            .with_flight_descriptor(Some(FlightDescriptor::new_cmd("p")))
            .build(futures::stream::iter([Ok(missing_columns)]));
        let status = client.put(data).await.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);

        let rows = crate::fixture::publish_batch(&[("orders", "a", 1, b"a1")]);
        let without_descriptor =
            FlightDataEncoderBuilder::new().build(futures::stream::iter([Ok(rows)]));
        let status = client.put(without_descriptor).await.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
        assert_eq!(client.pull("c", 10).await.unwrap().outcome, "no_pocket");
    })
    .await;
}
