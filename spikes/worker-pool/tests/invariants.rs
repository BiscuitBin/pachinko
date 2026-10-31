//! The module's invariants, held by property over a scripted sequence of accepted and
//! refused actions run through the single-threaded router, with the oracle in
//! `support/model.rs` saying before each step what the rules allow.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

#[path = "support/model.rs"]
mod model;

use std::collections::BTreeSet;

use model::{
    Prediction, Snapshot, ack, consumer_joins, matches, nack, predict, publish, publisher_joins,
    pull, release, renew,
};
use worker_pool::{
    Became, Config, Direct, Duration, Input, LeaseId, LeaseStatus, Machine, MachineId, PocketKey,
    PocketStatus, Response, Timestamp, invariants, partition_of,
};

/// Two partitions: `A` hashes to partition 0 and `B` to partition 1, so the script
/// crosses both pocket machines.
const PARTITIONS: u32 = 2;

/// One step of a script: a client input at a time, or the tick at a time.
#[derive(Clone, Debug)]
enum Step {
    Client(Input, Timestamp),
    Tick(Timestamp),
}

const fn ms(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

/// The lease the order machine mints for the `serial`-th claim of a pocket on `key`'s
/// partition: the router is deterministic, so a script can name leases in advance.
fn lease_on(key: &str, serial: u64) -> LeaseId {
    LeaseId::new(partition_of(&PocketKey::new(key), PARTITIONS), serial)
}

fn at(now: u64, inputs: Vec<Input>) -> Vec<Step> {
    inputs
        .into_iter()
        .map(|input| Step::Client(input, ms(now)))
        .collect()
}

/// Joins, duplicate joins, a pull with nothing to pull and an action naming a lease
/// nobody minted.
fn phase_joins() -> Vec<Step> {
    at(
        0,
        vec![
            publisher_joins("p1"),
            publisher_joins("p1"),
            consumer_joins("c1"),
            consumer_joins("c2"),
            consumer_joins("c2"),
            pull("c1", 1),
            ack("c1", lease_on("A", 0), "r1"),
        ],
    )
}

/// Every publish rule: a new pocket, a new key, a supersede, a stale drop, and the
/// uncovered equal rank; then a second pocket with three records.
fn phase_publishes() -> Vec<Step> {
    at(
        1,
        vec![
            publish("p1", "A", "r1", 1),
            publish("p1", "A", "r2", 1),
            publish("p1", "A", "r1", 2),
            publish("p1", "A", "r1", 1),
            publish("p1", "A", "r1", 2),
            publish("p1", "B", "s1", 1),
            publish("p1", "B", "s2", 1),
            publish("p1", "B", "s3", 1),
        ],
    )
}

/// `c1` claims `A` as lease 0: a pull within the lease, an ack that settles, a
/// supersede while held, an ack that defers, then the record guards of ack and nack
/// (a record not held, no such record) and the nack lease guards for another
/// consumer's lease and an unknown one, a renewal and its wrong-consumer refusal, and
/// `c2` claiming `B` as lease 1 with limit 0.
fn phase_first_lease() -> Vec<Step> {
    let l0 = lease_on("A", 0);
    at(
        2,
        vec![
            pull("c1", 5),
            pull("c1", 1),
            ack("c1", l0, "r1"),
            publish("p1", "A", "r2", 5),
            ack("c1", l0, "r2"),
            ack("c1", l0, "r2"),
            ack("c1", l0, "r9"),
            nack("c1", l0, "r9"),
            nack("c1", l0, "r2"),
            nack("c2", l0, "r2"),
            nack("c1", lease_on("A", 77), "r2"),
            renew("c1", l0),
            renew("c2", l0),
            pull("c2", 0),
        ],
    )
}

/// Releases that requeue, actions under released leases, a claim of `B` by `c2` who
/// then stalls (lease 2), a nack that settles, a release that idles the empty pocket
/// and more actions under ended leases (a nack, two acks, one by the wrong consumer,
/// which the ended lease refuses first), and a renewal and a release naming a lease
/// nobody minted.
fn phase_releases() -> Vec<Step> {
    let (l0, l1, l3) = (lease_on("A", 0), lease_on("B", 1), lease_on("A", 3));
    let mut steps = at(
        3,
        vec![
            release("c2", l1),
            release("c2", l1),
            renew("c2", l1),
            release("c1", l0),
            pull("c2", 2),
            pull("c1", 2),
            nack("c1", l3, "r2"),
            release("c1", l3),
            release("c1", l3),
            nack("c1", l3, "r2"),
            renew("c1", lease_on("A", 77)),
            release("c1", lease_on("A", 77)),
            ack("c1", l3, "r2"),
            ack("c2", l3, "r2"),
            release("c1", l0),
        ],
    );
    steps.extend(at(4, vec![publish("p1", "A", "r3", 1)]));
    steps
}

/// The stalled consumer's lease on `B` lapses three times, keeping the pocket at the
/// front twice and parking it the third time, while every decay runs; then `c2`
/// claims `A`, nacks its one record and stalls so that the lapse idles it.
fn phase_lapses() -> Vec<Step> {
    let mut steps = vec![Step::Tick(ms(13))];
    steps.extend(at(13, vec![pull("c2", 1)]));
    steps.push(Step::Tick(ms(23)));
    steps.extend(at(23, vec![pull("c2", 1)]));
    steps.push(Step::Tick(ms(33)));
    steps.extend(at(
        33,
        vec![pull("c2", 1), nack("c2", lease_on("A", 6), "r3")],
    ));
    steps.push(Step::Tick(ms(34)));
    steps.push(Step::Tick(ms(43)));
    steps.extend(at(43, vec![pull("c1", 1)]));
    steps
}

/// A publish revives the parked pocket; `c1` pulls it in two batches with an empty
/// limit-0 pull between, supersedes a held key, nacks it so it defers, acks another,
/// while the wrong consumer is refused an ack, a renewal and a release under the live
/// lease, and `c1` releases it to the back.
fn phase_revival() -> Vec<Step> {
    let l7 = lease_on("B", 7);
    let mut steps = at(
        44,
        vec![
            publish("p1", "B", "s9", 1),
            pull("c1", 10),
            pull("c1", 0),
            pull("c1", 10),
            publish("p1", "B", "s1", 3),
            nack("c1", l7, "s1"),
            ack("c1", l7, "s2"),
            ack("c2", l7, "s3"),
            renew("c2", l7),
            release("c2", l7),
        ],
    );
    steps.extend(at(45, vec![release("c1", l7)]));
    steps.push(Step::Tick(ms(60)));
    steps
}

fn script() -> Vec<Step> {
    [
        phase_joins(),
        phase_publishes(),
        phase_first_lease(),
        phase_releases(),
        phase_lapses(),
        phase_revival(),
    ]
    .concat()
}

fn scripted_config() -> Config {
    Config::default()
        .with_lease_ttl(Duration::from_millis(10))
        .with_max_consecutive_expiries(3)
        .with_max_pull_batch(2)
        .with_superseded_ttl(Duration::from_millis(5))
        .with_dead_letter_ttl(Duration::from_millis(5))
}

/// What the run saw, so the test can refuse a script that degenerated.
#[derive(Debug, Default)]
struct Tally {
    accepted: usize,
    refused: usize,
    /// Every accepted reply shape by its debug name, and every refusal as
    /// `action/reason`, so the tally tells which action a guard refused.
    seen: BTreeSet<String>,
}

/// The input's variant name: `Ack`, `Nack`, `Renew`, `Release`, and so on.
fn action_of(input: &Input) -> String {
    format!("{input:?}")
        .split(['{', '(', ' '])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn invariants_hold(direct: &Direct, context: &dyn std::fmt::Debug) {
    let violations = invariants::check(direct.order(), direct.pockets());
    assert_eq!(violations, Vec::new(), "after {context:?}");
}

fn drive(direct: &mut Direct, index: usize, step: &Step, tally: &mut Tally) -> Result<(), String> {
    match step {
        Step::Tick(now) => {
            direct
                .tick(*now)
                .map_err(|error| format!("step {index} tick at {now:?}: {error}"))?;
            invariants_hold(direct, &(index, step));
        }
        Step::Client(input, now) => {
            let predicted = predict(&Snapshot::of(direct), input, *now);
            let response = direct
                .submit(input.clone(), *now)
                .map_err(|error| format!("step {index} {input:?}: {error}"))?;
            assert!(
                matches(&predicted, &response),
                "step {index} {input:?} at {now:?}: predicted {predicted:?}, got {response:?}"
            );
            invariants_hold(direct, &(index, step));
            match (&predicted, &response) {
                (Prediction::Reject(reason), _) => {
                    tally.refused += 1;
                    tally
                        .seen
                        .insert(format!("{}/{reason:?}", action_of(input)));
                }
                (Prediction::Accept(_), Response::Published(published)) => {
                    tally.accepted += 1;
                    tally.seen.insert(format!("{published:?}"));
                }
                (Prediction::Accept(kind), _) => {
                    tally.accepted += 1;
                    tally.seen.insert(
                        format!("{kind:?}")
                            .split(['(', ' '])
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn became_in_order_log(direct: &Direct) -> Vec<Became> {
    direct.logs()[&MachineId::Order]
        .iter()
        .filter_map(|(input, _)| match input {
            Input::PocketBecame { became, .. } => Some(*became),
            _ => None,
        })
        .collect()
}

/// Every accepted reply shape the script must have produced, by its debug name, and
/// every refusal as `action/reason`: the three guards of the join and publish rules,
/// then each of the four lease actions refused by each guard it has, so that coverage
/// is per guard and not per reason. `Protocol` and `PocketNotLeased` are unreachable
/// by the invariants.
const EVERY_OUTCOME: [&str; 30] = [
    "Joined",
    "NewPocket",
    "NewKey",
    "Superseded",
    "Stale",
    "Batch",
    "NoPocketAvailable",
    "Acked",
    "Nacked",
    "Renewed",
    "Released",
    "PublisherJoins/DuplicatePublisher",
    "ConsumerJoins/DuplicateConsumer",
    "Publish/EqualRank",
    "Ack/UnknownLease",
    "Ack/LeaseNotActive",
    "Ack/NotYourLease",
    "Ack/NoSuchRecord",
    "Ack/RecordNotHeld",
    "Nack/UnknownLease",
    "Nack/LeaseNotActive",
    "Nack/NotYourLease",
    "Nack/NoSuchRecord",
    "Nack/RecordNotHeld",
    "Renew/UnknownLease",
    "Renew/LeaseNotActive",
    "Renew/NotYourLease",
    "Release/UnknownLease",
    "Release/LeaseNotActive",
    "Release/NotYourLease",
];

const EVERY_BECAME: [Became; 4] = [
    Became::RequeuedAtBack,
    Became::RequeuedKeepingPosition,
    Became::Idle,
    Became::Parked,
];

#[test]
fn every_invariant_holds_after_every_step_of_a_scripted_sequence_with_refusals() {
    let mut direct = Direct::new(scripted_config(), PARTITIONS);
    let mut tally = Tally::default();
    for (index, step) in script().iter().enumerate() {
        drive(&mut direct, index, step, &mut tally).unwrap();
    }
    assert!(tally.accepted >= 30, "{tally:?}");
    assert!(tally.refused >= 21, "{tally:?}");
    for expected in EVERY_OUTCOME {
        assert!(
            tally.seen.contains(expected),
            "never saw {expected}: {tally:?}"
        );
    }
    let became = became_in_order_log(&direct);
    for expected in EVERY_BECAME {
        assert!(
            became.contains(&expected),
            "never saw {expected:?}: {became:?}"
        );
    }
    assert_eq!(direct.order().violations(), 0);
    for machine in direct.pockets() {
        assert_eq!(machine.violations(), 0, "{:?}", machine.id());
    }
}

/// Runs one client input and returns its reply, naming the input on failure.
fn reply(direct: &mut Direct, input: &Input, now: Timestamp) -> Result<Response, String> {
    direct
        .submit(input.clone(), now)
        .map_err(|error| format!("{input:?} at {now:?}: {error}"))
}

const fn lease_of(response: &Response) -> Option<LeaseId> {
    match response {
        Response::Batch { lease, .. } => Some(*lease),
        _ => None,
    }
}

fn handed(response: &Response) -> Option<Vec<&str>> {
    match response {
        Response::Batch { records, .. } => Some(records.iter().map(|r| r.key.as_str()).collect()),
        _ => None,
    }
}

#[test]
fn an_action_naming_an_ended_lease_is_rejected_under_the_successor_lease_and_records_are_unchanged()
{
    let config = Config::default().with_lease_ttl(Duration::from_millis(10));
    let mut direct = Direct::new(config, PARTITIONS);
    reply(&mut direct, &publish("p1", "A", "r1", 1), ms(0)).unwrap();
    let first = lease_of(&reply(&mut direct, &pull("c1", 5), ms(1)).unwrap()).unwrap();
    direct.tick(ms(11)).unwrap();
    assert_eq!(
        reply(&mut direct, &publish("p1", "A", "r1", 2), ms(12)).unwrap(),
        Response::Published(worker_pool::Published::Superseded)
    );
    let second = reply(&mut direct, &pull("c1", 5), ms(12)).unwrap();
    assert_eq!(handed(&second), Some(vec!["r1"]));
    let second = lease_of(&second).unwrap();
    assert_ne!(first, second);
    let before = Snapshot::of(&direct);
    assert_eq!(
        reply(&mut direct, &ack("c1", first, "r1"), ms(13)).unwrap(),
        Response::Rejected(worker_pool::Reason::LeaseNotActive)
    );
    assert_eq!(Snapshot::of(&direct), before);
    let pocket = direct.pockets()[0].pocket(&PocketKey::new("A")).unwrap();
    assert_eq!(pocket.active_lease().map(|l| l.id), Some(second));
    assert_eq!(
        pocket.lease(first).map(|l| l.status),
        Some(LeaseStatus::Expired)
    );
    invariants_hold(&direct, &"the end");
}

#[test]
fn a_lapse_and_a_late_renewal_are_ordered_by_the_pocket_machine_either_way() {
    let config = Config::default().with_lease_ttl(Duration::from_millis(10));
    let key = PocketKey::new("A");
    let mut setup = Direct::new(config, PARTITIONS);
    reply(&mut setup, &publish("p1", "A", "r1", 1), ms(0)).unwrap();
    let lease = lease_of(&reply(&mut setup, &pull("c1", 5), ms(1)).unwrap()).unwrap();

    let mut tick_first = setup.clone();
    tick_first.tick(ms(11)).unwrap();
    assert_eq!(
        reply(&mut tick_first, &renew("c1", lease), ms(11)).unwrap(),
        Response::Rejected(worker_pool::Reason::LeaseNotActive)
    );
    let pocket = tick_first.pockets()[0].pocket(&key).unwrap();
    assert_eq!(
        pocket.lease(lease).map(|l| l.status),
        Some(LeaseStatus::Expired)
    );
    assert_eq!(pocket.status, PocketStatus::Queued);
    invariants_hold(&tick_first, &"tick then renew");

    let mut renew_first = setup;
    assert_eq!(
        reply(&mut renew_first, &renew("c1", lease), ms(11)).unwrap(),
        Response::Renewed { expires_at: ms(21) }
    );
    renew_first.tick(ms(11)).unwrap();
    let pocket = renew_first.pockets()[0].pocket(&key).unwrap();
    assert_eq!(
        pocket.lease(lease).map(|l| l.status),
        Some(LeaseStatus::Active)
    );
    assert_eq!(pocket.status, PocketStatus::Leased);
    assert_eq!(became_in_order_log(&renew_first), Vec::new());
    invariants_hold(&renew_first, &"renew then tick");
}

#[test]
fn one_pocket_became_per_lease_end_over_all_four_paths() {
    let config = Config::default()
        .with_lease_ttl(Duration::from_millis(10))
        .with_max_consecutive_expiries(2);
    let mut direct = Direct::new(config, PARTITIONS);
    // Release requeues `A`, then release idles it once its record is acked.
    reply(&mut direct, &publish("p1", "A", "r1", 1), ms(0)).unwrap();
    let lease = lease_of(&reply(&mut direct, &pull("c1", 5), ms(0)).unwrap()).unwrap();
    assert_eq!(
        reply(&mut direct, &release("c1", lease), ms(0)).unwrap(),
        Response::Released
    );
    let lease = lease_of(&reply(&mut direct, &pull("c1", 5), ms(1)).unwrap()).unwrap();
    assert_eq!(
        reply(&mut direct, &ack("c1", lease, "r1"), ms(1)).unwrap(),
        Response::Acked
    );
    assert_eq!(
        reply(&mut direct, &release("c1", lease), ms(1)).unwrap(),
        Response::Released
    );
    // A stalled consumer on `B`: the first lapse keeps it at the front, the second parks it.
    reply(&mut direct, &publish("p1", "B", "s1", 1), ms(2)).unwrap();
    reply(&mut direct, &pull("c2", 5), ms(2)).unwrap();
    direct.tick(ms(12)).unwrap();
    reply(&mut direct, &pull("c2", 5), ms(12)).unwrap();
    direct.tick(ms(22)).unwrap();
    assert_eq!(
        direct.pockets()[1]
            .pocket(&PocketKey::new("B"))
            .unwrap()
            .status,
        PocketStatus::Parked
    );
    // `A` revived, pulled, emptied by an ack, and idled by the lapse.
    reply(&mut direct, &publish("p1", "A", "r2", 1), ms(22)).unwrap();
    let lease = lease_of(&reply(&mut direct, &pull("c1", 5), ms(22)).unwrap()).unwrap();
    assert_eq!(
        reply(&mut direct, &ack("c1", lease, "r2"), ms(22)).unwrap(),
        Response::Acked
    );
    direct.tick(ms(32)).unwrap();

    let ended: usize = direct
        .pockets()
        .iter()
        .flat_map(worker_pool::PocketMachine::pockets)
        .flat_map(|(_, pocket)| pocket.leases.iter())
        .filter(|lease| lease.status != LeaseStatus::Active)
        .count();
    let became = became_in_order_log(&direct);
    assert_eq!(ended, 5);
    assert_eq!(
        became,
        vec![
            Became::RequeuedAtBack,
            Became::Idle,
            Became::RequeuedKeepingPosition,
            Became::Parked,
            Became::Idle,
        ]
    );
    invariants_hold(&direct, &"the end");
}

/// Three pulls under one lease, each run to quiescence before the next: `Direct`
/// cannot put the second pull inside the first `Claim`'s window, so the retry path is
/// unreachable here and `tests/window.rs` owns it.
#[test]
fn consecutive_pulls_under_one_lease_yield_exactly_one_reply_each() {
    let config = Config::default().with_max_pull_batch(1);
    let mut direct = Direct::new(config, PARTITIONS);
    reply(&mut direct, &publish("p1", "A", "r1", 1), ms(0)).unwrap();
    reply(&mut direct, &publish("p1", "A", "r2", 1), ms(0)).unwrap();
    let first = reply(&mut direct, &pull("c1", 10), ms(1)).unwrap();
    let second = reply(&mut direct, &pull("c1", 10), ms(1)).unwrap();
    let third = reply(&mut direct, &pull("c1", 10), ms(1)).unwrap();
    assert_eq!(handed(&first), Some(vec!["r1"]));
    assert_eq!(handed(&second), Some(vec!["r2"]));
    assert_eq!(handed(&third), Some(Vec::new()));
    assert_eq!(lease_of(&first), lease_of(&second));
    assert_eq!(lease_of(&first), lease_of(&third));
    invariants_hold(&direct, &"the end");
}
