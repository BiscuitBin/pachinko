//! The walk's generator and its predictions: the steps, the seeded runner, and the
//! state the rules predict after each step, derived from the snapshot before it.
//!
//! Before each step the walk projects the queue's `Snapshot` into a [`Model`],
//! applies the step to the model by following the guards and the effects of the
//! rules in `docs/specs/pachinko.allium`, and so predicts both the step's outcome
//! and the whole state after it. After the step it projects the queue's new
//! `Snapshot` and asserts it equals the prediction; a step that should change
//! nothing must leave the snapshot equal, `Vec` for `Vec`. The model is built
//! afresh from each pre-state, so an error in it never carries to a later step.
//!
//! Every function here is generic over the queue's store, clock and lease-id
//! source, and moves the clock only through the closure it is given, so another
//! `PocketRecords` runs the same walk with one new instantiation.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use libpachinko::clock::Clock;
use libpachinko::lease_ids::LeaseIds;
use libpachinko::records::{PocketRecords, RecordStatus};
use libpachinko::testing::{InspectRecords, PocketStatus, PocketView, Snapshot};
use libpachinko::{
    Config, ConsumerId, Lease, LeaseActionError, LeaseId, Mark, Payload, PocketKey, Publication,
    PublishError, PublishOutcome, PublisherId, PullOutcome, Queue, Rank, RecordKey, Timestamp,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as RunnerConfig, RngAlgorithm, TestRng, TestRunner};

use super::invariants;

mod model;

use model::{Ended, Model, ended};

/// The publishers a step may name; `p0` joins before the walk, `p1` only if a step
/// says so, so `UnknownPublisher` is reachable.
const PUBLISHERS: [&str; 2] = ["p0", "p1"];
/// The consumers a step may name; `c0` and `c1` join before the walk, `c2` only if
/// a step says so.
const CONSUMERS: [&str; 3] = ["c0", "c1", "c2"];
const POCKETS: [&str; 3] = ["a", "b", "c"];
const KEYS: [&str; 3] = ["k0", "k1", "k2"];

/// The instant the walk's clock starts at.
pub(crate) const START: Timestamp = Timestamp::from_millis(1_000);

/// How far one `Advance` moves the clock forwards at most: past the walk's 100 ms
/// `lease_ttl`, so one step can carry a lease beyond its `expires_at`.
const FORWARD_MILLIS: i64 = 120;

/// How far one `Advance` moves the clock back at most, as a corrected system clock
/// can; the queue's `now` must not follow it.
const BACKWARD_MILLIS: i64 = 120;

/// The walk's configuration: a short lease and small limits, so lapses, parking
/// and capped batches come within a few steps. Used once, to build the queue;
/// every check reads the queue's own `config()`.
pub(crate) fn config() -> Config {
    let mut config = Config::default();
    config.lease_ttl = Duration::from_millis(100);
    config.max_consecutive_expiries = 2;
    config.max_pull_batch = 2;
    config.superseded_ttl = Duration::from_millis(150);
    config.dead_letter_ttl = Duration::from_millis(250);
    config
}

/// Which lease an ack, nack, renew or release names.
#[derive(Clone, Copy, Debug)]
pub(crate) enum LeaseChoice {
    /// The consumer's active lease, or one never issued when it holds none.
    Own,
    /// Any lease handed out so far: active or ended, the consumer's or another's.
    Issued(u8),
    /// One never handed out.
    Never,
}

/// One step of the walk; the indices pick from the small domains above.
#[derive(Clone, Copy, Debug)]
pub(crate) enum WalkStep {
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
    /// The clock moves by this many milliseconds, back when negative.
    Advance(i64),
    /// `LeaseLapses` for every due lease, then both decays.
    Tick,
}

fn lease_choice() -> impl Strategy<Value = LeaseChoice> {
    prop_oneof![
        6 => Just(LeaseChoice::Own),
        2 => any::<u8>().prop_map(LeaseChoice::Issued),
        1 => Just(LeaseChoice::Never),
    ]
}

fn step() -> impl Strategy<Value = WalkStep> {
    prop_oneof![
        1 => (0..2_u8).prop_map(WalkStep::PublisherJoins),
        1 => (0..3_u8).prop_map(WalkStep::ConsumerJoins),
        10 => (0..2_u8, 0..3_u8, 0..3_u8, 0..4_i64).prop_map(|(publisher, pocket, key, rank)| {
            WalkStep::Publish { publisher, pocket, key, rank }
        }),
        7 => (0..3_u8, -1..=3_i64).prop_map(|(consumer, limit)| WalkStep::Pull { consumer, limit }),
        5 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| WalkStep::Ack { consumer, lease, key }),
        4 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| WalkStep::Nack { consumer, lease, key }),
        1 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| WalkStep::Renew { consumer, lease }),
        3 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| WalkStep::Release { consumer, lease }),
        6 => (0..=FORWARD_MILLIS).prop_map(WalkStep::Advance),
        1 => (-BACKWARD_MILLIS..=0).prop_map(WalkStep::Advance),
        4 => Just(WalkStep::Tick),
    ]
}

/// Walks of 1 to 80 steps.
pub(crate) fn steps() -> impl Strategy<Value = Vec<WalkStep>> {
    proptest::collection::vec(step(), 1..=80)
}

/// A seeded runner that persists nothing: a failure is reproduced from the seed
/// and the shrunk case the output prints, never from a file in the worktree.
pub(crate) fn runner(cases: u32) -> TestRunner {
    let config = RunnerConfig {
        cases,
        failure_persistence: None,
        ..RunnerConfig::default()
    };
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

/// What the walk carries between steps beyond the queue.
#[derive(Debug, Default)]
pub(crate) struct Walk {
    /// The instant the walk last set the clock to.
    clock: Timestamp,
    /// The queue's `now`: the latest clock reading it has taken (decision 0019's
    /// monotonic clamp). The queue reads its clock in an accepted step and in a
    /// publish by a joined publisher, never in a refused lease action, a
    /// `NoPocketAvailable` pull, a join or an `Advance`; the walk moves this only
    /// at those steps, so a later backward clock is measured against it.
    now: Timestamp,
    /// Every lease id a pull has handed out, in order.
    issued: Vec<LeaseId>,
    /// Every lease that has ended, as the release or the tick that ended it
    /// returned it.
    pub(crate) ended: BTreeMap<LeaseId, Lease>,
    /// The `(pocket, record)` keys each lease has handed out. A key stays until the
    /// lease ends, acked, nacked or neither, so it is never handed out twice under
    /// one lease.
    handed: BTreeMap<LeaseId, BTreeSet<(PocketKey, RecordKey)>>,
    /// The labels of the states and outcomes reached.
    pub(crate) reached: BTreeSet<String>,
}

/// One step in progress: the pre-state, the model the step is applied to, and the
/// instant the queue reads if the step reads its clock.
struct Turn<'w> {
    pre: Snapshot,
    expected: Model,
    now: Timestamp,
    behind: bool,
    config: Config,
    reads: bool,
    lapsed: Vec<Lease>,
    walk: &'w mut Walk,
}

impl<'w> Turn<'w> {
    fn new(pre: Snapshot, config: Config, walk: &'w mut Walk) -> Self {
        Self {
            expected: Model::of(&pre),
            pre,
            now: walk.now.max(walk.clock),
            behind: walk.clock < walk.now,
            config,
            reads: false,
            lapsed: Vec::new(),
            walk,
        }
    }

    fn reach(&mut self, label: impl Into<String>) {
        self.walk.reached.insert(label.into());
    }

    /// The lease id the choice names.
    fn resolve(&self, consumer: &ConsumerId, choice: LeaseChoice) -> LeaseId {
        match choice {
            LeaseChoice::Own => self
                .expected
                .active_lease(consumer)
                .unwrap_or_else(|| self.never_issued()),
            LeaseChoice::Issued(n) if !self.walk.issued.is_empty() => {
                self.walk.issued[usize::from(n) % self.walk.issued.len()]
            }
            LeaseChoice::Issued(_) | LeaseChoice::Never => self.never_issued(),
        }
    }

    /// The smallest lease id no pull has handed out so far, chosen afresh each
    /// step, so the walk reserves no id the lease-id source might return.
    fn never_issued(&self) -> LeaseId {
        (0..=u128::MAX)
            .map(LeaseId::from_u128)
            .find(|id| !self.walk.issued.contains(id))
            .expect("fewer lease ids issued than exist")
    }

    /// Whether `lease` is `consumer`'s active lease with `expires_at` at or before
    /// the step's `now`: no tick has lapsed it yet, so under decision 0019's branch
    /// A an action on it is decided by processing order and succeeds.
    fn late(&self, consumer: &ConsumerId, lease: LeaseId) -> bool {
        self.expected
            .leases
            .get(&lease)
            .is_some_and(|l| &l.consumer_id == consumer && l.expires_at <= self.now)
    }
}

fn publish_label(got: &Result<PublishOutcome, PublishError>) -> String {
    match got {
        Ok(outcome) => format!("{outcome:?}"),
        Err(PublishError::EqualRank { .. }) => "EqualRank".to_owned(),
        Err(PublishError::UnknownPublisher(_)) => "UnknownPublisher".to_owned(),
        Err(other) => format!("{other:?}"),
    }
}

fn action_label<T: std::fmt::Debug>(got: &Result<T, LeaseActionError>) -> String {
    match got {
        Ok(outcome) => format!("{outcome:?}"),
        Err(LeaseActionError::UnknownConsumer(_)) => "UnknownConsumer".to_owned(),
        Err(LeaseActionError::LeaseNotActive(_)) => "LeaseNotActive".to_owned(),
        Err(LeaseActionError::LeaseNotYours(_)) => "LeaseNotYours".to_owned(),
        Err(LeaseActionError::RecordNotHeld(_)) => "RecordNotHeld".to_owned(),
        Err(LeaseActionError::MarkConflict(_)) => "MarkConflict".to_owned(),
        Err(other) => format!("{other:?}"),
    }
}

fn publisher_joins<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    n: u8,
) {
    let id = PublisherId::new(PUBLISHERS[usize::from(n)]);
    let expected = turn.expected.publisher_joins(&id);
    let got = queue.publisher_joins(id);
    assert_eq!(got, expected, "PublisherJoins");
    turn.reach(format!("publisher joins {}", got.is_ok()));
}

fn consumer_joins<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    n: u8,
) {
    let id = ConsumerId::new(CONSUMERS[usize::from(n)]);
    let expected = turn.expected.consumer_joins(&id);
    let got = queue.consumer_joins(id);
    assert_eq!(got, expected, "ConsumerJoins");
    turn.reach(format!("consumer joins {}", got.is_ok()));
}

fn publish<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (publisher, pocket, key, rank): (u8, u8, u8, i64),
) {
    let id = PublisherId::new(PUBLISHERS[usize::from(publisher)]);
    let publication = Publication::new(
        PocketKey::new(POCKETS[usize::from(pocket)]),
        RecordKey::new(KEYS[usize::from(key)]),
        Rank::new(rank),
        Payload::from(format!("{publisher}{pocket}{key}{rank}").into_bytes()),
    );
    let pocket = find_pocket(&turn.pre, &publication.pocket_key);
    let parked = pocket.is_some_and(|p| p.status == PocketStatus::Parked);
    let held = pocket.and_then(|p| held_status(p, &publication.record_key));
    let deferred = held.map(|(_, deferred)| deferred);
    let (now, ttl) = (turn.now, turn.config.superseded_ttl);
    let (expected, revived) = turn.expected.publish(&id, &publication, now, ttl);
    let got = queue.publish(&id, publication);
    assert_eq!(got, expected, "publish");
    turn.reads = turn.pre.publishers.contains(&id);
    turn.reach(format!("publish {}", publish_label(&got)));
    match (&got, deferred) {
        (Ok(PublishOutcome::Superseded), Some(false)) => {
            turn.reach("publish defers behind a held version");
        }
        (Ok(PublishOutcome::Superseded), Some(true)) => {
            turn.reach("publish supersedes a deferred version");
        }
        (Ok(PublishOutcome::Stale), _) if parked => {
            turn.reach("stale publish leaves a parked pocket parked");
        }
        _ => {}
    }
    if let Some(status) = revived {
        turn.reach(format!("revive {status:?}"));
    }
}

fn find_pocket<'s>(s: &'s Snapshot, key: &PocketKey) -> Option<&'s PocketView> {
    s.pockets.iter().find(|pocket| &pocket.key == key)
}

/// A held record's mark and whether a version is deferred behind it; `None` when
/// the pocket has no such record or it is queued.
fn held_status(pocket: &PocketView, key: &RecordKey) -> Option<(Mark, bool)> {
    pocket
        .records
        .iter()
        .find(|record| &record.key == key)
        .and_then(|record| match &record.status {
            RecordStatus::Held {
                mark,
                deferred_position,
                ..
            } => Some((*mark, deferred_position.is_some())),
            _ => None,
        })
}

fn pull<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (consumer, limit): (u8, i64),
) {
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let own = turn.expected.active_lease(&consumer);
    let late = own.is_some_and(|lease| turn.late(&consumer, lease));
    let (now, config) = (turn.now, turn.config.clone());
    let mut expected = turn.expected.pull(&consumer, limit, now, &config);
    let got = queue.pull(&consumer, limit);
    if let (Ok(PullOutcome::Batch { lease: fresh, .. }), Ok(PullOutcome::Batch { lease, .. })) =
        (&mut expected, &got)
        && turn.expected.pending.is_some()
    {
        assert!(
            !turn.walk.issued.contains(lease),
            "a claim draws a lease id never issued before"
        );
        *fresh = *lease;
        turn.expected.adopt(*lease);
        turn.walk.issued.push(*lease);
    }
    assert_eq!(got, expected, "pull");
    match &got {
        Ok(batch @ PullOutcome::Batch { records, .. }) => {
            let size = usize::try_from(limit.clamp(0, i64::from(config.max_pull_batch))).unwrap();
            invariants::check_batch(&turn.pre, &mut turn.walk.handed, &consumer, batch, size);
            turn.reads = true;
            turn.reach(if own.is_some() {
                "pull within lease"
            } else {
                "pull claims"
            });
            turn.reach(batch_label(records.len()));
        }
        Ok(_) => turn.reach("pull NoPocketAvailable"),
        Err(_) => turn.reach("pull UnknownConsumer"),
    }
    if late && turn.reads {
        turn.reach("branch A pull");
    }
}

const fn batch_label(len: usize) -> &'static str {
    match len {
        0 => "empty batch",
        1 => "batch of one",
        _ => "batch of several",
    }
}

fn mark_held<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (mark, consumer, choice, key): (Mark, u8, LeaseChoice, u8),
) {
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = turn.resolve(&consumer, choice);
    let key = RecordKey::new(KEYS[usize::from(key)]);
    let late = turn.late(&consumer, lease);
    let pocket = (turn.pre.leases.iter())
        .find(|l| l.id == lease)
        .and_then(|l| find_pocket(&turn.pre, &l.pocket_key));
    let counted = pocket.is_some_and(|p| p.consecutive_expiries > 0);
    let before = pocket
        .and_then(|p| held_status(p, &key))
        .map(|(mark, _)| mark);
    let (now, config) = (turn.now, turn.config.clone());
    let expected = turn
        .expected
        .mark((mark, &consumer, lease, &key), now, &config);
    let nack = mark == Mark::Nacked;
    let got = if nack {
        queue.nack(&consumer, lease, &key)
    } else {
        queue.ack(&consumer, lease, &key)
    };
    let name = if nack { "nack" } else { "ack" };
    assert_eq!(got, expected, "{name}");
    turn.reads = got.is_ok();
    let first = before == Some(Mark::Unsettled);
    let label = match &got {
        Ok(_) if first => "marks".to_owned(),
        Ok(_) => "repeats".to_owned(),
        Err(_) => action_label(&got),
    };
    turn.reach(format!("{name} {label}"));
    if counted && first && turn.reads {
        turn.reach(format!("{name} resets a counted pocket"));
    }
    if late && turn.reads {
        turn.reach(format!("branch A {name}"));
    }
}

fn renew<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (consumer, choice): (u8, LeaseChoice),
) {
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = turn.resolve(&consumer, choice);
    let late = turn.late(&consumer, lease);
    let (now, ttl) = (turn.now, turn.config.lease_ttl);
    let expected = turn.expected.renew(&consumer, lease, now, ttl);
    let got = queue.renew(&consumer, lease);
    assert_eq!(got, expected, "renew");
    turn.reads = got.is_ok();
    let label = if got.is_ok() {
        "accepted".to_owned()
    } else {
        action_label(&got)
    };
    turn.reach(format!("renew {label}"));
    late_or_behind(turn, "renew", late);
}

fn release<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (consumer, choice): (u8, LeaseChoice),
) {
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = turn.resolve(&consumer, choice);
    let late = turn.late(&consumer, lease);
    let config = turn.config.clone();
    let (expected, rows) = turn.expected.release(&consumer, lease, turn.now, &config);
    let got = queue.release(&consumer, lease);
    let fields = got
        .as_ref()
        .map(|(fate, lease)| (*fate, ended(lease)))
        .map_err(Clone::clone);
    assert_eq!(fields, expected, "release");
    turn.reads = got.is_ok();
    turn.reach(format!(
        "release {}",
        action_label(&fields.map(|(fate, _)| fate))
    ));
    for row in rows {
        turn.reach(format!("release {row}"));
    }
    if let Ok((_, lease)) = got {
        turn.lapsed.push(lease);
    }
    late_or_behind(turn, "release", late);
}

fn late_or_behind(turn: &mut Turn<'_>, name: &str, late: bool) {
    if late && turn.reads {
        turn.reach(format!("branch A {name}"));
    }
    if turn.behind && turn.reads {
        turn.reach(format!("{name} with the clock behind"));
    }
}

fn tick<R: PocketRecords, C: Clock, L: LeaseIds>(queue: &mut Queue<R, C, L>, turn: &mut Turn<'_>) {
    let config = turn.config.clone();
    let (lapses, shed, letters) = turn.expected.tick(turn.now, &config);
    let report = queue.tick();
    let got: Vec<Ended> = report.lapsed.iter().map(ended).collect();
    let expected: Vec<Ended> = lapses.iter().map(|(lease, _, _)| lease.clone()).collect();
    assert_eq!(got, expected, "the lapses, in (expires_at, lease id) order");
    assert_eq!(
        (report.decayed_superseded, report.decayed_dead_letters),
        (shed, letters),
        "the decays"
    );
    turn.reads = true;
    for (_, fate, rows) in &lapses {
        turn.reach(format!("lapse {fate:?}"));
        for row in rows {
            turn.reach(format!("lapse {row}"));
        }
    }
    if shed > 0 {
        turn.reach("decay superseded");
    }
    if letters > 0 {
        turn.reach("decay dead letter");
    }
    turn.lapsed = report.lapsed;
}

fn advance(set_clock: &impl Fn(Timestamp), turn: &mut Turn<'_>, millis: i64) {
    let to = Timestamp::from_millis(turn.walk.clock.as_millis().saturating_add_signed(millis));
    turn.walk.clock = to;
    set_clock(to);
}

/// Runs one step: predicts it from the pre-state, applies it, and checks the
/// post-state against the prediction, every invariant and every transition.
fn run<R: InspectRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    set_clock: &impl Fn(Timestamp),
    walk: &mut Walk,
    step: WalkStep,
) {
    let pre = queue.snapshot();
    let mut turn = Turn::new(pre.clone(), queue.config().clone(), walk);
    match step {
        WalkStep::PublisherJoins(n) => publisher_joins(queue, &mut turn, n),
        WalkStep::ConsumerJoins(n) => consumer_joins(queue, &mut turn, n),
        WalkStep::Publish {
            publisher,
            pocket,
            key,
            rank,
        } => {
            publish(queue, &mut turn, (publisher, pocket, key, rank));
        }
        WalkStep::Pull { consumer, limit } => pull(queue, &mut turn, (consumer, limit)),
        WalkStep::Ack {
            consumer,
            lease,
            key,
        } => {
            mark_held(queue, &mut turn, (Mark::Acked, consumer, lease, key));
        }
        WalkStep::Nack {
            consumer,
            lease,
            key,
        } => {
            mark_held(queue, &mut turn, (Mark::Nacked, consumer, lease, key));
        }
        WalkStep::Renew { consumer, lease } => renew(queue, &mut turn, (consumer, lease)),
        WalkStep::Release { consumer, lease } => release(queue, &mut turn, (consumer, lease)),
        WalkStep::Advance(millis) => advance(set_clock, &mut turn, millis),
        WalkStep::Tick => tick(queue, &mut turn),
    }
    let post = queue.snapshot();
    finish(turn, &post, queue.config());
}

/// The checks after a step, and what the walk carries to the next.
fn finish(turn: Turn<'_>, post: &Snapshot, config: &Config) {
    let Turn {
        pre,
        expected,
        now,
        reads,
        lapsed,
        walk,
        ..
    } = turn;
    assert_eq!(Model::of(post), expected, "the state the rules predict");
    if expected == Model::of(&pre) {
        assert_eq!(
            post, &pre,
            "a step that changes nothing leaves the snapshot equal"
        );
    }
    if reads {
        walk.now = now;
    }
    invariants::check_invariants(post, config);
    invariants::check_transitions(&pre, post, &lapsed, &walk.ended);
    for lease in lapsed {
        invariants::ended_after_grant(&lease);
        walk.ended.insert(lease.id, lease);
    }
    note_states(post, walk);
}

/// Labels every pocket state and every record state, a held one by its mark, by
/// its pocket's.
fn note_states(snapshot: &Snapshot, walk: &mut Walk) {
    for pocket in &snapshot.pockets {
        walk.reached.insert(format!("pocket {:?}", pocket.status));
        for record in &pocket.records {
            let status = match &record.status {
                RecordStatus::Held { mark, .. } => format!("Held {mark:?}"),
                RecordStatus::Queued => "Queued".to_owned(),
                other => format!("{other:?}"),
            };
            walk.reached
                .insert(format!("record {status} in {:?}", pocket.status));
        }
    }
}

/// Runs `steps` over `queue`, which must be new and empty, after `p0`, `c0` and
/// `c1` join, with the clock set through `set_clock` from [`START`].
pub(crate) fn walk<R: InspectRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    set_clock: &impl Fn(Timestamp),
    steps: &[WalkStep],
) -> Walk {
    set_clock(START);
    let mut walk = Walk {
        clock: START,
        ..Walk::default()
    };
    for step in [
        WalkStep::PublisherJoins(0),
        WalkStep::ConsumerJoins(0),
        WalkStep::ConsumerJoins(1),
    ]
    .iter()
    .chain(steps)
    {
        run(queue, set_clock, &mut walk, *step);
    }
    walk
}
