//! `MemoryRecords`, the reference implementation of `PocketRecords`.

use std::collections::BTreeMap;

use super::{HeldRecord, PocketRecords, RecordState, RecordStatus, RecordView, Version};

/// One pocket's records in ordered maps: records by key, and the queued ones by
/// position. The trait's reference, and the store every library test runs on.
///
/// ```
/// use libpachinko::records::{MemoryRecords, PocketRecords, Version};
///
/// let mut records = MemoryRecords::default();
/// records.insert_queued("b", Version::new(1, Vec::new()), 2);
/// records.insert_queued("a", Version::new(1, Vec::new()), 1);
/// let batch = records.hold_oldest(1);
/// assert_eq!(batch[0].key, "a");
/// assert_eq!(records.record_count(), 2);
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct MemoryRecords {
    records: BTreeMap<String, Entry>,
    queued: BTreeMap<u64, String>,
}

/// One record's slot.
#[derive(Clone, Debug)]
struct Entry {
    latest: Version,
    position: u64,
    status: RecordStatus,
    held: Option<Version>,
}

impl PocketRecords for MemoryRecords {
    type Batch = Vec<HeldRecord>;

    fn lookup(&self, key: &str) -> Option<RecordState> {
        self.records.get(key).map(|entry| RecordState {
            rank: entry.latest.rank,
            status: entry.status,
            held_rank: entry.held.as_ref().map(|held| held.rank),
        })
    }

    fn insert_queued(&mut self, key: &str, version: Version, position: u64) {
        let entry = Entry {
            latest: version,
            position,
            status: RecordStatus::Queued,
            held: None,
        };
        self.records.insert(key.to_owned(), entry);
        self.queued.insert(position, key.to_owned());
    }

    fn supersede(&mut self, key: &str, version: Version) -> Option<Version> {
        self.records
            .get_mut(key)
            .map(|entry| std::mem::replace(&mut entry.latest, version))
    }

    fn hold_oldest(&mut self, limit: usize) -> Vec<HeldRecord> {
        let oldest: Vec<u64> = self.queued.keys().take(limit).copied().collect();
        oldest
            .iter()
            .filter_map(|position| self.queued.remove(position))
            .filter_map(|key| {
                let entry = self.records.get_mut(&key)?;
                entry.status = RecordStatus::Held;
                entry.held = Some(entry.latest.clone());
                Some(HeldRecord {
                    key,
                    rank: entry.latest.rank,
                    payload: entry.latest.payload.clone(),
                })
            })
            .collect()
    }

    fn settle(&mut self, key: &str) -> Option<Version> {
        let held = self.records.get(key)?.held.clone()?;
        self.records.remove(key);
        Some(held)
    }

    fn defer(&mut self, key: &str, position: u64) -> Option<Version> {
        let entry = self.records.get_mut(key)?;
        let held = entry.held.take()?;
        entry.status = RecordStatus::Deferred;
        entry.position = position;
        Some(held)
    }

    fn free_all(&mut self) {
        let queued = &mut self.queued;
        self.records
            .iter_mut()
            .filter(|(_, entry)| entry.status != RecordStatus::Queued)
            .for_each(|(key, entry)| {
                entry.status = RecordStatus::Queued;
                entry.held = None;
                queued.insert(entry.position, key.clone());
            });
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn held(&self) -> Vec<HeldRecord> {
        let mut held: Vec<(u64, HeldRecord)> = self
            .records
            .iter()
            .filter_map(|(key, entry)| {
                let version = entry.held.as_ref()?;
                let record = HeldRecord {
                    key: key.clone(),
                    rank: version.rank,
                    payload: version.payload.clone(),
                };
                Some((entry.position, record))
            })
            .collect();
        held.sort_by_key(|(position, _)| *position);
        held.into_iter().map(|(_, record)| record).collect()
    }

    fn snapshot(&self) -> Vec<RecordView> {
        self.records
            .iter()
            .map(|(key, entry)| RecordView {
                key: key.clone(),
                latest: entry.latest.clone(),
                position: entry.position,
                status: entry.status,
                held: entry.held.clone(),
            })
            .collect()
    }
}
