//! A chunk of the log: up to 64 consecutive rows, as slots and two arenas.
//!
//! A slot is 24 bytes: the latest rank, the position, and where the row's key and
//! payload end in the chunk's key arena and payload arena; a row starts where the
//! slot before it ends, or at 0. The key end's top bits hold the row's status and
//! its flags. A row that dies keeps its bytes until a repack gives it zero length,
//! and a repack rewrites the arenas without moving a slot, so no row id changes.
//!
//! The limits are small under test, so the unit tests reach the seal, the spill and
//! the renumbering with a handful of rows.

use std::ops::Range;

use libpachinko::records::RecordStatus;

/// The bits of a row id that name its slot: 64 rows a chunk, or 4 under test.
pub(super) const SLOT_BITS: u32 = if cfg!(test) { 2 } else { 6 };
/// The most rows a chunk holds.
const ROWS: usize = 1 << SLOT_BITS;

/// Where the status sits in a key word, above the three flags.
const STATUS_SHIFT: u32 = 30;
/// A key word's status bits.
const STATUS_BITS: u32 = 0b11 << STATUS_SHIFT;
/// Flag: the row is held, and its held version is in the store's superseded table.
pub(super) const SUPERSEDED: u32 = 1 << 29;
/// Flag: the row's key is stored out of line.
pub(super) const KEY_SPILLED: u32 = 1 << 28;
/// Flag: the row's payload is stored out of line.
pub(super) const PAYLOAD_SPILLED: u32 = 1 << 27;
/// A key word's bits that hold the key's end.
const KEY_END_BITS: u32 = PAYLOAD_SPILLED - 1;

/// The longest a key arena grows: the key end's 27 bits, or 16 bytes under test.
pub(super) const KEY_LIMIT: usize = if cfg!(test) { 16 } else { 0x07FF_FFFF };
/// The longest a payload arena grows: the payload end's 32 bits, or 32 bytes under
/// test.
pub(super) const PAYLOAD_LIMIT: usize = if cfg!(test) { 32 } else { 0xFFFF_FFFF };

/// The fewest slots a tail chunk grows by. An eighth of a small tail is under one
/// row, so the +1/8 rule alone would reallocate on every row; the floors cost at
/// most 4 slots and 64 bytes a pocket.
const SLOT_FLOOR: usize = 4;
/// The fewest bytes a tail chunk's arena grows by, for the same reason.
const BYTE_FLOOR: usize = 64;

/// The fewest dead bytes that make a repack worth its pass: 64, or 4 under test so
/// the unit tests reach it. With the sixteenth rule this bounds a chunk's dead
/// bytes, between repacks, to at most max(live bytes / 16, this floor).
const REPACK_FLOOR: usize = if cfg!(test) { 4 } else { 64 };

/// A row's status, as the top two bits of its slot's key word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowStatus {
    /// Waiting to be pulled.
    Queued,
    /// Handed out under the pocket's lease.
    Held,
    /// Pullable again once the lease ends.
    Deferred,
    /// Settled, or moved to a later row; its bytes wait for a repack.
    Dead,
}

impl RowStatus {
    /// This status's bits in a key word.
    const fn bits(self) -> u32 {
        let code = match self {
            Self::Queued => 0,
            Self::Held => 1,
            Self::Deferred => 2,
            Self::Dead => 3,
        };
        code << STATUS_SHIFT
    }

    /// The status a key word holds.
    const fn of_word(word: u32) -> Self {
        match word >> STATUS_SHIFT {
            0 => Self::Queued,
            1 => Self::Held,
            2 => Self::Deferred,
            _ => Self::Dead,
        }
    }

    /// The status the trait reports. Only a live row is ever reported, so a dead
    /// one never reaches here; it would read as queued.
    pub(super) const fn reported(self) -> RecordStatus {
        match self {
            Self::Held => RecordStatus::Held,
            Self::Deferred => RecordStatus::Deferred,
            Self::Queued | Self::Dead => RecordStatus::Queued,
        }
    }
}

/// One row: its latest rank, its position, and the ends of its key and payload,
/// with its status and flags in the key word's top bits.
#[derive(Clone, Copy, Debug)]
pub(super) struct Slot {
    /// The latest rank.
    pub(super) rank: i64,
    /// Where the record stands among its pocket's records.
    pub(super) position: u64,
    /// The key's end in the low 27 bits, then the three flags, then the status.
    key_word: u32,
    /// The payload's end.
    payload_end: u32,
}

impl Slot {
    /// A slot for a new row; its chunk sets the ends when it takes the row.
    pub(super) const fn new(rank: i64, position: u64, status: RowStatus, flags: u32) -> Self {
        Self {
            rank,
            position,
            key_word: status.bits() | (flags & !STATUS_BITS & !KEY_END_BITS),
            payload_end: 0,
        }
    }

    /// The row's status.
    pub(super) const fn status(&self) -> RowStatus {
        RowStatus::of_word(self.key_word)
    }

    /// Whether `flag` is raised.
    pub(super) const fn has(&self, flag: u32) -> bool {
        self.key_word & flag != 0
    }

    /// Sets the row's status, leaving its flags alone.
    pub(super) const fn restate(&mut self, status: RowStatus) {
        self.key_word = (self.key_word & !STATUS_BITS) | status.bits();
    }

    /// Raises `flag`.
    pub(super) const fn raise(&mut self, flag: u32) {
        self.key_word |= flag;
    }

    /// Lowers `flag`.
    pub(super) const fn lower(&mut self, flag: u32) {
        self.key_word &= !flag;
    }

    /// Where the row's key and payload end in their arenas.
    fn ends(&self) -> (usize, usize) {
        (widen(self.key_word & KEY_END_BITS), widen(self.payload_end))
    }

    /// Sets where the row's key and payload end.
    const fn set_ends(&mut self, key_end: u32, payload_end: u32) {
        self.key_word = (self.key_word & !KEY_END_BITS) | (key_end & KEY_END_BITS);
        self.payload_end = payload_end;
    }
}

/// Up to 64 consecutive rows of the log: their slots, a key arena and a payload
/// arena, and what the dead rows still occupy.
///
/// The tail chunk takes new rows; every other chunk is sealed. A sealed chunk's
/// buffers are shrunk to fit, and a sealed chunk whose rows have all died frees
/// them and stays as an empty placeholder, so the chunks after it keep their ids.
#[derive(Clone, Debug, Default)]
pub(super) struct Chunk {
    slots: Vec<Slot>,
    /// The keys' UTF-8 bytes. Bytes rather than a `String`, so a repack can move
    /// them in place; each key is copied in whole from a `&str`, so every span
    /// is valid UTF-8.
    keys: Vec<u8>,
    payloads: Vec<u8>,
    /// Key bytes that dead rows still occupy.
    dead_keys: u32,
    /// Payload bytes that dead rows still occupy.
    dead_payloads: u32,
    /// Rows that are not dead.
    live: u8,
    /// Whether the chunk takes no more rows.
    sealed: bool,
}

impl Chunk {
    /// The chunk's slots, live and dead, in row order.
    pub(super) fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// The slot at `slot`, if the chunk has one there.
    pub(super) fn slot_mut(&mut self, slot: usize) -> Option<&mut Slot> {
        self.slots.get_mut(slot)
    }

    /// Whether the chunk is sealed and none of its rows is live: it takes no row
    /// and holds none, so the front of the log can drop it.
    pub(super) const fn is_spent(&self) -> bool {
        self.sealed && self.live == 0
    }

    /// Whether the chunk takes a row whose stored key and payload are `key` and
    /// `payload` bytes long: it is the tail, it has a free slot, and neither arena
    /// would pass its limit.
    pub(super) const fn has_room(&self, key: usize, payload: usize) -> bool {
        !self.sealed
            && self.slots.len() < ROWS
            && self.keys.len().saturating_add(key) <= KEY_LIMIT
            && self.payloads.len().saturating_add(payload) <= PAYLOAD_LIMIT
    }

    /// Appends a row storing `key` and `payload`, and returns its slot. The caller
    /// checks [`Chunk::has_room`] first.
    pub(super) fn add_slot(&mut self, key: &str, payload: &[u8], slot: Slot) -> u32 {
        self.reserve_row(key.len(), payload.len());
        self.keys.extend_from_slice(key.as_bytes());
        self.payloads.extend_from_slice(payload);
        self.push_slot(slot)
    }

    /// Reserves room for one more row.
    fn reserve_row(&mut self, key: usize, payload: usize) {
        let slots = self.growth(self.slots.len(), self.slots.capacity(), 1, SLOT_FLOOR);
        let keys = self.growth(self.keys.len(), self.keys.capacity(), key, BYTE_FLOOR);
        let payloads = self.growth(
            self.payloads.len(),
            self.payloads.capacity(),
            payload,
            BYTE_FLOOR,
        );
        self.slots.reserve_exact(slots);
        self.keys.reserve_exact(keys);
        self.payloads.reserve_exact(payloads);
    }

    /// How many more elements to reserve for `additional` more: none when they
    /// already fit; exactly `additional` in a sealed chunk; and in the tail, an
    /// eighth of `len` (at least `floor`) when that is more than `additional`.
    ///
    /// Never doubling is what keeps the store within its memory budget: the tail of
    /// a 1k-record pocket of 1 KiB payloads holds about 40 KiB, and doubling would
    /// leave up to that much again unused, where an eighth leaves at most 5 KiB.
    fn growth(&self, len: usize, capacity: usize, additional: usize, floor: usize) -> usize {
        if capacity.saturating_sub(len) >= additional {
            0
        } else if self.sealed {
            additional
        } else {
            additional.max(len / 8).max(floor)
        }
    }

    /// Sets the new slot's ends to the arenas' lengths, pushes it, and returns its
    /// index.
    fn push_slot(&mut self, mut slot: Slot) -> u32 {
        slot.set_ends(narrow(self.keys.len()), narrow(self.payloads.len()));
        let index = narrow(self.slots.len());
        self.slots.push(slot);
        self.live = self.live.saturating_add(1);
        index
    }

    /// Seals the chunk: it takes no more rows, and its buffers shrink to fit, or
    /// are freed when none of its rows is live.
    pub(super) fn seal(&mut self) {
        self.sealed = true;
        if self.live == 0 {
            self.release();
        } else {
            self.slots.shrink_to_fit();
            self.keys.shrink_to_fit();
            self.payloads.shrink_to_fit();
        }
    }

    /// Frees the buffers of a sealed chunk with no live row, leaving an empty
    /// placeholder that keeps the ids of the chunks after it.
    fn release(&mut self) {
        *self = Self {
            sealed: true,
            ..Self::default()
        };
    }

    /// The byte ranges of the key and the payload of the row at `slot`.
    fn spans(&self, slot: usize) -> (Range<usize>, Range<usize>) {
        let start = slot
            .checked_sub(1)
            .and_then(|before| self.slots.get(before))
            .map_or((0, 0), Slot::ends);
        let end = self.slots.get(slot).map_or(start, Slot::ends);
        (start.0..end.0, start.1..end.1)
    }

    /// The key bytes the row at `slot` stores in the arena: empty when it is
    /// spilled.
    pub(super) fn key(&self, slot: usize) -> &[u8] {
        self.keys.get(self.spans(slot).0).unwrap_or_default()
    }

    /// The payload the row at `slot` stores in the arena: empty when it is spilled.
    pub(super) fn payload(&self, slot: usize) -> &[u8] {
        self.payloads.get(self.spans(slot).1).unwrap_or_default()
    }

    /// Marks the row at `slot` dead, counting the bytes it leaves behind. A row
    /// already dead is left alone.
    pub(super) fn kill_slot(&mut self, slot: usize) {
        let (keys, payloads) = self.spans(slot);
        let Some(row) = self
            .slots
            .get_mut(slot)
            .filter(|row| row.status() != RowStatus::Dead)
        else {
            return;
        };
        row.restate(RowStatus::Dead);
        row.lower(SUPERSEDED);
        self.dead_keys = self.dead_keys.saturating_add(narrow(keys.len()));
        self.dead_payloads = self.dead_payloads.saturating_add(narrow(payloads.len()));
        self.live = self.live.saturating_sub(1);
    }

    /// Tidies the chunk after a row died. A sealed chunk with no live row frees its
    /// buffers. The tail with no live row empties its arenas but keeps their
    /// capacity and its slots, so a pocket that churns one record at a time
    /// reuses one chunk instead of opening one per record. Any other chunk repacks
    /// once its dead bytes pass a sixteenth of its live bytes, so dead bytes stay
    /// within about 6% of a chunk and a repack costs amortised O(1) per byte that
    /// died.
    pub(super) fn tidy(&mut self) {
        if self.is_spent() {
            self.release();
        } else if self.live == 0 {
            self.empty_arenas();
        } else if self.repack_due() {
            self.repack();
        }
    }

    /// Empties both arenas of a chunk whose rows are all dead, keeping their
    /// capacity; every dead row's key and payload become empty.
    fn empty_arenas(&mut self) {
        self.keys.clear();
        self.payloads.clear();
        for slot in &mut self.slots {
            slot.set_ends(0, 0);
        }
        self.dead_keys = 0;
        self.dead_payloads = 0;
    }

    /// Whether the dead rows' bytes pass a sixteenth of the live rows' bytes and
    /// the floor. The floor keeps a small chunk drained front to back from
    /// repacking on almost every settle.
    fn repack_due(&self) -> bool {
        let dead = widen(self.dead_keys).saturating_add(widen(self.dead_payloads));
        let live = self
            .keys
            .len()
            .saturating_add(self.payloads.len())
            .saturating_sub(dead);
        dead > live / 16 && dead >= REPACK_FLOOR
    }

    /// Moves the live rows' bytes down over the dead rows' bytes, in place, and
    /// rewrites every slot's ends; no slot moves, and a dead row's key and payload
    /// become empty. Nothing is allocated. Then each arena gives back its spare
    /// capacity once that passes a quarter of its length.
    fn repack(&mut self) {
        let mut start = (0, 0);
        let mut kept = (0, 0);
        for slot in &mut self.slots {
            let end = slot.ends();
            if slot.status() != RowStatus::Dead {
                kept.0 = move_down(&mut self.keys, start.0..end.0, kept.0);
                kept.1 = move_down(&mut self.payloads, start.1..end.1, kept.1);
            }
            start = end;
            slot.set_ends(narrow(kept.0), narrow(kept.1));
        }
        self.keys.truncate(kept.0);
        self.payloads.truncate(kept.1);
        let sealed = self.sealed;
        trim_spare(&mut self.keys, sealed);
        trim_spare(&mut self.payloads, sealed);
        self.dead_keys = 0;
        self.dead_payloads = 0;
    }

    /// Whether the payload arena stays within its limit when the row at `slot`
    /// stores `len` bytes in place of its current payload.
    pub(super) fn payload_fits(&self, slot: usize, len: usize) -> bool {
        let (_, span) = self.spans(slot);
        self.payloads
            .len()
            .saturating_sub(span.len())
            .saturating_add(len)
            <= PAYLOAD_LIMIT
    }

    /// Replaces the payload the row at `slot` stores with `payload`, shifting the
    /// rows after it. The caller checks [`Chunk::payload_fits`] first.
    pub(super) fn splice_payload(&mut self, slot: usize, payload: &[u8]) {
        let (_, span) = self.spans(slot);
        if slot >= self.slots.len() || span.end > self.payloads.len() {
            return;
        }
        let (removed, added) = (span.len(), payload.len());
        let more = self.growth(
            self.payloads.len(),
            self.payloads.capacity(),
            added.saturating_sub(removed),
            BYTE_FLOOR,
        );
        self.payloads.reserve_exact(more);
        drop(self.payloads.splice(span, payload.iter().copied()));
        for later in self.slots.iter_mut().skip(slot) {
            let end = widen(later.payload_end)
                .saturating_sub(removed)
                .saturating_add(added);
            later.payload_end = narrow(end);
        }
    }
}

/// Copies `arena[span]` down to `to`, which is never past `span.start`, and
/// returns where the copy ends. A span outside the arena moves nothing.
fn move_down(arena: &mut [u8], span: Range<usize>, to: usize) -> usize {
    let len = span.len();
    if span.end <= arena.len() && to <= span.start {
        arena.copy_within(span, to);
        to.saturating_add(len)
    } else {
        to
    }
}

/// Gives back an arena's spare capacity once it passes a quarter of the length:
/// all of it in a sealed chunk, which never grows again, and all but an eighth in
/// the tail, which the next rows would otherwise grow straight back. A shrink
/// needs the length to fall by a fifth since the last one, so a chunk drained
/// row by row shrinks O(log) times.
fn trim_spare(arena: &mut Vec<u8>, sealed: bool) {
    let len = arena.len();
    if arena.capacity().saturating_sub(len) > len / 4 {
        arena.shrink_to(if sealed {
            len
        } else {
            len.saturating_add(len / 8)
        });
    }
}

/// A `u32` as a `usize`. Every target this builds for is at least 32 bits wide, so
/// the fallback is never taken.
pub(super) fn widen(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// An arena length or a slot index as a `u32`. The limits keep both within `u32`,
/// so the fallback, saturating, is never taken.
pub(super) fn narrow(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        Chunk, KEY_LIMIT, KEY_SPILLED, PAYLOAD_LIMIT, PAYLOAD_SPILLED, ROWS, RowStatus, SUPERSEDED,
        Slot, move_down, trim_spare,
    };
    use libpachinko::records::RecordStatus;

    fn slot(rank: i64) -> Slot {
        Slot::new(rank, 0, RowStatus::Queued, 0)
    }

    fn filled(rows: &[(&str, &str)]) -> Chunk {
        let mut chunk = Chunk::default();
        for (rank, (key, payload)) in (0..).zip(rows) {
            chunk.add_slot(key, payload.as_bytes(), slot(rank));
        }
        chunk
    }

    fn read_back(chunk: &Chunk) -> Vec<(String, Vec<u8>)> {
        (0..chunk.slots().len())
            .map(|index| {
                let key = String::from_utf8(chunk.key(index).to_vec()).unwrap();
                (key, chunk.payload(index).to_vec())
            })
            .collect()
    }

    fn pairs(rows: &[(&str, &str)]) -> Vec<(String, Vec<u8>)> {
        rows.iter()
            .map(|(key, payload)| ((*key).to_owned(), payload.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn a_slot_keeps_its_status_and_flags_apart_from_its_ends() {
        let mut slot = Slot::new(3, 9, RowStatus::Held, KEY_SPILLED);
        slot.set_ends(5, 7);
        assert_eq!(slot.status(), RowStatus::Held);
        assert!(slot.has(KEY_SPILLED) && !slot.has(PAYLOAD_SPILLED));
        slot.raise(SUPERSEDED);
        slot.restate(RowStatus::Deferred);
        slot.lower(KEY_SPILLED);
        assert_eq!(slot.status(), RowStatus::Deferred);
        assert!(slot.has(SUPERSEDED) && !slot.has(KEY_SPILLED));
        assert_eq!(slot.ends(), (5, 7));
        assert_eq!((slot.rank, slot.position), (3, 9));
    }

    #[test]
    fn every_status_survives_its_key_word_and_reports_as_the_trait_names_it() {
        for (status, reported) in [
            (RowStatus::Queued, RecordStatus::Queued),
            (RowStatus::Held, RecordStatus::Held),
            (RowStatus::Deferred, RecordStatus::Deferred),
            (RowStatus::Dead, RecordStatus::Queued),
        ] {
            let mut slot = Slot::new(0, 0, RowStatus::Queued, SUPERSEDED);
            slot.restate(status);
            assert_eq!(slot.status(), status);
            assert!(slot.has(SUPERSEDED));
            assert_eq!(status.reported(), reported);
        }
    }

    #[test]
    fn rows_read_back_their_keys_and_payloads() {
        let rows = [("a", "one"), ("", ""), ("ccc", "3")];
        let chunk = filled(&rows);
        assert_eq!(read_back(&chunk), pairs(&rows));
        assert_eq!(chunk.slots()[2].rank, 2);
        assert_eq!(chunk.key(ROWS), b"");
    }

    #[test]
    fn a_chunk_seals_at_its_row_limit() {
        let mut chunk = Chunk::default();
        for _ in 0..ROWS {
            assert!(chunk.has_room(1, 1));
            chunk.add_slot("k", b"v", slot(1));
        }
        assert!(!chunk.has_room(0, 0));
    }

    #[test]
    fn a_chunk_has_no_room_past_either_arena_limit() {
        let chunk = filled(&[("abcdefghij", "0123456789")]);
        assert!(chunk.has_room(KEY_LIMIT - 10, PAYLOAD_LIMIT - 10));
        assert!(!chunk.has_room(KEY_LIMIT - 9, 0));
        assert!(!chunk.has_room(0, PAYLOAD_LIMIT - 9));
    }

    #[test]
    fn a_sealed_chunk_takes_no_row_and_keeps_no_spare_capacity() {
        let mut chunk = filled(&[("a", "x")]);
        chunk.seal();
        assert!(!chunk.has_room(0, 0));
        assert_eq!(chunk.slots.capacity(), 1);
        assert_eq!(chunk.keys.capacity(), 1);
        assert_eq!(chunk.payloads.capacity(), 1);
    }

    #[test]
    fn a_dead_row_keeps_its_bytes_until_a_repack_drops_them() {
        let mut chunk = filled(&[("aa", "11"), ("bb", "22"), ("cc", "33")]);
        chunk.kill_slot(1);
        assert_eq!(chunk.slots()[1].status(), RowStatus::Dead);
        assert_eq!(
            (chunk.dead_keys, chunk.dead_payloads, chunk.live),
            (2, 2, 2)
        );
        assert_eq!(chunk.key(1), b"bb");
        chunk.tidy();
        assert_eq!(
            (chunk.keys.as_slice(), chunk.payloads.as_slice()),
            (&b"aacc"[..], &b"1133"[..])
        );
        assert_eq!((chunk.dead_keys, chunk.dead_payloads), (0, 0));
        let rows = [("aa", "11"), ("", ""), ("cc", "33")];
        assert_eq!(read_back(&chunk), pairs(&rows));
    }

    #[test]
    fn a_repack_waits_until_dead_bytes_pass_a_sixteenth_of_live_bytes() {
        let mut chunk = filled(&[("k", "0".repeat(31).as_str()), ("x", "")]);
        chunk.kill_slot(1);
        chunk.tidy();
        assert_eq!(chunk.keys, b"kx");
        assert_eq!(chunk.dead_keys, 1);
    }

    #[test]
    fn a_repack_waits_for_the_floor_of_dead_bytes() {
        let mut chunk = filled(&[("a", "1"), ("b", "2"), ("c", "3")]);
        chunk.kill_slot(1);
        chunk.tidy();
        assert_eq!(chunk.keys, b"abc");
        assert_eq!(chunk.dead_keys, 1);
    }

    #[test]
    fn a_repack_moves_the_live_bytes_down_in_place() {
        let rows = [("b", "333"), ("aaaa", "11111111"), ("aaaa", "22222222")];
        let mut chunk = filled(&rows);
        chunk.keys.shrink_to_fit();
        chunk.payloads.shrink_to_fit();
        let (keys, payloads) = (chunk.keys.as_ptr(), chunk.payloads.as_ptr());
        chunk.kill_slot(0);
        chunk.tidy();
        assert_eq!((chunk.dead_keys, chunk.dead_payloads), (0, 0));
        assert_eq!(
            (chunk.keys.as_ptr(), chunk.payloads.as_ptr()),
            (keys, payloads)
        );
        let after = [("", ""), ("aaaa", "11111111"), ("aaaa", "22222222")];
        assert_eq!(read_back(&chunk), pairs(&after));
    }

    #[test]
    fn a_move_down_outside_its_arena_moves_nothing() {
        let mut arena = b"..xy".to_vec();
        assert_eq!(move_down(&mut arena, 2..4, 0), 2);
        assert_eq!(arena, b"xyxy");
        assert_eq!(move_down(&mut arena, 3..9, 0), 0);
        assert_eq!(move_down(&mut arena, 1..2, 3), 3);
        assert_eq!(arena, b"xyxy");
    }

    #[test]
    fn spare_capacity_goes_back_once_it_passes_a_quarter_of_the_length() {
        let mut kept = Vec::<u8>::with_capacity(100);
        kept.resize(80, 0);
        trim_spare(&mut kept, true);
        assert_eq!(kept.capacity(), 100);
        let mut sealed = Vec::<u8>::with_capacity(100);
        sealed.resize(79, 0);
        trim_spare(&mut sealed, true);
        assert_eq!(sealed.capacity(), 79);
        let mut tail = Vec::<u8>::with_capacity(100);
        tail.resize(64, 0);
        trim_spare(&mut tail, false);
        assert_eq!(tail.capacity(), 72);
    }

    #[test]
    fn killing_a_dead_row_changes_nothing() {
        let mut chunk = filled(&[("a", "1"), ("b", "2")]);
        chunk.kill_slot(0);
        chunk.kill_slot(0);
        chunk.kill_slot(ROWS);
        assert_eq!(
            (chunk.live, chunk.dead_keys, chunk.dead_payloads),
            (1, 1, 1)
        );
    }

    #[test]
    fn a_sealed_chunk_whose_rows_all_die_frees_its_buffers() {
        let mut chunk = filled(&[("a", "1"), ("b", "2")]);
        chunk.seal();
        chunk.kill_slot(0);
        chunk.tidy();
        assert!(!chunk.is_spent());
        chunk.kill_slot(1);
        chunk.tidy();
        assert!(chunk.is_spent() && chunk.slots().is_empty());
        assert_eq!(chunk.slots.capacity(), 0);
        assert_eq!(chunk.keys.capacity(), 0);
        assert_eq!(chunk.payloads.capacity(), 0);
        assert!(!chunk.has_room(0, 0));
    }

    #[test]
    fn a_chunk_sealed_with_no_live_row_frees_its_buffers_at_once() {
        let mut chunk = filled(&[("a", "1")]);
        chunk.kill_slot(0);
        chunk.seal();
        assert!(chunk.is_spent() && chunk.slots().is_empty());
        assert_eq!(chunk.keys.capacity(), 0);
    }

    #[test]
    fn the_tail_whose_rows_all_die_keeps_its_slots_and_takes_more_rows() {
        let mut chunk = filled(&[("a", "1"), ("b", "2")]);
        chunk.kill_slot(0);
        chunk.kill_slot(1);
        chunk.tidy();
        assert!(!chunk.is_spent() && chunk.live == 0);
        assert_eq!(chunk.slots().len(), 2);
        assert!(chunk.keys.is_empty() && chunk.payloads.is_empty());
        assert!(chunk.keys.capacity() > 0 && chunk.payloads.capacity() > 0);
        assert_eq!((chunk.dead_keys, chunk.dead_payloads), (0, 0));
        assert!(chunk.has_room(1, 1));
        chunk.add_slot("c", b"3", slot(2));
        assert_eq!((chunk.key(2), chunk.payload(2)), (&b"c"[..], &b"3"[..]));
    }

    #[test]
    fn a_payload_splice_shifts_the_rows_after_it() {
        let mut chunk = filled(&[("a", "11"), ("b", "22"), ("c", "33")]);
        chunk.splice_payload(1, b"2222");
        let longer = [("a", "11"), ("b", "2222"), ("c", "33")];
        assert_eq!(read_back(&chunk), pairs(&longer));
        chunk.splice_payload(1, b"");
        chunk.splice_payload(0, b"1");
        let shorter = [("a", "1"), ("b", ""), ("c", "33")];
        assert_eq!(read_back(&chunk), pairs(&shorter));
        chunk.splice_payload(ROWS, b"ignored");
        assert_eq!(read_back(&chunk), pairs(&shorter));
    }

    #[test]
    fn a_payload_fits_while_the_arena_stays_within_its_limit() {
        let chunk = filled(&[("a", "0123456789"), ("b", "0123456789")]);
        assert!(chunk.payload_fits(1, PAYLOAD_LIMIT - 10));
        assert!(!chunk.payload_fits(1, PAYLOAD_LIMIT - 9));
    }

    #[test]
    fn a_tail_grows_by_an_eighth_and_a_sealed_chunk_by_what_it_needs() {
        let tail = Chunk::default();
        assert_eq!(tail.growth(800, 800, 10, 64), 100);
        assert_eq!(tail.growth(80, 80, 10, 64), 64);
        assert_eq!(tail.growth(80, 100, 10, 64), 0);
        assert_eq!(tail.growth(80, 80, 200, 64), 200);
        let sealed = Chunk {
            sealed: true,
            ..Chunk::default()
        };
        assert_eq!(sealed.growth(800, 800, 10, 64), 10);
    }
}
