//! `LogRecords`: one pocket's records as a log of chunks in position order, with a
//! hash index by key (decision 0021).
//!
//! The queue hands out positions that only increase, so every step has a fast
//! path. An insert appends at the log's tail. A pull walks forward from a cursor
//! that no queued record precedes. A settle marks its row dead, and a defer marks
//! its row dead and appends the record again at its new position. The index finds a
//! key's row by its hash. A position that does not increase, which only the
//! records contract and the doc examples hand out, rebuilds the store sorted by
//! position instead.
//!
//! The module's parts:
//!
//! - `chunk`: up to 64 rows, as 24-byte slots, a key arena and a payload arena.
//! - `rows`: the deque of chunks, the row ids, and the keys and payloads too large
//!   for a chunk, stored out of line. It is its own module so that addressing a row
//!   by id lives in one place, apart from the steps in this file that decide
//!   which rows to touch.
//! - `index`: the key index, a `hashbrown::HashTable` of row ids.
//! - `batch`: the `RecordBatch` a pull hands out.
//! - `compact`: the rebuild, for an out-of-order position and for renumbering.
//!
//! A held record is a status, not a copy: a second copy of its payload exists only
//! once a supersede lands on it while it is held.
//!
//! ```
//! use libpachinko::records::{PocketRecords, RecordStatus, Version};
//! use pachinko::arrow::held_records;
//! use pachinko::log::LogRecords;
//!
//! let mut records = LogRecords::default();
//! records.insert_queued("a", Version::new(1, b"one".to_vec()), 1);
//! records.insert_queued("b", Version::new(1, b"two".to_vec()), 2);
//! let batch = records.hold_oldest(1);
//! assert_eq!(held_records(&batch)[0].key, "a");
//! records.supersede("a", Version::new(2, b"newer".to_vec()));
//! assert_eq!(records.defer("a", 3), Some(Version::new(1, b"one".to_vec())));
//! assert_eq!(records.lookup("a").map(|state| state.status), Some(RecordStatus::Deferred));
//! records.free_all();
//! let rest = held_records(&records.hold_oldest(10));
//! assert_eq!((rest[0].key.as_str(), rest[1].rank), ("b", 2));
//! ```

mod batch;
mod chunk;
mod compact;
mod index;
mod rows;

use std::collections::BTreeMap;

use arrow_array::RecordBatch;
use libpachinko::records::{HeldRecord, PocketRecords, RecordState, RecordView, Version};

use chunk::{RowStatus, SUPERSEDED, Slot};
use index::Index;
use rows::{NewRow, RowId, Rows};

/// One pocket's records as a log of chunks with a key index. A pull hands out a
/// `RecordBatch` of the held versions' `record_key`, `rank` and `payload`, oldest
/// first: the batch [`ArrowRecords`](crate::arrow::ArrowRecords) hands out.
///
/// An empty pocket is one null pointer and holds no heap: the store frees
/// everything once its last record is settled, and once `free_all` finds it
/// empty.
///
/// # Panics
///
/// An insert or a defer panics when the pocket would hold more than `2^32 − 64`
/// live records, `2^26 − 1` chunks of 64 rows, whose slots alone take about 96 GiB:
/// a row id is a `u32`. Arena limits can seal chunks early and lower that, but
/// only for keys averaging megabytes or payloads averaging tens of megabytes. The
/// Arrow store has the same ceiling. No other size panics; a key or payload too
/// large for a chunk is stored out of line.
///
/// ```
/// use libpachinko::records::{PocketRecords, Version};
/// use pachinko::arrow::held_records;
/// use pachinko::log::LogRecords;
///
/// let mut records = LogRecords::default();
/// assert_eq!(std::mem::size_of::<LogRecords>(), std::mem::size_of::<usize>());
/// records.insert_queued("b", Version::new(1, b"two".to_vec()), 2);
/// records.insert_queued("a", Version::new(1, b"one".to_vec()), 1);
/// let batch = records.hold_oldest(1);
/// assert_eq!(held_records(&batch)[0].key, "a");
/// assert_eq!(records.settle("a"), Some(Version::new(1, b"one".to_vec())));
/// assert_eq!(records.record_count(), 1);
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct LogRecords {
    inner: Option<Box<Inner>>,
}

/// A pocket that has records.
#[derive(Clone, Debug, Default)]
struct Inner {
    rows: Rows,
    index: Index,
    /// The largest position appended; an insert or a defer at a position not
    /// past it rebuilds the store in order.
    last_position: Option<u64>,
    /// No queued row precedes it, so a pull starts its walk here.
    cursor: RowId,
    /// One past the last held row.
    held_end: RowId,
    /// The deferred rows, which `free_all` returns to queued.
    deferred: Vec<RowId>,
    /// Each held row's held version, once a supersede replaced its latest one.
    superseded: BTreeMap<RowId, Version>,
}

impl Inner {
    /// How many records the pocket has: one for each key in the index.
    fn count(&self) -> usize {
        self.index.entries()
    }

    /// The row holding `key`.
    fn find(&self, key: &str) -> Option<RowId> {
        self.index.locate(key, |row| self.rows.key_bytes_at(row))
    }

    /// The row holding `key`, if that record is held.
    fn held_row(&self, key: &str) -> Option<RowId> {
        self.find(key).filter(|row| {
            self.rows
                .slot(*row)
                .is_some_and(|slot| slot.status() == RowStatus::Held)
        })
    }

    /// The held version of the held row `row`: from the superseded table once a
    /// supersede landed on it, and otherwise its latest version, in place.
    fn held_version(&self, row: RowId, slot: Slot) -> (i64, &[u8]) {
        let superseded = slot
            .has(SUPERSEDED)
            .then(|| self.superseded.get(&row))
            .flatten();
        superseded.map_or_else(
            || (slot.rank, self.rows.payload_at(row)),
            |held| (held.rank, held.payload.as_slice()),
        )
    }

    /// `key`'s latest rank, status and held rank.
    fn state_of(&self, key: &str) -> Option<RecordState> {
        let row = self.find(key)?;
        let slot = self.rows.slot(row)?;
        let held_rank = (slot.status() == RowStatus::Held).then(|| self.held_version(row, slot).0);
        Some(RecordState::new(
            slot.rank,
            slot.status().reported(),
            held_rank,
        ))
    }

    /// Appends `row` and indexes it. When `row` is not past the last position,
    /// the store rebuilds itself in position order.
    fn add(&mut self, row: NewRow<'_>) {
        let id = self.place(&row);
        self.index
            .enter(row.key, id, |entry| self.rows.key_bytes_at(entry));
        let in_order = self.note(id, &row);
        if !in_order {
            self.rebuild();
        }
    }

    /// Appends `row` and returns its id. When the chunk ids have run out, the
    /// store rebuilds, which renumbers them from 0, and appends then.
    fn place(&mut self, row: &NewRow<'_>) -> RowId {
        self.rows.append(row).unwrap_or_else(|| {
            self.rebuild();
            self.rows.append(row).unwrap_or_else(compact::exhausted)
        })
    }

    /// Records the row `id` just appended: a deferred row joins the deferred list,
    /// and the last position moves on. Returns whether `row` came after every row
    /// before it.
    fn note(&mut self, id: RowId, row: &NewRow<'_>) -> bool {
        if row.status == RowStatus::Deferred {
            self.deferred.push(id);
        }
        let in_order = self.last_position.is_none_or(|last| row.position > last);
        self.last_position = self.last_position.max(Some(row.position));
        in_order
    }

    /// Replaces `key`'s latest version and returns the one replaced. A held
    /// record's held version moves to the superseded table first, once.
    fn supersede_key(&mut self, key: &str, version: Version) -> Option<Version> {
        let row = self.find(key)?;
        let slot = self.rows.slot(row)?;
        if slot.status() == RowStatus::Held && !slot.has(SUPERSEDED) {
            self.keep_held(row);
        }
        let rank = self.rows.slot_mut(row).map_or(slot.rank, |stored| {
            std::mem::replace(&mut stored.rank, version.rank)
        });
        let payload = self.rows.replace_payload(row, version.payload);
        Some(Version::new(rank, payload))
    }

    /// Copies the held row `row`'s latest version, which is its held version, into
    /// the superseded table, and flags the row.
    fn keep_held(&mut self, row: RowId) {
        let held = self.rows.version_at(row);
        self.superseded.insert(row, held);
        if let Some(slot) = self.rows.slot_mut(row) {
            slot.raise(SUPERSEDED);
        }
    }

    /// Holds up to `limit` queued rows, oldest first, and returns them as a batch.
    fn hold(&mut self, limit: usize) -> RecordBatch {
        let held: Vec<RowId> = self
            .rows
            .walk(self.cursor)
            .filter(|(_, slot)| slot.status() == RowStatus::Queued)
            .take(limit)
            .map(|(row, _)| row)
            .collect();
        for row in &held {
            if let Some(slot) = self.rows.slot_mut(*row) {
                slot.restate(RowStatus::Held);
            }
        }
        self.advance(&held, limit);
        batch::pulled(&self.rows, &held)
    }

    /// Moves the cursor past a pull's walk, and the held end past its last row. A
    /// pull that found fewer than `limit` rows walked to the end of the log.
    fn advance(&mut self, held: &[RowId], limit: usize) {
        let after_last = held.last().map(|row| row.saturating_add(1));
        self.cursor = if held.len() < limit {
            self.rows.next_row()
        } else {
            after_last.unwrap_or(self.cursor)
        };
        self.held_end = self.held_end.max(after_last.unwrap_or(0));
    }

    /// Removes `key`'s record if it is held, and returns its held version.
    fn settle_key(&mut self, key: &str) -> Option<Version> {
        let row = self.held_row(key)?;
        let held = self.take_held(row);
        self.drop_row(key, row);
        Some(held)
    }

    /// Moves `key`'s record, if it is held, to a deferred row at `position`, and
    /// returns its held version. The record's row dies and the record is appended
    /// again with its latest version.
    fn defer_key(&mut self, key: &str, position: u64) -> Option<Version> {
        let row = self.held_row(key)?;
        let held = self.superseded.remove(&row);
        let latest = self.rows.version_at(row);
        self.drop_row(key, row);
        self.add(NewRow {
            key,
            rank: latest.rank,
            payload: &latest.payload,
            position,
            status: RowStatus::Deferred,
        });
        Some(held.unwrap_or(latest))
    }

    /// The held row `row`'s held version, taken out of the superseded table or
    /// copied out of the log.
    fn take_held(&mut self, row: RowId) -> Version {
        self.superseded
            .remove(&row)
            .unwrap_or_else(|| self.rows.version_at(row))
    }

    /// Unindexes `key` at `row` and marks the row dead, then shrinks the index if
    /// it has grown four times too large, and compacts the store if dead slots
    /// have outgrown the live rows.
    fn drop_row(&mut self, key: &str, row: RowId) {
        self.index.forget(key, row);
        self.rows.kill(row);
        self.index.trim(|entry| self.rows.key_bytes_at(entry));
        self.compact_if_wasteful();
    }

    /// Returns every held and deferred row to queued, at the position it has, and
    /// drops the held versions.
    fn requeue(&mut self) {
        let front = self.rows.front_row();
        let held_end = self.held_end;
        let held: Vec<RowId> = self
            .rows
            .walk(front)
            .take_while(|(row, _)| *row < held_end)
            .filter(|(_, slot)| slot.status() == RowStatus::Held)
            .map(|(row, _)| row)
            .collect();
        let deferred = std::mem::take(&mut self.deferred);
        for row in held.iter().chain(&deferred) {
            if let Some(slot) = self.rows.slot_mut(*row) {
                slot.restate(RowStatus::Queued);
                slot.lower(SUPERSEDED);
            }
        }
        self.superseded.clear();
        self.cursor = front;
        self.held_end = front;
    }

    /// The held records, by position.
    fn held_records(&self) -> Vec<HeldRecord> {
        self.rows
            .walk(self.rows.front_row())
            .take_while(|(row, _)| *row < self.held_end)
            .filter(|(_, slot)| slot.status() == RowStatus::Held)
            .map(|(row, slot)| {
                let (rank, payload) = self.held_version(row, slot);
                HeldRecord::new(self.rows.key_at(row).to_owned(), rank, payload.to_vec())
            })
            .collect()
    }

    /// Every record, by key.
    fn views(&self) -> Vec<RecordView> {
        let mut views: Vec<RecordView> = self
            .rows
            .walk(self.rows.front_row())
            .filter(|(_, slot)| slot.status() != RowStatus::Dead)
            .map(|(row, slot)| self.view(row, slot))
            .collect();
        views.sort_by(|left, right| left.key.cmp(&right.key));
        views
    }

    /// The record at `row`, for inspection.
    fn view(&self, row: RowId, slot: Slot) -> RecordView {
        let held = (slot.status() == RowStatus::Held).then(|| {
            let (rank, payload) = self.held_version(row, slot);
            Version::new(rank, payload.to_vec())
        });
        RecordView::new(
            self.rows.key_at(row).to_owned(),
            self.rows.version_at(row),
            slot.position,
            slot.status().reported(),
            held,
        )
    }
}

impl LogRecords {
    /// Drops everything once the pocket has no record. With none, nothing can be
    /// held, deferred or superseded, so nothing is lost. The queue releases an
    /// emptied pocket without calling `free_all`, so a settle is where an acked-out
    /// pocket gives its heap back. The trade-off, accepted: a pocket that empties
    /// on every pull allocates its state again on the next insert.
    fn drop_if_empty(&mut self) {
        self.inner = self.inner.take().filter(|inner| inner.count() > 0);
    }
}

impl PocketRecords for LogRecords {
    type Batch = RecordBatch;

    fn lookup(&self, key: &str) -> Option<RecordState> {
        self.inner.as_ref()?.state_of(key)
    }

    fn insert_queued(&mut self, key: &str, version: Version, position: u64) {
        let row = NewRow {
            key,
            rank: version.rank,
            payload: &version.payload,
            position,
            status: RowStatus::Queued,
        };
        self.inner.get_or_insert_with(Box::default).add(row);
    }

    fn supersede(&mut self, key: &str, version: Version) -> Option<Version> {
        self.inner.as_mut()?.supersede_key(key, version)
    }

    fn hold_oldest(&mut self, limit: usize) -> RecordBatch {
        self.inner
            .as_mut()
            .map_or_else(batch::empty, |inner| inner.hold(limit))
    }

    fn settle(&mut self, key: &str) -> Option<Version> {
        let held = self.inner.as_mut()?.settle_key(key);
        self.drop_if_empty();
        held
    }

    fn defer(&mut self, key: &str, position: u64) -> Option<Version> {
        self.inner.as_mut()?.defer_key(key, position)
    }

    fn free_all(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            inner.requeue();
            inner.compact_if_wasteful();
        }
        self.drop_if_empty();
    }

    fn record_count(&self) -> usize {
        self.inner.as_ref().map_or(0, |inner| inner.count())
    }

    fn held(&self) -> Vec<HeldRecord> {
        self.inner
            .as_ref()
            .map_or_else(Vec::new, |inner| inner.held_records())
    }

    fn snapshot(&self) -> Vec<RecordView> {
        self.inner
            .as_ref()
            .map_or_else(Vec::new, |inner| inner.views())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use libpachinko::records::contract::check_contract;
    use libpachinko::records::{HeldRecord, MemoryRecords, PocketRecords, RecordStatus, Version};

    use super::chunk::{HEADER_BYTES, RowStatus, SLOT_BYTES, SUPERSEDED, Slot};
    use super::rows::RowId;
    use super::{Inner, LogRecords};
    use crate::arrow::held_records;

    fn version(rank: i64, payload: &str) -> Version {
        Version::new(rank, payload.as_bytes().to_vec())
    }

    /// Checks every invariant the store's fast paths rely on.
    pub(super) fn assert_invariants(records: &LogRecords) {
        let Some(inner) = records.inner.as_deref() else {
            return;
        };
        let live: Vec<(RowId, Slot)> = inner
            .rows
            .walk(inner.rows.front_row())
            .filter(|(_, slot)| slot.status() != RowStatus::Dead)
            .collect();
        assert_eq!(inner.count(), live.len(), "the index holds each live row");
        let positions: Vec<u64> = live.iter().map(|(_, slot)| slot.position).collect();
        assert!(
            positions.is_sorted(),
            "positions grow along the log: {positions:?}"
        );
        for (row, slot) in &live {
            assert_row(inner, *row, *slot);
        }
        let deferred: BTreeSet<RowId> = live
            .iter()
            .filter(|(_, slot)| slot.status() == RowStatus::Deferred)
            .map(|(row, _)| *row)
            .collect();
        let listed: BTreeSet<RowId> = inner.deferred.iter().copied().collect();
        assert_eq!(
            listed, deferred,
            "the deferred list names the deferred rows"
        );
        assert_eq!(listed.len(), inner.deferred.len(), "no row is listed twice");
        assert!(
            inner.rows.waste() <= inner.rows.live_bytes() + 64 * SLOT_BYTES + HEADER_BYTES,
            "the waste stays within its compaction bound"
        );
        let live_bytes: usize = live
            .iter()
            .map(|(row, _)| {
                SLOT_BYTES + inner.rows.key_bytes_at(*row).len() + inner.rows.payload_at(*row).len()
            })
            .sum();
        assert_eq!(inner.rows.live_bytes(), live_bytes, "the live bytes add up");
        let flagged = live.iter().filter(|(_, slot)| slot.has(SUPERSEDED)).count();
        assert_eq!(
            inner.superseded.len(),
            flagged,
            "a superseded entry for each flag"
        );
    }

    fn assert_row(inner: &Inner, row: RowId, slot: Slot) {
        assert_eq!(
            inner.find(inner.rows.key_at(row)),
            Some(row),
            "a row is found by its key"
        );
        assert!(row < inner.rows.next_row(), "a row precedes the next row");
        match slot.status() {
            RowStatus::Queued => assert!(row >= inner.cursor, "no queued row precedes the cursor"),
            RowStatus::Held => {
                assert!(row < inner.held_end, "every held row precedes the held end");
            }
            RowStatus::Deferred | RowStatus::Dead => {}
        }
        assert_eq!(
            slot.has(SUPERSEDED),
            inner.superseded.contains_key(&row),
            "the flag and the superseded table agree"
        );
        assert!(
            !slot.has(SUPERSEDED) || slot.status() == RowStatus::Held,
            "only a held row is superseded"
        );
    }

    #[test]
    fn log_records_keep_the_records_contract_under_the_small_limits() {
        check_contract::<LogRecords>(held_records);
    }

    #[test]
    fn an_empty_store_is_one_pointer_and_holds_nothing() {
        let records = LogRecords::default();
        assert_eq!(size_of::<LogRecords>(), size_of::<usize>());
        assert!(records.inner.is_none());
        let mut records = records;
        assert_eq!(records.hold_oldest(5).num_rows(), 0);
        assert_eq!(
            (
                records.lookup("a"),
                records.settle("a"),
                records.defer("a", 1)
            ),
            (None, None, None)
        );
        assert_eq!(records.supersede("a", version(1, "")), None);
        records.free_all();
        assert!(
            records.inner.is_none() && records.held().is_empty() && records.snapshot().is_empty()
        );
    }

    #[test]
    fn the_last_settle_drops_the_inner() {
        let mut records = LogRecords::default();
        records.insert_queued("a", version(1, "a"), 1);
        records.insert_queued("b", version(1, "b"), 2);
        records.hold_oldest(2);
        records.defer("b", 3);
        records.settle("a");
        assert!(
            records.inner.is_some(),
            "a deferred record keeps the pocket"
        );
        records.free_all();
        records.hold_oldest(1);
        assert_eq!(records.settle("b"), Some(version(1, "b")));
        assert!(records.inner.is_none());
        assert_eq!(records.record_count(), 0);
        records.insert_queued("c", version(1, "c"), 4);
        assert_eq!(records.lookup("c").map(|state| state.rank), Some(1));
    }

    #[test]
    fn free_all_keeps_the_inner_of_a_store_with_records() {
        let mut records = LogRecords::default();
        records.insert_queued("a", version(1, "a"), 1);
        records.hold_oldest(1);
        records.free_all();
        assert!(records.inner.is_some());
        assert_eq!(records.lookup("a").unwrap().status, RecordStatus::Queued);
        assert_invariants(&records);
    }

    #[test]
    fn a_supersede_of_a_held_record_keeps_one_copy_of_its_held_version() {
        let mut records = LogRecords::default();
        records.insert_queued("a", version(1, "a1"), 1);
        records.hold_oldest(1);
        assert_eq!(
            records.supersede("a", version(2, "a2")),
            Some(version(1, "a1"))
        );
        assert_eq!(
            records.supersede("a", version(3, "a3")),
            Some(version(2, "a2"))
        );
        let inner = records.inner.as_deref().unwrap();
        assert_eq!(inner.superseded.len(), 1);
        assert_eq!(
            records.held(),
            [HeldRecord::new("a".to_owned(), 1, b"a1".to_vec())]
        );
        assert_invariants(&records);
        assert_eq!(records.settle("a"), Some(version(1, "a1")));
        assert!(
            records.inner.is_none(),
            "the last settle drops the superseded copy too"
        );
    }

    #[test]
    fn a_defer_of_a_superseded_held_record_appends_its_latest_version() {
        let mut records = LogRecords::default();
        records.insert_queued("a", version(1, "a1"), 1);
        records.insert_queued("b", version(1, "b1"), 2);
        records.hold_oldest(2);
        records.supersede("a", version(2, "a2"));
        assert_eq!(records.defer("a", 3), Some(version(1, "a1")));
        assert_eq!(records.defer("b", 4), Some(version(1, "b1")));
        assert_invariants(&records);
        assert_eq!(records.held(), []);
        assert_eq!(records.hold_oldest(5).num_rows(), 0);
        records.free_all();
        let pulled = held_records(&records.hold_oldest(5));
        assert_eq!(
            pulled,
            [
                HeldRecord::new("a".to_owned(), 2, b"a2".to_vec()),
                HeldRecord::new("b".to_owned(), 1, b"b1".to_vec()),
            ]
        );
    }

    #[test]
    fn a_large_supersede_moves_the_payload_out_of_line_and_a_small_one_back() {
        let mut records = LogRecords::default();
        records.insert_queued("a", version(1, "small"), 1);
        records.insert_queued("b", version(1, "next"), 2);
        let large = "x".repeat(40);
        records.supersede("a", version(2, &large));
        assert_eq!(
            records.supersede("a", version(3, "s")),
            Some(version(2, &large))
        );
        assert_eq!(records.snapshot()[1].latest, version(1, "next"));
        assert_invariants(&records);
    }

    #[test]
    fn a_pull_resumes_after_the_rows_it_held() {
        let mut records = LogRecords::default();
        for position in 0..10 {
            records.insert_queued(&position.to_string(), version(1, ""), position);
        }
        let keys = |batch| {
            held_records(&batch)
                .into_iter()
                .map(|held| held.key)
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(records.hold_oldest(3)), ["0", "1", "2"]);
        assert_eq!(records.inner.as_deref().unwrap().cursor, 3);
        assert_eq!(keys(records.hold_oldest(0)), Vec::<String>::new());
        assert_eq!(keys(records.hold_oldest(4)), ["3", "4", "5", "6"]);
        assert_eq!(keys(records.hold_oldest(9)), ["7", "8", "9"]);
        assert_invariants(&records);
    }

    /// A xorshift generator: deterministic, so a failing seed replays.
    struct Draws(u64);

    impl Draws {
        fn below(&mut self, bound: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % bound.max(1)
        }

        fn chance(&mut self, percent: u64) -> bool {
            self.below(100) < percent
        }
    }

    /// The store under test and the reference, driven by the same steps.
    struct Pair {
        log: LogRecords,
        memory: MemoryRecords,
        draws: Draws,
        next_key: u64,
        next_position: u64,
    }

    impl Pair {
        fn new(seed: u64) -> Self {
            Self {
                log: LogRecords::default(),
                memory: MemoryRecords::default(),
                draws: Draws(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
                next_key: 0,
                next_position: 1,
            }
        }

        /// A position no record holds: mostly the next one, sometimes one below
        /// it, which sends the store down its rebuild.
        fn position(&mut self) -> u64 {
            let used: BTreeSet<u64> = self
                .memory
                .snapshot()
                .iter()
                .map(|view| view.position)
                .collect();
            let low = self.draws.below(self.next_position);
            if self.draws.chance(15) && !used.contains(&low) {
                return low;
            }
            self.next_position += 1 + self.draws.below(3);
            self.next_position
        }

        fn payload(&mut self) -> Version {
            let rank = i64::try_from(self.draws.below(10)).unwrap();
            let length = usize::try_from(self.draws.below(40)).unwrap();
            Version::new(rank, vec![u8::try_from(rank).unwrap(); length])
        }

        /// A key the store has, or an absent one.
        fn some_key(&mut self) -> String {
            let keys: Vec<String> = self
                .memory
                .snapshot()
                .into_iter()
                .map(|view| view.key)
                .collect();
            let pick =
                usize::try_from(self.draws.below(u64::try_from(keys.len()).unwrap() + 1)).unwrap();
            keys.get(pick)
                .cloned()
                .unwrap_or_else(|| "absent".to_owned())
        }

        fn insert(&mut self) {
            if self.memory.record_count() >= 20 {
                return;
            }
            self.next_key += 1;
            let key = format!(
                "{}{}",
                self.next_key,
                "x".repeat(usize::try_from(self.draws.below(20)).unwrap())
            );
            let (version, position) = (self.payload(), self.position());
            self.log.insert_queued(&key, version.clone(), position);
            self.memory.insert_queued(&key, version, position);
        }

        fn step(&mut self) {
            match self.draws.below(11) {
                0..=3 => self.insert(),
                4 | 5 => {
                    let (key, version) = (self.some_key(), self.payload());
                    assert_eq!(
                        self.log.supersede(&key, version.clone()),
                        self.memory.supersede(&key, version)
                    );
                }
                6 | 7 => {
                    let limit = usize::try_from(self.draws.below(5)).unwrap();
                    assert_eq!(
                        held_records(&self.log.hold_oldest(limit)),
                        self.memory.hold_oldest(limit)
                    );
                }
                8 => {
                    let key = self.some_key();
                    assert_eq!(self.log.settle(&key), self.memory.settle(&key));
                }
                9 => {
                    let (key, position) = (self.some_key(), self.position());
                    assert_eq!(
                        self.log.defer(&key, position),
                        self.memory.defer(&key, position)
                    );
                }
                _ => {
                    self.log.free_all();
                    self.memory.free_all();
                }
            }
        }

        fn step_and_check(&mut self) {
            self.step();
            self.assert_agree();
        }

        fn assert_agree(&self) {
            assert_eq!(self.log.record_count(), self.memory.record_count());
            assert_eq!(self.log.snapshot(), self.memory.snapshot());
            assert_eq!(self.log.held(), self.memory.held());
            for view in self.memory.snapshot() {
                assert_eq!(self.log.lookup(&view.key), self.memory.lookup(&view.key));
            }
            assert_eq!(self.log.lookup("absent"), None);
            assert_invariants(&self.log);
        }
    }

    #[test]
    fn log_records_agree_with_memory_records_under_the_small_limits() {
        for seed in 1..=16 {
            let mut pair = Pair::new(seed);
            for step in 0..1500 {
                let agreed = catch_unwind(AssertUnwindSafe(|| pair.step_and_check()));
                assert!(agreed.is_ok(), "seed {seed} diverged at step {step}");
            }
        }
    }
}
