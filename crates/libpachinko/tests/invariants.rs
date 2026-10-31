//! The module's invariants and the `Consuming` surface's guarantees over random
//! action sequences, the transition graphs, the entities' fields, relationships and
//! projections, and T19's trace replayed as one scenario.
//!
//! The walk is a proptest state machine over small domains (two publishers, three
//! consumers, three pockets, three keys, ranks 0 to 3, limits -1 to 3, clock steps
//! forwards and backwards around `lease_ttl`). Before each step it predicts the
//! step's outcome from the previous snapshot, following each rule's guards; after
//! each step it checks every invariant, every transition against the declared graph,
//! and `OldestFirst`, `ExclusivePocket` and `OneVisitPerKeyPerLease` against the
//! previous snapshot. The runner is seeded, so a run is reproducible, and it persists
//! no failures to disk.

// Test-only from the root, so clippy treats the helpers below as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use libpachinko::{
    Batch, Clock, Config, ConsumerId, CountingLeaseIds, Error, FakeClock, LeaseId, LeaseStatus,
    LeaseView, Payload, PocketFate, PocketKey, PocketStatus, PocketView, Position, Publication,
    PublishOutcome, PublisherId, PullOutcome, Queue, Rank, RecordKey, RecordStatus, RecordView,
    SettleOutcome, ShedReason, Snapshot, Timestamp,
};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};

type TestQueue = Queue<FakeClock, CountingLeaseIds>;

const LEASE_TTL: Duration = Duration::from_millis(100);
const MAX_CONSECUTIVE_EXPIRIES: u32 = 2;
const MAX_PULL_BATCH: u32 = 2;
const SUPERSEDED_TTL: Duration = Duration::from_millis(150);
const DEAD_LETTER_TTL: Duration = Duration::from_millis(250);
const PUBLISHERS: [&str; 2] = ["p0", "p1"];
const CONSUMERS: [&str; 3] = ["c0", "c1", "c2"];
const POCKETS: [&str; 3] = ["a", "b", "c"];
const KEYS: [&str; 3] = ["k0", "k1", "k2"];
/// A lease id the counting source reaches only after this many claims.
const NEVER_ISSUED: LeaseId = LeaseId::from_u128(1 << 64);

/// Which lease an ack, nack, renew or release names.
#[derive(Clone, Copy, Debug)]
enum LeaseChoice {
    /// The consumer's active lease, if it holds one.
    Own,
    /// Any lease handed out so far, active or ended, mine or another's.
    Issued(u8),
    /// One never handed out.
    Never,
}

/// One action of the walk. Indices pick from the small domains above.
#[derive(Clone, Copy, Debug)]
enum Step {
    PublisherJoins(u8),
    ConsumerJoins(u8),
    Publish {
        publisher: u8,
        pocket: u8,
        key: u8,
        rank: i64,
    },
    Pull {
        consumer: u8,
        limit: i64,
    },
    Ack {
        consumer: u8,
        lease: LeaseChoice,
        key: u8,
    },
    Nack {
        consumer: u8,
        lease: LeaseChoice,
        key: u8,
    },
    Renew {
        consumer: u8,
        lease: LeaseChoice,
    },
    Release {
        consumer: u8,
        lease: LeaseChoice,
    },
    /// The clock moves by this many milliseconds; back, as a system clock can, when
    /// negative, which the queue's `now` must not.
    Advance(i64),
    Lapse,
    Decay,
}

fn lease_choice() -> impl Strategy<Value = LeaseChoice> {
    prop_oneof![
        6 => Just(LeaseChoice::Own),
        2 => any::<u8>().prop_map(LeaseChoice::Issued),
        1 => Just(LeaseChoice::Never),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        1 => (0..2_u8).prop_map(Step::PublisherJoins),
        1 => (0..3_u8).prop_map(Step::ConsumerJoins),
        8 => (0..2_u8, 0..3_u8, 0..3_u8, 0..4_i64).prop_map(|(publisher, pocket, key, rank)| {
            Step::Publish { publisher, pocket, key, rank }
        }),
        6 => (0..3_u8, -1..=3_i64).prop_map(|(consumer, limit)| Step::Pull { consumer, limit }),
        4 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| Step::Ack { consumer, lease, key }),
        3 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| Step::Nack { consumer, lease, key }),
        2 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| Step::Renew { consumer, lease }),
        2 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| Step::Release { consumer, lease }),
        5 => (0..=120_i64).prop_map(Step::Advance),
        1 => (-120..=0_i64).prop_map(Step::Advance),
        2 => Just(Step::Lapse),
        1 => Just(Step::Decay),
    ]
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    proptest::collection::vec(step(), 1..80)
}

fn runner(cases: u32) -> TestRunner {
    let config = ProptestConfig {
        cases,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

fn config() -> Config {
    Config::new(
        LEASE_TTL,
        MAX_CONSECUTIVE_EXPIRIES,
        MAX_PULL_BATCH,
        SUPERSEDED_TTL,
        DEAD_LETTER_TTL,
    )
    .unwrap()
}

/// A queue with p0, c0 and c1 joined; p1 and c2 join only if the walk says so.
fn start() -> (TestQueue, FakeClock) {
    let clock = FakeClock::new(Timestamp::from_millis(1_000));
    let mut queue = Queue::new(config(), clock.clone(), CountingLeaseIds::default());
    queue.publisher_joins(PublisherId::new("p0")).unwrap();
    queue.consumer_joins(ConsumerId::new("c0")).unwrap();
    queue.consumer_joins(ConsumerId::new("c1")).unwrap();
    (queue, clock)
}

/// What the walk remembers between steps beyond the queue: the keys each lease has
/// handed out, for `OneVisitPerKeyPerLease`, the latest time any operation has read,
/// which is the queue's `now` however far the clock steps back, whether the clock is
/// behind it for the current step, and which situations it has reached.
#[derive(Debug, Default)]
struct Walk {
    handed: BTreeMap<LeaseId, BTreeSet<(PocketKey, RecordKey)>>,
    now: Timestamp,
    behind: bool,
    reached: BTreeSet<String>,
}

impl Walk {
    fn reach(&mut self, what: impl Into<String>) {
        self.reached.insert(what.into());
    }
}

fn pocket_view<'a>(snapshot: &'a Snapshot, key: &PocketKey) -> Option<&'a PocketView> {
    snapshot.pockets.iter().find(|p| &p.key == key)
}

fn record_view<'a>(pocket: &'a PocketView, key: &RecordKey) -> Option<&'a RecordView> {
    pocket.records.iter().find(|r| &r.key == key)
}

fn active_lease_of<'a>(snapshot: &'a Snapshot, consumer: &ConsumerId) -> Option<&'a LeaseView> {
    snapshot
        .leases
        .iter()
        .find(|l| &l.consumer == consumer && l.status == LeaseStatus::Active)
}

fn resolve(snapshot: &Snapshot, consumer: &ConsumerId, choice: LeaseChoice) -> LeaseId {
    match choice {
        LeaseChoice::Own => active_lease_of(snapshot, consumer).map_or(NEVER_ISSUED, |l| l.id),
        LeaseChoice::Issued(n) if !snapshot.leases.is_empty() => {
            snapshot.leases[usize::from(n) % snapshot.leases.len()].id
        }
        LeaseChoice::Issued(_) | LeaseChoice::Never => NEVER_ISSUED,
    }
}

/// The records a pull should hand out of `pocket`: its queued records by position,
/// at most `min(limit, max_pull_batch)`, none for a limit that is not positive.
fn oldest(pocket: &PocketView, limit: i64) -> Vec<(RecordKey, Rank, Payload)> {
    let n = usize::try_from(limit.clamp(0, i64::from(MAX_PULL_BATCH))).unwrap();
    let mut queued: Vec<&RecordView> = pocket
        .records
        .iter()
        .filter(|r| r.status == RecordStatus::Queued)
        .collect();
    queued.sort_by_key(|r| r.position);
    queued
        .into_iter()
        .take(n)
        .map(|r| (r.key.clone(), r.rank, r.payload.clone()))
        .collect()
}

fn publish(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk, args: (u8, u8, u8, i64)) {
    let (publisher, pocket, key, rank) = args;
    let publication = Publication::new(
        PublisherId::new(PUBLISHERS[usize::from(publisher)]),
        PocketKey::new(POCKETS[usize::from(pocket)]),
        RecordKey::new(KEYS[usize::from(key)]),
        Rank::new(rank),
        Payload::from(format!("{publisher}{pocket}{key}{rank}").into_bytes()),
    );
    let stored = pocket_view(pre, &publication.pocket_key).map(|p| {
        (
            p.status,
            record_view(p, &publication.record_key).map(|r| r.rank),
        )
    });
    let expected = if pre.publishers.contains(&publication.publisher) {
        match stored {
            None => Ok(PublishOutcome::Created),
            Some((_, None)) => Ok(PublishOutcome::NewKey),
            Some((_, Some(r))) if rank > r.get() => Ok(PublishOutcome::Superseded),
            Some((_, Some(r))) if rank < r.get() => Ok(PublishOutcome::Stale),
            Some((_, Some(r))) => Err(Error::EqualRank {
                pocket_key: publication.pocket_key.clone(),
                record_key: publication.record_key.clone(),
                rank: r,
            }),
        }
    } else {
        Err(Error::UnknownPublisher(publication.publisher.clone()))
    };
    let got = queue.publish(publication.clone());
    assert_eq!(got, expected);
    walk.reach(format!("publish {got:?}"));
    if let (Ok(_), Some((status @ (PocketStatus::Idle | PocketStatus::Parked), _))) = (&got, stored)
    {
        let post = queue.snapshot();
        let revived = pocket_view(&post, &publication.pocket_key).unwrap();
        assert_eq!(
            revived.status,
            PocketStatus::Queued,
            "revived from {status:?}"
        );
        assert_eq!(revived.consecutive_expiries, 0);
        assert_eq!(revived.position.get() + 1, post.next_position.get());
        walk.reach(format!("revive {status:?}"));
    }
}

fn pull(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk, args: (u8, i64)) {
    let (consumer, limit) = args;
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let got = queue.pull(&consumer, limit);
    if !pre.consumers.contains(&consumer) {
        assert_eq!(got, Err(Error::UnknownConsumer(consumer)));
        return;
    }
    let held = active_lease_of(pre, &consumer);
    let front = pre.pocket_order.first();
    let Ok(PullOutcome::Batch(batch)) = got else {
        assert_eq!(got, Ok(PullOutcome::NoPocketAvailable));
        assert!(
            held.is_none() && front.is_none(),
            "PullFindsNoPocket's guards"
        );
        walk.reach("pull NoPocketAvailable");
        return;
    };
    let pocket_key = held.map_or_else(|| front.unwrap().clone(), |l| l.pocket_key.clone());
    match held {
        Some(lease) => assert_eq!(batch.lease_id, lease.id, "PullWithinLease"),
        None => assert!(
            pre.leases.iter().all(|l| l.id != batch.lease_id),
            "a fresh id"
        ),
    }
    assert_eq!(batch.pocket_key, pocket_key);
    check_batch(pre, walk, &consumer, &batch, limit);
    assert_eq!(
        batch.expires_at,
        walk.now + LEASE_TTL,
        "the lease runs from this pull"
    );
    walk.reach(if held.is_some() {
        "pull within lease"
    } else {
        "pull claims"
    });
}

/// `OldestFirst`, `ExclusivePocket` and `OneVisitPerKeyPerLease` for one batch.
fn check_batch(pre: &Snapshot, walk: &mut Walk, consumer: &ConsumerId, batch: &Batch, limit: i64) {
    let pocket_key = &batch.pocket_key;
    let pocket = pocket_view(pre, pocket_key).unwrap();
    let got: Vec<_> = batch
        .records
        .iter()
        .map(|r| (r.record_key.clone(), r.rank, r.payload.clone()))
        .collect();
    assert_eq!(got, oldest(pocket, limit), "OldestFirst");
    let others = pre
        .leases
        .iter()
        .filter(|l| l.status == LeaseStatus::Active && &l.consumer != consumer);
    assert!(
        others.clone().all(|l| &l.pocket_key != pocket_key),
        "ExclusivePocket"
    );
    let handed = walk.handed.entry(batch.lease_id).or_default();
    for record in &batch.records {
        let visit = (pocket_key.clone(), record.record_key.clone());
        assert!(handed.insert(visit), "OneVisitPerKeyPerLease: {record:?}");
    }
    if batch.records.len() > 1 {
        walk.reach("batch of several");
    }
}

/// The error an ack, nack, renew or release should meet, in the rules' order.
fn lease_error(pre: &Snapshot, consumer: &ConsumerId, lease: LeaseId) -> Option<Error> {
    match pre.leases.iter().find(|l| l.id == lease) {
        Some(l) if l.status == LeaseStatus::Active && &l.consumer == consumer => None,
        Some(l) if l.status == LeaseStatus::Active => Some(Error::LeaseNotYours(lease)),
        _ => Some(Error::LeaseNotActive(lease)),
    }
}

fn settle(
    queue: &mut TestQueue,
    pre: &Snapshot,
    walk: &mut Walk,
    args: (bool, u8, LeaseChoice, u8),
) {
    let (nack, consumer, choice, key) = args;
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = resolve(pre, &consumer, choice);
    let key = RecordKey::new(KEYS[usize::from(key)]);
    let got = if nack {
        queue.nack(&consumer, lease, &key)
    } else {
        queue.ack(&consumer, lease, &key)
    };
    let expected = lease_error(pre, &consumer, lease).map_or_else(
        || {
            let pocket_key = &pre
                .leases
                .iter()
                .find(|l| l.id == lease)
                .unwrap()
                .pocket_key;
            let record = record_view(pocket_view(pre, pocket_key).unwrap(), &key);
            match record.map(|r| (&r.status, r.rank)) {
                Some((RecordStatus::Held { rank, .. }, latest)) if *rank == latest => {
                    Ok(SettleOutcome::Settled)
                }
                Some((RecordStatus::Held { .. }, _)) => Ok(SettleOutcome::Deferred),
                _ => Err(Error::RecordNotHeld(key.clone())),
            }
        },
        Err,
    );
    assert_eq!(got, expected);
    walk.reach(format!("{} {got:?}", if nack { "nack" } else { "ack" }));
    if got == Ok(SettleOutcome::Settled) {
        let pocket_key = pre
            .leases
            .iter()
            .find(|l| l.id == lease)
            .unwrap()
            .pocket_key
            .clone();
        walk.handed
            .entry(lease)
            .or_default()
            .remove(&(pocket_key, key));
    }
}

fn renew_or_release(
    queue: &mut TestQueue,
    pre: &Snapshot,
    walk: &mut Walk,
    args: (bool, u8, LeaseChoice),
) {
    let (release, consumer, choice) = args;
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = resolve(pre, &consumer, choice);
    let refused = lease_error(pre, &consumer, lease);
    if release {
        let got = queue.release(&consumer, lease);
        let expected = refused.map_or_else(
            || {
                let pocket_key = &pre
                    .leases
                    .iter()
                    .find(|l| l.id == lease)
                    .unwrap()
                    .pocket_key;
                let pocket = pocket_view(pre, pocket_key).unwrap();
                Ok(if pocket.records.is_empty() {
                    PocketFate::Idle
                } else {
                    PocketFate::Queued
                })
            },
            Err,
        );
        assert_eq!(got, expected);
        walk.reach(format!("release {got:?}"));
        if walk.behind && got.is_ok() {
            walk.reach("release with the clock behind");
        }
    } else {
        let got = queue.renew(&consumer, lease);
        assert_eq!(got, refused.map_or_else(|| Ok(walk.now + LEASE_TTL), Err));
        walk.reach(format!("renew {}", got.is_ok()));
        if walk.behind && got.is_ok() {
            walk.reach("renew with the clock behind");
        }
    }
}

/// `LeaseLapses` and the three expiry rules, predicted from the previous snapshot.
fn lapse(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk) {
    let now = walk.now;
    let mut due: Vec<&LeaseView> = pre
        .leases
        .iter()
        .filter(|l| l.status == LeaseStatus::Active && l.expires_at <= now)
        .collect();
    due.sort_by_key(|l| (l.expires_at, l.id));
    let expected: Vec<(LeaseId, PocketFate)> = due
        .iter()
        .map(|l| {
            let pocket = pocket_view(pre, &l.pocket_key).unwrap();
            let fate = if pocket.consecutive_expiries + 1 >= MAX_CONSECUTIVE_EXPIRIES {
                PocketFate::Parked
            } else if pocket.records.is_empty() {
                PocketFate::Idle
            } else {
                PocketFate::Queued
            };
            (l.id, fate)
        })
        .collect();
    let got: Vec<(LeaseId, PocketFate)> =
        queue.lapse().iter().map(|l| (l.lease_id, l.fate)).collect();
    assert_eq!(got, expected);
    for (_, fate) in got {
        walk.reach(format!("lapse {fate:?}"));
    }
}

fn decay(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk) {
    let now = walk.now;
    let decayed = queue.decay();
    let shed = pre
        .superseded
        .iter()
        .filter(|s| s.expires_at <= now)
        .count();
    let letters = pre
        .dead_letters
        .iter()
        .filter(|d| d.expires_at <= now)
        .count();
    assert_eq!((decayed.superseded, decayed.dead_letters), (shed, letters));
    if shed > 0 {
        walk.reach("decay superseded");
    }
    if letters > 0 {
        walk.reach("decay dead letter");
    }
}

fn publisher_joins(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk, n: u8) {
    let id = PublisherId::new(PUBLISHERS[usize::from(n)]);
    let joined = pre.publishers.contains(&id);
    let got = queue.publisher_joins(id.clone());
    assert_eq!(
        got,
        if joined {
            Err(Error::PublisherExists(id))
        } else {
            Ok(())
        }
    );
    walk.reach(format!("join {}", got.is_ok()));
}

fn consumer_joins(queue: &mut TestQueue, pre: &Snapshot, walk: &mut Walk, n: u8) {
    let id = ConsumerId::new(CONSUMERS[usize::from(n)]);
    let joined = pre.consumers.contains(&id);
    let got = queue.consumer_joins(id.clone());
    assert_eq!(
        got,
        if joined {
            Err(Error::ConsumerExists(id))
        } else {
            Ok(())
        }
    );
    walk.reach(format!("join {}", got.is_ok()));
}

/// Whether the step is an operation that reads the queue's clock.
const fn reads_the_clock(step: Step) -> bool {
    !matches!(
        step,
        Step::PublisherJoins(_) | Step::ConsumerJoins(_) | Step::Advance(_)
    )
}

/// Runs one step, checking its outcome against the rules' guards.
fn run(queue: &mut TestQueue, clock: &FakeClock, walk: &mut Walk, step: Step) {
    let pre = queue.snapshot();
    if reads_the_clock(step) {
        walk.behind = clock.now() < walk.now;
        walk.now = walk.now.max(clock.now());
    }
    match step {
        Step::PublisherJoins(n) => publisher_joins(queue, &pre, walk, n),
        Step::ConsumerJoins(n) => consumer_joins(queue, &pre, walk, n),
        Step::Publish {
            publisher,
            pocket,
            key,
            rank,
        } => {
            publish(queue, &pre, walk, (publisher, pocket, key, rank));
        }
        Step::Pull { consumer, limit } => pull(queue, &pre, walk, (consumer, limit)),
        Step::Ack {
            consumer,
            lease,
            key,
        } => settle(queue, &pre, walk, (false, consumer, lease, key)),
        Step::Nack {
            consumer,
            lease,
            key,
        } => settle(queue, &pre, walk, (true, consumer, lease, key)),
        Step::Renew { consumer, lease } => {
            renew_or_release(queue, &pre, walk, (false, consumer, lease));
        }
        Step::Release { consumer, lease } => {
            renew_or_release(queue, &pre, walk, (true, consumer, lease));
        }
        Step::Advance(ms) => clock.set(Timestamp::from_millis(
            clock.now().as_millis().saturating_add_signed(ms),
        )),
        Step::Lapse => lapse(queue, &pre, walk),
        Step::Decay => decay(queue, &pre, walk),
    }
    let post = queue.snapshot();
    check_invariants(&post);
    check_transitions(&pre, &post);
    note_states(&post, walk);
}

fn note_states(snapshot: &Snapshot, walk: &mut Walk) {
    for pocket in &snapshot.pockets {
        walk.reach(format!("pocket {:?}", pocket.status));
        for record in &pocket.records {
            let status = match record.status {
                RecordStatus::Held { .. } => "Held",
                RecordStatus::Deferred => "Deferred",
                _ => "Queued",
            };
            walk.reach(format!("record {status} in {:?}", pocket.status));
        }
    }
}

fn walk(steps: &[Step]) -> Walk {
    let (mut queue, clock) = start();
    let mut walk = Walk::default();
    for &step in steps {
        run(&mut queue, &clock, &mut walk, step);
    }
    walk
}

fn distinct<T: Ord>(items: impl IntoIterator<Item = T>) -> bool {
    let items: Vec<T> = items.into_iter().collect();
    let count = items.len();
    items.into_iter().collect::<BTreeSet<T>>().len() == count
}

/// Every module and entity invariant, on one snapshot.
fn check_invariants(s: &Snapshot) {
    let config = config();
    assert!(
        config.max_consecutive_expiries() >= 1,
        "ExpiryLimitPositive"
    );
    assert!(config.max_pull_batch() >= 1, "PullBatchPositive");
    check_uniqueness(s);
    check_records(s);
    check_pockets(s);
    check_entities(s);
}

fn active_leases(s: &Snapshot) -> Vec<&LeaseView> {
    s.leases
        .iter()
        .filter(|l| l.status == LeaseStatus::Active)
        .collect()
}

fn all_records(s: &Snapshot) -> impl Iterator<Item = (&PocketView, &RecordView)> {
    s.pockets
        .iter()
        .flat_map(|p| p.records.iter().map(move |r| (p, r)))
}

/// The ten module invariants that say something is unique.
fn check_uniqueness(s: &Snapshot) {
    assert!(distinct(&s.publishers), "UniquePublisherId");
    assert!(distinct(&s.consumers), "UniqueConsumerId");
    assert!(
        distinct(s.pockets.iter().map(|p| &p.key)),
        "OnePocketPerKey"
    );
    for p in &s.pockets {
        assert!(
            distinct(p.records.iter().map(|r| &r.key)),
            "OneRecordPerKeyPerPocket"
        );
    }
    let active = active_leases(s);
    assert!(
        distinct(active.iter().map(|l| &l.pocket_key)),
        "OneActiveLeasePerPocket"
    );
    assert!(
        distinct(active.iter().map(|l| &l.consumer)),
        "OneActiveLeasePerConsumer"
    );
    assert!(
        distinct(s.pockets.iter().map(|p| p.position)),
        "UniquePocketPosition"
    );
    assert!(
        distinct(all_records(s).map(|(_, r)| r.position)),
        "UniqueRecordPosition"
    );
}

fn check_records(s: &Snapshot) {
    for (p, r) in all_records(s) {
        let outstanding = !matches!(r.status, RecordStatus::Queued);
        assert!(
            !outstanding || p.status == PocketStatus::Leased,
            "OutstandingOnlyWhileLeased"
        );
        if let RecordStatus::Held { rank, .. } = r.status {
            assert!(rank <= r.rank, "HeldVersionNotNewer");
        }
        assert!(r.position < s.next_position, "RecordPositionsBelowNext");
    }
}

fn check_pockets(s: &Snapshot) {
    let active = active_leases(s);
    for p in &s.pockets {
        let count = active.iter().filter(|l| l.pocket_key == p.key).count();
        assert_eq!(
            count,
            usize::from(p.status == PocketStatus::Leased),
            "LeasedHasOneActiveLease, UnleasedHasNoActiveLease"
        );
        assert!(
            p.status != PocketStatus::Idle || p.records.is_empty(),
            "IdleHasNoRecords"
        );
        assert!(
            p.consecutive_expiries <= MAX_CONSECUTIVE_EXPIRIES,
            "ExpiriesWithinLimit"
        );
        assert!(p.position < s.next_position, "PocketPositionsBelowNext");
    }
}

/// The entity-level timestamps, the pocket order and the references between entities.
fn check_entities(s: &Snapshot) {
    for shed in &s.superseded {
        assert!(shed.shed_at <= shed.expires_at, "ShedBeforeExpiry");
    }
    for letter in &s.dead_letters {
        assert!(letter.nacked_at <= letter.expires_at, "NackedBeforeExpiry");
    }
    for lease in &s.leases {
        assert!(lease.granted_at <= lease.expires_at, "GrantedBeforeExpiry");
        if let LeaseStatus::Released { ended_at } | LeaseStatus::Expired { ended_at } = lease.status
        {
            assert!(ended_at >= lease.granted_at, "EndedAfterGrant");
        }
        assert!(s.consumers.contains(&lease.consumer));
        assert!(s.pockets.iter().any(|p| p.key == lease.pocket_key));
    }
    let mut queued: Vec<&PocketView> = s
        .pockets
        .iter()
        .filter(|p| p.status == PocketStatus::Queued)
        .collect();
    queued.sort_by_key(|p| p.position);
    let order: Vec<&PocketKey> = queued.iter().map(|p| &p.key).collect();
    assert_eq!(
        s.pocket_order.iter().collect::<Vec<_>>(),
        order,
        "the pocket order"
    );
}

fn record_kind(status: &RecordStatus) -> &'static str {
    match status {
        RecordStatus::Held { .. } => "held",
        RecordStatus::Deferred => "deferred",
        _ => "queued",
    }
}

fn lease_kind(status: LeaseStatus) -> &'static str {
    match status {
        LeaseStatus::Active => "active",
        LeaseStatus::Released { .. } => "released",
        _ => "expired",
    }
}

/// Every change of status is an edge its entity declares; ended leases never change.
fn check_transitions(pre: &Snapshot, post: &Snapshot) {
    use PocketStatus::{Idle, Leased, Parked, Queued};
    const POCKET_EDGES: [(PocketStatus, PocketStatus); 6] = [
        (Queued, Leased),
        (Leased, Queued),
        (Leased, Idle),
        (Leased, Parked),
        (Idle, Queued),
        (Parked, Queued),
    ];
    const RECORD_EDGES: [(&str, &str); 4] = [
        ("queued", "held"),
        ("held", "queued"),
        ("held", "deferred"),
        ("deferred", "queued"),
    ];
    for before in &pre.pockets {
        let after = pocket_view(post, &before.key).expect("a pocket is never removed");
        let edge = (before.status, after.status);
        assert!(
            edge.0 == edge.1 || POCKET_EDGES.contains(&edge),
            "pocket {edge:?}"
        );
        for r in &before.records {
            let Some(now) = record_view(after, &r.key) else {
                continue;
            };
            let edge = (record_kind(&r.status), record_kind(&now.status));
            assert!(
                edge.0 == edge.1 || RECORD_EDGES.contains(&edge),
                "record {edge:?}"
            );
            if let (RecordStatus::Held { .. }, RecordStatus::Held { .. }) = (&r.status, &now.status)
            {
                assert_eq!(
                    r.status, now.status,
                    "a held version does not change while held"
                );
            }
        }
    }
    for before in &pre.leases {
        let after = post
            .leases
            .iter()
            .find(|l| l.id == before.id)
            .expect("leases are kept");
        let edge = (lease_kind(before.status), lease_kind(after.status));
        match edge.0 {
            "active" => assert!(["active", "released", "expired"].contains(&edge.1)),
            _ => assert_eq!(
                before, after,
                "transition-terminal: an ended lease never changes"
            ),
        }
    }
}

// invariant.Pocket.LeasedHasOneActiveLease, invariant.Pocket.UnleasedHasNoActiveLease,
// invariant.Pocket.IdleHasNoRecords, invariant.Pocket.ExpiriesWithinLimit,
// invariant.Record.HeldVersionNotNewer, invariant.SupersededRecord.ShedBeforeExpiry,
// invariant.DeadLetter.NackedBeforeExpiry, invariant.Lease.GrantedBeforeExpiry,
// invariant.Lease.EndedAfterGrant, invariant.UniquePublisherId,
// invariant.UniqueConsumerId, invariant.OnePocketPerKey,
// invariant.OneRecordPerKeyPerPocket, invariant.OneActiveLeasePerPocket,
// invariant.OneActiveLeasePerConsumer, invariant.OutstandingOnlyWhileLeased,
// invariant.UniquePocketPosition, invariant.UniqueRecordPosition,
// invariant.PocketPositionsBelowNext, invariant.RecordPositionsBelowNext after every
// step; transition-rejected.Pocket.status, transition-rejected.Record.status,
// transition-rejected.Lease.status, transition-terminal.Lease.status;
// rule-failure.PullWithinLease.2 (a consumer's active lease's pocket is leased, by
// LeasedHasOneActiveLease), rule-failure.ReleaseRequeuesPocket.3 and
// rule-failure.ReleaseIdlesEmptyPocket.3 (the same guard), and the ExclusivePocket,
// OldestFirst and OneVisitPerKeyPerLease guarantees against the previous snapshot.
#[test]
fn every_invariant_holds_after_every_step_of_any_walk() {
    runner(512)
        .run(&steps(), |steps| {
            walk(&steps);
            Ok(())
        })
        .unwrap();
}

/// What a blind generator would miss: every state and outcome the module names.
const WANTED: [&str; 36] = [
    "join true",
    "join false",
    "publish Ok(Created)",
    "publish Ok(NewKey)",
    "publish Ok(Superseded)",
    "publish Ok(Stale)",
    "revive Idle",
    "revive Parked",
    "pull NoPocketAvailable",
    "pull within lease",
    "pull claims",
    "batch of several",
    "ack Ok(Settled)",
    "ack Ok(Deferred)",
    "nack Ok(Settled)",
    "nack Ok(Deferred)",
    "renew true",
    "renew false",
    "release Ok(Queued)",
    "release Ok(Idle)",
    "lapse Queued",
    "lapse Idle",
    "lapse Parked",
    "decay superseded",
    "decay dead letter",
    "pocket Queued",
    "pocket Leased",
    "pocket Idle",
    "pocket Parked",
    "record Queued in Queued",
    "record Queued in Leased",
    "record Queued in Parked",
    "record Held in Leased",
    "record Deferred in Leased",
    "renew with the clock behind",
    "release with the clock behind",
];

// The generator is not blind: the same seeded walks reach every state the module
// names and every outcome the rules give.
#[test]
fn the_walk_reaches_every_state_and_every_outcome() {
    let mut runner = runner(1);
    let strategy = steps();
    let mut reached = BTreeSet::new();
    for _ in 0..512 {
        let steps = strategy.new_tree(&mut runner).unwrap().current();
        reached.extend(walk(&steps).reached);
    }
    let missing: Vec<&str> = WANTED
        .into_iter()
        .filter(|w| !reached.contains(*w))
        .collect();
    assert_eq!(missing, Vec::<&str>::new(), "never reached");
    for refused in [
        "publish Err(EqualRank",
        "ack Err(LeaseNotYours",
        "ack Err(RecordNotHeld",
        "nack Err(LeaseNotActive",
    ] {
        assert!(reached.iter().any(|r| r.starts_with(refused)), "{refused}");
    }
}

fn t19_publish(queue: &mut TestQueue, key: &str, rank: i64) -> Result<PublishOutcome, Error> {
    queue.publish(Publication::new(
        PublisherId::new("p0"),
        PocketKey::new("A"),
        RecordKey::new(key),
        Rank::new(rank),
        Payload::from(format!("{key}@{rank}").into_bytes()),
    ))
}

fn t19_record(queue: &TestQueue, key: &str) -> Option<RecordView> {
    let snapshot = queue.snapshot();
    record_view(&snapshot.pockets[0], &RecordKey::new(key)).cloned()
}

/// T19's trace, first part: Publish(A, k1, r1), Publish(A, k4, r1), then H1's
/// Pull(2) claims A under L1 and holds k1 and k4.
fn t19_claim(queue: &mut TestQueue, h1: &ConsumerId) -> Batch {
    assert_eq!(t19_publish(queue, "k1", 1), Ok(PublishOutcome::Created));
    assert_eq!(t19_publish(queue, "k4", 1), Ok(PublishOutcome::NewKey));
    let Ok(PullOutcome::Batch(l1)) = queue.pull(h1, 2) else {
        panic!("A is queued")
    };
    let handed: Vec<&str> = l1.records.iter().map(|r| r.record_key.as_str()).collect();
    assert_eq!(handed, ["k1", "k4"]);
    assert_eq!(queue.snapshot().pockets[0].status, PocketStatus::Leased);
    l1
}

/// Publish(A, k4, r3) supersedes while k4 is held: r1 becomes a superseded record,
/// and the held version stays r1.
fn t19_supersede(queue: &mut TestQueue) {
    assert_eq!(t19_publish(queue, "k4", 3), Ok(PublishOutcome::Superseded));
    let shed = &queue.snapshot().superseded[0];
    assert_eq!(
        (shed.rank, shed.reason),
        (Rank::new(1), ShedReason::Superseded)
    );
    let k4 = t19_record(queue, "k4").unwrap();
    assert_eq!(k4.rank, Rank::new(3));
    assert!(matches!(k4.status, RecordStatus::Held { rank, .. } if rank == Rank::new(1)));
}

/// Ack(L1, k1) removes k1; Ack(L1, k4) defers k4 at r3 to the back, whose position
/// this returns.
fn t19_acks(queue: &mut TestQueue, h1: &ConsumerId, l1: LeaseId) -> Position {
    assert_eq!(
        queue.ack(h1, l1, &RecordKey::new("k1")),
        Ok(SettleOutcome::Settled)
    );
    assert_eq!(t19_record(queue, "k1"), None);
    let back = queue.snapshot().next_position;
    assert_eq!(
        queue.ack(h1, l1, &RecordKey::new("k4")),
        Ok(SettleOutcome::Deferred)
    );
    let k4 = t19_record(queue, "k4").unwrap();
    assert_eq!((k4.status, k4.position), (RecordStatus::Deferred, back));
    back
}

/// Pull(2) under L1 hands nothing; Release(L1) queues k4 and puts A at the back.
fn t19_release(queue: &mut TestQueue, h1: &ConsumerId, l1: LeaseId, back: Position) {
    let Ok(PullOutcome::Batch(again)) = queue.pull(h1, 2) else {
        panic!("H1 holds L1")
    };
    assert_eq!((again.lease_id, again.records.len()), (l1, 0));
    assert_eq!(queue.release(h1, l1), Ok(PocketFate::Queued));
    let snapshot = queue.snapshot();
    assert_eq!(
        t19_record(queue, "k4").unwrap().status,
        RecordStatus::Queued
    );
    assert_eq!(snapshot.pockets[0].status, PocketStatus::Queued);
    assert_eq!(
        snapshot.pockets[0].position,
        Position::new(back.get() + 1),
        "A at the back"
    );
}

// T19's approved trace, step by step, with the states it names; a later Ack(L1, ...)
// is rejected.
#[test]
fn t19s_trace_runs_through_the_queue_with_the_same_states() {
    let (mut queue, _) = start();
    let h1 = ConsumerId::new("c0");
    let l1 = t19_claim(&mut queue, &h1);
    t19_supersede(&mut queue);
    let back = t19_acks(&mut queue, &h1, l1.lease_id);
    t19_release(&mut queue, &h1, l1.lease_id, back);
    assert_eq!(
        queue.ack(&h1, l1.lease_id, &RecordKey::new("k4")),
        Err(Error::LeaseNotActive(l1.lease_id))
    );
    check_invariants(&queue.snapshot());
}

/// A queue holding one of everything: pockets queued (a), leased (b), idle (c) and
/// parked (d); records queued, held and deferred; leases active, released and
/// expired; a superseded record and a dead letter.
fn everything() -> (TestQueue, BTreeMap<&'static str, LeaseId>) {
    let (mut queue, clock) = start();
    let (c0, c1) = (ConsumerId::new("c0"), ConsumerId::new("c1"));
    let publish = |queue: &mut TestQueue, pocket: &str, key: &str, rank: i64| {
        queue
            .publish(Publication::new(
                PublisherId::new("p0"),
                PocketKey::new(pocket),
                RecordKey::new(key),
                Rank::new(rank),
                Payload::default(),
            ))
            .unwrap();
    };
    let batch = |queue: &mut TestQueue, consumer: &ConsumerId, limit: i64| match queue
        .pull(consumer, limit)
    {
        Ok(PullOutcome::Batch(batch)) => batch,
        other => panic!("{other:?}"),
    };
    let mut leases = BTreeMap::new();
    publish(&mut queue, "d", "k0", 0);
    for name in ["expired-1", "expired-2"] {
        leases.insert(name, batch(&mut queue, &c0, 1).lease_id);
        clock.advance(LEASE_TTL);
        queue.lapse();
    }
    publish(&mut queue, "c", "k0", 0);
    let released = batch(&mut queue, &c0, 1);
    queue
        .ack(&c0, released.lease_id, &RecordKey::new("k0"))
        .unwrap();
    queue.release(&c0, released.lease_id).unwrap();
    leases.insert("released", released.lease_id);
    publish(&mut queue, "b", "k0", 0);
    publish(&mut queue, "b", "k1", 0);
    publish(&mut queue, "b", "k2", 0);
    publish(&mut queue, "a", "k0", 0);
    let active = batch(&mut queue, &c1, 2);
    publish(&mut queue, "b", "k0", 1);
    queue
        .nack(&c1, active.lease_id, &RecordKey::new("k0"))
        .unwrap();
    leases.insert("active", active.lease_id);
    (queue, leases)
}

// entity-fields.Queue, entity-fields.Pocket, entity-fields.Record: every pocket state,
// every record state and next_position read back.
#[test]
fn every_pocket_and_record_field_reads_back() {
    use PocketStatus::{Idle, Leased, Parked, Queued};
    let (queue, _) = everything();
    let s = queue.snapshot();
    check_invariants(&s);
    let statuses: Vec<(&str, PocketStatus)> = s
        .pockets
        .iter()
        .map(|p| (p.key.as_str(), p.status))
        .collect();
    assert_eq!(
        statuses,
        [("a", Queued), ("b", Leased), ("c", Idle), ("d", Parked)]
    );
    assert_eq!(s.pockets[3].consecutive_expiries, MAX_CONSECUTIVE_EXPIRIES);
    let highest = all_records(&s)
        .map(|(_, r)| r.position)
        .chain(s.pockets.iter().map(|p| p.position))
        .max()
        .unwrap();
    assert_eq!(s.next_position.get(), highest.get() + 1);
    assert_eq!(s.superseded.len(), 1);
    assert_eq!(s.dead_letters.len(), 1);
}

// entity-fields.Lease, entity-relationship.Consumer.leases,
// projection.Consumer.active_leases, entity-relationship.Pocket.leases,
// projection.Pocket.active_leases
#[test]
fn every_lease_relationship_and_projection_reads_back() {
    let (queue, leases) = everything();
    let s = queue.snapshot();
    let of_consumer = |c: &str| -> BTreeSet<LeaseId> {
        s.leases
            .iter()
            .filter(|l| l.consumer.as_str() == c)
            .map(|l| l.id)
            .collect()
    };
    assert_eq!(
        of_consumer("c0"),
        [leases["expired-1"], leases["expired-2"], leases["released"]].into()
    );
    assert_eq!(of_consumer("c1"), [leases["active"]].into());
    assert_eq!(active_lease_of(&s, &ConsumerId::new("c0")), None);
    assert_eq!(
        active_lease_of(&s, &ConsumerId::new("c1")).unwrap().id,
        leases["active"]
    );
    let of_pocket = |p: &str, active: bool| -> Vec<LeaseId> {
        s.leases
            .iter()
            .filter(|l| l.pocket_key.as_str() == p && (!active || l.status == LeaseStatus::Active))
            .map(|l| l.id)
            .collect()
    };
    assert_eq!(
        of_pocket("d", false),
        [leases["expired-1"], leases["expired-2"]]
    );
    assert_eq!(of_pocket("d", true), []);
    assert_eq!(of_pocket("b", true), [leases["active"]]);
}

// entity-relationship.Pocket.records, projection.Pocket.queued_records,
// projection.Pocket.held_records, projection.Pocket.deferred_records
#[test]
fn every_record_projection_reads_back() {
    let (queue, _) = everything();
    let s = queue.snapshot();
    let b = &s.pockets[1];
    let in_status = |kind: &str| -> Vec<&str> {
        b.records
            .iter()
            .filter(|r| record_kind(&r.status) == kind)
            .map(|r| r.key.as_str())
            .collect()
    };
    assert_eq!(in_status("deferred"), ["k0"]);
    assert_eq!(in_status("held"), ["k1"]);
    assert_eq!(in_status("queued"), ["k2"]);
    assert_eq!(s.pockets[2].records, []);
}
