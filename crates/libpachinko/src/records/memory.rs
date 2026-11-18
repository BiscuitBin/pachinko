//! `MemoryRecords`, the one implementation of [`PocketRecords`].

use std::collections::{BTreeMap, BTreeSet};

use super::{EndedVersion, PocketRecords, Position, RecordStatus, RecordView};
use crate::ids::{HeldRecord, Mark, MarkedRecord, Payload, Rank, RecordKey};
use crate::timestamp::Timestamp;

/// One pocket's records in ordered collections: the records by key, an index of
/// the queued ones by position, and an index of the held ones by position, so
/// [`end_lease`](PocketRecords::end_lease) visits only those, smallest position
/// first.
///
/// ```
/// use libpachinko::records::{MemoryRecords, PocketRecords, Position};
/// use libpachinko::{Payload, Rank, RecordKey};
///
/// let mut records = MemoryRecords::default();
/// records.insert_queued(RecordKey::new("b"), Rank::new(1), Payload::default(), Position::new(2));
/// records.insert_queued(RecordKey::new("a"), Rank::new(1), Payload::default(), Position::new(1));
/// assert_eq!(records.hold_oldest(1)[0].key.as_str(), "a");
/// assert_eq!(records.record_count(), 2);
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct MemoryRecords {
    records: BTreeMap<RecordKey, Entry>,
    queued: BTreeSet<(Position, RecordKey)>,
    outstanding: BTreeSet<(Position, RecordKey)>,
}

/// One record's slot.
#[derive(Clone, Debug)]
struct Entry {
    rank: Rank,
    payload: Payload,
    position: Position,
    status: RecordStatus,
}

impl Entry {
    fn view(&self, key: &RecordKey) -> RecordView {
        RecordView::new(
            key.clone(),
            self.rank,
            self.payload.clone(),
            self.position,
            self.status.clone(),
        )
    }
}

/// What one held record's lease end does: the version it takes out, if any, and
/// where the record stands afterwards, or `None` when it is removed.
fn freed(key: &RecordKey, entry: &Entry) -> (Option<EndedVersion>, Option<Position>) {
    let RecordStatus::Held {
        rank,
        payload,
        mark,
        nacked_at,
        deferred_position,
    } = &entry.status
    else {
        return (None, Some(entry.position));
    };
    let held = || HeldRecord::new(key.clone(), *rank, payload.clone());
    let ended = match (mark, nacked_at, deferred_position) {
        (Mark::Unsettled, _, Some(_)) => Some(EndedVersion::Superseded(held())),
        (Mark::Nacked, Some(nacked_at), _) => Some(EndedVersion::DeadLettered {
            held: held(),
            nacked_at: *nacked_at,
        }),
        _ => None,
    };
    let position = if *mark == Mark::Unsettled {
        Some(entry.position)
    } else {
        *deferred_position
    };
    (ended, position)
}

impl PocketRecords for MemoryRecords {
    type Batch = Vec<HeldRecord>;

    fn lookup(&self, key: &RecordKey) -> Option<RecordView> {
        self.records.get(key).map(|entry| entry.view(key))
    }

    fn insert_queued(&mut self, key: RecordKey, rank: Rank, payload: Payload, position: Position) {
        if self.records.contains_key(&key) {
            return;
        }
        self.queued.insert((position, key.clone()));
        let entry = Entry {
            rank,
            payload,
            position,
            status: RecordStatus::Queued,
        };
        self.records.insert(key, entry);
    }

    fn supersede(
        &mut self,
        key: &RecordKey,
        rank: Rank,
        payload: Payload,
    ) -> Option<(Rank, Payload)> {
        let entry = self.records.get_mut(key)?;
        if let RecordStatus::Held {
            deferred_position: None,
            ..
        } = entry.status
        {
            return None;
        }
        let loser_rank = std::mem::replace(&mut entry.rank, rank);
        let loser_payload = std::mem::replace(&mut entry.payload, payload);
        Some((loser_rank, loser_payload))
    }

    fn defer_behind_held(
        &mut self,
        key: &RecordKey,
        rank: Rank,
        payload: Payload,
        position: Position,
    ) {
        let Some(entry) = self.records.get_mut(key) else {
            return;
        };
        if let RecordStatus::Held {
            deferred_position: deferred @ None,
            ..
        } = &mut entry.status
        {
            *deferred = Some(position);
            entry.rank = rank;
            entry.payload = payload;
        }
    }

    fn hold_oldest(&mut self, limit: usize) -> Vec<HeldRecord> {
        let mut batch = Vec::with_capacity(limit.min(self.queued.len()));
        while batch.len() < limit {
            let Some((position, key)) = self.queued.pop_first() else {
                break;
            };
            if let Some(entry) = self.records.get_mut(&key) {
                entry.status = RecordStatus::Held {
                    rank: entry.rank,
                    payload: entry.payload.clone(),
                    mark: Mark::Unsettled,
                    nacked_at: None,
                    deferred_position: None,
                };
                batch.push(HeldRecord::new(
                    key.clone(),
                    entry.rank,
                    entry.payload.clone(),
                ));
                self.outstanding.insert((position, key));
            }
        }
        batch
    }

    fn mark(&mut self, key: &RecordKey, mark: Mark, at: Timestamp) {
        if mark == Mark::Unsettled {
            return;
        }
        let Some(entry) = self.records.get_mut(key) else {
            return;
        };
        if let RecordStatus::Held {
            mark: stored @ Mark::Unsettled,
            nacked_at,
            ..
        } = &mut entry.status
        {
            *stored = mark;
            if mark == Mark::Nacked {
                *nacked_at = Some(at);
            }
        }
    }

    fn end_lease(&mut self) -> Vec<EndedVersion> {
        let mut ended = Vec::new();
        for (_, key) in std::mem::take(&mut self.outstanding) {
            let Some(entry) = self.records.get_mut(&key) else {
                continue;
            };
            let (version, position) = freed(&key, entry);
            ended.extend(version);
            match position {
                Some(position) => {
                    entry.position = position;
                    entry.status = RecordStatus::Queued;
                    self.queued.insert((position, key));
                }
                None => {
                    self.records.remove(&key);
                }
            }
        }
        ended
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn held(&self) -> Vec<MarkedRecord> {
        self.outstanding
            .iter()
            .filter_map(|(_, key)| match &self.records.get(key)?.status {
                RecordStatus::Held {
                    rank,
                    payload,
                    mark,
                    ..
                } => Some(MarkedRecord::new(
                    key.clone(),
                    *rank,
                    payload.clone(),
                    *mark,
                )),
                RecordStatus::Queued => None,
            })
            .collect()
    }
}

#[cfg(feature = "testing")]
impl super::contract::InspectRecords for MemoryRecords {
    fn snapshot(&self) -> Vec<RecordView> {
        self.records
            .iter()
            .map(|(key, entry)| entry.view(key))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{MemoryRecords, PocketRecords, Position};
    use crate::ids::{Mark, Payload, Rank, RecordKey};
    use crate::timestamp::Timestamp;

    fn insert(records: &mut MemoryRecords, key: &str, position: u64) {
        records.insert_queued(
            RecordKey::new(key),
            Rank::new(1),
            Payload::default(),
            Position::new(position),
        );
    }

    fn entry(position: u64, key: &str) -> (Position, RecordKey) {
        (Position::new(position), RecordKey::new(key))
    }

    /// The outstanding index, in its own order, as positions and keys.
    fn index(records: &MemoryRecords) -> Vec<(Position, RecordKey)> {
        records.outstanding.iter().cloned().collect()
    }

    #[test]
    fn the_outstanding_index_is_exactly_the_held_keys_by_position() {
        let mut records = MemoryRecords::default();
        for (key, position) in [("c", 1), ("b", 2), ("a", 3), ("d", 4)] {
            insert(&mut records, key, position);
        }
        records.hold_oldest(3);
        let held = vec![entry(1, "c"), entry(2, "b"), entry(3, "a")];
        assert_eq!(
            index(&records),
            held,
            "the held keys by position after hold_oldest"
        );
        records.mark(&RecordKey::new("b"), Mark::Acked, Timestamp::from_millis(1));
        records.mark(
            &RecordKey::new("a"),
            Mark::Nacked,
            Timestamp::from_millis(1),
        );
        assert_eq!(index(&records), held, "a mark keeps the index");
        for (key, position) in [("c", 9), ("b", 8)] {
            records.defer_behind_held(
                &RecordKey::new(key),
                Rank::new(2),
                Payload::default(),
                Position::new(position),
            );
        }
        assert_eq!(index(&records), held, "a deferral keeps the held position");
        records.end_lease();
        assert!(
            records.outstanding.is_empty(),
            "end_lease empties the index"
        );
        assert_eq!(
            records.queued,
            BTreeSet::from([entry(1, "c"), entry(4, "d"), entry(8, "b")]),
            "end_lease queues each record it keeps at its final position"
        );
        assert_eq!(records.queued.len(), records.records.len());
    }

    #[test]
    fn end_lease_visits_only_the_held_keys() {
        let mut records = MemoryRecords::default();
        for position in 0..1_000 {
            insert(&mut records, &format!("k{position:04}"), position);
        }
        records.hold_oldest(2);
        records.mark(
            &RecordKey::new("k0001"),
            Mark::Acked,
            Timestamp::from_millis(1),
        );
        assert_eq!(index(&records).len(), 2);
        assert_eq!(records.end_lease(), Vec::new());
        assert!(records.outstanding.is_empty());
        assert_eq!(records.queued.len(), 999);
        assert_eq!(records.records.len(), 999);
    }
}
