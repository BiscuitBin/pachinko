//// The specification's Publish section.
////
//// First the catalog rows of `crates/libpachinko/tests/queue/publish.rs`
//// that need no more than a publish and a pull, S011 to S020 and S022, each
//// a port of its Rust test, driving the owner. S012, S015, S019, S020 and
//// S022 end with a pull, which shows what the publishes left to a consumer.
//// A port
//// asserts what its Rust test asserts, with one change: where the Rust
//// checks a publish for an outcome, the port asserts that the publish
//// answered nothing and names the rule it fired, and asserts the state that
//// publish left through the snapshot (the maintainer's ruling on N32 in
//// `double/SPEC-NOTES.md`). A setup publish the Rust unwraps is asserted by
//// its reply alone. An absolute position is `impl-choice`: it holds only
//// because the queue's first position is 0 (spec notes G04, G16 and G29). The
//// order of the superseded queue, where a port asserts it, is `impl-choice`
//// too (spec note G19).
////
//// Then the catalog rows of the same file that publish onto a held record,
//// S021, S082 and S083, ported the same way. Each pulls first, so that the
//// record is held, and asserts the state through the snapshot: a higher rank
//// with nothing deferred is `DeferBehindHeldVersion`'s, and a version
//// deferred behind the held one is shed by a still higher rank. A refusal
//// goes through `harness.refused`, which compares the complete snapshot, the
//// clock's read count and the lease-id draws before and after. S082 and
//// S083 assert absolute positions, `impl-choice` too (spec note G04, and
//// G48).
////
//// Then the double's own tests, which pin no catalog row: the refusal of a
//// publisher that never joined, what each of the five rules changes and how
//// often it reads the clock, the events each emits, the held branches of
//// `Supersede`'s and `DeferBehindHeldVersion`'s guards over states built by
//// hand, and what `ReviveIdlePocket` changes when it answers
//// `PocketTouched`.

import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import harness.{type Double, Driven}
import pachinko_double.{type Reply, Outcome}
import pachinko_double/config
import pachinko_double/core
import pachinko_double/domain.{
  type Event, type Plan, NoPocketAvailable, PocketTouched,
}
import pachinko_double/inspection.{Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type Mark, type Reason, type Record, type State, Acked, Consumer, DeadLetter,
  Lease, LeaseActive, Nacked, Pocket, PocketIdle, PocketLeased, PocketParked,
  PocketQueued, Publisher, Queue, Record, RecordHeld, RecordQueued, Stale, State,
  Superseded, SupersededRecord, Unsettled,
}
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
      "a publish of a new key appends to an existing pocket",
      [Scenario("S012")],
      new_key_appends,
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
      "repeated supersede keeps the first slot and sheds every loser",
      [Scenario("S015")],
      repeated_supersede,
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
      "large ranks keep full precision across supersede and stale",
      [Scenario("S019")],
      large_ranks,
    ),
    Test(
      "payload bytes and keys round-trip unchanged",
      [Scenario("S020")],
      bytes_and_keys_round_trip,
    ),
    Test(
      "publishing into a queued pocket does not move it in the pocket order",
      [Scenario("S022")],
      queued_pocket_keeps_its_place,
    ),
    Test(
      "publishing into a leased pocket neither requeues it nor escapes the lease, and sheds no held version",
      [Scenario("S021")],
      into_leased_pocket,
    ),
    Test(
      "a publish onto a held record is judged alike whether it is unsettled, acked or nacked: higher defers, a second higher sheds the deferred version, equal is rejected, lower is stale",
      [Scenario("S082")],
      judged_alike,
    ),
    Test(
      "publishing again at the held rank is rejected while nothing is deferred and stale once a newer version is",
      [Scenario("S083")],
      held_rank_again,
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
      "DeferBehindHeldVersion changes exactly what it ensures, reads no clock and draws no lease id",
      [],
      defer_exactly,
    ),
    Test(
      "PublishNewKey, Supersede and DeferBehindHeldVersion touch the pocket, and the other two emit nothing",
      [],
      events,
    ),
    Test(
      "Supersede takes a held record only with a version deferred behind it, and DeferBehindHeldVersion only with none",
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
    Test(
      "DeferBehindHeldVersion's effect without its held record is a defect, and reads no clock",
      [],
      defer_without_held_record,
    ),
    Test(
      "ReviveIdlePocket changes exactly what it ensures and reads no clock",
      [],
      revive_idle_exactly,
    ),
    Test(
      "a touched pocket that is queued or leased, and a key no pocket has, are left as they were",
      [],
      touched_and_not_idle,
    ),
    Test(
      "ReviveIdlePocket's guard does not hold for a parked pocket",
      [],
      revive_guard_not_parked,
    ),
    Test(
      "ReviveIdlePocket's effect without its pocket is a defect",
      [],
      revive_without_pocket,
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

fn new_key_appends() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 1, "x")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "b", 7, "y")
    == answers_nothing("PublishNewKey")
  let snapshot = harness.inspect(double)
  assert list.length(snapshot.pockets) == 1
  // `impl-choice` (spec notes G04 and G29): the pocket's absolute position,
  // the record's, and the counter. That the new key takes the next position
  // and the pocket keeps its own is the specification's.
  assert harness.pocket(double, "pk").position == 0
  let record = harness.record(double, "pk", "b")
  assert #(record.position, record.status) == #(2, RecordQueued)
  assert snapshot.next_position == 3
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord", "HoldRecord"]
  assert harness.keys(batch.records) == ["a", "b"]
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

/// The Rust checks each of the three supersedes for an outcome; the port
/// asserts that each answered nothing and fired `Supersede` (the ruling on
/// N32), and the state they leave is asserted below as the Rust asserts it.
fn repeated_supersede() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 1, "v1")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "b", 1, "b")
    == answers_nothing("PublishNewKey")
  list.each([#(2, "v2"), #(3, "v3"), #(4, "v4")], fn(version) {
    assert harness.publish(double, "pk", "a", version.0, version.1)
      == answers_nothing("Supersede")
  })
  // `impl-choice` (spec notes G04 and G29): the absolute position. That the
  // record is where its first publish put it is the specification's.
  assert harness.record(double, "pk", "a").position == 1
  assert rank_and_payload(double, "pk", "a") == #(4, <<"v4":utf8>>)
  // `impl-choice` (spec note G19): the order of the superseded queue.
  assert list.map(harness.inspect(double).superseded, fn(shed) {
      #(shed.rank, shed.payload)
    })
    == [#(1, <<"v1":utf8>>), #(2, <<"v2":utf8>>), #(3, <<"v3":utf8>>)]
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord", "HoldRecord"]
  assert harness.keys(batch.records) == ["a", "b"]
  let assert [first, ..] = batch.records
  assert first.rank == 4
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

/// The Rust's ranks are a 64-bit integer's extremes: `i64::MIN`, the next
/// above it, and `i64::MAX` with the one below it. A rank here is an `Int`,
/// which has no bound on this target, so these are not its extremes; they
/// order exactly all the same (spec note G08). The Rust checks the four
/// publishes after the first for an outcome, three supersedes and a stale
/// one; the port asserts that each answered nothing and the rule it fired
/// (the ruling on N32).
fn large_ranks() -> List(String) {
  let lowest = -9_223_372_036_854_775_808
  let highest = 9_223_372_036_854_775_807
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", lowest, "min")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "a", lowest + 1, "min+1")
    == answers_nothing("Supersede")
  assert harness.publish(double, "pk", "a", highest - 1, "max-1")
    == answers_nothing("Supersede")
  assert harness.publish(double, "pk", "a", highest, "max")
    == answers_nothing("Supersede")
  assert harness.publish(double, "pk", "a", highest - 1, "late")
    == answers_nothing("DropStalePublish")
  assert rank_and_payload(double, "pk", "a") == #(highest, <<"max":utf8>>)
  // `impl-choice` (spec note G19): the order of the superseded queue.
  assert list.map(harness.inspect(double).superseded, fn(shed) { shed.rank })
    == [lowest, lowest + 1, highest - 1, highest - 1]
  let batch = harness.batch(double, "c1", 1)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let assert [first, ..] = batch.records
  assert first.rank == highest
  harness.fired(double)
}

/// A payload of every byte value, 0 to 255 in order, which is not text, under
/// a pocket key and a record key that hold accented, CJK and astral
/// characters and a NUL. The batch names the pocket by the same key and hands
/// back the same record key and the same bytes.
fn bytes_and_keys_round_trip() -> List(String) {
  let double = harness.fixture()
  let every_byte =
    int.range(from: 0, to: 256, with: <<>>, run: fn(bytes, byte) {
      <<bytes:bits, byte>>
    })
  let pocket = "pocket-\u{e9}\u{4e2d}\u{1f600}"
  let key = "key-\u{3b1}\u{3b2}\u{0}"
  assert harness.publish_publication(
      double,
      Publication(
        publisher: "p",
        pocket_key: pocket,
        record_key: key,
        rank: 1,
        payload: every_byte,
      ),
    )
    == answers_nothing("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 1)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  assert batch.pocket_key == pocket
  let assert [first, ..] = batch.records
  assert first.key == key
  assert first.payload == every_byte
  harness.fired(double)
}

/// The Rust also calls `invariants::order_is_the_queued_pockets`, which
/// compares its stored pocket order with the queued pockets. The double
/// stores no pocket order: `harness.order` is the queued pockets by position,
/// so that check has nothing of its own to compare here.
fn queued_pocket_keeps_its_place() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "first", "a", 1, "a")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "second", "a", 1, "a")
    == answers_nothing("PublishIntoNewPocket")
  assert harness.publish(double, "first", "b", 1, "b")
    == answers_nothing("PublishNewKey")
  assert harness.publish(double, "first", "a", 2, "a2")
    == answers_nothing("Supersede")
  assert harness.publish(double, "first", "a", 0, "a0")
    == answers_nothing("DropStalePublish")
  // `impl-choice` (spec notes G04 and G29): the absolute position. That the
  // pocket keeps the position it had is the specification's.
  assert harness.pocket(double, "first").position == 0
  assert harness.pocket(double, "first").status == PocketQueued
  assert harness.order(double) == ["first", "second"]
  let batch = harness.batch(double, "c1", 0)
  assert batch.fired == ["PullClaimsFrontPocket"]
  assert batch.pocket_key == "first"
  harness.fired(double)
}

// The catalog rows that publish onto a held record ----------

/// A superseded record of `k` in `pk`, shed at `at`, as the Rust's shed
/// rows give one.
fn shed_entry(
  rank: Int,
  payload: String,
  reason: Reason,
  at: Int,
) -> model.SupersededRecord {
  SupersededRecord(
    pocket: "pk",
    key: "k",
    rank:,
    payload: <<payload:utf8>>,
    reason:,
    shed_at: at,
    expires_at: at + superseded_ttl(),
  )
}

/// The Rust checks the three publishes for an outcome: a new key,
/// superseded and stale. The port asserts that each answered nothing and
/// names the rule it fired (the ruling on N32): the higher rank of the held
/// `a` is deferred behind it, and the lower one is stale against the
/// deferred version. For the deferral the port also asserts the state it
/// leaves, against the record and the counter read just before it, so no
/// absolute position is pinned. Each `PocketTouched` finds the pocket
/// leased, so no revive rule fires.
fn into_leased_pocket() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "a", 1, "a")
    == answers_nothing("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let pocket_before = harness.pocket(double, "pk")
  let lease_before = harness.inspect(double).leases
  assert harness.publish(double, "pk", "b", 1, "b")
    == answers_nothing("PublishNewKey")
  let a_before = harness.record(double, "pk", "a")
  let counter = harness.inspect(double).next_position
  assert harness.publish(double, "pk", "a", 2, "a2")
    == answers_nothing("DeferBehindHeldVersion")
  // The state the deferral leaves (the ruling on N32): `a` takes the rank
  // and payload, with the counter's value as its arrival position; the held
  // version, the mark, the nack time and the position stay; the counter
  // advances by one.
  assert harness.record(double, "pk", "a")
    == Record(
      ..a_before,
      rank: 2,
      payload: <<"a2":utf8>>,
      status: RecordHeld(
        held_rank: 1,
        held_payload: <<"a":utf8>>,
        mark: Unsettled,
        nacked_at: None,
        deferred_position: Some(counter),
      ),
    )
  assert harness.inspect(double).next_position == counter + 1
  assert harness.publish(double, "pk", "a", 0, "a0")
    == answers_nothing("DropStalePublish")
  // "The held a1 is not shed; a2 waits behind it."
  assert harness.inspect(double).superseded
    == [
      SupersededRecord(
        pocket: "pk",
        key: "a",
        rank: 0,
        payload: <<"a0":utf8>>,
        reason: Stale,
        shed_at: harness.t0,
        expires_at: harness.t0 + superseded_ttl(),
      ),
    ]
  let pocket = harness.pocket(double, "pk")
  assert pocket.status == PocketLeased
  assert pocket.position == pocket_before.position
  assert harness.order(double) == []
  assert harness.inspect(double).leases == lease_before
  assert harness.pull(double, "c2", 10)
    == Ok(Outcome(["PullFindsNoPocket"], Some(NoPocketAvailable("c2"))))
  let again = harness.batch(double, "c1", 10)
  assert again.fired == ["PullWithinLease", "HoldRecord"]
  assert again.lease == batch.lease
  assert harness.keys(again.records) == ["b"]
  harness.fired(double)
}

/// The Rust test file's `held_and_marked`: `pk` with `k` at rank 1, payload
/// `v1`, held by `c1` from `t0` and marked at 10 ms as `mark` says; an
/// unsettled record is not marked. `k` stands at 1, so the next position is
/// 2. It answers the time of the nack, if `mark` is a nack.
fn held_and_marked(mark: Mark) -> #(Double, Option(Int)) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == answers_nothing("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let marked_at = harness.at(double, 10)
  case mark {
    Unsettled -> #(double, None)
    Acked -> {
      assert harness.ack(double, "c1", batch.lease, "k")
        == answers_nothing("AckMarksRecord")
      #(double, None)
    }
    Nacked -> {
      assert harness.nack(double, "c1", batch.lease, "k")
        == answers_nothing("NackMarksRecord")
      #(double, Some(marked_at))
    }
  }
}

/// The status of `k` held at rank 1, payload `v1`, with this mark, this
/// nack time and this arrival position: the Rust fixture's `held_as`.
fn held_as(
  mark: Mark,
  nacked_at: Option(Int),
  arrival: Option(Int),
) -> model.RecordStatus {
  RecordHeld(
    held_rank: 1,
    held_payload: <<"v1":utf8>>,
    mark:,
    nacked_at:,
    deferred_position: arrival,
  )
}

/// The Rust loops over the three marks; so does the port, with a fixture
/// each. Where the Rust checks a publish for an outcome, the port asserts
/// that it answered nothing and names its rule (the ruling on N32).
fn judged_alike() -> List(String) {
  list.flat_map(marks(), judged_alike_marked)
}

/// The Rust's `the_publish_table_against_a_held_record` and
/// `equal_then_lower` for one mark. `impl-choice` (spec note G04, and
/// G48): the arrival position 2, the counter at 3 and the position 1 are
/// absolute; that the arrival position is the counter's value, that the
/// counter advances by one and that the record keeps its position are the
/// specification's. `impl-choice` (spec note G19): the order of the
/// superseded queue, `v3` then `v4`.
fn judged_alike_marked(mark: Mark) -> List(String) {
  let #(double, nacked_at) = held_and_marked(mark)
  let deferred = held_as(mark, nacked_at, Some(2))
  let _ = harness.at(double, 20)
  // Higher: "deferred behind the held version".
  assert harness.publish(double, "pk", "k", 3, "v3")
    == answers_nothing("DeferBehindHeldVersion")
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.rank) == #(deferred, 3)
  let snapshot = harness.inspect(double)
  // "Nothing is shed."
  assert snapshot.superseded == []
  assert snapshot.next_position == 3
  let thirty = harness.at(double, 30)
  // A second higher: "v3 at once".
  assert harness.publish(double, "pk", "k", 5, "v5")
    == answers_nothing("Supersede")
  let v3 = shed_entry(3, "v3", Superseded, thirty)
  assert harness.inspect(double).superseded == [v3]
  // Equal: refused, and "equal changes nothing".
  harness.refused(
    double,
    EqualRankRefused(pocket_key: "pk", record_key: "k", rank: 5),
    fn() { harness.publish(double, "pk", "k", 5, "again") },
  )
  // Lower: stale.
  assert harness.publish(double, "pk", "k", 4, "v4")
    == answers_nothing("DropStalePublish")
  let snapshot = harness.inspect(double)
  assert snapshot.superseded == [v3, shed_entry(4, "v4", Stale, thirty)]
  // "No dead letter while held."
  assert snapshot.dead_letters == []
  let record = harness.record(double, "pk", "k")
  // "v5 keeps v3's arrival position."
  assert #(record.status, record.rank, record.position) == #(deferred, 5, 1)
  // The Rust's `invariants::all`, by the checker.
  assert invariants.check(snapshot) == []
  harness.fired(double)
}

/// The Rust's equal-rank error carries the record key and the rank; the
/// double's refusal names the pocket as well (spec note G13). `impl-choice`
/// (spec note G04, and G48): the arrival position 2 is absolute; that it
/// is the counter's value is the specification's.
fn held_rank_again() -> List(String) {
  let #(double, _) = held_and_marked(Unsettled)
  // "Nothing deferred: the held rank is the stored rank."
  harness.refused(
    double,
    EqualRankRefused(pocket_key: "pk", record_key: "k", rank: 1),
    fn() { harness.publish(double, "pk", "k", 1, "again") },
  )
  assert harness.publish(double, "pk", "k", 3, "v3")
    == answers_nothing("DeferBehindHeldVersion")
  let deferred = held_as(Unsettled, None, Some(2))
  assert harness.record(double, "pk", "k").status == deferred
  // "v3 deferred: the held rank is below the stored rank."
  assert harness.publish(double, "pk", "k", 1, "again")
    == answers_nothing("DropStalePublish")
  assert harness.inspect(double).superseded
    == [shed_entry(1, "again", Stale, harness.t0 + 10)]
  assert harness.record(double, "pk", "k").status == deferred
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
// Through the owner a held record's pocket is leased, so the `PocketTouched`
// a publish onto it emits meets no revive rule and shows nothing; a pocket
// that goes idle has seen no expiry; and no two pockets of a scenario hold
// one key while one of them is leased. These tests read the Publish table
// and the `PocketTouched` table directly, or run the apply step over a
// state built by hand.

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

/// Publisher `p` and consumers `c1` and `c2` joined, and no list in
/// position order. `other` is queued, and holds an `a` of its own at rank 5,
/// listed first, so that a lookup by key alone finds it. `pk` is leased to
/// `c1` and holds `held_record(False, mark)`, `a` at 1 handed out at rank 5,
/// and `b`, queued at 4. The superseded queue holds an earlier version of
/// `pk`'s `a`, and the dead-letter queue a record `c2` once nacked out of
/// `pk`. The counter stands at 6.
fn deferring(mark: Mark) -> State {
  State(
    ..joined(),
    queue: Queue(next_position: 6),
    consumers: [Consumer("c1"), Consumer("c2")],
    pockets: [
      Pocket("other", PocketQueued, 2, 0),
      Pocket("pk", PocketLeased, 0, 0),
    ],
    records: [
      Record("other", "a", 5, <<"theirs":utf8>>, 3, RecordQueued),
      held_record(False, mark),
      Record("pk", "b", 4, <<"b":utf8>>, 4, RecordQueued),
    ],
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
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "a",
        rank: 3,
        payload: <<"older":utf8>>,
        reason: Superseded,
        shed_at: harness.t0 - 50,
        expires_at: harness.t0 - 50 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "pk",
        key: "k0",
        rank: 2,
        payload: <<"zero":utf8>>,
        nacked_by: "c2",
        nacked_at: harness.t0 - 90,
        expires_at: harness.t0 - 80 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// A publish of `pk`'s held `a` at rank 9, above the stored rank 5, with
/// nothing deferred. The record takes rank 9 and the new payload, and its
/// `deferred_position` is the counter's value, 6; the counter advances to
/// 7. The held version, the mark, a nacked record's nack time and the
/// record's position stay. Nothing else in the complete state changes:
/// `other`'s `a`, `pk`'s `b`, the pockets, the lease and both shed queues.
/// The `PocketTouched` it emits finds `pk` leased, so nothing more fires.
/// The step reads no clock and draws no lease id. Once for each mark, since
/// the rule reads none; run through the apply step, so the checker has read
/// the state it leaves.
fn defer_exactly() -> List(String) {
  list.flat_map(marks(), fn(mark) {
    let state = deferring(mark)
    let assert [theirs, held, b] = state.records
    let assert Record(status: RecordHeld(..) as status, ..) = held
    let step =
      core.apply(state, core.Publish(publication("pk", "a", 9, "newest")))
    assert harness.drive(step, harness.t0 + 40, "unused")
      == Driven(
        Ok(Applied(
          State(..state, queue: Queue(next_position: 7), records: [
            theirs,
            Record(
              ..held,
              rank: 9,
              payload: <<"newest":utf8>>,
              status: RecordHeld(..status, deferred_position: Some(6)),
            ),
            b,
          ]),
          ["DeferBehindHeldVersion"],
          None,
        )),
        clock_reads: 0,
        lease_id_draws: 0,
      )
    ["DeferBehindHeldVersion"]
  })
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
/// `PocketTouched` for that pocket, whichever of the three it is: a new key,
/// a supersede, or a deferral behind a held version.
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
  assert rule_and_events(
      leased(False, Unsettled),
      publication("pk", "a", 6, "y"),
    )
    == #("DeferBehindHeldVersion", [PocketTouched("pk")])
  assert rule_and_events(deferring(Acked), publication("pk", "a", 9, "y"))
    == #("DeferBehindHeldVersion", [PocketTouched("pk")])
  []
}

/// The stored rank is `record.rank` whatever the record's state. Above it,
/// `Supersede` takes a held record only when a version is already deferred
/// behind the held one, and `DeferBehindHeldVersion` only when none is, and
/// never a queued record. At it no rule holds, held or not: the held rank
/// while nothing is deferred, and the deferred version's rank once one is.
/// Below it the publish is stale, held or not, and the held rank is below
/// the stored rank once a version is deferred. All of it holds whatever the
/// record's mark: the two guards name the status and the arrival position,
/// and no mark. Dispatch evaluates every guard, so each `Some` here also
/// says that no other guard held.
fn held_record_guards() -> List(String) {
  list.each(marks(), fn(mark) {
    assert rule(leased(True, mark), publication("pk", "a", 7, "y"))
      == Some("Supersede")
    assert rule(leased(False, mark), publication("pk", "a", 6, "y"))
      == Some("DeferBehindHeldVersion")
    assert rule(leased(False, mark), publication("pk", "a", 5, "y")) == None
    assert rule(leased(True, mark), publication("pk", "a", 6, "y")) == None
    assert rule(leased(False, mark), publication("pk", "a", 4, "y"))
      == Some("DropStalePublish")
    assert rule(leased(True, mark), publication("pk", "a", 5, "y"))
      == Some("DropStalePublish")
  })
  assert rule(stored(), publication("pk", "a", 6, "y")) == Some("Supersede")
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

/// `DeferBehindHeldVersion`'s effect reads the held record its guard saw.
/// Where there is no record, or the record is queued, which only a wrong
/// guard could bring about, the outcome is a defect that names the rule.
/// The step is done without asking for the time.
fn defer_without_held_record() -> List(String) {
  let assert Ok(defer) =
    list.find(publish.publish().rows, fn(row) {
      row.name == "DeferBehindHeldVersion"
    })
  let unguarded =
    Table(
      "Publish",
      [
        Row(
          "DeferBehindHeldVersion",
          fn(_state, _publication) { True },
          defer.effect,
        ),
      ],
      NothingHappens,
    )
  assert core.run(unguarded, publication("pk", "a", 6, "y"), joined())
    == Done(
      Error(Defect(
        "Publish",
        EffectFailed("DeferBehindHeldVersion", "no record a in pocket pk"),
      )),
    )
  assert core.run(unguarded, publication("pk", "a", 6, "y"), stored())
    == Done(
      Error(Defect(
        "Publish",
        EffectFailed(
          "DeferBehindHeldVersion",
          "record a in pocket pk is not held",
        ),
      )),
    )
  []
}

/// Publisher `p` and consumer `c1` joined, and the counter stands at 9.
/// `idle` is idle: it holds no record, kept the position it had when it
/// was claimed, and has seen two expiries. `pk` and `other` are queued,
/// with a record each. The superseded queue holds an earlier version of
/// `pk`'s record, and the dead-letter queue a record `c1` once nacked out
/// of `idle`.
fn with_idle() -> State {
  State(
    ..joined(),
    queue: Queue(next_position: 9),
    pockets: [
      Pocket("pk", PocketQueued, 6, 0),
      Pocket("idle", PocketIdle, 0, 2),
      Pocket("other", PocketQueued, 2, 0),
    ],
    records: [
      Record("pk", "a", 5, <<"x":utf8>>, 7, RecordQueued),
      Record("other", "b", 5, <<"x":utf8>>, 3, RecordQueued),
    ],
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "a",
        rank: 4,
        payload: <<"older":utf8>>,
        reason: Superseded,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "idle",
        key: "gone",
        rank: 3,
        payload: <<"dead":utf8>>,
        nacked_by: "c1",
        nacked_at: 350,
        expires_at: 400 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// `PocketTouched` for the idle pocket, as its own outcome: the pocket is
/// queued at the counter's value, 9, with its count 0, and the counter is
/// 10. Nothing else in the complete state changes, and the step is done at
/// once, so the rule asked for no time.
///
/// Then a publish of a new key into the idle pocket, through the apply
/// step. `PublishNewKey` allots the record's position first, 9, and the
/// revival the pocket's after it, 10, so the pocket goes to the back of the
/// pocket order, behind `pk` at 6, and the counter is 11. The outcome reads
/// no clock and draws no lease id.
fn revive_idle_exactly() -> List(String) {
  let state = with_idle()
  let assert [pk, idle, other] = state.pockets
  let revive = "ReviveIdlePocket"
  assert core.run(publish.pocket_touched(), "idle", state)
    == Done(
      Ok(Applied(
        State(..state, queue: Queue(next_position: 10), pockets: [
          pk,
          Pocket(
            ..idle,
            status: PocketQueued,
            position: 9,
            consecutive_expiries: 0,
          ),
          other,
        ]),
        [revive],
        None,
      )),
    )
  let fired = ["PublishNewKey", revive]
  assert harness.drive(
      core.apply(state, core.Publish(publication("idle", "k", 1, "v"))),
      harness.t0,
      "unused",
    )
    == Driven(
      Ok(Applied(
        State(
          ..state,
          queue: Queue(next_position: 11),
          pockets: [
            pk,
            Pocket(
              ..idle,
              status: PocketQueued,
              position: 10,
              consecutive_expiries: 0,
            ),
            other,
          ],
          records: list.append(state.records, [
            Record("idle", "k", 1, <<"v":utf8>>, 9, RecordQueued),
          ]),
        ),
        fired,
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  [revive, ..fired]
}

/// `PocketTouched` is answered only for an idle or a parked pocket. For a
/// queued pocket, a leased one, and a key no pocket has, nothing happens:
/// no rule fires and the complete state is as it was, the pocket's position
/// included. A parked pocket's revival is `ReviveParkedPocket`'s, pinned in
/// `expiry_test`.
fn touched_and_not_idle() -> List(String) {
  let state = with_idle()
  let untouched = fn(state, pocket_key) {
    assert core.run(publish.pocket_touched(), pocket_key, state)
      == Done(Ok(Applied(state, [], None)))
  }
  untouched(state, "pk")
  untouched(state, "other")
  untouched(state, "nowhere")
  untouched(leased(False, Unsettled), "pk")
  []
}

/// `requires: pocket.status = idle`. The idle pocket of `with_idle` holds
/// the guard, and the same pocket parked does not. Only the row's guard is
/// asserted: what `PocketTouched` does for a parked pocket is
/// `ReviveParkedPocket`'s, pinned in `expiry_test`.
fn revive_guard_not_parked() -> List(String) {
  let assert Ok(revive) =
    list.find(publish.pocket_touched().rows, fn(row) {
      row.name == "ReviveIdlePocket"
    })
  let state = with_idle()
  let parked =
    State(
      ..state,
      pockets: list.map(state.pockets, fn(pocket) {
        case pocket.key == "idle" {
          True -> Pocket(..pocket, status: PocketParked)
          False -> pocket
        }
      }),
    )
  assert revive.guard(state, "idle")
  assert !revive.guard(parked, "idle")
  []
}

/// `ReviveIdlePocket`'s effect reads the pocket its guard saw. Where there
/// is none, which only a wrong guard could bring about, the outcome is a
/// defect that names the rule: a value, not a crash.
fn revive_without_pocket() -> List(String) {
  let assert Ok(revive) =
    list.find(publish.pocket_touched().rows, fn(row) {
      row.name == "ReviveIdlePocket"
    })
  let unguarded =
    Table(
      "PocketTouched",
      [Row(revive.name, fn(_state, _pocket_key) { True }, revive.effect)],
      NothingHappens,
    )
  assert core.run(unguarded, "nowhere", with_idle())
    == Done(
      Error(Defect(
        "PocketTouched",
        EffectFailed("ReviveIdlePocket", "no pocket nowhere"),
      )),
    )
  []
}
