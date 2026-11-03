//! The records contract over `LogRecords`, and a differential test that drives
//! `LogRecords` and the reference store, `MemoryRecords`, through the same random
//! steps and requires the same answer from both after every one.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use libpachinko::records::contract::check_contract;
use libpachinko::records::{HeldRecord, MemoryRecords, PocketRecords, RecordView, Version};
use pachinko::arrow::held_records;
use pachinko::log::LogRecords;

/// Steps each seed's walk takes.
const STEPS: usize = 10_000;
/// Every this many steps, and at the end, both stores are compared in full.
const FULL_CHECK_EVERY: usize = 64;
/// The key pool. Keys recur once settled, so the walk churns rather than only grows.
const KEY_POOL: u16 = 300;
/// The largest pull limit tried; limits run from 0 to it.
const MAX_LIMIT: usize = 150;
/// The largest payload tried, in bytes; sizes run from 0 to it.
const MAX_PAYLOAD: usize = 200;
/// A chunk's row count in `LogRecords`; every walk must grow past several chunks.
const CHUNK_ROWS: usize = 64;
/// Scattered positions are drawn from `0..NARROW_POSITIONS` half the time, so a
/// position freed by a settle or a defer is soon handed out again.
const NARROW_POSITIONS: u16 = 1024;

/// A 64-bit linear congruential generator: seeded, so a failing seed replays
/// exactly, and small, so the test needs no new dependency. Its draws read the
/// state's top bits, the well-mixed ones.
#[derive(Debug)]
struct Lcg(u64);

impl Lcg {
    /// Knuth's MMIX multiplier.
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    /// Knuth's MMIX increment.
    const INCREMENT: u64 = 1_442_695_040_888_963_407;

    /// A generator for `seed`, stepped once so nearby seeds diverge at once.
    const fn new(seed: u64) -> Self {
        let mut lcg = Self(seed);
        lcg.advance();
        lcg
    }

    /// Steps the state and returns its bytes, most significant first.
    const fn advance(&mut self) -> [u8; 8] {
        self.0 = self
            .0
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.0.to_be_bytes()
    }

    /// The top 16 bits of the next state.
    const fn top16(&mut self) -> u16 {
        let [high, low, ..] = self.advance();
        u16::from_be_bytes([high, low])
    }

    /// A value in `0..len`, for a nonzero `len` of at most 16 bits.
    fn index(&mut self, len: usize) -> usize {
        usize::from(self.top16()) % len
    }

    /// One byte of the next state.
    const fn byte(&mut self) -> u8 {
        self.advance()[0]
    }

    /// 64 bits, from the top halves of two states.
    const fn wide(&mut self) -> u64 {
        let high = self.advance();
        let low = self.advance();
        u64::from_be_bytes([
            high[0], high[1], high[2], high[3], low[0], low[1], low[2], low[3],
        ])
    }
}

/// One step of the walk.
#[derive(Clone, Copy, Debug)]
enum Op {
    Lookup,
    Insert,
    Supersede,
    Hold,
    Settle,
    Defer,
    FreeAll,
}

// The walk grows the pocket to a target of 96 to 260 records, then drains it to
// empty, and repeats; a drain that empties the pocket ends in a `free_all` on the
// empty pocket. GROW's insert weight is what carries the pocket past several
// 64-row chunks, and its holds and settles keep the front of the log churning
// meanwhile. DRAIN's settle weight, with holds to feed it, is what fully settles
// the front chunks. Both phases point supersedes at held keys (see
// `mostly_held_key`) with payloads of 0 to 200 bytes, so a chunk's payload arena
// is spliced and repacked. DRAIN keeps `free_all` rare, so held records are
// settled rather than requeued, yet present, so deferred records come back.
/// Weights while the pocket grows toward its target.
const GROW: [(Op, u16); 7] = [
    (Op::Insert, 40),
    (Op::Supersede, 12),
    (Op::Hold, 12),
    (Op::Settle, 10),
    (Op::Defer, 6),
    (Op::FreeAll, 2),
    (Op::Lookup, 8),
];
/// Weights while the pocket drains to empty.
const DRAIN: [(Op, u16); 7] = [
    (Op::Insert, 2),
    (Op::Supersede, 16),
    (Op::Hold, 14),
    (Op::Settle, 40),
    (Op::Defer, 6),
    (Op::FreeAll, 2),
    (Op::Lookup, 8),
];

/// How positions are handed out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Positions {
    /// One increasing counter shared by inserts and defers, as the queue's
    /// `Pockets::next_position` does.
    Increasing,
    /// Random fresh positions, out of order, never one a live record holds.
    Scattered,
}

/// What a walk reached, so the test proves its mix did the work it claims.
#[derive(Debug, Default)]
struct Reach {
    /// The most records the pocket held at once.
    largest: usize,
    /// Supersedes that landed on a held record.
    held_supersedes: usize,
    /// `free_all` calls on an empty pocket.
    empty_frees: usize,
}

/// One seed's walk over both stores.
#[derive(Debug)]
struct Walk {
    seed: u64,
    step: usize,
    rng: Lcg,
    keys: Vec<String>,
    log: LogRecords,
    memory: MemoryRecords,
    positions: Positions,
    next_position: u64,
    /// Each live record's position, by key.
    live: BTreeMap<String, u64>,
    /// The positions live records hold; `MemoryRecords` keys its queued records
    /// by position, so no two live records may share one.
    taken: BTreeSet<u64>,
    /// The keys held now.
    held: BTreeSet<String>,
    target: usize,
    draining: bool,
    reach: Reach,
}

impl Walk {
    /// A walk for `seed`; one seed in four scatters its positions.
    fn new(seed: u64) -> Self {
        let mut rng = Lcg::new(seed);
        let target = grow_target(&mut rng);
        let positions = if seed.is_multiple_of(4) {
            Positions::Scattered
        } else {
            Positions::Increasing
        };
        Self {
            seed,
            step: 0,
            rng,
            keys: (0..KEY_POOL).map(key).collect(),
            log: LogRecords::default(),
            memory: MemoryRecords::default(),
            positions,
            next_position: 0,
            live: BTreeMap::new(),
            taken: BTreeSet::new(),
            held: BTreeSet::new(),
            target,
            draining: false,
            reach: Reach::default(),
        }
    }

    /// Takes every step, comparing the record counts after each one and both
    /// stores in full every `FULL_CHECK_EVERY` steps and at the end.
    fn run(mut self) -> Reach {
        for step in 0..STEPS {
            self.step = step;
            self.take_step();
            let count = self.memory.record_count();
            assert_eq!(
                self.log.record_count(),
                count,
                "{}: record_count",
                self.at()
            );
            self.reach.largest = self.reach.largest.max(count);
            if step % FULL_CHECK_EVERY == FULL_CHECK_EVERY - 1 {
                self.compare_whole();
            }
        }
        self.compare_whole();
        self.reach
    }

    /// Where the walk stands, for an assertion message.
    fn at(&self) -> String {
        format!("seed {} step {}", self.seed, self.step)
    }

    fn take_step(&mut self) {
        if self.turn_phase() {
            self.free_all();
            return;
        }
        match self.choose() {
            Op::Lookup => {
                let key = self.any_key();
                self.compare_lookup(&key);
            }
            Op::Insert => self.insert(),
            Op::Supersede => self.supersede(),
            Op::Hold => self.hold(),
            Op::Settle => self.settle(),
            Op::Defer => self.defer(),
            Op::FreeAll => self.free_all(),
        }
    }

    /// Switches between growing and draining. Returns true when a drain has just
    /// emptied the pocket: that step is a `free_all` on the empty pocket, which
    /// drops `LogRecords`' inner state, and the walk goes on after it.
    fn turn_phase(&mut self) -> bool {
        let count = self.memory.record_count();
        if self.draining && count == 0 {
            self.draining = false;
            self.target = grow_target(&mut self.rng);
            return true;
        }
        if !self.draining && count >= self.target {
            self.draining = true;
        }
        false
    }

    /// An operation drawn by the current phase's weights.
    fn choose(&mut self) -> Op {
        let weights = if self.draining { &DRAIN } else { &GROW };
        let total: u16 = weights.iter().map(|&(_, weight)| weight).sum();
        let mut roll = self.rng.top16() % total;
        for &(op, weight) in weights {
            if roll < weight {
                return op;
            }
            roll -= weight;
        }
        Op::Lookup
    }

    fn any_key(&mut self) -> String {
        let index = self.rng.index(self.keys.len());
        self.keys[index].clone()
    }

    /// A held key seven times in eight, so supersedes, settles and defers land
    /// on held records; otherwise any key, so both stores also answer for keys
    /// that are queued, deferred or absent.
    fn mostly_held_key(&mut self) -> String {
        if self.held.is_empty() || self.rng.top16().is_multiple_of(8) {
            return self.any_key();
        }
        let index = self.rng.index(self.held.len());
        self.held.iter().nth(index).cloned().unwrap_or_default()
    }

    /// A key the pocket lacks, from a few tries, as the queue inserts only those.
    fn absent_key(&mut self) -> Option<String> {
        for _ in 0..4 {
            let key = self.any_key();
            if self.memory.lookup(&key).is_none() {
                return Some(key);
            }
        }
        None
    }

    /// A random rank and a payload of 0 to `MAX_PAYLOAD` random bytes, so a
    /// payload read back from the wrong place in an arena shows.
    fn version(&mut self) -> Version {
        let rank = i64::from_be_bytes(self.rng.wide().to_be_bytes());
        let size = self.rng.index(MAX_PAYLOAD + 1);
        let payload = (0..size).map(|_| self.rng.byte()).collect();
        Version::new(rank, payload)
    }

    /// A position no live record holds.
    fn fresh_position(&mut self) -> u64 {
        if self.positions == Positions::Increasing {
            let position = self.next_position;
            self.next_position += 1;
            return position;
        }
        loop {
            let candidate = if self.rng.top16().is_multiple_of(2) {
                u64::from(self.rng.top16() % NARROW_POSITIONS)
            } else {
                self.rng.wide() >> 16
            };
            if !self.taken.contains(&candidate) {
                return candidate;
            }
        }
    }

    fn compare_lookup(&self, key: &str) {
        let expected = self.memory.lookup(key);
        assert_eq!(
            self.log.lookup(key),
            expected,
            "{}: lookup {key}",
            self.at()
        );
    }

    fn compare_whole(&self) {
        let expected = self.memory.snapshot();
        assert_eq!(self.log.snapshot(), expected, "{}: snapshot", self.at());
        let expected = self.memory.held();
        assert_eq!(self.log.held(), expected, "{}: held", self.at());
    }

    fn insert(&mut self) {
        let Some(key) = self.absent_key() else {
            return;
        };
        let version = self.version();
        let position = self.fresh_position();
        self.log.insert_queued(&key, version.clone(), position);
        self.memory.insert_queued(&key, version, position);
        self.taken.insert(position);
        self.live.insert(key.clone(), position);
        self.compare_lookup(&key);
    }

    fn supersede(&mut self) {
        let key = self.mostly_held_key();
        let version = self.version();
        let replaced = self.log.supersede(&key, version.clone());
        let expected = self.memory.supersede(&key, version);
        assert_eq!(replaced, expected, "{}: supersede {key}", self.at());
        if self.held.contains(&key) {
            self.reach.held_supersedes += 1;
        }
        self.compare_lookup(&key);
    }

    fn hold(&mut self) {
        let limit = self.rng.index(MAX_LIMIT + 1);
        let batch = held_records(&self.log.hold_oldest(limit));
        let expected = self.memory.hold_oldest(limit);
        assert_eq!(batch, expected, "{}: hold_oldest({limit})", self.at());
        self.held
            .extend(expected.into_iter().map(|record| record.key));
    }

    fn settle(&mut self) {
        let key = self.mostly_held_key();
        let settled = self.log.settle(&key);
        let expected = self.memory.settle(&key);
        assert_eq!(settled, expected, "{}: settle {key}", self.at());
        if settled.is_some() {
            self.held.remove(&key);
            if let Some(position) = self.live.remove(&key) {
                self.taken.remove(&position);
            }
        }
        self.compare_lookup(&key);
    }

    fn defer(&mut self) {
        let key = self.mostly_held_key();
        let position = self.fresh_position();
        let deferred = self.log.defer(&key, position);
        let expected = self.memory.defer(&key, position);
        assert_eq!(
            deferred,
            expected,
            "{}: defer {key} to {position}",
            self.at()
        );
        if deferred.is_some() {
            self.held.remove(&key);
            self.taken.insert(position);
            if let Some(old) = self.live.insert(key.clone(), position) {
                self.taken.remove(&old);
            }
        }
        self.compare_lookup(&key);
    }

    fn free_all(&mut self) {
        if self.memory.record_count() == 0 {
            self.reach.empty_frees += 1;
        }
        self.log.free_all();
        self.memory.free_all();
        self.held.clear();
    }
}

/// The pool's key `index`, of varying length, so a chunk's key arena varies too.
fn key(index: u16) -> String {
    format!("key-{index}-{}", "k".repeat(usize::from(index % 40)))
}

/// The record count a growing phase aims for: 96 to 260, several chunks' worth.
fn grow_target(rng: &mut Lcg) -> usize {
    96 + rng.index(165)
}

/// Walks every seed in `seeds` and requires each walk to have reached the shapes
/// the mix is weighted for. Measured over seeds 0..64: every walk peaked at 192
/// records or more, freed an empty pocket 7 times or more, and superseded a held
/// record 1059 times or more.
fn differential(seeds: Range<u64>) {
    for seed in seeds {
        let reach = Walk::new(seed).run();
        assert!(
            reach.largest >= 3 * CHUNK_ROWS,
            "seed {seed}: the pocket peaked at {} records",
            reach.largest
        );
        assert!(
            reach.empty_frees >= 5,
            "seed {seed}: only {} free_all calls on an empty pocket",
            reach.empty_frees
        );
        assert!(
            reach.held_supersedes >= 1_000,
            "seed {seed}: only {} supersedes of held records",
            reach.held_supersedes
        );
    }
}

#[test]
fn log_records_keep_the_records_contract() {
    check_contract::<LogRecords>(held_records);
}

// 64 seeds, 16 of them scattered, split four ways so the test runner runs them in
// parallel.
#[test]
fn log_records_agree_with_memory_records_on_seeds_0_to_15() {
    differential(0..16);
}

#[test]
fn log_records_agree_with_memory_records_on_seeds_16_to_31() {
    differential(16..32);
}

#[test]
fn log_records_agree_with_memory_records_on_seeds_32_to_47() {
    differential(32..48);
}

#[test]
fn log_records_agree_with_memory_records_on_seeds_48_to_63() {
    differential(48..64);
}

fn held(key: &str, rank: i64, payload: Vec<u8>) -> HeldRecord {
    HeldRecord::new(key.to_owned(), rank, payload)
}

/// Supersedes a held record twice (growing, then shrinking its payload), defers
/// it, frees the lease and pulls again. Returns the pulls and the end state.
fn supersede_defer_free_pull<R: PocketRecords>(
    pull: fn(&R::Batch) -> Vec<HeldRecord>,
) -> (Vec<Vec<HeldRecord>>, Vec<RecordView>) {
    let mut records = R::default();
    records.insert_queued("a", Version::new(1, vec![1; 10]), 0);
    records.insert_queued("b", Version::new(1, vec![2; 30]), 1);
    let first = pull(&records.hold_oldest(2));
    let replaced = records.supersede("a", Version::new(2, vec![3; 150]));
    assert_eq!(replaced, Some(Version::new(1, vec![1; 10])));
    let replaced = records.supersede("a", Version::new(3, vec![4; 5]));
    assert_eq!(replaced, Some(Version::new(2, vec![3; 150])));
    assert_eq!(records.held(), first, "a supersede leaves the held version");
    assert_eq!(records.defer("a", 2), Some(Version::new(1, vec![1; 10])));
    records.free_all();
    let second = pull(&records.hold_oldest(10));
    (vec![first, second], records.snapshot())
}

#[test]
fn a_superseded_held_record_deferred_and_freed_is_pulled_at_its_latest_version() {
    let (pulls, end) = supersede_defer_free_pull::<LogRecords>(held_records);
    let (expected_pulls, expected_end) = supersede_defer_free_pull::<MemoryRecords>(Clone::clone);
    assert_eq!(pulls, expected_pulls);
    assert_eq!(end, expected_end);
    assert_eq!(
        pulls[1],
        vec![held("b", 1, vec![2; 30]), held("a", 3, vec![4; 5])]
    );
}

/// Pulls with limits past the queued count while a deferred record waits, then
/// frees the lease and pulls everything. Returns the pulls and the end state.
fn pull_past_the_queued_with_a_deferred<R: PocketRecords>(
    pull: fn(&R::Batch) -> Vec<HeldRecord>,
) -> (Vec<Vec<HeldRecord>>, Vec<RecordView>) {
    let mut records = R::default();
    for (position, key) in (0_u64..).zip(["a", "b", "c", "d"]) {
        records.insert_queued(key, Version::new(1, key.as_bytes().to_vec()), position);
    }
    let mut pulls = vec![pull(&records.hold_oldest(2))];
    records.supersede("a", Version::new(2, b"a2".to_vec()));
    records.defer("a", 4);
    records.insert_queued("e", Version::new(1, b"e".to_vec()), 5);
    pulls.push(pull(&records.hold_oldest(100)));
    records.settle("c");
    pulls.push(pull(&records.hold_oldest(100)));
    records.free_all();
    pulls.push(pull(&records.hold_oldest(100)));
    (pulls, records.snapshot())
}

#[test]
fn a_pull_past_the_queued_count_skips_a_deferred_record_until_the_lease_is_freed() {
    let (pulls, end) = pull_past_the_queued_with_a_deferred::<LogRecords>(held_records);
    let (expected_pulls, expected_end) =
        pull_past_the_queued_with_a_deferred::<MemoryRecords>(Clone::clone);
    assert_eq!(pulls, expected_pulls);
    assert_eq!(end, expected_end);
    let keys = |pull: &[HeldRecord]| -> Vec<String> {
        pull.iter().map(|record| record.key.clone()).collect()
    };
    assert_eq!(keys(&pulls[1]), ["c", "d", "e"]);
    assert_eq!(pulls[2], Vec::<HeldRecord>::new());
    assert_eq!(keys(&pulls[3]), ["b", "d", "a", "e"]);
    assert_eq!(pulls[3][2], held("a", 2, b"a2".to_vec()));
}
