//! The key index: each live key's row id, found by the key's hash.

use std::fmt;
use std::hash::{BuildHasher, RandomState};

use hashbrown::HashTable;

use super::rows::RowId;

/// Each live key's row id, in a `hashbrown::HashTable` that stores only the id
/// and reads the key's bytes back from the log: about 5 bytes a key at full load.
/// It hashes and compares bytes, so a lookup never validates UTF-8.
///
/// The table is never iterated. Its order comes from the store's random hasher,
/// so anything built by walking it would come out in an order that changes from
/// run to run; every read is a lookup by one key's hash. `Debug` prints only the
/// key count for the same reason: after churn even the bucket count depends on
/// where the random hasher left tombstones.
///
/// The hasher is a per-store `RandomState`, so a publisher cannot flood a pocket's
/// buckets with keys chosen to collide. That randomness is the service's own
/// (decision 0021).
#[derive(Clone, Default)]
pub(super) struct Index {
    table: HashTable<RowId>,
    hasher: RandomState,
}

impl fmt::Debug for Index {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Index")
            .field("len", &self.table.len())
            .finish_non_exhaustive()
    }
}

impl Index {
    /// How many keys the index holds: one for each live row.
    pub(super) fn entries(&self) -> usize {
        self.table.len()
    }

    /// The row holding `key`, where `key_of` reads a row's key from the log.
    pub(super) fn locate<'k>(
        &self,
        key: &str,
        key_of: impl Fn(RowId) -> &'k [u8],
    ) -> Option<RowId> {
        self.table
            .find(self.hasher.hash_one(key.as_bytes()), |row| {
                key_of(*row) == key.as_bytes()
            })
            .copied()
    }

    /// Adds `key` at `row`; the caller has checked that `key` is absent. A growth
    /// rehashes every key through `key_of`.
    pub(super) fn enter<'k>(&mut self, key: &str, row: RowId, key_of: impl Fn(RowId) -> &'k [u8]) {
        let hasher = &self.hasher;
        self.table
            .insert_unique(hasher.hash_one(key.as_bytes()), row, |entry| {
                hasher.hash_one(key_of(*entry))
            });
    }

    /// Removes `key`'s entry if it names `row`. It matches by row id, not by key,
    /// so it reads nothing from the log.
    pub(super) fn forget(&mut self, key: &str, row: RowId) {
        if let Ok(entry) = self
            .table
            .find_entry(self.hasher.hash_one(key.as_bytes()), |entry| *entry == row)
        {
            entry.remove();
        }
    }

    /// Removes every entry, keeping the table's buckets.
    pub(super) fn reset(&mut self) {
        self.table.clear();
    }

    /// Shrinks the table to fit once its buckets, at hashbrown's 7/8 load, would
    /// hold more than four times its length (at least 1). It reads the buckets, not
    /// `capacity()`: a removal can leave a tombstone that `capacity()` stops
    /// counting, so `capacity()` understates the memory the table holds.
    ///
    /// hashbrown sizes a table to a power of two buckets at 7/8 load, so after a
    /// drain a shrink to fit lands where a fresh store of that length grows to: a
    /// pocket drained from 100k keys to 1k has the 2048 buckets a fresh 1k pocket
    /// has, about 10 KiB.
    ///
    /// Churn is different. A removal can leave a tombstone, and once tombstones use
    /// up the spare slots while the table is more than half full, hashbrown grows
    /// the table rather than rehashing in place, so the table can grow at a
    /// constant key count. A pocket churning at 1k keys settles at 4096 buckets,
    /// twice a fresh one's, and stays there. That is within the rule, which lets
    /// the buckets reach four times the length before it shrinks them.
    ///
    /// Between a shrink and the next resize the table takes as many inserts or
    /// removals as a constant fraction of its length, so each rehash is paid for
    /// by the steps before it: amortised O(1). A 2× trigger would also shrink a
    /// churning table, rehashing back and forth as tombstones regrew it.
    pub(super) fn trim<'k>(&mut self, key_of: impl Fn(RowId) -> &'k [u8]) {
        let len = self.table.len();
        let full = self.table.num_buckets() / 8 * 7;
        if full > len.max(1).saturating_mul(4) {
            let hasher = &self.hasher;
            self.table
                .shrink_to(len, |entry| hasher.hash_one(key_of(*entry)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Index;
    use crate::log::rows::RowId;

    fn key_of<'k>(keys: &'k [String]) -> impl Fn(RowId) -> &'k [u8] + 'k {
        move |row| {
            usize::try_from(row)
                .ok()
                .and_then(|row| keys.get(row))
                .map_or(&[][..], String::as_bytes)
        }
    }

    fn entered(keys: &[String]) -> Index {
        let mut index = Index::default();
        for (row, key) in (0..).zip(keys) {
            index.enter(key, row, key_of(keys));
        }
        index
    }

    #[test]
    fn a_key_entered_is_located_until_it_is_forgotten() {
        let keys: Vec<String> = ["a", "b", "c"].map(str::to_owned).to_vec();
        let mut index = entered(&keys);
        assert_eq!(index.entries(), 3);
        assert_eq!(index.locate("b", key_of(&keys)), Some(1));
        assert_eq!(index.locate("z", key_of(&keys)), None);
        index.forget("b", 1);
        assert_eq!(index.locate("b", key_of(&keys)), None);
        assert_eq!(index.entries(), 2);
        index.reset();
        assert_eq!(index.locate("a", key_of(&keys)), None);
    }

    #[test]
    fn forgetting_matches_the_row_and_not_only_the_key() {
        let keys: Vec<String> = ["a"].map(str::to_owned).to_vec();
        let mut index = entered(&keys);
        index.forget("a", 7);
        assert_eq!(index.locate("a", key_of(&keys)), Some(0));
    }

    #[test]
    fn the_table_shrinks_once_its_buckets_pass_four_times_its_length() {
        let keys: Vec<String> = (0..1000).map(|key| key.to_string()).collect();
        let mut index = entered(&keys);
        assert_eq!(index.table.num_buckets(), 2048);
        for (row, key) in (0..).zip(&keys[..990]) {
            index.forget(key, row);
            index.trim(key_of(&keys));
        }
        assert_eq!(index.table.num_buckets(), 32);
        for (row, key) in (0..).zip(&keys).skip(990) {
            assert_eq!(index.locate(key, key_of(&keys)), Some(row));
        }
    }

    #[test]
    fn a_table_within_four_times_its_length_keeps_its_buckets() {
        let keys: Vec<String> = (0..100).map(|key| key.to_string()).collect();
        let mut index = entered(&keys);
        let buckets = index.table.num_buckets();
        for (row, key) in (0..).zip(&keys[..50]) {
            index.forget(key, row);
        }
        index.trim(key_of(&keys));
        assert_eq!(index.table.num_buckets(), buckets);
    }

    #[test]
    fn debug_prints_the_key_count_and_nothing_the_hasher_shapes() {
        let keys: Vec<String> = ["secret"].map(str::to_owned).to_vec();
        let index = entered(&keys);
        let printed = format!("{index:?}");
        assert_eq!(printed, "Index { len: 1, .. }");
    }
}
