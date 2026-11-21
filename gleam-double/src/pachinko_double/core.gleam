//// The one apply step. It runs a trigger's table, then the events that
//// outcome emits, each through its own table, then the invariant checker,
//// once, over the state the whole chained outcome left. It returns the names
//// of the rules that fired. It is pure: where an outcome needs `now` or a
//// lease id it returns a step that asks for one, and the owner supplies it.

import gleam/list
import gleam/option.{type Option}
import pachinko_double/domain.{
  type Answer, type Cursor, type Event, type Step, type Table, type World,
  PocketTouched,
}
import pachinko_double/identity
import pachinko_double/inspection
import pachinko_double/invariants
import pachinko_double/model.{type State, Publisher, Queue, State}
import pachinko_double/publish.{type Publication}
import pachinko_double/refusals.{UnknownPublisher}
import pachinko_double/rules.{BrokenInvariant, Refused, World}

/// The triggers a surface provides, one variant each.
pub type Trigger {
  PublisherJoins(publisher_id: String)
  ConsumerJoins(consumer_id: String)
  Publish(publication: Publication)
}

/// Applies one trigger to a state. The state it was given is untouched
/// whatever the result: a failure simply carries no new state.
pub fn apply(state: State, trigger: Trigger) -> Step {
  case trigger {
    PublisherJoins(publisher_id) ->
      run(identity.publisher_joins(), publisher_id, state)
    ConsumerJoins(consumer_id) ->
      run(identity.consumer_joins(), consumer_id, state)
    Publish(publication) ->
      // A publisher that never joined cannot publish: the trigger takes a
      // `Publisher`, and only a join makes one. It is refused before any
      // rule runs, so no table is consulted (spec note G12).
      case list.contains(state.publishers, Publisher(publication.publisher)) {
        True -> run(publish.publish(), publication, state)
        False -> rules.fail(Refused(UnknownPublisher(publication.publisher)))
      }
  }
}

/// Runs one table as a whole outcome over a state: the row whose guard
/// holds, the events it chains, then the checker. `apply` sends every
/// trigger through here, and a test can send a table of its own.
pub fn run(table: Table(input), input: input, state: State) -> Step {
  rules.run(table, input, rules.start(state), world(), conclude)
}

/// The end of every outcome: the checker reads the state the whole chain
/// left, never a state part-way through it. An outcome that leaves an
/// invariant broken fails, naming each one, and so leaves no new state.
fn conclude(cursor: Cursor, answer: Option(Answer)) -> Step {
  case invariants.check(inspection.snapshot(rules.state(cursor))) {
    [] -> rules.finish(cursor, answer)
    broken -> rules.fail(BrokenInvariant(broken))
  }
}

/// Every table, as its trigger's name and its rules' names. The coverage
/// check reads the rules from here, so a rule is registered only by being a
/// row of a table the apply step can reach.
pub fn registry() -> List(#(String, List(String))) {
  [
    entry(identity.publisher_joins()),
    entry(identity.consumer_joins()),
    entry(publish.publish()),
    entry(publish.pocket_touched()),
  ]
}

fn entry(table: Table(input)) -> #(String, List(String)) {
  #(table.trigger, rules.names(table))
}

/// What the machinery needs from the domain. A position is allotted in one
/// place, the interpreter in `rules`, which reads and moves the queue's
/// `next_position` through the two functions here and overwrites whatever
/// counter a row finishes with. Nothing else reads or moves it for a rule.
fn world() -> World {
  World(
    next_position: fn(state: State) { state.queue.next_position },
    with_next_position: fn(state, next_position) {
      State(..state, queue: Queue(next_position:))
    },
    chain:,
  )
}

/// Sends an emitted event to its table.
fn chain(event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
  case event {
    PocketTouched(pocket_key) ->
      rules.run(
        publish.pocket_touched(),
        pocket_key,
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
  }
}
