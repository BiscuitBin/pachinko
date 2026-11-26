//// The specification's Identity section: the two joins, one table each.

import gleam/list
import gleam/option.{None}
import pachinko_double/ids.{type ConsumerId, type PublisherId}
import pachinko_double/internal/domain.{type Table}
import pachinko_double/internal/rules.{Finish, Refuses, Row, Table}
import pachinko_double/model.{type State, Consumer, Publisher, State}
import pachinko_double/refusals.{ConsumerAlreadyJoined, PublisherAlreadyJoined}

/// `PublisherJoins(publisher_id)`. A join with an id already taken is
/// refused (spec note G03).
pub fn publisher_joins() -> Table(PublisherId) {
  Table(
    "PublisherJoins",
    [
      Row(
        "PublisherJoins",
        fn(state: State, publisher_id) {
          !list.contains(state.publishers, Publisher(id: publisher_id))
        },
        fn(state, publisher_id) {
          let publishers =
            list.append(state.publishers, [Publisher(id: publisher_id)])
          Finish(State(..state, publishers:), [], None)
        },
      ),
    ],
    Refuses(fn(_state, publisher_id) { PublisherAlreadyJoined(publisher_id) }),
  )
}

/// `ConsumerJoins(consumer_id)`. A join with an id already taken is refused
/// (spec note G03).
pub fn consumer_joins() -> Table(ConsumerId) {
  Table(
    "ConsumerJoins",
    [
      Row(
        "ConsumerJoins",
        fn(state: State, consumer_id) {
          !list.contains(state.consumers, Consumer(id: consumer_id))
        },
        fn(state, consumer_id) {
          let consumers =
            list.append(state.consumers, [Consumer(id: consumer_id)])
          Finish(State(..state, consumers:), [], None)
        },
      ),
    ],
    Refuses(fn(_state, consumer_id) { ConsumerAlreadyJoined(consumer_id) }),
  )
}
