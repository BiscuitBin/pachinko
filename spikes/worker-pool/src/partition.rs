//! The partition function: a pocket key to its partition, and a client input to the
//! machine that handles it.

use crate::protocol::{Input, MachineId, Partition, PocketKey};

/// 64-bit FNV-1a over the bytes: deterministic, dependency-free and stable across
/// runs, which `std::hash::RandomState` is not.
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The partition a pocket key belongs to, out of `partitions`, which is at least 1.
///
/// ```
/// use worker_pool::{Partition, PocketKey, partition_of};
/// let p = partition_of(&PocketKey::new("tenant-7"), 16);
/// assert!(p.index() < 16);
/// assert_eq!(p, partition_of(&PocketKey::new("tenant-7"), 16));
/// assert_eq!(partition_of(&PocketKey::new("anything"), 1), Partition::new(0));
/// ```
#[must_use]
pub fn partition_of(key: &PocketKey, partitions: u32) -> Partition {
    let count = u64::from(partitions.max(1));
    let index = fnv1a(key.as_str().as_bytes()) % count;
    Partition::new(u32::try_from(index).unwrap_or(0))
}

/// The machine a client input goes to, with no routing table: a publish by its pocket
/// key, a pull or a join to the order machine, an action naming a lease to the lease's
/// partition, folded into the pool's range so that a lease identity the pool never
/// granted reaches a real pocket machine and is answered `UnknownLease`. `None` for
/// the inputs the machines send each other and for the tick, which the pool injects
/// everywhere.
///
/// ```
/// use worker_pool::{ConsumerId, Input, MachineId, route};
/// let pull = Input::Pull { consumer: ConsumerId::new("c"), limit: 1 };
/// assert_eq!(route(&pull, 4), Some(MachineId::Order));
/// assert_eq!(route(&Input::Tick, 4), None);
/// ```
#[must_use]
pub fn route(input: &Input, partitions: u32) -> Option<MachineId> {
    match input {
        Input::PublisherJoins { .. } | Input::ConsumerJoins { .. } | Input::Pull { .. } => {
            Some(MachineId::Order)
        }
        Input::Publish { pocket_key, .. } => {
            Some(MachineId::Partition(partition_of(pocket_key, partitions)))
        }
        Input::Ack { lease, .. }
        | Input::Nack { lease, .. }
        | Input::Renew { lease, .. }
        | Input::Release { lease, .. } => Some(MachineId::Partition(Partition::new(
            lease.partition.index() % partitions.max(1),
        ))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{fnv1a, partition_of, route};
    use crate::protocol::{
        ConsumerId, Input, LeaseId, MachineId, Partition, Payload, PocketKey, PublisherId, Rank,
        RecordKey,
    };

    #[test]
    fn fnv1a_matches_the_reference_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn every_partition_gets_some_key() {
        let hit: std::collections::BTreeSet<u32> = (0..1000)
            .map(|i| partition_of(&PocketKey::new(format!("k{i}")), 8).index())
            .collect();
        assert_eq!(hit.len(), 8);
        assert_eq!(partition_of(&PocketKey::new("x"), 0), Partition::new(0));
    }

    #[test]
    fn client_inputs_route_and_internal_ones_do_not() {
        let lease = LeaseId::new(Partition::new(3), 1);
        let consumer = ConsumerId::new("c");
        let to_lease = MachineId::Partition(Partition::new(3));
        assert_eq!(
            route(
                &Input::PublisherJoins {
                    publisher: PublisherId::new("p")
                },
                4
            ),
            Some(MachineId::Order)
        );
        assert_eq!(
            route(
                &Input::ConsumerJoins {
                    consumer: consumer.clone()
                },
                4
            ),
            Some(MachineId::Order)
        );
        let publish = Input::Publish {
            publisher: PublisherId::new("p"),
            pocket_key: PocketKey::new("k"),
            record_key: RecordKey::new("r"),
            rank: Rank::new(1),
            payload: Payload::new("x"),
        };
        assert_eq!(
            route(&publish, 4),
            Some(MachineId::Partition(partition_of(&PocketKey::new("k"), 4)))
        );
        for input in [
            Input::Ack {
                consumer: consumer.clone(),
                lease,
                record_key: RecordKey::new("r"),
            },
            Input::Nack {
                consumer: consumer.clone(),
                lease,
                record_key: RecordKey::new("r"),
            },
            Input::Renew {
                consumer: consumer.clone(),
                lease,
            },
            Input::Release {
                consumer: consumer.clone(),
                lease,
            },
        ] {
            assert_eq!(route(&input, 4), Some(to_lease));
        }
        assert_eq!(route(&Input::RetryPull { consumer, limit: 1 }, 4), None);
        assert_eq!(route(&Input::Tick, 4), None);
    }

    #[test]
    fn a_lease_outside_the_pools_range_is_folded_into_it() {
        let consumer = ConsumerId::new("c");
        let renew = Input::Renew {
            consumer: consumer.clone(),
            lease: LeaseId::new(Partition::new(99), 0),
        };
        assert_eq!(
            route(&renew, 4),
            Some(MachineId::Partition(Partition::new(3)))
        );
        let release = Input::Release {
            consumer,
            lease: LeaseId::new(Partition::new(7), 0),
        };
        assert_eq!(
            route(&release, 0),
            Some(MachineId::Partition(Partition::new(0)))
        );
    }
}
