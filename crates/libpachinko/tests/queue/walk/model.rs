//! The walk's model: the queue's state as the rules read it, projected from a
//! `Snapshot`, and the rules applied to it, so the walk predicts each step's
//! outcome and the whole state after it.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use libpachinko::records::{Position, RecordStatus, RecordView};
use libpachinko::testing::{PocketStatus, PocketView, ShedReason, Snapshot};
use libpachinko::{
    Config, ConsumerId, HeldRecord, JoinError, Lease, LeaseActionError, LeaseId, LeaseStatus, Mark,
    Payload, PocketFate, PocketKey, Publication, PublishError, PublishOutcome, PublisherId,
    PullError, PullOutcome, Rank, RecordKey, SettleOutcome, Timestamp,
};

/// An active lease, as the rules see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ModelLease {
    pocket_key: PocketKey,
    pub(super) consumer_id: ConsumerId,
    granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
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
pub(super) type Ended = (LeaseId, ModelLease, LeaseStatus);

/// The `Freed` rules a lease end applied, one name per held record, smallest held
/// position first.
pub(super) type Rows = Vec<&'static str>;

/// One lapse the tick should process: the ended lease, its pocket's fate and the
/// `Freed` rules it applied.
pub(super) type Lapse = (Ended, PocketStatus, Rows);

/// The queue's state as the rules read it, projected from a `Snapshot`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Model {
    next_position: Position,
    publishers: BTreeSet<PublisherId>,
    consumers: BTreeMap<ConsumerId, Option<LeaseId>>,
    pockets: BTreeMap<PocketKey, ModelPocket>,
    pocket_order: Vec<(Position, PocketKey)>,
    pub(super) leases: BTreeMap<LeaseId, ModelLease>,
    /// The lease a claim creates, held apart from `leases` until the queue's answer
    /// names it: the id is the lease-id source's to choose, so the walk checks the
    /// queue's is fresh and then adopts it. The walk reserves no lease id.
    pub(super) pending: Option<ModelLease>,
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

pub(super) fn ended(lease: &Lease) -> Ended {
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

    pub(super) fn active_lease(&self, consumer: &ConsumerId) -> Option<LeaseId> {
        self.consumers.get(consumer).copied().flatten()
    }

    pub(super) fn publisher_joins(&mut self, id: &PublisherId) -> Result<(), JoinError> {
        if !self.publishers.insert(id.clone()) {
            return Err(JoinError::PublisherExists(id.clone()));
        }
        Ok(())
    }

    pub(super) fn consumer_joins(&mut self, id: &ConsumerId) -> Result<(), JoinError> {
        if self.consumers.contains_key(id) {
            return Err(JoinError::ConsumerExists(id.clone()));
        }
        self.consumers.insert(id.clone(), None);
        Ok(())
    }

    /// The five publish rules, then `PocketTouched` for the three that add or raise
    /// a record in an existing pocket, which returns the status a revive moved the
    /// pocket from. `DropStalePublish` does not touch the pocket, and an equal rank
    /// is rejected and changes nothing.
    pub(super) fn publish(
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
            Some(RecordView {
                rank,
                status:
                    RecordStatus::Held {
                        deferred_position: None,
                        ..
                    },
                ..
            }) if publication.rank > rank => self.defer_behind_held_version(publication),
            Some(record) if publication.rank > record.rank => {
                self.superseded
                    .push(shed(record.rank, record.payload, ShedReason::Superseded));
                self.supersede(publication)
            }
            Some(record) if publication.rank < record.rank => {
                let (rank, payload) = (publication.rank, publication.payload.clone());
                self.superseded.push(shed(rank, payload, ShedReason::Stale));
                return (Ok(PublishOutcome::Stale), None);
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

    /// `Supersede`, for a queued record or a held one with a version deferred
    /// behind it: rank and payload in place; position, status, the held version,
    /// its mark and the arrival position untouched. The caller sheds the loser.
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

    /// `DeferBehindHeldVersion`, for a held record with nothing deferred: rank and
    /// payload move on, the held version stays, nothing is shed, and the newer
    /// version takes the counter's value as its arrival position. The queue answers
    /// `Superseded`.
    fn defer_behind_held_version(&mut self, publication: &Publication) -> PublishOutcome {
        let arrival = self.take_position();
        let pocket = self.pocket(&publication.pocket_key);
        let record = pocket
            .records
            .get_mut(&publication.record_key)
            .expect("the record a version is deferred behind exists");
        record.rank = publication.rank;
        record.payload = publication.payload.clone();
        if let RecordStatus::Held {
            deferred_position, ..
        } = &mut record.status
        {
            *deferred_position = Some(arrival);
        }
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
    pub(super) fn pull(
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
                    mark: Mark::Unsettled,
                    nacked_at: None,
                    deferred_position: None,
                };
                HeldRecord::new(record.key.clone(), record.rank, record.payload.clone())
            })
            .collect()
    }

    /// Adopts the id the queue gave a claimed lease.
    pub(super) fn adopt(&mut self, id: LeaseId) {
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

    /// `AckMarksRecord` and `AckRepeats`, or with `Mark::Nacked` the two nack
    /// rules. The guards come first and read no clock: the lease is the consumer's
    /// and active, the record is held, and it is not marked the other way. A first
    /// mark extends the lease, sets the mark, stamps `nacked_at` for a nack and
    /// resets the pocket's `consecutive_expiries`; a repeat only extends the lease.
    /// Nothing is removed and no entry is made.
    pub(super) fn mark(
        &mut self,
        (mark, consumer, lease, key): (Mark, &ConsumerId, LeaseId, &RecordKey),
        now: Timestamp,
        config: &Config,
    ) -> Result<SettleOutcome, LeaseActionError> {
        let pocket_key = self.authorise(consumer, lease)?;
        let current = match self.pocket(&pocket_key).records.get(key) {
            Some(RecordView {
                status: RecordStatus::Held { mark: held, .. },
                ..
            }) => *held,
            _ => return Err(LeaseActionError::RecordNotHeld(key.clone())),
        };
        if current != Mark::Unsettled && current != mark {
            return Err(LeaseActionError::MarkConflict(key.clone()));
        }
        self.extend(lease, now + config.lease_ttl);
        if current == Mark::Unsettled {
            let pocket = self.pocket(&pocket_key);
            pocket.consecutive_expiries = 0;
            let record = pocket.records.get_mut(key).expect("held");
            if let RecordStatus::Held {
                mark: held_mark,
                nacked_at,
                ..
            } = &mut record.status
            {
                *held_mark = mark;
                *nacked_at = (mark == Mark::Nacked).then_some(now);
            }
        }
        Ok(if mark == Mark::Nacked {
            SettleOutcome::Nacked
        } else {
            SettleOutcome::Acked
        })
    }

    /// `Renew`.
    pub(super) fn renew(
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
    /// held record. The pocket is queued when a record outlasts the lease, which is
    /// when any is left once the `Freed` rules have run, and idle otherwise.
    pub(super) fn release(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        now: Timestamp,
        config: &Config,
    ) -> (Result<(PocketFate, Ended), LeaseActionError>, Rows) {
        let pocket_key = match self.authorise(consumer, lease) {
            Ok(key) => key,
            Err(refused) => return (Err(refused), Rows::new()),
        };
        let ended = self.end(lease, LeaseStatus::Released { ended_at: now });
        let rows = self.freed(&pocket_key, &ended.1.consumer_id, now, config);
        let pocket = self.pocket(&pocket_key);
        pocket.consecutive_expiries = 0;
        if pocket.records.is_empty() {
            pocket.status = PocketStatus::Idle;
            return (Ok((PocketFate::Idle, ended)), rows);
        }
        let position = self.take_position();
        let pocket = self.pocket(&pocket_key);
        pocket.status = PocketStatus::Queued;
        pocket.position = position;
        self.enqueue(&pocket_key, position);
        (Ok((PocketFate::Queued, ended)), rows)
    }

    /// `LeaseLapses` for every active lease with `expires_at <= now`, in
    /// `(expires_at, lease id)` order, then `SupersededRecordDecays` and
    /// `DeadLetterDecays` at the same `now`, so an entry a lapse makes with a zero
    /// TTL decays in that tick; the counts are the decayed entries.
    pub(super) fn tick(&mut self, now: Timestamp, config: &Config) -> (Vec<Lapse>, usize, usize) {
        let mut due: Vec<(Timestamp, LeaseId)> = self
            .leases
            .iter()
            .filter(|(_, lease)| lease.expires_at <= now)
            .map(|(id, lease)| (lease.expires_at, *id))
            .collect();
        due.sort();
        let lapsed = due
            .into_iter()
            .map(|(_, id)| self.lapse(id, now, config))
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
    /// `LeaseExpiryIdlesEmptyPocket`, by their guards on the pre-lapse count and on
    /// whether a record outlasts the lease, with `Freed` for each held record.
    fn lapse(&mut self, lease: LeaseId, now: Timestamp, config: &Config) -> Lapse {
        let ended = self.end(lease, LeaseStatus::Expired { ended_at: now });
        let key = ended.1.pocket_key.clone();
        let count = self.pocket(&key).consecutive_expiries;
        let at_threshold = count + 1 >= config.max_consecutive_expiries;
        let rows = self.freed(&key, &ended.1.consumer_id, now, config);
        let pocket = self.pocket(&key);
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
        (ended, status, rows)
    }

    /// `Freed` for every held record of the pocket, smallest held position first;
    /// a queued record is sent no `Freed` and stays as it is. The shed queues are
    /// kept by expiry, then arrival, as the queue keeps them.
    fn freed(
        &mut self,
        key: &PocketKey,
        consumer: &ConsumerId,
        ended_at: Timestamp,
        config: &Config,
    ) -> Rows {
        let mut held: Vec<(Position, RecordKey)> = self.pockets[key]
            .records
            .values()
            .filter(|record| matches!(record.status, RecordStatus::Held { .. }))
            .map(|record| (record.position, record.key.clone()))
            .collect();
        held.sort();
        let rows = held
            .into_iter()
            .map(|(_, record_key)| self.free(key, &record_key, (consumer, ended_at), config))
            .collect();
        self.superseded.sort_by_key(|entry| entry.expires_at);
        self.dead_letters.sort_by_key(|letter| letter.expires_at);
        rows
    }

    /// One `Freed`, answered by the rule its mark and its deferred version select,
    /// whose name it returns. `consumer` is the ended lease's; an entry the rule
    /// makes is stamped from `ended_at`.
    fn free(
        &mut self,
        key: &PocketKey,
        record_key: &RecordKey,
        (consumer, ended_at): (&ConsumerId, Timestamp),
        config: &Config,
    ) -> &'static str {
        let records = &mut self
            .pockets
            .get_mut(key)
            .expect("a lease's pocket exists")
            .records;
        let record = records
            .get_mut(record_key)
            .expect("Freed names a held record");
        let RecordStatus::Held {
            rank,
            payload,
            mark,
            nacked_at,
            deferred_position,
        } = record.status.clone()
        else {
            panic!("Freed names a held record");
        };
        let rule = freed_rule(mark, deferred_position);
        if rule == "SupersedeUnsettledHeldVersion" {
            self.superseded.push(ModelShed {
                pocket_key: key.clone(),
                key: record_key.clone(),
                rank,
                payload: payload.clone(),
                reason: ShedReason::Superseded,
                shed_at: ended_at,
                expires_at: ended_at + config.superseded_ttl,
            });
        }
        if mark == Mark::Nacked {
            self.dead_letters.push(ModelLetter {
                pocket_key: key.clone(),
                key: record_key.clone(),
                rank,
                payload,
                nacked_by: consumer.clone(),
                nacked_at: nacked_at.expect("NackedHasItsTime"),
                expires_at: ended_at + config.dead_letter_ttl,
            });
        }
        match (mark, deferred_position) {
            (Mark::Unsettled, _) => record.status = RecordStatus::Queued,
            (_, Some(arrival)) => {
                record.status = RecordStatus::Queued;
                record.position = arrival;
            }
            (_, None) => {
                records.remove(record_key);
            }
        }
        rule
    }
}

/// The one of the six rules that answers `Freed` for a held record with this mark
/// and this arrival position, by the reconciliation table's rows.
fn freed_rule(mark: Mark, deferred_position: Option<Position>) -> &'static str {
    match (mark, deferred_position) {
        (Mark::Unsettled, None) => "RequeueUnsettledRecord",
        (Mark::Unsettled, Some(_)) => "SupersedeUnsettledHeldVersion",
        (Mark::Acked, None) => "ConsumeAckedRecord",
        (Mark::Acked, Some(_)) => "ConsumeAckedVersionQueueDeferred",
        (_, None) => "DeadLetterNackedRecord",
        (_, Some(_)) => "DeadLetterNackedVersionQueueDeferred",
    }
}

/// A held record for the lease-end table: version 1 held, and version 2 deferred
/// behind it at `deferred` when that is given.
fn held_record(
    key: &str,
    position: u64,
    mark: Mark,
    nacked_at: Option<u64>,
    deferred: Option<u64>,
) -> (RecordKey, RecordView) {
    let held = Payload::from(format!("{key} held").into_bytes());
    let (rank, payload) = match deferred {
        Some(_) => (
            Rank::new(2),
            Payload::from(format!("{key} newer").into_bytes()),
        ),
        None => (Rank::new(1), held.clone()),
    };
    let status = RecordStatus::Held {
        rank: Rank::new(1),
        payload: held,
        mark,
        nacked_at: nacked_at.map(Timestamp::from_millis),
        deferred_position: deferred.map(Position::new),
    };
    let record = RecordView::new(
        RecordKey::new(key),
        rank,
        payload,
        Position::new(position),
        status,
    );
    (RecordKey::new(key), record)
}

/// A model with one leased pocket, `a`, holding one record per row of the
/// reconciliation table, keyed so that key order is the reverse of position
/// order, beside a queued record, `k0`.
fn six_rows() -> Model {
    let queued = RecordView::new(
        RecordKey::new("k0"),
        Rank::new(1),
        Payload::default(),
        Position::new(7),
        RecordStatus::Queued,
    );
    let records = BTreeMap::from([
        held_record("k6", 1, Mark::Unsettled, None, None),
        held_record("k5", 2, Mark::Unsettled, None, Some(10)),
        held_record("k4", 3, Mark::Acked, None, None),
        held_record("k3", 4, Mark::Acked, None, Some(11)),
        held_record("k2", 5, Mark::Nacked, Some(300), None),
        held_record("k1", 6, Mark::Nacked, Some(400), Some(12)),
        (RecordKey::new("k0"), queued),
    ]);
    let pocket = ModelPocket {
        status: PocketStatus::Leased,
        position: Position::new(0),
        consecutive_expiries: 0,
        records,
    };
    Model {
        next_position: Position::new(13),
        publishers: BTreeSet::new(),
        consumers: BTreeMap::new(),
        pockets: BTreeMap::from([(PocketKey::new("a"), pocket)]),
        pocket_order: Vec::new(),
        leases: BTreeMap::new(),
        pending: None,
        superseded: Vec::new(),
        dead_letters: Vec::new(),
    }
}

/// The model's own lease end, which the walk cannot run against the queue until
/// the queue follows the six `Freed` rules.
#[test]
fn freed_applies_the_six_rows_smallest_held_position_first() {
    let config = super::config();
    let (ended_at, consumer, a) = (
        Timestamp::from_millis(500),
        ConsumerId::new("c0"),
        PocketKey::new("a"),
    );
    let mut model = six_rows();
    let rows = model.freed(&a, &consumer, ended_at, &config);
    let table = [
        "RequeueUnsettledRecord",
        "SupersedeUnsettledHeldVersion",
        "ConsumeAckedRecord",
        "ConsumeAckedVersionQueueDeferred",
        "DeadLetterNackedRecord",
        "DeadLetterNackedVersionQueueDeferred",
    ];
    assert_eq!(rows, table, "smallest held position first, not key order");
    let after: Vec<(&str, u64, Rank, bool)> = (model.pockets[&a].records.values())
        .map(|r| {
            (
                r.key.as_str(),
                r.position.get(),
                r.rank,
                r.status == RecordStatus::Queued,
            )
        })
        .collect();
    let (one, two) = (Rank::new(1), Rank::new(2));
    assert_eq!(
        after,
        [
            ("k0", 7, one, true),
            ("k1", 12, two, true),
            ("k3", 11, two, true),
            ("k5", 2, two, true),
            ("k6", 1, one, true)
        ],
        "unsettled keep their position, marked take their arrival position, \
            marked with nothing deferred are removed"
    );
    let shed = ModelShed {
        pocket_key: a.clone(),
        key: RecordKey::new("k5"),
        rank: one,
        payload: Payload::from(b"k5 held".as_slice()),
        reason: ShedReason::Superseded,
        shed_at: ended_at,
        expires_at: ended_at + config.superseded_ttl,
    };
    assert_eq!(model.superseded, [shed]);
    let letter = |key: &str, nacked_at| ModelLetter {
        pocket_key: a.clone(),
        key: RecordKey::new(key),
        rank: one,
        payload: Payload::from(format!("{key} held").into_bytes()),
        nacked_by: consumer.clone(),
        nacked_at: Timestamp::from_millis(nacked_at),
        expires_at: ended_at + config.dead_letter_ttl,
    };
    assert_eq!(model.dead_letters, [letter("k2", 300), letter("k1", 400)]);
}
