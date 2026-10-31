//! The pocket machine: one partition's pockets, their records, the leases on them,
//! and the lapse, with every publish, pull, ack, nack, renew, release, expiry and
//! decay rule as a branch. Decomposition B of ticket T20.
//!
//! `step` dispatches in two layers: the input to its trigger handler (`on_publish`,
//! `on_claim`, ...), the handler to one function per rule of the module, named after
//! the rule (`publish_into_new_pocket`, `supersede`, `lease_expiry_parks_pocket`, ...).
//! The handlers resolve the guards and reply with the first guard that failed; the rule
//! functions apply the `ensures`. `now` enters only as the step's argument.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::machine::Machine;
use crate::protocol::{
    Became, ConsumerId, HandedRecord, Input, LeaseId, MachineId, Output, Partition, Payload,
    PocketKey, Published, Rank, Reason, RecordKey, Response,
};
use crate::state::{
    DeadLetter, Lease, LeaseStatus, Pocket, PocketStatus, Record, RecordStatus, ShedReason,
    SupersededRecord,
};
use crate::time::{Config, Timestamp};

#[cfg(test)]
mod tests;

/// The pocket machine for one partition, `MachineId::Partition(partition)`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PocketMachine {
    partition: Partition,
    config: Config,
    pockets: BTreeMap<PocketKey, Pocket>,
    /// Which pocket each lease ever granted here is on, active or ended.
    leases: BTreeMap<LeaseId, PocketKey>,
    /// The single writer of this partition's record positions.
    positions: RecordPositions,
    violations: u64,
}

impl PocketMachine {
    /// A pocket machine for `partition` out of `partitions`, with nothing in it.
    ///
    /// ```
    /// use worker_pool::{Config, Partition, PocketMachine};
    /// let machine = PocketMachine::new(Partition::new(2), 4, Config::default());
    /// assert_eq!(machine.partition(), Partition::new(2));
    /// assert_eq!(machine.pockets().count(), 0);
    /// ```
    #[must_use]
    pub const fn new(partition: Partition, partitions: u32, config: Config) -> Self {
        Self {
            partition,
            config,
            pockets: BTreeMap::new(),
            leases: BTreeMap::new(),
            positions: RecordPositions {
                serial: 0,
                partitions,
            },
            violations: 0,
        }
    }

    /// The partition this machine owns.
    #[must_use]
    pub const fn partition(&self) -> Partition {
        self.partition
    }

    /// The configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// One pocket.
    #[must_use]
    pub fn pocket(&self, key: &PocketKey) -> Option<&Pocket> {
        self.pockets.get(key)
    }

    /// Every pocket, by key.
    pub fn pockets(&self) -> impl Iterator<Item = (&PocketKey, &Pocket)> {
        self.pockets.iter()
    }

    /// The lease with this identity, active or ended, and its pocket.
    #[must_use]
    pub fn lease(&self, id: LeaseId) -> Option<(&PocketKey, &Lease)> {
        let key = self.leases.get(&id)?;
        let lease = self.pockets.get(key)?.lease(id)?;
        Some((key, lease))
    }

    /// The serial the next record position is built from.
    #[must_use]
    pub const fn next_record_serial(&self) -> u64 {
        self.positions.serial
    }

    /// The position the next record takes: the serial tagged with the partition.
    #[must_use]
    pub const fn next_record_position(&self) -> u64 {
        self.positions.next(self.partition)
    }

    /// An input the protocol never sends this machine in this state, under a client's
    /// reply handle: counted, and answered `Rejected(Protocol)`.
    fn protocol_violation(&mut self) -> Vec<Output> {
        self.violations += 1;
        reject(Reason::Protocol)
    }

    /// `Publish`: the four publish rules, then `PocketTouched` for an existing pocket.
    fn on_publish(&mut self, pocket_key: PocketKey, version: Version, at: At) -> Vec<Output> {
        let Some(pocket) = self.pockets.get_mut(&pocket_key) else {
            return self.publish_into_new_pocket(pocket_key, version);
        };
        let branch = match pocket.records.get(&version.key) {
            None => PublishBranch::NewKey,
            Some(record) => match version.rank.cmp(&record.rank) {
                Ordering::Greater => PublishBranch::Supersede,
                Ordering::Less => PublishBranch::Stale,
                Ordering::Equal => return reject(Reason::EqualRank),
            },
        };
        let published = match branch {
            PublishBranch::NewKey => {
                let position = self.positions.take(self.partition);
                publish_new_key(pocket, version, position)
            }
            PublishBranch::Supersede => supersede(pocket, version, at),
            PublishBranch::Stale => drop_stale_publish(pocket, version, at),
        };
        let mut outputs: Vec<Output> = pocket_touched(pocket, self.partition).into_iter().collect();
        outputs.push(Output::Reply(Response::Published(published)));
        outputs
    }

    /// `PublishIntoNewPocket`: the pocket is created queued with the record in it, and
    /// the order machine is asked for its position.
    fn publish_into_new_pocket(&mut self, pocket_key: PocketKey, version: Version) -> Vec<Output> {
        let mut pocket = Pocket::new(pocket_key.clone());
        let position = self.positions.take(self.partition);
        pocket
            .records
            .insert(version.key.clone(), queued_record(version, position));
        self.pockets.insert(pocket_key.clone(), pocket);
        vec![
            enqueue(pocket_key, self.partition),
            Output::Reply(Response::Published(Published::NewPocket)),
        ]
    }

    /// `Claim`: the order machine took the pocket off the front, so it is queued here;
    /// anything else is a protocol violation.
    fn on_claim(&mut self, pocket_key: PocketKey, claim: Claim, at: At) -> Vec<Output> {
        match self.pockets.get_mut(&pocket_key) {
            Some(pocket) if pocket.status == PocketStatus::Queued => {
                self.leases.insert(claim.lease, pocket_key.clone());
                pull_claims_front_pocket(pocket, pocket_key, claim, at)
            }
            _ => self.protocol_violation(),
        }
    }

    /// `PullUnderLease`: `PullWithinLease` when the lease is still active; a `RetryPull`
    /// back to the order machine when it has ended, since the order machine's view was
    /// stale; a protocol violation otherwise.
    fn on_pull_under_lease(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        limit: u64,
        at: At,
    ) -> Vec<Output> {
        let Some(under) = lease_pocket(&mut self.pockets, &self.leases, lease) else {
            return self.protocol_violation();
        };
        let held = under.lease_ref();
        if held.status != LeaseStatus::Active {
            let retry = Input::RetryPull {
                consumer: consumer.clone(),
                limit,
            };
            return vec![Output::Forward(MachineId::Order, retry)];
        }
        if held.consumer != *consumer || under.pocket.status != PocketStatus::Leased {
            return self.protocol_violation();
        }
        pull_within_lease(under, limit, at)
    }

    /// `Ack`: the lease guards, the record guards, then settle or defer.
    fn on_ack(
        &mut self,
        consumer: &ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
        at: At,
    ) -> Vec<Output> {
        let under = match guarded_lease(&mut self.pockets, &self.leases, consumer, lease) {
            Ok(under) => under,
            Err(reason) => return reject(reason),
        };
        match settlement(under.pocket, record_key) {
            Err(reason) => reject(reason),
            Ok(Settlement::Settle) => ack_settles_record(under, record_key, at),
            Ok(Settlement::Defer) => {
                let position = self.positions.take(self.partition);
                ack_defers_newer_version(under, record_key, position, at)
            }
            Ok(Settlement::HeldNewer) => self.protocol_violation(),
        }
    }

    /// `Nack`: the same guards as `Ack`, then settle or defer with a dead letter.
    fn on_nack(
        &mut self,
        consumer: ConsumerId,
        lease: LeaseId,
        record_key: &RecordKey,
        at: At,
    ) -> Vec<Output> {
        let under = match guarded_lease(&mut self.pockets, &self.leases, &consumer, lease) {
            Ok(under) => under,
            Err(reason) => return reject(reason),
        };
        match settlement(under.pocket, record_key) {
            Err(reason) => reject(reason),
            Ok(Settlement::Settle) => nack_settles_record(under, record_key, consumer, at),
            Ok(Settlement::Defer) => {
                let position = self.positions.take(self.partition);
                nack_defers_newer_version(under, record_key, consumer, position, at)
            }
            Ok(Settlement::HeldNewer) => self.protocol_violation(),
        }
    }

    /// `Renew`: the lease guards, then the extension.
    fn on_renew(&mut self, consumer: &ConsumerId, lease: LeaseId, at: At) -> Vec<Output> {
        match guarded_lease(&mut self.pockets, &self.leases, consumer, lease) {
            Ok(under) => renew(under, at),
            Err(reason) => reject(reason),
        }
    }

    /// `Release`: the lease guards and the pocket guard, then one of the two release
    /// rules by whether the pocket has records; the order machine hears what it became.
    fn on_release(&mut self, consumer: ConsumerId, lease: LeaseId, at: At) -> Vec<Output> {
        let under = match guarded_lease(&mut self.pockets, &self.leases, &consumer, lease) {
            Ok(under) => under,
            Err(reason) => return reject(reason),
        };
        if under.pocket.status != PocketStatus::Leased {
            return reject(Reason::PocketNotLeased);
        }
        let pocket_key = under.pocket.key.clone();
        let became = if under.pocket.records.is_empty() {
            release_idles_empty_pocket(under, at)
        } else {
            release_requeues_pocket(under, at)
        };
        vec![
            pocket_became(pocket_key, became, consumer, lease),
            Output::Reply(Response::Released),
        ]
    }

    /// `Tick`: `LeaseLapses` for every pocket in key order, then the two decays. Never
    /// a reply.
    fn on_tick(&mut self, at: At) -> Vec<Output> {
        let mut outputs = Vec::new();
        for pocket in self.pockets.values_mut() {
            match lease_lapses(pocket, at) {
                Lapse::NotYet => {}
                Lapse::Ended(output) => outputs.push(output),
                Lapse::NotLeased => self.violations += 1,
            }
            superseded_record_decays(pocket, at);
            dead_letter_decays(pocket, at);
        }
        outputs
    }
}

impl Machine for PocketMachine {
    fn id(&self) -> MachineId {
        MachineId::Partition(self.partition)
    }

    fn step(&mut self, input: Input, now: Timestamp) -> Vec<Output> {
        let at = At {
            now,
            config: self.config,
        };
        match input {
            Input::Publish {
                pocket_key,
                record_key,
                rank,
                payload,
                ..
            } => {
                let version = Version {
                    key: record_key,
                    rank,
                    payload,
                };
                self.on_publish(pocket_key, version, at)
            }
            Input::Claim {
                pocket_key,
                consumer,
                lease,
                limit,
            } => {
                let claim = Claim {
                    consumer,
                    lease,
                    limit,
                };
                self.on_claim(pocket_key, claim, at)
            }
            Input::PullUnderLease {
                consumer,
                lease,
                limit,
            } => self.on_pull_under_lease(&consumer, lease, limit, at),
            Input::Ack {
                consumer,
                lease,
                record_key,
            } => self.on_ack(&consumer, lease, &record_key, at),
            Input::Nack {
                consumer,
                lease,
                record_key,
            } => self.on_nack(consumer, lease, &record_key, at),
            Input::Renew { consumer, lease } => self.on_renew(&consumer, lease, at),
            Input::Release { consumer, lease } => self.on_release(consumer, lease, at),
            Input::Tick => self.on_tick(at),
            Input::Enqueue { .. } | Input::PocketBecame { .. } => {
                // Never sent here, and carrying no reply handle: counted, nothing more.
                self.violations += 1;
                Vec::new()
            }
            _ => self.protocol_violation(),
        }
    }

    fn violations(&self) -> u64 {
        self.violations
    }
}

/// The record position counter: `serial * partitions + partition.index()`, so that no
/// two records anywhere share a position (`UniqueRecordPosition`) and this machine is
/// the single writer of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecordPositions {
    serial: u64,
    /// How many partitions the pool has.
    partitions: u32,
}

impl RecordPositions {
    /// The position the next record takes.
    const fn next(self, partition: Partition) -> u64 {
        self.serial * (self.partitions as u64) + (partition.index() as u64)
    }

    /// Hands out the next position and advances the serial: `queue.next_position =
    /// position + 1` for records.
    const fn take(&mut self, partition: Partition) -> u64 {
        let position = self.next(partition);
        self.serial += 1;
        position
    }
}

/// The step's `now` and the configuration: what every `ensures` that stamps a time
/// reads.
#[derive(Clone, Copy, Debug)]
struct At {
    now: Timestamp,
    config: Config,
}

impl At {
    /// `now + config.lease_ttl`.
    const fn lease_expiry(self) -> Timestamp {
        self.now.plus(self.config.lease_ttl)
    }

    /// `now + config.superseded_ttl`.
    const fn superseded_expiry(self) -> Timestamp {
        self.now.plus(self.config.superseded_ttl)
    }

    /// `now + config.dead_letter_ttl`.
    const fn dead_letter_expiry(self) -> Timestamp {
        self.now.plus(self.config.dead_letter_ttl)
    }
}

/// One publish of a key: the record key, the rank and the payload.
#[derive(Clone, Debug)]
struct Version {
    key: RecordKey,
    rank: Rank,
    payload: Payload,
}

/// Which publish rule a publish into an existing pocket falls under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublishBranch {
    NewKey,
    Supersede,
    Stale,
}

/// What the order machine forwards in a `Claim`, less the pocket key.
#[derive(Clone, Debug)]
struct Claim {
    consumer: ConsumerId,
    lease: LeaseId,
    limit: u64,
}

/// A pocket together with the index of one of its leases: what the rules that act
/// under a lease receive once the guards have held.
#[derive(Debug)]
struct UnderLease<'a> {
    pocket: &'a mut Pocket,
    /// The lease's index in `pocket.leases`, found in the same step, so it is in bounds.
    index: usize,
}

impl UnderLease<'_> {
    fn lease_ref(&self) -> &Lease {
        &self.pocket.leases[self.index]
    }

    fn lease(&mut self) -> &mut Lease {
        &mut self.pocket.leases[self.index]
    }
}

/// Which of the two settling branches a held record falls under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Settlement {
    /// `held_rank = rank`: the held version is the latest.
    Settle,
    /// `held_rank < rank`: a newer version arrived while held.
    Defer,
    /// `held_rank > rank`, which `HeldVersionNotNewer` rules out.
    HeldNewer,
}

/// What `LeaseLapses` found in one pocket.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Lapse {
    /// No active lease has lapsed.
    NotYet,
    /// A lease lapsed and the pocket told the order machine what it became.
    Ended(Output),
    /// A lease lapsed on a pocket that is not leased, which the invariants rule out.
    NotLeased,
}

fn reply(response: Response) -> Vec<Output> {
    vec![Output::Reply(response)]
}

fn reject(reason: Reason) -> Vec<Output> {
    reply(Response::Rejected(reason))
}

/// `Enqueue` to the order machine: a pocket became queued and wants a position.
const fn enqueue(pocket_key: PocketKey, partition: Partition) -> Output {
    Output::Notify(
        MachineId::Order,
        Input::Enqueue {
            pocket_key,
            partition,
        },
    )
}

/// `PocketBecame` to the order machine: a lease ended and the pocket left `leased`.
const fn pocket_became(
    pocket_key: PocketKey,
    became: Became,
    consumer: ConsumerId,
    lease: LeaseId,
) -> Output {
    Output::Notify(
        MachineId::Order,
        Input::PocketBecame {
            pocket_key,
            became,
            consumer,
            lease,
        },
    )
}

/// The lease with this identity and its pocket, if this machine ever granted it.
fn lease_pocket<'a>(
    pockets: &'a mut BTreeMap<PocketKey, Pocket>,
    leases: &BTreeMap<LeaseId, PocketKey>,
    id: LeaseId,
) -> Option<UnderLease<'a>> {
    let pocket = pockets.get_mut(leases.get(&id)?)?;
    let index = pocket.leases.iter().position(|lease| lease.id == id)?;
    Some(UnderLease { pocket, index })
}

/// The guards ack, nack, renew and release share, first failure first: the lease is
/// known (`UnknownLease`), active (`LeaseNotActive`) and the consumer's
/// (`NotYourLease`). An ended lease stays rejected however the pocket was leased since.
fn guarded_lease<'a>(
    pockets: &'a mut BTreeMap<PocketKey, Pocket>,
    leases: &BTreeMap<LeaseId, PocketKey>,
    consumer: &ConsumerId,
    id: LeaseId,
) -> Result<UnderLease<'a>, Reason> {
    let under = lease_pocket(pockets, leases, id).ok_or(Reason::UnknownLease)?;
    let lease = under.lease_ref();
    if lease.status != LeaseStatus::Active {
        return Err(Reason::LeaseNotActive);
    }
    if lease.consumer != *consumer {
        return Err(Reason::NotYourLease);
    }
    Ok(under)
}

/// The guards ack and nack add: the record exists in the lease's pocket
/// (`NoSuchRecord`) and is held (`RecordNotHeld`); then which branch applies.
fn settlement(pocket: &Pocket, record_key: &RecordKey) -> Result<Settlement, Reason> {
    let record = pocket.records.get(record_key).ok_or(Reason::NoSuchRecord)?;
    if record.status != RecordStatus::Held {
        return Err(Reason::RecordNotHeld);
    }
    Ok(match record.held_rank.map(|held| held.cmp(&record.rank)) {
        Some(Ordering::Equal) => Settlement::Settle,
        Some(Ordering::Less) => Settlement::Defer,
        Some(Ordering::Greater) | None => Settlement::HeldNewer,
    })
}

/// `Record.created(..., status: queued)`, as both publish rules that create one say.
fn queued_record(version: Version, position: u64) -> Record {
    Record {
        key: version.key,
        rank: version.rank,
        payload: version.payload,
        position,
        status: RecordStatus::Queued,
        held_rank: None,
        held_payload: None,
    }
}

/// `PublishNewKey`: a queued record at a fresh position in an existing pocket.
fn publish_new_key(pocket: &mut Pocket, version: Version, position: u64) -> Published {
    pocket
        .records
        .insert(version.key.clone(), queued_record(version, position));
    Published::NewKey
}

/// `Supersede`: the stored version becomes a superseded record and the record takes the
/// new rank and payload; position, status and the held version stay as they were.
fn supersede(pocket: &mut Pocket, version: Version, at: At) -> Published {
    if let Some(record) = pocket.records.get_mut(&version.key) {
        pocket.superseded.push(SupersededRecord {
            key: record.key.clone(),
            rank: record.rank,
            payload: std::mem::replace(&mut record.payload, version.payload),
            reason: ShedReason::Superseded,
            shed_at: at.now,
            expires_at: at.superseded_expiry(),
        });
        record.rank = version.rank;
    }
    Published::Superseded
}

/// `DropStalePublish`: the arriving version becomes a superseded record and the record
/// is untouched.
fn drop_stale_publish(pocket: &mut Pocket, version: Version, at: At) -> Published {
    pocket.superseded.push(SupersededRecord {
        key: version.key,
        rank: version.rank,
        payload: version.payload,
        reason: ShedReason::Stale,
        shed_at: at.now,
        expires_at: at.superseded_expiry(),
    });
    Published::Stale
}

/// `PocketTouched`, which every publish into an existing pocket emits: the two revive
/// rules answer it when the pocket is idle or parked; a queued or leased pocket is left
/// as it is.
fn pocket_touched(pocket: &mut Pocket, partition: Partition) -> Option<Output> {
    match pocket.status {
        PocketStatus::Idle => Some(revive_idle_pocket(pocket, partition)),
        PocketStatus::Parked => Some(revive_parked_pocket(pocket, partition)),
        PocketStatus::Queued | PocketStatus::Leased => None,
    }
}

/// `ReviveIdlePocket`: queued again, the expiry count reset, and a fresh position asked
/// of the order machine.
fn revive_idle_pocket(pocket: &mut Pocket, partition: Partition) -> Output {
    pocket.status = PocketStatus::Queued;
    pocket.consecutive_expiries = 0;
    enqueue(pocket.key.clone(), partition)
}

/// `ReviveParkedPocket`: the same as the idle revival, from `parked`.
fn revive_parked_pocket(pocket: &mut Pocket, partition: Partition) -> Output {
    pocket.status = PocketStatus::Queued;
    pocket.consecutive_expiries = 0;
    enqueue(pocket.key.clone(), partition)
}

/// `PullClaimsFrontPocket`: the lease is created with the identity the order machine
/// minted, the pocket becomes leased, and the batch is handed and held.
fn pull_claims_front_pocket(
    pocket: &mut Pocket,
    pocket_key: PocketKey,
    claim: Claim,
    at: At,
) -> Vec<Output> {
    pocket.leases.push(Lease {
        id: claim.lease,
        consumer: claim.consumer,
        status: LeaseStatus::Active,
        granted_at: at.now,
        expires_at: at.lease_expiry(),
        ended_at: None,
    });
    pocket.status = PocketStatus::Leased;
    let records = hold_batch(pocket, claim.limit, at.config);
    reply(Response::Batch {
        lease: claim.lease,
        pocket_key,
        records,
    })
}

/// `PullWithinLease`: the lease is extended and the next batch handed and held.
fn pull_within_lease(mut under: UnderLease<'_>, limit: u64, at: At) -> Vec<Output> {
    under.lease().expires_at = at.lease_expiry();
    let lease = under.lease_ref().id;
    let pocket_key = under.pocket.key.clone();
    let records = hold_batch(under.pocket, limit, at.config);
    reply(Response::Batch {
        lease,
        pocket_key,
        records,
    })
}

/// `oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))`,
/// with `Handed` for each: the queued records with the smallest positions, in that
/// order, held as they are handed.
fn hold_batch(pocket: &mut Pocket, limit: u64, config: Config) -> Vec<HandedRecord> {
    let n = usize::try_from(limit.min(config.max_pull_batch)).unwrap_or(usize::MAX);
    let mut queued: Vec<(u64, RecordKey)> = pocket
        .records
        .values()
        .filter(|record| record.status == RecordStatus::Queued)
        .map(|record| (record.position, record.key.clone()))
        .collect();
    queued.sort_unstable();
    queued.truncate(n);
    queued
        .iter()
        .filter_map(|(_, key)| pocket.records.get_mut(key).map(hold_record))
        .collect()
}

/// `HoldRecord`: the record becomes held and its held version is the stored one.
fn hold_record(record: &mut Record) -> HandedRecord {
    record.status = RecordStatus::Held;
    record.held_rank = Some(record.rank);
    record.held_payload = Some(record.payload.clone());
    HandedRecord::new(record.key.clone(), record.rank, record.payload.clone())
}

/// `AckSettlesRecord`: the lease is extended and the record is gone.
fn ack_settles_record(mut under: UnderLease<'_>, record_key: &RecordKey, at: At) -> Vec<Output> {
    under.lease().expires_at = at.lease_expiry();
    under.pocket.records.remove(record_key);
    reply(Response::Acked)
}

/// `AckDefersNewerVersion`: the lease is extended and the record restarts deferred at
/// a fresh position with no held version.
fn ack_defers_newer_version(
    mut under: UnderLease<'_>,
    record_key: &RecordKey,
    position: u64,
    at: At,
) -> Vec<Output> {
    under.lease().expires_at = at.lease_expiry();
    if let Some(record) = under.pocket.records.get_mut(record_key) {
        defer_record(record, position);
    }
    reply(Response::Acked)
}

/// `NackSettlesRecord`: as the ack, with the held version dead-lettered.
fn nack_settles_record(
    mut under: UnderLease<'_>,
    record_key: &RecordKey,
    nacked_by: ConsumerId,
    at: At,
) -> Vec<Output> {
    under.lease().expires_at = at.lease_expiry();
    if let Some(record) = under.pocket.records.remove(record_key) {
        let letter = dead_letter(&record, nacked_by, at);
        under.pocket.dead_letters.push(letter);
    }
    reply(Response::Nacked)
}

/// `NackDefersNewerVersion`: as the ack, with the held version dead-lettered.
fn nack_defers_newer_version(
    mut under: UnderLease<'_>,
    record_key: &RecordKey,
    nacked_by: ConsumerId,
    position: u64,
    at: At,
) -> Vec<Output> {
    under.lease().expires_at = at.lease_expiry();
    if let Some(record) = under.pocket.records.get_mut(record_key) {
        let letter = dead_letter(record, nacked_by, at);
        under.pocket.dead_letters.push(letter);
        defer_record(record, position);
    }
    reply(Response::Nacked)
}

/// `DeadLetter.created(...)` from a held record: the held version, not the latest.
fn dead_letter(record: &Record, nacked_by: ConsumerId, at: At) -> DeadLetter {
    DeadLetter {
        key: record.key.clone(),
        rank: record.held_rank.unwrap_or(record.rank),
        payload: record
            .held_payload
            .clone()
            .unwrap_or_else(|| record.payload.clone()),
        nacked_by,
        nacked_at: at.now,
        expires_at: at.dead_letter_expiry(),
    }
}

/// The deferral both defer rules ensure: deferred, at the fresh position, held version
/// null.
fn defer_record(record: &mut Record, position: u64) {
    record.status = RecordStatus::Deferred;
    record.position = position;
    record.held_rank = None;
    record.held_payload = None;
}

/// `Renew`: the lease is extended.
fn renew(mut under: UnderLease<'_>, at: At) -> Vec<Output> {
    let expires_at = at.lease_expiry();
    under.lease().expires_at = expires_at;
    reply(Response::Renewed { expires_at })
}

/// `ReleaseRequeuesPocket`: the lease is released, every held and deferred record is
/// freed, and the pocket goes back to queued at the back with its expiry count reset.
fn release_requeues_pocket(mut under: UnderLease<'_>, at: At) -> Became {
    end_lease(under.lease(), LeaseStatus::Released, at.now);
    free_records(under.pocket);
    under.pocket.status = PocketStatus::Queued;
    under.pocket.consecutive_expiries = 0;
    Became::RequeuedAtBack
}

/// `ReleaseIdlesEmptyPocket`: the lease is released and the pocket, with no records,
/// goes idle with its expiry count reset.
fn release_idles_empty_pocket(mut under: UnderLease<'_>, at: At) -> Became {
    end_lease(under.lease(), LeaseStatus::Released, at.now);
    under.pocket.status = PocketStatus::Idle;
    under.pocket.consecutive_expiries = 0;
    Became::Idle
}

/// `lease.status = released | expired`, `lease.ended_at = now`.
const fn end_lease(lease: &mut Lease, status: LeaseStatus, now: Timestamp) {
    lease.status = status;
    lease.ended_at = Some(now);
}

/// `Freed` for every held and deferred record, as release and expiry emit it; the two
/// requeue rules answer by status.
fn free_records(pocket: &mut Pocket) {
    for record in pocket.records.values_mut() {
        match record.status {
            RecordStatus::Held => requeue_held_record(record),
            RecordStatus::Deferred => requeue_deferred_record(record),
            RecordStatus::Queued => {}
        }
    }
}

/// `RequeueHeldRecord`: queued again with no held version; the position is kept.
fn requeue_held_record(record: &mut Record) {
    record.status = RecordStatus::Queued;
    record.held_rank = None;
    record.held_payload = None;
}

/// `RequeueDeferredRecord`: queued again at the position the deferral gave it.
const fn requeue_deferred_record(record: &mut Record) {
    record.status = RecordStatus::Queued;
}

/// `LeaseLapses`: the active lease whose `expires_at` has passed emits `LeaseLapsed`,
/// which the three expiry rules answer. Fires only here, on a tick: an action that
/// arrives after `expires_at` but before the tick is judged by status alone.
fn lease_lapses(pocket: &mut Pocket, at: At) -> Lapse {
    let lapsed = |lease: &Lease| lease.status == LeaseStatus::Active && lease.expires_at <= at.now;
    let Some(index) = pocket.leases.iter().position(lapsed) else {
        return Lapse::NotYet;
    };
    let consumer = pocket.leases[index].consumer.clone();
    let lease = pocket.leases[index].id;
    let pocket_key = pocket.key.clone();
    match lease_lapsed(UnderLease { pocket, index }, at) {
        Some(became) => Lapse::Ended(pocket_became(pocket_key, became, consumer, lease)),
        None => Lapse::NotLeased,
    }
}

/// `LeaseLapsed`: the three expiry rules with disjoint guards. Parking has no records
/// guard, so an empty pocket parks at the threshold too. `None` when the pocket is not
/// leased, which no rule covers.
fn lease_lapsed(under: UnderLease<'_>, at: At) -> Option<Became> {
    if under.pocket.status != PocketStatus::Leased {
        return None;
    }
    let next_count = under.pocket.consecutive_expiries.saturating_add(1);
    Some(if next_count >= at.config.max_consecutive_expiries {
        lease_expiry_parks_pocket(under, at)
    } else if under.pocket.records.is_empty() {
        lease_expiry_idles_empty_pocket(under, at)
    } else {
        lease_expiry_keeps_pocket_at_front(under, at)
    })
}

/// `LeaseExpiryParksPocket`: the lease expires, the records are freed, the count rises
/// to the threshold and the pocket is set aside until a publish revives it.
fn lease_expiry_parks_pocket(mut under: UnderLease<'_>, at: At) -> Became {
    end_lease(under.lease(), LeaseStatus::Expired, at.now);
    free_records(under.pocket);
    under.pocket.consecutive_expiries = under.pocket.consecutive_expiries.saturating_add(1);
    under.pocket.status = PocketStatus::Parked;
    Became::Parked
}

/// `LeaseExpiryKeepsPocketAtFront`: the lease expires, the records are freed, the count
/// rises and the pocket is queued again at the position it had.
fn lease_expiry_keeps_pocket_at_front(mut under: UnderLease<'_>, at: At) -> Became {
    end_lease(under.lease(), LeaseStatus::Expired, at.now);
    free_records(under.pocket);
    under.pocket.consecutive_expiries = under.pocket.consecutive_expiries.saturating_add(1);
    under.pocket.status = PocketStatus::Queued;
    Became::RequeuedKeepingPosition
}

/// `LeaseExpiryIdlesEmptyPocket`: the lease expires, the count rises and the pocket,
/// with no records, goes idle.
fn lease_expiry_idles_empty_pocket(mut under: UnderLease<'_>, at: At) -> Became {
    end_lease(under.lease(), LeaseStatus::Expired, at.now);
    under.pocket.consecutive_expiries = under.pocket.consecutive_expiries.saturating_add(1);
    under.pocket.status = PocketStatus::Idle;
    Became::Idle
}

/// `SupersededRecordDecays`: every superseded record whose `expires_at` has passed is
/// gone.
fn superseded_record_decays(pocket: &mut Pocket, at: At) {
    pocket.superseded.retain(|shed| shed.expires_at > at.now);
}

/// `DeadLetterDecays`: every dead letter whose `expires_at` has passed is gone.
fn dead_letter_decays(pocket: &mut Pocket, at: At) {
    pocket
        .dead_letters
        .retain(|letter| letter.expires_at > at.now);
}
