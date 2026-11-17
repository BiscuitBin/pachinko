//! The records contract, behind the `testing` feature: what every
//! [`PocketRecords`] implementation must show, written once and run over each, and
//! [`InspectRecords`], the view of a store the contract and the queue's inspection
//! seam read it through.

use std::collections::BTreeSet;

use super::{EndedVersion, PocketRecords, Position, RecordStatus, RecordView};
use crate::ids::{HeldRecord, Mark, MarkedRecord, Payload, Rank, RecordKey};
use crate::timestamp::Timestamp;

/// A store a test can read whole: every record it holds, as values.
///
/// No rule of the specification reads a pocket's records this way, so the step is
/// kept off [`PocketRecords`], the trait the queue drives in every build. A store
/// implements it to pass [`check_contract`] and to sit under a queue whose
/// [`Queue::snapshot`](crate::Queue::snapshot) a test reads.
///
/// ```
/// use libpachinko::records::{MemoryRecords, PocketRecords, Position};
/// use libpachinko::testing::InspectRecords;
/// use libpachinko::{Payload, Rank, RecordKey};
///
/// let mut records = MemoryRecords::default();
/// records.insert_queued(RecordKey::new("b"), Rank::new(1), Payload::default(), Position::new(0));
/// records.insert_queued(RecordKey::new("a"), Rank::new(1), Payload::default(), Position::new(1));
/// let keys: Vec<_> = records.snapshot().into_iter().map(|view| view.key).collect();
/// assert_eq!(keys, [RecordKey::new("a"), RecordKey::new("b")]);
/// ```
pub trait InspectRecords: PocketRecords {
    /// Every record, by key: what a test reads the invariants through.
    ///
    /// ```
    /// use libpachinko::records::{MemoryRecords, PocketRecords, Position, RecordStatus};
    /// use libpachinko::testing::InspectRecords;
    /// use libpachinko::{Payload, Rank, RecordKey};
    ///
    /// let mut records = MemoryRecords::default();
    /// assert!(records.snapshot().is_empty());
    /// records.insert_queued(RecordKey::new("k"), Rank::new(1), Payload::default(), Position::new(0));
    /// assert_eq!(records.snapshot()[0].status, RecordStatus::Queued);
    /// ```
    fn snapshot(&self) -> Vec<RecordView>;
}

/// Checks every step of the records contract over fresh instances of `R`, and
/// after every call that [`record_count`](PocketRecords::record_count) and
/// [`held`](PocketRecords::held) agree with [`snapshot`](InspectRecords::snapshot).
///
/// It pins the order [`held`](PocketRecords::held) and
/// [`end_lease`](PocketRecords::end_lease) go in, smallest held position first,
/// with records whose key order disagrees with their position order.
///
/// The contract checks behaviour, not cost. The O(held × log records) bound on
/// [`end_lease`](PocketRecords::end_lease), and the other documented costs, are
/// obligations the implementor must meet and the contract does not prove;
/// [`MemoryRecords`](super::MemoryRecords)' index of the held keys by position
/// shows the required shape.
///
/// # Panics
///
/// When `R` breaks the contract, naming the step that broke it.
///
/// ```
/// use libpachinko::records::MemoryRecords;
/// use libpachinko::testing::check_contract;
///
/// check_contract::<MemoryRecords>();
/// ```
pub fn check_contract<R: InspectRecords>() {
    insert_then_lookup::<R>();
    a_second_insert_is_refused::<R>();
    hold_oldest_takes_smallest_positions::<R>();
    hold_oldest_with_limit_zero_holds_nothing::<R>();
    supersede_replaces_a_queued_version::<R>();
    supersede_never_sheds_a_held_version::<R>();
    defer_behind_held_sets_the_deferred_version::<R>();
    supersede_replaces_a_deferred_version::<R>();
    mark_acts_only_on_an_unsettled_held_record::<R>();
    held_lists_marks_smallest_position_first::<R>();
    end_lease_frees_by_the_six_rows::<R>();
    end_lease_takes_the_smallest_held_position_first::<R>();
    clone_is_independent::<R>();
}

fn key(text: &str) -> RecordKey {
    RecordKey::new(text)
}

/// The payload of `text`'s version at `rank`, so every version is told apart.
fn version(text: &str, rank: i64) -> Payload {
    Payload::from(format!("{text}{rank}").as_bytes())
}

const fn at(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

fn held(text: &str, rank: i64) -> HeldRecord {
    HeldRecord::new(key(text), Rank::new(rank), version(text, rank))
}

/// `text` held at its first version, rank 1, with `mark`.
fn marked(text: &str, mark: Mark) -> MarkedRecord {
    MarkedRecord::new(key(text), Rank::new(1), version(text, 1), mark)
}

/// The status of `text` held at its version at `rank`.
fn holding(
    text: &str,
    rank: i64,
    mark: Mark,
    nacked_at: Option<u64>,
    deferred: Option<u64>,
) -> RecordStatus {
    RecordStatus::Held {
        rank: Rank::new(rank),
        payload: version(text, rank),
        mark,
        nacked_at: nacked_at.map(at),
        deferred_position: deferred.map(Position::new),
    }
}

fn record(text: &str, rank: i64, position: u64, status: RecordStatus) -> RecordView {
    RecordView::new(
        key(text),
        Rank::new(rank),
        version(text, rank),
        Position::new(position),
        status,
    )
}

fn view<R: InspectRecords>(records: &R, text: &str) -> Option<RecordView> {
    records.lookup(&key(text))
}

/// Inserts `text` queued at its first version, rank 1.
fn insert<R: InspectRecords>(records: &mut R, text: &str, position: u64) {
    records.insert_queued(
        key(text),
        Rank::new(1),
        version(text, 1),
        Position::new(position),
    );
    agree(records, "insert_queued");
}

fn hold<R: InspectRecords>(records: &mut R, limit: usize) -> Vec<HeldRecord> {
    let batch = records.hold_oldest(limit).into_iter().collect();
    agree(records, "hold_oldest");
    batch
}

fn supersede<R: InspectRecords>(records: &mut R, text: &str, rank: i64) -> Option<(Rank, Payload)> {
    let loser = records.supersede(&key(text), Rank::new(rank), version(text, rank));
    agree(records, "supersede");
    loser
}

fn defer<R: InspectRecords>(records: &mut R, text: &str, rank: i64, position: u64) {
    records.defer_behind_held(
        &key(text),
        Rank::new(rank),
        version(text, rank),
        Position::new(position),
    );
    agree(records, "defer_behind_held");
}

fn mark<R: InspectRecords>(records: &mut R, text: &str, mark: Mark, millis: u64) {
    records.mark(&key(text), mark, at(millis));
    agree(records, "mark");
}

fn end<R: InspectRecords>(records: &mut R) -> Vec<EndedVersion> {
    let ended = records.end_lease();
    agree(records, "end_lease");
    ended
}

/// `record_count` and `held` read the same state `snapshot` does, and `snapshot`
/// has one record per key.
fn agree<R: InspectRecords>(records: &R, step: &str) {
    let views = records.snapshot();
    assert_eq!(
        records.record_count(),
        views.len(),
        "record_count after {step}"
    );
    let keys: BTreeSet<&RecordKey> = views.iter().map(|view| &view.key).collect();
    assert_eq!(keys.len(), views.len(), "one record per key after {step}");
    let mut from_views: Vec<(Position, MarkedRecord)> = views
        .into_iter()
        .filter_map(|view| {
            let RecordStatus::Held {
                rank,
                payload,
                mark,
                ..
            } = view.status
            else {
                return None;
            };
            Some((
                view.position,
                MarkedRecord::new(view.key, rank, payload, mark),
            ))
        })
        .collect();
    from_views.sort_by_key(|(position, _)| *position);
    let expected: Vec<MarkedRecord> = from_views.into_iter().map(|(_, record)| record).collect();
    assert_eq!(records.held(), expected, "held after {step}");
}

/// What a store shows, to compare before and after a step.
fn state<R: InspectRecords>(records: &R) -> (Vec<RecordView>, usize, Vec<MarkedRecord>) {
    (records.snapshot(), records.record_count(), records.held())
}

/// Runs `step` on `records`, checks it changed nothing the store shows, and
/// returns what it returned.
fn unchanged<R: InspectRecords, T>(
    records: &mut R,
    name: &str,
    step: impl FnOnce(&mut R) -> T,
) -> T {
    let before = state(records);
    let value = step(records);
    assert_eq!(state(records), before, "{name} changes nothing");
    value
}

fn insert_then_lookup<R: InspectRecords>() {
    let mut records = R::default();
    assert_eq!(view(&records, "a"), None, "lookup of an absent key");
    assert_eq!(records.record_count(), 0, "a new store is empty");
    insert(&mut records, "a", 7);
    let expected = record("a", 1, 7, RecordStatus::Queued);
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

fn a_second_insert_is_refused<R: InspectRecords>() {
    let again = |records: &mut R| {
        records.insert_queued(key("a"), Rank::new(5), version("a", 5), Position::new(5));
    };
    let mut records = R::default();
    insert(&mut records, "a", 1);
    unchanged(&mut records, "a second insert of a queued key", again);
    hold(&mut records, 1);
    unchanged(&mut records, "a second insert of a held key", again);
    defer(&mut records, "a", 2, 4);
    unchanged(
        &mut records,
        "a second insert of a key with a deferred version",
        again,
    );
    assert_eq!(records.record_count(), 1, "one record per key");
}

fn hold_oldest_takes_smallest_positions<R: InspectRecords>() {
    let mut records = R::default();
    insert(&mut records, "c", 30);
    insert(&mut records, "a", 10);
    insert(&mut records, "b", 20);
    assert_eq!(
        hold(&mut records, 2),
        vec![held("a", 1), held("b", 1)],
        "the two oldest, oldest first"
    );
    assert_eq!(
        hold(&mut records, 10),
        vec![held("c", 1)],
        "a held record is never taken again; a limit beyond the count takes all that remain"
    );
    assert!(
        hold(&mut records, 10).is_empty(),
        "nothing queued, nothing held"
    );
}

fn hold_oldest_with_limit_zero_holds_nothing<R: InspectRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1);
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

fn supersede_replaces_a_queued_version<R: InspectRecords>() {
    let mut records = R::default();
    let absent = unchanged(&mut records, "supersede of an absent key", |r| {
        r.supersede(&key("a"), Rank::new(2), version("a", 2))
    });
    assert_eq!(absent, None, "supersede of an absent key returns nothing");
    insert(&mut records, "a", 4);
    assert_eq!(
        supersede(&mut records, "a", 2),
        Some((Rank::new(1), version("a", 1))),
        "supersede of a queued record returns the loser"
    );
    assert_eq!(
        view(&records, "a"),
        Some(record("a", 2, 4, RecordStatus::Queued)),
        "supersede keeps a queued record's position and status"
    );
    assert_eq!(
        hold(&mut records, 1),
        vec![held("a", 2)],
        "the latest version is held"
    );
}

fn supersede_never_sheds_a_held_version<R: InspectRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 4);
    insert(&mut records, "b", 5);
    hold(&mut records, 2);
    mark(&mut records, "b", Mark::Acked, 3);
    for text in ["a", "b"] {
        let loser = unchanged(
            &mut records,
            "supersede of a held record with nothing deferred",
            |r| r.supersede(&key(text), Rank::new(2), version(text, 2)),
        );
        assert_eq!(loser, None, "supersede never returns a held version");
    }
}

fn defer_behind_held_sets_the_deferred_version<R: InspectRecords>() {
    let deferral = |records: &mut R| {
        records.defer_behind_held(&key("a"), Rank::new(2), version("a", 2), Position::new(9));
    };
    let mut records = R::default();
    unchanged(&mut records, "defer_behind_held of an absent key", deferral);
    insert(&mut records, "a", 1);
    unchanged(&mut records, "defer_behind_held of a queued key", deferral);
    hold(&mut records, 1);
    defer(&mut records, "a", 2, 9);
    assert_eq!(
        view(&records, "a"),
        Some(record(
            "a",
            2,
            1,
            holding("a", 1, Mark::Unsettled, None, Some(9))
        )),
        "defer_behind_held sets the latest version and deferred_position and keeps the rest"
    );
    unchanged(&mut records, "a second defer_behind_held", |r| {
        r.defer_behind_held(&key("a"), Rank::new(3), version("a", 3), Position::new(10));
    });
    assert!(
        hold(&mut records, 10).is_empty(),
        "a held record with a deferred version is never taken"
    );
}

fn supersede_replaces_a_deferred_version<R: InspectRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 4);
    insert(&mut records, "b", 5);
    hold(&mut records, 2);
    mark(&mut records, "a", Mark::Nacked, 3);
    mark(&mut records, "b", Mark::Acked, 3);
    defer(&mut records, "a", 2, 9);
    defer(&mut records, "b", 2, 10);
    let acked = record("b", 2, 5, holding("b", 1, Mark::Acked, None, Some(10)));
    assert_eq!(
        view(&records, "b"),
        Some(acked),
        "defer_behind_held keeps the mark"
    );
    assert_eq!(
        supersede(&mut records, "a", 3),
        Some((Rank::new(2), version("a", 2))),
        "supersede of a held record with a deferred version returns the deferred version"
    );
    let nacked = holding("a", 1, Mark::Nacked, Some(3), Some(9));
    assert_eq!(
        view(&records, "a"),
        Some(record("a", 3, 4, nacked)),
        "supersede keeps the held version, the mark, the nack time and both positions"
    );
}

fn mark_acts_only_on_an_unsettled_held_record<R: InspectRecords>() {
    let mut records = R::default();
    unchanged(&mut records, "mark of an absent key", |r| {
        r.mark(&key("a"), Mark::Acked, at(1));
    });
    insert(&mut records, "a", 1);
    insert(&mut records, "b", 2);
    unchanged(&mut records, "mark of a queued key", |r| {
        r.mark(&key("a"), Mark::Nacked, at(1));
    });
    hold(&mut records, 2);
    unchanged(&mut records, "a mark of unsettled", |r| {
        r.mark(&key("a"), Mark::Unsettled, at(1));
    });
    mark(&mut records, "a", Mark::Acked, 2);
    mark(&mut records, "b", Mark::Nacked, 5);
    let marks = (
        Some(holding("a", 1, Mark::Acked, None, None)),
        Some(holding("b", 1, Mark::Nacked, Some(5), None)),
    );
    let statuses = |records: &R| {
        (
            view(records, "a").map(|v| v.status),
            view(records, "b").map(|v| v.status),
        )
    };
    assert_eq!(
        statuses(&records),
        marks,
        "an ack stores no time and a nack stores its own"
    );
    for text in ["a", "b"] {
        for again in [Mark::Acked, Mark::Nacked, Mark::Unsettled] {
            unchanged(&mut records, "a second mark", |r| {
                r.mark(&key(text), again, at(9));
            });
        }
    }
    assert_eq!(
        statuses(&records),
        marks,
        "the first mark and its time stand"
    );
}

/// One held record per `Freed` row, the keys running against the positions:
/// unsettled `f` at 1 and `e` at 2, acked `d` at 3 and `c` at 4, nacked `b` at 5
/// and `a` at 6. `e`, `c` and `a` have a version deferred to 22, 21 and 20, and
/// `q` is queued at 10, between the held and the deferred positions. Each
/// deferral comes before the mark, as a publish while held does before an ack.
fn one_per_row<R: InspectRecords>() -> R {
    let mut records = R::default();
    let rows = [("f", 1), ("e", 2), ("d", 3), ("c", 4), ("b", 5), ("a", 6)];
    for (text, position) in rows.into_iter().chain([("q", 10)]) {
        insert(&mut records, text, position);
    }
    hold(&mut records, rows.len());
    for (text, position) in [("e", 22), ("c", 21), ("a", 20)] {
        defer(&mut records, text, 2, position);
    }
    let marks = [
        ("d", Mark::Acked, 7),
        ("c", Mark::Acked, 7),
        ("b", Mark::Nacked, 8),
        ("a", Mark::Nacked, 9),
    ];
    for (text, mark_as, millis) in marks {
        mark(&mut records, text, mark_as, millis);
    }
    records
}

fn held_lists_marks_smallest_position_first<R: InspectRecords>() {
    let records = one_per_row::<R>();
    assert_eq!(
        records.held(),
        vec![
            marked("f", Mark::Unsettled),
            marked("e", Mark::Unsettled),
            marked("d", Mark::Acked),
            marked("c", Mark::Acked),
            marked("b", Mark::Nacked),
            marked("a", Mark::Nacked),
        ],
        "held lists every held record at its held version with its mark, smallest position first"
    );
}

fn end_lease_frees_by_the_six_rows<R: InspectRecords>() {
    let mut records = one_per_row::<R>();
    let letter = |text, millis| EndedVersion::DeadLettered {
        held: held(text, 1),
        nacked_at: at(millis),
    };
    assert_eq!(
        end(&mut records),
        vec![
            EndedVersion::Superseded(held("e", 1)),
            letter("b", 8),
            letter("a", 9)
        ],
        "end_lease sheds a superseded held version and dead-letters a nacked one, in position order"
    );
    let queued = |text, rank, position| record(text, rank, position, RecordStatus::Queued);
    assert_eq!(
        records.snapshot(),
        vec![
            queued("a", 2, 20),
            queued("c", 2, 21),
            queued("e", 2, 2),
            queued("f", 1, 1),
            queued("q", 1, 10)
        ],
        "end_lease requeues by the six rows and leaves a queued record alone"
    );
    assert_eq!(
        records.record_count(),
        5,
        "end_lease removes the acked and the nacked record with nothing deferred"
    );
    let again = unchanged(&mut records, "a second end_lease", R::end_lease);
    assert!(again.is_empty(), "a second end_lease takes nothing out");
    assert_eq!(
        hold(&mut records, 10),
        vec![
            held("f", 1),
            held("e", 2),
            held("q", 1),
            held("a", 2),
            held("c", 2)
        ],
        "the records end_lease queued are taken in position order at their latest version"
    );
    assert_eq!(
        view(&records, "e").map(|v| v.status),
        Some(holding("e", 2, Mark::Unsettled, None, None)),
        "a record held again is unsettled with nothing deferred"
    );
}

fn end_lease_takes_the_smallest_held_position_first<R: InspectRecords>() {
    let mut records = R::default();
    let nothing = unchanged(&mut records, "end_lease of an empty store", R::end_lease);
    assert!(
        nothing.is_empty(),
        "end_lease of an empty store takes nothing out"
    );
    insert(&mut records, "z", 1);
    insert(&mut records, "a", 2);
    hold(&mut records, 2);
    mark(&mut records, "z", Mark::Nacked, 3);
    mark(&mut records, "a", Mark::Nacked, 4);
    defer(&mut records, "z", 2, 20);
    defer(&mut records, "a", 2, 10);
    let letter = |text, millis| EndedVersion::DeadLettered {
        held: held(text, 1),
        nacked_at: at(millis),
    };
    assert_eq!(
        end(&mut records),
        vec![letter("z", 3), letter("a", 4)],
        "end_lease goes smallest held position first, not by key or deferred position"
    );
}

/// Changes `records` by every kind of step.
fn change<R: InspectRecords>(records: &mut R) {
    hold(records, 2);
    mark(records, "a", Mark::Nacked, 1);
    defer(records, "b", 2, 4);
    supersede(records, "b", 3);
    insert(records, "c", 3);
    end(records);
}

fn clone_is_independent<R: InspectRecords>() {
    let mut records = R::default();
    insert(&mut records, "a", 1);
    insert(&mut records, "b", 2);
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
