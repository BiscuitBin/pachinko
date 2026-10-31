//! The Arrow Flight service: the module's two surfaces over gRPC (decision 0019).
//!
//! `do_put` is `Publish`, `do_get` is `Pull`, and `do_action` carries the joins,
//! the consumer's four lease actions and a `status` read of the `Consuming`
//! surface's `exposes`; `list_actions` lists those seven, and every other method
//! answers `UNIMPLEMENTED`. Every body and reply is an Arrow IPC stream, which
//! `codec` reads and writes; `actions` dispatches the action types. Each handler
//! sends one command to the queue's owner through a [`QueueHandle`].
//!
//! The status codes: an unknown party is `NOT_FOUND`; an action the rules reject,
//! or a join of an id that has joined, is `FAILED_PRECONDITION` with the error's
//! text; a malformed body, descriptor or ticket, or an unknown action type, is
//! `INVALID_ARGUMENT`; an owner that has stopped, or a request whose handler has
//! not begun or an upload still open when shutdown starts, is `UNAVAILABLE`. A pull
//! that finds no pocket is a reply, never an error.

mod actions;
mod codec;

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use arrow_flight::decode::{DecodedFlightData, FlightDataDecoder};
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::error::FlightError;
use arrow_flight::flight_service_server::{FlightService, FlightServiceServer};
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PollInfo, PutResult, SchemaResult, Ticket,
};
use futures::stream::{self, BoxStream};
use futures::{StreamExt, TryStreamExt};
use libpachinko::error::{ActionError, ConsumerError, JoinError, PublishError};
use libpachinko::queue::Publication;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tonic::body::Body;
use tonic::codegen::{BoxFuture, Service, http};
use tonic::server::NamedService;
use tonic::transport::Server;
use tonic::transport::server::TcpIncoming;
use tonic::{Code, Request, Response, Status, Streaming};

use crate::ServeError;
use crate::actor::{QueueHandle, Stopped};

/// A reply stream, as every streaming method returns one.
type Replies<T> = BoxStream<'static, Result<T, Status>>;

/// The Flight service over the queue's owner.
#[derive(Clone, Debug)]
struct PachinkoFlight {
    queue: QueueHandle,
    /// Turns `true` when shutdown starts, so an upload that is still open stops
    /// waiting for its client.
    stopping: watch::Receiver<bool>,
}

/// Marks, in a request's extensions, whether its handler has begun:
/// [`UntilStopping`] inserts it unset, and the handler sets it once tonic has
/// handed it the request.
#[derive(Clone, Debug, Default)]
struct Begun(Arc<AtomicBool>);

/// Answers `UNAVAILABLE`, once shutdown starts, to a request whose handler has not
/// begun. tonic reads a unary request's message and its end before it calls the
/// handler, so without this a `do_get` or `do_action` whose message never arrives
/// would hold the graceful shutdown open. A handler that has begun runs to its end,
/// so no command already sent to the owner is cut off.
#[derive(Clone, Debug)]
struct UntilStopping<S> {
    inner: S,
    stopping: watch::Receiver<bool>,
}

/// Serves Flight over `queue` on `listener` until `shutdown` resolves and tonic's
/// graceful shutdown has finished. When `shutdown` resolves the requests are told,
/// so a request whose handler has not begun, or an upload still open, is answered
/// `UNAVAILABLE` rather than holding the graceful shutdown open until its client
/// ends it.
///
/// # Errors
///
/// [`ServeError::Transport`] when the server fails.
pub(crate) async fn serve_flight(
    queue: QueueHandle,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServeError> {
    let (stop, stopping) = watch::channel(false);
    let shutdown = async move {
        shutdown.await;
        stop.send_replace(true);
    };
    let flight = PachinkoFlight {
        queue,
        stopping: stopping.clone(),
    };
    Server::builder()
        .add_service(UntilStopping {
            inner: FlightServiceServer::new(flight),
            stopping,
        })
        .serve_with_incoming_shutdown(TcpIncoming::from(listener), shutdown)
        .await
        .map_err(|error| ServeError::Transport(error.to_string()))
}

impl<S: NamedService> NamedService for UntilStopping<S> {
    const NAME: &'static str = S::NAME;
}

impl<S, B> Service<http::Request<B>> for UntilStopping<S>
where
    S: Service<http::Request<B>, Response = http::Response<Body>, Error = Infallible>,
    S::Future: Send + 'static,
{
    type Response = http::Response<Body>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<B>) -> Self::Future {
        let begun = Begun::default();
        request.extensions_mut().insert(begun.clone());
        let answer = self.inner.call(request);
        Box::pin(answer_until_stopping(answer, begun, self.stopping.clone()))
    }
}

/// Awaits `answer`, unless shutdown starts before its handler has begun, when the
/// request is answered `UNAVAILABLE` instead.
async fn answer_until_stopping(
    answer: impl Future<Output = Result<http::Response<Body>, Infallible>>,
    begun: Begun,
    mut stopping: watch::Receiver<bool>,
) -> Result<http::Response<Body>, Infallible> {
    let mut answer = std::pin::pin!(answer);
    tokio::select! {
        biased;
        response = &mut answer => response,
        // The guard `wait_for` returns is not `Send`, so it is dropped in this block
        // rather than held while the handler's answer is awaited below.
        () = async { let _ = stopping.wait_for(|stopped| *stopped).await; } => {
            if begun.0.load(Ordering::SeqCst) {
                answer.await
            } else {
                Ok(Status::unavailable("the service is shutting down").into_http())
            }
        }
    }
}

/// The flag [`UntilStopping`] put in `request`'s extensions; a request it did not
/// wrap carries none.
fn begun_flag<T>(request: &Request<T>) -> Option<Begun> {
    request.extensions().get::<Begun>().cloned()
}

/// Sets `begun`, so a shutdown that starts from now on lets the handler finish.
fn begin(begun: Option<Begun>) {
    if let Some(Begun(flag)) = begun {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tonic::async_trait]
impl FlightService for PachinkoFlight {
    type HandshakeStream = Replies<HandshakeResponse>;
    type ListFlightsStream = Replies<FlightInfo>;
    type DoGetStream = Replies<FlightData>;
    type DoPutStream = Replies<PutResult>;
    type DoExchangeStream = Replies<FlightData>;
    type DoActionStream = Replies<arrow_flight::Result>;
    type ListActionsStream = Replies<ActionType>;

    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        Err(not_offered("handshake"))
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        Err(not_offered("list_flights"))
    }

    async fn get_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        Err(not_offered("get_flight_info"))
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        Err(not_offered("poll_flight_info"))
    }

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        Err(not_offered("get_schema"))
    }

    /// `Pull`: the ticket names the consumer and the limit, and the reply is the
    /// batch under the consumer's lease, or a schema-only `no_pocket` stream.
    async fn do_get(
        &self,
        request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        begin(begun_flag(&request));
        let (consumer_id, limit) = codec::pull_ticket(&request.into_inner().ticket)?;
        let outcome = self
            .queue
            .pull_batch(consumer_id, limit)
            .await
            .map_err(unavailable)?
            .map_err(|error| consumer_status(&error))?;
        let (schema, records) = codec::pull_reply(outcome)?;
        let data = FlightDataEncoderBuilder::new()
            .with_schema(schema)
            .build(stream::iter(records.map(Ok)))
            .map_err(Status::from);
        Ok(Response::new(data.boxed()))
    }

    /// `Publish`: the descriptor's command names the publisher, and each input
    /// batch is published as one command and answered by one `PutResult`. Every
    /// batch is read before any is published, so a malformed one, or one still open
    /// when shutdown starts and answered `UNAVAILABLE`, publishes nothing. The
    /// handler counts as begun only once every batch is read.
    async fn do_put(
        &self,
        request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let begun = begun_flag(&request);
        let input = request.into_inner().map_err(FlightError::from);
        let mut stopping = self.stopping.clone();
        let decoded: Vec<DecodedFlightData> = tokio::select! {
            biased;
            decoded = FlightDataDecoder::new(input).try_collect() => decoded.map_err(malformed)?,
            _ = stopping.wait_for(|stopped| *stopped) => {
                return Err(Status::unavailable("the service is shutting down"));
            }
        };
        begin(begun);
        let (publisher_id, batches) = codec::put_input(&decoded)?;
        let results: Vec<PutResult> = stream::iter(batches)
            .then(|publications| self.publish_put(&publisher_id, publications))
            .try_collect()
            .await?;
        Ok(Response::new(
            stream::iter(results.into_iter().map(Ok)).boxed(),
        ))
    }

    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        Err(not_offered("do_exchange"))
    }

    /// The joins, the consumer's lease actions and `status`, each with one reply.
    async fn do_action(
        &self,
        request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        begin(begun_flag(&request));
        let action = request.into_inner();
        let bodies = actions::perform(&self.queue, &action.r#type, &action.body).await?;
        let replies = bodies
            .into_iter()
            .map(|body| Ok(arrow_flight::Result { body }));
        Ok(Response::new(stream::iter(replies).boxed()))
    }

    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        let listing = actions::listing().into_iter().map(Ok);
        Ok(Response::new(stream::iter(listing).boxed()))
    }
}

impl PachinkoFlight {
    /// Publishes one input batch's rows as one command and writes its `PutResult`.
    async fn publish_put(
        &self,
        publisher_id: &str,
        publications: Vec<Publication>,
    ) -> Result<PutResult, Status> {
        let outcomes = self
            .queue
            .publish_batch(publisher_id.to_owned(), publications)
            .await
            .map_err(unavailable)?
            .map_err(|error| publish_status(&error))?;
        codec::put_result(&outcomes)
    }
}

/// `UNIMPLEMENTED`, for a Flight method this service does not offer.
fn not_offered(method: &str) -> Status {
    Status::unimplemented(format!("{method} is not part of the pachinko Flight API"))
}

/// `UNAVAILABLE`: the queue's owner has stopped.
fn unavailable(error: Stopped) -> Status {
    Status::unavailable(error.to_string())
}

/// A request stream that could not be decoded is `INVALID_ARGUMENT`; a transport
/// failure keeps its own status.
fn malformed(error: FlightError) -> Status {
    match error {
        FlightError::Tonic(status) => *status,
        other => Status::invalid_argument(other.to_string()),
    }
}

/// A refused join is `FAILED_PRECONDITION`.
fn join_status(error: &JoinError) -> Status {
    Status::failed_precondition(error.to_string())
}

/// An unknown publisher is `NOT_FOUND`; any other refused publish is
/// `FAILED_PRECONDITION`.
fn publish_status(error: &PublishError) -> Status {
    let code = match error {
        PublishError::UnknownPublisher(_) => Code::NotFound,
        _ => Code::FailedPrecondition,
    };
    Status::new(code, error.to_string())
}

/// An unknown consumer is `NOT_FOUND`; any other refusal is `FAILED_PRECONDITION`.
fn consumer_status(error: &ConsumerError) -> Status {
    let code = match error {
        ConsumerError::UnknownConsumer(_) => Code::NotFound,
        _ => Code::FailedPrecondition,
    };
    Status::new(code, error.to_string())
}

/// An unknown consumer is `NOT_FOUND`; a lease that is not active, not the
/// consumer's, or a record not held is `FAILED_PRECONDITION`.
fn action_status(error: &ActionError) -> Status {
    let code = match error {
        ActionError::UnknownConsumer(_) => Code::NotFound,
        _ => Code::FailedPrecondition,
    };
    Status::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::task::{Context, Poll};

    use super::{
        Begun, PachinkoFlight, UntilStopping, action_status, begin, consumer_status, join_status,
        malformed, publish_status, unavailable,
    };
    use crate::actor::{QueueHandle, Stopped};
    use arrow_flight::error::FlightError;
    use arrow_flight::flight_service_server::FlightService;
    use arrow_flight::{Criteria, FlightDescriptor};
    use libpachinko::error::{ActionError, ConsumerError, JoinError, PublishError};
    use libpachinko::queue::LeaseId;
    use tokio::sync::{oneshot, watch};
    use tonic::body::Body;
    use tonic::codec::{DecodeBuf, Decoder};
    use tonic::codegen::{BoxFuture, Service, http};
    use tonic::{Code, Request, Status, Streaming};

    /// A decoder for a request stream that carries no message.
    struct NoMessages<T>(std::marker::PhantomData<T>);

    impl<T> Decoder for NoMessages<T> {
        type Item = T;
        type Error = Status;

        fn decode(&mut self, _src: &mut DecodeBuf<'_>) -> Result<Option<T>, Status> {
            Ok(None)
        }
    }

    fn empty_stream<T: Send + 'static>() -> Request<Streaming<T>> {
        let decoder = NoMessages(std::marker::PhantomData);
        Request::new(Streaming::new_empty(decoder, Body::default()))
    }

    /// A service whose owner has already stopped; the methods under test never
    /// reach it.
    fn service() -> PachinkoFlight {
        PachinkoFlight {
            queue: QueueHandle::stopped(),
            stopping: watch::channel(false).1,
        }
    }

    /// An inner service whose handler sets the request's `Begun` when `begins`, then
    /// answers with an empty response once `answer` is sent.
    struct Parked {
        begins: bool,
        answer: Option<oneshot::Receiver<()>>,
    }

    impl Service<http::Request<()>> for Parked {
        type Response = http::Response<Body>;
        type Error = Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: http::Request<()>) -> Self::Future {
            let begins = self.begins;
            let begun = request.extensions().get::<Begun>().cloned();
            let answer = self.answer.take().unwrap();
            Box::pin(async move {
                begin(begun.filter(|_| begins));
                answer.await.unwrap();
                Ok(http::Response::new(Body::default()))
            })
        }
    }

    /// `Parked` behind `UntilStopping`, with the sender that starts shutdown and
    /// the one that lets the handler answer.
    fn until_stopping(
        begins: bool,
    ) -> (
        UntilStopping<Parked>,
        watch::Sender<bool>,
        oneshot::Sender<()>,
    ) {
        let (stop, stopping) = watch::channel(false);
        let (answer, answered) = oneshot::channel();
        let inner = Parked {
            begins,
            answer: Some(answered),
        };
        (UntilStopping { inner, stopping }, stop, answer)
    }

    #[tokio::test]
    async fn shutdown_answers_a_request_whose_handler_has_not_begun_unavailable() {
        let (mut service, stop, _answer) = until_stopping(false);
        let mut answering = service.call(http::Request::new(()));
        assert!(futures::poll!(&mut answering).is_pending());
        stop.send_replace(true);
        let response = answering.await.unwrap();
        let status = Status::from_header_map(response.headers()).unwrap();
        assert_eq!(status.code(), Code::Unavailable);
        assert_eq!(status.message(), "the service is shutting down");
    }

    #[tokio::test]
    async fn shutdown_lets_a_handler_that_has_begun_answer() {
        let (mut service, stop, answer) = until_stopping(true);
        let mut answering = service.call(http::Request::new(()));
        assert!(futures::poll!(&mut answering).is_pending());
        stop.send_replace(true);
        assert!(futures::poll!(&mut answering).is_pending());
        answer.send(()).unwrap();
        let response = answering.await.unwrap();
        assert!(Status::from_header_map(response.headers()).is_none());
    }

    fn descriptor() -> Request<FlightDescriptor> {
        Request::new(FlightDescriptor::new_cmd("x"))
    }

    fn assert_unimplemented(status: &Status, method: &str) {
        assert_eq!(status.code(), Code::Unimplemented);
        assert_eq!(
            status.message(),
            format!("{method} is not part of the pachinko Flight API")
        );
    }

    #[tokio::test]
    async fn every_method_outside_the_api_is_unimplemented() {
        let flight = service();
        let status = flight.handshake(empty_stream()).await.err().unwrap();
        assert_unimplemented(&status, "handshake");
        let criteria = Request::new(Criteria::default());
        let status = flight.list_flights(criteria).await.err().unwrap();
        assert_unimplemented(&status, "list_flights");
        let status = flight.get_flight_info(descriptor()).await.unwrap_err();
        assert_unimplemented(&status, "get_flight_info");
        let status = flight.poll_flight_info(descriptor()).await.unwrap_err();
        assert_unimplemented(&status, "poll_flight_info");
        let status = flight.get_schema(descriptor()).await.unwrap_err();
        assert_unimplemented(&status, "get_schema");
        let status = flight.do_exchange(empty_stream()).await.err().unwrap();
        assert_unimplemented(&status, "do_exchange");
    }

    #[tokio::test]
    async fn a_put_without_a_message_is_an_invalid_argument() {
        let status = service().do_put(empty_stream()).await.err().unwrap();
        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[test]
    fn a_stopped_owner_is_unavailable() {
        let status = unavailable(Stopped::OwnerTask);
        assert_eq!(status.code(), Code::Unavailable);
        assert_eq!(status.message(), "the queue's owner task has stopped");
    }

    #[test]
    fn a_decode_failure_is_an_invalid_argument_and_a_transport_one_keeps_its_code() {
        let status = malformed(FlightError::DecodeError("bad".to_owned()));
        assert_eq!(status.code(), Code::InvalidArgument);
        let status = malformed(FlightError::Tonic(Box::new(Status::cancelled("gone"))));
        assert_eq!(status.code(), Code::Cancelled);
    }

    #[test]
    fn a_refused_join_is_a_failed_precondition() {
        let status = join_status(&JoinError::AlreadyJoined("p".to_owned()));
        assert_eq!(status.code(), Code::FailedPrecondition);
        assert_eq!(status.message(), "p has already joined");
    }

    #[test]
    fn an_unknown_publisher_is_not_found_and_an_equal_rank_a_failed_precondition() {
        let status = publish_status(&PublishError::UnknownPublisher("p".to_owned()));
        assert_eq!(status.code(), Code::NotFound);
        let status = publish_status(&PublishError::EqualRank {
            pocket_key: "pk".to_owned(),
            record_key: "rk".to_owned(),
            rank: 1,
        });
        assert_eq!(status.code(), Code::FailedPrecondition);
    }

    #[test]
    fn an_unknown_consumer_is_not_found() {
        let status = consumer_status(&ConsumerError::UnknownConsumer("c".to_owned()));
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no consumer c has joined");
        let status = action_status(&ActionError::UnknownConsumer("c".to_owned()));
        assert_eq!(status.code(), Code::NotFound);
    }

    #[test]
    fn a_rejected_action_is_a_failed_precondition_with_the_errors_text() {
        let lease = LeaseId::from_raw(3);
        for (error, text) in [
            (ActionError::LeaseNotActive(lease), "lease 3 is not active"),
            (
                ActionError::LeaseNotYours(lease),
                "lease 3 belongs to another consumer",
            ),
            (
                ActionError::RecordNotHeld("rk".to_owned()),
                "record rk is not held under this lease",
            ),
        ] {
            let status = action_status(&error);
            assert_eq!(status.code(), Code::FailedPrecondition);
            assert_eq!(status.message(), text);
        }
    }
}
