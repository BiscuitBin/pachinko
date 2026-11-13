//! The queue's five workloads, W1 to W5, run by `just bench` at 10, 1,000 and
//! 100,000 pockets (decision 0019's "Evidence from the MVP").
//!
//! Every sample runs over one `Queue<MemoryRecords, FakeClock, CountingLeaseIds>`
//! with `Config::default()`, one publisher and one consumer joined and the P pockets
//! pre-created, built outside the timed region by divan's `with_inputs`, as are the
//! publications the sample sends; the queue is dropped outside it too. Payloads are
//! 64 bytes and keys go round-robin over the P pockets.
//!
//! The unit of one op is one library call on the queue: a `publish`, a `pull`, an
//! `ack`, a `release` or a `tick`. Each workload declares the calls a sample makes
//! through divan's `ItemsCount`, so the table's throughput column is ops per second,
//! and every call's result passes through `divan::black_box`. A sample asserts the
//! outcome it expects, so a workload that stops doing what it says fails loudly
//! rather than timing something else.
//!
//! - **W1 publish**: each op publishes a fresh key into the next pocket, which holds
//!   one seed record. `OPS` ops per sample.
//! - **W2 supersede**: one key per pocket pre-published at rank 1; each op publishes
//!   the next pocket's key at its rank plus one. `OPS` ops per sample.
//! - **W3 pull 16, ack, release**: every pocket pre-filled with 16 keys; one cycle
//!   is a pull with limit 16, 16 acks and a release, 18 ops. The release finds the
//!   pocket empty and idles it, so a sample runs `min(CYCLES, P)` cycles, one per
//!   pocket.
//! - **W4 publish with every fifth op a consume step**: pockets pre-filled as W3;
//!   one round is four publishes of fresh keys round-robin, then one W3 cycle on the
//!   front pocket, 22 ops. `min(CYCLES, P)` rounds, so every pull finds the 16
//!   pre-filled keys.
//! - **W5 pull then lapse on tick**: every pocket holds one record; one cycle is a
//!   pull with limit 16 that claims the front pocket, the fake clock advanced past
//!   `lease_ttl`, and a `tick` that lapses the lease, 2 ops. The clock step is inside
//!   the timed region and is not counted: the fake's step is one atomic
//!   read-modify-write, and the lapse cannot be reached without it. The lapsed
//!   pocket keeps the front (`LeaseExpiryKeepsPocketAtFront`), so the next cycle
//!   claims it again, until its third consecutive lapse parks it
//!   (`LeaseExpiryParksPocket`, `max_consecutive_expiries` 3) and the next pocket
//!   is the front; a sample runs `min(CYCLES, 3 P)` cycles.

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a benchmark is a harness: a broken queue should stop it loudly, as in a test"
)]

use std::time::Duration;

use divan::counter::ItemsCount;
use divan::{Bencher, black_box};
use libpachinko::clock::FakeClock;
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::records::MemoryRecords;
use libpachinko::{
    Config, ConsumerId, Payload, PocketKey, Publication, PublishOutcome, PublisherId, PullOutcome,
    Queue, Rank, RecordKey,
};

fn main() {
    divan::main();
}

/// The pocket counts every workload runs at.
const POCKETS: [usize; 3] = [10, 1_000, 100_000];

/// The publishes a W1 or W2 sample makes.
const OPS: usize = 10_000;

/// The most cycles a W3 or W5 sample, or rounds a W4 sample, runs.
const CYCLES: usize = 1_000;

/// The pull limit of W3, W4 and W5.
const PULL_LIMIT: i64 = 16;

/// The records W3 and W4 pre-fill each pocket with.
const FILL: usize = 16;

/// The payload size, in bytes.
const PAYLOAD_BYTES: usize = 64;

/// How many consecutive lapses park a pocket under `Config::default()`.
const LAPSES_TO_PARK: usize = 3;

type BenchQueue = Queue<MemoryRecords, FakeClock, CountingLeaseIds>;

/// A queue under test, the clock it reads and the publications its sample sends.
struct Input {
    queue: BenchQueue,
    clock: FakeClock,
    publications: Vec<Publication>,
}

fn publisher() -> PublisherId {
    PublisherId::new("publisher")
}

fn consumer() -> ConsumerId {
    ConsumerId::new("consumer")
}

fn publication(pocket: usize, key: &str, rank: i64) -> Publication {
    Publication::new(
        PocketKey::new(format!("pocket-{pocket}")),
        RecordKey::new(key),
        Rank::new(rank),
        Payload::from(vec![0; PAYLOAD_BYTES]),
    )
}

/// An empty queue with the publisher and the consumer joined.
fn joined() -> (BenchQueue, FakeClock) {
    let clock = FakeClock::default();
    let mut queue = Queue::new(
        Config::default(),
        clock.clone(),
        CountingLeaseIds::default(),
    )
    .unwrap();
    queue.publisher_joins(publisher()).unwrap();
    queue.consumer_joins(consumer()).unwrap();
    (queue, clock)
}

/// A queue whose P pockets each hold `fill` records, keyed `fill-{index}`.
fn filled(pockets: usize, fill: usize) -> Input {
    let (mut queue, clock) = joined();
    let publisher = publisher();
    for index in 0..pockets * fill {
        let seed = publication(index % pockets, &format!("fill-{index}"), 1);
        queue.publish(&publisher, seed).unwrap();
    }
    Input {
        queue,
        clock,
        publications: Vec::new(),
    }
}

/// `count` publications of fresh keys, round-robin over the P pockets.
fn fresh(pockets: usize, count: usize) -> Vec<Publication> {
    (0..count)
        .map(|index| publication(index % pockets, &format!("fresh-{index}"), 1))
        .collect()
}

/// One W3 cycle on the front pocket: a pull with limit 16, an ack of each record it
/// hands out and a release. Asserts the 16 records the fill guarantees.
fn consume_cycle(queue: &mut BenchQueue, consumer: &ConsumerId) {
    let pulled = black_box(queue.pull(consumer, PULL_LIMIT));
    let Ok(PullOutcome::Batch { lease, records, .. }) = pulled else {
        panic!("the fill keeps a pocket queued: {pulled:?}");
    };
    assert_eq!(records.len(), FILL, "a cycle acks the 16 pre-filled keys");
    for record in &records {
        black_box(queue.ack(consumer, lease, &record.key)).unwrap();
    }
    black_box(queue.release(consumer, lease)).unwrap();
}

/// W1: each op publishes a fresh key into the next pocket.
#[divan::bench(args = POCKETS, sample_count = 20, sample_size = 1)]
fn w1_publish(bencher: Bencher<'_, '_>, pockets: usize) {
    bencher
        .counter(ItemsCount::new(OPS))
        .with_inputs(|| Input {
            publications: fresh(pockets, OPS),
            ..filled(pockets, 1)
        })
        .bench_local_values(|mut input| {
            let publisher = publisher();
            for publication in input.publications.drain(..) {
                let outcome = black_box(input.queue.publish(&publisher, publication));
                assert_eq!(outcome, Ok(PublishOutcome::NewKey));
            }
            input
        });
}

/// W2: each op publishes the next pocket's one key at its rank plus one.
#[divan::bench(args = POCKETS, sample_count = 20, sample_size = 1)]
fn w2_supersede(bencher: Bencher<'_, '_>, pockets: usize) {
    bencher
        .counter(ItemsCount::new(OPS))
        .with_inputs(|| {
            let mut input = filled(pockets, 1);
            input.publications = (0..OPS)
                .map(|index| {
                    let rank = i64::try_from(index / pockets).unwrap() + 2;
                    let key = format!("fill-{}", index % pockets);
                    publication(index % pockets, &key, rank)
                })
                .collect();
            input
        })
        .bench_local_values(|mut input| {
            let publisher = publisher();
            for publication in input.publications.drain(..) {
                let outcome = black_box(input.queue.publish(&publisher, publication));
                assert_eq!(outcome, Ok(PublishOutcome::Superseded));
            }
            input
        });
}

/// W3: one cycle is a pull with limit 16, 16 acks and a release, 18 ops.
#[divan::bench(args = POCKETS, sample_count = 20, sample_size = 1)]
fn w3_pull_ack_release(bencher: Bencher<'_, '_>, pockets: usize) {
    let cycles = CYCLES.min(pockets);
    bencher
        .counter(ItemsCount::new(cycles * (FILL + 2)))
        .with_inputs(|| filled(pockets, FILL))
        .bench_local_values(|mut input| {
            let consumer = consumer();
            for _ in 0..cycles {
                consume_cycle(&mut input.queue, &consumer);
            }
            input
        });
}

/// W4: four publishes of fresh keys, then one W3 cycle, 22 ops a round.
#[divan::bench(args = POCKETS, sample_count = 20, sample_size = 1)]
fn w4_publish_and_consume(bencher: Bencher<'_, '_>, pockets: usize) {
    let rounds = CYCLES.min(pockets);
    bencher
        .counter(ItemsCount::new(rounds * (4 + FILL + 2)))
        .with_inputs(|| Input {
            publications: fresh(pockets, rounds * 4),
            ..filled(pockets, FILL)
        })
        .bench_local_values(|mut input| {
            let (publisher, consumer) = (publisher(), consumer());
            let mut publications = input.publications.drain(..);
            for _ in 0..rounds {
                for publication in publications.by_ref().take(4) {
                    let outcome = black_box(input.queue.publish(&publisher, publication));
                    assert_eq!(outcome, Ok(PublishOutcome::NewKey));
                }
                consume_cycle(&mut input.queue, &consumer);
            }
            drop(publications);
            input
        });
}

/// W5: a pull that claims the front pocket, the clock past `lease_ttl`, and a tick
/// that lapses the lease; 2 ops, the clock step uncounted.
#[divan::bench(args = POCKETS, sample_count = 20, sample_size = 1)]
fn w5_pull_then_lapse(bencher: Bencher<'_, '_>, pockets: usize) {
    let cycles = CYCLES.min(LAPSES_TO_PARK * pockets);
    let past_ttl = Config::default().lease_ttl + Duration::from_millis(1);
    bencher
        .counter(ItemsCount::new(cycles * 2))
        .with_inputs(|| filled(pockets, 1))
        .bench_local_values(|mut input| {
            let consumer = consumer();
            for _ in 0..cycles {
                let pulled = black_box(input.queue.pull(&consumer, PULL_LIMIT));
                assert!(
                    matches!(pulled, Ok(PullOutcome::Batch { .. })),
                    "{pulled:?}"
                );
                input.clock.advance(past_ttl);
                let report = black_box(input.queue.tick());
                assert_eq!(report.lapsed.len(), 1, "the tick lapses the one lease");
            }
            input
        });
}
