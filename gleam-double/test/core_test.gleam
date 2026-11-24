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
import gleam/option.{None, Some}
import pachinko_double.{Outcome}
import pachinko_double/config
import pachinko_double/failure.{BrokenInvariant, Defect, EffectFailed}
import pachinko_double/ids.{
  type ConsumerId, type PocketKey, type PublisherId, ConsumerId, PocketKey,
  PublisherId, RecordKey,
}
import pachinko_double/internal/core
import pachinko_double/internal/domain.{type Step, type Table, PocketTouched}
import pachinko_double/internal/rules.{
  Applied, Finish, LeaseId, NothingHappens, Now, Row, Table,
}
import pachinko_double/invariants
import pachinko_double/model.{
  type State, Consumer, PocketIdle, PocketQueued, Publisher, State,
}
import support/harness.{type Double}
import support/runner.{type Test, Test}

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
      "a run of due outcomes keeps each outcome that succeeded before a failure, stops at the failure, and checks each outcome at its end",
      [],
      run_due_keeps_what_succeeded,
    ),
    Test(
      "a due outcome that asks for a lease id or for the time is a defect of the tick",
      [],
      run_due_asks_nothing,
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
  State(
    ..model.new(config.default()),
    publishers: [Publisher(PublisherId("p"))],
    consumers: [
      Consumer(ConsumerId("c")),
    ],
  )
}

fn with_publisher(state: State, id: PublisherId) -> State {
  State(..state, publishers: list.append(state.publishers, [Publisher(id)]))
}

fn with_consumer(state: State, id: ConsumerId) -> State {
  State(..state, consumers: list.append(state.consumers, [Consumer(id)]))
}

/// A table of one row, whose guard always holds and which checks nothing:
/// it adds a publisher with the id it is given, taken or not, and emits a
/// `PocketTouched` for each pocket key in `touching`.
fn adds_publisher(touching: List(PocketKey)) -> Table(PublisherId) {
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
  rules.drive(step, fn() { 0 }, fn() { ids.LeaseId("unused") })
}

/// The same row on the same state, twice. With an id that is free the
/// outcome stands; with `p`, which is taken, it leaves two publishers with
/// one id, and the outcome fails, naming the invariant.
fn broken_invariant() -> List(String) {
  let table = adds_publisher([])
  assert drive(core.run(table, PublisherId("q"), joined()))
    == Ok(Applied(
      with_publisher(joined(), PublisherId("q")),
      ["AddsPublisher"],
      None,
    ))
  assert drive(core.run(table, PublisherId("p"), joined()))
    == Error(BrokenInvariant(["UniquePublisherId"]))
  []
}

fn two_broken_invariants() -> List(String) {
  let table =
    Table(
      "Scratch",
      [
        Row("AddsBoth", fn(_state, _id) { True }, fn(state, _id) {
          Finish(
            state
              |> with_consumer(ConsumerId("c"))
              |> with_publisher(PublisherId("p")),
            [],
            None,
          )
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
  let table = adds_publisher([PocketKey("no-such-pocket")])
  assert drive(core.run(table, PublisherId("q"), joined()))
    == Ok(Applied(
      with_publisher(joined(), PublisherId("q")),
      ["AddsPublisher"],
      None,
    ))
  assert drive(core.run(table, PublisherId("p"), joined()))
    == Error(BrokenInvariant(["UniquePublisherId"]))
  []
}

/// The checker does not refuse everything: the real join, through
/// `core.apply`, on a state that breaks nothing.
fn join_still_succeeds() -> List(String) {
  let assert Ok(applied) =
    drive(core.apply(joined(), core.PublisherJoins(PublisherId("q"))))
  assert applied
    == Applied(
      with_publisher(joined(), PublisherId("q")),
      ["PublisherJoins"],
      None,
    )
  applied.fired
}

// A tick's due outcomes, run one after another --------------

/// Step 5's choice, on scratch tables. Three outcomes, each a whole outcome
/// through `core.run`: the first adds `q`, which stands; the second adds
/// `p`, which is taken, so the checker fails that outcome with
/// `BrokenInvariant`; the third would add `r`. The run keeps the first
/// outcome's state, reports only its rules, never runs the third, and names
/// the broken invariant. Were the checker run once at the end of the run
/// instead, the second outcome would not stop it. The tick keeps what
/// succeeded and does not roll back to the state before it.
fn run_due_keeps_what_succeeded() -> List(String) {
  let table = adds_publisher([])
  let outcomes =
    list.map([PublisherId("q"), PublisherId("p"), PublisherId("r")], fn(id) {
      fn(state) { core.run(table, id, state) }
    })
  assert core.run_due(joined(), outcomes)
    == core.Ran(
      state: with_publisher(joined(), PublisherId("q")),
      fired: [["AddsPublisher"]],
      failure: Some(BrokenInvariant(["UniquePublisherId"])),
    )
  // With no failure every outcome runs, each over the state the one before
  // left.
  assert core.run_due(joined(), [
      fn(state) { core.run(table, PublisherId("q"), state) },
      fn(state) { core.run(table, PublisherId("r"), state) },
    ])
    == core.Ran(
      state: joined()
        |> with_publisher(PublisherId("q"))
        |> with_publisher(PublisherId("r")),
      fired: [["AddsPublisher"], ["AddsPublisher"]],
      failure: None,
    )
  ["AddsPublisher"]
}

/// An outcome a tick runs must be done without asking for anything. One
/// whose row asks for a lease id, or one not run with the tick's `now` in
/// hand whose row asks for the time, stops the run as a defect, keeping
/// what came before.
fn run_due_asks_nothing() -> List(String) {
  let asks = fn(plan) {
    Table(
      "Scratch",
      [Row("Asks", fn(_state, _id) { True }, fn(state, _id) { plan(state) })],
      NothingHappens,
    )
  }
  let id_table =
    asks(fn(state) { LeaseId(fn(_id) { Finish(state, [], None) }) })
  let now_table = asks(fn(state) { Now(fn(_now) { Finish(state, [], None) }) })
  let first = fn(state) {
    core.run(adds_publisher([]), PublisherId("q"), state)
  }
  assert core.run_due(joined(), [
      first,
      fn(state) { core.run(id_table, "x", state) },
      fn(state) { core.run(adds_publisher([]), PublisherId("r"), state) },
    ])
    == core.Ran(
      state: with_publisher(joined(), PublisherId("q")),
      fired: [["AddsPublisher"]],
      failure: Some(Defect(
        "Tick",
        EffectFailed("Tick", "an outcome of a tick asked for a lease id"),
      )),
    )
  assert core.run_due(joined(), [
      first,
      fn(state) { core.run(now_table, "x", state) },
    ])
    == core.Ran(
      state: with_publisher(joined(), PublisherId("q")),
      fired: [["AddsPublisher"]],
      failure: Some(Defect(
        "Tick",
        EffectFailed("Tick", "an outcome of a tick asked for the time"),
      )),
    )
  // The same row run with a `now` in hand asks nothing of the outside.
  assert core.run_due(joined(), [
      fn(state) { core.run_at(now_table, "x", state, 5) },
    ])
    == core.Ran(state: joined(), fired: [["Asks"]], failure: None)
  []
}

// The witness: the checker runs after the whole chained outcome ----------

/// The fixture with pocket `pk` holding `k`, pulled by `c1`, and that
/// lease's id.
fn held() -> #(Double, ids.LeaseId) {
  let double = harness.fixture()
  assert harness.publish(double, PocketKey("pk"), RecordKey("k"), 1, "v")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  let batch = harness.batch(double, ConsumerId("c1"), 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  #(double, batch.lease)
}

/// `pk`, with its one record acked, released by `c1`: the pocket is idle
/// and holds no record.
fn idle() -> Double {
  let #(double, lease) = held()
  assert harness.ack(double, ConsumerId("c1"), lease, RecordKey("k"))
    == Ok(Outcome(["AckMarksRecord"], None))
  assert harness.release(double, ConsumerId("c1"), lease)
    == Ok(Outcome(["ReleaseIdlesEmptyPocket", "ConsumeAckedRecord"], None))
  assert harness.pocket(double, PocketKey("pk")).status == PocketIdle
  double
}

/// A publish of a new key into the idle `pk`. Once `PublishNewKey` has
/// added the record, and before `ReviveIdlePocket` answers the
/// `PocketTouched` it emitted, the pocket is idle and has a record, which
/// breaks `IdleHasNoRecords`. The revival queues the pocket, and the whole
/// outcome holds every invariant: the publish stands.
fn revival_mends_the_publish() -> List(String) {
  let double = idle()
  assert harness.publish(double, PocketKey("pk"), RecordKey("k2"), 1, "v2")
    == Ok(Outcome(["PublishNewKey", "ReviveIdlePocket"], None))
  assert harness.pocket(double, PocketKey("pk")).status == PocketQueued
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
  assert harness.release(double, ConsumerId("c1"), lease)
    == Ok(Outcome(["ReleaseRequeuesPocket", "RequeueUnsettledRecord"], None))
  assert harness.pocket(double, PocketKey("pk")).status == PocketQueued
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
