//// The specification's entities, and the queue's whole state as one value.
//// This ticket lands `Queue`, `Publisher` and `Consumer`.

import pachinko_double/config.{type Config}

/// The deployment's one queue. `next_position` is the position the next
/// pocket or record to enter an order takes. Only the apply step moves it.
pub type Queue {
  Queue(next_position: Int)
}

pub type Publisher {
  Publisher(id: String)
}

pub type Consumer {
  Consumer(id: String)
}

/// Everything the owner holds. Each collection is in the order its members
/// were created.
pub type State {
  State(
    config: Config,
    queue: Queue,
    publishers: List(Publisher),
    consumers: List(Consumer),
  )
}

/// A new queue: nothing has joined, and the first position is 0 (spec note
/// G04).
pub fn new(config: Config) -> State {
  State(config:, queue: Queue(next_position: 0), publishers: [], consumers: [])
}
