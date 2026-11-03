//! The record stores side by side, run by `just bench`: every store under the same
//! workloads, so one run compares them. `ArrowRecords` is the baseline the others
//! are measured against, and `MemoryRecords` the trait's reference implementation.
//!
//! The store's own steps run on one pocket of 100, 1,000 and 10,000 records with
//! 16 B and 1 KiB payloads; `publish` and `pull_and_ack` drive the whole queue on
//! the fake clock, so the timings are the queue's and the store's, not the system
//! clock's. The allocation profiler counts each step's allocations beside its time.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark is a harness: a broken queue should stop it loudly, as in a test"
)]

use std::fmt;

use divan::counter::ItemsCount;
use divan::{AllocProfiler, Bencher};
use libpachinko::clock::FakeClock;
use libpachinko::config::Config;
use libpachinko::queue::{Publication, PullOutcome, Queue};
use libpachinko::records::{HeldRecord, MemoryRecords, PocketRecords, Version};
use pachinko::arrow::{ArrowRecords, held_records};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

/// What the generic benches need from a store beyond `PocketRecords`: its batch
/// read back as records, and the batch's length.
trait Store: PocketRecords {
    fn held_of(batch: &Self::Batch) -> Vec<HeldRecord>;
    fn batch_len(batch: &Self::Batch) -> usize;
}

impl Store for MemoryRecords {
    fn held_of(batch: &Self::Batch) -> Vec<HeldRecord> {
        batch.clone()
    }

    fn batch_len(batch: &Self::Batch) -> usize {
        batch.len()
    }
}

impl Store for ArrowRecords {
    fn held_of(batch: &Self::Batch) -> Vec<HeldRecord> {
        held_records(batch)
    }

    fn batch_len(batch: &Self::Batch) -> usize {
        batch.num_rows()
    }
}

// T21: LogRecords joins here

/// One pocket of `records` records, each with a `payload`-byte payload.
#[derive(Clone, Copy)]
struct Pocket {
    records: usize,
    payload: usize,
}

impl fmt::Debug for Pocket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.payload < 1024 {
            write!(formatter, "{} records x {} B", self.records, self.payload)
        } else {
            let kibibytes = self.payload / 1024;
            write!(formatter, "{} records x {kibibytes} KiB", self.records)
        }
    }
}

const POCKETS: [Pocket; 6] = [
    Pocket {
        records: 100,
        payload: 16,
    },
    Pocket {
        records: 100,
        payload: 1024,
    },
    Pocket {
        records: 1_000,
        payload: 16,
    },
    Pocket {
        records: 1_000,
        payload: 1024,
    },
    Pocket {
        records: 10_000,
        payload: 16,
    },
    Pocket {
        records: 10_000,
        payload: 1024,
    },
];

/// The records a step holds at once: one pull's worth, as `PULL_LIMIT` below.
const HELD: usize = 100;

/// Inputs prepared per sample of a single-op step. Each timed step runs on its own
/// clone of the pocket, and divan's tuning ignores the cost of preparing inputs: left
/// to tune, a fast step at 10,000 records would clone the pocket hundreds of times
/// per sample. Sixteen holds a step's run over all six pockets to a few seconds,
/// and at 10,000 records, where the ticket's claims sit, keeps each sample over
/// 100 times the timer's precision for a step of 300 ns or more; the fastest steps
/// at 100 records sit nearer the timer's floor. Divan's default of 100 samples
/// stands.
const STEP_SAMPLE_SIZE: u32 = 16;

/// Inputs prepared per sample of a batched step, which runs 100 ops on one clone.
/// The batch is long enough for the timer on its own, and `ArrowRecords` copies the
/// whole pocket on each op, so one batch per sample keeps a 10,000-record run to
/// seconds rather than minutes.
const BATCH_SAMPLE_SIZE: u32 = 1;

fn key(index: usize) -> String {
    format!("record-{index}")
}

fn position(index: usize) -> u64 {
    u64::try_from(index).unwrap()
}

fn version(pocket: Pocket) -> Version {
    Version::new(1, vec![0; pocket.payload])
}

/// The pocket with every record queued: keys `record-{i}` at positions `i`.
fn queued<R: Store>(pocket: Pocket) -> R {
    let mut store = R::default();
    for index in 0..pocket.records {
        store.insert_queued(&key(index), version(pocket), position(index));
    }
    store
}

/// The pocket with its oldest `HELD` records held.
fn with_held<R: Store>(pocket: Pocket) -> R {
    let mut store = queued::<R>(pocket);
    assert_eq!(
        R::batch_len(&store.hold_oldest(HELD)),
        HELD.min(pocket.records)
    );
    store
}

/// A key the step finds: the middle of the pocket.
fn middle(pocket: Pocket) -> String {
    key(pocket.records / 2)
}

/// The keys of the records `with_held` holds.
fn held_keys(pocket: Pocket) -> Vec<String> {
    (0..HELD.min(pocket.records)).map(key).collect()
}

/// Looking up a present key. `lookup` reads the pocket, so it runs on one shared
/// pocket rather than a clone per call, and divan tunes its sample size.
#[divan::bench(types = [MemoryRecords, ArrowRecords], args = POCKETS)] // T21: LogRecords joins here
fn lookup<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let store = queued::<R>(pocket);
    bencher
        .with_inputs(|| middle(pocket))
        .bench_local_values(|found| (store.lookup(&found), found));
}

/// Inserting 100 new keys, one after another, at the next positions `n`, `n + 1`,
/// …; the reported time covers all 100 inserts.
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    counter = ItemsCount::new(HELD),
    sample_size = BATCH_SAMPLE_SIZE,
)]
fn insert<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = queued::<R>(pocket);
    let fresh = pocket.records..pocket.records + HELD;
    bencher
        .with_inputs(|| {
            let keys: Vec<String> = fresh.clone().map(key).collect();
            let versions: Vec<Version> = fresh.clone().map(|_| version(pocket)).collect();
            (base.clone(), keys, versions)
        })
        .bench_local_values(|(mut store, keys, mut versions)| {
            for ((new_key, latest), index) in keys.iter().zip(versions.drain(..)).zip(fresh.clone())
            {
                store.insert_queued(new_key, latest, position(index));
            }
            (store, keys, versions)
        });
}

/// Superseding a queued key in the middle with a new version of the same size.
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    sample_size = STEP_SAMPLE_SIZE,
)]
fn supersede<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = queued::<R>(pocket);
    bencher
        .with_inputs(|| {
            let latest = Version::new(2, vec![0; pocket.payload]);
            (base.clone(), middle(pocket), latest)
        })
        .bench_local_values(|(mut store, found, latest)| {
            let old = store.supersede(&found, latest);
            assert!(old.is_some());
            (old, store, found)
        });
}

/// Settling all 100 held keys, one after another, with the pocket's oldest 100
/// records held (at 100 records, every record: still 100 keys); the reported time
/// covers all 100 settles.
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    counter = ItemsCount::new(HELD),
    sample_size = BATCH_SAMPLE_SIZE,
)]
fn settle<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = with_held::<R>(pocket);
    bencher
        .with_inputs(|| (base.clone(), held_keys(pocket), Vec::with_capacity(HELD)))
        .bench_local_values(|(mut store, keys, mut settled)| {
            for held in &keys {
                settled.push(store.settle(held));
            }
            assert!(settled.iter().all(Option::is_some));
            (store, keys, settled)
        });
}

/// Deferring all 100 held keys, one after another, past every queued record to
/// positions `n`, `n + 1`, …, with the pocket's oldest 100 records held; the
/// reported time covers all 100 defers.
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    counter = ItemsCount::new(HELD),
    sample_size = BATCH_SAMPLE_SIZE,
)]
fn defer<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = with_held::<R>(pocket);
    bencher
        .with_inputs(|| (base.clone(), held_keys(pocket), Vec::with_capacity(HELD)))
        .bench_local_values(|(mut store, keys, mut deferred)| {
            for (offset, held) in keys.iter().enumerate() {
                deferred.push(store.defer(held, position(pocket.records + offset)));
            }
            assert!(deferred.iter().all(Option::is_some));
            (store, keys, deferred)
        });
}

/// Holding the oldest 100 records of a pocket where every record is queued.
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    sample_size = STEP_SAMPLE_SIZE,
)]
fn hold_oldest<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = queued::<R>(pocket);
    bencher
        .with_inputs(|| base.clone())
        .bench_local_values(|mut store| {
            let batch = store.hold_oldest(HELD);
            assert_eq!(R::batch_len(&batch), HELD.min(pocket.records));
            (batch, store)
        });
}

/// The pocket for `free_all`: its oldest `2k` records held, then the first `k` of
/// them deferred to fresh positions `n`, `n + 1`, …, leaving `k` held and `k`
/// deferred, where `k` is 100, or half the pocket at 100 records.
fn held_and_deferred<R: Store>(pocket: Pocket) -> R {
    let mut store = queued::<R>(pocket);
    let each = HELD.min(pocket.records / 2);
    let held = R::held_of(&store.hold_oldest(2 * each));
    for (offset, record) in held.iter().take(each).enumerate() {
        let deferred = store.defer(&record.key, position(pocket.records + offset));
        assert!(deferred.is_some());
    }
    store
}

/// Freeing every held and deferred record when the lease ends, with 100 held and
/// 100 deferred (50 of each at 100 records).
#[divan::bench(
    types = [MemoryRecords, ArrowRecords], // T21: LogRecords joins here
    args = POCKETS,
    sample_size = STEP_SAMPLE_SIZE,
)]
fn free_all<R: Store>(bencher: Bencher<'_, '_>, pocket: Pocket) {
    let base = held_and_deferred::<R>(pocket);
    bencher
        .with_inputs(|| base.clone())
        .bench_local_values(|mut store| {
            store.free_all();
            store
        });
}

/// N records spread round-robin over M pockets.
#[derive(Clone, Copy)]
struct Shape {
    records: usize,
    pockets: usize,
}

impl fmt::Debug for Shape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} records x {} pockets",
            self.records, self.pockets
        )
    }
}

const SHAPES: [Shape; 3] = [
    Shape {
        records: 1_000,
        pockets: 10,
    },
    Shape {
        records: 10_000,
        pockets: 100,
    },
    Shape {
        records: 10_000,
        pockets: 10,
    },
];

/// The records a pull asks for.
const PULL_LIMIT: i64 = 100;

fn empty_queue<R: Store>() -> Queue<R, FakeClock> {
    let mut queue = Queue::new(Config::default(), FakeClock::default()).unwrap();
    queue.publisher_joins("publisher").unwrap();
    queue.consumer_joins("consumer").unwrap();
    queue
}

fn publish_all<R: Store>(queue: &mut Queue<R, FakeClock>, shape: Shape) {
    for index in 0..shape.records {
        let pocket = format!("pocket-{}", index % shape.pockets);
        let record = format!("record-{index}");
        let publication = Publication::new(&pocket, &record, 1, vec![0; 64]);
        queue.publish("publisher", publication).unwrap();
    }
}

fn filled_queue<R: Store>(shape: Shape) -> Queue<R, FakeClock> {
    let mut queue = empty_queue();
    publish_all(&mut queue, shape);
    queue
}

/// Pulls and acks until no pocket is left; returns how many records it acked.
fn drain<R: Store>(queue: &mut Queue<R, FakeClock>) -> usize {
    let mut acked = 0;
    while let PullOutcome::Batch(batch) = queue.pull("consumer", PULL_LIMIT).unwrap() {
        let records = R::held_of(&batch.records);
        if records.is_empty() {
            queue.release("consumer", batch.lease).unwrap();
            continue;
        }
        for record in &records {
            queue.ack("consumer", batch.lease, &record.key).unwrap();
        }
        acked += records.len();
    }
    acked
}

/// Publishing every record of the shape into a fresh queue.
#[divan::bench(types = [MemoryRecords, ArrowRecords], args = SHAPES)] // T21: LogRecords joins here
fn publish<R: Store>(bencher: Bencher<'_, '_>, shape: Shape) {
    bencher
        .with_inputs(empty_queue::<R>)
        .bench_local_values(|mut queue| {
            publish_all(&mut queue, shape);
            queue
        });
}

/// Draining a filled queue: pull a batch, ack each record, release each empty
/// pocket.
#[divan::bench(types = [MemoryRecords, ArrowRecords], args = SHAPES)] // T21: LogRecords joins here
fn pull_and_ack<R: Store>(bencher: Bencher<'_, '_>, shape: Shape) {
    bencher
        .with_inputs(|| filled_queue::<R>(shape))
        .bench_local_values(|mut queue| {
            assert_eq!(drain(&mut queue), shape.records);
            queue
        });
}
