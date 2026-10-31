//! The queue's entities, as the rules in the sibling modules read and change them,
//! and the read model cloned out of them.

use std::collections::{BTreeMap, BTreeSet};

use super::view::{
    DeadLetter, LeaseStatus, LeaseView, PocketStatus, PocketView, RecordStatus, RecordView,
    Snapshot, SupersededRecord,
};
use crate::{
    Config, ConsumerId, LeaseId, Payload, PocketKey, Position, PublisherId, Rank, RecordKey,
    Timestamp,
};

/// A record in its pocket. Its key is the pocket's map key.
#[derive(Clone, Debug)]
pub(super) struct Record {
    pub(super) rank: Rank,
    pub(super) payload: Payload,
    pub(super) position: Position,
    pub(super) status: RecordStatus,
}

/// A pocket and its records, by key, with the queued ones indexed by position. Its
/// key is the queue's map key.
#[derive(Clone, Debug)]
pub(super) struct Pocket {
    pub(super) status: PocketStatus,
    pub(super) position: Position,
    pub(super) consecutive_expiries: u32,
    pub(super) records: BTreeMap<RecordKey, Record>,
    /// The queued records, by position: exactly the keys of `records` whose status is
    /// `Queued`, each at its record's position, so a pull hands out the oldest without
    /// reading the rest. A rule that makes a record queued inserts it here; one that
    /// takes a record out of queued (only `HoldRecord`) removes it.
    pub(super) queued: BTreeMap<Position, RecordKey>,
}

/// A lease, active or ended. Its id is the queue's map key.
#[derive(Clone, Debug)]
pub(super) struct Lease {
    pub(super) pocket_key: PocketKey,
    pub(super) consumer: ConsumerId,
    pub(super) status: LeaseStatus,
    pub(super) granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
}

/// Every entity of the module. Ended leases are kept, as the module keeps them, so
/// an id is never reissued.
#[derive(Clone, Debug)]
pub(super) struct State {
    pub(super) config: Config,
    pub(super) next_position: u64,
    pub(super) publishers: BTreeSet<PublisherId>,
    pub(super) consumers: BTreeSet<ConsumerId>,
    pub(super) pockets: BTreeMap<PocketKey, Pocket>,
    /// The pocket order: every queued pocket, by position.
    pub(super) order: BTreeMap<Position, PocketKey>,
    pub(super) leases: BTreeMap<LeaseId, Lease>,
    /// Each consumer's active lease, the `Consumer.active_leases` projection.
    pub(super) active: BTreeMap<ConsumerId, LeaseId>,
    pub(super) superseded: Vec<SupersededRecord>,
    pub(super) dead_letters: Vec<DeadLetter>,
    /// The latest time any operation has read: the queue's `now`, which never goes
    /// back however the clock moves.
    latest: Timestamp,
}

/// Takes `next_position` and advances it, as every rule that assigns one does.
pub(super) const fn take_position(next_position: &mut u64) -> Position {
    let position = Position::new(*next_position);
    *next_position += 1;
    position
}

/// `RequeueHeldRecord` and `RequeueDeferredRecord`, for every record of a pocket whose
/// lease has ended: held and deferred records return to queued; a held record's held
/// version is dropped, and each keeps its position. Every record is then queued, so
/// every record is in the queued index; positions are unique, so the records already
/// there are inserted again unchanged.
pub(super) fn free_records(pocket: &mut Pocket) {
    for (key, record) in &mut pocket.records {
        record.status = RecordStatus::Queued;
        pocket.queued.insert(record.position, key.clone());
    }
}

impl State {
    pub(super) const fn new(config: Config) -> Self {
        Self {
            config,
            next_position: 0,
            publishers: BTreeSet::new(),
            consumers: BTreeSet::new(),
            pockets: BTreeMap::new(),
            order: BTreeMap::new(),
            leases: BTreeMap::new(),
            active: BTreeMap::new(),
            superseded: Vec::new(),
            dead_letters: Vec::new(),
            latest: Timestamp::from_millis(0),
        }
    }

    /// The queue's `now` for an operation whose clock reads `read`: the reading, or
    /// the latest earlier one if the clock has stepped back since. The module's time
    /// only moves forwards, and every `granted_at <= expires_at` and
    /// `ended_at >= granted_at` rests on it.
    pub(super) fn now(&mut self, read: Timestamp) -> Timestamp {
        self.latest = self.latest.max(read);
        self.latest
    }

    pub(super) const fn config(&self) -> &Config {
        &self.config
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            next_position: Position::new(self.next_position),
            publishers: self.publishers.iter().cloned().collect(),
            consumers: self.consumers.iter().cloned().collect(),
            pockets: self.pockets.iter().map(pocket_view).collect(),
            pocket_order: self.order.values().cloned().collect(),
            leases: self.leases.iter().map(lease_view).collect(),
            superseded: self.superseded.clone(),
            dead_letters: self.dead_letters.clone(),
        }
    }
}

fn pocket_view((key, pocket): (&PocketKey, &Pocket)) -> PocketView {
    PocketView {
        key: key.clone(),
        status: pocket.status,
        position: pocket.position,
        consecutive_expiries: pocket.consecutive_expiries,
        records: pocket
            .records
            .iter()
            .map(|(key, record)| RecordView {
                key: key.clone(),
                rank: record.rank,
                payload: record.payload.clone(),
                position: record.position,
                status: record.status.clone(),
            })
            .collect(),
    }
}

fn lease_view((id, lease): (&LeaseId, &Lease)) -> LeaseView {
    LeaseView {
        id: *id,
        pocket_key: lease.pocket_key.clone(),
        consumer: lease.consumer.clone(),
        status: lease.status,
        granted_at: lease.granted_at,
        expires_at: lease.expires_at,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use super::State;
    use crate::queue::outcome::{
        Batch, PocketFate, Publication, PublishOutcome, PullOutcome, SettleOutcome,
    };
    use crate::queue::view::RecordStatus;
    use crate::{
        Config, ConsumerId, CountingLeaseIds, Payload, PocketKey, Position, PublisherId, Rank,
        RecordKey, Timestamp,
    };

    const TTL: Duration = Duration::from_millis(100);

    fn state() -> State {
        let mut state = State::new(Config::new(TTL, 1, 2, TTL, TTL).unwrap());
        state.publisher_joins(PublisherId::new("p")).unwrap();
        state.consumer_joins(ConsumerId::new("c")).unwrap();
        state
    }

    fn publish(state: &mut State, key: &str, rank: i64) -> PublishOutcome {
        let publication = Publication::new(
            PublisherId::new("p"),
            PocketKey::new("a"),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::default(),
        );
        state
            .publish(Timestamp::from_millis(0), publication)
            .unwrap()
    }

    fn pull(state: &mut State, at: u64, limit: i64) -> Batch {
        let lease_ids = CountingLeaseIds::default();
        let outcome = state
            .pull(
                Timestamp::from_millis(at),
                &lease_ids,
                &ConsumerId::new("c"),
                limit,
            )
            .unwrap();
        let PullOutcome::Batch(batch) = outcome else {
            panic!("no pocket: {outcome:?}")
        };
        batch
    }

    fn keys(batch: &Batch) -> Vec<&str> {
        batch
            .records
            .iter()
            .map(|r| r.record_key.as_str())
            .collect()
    }

    /// The queued index's invariant over every pocket: it is exactly the queued
    /// records, each at its position. Returns pocket `a`'s index, by key, oldest first.
    fn queued_index(state: &State) -> Vec<&str> {
        for (pocket_key, pocket) in &state.pockets {
            let queued: BTreeMap<Position, RecordKey> = pocket
                .records
                .iter()
                .filter(|(_, record)| record.status == RecordStatus::Queued)
                .map(|(key, record)| (record.position, key.clone()))
                .collect();
            assert_eq!(pocket.queued, queued, "the queued index of {pocket_key:?}");
        }
        state.pockets[&PocketKey::new("a")]
            .queued
            .values()
            .map(RecordKey::as_str)
            .collect()
    }

    #[test]
    fn publishing_indexes_each_new_record_and_superseding_keeps_its_place() {
        let mut state = state();
        assert_eq!(publish(&mut state, "k1", 1), PublishOutcome::Created);
        assert_eq!(queued_index(&state), ["k1"]);
        assert_eq!(publish(&mut state, "k2", 1), PublishOutcome::NewKey);
        assert_eq!(queued_index(&state), ["k1", "k2"]);
        assert_eq!(publish(&mut state, "k1", 2), PublishOutcome::Superseded);
        assert_eq!(publish(&mut state, "k2", 0), PublishOutcome::Stale);
        assert_eq!(queued_index(&state), ["k1", "k2"]);
    }

    #[test]
    fn a_pull_pops_the_oldest_and_a_limit_of_zero_touches_none() {
        let mut state = state();
        for key in ["k1", "k2", "k3"] {
            publish(&mut state, key, 1);
        }
        let batch = pull(&mut state, 10, 0);
        assert_eq!(keys(&batch), Vec::<&str>::new());
        assert_eq!(batch.expires_at, Timestamp::from_millis(10) + TTL);
        assert_eq!(queued_index(&state), ["k1", "k2", "k3"]);
        assert_eq!(keys(&pull(&mut state, 20, -1)), Vec::<&str>::new());
        assert_eq!(keys(&pull(&mut state, 30, 1)), ["k1"]);
        assert_eq!(queued_index(&state), ["k2", "k3"]);
        assert_eq!(keys(&pull(&mut state, 40, 5)), ["k2", "k3"]);
        assert_eq!(queued_index(&state), Vec::<&str>::new());
        publish(&mut state, "k4", 1);
        assert_eq!(queued_index(&state), ["k4"]);
    }

    #[test]
    fn settling_and_deferring_leave_the_index_and_release_refills_it() {
        let mut state = state();
        for key in ["k1", "k2", "k3"] {
            publish(&mut state, key, 1);
        }
        let batch = pull(&mut state, 0, 2);
        let (consumer, lease) = (ConsumerId::new("c"), batch.lease_id);
        let now = Timestamp::from_millis(1);
        let settled = state.ack(now, &consumer, lease, &RecordKey::new("k1"));
        assert_eq!(settled, Ok(SettleOutcome::Settled));
        assert_eq!(queued_index(&state), ["k3"]);
        publish(&mut state, "k2", 2);
        let deferred = state.nack(now, &consumer, lease, &RecordKey::new("k2"));
        assert_eq!(deferred, Ok(SettleOutcome::Deferred));
        assert_eq!(queued_index(&state), ["k3"]);
        assert_eq!(state.release(now, &consumer, lease), Ok(PocketFate::Queued));
        assert_eq!(queued_index(&state), ["k3", "k2"]);
    }

    #[test]
    fn a_lapse_frees_held_and_deferred_records_into_the_index() {
        let mut state = state();
        for key in ["k1", "k2", "k3"] {
            publish(&mut state, key, 1);
        }
        let lease = pull(&mut state, 0, 2).lease_id;
        publish(&mut state, "k1", 2);
        let consumer = ConsumerId::new("c");
        let deferred = state.ack(
            Timestamp::from_millis(1),
            &consumer,
            lease,
            &RecordKey::new("k1"),
        );
        assert_eq!(deferred, Ok(SettleOutcome::Deferred));
        assert_eq!(queued_index(&state), ["k3"]);
        let lapsed = state.lapse(Timestamp::from_millis(1) + TTL);
        assert_eq!(lapsed.len(), 1);
        assert_eq!(queued_index(&state), ["k2", "k3", "k1"]);
    }

    #[test]
    #[should_panic(expected = "which is not a queued record")]
    fn a_pull_fails_on_an_index_entry_for_a_record_that_is_not_queued() {
        let mut state = state();
        publish(&mut state, "k1", 1);
        if let Some(pocket) = state.pockets.get_mut(&PocketKey::new("a")) {
            pocket
                .queued
                .insert(Position::new(99), RecordKey::new("gone"));
        }
        pull(&mut state, 0, 2);
    }
}
