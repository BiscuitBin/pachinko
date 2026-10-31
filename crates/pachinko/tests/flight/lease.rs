//! `renew`, `release` and `status`, and the status codes of a rejected or
//! malformed action.

use std::time::Duration;

use pachinko::ServeOptions;
use tonic::Code;

use crate::fixture::{Client, Record, body, config, utf8, with_server};

/// Joins `p`, `c1` and `c2`, publishes `a` into `orders`, and pulls it as `c1`,
/// returning the lease id and its expiry.
async fn c1_holding(client: &mut Client) -> (u64, u64) {
    client.join_publisher("p").await.unwrap();
    client.join_consumer("c1").await.unwrap();
    client.join_consumer("c2").await.unwrap();
    client
        .publish("p", &[("orders", "a", 1, b"a1")])
        .await
        .unwrap();
    let pulled = client.pull("c1", 10).await.unwrap();
    (pulled.lease_id.unwrap(), pulled.expires_at.unwrap())
}

#[tokio::test]
async fn status_exposes_the_pocket_key_the_expiry_and_the_held_records() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        client.join_consumer("c0").await.unwrap();
        let before = client.status("c0").await.unwrap();
        assert_eq!(before.lease, "none");
        assert_eq!(
            (before.lease_id, before.pocket_key, before.expires_at),
            (None, None, None)
        );
        assert_eq!(before.held, Vec::<Record>::new());

        let (lease, expires_at) = c1_holding(&mut client).await;
        let status = client.status("c1").await.unwrap();
        assert_eq!(status.lease, "active");
        assert_eq!(status.lease_id, Some(lease));
        assert_eq!(status.pocket_key.as_deref(), Some("orders"));
        assert_eq!(status.expires_at, Some(expires_at));
        assert_eq!(status.held, [("a".to_owned(), 1, b"a1".to_vec())]);
    })
    .await;
}

#[tokio::test]
async fn a_renewal_moves_the_expiry() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (lease, expires_at) = c1_holding(&mut client).await;
        // Let the wall clock pass the grant's millisecond, so the renewal's
        // expiry is later; no deadline is being approached.
        tokio::time::sleep(Duration::from_millis(5)).await;
        let renewed = client.renew("c1", lease).await.unwrap();
        assert!(renewed > expires_at, "{renewed} > {expires_at}");
        assert_eq!(client.status("c1").await.unwrap().expires_at, Some(renewed));
    })
    .await;
}

#[tokio::test]
async fn a_release_ends_the_lease_and_requeues_the_pocket() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (lease, _) = c1_holding(&mut client).await;
        let (released, ended_at) = client.release("c1", lease).await.unwrap();
        assert_eq!(released, lease);
        assert!(ended_at.is_some());
        assert_eq!(client.status("c1").await.unwrap().lease, "none");
        let next = client.pull("c2", 10).await.unwrap();
        assert_eq!(next.pocket_key.as_deref(), Some("orders"));
        assert!(next.lease_id.unwrap() > lease);
        assert_eq!(next.records, [("a".to_owned(), 1, b"a1".to_vec())]);
    })
    .await;
}

#[tokio::test]
async fn an_action_naming_an_ended_lease_is_a_failed_precondition() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (ended, _) = c1_holding(&mut client).await;
        client.release("c1", ended).await.unwrap();
        // The same consumer holds the same pocket again, under a newer id: the
        // ended id still names nothing.
        let current = client.pull("c1", 10).await.unwrap().lease_id.unwrap();
        assert!(current > ended);
        let message = format!("lease {ended} is not active");
        let status = client.ack("c1", ended, "a").await.unwrap_err();
        assert_eq!(
            (status.code(), status.message()),
            (Code::FailedPrecondition, message.as_str())
        );
        let status = client.nack("c1", ended, "a").await.unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        let status = client.renew("c1", ended).await.unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        let status = client.release("c1", ended).await.unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        assert_eq!(client.status("c1").await.unwrap().lease_id, Some(current));
    })
    .await;
}

#[tokio::test]
async fn another_consumers_lease_is_a_failed_precondition() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (lease, _) = c1_holding(&mut client).await;
        let status = client.renew("c2", lease).await.unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
        assert_eq!(
            status.message(),
            format!("lease {lease} belongs to another consumer")
        );
        let status = client.ack("c2", lease, "a").await.unwrap_err();
        assert_eq!(status.code(), Code::FailedPrecondition);
    })
    .await;
}

#[tokio::test]
async fn an_unknown_consumer_is_not_found() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (lease, _) = c1_holding(&mut client).await;
        let status = client.ack("nobody", lease, "a").await.unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no consumer nobody has joined");
        let status = client.status("nobody").await.unwrap_err();
        assert_eq!(status.code(), Code::NotFound);
    })
    .await;
}

#[tokio::test]
async fn a_malformed_body_or_an_unknown_action_type_is_an_invalid_argument() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let (lease, _) = c1_holding(&mut client).await;
        let garbage = client.action("ack", "not ipc".into()).await.unwrap_err();
        assert_eq!(garbage.code(), Code::InvalidArgument);
        let without_lease = body(vec![("consumer_id", utf8("c1")), ("record_key", utf8("a"))]);
        let status = client.action("ack", without_lease).await.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
        assert_eq!(
            status.message(),
            "the body needs a non-null UInt64 column lease_id"
        );
        let status = client
            .action("truncate", body(vec![("consumer_id", utf8("c1"))]))
            .await;
        assert_eq!(status.unwrap_err().code(), Code::InvalidArgument);
        // Nothing was settled by any of them.
        assert_eq!(client.status("c1").await.unwrap().lease_id, Some(lease));
        assert_eq!(client.status("c1").await.unwrap().held.len(), 1);
    })
    .await;
}
