//! Replay: a machine is a function of its input log. Every machine the single-threaded
//! router drove through a scripted sequence equals a fresh machine stepped through the
//! same inputs at the same times, which is what lets the pool recover a machine from
//! its log and what makes the machines comparable across execution models.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use worker_pool::{
    Config, ConsumerId, Direct, Duration, Input, LeaseId, Machine, MachineId, OrderMachine,
    Payload, PocketKey, PocketMachine, PublisherId, Rank, RecordKey, Response, Timestamp,
    partition_of,
};

/// Three partitions: `A` and `B` hash to partition 0 and `C` to partition 2, so one
/// partition is shared, one is alone and partition 1 sees only ticks.
const PARTITIONS: u32 = 3;

const fn ms(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

fn lease_on(key: &str, serial: u64) -> LeaseId {
    LeaseId::new(partition_of(&PocketKey::new(key), PARTITIONS), serial)
}

fn join_publisher(publisher: &str) -> Input {
    Input::PublisherJoins {
        publisher: PublisherId::new(publisher),
    }
}

fn join_consumer(consumer: &str) -> Input {
    Input::ConsumerJoins {
        consumer: ConsumerId::new(consumer),
    }
}

fn publish(pocket: &str, record: &str, rank: u64) -> Input {
    Input::Publish {
        publisher: PublisherId::new("p1"),
        pocket_key: PocketKey::new(pocket),
        record_key: RecordKey::new(record),
        rank: Rank::new(rank),
        payload: Payload::new(format!("{record}@{rank}")),
    }
}

fn pull(consumer: &str, limit: u64) -> Input {
    Input::Pull {
        consumer: ConsumerId::new(consumer),
        limit,
    }
}

fn ack(consumer: &str, lease: LeaseId, record: &str) -> Input {
    Input::Ack {
        consumer: ConsumerId::new(consumer),
        lease,
        record_key: RecordKey::new(record),
    }
}

fn nack(consumer: &str, lease: LeaseId, record: &str) -> Input {
    Input::Nack {
        consumer: ConsumerId::new(consumer),
        lease,
        record_key: RecordKey::new(record),
    }
}

fn renew(consumer: &str, lease: LeaseId) -> Input {
    Input::Renew {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

fn release(consumer: &str, lease: LeaseId) -> Input {
    Input::Release {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

/// Submits each input at `now`, requiring the one reply the protocol promises, and
/// returns the replies for the script's own checks.
fn submit_all(
    direct: &mut Direct,
    now: Timestamp,
    inputs: Vec<Input>,
) -> Result<Vec<Response>, String> {
    inputs
        .into_iter()
        .map(|input| {
            direct
                .submit(input.clone(), now)
                .map_err(|error| format!("{input:?} at {now:?}: {error}"))
        })
        .collect()
}

fn tick(direct: &mut Direct, now: Timestamp) -> Result<(), String> {
    direct
        .tick(now)
        .map_err(|error| format!("tick at {now:?}: {error}"))
}

/// Joins and every publish rule over three pockets.
fn phase_publishes(direct: &mut Direct) -> Result<(), String> {
    submit_all(
        direct,
        ms(0),
        vec![
            join_publisher("p1"),
            join_consumer("c1"),
            join_consumer("c2"),
            publish("A", "a1", 1),
            publish("A", "a2", 1),
            publish("A", "a1", 2),
            publish("A", "a1", 1),
            publish("A", "a1", 2),
            publish("B", "b1", 1),
            publish("C", "c1", 1),
            publish("C", "c2", 1),
        ],
    )?;
    Ok(())
}

/// `c1` on `A`: a claim, a pull within the lease, an ack that settles, a supersede
/// while held, an ack that defers, a renewal and a release that requeues; `c2` on `B`:
/// a claim, a nack that settles and a release that idles the empty pocket.
fn phase_leases(direct: &mut Direct) -> Result<(), String> {
    let (l0, l1) = (lease_on("A", 0), lease_on("B", 1));
    submit_all(
        direct,
        ms(1),
        vec![
            pull("c1", 5),
            pull("c1", 5),
            ack("c1", l0, "a1"),
            publish("A", "a2", 7),
            ack("c1", l0, "a2"),
            renew("c1", l0),
            pull("c2", 1),
            nack("c2", l1, "b1"),
            release("c2", l1),
            release("c1", l0),
        ],
    )?;
    Ok(())
}

/// `c2` claims `C` and stalls; `c1` claims `A`, nacks its last record and stalls. The
/// first tick keeps `C` at the front and idles `A`, and decays everything shed so far.
/// `c2` claims `C` again and stalls, a publish revives idle `B`, and the second tick
/// parks `C`. A publish revives parked `C`, `c1` claims `B` and releases it.
fn phase_lapses(direct: &mut Direct) -> Result<(), String> {
    let (l3, l5) = (lease_on("A", 3), lease_on("B", 5));
    submit_all(
        direct,
        ms(2),
        vec![pull("c2", 2), pull("c1", 2), nack("c1", l3, "a2")],
    )?;
    tick(direct, ms(12))?;
    submit_all(direct, ms(12), vec![pull("c2", 1), publish("B", "b2", 1)])?;
    tick(direct, ms(22))?;
    submit_all(
        direct,
        ms(22),
        vec![publish("C", "c3", 1), pull("c1", 5), release("c1", l5)],
    )?;
    tick(direct, ms(40))
}

fn scripted() -> Result<Direct, String> {
    let config = Config::default()
        .with_lease_ttl(Duration::from_millis(10))
        .with_max_consecutive_expiries(2)
        .with_max_pull_batch(2)
        .with_superseded_ttl(Duration::from_millis(5))
        .with_dead_letter_ttl(Duration::from_millis(5));
    let mut direct = Direct::new(config, PARTITIONS);
    phase_publishes(&mut direct)?;
    phase_leases(&mut direct)?;
    phase_lapses(&mut direct)?;
    Ok(direct)
}

#[test]
fn each_machine_replayed_from_its_input_log_equals_itself() {
    let direct = scripted().unwrap();
    assert_eq!(direct.logs().len(), 1 + PARTITIONS as usize);
    for (id, log) in direct.logs() {
        assert!(!log.is_empty(), "{id:?} was never stepped");
        match id {
            MachineId::Order => {
                let mut fresh = OrderMachine::new(*direct.config());
                for (input, now) in log {
                    fresh.step(input.clone(), *now);
                }
                assert_eq!(&fresh, direct.order());
            }
            MachineId::Partition(partition) => {
                let mut fresh = PocketMachine::new(*partition, PARTITIONS, *direct.config());
                for (input, now) in log {
                    fresh.step(input.clone(), *now);
                }
                assert_eq!(&fresh, &direct.pockets()[partition.index() as usize]);
            }
            other => panic!("unexpected machine {other:?}"),
        }
    }
    // The script reached every machine: partition 1 heard only the ticks, the others
    // heard client inputs and the inputs chained from them.
    let idle = &direct.logs()[&MachineId::Partition(worker_pool::Partition::new(1))];
    assert_eq!(idle.len(), 3);
    assert!(idle.iter().all(|(input, _)| *input == Input::Tick));
    assert!(
        direct.logs()[&MachineId::Order]
            .iter()
            .any(|(input, _)| matches!(input, Input::PocketBecame { .. }))
    );
    assert!(
        direct.logs()[&MachineId::Partition(worker_pool::Partition::new(2))]
            .iter()
            .any(|(input, _)| matches!(input, Input::Claim { .. }))
    );
    assert_eq!(direct.order().violations(), 0);
    assert!(
        direct
            .pockets()
            .iter()
            .all(|machine| machine.violations() == 0)
    );
}
