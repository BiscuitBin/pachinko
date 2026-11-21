//// The owner: the one process that holds the queue's state. Every trigger is
//// one message in its mailbox, applied one at a time through the pure core,
//// so the mailbox is the specification's "one serialized stream". The clock
//// and the lease-id source are functions it is started with.

import gleam/erlang/process.{type Subject}
import gleam/option.{type Option}
import gleam/otp/actor
import pachinko_double/config.{type Config}
import pachinko_double/core.{type Trigger}
import pachinko_double/domain.{type Answer, type Failure}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/model.{type State}
import pachinko_double/publish.{type Publication}
import pachinko_double/rules
import pachinko_double/time.{type Timestamp}

/// How long a caller waits for the owner's reply, in milliseconds. A caller
/// that gets no reply in that time, or whose owner has died, crashes: the
/// call does not return a failure. In a test that fails the test, which is
/// the intent. The trigger it sent may still be applied afterwards.
pub const call_timeout = 1000

/// What a trigger that succeeded reports: the names of the rules that
/// fired, in order, and the answer of the rule the trigger reached.
pub type Outcome {
  Outcome(fired: List(String), answer: Option(Answer))
}

pub type Reply =
  Result(Outcome, Failure)

/// The owner's mailbox. `Act` carries any one trigger a surface provides,
/// so each trigger sent is one message and `core.Trigger` holds the variant
/// per trigger; `Inspect` is the other message.
pub opaque type Message {
  Act(trigger: Trigger, reply: Subject(Reply))
  Inspect(reply: Subject(Snapshot))
}

type Owner {
  Owner(state: State, now: fn() -> Timestamp, lease_id: fn() -> String)
}

/// Starts an owner over a new, empty queue.
///
/// `now` and `lease_id` must return. The owner calls them in its own process
/// while it applies a trigger, so one that crashes or never answers takes the
/// owner down, and with it the process that started the owner. That is not
/// one of the failures the owner answers with: nothing here can catch it.
pub fn start(
  config: Config,
  now: fn() -> Timestamp,
  lease_id: fn() -> String,
) -> actor.StartResult(Subject(Message)) {
  actor.new(Owner(state: model.new(config), now:, lease_id:))
  |> actor.on_message(handle)
  |> actor.start
}

fn handle(owner: Owner, message: Message) -> actor.Next(Owner, Message) {
  case message {
    Inspect(reply) -> {
      process.send(reply, inspection.snapshot(owner.state))
      actor.continue(owner)
    }
    Act(trigger, reply) ->
      case
        rules.drive(core.apply(owner.state, trigger), owner.now, owner.lease_id)
      {
        Ok(applied) -> {
          process.send(reply, Ok(Outcome(applied.fired, applied.answer)))
          actor.continue(Owner(..owner, state: applied.state))
        }
        Error(failure) -> {
          process.send(reply, Error(failure))
          actor.continue(owner)
        }
      }
  }
}

/// `PublisherJoins(publisher_id)`, from the `Publishing` surface.
pub fn publisher_joins(owner: Subject(Message), publisher_id: String) -> Reply {
  act(owner, core.PublisherJoins(publisher_id))
}

/// `ConsumerJoins(consumer_id)`, from the `Consuming` surface.
pub fn consumer_joins(owner: Subject(Message), consumer_id: String) -> Reply {
  act(owner, core.ConsumerJoins(consumer_id))
}

/// `Publish(publisher, pocket_key, record_key, rank, payload)`, from the
/// `Publishing` surface. `publisher` is the publisher's id.
pub fn publish(owner: Subject(Message), publication: Publication) -> Reply {
  act(owner, core.Publish(publication))
}

/// The complete snapshot of the state, as it is when the owner reaches this
/// message.
pub fn inspect(owner: Subject(Message)) -> Snapshot {
  process.call(owner, call_timeout, Inspect)
}

fn act(owner: Subject(Message), trigger: Trigger) -> Reply {
  process.call(owner, call_timeout, Act(trigger, _))
}
