//// The inspection seam: one complete, immutable snapshot of the queue's
//// state, which the tests read and compare with `==`. Every collection is a
//// list in a stated order.

import pachinko_double/model.{type Consumer, type Publisher, type State}

/// The state at one moment. Publishers and consumers are in the order they
/// joined.
pub type Snapshot {
  Snapshot(
    next_position: Int,
    publishers: List(Publisher),
    consumers: List(Consumer),
  )
}

pub fn snapshot(state: State) -> Snapshot {
  Snapshot(
    next_position: state.queue.next_position,
    publishers: state.publishers,
    consumers: state.consumers,
  )
}
