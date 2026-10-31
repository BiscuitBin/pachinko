//! The Arrow record store under load, run by `just bench`: publish N records into
//! M pockets, drain them by pull and ack, and the store's own steps on one pocket.
//! The queue runs on the fake clock, so the timings are the queue's and the
//! store's, not the system clock's.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark is a harness: a broken queue should stop it loudly, as in a test"
)]

use std::fmt;

use divan::{Bencher, black_box};
use libpachinko::clock::FakeClock;
use libpachinko::config::Config;
use libpachinko::queue::{Publication, PullOutcome, Queue};
use libpachinko::records::{PocketRecords, Version};
use pachinko::arrow::{ArrowRecords, held_records};

fn main() {
    divan::main();
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

type ArrowQueue = Queue<ArrowRecords, FakeClock>;

fn empty_queue() -> ArrowQueue {
    let mut queue = Queue::new(Config::default(), FakeClock::default()).unwrap();
    queue.publisher_joins("publisher").unwrap();
    queue.consumer_joins("consumer").unwrap();
    queue
}

fn publish_all(queue: &mut ArrowQueue, shape: Shape) {
    for index in 0..shape.records {
        let pocket = format!("pocket-{}", index % shape.pockets);
        let record = format!("record-{index}");
        let publication = Publication::new(&pocket, &record, 1, vec![0; 64]);
        queue.publish("publisher", publication).unwrap();
    }
}

fn filled_queue(shape: Shape) -> ArrowQueue {
    let mut queue = empty_queue();
    publish_all(&mut queue, shape);
    queue
}

/// Pulls and acks until no pocket is left; returns how many records it acked.
fn drain(queue: &mut ArrowQueue) -> usize {
    let mut acked = 0;
    while let PullOutcome::Batch(batch) = queue.pull("consumer", PULL_LIMIT).unwrap() {
        let records = held_records(&batch.records);
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
#[divan::bench(args = SHAPES)]
fn publish(bencher: Bencher<'_, '_>, shape: Shape) {
    bencher
        .with_inputs(empty_queue)
        .bench_local_values(|mut queue| {
            publish_all(&mut queue, shape);
            queue
        });
}

/// Draining a filled queue: pull a batch, ack each record, release each empty
/// pocket.
#[divan::bench(args = SHAPES)]
fn pull_and_ack(bencher: Bencher<'_, '_>, shape: Shape) {
    bencher
        .with_inputs(|| filled_queue(shape))
        .bench_local_values(|mut queue| {
            assert_eq!(drain(&mut queue), shape.records);
            queue
        });
}

/// The store's own steps on one pocket of `records` records: insert them all, then
/// hold them in batches of the pull limit.
#[divan::bench(args = [100, 1_000, 10_000])]
fn store_insert_then_hold(bencher: Bencher<'_, '_>, records: usize) {
    bencher.bench_local(|| {
        let mut store = ArrowRecords::default();
        for index in 0..records {
            let key = format!("record-{index}");
            let position = u64::try_from(index).unwrap();
            store.insert_queued(&key, Version::new(1, vec![0; 64]), position);
        }
        let mut held = 0;
        while held < records {
            held += store.hold_oldest(100).num_rows();
        }
        black_box(store)
    });
}
