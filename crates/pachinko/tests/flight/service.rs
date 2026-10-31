//! The service as a whole: `list_actions`, a lapse the ticker processes, and
//! shutdown.

use std::task::Poll;
use std::time::Duration;

use arrow_flight::{FlightClient, FlightData};
use futures::TryStreamExt;
use pachinko::{ServeOptions, serve};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tonic::Code;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::http::uri::PathAndQuery;
use tonic::transport::Channel;

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

#[tokio::test]
async fn shutdown_answers_an_upload_still_open_unavailable_and_returns_ok() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        stopped.await.unwrap();
    };
    let serving = serve(config(), listener, ServeOptions::default(), shutdown);
    let mut client = Client::connect(&address).await;
    let mut raw = client.flight().inner().clone();
    // An upload that never ends. hyper polls a request's body only after it has
    // sent the headers, so the first poll says the upload is on the connection.
    let (sent, on_the_wire) = oneshot::channel::<()>();
    let mut sent = Some(sent);
    let input = futures::stream::poll_fn(move |_| {
        if let Some(sent) = sent.take() {
            sent.send(()).unwrap();
        }
        Poll::<Option<FlightData>>::Pending
    });
    let uploading = async move { raw.do_put(input).await };
    let stopping = async move {
        on_the_wire.await.unwrap();
        // The join goes out on the same connection after the upload's headers, so
        // its reply means the server has taken the upload before shutdown starts.
        client.join_consumer("c").await.unwrap();
        stop.send(()).unwrap();
    };
    let (outcome, uploaded, ()) = tokio::time::timeout(SCENARIO_DEADLINE, async {
        tokio::join!(serving, uploading, stopping)
    })
    .await
    .expect("serve returns within the deadline while an upload is open");
    assert_eq!(outcome, Ok(()));
    let status = uploaded.unwrap_err();
    assert_eq!(status.code(), Code::Unavailable);
    assert_eq!(status.message(), "the service is shutting down");
}

/// A codec for a request that never sends a message, so neither half is asked to
/// do anything.
#[derive(Clone, Copy, Debug, Default)]
struct Silent;

impl Encoder for Silent {
    type Item = ();
    type Error = tonic::Status;

    fn encode(&mut self, (): (), _dst: &mut EncodeBuf<'_>) -> Result<(), tonic::Status> {
        Ok(())
    }
}

impl Decoder for Silent {
    type Item = ();
    type Error = tonic::Status;

    fn decode(&mut self, _src: &mut DecodeBuf<'_>) -> Result<Option<()>, tonic::Status> {
        Ok(Some(()))
    }
}

impl Codec for Silent {
    type Encode = ();
    type Decode = ();
    type Encoder = Self;
    type Decoder = Self;

    fn encoder(&mut self) -> Self {
        Self
    }

    fn decoder(&mut self) -> Self {
        Self
    }
}

/// Opens a request to the unary-request method at `path` whose message never
/// arrives, starts shutdown once the server has taken it, and asserts that `serve`
/// returns `Ok` within the deadline and the request is answered `UNAVAILABLE`.
/// tonic reads such a request's message before it calls the handler, so this
/// stalls before `do_get` or `do_action` is reached.
async fn assert_shutdown_answers_an_unfinished_request_unavailable(path: &'static str) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        stopped.await.unwrap();
    };
    let serving = serve(config(), listener, ServeOptions::default(), shutdown);
    // One connection carries both the unfinished request and the round trip that
    // orders shutdown after it.
    let channel = Channel::from_shared(format!("http://{address}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut flight = FlightClient::new(channel.clone());
    let mut grpc = tonic::client::Grpc::new(channel);
    // A message that never comes. hyper polls a request's body only after it has
    // sent the headers, so the first poll says the request is on the connection.
    let (sent, on_the_wire) = oneshot::channel::<()>();
    let mut sent = Some(sent);
    let input = futures::stream::poll_fn(move |_| {
        if let Some(sent) = sent.take() {
            sent.send(()).unwrap();
        }
        Poll::<Option<()>>::Pending
    });
    let requesting = async move {
        grpc.ready().await.unwrap();
        let path = PathAndQuery::from_static(path);
        grpc.streaming(tonic::Request::new(input), path, Silent)
            .await
            .map(|_| ())
    };
    let stopping = async move {
        on_the_wire.await.unwrap();
        // `list_actions` goes out on the same connection after the request's
        // headers, so its reply means the server has taken the request before
        // shutdown starts.
        let _listed = flight.list_actions().await.map_err(status_of).unwrap();
        stop.send(()).unwrap();
    };
    let (outcome, requested, ()) = tokio::time::timeout(SCENARIO_DEADLINE, async {
        tokio::join!(serving, requesting, stopping)
    })
    .await
    .expect("serve returns within the deadline while a request is unfinished");
    assert_eq!(outcome, Ok(()));
    let status = requested.unwrap_err();
    assert_eq!(status.code(), Code::Unavailable);
    assert_eq!(status.message(), "the service is shutting down");
}

#[tokio::test]
async fn shutdown_answers_a_do_action_whose_message_never_arrives_unavailable_and_returns_ok() {
    assert_shutdown_answers_an_unfinished_request_unavailable(
        "/arrow.flight.protocol.FlightService/DoAction",
    )
    .await;
}

#[tokio::test]
async fn shutdown_answers_a_do_get_whose_message_never_arrives_unavailable_and_returns_ok() {
    assert_shutdown_answers_an_unfinished_request_unavailable(
        "/arrow.flight.protocol.FlightService/DoGet",
    )
    .await;
}
