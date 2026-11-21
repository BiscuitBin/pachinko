//// The one apply step. It runs a trigger's table, then the events that
//// outcome emits, each through its own table, and returns the names of the
//// rules that fired. It is pure: where an outcome needs `now` or a lease id
//// it returns a step that asks for one, and the owner supplies it.

import pachinko_double/domain.{type Cursor, type Event, type Step, type World}
import pachinko_double/identity
import pachinko_double/model.{type State, Queue, State}
import pachinko_double/rules.{World}

/// The triggers a surface provides, one variant each.
pub type Trigger {
  PublisherJoins(publisher_id: String)
  ConsumerJoins(consumer_id: String)
}

/// Applies one trigger to a state. The state it was given is untouched
/// whatever the result: a failure simply carries no new state.
pub fn apply(state: State, trigger: Trigger) -> Step {
  let cursor = rules.start(state)
  case trigger {
    PublisherJoins(publisher_id) ->
      rules.run(
        identity.publisher_joins(),
        publisher_id,
        cursor,
        world(),
        rules.finish,
      )
    ConsumerJoins(consumer_id) ->
      rules.run(
        identity.consumer_joins(),
        consumer_id,
        cursor,
        world(),
        rules.finish,
      )
  }
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

/// Sends an emitted event to its table. No event exists yet.
fn chain(_event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
  then(cursor)
}
