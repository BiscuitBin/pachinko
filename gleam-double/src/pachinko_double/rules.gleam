//// The rule-table machinery. It knows nothing of the domain: a table is the
//// rules that share one trigger, each row `(name, guard, effect)`, with what
//// the table answers when no guard holds. `core` fixes it to the domain's
//// state, events, answers and refusals.
////
//// A guard is a pure function of the state and the trigger's input. It
//// cannot read the clock, because nothing hands it one. An effect returns a
//// `Plan`: it asks for `now`, a lease id or a position only by returning a
//// continuation, and `run` decides how each is supplied. `now` and a lease
//// id come from outside, so `run` returns a `Step` that asks for them, and
//// whoever drives the `Step` injects them.

import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/time.{type Timestamp}

/// What a row's effect does, as a program `run` interprets.
pub type Plan(state, event, answer) {
  /// The outcome of the row: the state after it, the events it emits, in
  /// order, and what it answers its caller, if anything.
  Finish(state: state, events: List(event), answer: Option(answer))
  /// The row reads `now`. One outcome reads the clock once, however many of
  /// its rows ask.
  Now(next: fn(Timestamp) -> Plan(state, event, answer))
  /// The row creates a lease and needs an id for it.
  LeaseId(next: fn(String) -> Plan(state, event, answer))
  /// The row allots a position: it is handed the counter's value, and the
  /// counter advances. The state the row finishes with has its counter
  /// overwritten, so only `run` ever moves it.
  Position(next: fn(Int) -> Plan(state, event, answer))
  /// The row did not find what its guard promised: a lease, a record or a
  /// pocket the guard saw is not there. Only a mistake in the double's own
  /// rules gets here, so the outcome ends as a `Defect`, and nothing it did
  /// so far is kept.
  Impossible(reason: String)
}

/// One rule: its name in the specification, its guard and its effect.
pub type Row(state, input, event, answer) {
  Row(
    name: String,
    guard: fn(state, input) -> Bool,
    effect: fn(state, input) -> Plan(state, event, answer),
  )
}

/// What a table answers when no guard holds.
pub type Otherwise(state, input, refusal) {
  /// The action is refused, and the function says with what.
  Refuses(refusal: fn(state, input) -> refusal)
  /// Nothing happens, and that is not an error.
  NothingHappens
  /// The trigger was sent only where one of its rules should have held.
  Defective
}

/// The rules that share one trigger.
pub type Table(state, input, event, answer, refusal) {
  Table(
    trigger: String,
    rows: List(Row(state, input, event, answer)),
    otherwise: Otherwise(state, input, refusal),
  )
}

/// Every way an outcome fails. Each is a value: nothing here panics.
pub type Failure(refusal) {
  /// The action was refused and changed nothing.
  Refused(refusal: refusal)
  /// More than one guard of a table held, which the specification says
  /// cannot happen: its guards are disjoint.
  GuardOverlap(trigger: String, rules: List(String))
  /// The double's own rules are wrong, and `cause` says how. `trigger` is the
  /// table the outcome had reached.
  Defect(trigger: String, cause: Cause)
  /// The outcome left the state breaking these invariants.
  BrokenInvariant(invariants: List(String))
}

/// Why an outcome is a defect.
pub type Cause {
  /// The trigger reached a `Defective` table, and none of its rules held.
  NoRuleHeld
  /// The effect of `rule` did not find what its guard promised.
  EffectFailed(rule: String, reason: String)
}

/// A finished outcome: the state after it, the names of the rules that
/// fired, in order, and the answer of the rule the trigger reached.
pub type Applied(state, answer) {
  Applied(state: state, fired: List(String), answer: Option(answer))
}

/// An outcome in progress. It is done, or it waits for a value only the
/// outside can give.
pub type Step(state, answer, refusal) {
  Done(result: Result(Applied(state, answer), Failure(refusal)))
  NeedsNow(next: fn(Timestamp) -> Step(state, answer, refusal))
  NeedsLeaseId(next: fn(String) -> Step(state, answer, refusal))
}

/// Where one outcome stands: the state so far, the `now` it read, if it has
/// read one, and the rules that have fired.
pub opaque type Cursor(state) {
  Cursor(state: state, now: Option(Timestamp), fired: List(String))
}

/// What `run` needs from the domain: the position counter, and where an
/// emitted event goes. `chain` runs the event's table and then calls the
/// continuation it is given.
pub type World(state, event, answer, refusal) {
  World(
    next_position: fn(state) -> Int,
    with_next_position: fn(state, Int) -> state,
    chain: fn(
      event,
      Cursor(state),
      fn(Cursor(state)) -> Step(state, answer, refusal),
    ) -> Step(state, answer, refusal),
  )
}

/// The start of an outcome: nothing has fired and no clock has been read.
pub fn start(state: state) -> Cursor(state) {
  Cursor(state:, now: None, fired: [])
}

/// The state an outcome has reached.
pub fn state(cursor: Cursor(state)) -> state {
  cursor.state
}

/// The same outcome, already holding the `now` its trigger carries.
pub fn at(cursor: Cursor(state), now: Timestamp) -> Cursor(state) {
  Cursor(..cursor, now: Some(now))
}

/// The end of an outcome that succeeded.
pub fn finish(
  cursor: Cursor(state),
  answer: Option(answer),
) -> Step(state, answer, refusal) {
  Done(Ok(Applied(cursor.state, list.reverse(cursor.fired), answer)))
}

/// The end of an outcome that failed.
pub fn fail(failure: Failure(refusal)) -> Step(state, answer, refusal) {
  Done(Error(failure))
}

/// The names of a table's rules, in the table's order.
pub fn names(
  table: Table(state, input, event, answer, refusal),
) -> List(String) {
  list.map(table.rows, fn(row) { row.name })
}

/// Finds the one row whose guard holds. It evaluates every guard, never
/// stopping at the first that holds, so that two holding at once is seen.
pub fn dispatch(
  table: Table(state, input, event, answer, refusal),
  state: state,
  input: input,
) -> Result(Option(Row(state, input, event, answer)), Failure(refusal)) {
  case list.filter(table.rows, fn(row) { row.guard(state, input) }) {
    [] -> Ok(None)
    [row] -> Ok(Some(row))
    rows ->
      Error(GuardOverlap(table.trigger, list.map(rows, fn(row) { row.name })))
  }
}

/// Runs one trigger's table within an outcome: the row whose guard holds,
/// then each event it emits through `world.chain`, in order, then `then`,
/// with the row's answer. When no guard holds the table's own answer
/// applies: a refusal or a defect ends the outcome, and "nothing happens"
/// goes on with no answer. A row that finds the impossible ends the outcome
/// as a defect too, wherever in the chain it sits.
pub fn run(
  table: Table(state, input, event, answer, refusal),
  input: input,
  cursor: Cursor(state),
  world: World(state, event, answer, refusal),
  then: fn(Cursor(state), Option(answer)) -> Step(state, answer, refusal),
) -> Step(state, answer, refusal) {
  case dispatch(table, cursor.state, input) {
    Error(failure) -> fail(failure)
    Ok(None) ->
      case table.otherwise {
        Refuses(refusal) -> fail(Refused(refusal(cursor.state, input)))
        NothingHappens -> then(cursor, None)
        Defective -> fail(Defect(table.trigger, NoRuleHeld))
      }
    Ok(Some(row)) ->
      follow(
        row.effect(cursor.state, input),
        Cursor(..cursor, fired: [row.name, ..cursor.fired]),
        world.next_position(cursor.state),
        world,
        fn(reason) { Defect(table.trigger, EffectFailed(row.name, reason)) },
        then,
      )
  }
}

/// Interprets one row's plan. `next_position` is the counter as this row has
/// advanced it so far, and `defect` is the failure to end with if the row
/// finds the impossible.
fn follow(
  plan: Plan(state, event, answer),
  cursor: Cursor(state),
  next_position: Int,
  world: World(state, event, answer, refusal),
  defect: fn(String) -> Failure(refusal),
  then: fn(Cursor(state), Option(answer)) -> Step(state, answer, refusal),
) -> Step(state, answer, refusal) {
  case plan {
    Now(next) ->
      case cursor.now {
        Some(now) ->
          follow(next(now), cursor, next_position, world, defect, then)
        None ->
          NeedsNow(fn(now) {
            follow(
              next(now),
              at(cursor, now),
              next_position,
              world,
              defect,
              then,
            )
          })
      }
    LeaseId(next) ->
      NeedsLeaseId(fn(lease_id) {
        follow(next(lease_id), cursor, next_position, world, defect, then)
      })
    Position(next) ->
      follow(
        next(next_position),
        cursor,
        next_position + 1,
        world,
        defect,
        then,
      )
    Finish(state:, events:, answer:) -> {
      let state = world.with_next_position(state, next_position)
      emit(events, Cursor(..cursor, state:), world, then(_, answer))
    }
    Impossible(reason) -> fail(defect(reason))
  }
}

/// Runs the events one row emitted, in order, each to its end before the
/// next begins.
fn emit(
  events: List(event),
  cursor: Cursor(state),
  world: World(state, event, answer, refusal),
  then: fn(Cursor(state)) -> Step(state, answer, refusal),
) -> Step(state, answer, refusal) {
  case events {
    [] -> then(cursor)
    [event, ..rest] ->
      world.chain(event, cursor, fn(cursor) { emit(rest, cursor, world, then) })
  }
}

/// Drives a step to its end with the two injected functions: the clock, read
/// each time the step asks for `now`, and the lease-id source, drawn from
/// each time it asks for an id.
pub fn drive(
  step: Step(state, answer, refusal),
  now: fn() -> Timestamp,
  lease_id: fn() -> String,
) -> Result(Applied(state, answer), Failure(refusal)) {
  case step {
    Done(result) -> result
    NeedsNow(next) -> drive(next(now()), now, lease_id)
    NeedsLeaseId(next) -> drive(next(lease_id()), now, lease_id)
  }
}
