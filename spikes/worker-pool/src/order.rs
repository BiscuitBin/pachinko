//! The order machine: the pocket order, the counter that positions pockets, the
//! counter that mints lease identities, and the consumer to active lease routing that
//! holds `OneActiveLeasePerConsumer`. Decomposition B of ticket T20.

use std::collections::{BTreeMap, BTreeSet};

use crate::machine::Machine;
use crate::protocol::{ConsumerId, Input, LeaseId, MachineId, Output, PocketKey, PublisherId};
use crate::state::OrderView;
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
}

impl Machine for OrderMachine {
    fn id(&self) -> MachineId {
        MachineId::Order
    }

    fn step(&mut self, input: Input, now: Timestamp) -> Vec<Output> {
        // Agent (b) fills this in: one function per rule branch.
        let _ = (input, now);
        Vec::new()
    }

    fn violations(&self) -> u64 {
        self.violations
    }
}
