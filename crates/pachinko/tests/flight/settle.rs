//! `ack` and `nack`: settling a held version, deferring a newer one, and a dead
//! letter that no pull sees again.

use pachinko::ServeOptions;
use tonic::Code;

use crate::fixture::{Client, Record, config, with_server};

/// Joins `p` and `c`, publishes `a` and `b` at rank 1 into `orders`, and pulls
/// both, returning the lease id.
async fn holding_two(client: &mut Client) -> u64 {
    client.join_publisher("p").await.unwrap();
    client.join_consumer("c").await.unwrap();
    let rows = [("orders", "a", 1, &b"a1"[..]), ("orders", "b", 1, b"b1")];
    client.publish("p", &rows).await.unwrap();
    client.pull("c", 10).await.unwrap().lease_id.unwrap()
}

#[tokio::test]
async fn an_ack_of_the_held_version_settles_the_record() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let lease = holding_two(&mut client).await;
        assert_eq!(client.ack("c", lease, "a").await.unwrap(), "settled");
        let held = client.status("c").await.unwrap().held;
        assert_eq!(held, [("b".to_owned(), 1, b"b1".to_vec())]);
        let again = client.ack("c", lease, "a").await.unwrap_err();
        assert_eq!(again.code(), Code::FailedPrecondition);
        assert_eq!(again.message(), "record a is not held under this lease");
    })
    .await;
}

#[tokio::test]
async fn an_ack_defers_a_newer_version_which_the_next_lease_hands_out() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let lease = holding_two(&mut client).await;
        let outcome = client.publish("p", &[("orders", "a", 2, b"a2")]).await;
        assert_eq!(outcome.unwrap(), ["superseded"]);
        assert_eq!(client.ack("c", lease, "a").await.unwrap(), "deferred");
        // Deferred: no longer held, and not handed out again under this lease.
        let held = client.status("c").await.unwrap().held;
        assert_eq!(held, [("b".to_owned(), 1, b"b1".to_vec())]);
        let within = client.pull("c", 10).await.unwrap();
        assert_eq!(within.records, Vec::<Record>::new());
        // Once the lease ends the newer version is queued, and the next lease
        // hands it out behind the record still held.
        client.release("c", lease).await.unwrap();
        let next = client.pull("c", 10).await.unwrap();
        assert_ne!(next.lease_id, Some(lease));
        assert_eq!(
            next.records,
            [
                ("b".to_owned(), 1, b"b1".to_vec()),
                ("a".to_owned(), 2, b"a2".to_vec())
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn a_nack_dead_letters_the_record_and_no_pull_sees_it_again() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let lease = holding_two(&mut client).await;
        assert_eq!(client.nack("c", lease, "a").await.unwrap(), "settled");
        let held = client.status("c").await.unwrap().held;
        assert_eq!(held, [("b".to_owned(), 1, b"b1".to_vec())]);
        client.release("c", lease).await.unwrap();
        let next = client.pull("c", 10).await.unwrap();
        assert_ne!(next.lease_id, Some(lease));
        assert_eq!(next.records, [("b".to_owned(), 1, b"b1".to_vec())]);
    })
    .await;
}

#[tokio::test]
async fn a_nack_of_a_superseded_hold_defers_the_newer_version() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let lease = holding_two(&mut client).await;
        client
            .publish("p", &[("orders", "b", 5, b"b5")])
            .await
            .unwrap();
        assert_eq!(client.nack("c", lease, "b").await.unwrap(), "deferred");
        client.release("c", lease).await.unwrap();
        let next = client.pull("c", 10).await.unwrap();
        assert_eq!(
            next.records,
            [
                ("a".to_owned(), 1, b"a1".to_vec()),
                ("b".to_owned(), 5, b"b5".to_vec())
            ]
        );
    })
    .await;
}
