//! The records contract, behind the `testing` feature: what every
//! [`PocketRecords`] implementation must show, written once and run over each.

use std::collections::BTreeSet;

use super::{HeldRecord, PocketRecords, RecordStatus, RecordView};
use crate::ids::{Payload, Position, Rank, RecordKey};

/// Checks every step of the records contract over fresh instances of `R`, and
/// after every call that [`record_count`](PocketRecords::record_count) and
/// [`held`](PocketRecords::held) agree with [`snapshot`](PocketRecords::snapshot).
///
/// The contract checks behaviour, not cost. The O(outstanding) bound on
/// [`free_held`](PocketRecords::free_held), and the other documented costs, are
/// obligations the implementor must meet and the contract does not prove;
/// [`MemoryRecords`](super::MemoryRecords)' index of outstanding keys shows the
/// required shape.
///
/// # Panics
///
/// When `R` breaks the contract, naming the step that broke it.
///
/// ```
/// use libpachinko::records::MemoryRecords;
/// use libpachinko::records::contract::check_contract;
///
/// check_contract::<MemoryRecords>();
/// ```
pub fn check_contract<R: PocketRecords>() {
    insert_then_lookup::<R>();
    a_second_insert_is_refused::<R>();
    supersede_keeps_position_and_status::<R>();
    hold_oldest_takes_smallest_positions::<R>();
    hold_oldest_with_limit_zero_holds_nothing::<R>();
    settle_removes_only_held::<R>();
    defer_takes_the_fresh_position::<R>();
    free_held_requeues_held_and_deferred::<R>();
    clone_is_independent::<R>();
}

fn key(text: &str) -> RecordKey {
    RecordKey::new(text)
}

fn payload(text: &str) -> Payload {
    Payload::from(text.as_bytes())
}

fn held(text: &str, rank: i64, bytes: &str) -> HeldRecord {
    HeldRecord::new(key(text), Rank::new(rank), payload(bytes))
}

fn insert<R: PocketRecords>(records: &mut R, text: &str, rank: i64, position: u64) {
    records.insert_queued(
        key(text),
        Rank::new(rank),
        payload(text),
        Position::new(position),
    );
    agree(records, "insert_queued");
}

fn hold<R: PocketRecords>(records: &mut R, limit: usize) -> Vec<HeldRecord> {
    let batch = records.hold_oldest(limit).into_iter().collect();
    agree(records, "hold_oldest");
    batch
}

/// `record_count` and `held` read the same state `snapshot` does, and `snapshot`
/// has one record per key.
fn agree<R: PocketRecords>(records: &R, step: &str) {
    let views = records.snapshot();
    assert_eq!(
        records.record_count(),
        views.len(),
        "record_count after {step}"
    );
    let keys: BTreeSet<&RecordKey> = views.iter().map(|view| &view.key).collect();
    assert_eq!(keys.len(), views.len(), "one record per key after {step}");
    let mut from_views: Vec<(Position, HeldRecord)> = views
        .into_iter()
        .filter_map(|view| match view.status {
            RecordStatus::Held { rank, payload } => {
                Some((view.position, HeldRecord::new(view.key, rank, payload)))
            }
            RecordStatus::Queued | RecordStatus::Deferred => None,
        })
        .collect();
    from_views.sort_by_key(|(position, _)| *position);
    let expected: Vec<HeldRecord> = from_views.into_iter().map(|(_, record)| record).collect();
    assert_eq!(records.held(), expected, "held after {step}");
}

fn view<R: PocketRecords>(records: &R, text: &str) -> Option<RecordView> {
    records.lookup(&key(text))
}

fn insert_then_lookup<R: PocketRecords>() {
    let mut records = R::default();
    assert_eq!(view(&records, "a"), None, "lookup of an absent key");
    assert_eq!(records.record_count(), 0, "a new store is empty");
    insert(&mut records, "a", 3, 7);
    let expected = RecordView::new(
        key("a"),
        Rank::new(3),
        payload("a"),
        Position::new(7),
        RecordStatus::Queued,
    );
    assert_eq!(
        view(&records, "a"),
        Some(expected.clone()),
        "insert_queued then lookup"
    );
    assert_eq!(
        records.snapshot(),
        vec![expected],
        "snapshot after one insert"
    );
}

fn a_second_insert_is_refused<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1, 1);
    insert(&mut records, "a", 2, 2);
    assert_eq!(
        records.record_count(),
        1,
        "a second insert of a queued key adds nothing"
    );
    assert_eq!(
        view(&records, "a").map(|v| (v.rank, v.position)),
        Some((Rank::new(1), Position::new(1))),
        "a second insert of a queued key changes nothing"
    );
    hold(&mut records, 1);
    insert(&mut records, "a", 3, 3);
    assert_eq!(
        records.record_count(),
        1,
        "a second insert of a held key adds nothing"
    );
    assert!(
        hold(&mut records, 10).is_empty(),
        "a refused insert of a held key queues nothing"
    );
    assert_eq!(
        records.held(),
        vec![held("a", 1, "a")],
        "a refused insert leaves the held version"
    );
    records.defer(&key("a"), Position::new(4));
    agree(&records, "defer");
    insert(&mut records, "a", 5, 5);
    assert_eq!(
        records.record_count(),
        1,
        "a second insert of a deferred key adds nothing"
    );
    assert_eq!(
        view(&records, "a").map(|v| (v.rank, v.position, v.status)),
        Some((Rank::new(1), Position::new(4), RecordStatus::Deferred)),
        "a second insert of a deferred key changes nothing"
    );
}

fn supersede_keeps_position_and_status<R: PocketRecords>() {
    let mut records = R::default();
    assert_eq!(
        records.supersede(&key("a"), Rank::new(1), payload("z")),
        None,
        "supersede of an absent key"
    );
    agree(&records, "supersede of an absent key");
    insert(&mut records, "a", 1, 4);
    assert_eq!(
        records.supersede(&key("a"), Rank::new(2), payload("new")),
        Some((Rank::new(1), payload("a"))),
        "supersede returns the loser"
    );
    agree(&records, "supersede");
    assert_eq!(
        hold(&mut records, 1),
        vec![held("a", 2, "new")],
        "the latest version is held"
    );
    assert_eq!(
        records.supersede(&key("a"), Rank::new(5), payload("newer")),
        Some((Rank::new(2), payload("new"))),
        "supersede of a held record"
    );
    agree(&records, "supersede of a held record");
    let after = view(&records, "a");
    let expected_status = RecordStatus::Held {
        rank: Rank::new(2),
        payload: payload("new"),
    };
    assert_eq!(
        after.map(|v| (v.rank, v.position, v.status)),
        Some((Rank::new(5), Position::new(4), expected_status)),
        "a supersede keeps position, status and held version"
    );
    assert!(
        hold(&mut records, 10).is_empty(),
        "a superseded held record is never taken again"
    );
    records.defer(&key("a"), Position::new(8));
    agree(&records, "defer");
    assert_eq!(
        records.supersede(&key("a"), Rank::new(6), payload("newest")),
        Some((Rank::new(5), payload("newer"))),
        "supersede of a deferred record"
    );
    agree(&records, "supersede of a deferred record");
    assert_eq!(
        view(&records, "a").map(|v| (v.rank, v.position, v.status)),
        Some((Rank::new(6), Position::new(8), RecordStatus::Deferred)),
        "a supersede keeps a deferred record deferred at its position"
    );
    assert!(
        hold(&mut records, 10).is_empty(),
        "a superseded deferred record is never taken"
    );
}

fn hold_oldest_takes_smallest_positions<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "c", 1, 30);
    insert(&mut records, "a", 1, 10);
    insert(&mut records, "b", 1, 20);
    assert_eq!(
        hold(&mut records, 2),
        vec![held("a", 1, "a"), held("b", 1, "b")],
        "the two oldest, oldest first"
    );
    assert_eq!(
        hold(&mut records, 10),
        vec![held("c", 1, "c")],
        "a held record is never taken again; a limit beyond the count takes all that remain"
    );
    assert!(
        hold(&mut records, 10).is_empty(),
        "nothing queued, nothing held"
    );
    records.defer(&key("a"), Position::new(40));
    agree(&records, "defer");
    assert!(
        hold(&mut records, 10).is_empty(),
        "a deferred record is never taken"
    );
}

fn hold_oldest_with_limit_zero_holds_nothing<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1, 1);
    assert!(
        hold(&mut records, 0).is_empty(),
        "a zero limit holds nothing"
    );
    assert_eq!(
        view(&records, "a").map(|v| v.status),
        Some(RecordStatus::Queued),
        "a zero limit leaves the record queued"
    );
}

fn settle_removes_only_held<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1, 1);
    insert(&mut records, "b", 1, 2);
    assert_eq!(records.settle(&key("a")), None, "settle of a queued key");
    agree(&records, "settle of a queued key");
    hold(&mut records, 1);
    records.supersede(&key("a"), Rank::new(4), payload("a4"));
    assert_eq!(
        records.settle(&key("a")),
        Some(held("a", 1, "a")),
        "settle returns the held version"
    );
    agree(&records, "settle");
    assert_eq!(view(&records, "a"), None, "settle removes the record");
    assert_eq!(records.record_count(), 1, "settle removes one record");
    assert_eq!(records.settle(&key("zz")), None, "settle of an absent key");
}

fn defer_takes_the_fresh_position<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1, 1);
    assert_eq!(
        records.defer(&key("a"), Position::new(9)),
        None,
        "defer of a queued key"
    );
    agree(&records, "defer of a queued key");
    hold(&mut records, 1);
    records.supersede(&key("a"), Rank::new(2), payload("a2"));
    assert_eq!(
        records.defer(&key("a"), Position::new(9)),
        Some(held("a", 1, "a")),
        "defer returns the held version"
    );
    agree(&records, "defer");
    let expected = RecordView::new(
        key("a"),
        Rank::new(2),
        payload("a2"),
        Position::new(9),
        RecordStatus::Deferred,
    );
    assert_eq!(
        view(&records, "a"),
        Some(expected),
        "defer takes the fresh position and drops the held version"
    );
    assert_eq!(
        records.defer(&key("a"), Position::new(10)),
        None,
        "defer of a deferred key"
    );
    assert_eq!(
        records.defer(&key("zz"), Position::new(10)),
        None,
        "defer of an absent key"
    );
}

fn free_held_requeues_held_and_deferred<R: PocketRecords>() {
    let mut records = R::default();
    assert_eq!(records.free_held(), 0, "free_held of an empty store");
    insert(&mut records, "held", 1, 1);
    insert(&mut records, "deferred", 1, 2);
    insert(&mut records, "queued", 1, 3);
    hold(&mut records, 2);
    records.supersede(&key("deferred"), Rank::new(2), payload("d2"));
    records.defer(&key("deferred"), Position::new(10));
    assert_eq!(
        records.free_held(),
        2,
        "free_held counts the held and the deferred"
    );
    agree(&records, "free_held");
    assert!(
        records
            .snapshot()
            .iter()
            .all(|v| v.status == RecordStatus::Queued),
        "free_held leaves every record queued"
    );
    assert_eq!(
        records.free_held(),
        0,
        "a second free_held finds nothing outstanding"
    );
    assert_eq!(
        hold(&mut records, 10),
        vec![
            held("held", 1, "held"),
            held("queued", 1, "queued"),
            held("deferred", 2, "d2")
        ],
        "free_held keeps each record's position"
    );
}

/// What a store shows, to compare before and after a change to its clone.
fn state<R: PocketRecords>(records: &R) -> (Vec<RecordView>, usize, Vec<HeldRecord>) {
    (records.snapshot(), records.record_count(), records.held())
}

/// Changes `records` by every kind of step.
fn change<R: PocketRecords>(records: &mut R) {
    hold(records, 1);
    records.supersede(&key("b"), Rank::new(9), payload("b9"));
    insert(records, "c", 1, 3);
    records.settle(&key("a"));
    agree(records, "a change to one copy");
}

fn clone_is_independent<R: PocketRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1, 1);
    insert(&mut records, "b", 1, 2);
    let mut other = records.clone();
    let before = state(&other);
    change(&mut records);
    assert_eq!(
        state(&other),
        before,
        "a change to the original never shows in its clone"
    );
    let before = state(&records);
    change(&mut other);
    assert_eq!(
        state(&records),
        before,
        "a change to the clone never shows in the original"
    );
}
