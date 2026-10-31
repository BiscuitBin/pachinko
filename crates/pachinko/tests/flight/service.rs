//! The service as a whole: `list_actions`, a lapse the ticker processes, and
//! shutdown.

use std::time::Duration;

use futures::TryStreamExt;
use pachinko::{ServeOptions, serve};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::fixture::{Client, SCENARIO_DEADLINE, config, status_of, with_server};

/// The lapse test's tick.
const TICK: Duration = Duration::from_millis(10);

#[tokio::test]
async fn list_actions_lists_the_seven_with_descriptions() {
    with_server(config(), ServeOptions::default(), |mut client| async move {
        let listed: Vec<_> = client
            .flight()
            .list_actions()
            .await
            .map_err(status_of)
            .unwrap()
            .try_collect()
            .await
            .map_err(status_of)
            .unwrap();
        let kinds: Vec<&str> = listed.iter().map(|action| action.r#type.as_str()).collect();
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
        assert!(listed.iter().all(|action| !action.description.is_empty()));
    })
    .await;
}

/// Polls `status` for `c`, one tick apart, until it has no active lease.
async fn until_no_lease(client: &mut Client) {
    while client.status("c").await.unwrap().lease != "none" {
        tokio::time::sleep(TICK).await;
    }
}

#[tokio::test]
async fn the_ticker_processes_a_lapse() {
    let mut lapsing = config();
    lapsing.lease_ttl = Duration::ZERO;
    let options = ServeOptions::default().with_tick_interval(TICK);
    with_server(lapsing, options, |mut client| async move {
        client.join_publisher("p").await.unwrap();
        client.join_consumer("c").await.unwrap();
        client
            .publish("p", &[("orders", "a", 1, b"a1")])
            .await
            .unwrap();
        let lapsed = client.pull("c", 10).await.unwrap().lease_id.unwrap();

        // Poll until the ticker has processed the lapse, under a deadline of a
        // hundred ticks; each poll waits one tick.
        let polled = tokio::time::timeout(TICK * 100, until_no_lease(&mut client)).await;
        assert!(polled.is_ok(), "no lapse within a hundred ticks");

        // The pocket went back to the queue, and the record is handed out again.
        let again = client.pull("c", 10).await.unwrap();
        assert!(again.lease_id.unwrap() > lapsed);
        assert_eq!(again.records, [("a".to_owned(), 1, b"a1".to_vec())]);
    })
    .await;
}

#[tokio::test]
async fn shutdown_returns_ok_while_a_client_is_still_connected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        stopped.await.unwrap();
    };
    let serving = serve(config(), listener, ServeOptions::default(), shutdown);
    let connected = async {
        let mut client = Client::connect(&address).await;
        client.join_consumer("c").await.unwrap();
        stop.send(()).unwrap();
        client
    };
    let (outcome, mut client) = tokio::time::timeout(SCENARIO_DEADLINE, async {
        tokio::join!(serving, connected)
    })
    .await
    .expect("serve returns within the deadline");
    assert_eq!(outcome, Ok(()));
    assert!(client.join_consumer("d").await.is_err());
}
