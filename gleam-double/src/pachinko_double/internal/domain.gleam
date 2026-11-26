//// The rule-table machinery fixed to this domain: the state is the model's,
//// a refusal is one of `refusals`, and the events and answers are the
//// specification's. The rule modules write their tables in these types.

import pachinko_double/answers.{type Answer}
import pachinko_double/failure
import pachinko_double/ids.{type LeaseId, type PocketKey, type RecordKey}
import pachinko_double/internal/rules
import pachinko_double/model.{type State}
import pachinko_double/refusals.{type Refusal}
import pachinko_double/time.{type Timestamp}

/// The events a rule emits for another table to answer within the same
/// outcome.
pub type Event {
  /// A publish added or raised a record in this pocket.
  PocketTouched(pocket_key: PocketKey)
  /// A pull handed this record of this pocket out: one for each record of
  /// the batch, in the batch's order.
  Handed(pocket_key: PocketKey, record_key: RecordKey)
  /// A lease ended while this record of its pocket was held: one for each
  /// record of the held set. `lease` is the lease's id, and `ended_at` the
  /// time the rule that ended the lease wrote to its `ended_at`.
  Freed(lease: LeaseId, record_key: RecordKey, ended_at: Timestamp)
  /// The clock found this lease active with its `expires_at` passed. `lease`
  /// is the lease's id. The specification's `LeaseLapsed(lease)` carries no
  /// time; `now` is the instant of the outcome that emitted it, the `now`
  /// its `Tick` read, which the expiry rules' guards compare against.
  LeaseLapsed(lease: LeaseId, now: Timestamp)
}

pub type Plan =
  rules.Plan(State, Event, Answer)

pub type Row(input) =
  rules.Row(State, input, Event, Answer)

pub type Table(input) =
  rules.Table(State, input, Event, Answer, Refusal)

pub type Failure =
  failure.Failure(Refusal)

pub type Applied =
  rules.Applied(State, Answer)

pub type Step =
  rules.Step(State, Answer, Refusal)

pub type Cursor =
  rules.Cursor(State)

pub type World =
  rules.World(State, Event, Answer, Refusal)
