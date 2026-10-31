//! The pocket machine: one partition's pockets, their records, the leases on them,
//! and the lapse, with every publish, pull, ack, nack, renew, release, expiry and
//! decay rule as a branch. Decomposition B of ticket T20.

use std::collections::BTreeMap;

use crate::machine::Machine;
use crate::protocol::{Input, LeaseId, MachineId, Output, Partition, PocketKey};
use crate::state::{Lease, Pocket};
use crate::time::{Config, Timestamp};

/// The pocket machine for one partition, `MachineId::Partition(partition)`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PocketMachine {
    partition: Partition,
    /// How many partitions the pool has; a record position is
    /// `serial * partitions + partition.index()`, so no two records anywhere share one.
    partitions: u32,
    config: Config,
    pockets: BTreeMap<PocketKey, Pocket>,
    /// Which pocket each lease ever granted here is on, active or ended.
    leases: BTreeMap<LeaseId, PocketKey>,
    next_record_serial: u64,
    violations: u64,
}

impl PocketMachine {
    /// A pocket machine for `partition` out of `partitions`, with nothing in it.
    ///
    /// ```
    /// use worker_pool::{Config, Partition, PocketMachine};
    /// let machine = PocketMachine::new(Partition::new(2), 4, Config::default());
    /// assert_eq!(machine.partition(), Partition::new(2));
    /// assert_eq!(machine.pockets().count(), 0);
    /// ```
    #[must_use]
    pub const fn new(partition: Partition, partitions: u32, config: Config) -> Self {
        Self {
            partition,
            partitions,
            config,
            pockets: BTreeMap::new(),
            leases: BTreeMap::new(),
            next_record_serial: 0,
            violations: 0,
        }
    }

    /// The partition this machine owns.
    #[must_use]
    pub const fn partition(&self) -> Partition {
        self.partition
    }

    /// The configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// One pocket.
    #[must_use]
    pub fn pocket(&self, key: &PocketKey) -> Option<&Pocket> {
        self.pockets.get(key)
    }

    /// Every pocket, by key.
    pub fn pockets(&self) -> impl Iterator<Item = (&PocketKey, &Pocket)> {
        self.pockets.iter()
    }

    /// The lease with this identity, active or ended, and its pocket.
    #[must_use]
    pub fn lease(&self, id: LeaseId) -> Option<(&PocketKey, &Lease)> {
        let key = self.leases.get(&id)?;
        let lease = self.pockets.get(key)?.lease(id)?;
        Some((key, lease))
    }

    /// The serial the next record position is built from.
    #[must_use]
    pub const fn next_record_serial(&self) -> u64 {
        self.next_record_serial
    }

    /// The position the next record takes: the serial tagged with the partition.
    #[must_use]
    pub const fn next_record_position(&self) -> u64 {
        self.next_record_serial * (self.partitions as u64) + (self.partition.index() as u64)
    }
}

impl Machine for PocketMachine {
    fn id(&self) -> MachineId {
        MachineId::Partition(self.partition)
    }

    fn step(&mut self, input: Input, now: Timestamp) -> Vec<Output> {
        // Agent (a) fills this in: one function per rule branch.
        let _ = (input, now);
        Vec::new()
    }

    fn violations(&self) -> u64 {
        self.violations
    }
}
