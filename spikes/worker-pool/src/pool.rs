//! The pool: the worker threads, the bounded queues between them, the clock that stamps
//! each input, the tick, and the start and stop. The service side of the spike: the one
//! module, with the load generator and the binary, that reaches an effect.
//!
//! [`Pool::start`] spawns a fixed number of worker threads under
//! [`std::thread::scope`], gives each one two bounded inbound lanes (one clients block
//! on, one its peers `try_send` into) and a static share of the machines (the order
//! machine on worker 0, partition `p` on worker `p % N`), runs the caller's body with
//! a [`Handle`], and when the body returns stops the workers once nothing is in
//! flight, joins them and hands back the settled machines and the [`PoolStats`].
//!
//! A worker never blocks on a send: a message whose destination lane is full waits in
//! the sender's outbox, so two full queues cannot deadlock (the argument is on the
//! worker loop in the `worker` module). A worker's panic reaches the caller of
//! [`Pool::start`] as the original payload, after every other worker has exited.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::sync::{Arc, OnceLock};
use std::thread::{Builder, Scope, ScopedJoinHandle, Thread, ThreadId};
use std::time::Instant;

use crate::order::OrderMachine;
use crate::partition::route;
use crate::pocket::PocketMachine;
use crate::protocol::{Input, MachineId, Partition, Response};
use crate::time::{Config, Duration};

mod worker;

use worker::{Envelope, Lanes, MachineSlot, Settings, Worker, WorkerReport, worker_of};

type Payload = Box<dyn Any + Send + 'static>;

/// How the pool is shaped: threads, partitions, queue depth, tick and the one test hook.
///
/// ```
/// use worker_pool::pool::PoolConfig;
/// let pool = PoolConfig::default().with_worker_threads(3).with_partitions(8);
/// assert_eq!(pool.worker_threads, 3);
/// assert_eq!(pool.partitions, 8);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolConfig {
    /// How many worker threads to spawn; at least 1.
    pub worker_threads: usize,
    /// How many partitions, each with one pocket machine; at least 1.
    pub partitions: u32,
    /// The depth of each worker's inbound queue; at least 1.
    pub queue_capacity: usize,
    /// How often every machine receives [`Input::Tick`].
    pub tick_interval: Duration,
    /// The one test hook: panic on the `n`-th enveloped input (ticks do not count)
    /// stepped into this machine, so that a test can watch the panic reach the caller.
    pub panic_on: Option<(MachineId, u64)>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            worker_threads: 1,
            partitions: 1,
            queue_capacity: 1024,
            tick_interval: Duration::from_millis(10),
            panic_on: None,
        }
    }
}

impl PoolConfig {
    /// The same configuration with this many worker threads, at least 1.
    #[must_use]
    pub const fn with_worker_threads(mut self, worker_threads: usize) -> Self {
        self.worker_threads = if worker_threads == 0 {
            1
        } else {
            worker_threads
        };
        self
    }

    /// The same configuration with this many partitions, at least 1.
    #[must_use]
    pub const fn with_partitions(mut self, partitions: u32) -> Self {
        self.partitions = if partitions == 0 { 1 } else { partitions };
        self
    }

    /// The same configuration with this inbound queue depth, at least 1.
    #[must_use]
    pub const fn with_queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = if queue_capacity == 0 {
            1
        } else {
            queue_capacity
        };
        self
    }

    /// The same configuration with this tick interval.
    #[must_use]
    pub const fn with_tick_interval(mut self, tick_interval: Duration) -> Self {
        self.tick_interval = tick_interval;
        self
    }

    /// The same configuration poisoned: the `nth` enveloped input into `machine` panics.
    #[must_use]
    pub const fn with_panic_on(mut self, machine: MachineId, nth: u64) -> Self {
        self.panic_on = Some((machine, nth));
        self
    }
}

/// Why a client input could not be sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SendError {
    /// The input is one the machines send each other, or the tick; a client never sends it.
    NotAClientInput,
    /// The owning worker is gone: the pool stopped or a worker panicked.
    PoolStopped,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAClientInput => f.write_str("the input is not one a client sends"),
            Self::PoolStopped => f.write_str("the pool has stopped"),
        }
    }
}

impl std::error::Error for SendError {}

/// Why a call got no response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CallError {
    /// The input could not be sent.
    Send(SendError),
    /// No reply within the timeout.
    Timeout,
    /// The reply handle was dropped without a reply: the machine stepping the input
    /// panicked, or the pool aborted.
    NoReply,
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Send(error) => write!(f, "the call was not sent: {error}"),
            Self::Timeout => f.write_str("no reply within the timeout"),
            Self::NoReply => f.write_str("the reply handle was dropped without a reply"),
        }
    }
}

impl std::error::Error for CallError {}

impl From<SendError> for CallError {
    fn from(error: SendError) -> Self {
        Self::Send(error)
    }
}

/// Why the pool could not run.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PoolError {
    /// The operating system refused a worker thread.
    Spawn(String),
    /// A worker came back without a machine it was given; the pool's own defect.
    MachineLost(MachineId),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(f, "a worker thread could not be spawned: {error}"),
            Self::MachineLost(id) => write!(f, "a worker lost its machine {id:?}"),
        }
    }
}

impl std::error::Error for PoolError {}

/// What one worker thread counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WorkerStats {
    /// Enveloped inputs stepped (ticks not included).
    pub inputs_stepped: u64,
    /// Messages one machine sent another through this worker, local or not.
    pub messages_crossed: u64,
    /// Of those, the ones for a machine on this same worker.
    pub local_deliveries: u64,
    /// Of those, the ones handed to another worker's queue.
    pub channel_deliveries: u64,
    /// The most messages ever waiting in this worker's outboxes at once.
    pub outbox_high_water: u64,
    /// Ticks stepped into this worker's machines.
    pub ticks: u64,
    /// Nanoseconds spent inside `Machine::step`.
    pub busy_nanos: u64,
    /// Replies a machine produced for an input that was already answered.
    pub double_replies: u64,
    /// Replies a machine produced for an input that carried no reply handle.
    pub replies_without_request: u64,
    /// Envelopes for a machine this worker does not own; zero in every correct run.
    pub misrouted: u64,
}

impl WorkerStats {
    /// All zeros.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Which worker and which thread stepped one machine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct MachineStats {
    /// The worker indices that stepped the machine; one in every correct run.
    pub stepped_by: BTreeSet<usize>,
    /// The threads that stepped the machine, deduplicated; one in every correct run.
    pub thread_ids: Vec<ThreadId>,
    /// Enveloped inputs stepped into the machine.
    pub inputs_stepped: u64,
}

impl MachineStats {
    /// Nothing recorded.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one step by this worker on this thread.
    pub fn record(&mut self, worker: usize, thread: ThreadId) {
        self.stepped_by.insert(worker);
        if !self.thread_ids.contains(&thread) {
            self.thread_ids.push(thread);
        }
    }
}

/// What the pool counted over its life.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolStats {
    /// The worker threads the pool ran.
    pub worker_threads: usize,
    /// The partitions.
    pub partitions: u32,
    /// The workers whose join returned; equals `worker_threads` after a normal stop.
    pub joined_workers: usize,
    /// Whether a worker saw a peer gone, or a panic set the flag.
    pub aborted: bool,
    /// Every worker's counters, by worker index.
    pub workers: Vec<WorkerStats>,
    /// Every machine's record of who stepped it.
    pub machines: BTreeMap<MachineId, MachineStats>,
}

impl PoolStats {
    /// Nothing counted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// What [`Pool::start`] hands back: the body's result, the settled machines and the
/// counters.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Outcome<R> {
    result: R,
    order: OrderMachine,
    pockets: Vec<PocketMachine>,
    stats: PoolStats,
}

impl<R> Outcome<R> {
    /// The body's result.
    #[must_use]
    pub const fn result(&self) -> &R {
        &self.result
    }

    /// The body's result, moved out.
    #[must_use]
    pub fn into_result(self) -> R {
        self.result
    }

    /// The order machine, settled.
    #[must_use]
    pub const fn order(&self) -> &OrderMachine {
        &self.order
    }

    /// The pocket machines, settled, by partition index.
    #[must_use]
    pub fn pockets(&self) -> &[PocketMachine] {
        &self.pockets
    }

    /// The counters.
    #[must_use]
    pub const fn stats(&self) -> &PoolStats {
        &self.stats
    }
}

/// What the handle and every worker share: the lane senders, the worker threads to
/// wake, and the flags.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) pool: PoolConfig,
    /// Each worker's client lane: clients block on it; no worker sends into it.
    pub(crate) client_lane: Vec<SyncSender<Envelope>>,
    /// Each worker's peer lane: workers `try_send` into it; no client touches it.
    pub(crate) peer_lane: Vec<SyncSender<Envelope>>,
    /// Each worker's thread, set by the worker before its first park.
    pub(crate) threads: Vec<OnceLock<Thread>>,
    /// Set when the body has returned: workers exit once nothing is in flight.
    pub(crate) stop: AtomicBool,
    /// Set when a worker panicked or saw a peer gone: workers exit at once.
    pub(crate) aborted: AtomicBool,
    /// Every envelope anywhere (a queue, an outbox, or being stepped) plus every tick in
    /// progress: zero means nothing can produce another message.
    pub(crate) in_flight: AtomicUsize,
    /// Worker threads that have not exited.
    pub(crate) live_workers: AtomicUsize,
}

impl Shared {
    pub(crate) fn new(pool: PoolConfig) -> (Self, Vec<Lanes>) {
        let mut client_lane = Vec::with_capacity(pool.worker_threads);
        let mut peer_lane = Vec::with_capacity(pool.worker_threads);
        let mut lanes = Vec::with_capacity(pool.worker_threads);
        for _ in 0..pool.worker_threads {
            let (clients_tx, clients) = sync_channel::<Envelope>(pool.queue_capacity);
            let (peers_tx, peers) = sync_channel::<Envelope>(pool.queue_capacity);
            client_lane.push(clients_tx);
            peer_lane.push(peers_tx);
            lanes.push(Lanes { clients, peers });
        }
        let shared = Self {
            pool,
            client_lane,
            peer_lane,
            threads: (0..pool.worker_threads).map(|_| OnceLock::new()).collect(),
            stop: AtomicBool::new(false),
            aborted: AtomicBool::new(false),
            in_flight: AtomicUsize::new(0),
            live_workers: AtomicUsize::new(pool.worker_threads),
        };
        (shared, lanes)
    }

    /// Wakes one parked worker; a worker not yet registered has not parked.
    pub(crate) fn wake(&self, worker: usize) {
        if let Some(thread) = self.threads.get(worker).and_then(OnceLock::get) {
            thread.unpark();
        }
    }

    /// Wakes every worker, so that a stop or an abort is seen at once.
    pub(crate) fn wake_all(&self) {
        for worker in 0..self.threads.len() {
            self.wake(worker);
        }
    }

    /// Asks every worker to exit at once.
    pub(crate) fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
        self.stop.store(true, Ordering::SeqCst);
        self.wake_all();
    }

    /// Asks every worker to exit once nothing is in flight.
    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.wake_all();
    }
}

/// A client's way into the pool: routes an input to the worker that owns its machine and
/// hands back the reply handle. Clone it into client threads spawned inside the body.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Handle {
    shared: Arc<Shared>,
}

impl Handle {
    /// Sends one client input, blocking while the owning worker's queue is full, and
    /// returns the receiver its one reply arrives on.
    ///
    /// # Errors
    ///
    /// `NotAClientInput` when the input is one the machines send each other or the
    /// tick; `PoolStopped` when the owning worker is gone.
    pub fn send(&self, input: Input) -> Result<Receiver<Response>, SendError> {
        let partitions = self.shared.pool.partitions;
        let to = route(&input, partitions).ok_or(SendError::NotAClientInput)?;
        if self.shared.aborted.load(Ordering::SeqCst) {
            return Err(SendError::PoolStopped);
        }
        let (reply, receiver) = sync_channel::<Response>(1);
        let envelope = Envelope::new(to, input, Some(reply));
        let worker = worker_of(to, self.shared.pool.worker_threads);
        // Counted before it exists anywhere, so no worker sees zero in flight while
        // this envelope is about to enter a queue.
        self.shared.in_flight.fetch_add(1, Ordering::SeqCst);
        if self.shared.client_lane[worker].send(envelope).is_ok() {
            self.shared.wake(worker);
            Ok(receiver)
        } else {
            self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
            Err(SendError::PoolStopped)
        }
    }

    /// Sends one client input and waits up to `timeout` for its reply.
    ///
    /// # Errors
    ///
    /// The send's error, `Timeout` when no reply arrived in time, `NoReply` when the
    /// reply handle was dropped unanswered (the pool aborted).
    pub fn call(&self, input: Input, timeout: std::time::Duration) -> Result<Response, CallError> {
        let receiver = self.send(input)?;
        let deadline = Instant::now() + timeout;
        loop {
            let slice = deadline
                .saturating_duration_since(Instant::now())
                .min(std::time::Duration::from_millis(10));
            match receiver.recv_timeout(slice) {
                Ok(response) => return Ok(response),
                Err(RecvTimeoutError::Disconnected) => return Err(CallError::NoReply),
                Err(RecvTimeoutError::Timeout) => {}
            }
            if self.shared.aborted.load(Ordering::SeqCst) {
                return Err(CallError::NoReply);
            }
            if Instant::now() >= deadline {
                return Err(CallError::Timeout);
            }
        }
    }

    /// Worker threads that have not exited yet.
    #[must_use]
    pub fn live_workers(&self) -> usize {
        self.shared.live_workers.load(Ordering::SeqCst)
    }

    /// Whether a worker panicked or saw a peer gone.
    #[must_use]
    pub fn aborted(&self) -> bool {
        self.shared.aborted.load(Ordering::SeqCst)
    }
}

/// The pool's one entry point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pool;

impl Pool {
    /// Runs the pool around `body`: spawns the workers, runs the body on the calling
    /// thread with a [`Handle`], then stops the workers once nothing is in flight,
    /// joins them and returns the settled machines, the body's result and the stats.
    ///
    /// ```
    /// use worker_pool::pool::{Pool, PoolConfig};
    /// use worker_pool::{Config, ConsumerId, Input, Response};
    /// let outcome = Pool::start(Config::default(), PoolConfig::default(), |handle| {
    ///     let pull = Input::Pull { consumer: ConsumerId::new("c"), limit: 1 };
    ///     handle.call(pull, std::time::Duration::from_secs(5))
    /// })
    /// .unwrap();
    /// assert_eq!(*outcome.result(), Ok(Response::NoPocketAvailable));
    /// assert_eq!(outcome.stats().joined_workers, 1);
    /// ```
    ///
    /// # Errors
    ///
    /// `Spawn` when a worker thread could not be started; `MachineLost` when a worker
    /// came back without one of its machines, which is the pool's own defect.
    ///
    /// # Panics
    ///
    /// Re-raises, with its original payload, the panic of a worker or of the body,
    /// after every worker thread has exited.
    pub fn start<R>(
        config: Config,
        pool: PoolConfig,
        body: impl FnOnce(&Handle) -> R,
    ) -> Result<Outcome<R>, PoolError> {
        let (shared, lanes) = Shared::new(pool);
        let shared = Arc::new(shared);
        let handle = Handle {
            shared: Arc::clone(&shared),
        };
        let epoch = Instant::now();
        let run = std::thread::scope(|scope| {
            let joins = match spawn_workers(scope, &shared, &config, lanes, epoch) {
                Ok(joins) => joins,
                Err((error, joins)) => {
                    shared.abort();
                    return Err((error, join_all(joins)));
                }
            };
            let result = catch_unwind(AssertUnwindSafe(|| body(&handle)));
            if result.is_err() {
                shared.abort();
            }
            shared.stop();
            Ok((result, join_all(joins)))
        });
        // Every worker is joined by now, so re-raising cannot collide with a join. A
        // worker's panic is re-raised ahead of the body's: it is the likelier cause.
        match run {
            Err((_, Err(payload))) | Ok((_, Err(payload)) | (Err(payload), Ok(_))) => {
                resume_unwind(payload)
            }
            Err((error, Ok(_))) => Err(error),
            Ok((Ok(result), Ok(reports))) => assemble(result, &shared, reports),
        }
    }
}

type Spawned<'scope> = Vec<ScopedJoinHandle<'scope, WorkerReport>>;

/// Spawns one worker per pair of lanes, giving worker 0 the order machine and worker
/// `p % N` partition `p`. On a spawn failure, hands back the handles already running.
fn spawn_workers<'scope, 'env>(
    scope: &'scope Scope<'scope, 'env>,
    shared: &'env Arc<Shared>,
    config: &Config,
    lanes: Vec<Lanes>,
    epoch: Instant,
) -> Result<Spawned<'scope>, (PoolError, Spawned<'scope>)> {
    let pool = shared.pool;
    let mut joins = Vec::with_capacity(pool.worker_threads);
    for (index, lanes) in lanes.into_iter().enumerate() {
        let machines = machines_for(index, config, pool);
        let settings = Settings::new(epoch, pool.tick_interval.as_std(), pool.panic_on);
        let worker = Worker::new(index, lanes, shared, machines, settings);
        let spawned = Builder::new()
            .name(format!("worker-pool-{index}"))
            .spawn_scoped(scope, move || worker.run());
        match spawned {
            Ok(join) => joins.push(join),
            Err(error) => return Err((PoolError::Spawn(error.to_string()), joins)),
        }
    }
    Ok(joins)
}

/// The machines worker `index` owns for the pool's whole life.
fn machines_for(index: usize, config: &Config, pool: PoolConfig) -> Vec<MachineSlot> {
    let mut machines = Vec::new();
    if index == 0 {
        machines.push(MachineSlot::Order(OrderMachine::new(*config)));
    }
    for p in 0..pool.partitions {
        let id = MachineId::Partition(Partition::new(p));
        if worker_of(id, pool.worker_threads) == index {
            machines.push(MachineSlot::Pocket(PocketMachine::new(
                Partition::new(p),
                pool.partitions,
                *config,
            )));
        }
    }
    machines
}

/// Joins every worker, then reports the first panic payload if any worker panicked.
fn join_all(joins: Spawned<'_>) -> Result<Vec<WorkerReport>, Payload> {
    let mut reports = Vec::with_capacity(joins.len());
    let mut first_panic = None;
    for join in joins {
        match join.join() {
            Ok(report) => reports.push(report),
            Err(payload) => {
                if first_panic.is_none() {
                    first_panic = Some(payload);
                }
            }
        }
    }
    first_panic.map_or(Ok(reports), Err)
}

/// Builds the outcome from the workers' reports.
fn assemble<R>(
    result: R,
    shared: &Shared,
    mut reports: Vec<WorkerReport>,
) -> Result<Outcome<R>, PoolError> {
    reports.sort_by_key(|report| report.index);
    let mut stats = PoolStats {
        worker_threads: shared.pool.worker_threads,
        partitions: shared.pool.partitions,
        joined_workers: reports.len(),
        aborted: shared.aborted.load(Ordering::SeqCst),
        ..PoolStats::default()
    };
    let mut order = None;
    let mut pockets = Vec::new();
    for report in reports {
        stats.workers.push(report.stats);
        for (slot, machine_stats) in report.machines {
            match slot {
                MachineSlot::Order(machine) => {
                    stats.machines.insert(MachineId::Order, machine_stats);
                    order = Some(machine);
                }
                MachineSlot::Pocket(machine) => {
                    stats
                        .machines
                        .insert(MachineId::Partition(machine.partition()), machine_stats);
                    pockets.push(machine);
                }
            }
        }
    }
    pockets.sort_by_key(PocketMachine::partition);
    let order = order.ok_or(PoolError::MachineLost(MachineId::Order))?;
    Ok(Outcome {
        result,
        order,
        pockets,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::{CallError, MachineStats, PoolConfig, SendError};
    use crate::protocol::{MachineId, Partition};
    use crate::time::Duration;

    #[test]
    fn the_builders_clamp_to_at_least_one() {
        let pool = PoolConfig::default()
            .with_worker_threads(0)
            .with_partitions(0)
            .with_queue_capacity(0)
            .with_tick_interval(Duration::from_millis(3))
            .with_panic_on(MachineId::Partition(Partition::new(1)), 2);
        assert_eq!(pool.worker_threads, 1);
        assert_eq!(pool.partitions, 1);
        assert_eq!(pool.queue_capacity, 1);
        assert_eq!(pool.tick_interval, Duration::from_millis(3));
        assert_eq!(
            pool.panic_on,
            Some((MachineId::Partition(Partition::new(1)), 2))
        );
    }

    #[test]
    fn machine_stats_dedupe_workers_and_threads() {
        let mut stats = MachineStats::new();
        let thread = std::thread::current().id();
        stats.record(2, thread);
        stats.record(2, thread);
        assert_eq!(stats.stepped_by.len(), 1);
        assert_eq!(stats.thread_ids, vec![thread]);
        let other = std::thread::spawn(|| std::thread::current().id())
            .join()
            .unwrap();
        stats.record(3, other);
        assert_eq!(stats.stepped_by.len(), 2);
        assert_eq!(stats.thread_ids.len(), 2);
    }

    #[test]
    fn the_errors_describe_themselves() {
        assert_eq!(
            CallError::from(SendError::PoolStopped),
            CallError::Send(SendError::PoolStopped)
        );
        assert!(CallError::Timeout.to_string().contains("timeout"));
        assert!(
            SendError::NotAClientInput
                .to_string()
                .contains("not one a client sends")
        );
    }
}
