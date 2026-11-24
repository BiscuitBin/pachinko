//// The double's interface, and its owner: the one process that holds the
//// queue's state. Every trigger is one message in its mailbox, applied one at
//// a time through the pure core, so the mailbox is the specification's "one
//// serialized stream". The clock and the lease-id source are functions it is
//// started with. This is the only module under `src/` that imports
//// `gleam/erlang` or `gleam/otp`.
////
//// A caller calls this module by its name. Its seventeen items:
////
//// - Starting: `start`, `Message` and `call_timeout`.
//// - The `Publishing` surface: `publisher_joins` and `publish`.
//// - The `Consuming` surface: `consumer_joins`, `pull`, `renew`, `ack`,
////   `nack` and `release`, and the read `lease_view`.
//// - The clock: `tick`.
//// - What they answer: `Reply`, `Outcome` and `PartialTick`.
//// - The inspection seam: `inspect`.
////
//// Three things a caller reads are the double's own channels, for tests and
//// for the coverage check, and not something the specification answers:
//// `inspect`, the complete snapshot; the field `Outcome.fired`, the names of
//// the rules an outcome fired; and the field `TickReport.fired`, the same for
//// the outcomes of a `tick`.
////
//// A caller's other names live in other modules. What it names when it sends
//// a trigger or reads a reply is in `config`, `time`, `model` and `refusals`,
//// and the reply types are in two more: `failure` holds `Failure` and
//// `Cause`, and `answers` holds `Answer`, `BatchRecord`, `TickReport`,
//// `LeaseView` and `HeldView`. The snapshot that `inspect` returns, and the
//// checker of the invariants, are in `inspection` and `invariants`. These
//// eight modules sit directly under `src/pachinko_double/`, and a name in one
//// of them that is not part of the interface is marked `@internal`. The
//// engine, the rule-table machinery and the apply step, the specification's
//// rules, and the `Consuming` surface's lease-view read sit under
//// `src/pachinko_double/internal/`, which is no part of the interface.

import gleam/erlang/process.{type Subject}
import gleam/option.{type Option, None, Some}
import gleam/otp/actor
import pachinko_double/answers.{type Answer, type LeaseView, type TickReport}
import pachinko_double/config.{type Config}
import pachinko_double/failure.{type Failure}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/internal/core.{type Trigger}
import pachinko_double/internal/publish.{Publication}
import pachinko_double/internal/pull.{Pull}
import pachinko_double/internal/rules
import pachinko_double/internal/settle.{Ack, Nack, Release, Renew}
import pachinko_double/model.{type State}
import pachinko_double/refusals.{type Refusal}
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
  Result(Outcome, Failure(Refusal))

/// What a tick answers when one of its outcomes failed: what the outcomes
/// before it did, which the owner keeps, and the failure that stopped it.
/// The outcomes after it never ran.
pub type PartialTick {
  PartialTick(report: TickReport, failure: Failure(Refusal))
}

/// The owner's mailbox. `Act` carries any one trigger a surface provides,
/// so each trigger sent is one message and `core.Trigger` holds the variant
/// per trigger. `Tick` is the clock: it reads `now` once and applies
/// everything due, and nothing else makes time pass. The other two are
/// reads, which change nothing: `Inspect`, the complete snapshot, and
/// `LeaseView`, what the `Consuming` surface exposes to one consumer.
pub opaque type Message {
  Act(trigger: Trigger, reply: Subject(Reply))
  Tick(reply: Subject(Result(TickReport, PartialTick)))
  Inspect(reply: Subject(Snapshot))
  LeaseView(
    consumer: String,
    reply: Subject(Result(Option(LeaseView), Failure(Refusal))),
  )
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
    LeaseView(consumer, reply) -> {
      process.send(reply, core.lease_view(owner.state, consumer))
      actor.continue(owner)
    }
    Tick(reply) -> {
      let ticked = core.tick(owner.state, owner.now())
      process.send(reply, case ticked.failure {
        None -> Ok(ticked.report)
        Some(failure) -> Error(PartialTick(ticked.report, failure))
      })
      actor.continue(Owner(..owner, state: ticked.state))
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
pub fn publish(
  owner: Subject(Message),
  publisher: String,
  pocket_key: String,
  record_key: String,
  rank: Int,
  payload: BitArray,
) -> Reply {
  act(
    owner,
    core.Publish(Publication(
      publisher:,
      pocket_key:,
      record_key:,
      rank:,
      payload:,
    )),
  )
}

/// `Pull(consumer, limit)`, from the `Consuming` surface. `consumer` is the
/// consumer's id.
pub fn pull(owner: Subject(Message), consumer: String, limit: Int) -> Reply {
  act(owner, core.Pull(Pull(consumer:, limit:)))
}

/// `Renew(consumer, lease)`, from the `Consuming` surface. `consumer` is the
/// consumer's id and `lease` the lease's id.
pub fn renew(
  owner: Subject(Message),
  consumer: String,
  lease: String,
) -> Reply {
  act(owner, core.Renew(Renew(consumer:, lease:)))
}

/// `Ack(consumer, lease, record_key)`, from the `Consuming` surface.
/// `consumer` is the consumer's id and `lease` the lease's id.
pub fn ack(
  owner: Subject(Message),
  consumer: String,
  lease: String,
  record_key: String,
) -> Reply {
  act(owner, core.Ack(Ack(consumer:, lease:, record_key:)))
}

/// `Nack(consumer, lease, record_key)`, from the `Consuming` surface.
/// `consumer` is the consumer's id and `lease` the lease's id.
pub fn nack(
  owner: Subject(Message),
  consumer: String,
  lease: String,
  record_key: String,
) -> Reply {
  act(owner, core.Nack(Nack(consumer:, lease:, record_key:)))
}

/// `Release(consumer, lease)`, from the `Consuming` surface. `consumer` is
/// the consumer's id and `lease` the lease's id.
pub fn release(
  owner: Subject(Message),
  consumer: String,
  lease: String,
) -> Reply {
  act(owner, core.Release(Release(consumer:, lease:)))
}

/// What the `Consuming` surface exposes to this consumer, as the state is
/// when the owner reaches this message: the view of its active lease, or
/// `None` when it has none. A read: no rule fires, no clock is read and
/// nothing changes. `consumer` is the consumer's id.
pub fn lease_view(
  owner: Subject(Message),
  consumer: String,
) -> Result(Option(LeaseView), Failure(Refusal)) {
  process.call(owner, call_timeout, LeaseView(consumer, _))
}

/// The clock's trigger. The owner reads the clock once, whatever is due,
/// and applies every lapse and every decay due at that `now`, each its own
/// outcome, checked at its end. It keeps every outcome that succeeded: where
/// one fails it stops there and answers `PartialTick`.
pub fn tick(owner: Subject(Message)) -> Result(TickReport, PartialTick) {
  process.call(owner, call_timeout, Tick)
}

/// The complete snapshot of the state, as it is when the owner reaches this
/// message.
pub fn inspect(owner: Subject(Message)) -> Snapshot {
  process.call(owner, call_timeout, Inspect)
}

fn act(owner: Subject(Message), trigger: Trigger) -> Reply {
  process.call(owner, call_timeout, Act(trigger, _))
}
