//// The rule-table machinery fixed to this domain: the state is the model's,
//// a refusal is one of `refusals`, and the events and answers are the
//// specification's. The rule modules write their tables in these types.

import pachinko_double/model.{type State}
import pachinko_double/refusals.{type Refusal}
import pachinko_double/rules

/// The events a rule emits for another table to answer within the same
/// outcome. None exists yet: the first, `PocketTouched`, lands with Publish.
pub type Event

/// What a rule answers its caller. No rule landed so far answers anything.
pub type Answer

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
