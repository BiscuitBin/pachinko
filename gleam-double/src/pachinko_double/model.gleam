//// The specification's entities, and the queue's whole state as one value.
//// Every field the specification declares is here, under its name. A
//// reference to another entity is that entity's key (spec note G09).

import gleam/int
import gleam/list
import gleam/option.{type Option}
import gleam/result
import pachinko_double/config.{type Config}
import pachinko_double/time.{type Timestamp}

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

pub type PocketStatus {
  PocketQueued
  PocketLeased
  PocketIdle
  PocketParked
}

/// One entry of the queue, named by its key. Queued pockets are ordered by
/// `position`, smallest first.
pub type Pocket {
  Pocket(
    key: String,
    status: PocketStatus,
    position: Int,
    consecutive_expiries: Int,
  )
}

/// What a consumer has said of a held record.
pub type Mark {
  Unsettled
  Acked
  Nacked
}

/// A record's status. The five fields the specification declares `when
/// status = held` are fields of the held variant alone, so a queued record
/// cannot carry one.
pub type RecordStatus {
  RecordQueued
  RecordHeld(
    held_rank: Int,
    held_payload: BitArray,
    mark: Mark,
    nacked_at: Option(Timestamp),
    deferred_position: Option(Int),
  )
}

/// The one slot a record key has in a pocket. `pocket` is the pocket's key.
pub type Record {
  Record(
    pocket: String,
    key: String,
    rank: Int,
    payload: BitArray,
    position: Int,
    status: RecordStatus,
  )
}

pub type Reason {
  Superseded
  Stale
}

/// A version no consumer acked or nacked that lost a supersede or arrived
/// stale, kept until it decays. `pocket` is the pocket's key.
pub type SupersededRecord {
  SupersededRecord(
    pocket: String,
    key: String,
    rank: Int,
    payload: BitArray,
    reason: Reason,
    shed_at: Timestamp,
    expires_at: Timestamp,
  )
}

/// A nacked version, kept until it decays. `pocket` is the pocket's key and
/// `nacked_by` the consumer's id.
pub type DeadLetter {
  DeadLetter(
    pocket: String,
    key: String,
    rank: Int,
    payload: BitArray,
    nacked_by: String,
    nacked_at: Timestamp,
    expires_at: Timestamp,
  )
}

/// A lease's status. `ended_at`, which the specification declares `when
/// status = released | expired`, is a field of those two variants alone.
pub type LeaseStatus {
  LeaseActive
  LeaseReleased(ended_at: Timestamp)
  LeaseExpired(ended_at: Timestamp)
}

/// A consumer's claim on one pocket. `id` is the double's: the
/// specification declares none (spec note G10). `pocket` is the pocket's key
/// and `consumer` the consumer's id.
pub type Lease {
  Lease(
    id: String,
    pocket: String,
    consumer: String,
    status: LeaseStatus,
    granted_at: Timestamp,
    expires_at: Timestamp,
  )
}

/// Everything the owner holds. Each collection is in the order its members
/// were created, and a member that is changed keeps its place.
pub type State {
  State(
    config: Config,
    queue: Queue,
    publishers: List(Publisher),
    consumers: List(Consumer),
    pockets: List(Pocket),
    records: List(Record),
    leases: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}

/// A new queue: nothing has joined, nothing is stored, and the first
/// position is 0 (spec note G04).
pub fn new(config: Config) -> State {
  State(
    config:,
    queue: Queue(next_position: 0),
    publishers: [],
    consumers: [],
    pockets: [],
    records: [],
    leases: [],
    superseded: [],
    dead_letters: [],
  )
}

// The specification's lookups and derived sets, each over the plain lists, so
// that a rule reading the state and the checker reading a snapshot share one
// definition.

/// `Pocket{key: key}`.
pub fn pocket(pockets: List(Pocket), key: String) -> Result(Pocket, Nil) {
  list.find(pockets, fn(pocket) { pocket.key == key })
}

/// `Record{pocket: pocket, key: key}`.
pub fn record(
  records: List(Record),
  pocket pocket_key: String,
  key key: String,
) -> Result(Record, Nil) {
  list.find(records, fn(record) {
    record.pocket == pocket_key && record.key == key
  })
}

/// A held record as a rule reads it: the lease it is held under, for
/// `lease.pocket` and `lease.consumer`, the record, and the five fields only
/// a held record has.
pub type HeldRecord {
  HeldRecord(
    lease: Lease,
    record: Record,
    held_rank: Int,
    held_payload: BitArray,
    mark: Mark,
    nacked_at: Option(Timestamp),
    deferred_position: Option(Int),
  )
}

/// `Record{pocket: lease.pocket, key: key}`, where it `exists` and
/// `record.status = held`. The lease is the caller's, whatever its status.
pub fn held_under(
  records: List(Record),
  lease: Lease,
  key: String,
) -> Result(HeldRecord, Nil) {
  use record <- result.try(record(records, pocket: lease.pocket, key:))
  case record.status {
    RecordQueued -> Error(Nil)
    RecordHeld(held_rank:, held_payload:, mark:, nacked_at:, deferred_position:) ->
      Ok(HeldRecord(
        lease:,
        record:,
        held_rank:,
        held_payload:,
        mark:,
        nacked_at:,
        deferred_position:,
      ))
  }
}

/// The records with `record` in the place of the one it replaces, the
/// record of its pocket and key. No other record changes and none moves: a
/// changed entity keeps its place in its list.
pub fn replace_record(records: List(Record), record: Record) -> List(Record) {
  list.map(records, fn(stored) {
    case stored.pocket == record.pocket && stored.key == record.key {
      True -> record
      False -> stored
    }
  })
}

/// The pockets with `pocket` in the place of the one it replaces, the pocket
/// of its key. No other pocket changes and none moves.
pub fn replace_pocket(pockets: List(Pocket), pocket: Pocket) -> List(Pocket) {
  list.map(pockets, fn(stored) {
    case stored.key == pocket.key {
      True -> pocket
      False -> stored
    }
  })
}

/// The lease with this id. The id is the double's, and it is how an action
/// names its lease (spec note G10).
pub fn lease(leases: List(Lease), id: String) -> Result(Lease, Nil) {
  list.find(leases, fn(lease) { lease.id == id })
}

/// The leases with `lease` in the place of the one it replaces, the lease of
/// its id. No other lease changes and none moves.
pub fn replace_lease(leases: List(Lease), lease: Lease) -> List(Lease) {
  list.map(leases, fn(stored) {
    case stored.id == lease.id {
      True -> lease
      False -> stored
    }
  })
}

/// A pocket's `records`: `Record with pocket = this`.
pub fn records_of(records: List(Record), pocket_key: String) -> List(Record) {
  list.filter(records, fn(record) { record.pocket == pocket_key })
}

/// `records where status = queued`.
pub fn queued(records: List(Record)) -> List(Record) {
  list.filter(records, fn(record) { record.status == RecordQueued })
}

/// `records where status = held`.
pub fn held(records: List(Record)) -> List(Record) {
  list.filter(records, fn(record) { record.status != RecordQueued })
}

/// The records smallest position first: the order a pull hands them out in,
/// and the order a lease's end frees them in.
pub fn oldest_first(records: List(Record)) -> List(Record) {
  list.sort(records, fn(a, b) { int.compare(a.position, b.position) })
}

/// A pocket's `ending_records`: `records where status = held and mark !=
/// unsettled and deferred_position = null`.
pub fn ending(records: List(Record)) -> List(Record) {
  list.filter(records, fn(record) {
    case record.status {
      RecordQueued -> False
      RecordHeld(mark:, deferred_position:, ..) ->
        mark != Unsettled && deferred_position == option.None
    }
  })
}

/// `leases where status = active`.
pub fn active(leases: List(Lease)) -> List(Lease) {
  list.filter(leases, fn(lease) { lease.status == LeaseActive })
}

/// A pocket's `leases`: `Lease with pocket = this`.
pub fn leases_of_pocket(
  leases: List(Lease),
  pocket_key: String,
) -> List(Lease) {
  list.filter(leases, fn(lease) { lease.pocket == pocket_key })
}

/// A consumer's `leases`: `Lease with consumer = this`.
pub fn leases_of_consumer(
  leases: List(Lease),
  consumer_id: String,
) -> List(Lease) {
  list.filter(leases, fn(lease) { lease.consumer == consumer_id })
}
