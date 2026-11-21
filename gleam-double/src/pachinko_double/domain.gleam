//// The rule-table machinery fixed to this domain: the state is the model's,
//// a refusal is one of `refusals`, and the events and answers are the
//// specification's. The rule modules write their tables in these types.

import pachinko_double/model.{type State}
import pachinko_double/refusals.{type Refusal}
import pachinko_double/rules

/// The events a rule emits for another table to answer within the same
/// outcome.
pub type Event {
  /// A publish added or raised a record in this pocket.
  PocketTouched(pocket_key: String)
  /// A pull handed this record of this pocket out: one for each record of
  /// the batch, in the batch's order.
  Handed(pocket_key: String, record_key: String)
}

/// What a rule answers its caller. A publish, a join and a renew answer
/// nothing; a pull answers one of these.
pub type Answer {
  /// What a pull hands back. `consumer` is the consumer's id, `lease` the id
  /// of the lease the records were handed under and `pocket` the pocket's
  /// key. It carries no expiry: the specification's `Batch` has none.
  /// `records` is the batch, smallest position first.
  Batch(
    consumer: String,
    lease: String,
    pocket: String,
    records: List(BatchRecord),
  )
  /// The consumer holds no lease and no pocket is queued. An answer, not a
  /// refusal.
  NoPocketAvailable(consumer: String)
}

/// One version a pull handed out: the record's key, and the rank and payload
/// it had when the batch was computed.
pub type BatchRecord {
  BatchRecord(key: String, rank: Int, payload: BitArray)
}

pub type Plan =
  rules.Plan(State, Event, Answer)

pub type Row(input) =
  rules.Row(State, input, Event, Answer)

pub type Table(input) =
  rules.Table(State, input, Event, Answer, Refusal)

pub type Failure =
  rules.Failure(Refusal)

pub type Applied =
  rules.Applied(State, Answer)

pub type Step =
  rules.Step(State, Answer, Refusal)

pub type Cursor =
  rules.Cursor(State)

pub type World =
  rules.World(State, Event, Answer, Refusal)
