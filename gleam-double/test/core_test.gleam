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
//// Not shown here: an outcome whose first row leaves an invariant broken
//// and whose chained event mends it, which the checker must accept since
//// it runs once, after the whole chain. Of the domain's two events,
//// `PocketTouched` goes to a table with no row yet, and `Handed` mends
//// nothing: no state between a pull's own row and its `HoldRecord`s breaks
//// an invariant. A test cannot hand `core.run` another chain. The first
//// revive rule makes that case writable.

import gleam/list
import gleam/option.{None}
import pachinko_double/config
import pachinko_double/core
import pachinko_double/domain.{type Step, type Table, PocketTouched}
import pachinko_double/model.{type State, Consumer, Publisher, State}
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
