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

    /// The four publish rules, then `PocketTouched` for all but the first, which
    /// returns the status a revive moved the pocket from.
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

    /// `AckSettlesRecord` and `AckDefersNewerVersion`, or with `nack` the two nack
    /// rules, whose dead letter carries the held version.
    pub(super) fn settle(
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
                    ..
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
    /// held and deferred record.
    pub(super) fn release(
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
    pub(super) fn tick(&mut self, now: Timestamp, limit: u32) -> (Vec<Lapse>, usize, usize) {
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
