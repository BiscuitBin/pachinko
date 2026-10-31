//! The order machine: the pocket order, the counter that positions pockets, the
//! counter that mints lease identities, and the consumer to active lease routing that
//! holds `OneActiveLeasePerConsumer`. Decomposition B of ticket T20.
//!
//! The machine owns the module's `queue.next_position` for pockets and the pocket
//! order; the pocket machines own the records. A `Pull` is routed here first: under an
//! active lease it is forwarded to the lease's partition, otherwise the front pocket is
//! popped, a lease identity minted and a `Claim` forwarded. The pocket machines tell
//! this one when a pocket enters the order (`Enqueue`) and what it became when a lease
//! ended (`PocketBecame`).

use std::collections::{BTreeMap, BTreeSet};

use crate::machine::Machine;
use crate::protocol::{
    Became, ConsumerId, Input, LeaseId, MachineId, Output, Partition, PocketKey, PublisherId,
    Reason, Response,
};
use crate::state::{OrderStatus, OrderView};
use crate::time::{Config, Timestamp};

/// The order machine. One instance per pool, `MachineId::Order`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrderMachine {
    config: Config,
    /// The single writer of pocket positions.
    next_pocket_position: u64,
    /// The single writer of lease serials.
    next_lease_serial: u64,
    /// Every pocket the order machine has heard of.
    pockets: BTreeMap<PocketKey, OrderView>,
    /// The pocket order: queued pockets by position, front first.
    order: BTreeMap<u64, PocketKey>,
    /// Each consumer's active lease, as far as the order machine has heard.
    active: BTreeMap<ConsumerId, LeaseId>,
    consumers: BTreeSet<ConsumerId>,
    publishers: BTreeSet<PublisherId>,
    retry_pulls: u64,
    violations: u64,
}

impl OrderMachine {
    /// An order machine with nothing in it.
    ///
    /// ```
    /// use worker_pool::{Config, OrderMachine};
    /// let machine = OrderMachine::new(Config::default());
    /// assert_eq!(machine.queued().count(), 0);
    /// ```
    #[must_use]
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            next_pocket_position: 0,
            next_lease_serial: 0,
            pockets: BTreeMap::new(),
            order: BTreeMap::new(),
            active: BTreeMap::new(),
            consumers: BTreeSet::new(),
            publishers: BTreeSet::new(),
            retry_pulls: 0,
            violations: 0,
        }
    }

    /// The configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The view of one pocket.
    #[must_use]
    pub fn view(&self, key: &PocketKey) -> Option<&OrderView> {
        self.pockets.get(key)
    }

    /// Every pocket's view, by key.
    pub fn views(&self) -> impl Iterator<Item = (&PocketKey, &OrderView)> {
        self.pockets.iter()
    }

    /// The pocket order, front first: position and key.
    pub fn queued(&self) -> impl Iterator<Item = (u64, &PocketKey)> {
        self.order.iter().map(|(position, key)| (*position, key))
    }

    /// The lease the order machine believes this consumer holds.
    #[must_use]
    pub fn active_lease(&self, consumer: &ConsumerId) -> Option<LeaseId> {
        self.active.get(consumer).copied()
    }

    /// Every consumer with an active lease, as far as the order machine has heard.
    pub fn active_leases(&self) -> impl Iterator<Item = (&ConsumerId, LeaseId)> {
        self.active
            .iter()
            .map(|(consumer, lease)| (consumer, *lease))
    }

    /// `queue.next_position` for pockets.
    #[must_use]
    pub const fn next_pocket_position(&self) -> u64 {
        self.next_pocket_position
    }

    /// The serial the next lease will take.
    #[must_use]
    pub const fn next_lease_serial(&self) -> u64 {
        self.next_lease_serial
    }

    /// The consumers that joined.
    pub fn consumers(&self) -> impl Iterator<Item = &ConsumerId> {
        self.consumers.iter()
    }

    /// The publishers that joined.
    pub fn publishers(&self) -> impl Iterator<Item = &PublisherId> {
        self.publishers.iter()
    }

    /// How many `RetryPull` inputs arrived: the visible cost of the window between a
    /// pocket machine's decision and this machine's hearing of it.
    #[must_use]
    pub const fn retry_pulls(&self) -> u64 {
        self.retry_pulls
    }

    // Identity ------------------------------------------------------------------

    /// `PublisherJoins`: `requires: not exists Publisher{id}`; `ensures: Publisher.created`.
    fn publisher_joins(&mut self, publisher: PublisherId) -> Vec<Output> {
        if self.publishers.insert(publisher) {
            vec![Output::Reply(Response::Joined)]
        } else {
            vec![Output::Reply(Response::Rejected(
                Reason::DuplicatePublisher,
            ))]
        }
    }

    /// `ConsumerJoins`: `requires: not exists Consumer{id}`; `ensures: Consumer.created`.
    fn consumer_joins(&mut self, consumer: ConsumerId) -> Vec<Output> {
        if self.consumers.insert(consumer) {
            vec![Output::Reply(Response::Joined)]
        } else {
            vec![Output::Reply(Response::Rejected(Reason::DuplicateConsumer))]
        }
    }

    // Pull ----------------------------------------------------------------------

    /// `Pull(consumer, limit)`: the three pull rules, guards in the module's order.
    fn pull(&mut self, consumer: ConsumerId, limit: u64) -> Vec<Output> {
        if let Some(lease) = self.active.get(&consumer).copied() {
            return Self::pull_within_lease(consumer, lease, limit);
        }
        if let Some((position, key)) = self.order.pop_first() {
            return self.pull_claims_front_pocket(consumer, position, key, limit);
        }
        Self::pull_finds_no_pocket()
    }

    /// `RetryPull(consumer, limit)`: a `Pull` the pocket machine sent back because the
    /// lease this machine routed it under had already ended. Counted, then handled
    /// exactly as a `Pull`; the `PocketBecame` that ended the lease arrived first.
    fn retry_pull(&mut self, consumer: ConsumerId, limit: u64) -> Vec<Output> {
        self.retry_pulls += 1;
        self.pull(consumer, limit)
    }

    /// `PullWithinLease`, the routing half: `requires: exists Lease{consumer, active}`.
    /// The lease's partition holds the pocket and the records, so the pull is forwarded
    /// there under the client's reply handle; the pocket machine checks the rest of the
    /// guards and replies, or sends a `RetryPull` back when the lease has ended.
    fn pull_within_lease(consumer: ConsumerId, lease: LeaseId, limit: u64) -> Vec<Output> {
        vec![Output::Forward(
            MachineId::Partition(lease.partition),
            Input::PullUnderLease {
                consumer,
                lease,
                limit,
            },
        )]
    }

    /// `PullClaimsFrontPocket`, the claim half: `requires: not exists Lease{consumer,
    /// active}` and `free.count > 0`; `pocket = min_by(free, position)`. The front
    /// pocket leaves the order, the lease identity is minted with the pocket's
    /// partition and the next serial, the consumer's active lease is recorded, and a
    /// `Claim` is forwarded under the client's reply handle; the pocket machine creates
    /// the lease, holds the batch and replies.
    fn pull_claims_front_pocket(
        &mut self,
        consumer: ConsumerId,
        position: u64,
        key: PocketKey,
        limit: u64,
    ) -> Vec<Output> {
        let Some(view) = self.pockets.get_mut(&key) else {
            // Every order entry has a view; a missing one is this machine's own defect.
            self.violations += 1;
            return vec![Output::Reply(Response::Rejected(Reason::Protocol))];
        };
        let lease = LeaseId::new(view.partition, self.next_lease_serial);
        self.next_lease_serial += 1;
        view.status = OrderStatus::Leased {
            position,
            consumer: consumer.clone(),
            lease,
        };
        let partition = view.partition;
        self.active.insert(consumer.clone(), lease);
        vec![Output::Forward(
            MachineId::Partition(partition),
            Input::Claim {
                pocket_key: key,
                consumer,
                lease,
                limit,
            },
        )]
    }

    /// `PullFindsNoPocket`: `requires: not exists Lease{consumer, active}` and
    /// `free.count = 0`; `ensures: NoPocketAvailable(consumer)`.
    fn pull_finds_no_pocket() -> Vec<Output> {
        vec![Output::Reply(Response::NoPocketAvailable)]
    }

    // Enqueue -------------------------------------------------------------------

    /// `Enqueue(pocket_key, partition)`: a pocket entered the pocket order and takes
    /// `queue.next_position`. Which rule sent it depends on the view: absent is
    /// `PublishIntoNewPocket`, idle is `ReviveIdlePocket`, parked is
    /// `ReviveParkedPocket`. A view already queued or leased is a protocol violation;
    /// the pocket machine is authoritative for the status, so the pocket is requeued
    /// anyway, after any stale order entry is dropped.
    fn enqueue(&mut self, key: PocketKey, partition: Partition) -> Vec<Output> {
        match self.pockets.get(&key).map(|view| &view.status) {
            None => self.publish_into_new_pocket(key, partition),
            Some(OrderStatus::Idle) => self.revive_idle_pocket(key, partition),
            Some(OrderStatus::Parked) => self.revive_parked_pocket(key, partition),
            Some(OrderStatus::Queued { position }) => {
                let stale = *position;
                self.violations += 1;
                self.order.remove(&stale);
                self.queue_at_back(key, partition);
            }
            Some(OrderStatus::Leased { .. }) => {
                self.violations += 1;
                self.queue_at_back(key, partition);
            }
        }
        Vec::new()
    }

    /// `PublishIntoNewPocket`, the position half: `requires: not exists Pocket{key}`;
    /// `ensures: Pocket.created(status: queued, position: queue.next_position)` and
    /// the counter advances.
    fn publish_into_new_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.queue_at_back(key, partition);
    }

    /// `ReviveIdlePocket`, the position half: `requires: pocket.status = idle`;
    /// `ensures: status = queued, position = queue.next_position`.
    fn revive_idle_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.queue_at_back(key, partition);
    }

    /// `ReviveParkedPocket`, the position half: `requires: pocket.status = parked`;
    /// `ensures: status = queued, position = queue.next_position`.
    fn revive_parked_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.queue_at_back(key, partition);
    }

    // PocketBecame --------------------------------------------------------------

    /// `PocketBecame(pocket_key, became, consumer, lease)`: the lease this machine
    /// routed the consumer under has ended and the pocket left `leased`. The
    /// consumer's active lease is forgotten, then the pocket goes where `became` says.
    fn pocket_became(
        &mut self,
        key: PocketKey,
        became: Became,
        consumer: &ConsumerId,
        lease: LeaseId,
    ) -> Vec<Output> {
        let kept = self.ended_lease_position(&key, consumer, lease);
        if self.active.get(consumer) == Some(&lease) {
            self.active.remove(consumer);
        }
        // A pocket never heard of has no view to read the partition from; the lease
        // identity carries it.
        let partition = self
            .pockets
            .get(&key)
            .map_or(lease.partition, |view| view.partition);
        match became {
            Became::RequeuedAtBack => self.release_requeues_pocket(key, partition),
            Became::RequeuedKeepingPosition => {
                let position = kept.unwrap_or_else(|| self.fresh_position());
                self.lease_expiry_keeps_pocket_at_front(key, partition, position);
            }
            Became::Idle => self.idles_empty_pocket(key, partition),
            Became::Parked => self.lease_expiry_parks_pocket(key, partition),
        }
        Vec::new()
    }

    /// The position the view kept for the pocket whose lease ended, when the view is
    /// `Leased` by that consumer under that lease. Any other view is a protocol
    /// violation: it is counted, a stale order entry is dropped, and the position is
    /// still returned when the view has one.
    fn ended_lease_position(
        &mut self,
        key: &PocketKey,
        consumer: &ConsumerId,
        lease: LeaseId,
    ) -> Option<u64> {
        match self.pockets.get(key).map(|view| &view.status) {
            Some(OrderStatus::Leased {
                position,
                consumer: held_by,
                lease: under,
            }) if held_by == consumer && *under == lease => Some(*position),
            Some(OrderStatus::Leased { position, .. }) => {
                self.violations += 1;
                Some(*position)
            }
            Some(OrderStatus::Queued { position }) => {
                let stale = *position;
                self.violations += 1;
                self.order.remove(&stale);
                Some(stale)
            }
            Some(OrderStatus::Idle | OrderStatus::Parked) | None => {
                self.violations += 1;
                None
            }
        }
    }

    /// `ReleaseRequeuesPocket`, the position half: `ensures: pocket.status = queued,
    /// pocket.position = queue.next_position`, so the pocket joins at the back.
    fn release_requeues_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.queue_at_back(key, partition);
    }

    /// `LeaseExpiryKeepsPocketAtFront`, the position half: `ensures: pocket.status =
    /// queued` with the position untouched, so the pocket stands ahead of everything
    /// enqueued since it was claimed.
    fn lease_expiry_keeps_pocket_at_front(
        &mut self,
        key: PocketKey,
        partition: Partition,
        position: u64,
    ) {
        self.queue_at(key, partition, position);
    }

    /// `ReleaseIdlesEmptyPocket` or `LeaseExpiryIdlesEmptyPocket`: `ensures:
    /// pocket.status = idle`. The two rules differ only in what ended the lease, which
    /// the pocket machine saw and this machine does not need; both leave the pocket out
    /// of the order until a publish revives it.
    fn idles_empty_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.pockets
            .insert(key, OrderView::new(partition, OrderStatus::Idle));
    }

    /// `LeaseExpiryParksPocket`: `ensures: pocket.status = parked`; out of the order
    /// until a publish revives it.
    fn lease_expiry_parks_pocket(&mut self, key: PocketKey, partition: Partition) {
        self.pockets
            .insert(key, OrderView::new(partition, OrderStatus::Parked));
    }

    // Helpers -------------------------------------------------------------------

    /// `queue.next_position`, advanced: positions are unique and assigned in order.
    const fn fresh_position(&mut self) -> u64 {
        let position = self.next_pocket_position;
        self.next_pocket_position += 1;
        position
    }

    /// Queues the pocket at a fresh position, the back of the order.
    fn queue_at_back(&mut self, key: PocketKey, partition: Partition) {
        let position = self.fresh_position();
        self.queue_at(key, partition, position);
    }

    /// Queues the pocket at this position.
    fn queue_at(&mut self, key: PocketKey, partition: Partition, position: u64) {
        self.pockets.insert(
            key.clone(),
            OrderView::new(partition, OrderStatus::Queued { position }),
        );
        self.order.insert(position, key);
    }

    /// An input the protocol never sends the order machine. Every such input carries a
    /// client's reply handle, so the client is answered.
    fn misrouted(&mut self) -> Vec<Output> {
        self.violations += 1;
        vec![Output::Reply(Response::Rejected(Reason::Protocol))]
    }
}

impl Machine for OrderMachine {
    fn id(&self) -> MachineId {
        MachineId::Order
    }

    /// The order machine reads no clock: none of its rules stamps a time, so `now` is
    /// unused here and only the pocket machines consume it.
    fn step(&mut self, input: Input, _now: Timestamp) -> Vec<Output> {
        match input {
            Input::PublisherJoins { publisher } => self.publisher_joins(publisher),
            Input::ConsumerJoins { consumer } => self.consumer_joins(consumer),
            Input::Pull { consumer, limit } => self.pull(consumer, limit),
            Input::RetryPull { consumer, limit } => self.retry_pull(consumer, limit),
            Input::Enqueue {
                pocket_key,
                partition,
            } => self.enqueue(pocket_key, partition),
            Input::PocketBecame {
                pocket_key,
                became,
                consumer,
                lease,
            } => self.pocket_became(pocket_key, became, &consumer, lease),
            Input::Tick => Vec::new(),
            Input::Publish { .. }
            | Input::Ack { .. }
            | Input::Nack { .. }
            | Input::Renew { .. }
            | Input::Release { .. }
            | Input::Claim { .. }
            | Input::PullUnderLease { .. } => self.misrouted(),
        }
    }

    fn violations(&self) -> u64 {
        self.violations
    }
}

#[cfg(test)]
mod tests {
    use super::OrderMachine;
    use crate::machine::Machine;
    use crate::protocol::{
        Became, ConsumerId, Input, LeaseId, MachineId, Output, Partition, PocketKey, PublisherId,
        Reason, RecordKey, Response,
    };
    use crate::state::{OrderStatus, OrderView};
    use crate::time::{Config, Timestamp};

    const NOW: Timestamp = Timestamp::ZERO;

    fn machine() -> OrderMachine {
        OrderMachine::new(Config::default())
    }

    fn key(name: &str) -> PocketKey {
        PocketKey::new(name)
    }

    fn consumer(name: &str) -> ConsumerId {
        ConsumerId::new(name)
    }

    fn enqueue(machine: &mut OrderMachine, name: &str, partition: u32) -> Vec<Output> {
        machine.step(
            Input::Enqueue {
                pocket_key: key(name),
                partition: Partition::new(partition),
            },
            NOW,
        )
    }

    fn pull(machine: &mut OrderMachine, who: &str, limit: u64) -> Vec<Output> {
        machine.step(
            Input::Pull {
                consumer: consumer(who),
                limit,
            },
            NOW,
        )
    }

    fn became(machine: &mut OrderMachine, name: &str, what: Became, who: &str, lease: LeaseId) {
        let outputs = machine.step(
            Input::PocketBecame {
                pocket_key: key(name),
                became: what,
                consumer: consumer(who),
                lease,
            },
            NOW,
        );
        assert!(
            outputs.is_empty(),
            "PocketBecame never replies: {outputs:?}"
        );
    }

    /// The lease a `Claim` forward carries.
    fn claimed_lease(outputs: &[Output]) -> LeaseId {
        match outputs {
            [Output::Forward(_, Input::Claim { lease, .. })] => *lease,
            other => panic!("expected one Claim forward, got {other:?}"),
        }
    }

    /// Positions and keys of the pocket order, front first.
    fn order(machine: &OrderMachine) -> Vec<(u64, PocketKey)> {
        machine.queued().map(|(p, k)| (p, k.clone())).collect()
    }

    /// Two pockets queued and the first claimed by `c1`: the fixture most rules start
    /// from. Returns the lease minted for pocket `a`.
    fn claimed_front(machine: &mut OrderMachine) -> LeaseId {
        enqueue(machine, "a", 1);
        enqueue(machine, "b", 0);
        claimed_lease(&pull(machine, "c1", 10))
    }

    // Identity ------------------------------------------------------------------

    #[test]
    fn publisher_joins_creates_the_publisher() {
        let mut m = machine();
        let outputs = m.step(
            Input::PublisherJoins {
                publisher: PublisherId::new("p"),
            },
            NOW,
        );
        assert_eq!(outputs, vec![Output::Reply(Response::Joined)]);
        assert_eq!(
            m.publishers().collect::<Vec<_>>(),
            vec![&PublisherId::new("p")]
        );
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn publisher_joins_refuses_a_duplicate() {
        let mut m = machine();
        let join = Input::PublisherJoins {
            publisher: PublisherId::new("p"),
        };
        m.step(join.clone(), NOW);
        let outputs = m.step(join, NOW);
        assert_eq!(
            outputs,
            vec![Output::Reply(Response::Rejected(
                Reason::DuplicatePublisher
            ))]
        );
        assert_eq!(m.publishers().count(), 1);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn consumer_joins_creates_the_consumer() {
        let mut m = machine();
        let outputs = m.step(
            Input::ConsumerJoins {
                consumer: consumer("c"),
            },
            NOW,
        );
        assert_eq!(outputs, vec![Output::Reply(Response::Joined)]);
        assert_eq!(m.consumers().collect::<Vec<_>>(), vec![&consumer("c")]);
    }

    #[test]
    fn consumer_joins_refuses_a_duplicate() {
        let mut m = machine();
        let join = Input::ConsumerJoins {
            consumer: consumer("c"),
        };
        m.step(join.clone(), NOW);
        let outputs = m.step(join, NOW);
        assert_eq!(
            outputs,
            vec![Output::Reply(Response::Rejected(Reason::DuplicateConsumer))]
        );
        assert_eq!(m.consumers().count(), 1);
    }

    // Pull ----------------------------------------------------------------------

    #[test]
    fn pull_claims_front_pocket_pops_the_smallest_position_across_partitions() {
        let mut m = machine();
        enqueue(&mut m, "a", 1);
        enqueue(&mut m, "b", 0);
        let outputs = pull(&mut m, "c1", 7);
        let lease = LeaseId::new(Partition::new(1), 0);
        assert_eq!(
            outputs,
            vec![Output::Forward(
                MachineId::Partition(Partition::new(1)),
                Input::Claim {
                    pocket_key: key("a"),
                    consumer: consumer("c1"),
                    lease,
                    limit: 7,
                },
            )]
        );
        assert_eq!(
            m.view(&key("a")),
            Some(&OrderView::new(
                Partition::new(1),
                OrderStatus::Leased {
                    position: 0,
                    consumer: consumer("c1"),
                    lease,
                },
            ))
        );
        assert_eq!(order(&m), vec![(1, key("b"))]);
        assert_eq!(m.active_lease(&consumer("c1")), Some(lease));
        assert_eq!(m.next_lease_serial(), 1);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn pull_claims_front_pocket_mints_strictly_increasing_serials() {
        let mut m = machine();
        enqueue(&mut m, "a", 0);
        enqueue(&mut m, "b", 3);
        let first = claimed_lease(&pull(&mut m, "c1", 1));
        let second = claimed_lease(&pull(&mut m, "c2", 1));
        assert_eq!(first, LeaseId::new(Partition::new(0), 0));
        assert_eq!(second, LeaseId::new(Partition::new(3), 1));
        assert!(first.serial < second.serial);
        assert_eq!(m.next_lease_serial(), 2);
        assert_eq!(
            m.active_leases()
                .map(|(c, l)| (c.clone(), l))
                .collect::<Vec<_>>(),
            vec![(consumer("c1"), first), (consumer("c2"), second)]
        );
        assert_eq!(order(&m), Vec::new());
    }

    #[test]
    fn pull_within_lease_forwards_under_the_active_lease_and_replies_nothing() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        let outputs = pull(&mut m, "c1", 4);
        assert_eq!(
            outputs,
            vec![Output::Forward(
                MachineId::Partition(lease.partition),
                Input::PullUnderLease {
                    consumer: consumer("c1"),
                    lease,
                    limit: 4,
                },
            )]
        );
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn one_active_lease_per_consumer_so_a_second_pull_claims_no_second_pocket() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        pull(&mut m, "c1", 4);
        assert_eq!(order(&m), vec![(1, key("b"))], "b stays queued");
        assert_eq!(m.active_lease(&consumer("c1")), Some(lease));
        assert_eq!(m.next_lease_serial(), 1, "no second lease minted");
        assert_eq!(
            m.view(&key("b")).map(|v| &v.status),
            Some(&OrderStatus::Queued { position: 1 })
        );
    }

    #[test]
    fn pull_finds_no_pocket_replies_no_pocket_available() {
        let mut m = machine();
        let outputs = pull(&mut m, "c1", 3);
        assert_eq!(outputs, vec![Output::Reply(Response::NoPocketAvailable)]);
        assert_eq!(m.active_lease(&consumer("c1")), None);
        assert_eq!(m.next_lease_serial(), 0);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn pull_finds_no_pocket_refused_once_the_consumer_holds_a_lease() {
        // The same empty order, but the consumer has an active lease: the guard
        // `not exists Lease{consumer, active}` fails and the pull routes instead.
        let mut m = machine();
        enqueue(&mut m, "a", 2);
        let lease = claimed_lease(&pull(&mut m, "c1", 1));
        assert_eq!(order(&m), Vec::new());
        let outputs = pull(&mut m, "c1", 1);
        assert!(matches!(
            outputs.as_slice(),
            [Output::Forward(MachineId::Partition(p), Input::PullUnderLease { .. })]
                if *p == lease.partition
        ));
    }

    #[test]
    fn retry_pull_counts_and_behaves_as_pull() {
        let mut m = machine();
        enqueue(&mut m, "a", 0);
        let outputs = m.step(
            Input::RetryPull {
                consumer: consumer("c1"),
                limit: 2,
            },
            NOW,
        );
        assert_eq!(m.retry_pulls(), 1);
        assert_eq!(
            outputs,
            vec![Output::Forward(
                MachineId::Partition(Partition::new(0)),
                Input::Claim {
                    pocket_key: key("a"),
                    consumer: consumer("c1"),
                    lease: LeaseId::new(Partition::new(0), 0),
                    limit: 2,
                },
            )]
        );
        let outputs = m.step(
            Input::RetryPull {
                consumer: consumer("c2"),
                limit: 2,
            },
            NOW,
        );
        assert_eq!(m.retry_pulls(), 2);
        assert_eq!(outputs, vec![Output::Reply(Response::NoPocketAvailable)]);
        assert_eq!(m.violations(), 0);
    }

    // Enqueue -------------------------------------------------------------------

    #[test]
    fn publish_into_new_pocket_assigns_strictly_increasing_positions() {
        let mut m = machine();
        assert_eq!(enqueue(&mut m, "c", 0), Vec::new());
        assert_eq!(enqueue(&mut m, "a", 1), Vec::new());
        assert_eq!(enqueue(&mut m, "b", 0), Vec::new());
        assert_eq!(
            order(&m),
            vec![(0, key("c")), (1, key("a")), (2, key("b"))],
            "the order is by position, not by key"
        );
        assert_eq!(m.next_pocket_position(), 3);
        assert_eq!(
            m.view(&key("a")),
            Some(&OrderView::new(
                Partition::new(1),
                OrderStatus::Queued { position: 1 }
            ))
        );
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn revive_idle_pocket_takes_a_fresh_position_behind_everything_queued() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::Idle, "c1", lease);
        enqueue(&mut m, "c", 0);
        assert_eq!(enqueue(&mut m, "a", 1), Vec::new());
        assert_eq!(order(&m), vec![(1, key("b")), (2, key("c")), (3, key("a"))]);
        assert_eq!(
            m.view(&key("a")).map(|v| &v.status),
            Some(&OrderStatus::Queued { position: 3 })
        );
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn revive_parked_pocket_takes_a_fresh_position_behind_everything_queued() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::Parked, "c1", lease);
        enqueue(&mut m, "c", 0);
        assert_eq!(enqueue(&mut m, "a", 1), Vec::new());
        assert_eq!(order(&m), vec![(1, key("b")), (2, key("c")), (3, key("a"))]);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn enqueue_of_a_queued_pocket_counts_a_violation_and_requeues_it_once() {
        let mut m = machine();
        enqueue(&mut m, "a", 0);
        enqueue(&mut m, "b", 0);
        assert_eq!(enqueue(&mut m, "a", 0), Vec::new());
        assert_eq!(m.violations(), 1);
        assert_eq!(
            order(&m),
            vec![(1, key("b")), (2, key("a"))],
            "the stale entry at position 0 is gone"
        );
    }

    #[test]
    fn enqueue_of_a_leased_pocket_counts_a_violation_and_requeues_it() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        assert_eq!(enqueue(&mut m, "a", 1), Vec::new());
        assert_eq!(m.violations(), 1);
        assert_eq!(order(&m), vec![(1, key("b")), (2, key("a"))]);
        assert_eq!(
            m.active_lease(&consumer("c1")),
            Some(lease),
            "only PocketBecame ends a lease here"
        );
    }

    // PocketBecame --------------------------------------------------------------

    #[test]
    fn release_requeues_pocket_at_a_fresh_position_behind_everything_queued() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        enqueue(&mut m, "c", 0);
        became(&mut m, "a", Became::RequeuedAtBack, "c1", lease);
        assert_eq!(order(&m), vec![(1, key("b")), (2, key("c")), (3, key("a"))]);
        assert_eq!(
            m.view(&key("a")),
            Some(&OrderView::new(
                Partition::new(1),
                OrderStatus::Queued { position: 3 }
            ))
        );
        assert_eq!(m.active_lease(&consumer("c1")), None);
        assert_eq!(m.next_pocket_position(), 4);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn lease_expiry_keeps_pocket_at_front_ahead_of_pockets_enqueued_since() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        enqueue(&mut m, "c", 0);
        became(&mut m, "a", Became::RequeuedKeepingPosition, "c1", lease);
        assert_eq!(order(&m), vec![(0, key("a")), (1, key("b")), (2, key("c"))]);
        assert_eq!(
            m.view(&key("a")).map(|v| &v.status),
            Some(&OrderStatus::Queued { position: 0 })
        );
        assert_eq!(m.active_lease(&consumer("c1")), None);
        assert_eq!(m.next_pocket_position(), 3, "no position spent");
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn idles_empty_pocket_leaves_the_order() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::Idle, "c1", lease);
        assert_eq!(order(&m), vec![(1, key("b"))]);
        assert_eq!(
            m.view(&key("a")),
            Some(&OrderView::new(Partition::new(1), OrderStatus::Idle))
        );
        assert_eq!(m.active_lease(&consumer("c1")), None);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn lease_expiry_parks_pocket_out_of_the_order() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::Parked, "c1", lease);
        assert_eq!(order(&m), vec![(1, key("b"))]);
        assert_eq!(
            m.view(&key("a")),
            Some(&OrderView::new(Partition::new(1), OrderStatus::Parked))
        );
        assert_eq!(m.active_lease(&consumer("c1")), None);
        assert_eq!(m.violations(), 0);
    }

    #[test]
    fn pocket_became_frees_the_consumer_to_claim_the_next_pocket() {
        // The retry story: the PocketBecame arrives before the RetryPull, so the
        // consumer's next pull claims afresh with the next serial.
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::Idle, "c1", lease);
        let next = claimed_lease(&m.step(
            Input::RetryPull {
                consumer: consumer("c1"),
                limit: 1,
            },
            NOW,
        ));
        assert_eq!(next, LeaseId::new(Partition::new(0), 1));
        assert_eq!(m.active_lease(&consumer("c1")), Some(next));
        assert_eq!(order(&m), Vec::new());
        assert_eq!(m.retry_pulls(), 1);
    }

    #[test]
    fn pocket_became_for_another_consumer_counts_a_violation_and_keeps_the_lease() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        became(&mut m, "a", Became::RequeuedAtBack, "c2", lease);
        assert_eq!(m.violations(), 1);
        assert_eq!(
            m.active_lease(&consumer("c1")),
            Some(lease),
            "c1's lease is not the one named for c2"
        );
        assert_eq!(
            order(&m),
            vec![(1, key("b")), (2, key("a"))],
            "requeued anyway"
        );
    }

    #[test]
    fn pocket_became_under_another_lease_counts_a_violation_and_keeps_the_lease() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        let other = LeaseId::new(lease.partition, lease.serial + 40);
        became(&mut m, "a", Became::RequeuedKeepingPosition, "c1", other);
        assert_eq!(m.violations(), 1);
        assert_eq!(m.active_lease(&consumer("c1")), Some(lease));
        assert_eq!(
            order(&m),
            vec![(0, key("a")), (1, key("b"))],
            "the kept position is the view's"
        );
    }

    #[test]
    fn pocket_became_for_a_queued_view_counts_a_violation_without_duplicating_it() {
        let mut m = machine();
        enqueue(&mut m, "a", 0);
        let lease = LeaseId::new(Partition::new(0), 0);
        became(&mut m, "a", Became::RequeuedAtBack, "c1", lease);
        assert_eq!(m.violations(), 1);
        assert_eq!(order(&m), vec![(1, key("a"))], "one entry, at the back");
    }

    #[test]
    fn pocket_became_for_an_unknown_pocket_counts_a_violation_and_uses_the_lease_partition() {
        let mut m = machine();
        let lease = LeaseId::new(Partition::new(5), 9);
        became(
            &mut m,
            "ghost",
            Became::RequeuedKeepingPosition,
            "c1",
            lease,
        );
        assert_eq!(m.violations(), 1);
        assert_eq!(
            m.view(&key("ghost")),
            Some(&OrderView::new(
                Partition::new(5),
                OrderStatus::Queued { position: 0 }
            )),
            "no position to keep, so a fresh one"
        );
        became(&mut m, "ghost2", Became::Idle, "c1", lease);
        assert_eq!(m.violations(), 2);
        assert_eq!(
            m.view(&key("ghost2")),
            Some(&OrderView::new(Partition::new(5), OrderStatus::Idle))
        );
        assert_eq!(m.next_pocket_position(), 1, "idle spends no position");
    }

    // Tick and misrouted inputs -------------------------------------------------

    #[test]
    fn tick_does_nothing() {
        let mut m = machine();
        let lease = claimed_front(&mut m);
        let before = m.clone();
        assert_eq!(m.step(Input::Tick, NOW), Vec::new());
        assert_eq!(m.step(Input::Tick, Timestamp::from_millis(5)), Vec::new());
        assert_eq!(m, before);
        assert_eq!(m.active_lease(&consumer("c1")), Some(lease));
    }

    #[test]
    fn a_misrouted_ack_counts_a_violation_and_is_rejected_as_protocol() {
        let mut m = machine();
        let outputs = m.step(
            Input::Ack {
                consumer: consumer("c1"),
                lease: LeaseId::new(Partition::new(0), 0),
                record_key: RecordKey::new("r"),
            },
            NOW,
        );
        assert_eq!(
            outputs,
            vec![Output::Reply(Response::Rejected(Reason::Protocol))]
        );
        assert_eq!(m.violations(), 1);
    }

    #[test]
    fn every_pocket_machine_input_is_misrouted_here() {
        let mut m = machine();
        let c = consumer("c1");
        let lease = LeaseId::new(Partition::new(0), 0);
        let inputs = [
            Input::Publish {
                publisher: PublisherId::new("p"),
                pocket_key: key("a"),
                record_key: RecordKey::new("r"),
                rank: crate::protocol::Rank::new(1),
                payload: crate::protocol::Payload::new("x"),
            },
            Input::Nack {
                consumer: c.clone(),
                lease,
                record_key: RecordKey::new("r"),
            },
            Input::Renew {
                consumer: c.clone(),
                lease,
            },
            Input::Release {
                consumer: c.clone(),
                lease,
            },
            Input::Claim {
                pocket_key: key("a"),
                consumer: c.clone(),
                lease,
                limit: 1,
            },
            Input::PullUnderLease {
                consumer: c,
                lease,
                limit: 1,
            },
        ];
        let expected = u64::try_from(inputs.len()).unwrap();
        for input in inputs {
            assert_eq!(
                m.step(input, NOW),
                vec![Output::Reply(Response::Rejected(Reason::Protocol))]
            );
        }
        assert_eq!(m.violations(), expected);
        assert_eq!(m.id(), MachineId::Order);
    }
}
