//// The inspection seam: one complete, immutable snapshot of the queue's
//// state, which the tests and the invariant checker read and compare with
//// `==`. Every collection is a list in a stated order: the order its
//// members were created in, with a changed member keeping its place. For
//// the publishers and the consumers that is the order they joined in (spec
//// note G06); for the pockets, the records, the leases and the two shed
//// queues it is spec note G19. No rule reads that order: the pocket order
//// and a pocket's record order are the positions'.
////
//// The specification's derived sets, such as a pocket's `held_records`, are
//// computed here, once per snapshot, by `views`: one view for each pocket
//// and each consumer, holding that entity's sets under the specification's
//// names. Each set is built with `model`'s functions over the snapshot's
//// lists, so a rule reading the state and a reader of a snapshot share one
//// definition. Nothing derived is stored in the `Snapshot` itself: it holds
//// the nine plain fields, so a snapshot built by hand cannot hold a set
//// that disagrees with its lists.

import gleam/list
import pachinko_double/config.{type Settings}
import pachinko_double/model.{
  type Consumer, type DeadLetter, type Lease, type Pocket, type Publisher,
  type Record, type State, type SupersededRecord,
}

/// The state at one moment. `config` is the plain values, so that a snapshot
/// built by hand can hold a configuration `config.new` would refuse.
pub type Snapshot {
  Snapshot(
    config: Settings,
    next_position: Int,
    publishers: List(Publisher),
    consumers: List(Consumer),
    pockets: List(Pocket),
    records: List(Record),
    leases: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}

/// The snapshot of a new queue over these settings: nothing joined, nothing
/// stored, the first position 0.
pub fn empty(settings: Settings) -> Snapshot {
  Snapshot(
    config: settings,
    next_position: 0,
    publishers: [],
    consumers: [],
    pockets: [],
    records: [],
    leases: [],
    superseded: [],
    dead_letters: [],
  )
}

pub fn snapshot(state: State) -> Snapshot {
  Snapshot(
    config: config.settings(state.config),
    next_position: state.queue.next_position,
    publishers: state.publishers,
    consumers: state.consumers,
    pockets: state.pockets,
    records: state.records,
    leases: state.leases,
    superseded: state.superseded,
    dead_letters: state.dead_letters,
  )
}

/// One pocket with the specification's derived sets of `Pocket`, each in
/// the snapshot's order.
pub type PocketView {
  PocketView(
    pocket: Pocket,
    /// `Record with pocket = this`.
    records: List(Record),
    /// `records where status = queued`.
    queued_records: List(Record),
    /// `records where status = held`.
    held_records: List(Record),
    /// `records where status = held and mark != unsettled and
    /// deferred_position = null`.
    ending_records: List(Record),
    /// `Lease with pocket = this`.
    leases: List(Lease),
    /// `leases where status = active`.
    active_leases: List(Lease),
  )
}

/// One consumer with the specification's derived sets of `Consumer`, each
/// in the snapshot's order.
pub type ConsumerView {
  ConsumerView(
    consumer: Consumer,
    /// `Lease with consumer = this`.
    leases: List(Lease),
    /// `leases where status = active`.
    active_leases: List(Lease),
  )
}

/// Every derived set of one snapshot: a view for each of its pockets and
/// each of its consumers, in the snapshot's order.
pub type Views {
  Views(pockets: List(PocketView), consumers: List(ConsumerView))
}

/// The derived sets of a snapshot, computed once. A reference is a key, so
/// an entity's set is every member whose reference holds that entity's key.
/// In a snapshot built by hand, where several pockets may hold one key or
/// several consumers one id, each of them has its own view and each view
/// holds every such member; a member whose reference names no entity is in
/// no view.
pub fn views(snapshot: Snapshot) -> Views {
  let pockets =
    list.map(snapshot.pockets, fn(pocket) {
      let records = model.records_of(snapshot.records, pocket.key)
      let leases = model.leases_of_pocket(snapshot.leases, pocket.key)
      PocketView(
        pocket:,
        records:,
        queued_records: model.queued(records),
        held_records: model.held(records),
        ending_records: model.ending(records),
        leases:,
        active_leases: model.active(leases),
      )
    })
  let consumers =
    list.map(snapshot.consumers, fn(consumer) {
      let leases = model.leases_of_consumer(snapshot.leases, consumer.id)
      ConsumerView(consumer:, leases:, active_leases: model.active(leases))
    })
  Views(pockets:, consumers:)
}
