//! Identity and publish: `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`,
//! `PublishNewKey`, `Supersede`, `DropStalePublish`, and the `PocketTouched` chain's
//! `ReviveIdlePocket` and `ReviveParkedPocket`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::outcome::{Publication, PublishOutcome};
use super::state::{Pocket, Record, State, take_position};
use super::view::{PocketStatus, RecordStatus, ShedReason, SupersededRecord};
use crate::{ConsumerId, Error, PocketKey, PublisherId, Timestamp};

/// The publish rule whose guards a publication meets; the four are disjoint.
enum PublishRule {
    IntoNewPocket,
    NewKey,
    Supersede,
    DropStale,
}

impl State {
    /// `PublisherJoins`.
    pub(super) fn publisher_joins(&mut self, id: PublisherId) -> Result<(), Error> {
        if self.publishers.contains(&id) {
            return Err(Error::PublisherExists(id));
        }
        self.publishers.insert(id);
        Ok(())
    }

    /// `ConsumerJoins`.
    pub(super) fn consumer_joins(&mut self, id: ConsumerId) -> Result<(), Error> {
        if self.consumers.contains(&id) {
            return Err(Error::ConsumerExists(id));
        }
        self.consumers.insert(id);
        Ok(())
    }

    /// `Publish`, dispatched to the one rule whose guards hold.
    pub(super) fn publish(
        &mut self,
        now: Timestamp,
        publication: Publication,
    ) -> Result<PublishOutcome, Error> {
        let rule = publish_rule(self, &publication)?;
        Ok(match rule {
            PublishRule::IntoNewPocket => publish_into_new_pocket(self, publication),
            PublishRule::NewKey => publish_new_key(self, publication),
            PublishRule::Supersede => supersede(self, now, publication),
            PublishRule::DropStale => drop_stale_publish(self, now, publication),
        })
    }
}

/// Reads the guards. An equal rank meets none of them: no rule covers it, which the
/// module's first open question leaves undecided, so the publish is refused.
fn publish_rule(state: &State, publication: &Publication) -> Result<PublishRule, Error> {
    if !state.publishers.contains(&publication.publisher) {
        return Err(Error::UnknownPublisher(publication.publisher.clone()));
    }
    let Some(pocket) = state.pockets.get(&publication.pocket_key) else {
        return Ok(PublishRule::IntoNewPocket);
    };
    let Some(record) = pocket.records.get(&publication.record_key) else {
        return Ok(PublishRule::NewKey);
    };
    match publication.rank.cmp(&record.rank) {
        Ordering::Greater => Ok(PublishRule::Supersede),
        Ordering::Less => Ok(PublishRule::DropStale),
        Ordering::Equal => Err(Error::EqualRank {
            pocket_key: publication.pocket_key.clone(),
            record_key: publication.record_key.clone(),
            rank: record.rank,
        }),
    }
}

/// `PublishIntoNewPocket`: the pocket takes `next_position`, its record the one after.
fn publish_into_new_pocket(state: &mut State, publication: Publication) -> PublishOutcome {
    let pocket_position = take_position(&mut state.next_position);
    let record_position = take_position(&mut state.next_position);
    let record = Record {
        rank: publication.rank,
        payload: publication.payload,
        position: record_position,
        status: RecordStatus::Queued,
    };
    let pocket = Pocket {
        status: PocketStatus::Queued,
        position: pocket_position,
        consecutive_expiries: 0,
        queued: BTreeMap::from([(record_position, publication.record_key.clone())]),
        records: BTreeMap::from([(publication.record_key, record)]),
    };
    state
        .order
        .insert(pocket_position, publication.pocket_key.clone());
    state.pockets.insert(publication.pocket_key, pocket);
    PublishOutcome::Created
}

/// `PublishNewKey`, then `PocketTouched`.
fn publish_new_key(state: &mut State, publication: Publication) -> PublishOutcome {
    let position = take_position(&mut state.next_position);
    let record = Record {
        rank: publication.rank,
        payload: publication.payload,
        position,
        status: RecordStatus::Queued,
    };
    if let Some(pocket) = state.pockets.get_mut(&publication.pocket_key) {
        pocket
            .queued
            .insert(position, publication.record_key.clone());
        pocket.records.insert(publication.record_key, record);
    }
    pocket_touched(state, &publication.pocket_key);
    PublishOutcome::NewKey
}

/// `Supersede`: the old version becomes a superseded record and the new one takes
/// its place, keeping its position; then `PocketTouched`.
fn supersede(state: &mut State, now: Timestamp, publication: Publication) -> PublishOutcome {
    let expires_at = now + state.config.superseded_ttl();
    let record = state
        .pockets
        .get_mut(&publication.pocket_key)
        .and_then(|pocket| pocket.records.get_mut(&publication.record_key));
    if let Some(record) = record {
        state.superseded.push(SupersededRecord {
            pocket_key: publication.pocket_key.clone(),
            record_key: publication.record_key,
            rank: record.rank,
            payload: std::mem::replace(&mut record.payload, publication.payload),
            reason: ShedReason::Superseded,
            shed_at: now,
            expires_at,
        });
        record.rank = publication.rank;
    }
    pocket_touched(state, &publication.pocket_key);
    PublishOutcome::Superseded
}

/// `DropStalePublish`: the publication itself becomes a superseded record; then
/// `PocketTouched`.
fn drop_stale_publish(
    state: &mut State,
    now: Timestamp,
    publication: Publication,
) -> PublishOutcome {
    state.superseded.push(SupersededRecord {
        pocket_key: publication.pocket_key.clone(),
        record_key: publication.record_key,
        rank: publication.rank,
        payload: publication.payload,
        reason: ShedReason::Stale,
        shed_at: now,
        expires_at: now + state.config.superseded_ttl(),
    });
    pocket_touched(state, &publication.pocket_key);
    PublishOutcome::Stale
}

/// `ReviveIdlePocket` and `ReviveParkedPocket`: an idle or parked pocket touched by a
/// publish returns to the back of the pocket order with its expiries forgotten.
fn pocket_touched(state: &mut State, pocket_key: &PocketKey) {
    let Some(pocket) = state.pockets.get_mut(pocket_key) else {
        return;
    };
    if matches!(pocket.status, PocketStatus::Idle | PocketStatus::Parked) {
        let position = take_position(&mut state.next_position);
        pocket.status = PocketStatus::Queued;
        pocket.position = position;
        pocket.consecutive_expiries = 0;
        state.order.insert(position, pocket_key.clone());
    }
}
