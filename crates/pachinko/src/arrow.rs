//! `ArrowRecords`: one pocket's records as one Arrow `RecordBatch` (decision 0020).
//!
//! Every step of `PocketRecords` runs the Arrow kernels over the whole batch and
//! rebuilds it: `arrow_ord` compares keys and statuses and sorts by position, and
//! `arrow_select` filters, takes, zips and concatenates. That rebuild is the cost the
//! prototype exists to measure, so no step keeps a side index.
//!
//! Two things are done by hand, because no kernel does them. Turning the row
//! indexes a pull chose into a mask over the batch is a scatter, which Arrow has
//! no kernel for, so `rows_mask` writes it in one loop. And reading rows back out
//! as Rust values, for the held list and the inspection seam, walks the rows.
//!
//! The trait's steps cannot fail, but the kernels return `Result`. The schema is
//! fixed by this module, so a kernel error is a programming error, and `kernel`
//! turns it into a panic in one place.

mod schema;

use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{Int64Type, UInt8Type, UInt32Type, UInt64Type};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Datum, Int64Array, LargeBinaryArray, LargeStringArray,
    RecordBatch, Scalar, UInt8Array, UInt32Array, UInt64Array,
};
use arrow_ord::cmp::{eq, neq};
use arrow_ord::sort::sort_to_indices;
use arrow_schema::{ArrowError, SchemaRef};
use arrow_select::concat::concat_batches;
use arrow_select::filter::{filter, filter_record_batch};
use arrow_select::take::{take, take_record_batch};
use arrow_select::zip::zip;
use libpachinko::records::{
    HeldRecord, PocketRecords, RecordState, RecordStatus, RecordView, Version,
};

use schema::{
    BATCH, DEFERRED, HELD, HELD_PAYLOAD, HELD_RANK, PAYLOAD, POSITION, QUEUED, RANK, RECORD_KEY,
    RECORDS, STATUS,
};

/// One pocket's records as one `RecordBatch` with the columns `record_key`,
/// `rank`, `payload`, `position`, `status`, `held_rank` and `held_payload`. A pull
/// hands out a `RecordBatch` of the held versions' `record_key`, `rank` and
/// `payload`, oldest first.
///
/// ```
/// use libpachinko::records::{PocketRecords, Version};
/// use pachinko::arrow::{ArrowRecords, held_records};
///
/// let mut records = ArrowRecords::default();
/// records.insert_queued("b", Version::new(1, b"two".to_vec()), 2);
/// records.insert_queued("a", Version::new(1, b"one".to_vec()), 1);
/// let batch = records.hold_oldest(1);
/// assert_eq!(batch.num_rows(), 1);
/// assert_eq!(held_records(&batch)[0].key, "a");
/// assert_eq!(records.record_count(), 2);
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ArrowRecords {
    batch: RecordBatch,
}

impl Default for ArrowRecords {
    fn default() -> Self {
        Self {
            batch: RecordBatch::new_empty(Arc::clone(&RECORDS)),
        }
    }
}

/// The schema of the batches a pull hands out: `record_key: LargeUtf8`, `rank: Int64`
/// and `payload: LargeBinary`.
///
/// ```
/// use pachinko::arrow::batch_schema;
///
/// let schema = batch_schema();
/// let names: Vec<&str> = schema.fields().iter().map(|field| field.name().as_str()).collect();
/// assert_eq!(names, ["record_key", "rank", "payload"]);
/// ```
#[must_use]
pub fn batch_schema() -> SchemaRef {
    Arc::clone(&BATCH)
}

/// A pulled batch's rows as held records, in batch order.
///
/// # Panics
///
/// When `batch` does not have the [`batch_schema`] columns in order.
///
/// ```
/// use libpachinko::records::{PocketRecords, Version};
/// use pachinko::arrow::{ArrowRecords, held_records};
///
/// let mut records = ArrowRecords::default();
/// records.insert_queued("k", Version::new(4, b"v".to_vec()), 0);
/// let held = held_records(&records.hold_oldest(10));
/// assert_eq!((held[0].rank, held[0].payload.as_slice()), (4, &b"v"[..]));
/// ```
#[must_use]
pub fn held_records(batch: &RecordBatch) -> Vec<HeldRecord> {
    let keys = batch.column(RECORD_KEY).as_string::<i64>();
    let ranks = batch.column(RANK).as_primitive::<Int64Type>();
    let payloads = batch.column(PAYLOAD).as_binary::<i64>();
    (0..batch.num_rows())
        .map(|row| {
            HeldRecord::new(
                keys.value(row).to_owned(),
                ranks.value(row),
                payloads.value(row).to_vec(),
            )
        })
        .collect()
}

/// Unwraps a kernel's result. The batch's schema is this module's, so a kernel
/// error is a programming error, never an input error.
#[expect(
    clippy::expect_used,
    reason = "the schema is fixed by this module; a kernel error is a programming error"
)]
fn kernel<T>(result: Result<T, ArrowError>) -> T {
    result.expect("an Arrow kernel failed on the records schema")
}

/// Rows whose `column` equals `value`.
fn mask_where(batch: &RecordBatch, column: usize, value: &dyn Datum) -> BooleanArray {
    kernel(eq(batch.column(column), value))
}

/// Rows whose `record_key` is `key`.
fn key_mask(batch: &RecordBatch, key: &str) -> BooleanArray {
    mask_where(
        batch,
        RECORD_KEY,
        &Scalar::new(LargeStringArray::from(vec![key])),
    )
}

/// Rows whose `status` is `status`.
fn status_mask(batch: &RecordBatch, status: u8) -> BooleanArray {
    mask_where(batch, STATUS, &Scalar::new(UInt8Array::from(vec![status])))
}

/// The row holding `key`, if any.
fn row_of(batch: &RecordBatch, key: &str) -> Option<usize> {
    key_mask(batch, key).values().set_indices().next()
}

/// The row holding `key` while it is held, if any.
fn held_row(batch: &RecordBatch, key: &str) -> Option<usize> {
    let statuses = batch.column(STATUS).as_primitive::<UInt8Type>();
    row_of(batch, key).filter(|row| statuses.value(*row) == HELD)
}

/// The status a `status` code stands for.
const fn status_of(code: u8) -> RecordStatus {
    match code {
        HELD => RecordStatus::Held,
        DEFERRED => RecordStatus::Deferred,
        _ => RecordStatus::Queued,
    }
}

/// The record at `row` as the rules see it.
fn state_at(batch: &RecordBatch, row: usize) -> RecordState {
    let held_ranks = batch.column(HELD_RANK).as_primitive::<Int64Type>();
    RecordState::new(
        batch.column(RANK).as_primitive::<Int64Type>().value(row),
        status_of(batch.column(STATUS).as_primitive::<UInt8Type>().value(row)),
        held_ranks.is_valid(row).then(|| held_ranks.value(row)),
    )
}

/// The latest version at `row`.
fn latest_at(batch: &RecordBatch, row: usize) -> Version {
    Version::new(
        batch.column(RANK).as_primitive::<Int64Type>().value(row),
        batch.column(PAYLOAD).as_binary::<i64>().value(row).to_vec(),
    )
}

/// The held version at `row`, if the row is held.
fn held_at(batch: &RecordBatch, row: usize) -> Option<Version> {
    let ranks = batch.column(HELD_RANK).as_primitive::<Int64Type>();
    let payloads = batch.column(HELD_PAYLOAD).as_binary::<i64>();
    ranks
        .is_valid(row)
        .then(|| Version::new(ranks.value(row), payloads.value(row).to_vec()))
}

/// `batch`'s columns with each `(index, array)` replaced.
fn replace_columns<const N: usize>(
    batch: &RecordBatch,
    replaced: [(usize, ArrayRef); N],
) -> Vec<ArrayRef> {
    let mut columns = batch.columns().to_vec();
    for (index, array) in replaced {
        columns[index] = array;
    }
    columns
}

/// `batch` with each `(index, array)` column replaced.
fn with_columns<const N: usize>(
    batch: &RecordBatch,
    replaced: [(usize, ArrayRef); N],
) -> RecordBatch {
    let columns = replace_columns(batch, replaced);
    kernel(RecordBatch::try_new(Arc::clone(&RECORDS), columns))
}

/// Where `mask` is true, `truthy`; elsewhere `batch`'s `column`.
fn zip_column(
    batch: &RecordBatch,
    mask: &BooleanArray,
    truthy: &dyn Datum,
    column: usize,
) -> ArrayRef {
    kernel(zip(mask, truthy, batch.column(column)))
}

/// A one-element scalar of `status`.
fn status_scalar(status: u8) -> Scalar<UInt8Array> {
    Scalar::new(UInt8Array::from(vec![status]))
}

/// A null `held_rank` and a null `held_payload`, as scalars.
fn no_held_version() -> (Scalar<Int64Array>, Scalar<LargeBinaryArray>) {
    (
        Scalar::new(Int64Array::new_null(1)),
        Scalar::new(LargeBinaryArray::new_null(1)),
    )
}

/// The rows of the `limit` queued records with the smallest positions, in
/// position order.
fn oldest_queued(batch: &RecordBatch, limit: usize) -> UInt32Array {
    let queued = status_mask(batch, QUEUED);
    let rows = u32::try_from(batch.num_rows()).unwrap_or(u32::MAX);
    let indexes = kernel(filter(&UInt32Array::from_iter_values(0..rows), &queued));
    let positions = kernel(filter(batch.column(POSITION), &queued));
    let order = kernel(sort_to_indices(&positions, None, Some(limit)));
    kernel(take(&indexes, &order, None))
        .as_primitive::<UInt32Type>()
        .clone()
}

/// A mask over `len` rows, true at each of `rows`: the scatter no kernel does.
fn rows_mask(len: usize, rows: &UInt32Array) -> BooleanArray {
    let mut mask = vec![false; len];
    for row in rows.values() {
        if let Some(slot) = usize::try_from(*row).ok().and_then(|row| mask.get_mut(row)) {
            *slot = true;
        }
    }
    BooleanArray::from(mask)
}

/// `batch` with the rows under `mask` held: status held, and the latest version
/// copied into the held columns.
fn hold_rows(batch: &RecordBatch, mask: &BooleanArray) -> RecordBatch {
    let status = zip_column(batch, mask, &status_scalar(HELD), STATUS);
    let held_rank = zip_column(batch, mask, batch.column(RANK), HELD_RANK);
    let held_payload = zip_column(batch, mask, batch.column(PAYLOAD), HELD_PAYLOAD);
    with_columns(
        batch,
        [
            (STATUS, status),
            (HELD_RANK, held_rank),
            (HELD_PAYLOAD, held_payload),
        ],
    )
}

/// `batch` with the rows under `mask` at `status` and without a held version.
fn release_rows(batch: &RecordBatch, mask: &BooleanArray, status: u8) -> RecordBatch {
    let (no_rank, no_payload) = no_held_version();
    let statuses = zip_column(batch, mask, &status_scalar(status), STATUS);
    let held_rank = zip_column(batch, mask, &no_rank, HELD_RANK);
    let held_payload = zip_column(batch, mask, &no_payload, HELD_PAYLOAD);
    with_columns(
        batch,
        [
            (STATUS, statuses),
            (HELD_RANK, held_rank),
            (HELD_PAYLOAD, held_payload),
        ],
    )
}

/// `batch`'s rows in the order of its `column`.
fn sorted_by(batch: &RecordBatch, column: usize) -> RecordBatch {
    let order = kernel(sort_to_indices(batch.column(column), None, None));
    kernel(take_record_batch(batch, &order))
}

/// Every row of `batch`, as views.
fn views(batch: &RecordBatch) -> Vec<RecordView> {
    let keys = batch.column(RECORD_KEY).as_string::<i64>();
    let positions = batch.column(POSITION).as_primitive::<UInt64Type>();
    (0..batch.num_rows())
        .map(|row| {
            RecordView::new(
                keys.value(row).to_owned(),
                latest_at(batch, row),
                positions.value(row),
                state_at(batch, row).status,
                held_at(batch, row),
            )
        })
        .collect()
}

impl PocketRecords for ArrowRecords {
    type Batch = RecordBatch;

    fn lookup(&self, key: &str) -> Option<RecordState> {
        row_of(&self.batch, key).map(|row| state_at(&self.batch, row))
    }

    fn insert_queued(&mut self, key: &str, version: Version, position: u64) {
        let row = kernel(RecordBatch::try_new(
            Arc::clone(&RECORDS),
            vec![
                Arc::new(LargeStringArray::from(vec![key])),
                Arc::new(Int64Array::from(vec![version.rank])),
                Arc::new(LargeBinaryArray::from(vec![version.payload.as_slice()])),
                Arc::new(UInt64Array::from(vec![position])),
                Arc::new(UInt8Array::from(vec![QUEUED])),
                Arc::new(Int64Array::new_null(1)),
                Arc::new(LargeBinaryArray::new_null(1)),
            ],
        ));
        self.batch = kernel(concat_batches(&RECORDS, [&self.batch, &row]));
    }

    fn supersede(&mut self, key: &str, version: Version) -> Option<Version> {
        let row = row_of(&self.batch, key)?;
        let old = latest_at(&self.batch, row);
        let mask = key_mask(&self.batch, key);
        let rank = Scalar::new(Int64Array::from(vec![version.rank]));
        let payload = Scalar::new(LargeBinaryArray::from(vec![version.payload.as_slice()]));
        let ranks = zip_column(&self.batch, &mask, &rank, RANK);
        let payloads = zip_column(&self.batch, &mask, &payload, PAYLOAD);
        self.batch = with_columns(&self.batch, [(RANK, ranks), (PAYLOAD, payloads)]);
        Some(old)
    }

    fn hold_oldest(&mut self, limit: usize) -> RecordBatch {
        let rows = oldest_queued(&self.batch, limit);
        let latest = kernel(self.batch.project(&[RECORD_KEY, RANK, PAYLOAD]));
        let handed = kernel(take_record_batch(&latest, &rows));
        let mask = rows_mask(self.batch.num_rows(), &rows);
        self.batch = hold_rows(&self.batch, &mask);
        handed
    }

    fn settle(&mut self, key: &str) -> Option<Version> {
        let row = held_row(&self.batch, key)?;
        let held = held_at(&self.batch, row);
        let others = kernel(neq(
            self.batch.column(RECORD_KEY),
            &Scalar::new(LargeStringArray::from(vec![key])),
        ));
        self.batch = kernel(filter_record_batch(&self.batch, &others));
        held
    }

    fn defer(&mut self, key: &str, position: u64) -> Option<Version> {
        let row = held_row(&self.batch, key)?;
        let held = held_at(&self.batch, row);
        let mask = key_mask(&self.batch, key);
        let positions = zip_column(
            &self.batch,
            &mask,
            &Scalar::new(UInt64Array::from(vec![position])),
            POSITION,
        );
        let moved = with_columns(&self.batch, [(POSITION, positions)]);
        self.batch = release_rows(&moved, &mask, DEFERRED);
        held
    }

    fn free_all(&mut self) {
        let outstanding = kernel(neq(self.batch.column(STATUS), &status_scalar(QUEUED)));
        self.batch = release_rows(&self.batch, &outstanding, QUEUED);
    }

    fn record_count(&self) -> usize {
        self.batch.num_rows()
    }

    fn held(&self) -> Vec<HeldRecord> {
        let held = kernel(filter_record_batch(
            &self.batch,
            &status_mask(&self.batch, HELD),
        ));
        let by_position = sorted_by(&held, POSITION);
        let versions = kernel(by_position.project(&[RECORD_KEY, HELD_RANK, HELD_PAYLOAD]));
        held_records(&versions)
    }

    fn snapshot(&self) -> Vec<RecordView> {
        views(&sorted_by(&self.batch, RECORD_KEY))
    }
}

#[cfg(test)]
mod tests {
    use super::ArrowRecords;
    use super::schema::{BATCH, RECORDS};
    use arrow_schema::{DataType, Schema};
    use libpachinko::records::{PocketRecords, Version};

    fn type_of<'schema>(schema: &'schema Schema, name: &str) -> &'schema DataType {
        schema.field_with_name(name).unwrap().data_type()
    }

    #[test]
    fn the_variable_length_columns_have_64_bit_offsets() {
        assert_eq!(type_of(&RECORDS, "record_key"), &DataType::LargeUtf8);
        assert_eq!(type_of(&RECORDS, "payload"), &DataType::LargeBinary);
        assert_eq!(type_of(&RECORDS, "held_payload"), &DataType::LargeBinary);
        assert_eq!(type_of(&BATCH, "record_key"), &DataType::LargeUtf8);
        assert_eq!(type_of(&BATCH, "payload"), &DataType::LargeBinary);
    }

    #[test]
    #[ignore = "allocates over 2 GiB"]
    fn a_pocket_holds_more_than_2_gib_of_payload() {
        let mut records = ArrowRecords::default();
        records.insert_queued("a", Version::new(1, vec![0; 1 << 30]), 1);
        records.insert_queued("b", Version::new(1, vec![0; 1 << 30]), 2);
        assert_eq!(records.record_count(), 2);
    }
}
