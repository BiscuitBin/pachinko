//! `MemoryRecords`, the one implementation of [`PocketRecords`].

use std::collections::{BTreeMap, BTreeSet};

use super::{HeldRecord, PocketRecords, RecordStatus, RecordView};
use crate::ids::{Payload, Position, Rank, RecordKey};

/// One pocket's records in ordered collections: the records by key, an index of
/// the queued ones by position, and an index of the outstanding keys, held and
/// deferred, so [`free_held`](PocketRecords::free_held) visits only those.
///
/// ```
/// use libpachinko::ids::{Payload, Position, Rank, RecordKey};
/// use libpachinko::records::{MemoryRecords, PocketRecords};
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
    outstanding: BTreeSet<RecordKey>,
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

/// The held version of a held entry, or `None`.
fn held_version(key: &RecordKey, entry: &Entry) -> Option<HeldRecord> {
    match &entry.status {
        RecordStatus::Held { rank, payload } => {
            Some(HeldRecord::new(key.clone(), *rank, payload.clone()))
        }
        RecordStatus::Queued | RecordStatus::Deferred => None,
    }
}

impl MemoryRecords {
    /// Takes the held version out of a held entry, leaving `status` in its place.
    fn unhold(
        &mut self,
        key: &RecordKey,
        status: RecordStatus,
    ) -> Option<(HeldRecord, &mut Entry)> {
        let entry = self.records.get_mut(key)?;
        let held = held_version(key, entry)?;
        entry.status = status;
        Some((held, entry))
    }
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
        let loser_rank = std::mem::replace(&mut entry.rank, rank);
        let loser_payload = std::mem::replace(&mut entry.payload, payload);
        Some((loser_rank, loser_payload))
    }

    fn hold_oldest(&mut self, limit: usize) -> Vec<HeldRecord> {
        let mut batch = Vec::with_capacity(limit.min(self.queued.len()));
        while batch.len() < limit {
            let Some((_, key)) = self.queued.pop_first() else {
                break;
            };
            if let Some(entry) = self.records.get_mut(&key) {
                entry.status = RecordStatus::Held {
                    rank: entry.rank,
                    payload: entry.payload.clone(),
                };
                batch.push(HeldRecord::new(
                    key.clone(),
                    entry.rank,
                    entry.payload.clone(),
                ));
                self.outstanding.insert(key);
            }
        }
        batch
    }

    fn settle(&mut self, key: &RecordKey) -> Option<HeldRecord> {
        let (held, _) = self.unhold(key, RecordStatus::Queued)?;
        self.records.remove(key);
        self.outstanding.remove(key);
        Some(held)
    }

    fn defer(&mut self, key: &RecordKey, position: Position) -> Option<HeldRecord> {
        let (held, entry) = self.unhold(key, RecordStatus::Deferred)?;
        entry.position = position;
        Some(held)
    }

    fn free_held(&mut self) -> usize {
        let outstanding = std::mem::take(&mut self.outstanding);
        let count = outstanding.len();
        for key in outstanding {
            if let Some(entry) = self.records.get_mut(&key) {
                entry.status = RecordStatus::Queued;
                self.queued.insert((entry.position, key));
            }
        }
        count
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn held(&self) -> Vec<HeldRecord> {
        let mut held: Vec<(Position, HeldRecord)> = self
            .outstanding
            .iter()
            .filter_map(|key| {
                let entry = self.records.get(key)?;
                held_version(key, entry).map(|record| (entry.position, record))
            })
            .collect();
        held.sort_by_key(|(position, _)| *position);
        held.into_iter().map(|(_, record)| record).collect()
    }

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

    use super::{MemoryRecords, PocketRecords, RecordStatus};
    use crate::ids::{Payload, Position, Rank, RecordKey};

    fn insert(records: &mut MemoryRecords, key: &str, position: u64) {
        records.insert_queued(
            RecordKey::new(key),
            Rank::new(1),
            Payload::default(),
            Position::new(position),
        );
    }

    /// The keys `snapshot` shows held or deferred.
    fn outstanding_in_views(records: &MemoryRecords) -> BTreeSet<RecordKey> {
        records
            .snapshot()
            .into_iter()
            .filter(|view| view.status != RecordStatus::Queued)
            .map(|view| view.key)
            .collect()
    }

    #[test]
    fn the_outstanding_index_is_exactly_the_held_and_deferred_keys() {
        let mut records = MemoryRecords::default();
        for (key, position) in [("a", 1), ("b", 2), ("c", 3), ("d", 4)] {
            insert(&mut records, key, position);
        }
        records.hold_oldest(3);
        assert_eq!(records.outstanding, outstanding_in_views(&records));
        records.defer(&RecordKey::new("b"), Position::new(9));
        assert_eq!(records.outstanding, outstanding_in_views(&records));
        records.settle(&RecordKey::new("a"));
        assert_eq!(records.outstanding, outstanding_in_views(&records));
        assert_eq!(records.free_held(), 2);
        assert!(records.outstanding.is_empty());
        assert_eq!(records.queued.len(), records.records.len());
    }

    #[test]
    fn free_held_visits_only_the_outstanding() {
        let mut records = MemoryRecords::default();
        for position in 0..1_000 {
            insert(&mut records, &format!("k{position:04}"), position);
        }
        records.hold_oldest(1);
        assert_eq!(records.outstanding.len(), 1);
        assert_eq!(records.free_held(), 1);
        assert_eq!(records.queued.len(), 1_000);
    }
}
