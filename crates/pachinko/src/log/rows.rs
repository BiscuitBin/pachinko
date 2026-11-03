//! The log: a pocket's rows in position order, as a deque of chunks.
//!
//! A row id is a `u32`: its chunk's id above the slot bits and its slot below, so
//! row ids grow in log order. Chunk ids only grow, so a row's chunk is
//! `chunks[chunk id − front id]`, and popping the front chunk renames nothing. A
//! key or payload too large for an empty chunk's arena is stored out of line, keyed
//! by its row id, with zero length in the arena, so no record is too large to
//! store.

use std::collections::{BTreeMap, VecDeque};

use libpachinko::records::Version;

use super::chunk::{
    Chunk, KEY_LIMIT, KEY_SPILLED, PAYLOAD_LIMIT, PAYLOAD_SPILLED, RowStatus, SLOT_BITS, Slot,
    narrow, widen,
};

/// A row's id: its chunk's id above [`SLOT_BITS`], its slot below.
pub(super) type RowId = u32;

/// The slot bits of a row id.
const SLOT_MASK: u32 = (1 << SLOT_BITS) - 1;

/// How many chunk ids a row id has room for: `2^26 − 1`, one short of the width, so
/// that one past the last row still fits in a `u32`; 32 under test, so a unit test
/// reaches the renumbering.
pub(super) const ID_LIMIT: u32 = if cfg!(test) {
    32
} else {
    (1 << (u32::BITS - SLOT_BITS)) - 1
};

/// The row at `slot` of chunk `chunk`.
pub(super) const fn row_id(chunk: u32, slot: u32) -> RowId {
    (chunk << SLOT_BITS) | slot
}

/// The id of a row's chunk.
const fn chunk_of(row: RowId) -> u32 {
    row >> SLOT_BITS
}

/// A row's slot within its chunk.
const fn slot_of(row: RowId) -> u32 {
    row & SLOT_MASK
}

/// A row to append: a record's latest rank and payload, its key, where it stands
/// and its status.
#[derive(Clone, Copy, Debug)]
pub(super) struct NewRow<'a> {
    /// The record key.
    pub(super) key: &'a str,
    /// The latest rank.
    pub(super) rank: i64,
    /// The latest payload.
    pub(super) payload: &'a [u8],
    /// Where the record stands among its pocket's records.
    pub(super) position: u64,
    /// The row's status.
    pub(super) status: RowStatus,
}

/// What of a new row the arenas store: its key and payload, or nothing for each one
/// too large for an empty chunk, with the flags that say which went out of line.
struct Stored<'a> {
    key: &'a str,
    payload: &'a [u8],
    flags: u32,
}

impl<'a> Stored<'a> {
    /// What the arenas store of `row`.
    const fn of(row: &NewRow<'a>) -> Self {
        let key_out = row.key.len() > KEY_LIMIT;
        let payload_out = row.payload.len() > PAYLOAD_LIMIT;
        Self {
            key: if key_out { "" } else { row.key },
            payload: if payload_out { &[] } else { row.payload },
            flags: (if key_out { KEY_SPILLED } else { 0 })
                | (if payload_out { PAYLOAD_SPILLED } else { 0 }),
        }
    }
}

/// A pocket's rows: the chunks in log order, the id of the front one, and the keys
/// and payloads stored out of line.
///
/// Rows are appended at the tail. The live rows' positions grow along the log; the
/// store keeps that so, rebuilding when a position would break it.
#[derive(Clone, Debug, Default)]
pub(super) struct Rows {
    chunks: VecDeque<Chunk>,
    /// The id of `chunks[0]`.
    front_id: u32,
    /// The keys too large for a chunk, by row id.
    spilled_keys: BTreeMap<RowId, String>,
    /// The payloads too large for a chunk, by row id.
    spilled_payloads: BTreeMap<RowId, Vec<u8>>,
}

impl Rows {
    /// The front chunk's first row: no row precedes it.
    pub(super) const fn front_row(&self) -> RowId {
        row_id(self.front_id, 0)
    }

    /// One past the last row: every row id is smaller, and every row appended
    /// later is at least this.
    pub(super) fn next_row(&self) -> RowId {
        let ids = self.front_id.saturating_add(narrow(self.chunks.len()));
        self.chunks.back().map_or(row_id(ids, 0), |tail| {
            row_id(ids.saturating_sub(1), 0).saturating_add(narrow(tail.slots().len()))
        })
    }

    /// The deque index and the slot of `row`, if `row` is not before the front.
    fn index_of(&self, row: RowId) -> Option<(usize, usize)> {
        let chunk = chunk_of(row).checked_sub(self.front_id)?;
        Some((widen(chunk), widen(slot_of(row))))
    }

    /// `row`'s chunk and its slot there.
    fn place(&self, row: RowId) -> Option<(&Chunk, usize)> {
        let (chunk, slot) = self.index_of(row)?;
        Some((self.chunks.get(chunk)?, slot))
    }

    /// `row`'s chunk, mutably, and its slot there.
    fn place_mut(&mut self, row: RowId) -> Option<(&mut Chunk, usize)> {
        let (chunk, slot) = self.index_of(row)?;
        Some((self.chunks.get_mut(chunk)?, slot))
    }

    /// `row`'s slot.
    pub(super) fn slot(&self, row: RowId) -> Option<Slot> {
        let (chunk, slot) = self.place(row)?;
        chunk.slots().get(slot).copied()
    }

    /// `row`'s slot, mutably.
    pub(super) fn slot_mut(&mut self, row: RowId) -> Option<&mut Slot> {
        let (chunk, slot) = self.place_mut(row)?;
        chunk.slot_mut(slot)
    }

    /// `row`'s key bytes, from its chunk or from out of line; empty for a row that
    /// does not exist. The index reads these, so a lookup never validates UTF-8.
    pub(super) fn key_bytes_at(&self, row: RowId) -> &[u8] {
        let Some((chunk, slot)) = self.place(row) else {
            return &[];
        };
        let spilled = chunk
            .slots()
            .get(slot)
            .is_some_and(|stored| stored.has(KEY_SPILLED));
        if spilled {
            self.spilled_keys.get(&row).map_or(&[], String::as_bytes)
        } else {
            chunk.key(slot)
        }
    }

    /// `row`'s key, for what leaves the store. Every key went in whole from a
    /// `&str`, so its bytes are valid UTF-8 and the empty fallback is never taken.
    pub(super) fn key_at(&self, row: RowId) -> &str {
        std::str::from_utf8(self.key_bytes_at(row)).unwrap_or_default()
    }

    /// `row`'s latest payload, from its chunk or from out of line; empty for a row
    /// that does not exist.
    pub(super) fn payload_at(&self, row: RowId) -> &[u8] {
        let Some((chunk, slot)) = self.place(row) else {
            return &[];
        };
        let spilled = chunk
            .slots()
            .get(slot)
            .is_some_and(|stored| stored.has(PAYLOAD_SPILLED));
        if spilled {
            self.spilled_payloads.get(&row).map_or(&[], Vec::as_slice)
        } else {
            chunk.payload(slot)
        }
    }

    /// `row`'s latest version, copied out.
    pub(super) fn version_at(&self, row: RowId) -> Version {
        let rank = self.slot(row).map_or(0, |slot| slot.rank);
        Version::new(rank, self.payload_at(row).to_vec())
    }

    /// Every row from `from` on, live and dead, in log order. A placeholder chunk
    /// has no slots, so the walk passes over it.
    pub(super) fn walk(&self, from: RowId) -> impl Iterator<Item = (RowId, Slot)> + '_ {
        let skip = chunk_of(from).saturating_sub(self.front_id);
        self.chunks
            .iter()
            .skip(widen(skip))
            .zip(self.front_id.saturating_add(skip)..)
            .flat_map(|(chunk, id)| {
                chunk
                    .slots()
                    .iter()
                    .zip(0..)
                    .map(move |(slot, index)| (row_id(id, index), *slot))
            })
            .skip_while(move |(row, _)| *row < from)
    }

    /// The live rows' ids, sorted by position; rows with one position keep their
    /// log order. On a log already in order the sort is one pass.
    pub(super) fn by_position(&self) -> Vec<RowId> {
        let mut rows: Vec<(u64, RowId)> = self
            .walk(self.front_row())
            .filter(|(_, slot)| slot.status() != RowStatus::Dead)
            .map(|(row, slot)| (slot.position, row))
            .collect();
        rows.sort_by_key(|(position, _)| *position);
        rows.into_iter().map(|(_, row)| row).collect()
    }

    /// Appends `row` at the tail, sealing the tail chunk first when the row does
    /// not fit it, and returns the new row's id. `None`, changing nothing, when
    /// that needs a chunk id past [`ID_LIMIT`].
    pub(super) fn append(&mut self, row: &NewRow<'_>) -> Option<RowId> {
        let stored = Stored::of(row);
        let chunk = self.tail_for(&stored)?;
        let slot = Slot::new(row.rank, row.position, row.status, stored.flags);
        let index = self
            .chunks
            .back_mut()?
            .add_slot(stored.key, stored.payload, slot);
        let id = row_id(chunk, index);
        self.spill(id, row, stored.flags);
        Some(id)
    }

    /// The id of a tail chunk with room for `stored`, opening one when the tail
    /// has none; `None` when that needs a chunk id past [`ID_LIMIT`].
    fn tail_for(&mut self, stored: &Stored<'_>) -> Option<u32> {
        let room = self
            .chunks
            .back()
            .is_some_and(|tail| tail.has_room(stored.key.len(), stored.payload.len()));
        if !room && !self.open_chunk() {
            return None;
        }
        Some(
            self.front_id
                .saturating_add(narrow(self.chunks.len()))
                .saturating_sub(1),
        )
    }

    /// Seals the tail and opens a new one, unless its id would pass [`ID_LIMIT`].
    /// A tail sealed with no live row is spent, and popped if it is the front.
    fn open_chunk(&mut self) -> bool {
        let id = self.front_id.saturating_add(narrow(self.chunks.len()));
        if id >= ID_LIMIT {
            return false;
        }
        if let Some(tail) = self.chunks.back_mut() {
            tail.seal();
        }
        self.chunks.push_back(Chunk::default());
        self.pop_spent_front();
        true
    }

    /// Stores out of line the key or payload `flags` names, under `id`.
    fn spill(&mut self, id: RowId, row: &NewRow<'_>, flags: u32) {
        if flags & KEY_SPILLED != 0 {
            self.spilled_keys.insert(id, row.key.to_owned());
        }
        if flags & PAYLOAD_SPILLED != 0 {
            self.spilled_payloads.insert(id, row.payload.to_vec());
        }
    }

    /// Marks `row` dead and drops what it stored out of line. Then its chunk
    /// tidies itself, and the spent chunks at the front are popped.
    pub(super) fn kill(&mut self, row: RowId) {
        if let Some((chunk, slot)) = self.place_mut(row) {
            chunk.kill_slot(slot);
            chunk.tidy();
        }
        self.spilled_keys.remove(&row);
        self.spilled_payloads.remove(&row);
        self.pop_spent_front();
    }

    /// Pops the spent chunks at the front, then shrinks the deque to fit once its
    /// capacity passes four times its length (at least 1). The tail is never
    /// spent, so it stays even with no live row.
    ///
    /// After a shrink to fit, the deque must double before it grows, or fall to a
    /// quarter before it shrinks again, so each reallocation is paid for by as many
    /// pushes or pops as it copies: amortised O(1). A pocket grown to 100k rows and
    /// drained to 1k keeps at most four times the chunk headers it needs, a few KiB
    /// at most.
    fn pop_spent_front(&mut self) {
        while self.chunks.front().is_some_and(Chunk::is_spent) {
            self.chunks.pop_front();
            self.front_id = self.front_id.saturating_add(1);
        }
        if self.chunks.capacity() > self.chunks.len().max(1).saturating_mul(4) {
            self.chunks.shrink_to_fit();
        }
    }

    /// Replaces `row`'s latest payload with `payload` and returns the one it
    /// replaced. The new payload goes into the chunk's arena when the arena stays
    /// within its limit, and out of line when it would not.
    pub(super) fn replace_payload(&mut self, row: RowId, payload: Vec<u8>) -> Vec<u8> {
        let old = self.take_payload(row);
        let Some((chunk, slot)) = self.place_mut(row) else {
            return old;
        };
        let fits = chunk.payload_fits(slot, payload.len());
        chunk.splice_payload(slot, if fits { &payload } else { &[] });
        let Some(stored) = chunk.slot_mut(slot) else {
            return old;
        };
        if fits {
            stored.lower(PAYLOAD_SPILLED);
        } else {
            stored.raise(PAYLOAD_SPILLED);
            self.spilled_payloads.insert(row, payload);
        }
        old
    }

    /// `row`'s latest payload: moved out when it is stored out of line, copied out
    /// of the arena when it is not.
    fn take_payload(&mut self, row: RowId) -> Vec<u8> {
        self.spilled_payloads
            .remove(&row)
            .unwrap_or_else(|| self.payload_at(row).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::{ID_LIMIT, NewRow, RowId, Rows, row_id};
    use crate::log::chunk::{KEY_LIMIT, KEY_SPILLED, PAYLOAD_LIMIT, PAYLOAD_SPILLED, RowStatus};

    fn queued<'a>(key: &'a str, payload: &'a [u8], position: u64) -> NewRow<'a> {
        NewRow {
            key,
            rank: 1,
            payload,
            position,
            status: RowStatus::Queued,
        }
    }

    fn appended(rows: &mut Rows, key: &str, payload: &[u8], position: u64) -> RowId {
        rows.append(&queued(key, payload, position)).unwrap()
    }

    fn keys_from(rows: &Rows, from: RowId) -> Vec<String> {
        rows.walk(from)
            .map(|(row, _)| rows.key_at(row).to_owned())
            .collect()
    }

    #[test]
    fn rows_fill_a_chunk_and_then_open_the_next() {
        let mut rows = Rows::default();
        assert_eq!((rows.front_row(), rows.next_row()), (0, 0));
        let ids: Vec<RowId> = (0..5)
            .map(|position| appended(&mut rows, "k", b"v", position))
            .collect();
        assert_eq!(ids, [0, 1, 2, 3, row_id(1, 0)]);
        assert_eq!(rows.next_row(), row_id(1, 1));
        assert_eq!(rows.chunks.len(), 2);
    }

    #[test]
    fn a_payload_that_would_pass_the_arena_limit_seals_the_tail_early() {
        let mut rows = Rows::default();
        appended(&mut rows, "a", &[1; 20], 1);
        assert_eq!(rows.next_row(), 1);
        let second = appended(&mut rows, "b", &[2; 20], 2);
        assert_eq!(second, row_id(1, 0));
        assert_eq!(rows.payload_at(0), [1; 20]);
        assert_eq!(rows.payload_at(second), [2; 20]);
    }

    #[test]
    fn an_oversize_key_and_payload_are_stored_out_of_line() {
        let mut rows = Rows::default();
        let key = "k".repeat(KEY_LIMIT + 1);
        let payload = vec![7; PAYLOAD_LIMIT + 1];
        let row = appended(&mut rows, &key, &payload, 1);
        let small = appended(&mut rows, "s", b"x", 2);
        assert_eq!(
            (rows.key_at(row), rows.payload_at(row)),
            (key.as_str(), payload.as_slice())
        );
        let slot = rows.slot(row).unwrap();
        assert!(slot.has(KEY_SPILLED) && slot.has(PAYLOAD_SPILLED));
        assert_eq!(rows.chunks[0].key(0), b"");
        assert_eq!(
            (rows.key_at(small), rows.payload_at(small)),
            ("s", &b"x"[..])
        );
        rows.kill(row);
        assert!(rows.spilled_keys.is_empty() && rows.spilled_payloads.is_empty());
    }

    #[test]
    fn a_payload_moves_out_of_line_and_back() {
        let mut rows = Rows::default();
        let row = appended(&mut rows, "a", b"small", 1);
        let after = appended(&mut rows, "b", b"next", 2);
        let large = vec![9; PAYLOAD_LIMIT];
        assert_eq!(rows.replace_payload(row, large.clone()), b"small");
        assert!(rows.slot(row).unwrap().has(PAYLOAD_SPILLED));
        assert_eq!(rows.payload_at(row), large.as_slice());
        assert_eq!(rows.payload_at(after), b"next");
        assert_eq!(rows.replace_payload(row, b"back".to_vec()), large);
        assert!(!rows.slot(row).unwrap().has(PAYLOAD_SPILLED));
        assert!(rows.spilled_payloads.is_empty());
        assert_eq!(
            (rows.payload_at(row), rows.payload_at(after)),
            (&b"back"[..], &b"next"[..])
        );
    }

    #[test]
    fn the_front_chunk_is_popped_once_its_rows_die() {
        let mut rows = Rows::default();
        let ids: Vec<RowId> = (0..6)
            .map(|position| appended(&mut rows, &position.to_string(), b"", position))
            .collect();
        for row in &ids[..3] {
            rows.kill(*row);
        }
        assert_eq!(rows.front_row(), 0);
        rows.kill(ids[3]);
        assert_eq!(rows.front_row(), row_id(1, 0));
        assert_eq!(keys_from(&rows, 0), ["4", "5"]);
        assert_eq!(appended(&mut rows, "6", b"", 6), row_id(1, 2));
        for row in [ids[4], ids[5], row_id(1, 2)] {
            rows.kill(row);
        }
        assert_eq!(rows.chunks.len(), 1, "the tail stays with no live row");
        assert_eq!(rows.next_row(), row_id(1, 3));
        assert_eq!(appended(&mut rows, "7", b"", 7), row_id(1, 3));
        rows.kill(row_id(1, 3));
        assert_eq!(appended(&mut rows, "8", b"", 8), row_id(2, 0));
        assert_eq!(rows.front_row(), row_id(2, 0), "the spent tail was popped");
        assert_eq!(keys_from(&rows, 0), ["8"]);
    }

    #[test]
    fn a_dead_middle_chunk_frees_its_buffers_and_keeps_the_ids_after_it() {
        let mut rows = Rows::default();
        for position in 0..12 {
            appended(&mut rows, &position.to_string(), b"p", position);
        }
        for slot in 0..4 {
            rows.kill(row_id(1, slot));
        }
        assert!(rows.chunks[1].is_spent() && rows.chunks[1].slots().is_empty());
        assert_eq!(rows.front_row(), 0);
        let keys = keys_from(&rows, row_id(0, 2));
        assert_eq!(keys, ["2", "3", "8", "9", "10", "11"]);
        assert_eq!(rows.key_at(row_id(2, 1)), "9");
        assert_eq!(rows.slot(row_id(1, 0)).map(|slot| slot.rank), None);
    }

    #[test]
    fn the_deque_shrinks_once_its_capacity_passes_four_times_its_length() {
        let mut rows = Rows::default();
        for position in 0..64 {
            appended(&mut rows, "k", b"", position);
        }
        assert!(rows.chunks.capacity() >= 16);
        for row in rows.by_position().into_iter().take(60) {
            rows.kill(row);
        }
        assert_eq!(rows.chunks.len(), 1);
        assert!(rows.chunks.capacity() <= 4, "{}", rows.chunks.capacity());
    }

    #[test]
    fn append_refuses_a_chunk_id_past_the_limit_and_changes_nothing() {
        let mut rows = Rows::default();
        let limit = u64::from(ID_LIMIT) * 4;
        for position in 0..limit {
            appended(&mut rows, "k", b"", position);
        }
        let next = rows.next_row();
        assert_eq!(rows.append(&queued("k", b"", limit)), None);
        assert_eq!(rows.next_row(), next);
        assert_eq!(rows.chunks.len(), 32);
    }

    #[test]
    fn by_position_lists_the_live_rows_in_position_order() {
        let mut rows = Rows::default();
        let five = appended(&mut rows, "five", b"", 5);
        let three = appended(&mut rows, "three", b"", 3);
        let nine = appended(&mut rows, "nine", b"", 9);
        let one = appended(&mut rows, "one", b"", 1);
        rows.kill(nine);
        assert_eq!(rows.by_position(), [one, three, five]);
    }

    #[test]
    fn a_missing_row_reads_as_empty() {
        let mut rows = Rows::default();
        assert_eq!((rows.key_at(3), rows.payload_at(3)), ("", &b""[..]));
        assert_eq!(rows.version_at(3).payload, b"");
        appended(&mut rows, "a", b"x", 0);
        rows.kill(row_id(5, 0));
        assert_eq!(rows.replace_payload(row_id(5, 0), b"y".to_vec()), b"");
        assert_eq!(rows.payload_at(0), b"x");
    }
}
