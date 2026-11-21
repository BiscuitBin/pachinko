//// The one apply step. It runs a trigger's table, then the events that
//// outcome emits, each through its own table, then the invariant checker,
//// once, over the state the whole chained outcome left. It returns the names
//// of the rules that fired. It is pure: where an outcome needs `now` or a
//// lease id it returns a step that asks for one, and the owner supplies it.

import gleam/list
import gleam/option.{type Option}
import pachinko_double/consuming.{type LeaseView}
import pachinko_double/domain.{
  type Answer, type Cursor, type Event, type Failure, type Step, type Table,
  type World, Freed, Handed, PocketTouched,
}
import pachinko_double/identity
import pachinko_double/inspection
import pachinko_double/invariants
import pachinko_double/lease_end.{Freeing}
import pachinko_double/model.{type State, Consumer, Publisher, Queue, State}
import pachinko_double/publish.{type Publication}
import pachinko_double/pull.{type Pull, Handover}
import pachinko_double/refusals.{UnknownConsumer, UnknownPublisher}
import pachinko_double/rules.{BrokenInvariant, Refused, World}
import pachinko_double/settle.{type Ack, type Nack, type Release, type Renew}

/// The triggers a surface provides, one variant each.
pub type Trigger {
  PublisherJoins(publisher_id: String)
  ConsumerJoins(consumer_id: String)
  Publish(publication: Publication)
  Pull(pull: Pull)
  Ack(ack: Ack)
  Nack(nack: Nack)
  Renew(renew: Renew)
  Release(release: Release)
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
    // A consumer that never joined can neither pull, ack, nack, renew nor
    // release: each trigger takes a `Consumer`, and only a join makes one.
    // Each is refused before any rule runs, as the unknown publisher is.
    Pull(request) ->
      case joined(state, request.consumer) {
        True -> run(pull.pull(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Renew(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.renew(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Ack(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.ack(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Nack(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.nack(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Release(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.release(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
  }
}

/// The `Consuming` surface's lease view for this consumer id: a read, not a
/// trigger. It goes through no table, asks for no time and returns no state.
/// The surface faces a `Consumer`, so an id that never joined is refused, as
/// it is for every trigger the surface provides.
pub fn lease_view(
  state: State,
  consumer_id: String,
) -> Result(Option(LeaseView), Failure) {
  case joined(state, consumer_id) {
    True -> Ok(consuming.lease_view(state, consumer_id))
    False -> Error(Refused(UnknownConsumer(consumer_id)))
  }
}

/// Whether a consumer with this id has joined.
fn joined(state: State, consumer_id: String) -> Bool {
  list.contains(state.consumers, Consumer(consumer_id))
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
    entry(pull.pull()),
    entry(pull.handed()),
    entry(settle.renew()),
    entry(settle.ack()),
    entry(settle.nack()),
    entry(settle.release()),
    entry(lease_end.freed()),
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
    Handed(pocket_key, record_key) ->
      rules.run(
        pull.handed(),
        Handover(pocket_key:, record_key:),
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
    Freed(lease, record_key, ended_at) ->
      rules.run(
        lease_end.freed(),
        Freeing(lease:, record_key:, ended_at:),
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
  }
}
