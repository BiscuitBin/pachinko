//! The batch a pull hands out: the rows it held, as the Arrow store's pull schema,
//! built with exact capacities.
//!
//! It takes the schema from the public `batch_schema`, not the Arrow store's
//! private `BATCH` static, so the two stores share one schema without widening
//! `arrow::schema`'s visibility.

use std::sync::Arc;

use arrow_array::builder::{Int64Builder, LargeBinaryBuilder, LargeStringBuilder};
use arrow_array::{ArrayRef, RecordBatch};

use crate::arrow::{batch_schema, kernel};

use super::rows::{RowId, Rows};

/// A batch of no rows, with the pull schema.
pub(super) fn empty() -> RecordBatch {
    RecordBatch::new_empty(batch_schema())
}

/// The `record_key`, `rank` and `payload` of each of `held`, in order, as one
/// batch. Each column is sized from the rows' summed lengths before it is
/// filled, so no builder grows.
pub(super) fn pulled(rows: &Rows, held: &[RowId]) -> RecordBatch {
    let key_bytes = held.iter().map(|row| rows.key_at(*row).len()).sum();
    let payload_bytes = held.iter().map(|row| rows.payload_at(*row).len()).sum();
    let mut keys = LargeStringBuilder::with_capacity(held.len(), key_bytes);
    let mut ranks = Int64Builder::with_capacity(held.len());
    let mut payloads = LargeBinaryBuilder::with_capacity(held.len(), payload_bytes);
    for row in held {
        keys.append_value(rows.key_at(*row));
        ranks.append_value(rows.slot(*row).map_or(0, |slot| slot.rank));
        payloads.append_value(rows.payload_at(*row));
    }
    let columns: Vec<ArrayRef> = vec![
        Arc::new(keys.finish()),
        Arc::new(ranks.finish()),
        Arc::new(payloads.finish()),
    ];
    kernel(RecordBatch::try_new(batch_schema(), columns))
}

#[cfg(test)]
mod tests {
    use super::{empty, pulled};
    use crate::arrow::{batch_schema, held_records};
    use crate::log::chunk::{KEY_LIMIT, PAYLOAD_LIMIT, RowStatus};
    use crate::log::rows::{NewRow, Rows};
    use libpachinko::records::HeldRecord;

    #[test]
    fn a_pull_batch_carries_its_rows_in_the_order_given() {
        let mut rows = Rows::default();
        let long_key = "k".repeat(KEY_LIMIT + 1);
        let long_payload = vec![5; PAYLOAD_LIMIT + 1];
        let records = [
            ("a", 3, b"one".to_vec()),
            (long_key.as_str(), 4, long_payload),
        ];
        let ids: Vec<u32> = (0..)
            .zip(&records)
            .map(|(position, (key, rank, payload))| {
                let row = NewRow {
                    key,
                    rank: *rank,
                    payload,
                    position,
                    status: RowStatus::Queued,
                };
                rows.append(&row).unwrap()
            })
            .collect();
        let batch = pulled(&rows, &[ids[1], ids[0]]);
        assert_eq!(batch.schema(), batch_schema());
        let expected: Vec<HeldRecord> = [&records[1], &records[0]]
            .into_iter()
            .map(|(key, rank, payload)| HeldRecord::new((*key).to_owned(), *rank, payload.clone()))
            .collect();
        assert_eq!(held_records(&batch), expected);
    }

    #[test]
    fn an_empty_pull_is_a_batch_of_the_schema_with_no_rows() {
        for batch in [empty(), pulled(&Rows::default(), &[])] {
            assert_eq!(batch.num_rows(), 0);
            assert_eq!(batch.schema(), batch_schema());
        }
    }
}
