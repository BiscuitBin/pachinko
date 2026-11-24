//// Branch A (decision 0019): an action processed after its lease's
//// `expires_at` but before the lapse succeeds, as long as its other guards
//// hold, and a lapse processed first fences it. The owner's mailbox is the
//// one serialized stream, and only a `Tick` lapses a lease.
////
//// First S066 and S067, each in two halves, as the Rust has them in four
//// tests of `crates/libpachinko/tests/queue/expiry.rs`; each half is its own
//// registered test carrying the scenario's tag. The Rust forks one queue
//// into two that share a clock, and acts on each in one order. The double
//// cannot fork an owner, so each half builds the same setup twice, on two
//// owners, and drives both from this one test process, whose messages each
//// owner takes in the order they were sent: on the first the `Tick` is sent
//// before the action, on the second after it.
////
//// A port asserts what its Rust test asserts, with these changes. The
//// Rust's renew answers the new `expires_at`, its ack `Acked` and its
//// release the pocket's fate and the ended lease; the specification's
//// answer nothing, so a port asserts the rules fired and reads the state
//// from the snapshot (N24, N32). The Rust's refusal is `LeaseNotActive`;
//// the double's is `LeaseFenced`, through `harness.refused` (G28). S066's
//// release half asserts that the released lease stays, released, and that
//// no lease is active, where the Rust's asserts an empty registry (the
//// ruling on N38). S067's pull half reads `expires_at` from the snapshot's
//// lease, since the specification's `Batch` carries none (N13, G22).
////
//// Then a nack after `expires_at`, which no Rust test has and which pins no
//// catalog row, in the same two orders.
////
//// Then one test with competing callers, which pins no catalog row: one
//// process renews and another ticks, at once, and whichever the mailbox
//// puts first, the renew's answer, the tick's report and the final state
//// agree.

import gleam/erlang/process
import gleam/list
import gleam/option.{None, Some}
import pachinko_double.{type PartialTick, type Reply, Outcome}
import pachinko_double/answers.{type TickReport, TickReport}
import pachinko_double/config.{Settings}
import pachinko_double/failure.{Refused}
import pachinko_double/invariants
import pachinko_double/model.{
  LeaseActive, LeaseExpired, LeaseReleased, PocketLeased, PocketQueued,
  RecordQueued,
}
import pachinko_double/refusals.{LeaseFenced}
import support/harness.{type Double}
import support/runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a renew after expires_at but before the lapse succeeds (branch A)",
      [Scenario("S066")],
      renew_after_expiry,
    ),
    Test(
      "a release after expires_at but before the lapse succeeds (branch A)",
      [Scenario("S066")],
      release_after_expiry,
    ),
    Test(
      "an ack after expires_at but before the lapse succeeds (branch A)",
      [Scenario("S067")],
      ack_after_expiry,
    ),
    Test(
      "a pull after expires_at but before the lapse extends the lease (branch A)",
      [Scenario("S067")],
      pull_after_expiry,
    ),
    Test(
      "a nack after expires_at but before the lapse succeeds and extends the lease from its own time (branch A)",
      [],
      nack_after_expiry,
    ),
    Test(
      "a renew and a tick sent at once by two callers agree whichever the owner takes first",
      [],
      renew_and_tick_compete,
    ),
  ]
}

const ttl = 10

fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

/// One owner of the Rust test file's `late`: a 10 ms lease, pocket `a`
/// holding `k`, then pocket `b` holding `b1`, `c1`'s lease on `a` granted at
/// t0, and the clock at t0 + 11, past its `expires_at`. Answers the lease's
/// id.
fn late_one() -> #(Double, String) {
  let double =
    harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: ttl))
  assert harness.publish(double, "a", "k", 1, "k")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "b", "b1", 1, "b1")
    == fired("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 10)
  assert batch.pocket_key == "a"
  let _ = harness.at(double, 11)
  #(double, batch.lease)
}

/// The Rust's `late`: two owners in the same state, each past the lease's
/// `expires_at`, and the lease's id, which is the same on both.
fn late() -> #(Double, Double, String) {
  let #(first, lease) = late_one()
  let #(second, again) = late_one()
  assert again == lease
  assert harness.inspect(second) == harness.inspect(first)
  #(first, second, lease)
}

/// One tick, which must not fail, after which every invariant holds.
fn tick(double: Double) -> TickReport {
  let assert Ok(report) = harness.tick(double)
  assert invariants.check(harness.inspect(double)) == []
  report
}

/// The Rust's `tick_then_refused`: the tick, sent first, lapses the lease
/// and keeps `a` at the front with `k` queued, and then `action`, naming the
/// lapsed lease, is fenced and changes nothing.
fn tick_then_refused(
  first: Double,
  lease: String,
  name: String,
  action: fn() -> Reply,
) -> Nil {
  let report = tick(first)
  let assert [lapsed] = report.lapsed
  assert lapsed.id == lease
  assert lapsed.status == LeaseExpired(ended_at: harness.t0 + 11)
  assert harness.pocket(first, "a").status == PocketQueued
  assert harness.order(first) == ["a", "b"]
  assert harness.record(first, "a", "k").status == RecordQueued
  harness.refused(first, LeaseFenced(name, "c1", lease), action)
}

/// The Rust's `extended_and_survives`: the action, sent before the tick,
/// left the lease active and expiring at t0 + 21 with `a` leased, and the
/// tick behind it lapses nothing.
fn extended_and_survives(second: Double, lease: String) -> Nil {
  let assert Ok(Some(view)) = harness.lease_view(second, "c1")
  assert view.expires_at == harness.t0 + 21
  let assert [only] = harness.inspect(second).leases
  assert only.id == lease
  assert only.status == LeaseActive
  assert harness.pocket(second, "a").status == PocketLeased
  assert tick(second) == TickReport([], [], [], [])
  assert harness.pocket(second, "a").status == PocketLeased
}

fn both(first: Double, second: Double) -> List(String) {
  list.append(harness.fired(first), harness.fired(second))
}

fn renew_after_expiry() -> List(String) {
  let #(first, second, lease) = late()
  tick_then_refused(first, lease, "Renew", fn() {
    harness.renew(first, "c1", lease)
  })
  assert harness.renew(second, "c1", lease) == fired("Renew")
  extended_and_survives(second, lease)
  both(first, second)
}

/// The second owner's release ends the lease at t0 + 11, requeues `a` at a
/// fresh position behind `b` and `k` where it stood. The ended lease stays
/// in the state, released, and no lease is active (the ruling on N38).
fn release_after_expiry() -> List(String) {
  let #(first, second, lease) = late()
  tick_then_refused(first, lease, "Release", fn() {
    harness.release(first, "c1", lease)
  })
  let next = harness.inspect(second).next_position
  assert harness.release(second, "c1", lease)
    == Ok(Outcome(["ReleaseRequeuesPocket", "RequeueUnsettledRecord"], None))
  let assert [ended] = harness.inspect(second).leases
  assert ended.id == lease
  assert ended.status == LeaseReleased(ended_at: harness.t0 + 11)
  assert model.active(harness.inspect(second).leases) == []
  assert tick(second) == TickReport([], [], [], [])
  // A fresh position.
  assert harness.pocket(second, "a").position == next
  assert harness.order(second) == ["b", "a"]
  both(first, second)
}

fn ack_after_expiry() -> List(String) {
  let #(first, second, lease) = late()
  tick_then_refused(first, lease, "Ack", fn() {
    harness.ack(first, "c1", lease, "k")
  })
  assert harness.ack(second, "c1", lease, "k") == fired("AckMarksRecord")
  extended_and_survives(second, lease)
  both(first, second)
}

/// On the first owner the pull behind the tick claims `a` again under a new
/// lease and is handed `k` again. On the second the pull before the tick is
/// within the lease: it hands nothing, since `k` is still held, and moves
/// the lease's `expires_at` on.
fn pull_after_expiry() -> List(String) {
  let #(first, second, lease) = late()
  let assert [lapsed] = tick(first).lapsed
  assert lapsed.id == lease
  let again = harness.batch(first, "c1", 10)
  assert again.pocket_key == "a"
  // A new lease.
  assert again.lease != lease
  // The record is handed out again.
  assert harness.keys(again.records) == ["k"]
  assert invariants.check(harness.inspect(first)) == []
  let within = harness.batch(second, "c1", 10)
  assert within.fired == ["PullWithinLease"]
  assert within.lease == lease
  // The record is still held.
  assert within.records == []
  extended_and_survives(second, lease)
  both(first, second)
}

/// No Rust test, and no catalog row: the Rust's late actions are a renew,
/// a release, an ack and a pull, and decision 0019's branch A names a nack
/// too. On the first owner the tick lapses the lease and the nack behind
/// it is fenced; on the second the nack, at t0 + 11, extends the lease to
/// t0 + 21, from the nack's own time and not from the passed `expires_at`,
/// and the tick behind it lapses nothing.
fn nack_after_expiry() -> List(String) {
  let #(first, second, lease) = late()
  tick_then_refused(first, lease, "Nack", fn() {
    harness.nack(first, "c1", lease, "k")
  })
  assert harness.nack(second, "c1", lease, "k") == fired("NackMarksRecord")
  extended_and_survives(second, lease)
  both(first, second)
}

/// What each of the two callers was answered.
type Answer {
  Renewed(Reply)
  Ticked(Result(TickReport, PartialTick))
}

/// Starts a caller that waits at a gate, then does `act` and sends back
/// what it was answered.
fn caller(
  ready: process.Subject(process.Subject(Nil)),
  replies: process.Subject(Answer),
  act: fn() -> Answer,
) -> Nil {
  process.spawn(fn() {
    let gate = process.new_subject()
    process.send(ready, gate)
    process.receive_forever(gate)
    process.send(replies, act())
  })
  Nil
}

/// Over `late_one`'s owner, one caller renews `c1`'s lease and another
/// ticks, both let go at once from their gates; the test waits for both
/// replies. It runs ten times with the renewing caller started first and ten
/// with the ticking one first. In every run the snapshot passes the checker,
/// and exactly one of two things holds:
///
/// - the renew came first: it succeeded, the tick lapsed nothing, and the
///   lease is active, expiring at t0 + 21, with `a` leased;
/// - the tick came first: it lapsed the lease, the renew was fenced, and the
///   lease is expired at t0 + 11, with `a` queued and `k` queued.
///
/// The test never asserts which came first.
fn renew_and_tick_compete() -> List(String) {
  list.append(list.repeat(True, 10), list.repeat(False, 10))
  |> list.flat_map(compete)
}

fn compete(renew_first: Bool) -> List(String) {
  let #(double, lease) = late_one()
  let ready = process.new_subject()
  let replies = process.new_subject()
  let renew = fn() { Renewed(harness.renew(double, "c1", lease)) }
  let tick = fn() { Ticked(harness.tick(double)) }
  case renew_first {
    True -> {
      caller(ready, replies, renew)
      caller(ready, replies, tick)
    }
    False -> {
      caller(ready, replies, tick)
      caller(ready, replies, renew)
    }
  }
  let gates =
    list.map([1, 2], fn(_) {
      let assert Ok(gate) = process.receive(ready, pachinko_double.call_timeout)
      gate
    })
  list.each(gates, process.send(_, Nil))
  let answers =
    list.map([1, 2], fn(_) {
      let assert Ok(answer) =
        process.receive(replies, pachinko_double.call_timeout)
      answer
    })
  assert process.receive(replies, 0) == Error(Nil)
  let renewed =
    list.find_map(answers, fn(answer) {
      case answer {
        Renewed(reply) -> Ok(reply)
        Ticked(_) -> Error(Nil)
      }
    })
  let ticked =
    list.find_map(answers, fn(answer) {
      case answer {
        Ticked(reply) -> Ok(reply)
        Renewed(_) -> Error(Nil)
      }
    })
  let snapshot = harness.inspect(double)
  assert invariants.check(snapshot) == []
  let assert [only] = snapshot.leases
  assert only.id == lease
  case renewed, ticked {
    Ok(Ok(Outcome(["Renew"], None))), Ok(Ok(report)) -> {
      assert report == TickReport([], [], [], [])
      assert only.status == LeaseActive
      assert only.expires_at == harness.t0 + 11 + ttl
      assert harness.pocket(double, "a").status == PocketLeased
    }
    Ok(Error(failure)), Ok(Ok(report)) -> {
      assert failure == Refused(LeaseFenced("Renew", "c1", lease))
      assert report.lapsed == [only]
      assert only.status == LeaseExpired(ended_at: harness.t0 + 11)
      assert harness.pocket(double, "a").status == PocketQueued
      assert harness.record(double, "a", "k").status == RecordQueued
    }
    _, _ -> panic as "the renew and the tick answered inconsistently"
  }
  harness.fired(double)
}
