//! `do_get` as `Pull`: the batch and its lease in the reply schema's metadata, a
//! pull that finds no pocket, a non-positive limit, and a batch too large for one
//! gRPC message.

use arrow_flight::Ticket;
use pachinko::ServeOptions;
use tonic::Code;

use crate::fixture::{Client, Record, Row, config, status_of, with_server};

/// Joins `p`, `c1` and `c2`, and publishes `a` and `b` into `first` and `c` into
/// `second`, in that order.
async fn two_pockets(client: &mut Client) {
    client.join_publisher("p").await.unwrap();
    client.join_consumer("c1").await.unwrap();
    client.join_consumer("c2").await.unwrap();
    let rows = [
        ("first", "a", 1, &b"a"[..]),
        ("first", "b", 1, b"b"),
        ("second", "c", 1, b"c"),
    ];
    client.publish("p", &rows).await.unwrap();
}

#[tokio::test]
async fn a_pull_claims_the_front_pocket_and_the_next_pulls_within_the_lease() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        two_pockets(&mut client).await;
        let claimed = client.pull("c1", 1).await.unwrap();
        assert_eq!(claimed.outcome, "batch");
        assert_eq!(claimed.pocket_key.as_deref(), Some("first"));
        assert_eq!(claimed.records, [("a".to_owned(), 1, b"a".to_vec())]);
        let lease_id = claimed.lease_id.unwrap();
        assert!(claimed.expires_at.is_some());

        let within = client.pull("c1", 10).await.unwrap();
        assert_eq!(within.outcome, "batch");
        assert_eq!(within.lease_id, Some(lease_id));
        assert_eq!(within.pocket_key.as_deref(), Some("first"));
        assert_eq!(within.records, [("b".to_owned(), 1, b"b".to_vec())]);
    })
    .await;
}

#[tokio::test]
async fn two_consumers_are_never_handed_one_pocket() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        two_pockets(&mut client).await;
        let first = client.pull("c1", 10).await.unwrap();
        let second = client.pull("c2", 10).await.unwrap();
        assert_eq!(first.pocket_key.as_deref(), Some("first"));
        assert_eq!(second.pocket_key.as_deref(), Some("second"));
        assert_ne!(first.lease_id, second.lease_id);
        client.join_consumer("c3").await.unwrap();
        assert_eq!(client.pull("c3", 10).await.unwrap().outcome, "no_pocket");
    })
    .await;
}

#[tokio::test]
async fn a_pull_that_finds_no_pocket_is_a_schema_only_reply_not_an_error() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_consumer("c").await.unwrap();
        let pulled = client.pull("c", 10).await.unwrap();
        assert_eq!(pulled.outcome, "no_pocket");
        assert_eq!(pulled.records, Vec::<Record>::new());
        assert_eq!(pulled.lease_id, None);
        assert_eq!(pulled.pocket_key, None);
        assert_eq!(pulled.expires_at, None);
    })
    .await;
}

#[tokio::test]
async fn a_non_positive_limit_is_an_empty_batch_not_an_error() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        two_pockets(&mut client).await;
        for limit in [0, -3] {
            let pulled = client.pull("c1", limit).await.unwrap();
            assert_eq!(pulled.outcome, "batch", "limit {limit}");
            assert_eq!(pulled.pocket_key.as_deref(), Some("first"));
            assert_eq!(pulled.records, Vec::<Record>::new(), "limit {limit}");
        }
        let held = client.status("c1").await.unwrap().held;
        assert_eq!(held, Vec::<Record>::new());
    })
    .await;
}

#[tokio::test]
async fn an_unknown_consumer_is_not_found_and_a_malformed_ticket_an_invalid_argument() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let status = client.pull("nobody", 10).await.unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no consumer nobody has joined");
        let status = client
            .flight()
            .do_get(Ticket::new("not ipc"))
            .await
            .map(drop)
            .map_err(status_of)
            .unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
    })
    .await;
}

#[tokio::test]
async fn a_pull_answers_when_two_large_records_lead_many_small_ones() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_publisher("p").await.unwrap();
        client.join_consumer("c").await.unwrap();
        let large = vec![7_u8; 3 << 20];
        let keys: Vec<String> = (0..100).map(|index| format!("k{index:03}")).collect();
        let payloads = [large.as_slice(); 2].into_iter().chain([&b"t"[..]; 98]);
        let rows: Vec<Row<'_>> = keys
            .iter()
            .zip(payloads)
            .map(|(key, payload)| ("orders", key.as_str(), 1, payload))
            .collect();
        // One batch per large record, so no publish message nears the 4 MiB limit.
        client
            .publish_batches("p", &[&rows[..1], &rows[1..2], &rows[2..]])
            .await
            .unwrap();
        let pulled = client.pull("c", 100).await.unwrap();
        let pulled_keys: Vec<&str> = pulled
            .records
            .iter()
            .map(|(key, ..)| key.as_str())
            .collect();
        assert_eq!(pulled_keys, keys, "every record, in order");
        assert_eq!(pulled.records[1].2.len(), 3 << 20);
    })
    .await;
}
