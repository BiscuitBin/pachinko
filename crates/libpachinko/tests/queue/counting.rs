//! `Counting`, a `PocketRecords` and `InspectRecords` that delegates every step to
//! `MemoryRecords` and,
//! while a test records, logs each call with the keys of the pocket it was made
//! on. The queue builds each pocket's store with `Default`, so the log is a
//! thread-local the test reads back rather than a handle it passes in; each test
//! runs on its own thread.

use std::cell::RefCell;

use libpachinko::records::{EndedVersion, MemoryRecords, PocketRecords, Position, RecordView};
use libpachinko::testing::{InspectRecords, check_contract};
use libpachinko::{Mark, MarkedRecord, Payload, Rank, RecordKey, Timestamp};

/// One call: the step's name and the record keys the pocket held when it ran.
pub(crate) type Call = (&'static str, Vec<String>);

thread_local! {
    /// The calls logged while recording, or `None` while not.
    static LOG: RefCell<Option<Vec<Call>>> = const { RefCell::new(None) };
}

/// Runs `f`, logging every call any `Counting` store receives meanwhile, and
/// returns its value with the calls in order.
pub(crate) fn recording<T>(f: impl FnOnce() -> T) -> (T, Vec<Call>) {
    LOG.with(|log| *log.borrow_mut() = Some(Vec::new()));
    let value = f();
    let calls = LOG.with(|log| log.borrow_mut().take()).unwrap_or_default();
    (value, calls)
}

/// A `MemoryRecords` whose calls are logged while a test records. `Clone` is the
/// inner store's deep copy; the log is not part of the store.
#[derive(Clone, Debug, Default)]
pub(crate) struct Counting {
    inner: MemoryRecords,
}

impl Counting {
    fn log(&self, step: &'static str) {
        LOG.with(|log| {
            if let Some(calls) = log.borrow_mut().as_mut() {
                let keys = self
                    .inner
                    .snapshot()
                    .into_iter()
                    .map(|record| record.key.as_str().to_owned())
                    .collect();
                calls.push((step, keys));
            }
        });
    }
}

impl PocketRecords for Counting {
    type Batch = <MemoryRecords as PocketRecords>::Batch;

    fn lookup(&self, key: &RecordKey) -> Option<RecordView> {
        self.log("lookup");
        self.inner.lookup(key)
    }

    fn insert_queued(&mut self, key: RecordKey, rank: Rank, payload: Payload, position: Position) {
        self.log("insert_queued");
        self.inner.insert_queued(key, rank, payload, position);
    }

    fn supersede(
        &mut self,
        key: &RecordKey,
        rank: Rank,
        payload: Payload,
    ) -> Option<(Rank, Payload)> {
        self.log("supersede");
        self.inner.supersede(key, rank, payload)
    }

    fn defer_behind_held(
        &mut self,
        key: &RecordKey,
        rank: Rank,
        payload: Payload,
        position: Position,
    ) {
        self.log("defer_behind_held");
        self.inner.defer_behind_held(key, rank, payload, position);
    }

    fn hold_oldest(&mut self, limit: usize) -> Self::Batch {
        self.log("hold_oldest");
        self.inner.hold_oldest(limit)
    }

    fn mark(&mut self, key: &RecordKey, mark: Mark, at: Timestamp) {
        self.log("mark");
        self.inner.mark(key, mark, at);
    }

    fn end_lease(&mut self) -> Vec<EndedVersion> {
        self.log("end_lease");
        self.inner.end_lease()
    }

    fn record_count(&self) -> usize {
        self.log("record_count");
        self.inner.record_count()
    }

    fn held(&self) -> Vec<MarkedRecord> {
        self.log("held");
        self.inner.held()
    }
}

impl InspectRecords for Counting {
    fn snapshot(&self) -> Vec<RecordView> {
        self.log("snapshot");
        self.inner.snapshot()
    }
}

// The wrapper keeps the contract every store passes, so the O(due) test measures a
// lawful store.
#[test]
fn counting_passes_the_records_contract() {
    check_contract::<Counting>();
}
