//! Unit tests of the pocket machine: one accepted case per rule branch and one refused
//! case per guard, each expectation built from the rule's `requires` and `ensures` in
//! `docs/specs/pachinko.allium`. The machine is on partition 0 of 2, so its record
//! positions run 0, 2, 4, ...

use super::PocketMachine;
use crate::machine::Machine;
use crate::protocol::{
    Became, ConsumerId, HandedRecord, Input, LeaseId, MachineId, Output, Partition, Payload,
    PocketKey, Published, PublisherId, Rank, Reason, RecordKey, Response,
};
use crate::state::{
    DeadLetter, Lease, LeaseStatus, Pocket, PocketStatus, Record, RecordStatus, ShedReason,
    SupersededRecord,
};
use crate::time::{Config, Duration, Timestamp};

const PARTITION: Partition = Partition::new(0);
const PARTITIONS: u32 = 2;
const LEASE_TTL: Duration = Duration::from_millis(100);
const SHED_TTL: Duration = Duration::from_millis(1_000);
const MAX_PULL_BATCH: u64 = 3;

fn config() -> Config {
    Config::default()
        .with_lease_ttl(LEASE_TTL)
        .with_superseded_ttl(SHED_TTL)
        .with_dead_letter_ttl(SHED_TTL)
        .with_max_pull_batch(MAX_PULL_BATCH)
}

fn machine() -> PocketMachine {
    PocketMachine::new(PARTITION, PARTITIONS, config())
}

fn ms(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

fn key() -> PocketKey {
    PocketKey::new("k")
}

fn consumer() -> ConsumerId {
    ConsumerId::new("c")
}

fn lease(serial: u64) -> LeaseId {
    LeaseId::new(PARTITION, serial)
}

fn reply(response: Response) -> Vec<Output> {
    vec![Output::Reply(response)]
}

fn rejected(reason: Reason) -> Vec<Output> {
    reply(Response::Rejected(reason))
}

fn enqueue() -> Output {
    Output::Notify(
        MachineId::Order,
        Input::Enqueue {
            pocket_key: key(),
            partition: PARTITION,
        },
    )
}

fn became(became: Became, serial: u64) -> Output {
    Output::Notify(
        MachineId::Order,
        Input::PocketBecame {
            pocket_key: key(),
            became,
            consumer: consumer(),
            lease: lease(serial),
        },
    )
}

fn handed(record: &str, rank: u64, payload: &str) -> HandedRecord {
    HandedRecord::new(
        RecordKey::new(record),
        Rank::new(rank),
        Payload::new(payload),
    )
}

fn batch(serial: u64, records: Vec<HandedRecord>) -> Vec<Output> {
    reply(Response::Batch {
        lease: lease(serial),
        pocket_key: key(),
        records,
    })
}

fn publish_input(record: &str, rank: u64, payload: &str) -> Input {
    Input::Publish {
        publisher: PublisherId::new("p"),
        pocket_key: key(),
        record_key: RecordKey::new(record),
        rank: Rank::new(rank),
        payload: Payload::new(payload),
    }
}

fn publish(
    m: &mut PocketMachine,
    record: &str,
    rank: u64,
    payload: &str,
    now: Timestamp,
) -> Vec<Output> {
    m.step(publish_input(record, rank, payload), now)
}

fn claim_by(
    m: &mut PocketMachine,
    who: &str,
    serial: u64,
    limit: u64,
    now: Timestamp,
) -> Vec<Output> {
    m.step(
        Input::Claim {
            pocket_key: key(),
            consumer: ConsumerId::new(who),
            lease: lease(serial),
            limit,
        },
        now,
    )
}

fn claim(m: &mut PocketMachine, serial: u64, limit: u64, now: Timestamp) -> Vec<Output> {
    claim_by(m, "c", serial, limit, now)
}

fn pull(m: &mut PocketMachine, serial: u64, limit: u64, now: Timestamp) -> Vec<Output> {
    m.step(
        Input::PullUnderLease {
            consumer: consumer(),
            lease: lease(serial),
            limit,
        },
        now,
    )
}

fn ack_input(consumer: ConsumerId, lease: LeaseId) -> Input {
    Input::Ack {
        consumer,
        lease,
        record_key: RecordKey::new("r1"),
    }
}

fn nack_input(consumer: ConsumerId, lease: LeaseId) -> Input {
    Input::Nack {
        consumer,
        lease,
        record_key: RecordKey::new("r1"),
    }
}

fn renew_input(consumer: ConsumerId, lease: LeaseId) -> Input {
    Input::Renew { consumer, lease }
}

fn release_input(consumer: ConsumerId, lease: LeaseId) -> Input {
    Input::Release { consumer, lease }
}

fn ack(m: &mut PocketMachine, serial: u64, record: &str, now: Timestamp) -> Vec<Output> {
    m.step(
        Input::Ack {
            consumer: consumer(),
            lease: lease(serial),
            record_key: RecordKey::new(record),
        },
        now,
    )
}

fn nack(m: &mut PocketMachine, serial: u64, record: &str, now: Timestamp) -> Vec<Output> {
    m.step(
        Input::Nack {
            consumer: consumer(),
            lease: lease(serial),
            record_key: RecordKey::new(record),
        },
        now,
    )
}

fn renew(m: &mut PocketMachine, serial: u64, now: Timestamp) -> Vec<Output> {
    m.step(renew_input(consumer(), lease(serial)), now)
}

fn release(m: &mut PocketMachine, serial: u64, now: Timestamp) -> Vec<Output> {
    m.step(release_input(consumer(), lease(serial)), now)
}

fn tick(m: &mut PocketMachine, now: Timestamp) -> Vec<Output> {
    m.step(Input::Tick, now)
}

fn pocket(m: &PocketMachine) -> &Pocket {
    m.pocket(&key()).unwrap()
}

fn record<'a>(m: &'a PocketMachine, record: &str) -> &'a Record {
    pocket(m).records.get(&RecordKey::new(record)).unwrap()
}

fn the_lease(m: &PocketMachine, serial: u64) -> &Lease {
    m.lease(lease(serial)).unwrap().1
}

/// Pocket `k` with `r1` (rank 1, `a`) and `r2` (rank 2, `b`), both held by `c` under
/// lease 1 granted at t=0.
fn held_pocket() -> PocketMachine {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    publish(&mut m, "r2", 2, "b", ms(0));
    assert_eq!(
        claim(&mut m, 1, 10, ms(0)),
        batch(1, vec![handed("r1", 1, "a"), handed("r2", 2, "b")])
    );
    m
}

/// The held pocket after `r1` was superseded at rank 3 while held and the held version
/// acked, so `r1` is deferred and `r2` still held.
fn pocket_with_deferred_record() -> PocketMachine {
    let mut m = held_pocket();
    assert_eq!(
        publish(&mut m, "r1", 3, "a3", ms(1)),
        reply(Response::Published(Published::Superseded))
    );
    assert_eq!(ack(&mut m, 1, "r1", ms(2)), reply(Response::Acked));
    assert_eq!(record(&m, "r1").status, RecordStatus::Deferred);
    m
}

/// The held pocket after both records were acked: leased and empty.
fn empty_leased_pocket() -> PocketMachine {
    let mut m = held_pocket();
    assert_eq!(ack(&mut m, 1, "r1", ms(1)), reply(Response::Acked));
    assert_eq!(ack(&mut m, 1, "r2", ms(1)), reply(Response::Acked));
    assert!(pocket(&m).records.is_empty());
    m
}

fn only_reply_last(outputs: &[Output]) {
    let (last, rest) = outputs.split_last().unwrap();
    assert!(
        matches!(last, Output::Reply(_)),
        "last output is the reply: {outputs:?}"
    );
    assert!(
        rest.iter().all(|o| !matches!(o, Output::Reply(_))),
        "one reply, after every forward and notify: {outputs:?}"
    );
}

fn rejects_unknown_lease(action: fn(ConsumerId, LeaseId) -> Input) {
    let mut m = held_pocket();
    let before = m.clone();
    assert_eq!(
        m.step(action(consumer(), lease(9)), ms(1)),
        rejected(Reason::UnknownLease)
    );
    assert_eq!(m, before);
}

fn rejects_ended_lease(action: fn(ConsumerId, LeaseId) -> Input) {
    let mut m = held_pocket();
    release(&mut m, 1, ms(1));
    let before = m.clone();
    assert_eq!(
        m.step(action(consumer(), lease(1)), ms(2)),
        rejected(Reason::LeaseNotActive)
    );
    assert_eq!(m, before);
}

fn rejects_another_consumers_lease(action: fn(ConsumerId, LeaseId) -> Input) {
    let mut m = held_pocket();
    let before = m.clone();
    assert_eq!(
        m.step(action(ConsumerId::new("other"), lease(1)), ms(1)),
        rejected(Reason::NotYourLease)
    );
    assert_eq!(m, before);
}

// Publish ------------------------------------------------------------------

#[test]
fn publish_into_new_pocket_creates_a_queued_pocket_and_record_and_enqueues() {
    let mut m = machine();
    let outputs = publish(&mut m, "r1", 1, "a", ms(5));
    assert_eq!(
        outputs,
        vec![
            enqueue(),
            Output::Reply(Response::Published(Published::NewPocket))
        ]
    );
    let p = pocket(&m);
    assert_eq!(p.status, PocketStatus::Queued);
    assert_eq!(p.consecutive_expiries, 0);
    assert_eq!(
        record(&m, "r1"),
        &Record {
            key: RecordKey::new("r1"),
            rank: Rank::new(1),
            payload: Payload::new("a"),
            position: 0,
            status: RecordStatus::Queued,
            held_rank: None,
            held_payload: None,
        }
    );
    assert_eq!(m.next_record_serial(), 1);
    assert_eq!(m.next_record_position(), 2);
}

#[test]
fn publish_into_new_pocket_tags_record_positions_with_the_partition() {
    let mut m = PocketMachine::new(Partition::new(1), PARTITIONS, config());
    publish(&mut m, "r1", 1, "a", ms(0));
    publish(&mut m, "r2", 1, "b", ms(0));
    assert_eq!(record(&m, "r1").position, 1);
    assert_eq!(record(&m, "r2").position, 3);
}

#[test]
fn publish_new_key_adds_a_queued_record_at_a_fresh_position() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    let outputs = publish(&mut m, "r2", 7, "b", ms(1));
    assert_eq!(outputs, reply(Response::Published(Published::NewKey)));
    let r2 = record(&m, "r2");
    assert_eq!(
        (r2.rank, r2.position, r2.status),
        (Rank::new(7), 2, RecordStatus::Queued)
    );
    assert_eq!(r2.payload, Payload::new("b"));
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
}

#[test]
fn publish_new_key_into_a_leased_pocket_leaves_it_leased() {
    let mut m = held_pocket();
    assert_eq!(
        publish(&mut m, "r3", 1, "c", ms(1)),
        reply(Response::Published(Published::NewKey))
    );
    assert_eq!(pocket(&m).status, PocketStatus::Leased);
    assert_eq!(record(&m, "r3").status, RecordStatus::Queued);
}

#[test]
fn supersede_replaces_rank_and_payload_and_keeps_the_position() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    let outputs = publish(&mut m, "r1", 2, "a2", ms(3));
    assert_eq!(outputs, reply(Response::Published(Published::Superseded)));
    let r1 = record(&m, "r1");
    assert_eq!(
        (r1.rank, r1.payload.clone()),
        (Rank::new(2), Payload::new("a2"))
    );
    assert_eq!((r1.position, r1.status), (0, RecordStatus::Queued));
    assert_eq!(
        pocket(&m).superseded,
        vec![SupersededRecord {
            key: RecordKey::new("r1"),
            rank: Rank::new(1),
            payload: Payload::new("a"),
            reason: ShedReason::Superseded,
            shed_at: ms(3),
            expires_at: ms(3).plus(SHED_TTL),
        }]
    );
    assert_eq!(m.next_record_serial(), 1, "a supersede takes no position");
}

#[test]
fn supersede_keeps_the_held_version_of_a_held_record() {
    let mut m = held_pocket();
    publish(&mut m, "r1", 5, "a5", ms(1));
    let r1 = record(&m, "r1");
    assert_eq!(r1.status, RecordStatus::Held);
    assert_eq!((r1.rank, r1.held_rank), (Rank::new(5), Some(Rank::new(1))));
    assert_eq!(r1.held_payload, Some(Payload::new("a")));
}

#[test]
fn drop_stale_publish_sheds_the_new_version_and_keeps_the_record() {
    let mut m = machine();
    publish(&mut m, "r1", 5, "a", ms(0));
    let before = pocket(&m).records.clone();
    let outputs = publish(&mut m, "r1", 4, "old", ms(3));
    assert_eq!(outputs, reply(Response::Published(Published::Stale)));
    assert_eq!(pocket(&m).records, before);
    assert_eq!(
        pocket(&m).superseded,
        vec![SupersededRecord {
            key: RecordKey::new("r1"),
            rank: Rank::new(4),
            payload: Payload::new("old"),
            reason: ShedReason::Stale,
            shed_at: ms(3),
            expires_at: ms(3).plus(SHED_TTL),
        }]
    );
}

#[test]
fn publish_equal_rank_is_rejected_and_changes_nothing() {
    let mut m = empty_leased_pocket();
    release(&mut m, 1, ms(2));
    publish(&mut m, "r1", 1, "a", ms(3));
    tick(&mut m, ms(3).plus(LEASE_TTL));
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
    let before = m.clone();
    assert_eq!(
        publish(&mut m, "r1", 1, "other", ms(4)),
        rejected(Reason::EqualRank)
    );
    assert_eq!(m, before);
}

#[test]
fn publish_equal_rank_does_not_revive_a_parked_pocket() {
    // A parked pocket is the one revivable state that can hold a record, so it is where
    // "no rule covers rank = stored rank, hence no PocketTouched" is observable.
    let mut m = PocketMachine::new(
        PARTITION,
        PARTITIONS,
        config().with_max_consecutive_expiries(1),
    );
    publish(&mut m, "r1", 1, "a", ms(0));
    claim(&mut m, 1, 10, ms(0));
    tick(&mut m, ms(0).plus(LEASE_TTL));
    assert_eq!(pocket(&m).status, PocketStatus::Parked);
    let before = m.clone();
    assert_eq!(
        publish(&mut m, "r1", 1, "other", ms(4)),
        rejected(Reason::EqualRank)
    );
    assert_eq!(pocket(&m).status, PocketStatus::Parked);
    assert_eq!(m, before);
}

#[test]
fn revive_idle_pocket_requeues_and_asks_for_a_position() {
    let mut m = empty_leased_pocket();
    release(&mut m, 1, ms(2));
    assert_eq!(pocket(&m).status, PocketStatus::Idle);
    let outputs = publish(&mut m, "r3", 1, "c", ms(5));
    assert_eq!(
        outputs,
        vec![
            enqueue(),
            Output::Reply(Response::Published(Published::NewKey))
        ]
    );
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
    assert_eq!(pocket(&m).consecutive_expiries, 0);
}

#[test]
fn revive_parked_pocket_requeues_and_asks_for_a_position() {
    let mut m = PocketMachine::new(
        PARTITION,
        PARTITIONS,
        config().with_max_consecutive_expiries(1),
    );
    publish(&mut m, "r1", 1, "a", ms(0));
    claim(&mut m, 1, 10, ms(0));
    tick(&mut m, ms(0).plus(LEASE_TTL));
    assert_eq!(pocket(&m).status, PocketStatus::Parked);
    assert_eq!(pocket(&m).consecutive_expiries, 1);
    let outputs = publish(&mut m, "r1", 2, "a2", ms(5));
    assert_eq!(
        outputs,
        vec![
            enqueue(),
            Output::Reply(Response::Published(Published::Superseded))
        ]
    );
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
    assert_eq!(pocket(&m).consecutive_expiries, 0);
}

#[test]
fn revive_does_not_touch_a_queued_or_leased_pocket() {
    let mut m = machine();
    publish(&mut m, "r1", 2, "a", ms(0));
    assert_eq!(
        publish(&mut m, "r1", 1, "stale", ms(1)),
        reply(Response::Published(Published::Stale))
    );
    claim(&mut m, 1, 10, ms(1));
    assert_eq!(
        publish(&mut m, "r1", 3, "a3", ms(2)),
        reply(Response::Published(Published::Superseded))
    );
    assert_eq!(pocket(&m).status, PocketStatus::Leased);
}

// Claim --------------------------------------------------------------------

#[test]
fn pull_claims_front_pocket_grants_the_lease_and_hands_the_oldest_first() {
    let mut m = machine();
    publish(&mut m, "b", 1, "pb", ms(0));
    publish(&mut m, "a", 1, "pa", ms(0));
    publish(&mut m, "c", 1, "pc", ms(0));
    let outputs = claim(&mut m, 4, 2, ms(7));
    assert_eq!(
        outputs,
        batch(4, vec![handed("b", 1, "pb"), handed("a", 1, "pa")])
    );
    assert_eq!(pocket(&m).status, PocketStatus::Leased);
    assert_eq!(
        the_lease(&m, 4),
        &Lease {
            id: lease(4),
            consumer: consumer(),
            status: LeaseStatus::Active,
            granted_at: ms(7),
            expires_at: ms(7).plus(LEASE_TTL),
            ended_at: None,
        }
    );
    assert_eq!(record(&m, "b").status, RecordStatus::Held);
    assert_eq!(record(&m, "a").status, RecordStatus::Held);
    assert_eq!(record(&m, "c").status, RecordStatus::Queued);
    assert_eq!(m.lease(lease(4)).unwrap().0, &key());
}

#[test]
fn pull_claims_front_pocket_caps_the_batch_at_max_pull_batch() {
    let mut m = machine();
    for r in ["r1", "r2", "r3", "r4"] {
        publish(&mut m, r, 1, "p", ms(0));
    }
    let outputs = claim(&mut m, 1, 10, ms(0));
    assert_eq!(
        outputs,
        batch(
            1,
            vec![
                handed("r1", 1, "p"),
                handed("r2", 1, "p"),
                handed("r3", 1, "p")
            ]
        )
    );
    assert_eq!(record(&m, "r4").status, RecordStatus::Queued);
}

#[test]
fn pull_claims_front_pocket_with_a_zero_limit_hands_nothing() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    assert_eq!(claim(&mut m, 1, 0, ms(0)), batch(1, vec![]));
    assert_eq!(pocket(&m).status, PocketStatus::Leased);
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Active);
    assert_eq!(record(&m, "r1").status, RecordStatus::Queued);
}

#[test]
fn hold_record_copies_the_stored_version_into_the_held_one() {
    let m = held_pocket();
    let r2 = record(&m, "r2");
    assert_eq!(r2.status, RecordStatus::Held);
    assert_eq!(r2.held_rank, Some(r2.rank));
    assert_eq!(r2.held_payload, Some(r2.payload.clone()));
}

#[test]
fn pull_claims_front_pocket_refused_when_the_pocket_is_absent() {
    let mut m = machine();
    assert_eq!(claim(&mut m, 1, 10, ms(0)), rejected(Reason::Protocol));
    assert_eq!(m.violations(), 1);
    assert!(m.lease(lease(1)).is_none());
}

#[test]
fn pull_claims_front_pocket_refused_when_the_pocket_is_not_queued() {
    let mut m = held_pocket();
    let before = m.clone();
    assert_eq!(
        claim_by(&mut m, "d", 2, 10, ms(1)),
        rejected(Reason::Protocol)
    );
    assert_eq!(m.violations(), 1);
    assert!(m.lease(lease(2)).is_none());
    assert_eq!(pocket(&m), pocket(&before));
}

// Pull under lease ---------------------------------------------------------

#[test]
fn pull_within_lease_extends_the_lease_and_hands_the_remaining_queued_records() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    publish(&mut m, "r2", 1, "b", ms(0));
    publish(&mut m, "r3", 1, "c", ms(0));
    claim(&mut m, 1, 1, ms(0));
    let outputs = pull(&mut m, 1, 10, ms(40));
    assert_eq!(
        outputs,
        batch(1, vec![handed("r2", 1, "b"), handed("r3", 1, "c")])
    );
    assert_eq!(the_lease(&m, 1).expires_at, ms(40).plus(LEASE_TTL));
    assert_eq!(record(&m, "r3").status, RecordStatus::Held);
}

#[test]
fn pull_within_lease_skips_held_and_deferred_records() {
    let mut m = pocket_with_deferred_record();
    assert_eq!(pull(&mut m, 1, 10, ms(3)), batch(1, vec![]));
    assert_eq!(record(&m, "r1").status, RecordStatus::Deferred);
    assert_eq!(record(&m, "r2").status, RecordStatus::Held);
}

#[test]
fn pull_within_lease_with_a_zero_limit_hands_nothing_but_extends() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    claim(&mut m, 1, 0, ms(0));
    assert_eq!(pull(&mut m, 1, 0, ms(9)), batch(1, vec![]));
    assert_eq!(the_lease(&m, 1).expires_at, ms(9).plus(LEASE_TTL));
}

#[test]
fn pull_within_lease_refused_when_the_lease_is_unknown() {
    let mut m = held_pocket();
    assert_eq!(pull(&mut m, 9, 10, ms(1)), rejected(Reason::Protocol));
    assert_eq!(m.violations(), 1);
}

#[test]
fn pull_within_lease_forwards_a_retry_when_the_lease_has_ended() {
    let mut m = held_pocket();
    release(&mut m, 1, ms(1));
    let outputs = pull(&mut m, 1, 5, ms(2));
    assert_eq!(
        outputs,
        vec![Output::Forward(
            MachineId::Order,
            Input::RetryPull {
                consumer: consumer(),
                limit: 5
            }
        )]
    );
    assert_eq!(m.violations(), 0);
}

#[test]
fn pull_within_lease_refused_when_the_consumer_differs() {
    let mut m = held_pocket();
    let outputs = m.step(
        Input::PullUnderLease {
            consumer: ConsumerId::new("other"),
            lease: lease(1),
            limit: 10,
        },
        ms(1),
    );
    assert_eq!(outputs, rejected(Reason::Protocol));
    assert_eq!(m.violations(), 1);
}

// Ack ----------------------------------------------------------------------

#[test]
fn ack_settles_record_removes_it_and_extends_the_lease() {
    let mut m = held_pocket();
    assert_eq!(ack(&mut m, 1, "r1", ms(30)), reply(Response::Acked));
    assert!(!pocket(&m).records.contains_key(&RecordKey::new("r1")));
    assert_eq!(record(&m, "r2").status, RecordStatus::Held);
    assert_eq!(the_lease(&m, 1).expires_at, ms(30).plus(LEASE_TTL));
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Active);
    assert_eq!(pocket(&m).dead_letters, Vec::new());
}

#[test]
fn ack_defers_newer_version_restarts_the_record_at_the_back() {
    let mut m = held_pocket();
    publish(&mut m, "r1", 3, "a3", ms(1));
    let fresh = m.next_record_position();
    assert_eq!(ack(&mut m, 1, "r1", ms(30)), reply(Response::Acked));
    assert_eq!(
        record(&m, "r1"),
        &Record {
            key: RecordKey::new("r1"),
            rank: Rank::new(3),
            payload: Payload::new("a3"),
            position: fresh,
            status: RecordStatus::Deferred,
            held_rank: None,
            held_payload: None,
        }
    );
    assert_eq!(m.next_record_position(), fresh + u64::from(PARTITIONS));
    assert_eq!(the_lease(&m, 1).expires_at, ms(30).plus(LEASE_TTL));
}

#[test]
fn ack_refused_when_the_lease_is_unknown() {
    rejects_unknown_lease(ack_input);
}

#[test]
fn ack_refused_when_the_lease_has_ended() {
    rejects_ended_lease(ack_input);
}

#[test]
fn ack_refused_when_the_lease_is_another_consumers() {
    rejects_another_consumers_lease(ack_input);
}

#[test]
fn ack_refused_when_the_record_is_absent() {
    let mut m = held_pocket();
    let before = m.clone();
    assert_eq!(
        ack(&mut m, 1, "nope", ms(1)),
        rejected(Reason::NoSuchRecord)
    );
    assert_eq!(m, before);
}

#[test]
fn ack_refused_when_the_record_is_not_held() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    publish(&mut m, "r2", 1, "b", ms(0));
    claim(&mut m, 1, 1, ms(0));
    let before = m.clone();
    assert_eq!(ack(&mut m, 1, "r2", ms(1)), rejected(Reason::RecordNotHeld));
    assert_eq!(m, before);
}

#[test]
fn ack_naming_an_ended_lease_is_rejected_despite_a_successor_lease() {
    // The fencing token: the same consumer, the same pocket, the same key at a newer
    // rank under lease 2; an ack naming lease 1 lands nowhere.
    let mut m = held_pocket();
    release(&mut m, 1, ms(1));
    publish(&mut m, "r1", 2, "a2", ms(2));
    assert_eq!(
        claim(&mut m, 2, 10, ms(3)),
        batch(2, vec![handed("r1", 2, "a2"), handed("r2", 2, "b")])
    );
    let before = m.clone();
    assert_eq!(
        ack(&mut m, 1, "r1", ms(4)),
        rejected(Reason::LeaseNotActive)
    );
    assert_eq!(m, before);
    assert_eq!(record(&m, "r1").status, RecordStatus::Held);
    assert_eq!(record(&m, "r1").held_rank, Some(Rank::new(2)));
    assert_eq!(the_lease(&m, 2).status, LeaseStatus::Active);
    assert_eq!(ack(&mut m, 2, "r1", ms(5)), reply(Response::Acked));
}

#[test]
fn ack_refused_as_a_violation_when_the_held_version_is_newer() {
    // `HeldVersionNotNewer` rules this out; the state is forced to reach the branch.
    let mut m = held_pocket();
    let r1 = m
        .pockets
        .get_mut(&key())
        .unwrap()
        .records
        .get_mut(&RecordKey::new("r1"))
        .unwrap();
    r1.held_rank = Some(Rank::new(9));
    assert_eq!(ack(&mut m, 1, "r1", ms(1)), rejected(Reason::Protocol));
    assert_eq!(m.violations(), 1);
    assert_eq!(record(&m, "r1").status, RecordStatus::Held);
}

// Nack ---------------------------------------------------------------------

#[test]
fn nack_settles_record_dead_letters_it_and_removes_it() {
    let mut m = held_pocket();
    assert_eq!(nack(&mut m, 1, "r1", ms(30)), reply(Response::Nacked));
    assert!(!pocket(&m).records.contains_key(&RecordKey::new("r1")));
    assert_eq!(
        pocket(&m).dead_letters,
        vec![DeadLetter {
            key: RecordKey::new("r1"),
            rank: Rank::new(1),
            payload: Payload::new("a"),
            nacked_by: consumer(),
            nacked_at: ms(30),
            expires_at: ms(30).plus(SHED_TTL),
        }]
    );
    assert_eq!(the_lease(&m, 1).expires_at, ms(30).plus(LEASE_TTL));
}

#[test]
fn nack_defers_newer_version_dead_letters_the_held_version() {
    let mut m = held_pocket();
    publish(&mut m, "r1", 3, "a3", ms(1));
    let fresh = m.next_record_position();
    assert_eq!(nack(&mut m, 1, "r1", ms(30)), reply(Response::Nacked));
    assert_eq!(
        pocket(&m).dead_letters,
        vec![DeadLetter {
            key: RecordKey::new("r1"),
            rank: Rank::new(1),
            payload: Payload::new("a"),
            nacked_by: consumer(),
            nacked_at: ms(30),
            expires_at: ms(30).plus(SHED_TTL),
        }]
    );
    let r1 = record(&m, "r1");
    assert_eq!((r1.status, r1.position), (RecordStatus::Deferred, fresh));
    assert_eq!(
        (r1.rank, r1.held_rank, r1.held_payload.clone()),
        (Rank::new(3), None, None)
    );
    assert_eq!(the_lease(&m, 1).expires_at, ms(30).plus(LEASE_TTL));
}

#[test]
fn nack_refused_when_the_lease_is_unknown() {
    rejects_unknown_lease(nack_input);
}

#[test]
fn nack_refused_when_the_lease_has_ended() {
    rejects_ended_lease(nack_input);
}

#[test]
fn nack_refused_when_the_lease_is_another_consumers() {
    rejects_another_consumers_lease(nack_input);
}

#[test]
fn nack_refused_when_the_record_is_absent() {
    let mut m = held_pocket();
    let before = m.clone();
    assert_eq!(
        nack(&mut m, 1, "nope", ms(1)),
        rejected(Reason::NoSuchRecord)
    );
    assert_eq!(m, before);
}

#[test]
fn nack_refused_when_the_record_is_not_held() {
    let mut m = pocket_with_deferred_record();
    let before = m.clone();
    assert_eq!(
        nack(&mut m, 1, "r1", ms(3)),
        rejected(Reason::RecordNotHeld)
    );
    assert_eq!(m, before);
}

// Renew --------------------------------------------------------------------

#[test]
fn renew_extends_the_lease() {
    let mut m = held_pocket();
    let expires_at = ms(60).plus(LEASE_TTL);
    assert_eq!(
        renew(&mut m, 1, ms(60)),
        reply(Response::Renewed { expires_at })
    );
    assert_eq!(the_lease(&m, 1).expires_at, expires_at);
    assert_eq!(the_lease(&m, 1).granted_at, ms(0));
}

#[test]
fn renew_refused_when_the_lease_is_unknown() {
    rejects_unknown_lease(renew_input);
}

#[test]
fn renew_refused_when_the_lease_has_ended() {
    rejects_ended_lease(renew_input);
}

#[test]
fn renew_refused_when_the_lease_is_another_consumers() {
    rejects_another_consumers_lease(renew_input);
}

// Release ------------------------------------------------------------------

#[test]
fn release_requeues_pocket_ends_the_lease_and_frees_the_records() {
    let mut m = pocket_with_deferred_record();
    let outputs = release(&mut m, 1, ms(9));
    assert_eq!(
        outputs,
        vec![
            became(Became::RequeuedAtBack, 1),
            Output::Reply(Response::Released)
        ]
    );
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Released);
    assert_eq!(the_lease(&m, 1).ended_at, Some(ms(9)));
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
    assert_eq!(pocket(&m).consecutive_expiries, 0);
    assert!(pocket(&m).active_lease().is_none());
}

#[test]
fn requeue_held_record_clears_the_held_version() {
    let mut m = held_pocket();
    release(&mut m, 1, ms(9));
    let r2 = record(&m, "r2");
    assert_eq!(r2.status, RecordStatus::Queued);
    assert_eq!((r2.held_rank, r2.held_payload.clone()), (None, None));
    assert_eq!(r2.position, 2, "the position is kept");
}

#[test]
fn requeue_deferred_record_keeps_the_deferred_position() {
    let mut m = pocket_with_deferred_record();
    let deferred_position = record(&m, "r1").position;
    release(&mut m, 1, ms(9));
    let r1 = record(&m, "r1");
    assert_eq!(r1.status, RecordStatus::Queued);
    assert_eq!(r1.position, deferred_position);
    assert_eq!(r1.rank, Rank::new(3));
}

#[test]
fn release_requeues_pocket_resets_the_expiry_count() {
    let mut m = held_pocket();
    tick(&mut m, ms(0).plus(LEASE_TTL));
    assert_eq!(pocket(&m).consecutive_expiries, 1);
    claim(&mut m, 2, 10, ms(200));
    release(&mut m, 2, ms(201));
    assert_eq!(pocket(&m).consecutive_expiries, 0);
}

#[test]
fn release_idles_empty_pocket_ends_the_lease_and_parks_nothing() {
    let mut m = empty_leased_pocket();
    let outputs = release(&mut m, 1, ms(9));
    assert_eq!(
        outputs,
        vec![became(Became::Idle, 1), Output::Reply(Response::Released)]
    );
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Released);
    assert_eq!(the_lease(&m, 1).ended_at, Some(ms(9)));
    assert_eq!(pocket(&m).status, PocketStatus::Idle);
    assert_eq!(pocket(&m).consecutive_expiries, 0);
}

#[test]
fn release_refused_when_the_lease_is_unknown() {
    rejects_unknown_lease(release_input);
}

#[test]
fn release_refused_when_the_lease_has_ended() {
    rejects_ended_lease(release_input);
}

#[test]
fn release_refused_when_the_lease_is_another_consumers() {
    rejects_another_consumers_lease(release_input);
}

#[test]
fn release_refused_when_the_pocket_is_not_leased() {
    // `LeasedHasOneActiveLease` rules this out; the state is forced to reach the guard.
    let mut m = held_pocket();
    m.pockets.get_mut(&key()).unwrap().status = PocketStatus::Queued;
    let before = m.clone();
    assert_eq!(release(&mut m, 1, ms(1)), rejected(Reason::PocketNotLeased));
    assert_eq!(m, before);
    assert_eq!(m.violations(), 0);
}

// Tick ---------------------------------------------------------------------

#[test]
fn lease_lapses_fires_only_on_a_tick() {
    let mut m = held_pocket();
    let late = ms(0).plus(LEASE_TTL).plus(Duration::from_millis(50));
    assert_eq!(ack(&mut m, 1, "r1", late), reply(Response::Acked));
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Active);
    assert_eq!(the_lease(&m, 1).expires_at, late.plus(LEASE_TTL));
    assert_eq!(tick(&mut m, late), vec![]);
    assert_eq!(
        tick(&mut m, late.plus(LEASE_TTL)),
        vec![became(Became::RequeuedKeepingPosition, 1)]
    );
}

#[test]
fn lease_lapses_at_expires_at_and_not_before() {
    let mut m = held_pocket();
    let expires_at = ms(0).plus(LEASE_TTL);
    assert_eq!(
        tick(&mut m, Timestamp::from_nanos(expires_at.nanos() - 1)),
        vec![]
    );
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Active);
    assert_eq!(
        tick(&mut m, expires_at),
        vec![became(Became::RequeuedKeepingPosition, 1)]
    );
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Expired);
}

#[test]
fn lease_expiry_keeps_pocket_at_front_and_frees_the_records() {
    let mut m = pocket_with_deferred_record();
    let now = ms(2).plus(LEASE_TTL);
    assert_eq!(
        tick(&mut m, now),
        vec![became(Became::RequeuedKeepingPosition, 1)]
    );
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Expired);
    assert_eq!(the_lease(&m, 1).ended_at, Some(now));
    assert_eq!(pocket(&m).status, PocketStatus::Queued);
    assert_eq!(pocket(&m).consecutive_expiries, 1);
    assert_eq!(record(&m, "r1").status, RecordStatus::Queued);
    assert_eq!(record(&m, "r2").status, RecordStatus::Queued);
    assert_eq!(record(&m, "r2").held_rank, None);
}

#[test]
fn lease_expiry_parks_pocket_at_the_threshold() {
    let mut m = PocketMachine::new(
        PARTITION,
        PARTITIONS,
        config().with_max_consecutive_expiries(2),
    );
    publish(&mut m, "r1", 1, "a", ms(0));
    claim(&mut m, 1, 10, ms(0));
    assert_eq!(
        tick(&mut m, ms(100)),
        vec![became(Became::RequeuedKeepingPosition, 1)]
    );
    claim(&mut m, 2, 10, ms(100));
    assert_eq!(tick(&mut m, ms(200)), vec![became(Became::Parked, 2)]);
    assert_eq!(the_lease(&m, 2).status, LeaseStatus::Expired);
    assert_eq!(the_lease(&m, 2).ended_at, Some(ms(200)));
    assert_eq!(pocket(&m).status, PocketStatus::Parked);
    assert_eq!(pocket(&m).consecutive_expiries, 2);
    assert_eq!(record(&m, "r1").status, RecordStatus::Queued);
    assert_eq!(record(&m, "r1").held_rank, None);
}

#[test]
fn lease_expiry_parks_an_empty_pocket_at_the_threshold() {
    let mut m = PocketMachine::new(
        PARTITION,
        PARTITIONS,
        config().with_max_consecutive_expiries(1),
    );
    publish(&mut m, "r1", 1, "a", ms(0));
    claim(&mut m, 1, 10, ms(0));
    ack(&mut m, 1, "r1", ms(1));
    assert_eq!(tick(&mut m, ms(101)), vec![became(Became::Parked, 1)]);
    assert_eq!(pocket(&m).status, PocketStatus::Parked);
    assert_eq!(pocket(&m).consecutive_expiries, 1);
}

#[test]
fn lease_expiry_idles_empty_pocket_below_the_threshold() {
    let mut m = empty_leased_pocket();
    let now = ms(1).plus(LEASE_TTL);
    assert_eq!(tick(&mut m, now), vec![became(Became::Idle, 1)]);
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Expired);
    assert_eq!(the_lease(&m, 1).ended_at, Some(now));
    assert_eq!(pocket(&m).status, PocketStatus::Idle);
    assert_eq!(pocket(&m).consecutive_expiries, 1);
}

#[test]
fn lease_expiry_refused_as_a_violation_when_the_pocket_is_not_leased() {
    // `LeasedHasOneActiveLease` rules this out; the state is forced to reach the guard.
    let mut m = held_pocket();
    m.pockets.get_mut(&key()).unwrap().status = PocketStatus::Queued;
    assert_eq!(tick(&mut m, ms(100)), vec![]);
    assert_eq!(m.violations(), 1);
    assert_eq!(the_lease(&m, 1).status, LeaseStatus::Active);
}

#[test]
fn tick_produces_no_reply_and_visits_pockets_in_key_order() {
    let mut m = machine();
    for (pocket_key, serial) in [("b", 1), ("a", 2)] {
        m.step(
            Input::Publish {
                publisher: PublisherId::new("p"),
                pocket_key: PocketKey::new(pocket_key),
                record_key: RecordKey::new("r"),
                rank: Rank::new(1),
                payload: Payload::new("x"),
            },
            ms(0),
        );
        m.step(
            Input::Claim {
                pocket_key: PocketKey::new(pocket_key),
                consumer: ConsumerId::new(pocket_key),
                lease: lease(serial),
                limit: 1,
            },
            ms(0),
        );
    }
    let outputs = tick(&mut m, ms(100));
    assert_eq!(outputs.len(), 2);
    assert!(outputs.iter().all(|o| matches!(o, Output::Notify(..))));
    let keys: Vec<&str> = outputs
        .iter()
        .map(|o| match o {
            Output::Notify(_, Input::PocketBecame { pocket_key, .. }) => pocket_key.as_str(),
            other => panic!("not a PocketBecame: {other:?}"),
        })
        .collect();
    assert_eq!(keys, vec!["a", "b"]);
}

#[test]
fn superseded_record_decays_at_its_expiry() {
    let mut m = machine();
    publish(&mut m, "r1", 1, "a", ms(0));
    publish(&mut m, "r1", 2, "a2", ms(10));
    let expires_at = ms(10).plus(SHED_TTL);
    tick(&mut m, Timestamp::from_nanos(expires_at.nanos() - 1));
    assert_eq!(pocket(&m).superseded.len(), 1);
    assert_eq!(tick(&mut m, expires_at), vec![]);
    assert_eq!(pocket(&m).superseded, Vec::new());
    assert_eq!(record(&m, "r1").rank, Rank::new(2));
}

#[test]
fn dead_letter_decays_at_its_expiry() {
    let mut m = held_pocket();
    nack(&mut m, 1, "r1", ms(10));
    release(&mut m, 1, ms(10));
    let expires_at = ms(10).plus(SHED_TTL);
    tick(&mut m, Timestamp::from_nanos(expires_at.nanos() - 1));
    assert_eq!(pocket(&m).dead_letters.len(), 1);
    assert_eq!(tick(&mut m, expires_at), vec![]);
    assert_eq!(pocket(&m).dead_letters, Vec::new());
}

// Protocol -----------------------------------------------------------------

#[test]
fn unexpected_inputs_are_counted_and_rejected_only_under_a_reply_handle() {
    let mut m = machine();
    let pull = Input::Pull {
        consumer: consumer(),
        limit: 1,
    };
    assert_eq!(m.step(pull, ms(0)), rejected(Reason::Protocol));
    let enqueue = Input::Enqueue {
        pocket_key: key(),
        partition: PARTITION,
    };
    assert_eq!(m.step(enqueue, ms(0)), vec![]);
    assert_eq!(m.violations(), 2);
    assert_eq!(m.id(), MachineId::Partition(PARTITION));
}

#[test]
fn every_forward_and_notify_comes_before_the_one_reply() {
    let mut m = empty_leased_pocket();
    only_reply_last(&release(&mut m, 1, ms(1)));
    only_reply_last(&publish(&mut m, "r1", 1, "a", ms(2)));
    only_reply_last(&claim(&mut m, 2, 10, ms(3)));
    only_reply_last(&pull(&mut m, 2, 10, ms(3)));
    only_reply_last(&renew(&mut m, 2, ms(3)));
    only_reply_last(&nack(&mut m, 2, "r1", ms(3)));
    only_reply_last(&release(&mut m, 2, ms(4)));
}
