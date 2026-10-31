//! One worker thread: its two inbound lanes, its machines, its outboxes and the loop
//! that ties them together.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use super::{MachineStats, Shared, WorkerStats};
use crate::machine::Machine;
use crate::order::OrderMachine;
use crate::pocket::PocketMachine;
use crate::protocol::{Input, MachineId, Output, Response};
use crate::time::Timestamp;

/// The most local-queue items stepped per iteration before the lanes are looked at.
const LOCAL_BURST: usize = 64;
/// The most items taken from one lane per iteration.
const LANE_BURST: usize = 64;

/// What travels through a queue: the input, the machine it is for, and the client's
/// reply handle when there is one.
#[derive(Debug)]
pub(crate) struct Envelope {
    to: MachineId,
    input: Input,
    reply: Option<SyncSender<Response>>,
}

impl Envelope {
    pub(crate) const fn new(
        to: MachineId,
        input: Input,
        reply: Option<SyncSender<Response>>,
    ) -> Self {
        Self { to, input, reply }
    }
}

/// A worker's two inbound receivers: the lane clients block on, and the lane peers
/// `try_send` into, which no client ever touches.
#[derive(Debug)]
pub(crate) struct Lanes {
    pub(crate) clients: Receiver<Envelope>,
    pub(crate) peers: Receiver<Envelope>,
}

/// The worker that owns a machine: the order machine on worker 0, partition `p` on
/// worker `p % worker_threads`.
pub(crate) const fn worker_of(machine: MachineId, worker_threads: usize) -> usize {
    let threads = if worker_threads == 0 {
        1
    } else {
        worker_threads
    };
    match machine {
        MachineId::Order => 0,
        MachineId::Partition(partition) => (partition.index() as usize) % threads,
    }
}

/// One machine as a worker holds it.
#[derive(Clone, Debug)]
pub(crate) enum MachineSlot {
    Order(OrderMachine),
    Pocket(PocketMachine),
}

impl MachineSlot {
    fn id(&self) -> MachineId {
        match self {
            Self::Order(machine) => machine.id(),
            Self::Pocket(machine) => machine.id(),
        }
    }

    fn step(&mut self, input: Input, now: Timestamp) -> Vec<Output> {
        match self {
            Self::Order(machine) => machine.step(input, now),
            Self::Pocket(machine) => machine.step(input, now),
        }
    }
}

/// The clock and the test hook a worker runs with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings {
    epoch: Instant,
    tick_interval: Duration,
    panic_on: Option<(MachineId, u64)>,
}

impl Settings {
    pub(crate) const fn new(
        epoch: Instant,
        tick_interval: Duration,
        panic_on: Option<(MachineId, u64)>,
    ) -> Self {
        Self {
            epoch,
            tick_interval,
            panic_on,
        }
    }
}

/// What a worker hands back when it exits normally.
#[derive(Debug)]
pub(crate) struct WorkerReport {
    pub(crate) index: usize,
    pub(crate) machines: Vec<(MachineSlot, MachineStats)>,
    pub(crate) stats: WorkerStats,
}

/// A machine and the record of who stepped it.
#[derive(Debug)]
struct Owned {
    slot: MachineSlot,
    stats: MachineStats,
}

/// One worker thread's state.
#[derive(Debug)]
pub(crate) struct Worker<'a> {
    index: usize,
    lanes: Lanes,
    shared: &'a Shared,
    machines: BTreeMap<MachineId, Owned>,
    /// One queue per destination worker; the one at `index` is the local queue.
    outbox: Vec<VecDeque<Envelope>>,
    settings: Settings,
    next_tick: Instant,
    thread: ThreadId,
    stats: WorkerStats,
}

/// Marks the worker's exit however it happens: a panic sets the flags that release the
/// other workers, and either way the live count drops.
struct ExitGuard<'a> {
    shared: &'a Shared,
}

impl Drop for ExitGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.shared.abort();
        }
        self.shared.live_workers.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<'a> Worker<'a> {
    pub(crate) fn new(
        index: usize,
        lanes: Lanes,
        shared: &'a Shared,
        machines: Vec<MachineSlot>,
        settings: Settings,
    ) -> Self {
        let outbox = shared.peer_lane.iter().map(|_| VecDeque::new()).collect();
        let machines = machines
            .into_iter()
            .map(|slot| {
                let id = slot.id();
                let owned = Owned {
                    slot,
                    stats: MachineStats::new(),
                };
                (id, owned)
            })
            .collect();
        Self {
            index,
            lanes,
            shared,
            machines,
            outbox,
            settings,
            next_tick: settings.epoch + settings.tick_interval,
            thread: std::thread::current().id(),
            stats: WorkerStats::new(),
        }
    }

    /// Runs the loop to its end and reports; a panic inside releases the other workers
    /// through the exit guard and the dropped lane receivers, and unwinds to the join.
    pub(crate) fn run(mut self) -> WorkerReport {
        let _guard = ExitGuard {
            shared: self.shared,
        };
        let current = std::thread::current();
        self.thread = current.id();
        // Registered before the first park, so a sender that finds the slot empty
        // knows this worker has not parked yet and will look at its lanes anyway.
        let _ = self.shared.threads[self.index].set(current);
        self.run_loop();
        WorkerReport {
            index: self.index,
            machines: self
                .machines
                .into_values()
                .map(|owned| (owned.slot, owned.stats))
                .collect(),
            stats: self.stats,
        }
    }

    /// The loop. Each iteration ticks if due, pushes the outboxes towards the peers,
    /// steps a burst of the local queue, a burst of the peer lane, then a burst of the
    /// client lane, and parks only when all of that found nothing to do.
    ///
    /// Why two full queues cannot deadlock: no worker ever blocks on a send. A message
    /// whose destination lane is full waits in this worker's outbox, and the worker goes
    /// on to drain its own lanes every iteration whatever its peers are doing; so every
    /// full lane is always being emptied by a worker that is never itself waiting,
    /// every pending `try_send` eventually succeeds, and the one blocking send in the
    /// pool, a client's, blocks on a lane a live worker drains.
    ///
    /// Why the peers have their own lane: a client blocked in `send` on a full
    /// `sync_channel` wins the freed slot from a peer's `try_send` nearly every time,
    /// so with one shared queue a saturating client load starved machine-to-machine
    /// traffic (49 deliveries in a 300 ms flood on the first cut). On their own lane
    /// the peers contend with nobody, and the client lane's back-pressure falls on the
    /// clients alone. What stays unfair is client against client on the client lane,
    /// where `std::sync::mpsc` promises no order among blocked senders; and an outbox
    /// is unbounded, growing while a peer takes less than this worker produces.
    ///
    /// Everything one machine sends another travels through the same outbox to the same
    /// peer lane, so the per-destination FIFO the machines rely on (a `PocketBecame`
    /// before the `RetryPull` that follows it) holds across the lane split.
    fn run_loop(&mut self) {
        loop {
            if self.should_exit() {
                return;
            }
            self.tick_if_due();
            self.drain_remote_outboxes();
            let mut busy = self.drain_local(LOCAL_BURST) > 0;
            busy |= self.drain_lane(Lane::Peers, LANE_BURST) > 0;
            busy |= self.drain_lane(Lane::Clients, LANE_BURST) > 0;
            if !busy {
                self.idle();
            }
        }
    }

    /// Exit at once on abort; exit on stop once nothing anywhere can produce a message.
    fn should_exit(&self) -> bool {
        if self.shared.aborted.load(Ordering::SeqCst) {
            return true;
        }
        self.shared.stop.load(Ordering::SeqCst)
            && self.shared.in_flight.load(Ordering::SeqCst) == 0
            && self.outbox.iter().all(VecDeque::is_empty)
    }

    /// Steps `Input::Tick` into every owned machine when the interval has elapsed. The
    /// tick holds an in-flight token while it runs so that no peer exits under its
    /// outputs; after stop no tick runs at all.
    fn tick_if_due(&mut self) {
        let now = Instant::now();
        if now < self.next_tick {
            return;
        }
        self.next_tick += self.settings.tick_interval;
        if self.next_tick < now {
            self.next_tick = now + self.settings.tick_interval;
        }
        self.shared.in_flight.fetch_add(1, Ordering::SeqCst);
        if self.shared.stop.load(Ordering::SeqCst) {
            self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
            return;
        }
        let stamp = self.stamp();
        let ids: Vec<MachineId> = self.machines.keys().copied().collect();
        for id in ids {
            let outputs = self.step_machine(id, Input::Tick, stamp);
            self.handle_outputs(outputs, None);
        }
        self.stats.ticks += 1;
        self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    /// Pushes each remote outbox towards its peer until one is full or all are empty.
    fn drain_remote_outboxes(&mut self) {
        for worker in 0..self.outbox.len() {
            if worker == self.index {
                continue;
            }
            while self.push_front_to_peer(worker) {}
        }
    }

    /// Tries the front of one outbox on its peer's lane. True when something was
    /// delivered and more may follow; false when the outbox is empty, the lane is full
    /// (the item goes back to the front) or the peer is gone (the pool aborts).
    fn push_front_to_peer(&mut self, worker: usize) -> bool {
        let Some(envelope) = self.outbox[worker].pop_front() else {
            return false;
        };
        match self.shared.peer_lane[worker].try_send(envelope) {
            Ok(()) => {
                self.stats.channel_deliveries += 1;
                self.shared.wake(worker);
                true
            }
            Err(TrySendError::Full(envelope)) => {
                self.outbox[worker].push_front(envelope);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.shared.abort();
                false
            }
        }
    }

    /// Steps up to `limit` envelopes from the local queue; returns how many.
    fn drain_local(&mut self, limit: usize) -> usize {
        for taken in 0..limit {
            let Some(envelope) = self.outbox[self.index].pop_front() else {
                return taken;
            };
            self.step_envelope(envelope);
        }
        limit
    }

    /// Steps up to `limit` envelopes waiting on one lane, without blocking; returns how
    /// many. A disconnected lane cannot happen while the handle holds a sender, and is
    /// treated as an exit.
    fn drain_lane(&mut self, lane: Lane, limit: usize) -> usize {
        for taken in 0..limit {
            let receiver = match lane {
                Lane::Peers => &self.lanes.peers,
                Lane::Clients => &self.lanes.clients,
            };
            match receiver.try_recv() {
                Ok(envelope) => self.step_envelope(envelope),
                Err(TryRecvError::Empty) => return taken,
                Err(TryRecvError::Disconnected) => {
                    self.shared.abort();
                    return taken;
                }
            }
        }
        limit
    }

    /// Nothing was found to do: yield while an outbox is pending, since the peer frees a
    /// slot at any moment and a sleep here was measured to throttle a capacity-1 hop to
    /// one message per sleep; else park until the next tick or a sender's wake-up.
    fn idle(&self) {
        if self.outbox.iter().any(|queue| !queue.is_empty()) {
            std::thread::yield_now();
            return;
        }
        let until_tick = self
            .next_tick
            .saturating_duration_since(Instant::now())
            .min(self.settings.tick_interval);
        std::thread::park_timeout(until_tick);
    }

    /// The pool's clock as a value: nanoseconds since the epoch.
    fn stamp(&self) -> Timestamp {
        Timestamp::from_nanos(
            u64::try_from(self.settings.epoch.elapsed().as_nanos()).unwrap_or(u64::MAX),
        )
    }

    /// Steps one envelope into its machine and performs the outputs, then releases the
    /// envelope's in-flight token.
    fn step_envelope(&mut self, envelope: Envelope) {
        let Envelope { to, input, reply } = envelope;
        if self.machines.contains_key(&to) {
            self.stats.inputs_stepped += 1;
            let stamp = self.stamp();
            let outputs = self.step_machine(to, input, stamp);
            self.handle_outputs(outputs, reply.as_ref());
        } else {
            self.stats.misrouted += 1;
        }
        self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    /// Steps one input into the machine `id`, recording the stepping worker and thread
    /// and the time spent inside the step. The poison hook counts enveloped inputs only.
    fn step_machine(&mut self, id: MachineId, input: Input, stamp: Timestamp) -> Vec<Output> {
        let Some(owned) = self.machines.get_mut(&id) else {
            return Vec::new();
        };
        owned.stats.record(self.index, self.thread);
        if input != Input::Tick {
            owned.stats.inputs_stepped += 1;
            // The deliberate poison the stop-and-panic test asks for: the one panic
            // outside a test module, reached only through `PoolConfig::panic_on`.
            assert!(
                self.settings.panic_on != Some((id, owned.stats.inputs_stepped)),
                "poisoned by PoolConfig::panic_on"
            );
        }
        let started = Instant::now();
        let outputs = owned.slot.step(input, stamp);
        self.stats.busy_nanos = self
            .stats
            .busy_nanos
            .saturating_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        outputs
    }

    /// Performs a step's outputs in order: a `Forward` inherits the reply handle, a
    /// `Notify` carries none, a `Reply` answers the client.
    fn handle_outputs(&mut self, outputs: Vec<Output>, reply: Option<&SyncSender<Response>>) {
        for output in outputs {
            match output {
                Output::Forward(to, input) => {
                    self.enqueue(Envelope::new(to, input, reply.cloned()));
                }
                Output::Notify(to, input) => self.enqueue(Envelope::new(to, input, None)),
                Output::Reply(response) => self.reply(reply, response),
            }
        }
    }

    /// Answers the client, counting a second reply or a reply nobody asked for.
    fn reply(&mut self, reply: Option<&SyncSender<Response>>, response: Response) {
        match reply {
            None => self.stats.replies_without_request += 1,
            Some(sender) => match sender.try_send(response) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => {}
                Err(TrySendError::Full(_)) => self.stats.double_replies += 1,
            },
        }
    }

    /// The outbox rule. A message for one of this worker's own machines joins the local
    /// queue, never stepped inline, so that it keeps FIFO with the remote case. A message
    /// for a peer goes behind anything already waiting for that peer, never overtaking;
    /// with nothing waiting it is tried on the peer lane at once and waits in the outbox
    /// only when the lane is full. A peer whose receiver is gone aborts the pool.
    fn enqueue(&mut self, envelope: Envelope) {
        // Counted before it exists anywhere, so no worker sees zero in flight while
        // this message is between the step that made it and a queue.
        self.shared.in_flight.fetch_add(1, Ordering::SeqCst);
        self.stats.messages_crossed += 1;
        let worker = worker_of(envelope.to, self.outbox.len());
        if worker == self.index {
            self.stats.local_deliveries += 1;
            self.outbox[worker].push_back(envelope);
        } else if !self.outbox[worker].is_empty() {
            self.outbox[worker].push_back(envelope);
        } else {
            match self.shared.peer_lane[worker].try_send(envelope) {
                Ok(()) => {
                    self.stats.channel_deliveries += 1;
                    self.shared.wake(worker);
                }
                Err(TrySendError::Full(envelope)) => self.outbox[worker].push_back(envelope),
                Err(TrySendError::Disconnected(_)) => {
                    self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
                    self.shared.abort();
                }
            }
        }
        let waiting: usize = self.outbox.iter().map(VecDeque::len).sum();
        let waiting = u64::try_from(waiting).unwrap_or(u64::MAX);
        self.stats.outbox_high_water = self.stats.outbox_high_water.max(waiting);
    }
}

/// Which inbound lane to drain.
#[derive(Clone, Copy, Debug)]
enum Lane {
    Peers,
    Clients,
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use super::{Envelope, Lanes, MachineSlot, Settings, Worker, worker_of};
    use crate::pool::{PoolConfig, Shared};
    use crate::protocol::{ConsumerId, Input, MachineId, Partition};

    fn pull(n: u64) -> Input {
        Input::Pull {
            consumer: ConsumerId::new(format!("c{n}")),
            limit: n,
        }
    }

    fn limit_of(envelope: &Envelope) -> u64 {
        match &envelope.input {
            Input::Pull { limit, .. } => *limit,
            _ => 0,
        }
    }

    /// The worker under test is index 1 and owns no machine; worker 0's lanes come back
    /// for the test to hold.
    fn worker_one(shared: &Shared, mut lanes: Vec<Lanes>) -> (Worker<'_>, Lanes) {
        let own = lanes.remove(1);
        let settings = Settings::new(Instant::now(), Duration::from_secs(1), None);
        let worker = Worker::new(1, own, shared, Vec::<MachineSlot>::new(), settings);
        (worker, lanes.remove(0))
    }

    #[test]
    fn worker_of_puts_order_on_zero_and_partitions_modulo_threads() {
        assert_eq!(worker_of(MachineId::Order, 3), 0);
        assert_eq!(worker_of(MachineId::Partition(Partition::new(0)), 3), 0);
        assert_eq!(worker_of(MachineId::Partition(Partition::new(4)), 3), 1);
        assert_eq!(worker_of(MachineId::Partition(Partition::new(5)), 3), 2);
        assert_eq!(worker_of(MachineId::Partition(Partition::new(5)), 0), 0);
    }

    /// Two workers with lanes of one; the worker under test is index 1 and sends
    /// towards worker 0, whose peer lane the test holds and has filled by hand.
    fn two_workers_with_a_full_lane() -> (Shared, Vec<Lanes>) {
        let pool = PoolConfig::default()
            .with_worker_threads(2)
            .with_queue_capacity(1);
        let (shared, lanes) = Shared::new(pool);
        shared.peer_lane[0]
            .try_send(Envelope::new(MachineId::Order, pull(0), None))
            .unwrap();
        (shared, lanes)
    }

    #[test]
    fn a_full_lane_appends_to_the_outbox() {
        let (shared, lanes) = two_workers_with_a_full_lane();
        let (mut worker, zero) = worker_one(&shared, lanes);
        worker.enqueue(Envelope::new(MachineId::Order, pull(1), None));
        assert_eq!(worker.outbox[0].len(), 1, "Full appends to the outbox");
        assert_eq!(worker.stats.channel_deliveries, 0);
        assert_eq!(worker.stats.messages_crossed, 1);
        assert_eq!(worker.stats.outbox_high_water, 1);
        assert_eq!(shared.in_flight.load(Ordering::SeqCst), 1);
        assert_eq!(limit_of(&zero.peers.try_recv().unwrap()), 0);
        assert!(
            zero.clients.try_recv().is_err(),
            "peers never use the client lane"
        );
    }

    #[test]
    fn a_non_empty_outbox_appends_behind_and_nothing_overtakes() {
        let (shared, lanes) = two_workers_with_a_full_lane();
        let (mut worker, zero) = worker_one(&shared, lanes);
        worker.enqueue(Envelope::new(MachineId::Order, pull(1), None));
        // The peer drains one, so the lane has room; a second message must still queue
        // behind the first instead of overtaking it.
        assert_eq!(limit_of(&zero.peers.try_recv().unwrap()), 0);
        worker.enqueue(Envelope::new(MachineId::Order, pull(2), None));
        assert_eq!(worker.outbox[0].len(), 2, "appends behind");
        assert!(
            zero.peers.try_recv().is_err(),
            "nothing went around the outbox"
        );
        assert_eq!(worker.stats.outbox_high_water, 2);

        // Draining delivers in order, stopping again when the lane is full.
        worker.drain_remote_outboxes();
        assert_eq!(worker.outbox[0].len(), 1);
        assert_eq!(limit_of(&zero.peers.try_recv().unwrap()), 1);
        worker.drain_remote_outboxes();
        assert!(worker.outbox[0].is_empty());
        assert_eq!(limit_of(&zero.peers.try_recv().unwrap()), 2);
        assert_eq!(worker.stats.channel_deliveries, 2);
        assert_eq!(worker.stats.messages_crossed, 2);
        assert_eq!(shared.in_flight.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_message_for_an_owned_machine_joins_the_local_queue() {
        let pool = PoolConfig::default()
            .with_worker_threads(2)
            .with_partitions(2)
            .with_queue_capacity(1);
        let (shared, lanes) = Shared::new(pool);
        let (mut worker, _zero) = worker_one(&shared, lanes);
        let own = MachineId::Partition(Partition::new(1));
        worker.enqueue(Envelope::new(own, pull(7), None));
        assert_eq!(worker.outbox[1].len(), 1);
        assert_eq!(worker.stats.local_deliveries, 1);
        assert_eq!(worker.stats.channel_deliveries, 0);
        // Stepping it finds no machine (the test worker owns none) and counts that.
        assert_eq!(worker.drain_local(64), 1);
        assert_eq!(worker.stats.misrouted, 1);
        assert_eq!(shared.in_flight.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_disconnected_peer_aborts_the_pool() {
        let pool = PoolConfig::default()
            .with_worker_threads(2)
            .with_queue_capacity(1);
        let (shared, lanes) = Shared::new(pool);
        let (mut worker, zero) = worker_one(&shared, lanes);
        drop(zero);
        worker.enqueue(Envelope::new(MachineId::Order, pull(1), None));
        assert!(shared.aborted.load(Ordering::SeqCst));
        assert!(worker.should_exit());
    }
}
