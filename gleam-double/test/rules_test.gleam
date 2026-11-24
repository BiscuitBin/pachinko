//// The rule-table machinery, on scratch tables: no specification rule is
//// involved. The scratch state is a counter and a log of what each row did.

import gleam/int
import gleam/option.{None, Some}
import pachinko_double/failure.{
  Defect, EffectFailed, GuardOverlap, NoRuleHeld, Refused,
}
import pachinko_double/internal/rules.{
  type Cursor, type Plan, type Row, type Step, type Table, type World, Applied,
  Defective, Finish, Impossible, LeaseId, NothingHappens, Now, Position, Refuses,
  Row, Table, World,
}
import support/harness
import support/runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test("two guards that hold are an overlap naming both", [], overlap),
    Test("no guard holds: a refusal", [], otherwise_refuses),
    Test("no guard holds: nothing happens", [], otherwise_nothing),
    Test("no guard holds: a defect", [], otherwise_defect),
    Test("an emitted event runs its table in the same outcome", [], chained),
    Test("a row that reads now reads the clock once", [], timed_once),
    Test("a refusal reads no clock and draws no lease id", [], refusal_is_dry),
    Test("two timed rows in one outcome read the clock once", [], timed_twice),
    Test("only a row that creates a lease draws a lease id", [], lease_id),
    Test("a row is allotted positions and cannot move the counter", [], allots),
    Test("an effect that finds the impossible is a defect", [], impossible),
    Test("a defect after now and a position keeps nothing", [], impossible_late),
    Test("a defect in a chained row ends the outcome", [], impossible_chained),
  ]
}

type Scratch {
  Scratch(next_position: Int, log: List(String))
}

type Event {
  Echo(word: String)
}

type ScratchRow =
  Row(Scratch, String, Event, String)

type ScratchTable =
  Table(Scratch, String, Event, String, String)

type ScratchPlan =
  Plan(Scratch, Event, String)

type ScratchStep =
  Step(Scratch, String, String)

fn note(state: Scratch, entry: String) -> Scratch {
  Scratch(..state, log: [entry, ..state.log])
}

fn holds(_state: Scratch, _input: String) -> Bool {
  True
}

fn never(_state: Scratch, _input: String) -> Bool {
  False
}

fn plain(name: String) -> ScratchRow {
  Row(name, holds, fn(state, input) {
    Finish(note(state, name <> ":" <> input), [], Some(name))
  })
}

/// The table every scratch event goes to. Its first row reads `now`. Its
/// second answers the word `bad`, and finds that it cannot go on.
fn echo_table() -> ScratchTable {
  Table(
    "Echo",
    [
      Row(
        "EchoRow",
        fn(_state, word) { word != "bad" },
        fn(state, word) -> ScratchPlan {
          use now <- Now
          Finish(note(state, word <> "@" <> int.to_string(now)), [], None)
        },
      ),
      Row("BadEcho", fn(_state, word) { word == "bad" }, fn(_state, word) {
        Impossible(word)
      }),
    ],
    Defective,
  )
}

fn world() -> World(Scratch, Event, String, String) {
  World(
    next_position: fn(state: Scratch) { state.next_position },
    with_next_position: fn(state, next_position) {
      Scratch(..state, next_position:)
    },
    chain: fn(event: Event, cursor: Cursor(Scratch), then) {
      rules.run(echo_table(), event.word, cursor, world(), fn(cursor, _) {
        then(cursor)
      })
    },
  )
}

fn begin(table: ScratchTable, input: String) -> ScratchStep {
  rules.run(table, input, rules.start(Scratch(7, [])), world(), rules.finish)
}

fn overlap() -> List(String) {
  let never_row = Row("Second", never, plain("Second").effect)
  let table =
    Table(
      "Scratch",
      [plain("First"), never_row, plain("Third")],
      NothingHappens,
    )
  let fakes = #(harness.start_clock(), harness.start_lease_ids())
  assert rules.dispatch(table, Scratch(7, []), "x")
    == Error(GuardOverlap("Scratch", ["First", "Third"]))
  assert rules.drive(
      begin(table, "x"),
      harness.clock(fakes.0),
      harness.lease_ids(fakes.1),
    )
    == Error(GuardOverlap("Scratch", ["First", "Third"]))
  []
}

fn no_guard(otherwise) -> ScratchTable {
  Table("Scratch", [Row("Never", never, plain("Never").effect)], otherwise)
}

fn drive(step: ScratchStep) {
  rules.drive(step, fn() { 0 }, fn() { "unused" })
}

fn otherwise_refuses() -> List(String) {
  let table = no_guard(Refuses(fn(_state, input) { "refused " <> input }))
  assert drive(begin(table, "x")) == Error(Refused("refused x"))
  []
}

fn otherwise_nothing() -> List(String) {
  assert drive(begin(no_guard(NothingHappens), "x"))
    == Ok(Applied(Scratch(7, []), [], None))
  []
}

fn otherwise_defect() -> List(String) {
  assert drive(begin(no_guard(Defective), "x"))
    == Error(Defect("Scratch", NoRuleHeld))
  []
}

fn emitting(name: String) -> ScratchRow {
  Row(name, holds, fn(state, input) {
    Finish(note(state, name), [Echo(input), Echo(input <> "!")], Some(name))
  })
}

fn chained() -> List(String) {
  let table = Table("Scratch", [emitting("First")], NothingHappens)
  assert rules.drive(begin(table, "x"), fn() { 5 }, fn() { "unused" })
    == Ok(Applied(
      Scratch(7, ["x!@5", "x@5", "First"]),
      ["First", "EchoRow", "EchoRow"],
      Some("First"),
    ))
  []
}

fn timed(name: String) -> ScratchRow {
  Row(name, holds, fn(state, input) {
    use now <- Now
    Finish(note(state, name <> "@" <> int.to_string(now)), [Echo(input)], None)
  })
}

fn timed_once() -> List(String) {
  let clock = harness.start_clock()
  harness.set_time(clock, 40)
  let lease_ids = harness.start_lease_ids()
  let table =
    Table(
      "Scratch",
      [
        Row("Timed", holds, fn(state, _input) {
          use now <- Now
          Finish(note(state, int.to_string(now)), [], None)
        }),
      ],
      NothingHappens,
    )
  assert rules.drive(
      begin(table, "x"),
      harness.clock(clock),
      harness.lease_ids(lease_ids),
    )
    == Ok(Applied(Scratch(7, ["40"]), ["Timed"], None))
  assert harness.clock_reads(clock) == 1
  assert harness.lease_id_draws(lease_ids) == 0
  []
}

fn refusal_is_dry() -> List(String) {
  let clock = harness.start_clock()
  let lease_ids = harness.start_lease_ids()
  let table =
    Table(
      "Scratch",
      [Row("Never", never, timed("Never").effect)],
      Refuses(fn(_state, _input) { "no" }),
    )
  assert rules.drive(
      begin(table, "x"),
      harness.clock(clock),
      harness.lease_ids(lease_ids),
    )
    == Error(Refused("no"))
  assert harness.clock_reads(clock) == 0
  assert harness.lease_id_draws(lease_ids) == 0
  []
}

fn timed_twice() -> List(String) {
  let clock = harness.start_clock()
  harness.set_time(clock, 40)
  let table = Table("Scratch", [timed("First")], NothingHappens)
  assert rules.drive(begin(table, "x"), harness.clock(clock), fn() { "unused" })
    == Ok(Applied(Scratch(7, ["x@40", "First@40"]), ["First", "EchoRow"], None))
  assert harness.clock_reads(clock) == 1
  []
}

fn lease_id() -> List(String) {
  let lease_ids = harness.start_lease_ids()
  let granting =
    Row("Granting", fn(_state, input) { input == "grant" }, fn(state, _input) {
      use id <- LeaseId
      Finish(note(state, id), [], None)
    })
  let other =
    Row("Other", fn(_state, input) { input != "grant" }, plain("Other").effect)
  let table = Table("Scratch", [granting, other], NothingHappens)
  let run = fn(input) {
    rules.drive(begin(table, input), fn() { 0 }, harness.lease_ids(lease_ids))
  }
  assert run("other")
    == Ok(Applied(Scratch(7, ["Other:other"]), ["Other"], Some("Other")))
  assert harness.lease_id_draws(lease_ids) == 0
  assert run("grant")
    == Ok(Applied(Scratch(7, ["lease-1"]), ["Granting"], None))
  assert harness.lease_id_draws(lease_ids) == 1
  []
}

fn allots() -> List(String) {
  let row =
    Row("Allots", holds, fn(_state: Scratch, _input) {
      use first <- Position
      use second <- Position
      let entry = int.to_string(first) <> "," <> int.to_string(second)
      // The row writes a counter of its own, which must not survive.
      Finish(Scratch(next_position: 99, log: [entry]), [], None)
    })
  let table = Table("Scratch", [row], NothingHappens)
  assert drive(begin(table, "x"))
    == Ok(Applied(Scratch(9, ["7,8"]), ["Allots"], None))
  []
}

fn impossible() -> List(String) {
  let row =
    Row("First", holds, fn(_state: Scratch, input) {
      Impossible("no " <> input)
    })
  let table = Table("Scratch", [row], NothingHappens)
  assert drive(begin(table, "x"))
    == Error(Defect("Scratch", EffectFailed("First", "no x")))
  []
}

/// The row has read `now` and been allotted a position when it fails. A
/// defect is not a refusal, so the one clock read stands; the failure carries
/// no state, so the position it was allotted is not spent.
fn impossible_late() -> List(String) {
  let clock = harness.start_clock()
  let row =
    Row("Late", holds, fn(_state: Scratch, _input) {
      use _now <- Now
      use _position <- Position
      Impossible("gone")
    })
  let table = Table("Scratch", [row], NothingHappens)
  assert rules.drive(begin(table, "x"), harness.clock(clock), fn() { "unused" })
    == Error(Defect("Scratch", EffectFailed("Late", "gone")))
  assert harness.clock_reads(clock) == 1
  []
}

/// The first row changes the state and emits two events. The first event's
/// row fails, so the whole outcome does, naming the event's table and row.
/// The second event never runs: its row would have read the clock.
fn impossible_chained() -> List(String) {
  let clock = harness.start_clock()
  let row =
    Row("First", holds, fn(state, _input) {
      Finish(note(state, "First"), [Echo("bad"), Echo("after")], Some("First"))
    })
  let table = Table("Scratch", [row], NothingHappens)
  assert rules.drive(begin(table, "x"), harness.clock(clock), fn() { "unused" })
    == Error(Defect("Echo", EffectFailed("BadEcho", "bad")))
  assert harness.clock_reads(clock) == 0
  []
}
