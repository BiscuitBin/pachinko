//// The specification's Pull section.
////
//// First the ten catalog rows of `crates/libpachinko/tests/queue/pull.rs`
//// this ticket owns, S027 to S033, S036, S038 and S039, each a port of its
//// Rust test, driving the owner. A port asserts what its Rust test asserts,
//// with these changes. A batch is the owner's answer, read through
//// `harness.batch`, and each pull also asserts the names of the rules it
//// fired, in order. The Rust's batch carries its lease's `expires_at` and
//// the specification's does not, so a port reads the expiry from the
//// snapshot's lease (N13 in `double/SPEC-NOTES.md`). The Rust's
//// `lease_view` asks its queue; the port asks the owner, through
//// `harness.lease_view`.
////
//// S037, a row of the same file, publishes onto a held record between its
//// pulls. Its port asserts that the publish answered nothing and names the
//// rule it fired (the ruling on N32).
////
//// Then the double's own tests, which pin no catalog row: the refusal of a
//// consumer that never joined, what each of the four rules changes, how
//// often it reads the clock and whether it draws a lease id, the orders a
//// pull reads where a list's order and the positions disagree, and the two
//// tables' answers when no guard holds.

import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import harness.{type Double, Pulled}
import pachinko_double.{type Reply, Outcome}
import pachinko_double/answers.{Batch, BatchRecord, NoPocketAvailable}
import pachinko_double/config.{Settings}
import pachinko_double/failure.{Defect, NoRuleHeld, Refused}
import pachinko_double/inspection.{Snapshot}
import pachinko_double/internal/core
import pachinko_double/internal/domain.{type Applied, type Failure}
import pachinko_double/internal/pull.{type Pull, Handover, Pull}
import pachinko_double/internal/rules.{Applied, Done}
import pachinko_double/model.{
  type Record, type State, Consumer, DeadLetter, Lease, LeaseActive,
  LeaseExpired, LeaseReleased, Pocket, PocketIdle, PocketLeased, PocketParked,
  PocketQueued, Publisher, Queue, Record, RecordHeld, RecordQueued, State,
  Superseded, SupersededRecord, Unsettled,
}
import pachinko_double/refusals.{PullRefused, UnknownConsumer}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a pull claims the front pocket, leases it and holds the batch",
      [Scenario("S027")],
      claims_front_pocket,
    ),
    Test(
      "a positive limit returns at most that many, oldest first, and later pulls continue",
      [Scenario("S028")],
      positive_limit,
    ),
    Test(
      "a limit above max_pull_batch is capped",
      [Scenario("S029")],
      limit_above_cap,
    ),
    Test(
      "a limit above the queued count returns every queued record",
      [Scenario("S030")],
      limit_above_count,
    ),
    Test(
      "a zero-limit pull claims a pocket and returns an empty batch",
      [Scenario("S031")],
      zero_limit,
    ),
    Test("a negative limit behaves like zero", [Scenario("S032")], negative),
    Test(
      "a pull with no queued pocket returns NoPocketAvailable, not an error",
      [Scenario("S033")],
      no_queued_pocket,
    ),
    Test(
      "a pull under a lease extends the lease",
      [Scenario("S036")],
      extends_lease,
    ),
    Test(
      "a held record is not handed out again under the same lease",
      [Scenario("S037")],
      not_handed_again,
    ),
    Test(
      "pockets are claimed in the order they entered",
      [Scenario("S038")],
      claimed_in_order,
    ),
    Test(
      "a second consumer never receives records from a leased pocket",
      [Scenario("S039")],
      leased_pocket_is_exclusive,
    ),
    Test(
      "a pull by a consumer that never joined is refused",
      [],
      unknown_consumer,
    ),
    Test(
      "PullClaimsFrontPocket changes exactly what it ensures, reads the clock once and draws one lease id",
      [],
      claims_exactly,
    ),
    Test(
      "PullWithinLease changes exactly what it ensures, on an empty batch too, reads the clock once and draws no lease id",
      [],
      within_lease_exactly,
    ),
    Test(
      "PullFindsNoPocket changes nothing, reads no clock and draws no lease id",
      [],
      finds_no_pocket_exactly,
    ),
    Test(
      "HoldRecord changes exactly what it ensures, and a Handed no rule answers is a defect",
      [],
      hold_record_exactly,
    ),
    Test(
      "a claim takes the queued pocket with the smallest position, and a batch the queued records with the smallest",
      [],
      orders_by_position,
    ),
    Test(
      "a consumer whose lease has ended pulls as one with no lease",
      [],
      ended_lease,
    ),
    Test(
      "no Pull rule holds for an active lease on a pocket that is not leased",
      [],
      lease_on_unleased_pocket,
    ),
  ]
}

const claims = "PullClaimsFrontPocket"

const within = "PullWithinLease"

const finds_none = "PullFindsNoPocket"

const hold = "HoldRecord"

/// The Rust fixture's `LEASE_TTL`.
fn lease_ttl() -> Int {
  harness.fixture_settings().lease_ttl
}

/// What a publish that fired this rule replies: a publish answers nothing.
fn published(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

/// Publishes `count` records `r0`, `r1`, ... into pocket `pk`, each at rank
/// 1 with its key as its payload.
fn publish_records(double: Double, count: Int) -> Nil {
  int.range(from: 0, to: count, with: Nil, run: fn(_, index) {
    let key = "r" <> int.to_string(index)
    let rule = case index {
      0 -> "PublishIntoNewPocket"
      _ -> "PublishNewKey"
    }
    assert harness.publish(double, "pk", key, 1, key) == published(rule)
    Nil
  })
}

/// The Rust test file's `pocket_of`: a fixture with `count` records `r0`,
/// `r1`, ... published into pocket `pk`.
fn pocket_of(count: Int) -> Double {
  let double = harness.fixture()
  publish_records(double, count)
  double
}

/// The record as `HoldRecord` leaves it when nothing was deferred behind
/// it: held at its own rank and payload, unsettled.
fn held(record: Record) -> Record {
  Record(
    ..record,
    status: RecordHeld(
      held_rank: record.rank,
      held_payload: record.payload,
      mark: Unsettled,
      nacked_at: None,
      deferred_position: None,
    ),
  )
}

// The catalog rows -----------------------------------------

fn claims_front_pocket() -> List(String) {
  let double = pocket_of(2)
  let now = harness.advance(double.clock, harness.t0, 5)
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  // `impl-choice` (spec note G10): the specification declares no lease id,
  // and this one is the fake source's first, as the Rust's `1` is its
  // counting source's.
  assert batch.lease == "lease-1"
  assert batch.pocket_key == "pk"
  assert harness.keys(batch.records) == ["r0", "r1"]
  let assert [first, ..] = batch.records
  assert first.payload == <<"r0":utf8>>
  assert harness.pocket(double, "pk").status == PocketLeased
  assert harness.order(double) == []
  assert harness.record(double, "pk", "r0").status
    == RecordHeld(
      held_rank: 1,
      held_payload: <<"r0":utf8>>,
      mark: Unsettled,
      nacked_at: None,
      deferred_position: None,
    )
  let snapshot = harness.inspect(double)
  let assert [lease] = snapshot.leases
  assert #(lease.id, lease.consumer, lease.pocket) == #(batch.lease, "c1", "pk")
  // The Rust asserts the batch's `expires_at` too; the lease's is the same
  // fact (N13).
  assert #(lease.status, lease.granted_at, lease.expires_at)
    == #(LeaseActive, now, now + lease_ttl())
  // The Rust's snapshot pairs each consumer with its lease; here that is the
  // consumer's `active_leases`.
  let assert Ok(consumer) =
    list.find(inspection.views(snapshot).consumers, fn(view) {
      view.consumer == Consumer("c1")
    })
  assert list.map(consumer.active_leases, fn(lease) { lease.id })
    == [batch.lease]
  harness.fired(double)
}

fn positive_limit() -> List(String) {
  let double = pocket_of(5)
  let first = harness.batch(double, "c1", 2)
  assert first.fired == [claims, hold, hold]
  assert harness.keys(first.records) == ["r0", "r1"]
  let second = harness.batch(double, "c1", 2)
  assert second.fired == [within, hold, hold]
  assert harness.keys(second.records) == ["r2", "r3"]
  let third = harness.batch(double, "c1", 2)
  assert third.fired == [within, hold]
  assert harness.keys(third.records) == ["r4"]
  harness.fired(double)
}

fn limit_above_cap() -> List(String) {
  let double =
    harness.fixture_with(
      Settings(..harness.fixture_settings(), max_pull_batch: 3),
    )
  publish_records(double, 5)
  let batch = harness.batch(double, "c1", 1000)
  assert batch.fired == [claims, hold, hold, hold]
  assert harness.keys(batch.records) == ["r0", "r1", "r2"]
  harness.fired(double)
}

fn limit_above_count() -> List(String) {
  let double = pocket_of(3)
  let first = harness.batch(double, "c1", 50)
  assert first.fired == [claims, hold, hold, hold]
  assert harness.keys(first.records) == ["r0", "r1", "r2"]
  let second = harness.batch(double, "c1", 50)
  assert second.fired == [within]
  assert second.records == []
  harness.fired(double)
}

fn zero_limit() -> List(String) {
  let double = pocket_of(2)
  let batch = harness.batch(double, "c1", 0)
  assert batch.fired == [claims]
  assert batch.records == []
  assert harness.pocket(double, "pk").status == PocketLeased
  assert harness.record(double, "pk", "r0").status == RecordQueued
  assert list.length(harness.inspect(double).leases) == 1
  harness.fired(double)
}

/// The Rust's second pull has the limit `i64::MIN`.
fn negative() -> List(String) {
  let double = pocket_of(2)
  let batch = harness.batch(double, "c1", -1)
  assert batch.fired == [claims]
  assert batch.records == []
  assert harness.pocket(double, "pk").status == PocketLeased
  let lowest = harness.batch(double, "c1", -9_223_372_036_854_775_808)
  assert lowest.fired == [within]
  assert lowest.records == []
  let rest = harness.batch(double, "c1", 10)
  assert rest.fired == [within, hold, hold]
  assert harness.keys(rest.records) == ["r0", "r1"]
  harness.fired(double)
}

/// "Not an error": the pull succeeds, and `NoPocketAvailable` is its answer.
fn no_queued_pocket() -> List(String) {
  let double = harness.fixture()
  assert harness.pull(double, "c1", 10)
    == Ok(Outcome([finds_none], Some(NoPocketAvailable("c1"))))
  assert harness.inspect(double).leases == []
  assert harness.publish(double, "pk", "a", 1, "a")
    == published("PublishIntoNewPocket")
  assert harness.batch(double, "c1", 10).fired == [claims, hold]
  assert harness.pull(double, "c2", 10)
    == Ok(Outcome([finds_none], Some(NoPocketAvailable("c2"))))
  assert harness.lease_view(double, "c2") == Ok(None)
  harness.fired(double)
}

fn extends_lease() -> List(String) {
  let double = pocket_of(3)
  assert harness.lease_view(double, "c1") == Ok(None)
  let first = harness.batch(double, "c1", 1)
  assert first.fired == [claims, hold]
  let assert Ok(Some(before)) = harness.lease_view(double, "c1")
  assert before.expires_at == harness.t0 + lease_ttl()
  assert before.pocket_key == "pk"
  assert harness.marked_keys(before.records) == ["r0"]
  let now = harness.advance(double.clock, harness.t0, 400)
  let second = harness.batch(double, "c1", 1)
  assert second.fired == [within, hold]
  let assert Ok(Some(after)) = harness.lease_view(double, "c1")
  let extended = now + lease_ttl()
  // The Rust asserts the second batch's `expires_at` too; the lease's is the
  // same fact (N13).
  assert after.expires_at == extended
  assert second.lease == first.lease
  // `impl-choice` (spec note G31, as N23): the order. The surface exposes
  // the held records as a set, and the lease view lists them smallest
  // position first.
  assert harness.marked_keys(after.records) == ["r0", "r1"]
  let assert [lease, ..] = harness.inspect(double).leases
  assert #(lease.granted_at, lease.expires_at) == #(harness.t0, extended)
  harness.fired(double)
}

/// The Rust's publish of `r0` while it is held is unwrapped; it answers
/// nothing, and is deferred behind the held version (the ruling on N32).
/// `impl-choice` (spec note G31, as N23): the lease view's first record is
/// `r0` because the view lists the held records smallest position first;
/// the surface exposes them as a set.
fn not_handed_again() -> List(String) {
  let double = pocket_of(2)
  let first = harness.batch(double, "c1", 1)
  assert first.fired == [claims, hold]
  assert harness.publish(double, "pk", "r0", 2, "r0-v2")
    == published("DeferBehindHeldVersion")
  let second = harness.batch(double, "c1", 10)
  assert second.fired == [within, hold]
  assert second.lease == first.lease
  assert harness.keys(second.records) == ["r1"]
  let third = harness.batch(double, "c1", 10)
  assert third.fired == [within]
  assert third.records == []
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  let assert [first_held, ..] = view.records
  assert first_held.held_rank == 1
  harness.fired(double)
}

fn claimed_in_order() -> List(String) {
  let double = harness.fixture()
  list.each(["b", "c", "a"], fn(pocket) {
    assert harness.publish(double, pocket, "k", 1, "v")
      == published("PublishIntoNewPocket")
  })
  let first = harness.batch(double, "c1", 1)
  assert #(first.fired, first.pocket_key) == #([claims, hold], "b")
  let second = harness.batch(double, "c2", 1)
  assert #(second.fired, second.pocket_key) == #([claims, hold], "c")
  let third = harness.batch(double, "c3", 1)
  assert #(third.fired, third.pocket_key) == #([claims, hold], "a")
  harness.fired(double)
}

fn leased_pocket_is_exclusive() -> List(String) {
  let double = pocket_of(3)
  assert harness.publish(double, "other", "o", 1, "o")
    == published("PublishIntoNewPocket")
  assert harness.batch(double, "c1", 1).fired == [claims, hold]
  let second = harness.batch(double, "c2", 10)
  assert second.fired == [claims, hold]
  assert second.pocket_key == "other"
  assert harness.keys(second.records) == ["o"]
  assert harness.pull(double, "c3", 10)
    == Ok(Outcome([finds_none], Some(NoPocketAvailable("c3"))))
  let again = harness.batch(double, "c2", 10)
  assert again.fired == [within]
  assert harness.keys(again.records) == []
  let rest = harness.batch(double, "c1", 10)
  assert rest.fired == [within, hold, hold]
  assert harness.keys(rest.records) == ["r1", "r2"]
  harness.fired(double)
}

// The refusal of a consumer that never joined ----------------

/// `impl-choice` (spec note G26, as N18): the specification states no
/// such refusal. `cx` never joined, and `p` joined as a publisher only.
/// Over an empty queue the pull would be `PullFindsNoPocket`'s had `c1`
/// sent it, and with a pocket queued `PullClaimsFrontPocket`'s, which reads
/// the clock and draws a lease id. Each is refused, the complete snapshot is
/// unchanged, no clock was read and no lease id was drawn.
fn unknown_consumer() -> List(String) {
  let double = harness.fixture()
  let refused = fn(consumer) { Error(Refused(UnknownConsumer(consumer))) }
  let empty = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  assert harness.pull(double, "cx", 10) == refused("cx")
  assert harness.inspect(double) == empty
  publish_records(double, 2)
  let before = harness.inspect(double)
  assert harness.pull(double, "cx", 10) == refused("cx")
  assert harness.pull(double, "cx", 0) == refused("cx")
  assert harness.pull(double, "p", 10) == refused("p")
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == 0
  harness.fired(double)
}

// What each rule changes, how often it reads the clock, and whether it
// draws a lease id -------------------------------------------
//
// Each test compares the complete snapshot after one pull with the snapshot
// before it, changed as the rule's `ensures` lines say and in no other way.
// That a new lease is added at the end of the leases is `impl-choice` (spec
// note G19). Each test's superseded queue holds a retained version before
// the pull, so that a rule which emptied it would show.

/// Pockets `x`, `y` and `z` in that order, `x` with a superseded version
/// retained, and `c2` already holds `x`. A zero limit shows the claim alone:
/// `c1` takes `y`, the queued pocket with the smallest position, under a new
/// lease granted at the time of the pull, and no record moves. Then `c3`
/// claims `z` with a batch, and the record it is handed is held as well.
fn claims_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "x", "a", 1, "xa")
    == published("PublishIntoNewPocket")
  assert harness.publish(double, "x", "b", 1, "xb")
    == published("PublishNewKey")
  assert harness.publish(double, "y", "a", 2, "ya")
    == published("PublishIntoNewPocket")
  assert harness.publish(double, "z", "a", 3, "za")
    == published("PublishIntoNewPocket")
  assert harness.publish(double, "x", "b", 2, "xb2") == published("Supersede")
  let theirs = harness.batch(double, "c2", 1)
  assert #(theirs.fired, theirs.pocket_key) == #([claims, hold], "x")
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let assert [x, y, z] = before.pockets
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)

  let now = harness.advance(double.clock, harness.t0, 30)
  let mine = harness.batch(double, "c1", 0)
  assert mine == Pulled([claims], mine.lease, "y", [])
  assert mine.lease != theirs.lease
  let claimed =
    Snapshot(
      ..before,
      pockets: [x, Pocket(..y, status: PocketLeased), z],
      leases: list.append(before.leases, [
        Lease(
          id: mine.lease,
          pocket: "y",
          consumer: "c1",
          status: LeaseActive,
          granted_at: now,
          expires_at: now + lease_ttl(),
        ),
      ]),
    )
  assert harness.inspect(double) == claimed
  assert harness.clock_reads(double.clock) == reads + 1
  assert harness.lease_id_draws(double.lease_ids) == draws + 1

  let later = harness.advance(double.clock, now, 45)
  let last = harness.batch(double, "c3", 7)
  assert last
    == Pulled([claims, hold], last.lease, "z", [
      BatchRecord(key: "a", rank: 3, payload: <<"za":utf8>>),
    ])
  assert !list.contains([theirs.lease, mine.lease], last.lease)
  let assert [xa, xb, ya, za] = claimed.records
  assert harness.inspect(double)
    == Snapshot(
      ..claimed,
      pockets: [
        x,
        Pocket(..y, status: PocketLeased),
        Pocket(..z, status: PocketLeased),
      ],
      records: [
        xa,
        xb,
        ya,
        Record(
          ..za,
          status: RecordHeld(
            held_rank: 3,
            held_payload: <<"za":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: None,
          ),
        ),
      ],
      leases: list.append(claimed.leases, [
        Lease(
          id: last.lease,
          pocket: "z",
          consumer: "c3",
          status: LeaseActive,
          granted_at: later,
          expires_at: later + lease_ttl(),
        ),
      ]),
    )
  assert harness.clock_reads(double.clock) == reads + 2
  assert harness.lease_id_draws(double.lease_ids) == draws + 2
  harness.fired(double)
}

/// `c1` holds `pk` and `c2` holds `other`, while `spare` stays queued, with
/// a superseded version retained: a consumer that holds a lease pulls within
/// it and claims no second pocket. The clock moves before each pull but the
/// second, so each `expires_at` is that pull's `now` plus `lease_ttl`, the
/// same one again where the clock has not moved, and `granted_at` stays.
/// With a limit of 0 and of -1 nothing but the lease's `expires_at` changes;
/// then the rest of the pocket is handed out, and a pull of the drained
/// pocket again changes nothing but the `expires_at`. No pull here draws a
/// lease id.
fn within_lease_exactly() -> List(String) {
  let double = pocket_of(3)
  assert harness.publish(double, "other", "o", 1, "o")
    == published("PublishIntoNewPocket")
  assert harness.publish(double, "spare", "s", 1, "s")
    == published("PublishIntoNewPocket")
  assert harness.publish(double, "spare", "s", 2, "s2")
    == published("Supersede")
  let mine = harness.batch(double, "c1", 1)
  assert #(mine.fired, mine.pocket_key) == #([claims, hold], "pk")
  let theirs = harness.batch(double, "c2", 1)
  assert #(theirs.fired, theirs.pocket_key) == #([claims, hold], "other")
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let assert [lease, other_lease] = before.leases
  assert lease.id == mine.lease
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  let extended = fn(now) {
    [Lease(..lease, expires_at: now + lease_ttl()), other_lease]
  }

  let first = harness.advance(double.clock, harness.t0, 40)
  assert harness.batch(double, "c1", 0) == Pulled([within], lease.id, "pk", [])
  assert harness.inspect(double) == Snapshot(..before, leases: extended(first))
  assert harness.clock_reads(double.clock) == reads + 1

  // The clock has not moved, so neither has the `expires_at`: pulls within a
  // lease do not add up.
  assert harness.batch(double, "c1", 0) == Pulled([within], lease.id, "pk", [])
  assert harness.inspect(double) == Snapshot(..before, leases: extended(first))
  assert harness.clock_reads(double.clock) == reads + 2

  let second = harness.advance(double.clock, first, 25)
  assert harness.batch(double, "c1", -1) == Pulled([within], lease.id, "pk", [])
  assert harness.inspect(double) == Snapshot(..before, leases: extended(second))
  assert harness.clock_reads(double.clock) == reads + 3

  let third = harness.advance(double.clock, second, 300)
  assert harness.batch(double, "c1", 10)
    == Pulled([within, hold, hold], lease.id, "pk", [
      BatchRecord(key: "r1", rank: 1, payload: <<"r1":utf8>>),
      BatchRecord(key: "r2", rank: 1, payload: <<"r2":utf8>>),
    ])
  let assert [r0, r1, r2, o, s] = before.records
  let drained =
    Snapshot(
      ..before,
      records: [r0, held(r1), held(r2), o, s],
      leases: extended(third),
    )
  assert harness.inspect(double) == drained
  assert harness.clock_reads(double.clock) == reads + 4

  let fourth = harness.advance(double.clock, third, 500)
  assert harness.batch(double, "c1", 10) == Pulled([within], lease.id, "pk", [])
  assert harness.inspect(double)
    == Snapshot(..drained, leases: extended(fourth))
  assert harness.clock_reads(double.clock) == reads + 5
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}

/// Over an empty queue, and then with the one pocket leased to `c2`, a
/// superseded version retained and the clock moved: the pull answers
/// `NoPocketAvailable` whatever its limit, and the complete snapshot, the
/// clock's reads and the lease-id draws are as they were.
fn finds_no_pocket_exactly() -> List(String) {
  let double = harness.fixture()
  let none = Ok(Outcome([finds_none], Some(NoPocketAvailable("c1"))))
  let empty = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  assert harness.pull(double, "c1", 5) == none
  assert harness.inspect(double) == empty
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == 0

  publish_records(double, 2)
  assert harness.publish(double, "pk", "r1", 2, "r1") == published("Supersede")
  assert harness.batch(double, "c2", 1).fired == [claims, hold]
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let reads = harness.clock_reads(double.clock)
  harness.set_time(double.clock, harness.t0 + 70)
  assert harness.pull(double, "c1", 5) == none
  assert harness.pull(double, "c1", 0) == none
  assert harness.pull(double, "c1", -1) == none
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == 1
  harness.fired(double)
}

// The rows themselves, over states built by hand -------------
//
// Through the owner a list's order and the positions agree: until a pocket
// or a record can enter an order a second time, each was created in the
// order of its position. These tests run the apply step over a state built
// by hand in which they disagree.

/// The time a hand-built lease was granted at.
const granted = 500

/// Consumers `c1` and `c2` joined, and no list is in position order.
/// `taken` is leased to `c2`, and of its three records the one with the
/// smallest position is already held. `early` and `late` are queued, `late`
/// listed first; `parked` and `idle`, with smaller positions than either,
/// are not. `early`'s records are listed largest position first but one.
/// Each record's rank and payload are unlike its position. `late` holds the
/// key `k8`, as `taken` does. The superseded queue holds an earlier version
/// of `early`'s `k1`, and the dead-letter queue a record `c2` once nacked
/// out of `taken`. Each test that compares a complete state over this one
/// shows that its rule left both queues as they were.
fn disordered() -> State {
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 20),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1"), Consumer("c2")],
    pockets: [
      Pocket("late", PocketQueued, 12, 0),
      Pocket("parked", PocketParked, 0, 3),
      Pocket("taken", PocketLeased, 1, 0),
      Pocket("idle", PocketIdle, 2, 0),
      Pocket("early", PocketQueued, 5, 0),
    ],
    records: [
      Record("late", "k8", 41, <<"late":utf8>>, 13, RecordQueued),
      Record("parked", "k", 42, <<"parked":utf8>>, 3, RecordQueued),
      Record("taken", "k9", 43, <<"nine":utf8>>, 10, RecordQueued),
      held(Record("taken", "k7", 44, <<"seven":utf8>>, 4, RecordQueued)),
      Record("taken", "k8", 45, <<"eight":utf8>>, 7, RecordQueued),
      Record("early", "k3", 46, <<"three":utf8>>, 11, RecordQueued),
      Record("early", "k1", 47, <<"one":utf8>>, 6, RecordQueued),
      Record("early", "k2", 48, <<"two":utf8>>, 9, RecordQueued),
    ],
    leases: [
      Lease(
        id: "lease-1",
        pocket: "taken",
        consumer: "c2",
        status: LeaseActive,
        granted_at: granted,
        expires_at: granted + config.defaults().lease_ttl,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: "early",
        key: "k1",
        rank: 40,
        payload: <<"older":utf8>>,
        reason: Superseded,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "taken",
        key: "k6",
        rank: 39,
        payload: <<"six":utf8>>,
        nacked_by: "c2",
        nacked_at: 350,
        expires_at: 350 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// One trigger through the apply step, with this time and this lease id.
fn applied(
  state: State,
  trigger: core.Trigger,
  now: Int,
  lease_id: String,
) -> Result(Applied, Failure) {
  rules.drive(core.apply(state, trigger), fn() { now }, fn() { lease_id })
}

/// `HoldRecord` over `taken`'s queued `k8`: the record is held at its own
/// rank and payload, unsettled, with no nack time and nothing deferred, and
/// nothing else changes, `late`'s `k8` included. The step is done without
/// asking for the time. A `Handed` for a record that is already held, for a
/// key the pocket does not hold, or for a pocket that does not hold the key,
/// finds no rule, which is a defect.
fn hold_record_exactly() -> List(String) {
  let state = disordered()
  let assert [late, parked, k9, k7, k8, k3, k1, k2] = state.records
  let handed = fn(pocket_key, record_key) {
    core.run(pull.handed(), Handover(pocket_key:, record_key:), state)
  }
  assert handed("taken", "k8")
    == Done(
      Ok(Applied(
        State(..state, records: [
          late,
          parked,
          k9,
          k7,
          Record(
            ..k8,
            status: RecordHeld(
              held_rank: 45,
              held_payload: <<"eight":utf8>>,
              mark: Unsettled,
              nacked_at: None,
              deferred_position: None,
            ),
          ),
          k3,
          k1,
          k2,
        ]),
        [hold],
        None,
      )),
    )
  let defect = Done(Error(Defect("Handed", NoRuleHeld)))
  assert handed("taken", "k7") == defect
  assert handed("taken", "k1") == defect
  assert handed("idle", "k8") == defect
  assert handed("nowhere", "k8") == defect
  [hold]
}

/// `c1`, with no lease, claims `early`: the queued pocket with the smallest
/// position, though `late` is listed before it and `parked`, `taken` and
/// `idle` have smaller positions and are not queued. Its batch of two is
/// `k1` then `k2`, the smallest positions first, though `k3` is listed
/// first. Its next pull hands `k3`. `c2`, pulling one record within its
/// lease, is handed `k8`: `k7` has a smaller position and is held already,
/// and `k9` is listed first with a larger one.
fn orders_by_position() -> List(String) {
  let state = disordered()
  let assert [late, parked, taken, idle, early] = state.pockets
  let assert [late_k8, parked_k, k9, k7, k8, k3, k1, k2] = state.records
  let assert [lease] = state.leases
  let ttl = config.defaults().lease_ttl

  let first = 900
  let mine =
    Lease(
      id: "lease-2",
      pocket: "early",
      consumer: "c1",
      status: LeaseActive,
      granted_at: first,
      expires_at: first + ttl,
    )
  let claimed =
    State(
      ..state,
      pockets: [
        late,
        parked,
        taken,
        idle,
        Pocket(..early, status: PocketLeased),
      ],
      records: [late_k8, parked_k, k9, k7, k8, k3, held(k1), held(k2)],
      leases: [lease, mine],
    )
  assert applied(state, core.Pull(Pull("c1", 2)), first, "lease-2")
    == Ok(Applied(
      claimed,
      [claims, hold, hold],
      Some(
        Batch(consumer: "c1", lease: "lease-2", pocket: "early", records: [
          BatchRecord(key: "k1", rank: 47, payload: <<"one":utf8>>),
          BatchRecord(key: "k2", rank: 48, payload: <<"two":utf8>>),
        ]),
      ),
    ))

  let second = 1300
  let drained =
    State(
      ..claimed,
      records: [late_k8, parked_k, k9, k7, k8, held(k3), held(k1), held(k2)],
      leases: [lease, Lease(..mine, expires_at: second + ttl)],
    )
  assert applied(claimed, core.Pull(Pull("c1", 5)), second, "unused")
    == Ok(Applied(
      drained,
      [within, hold],
      Some(
        Batch(consumer: "c1", lease: "lease-2", pocket: "early", records: [
          BatchRecord(key: "k3", rank: 46, payload: <<"three":utf8>>),
        ]),
      ),
    ))

  let third = 1700
  assert applied(drained, core.Pull(Pull("c2", 1)), third, "unused")
    == Ok(Applied(
      State(
        ..drained,
        records: [
          late_k8,
          parked_k,
          k9,
          k7,
          held(k8),
          held(k3),
          held(k1),
          held(k2),
        ],
        leases: [
          Lease(..lease, expires_at: third + ttl),
          Lease(..mine, expires_at: second + ttl),
        ],
      ),
      [within, hold],
      Some(
        Batch(consumer: "c2", lease: "lease-1", pocket: "taken", records: [
          BatchRecord(key: "k8", rank: 45, payload: <<"eight":utf8>>),
        ]),
      ),
    ))
  [claims, hold, hold, within, hold, within, hold]
}

/// The rule the Pull table sends this pull to over this state, if any.
fn rule(state: State, request: Pull) -> Option(String) {
  let assert Ok(row) = rules.dispatch(pull.pull(), state, request)
  option.map(row, fn(row) { row.name })
}

/// All three rules read `Lease{consumer: consumer, status: active}`. No
/// lease expires through the owner, so the state is built by hand. `c1`
/// once held `taken` under a lease that has since been released or has
/// expired, and `c2` holds it now. That ended lease is not `c1`'s active
/// lease: with a pocket queued `c1` claims the front one, `early`, and is
/// handed nothing of `taken`'s; with none queued it is told
/// `NoPocketAvailable`. It never pulls within `c2`'s lease.
fn ended_lease() -> List(String) {
  list.flat_map(
    [LeaseReleased(granted + 5), LeaseExpired(granted + 5)],
    fn(ended) {
      let state = disordered()
      let state =
        State(..state, leases: [
          Lease(
            id: "lease-0",
            pocket: "taken",
            consumer: "c1",
            status: ended,
            granted_at: granted,
            expires_at: granted + config.defaults().lease_ttl,
          ),
          ..state.leases
        ])
      assert rule(state, Pull("c1", 1)) == Some(claims)
      let assert Ok(claim) =
        applied(state, core.Pull(Pull("c1", 1)), 900, "new")
      assert claim.fired == [claims, hold]
      assert claim.answer
        == Some(
          Batch(consumer: "c1", lease: "new", pocket: "early", records: [
            BatchRecord(key: "k1", rank: 47, payload: <<"one":utf8>>),
          ]),
        )

      let none_queued =
        State(
          ..state,
          pockets: list.map(state.pockets, fn(pocket) {
            case pocket.status {
              PocketQueued -> Pocket(..pocket, status: PocketParked)
              _ -> pocket
            }
          }),
        )
      assert rule(none_queued, Pull("c1", 1)) == Some(finds_none)
      assert applied(none_queued, core.Pull(Pull("c1", 1)), 900, "unused")
        == Ok(Applied(none_queued, [finds_none], Some(NoPocketAvailable("c1"))))
      [claims, hold, finds_none]
    },
  )
}

/// `impl-choice` (spec note G23, as N14): a state no pull through the
/// owner reaches, since it breaks `UnleasedHasNoActiveLease`. `c2`'s lease
/// is active and its pocket is queued, idle or parked. `PullWithinLease`
/// requires the pocket leased, and the other two rules require a consumer
/// with no active lease, so no guard holds and the table refuses. The step
/// is done at once: no clock is read, no lease id is drawn and no state is
/// handed back.
fn lease_on_unleased_pocket() -> List(String) {
  list.each([PocketQueued, PocketIdle, PocketParked], fn(status) {
    let state = disordered()
    let pockets =
      list.map(state.pockets, fn(pocket) {
        case pocket.key {
          "taken" -> Pocket(..pocket, status:)
          _ -> pocket
        }
      })
    let state = State(..state, pockets:)
    assert rules.dispatch(pull.pull(), state, Pull("c2", 1)) == Ok(None)
    assert core.apply(state, core.Pull(Pull("c2", 1)))
      == Done(Error(Refused(PullRefused("c2"))))
  })
  []
}
