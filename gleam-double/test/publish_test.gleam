//// The specification's Publish section.
////
//// First the six catalog rows this ticket owns, S011, S013, S014, S016, S017
//// and S018, each a port of its Rust test in
//// `crates/libpachinko/tests/queue/publish.rs`, driving the owner. A port
//// asserts what its Rust test asserts, with one change: where the Rust
//// checks a publish for an outcome, the port asserts that the publish
//// answered nothing and names the rule it fired, and asserts the state that
//// publish left through the snapshot (the maintainer's ruling on N32 in
//// `double/SPEC-NOTES.md`). A setup publish the Rust unwraps is asserted by
//// its reply alone. An absolute position is `impl-choice`: it holds only
//// because the queue's first position is 0 (spec notes G04 and G16).
////
//// Then the double's own tests, which pin no catalog row: the refusal of a
//// publisher that never joined, what each of the four rules changes and how
//// often it reads the clock, the events each emits, and the held branch of
//// `Supersede`'s guard, which no publish through the owner reaches until a
//// record can be held.

import gleam/list
import gleam/option.{type Option, None, Some}
import harness.{type Double}
import pachinko_double/config
import pachinko_double/core
import pachinko_double/domain.{type Event, type Plan, PocketTouched}
import pachinko_double/inspection.{Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type Mark, type Reason, type Record, type State, Acked, Consumer, Lease,
  LeaseActive, Nacked, Pocket, PocketLeased, PocketQueued, Publisher, Queue,
  Record, RecordHeld, RecordQueued, Stale, State, Superseded, SupersededRecord,
  Unsettled,
}
import pachinko_double/owner.{type Reply, Outcome}
import pachinko_double/publish.{type Publication, Publication}
import pachinko_double/refusals.{EqualRankRefused, UnknownPublisher}
import pachinko_double/rules.{
  Applied, Defect, Done, EffectFailed, Finish, Impossible, LeaseId,
  NothingHappens, Now, Position, Refused, Row, Table,
}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a publish into a new pocket creates a queued pocket and record",
      [Scenario("S011")],
      new_pocket,
    ),
    Test(
      "the same record key in two pockets is two records",
      [Scenario("S013")],
      same_key_in_two_pockets,
    ),
    Test(
      "a supersede replaces rank and payload in place and sheds the loser",
      [Scenario("S014")],
      supersede_in_place,
    ),
    Test(
      "a stale publish is shed, not applied and not an error",
      [Scenario("S016")],
      stale_is_shed,
    ),
    Test(
      "an equal-rank publish is rejected",
      [Scenario("S017")],
      equal_rank_rejected,
    ),
    Test(
      "negative and zero ranks order like integers",
      [Scenario("S018")],
      negative_and_zero_ranks,
    ),
    Test(
      "a publish by a publisher that never joined is refused",
      [],
      unknown_publisher,
    ),
    Test(
      "PublishIntoNewPocket changes exactly what it ensures and reads no clock",
      [],
      into_new_pocket_exactly,
    ),
    Test(
      "PublishNewKey changes exactly what it ensures and reads no clock",
      [],
      new_key_exactly,
    ),
    Test(
      "Supersede changes exactly what it ensures and reads the clock once",
      [],
      supersede_exactly,
    ),
    Test(
      "DropStalePublish changes exactly what it ensures and reads the clock once",
      [],
      stale_exactly,
    ),
    Test(
      "PublishNewKey and Supersede touch the pocket, and the other two emit nothing",
      [],
      events,
    ),
    Test(
      "Supersede takes a held record only with a version deferred behind it",
      [],
      held_record_guards,
    ),
    Test(
      "Supersede sheds the version deferred behind a held one",
      [],
      supersede_deferred,
    ),
    Test(
      "Supersede's effect without its record is a defect, and reads no clock",
      [],
      supersede_without_record,
    ),
  ]
}

/// What a publish that fired this rule replies: the rule's name, and no
/// answer. The Rust names an outcome here; the specification's Publish rules
/// answer nothing (the ruling on N32).
fn answers_nothing(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

/// The Rust test file's own helper: a record's rank and payload.
fn rank_and_payload(
  double: Double,
  pocket_key: String,
  record_key: String,
) -> #(Int, BitArray) {
  let record = harness.record(double, pocket_key, record_key)
  #(record.rank, record.payload)
}

/// What the superseded queue holds at this rank: each entry's payload and
/// reason. It reads the queue by rank, so a port that uses it asserts no
/// order of the queue (spec note G19).
fn shed_at_rank(double: Double, rank: Int) -> List(#(BitArray, Reason)) {
  harness.inspect(double).superseded
  |> list.filter(fn(shed) { shed.rank == rank })
  |> list.map(fn(shed) { #(shed.payload, shed.reason) })
}

/// How many entries the superseded queue holds.
fn shed_count(double: Double) -> Int {
  list.length(harness.inspect(double).superseded)
}

/// The Rust fixture's `SUPERSEDED_TTL`.
fn superseded_ttl() -> Int {
  harness.fixture_settings().superseded_ttl
}

// The catalog rows -----------------------------------------

fn new_pocket() -> List(String) {
  let double = harness.fixture()
  // `impl-choice` (spec notes G04 and G16): the specification gives the
  // counter no first value. So are the three absolute positions below.
  assert harness.inspect(double).next_position == 0
  assert harness.publish(double, "pk", "a", 3, "x")
    == answers_nothing("PublishIntoNewPocket")
  let pocket = harness.pocket(double, "pk")
  assert pocket.status == PocketQueued
  assert pocket.position == 0
  assert pocket.consecutive_expiries == 0
  let record = harness.record(double, "pk", "a")
  assert record.position == 1
  assert record.status == RecordQueued
  assert rank_and_payload(double, "pk", "a") == #(3, <<"x":utf8>>)
  assert harness.inspect(double).next_position == 2
  assert harness.order(double) == ["pk"]
  harness.fired(double)
}

fn same_key_in_two_pockets() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "one", "k", 1, "x")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "two", "k", 1, "y")
    == answers_nothing("PublishIntoNewPocket")
  assert rank_and_payload(double, "one", "k") == #(1, <<"x":utf8>>)
  assert rank_and_payload(double, "two", "k") == #(1, <<"y":utf8>>)
  assert harness.record(double, "one", "k").position
    != harness.record(double, "two", "k").position
  assert harness.inspect(double).superseded == []
  harness.fired(double)
}

fn supersede_in_place() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 1, "old")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "b", 1, "b")
    == answers_nothing("PublishNewKey")
  let now = harness.advance(double.clock, harness.t0, 7)
  assert harness.publish(double, "pk", "a", 2, "new")
    == answers_nothing("Supersede")
  let record = harness.record(double, "pk", "a")
  // `impl-choice` (spec notes G04 and G16): the absolute position 1, and the
  // counter at 3 below. That the record is where its first publish put it is
  // the specification's.
  assert #(record.position, record.status) == #(1, RecordQueued)
  assert rank_and_payload(double, "pk", "a") == #(2, <<"new":utf8>>)
  let snapshot = harness.inspect(double)
  assert snapshot.next_position == 3
  let assert [shed] = snapshot.superseded
  assert shed.pocket == "pk"
  assert shed.key == "a"
  assert #(shed.rank, shed.payload) == #(1, <<"old":utf8>>)
  assert shed.reason == Superseded
  assert #(shed.shed_at, shed.expires_at) == #(now, now + superseded_ttl())
  // The Rust's `invariants::shed_before_expiry`, by the checker.
  assert !list.contains(invariants.check(snapshot), "ShedBeforeExpiry")
  harness.fired(double)
}

fn stale_is_shed() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 5, "kept")
    == answers_nothing("PublishIntoNewPocket")
  let now = harness.advance(double.clock, harness.t0, 3)
  // "Not an error": the publish succeeds, and answers nothing.
  assert harness.publish(double, "pk", "a", 4, "late")
    == answers_nothing("DropStalePublish")
  assert rank_and_payload(double, "pk", "a") == #(5, <<"kept":utf8>>)
  let snapshot = harness.inspect(double)
  // `impl-choice` (spec notes G04 and G16): the absolute counter. That a
  // stale publish allots nothing is the specification's.
  assert snapshot.next_position == 2
  let assert [shed] = snapshot.superseded
  assert #(shed.rank, shed.reason) == #(4, Stale)
  assert shed.payload == <<"late":utf8>>
  assert #(shed.shed_at, shed.expires_at) == #(now, now + superseded_ttl())
  harness.fired(double)
}

/// No rule covers an equal rank, so the publish is refused and changes
/// nothing, whatever its payload. The Rust's `EqualRank` carries the record
/// key and the rank; the double's refusal names the pocket as well (spec
/// note G13). The refusal reads no clock.
fn equal_rank_rejected() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 2, "first")
    == answers_nothing("PublishIntoNewPocket")
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  assert harness.publish(double, "pk", "a", 2, "second")
    == Error(
      Refused(EqualRankRefused(pocket_key: "pk", record_key: "a", rank: 2)),
    )
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  harness.fired(double)
}

/// The Rust checks each of the last three publishes for an outcome:
/// superseded, superseded and stale. The port asserts the state each one
/// leaves instead (the ruling on N32): the version the record stores, and
/// the one version that publish shed, with its reason.
fn negative_and_zero_ranks() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", -5, "m5")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "a", -1, "m1")
    == answers_nothing("Supersede")
  assert rank_and_payload(double, "pk", "a") == #(-1, <<"m1":utf8>>)
  assert shed_at_rank(double, -5) == [#(<<"m5":utf8>>, Superseded)]
  assert shed_count(double) == 1
  assert harness.publish(double, "pk", "a", 0, "z")
    == answers_nothing("Supersede")
  assert rank_and_payload(double, "pk", "a") == #(0, <<"z":utf8>>)
  assert shed_at_rank(double, -1) == [#(<<"m1":utf8>>, Superseded)]
  assert shed_count(double) == 2
  assert harness.publish(double, "pk", "a", -2, "m2")
    == answers_nothing("DropStalePublish")
  assert shed_at_rank(double, -2) == [#(<<"m2":utf8>>, Stale)]
  assert shed_count(double) == 3
  assert rank_and_payload(double, "pk", "a") == #(0, <<"z":utf8>>)
  harness.fired(double)
}

// The refusal of a publisher that never joined --------------

/// `impl-choice` (spec note G12): the specification states no such refusal.
/// `q` never joined, and `c1` joined as a consumer only. Each publish here
/// would fire a rule had `p` sent it: a supersede, which reads the clock, a
/// new key, a new pocket and a stale publish, which reads it too. Each is
/// refused, the complete snapshot is unchanged and no clock was read.
fn unknown_publisher() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 1, "x")
    == answers_nothing("PublishIntoNewPocket")
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  let refused = fn(publisher) { Error(Refused(UnknownPublisher(publisher))) }
  assert harness.publish_as(double, "q", "pk", "a", 2, "y") == refused("q")
  assert harness.publish_as(double, "q", "pk", "b", 1, "y") == refused("q")
  assert harness.publish_as(double, "q", "other", "a", 1, "y") == refused("q")
  assert harness.publish_as(double, "q", "pk", "a", 0, "y") == refused("q")
  assert harness.publish_as(double, "c1", "pk", "a", 2, "y") == refused("c1")
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  harness.fired(double)
}

// What each rule changes, and how often it reads the clock ---
//
// Each test compares the complete snapshot after one publish with the
// snapshot before it, changed as the rule's `ensures` lines say and in no
// other way. A position is the counter's value before the publish, never an
// absolute number. Where a list gains an entry, that it gains it at the end
// is `impl-choice` (spec note G19): the specification orders the queued
// pockets and a pocket's queued records, both by position, and gives no
// order to the pockets as a collection, to all the records, to the leases
// or to either shed queue.

/// The superseded queue already holds an entry, which the rule leaves alone.
fn into_new_pocket_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "one", "a", 1, "x")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "one", "a", 2, "w")
    == answers_nothing("Supersede")
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let reads = harness.clock_reads(double.clock)
  assert harness.publish(double, "two", "b", -4, "y")
    == answers_nothing("PublishIntoNewPocket")
  // The pocket takes the counter's value and its first record the next.
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      next_position: before.next_position + 2,
      pockets: list.append(before.pockets, [
        Pocket(
          key: "two",
          status: PocketQueued,
          position: before.next_position,
          consecutive_expiries: 0,
        ),
      ]),
      records: list.append(before.records, [
        Record(
          pocket: "two",
          key: "b",
          rank: -4,
          payload: <<"y":utf8>>,
          position: before.next_position + 1,
          status: RecordQueued,
        ),
      ]),
    )
  assert harness.clock_reads(double.clock) == reads
  harness.fired(double)
}

/// The key `a` is new to pocket `two` though pocket `one` holds it, and at a
/// higher rank: a record of another pocket is not this pocket's. The
/// superseded queue already holds an entry, which the rule leaves alone.
fn new_key_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "one", "a", 5, "x")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "two", "b", 1, "y")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "one", "a", 6, "w")
    == answers_nothing("Supersede")
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let reads = harness.clock_reads(double.clock)
  assert harness.publish(double, "two", "a", 2, "z")
    == answers_nothing("PublishNewKey")
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      next_position: before.next_position + 1,
      records: list.append(before.records, [
        Record(
          pocket: "two",
          key: "a",
          rank: 2,
          payload: <<"z":utf8>>,
          position: before.next_position,
          status: RecordQueued,
        ),
      ]),
    )
  assert harness.clock_reads(double.clock) == reads
  harness.fired(double)
}

/// Two supersedes of one record, `b` of pocket `other`, with the clock
/// moved before each: each reads the clock once, and each loser is shed
/// under the publication's pocket and key, at the time of the publish that
/// beat it, for the configured `superseded_ttl`. The record keeps its
/// position and its place, and the counter does not move. Pocket `pk` holds
/// the same key at the same rank, and its record is left alone.
fn supersede_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "other", "b", 1, "old")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "other", "a", 1, "a")
    == answers_nothing("PublishNewKey")
  assert harness.publish(double, "pk", "b", 1, "pk")
    == answers_nothing("PublishIntoNewPocket")
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  let assert [b, a, elsewhere] = before.records

  let first = harness.advance(double.clock, harness.t0, 20)
  assert harness.publish(double, "other", "b", 3, "new")
    == answers_nothing("Supersede")
  let shed_first =
    SupersededRecord(
      pocket: "other",
      key: "b",
      rank: 1,
      payload: <<"old":utf8>>,
      reason: Superseded,
      shed_at: first,
      expires_at: first + superseded_ttl(),
    )
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      records: [Record(..b, rank: 3, payload: <<"new":utf8>>), a, elsewhere],
      superseded: [shed_first],
    )
  assert harness.clock_reads(double.clock) == reads + 1

  let second = harness.advance(double.clock, first, 25)
  assert harness.publish(double, "other", "b", 9, "newer")
    == answers_nothing("Supersede")
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      records: [Record(..b, rank: 9, payload: <<"newer":utf8>>), a, elsewhere],
      superseded: [
        shed_first,
        SupersededRecord(
          pocket: "other",
          key: "b",
          rank: 3,
          payload: <<"new":utf8>>,
          reason: Superseded,
          shed_at: second,
          expires_at: second + superseded_ttl(),
        ),
      ],
    )
  assert harness.clock_reads(double.clock) == reads + 2
  harness.fired(double)
}

/// The stale version itself is shed, under the publication's pocket and
/// key and at the time of its publish, behind the entry the superseded
/// queue already holds. No record, no pocket and no counter changes.
fn stale_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "other", "b", 3, "old")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "other", "b", 5, "kept")
    == answers_nothing("Supersede")
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let reads = harness.clock_reads(double.clock)
  let now = harness.advance(double.clock, harness.t0, 33)
  assert harness.publish(double, "other", "b", 4, "late")
    == answers_nothing("DropStalePublish")
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      superseded: list.append(before.superseded, [
        SupersededRecord(
          pocket: "other",
          key: "b",
          rank: 4,
          payload: <<"late":utf8>>,
          reason: Stale,
          shed_at: now,
          expires_at: now + superseded_ttl(),
        ),
      ]),
    )
  assert harness.clock_reads(double.clock) == reads + 1
  harness.fired(double)
}

// The rows themselves, over states built by hand -------------
//
// `PocketTouched`'s table has no row yet, so an emitted event shows neither
// in the rules that fired nor in the snapshot; and no publish through the
// owner meets a held record until a pull lands. These tests read the Publish
// table directly.

fn publication(
  pocket_key: String,
  record_key: String,
  rank: Int,
  payload: String,
) -> Publication {
  Publication(publisher: "p", pocket_key:, record_key:, rank:, payload: <<
    payload:utf8,
  >>)
}

/// Publisher `p` and consumer `c1` joined, and nothing stored.
fn joined() -> State {
  State(..model.new(config.default()), publishers: [Publisher("p")], consumers: [
    Consumer("c1"),
  ])
}

/// Two queued pockets, each holding one queued record at rank 5: `pk` holds
/// `a`, and `other` holds `b`.
fn stored() -> State {
  State(
    ..joined(),
    queue: Queue(next_position: 4),
    pockets: [
      Pocket("pk", PocketQueued, 0, 0),
      Pocket("other", PocketQueued, 2, 0),
    ],
    records: [
      Record("pk", "a", 5, <<"x":utf8>>, 1, RecordQueued),
      Record("other", "b", 5, <<"x":utf8>>, 3, RecordQueued),
    ],
  )
}

/// The three marks a held record can carry. The Publish rules read none of
/// them: the stored rank is the record's whatever its mark.
fn marks() -> List(Mark) {
  [Unsettled, Acked, Nacked]
}

/// Record `a` of the leased pocket `pk`, handed out at rank 5 and since
/// marked so; a nacked record has the time of its nack. With `deferred`, a
/// version at rank 6 arrived since and waits behind the held one, at the
/// arrival position 2; without, nothing is deferred.
fn held_record(deferred: Bool, mark: Mark) -> Record {
  let nacked_at = case mark {
    Nacked -> Some(harness.t0 + 5)
    Unsettled | Acked -> None
  }
  let held = fn(deferred_position) {
    RecordHeld(
      held_rank: 5,
      held_payload: <<"x":utf8>>,
      mark:,
      nacked_at:,
      deferred_position:,
    )
  }
  case deferred {
    True -> Record("pk", "a", 6, <<"newer":utf8>>, 1, held(Some(2)))
    False -> Record("pk", "a", 5, <<"x":utf8>>, 1, held(None))
  }
}

/// Pocket `pk` leased to `c1`, holding `held_record(deferred, mark)`.
fn leased(deferred: Bool, mark: Mark) -> State {
  State(
    ..joined(),
    queue: Queue(next_position: 3),
    pockets: [Pocket("pk", PocketLeased, 0, 0)],
    records: [held_record(deferred, mark)],
    leases: [
      Lease(
        id: "lease-1",
        pocket: "pk",
        consumer: "c1",
        status: LeaseActive,
        granted_at: harness.t0,
        expires_at: harness.t0 + 1000,
      ),
    ],
  )
}

/// Walks a row's plan by hand to the events it finishes with, answering a
/// request for a position with the counter and one for the time with `t0`.
fn walk(plan: Plan, next_position: Int) -> Result(List(Event), String) {
  case plan {
    Finish(events:, ..) -> Ok(events)
    Now(next) -> walk(next(harness.t0), next_position)
    Position(next) -> walk(next(next_position), next_position + 1)
    LeaseId(next) -> walk(next("no-lease"), next_position)
    Impossible(reason) -> Error(reason)
  }
}

/// The one rule the Publish table sends this publication to over this
/// state, and the events that rule's effect emits there.
fn rule_and_events(
  state: State,
  publication: Publication,
) -> #(String, List(Event)) {
  let assert Ok(Some(row)) =
    rules.dispatch(publish.publish(), state, publication)
  let assert Ok(events) =
    walk(row.effect(state, publication), state.queue.next_position)
  #(row.name, events)
}

/// The rule the Publish table sends this publication to over this state.
fn rule(state: State, publication: Publication) -> Option(String) {
  let assert Ok(row) = rules.dispatch(publish.publish(), state, publication)
  option.map(row, fn(row) { row.name })
}

/// A publish into an existing pocket that adds or raises a record emits
/// `PocketTouched` for that pocket, whichever of the two it is.
/// `PublishIntoNewPocket` ensures none, and a stale publish does not touch
/// the pocket.
fn events() -> List(String) {
  assert rule_and_events(stored(), publication("new", "a", 1, "y"))
    == #("PublishIntoNewPocket", [])
  assert rule_and_events(stored(), publication("pk", "b", 1, "y"))
    == #("PublishNewKey", [PocketTouched("pk")])
  assert rule_and_events(stored(), publication("other", "a", 1, "y"))
    == #("PublishNewKey", [PocketTouched("other")])
  assert rule_and_events(stored(), publication("pk", "a", 6, "y"))
    == #("Supersede", [PocketTouched("pk")])
  assert rule_and_events(stored(), publication("other", "b", 6, "y"))
    == #("Supersede", [PocketTouched("other")])
  assert rule_and_events(stored(), publication("pk", "a", 4, "y"))
    == #("DropStalePublish", [])
  assert rule_and_events(stored(), publication("other", "b", 4, "y"))
    == #("DropStalePublish", [])
  assert rule_and_events(
      leased(True, Unsettled),
      publication("pk", "a", 7, "y"),
    )
    == #("Supersede", [PocketTouched("pk")])
  []
}

/// The stored rank is `record.rank` whatever the record's state. Above it,
/// `Supersede` takes a held record only when a version is already deferred
/// behind the held one; with nothing deferred the publish is
/// `DeferBehindHeldVersion`'s, whose row is not landed, so the only claim
/// made of it here is that `Supersede`'s guard does not hold. Below it the
/// publish is stale, held or not, and the held rank is below the stored rank
/// once a version is deferred. All of it holds whatever the record's mark:
/// `Supersede`'s guard names the status and the arrival position, and no
/// mark.
fn held_record_guards() -> List(String) {
  let assert Ok(supersede) =
    list.find(publish.publish().rows, fn(row) { row.name == "Supersede" })
  list.each(marks(), fn(mark) {
    assert rule(leased(True, mark), publication("pk", "a", 7, "y"))
      == Some("Supersede")
    assert !supersede.guard(leased(False, mark), publication("pk", "a", 6, "y"))
    assert rule(leased(False, mark), publication("pk", "a", 4, "y"))
      == Some("DropStalePublish")
    assert rule(leased(True, mark), publication("pk", "a", 5, "y"))
      == Some("DropStalePublish")
  })
  []
}

/// The loser is the deferred version, not the held one: it is shed at once,
/// with the time of the publish, and the record takes the new rank and
/// payload. The held version, its mark, a nacked record's nack time and the
/// arrival position stay as they were, and the counter does not move. Once
/// for each mark, since the rule reads none. Run through the apply step, so
/// the checker has read the state it leaves.
fn supersede_deferred() -> List(String) {
  list.flat_map(marks(), supersede_deferred_marked)
}

fn supersede_deferred_marked(mark: Mark) -> List(String) {
  let state = leased(True, mark)
  let now = harness.t0 + 9
  let step =
    core.apply(state, core.Publish(publication("pk", "a", 7, "newest")))
  let assert Ok(applied) = rules.drive(step, fn() { now }, fn() { "no-lease" })
  assert applied
    == Applied(
      State(
        ..state,
        records: [
          Record(..held_record(True, mark), rank: 7, payload: <<"newest":utf8>>),
        ],
        superseded: [
          SupersededRecord(
            pocket: "pk",
            key: "a",
            rank: 6,
            payload: <<"newer":utf8>>,
            reason: Superseded,
            shed_at: now,
            expires_at: now + config.defaults().superseded_ttl,
          ),
        ],
      ),
      ["Supersede"],
      None,
    )
  applied.fired
}

/// `Supersede`'s effect reads the record its guard saw. Where there is none,
/// which only a wrong guard could bring about, the outcome is a defect that
/// names the rule: a value, not a crash. The step is done without asking for
/// the time, so no clock is read.
fn supersede_without_record() -> List(String) {
  let assert Ok(supersede) =
    list.find(publish.publish().rows, fn(row) { row.name == "Supersede" })
  let unguarded =
    Table(
      "Publish",
      [Row("Supersede", fn(_state, _publication) { True }, supersede.effect)],
      NothingHappens,
    )
  assert core.run(unguarded, publication("pk", "a", 6, "y"), joined())
    == Done(
      Error(Defect(
        "Publish",
        EffectFailed("Supersede", "no record a in pocket pk"),
      )),
    )
  []
}
