//// The end of the apply step: `core.run` hands the state a whole outcome
//// left to the invariant checker, and an outcome that leaves an invariant
//// broken fails, naming it. The tables here are scratch tables of the
//// domain's types; no specification rule is involved but the join in the
//// last test.
////
//// A failure carries no state, so there is no new state to keep: the only
//// state after `Error(BrokenInvariant(..))` is the one the caller passed
//// in.
////
//// The last three tests are the witness that the checker runs once, after
//// the whole chain, and never part-way: an outcome whose first row leaves
//// an invariant broken and whose chained event mends it, which the checker
//// must accept. A test cannot hand `core.run` another chain, so the case
//// needs the specification's own rules, and these tests drive the owner.
//// `Handed` mends nothing: no state between a pull's own row and its
//// `HoldRecord`s breaks an invariant. Two outcomes do pass through such a
//// state: a publish of a new key into an idle pocket, before
//// `ReviveIdlePocket` answers its `PocketTouched`, and a release, before
//// the `Freed` rules answer. A checker that also ran before each chained
//// event's table would fail each of these outcomes with `BrokenInvariant`,
//// naming the invariant each test's comment names.

import gleam/list
import gleam/option.{None}
import harness.{type Double}
import pachinko_double/config
import pachinko_double/core
import pachinko_double/domain.{type Step, type Table, PocketTouched}
import pachinko_double/invariants
import pachinko_double/model.{
  type State, Consumer, PocketIdle, PocketQueued, Publisher, State,
}
import pachinko_double/owner.{Outcome}
import pachinko_double/rules.{
  Applied, BrokenInvariant, Finish, NothingHappens, Row, Table,
}
import runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "an effect that leaves an invariant broken fails, naming it",
      [],
      broken_invariant,
    ),
    Test(
      "an outcome that breaks two invariants names both, in the registry's order",
      [],
      two_broken_invariants,
    ),
    Test(
      "an outcome that emits an event is still checked",
      [],
      checked_after_chain,
    ),
    Test(
      "a publisher join through the apply step still succeeds",
      [],
      join_still_succeeds,
    ),
    Test(
      "a publish of a new key into an idle pocket stands, though the pocket is idle with a record until the revival answers",
      [],
      revival_mends_the_publish,
    ),
    Test(
      "a release that requeues its pocket stands, though the pocket is queued with a held record until Freed is answered",
      [],
      freed_mends_the_requeue,
    ),
    Test(
      "a release that idles its pocket stands, though the pocket is idle with a held record until Freed is answered",
      [],
      freed_mends_the_idling,
    ),
  ]
}

/// A queue one publisher, `p`, and one consumer, `c`, have joined.
fn joined() -> State {
  State(..model.new(config.default()), publishers: [Publisher("p")], consumers: [
    Consumer("c"),
  ])
}

fn with_publisher(state: State, id: String) -> State {
  State(..state, publishers: list.append(state.publishers, [Publisher(id)]))
}

fn with_consumer(state: State, id: String) -> State {
  State(..state, consumers: list.append(state.consumers, [Consumer(id)]))
}

/// A table of one row, whose guard always holds and which checks nothing:
/// it adds a publisher with the id it is given, taken or not, and emits a
/// `PocketTouched` for each pocket key in `touching`.
fn adds_publisher(touching: List(String)) -> Table(String) {
  Table(
    "Scratch",
    [
      Row("AddsPublisher", fn(_state, _id) { True }, fn(state, id) {
        Finish(
          with_publisher(state, id),
          list.map(touching, PocketTouched),
          None,
        )
      }),
    ],
    NothingHappens,
  )
}

fn drive(step: Step) {
  rules.drive(step, fn() { 0 }, fn() { "unused" })
}

/// The same row on the same state, twice. With an id that is free the
/// outcome stands; with `p`, which is taken, it leaves two publishers with
/// one id, and the outcome fails, naming the invariant.
fn broken_invariant() -> List(String) {
  let table = adds_publisher([])
  assert drive(core.run(table, "q", joined()))
    == Ok(Applied(with_publisher(joined(), "q"), ["AddsPublisher"], None))
  assert drive(core.run(table, "p", joined()))
    == Error(BrokenInvariant(["UniquePublisherId"]))
  []
}

fn two_broken_invariants() -> List(String) {
  let table =
    Table(
      "Scratch",
      [
        Row("AddsBoth", fn(_state, _id) { True }, fn(state, _id) {
          Finish(state |> with_consumer("c") |> with_publisher("p"), [], None)
        }),
      ],
      NothingHappens,
    )
  assert drive(core.run(table, "x", joined()))
    == Error(BrokenInvariant(["UniquePublisherId", "UniqueConsumerId"]))
  []
}

/// The row emits `PocketTouched`, whose table answers that nothing
/// happens. The outcome still ends in the checker: the valid one stands,
/// with only the scratch row fired, and the broken one fails.
fn checked_after_chain() -> List(String) {
  let table = adds_publisher(["no-such-pocket"])
  assert drive(core.run(table, "q", joined()))
    == Ok(Applied(with_publisher(joined(), "q"), ["AddsPublisher"], None))
  assert drive(core.run(table, "p", joined()))
    == Error(BrokenInvariant(["UniquePublisherId"]))
  []
}

/// The checker does not refuse everything: the real join, through
/// `core.apply`, on a state that breaks nothing.
fn join_still_succeeds() -> List(String) {
  let assert Ok(applied) = drive(core.apply(joined(), core.PublisherJoins("q")))
  assert applied
    == Applied(with_publisher(joined(), "q"), ["PublisherJoins"], None)
  applied.fired
}

// The witness: the checker runs after the whole chained outcome ----------

/// The fixture with pocket `pk` holding `k`, pulled by `c1`, and that
/// lease's id.
fn held() -> #(Double, String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  #(double, batch.lease)
}

/// `pk`, with its one record acked, released by `c1`: the pocket is idle
/// and holds no record.
fn idle() -> Double {
  let #(double, lease) = held()
  assert harness.ack(double, "c1", lease, "k")
    == Ok(Outcome(["AckMarksRecord"], None))
  assert harness.release(double, "c1", lease)
    == Ok(Outcome(["ReleaseIdlesEmptyPocket", "ConsumeAckedRecord"], None))
  assert harness.pocket(double, "pk").status == PocketIdle
  double
}

/// A publish of a new key into the idle `pk`. Once `PublishNewKey` has
/// added the record, and before `ReviveIdlePocket` answers the
/// `PocketTouched` it emitted, the pocket is idle and has a record, which
/// breaks `IdleHasNoRecords`. The revival queues the pocket, and the whole
/// outcome holds every invariant: the publish stands.
fn revival_mends_the_publish() -> List(String) {
  let double = idle()
  assert harness.publish(double, "pk", "k2", 1, "v2")
    == Ok(Outcome(["PublishNewKey", "ReviveIdlePocket"], None))
  assert harness.pocket(double, "pk").status == PocketQueued
  assert invariants.check(harness.inspect(double)) == []
  harness.fired(double)
}

/// A release of `pk` with its one record held and unsettled. Once
/// `ReleaseRequeuesPocket` has queued the pocket, and before
/// `RequeueUnsettledRecord` answers the `Freed` it emitted, the record is
/// held in a pocket that is not leased, which breaks
/// `OutstandingOnlyWhileLeased`. The `Freed` rule queues the record, and the
/// whole outcome holds every invariant: the release stands.
fn freed_mends_the_requeue() -> List(String) {
  let #(double, lease) = held()
  assert harness.release(double, "c1", lease)
    == Ok(Outcome(["ReleaseRequeuesPocket", "RequeueUnsettledRecord"], None))
  assert harness.pocket(double, "pk").status == PocketQueued
  assert invariants.check(harness.inspect(double)) == []
  harness.fired(double)
}

/// A release of `pk` with its one record held and acked, the setup of the
/// first of these three tests, as a test of its own. Once
/// `ReleaseIdlesEmptyPocket` has idled the pocket, and before
/// `ConsumeAckedRecord` answers the `Freed` it emitted, the pocket is idle
/// and has a record, and that record is held in a pocket that is not
/// leased: `IdleHasNoRecords` and `OutstandingOnlyWhileLeased` are both
/// broken. The `Freed` rule removes the record, and the release stands.
fn freed_mends_the_idling() -> List(String) {
  let double = idle()
  assert invariants.check(harness.inspect(double)) == []
  harness.fired(double)
}
