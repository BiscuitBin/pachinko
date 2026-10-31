//! A server started in-process on loopback, an `arrow_flight::FlightClient` against
//! it, and the IPC bodies and replies the tests write and read.

use std::sync::Arc;
use std::time::Duration;

use arrow_array::cast::AsArray;
use arrow_array::types::{Int64Type, UInt64Type};
use arrow_array::{ArrayRef, BinaryArray, Int64Array, RecordBatch, StringArray, UInt64Array};
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::error::FlightError;
use arrow_flight::{Action, FlightClient, FlightDescriptor, Ticket};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use libpachinko::config::Config;
use pachinko::{ServeOptions, serve};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tonic::Status;
use tonic::transport::Channel;

/// How long a whole scenario, the server's shutdown included, may take before the
/// test fails instead of hanging.
pub(crate) const SCENARIO_DEADLINE: Duration = Duration::from_secs(30);

/// One publish row: pocket key, record key, rank and payload.
pub(crate) type Row<'row> = (&'row str, &'row str, i64, &'row [u8]);

/// One record as a pull or `status` hands it out: key, rank and payload.
pub(crate) type Record = (String, i64, Vec<u8>);

/// A pull's reply: the schema's metadata and the records.
#[derive(Debug)]
pub(crate) struct Pulled {
    pub(crate) outcome: String,
    pub(crate) lease_id: Option<u64>,
    pub(crate) pocket_key: Option<String>,
    pub(crate) expires_at: Option<u64>,
    pub(crate) records: Vec<Record>,
}

/// A `status` reply: the lease, when one is active, and the held records.
#[derive(Debug)]
pub(crate) struct LeaseStatus {
    pub(crate) lease: String,
    pub(crate) lease_id: Option<u64>,
    pub(crate) pocket_key: Option<String>,
    pub(crate) expires_at: Option<u64>,
    pub(crate) held: Vec<Record>,
}

/// The fixture's configuration: the module's defaults, with a lease long enough
/// that no test sees one lapse unless it asks for that.
pub(crate) fn config() -> Config {
    let mut config = Config::default();
    config.lease_ttl = Duration::from_mins(10);
    config
}

/// Runs `scenario` against a server serving `config` on a fresh loopback port, then
/// resolves the server's shutdown future and asserts `serve` returned `Ok`. The
/// scenario owns the client, so its connection is gone before the shutdown.
pub(crate) async fn with_server<Scenario, Run>(
    config: Config,
    options: ServeOptions,
    scenario: Scenario,
) where
    Scenario: FnOnce(Client) -> Run,
    Run: Future<Output = ()>,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        stopped.await.unwrap();
    };
    let serving = serve(config, listener, options, shutdown);
    let driving = async move {
        scenario(Client::connect(&address.to_string()).await).await;
        stop.send(()).unwrap();
    };
    let (outcome, ()) =
        tokio::time::timeout(SCENARIO_DEADLINE, async { tokio::join!(serving, driving) })
            .await
            .expect("the scenario and the shutdown finish within the deadline");
    assert_eq!(outcome, Ok(()));
}

/// A Flight client that speaks the pachinko mapping.
pub(crate) struct Client {
    flight: FlightClient,
}

impl Client {
    /// Connects to the server at `address`.
    pub(crate) async fn connect(address: &str) -> Self {
        let channel = Channel::from_shared(format!("http://{address}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self {
            flight: FlightClient::new(channel),
        }
    }

    /// The raw `FlightClient`, for the calls the mapping does not wrap.
    pub(crate) const fn flight(&mut self) -> &mut FlightClient {
        &mut self.flight
    }

    /// Performs action `kind` with `body` and reads its one-batch reply.
    pub(crate) async fn action(&mut self, kind: &str, body: Bytes) -> Result<RecordBatch, Status> {
        let replies: Vec<Bytes> = self
            .flight
            .do_action(Action::new(kind, body))
            .await
            .map_err(status_of)?
            .try_collect()
            .await
            .map_err(status_of)?;
        assert_eq!(replies.len(), 1, "one reply per action");
        Ok(read_ipc(&replies[0]))
    }

    /// `publisher_joins`, returning the reply's outcome.
    pub(crate) async fn join_publisher(&mut self, publisher_id: &str) -> Result<String, Status> {
        let reply = self
            .action(
                "publisher_joins",
                body(vec![("publisher_id", utf8(publisher_id))]),
            )
            .await?;
        Ok(string_cell(&reply, "outcome"))
    }

    /// `consumer_joins`, returning the reply's outcome.
    pub(crate) async fn join_consumer(&mut self, consumer_id: &str) -> Result<String, Status> {
        let reply = self
            .action(
                "consumer_joins",
                body(vec![("consumer_id", utf8(consumer_id))]),
            )
            .await?;
        Ok(string_cell(&reply, "outcome"))
    }

    /// Publishes each of `batches` as `publisher_id` in one `do_put`, returning
    /// each batch's per-row outcomes.
    pub(crate) async fn publish_batches(
        &mut self,
        publisher_id: &str,
        batches: &[&[Row<'_>]],
    ) -> Result<Vec<Vec<String>>, Status> {
        let batches: Vec<_> = batches.iter().map(|rows| Ok(publish_batch(rows))).collect();
        let data = FlightDataEncoderBuilder::new()
            .with_flight_descriptor(Some(FlightDescriptor::new_cmd(publisher_id.to_owned())))
            .build(futures::stream::iter(batches));
        self.put(data).await
    }

    /// Sends `data` as one `do_put`, returning each `PutResult`'s outcomes.
    pub(crate) async fn put(
        &mut self,
        data: impl futures::Stream<Item = Result<arrow_flight::FlightData, FlightError>>
        + Send
        + 'static,
    ) -> Result<Vec<Vec<String>>, Status> {
        let results: Vec<_> = self
            .flight
            .do_put(data)
            .await
            .map_err(status_of)?
            .try_collect()
            .await
            .map_err(status_of)?;
        Ok(results
            .iter()
            .map(|result| {
                let reply = read_ipc(&result.app_metadata);
                reply
                    .column_by_name("outcome")
                    .unwrap()
                    .as_string::<i32>()
                    .iter()
                    .map(|outcome| outcome.unwrap().to_owned())
                    .collect()
            })
            .collect())
    }

    /// Publishes `rows` as one batch, returning their outcomes.
    pub(crate) async fn publish(
        &mut self,
        publisher_id: &str,
        rows: &[Row<'_>],
    ) -> Result<Vec<String>, Status> {
        let mut outcomes = self.publish_batches(publisher_id, &[rows]).await?;
        assert_eq!(outcomes.len(), 1, "one PutResult per batch");
        Ok(outcomes.remove(0))
    }

    /// Pulls up to `limit` records for `consumer_id`.
    pub(crate) async fn pull(&mut self, consumer_id: &str, limit: i64) -> Result<Pulled, Status> {
        let ticket = body(vec![
            ("consumer_id", utf8(consumer_id)),
            ("limit", Arc::new(Int64Array::from(vec![limit]))),
        ]);
        let mut stream = self
            .flight
            .do_get(Ticket::new(ticket))
            .await
            .map_err(status_of)?;
        let mut records = Vec::new();
        while let Some(batch) = stream.next().await {
            records.extend(records_of(&batch.map_err(status_of)?));
        }
        let schema = stream
            .schema()
            .expect("a pull's reply always carries its schema");
        let metadata = schema.metadata();
        Ok(Pulled {
            outcome: metadata.get("outcome").unwrap().clone(),
            lease_id: metadata.get("lease_id").map(|id| id.parse().unwrap()),
            pocket_key: metadata.get("pocket_key").cloned(),
            expires_at: metadata.get("expires_at").map(|at| at.parse().unwrap()),
            records,
        })
    }

    /// Acks `record_key` under `lease_id`, returning the outcome.
    pub(crate) async fn ack(
        &mut self,
        consumer_id: &str,
        lease_id: u64,
        record_key: &str,
    ) -> Result<String, Status> {
        let reply = self
            .action("ack", settle_body(consumer_id, lease_id, record_key))
            .await?;
        Ok(string_cell(&reply, "outcome"))
    }

    /// Nacks `record_key` under `lease_id`, returning the outcome.
    pub(crate) async fn nack(
        &mut self,
        consumer_id: &str,
        lease_id: u64,
        record_key: &str,
    ) -> Result<String, Status> {
        let reply = self
            .action("nack", settle_body(consumer_id, lease_id, record_key))
            .await?;
        Ok(string_cell(&reply, "outcome"))
    }

    /// Renews `lease_id`, returning the new expiry.
    pub(crate) async fn renew(&mut self, consumer_id: &str, lease_id: u64) -> Result<u64, Status> {
        let reply = self
            .action("renew", lease_body(consumer_id, lease_id))
            .await?;
        Ok(u64_cell(&reply, "expires_at").unwrap())
    }

    /// Releases `lease_id`, returning the ended lease's id and end.
    pub(crate) async fn release(
        &mut self,
        consumer_id: &str,
        lease_id: u64,
    ) -> Result<(u64, Option<u64>), Status> {
        let reply = self
            .action("release", lease_body(consumer_id, lease_id))
            .await?;
        Ok((
            u64_cell(&reply, "lease_id").unwrap(),
            u64_cell(&reply, "ended_at"),
        ))
    }

    /// `status` for `consumer_id`.
    pub(crate) async fn status(&mut self, consumer_id: &str) -> Result<LeaseStatus, Status> {
        let reply = self
            .action("status", body(vec![("consumer_id", utf8(consumer_id))]))
            .await?;
        let schema = reply.schema();
        let metadata = schema.metadata();
        Ok(LeaseStatus {
            lease: metadata.get("lease").unwrap().clone(),
            lease_id: metadata.get("lease_id").map(|id| id.parse().unwrap()),
            pocket_key: metadata.get("pocket_key").cloned(),
            expires_at: metadata.get("expires_at").map(|at| at.parse().unwrap()),
            held: records_of(&reply),
        })
    }
}

/// The tonic status a Flight call failed with.
pub(crate) fn status_of(error: FlightError) -> Status {
    match error {
        FlightError::Tonic(status) => *status,
        other => panic!("not a gRPC status: {other}"),
    }
}

/// A one-element `Utf8` column.
pub(crate) fn utf8(value: &str) -> ArrayRef {
    Arc::new(StringArray::from(vec![value]))
}

/// A one-element `UInt64` column.
pub(crate) fn uint64(value: u64) -> ArrayRef {
    Arc::new(UInt64Array::from(vec![value]))
}

/// `columns` as a one-batch IPC stream.
pub(crate) fn body(columns: Vec<(&str, ArrayRef)>) -> Bytes {
    let batch = RecordBatch::try_from_iter(columns).unwrap();
    let mut writer = StreamWriter::try_new(Vec::new(), &batch.schema()).unwrap();
    writer.write(&batch).unwrap();
    Bytes::from(writer.into_inner().unwrap())
}

/// An ack's or nack's body.
fn settle_body(consumer_id: &str, lease_id: u64, record_key: &str) -> Bytes {
    body(vec![
        ("consumer_id", utf8(consumer_id)),
        ("lease_id", uint64(lease_id)),
        ("record_key", utf8(record_key)),
    ])
}

/// A renew's or release's body.
fn lease_body(consumer_id: &str, lease_id: u64) -> Bytes {
    body(vec![
        ("consumer_id", utf8(consumer_id)),
        ("lease_id", uint64(lease_id)),
    ])
}

/// `rows` as one publish batch.
pub(crate) fn publish_batch(rows: &[Row<'_>]) -> RecordBatch {
    let pocket_keys: ArrayRef =
        Arc::new(StringArray::from_iter_values(rows.iter().map(|row| row.0)));
    let record_keys: ArrayRef =
        Arc::new(StringArray::from_iter_values(rows.iter().map(|row| row.1)));
    let ranks: ArrayRef = Arc::new(Int64Array::from_iter_values(rows.iter().map(|row| row.2)));
    let payloads: ArrayRef = Arc::new(BinaryArray::from_iter_values(rows.iter().map(|row| row.3)));
    RecordBatch::try_from_iter([
        ("pocket_key", pocket_keys),
        ("record_key", record_keys),
        ("rank", ranks),
        ("payload", payloads),
    ])
    .unwrap()
}

/// The one batch an IPC reply holds.
fn read_ipc(bytes: &[u8]) -> RecordBatch {
    let mut reader = StreamReader::try_new(bytes, None).unwrap();
    let batch = reader.next().unwrap().unwrap();
    assert!(reader.next().is_none(), "one batch per reply");
    batch
}

/// A batch of `record_key`, `rank` and `payload` as records.
fn records_of(batch: &RecordBatch) -> Vec<Record> {
    let keys = batch
        .column_by_name("record_key")
        .unwrap()
        .as_string::<i32>();
    let ranks = batch
        .column_by_name("rank")
        .unwrap()
        .as_primitive::<Int64Type>();
    let payloads = batch.column_by_name("payload").unwrap().as_binary::<i32>();
    (0..batch.num_rows())
        .map(|row| {
            (
                keys.value(row).to_owned(),
                ranks.value(row),
                payloads.value(row).to_vec(),
            )
        })
        .collect()
}

/// Column `name`'s first value, as a string.
fn string_cell(batch: &RecordBatch, name: &str) -> String {
    batch
        .column_by_name(name)
        .unwrap()
        .as_string::<i32>()
        .value(0)
        .to_owned()
}

/// Column `name`'s first value, or `None` when it is null.
fn u64_cell(batch: &RecordBatch, name: &str) -> Option<u64> {
    batch
        .column_by_name(name)
        .unwrap()
        .as_primitive::<UInt64Type>()
        .iter()
        .next()
        .unwrap()
}
