//! The rebuild: the store's live rows appended again, in position order, to a
//! fresh log whose chunk ids start at 0.
//!
//! Two things call it. An insert or a defer at a position not past the last one
//! breaks the log's order; only the records contract and the doc examples do that,
//! so the store sorts itself again in O(n log n) instead of keeping a structure for
//! it. And chunk ids only grow, so once the next one would not fit in a row id the
//! store renumbers. That happens once every 2^26 chunks or so, on a log already in
//! order, where the sort is one pass and the rebuild O(n).
//!
//! Renumbering reuses the rebuild rather than shifting every id in place. A shift
//! would still rewrite each table that names rows by id and rehash the index,
//! which cannot be walked, so it would cost the same O(n) once in 2^26 chunks
//! while adding a second path to keep right; the rebuild also compacts.
//!
//! Every table that names rows by id is rebuilt from the new ids in this one
//! place: the index, the superseded table, the deferred list, the cursor and the
//! held end, and the out-of-line keys and payloads, which the fresh log stores
//! anew. The rebuild also drops the dead rows' bytes and the placeholder chunks.

use std::collections::BTreeMap;

use libpachinko::records::Version;

use super::Inner;
use super::chunk::{RowStatus, SUPERSEDED};
use super::rows::{NewRow, RowId, Rows};

impl Inner {
    /// Rebuilds the store from its live rows, sorted by position.
    pub(super) fn rebuild(&mut self) {
        let old = std::mem::take(&mut self.rows);
        let mut superseded = std::mem::take(&mut self.superseded);
        self.deferred.clear();
        self.cursor = self.rows.front_row();
        self.held_end = self.rows.front_row();
        self.index.reset();
        for row in old.by_position() {
            self.readd(&old, &mut superseded, row);
        }
        self.index.trim(|entry| self.rows.key_bytes_at(entry));
    }

    /// Appends the old log's row `row` to the new log, and records it in the
    /// tables that name rows by id.
    fn readd(&mut self, old: &Rows, superseded: &mut BTreeMap<RowId, Version>, row: RowId) {
        let Some(slot) = old.slot(row) else {
            return;
        };
        let new = NewRow {
            key: old.key_at(row),
            rank: slot.rank,
            payload: old.payload_at(row),
            position: slot.position,
            status: slot.status(),
        };
        let id = self.rows.append(&new).unwrap_or_else(exhausted);
        self.index
            .enter(new.key, id, |entry| self.rows.key_bytes_at(entry));
        if let Some(held) = superseded.remove(&row) {
            self.superseded.insert(id, held);
            if let Some(stored) = self.rows.slot_mut(id) {
                stored.raise(SUPERSEDED);
            }
        }
        match new.status {
            RowStatus::Held => self.held_end = id.saturating_add(1),
            RowStatus::Deferred => self.deferred.push(id),
            RowStatus::Queued | RowStatus::Dead => {}
        }
        self.last_position = Some(new.position);
    }
}

/// Stops the store when its live rows need more chunks than a row id can name.
///
/// A rebuild packs the live rows into as few chunks as their sizes allow, so only
/// a pocket of about 2^32 live rows reaches here, whose slots alone would take
/// 96 GiB. A `u32` row id cannot name more rows, and dropping a record silently
/// would be worse than stopping. The Arrow store has the same ceiling: its pull
/// sorts into `UInt32Array` row indexes. A `u64` row id would lift it at about 8
/// more bytes a record.
#[expect(
    clippy::panic,
    reason = "a u32 row id names at most 2^32 rows, the ceiling ArrowRecords' u32 row indexes already set"
)]
pub(super) fn exhausted() -> RowId {
    panic!("a pocket's live rows outnumber its row ids")
}

#[cfg(test)]
mod tests {
    use crate::log::LogRecords;
    use crate::log::chunk::RowStatus;
    use crate::log::rows::{ID_LIMIT, row_id};
    use crate::log::tests::assert_invariants;
    use libpachinko::records::{PocketRecords, RecordStatus, Version};

    fn version(rank: i64, payload: &str) -> Version {
        Version::new(rank, payload.as_bytes().to_vec())
    }

    fn positions(records: &LogRecords) -> Vec<u64> {
        let inner = records.inner.as_deref().unwrap();
        inner
            .rows
            .walk(inner.rows.front_row())
            .filter(|(_, slot)| slot.status() != RowStatus::Dead)
            .map(|(_, slot)| slot.position)
            .collect()
    }

    #[test]
    fn an_out_of_order_insert_rebuilds_the_log_in_position_order() {
        let mut records = LogRecords::default();
        for (key, position) in [("c", 30), ("a", 10), ("d", 40), ("b", 20)] {
            records.insert_queued(key, version(1, key), position);
            assert_invariants(&records);
        }
        assert_eq!(positions(&records), [10, 20, 30, 40]);
        records.insert_queued("e", version(1, "e"), 50);
        assert_eq!(positions(&records), [10, 20, 30, 40, 50]);
    }

    #[test]
    fn an_out_of_order_defer_rebuilds_and_keeps_the_held_and_superseded_rows() {
        let mut records = LogRecords::default();
        for (key, position) in [("a", 10), ("b", 20), ("c", 30)] {
            records.insert_queued(key, version(1, key), position);
        }
        records.hold_oldest(2);
        records.supersede("b", version(2, "b2"));
        assert_eq!(records.defer("a", 5), Some(version(1, "a")));
        assert_invariants(&records);
        assert_eq!(positions(&records), [5, 20, 30]);
        let state = records.lookup("b").unwrap();
        assert_eq!(
            (state.status, state.held_rank, state.rank),
            (RecordStatus::Held, Some(1), 2)
        );
        assert_eq!(records.settle("b"), Some(version(1, "b")));
        assert_eq!(records.lookup("a").unwrap().status, RecordStatus::Deferred);
    }

    fn next_row(records: &LogRecords) -> u32 {
        records
            .inner
            .as_deref()
            .map_or(0, |inner| inner.rows.next_row())
    }

    /// Churns `cycles` records through the store, one insert, pull and settle at a
    /// time, and returns how often the row ids started again lower.
    fn churn(records: &mut LogRecords, cycles: u64) -> usize {
        let mut renumbered = 0;
        for position in 1..=cycles {
            let key = position.to_string();
            let before = next_row(records);
            records.insert_queued(&key, version(1, &key), position);
            renumbered += usize::from(next_row(records) <= before);
            records.hold_oldest(1);
            assert_eq!(records.settle(&key), Some(version(1, &key)));
            assert_invariants(records);
        }
        renumbered
    }

    #[test]
    fn renumbering_starts_the_chunk_ids_again_from_zero() {
        let mut records = LogRecords::default();
        records.insert_queued("first", version(1, "f"), 0);
        records.hold_oldest(1);
        records.settle("first");
        assert!(churn(&mut records, u64::from(ID_LIMIT) * 10) >= 2);
        assert_eq!(records.record_count(), 0);
    }

    #[test]
    fn renumbering_keeps_a_held_row_and_its_superseded_version() {
        let mut records = LogRecords::default();
        records.insert_queued("kept", version(1, "k"), 0);
        records.hold_oldest(1);
        records.supersede("kept", version(2, "k2"));
        assert!(churn(&mut records, u64::from(ID_LIMIT) * 10) >= 2);
        let inner = records.inner.as_deref().unwrap();
        assert!(inner.rows.next_row() < row_id(ID_LIMIT, 0));
        assert_eq!(records.lookup("kept").unwrap().held_rank, Some(1));
        assert_eq!(records.settle("kept"), Some(version(1, "k")));
    }

    #[test]
    #[should_panic(expected = "outnumber its row ids")]
    fn a_pocket_whose_rows_outnumber_its_row_ids_stops() {
        let mut records = LogRecords::default();
        let rows = u64::from(row_id(ID_LIMIT, 0));
        for position in 0..=rows {
            records.insert_queued(&position.to_string(), version(1, ""), position);
        }
    }
}
