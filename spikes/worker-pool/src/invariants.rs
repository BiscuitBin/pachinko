//! The module's invariants, checked by name over the aggregate state of the order
//! machine and every pocket machine, as far as that state shows them.
//!
//! One function per invariant, named after it; [`check`] runs them all and returns
//! every failure. Two checks of the spike's own, prefixed `spike:`, assert what the
//! protocol promises at quiescence: the order machine's view of every pocket agrees
//! with the pocket machine that owns it, and no machine saw a protocol violation.

use std::collections::{BTreeMap, BTreeSet};

use crate::machine::Machine;
use crate::order::OrderMachine;
use crate::pocket::PocketMachine;
use crate::protocol::{ConsumerId, LeaseId, PocketKey, RecordKey};
use crate::state::{LeaseStatus, OrderStatus, Pocket, PocketStatus, RecordStatus};
use crate::time::Config;

/// One invariant that does not hold, by its name in the module.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Violation {
    /// The invariant's name in `docs/specs/pachinko.allium`, or `spike:` and the name
    /// of one of the spike's own consistency checks.
    pub invariant: &'static str,
    /// What was found.
    pub detail: String,
}

impl Violation {
    const fn new(invariant: &'static str, detail: String) -> Self {
        Self { invariant, detail }
    }
}

/// Every invariant that does not hold over the order machine and the pocket machines,
/// evaluated at quiescence: when no message is in flight between them.
///
/// ```
/// use worker_pool::{Config, Direct, invariants};
/// let direct = Direct::new(Config::default(), 2);
/// assert_eq!(invariants::check(direct.order(), direct.pockets()), Vec::new());
/// ```
#[must_use]
pub fn check(order: &OrderMachine, pockets: &[PocketMachine]) -> Vec<Violation> {
    let mut found = Vec::new();
    for machine in pockets {
        for (key, pocket) in machine.pockets() {
            found.extend(check_pocket(machine, key, pocket));
        }
        found.extend(expiry_limit_positive(machine.config()));
        found.extend(pull_batch_positive(machine.config()));
    }
    found.extend(expiry_limit_positive(order.config()));
    found.extend(pull_batch_positive(order.config()));
    found.extend(one_pocket_per_key(pockets));
    found.extend(one_active_lease_per_consumer(pockets));
    found.extend(unique_pocket_position(order));
    found.extend(unique_record_position(pockets));
    found.extend(pocket_positions_below_next(order));
    found.extend(order_view_consistent(order, pockets));
    found.extend(no_protocol_violations(order, pockets));
    found
}

/// The per-pocket invariants, over one pocket of one pocket machine.
fn check_pocket(machine: &PocketMachine, key: &PocketKey, pocket: &Pocket) -> Vec<Violation> {
    let mut found = Vec::new();
    found.extend(leased_has_one_active_lease(pocket));
    found.extend(unleased_has_no_active_lease(pocket));
    found.extend(idle_has_no_records(pocket));
    found.extend(expiries_within_limit(machine.config(), pocket));
    found.extend(held_version_not_newer(pocket));
    found.extend(shed_before_expiry(pocket));
    found.extend(nacked_before_expiry(pocket));
    found.extend(granted_before_expiry(pocket));
    found.extend(ended_after_grant(pocket));
    found.extend(one_record_per_key_per_pocket(key, pocket));
    found.extend(one_active_lease_per_pocket(pocket));
    found.extend(outstanding_only_while_leased(pocket));
    found.extend(record_positions_below_next(machine, pocket));
    found
}

fn active_count(pocket: &Pocket) -> usize {
    pocket
        .leases
        .iter()
        .filter(|lease| lease.status == LeaseStatus::Active)
        .count()
}

fn leased_has_one_active_lease(pocket: &Pocket) -> Option<Violation> {
    let active = active_count(pocket);
    (pocket.status == PocketStatus::Leased && active != 1).then(|| {
        Violation::new(
            "LeasedHasOneActiveLease",
            format!(
                "pocket {:?} is leased with {active} active leases",
                pocket.key
            ),
        )
    })
}

fn unleased_has_no_active_lease(pocket: &Pocket) -> Option<Violation> {
    let active = active_count(pocket);
    (pocket.status != PocketStatus::Leased && active != 0).then(|| {
        Violation::new(
            "UnleasedHasNoActiveLease",
            format!(
                "pocket {:?} is {:?} with {active} active leases",
                pocket.key, pocket.status
            ),
        )
    })
}

fn idle_has_no_records(pocket: &Pocket) -> Option<Violation> {
    (pocket.status == PocketStatus::Idle && !pocket.records.is_empty()).then(|| {
        Violation::new(
            "IdleHasNoRecords",
            format!(
                "pocket {:?} is idle with {} records",
                pocket.key,
                pocket.records.len()
            ),
        )
    })
}

fn expiries_within_limit(config: &Config, pocket: &Pocket) -> Option<Violation> {
    (pocket.consecutive_expiries > config.max_consecutive_expiries).then(|| {
        Violation::new(
            "ExpiriesWithinLimit",
            format!(
                "pocket {:?} has {} consecutive expiries, limit {}",
                pocket.key, pocket.consecutive_expiries, config.max_consecutive_expiries
            ),
        )
    })
}

fn held_version_not_newer(pocket: &Pocket) -> Vec<Violation> {
    pocket
        .records
        .values()
        .filter(|record| record.status == RecordStatus::Held)
        .filter(|record| record.held_rank.is_none_or(|held| held > record.rank))
        .map(|record| {
            Violation::new(
                "HeldVersionNotNewer",
                format!(
                    "record {:?} in pocket {:?} is held with held_rank {:?} over rank {:?}",
                    record.key, pocket.key, record.held_rank, record.rank
                ),
            )
        })
        .collect()
}

fn shed_before_expiry(pocket: &Pocket) -> Vec<Violation> {
    pocket
        .superseded
        .iter()
        .filter(|shed| shed.shed_at > shed.expires_at)
        .map(|shed| {
            Violation::new(
                "ShedBeforeExpiry",
                format!(
                    "superseded {:?} in pocket {:?} shed at {:?} after its expiry {:?}",
                    shed.key, pocket.key, shed.shed_at, shed.expires_at
                ),
            )
        })
        .collect()
}

fn nacked_before_expiry(pocket: &Pocket) -> Vec<Violation> {
    pocket
        .dead_letters
        .iter()
        .filter(|letter| letter.nacked_at > letter.expires_at)
        .map(|letter| {
            Violation::new(
                "NackedBeforeExpiry",
                format!(
                    "dead letter {:?} in pocket {:?} nacked at {:?} after its expiry {:?}",
                    letter.key, pocket.key, letter.nacked_at, letter.expires_at
                ),
            )
        })
        .collect()
}

fn granted_before_expiry(pocket: &Pocket) -> Vec<Violation> {
    pocket
        .leases
        .iter()
        .filter(|lease| lease.granted_at > lease.expires_at)
        .map(|lease| {
            Violation::new(
                "GrantedBeforeExpiry",
                format!(
                    "lease {:?} granted at {:?} after its expiry {:?}",
                    lease.id, lease.granted_at, lease.expires_at
                ),
            )
        })
        .collect()
}

fn ended_after_grant(pocket: &Pocket) -> Vec<Violation> {
    pocket
        .leases
        .iter()
        .filter(|lease| lease.status != LeaseStatus::Active)
        .filter(|lease| lease.ended_at.is_none_or(|ended| ended < lease.granted_at))
        .map(|lease| {
            Violation::new(
                "EndedAfterGrant",
                format!(
                    "lease {:?} is {:?} with ended_at {:?} against granted_at {:?}",
                    lease.id, lease.status, lease.ended_at, lease.granted_at
                ),
            )
        })
        .collect()
}

/// The map holds one record per key by construction; what remains to check is that
/// each record carries the key it is filed under.
fn one_record_per_key_per_pocket(key: &PocketKey, pocket: &Pocket) -> Vec<Violation> {
    let mut found = Vec::new();
    if pocket.key != *key {
        found.push(Violation::new(
            "OnePocketPerKey",
            format!("pocket {:?} is filed under key {key:?}", pocket.key),
        ));
    }
    found.extend(
        pocket
            .records
            .iter()
            .filter(|(filed, record)| record.key != **filed)
            .map(|(filed, record)| {
                Violation::new(
                    "OneRecordPerKeyPerPocket",
                    format!(
                        "record {:?} in pocket {key:?} is filed under key {filed:?}",
                        record.key
                    ),
                )
            }),
    );
    found
}

fn one_active_lease_per_pocket(pocket: &Pocket) -> Option<Violation> {
    let active = active_count(pocket);
    (active > 1).then(|| {
        Violation::new(
            "OneActiveLeasePerPocket",
            format!("pocket {:?} has {active} active leases", pocket.key),
        )
    })
}

fn outstanding_only_while_leased(pocket: &Pocket) -> Vec<Violation> {
    if pocket.status == PocketStatus::Leased {
        return Vec::new();
    }
    pocket
        .records
        .values()
        .filter(|record| record.status != RecordStatus::Queued)
        .map(|record| {
            Violation::new(
                "OutstandingOnlyWhileLeased",
                format!(
                    "record {:?} is {:?} in pocket {:?}, which is {:?}",
                    record.key, record.status, pocket.key, pocket.status
                ),
            )
        })
        .collect()
}

fn record_positions_below_next(machine: &PocketMachine, pocket: &Pocket) -> Vec<Violation> {
    let next = machine.next_record_position();
    pocket
        .records
        .values()
        .filter(|record| record.position >= next)
        .map(|record| {
            Violation::new(
                "RecordPositionsBelowNext",
                format!(
                    "record {:?} in pocket {:?} at position {} with next {next}",
                    record.key, pocket.key, record.position
                ),
            )
        })
        .collect()
}

fn expiry_limit_positive(config: &Config) -> Option<Violation> {
    (config.max_consecutive_expiries < 1).then(|| {
        Violation::new(
            "ExpiryLimitPositive",
            format!(
                "max_consecutive_expiries is {}",
                config.max_consecutive_expiries
            ),
        )
    })
}

fn pull_batch_positive(config: &Config) -> Option<Violation> {
    (config.max_pull_batch < 1).then(|| {
        Violation::new(
            "PullBatchPositive",
            format!("max_pull_batch is {}", config.max_pull_batch),
        )
    })
}

fn one_pocket_per_key(pockets: &[PocketMachine]) -> Vec<Violation> {
    let mut owners: BTreeMap<&PocketKey, Vec<u32>> = BTreeMap::new();
    for machine in pockets {
        for (key, _) in machine.pockets() {
            owners
                .entry(key)
                .or_default()
                .push(machine.partition().index());
        }
    }
    owners
        .into_iter()
        .filter(|(_, machines)| machines.len() > 1)
        .map(|(key, machines)| {
            Violation::new(
                "OnePocketPerKey",
                format!("pocket {key:?} is in partitions {machines:?}"),
            )
        })
        .collect()
}

/// Every active lease everywhere, by consumer.
fn active_by_consumer(pockets: &[PocketMachine]) -> BTreeMap<&ConsumerId, Vec<LeaseId>> {
    let mut held: BTreeMap<&ConsumerId, Vec<LeaseId>> = BTreeMap::new();
    for lease in pockets
        .iter()
        .flat_map(PocketMachine::pockets)
        .filter_map(|(_, pocket)| pocket.active_lease())
    {
        held.entry(&lease.consumer).or_default().push(lease.id);
    }
    held
}

fn one_active_lease_per_consumer(pockets: &[PocketMachine]) -> Vec<Violation> {
    active_by_consumer(pockets)
        .into_iter()
        .filter(|(_, leases)| leases.len() > 1)
        .map(|(consumer, leases)| {
            Violation::new(
                "OneActiveLeasePerConsumer",
                format!("consumer {consumer:?} holds active leases {leases:?}"),
            )
        })
        .collect()
}

/// The position of a view that has one: queued, or leased and keeping its place.
const fn view_position(status: &OrderStatus) -> Option<u64> {
    match status {
        OrderStatus::Queued { position } | OrderStatus::Leased { position, .. } => Some(*position),
        _ => None,
    }
}

fn unique_pocket_position(order: &OrderMachine) -> Vec<Violation> {
    let mut by_position: BTreeMap<u64, Vec<&PocketKey>> = BTreeMap::new();
    for (key, view) in order.views() {
        if let Some(position) = view_position(&view.status) {
            by_position.entry(position).or_default().push(key);
        }
    }
    by_position
        .into_iter()
        .filter(|(_, keys)| keys.len() > 1)
        .map(|(position, keys)| {
            Violation::new(
                "UniquePocketPosition",
                format!("pockets {keys:?} share position {position}"),
            )
        })
        .collect()
}

fn unique_record_position(pockets: &[PocketMachine]) -> Vec<Violation> {
    let mut seen: BTreeMap<u64, (&PocketKey, &RecordKey)> = BTreeMap::new();
    let mut found = Vec::new();
    for (key, pocket) in pockets.iter().flat_map(PocketMachine::pockets) {
        for record in pocket.records.values() {
            if let Some(other) = seen.insert(record.position, (key, &record.key)) {
                found.push(Violation::new(
                    "UniqueRecordPosition",
                    format!(
                        "record {:?} in pocket {key:?} shares position {} with {other:?}",
                        record.key, record.position
                    ),
                ));
            }
        }
    }
    found
}

fn pocket_positions_below_next(order: &OrderMachine) -> Vec<Violation> {
    let next = order.next_pocket_position();
    order
        .views()
        .filter_map(|(key, view)| view_position(&view.status).map(|position| (key, position)))
        .filter(|(_, position)| *position >= next)
        .map(|(key, position)| {
            Violation::new(
                "PocketPositionsBelowNext",
                format!("pocket {key:?} at position {position} with next {next}"),
            )
        })
        .collect()
}

/// `spike:OrderViewConsistent`: at quiescence the order machine's view of every pocket
/// has the pocket machine's status, a leased view names the pocket's active lease and
/// its consumer, and the order machine's consumer-to-lease table is exactly the set of
/// active leases.
fn order_view_consistent(order: &OrderMachine, pockets: &[PocketMachine]) -> Vec<Violation> {
    const NAME: &str = "spike:OrderViewConsistent";
    let mut found = Vec::new();
    let mut viewed: BTreeSet<&PocketKey> = BTreeSet::new();
    for (key, view) in order.views() {
        viewed.insert(key);
        let Some(pocket) = pockets
            .get(view.partition.index() as usize)
            .and_then(|machine| machine.pocket(key))
        else {
            found.push(Violation::new(
                NAME,
                format!("view of {key:?} in {:?} has no pocket", view.partition),
            ));
            continue;
        };
        found.extend(view_matches_pocket(key, &view.status, pocket));
    }
    for (key, _) in pockets.iter().flat_map(PocketMachine::pockets) {
        if !viewed.contains(key) {
            found.push(Violation::new(NAME, format!("pocket {key:?} has no view")));
        }
    }
    let active: BTreeMap<&ConsumerId, LeaseId> = active_by_consumer(pockets)
        .into_iter()
        .filter_map(|(consumer, leases)| leases.first().map(|lease| (consumer, *lease)))
        .collect();
    let routed: BTreeMap<&ConsumerId, LeaseId> = order.active_leases().collect();
    if active != routed {
        found.push(Violation::new(
            NAME,
            format!("order routes {routed:?} but the active leases are {active:?}"),
        ));
    }
    found
}

fn view_matches_pocket(
    key: &PocketKey,
    status: &OrderStatus,
    pocket: &Pocket,
) -> Option<Violation> {
    let detail = match (status, pocket.status) {
        (OrderStatus::Queued { .. }, PocketStatus::Queued)
        | (OrderStatus::Idle, PocketStatus::Idle)
        | (OrderStatus::Parked, PocketStatus::Parked) => return None,
        (
            OrderStatus::Leased {
                consumer, lease, ..
            },
            PocketStatus::Leased,
        ) => match pocket.active_lease() {
            Some(active) if active.id == *lease && active.consumer == *consumer => {
                return None;
            }
            Some(active) => format!(
                "view of {key:?} names lease {lease:?} of {consumer:?}, pocket's is {:?} of {:?}",
                active.id, active.consumer
            ),
            None => format!("view of {key:?} names lease {lease:?}, pocket has no active lease"),
        },
        (view, actual) => format!("view of {key:?} is {view:?}, pocket is {actual:?}"),
    };
    Some(Violation::new("spike:OrderViewConsistent", detail))
}

/// `spike:NoProtocolViolations`: no machine counted an input the protocol never sends it.
fn no_protocol_violations(order: &OrderMachine, pockets: &[PocketMachine]) -> Vec<Violation> {
    let machines: Vec<&dyn Machine> = std::iter::once::<&dyn Machine>(order)
        .chain(pockets.iter().map(|machine| -> &dyn Machine { machine }))
        .collect();
    machines
        .into_iter()
        .filter(|machine| machine.violations() > 0)
        .map(|machine| {
            Violation::new(
                "spike:NoProtocolViolations",
                format!(
                    "{:?} counted {} protocol violations",
                    machine.id(),
                    machine.violations()
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        check, check_pocket, ended_after_grant, expiry_limit_positive, granted_before_expiry,
        held_version_not_newer, idle_has_no_records, leased_has_one_active_lease,
        nacked_before_expiry, one_active_lease_per_pocket, one_record_per_key_per_pocket,
        outstanding_only_while_leased, pull_batch_positive, shed_before_expiry,
        unleased_has_no_active_lease, view_matches_pocket,
    };
    use crate::direct::Direct;
    use crate::pocket::PocketMachine;
    use crate::protocol::{ConsumerId, LeaseId, Partition, Payload, PocketKey, Rank, RecordKey};
    use crate::state::{
        DeadLetter, Lease, LeaseStatus, OrderStatus, Pocket, PocketStatus, Record, RecordStatus,
        ShedReason, SupersededRecord,
    };
    use crate::time::{Config, Timestamp};

    fn names(violations: &[super::Violation]) -> Vec<&'static str> {
        violations.iter().map(|v| v.invariant).collect()
    }

    fn lease(serial: u64, status: LeaseStatus, ended_at: Option<Timestamp>) -> Lease {
        Lease {
            id: LeaseId::new(Partition::new(0), serial),
            consumer: ConsumerId::new("c"),
            status,
            granted_at: Timestamp::from_millis(10),
            expires_at: Timestamp::from_millis(20),
            ended_at,
        }
    }

    fn record(key: &str, status: RecordStatus, rank: u64, held_rank: Option<u64>) -> Record {
        Record {
            key: RecordKey::new(key),
            rank: Rank::new(rank),
            payload: Payload::new("x"),
            position: 0,
            status,
            held_rank: held_rank.map(Rank::new),
            held_payload: held_rank.map(|_| Payload::new("x")),
        }
    }

    #[test]
    fn an_empty_system_has_no_violations() {
        let direct = Direct::new(Config::default(), 3);
        assert_eq!(check(direct.order(), direct.pockets()), Vec::new());
    }

    #[test]
    fn a_driven_system_has_no_violations_after_each_step() {
        use crate::protocol::{Input, Payload, PublisherId};
        let mut direct = Direct::new(Config::default(), 2);
        let publish = Input::Publish {
            publisher: PublisherId::new("p"),
            pocket_key: PocketKey::new("k"),
            record_key: RecordKey::new("r"),
            rank: Rank::new(1),
            payload: Payload::new("x"),
        };
        let pull = Input::Pull {
            consumer: ConsumerId::new("c"),
            limit: 5,
        };
        for input in [publish, pull.clone(), pull] {
            direct.submit(input, Timestamp::ZERO).unwrap();
            assert_eq!(check(direct.order(), direct.pockets()), Vec::new());
        }
        direct
            .tick(Timestamp::ZERO.plus(crate::time::Duration::from_secs(600)))
            .unwrap();
        assert_eq!(check(direct.order(), direct.pockets()), Vec::new());
    }

    #[test]
    fn a_fresh_pocket_passes_every_per_pocket_check() {
        let machine = PocketMachine::new(Partition::new(0), 1, Config::default());
        let key = PocketKey::new("k");
        assert_eq!(
            check_pocket(&machine, &key, &Pocket::new(key.clone())),
            Vec::new()
        );
    }

    #[test]
    fn a_leased_pocket_without_an_active_lease_breaks_leased_has_one_active_lease() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.status = PocketStatus::Leased;
        assert_eq!(
            leased_has_one_active_lease(&pocket).map(|v| v.invariant),
            Some("LeasedHasOneActiveLease")
        );
        assert_eq!(unleased_has_no_active_lease(&pocket), None);
    }

    #[test]
    fn a_queued_pocket_with_an_active_lease_breaks_unleased_has_no_active_lease() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.leases.push(lease(0, LeaseStatus::Active, None));
        assert_eq!(
            unleased_has_no_active_lease(&pocket).map(|v| v.invariant),
            Some("UnleasedHasNoActiveLease")
        );
        pocket.leases.push(lease(1, LeaseStatus::Active, None));
        assert_eq!(
            one_active_lease_per_pocket(&pocket).map(|v| v.invariant),
            Some("OneActiveLeasePerPocket")
        );
    }

    #[test]
    fn an_idle_pocket_with_a_record_breaks_idle_has_no_records() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.status = PocketStatus::Idle;
        pocket.records.insert(
            RecordKey::new("r"),
            record("r", RecordStatus::Queued, 1, None),
        );
        assert_eq!(
            idle_has_no_records(&pocket).map(|v| v.invariant),
            Some("IdleHasNoRecords")
        );
    }

    #[test]
    fn a_held_record_with_a_newer_held_rank_breaks_held_version_not_newer() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.status = PocketStatus::Leased;
        pocket.leases.push(lease(0, LeaseStatus::Active, None));
        pocket.records.insert(
            RecordKey::new("newer"),
            record("newer", RecordStatus::Held, 1, Some(2)),
        );
        pocket.records.insert(
            RecordKey::new("missing"),
            record("missing", RecordStatus::Held, 1, None),
        );
        pocket.records.insert(
            RecordKey::new("fine"),
            record("fine", RecordStatus::Held, 3, Some(2)),
        );
        let found = held_version_not_newer(&pocket);
        assert_eq!(names(&found), vec!["HeldVersionNotNewer"; 2]);
        assert!(found.iter().any(|v| v.detail.contains("\"newer\"")));
        assert!(found.iter().any(|v| v.detail.contains("\"missing\"")));
    }

    #[test]
    fn a_held_record_in_a_queued_pocket_breaks_outstanding_only_while_leased() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.records.insert(
            RecordKey::new("h"),
            record("h", RecordStatus::Held, 1, Some(1)),
        );
        pocket.records.insert(
            RecordKey::new("d"),
            record("d", RecordStatus::Deferred, 1, None),
        );
        assert_eq!(
            names(&outstanding_only_while_leased(&pocket)),
            vec!["OutstandingOnlyWhileLeased"; 2]
        );
    }

    #[test]
    fn a_record_filed_under_another_key_breaks_one_record_per_key_per_pocket() {
        let key = PocketKey::new("k");
        let mut pocket = Pocket::new(key.clone());
        pocket.records.insert(
            RecordKey::new("filed"),
            record("actual", RecordStatus::Queued, 1, None),
        );
        assert_eq!(
            names(&one_record_per_key_per_pocket(&key, &pocket)),
            vec!["OneRecordPerKeyPerPocket"]
        );
        assert_eq!(
            names(&one_record_per_key_per_pocket(
                &PocketKey::new("other"),
                &Pocket::new(key)
            )),
            vec!["OnePocketPerKey"]
        );
    }

    #[test]
    fn timestamps_out_of_order_break_the_three_before_expiry_invariants_and_ended_after_grant() {
        let mut pocket = Pocket::new(PocketKey::new("k"));
        pocket.superseded.push(SupersededRecord {
            key: RecordKey::new("s"),
            rank: Rank::new(1),
            payload: Payload::new("x"),
            reason: ShedReason::Stale,
            shed_at: Timestamp::from_millis(5),
            expires_at: Timestamp::from_millis(4),
        });
        pocket.dead_letters.push(DeadLetter {
            key: RecordKey::new("d"),
            rank: Rank::new(1),
            payload: Payload::new("x"),
            nacked_by: ConsumerId::new("c"),
            nacked_at: Timestamp::from_millis(5),
            expires_at: Timestamp::from_millis(4),
        });
        let mut early = lease(0, LeaseStatus::Released, Some(Timestamp::from_millis(1)));
        early.expires_at = Timestamp::from_millis(1);
        pocket.leases.push(early);
        pocket.leases.push(lease(1, LeaseStatus::Expired, None));
        pocket.leases.push(lease(
            2,
            LeaseStatus::Released,
            Some(Timestamp::from_millis(10)),
        ));
        assert_eq!(
            names(&shed_before_expiry(&pocket)),
            vec!["ShedBeforeExpiry"]
        );
        assert_eq!(
            names(&nacked_before_expiry(&pocket)),
            vec!["NackedBeforeExpiry"]
        );
        assert_eq!(
            names(&granted_before_expiry(&pocket)),
            vec!["GrantedBeforeExpiry"]
        );
        assert_eq!(
            names(&ended_after_grant(&pocket)),
            vec!["EndedAfterGrant"; 2]
        );
    }

    #[test]
    fn a_zero_limit_in_the_config_breaks_the_config_invariants() {
        let config = Config::default()
            .with_max_consecutive_expiries(0)
            .with_max_pull_batch(0);
        assert_eq!(
            expiry_limit_positive(&config).map(|v| v.invariant),
            Some("ExpiryLimitPositive")
        );
        assert_eq!(
            pull_batch_positive(&config).map(|v| v.invariant),
            Some("PullBatchPositive")
        );
        let direct = Direct::new(config, 1);
        let found = check(direct.order(), direct.pockets());
        assert_eq!(
            names(&found),
            vec![
                "ExpiryLimitPositive",
                "PullBatchPositive",
                "ExpiryLimitPositive",
                "PullBatchPositive"
            ]
        );
    }

    #[test]
    fn a_view_disagreeing_with_its_pocket_breaks_order_view_consistent() {
        let key = PocketKey::new("k");
        let mut pocket = Pocket::new(key.clone());
        assert_eq!(
            view_matches_pocket(&key, &OrderStatus::Queued { position: 0 }, &pocket),
            None
        );
        let leased = OrderStatus::Leased {
            position: 0,
            consumer: ConsumerId::new("c"),
            lease: LeaseId::new(Partition::new(0), 0),
        };
        assert_eq!(
            view_matches_pocket(&key, &leased, &pocket).map(|v| v.invariant),
            Some("spike:OrderViewConsistent")
        );
        pocket.status = PocketStatus::Leased;
        pocket.leases.push(lease(0, LeaseStatus::Active, None));
        assert_eq!(view_matches_pocket(&key, &leased, &pocket), None);
        let other = OrderStatus::Leased {
            position: 0,
            consumer: ConsumerId::new("other"),
            lease: LeaseId::new(Partition::new(0), 0),
        };
        assert!(
            view_matches_pocket(&key, &other, &pocket)
                .is_some_and(|v| v.detail.contains("\"other\""))
        );
        let successor = OrderStatus::Leased {
            position: 0,
            consumer: ConsumerId::new("c"),
            lease: LeaseId::new(Partition::new(0), 1),
        };
        assert!(
            view_matches_pocket(&key, &successor, &pocket)
                .is_some_and(|v| v.detail.contains("serial: 1"))
        );
        assert_eq!(
            view_matches_pocket(&key, &OrderStatus::Idle, &pocket).map(|v| v.invariant),
            Some("spike:OrderViewConsistent")
        );
    }
}
