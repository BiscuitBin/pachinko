//! A proptest state machine: random sequences of every trigger, with the module's
//! invariants and declared transition graphs checked on the queue after each step.
//!
//! The guards no direct test can make fail are covered here, because each is
//! implied by an invariant this machine checks after every step:
//! `rule-failure.PullWithinLease.2`, `rule-failure.ReleaseRequeuesPocket.3`,
//! `rule-failure.ReleaseIdlesEmptyPocket.3`, `rule-failure.LeaseExpiryParksPocket.3`,
//! `rule-failure.LeaseExpiryKeepsPocketAtFront.3` and
//! `rule-failure.LeaseExpiryIdlesEmptyPocket.3` all require a lease's pocket to be
//! leased, which `LeasedHasOneActiveLease` and `UnleasedHasNoActiveLease` make true
//! of every active lease.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use libpachinko::error::ActionError;
use libpachinko::queue::{Lease, LeaseId, LeaseStatus, PocketStatus, PullOutcome, QueueSnapshot};
use libpachinko::records::RecordStatus;
use proptest::prelude::*;

use crate::fixture::{Fixture, config};

const POCKETS: [&str; 3] = ["p0", "p1", "p2"];
const KEYS: [&str; 3] = ["k0", "k1", "k2"];
const CONSUMERS: [&str; 3] = ["c1", "c2", "c3"];
const MAX_EXPIRIES: u32 = 2;
const MAX_BATCH: u32 = 2;

/// One trigger, with indexes into the constants above.
#[derive(Clone, Debug)]
enum Step {
    Publish {
        pocket: usize,
        key: usize,
        rank: i64,
    },
    Pull {
        consumer: usize,
        limit: i64,
    },
    Ack {
        consumer: usize,
        lease: usize,
        key: usize,
    },
    Nack {
        consumer: usize,
        lease: usize,
        key: usize,
    },
    Renew {
        consumer: usize,
        lease: usize,
    },
    Release {
        consumer: usize,
        lease: usize,
    },
    Advance {
        millis: u64,
    },
}

fn step() -> impl Strategy<Value = Step> {
    let pocket = 0..POCKETS.len();
    let key = 0..KEYS.len();
    let consumer = 0..CONSUMERS.len();
    prop_oneof![
        3 => (pocket, key.clone(), 0..4_i64)
            .prop_map(|(pocket, key, rank)| Step::Publish { pocket, key, rank }),
        2 => (consumer.clone(), -1..4_i64).prop_map(|(consumer, limit)| Step::Pull { consumer, limit }),
        2 => (consumer.clone(), 0..4_usize, key.clone())
            .prop_map(|(consumer, lease, key)| Step::Ack { consumer, lease, key }),
        1 => (consumer.clone(), 0..4_usize, key)
            .prop_map(|(consumer, lease, key)| Step::Nack { consumer, lease, key }),
        1 => (consumer.clone(), 0..4_usize).prop_map(|(consumer, lease)| Step::Renew { consumer, lease }),
        1 => (consumer, 0..4_usize).prop_map(|(consumer, lease)| Step::Release { consumer, lease }),
        2 => (0..1_500_u64).prop_map(|millis| Step::Advance { millis }),
    ]
}

/// The machine: the fixture, every lease id handed out, and the ended ones.
struct Machine {
    fixture: Fixture,
    granted: Vec<LeaseId>,
    ended: BTreeSet<LeaseId>,
}

impl Machine {
    fn new() -> Self {
        let mut config = config();
        config.max_consecutive_expiries = MAX_EXPIRIES;
        config.max_pull_batch = MAX_BATCH;
        let mut fixture = Fixture::with_config(config);
        fixture.queue.consumer_joins("c3").unwrap();
        Self {
            fixture,
            granted: Vec::new(),
            ended: BTreeSet::new(),
        }
    }

    /// A lease id for an action: index 0 is the consumer's own active lease, the
    /// rest pick from every id handed out, ended ones included.
    fn lease(&self, consumer: usize, choice: usize) -> LeaseId {
        let own = self.fixture.active_lease(CONSUMERS[consumer]);
        match (choice, own) {
            (0, Some(own)) => own,
            _ if self.granted.is_empty() => LeaseId::from_raw(0),
            _ => self.granted[choice % self.granted.len()],
        }
    }

    fn apply(&mut self, step: &Step) {
        match *step {
            Step::Publish { pocket, key, rank } => {
                let _ = self
                    .fixture
                    .publish(POCKETS[pocket], KEYS[key], rank, "payload");
            }
            Step::Pull { consumer, limit } => self.pull(consumer, limit),
            Step::Ack {
                consumer,
                lease,
                key,
            } => {
                let id = self.lease(consumer, lease);
                let result = self.fixture.queue.ack(CONSUMERS[consumer], id, KEYS[key]);
                self.check_action(id, &result.map(|_| ()));
            }
            Step::Nack {
                consumer,
                lease,
                key,
            } => {
                let id = self.lease(consumer, lease);
                let result = self.fixture.queue.nack(CONSUMERS[consumer], id, KEYS[key]);
                self.check_action(id, &result.map(|_| ()));
            }
            Step::Renew { consumer, lease } => {
                let id = self.lease(consumer, lease);
                let result = self.fixture.queue.renew(CONSUMERS[consumer], id);
                self.check_action(id, &result.map(|_| ()));
            }
            Step::Release { consumer, lease } => {
                let id = self.lease(consumer, lease);
                let result = self.fixture.queue.release(CONSUMERS[consumer], id);
                self.check_action(id, &result.clone().map(|_| ()));
                self.record_ends(result);
            }
            Step::Advance { millis } => {
                self.fixture.advance(Duration::from_millis(millis));
                let report = self.fixture.queue.tick();
                self.record_ends(report.lapsed);
            }
        }
    }

    fn pull(&mut self, consumer: usize, limit: i64) {
        let outcome = self.fixture.queue.pull(CONSUMERS[consumer], limit);
        // A pull is never an error for a joined consumer, `no_pocket` included.
        if let PullOutcome::Batch(batch) = outcome.unwrap() {
            let cap = usize::try_from(limit.clamp(0, i64::from(MAX_BATCH))).unwrap();
            assert!(batch.records.len() <= cap);
            if !self.granted.contains(&batch.lease) {
                // TransitionEdge: a new lease takes an id above every earlier one.
                assert!(self.granted.iter().all(|earlier| *earlier < batch.lease));
                self.granted.push(batch.lease);
            }
        }
    }

    /// An action on an ended lease is rejected: `released` and `expired` are
    /// terminal.
    fn check_action(&self, id: LeaseId, result: &Result<(), ActionError>) {
        if self.ended.contains(&id) {
            assert_eq!(*result, Err(ActionError::LeaseNotActive(id)));
        }
    }

    fn record_ends(&mut self, ended: impl IntoIterator<Item = Lease>) {
        for lease in ended {
            assert_ne!(lease.status, LeaseStatus::Active);
            let ended_at = lease.ended_at.expect("an ended lease has ended_at");
            assert!(ended_at >= lease.granted_at, "EndedAfterGrant");
            assert!(self.ended.insert(lease.id), "a lease ends once");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    // obligation: invariant.Pocket.LeasedHasOneActiveLease
    // obligation: invariant.Pocket.UnleasedHasNoActiveLease
    // obligation: invariant.Pocket.IdleHasNoRecords
    // obligation: invariant.Pocket.ExpiriesWithinLimit
    // obligation: invariant.Record.HeldVersionNotNewer
    // obligation: invariant.SupersededRecord.ShedBeforeExpiry
    // obligation: invariant.DeadLetter.NackedBeforeExpiry
    // obligation: invariant.Lease.GrantedBeforeExpiry
    // obligation: invariant.Lease.EndedAfterGrant
    // obligation: invariant.UniquePublisherId
    // obligation: invariant.UniqueConsumerId
    // obligation: invariant.OnePocketPerKey
    // obligation: invariant.OneRecordPerKeyPerPocket
    // obligation: invariant.OneActiveLeasePerPocket
    // obligation: invariant.OneActiveLeasePerConsumer
    // obligation: invariant.OutstandingOnlyWhileLeased
    // obligation: invariant.UniquePocketPosition
    // obligation: invariant.UniqueRecordPosition
    // obligation: invariant.PocketPositionsBelowNext
    // obligation: invariant.RecordPositionsBelowNext
    // obligation: transition-rejected.Pocket.status
    // obligation: transition-rejected.Record.status
    // obligation: transition-rejected.Lease.status
    // obligation: transition-terminal.Lease.status
    #[test]
    fn every_step_keeps_the_invariants_and_the_transition_graphs(
        steps in proptest::collection::vec(step(), 1..60)
    ) {
        let mut machine = Machine::new();
        let mut before = machine.fixture.snapshot();
        check_invariants(&before);
        for step in &steps {
            machine.apply(step);
            let after = machine.fixture.snapshot();
            check_invariants(&after);
            check_transitions(&before, &after);
            check_ended_stay_ended(&machine, &before, &after);
            before = after;
        }
    }
}

fn check_invariants(snapshot: &QueueSnapshot) {
    check_unique_ids(snapshot);
    check_pockets(snapshot);
    check_records(snapshot);
    check_leases(snapshot);
    check_shed(snapshot);
}

fn check_unique_ids(snapshot: &QueueSnapshot) {
    let publishers: BTreeSet<&String> = snapshot.publishers.iter().collect();
    assert_eq!(
        publishers.len(),
        snapshot.publishers.len(),
        "UniquePublisherId"
    );
    let consumers: BTreeSet<&String> = snapshot.consumers.iter().map(|(id, _)| id).collect();
    assert_eq!(
        consumers.len(),
        snapshot.consumers.len(),
        "UniqueConsumerId"
    );
    let pockets: BTreeSet<&String> = snapshot.pockets.iter().map(|pocket| &pocket.key).collect();
    assert_eq!(pockets.len(), snapshot.pockets.len(), "OnePocketPerKey");
}

fn check_pockets(snapshot: &QueueSnapshot) {
    let mut positions = BTreeSet::new();
    for pocket in &snapshot.pockets {
        let active = snapshot
            .leases
            .iter()
            .filter(|lease| lease.pocket_key == pocket.key)
            .count();
        match pocket.status {
            PocketStatus::Leased => assert_eq!(active, 1, "LeasedHasOneActiveLease"),
            _ => assert_eq!(active, 0, "UnleasedHasNoActiveLease"),
        }
        if pocket.status == PocketStatus::Idle {
            assert!(pocket.records.is_empty(), "IdleHasNoRecords");
        }
        assert!(
            pocket.consecutive_expiries <= MAX_EXPIRIES,
            "ExpiriesWithinLimit"
        );
        assert!(positions.insert(pocket.position), "UniquePocketPosition");
        assert!(
            pocket.position < snapshot.next_position,
            "PocketPositionsBelowNext"
        );
    }
}

fn check_records(snapshot: &QueueSnapshot) {
    let mut positions = BTreeSet::new();
    for pocket in &snapshot.pockets {
        let keys: BTreeSet<&String> = pocket.records.iter().map(|record| &record.key).collect();
        assert_eq!(keys.len(), pocket.records.len(), "OneRecordPerKeyPerPocket");
        for record in &pocket.records {
            assert_eq!(
                record.held.is_some(),
                record.status == RecordStatus::Held,
                "held_rank and held_payload exactly when held"
            );
            if let Some(held) = &record.held {
                assert!(held.rank <= record.latest.rank, "HeldVersionNotNewer");
            }
            if record.status != RecordStatus::Queued {
                assert_eq!(
                    pocket.status,
                    PocketStatus::Leased,
                    "OutstandingOnlyWhileLeased"
                );
            }
            assert!(positions.insert(record.position), "UniqueRecordPosition");
            assert!(
                record.position < snapshot.next_position,
                "RecordPositionsBelowNext"
            );
        }
    }
}

fn check_leases(snapshot: &QueueSnapshot) {
    let pockets: BTreeSet<&String> = snapshot
        .leases
        .iter()
        .map(|lease| &lease.pocket_key)
        .collect();
    assert_eq!(
        pockets.len(),
        snapshot.leases.len(),
        "OneActiveLeasePerPocket"
    );
    let holders: BTreeSet<&String> = snapshot
        .leases
        .iter()
        .map(|lease| &lease.consumer_id)
        .collect();
    assert_eq!(
        holders.len(),
        snapshot.leases.len(),
        "OneActiveLeasePerConsumer"
    );
    for lease in &snapshot.leases {
        assert_eq!(lease.status, LeaseStatus::Active);
        assert_eq!(lease.ended_at, None, "ended_at only once ended");
        assert!(lease.granted_at <= lease.expires_at, "GrantedBeforeExpiry");
        let bound = snapshot
            .consumers
            .iter()
            .find(|(id, _)| *id == lease.consumer_id)
            .and_then(|(_, active)| *active);
        assert_eq!(
            bound,
            Some(lease.id),
            "a consumer's active lease is the registry's"
        );
    }
}

fn check_shed(snapshot: &QueueSnapshot) {
    for record in &snapshot.superseded {
        assert!(record.shed_at <= record.expires_at, "ShedBeforeExpiry");
    }
    for letter in &snapshot.dead_letters {
        assert!(letter.nacked_at <= letter.expires_at, "NackedBeforeExpiry");
    }
}

/// The declared edges of `Pocket.status` and `Record.status`, and creation states.
fn check_transitions(before: &QueueSnapshot, after: &QueueSnapshot) {
    use PocketStatus::{Idle, Leased, Parked, Queued};
    use RecordStatus::{Deferred, Held};
    let pocket_edges = [
        (Queued, Leased),
        (Leased, Queued),
        (Leased, Idle),
        (Leased, Parked),
        (Idle, Queued),
        (Parked, Queued),
    ];
    let record_edges = [
        (RecordStatus::Queued, Held),
        (Held, RecordStatus::Queued),
        (Held, Deferred),
        (Deferred, RecordStatus::Queued),
    ];
    let old: BTreeMap<&String, _> = before
        .pockets
        .iter()
        .map(|pocket| (&pocket.key, pocket))
        .collect();
    for pocket in &after.pockets {
        let Some(previous) = old.get(&pocket.key) else {
            assert_eq!(pocket.status, Queued, "a pocket is created queued");
            continue;
        };
        let edge = (previous.status, pocket.status);
        assert!(
            edge.0 == edge.1 || pocket_edges.contains(&edge),
            "undeclared {edge:?}"
        );
        let records: BTreeMap<&String, _> = previous
            .records
            .iter()
            .map(|record| (&record.key, record.status))
            .collect();
        for record in &pocket.records {
            let edge = records
                .get(&record.key)
                .map(|status| (*status, record.status));
            match edge {
                None => assert_eq!(
                    record.status,
                    RecordStatus::Queued,
                    "a record is created queued"
                ),
                Some(edge) => assert!(
                    edge.0 == edge.1 || record_edges.contains(&edge),
                    "undeclared {edge:?}"
                ),
            }
        }
    }
    assert_eq!(old.len(), before.pockets.len());
    assert!(
        after.pockets.len() >= before.pockets.len(),
        "a pocket is never removed"
    );
}

/// A lease leaves the active set only by ending as released or expired, and
/// those are terminal: an ended lease never shows as active again.
fn check_ended_stay_ended(machine: &Machine, before: &QueueSnapshot, after: &QueueSnapshot) {
    for lease in &after.leases {
        assert!(
            !machine.ended.contains(&lease.id),
            "an ended lease came back"
        );
    }
    for lease in &before.leases {
        let still = after.leases.iter().any(|active| active.id == lease.id);
        assert!(
            still || machine.ended.contains(&lease.id),
            "a lease vanished without ending"
        );
    }
}
