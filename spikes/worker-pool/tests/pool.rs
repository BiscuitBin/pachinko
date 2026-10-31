//! The pool's integration tests, each named after the clause of ticket T20 step 5 it
//! proves: one thread per partition and the thread bound, structured stop with panic
//! propagation, the fencing token, freedom from the two-full-queues deadlock, per-
//! publisher order, and the lapse-versus-renewal race through the real pool.
//!
//! Every test ends with the settled machines passing `invariants::check` and reporting
//! no protocol violation; only the panic half of the stop test cannot, since a panic
//! hands back no outcome.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread::ThreadId;
use std::time::{Duration as StdDuration, Instant};

use worker_pool::pool::{Handle, Outcome, Pool, PoolConfig, PoolStats, WorkerStats};
use worker_pool::{
    Config, ConsumerId, Duration, Input, LeaseId, LeaseStatus, Machine, MachineId, Partition,
    Payload, PocketKey, PocketMachine, Published, PublisherId, Rank, Reason, RecordKey,
    RecordStatus, Response, invariants, partition_of,
};

/// How long a client waits for one reply.
const REPLY: StdDuration = StdDuration::from_secs(5);

/// A client's tally: requests sent and replies received.
type Tally = (u64, u64);

fn publish(publisher: &str, pocket: &str, record: &str, rank: u64) -> Input {
    Input::Publish {
        publisher: PublisherId::new(publisher),
        pocket_key: PocketKey::new(pocket),
        record_key: RecordKey::new(record),
        rank: Rank::new(rank),
        payload: Payload::new(format!("{pocket}/{record}/{rank}")),
    }
}

fn pull(consumer: &str, limit: u64) -> Input {
    Input::Pull {
        consumer: ConsumerId::new(consumer),
        limit,
    }
}

fn release(consumer: &str, lease: LeaseId) -> Input {
    Input::Release {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

fn renew(consumer: &str, lease: LeaseId) -> Input {
    Input::Renew {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

fn ack(consumer: &str, lease: LeaseId, record: &str) -> Input {
    Input::Ack {
        consumer: ConsumerId::new(consumer),
        lease,
        record_key: RecordKey::new(record),
    }
}

/// One call that must be answered.
fn call(handle: &Handle, input: Input) -> Result<Response, String> {
    let shown = format!("{input:?}");
    handle
        .call(input, REPLY)
        .map_err(|error| format!("{shown} got no reply: {error}"))
}

/// One call whose reply must be `want`.
fn expect(handle: &Handle, input: Input, want: &Response) -> Result<(), String> {
    let shown = format!("{input:?}");
    let got = call(handle, input)?;
    if got == *want {
        Ok(())
    } else {
        Err(format!("{shown} replied {got:?}, wanted {want:?}"))
    }
}

/// The lease, pocket and record keys of a batch reply.
fn batch(response: &Response) -> Result<(LeaseId, PocketKey, Vec<RecordKey>), String> {
    match response {
        Response::Batch {
            lease,
            pocket_key,
            records,
        } => Ok((
            *lease,
            pocket_key.clone(),
            records.iter().map(|record| record.key.clone()).collect(),
        )),
        other => Err(format!("expected a batch, got {other:?}")),
    }
}

/// The `seed`-th key, counting from zero, that hashes to `partition`.
fn key_in(partition: u32, partitions: u32, seed: usize) -> Option<PocketKey> {
    (0..)
        .map(|i| PocketKey::new(format!("k{i}")))
        .filter(|key| partition_of(key, partitions) == Partition::new(partition))
        .nth(seed)
}

/// The pocket machine owning `key`.
fn machine_of<'a, R>(outcome: &'a Outcome<R>, key: &PocketKey) -> &'a PocketMachine {
    let partitions = outcome.stats().partitions;
    let index = partition_of(key, partitions).index();
    &outcome.pockets()[index as usize]
}

/// Every machine settled as the rules leave it, and no machine saw a protocol violation.
fn settled<R>(outcome: &Outcome<R>) {
    assert_eq!(
        invariants::check(outcome.order(), outcome.pockets()),
        Vec::new()
    );
    assert_eq!(outcome.order().violations(), 0, "order machine violations");
    for machine in outcome.pockets() {
        assert_eq!(
            machine.violations(),
            0,
            "partition {:?} violations",
            machine.partition()
        );
    }
}

/// A publisher until the deadline: into fresh pockets when `fresh`, else cycling over
/// fifty pockets and five records with rising ranks. Every reply must be a `Published`.
fn publisher_loop(
    handle: &Handle,
    name: &str,
    deadline: Instant,
    fresh: bool,
) -> Result<Tally, String> {
    let mut tally = (0, 0);
    for n in 0u64.. {
        if Instant::now() >= deadline {
            break;
        }
        let (pocket, record, rank) = if fresh {
            (format!("{name}-{n}"), "r".to_owned(), 1)
        } else {
            (format!("{name}-{}", n % 50), format!("r{}", n % 5), n + 1)
        };
        tally.0 += 1;
        let reply = call(handle, publish(name, &pocket, &record, rank))?;
        if !matches!(reply, Response::Published(_)) {
            return Err(format!("publish by {name} replied {reply:?}"));
        }
        tally.1 += 1;
    }
    Ok(tally)
}

/// A consumer until the deadline: pull, ack every handed record when `settle`, release.
fn consumer_loop(
    handle: &Handle,
    name: &str,
    deadline: Instant,
    settle: bool,
) -> Result<Tally, String> {
    let mut tally = (0, 0);
    while Instant::now() < deadline {
        tally.0 += 1;
        let pulled = call(handle, pull(name, 10))?;
        tally.1 += 1;
        if pulled == Response::NoPocketAvailable {
            std::thread::yield_now();
            continue;
        }
        let (lease, _, records) = batch(&pulled)?;
        for record in records.iter().filter(|_| settle) {
            tally.0 += 1;
            expect(handle, ack(name, lease, record.as_str()), &Response::Acked)?;
            tally.1 += 1;
        }
        tally.0 += 1;
        expect(handle, release(name, lease), &Response::Released)?;
        tally.1 += 1;
    }
    Ok(tally)
}

/// Runs `publishers` publisher threads and `consumers` consumer threads against the
/// handle until the deadline and sums their tallies.
fn clients(
    handle: &Handle,
    counts: (usize, usize),
    deadline: Instant,
    modes: (bool, bool),
) -> Result<Tally, String> {
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for i in 0..counts.0 {
            let name = format!("p{i}");
            threads.push(scope.spawn(move || publisher_loop(handle, &name, deadline, modes.0)));
        }
        for j in 0..counts.1 {
            let name = format!("c{j}");
            threads.push(scope.spawn(move || consumer_loop(handle, &name, deadline, modes.1)));
        }
        let mut total = (0, 0);
        for thread in threads {
            let tally = thread
                .join()
                .map_err(|_| "a client panicked".to_owned())??;
            total.0 += tally.0;
            total.1 += tally.1;
        }
        Ok(total)
    })
}

/// Every machine was stepped by exactly one worker, the one the static assignment
/// names, on exactly one thread; returns the distinct threads seen.
fn one_thread_each(stats: &PoolStats) -> Vec<ThreadId> {
    let mut threads: Vec<ThreadId> = Vec::new();
    for (id, machine) in &stats.machines {
        assert_eq!(machine.stepped_by.len(), 1, "{id:?} stepped by {machine:?}");
        assert_eq!(machine.thread_ids.len(), 1, "{id:?} stepped by {machine:?}");
        let expected = match id {
            MachineId::Order => Some(0),
            MachineId::Partition(p) => Some((p.index() % 3) as usize),
            _ => None,
        };
        assert_eq!(machine.stepped_by.first().copied(), expected, "{id:?}");
        for thread in &machine.thread_ids {
            if !threads.contains(thread) {
                threads.push(*thread);
            }
        }
    }
    threads
}

#[test]
fn each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has() {
    let pool = PoolConfig::default()
        .with_worker_threads(3)
        .with_partitions(8)
        .with_tick_interval(Duration::from_millis(5));
    let outcome = Pool::start(Config::default(), pool, |handle| {
        let deadline = Instant::now() + StdDuration::from_millis(200);
        clients(handle, (2, 2), deadline, (false, true))
    })
    .unwrap();

    let (requests, replies) = outcome.result().clone().unwrap();
    assert!(requests > 0);
    assert_eq!(requests, replies);
    let stats = outcome.stats();
    assert_eq!(
        stats.machines.len(),
        9,
        "the order machine and eight pockets"
    );
    let threads = one_thread_each(stats);
    assert!(
        threads.len() <= 3,
        "{} threads stepped machines",
        threads.len()
    );
    assert!(stats.workers.iter().all(|worker| worker.misrouted == 0));
    settled(&outcome);
}

#[test]
fn stopping_joins_every_worker_thread_and_a_panic_reaches_the_caller() {
    // (a) A normal stop: every worker's join returned and none is alive afterwards.
    let pool = PoolConfig::default()
        .with_worker_threads(3)
        .with_partitions(2)
        .with_tick_interval(Duration::from_millis(5));
    let mut stash: Option<Handle> = None;
    let outcome = Pool::start(Config::default(), pool, |handle| {
        stash = Some(handle.clone());
        let published = Response::Published(Published::NewPocket);
        expect(handle, publish("p", "k", "r", 1), &published)?;
        let (lease, _, _) = batch(&call(handle, pull("c", 10))?)?;
        expect(handle, release("c", lease), &Response::Released)
    })
    .unwrap();
    outcome.result().clone().unwrap();
    assert_eq!(outcome.stats().joined_workers, 3);
    assert!(!outcome.stats().aborted);
    let handle = stash.take().unwrap();
    assert_eq!(handle.live_workers(), 0);
    assert!(!handle.aborted());
    settled(&outcome);

    // (b) A poisoned worker: the third input into partition 1 panics. The panic reaches
    // this thread with its own payload; that `catch_unwind` returns at all is the proof
    // that every worker was joined, since `std::thread::scope` cannot return or unwind
    // before its threads have, and the handle's live count says the same. The panic
    // prints to stderr on the way; no silent hook is installed, because a hook is
    // process-wide and would mute the other tests running beside this one.
    let started = Instant::now();
    let pool = PoolConfig::default()
        .with_worker_threads(2)
        .with_partitions(2)
        .with_tick_interval(Duration::from_millis(5))
        .with_panic_on(MachineId::Partition(Partition::new(1)), 3);
    let mut stash: Option<Handle> = None;
    let run = catch_unwind(AssertUnwindSafe(|| {
        Pool::start(Config::default(), pool, |handle| {
            stash = Some(handle.clone());
            let mut failed = 0;
            for seed in 0..4 {
                let key = key_in(1, 2, seed).unwrap();
                let sent = handle.call(publish("p", key.as_str(), "r", 1), REPLY);
                failed += usize::from(sent.is_err());
            }
            failed
        })
    }));
    let payload = run.expect_err("the worker's panic reaches the caller");
    let message = payload
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap();
    assert_eq!(message, "poisoned by PoolConfig::panic_on");
    let handle = stash.take().unwrap();
    assert_eq!(handle.live_workers(), 0);
    assert!(handle.aborted());
    assert!(started.elapsed() < StdDuration::from_secs(10));
}

#[test]
fn an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease() {
    // The lease TTL is 60 ms rather than the ticket's 20 so that the successor lease is
    // still active when the pool settles even if this thread is held off for a while.
    let config = Config::default().with_lease_ttl(Duration::from_millis(60));
    let pool = PoolConfig::default()
        .with_worker_threads(2)
        .with_partitions(2)
        .with_tick_interval(Duration::from_millis(5));
    let outcome = Pool::start(config, pool, |handle| {
        let published = Response::Published(Published::NewPocket);
        expect(handle, publish("p", "k", "r", 1), &published)?;
        let (first, pocket, records) = batch(&call(handle, pull("c", 10))?)?;
        assert_eq!(pocket, PocketKey::new("k"));
        assert_eq!(records, vec![RecordKey::new("r")]);
        // Several TTLs: the lease lapses on a tick and the pocket keeps its position.
        std::thread::sleep(StdDuration::from_millis(250));
        let (second, pocket_again, _) = batch(&call(handle, pull("c", 10))?)?;
        assert_eq!(pocket_again, pocket, "the same pocket is claimed again");
        assert_ne!(second, first, "under a new lease");
        let superseded = Response::Published(Published::Superseded);
        expect(handle, publish("p", "k", "r", 2), &superseded)?;
        let refused = Response::Rejected(Reason::LeaseNotActive);
        expect(handle, ack("c", first, "r"), &refused)?;
        Ok::<_, String>((first, second))
    })
    .unwrap();

    // The machines cannot be read mid-run, so "the records stay as they were" is read
    // from the settled state: an accepted ack would have deferred the record (its held
    // rank 1 is below the superseded rank 2), giving it a fresh position and clearing
    // the hold. Nothing of that happened.
    let (first, second) = outcome.result().clone().unwrap();
    let key = PocketKey::new("k");
    let machine = machine_of(&outcome, &key);
    let pocket = machine.pocket(&key).unwrap();
    let record = &pocket.records[&RecordKey::new("r")];
    assert_eq!(record.rank, Rank::new(2));
    assert_eq!(record.status, RecordStatus::Held);
    assert_eq!(record.held_rank, Some(Rank::new(1)));
    assert_eq!(record.position, u64::from(machine.partition().index()));
    let status = |lease: LeaseId| pocket.lease(lease).map(|lease| lease.status);
    assert_eq!(status(first), Some(LeaseStatus::Expired));
    assert_eq!(status(second), Some(LeaseStatus::Active));
    settled(&outcome);
}

/// Messages crossed in both directions, a queue filled at some point, and no reply was
/// doubled or lost.
fn crossed_both_ways(workers: &[WorkerStats]) {
    assert_eq!(workers.len(), 2);
    assert!(
        workers[0].channel_deliveries > 0,
        "worker 0 sent nothing to worker 1: {:?}",
        workers[0]
    );
    assert!(
        workers[1].channel_deliveries > 0,
        "worker 1 sent nothing to worker 0: {:?}",
        workers[1]
    );
    assert!(
        workers.iter().any(|worker| worker.outbox_high_water > 0),
        "no queue ever filled: {workers:?}"
    );
    assert!(workers.iter().all(|worker| worker.double_replies == 0));
    assert!(
        workers
            .iter()
            .all(|worker| worker.replies_without_request == 0)
    );
}

#[test]
fn two_machines_flooding_each_other_through_full_queues_make_progress() {
    // Capacity 1 and two workers: the order machine and partition 0 on worker 0,
    // partition 1 on worker 1. Every publish into partition 1 sends an `Enqueue` back
    // to worker 0, every pull of such a pocket sends a `Claim` to worker 1, every
    // release sends a `PocketBecame` back: both directions through queues of one.
    let started = Instant::now();
    let pool = PoolConfig::default()
        .with_worker_threads(2)
        .with_partitions(2)
        .with_queue_capacity(1)
        .with_tick_interval(Duration::from_millis(5));
    let outcome = Pool::start(Config::default(), pool, |handle| {
        let deadline = Instant::now() + StdDuration::from_millis(300);
        clients(handle, (4, 4), deadline, (true, false))
    })
    .unwrap();

    let (requests, replies) = outcome.result().clone().unwrap();
    assert!(requests > 0);
    assert_eq!(requests, replies, "exactly one reply per request");
    let elapsed = started.elapsed();
    assert!(elapsed < StdDuration::from_secs(10), "took {elapsed:?}");
    crossed_both_ways(&outcome.stats().workers);
    settled(&outcome);
}

#[test]
fn a_publisher_s_publishes_into_one_pocket_arrive_in_order() {
    let pool = PoolConfig::default()
        .with_worker_threads(4)
        .with_partitions(8)
        .with_tick_interval(Duration::from_millis(5));
    let outcome = Pool::start(Config::default(), pool, |handle| {
        // Pipelined: every publish is sent before any reply is read, so the order the
        // replies show is the order the queue kept.
        let receivers: Result<Vec<_>, _> = (1..=2000)
            .map(|rank| handle.send(publish("p", "ordered", "r", rank)))
            .collect();
        receivers
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|receiver| receiver.recv_timeout(REPLY))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    })
    .unwrap();

    let replies = outcome.result().clone().unwrap();
    assert_eq!(replies.len(), 2000);
    assert_eq!(replies[0], Response::Published(Published::NewPocket));
    for (i, reply) in replies.iter().enumerate().skip(1) {
        let superseded = Response::Published(Published::Superseded);
        assert_eq!(*reply, superseded, "publish {} out of order", i + 1);
    }
    let key = PocketKey::new("ordered");
    let machine = machine_of(&outcome, &key);
    let pocket = machine.pocket(&key).unwrap();
    assert_eq!(pocket.records[&RecordKey::new("r")].rank, Rank::new(2000));
    assert_eq!(pocket.superseded.len(), 1999);
    let ranks: Vec<u64> = pocket.superseded.iter().map(|s| s.rank.get()).collect();
    assert_eq!(ranks, (1..2000).collect::<Vec<_>>());
    let stats = &outcome.stats().machines[&MachineId::Partition(machine.partition())];
    assert_eq!(stats.inputs_stepped, 2000);
    settled(&outcome);
}

/// The lapse half's outcome: the body returns the lease that lapsed.
type RaceOutcome = Outcome<Result<LeaseId, String>>;

/// The lapse wins: the tick lapses the lease before the late renewal arrives.
fn lapse_wins(pool: PoolConfig) -> Result<RaceOutcome, String> {
    let config = Config::default().with_lease_ttl(Duration::from_millis(30));
    Pool::start(config, pool, |handle| {
        let published = Response::Published(Published::NewPocket);
        expect(handle, publish("p", "k", "r", 1), &published)?;
        let (lease, _, _) = batch(&call(handle, pull("c", 10))?)?;
        std::thread::sleep(StdDuration::from_millis(100));
        let refused = Response::Rejected(Reason::LeaseNotActive);
        expect(handle, renew("c", lease), &refused)?;
        Ok(lease)
    })
    .map_err(|error| error.to_string())
}

/// The renewal wins: it arrives before the lapse and pushes the expiry out, so the ticks
/// that follow leave the lease active. A longer TTL than the lapse half's keeps the
/// ordering certain while this thread shares the machine with the other tests; the race
/// the pool resolves is the same.
fn renewal_wins(pool: PoolConfig) -> Result<RaceOutcome, String> {
    let config = Config::default().with_lease_ttl(Duration::from_millis(200));
    Pool::start(config, pool, |handle| {
        let published = Response::Published(Published::NewPocket);
        expect(handle, publish("p", "k", "r", 1), &published)?;
        let (lease, _, _) = batch(&call(handle, pull("c", 10))?)?;
        let Response::Renewed { expires_at: first } = call(handle, renew("c", lease))? else {
            return Err("the renewal lost".to_owned());
        };
        // Several ticks later the lease is still active, and renews again, later.
        std::thread::sleep(StdDuration::from_millis(30));
        let Response::Renewed { expires_at: again } = call(handle, renew("c", lease))? else {
            return Err("the lease did not survive the ticks".to_owned());
        };
        if again <= first {
            return Err(format!(
                "the second renewal did not move the expiry: {again:?} <= {first:?}"
            ));
        }
        Ok(lease)
    })
    .map_err(|error| error.to_string())
}

/// The settled status of the body's lease and of the one record in pocket `k`.
fn lease_and_record(outcome: &RaceOutcome) -> Option<(LeaseStatus, RecordStatus)> {
    let key = PocketKey::new("k");
    let pocket = machine_of(outcome, &key).pocket(&key)?;
    let lease = outcome.result().clone().ok()?;
    let lease = pocket.lease(lease)?.status;
    let record = pocket.records.get(&RecordKey::new("r"))?.status;
    Some((lease, record))
}

#[test]
fn a_lapse_and_a_late_renewal_are_ordered_by_one_machine_either_way() {
    // Through the real pool both orders occur and each is handled as the rules say for
    // that order; the deterministic proof of both orders, with `now` chosen outright,
    // lives in tests/invariants.rs through `Direct`.
    let pool = PoolConfig::default()
        .with_worker_threads(2)
        .with_partitions(2)
        .with_tick_interval(Duration::from_millis(5));
    let outcome = lapse_wins(pool).unwrap();
    outcome.result().clone().unwrap();
    assert_eq!(
        lease_and_record(&outcome),
        Some((LeaseStatus::Expired, RecordStatus::Queued))
    );
    settled(&outcome);

    let outcome = renewal_wins(pool).unwrap();
    outcome.result().clone().unwrap();
    assert_eq!(
        lease_and_record(&outcome),
        Some((LeaseStatus::Active, RecordStatus::Held))
    );
    settled(&outcome);
}
