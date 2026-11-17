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
use libpachinko::records::{PocketRecords, Position, RecordStatus, RecordView};
use libpachinko::testing::{InspectRecords, PocketStatus, PocketView, ShedReason, Snapshot};
use libpachinko::{
    Config, ConsumerId, HeldRecord, JoinError, Lease, LeaseActionError, LeaseId, LeaseStatus,
    Payload, PocketFate, PocketKey, Publication, PublishError, PublishOutcome, PublisherId,
    PullError, PullOutcome, Queue, Rank, RecordKey, SettleOutcome, Timestamp,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as RunnerConfig, RngAlgorithm, TestRng, TestRunner};

use super::invariants;

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
        8 => (0..2_u8, 0..3_u8, 0..3_u8, 0..4_i64).prop_map(|(publisher, pocket, key, rank)| {
            WalkStep::Publish { publisher, pocket, key, rank }
        }),
        6 => (0..3_u8, -1..=3_i64).prop_map(|(consumer, limit)| WalkStep::Pull { consumer, limit }),
        4 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| WalkStep::Ack { consumer, lease, key }),
        3 => (0..3_u8, lease_choice(), 0..3_u8)
            .prop_map(|(consumer, lease, key)| WalkStep::Nack { consumer, lease, key }),
        2 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| WalkStep::Renew { consumer, lease }),
        2 => (0..3_u8, lease_choice()).prop_map(|(consumer, lease)| WalkStep::Release { consumer, lease }),
        5 => (0..=FORWARD_MILLIS).prop_map(WalkStep::Advance),
        1 => (-BACKWARD_MILLIS..=0).prop_map(WalkStep::Advance),
        3 => Just(WalkStep::Tick),
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

/// An active lease, as the rules see it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelLease {
    pocket_key: PocketKey,
    consumer_id: ConsumerId,
    granted_at: Timestamp,
    expires_at: Timestamp,
}

/// A pocket, its records keyed by record key so the store's order cannot matter.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelPocket {
    status: PocketStatus,
    position: Position,
    consecutive_expiries: u32,
    records: BTreeMap<RecordKey, RecordView>,
}

/// A `SupersededRecord`'s fields.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelShed {
    pocket_key: PocketKey,
    key: RecordKey,
    rank: Rank,
    payload: Payload,
    reason: ShedReason,
    shed_at: Timestamp,
    expires_at: Timestamp,
}

/// A `DeadLetter`'s fields.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelLetter {
    pocket_key: PocketKey,
    key: RecordKey,
    rank: Rank,
    payload: Payload,
    nacked_by: ConsumerId,
    nacked_at: Timestamp,
    expires_at: Timestamp,
}

/// An ended lease's fields, as a release or a tick returns them.
type Ended = (LeaseId, ModelLease, LeaseStatus);

/// One lapse the tick should process: the ended lease and its pocket's fate.
type Lapse = (Ended, PocketStatus);

/// The queue's state as the rules read it, projected from a `Snapshot`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Model {
    next_position: Position,
    publishers: BTreeSet<PublisherId>,
    consumers: BTreeMap<ConsumerId, Option<LeaseId>>,
    pockets: BTreeMap<PocketKey, ModelPocket>,
    pocket_order: Vec<(Position, PocketKey)>,
    leases: BTreeMap<LeaseId, ModelLease>,
    /// The lease a claim creates, held apart from `leases` until the queue's answer
    /// names it: the id is the lease-id source's to choose, so the walk checks the
    /// queue's is fresh and then adopts it. The walk reserves no lease id.
    pending: Option<ModelLease>,
    superseded: Vec<ModelShed>,
    dead_letters: Vec<ModelLetter>,
}

fn model_pocket(pocket: &PocketView) -> ModelPocket {
    ModelPocket {
        status: pocket.status,
        position: pocket.position,
        consecutive_expiries: pocket.consecutive_expiries,
        records: pocket
            .records
            .iter()
            .map(|record| (record.key.clone(), record.clone()))
            .collect(),
    }
}

fn model_lease(lease: &Lease) -> ModelLease {
    ModelLease {
        pocket_key: lease.pocket_key.clone(),
        consumer_id: lease.consumer_id.clone(),
        granted_at: lease.granted_at,
        expires_at: lease.expires_at,
    }
}

fn ended(lease: &Lease) -> Ended {
    (lease.id, model_lease(lease), lease.status)
}

impl Model {
    pub(crate) fn of(s: &Snapshot) -> Self {
        Self {
            next_position: s.next_position,
            publishers: s.publishers.iter().cloned().collect(),
            consumers: s.consumers.iter().cloned().collect(),
            pockets: s
                .pockets
                .iter()
                .map(|pocket| (pocket.key.clone(), model_pocket(pocket)))
                .collect(),
            pocket_order: s.pocket_order.clone(),
            leases: s
                .leases
                .iter()
                .map(|lease| (lease.id, model_lease(lease)))
                .collect(),
            pending: None,
            superseded: s
                .superseded
                .iter()
                .map(|r| ModelShed {
                    pocket_key: r.pocket_key.clone(),
                    key: r.key.clone(),
                    rank: r.rank,
                    payload: r.payload.clone(),
                    reason: r.reason,
                    shed_at: r.shed_at,
                    expires_at: r.expires_at,
                })
                .collect(),
            dead_letters: s
                .dead_letters
                .iter()
                .map(|d| ModelLetter {
                    pocket_key: d.pocket_key.clone(),
                    key: d.key.clone(),
                    rank: d.rank,
                    payload: d.payload.clone(),
                    nacked_by: d.nacked_by.clone(),
                    nacked_at: d.nacked_at,
                    expires_at: d.expires_at,
                })
                .collect(),
        }
    }

    /// `queue.next_position`, taken: the value, with the counter moved past it.
    const fn take_position(&mut self) -> Position {
        let position = self.next_position;
        self.next_position = position.next();
        position
    }

    fn pocket(&mut self, key: &PocketKey) -> &mut ModelPocket {
        self.pockets.get_mut(key).expect("a lease's pocket exists")
    }

    /// A pocket enters the pocket order at `position`.
    fn enqueue(&mut self, key: &PocketKey, position: Position) {
        self.pocket_order.push((position, key.clone()));
        self.pocket_order.sort();
    }

    fn active_lease(&self, consumer: &ConsumerId) -> Option<LeaseId> {
        self.consumers.get(consumer).copied().flatten()
    }

    fn publisher_joins(&mut self, id: &PublisherId) -> Result<(), JoinError> {
        if !self.publishers.insert(id.clone()) {
            return Err(JoinError::PublisherExists(id.clone()));
        }
        Ok(())
    }

    fn consumer_joins(&mut self, id: &ConsumerId) -> Result<(), JoinError> {
        if self.consumers.contains_key(id) {
            return Err(JoinError::ConsumerExists(id.clone()));
        }
        self.consumers.insert(id.clone(), None);
        Ok(())
    }

    /// The four publish rules, then `PocketTouched` for all but the first, which
    /// returns the status a revive moved the pocket from.
    fn publish(
        &mut self,
        publisher: &PublisherId,
        publication: &Publication,
        now: Timestamp,
        superseded_ttl: Duration,
    ) -> (Result<PublishOutcome, PublishError>, Option<PocketStatus>) {
        if !self.publishers.contains(publisher) {
            return (Err(PublishError::UnknownPublisher(publisher.clone())), None);
        }
        let Some(pocket) = self.pockets.get(&publication.pocket_key) else {
            self.publish_into_new_pocket(publication);
            return (Ok(PublishOutcome::Created), None);
        };
        let stored = pocket.records.get(&publication.record_key).cloned();
        let shed = |rank, payload, reason| ModelShed {
            pocket_key: publication.pocket_key.clone(),
            key: publication.record_key.clone(),
            rank,
            payload,
            reason,
            shed_at: now,
            expires_at: now + superseded_ttl,
        };
        let outcome = match stored {
            None => self.publish_new_key(publication),
            Some(record) if publication.rank > record.rank => {
                self.superseded
                    .push(shed(record.rank, record.payload, ShedReason::Superseded));
                self.supersede(publication)
            }
            Some(record) if publication.rank < record.rank => {
                let (rank, payload) = (publication.rank, publication.payload.clone());
                self.superseded.push(shed(rank, payload, ShedReason::Stale));
                PublishOutcome::Stale
            }
            Some(_) => {
                let key = publication.record_key.clone();
                let rank = publication.rank;
                return (Err(PublishError::EqualRank { key, rank }), None);
            }
        };
        (Ok(outcome), self.touch(&publication.pocket_key))
    }

    /// `PublishIntoNewPocket`.
    fn publish_into_new_pocket(&mut self, publication: &Publication) {
        let pocket_position = self.take_position();
        let record_position = self.take_position();
        let record = RecordView::new(
            publication.record_key.clone(),
            publication.rank,
            publication.payload.clone(),
            record_position,
            RecordStatus::Queued,
        );
        let pocket = ModelPocket {
            status: PocketStatus::Queued,
            position: pocket_position,
            consecutive_expiries: 0,
            records: BTreeMap::from([(publication.record_key.clone(), record)]),
        };
        self.pockets.insert(publication.pocket_key.clone(), pocket);
        self.enqueue(&publication.pocket_key, pocket_position);
    }

    /// `PublishNewKey`.
    fn publish_new_key(&mut self, publication: &Publication) -> PublishOutcome {
        let position = self.take_position();
        let record = RecordView::new(
            publication.record_key.clone(),
            publication.rank,
            publication.payload.clone(),
            position,
            RecordStatus::Queued,
        );
        self.pocket(&publication.pocket_key)
            .records
            .insert(publication.record_key.clone(), record);
        PublishOutcome::NewKey
    }

    /// `Supersede`: rank and payload in place; position and status, and a held
    /// version, untouched.
    fn supersede(&mut self, publication: &Publication) -> PublishOutcome {
        let pocket = self.pocket(&publication.pocket_key);
        let record = pocket
            .records
            .get_mut(&publication.record_key)
            .expect("the record a supersede replaces exists");
        record.rank = publication.rank;
        record.payload = publication.payload.clone();
        PublishOutcome::Superseded
    }

    /// `PocketTouched`: `ReviveIdlePocket` or `ReviveParkedPocket` for an idle or
    /// parked pocket; a queued or leased one is left where it is.
    fn touch(&mut self, key: &PocketKey) -> Option<PocketStatus> {
        let status = self.pockets.get(key)?.status;
        if !matches!(status, PocketStatus::Idle | PocketStatus::Parked) {
            return None;
        }
        let position = self.take_position();
        let pocket = self.pocket(key);
        pocket.status = PocketStatus::Queued;
        pocket.position = position;
        pocket.consecutive_expiries = 0;
        self.enqueue(key, position);
        Some(status)
    }

    /// `PullWithinLease`, `PullClaimsFrontPocket` or `PullFindsNoPocket`, with
    /// `HoldRecord` for each record in the batch. A claimed lease is pending, and
    /// the batch names a placeholder id, until the queue's answer names it.
    fn pull(
        &mut self,
        consumer: &ConsumerId,
        limit: i64,
        now: Timestamp,
        config: &Config,
    ) -> Result<PullOutcome, PullError> {
        let Some(active) = self.consumers.get(consumer).copied() else {
            return Err(PullError::UnknownConsumer(consumer.clone()));
        };
        let expires_at = now + config.lease_ttl;
        let (lease, pocket_key) = if let Some(lease) = active {
            let held = self.leases.get_mut(&lease).expect("an active lease");
            held.expires_at = expires_at;
            (lease, held.pocket_key.clone())
        } else {
            let Some(pocket_key) = self.front() else {
                return Ok(PullOutcome::NoPocketAvailable);
            };
            self.claim(consumer, &pocket_key, now, expires_at);
            (LeaseId::from_u128(0), pocket_key)
        };
        let size = usize::try_from(limit.clamp(0, i64::from(config.max_pull_batch))).unwrap();
        let records = self.hold(&pocket_key, size);
        Ok(PullOutcome::Batch {
            lease,
            pocket_key,
            expires_at,
            records,
        })
    }

    /// `min_by(Pockets where status = queued, position)`, read from the pockets,
    /// not from the queue's order index.
    fn front(&self) -> Option<PocketKey> {
        self.pockets
            .iter()
            .filter(|(_, pocket)| pocket.status == PocketStatus::Queued)
            .min_by_key(|(_, pocket)| pocket.position)
            .map(|(key, _)| key.clone())
    }

    fn claim(
        &mut self,
        consumer: &ConsumerId,
        key: &PocketKey,
        now: Timestamp,
        expires: Timestamp,
    ) {
        let pocket = self.pocket(key);
        pocket.status = PocketStatus::Leased;
        let entry = (pocket.position, key.clone());
        self.pocket_order.retain(|queued| queued != &entry);
        let lease = ModelLease {
            pocket_key: key.clone(),
            consumer_id: consumer.clone(),
            granted_at: now,
            expires_at: expires,
        };
        self.pending = Some(lease);
    }

    /// `oldest(pocket.queued_records, position, size)`, each held at its version.
    fn hold(&mut self, key: &PocketKey, size: usize) -> Vec<HeldRecord> {
        let pocket = self.pocket(key);
        let mut queued: Vec<&mut RecordView> = pocket
            .records
            .values_mut()
            .filter(|record| record.status == RecordStatus::Queued)
            .collect();
        queued.sort_by_key(|record| record.position);
        queued
            .into_iter()
            .take(size)
            .map(|record| {
                record.status = RecordStatus::Held {
                    rank: record.rank,
                    payload: record.payload.clone(),
                };
                HeldRecord::new(record.key.clone(), record.rank, record.payload.clone())
            })
            .collect()
    }

    /// Adopts the id the queue gave a claimed lease.
    fn adopt(&mut self, id: LeaseId) {
        if let Some(lease) = self.pending.take() {
            self.consumers.insert(lease.consumer_id.clone(), Some(id));
            self.leases.insert(id, lease);
        }
    }

    /// The guards every lease action shares, in the rules' order: the lease is
    /// active, then it is the consumer's.
    fn authorise(
        &self,
        consumer: &ConsumerId,
        lease: LeaseId,
    ) -> Result<PocketKey, LeaseActionError> {
        if !self.consumers.contains_key(consumer) {
            return Err(LeaseActionError::UnknownConsumer(consumer.clone()));
        }
        match self.leases.get(&lease) {
            None => Err(LeaseActionError::LeaseNotActive(lease)),
            Some(active) if &active.consumer_id != consumer => {
                Err(LeaseActionError::LeaseNotYours(lease))
            }
            Some(active) => Ok(active.pocket_key.clone()),
        }
    }

    fn extend(&mut self, lease: LeaseId, expires_at: Timestamp) {
        self.leases
            .get_mut(&lease)
            .expect("an active lease")
            .expires_at = expires_at;
    }

    /// `AckSettlesRecord` and `AckDefersNewerVersion`, or with `nack` the two nack
    /// rules, whose dead letter carries the held version.
    fn settle(
        &mut self,
        (nack, consumer, lease, key): (bool, &ConsumerId, LeaseId, &RecordKey),
        now: Timestamp,
        config: &Config,
    ) -> Result<SettleOutcome, LeaseActionError> {
        let pocket_key = self.authorise(consumer, lease)?;
        let record = self.pocket(&pocket_key).records.get(key).cloned();
        let Some(RecordView {
            rank,
            status:
                RecordStatus::Held {
                    rank: held_rank,
                    payload: held_payload,
                },
            ..
        }) = record
        else {
            return Err(LeaseActionError::RecordNotHeld(key.clone()));
        };
        self.extend(lease, now + config.lease_ttl);
        if nack {
            self.dead_letters.push(ModelLetter {
                pocket_key: pocket_key.clone(),
                key: key.clone(),
                rank: held_rank,
                payload: held_payload,
                nacked_by: consumer.clone(),
                nacked_at: now,
                expires_at: now + config.dead_letter_ttl,
            });
        } else {
            self.pocket(&pocket_key).consecutive_expiries = 0;
        }
        if held_rank == rank {
            self.pocket(&pocket_key).records.remove(key);
            return Ok(SettleOutcome::Settled);
        }
        let position = self.take_position();
        let record = self.pocket(&pocket_key).records.get_mut(key).expect("held");
        record.status = RecordStatus::Deferred;
        record.position = position;
        Ok(SettleOutcome::Deferred)
    }

    /// `Renew`.
    fn renew(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        now: Timestamp,
        lease_ttl: Duration,
    ) -> Result<Timestamp, LeaseActionError> {
        self.authorise(consumer, lease)?;
        self.extend(lease, now + lease_ttl);
        Ok(now + lease_ttl)
    }

    /// Ends `lease` with `status`: it leaves the registry and its consumer.
    fn end(&mut self, lease: LeaseId, status: LeaseStatus) -> Ended {
        let ended = self.leases.remove(&lease).expect("an active lease");
        self.consumers.insert(ended.consumer_id.clone(), None);
        (lease, ended, status)
    }

    /// `ReleaseRequeuesPocket` or `ReleaseIdlesEmptyPocket`, with `Freed` for each
    /// held and deferred record.
    fn release(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        now: Timestamp,
    ) -> Result<(PocketFate, Ended), LeaseActionError> {
        let pocket_key = self.authorise(consumer, lease)?;
        let ended = self.end(lease, LeaseStatus::Released { ended_at: now });
        let pocket = self.pocket(&pocket_key);
        free(pocket);
        pocket.consecutive_expiries = 0;
        if pocket.records.is_empty() {
            pocket.status = PocketStatus::Idle;
            return Ok((PocketFate::Idle, ended));
        }
        let position = self.take_position();
        let pocket = self.pocket(&pocket_key);
        pocket.status = PocketStatus::Queued;
        pocket.position = position;
        self.enqueue(&pocket_key, position);
        Ok((PocketFate::Queued, ended))
    }

    /// `LeaseLapses` for every active lease with `expires_at <= now`, in
    /// `(expires_at, lease id)` order, then `SupersededRecordDecays` and
    /// `DeadLetterDecays`; the counts are the decayed entries.
    fn tick(&mut self, now: Timestamp, limit: u32) -> (Vec<Lapse>, usize, usize) {
        let mut due: Vec<(Timestamp, LeaseId)> = self
            .leases
            .iter()
            .filter(|(_, lease)| lease.expires_at <= now)
            .map(|(id, lease)| (lease.expires_at, *id))
            .collect();
        due.sort();
        let lapsed = due
            .into_iter()
            .map(|(_, id)| self.lapse(id, now, limit))
            .collect();
        let (shed, letters) = (self.superseded.len(), self.dead_letters.len());
        self.superseded.retain(|entry| entry.expires_at > now);
        self.dead_letters.retain(|letter| letter.expires_at > now);
        let decayed = (
            shed - self.superseded.len(),
            letters - self.dead_letters.len(),
        );
        (lapsed, decayed.0, decayed.1)
    }

    /// `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` or
    /// `LeaseExpiryIdlesEmptyPocket`, by their guards on the pre-lapse count.
    fn lapse(&mut self, lease: LeaseId, now: Timestamp, limit: u32) -> Lapse {
        let ended = self.end(lease, LeaseStatus::Expired { ended_at: now });
        let key = ended.1.pocket_key.clone();
        let pocket = self.pocket(&key);
        let at_threshold = pocket.consecutive_expiries + 1 >= limit;
        free(pocket);
        pocket.consecutive_expiries += 1;
        pocket.status = if at_threshold {
            PocketStatus::Parked
        } else if pocket.records.is_empty() {
            PocketStatus::Idle
        } else {
            PocketStatus::Queued
        };
        let (status, position) = (pocket.status, pocket.position);
        if status == PocketStatus::Queued {
            self.enqueue(&key, position);
        }
        (ended, status)
    }
}

/// `Freed` for every record: `RequeueHeldRecord` and `RequeueDeferredRecord`; a
/// queued record matches neither and stays as it is. No position changes.
fn free(pocket: &mut ModelPocket) {
    for record in pocket.records.values_mut() {
        record.status = RecordStatus::Queued;
    }
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
    /// The `(pocket, record)` keys each lease has handed out and not settled.
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
    settled: Option<(PocketKey, RecordKey)>,
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
            settled: None,
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
    let (now, ttl) = (turn.now, turn.config.superseded_ttl);
    let (expected, revived) = turn.expected.publish(&id, &publication, now, ttl);
    let got = queue.publish(&id, publication);
    assert_eq!(got, expected, "publish");
    turn.reads = turn.pre.publishers.contains(&id);
    turn.reach(format!("publish {}", publish_label(&got)));
    if let Some(status) = revived {
        turn.reach(format!("revive {status:?}"));
    }
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

fn settle<R: PocketRecords, C: Clock, L: LeaseIds>(
    queue: &mut Queue<R, C, L>,
    turn: &mut Turn<'_>,
    (nack, consumer, choice, key): (bool, u8, LeaseChoice, u8),
) {
    let consumer = ConsumerId::new(CONSUMERS[usize::from(consumer)]);
    let lease = turn.resolve(&consumer, choice);
    let key = RecordKey::new(KEYS[usize::from(key)]);
    let late = turn.late(&consumer, lease);
    let (now, config) = (turn.now, turn.config.clone());
    let expected = turn
        .expected
        .settle((nack, &consumer, lease, &key), now, &config);
    let got = if nack {
        queue.nack(&consumer, lease, &key)
    } else {
        queue.ack(&consumer, lease, &key)
    };
    let name = if nack { "nack" } else { "ack" };
    assert_eq!(got, expected, "{name}");
    turn.reads = got.is_ok();
    turn.reach(format!("{name} {}", action_label(&got)));
    let pocket_key = turn
        .pre
        .leases
        .iter()
        .find(|l| l.id == lease)
        .map(|l| l.pocket_key.clone());
    let counted = (turn.pre.pockets.iter())
        .any(|p| Some(&p.key) == pocket_key.as_ref() && p.consecutive_expiries > 0);
    if counted && turn.reads {
        turn.reach(if nack {
            "nack keeps a counted pocket"
        } else {
            "ack resets a counted pocket"
        });
    }
    if got == Ok(SettleOutcome::Settled) {
        let settled = (
            pocket_key.expect("a settled record's lease was active"),
            key,
        );
        turn.walk.handed.entry(lease).or_default().remove(&settled);
        turn.settled = Some(settled);
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
    let expected = turn.expected.release(&consumer, lease, turn.now);
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
    let limit = turn.config.max_consecutive_expiries;
    let (lapses, shed, letters) = turn.expected.tick(turn.now, limit);
    let report = queue.tick();
    let got: Vec<Ended> = report.lapsed.iter().map(ended).collect();
    let expected: Vec<Ended> = lapses.iter().map(|(lease, _)| lease.clone()).collect();
    assert_eq!(got, expected, "the lapses, in (expires_at, lease id) order");
    assert_eq!(
        (report.decayed_superseded, report.decayed_dead_letters),
        (shed, letters),
        "the decays"
    );
    turn.reads = true;
    for (_, fate) in &lapses {
        turn.reach(format!("lapse {fate:?}"));
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
            settle(queue, &mut turn, (false, consumer, lease, key));
        }
        WalkStep::Nack {
            consumer,
            lease,
            key,
        } => {
            settle(queue, &mut turn, (true, consumer, lease, key));
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
        settled,
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
    invariants::check_transitions(&pre, post, settled.as_ref(), &lapsed, &walk.ended);
    for lease in lapsed {
        invariants::ended_after_grant(&lease);
        walk.ended.insert(lease.id, lease);
    }
    note_states(post, walk);
}

/// Labels every pocket state and every record state by its pocket's.
fn note_states(snapshot: &Snapshot, walk: &mut Walk) {
    for pocket in &snapshot.pockets {
        walk.reached.insert(format!("pocket {:?}", pocket.status));
        for record in &pocket.records {
            let status = match record.status {
                RecordStatus::Held { .. } => "Held",
                RecordStatus::Deferred => "Deferred",
                _ => "Queued",
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
