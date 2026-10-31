//! The wire's Arrow IPC: reading an action body, a pull ticket and the publish
//! batches, and writing every reply.
//!
//! A body or ticket is an IPC stream of one batch with one row and nothing after
//! it, whose columns are read by name and type, so a second batch, trailing
//! bytes, a missing column, a wrong type or a null is `INVALID_ARGUMENT`, never a
//! panic. A reply that cannot be written is
//! `INTERNAL`: its schema is this module's, so that is a programming error.

use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{Int64Type, UInt64Type};
use arrow_array::{
    Array, ArrayRef, Int64Array, LargeBinaryArray, LargeStringArray, RecordBatch, StringArray,
    UInt64Array,
};
use arrow_flight::decode::{DecodedFlightData, DecodedPayload};
use arrow_flight::flight_descriptor::DescriptorType;
use arrow_flight::{FlightDescriptor, PutResult};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;
use arrow_schema::{ArrowError, Metadata, Schema, SchemaRef};
use bytes::Bytes;
use libpachinko::clock::Timestamp;
use libpachinko::queue::{
    Lease, LeaseId, LeaseView, Publication, PublishOutcome, PullOutcome, PulledBatch, SettleOutcome,
};
use libpachinko::records::HeldRecord;
use tonic::Status;

use crate::actor::RowOutcome;
use crate::arrow::batch_schema;

/// The consumer's id, `Utf8`.
pub(super) const CONSUMER_ID: &str = "consumer_id";
/// The publisher's id, `Utf8`.
pub(super) const PUBLISHER_ID: &str = "publisher_id";
/// The lease id, `UInt64`.
pub(super) const LEASE_ID: &str = "lease_id";
/// The record key, `Utf8`.
pub(super) const RECORD_KEY: &str = "record_key";
/// A pull's limit, `Int64`.
const LIMIT: &str = "limit";
/// A publish row's pocket key, `Utf8`.
const POCKET_KEY: &str = "pocket_key";
/// A publish row's rank, `Int64`.
const RANK: &str = "rank";
/// A publish row's payload, `Binary`.
const PAYLOAD: &str = "payload";
/// A reply's outcome, `Utf8` in a batch and a key in a schema's metadata.
const OUTCOME: &str = "outcome";
/// When a lease lapses, `UInt64` milliseconds in a batch, decimal in metadata.
const EXPIRES_AT: &str = "expires_at";
/// When a lease ended, `UInt64` milliseconds, null while it has not.
const ENDED_AT: &str = "ended_at";
/// The `status` reply's metadata key: `active` or `none`.
const LEASE: &str = "lease";

/// The one-row batch an IPC body holds.
pub(super) fn read_one_row(body: &[u8]) -> Result<RecordBatch, Status> {
    let mut reader = StreamReader::try_new(body, None).map_err(|error| invalid(&error))?;
    let batch = reader
        .next()
        .ok_or_else(|| Status::invalid_argument("the body holds no batch"))?
        .map_err(|error| invalid(&error))?;
    end_of_body(reader)?;
    exactly_one_row(batch)
}

/// `Ok` when the stream ends after the batch `reader` has read, and the body
/// holds no bytes after its end of stream.
fn end_of_body(mut reader: StreamReader<&[u8]>) -> Result<(), Status> {
    match reader.next() {
        None if reader.get_ref().is_empty() => Ok(()),
        None => Err(Status::invalid_argument(
            "the body holds bytes after its end of stream",
        )),
        Some(Ok(_)) => Err(Status::invalid_argument("the body must hold one batch")),
        Some(Err(error)) => Err(invalid(&error)),
    }
}

/// `batch` when it has one row.
fn exactly_one_row(batch: RecordBatch) -> Result<RecordBatch, Status> {
    if batch.num_rows() == 1 {
        Ok(batch)
    } else {
        Err(Status::invalid_argument(format!(
            "the body must hold one row, not {}",
            batch.num_rows()
        )))
    }
}

/// The non-null `Utf8` value of column `name` in a one-row batch.
pub(super) fn utf8_value(batch: &RecordBatch, name: &str) -> Result<String, Status> {
    utf8_column(batch, name)?
        .iter()
        .next()
        .flatten()
        .map(str::to_owned)
        .ok_or_else(|| wrong_column(name, "Utf8"))
}

/// The non-null `UInt64` value of column `name` in a one-row batch, as a lease id.
pub(super) fn lease_value(batch: &RecordBatch) -> Result<LeaseId, Status> {
    batch
        .column_by_name(LEASE_ID)
        .and_then(|column| column.as_primitive_opt::<UInt64Type>())
        .and_then(|column| column.iter().next().flatten())
        .map(LeaseId::from_raw)
        .ok_or_else(|| wrong_column(LEASE_ID, "UInt64"))
}

/// The consumer and the limit a pull's ticket names.
pub(super) fn pull_ticket(ticket: &[u8]) -> Result<(String, i64), Status> {
    let batch = read_one_row(ticket)?;
    let consumer_id = utf8_value(&batch, CONSUMER_ID)?;
    let limit = batch
        .column_by_name(LIMIT)
        .and_then(|column| column.as_primitive_opt::<Int64Type>())
        .and_then(|column| column.iter().next().flatten())
        .ok_or_else(|| wrong_column(LIMIT, "Int64"))?;
    Ok((consumer_id, limit))
}

/// The publisher a put's descriptor names, and each input batch's publications.
pub(super) fn put_input(
    decoded: &[DecodedFlightData],
) -> Result<(String, Vec<Vec<Publication>>), Status> {
    let publisher_id = decoded
        .first()
        .and_then(|data| data.inner.flight_descriptor.as_ref())
        .ok_or_else(|| Status::invalid_argument("a put needs a descriptor naming the publisher"))
        .and_then(publisher_of)?;
    let batches = decoded
        .iter()
        .filter_map(|data| match &data.payload {
            DecodedPayload::RecordBatch(batch) => Some(batch),
            DecodedPayload::None | DecodedPayload::Schema(_) => None,
        })
        .map(publications)
        .collect::<Result<_, _>>()?;
    Ok((publisher_id, batches))
}

/// The publisher id a command descriptor carries, in UTF-8.
fn publisher_of(descriptor: &FlightDescriptor) -> Result<String, Status> {
    if descriptor.r#type() == DescriptorType::Cmd {
        String::from_utf8(descriptor.cmd.to_vec())
            .map_err(|_| Status::invalid_argument("the publisher id must be UTF-8"))
    } else {
        Err(Status::invalid_argument(
            "the descriptor must be a command naming the publisher",
        ))
    }
}

/// One input batch's rows as publications, in row order.
fn publications(batch: &RecordBatch) -> Result<Vec<Publication>, Status> {
    let pocket_keys = utf8_column(batch, POCKET_KEY)?;
    let record_keys = utf8_column(batch, RECORD_KEY)?;
    let ranks = batch
        .column_by_name(RANK)
        .and_then(|column| column.as_primitive_opt::<Int64Type>())
        .filter(|column| column.null_count() == 0)
        .ok_or_else(|| wrong_column(RANK, "Int64"))?;
    let payloads = batch
        .column_by_name(PAYLOAD)
        .and_then(|column| column.as_binary_opt::<i32>())
        .filter(|column| column.null_count() == 0)
        .ok_or_else(|| wrong_column(PAYLOAD, "Binary"))?;
    Ok((0..batch.num_rows())
        .map(|row| {
            Publication::new(
                pocket_keys.value(row),
                record_keys.value(row),
                ranks.value(row),
                payloads.value(row).to_vec(),
            )
        })
        .collect())
}

/// Column `name` of `batch`, when it is `Utf8` with no nulls.
fn utf8_column<'batch>(
    batch: &'batch RecordBatch,
    name: &str,
) -> Result<&'batch StringArray, Status> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_string_opt::<i32>())
        .filter(|column| column.null_count() == 0)
        .ok_or_else(|| wrong_column(name, "Utf8"))
}

/// One input batch's `PutResult`: one `outcome` row per input row.
pub(super) fn put_result(outcomes: &[RowOutcome]) -> Result<PutResult, Status> {
    let names: StringArray = outcomes
        .iter()
        .map(|outcome| Some(outcome_name(*outcome)))
        .collect();
    let batch = one_batch(vec![(OUTCOME, Arc::new(names))])?;
    write_stream(&batch).map(|app_metadata| PutResult { app_metadata })
}

/// A publish row's outcome on the wire: `new`, `superseded`, `stale` or
/// `rejected` for the equal-rank placeholder.
const fn outcome_name(outcome: RowOutcome) -> &'static str {
    match outcome {
        Some(PublishOutcome::New) => "new",
        Some(PublishOutcome::Superseded) => "superseded",
        Some(PublishOutcome::Stale) => "stale",
        Some(_) => "unknown",
        None => "rejected",
    }
}

/// A pull's reply: the schema, with the outcome and, for a batch, the lease in
/// its metadata, and the records when there is a batch.
pub(super) fn pull_reply(
    outcome: PullOutcome<RecordBatch>,
) -> Result<(SchemaRef, Option<RecordBatch>), Status> {
    match outcome {
        PullOutcome::Batch(batch) => pulled_batch_reply(batch),
        PullOutcome::NoPocketAvailable => Ok((no_pocket_schema(), None)),
        _ => Err(Status::internal(
            "the queue returned a pull outcome this service does not know",
        )),
    }
}

/// A pull's reply schema when no pocket was available: `outcome` = `no_pocket`.
fn no_pocket_schema() -> SchemaRef {
    schema_with(&batch_schema(), &[(OUTCOME, "no_pocket".to_owned())])
}

/// A pulled batch's reply: its records, under a schema that names the lease.
fn pulled_batch_reply(
    batch: PulledBatch<RecordBatch>,
) -> Result<(SchemaRef, Option<RecordBatch>), Status> {
    let metadata = [
        (OUTCOME, "batch".to_owned()),
        (LEASE_ID, batch.lease.as_raw().to_string()),
        (POCKET_KEY, batch.pocket_key),
        (EXPIRES_AT, batch.expires_at.as_millis().to_string()),
    ];
    let schema = schema_with(&batch_schema(), &metadata);
    let records = batch
        .records
        .with_schema(Arc::clone(&schema))
        .map_err(|error| internal(&error))?;
    Ok((schema, Some(records)))
}

/// The reply to a join: `outcome` = `joined`.
pub(super) fn joined_reply() -> Result<Bytes, Status> {
    outcome_reply("joined")
}

/// The reply to an ack or nack: `outcome` = `settled` or `deferred`.
pub(super) fn settle_reply(outcome: SettleOutcome) -> Result<Bytes, Status> {
    outcome_reply(settle_name(outcome))
}

/// A settle's outcome on the wire.
const fn settle_name(outcome: SettleOutcome) -> &'static str {
    match outcome {
        SettleOutcome::Settled => "settled",
        SettleOutcome::Deferred => "deferred",
        _ => "unknown",
    }
}

/// A one-row reply of `outcome`.
fn outcome_reply(outcome: &str) -> Result<Bytes, Status> {
    let column: ArrayRef = Arc::new(StringArray::from(vec![outcome]));
    write_stream(&one_batch(vec![(OUTCOME, column)])?)
}

/// The reply to a renew: the lease's new `expires_at`.
pub(super) fn renew_reply(expires_at: Timestamp) -> Result<Bytes, Status> {
    let column: ArrayRef = Arc::new(UInt64Array::from(vec![expires_at.as_millis()]));
    write_stream(&one_batch(vec![(EXPIRES_AT, column)])?)
}

/// The reply to a release: the lease's id and when it ended.
pub(super) fn release_reply(lease: &Lease) -> Result<Bytes, Status> {
    let id: ArrayRef = Arc::new(UInt64Array::from(vec![lease.id.as_raw()]));
    let ended_at = lease.ended_at.map(Timestamp::as_millis);
    let ended: ArrayRef = Arc::new(UInt64Array::from(vec![ended_at]));
    write_stream(&one_batch(vec![(LEASE_ID, id), (ENDED_AT, ended)])?)
}

/// The reply to `status`: the `Consuming` surface's `exposes`. The schema's
/// metadata says whether a lease is active and, when it is, its id, pocket key and
/// expiry; the rows are its held records' `record_key`, `rank` and `payload`.
pub(super) fn status_reply(view: Option<LeaseView>) -> Result<Bytes, Status> {
    let (metadata, held) = view.map_or_else(
        || (vec![(LEASE, "none".to_owned())], Vec::new()),
        |view| {
            let metadata = vec![
                (LEASE, "active".to_owned()),
                (LEASE_ID, view.lease.as_raw().to_string()),
                (POCKET_KEY, view.pocket_key),
                (EXPIRES_AT, view.expires_at.as_millis().to_string()),
            ];
            (metadata, view.held)
        },
    );
    let schema = schema_with(&batch_schema(), &metadata);
    write_stream(&held_batch(schema, &held)?)
}

/// `held` as a batch of `schema`.
fn held_batch(schema: SchemaRef, held: &[HeldRecord]) -> Result<RecordBatch, Status> {
    let keys: ArrayRef = Arc::new(LargeStringArray::from_iter_values(
        held.iter().map(|record| record.key.as_str()),
    ));
    let ranks: ArrayRef = Arc::new(Int64Array::from_iter_values(
        held.iter().map(|record| record.rank),
    ));
    let payloads: ArrayRef = Arc::new(LargeBinaryArray::from_iter_values(
        held.iter().map(|record| record.payload.as_slice()),
    ));
    RecordBatch::try_new(schema, vec![keys, ranks, payloads]).map_err(|error| internal(&error))
}

/// `schema` with `metadata`.
fn schema_with(schema: &Schema, metadata: &[(&str, String)]) -> SchemaRef {
    let metadata: Metadata = metadata
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    Arc::new(Schema::new(schema.fields().clone()).with_metadata(metadata))
}

/// A batch of `columns`.
fn one_batch(columns: Vec<(&str, ArrayRef)>) -> Result<RecordBatch, Status> {
    RecordBatch::try_from_iter(columns).map_err(|error| internal(&error))
}

/// `batch` as an IPC stream.
fn write_stream(batch: &RecordBatch) -> Result<Bytes, Status> {
    let mut writer =
        StreamWriter::try_new(Vec::new(), &batch.schema()).map_err(|error| internal(&error))?;
    writer.write(batch).map_err(|error| internal(&error))?;
    writer
        .into_inner()
        .map(Bytes::from)
        .map_err(|error| internal(&error))
}

/// `INVALID_ARGUMENT`, for a column that is missing, of another type, or null.
fn wrong_column(name: &str, data_type: &str) -> Status {
    Status::invalid_argument(format!(
        "the body needs a non-null {data_type} column {name}"
    ))
}

/// `INVALID_ARGUMENT`, for bytes that are not an IPC stream.
fn invalid(error: &ArrowError) -> Status {
    Status::invalid_argument(format!("the body is not an Arrow IPC stream: {error}"))
}

/// `INTERNAL`, for a reply this module could not build.
fn internal(error: &ArrowError) -> Status {
    Status::internal(format!("cannot write the reply: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        lease_value, outcome_name, pull_reply, pull_ticket, put_input, put_result, read_one_row,
        release_reply, renew_reply, settle_reply, status_reply, utf8_value, write_stream,
    };
    use arrow_array::cast::AsArray;
    use arrow_array::types::UInt64Type;
    use arrow_array::{ArrayRef, BinaryArray, Int64Array, RecordBatch, StringArray, UInt64Array};
    use arrow_flight::decode::DecodedFlightData;
    use arrow_flight::{FlightData, FlightDescriptor};
    use arrow_ipc::reader::StreamReader;
    use arrow_ipc::writer::StreamWriter;
    use arrow_schema::{DataType, Field, Schema};
    use bytes::Bytes;
    use libpachinko::clock::{FakeClock, Timestamp};
    use libpachinko::config::Config;
    use libpachinko::queue::{
        LeaseId, Publication, PublishOutcome, PullOutcome, Queue, SettleOutcome,
    };
    use libpachinko::records::MemoryRecords;
    use std::sync::Arc;
    use tonic::Code;

    fn batch(columns: Vec<(&str, ArrayRef)>) -> RecordBatch {
        RecordBatch::try_from_iter(columns).unwrap()
    }

    fn ipc(batch: &RecordBatch) -> Bytes {
        write_stream(batch).unwrap()
    }

    fn read(bytes: &[u8]) -> RecordBatch {
        StreamReader::try_new(bytes, None)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
    }

    fn strings(values: &[&str]) -> ArrayRef {
        Arc::new(StringArray::from(values.to_vec()))
    }

    fn int64s(values: Vec<Option<i64>>) -> ArrayRef {
        Arc::new(Int64Array::from(values))
    }

    fn outcome(bytes: &[u8]) -> String {
        let batch = read(bytes);
        batch
            .column_by_name("outcome")
            .unwrap()
            .as_string::<i32>()
            .value(0)
            .to_owned()
    }

    fn assert_invalid<T: std::fmt::Debug>(result: Result<T, tonic::Status>, message: &str) {
        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::InvalidArgument);
        assert!(status.message().contains(message), "{}", status.message());
    }

    /// The decoded messages of a put: the descriptor on the first, then `batches`.
    fn put(
        descriptor: Option<FlightDescriptor>,
        batches: Vec<RecordBatch>,
    ) -> Vec<DecodedFlightData> {
        let mut first = FlightData::new();
        first.flight_descriptor = descriptor;
        let schema = batches
            .first()
            .map_or_else(|| Arc::new(Schema::empty()), RecordBatch::schema);
        std::iter::once(DecodedFlightData::new_schema(first, schema))
            .chain(
                batches
                    .into_iter()
                    .map(|batch| DecodedFlightData::new_record_batch(FlightData::new(), batch)),
            )
            .collect()
    }

    /// `batch` with column `name` replaced by `replacement`.
    fn replaced(batch: &RecordBatch, name: &str, replacement: &ArrayRef) -> RecordBatch {
        let columns: Vec<(&str, ArrayRef)> = batch
            .schema_ref()
            .fields()
            .iter()
            .zip(batch.columns())
            .map(|(field, column)| {
                let kept = if field.name() == name {
                    replacement
                } else {
                    column
                };
                (field.name().as_str(), Arc::clone(kept))
            })
            .collect();
        RecordBatch::try_from_iter(columns).unwrap()
    }

    fn rows() -> RecordBatch {
        batch(vec![
            ("pocket_key", strings(&["pk", "pk"])),
            ("record_key", strings(&["a", "b"])),
            ("rank", Arc::new(Int64Array::from(vec![1, 2]))),
            ("payload", Arc::new(BinaryArray::from_vec(vec![b"x", b"y"]))),
        ])
    }

    #[test]
    fn a_body_is_one_row_of_an_ipc_stream() {
        let body = ipc(&batch(vec![("consumer_id", strings(&["c"]))]));
        let request = read_one_row(&body).unwrap();
        assert_eq!(utf8_value(&request, "consumer_id").unwrap(), "c");
    }

    #[test]
    fn bytes_that_are_not_ipc_are_an_invalid_argument() {
        assert_invalid(read_one_row(b"not ipc"), "not an Arrow IPC stream");
    }

    #[test]
    fn a_stream_without_a_batch_is_an_invalid_argument() {
        let schema = Schema::new(vec![Field::new("consumer_id", DataType::Utf8, false)]);
        let writer = StreamWriter::try_new(Vec::new(), &schema).unwrap();
        let body = writer.into_inner().unwrap();
        assert_invalid(read_one_row(&body), "holds no batch");
    }

    #[test]
    fn a_body_of_two_rows_is_an_invalid_argument() {
        let body = ipc(&batch(vec![("consumer_id", strings(&["c", "d"]))]));
        assert_invalid(read_one_row(&body), "one row, not 2");
    }

    #[test]
    fn a_second_batch_or_trailing_bytes_are_an_invalid_argument() {
        let request = batch(vec![("consumer_id", strings(&["c"]))]);
        let mut writer = StreamWriter::try_new(Vec::new(), &request.schema()).unwrap();
        writer.write(&request).unwrap();
        writer.write(&request).unwrap();
        let two_batches = writer.into_inner().unwrap();
        assert_invalid(read_one_row(&two_batches), "must hold one batch");

        let complete = ipc(&request);
        let mut malformed = complete[..complete.len() - 8].to_vec();
        malformed.extend_from_slice(b"not ipc");
        assert_invalid(read_one_row(&malformed), "not an Arrow IPC stream");

        let mut trailing = complete.to_vec();
        trailing.extend_from_slice(b"not ipc");
        assert_invalid(read_one_row(&trailing), "bytes after its end of stream");

        let ticket = batch(vec![
            ("consumer_id", strings(&["c"])),
            ("limit", int64s(vec![Some(1)])),
        ]);
        let mut writer = StreamWriter::try_new(Vec::new(), &ticket.schema()).unwrap();
        writer.write(&ticket).unwrap();
        writer.write(&ticket).unwrap();
        assert_invalid(
            pull_ticket(&writer.into_inner().unwrap()),
            "must hold one batch",
        );
    }

    #[test]
    fn a_missing_mistyped_or_null_column_is_an_invalid_argument() {
        let request = batch(vec![
            ("consumer_id", int64s(vec![Some(1)])),
            (
                "record_key",
                Arc::new(StringArray::from(vec![None::<&str>])),
            ),
            ("lease_id", int64s(vec![Some(1)])),
        ]);
        assert_invalid(
            utf8_value(&request, "consumer_id"),
            "Utf8 column consumer_id",
        );
        assert_invalid(utf8_value(&request, "record_key"), "Utf8 column record_key");
        assert_invalid(
            utf8_value(&request, "publisher_id"),
            "Utf8 column publisher_id",
        );
        assert_invalid(lease_value(&request), "UInt64 column lease_id");
    }

    #[test]
    fn a_lease_id_is_read_as_the_fencing_token() {
        let lease_id: ArrayRef = Arc::new(UInt64Array::from(vec![9]));
        let request = batch(vec![("lease_id", lease_id)]);
        assert_eq!(lease_value(&request).unwrap(), LeaseId::from_raw(9));
    }

    #[test]
    fn a_ticket_names_the_consumer_and_the_limit() {
        let ticket = ipc(&batch(vec![
            ("consumer_id", strings(&["c"])),
            ("limit", Arc::new(Int64Array::from(vec![-4]))),
        ]));
        assert_eq!(pull_ticket(&ticket).unwrap(), ("c".to_owned(), -4));
        let without_limit = ipc(&batch(vec![("consumer_id", strings(&["c"]))]));
        assert_invalid(pull_ticket(&without_limit), "Int64 column limit");
        let without_consumer = ipc(&batch(vec![("limit", int64s(vec![Some(1)]))]));
        assert_invalid(pull_ticket(&without_consumer), "Utf8 column consumer_id");
    }

    #[test]
    fn a_put_reads_its_publisher_and_each_batch_in_row_order() {
        let decoded = put(Some(FlightDescriptor::new_cmd("p")), vec![rows(), rows()]);
        let (publisher_id, batches) = put_input(&decoded).unwrap();
        assert_eq!(publisher_id, "p");
        assert_eq!(batches.len(), 2);
        assert_eq!(
            batches[0],
            [
                Publication::new("pk", "a", 1, b"x".to_vec()),
                Publication::new("pk", "b", 2, b"y".to_vec())
            ]
        );
    }

    #[test]
    fn a_put_without_a_command_naming_the_publisher_is_an_invalid_argument() {
        assert_invalid(
            put_input(&put(None, vec![rows()])),
            "descriptor naming the publisher",
        );
        assert_invalid(put_input(&[]), "descriptor naming the publisher");
        let path = FlightDescriptor::new_path(vec!["p".to_owned()]);
        assert_invalid(
            put_input(&put(Some(path), vec![rows()])),
            "must be a command",
        );
        let not_utf8 = FlightDescriptor::new_cmd(vec![0xff, 0xfe]);
        assert_invalid(put_input(&put(Some(not_utf8), vec![rows()])), "UTF-8");
    }

    #[test]
    fn a_put_batch_with_a_missing_mistyped_or_null_column_is_an_invalid_argument() {
        let good = rows();
        for (name, replacement, expected) in [
            (
                "pocket_key",
                int64s(vec![Some(1), Some(2)]),
                "Utf8 column pocket_key",
            ),
            (
                "record_key",
                Arc::new(StringArray::from(vec![Some("a"), None])),
                "Utf8 column record_key",
            ),
            ("rank", int64s(vec![Some(1), None]), "Int64 column rank"),
            ("payload", strings(&["x", "y"]), "Binary column payload"),
        ] {
            let broken = replaced(&good, name, &replacement);
            let decoded = put(Some(FlightDescriptor::new_cmd("p")), vec![broken]);
            assert_invalid(put_input(&decoded), expected);
        }
    }

    #[test]
    fn each_publish_outcome_has_its_name() {
        assert_eq!(outcome_name(Some(PublishOutcome::New)), "new");
        assert_eq!(outcome_name(Some(PublishOutcome::Superseded)), "superseded");
        assert_eq!(outcome_name(Some(PublishOutcome::Stale)), "stale");
        assert_eq!(outcome_name(None), "rejected");
        let result = put_result(&[Some(PublishOutcome::New), None]).unwrap();
        let reply = read(&result.app_metadata);
        let names: Vec<&str> = reply
            .column(0)
            .as_string::<i32>()
            .iter()
            .flatten()
            .collect();
        assert_eq!(names, ["new", "rejected"]);
    }

    #[test]
    fn a_settle_reply_names_its_outcome() {
        assert_eq!(
            outcome(&settle_reply(SettleOutcome::Settled).unwrap()),
            "settled"
        );
        assert_eq!(
            outcome(&settle_reply(SettleOutcome::Deferred).unwrap()),
            "deferred"
        );
    }

    #[test]
    fn the_lease_replies_carry_their_lease() {
        let clock = FakeClock::new(Timestamp::from_millis(7));
        let mut queue: Queue<MemoryRecords, _> = Queue::new(Config::default(), clock).unwrap();
        queue.publisher_joins("p").unwrap();
        queue.consumer_joins("c").unwrap();
        queue
            .publish("p", Publication::new("pk", "rk", 3, b"v".to_vec()))
            .unwrap();
        let PullOutcome::Batch(pulled) = queue.pull("c", 10).unwrap() else {
            panic!("no batch")
        };

        let renewed = read(&renew_reply(Timestamp::from_millis(42)).unwrap());
        assert_eq!(renewed.column(0).as_primitive::<UInt64Type>().value(0), 42);

        let status = read(&status_reply(queue.lease_view("c").unwrap()).unwrap());
        let metadata = status.schema().metadata().clone();
        assert_eq!(metadata.get("lease").unwrap(), "active");
        assert_eq!(metadata.get("pocket_key").unwrap(), "pk");
        assert_eq!(
            metadata.get("lease_id").unwrap(),
            &pulled.lease.as_raw().to_string()
        );
        assert_eq!(status.num_rows(), 1);
        assert_eq!(
            status
                .column(1)
                .as_primitive::<arrow_array::types::Int64Type>()
                .value(0),
            3
        );

        let ended = queue.release("c", pulled.lease).unwrap();
        let released = read(&release_reply(&ended).unwrap());
        let ended_at = released.column_by_name("ended_at").unwrap();
        assert_eq!(ended_at.as_primitive::<UInt64Type>().value(0), 7);

        let none = read(&status_reply(None).unwrap());
        assert_eq!(none.schema().metadata().get("lease").unwrap(), "none");
        assert_eq!(none.num_rows(), 0);
    }

    #[test]
    fn a_pull_that_finds_no_pocket_is_a_schema_without_records() {
        let (schema, records) = pull_reply(PullOutcome::NoPocketAvailable).unwrap();
        assert_eq!(schema.metadata().get("outcome").unwrap(), "no_pocket");
        assert!(records.is_none());
    }
}
