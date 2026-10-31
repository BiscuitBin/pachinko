//! The records contract, behind the `testing` feature: what every
//! [`PocketRecords`] implementation must show, written once and run over each.
//!
//! libpachinko's tests run it over [`MemoryRecords`](super::MemoryRecords) and the
//! service's tests over its Arrow store, so the two cannot drift apart.

use super::{HeldRecord, PocketRecords, RecordState, RecordStatus, Version};

/// Checks every step of the records contract over fresh instances of `R`.
/// `batch_records` reads a batch back as held records, oldest first.
///
/// # Panics
///
/// When `R` breaks the contract, with the step that broke it.
///
/// ```
/// use libpachinko::records::MemoryRecords;
/// use libpachinko::records::contract::check_contract;
///
/// check_contract::<MemoryRecords>(Clone::clone);
/// ```
pub fn check_contract<R: PocketRecords>(batch_records: fn(&R::Batch) -> Vec<HeldRecord>) {
    insert_then_lookup::<R>();
    supersede_keeps_position_and_status::<R>(batch_records);
    hold_oldest_takes_smallest_positions::<R>(batch_records);
    hold_oldest_with_limit_zero_holds_nothing::<R>(batch_records);
    settle_removes_only_held::<R>();
    defer_moves_to_a_fresh_position::<R>(batch_records);
    free_all_requeues_held_and_deferred::<R>(batch_records);
    held_lists_held_records_by_position::<R>();
}

fn version(rank: i64, payload: &str) -> Version {
    Version::new(rank, payload.as_bytes().to_vec())
}

fn held(key: &str, rank: i64, payload: &str) -> HeldRecord {
    HeldRecord::new(key.to_owned(), rank, payload.as_bytes().to_vec())
}

fn insert_then_lookup<R: PocketRecords>() {
    let mut records = R::default();
    assert_eq!(records.lookup("a"), None, "lookup of an absent key");
    assert_eq!(records.record_count(), 0, "a new store is empty");
    records.insert_queued("a", version(3, "x"), 7);
    assert_eq!(
        records.lookup("a"),
        Some(RecordState::new(3, RecordStatus::Queued, None)),
        "insert_queued then lookup"
    );
    assert_eq!(records.record_count(), 1, "record_count after one insert");
    let view = &records.snapshot()[0];
    assert_eq!(
        (view.position, view.status, view.held.clone()),
        (7, RecordStatus::Queued, None),
        "insert_queued keeps the position it was given"
    );
}

fn supersede_keeps_position_and_status<R: PocketRecords>(
    batch_records: fn(&R::Batch) -> Vec<HeldRecord>,
) {
    let mut records = R::default();
    assert_eq!(
        records.supersede("a", version(1, "z")),
        None,
        "supersede of an absent key"
    );
    assert_eq!(
        records.record_count(),
        0,
        "supersede of an absent key changes nothing"
    );
    records.insert_queued("a", version(1, "old"), 4);
    assert_eq!(
        records.supersede("a", version(2, "new")),
        Some(version(1, "old")),
        "supersede returns the replaced version"
    );
    let batch = batch_records(&records.hold_oldest(1));
    assert_eq!(
        batch,
        vec![held("a", 2, "new")],
        "the latest version is held"
    );
    assert_eq!(
        records.supersede("a", version(5, "newer")),
        Some(version(2, "new")),
        "supersede of a held record"
    );
    assert_eq!(
        records.lookup("a"),
        Some(RecordState::new(5, RecordStatus::Held, Some(2))),
        "a supersede leaves the status and the held version alone"
    );
    assert_eq!(
        records.snapshot()[0].position,
        4,
        "a supersede keeps the position"
    );
}

fn hold_oldest_takes_smallest_positions<R: PocketRecords>(
    batch_records: fn(&R::Batch) -> Vec<HeldRecord>,
) {
    let mut records = R::default();
    records.insert_queued("c", version(1, "c"), 30);
    records.insert_queued("a", version(1, "a"), 10);
    records.insert_queued("b", version(1, "b"), 20);
    let first = batch_records(&records.hold_oldest(2));
    assert_eq!(
        first,
        vec![held("a", 1, "a"), held("b", 1, "b")],
        "the two oldest, oldest first"
    );
    assert_eq!(
        records
            .lookup("b")
            .map(|state| (state.status, state.held_rank)),
        Some((RecordStatus::Held, Some(1))),
        "a held record holds its latest rank"
    );
    let rest = batch_records(&records.hold_oldest(10));
    assert_eq!(
        rest,
        vec![held("c", 1, "c")],
        "a limit beyond the count holds all that remain"
    );
    assert!(
        batch_records(&records.hold_oldest(10)).is_empty(),
        "nothing queued, nothing held"
    );
}

fn hold_oldest_with_limit_zero_holds_nothing<R: PocketRecords>(
    batch_records: fn(&R::Batch) -> Vec<HeldRecord>,
) {
    let mut records = R::default();
    records.insert_queued("a", version(1, "a"), 1);
    assert!(
        batch_records(&records.hold_oldest(0)).is_empty(),
        "a zero limit holds nothing"
    );
    assert_eq!(
        records.lookup("a").map(|state| state.status),
        Some(RecordStatus::Queued),
        "a zero limit leaves the record queued"
    );
}

fn settle_removes_only_held<R: PocketRecords>() {
    let mut records = R::default();
    records.insert_queued("a", version(1, "a"), 1);
    records.insert_queued("b", version(1, "b"), 2);
    assert_eq!(records.settle("a"), None, "settle of a queued key");
    assert_eq!(
        records.record_count(),
        2,
        "settle of a queued key changes nothing"
    );
    records.hold_oldest(1);
    records.supersede("a", version(4, "a4"));
    assert_eq!(
        records.settle("a"),
        Some(version(1, "a")),
        "settle returns the held version"
    );
    assert_eq!(records.lookup("a"), None, "settle removes the record");
    assert_eq!(records.record_count(), 1, "settle removes one record");
    assert_eq!(records.settle("zz"), None, "settle of an absent key");
}

fn defer_moves_to_a_fresh_position<R: PocketRecords>(
    batch_records: fn(&R::Batch) -> Vec<HeldRecord>,
) {
    let mut records = R::default();
    records.insert_queued("a", version(1, "a"), 1);
    assert_eq!(records.defer("a", 9), None, "defer of a queued key");
    records.hold_oldest(1);
    records.supersede("a", version(2, "a2"));
    assert_eq!(
        records.defer("a", 9),
        Some(version(1, "a")),
        "defer returns the held version"
    );
    assert_eq!(
        records.lookup("a"),
        Some(RecordState::new(2, RecordStatus::Deferred, None)),
        "a deferred record has no held version"
    );
    assert_eq!(
        records.snapshot()[0].position,
        9,
        "defer takes the new position"
    );
    assert!(
        batch_records(&records.hold_oldest(10)).is_empty(),
        "a deferred record is not pullable"
    );
    assert_eq!(records.defer("zz", 10), None, "defer of an absent key");
}

fn free_all_requeues_held_and_deferred<R: PocketRecords>(
    batch_records: fn(&R::Batch) -> Vec<HeldRecord>,
) {
    let mut records = R::default();
    records.insert_queued("held", version(1, "h"), 1);
    records.insert_queued("deferred", version(1, "d"), 2);
    records.insert_queued("queued", version(1, "q"), 3);
    records.hold_oldest(2);
    records.supersede("deferred", version(2, "d2"));
    records.defer("deferred", 10);
    records.free_all();
    let views = records.snapshot();
    assert!(
        views
            .iter()
            .all(|view| view.status == RecordStatus::Queued && view.held.is_none()),
        "free_all leaves every record queued with no held version"
    );
    let order = batch_records(&records.hold_oldest(10));
    assert_eq!(
        order,
        vec![
            held("held", 1, "h"),
            held("queued", 1, "q"),
            held("deferred", 2, "d2")
        ],
        "free_all keeps each record's position"
    );
}

fn held_lists_held_records_by_position<R: PocketRecords>() {
    let mut records = R::default();
    records.insert_queued("z", version(1, "z"), 1);
    records.insert_queued("a", version(1, "a"), 2);
    records.insert_queued("m", version(1, "m"), 3);
    assert!(records.held().is_empty(), "nothing held yet");
    records.hold_oldest(2);
    records.supersede("z", version(3, "z3"));
    assert_eq!(
        records.held(),
        vec![held("z", 1, "z"), held("a", 1, "a")],
        "held lists the held versions by position"
    );
    let keys: Vec<String> = records
        .snapshot()
        .into_iter()
        .map(|view| view.key)
        .collect();
    assert_eq!(keys, ["a", "m", "z"], "snapshot lists records by key");
}
