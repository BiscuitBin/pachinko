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
//! `INVALID_ARGUMENT`; an owner that has stopped, or an upload still open when
//! shutdown starts, is `UNAVAILABLE`. A pull that finds no pocket is a reply,
//! never an error.

mod actions;
mod codec;

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

/// Serves Flight over `queue` on `listener` until `shutdown` resolves and tonic's
/// graceful shutdown has finished. When `shutdown` resolves the handlers are told,
/// so an upload still open then is answered `UNAVAILABLE` rather than holding the
/// graceful shutdown open until its client ends it.
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
    Server::builder()
        .add_service(FlightServiceServer::new(PachinkoFlight { queue, stopping }))
        .serve_with_incoming_shutdown(TcpIncoming::from(listener), shutdown)
        .await
        .map_err(|error| ServeError::Transport(error.to_string()))
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
    /// when shutdown starts and answered `UNAVAILABLE`, publishes nothing.
    async fn do_put(
        &self,
        request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let input = request.into_inner().map_err(FlightError::from);
        let mut stopping = self.stopping.clone();
        let decoded: Vec<DecodedFlightData> = tokio::select! {
            biased;
            decoded = FlightDataDecoder::new(input).try_collect() => decoded.map_err(malformed)?,
            _ = stopping.wait_for(|stopped| *stopped) => {
                return Err(Status::unavailable("the service is shutting down"));
            }
        };
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
        let action = request.into_inner();
        let body = actions::perform(&self.queue, &action.r#type, &action.body).await?;
        let reply = arrow_flight::Result { body };
        Ok(Response::new(stream::iter([Ok(reply)]).boxed()))
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
    use super::{
        PachinkoFlight, action_status, consumer_status, join_status, malformed, publish_status,
        unavailable,
    };
    use crate::actor::{QueueHandle, Stopped};
    use arrow_flight::error::FlightError;
    use arrow_flight::flight_service_server::FlightService;
    use arrow_flight::{Criteria, FlightDescriptor};
    use libpachinko::error::{ActionError, ConsumerError, JoinError, PublishError};
    use libpachinko::queue::LeaseId;
    use tokio::sync::watch;
    use tonic::codec::{DecodeBuf, Decoder};
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
        Request::new(Streaming::new_empty(decoder, tonic::body::Body::default()))
    }

    /// A service whose owner has already stopped; the methods under test never
    /// reach it.
    fn service() -> PachinkoFlight {
        PachinkoFlight {
            queue: QueueHandle::stopped(),
            stopping: watch::channel(false).1,
        }
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
        let status = unavailable(Stopped);
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
