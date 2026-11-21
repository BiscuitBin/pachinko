//// The one apply step. It runs a trigger's table, then the events that
//// outcome emits, each through its own table, then the invariant checker,
//// once, over the state the whole chained outcome left. It returns the names
//// of the rules that fired. It is pure: where an outcome needs `now` or a
//// lease id it returns a step that asks for one, and the owner supplies it.

import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/consuming.{type LeaseView}
import pachinko_double/decay
import pachinko_double/domain.{
  type Answer, type Cursor, type Event, type Failure, type Step, type Table,
  type World, Freed, Handed, LeaseLapsed, PocketTouched,
}
import pachinko_double/expiry.{Lapse}
import pachinko_double/identity
import pachinko_double/inspection
import pachinko_double/invariants
import pachinko_double/lease_end.{Freeing}
import pachinko_double/model.{
  type DeadLetter, type Lease, type State, type SupersededRecord, Consumer,
  LeaseActive, Publisher, Queue, State,
}
import pachinko_double/publish.{type Publication}
import pachinko_double/pull.{type Pull, Handover}
import pachinko_double/refusals.{UnknownConsumer, UnknownPublisher}
import pachinko_double/rules.{
  BrokenInvariant, Defect, Done, EffectFailed, NeedsLeaseId, NeedsNow, Refused,
  World,
}
import pachinko_double/settle.{type Ack, type Nack, type Release, type Renew}
import pachinko_double/time.{type Timestamp}

/// The triggers a surface provides, one variant each. The clock's triggers,
/// the lapse and the two decays, are not among them: only `tick` reaches
/// their tables.
pub type Trigger {
  PublisherJoins(publisher_id: String)
  ConsumerJoins(consumer_id: String)
  Publish(publication: Publication)
  Pull(pull: Pull)
  Ack(ack: Ack)
  Nack(nack: Nack)
  Renew(renew: Renew)
  Release(release: Release)
}

/// Applies one trigger to a state. The state it was given is untouched
/// whatever the result: a failure simply carries no new state.
pub fn apply(state: State, trigger: Trigger) -> Step {
  case trigger {
    PublisherJoins(publisher_id) ->
      run(identity.publisher_joins(), publisher_id, state)
    ConsumerJoins(consumer_id) ->
      run(identity.consumer_joins(), consumer_id, state)
    Publish(publication) ->
      // A publisher that never joined cannot publish: the trigger takes a
      // `Publisher`, and only a join makes one. It is refused before any
      // rule runs, so no table is consulted (spec note G12).
      case list.contains(state.publishers, Publisher(publication.publisher)) {
        True -> run(publish.publish(), publication, state)
        False -> rules.fail(Refused(UnknownPublisher(publication.publisher)))
      }
    // A consumer that never joined can neither pull, ack, nack, renew nor
    // release: each trigger takes a `Consumer`, and only a join makes one.
    // Each is refused before any rule runs, as the unknown publisher is.
    Pull(request) ->
      case joined(state, request.consumer) {
        True -> run(pull.pull(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Renew(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.renew(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Ack(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.ack(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Nack(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.nack(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
    Release(request) ->
      case joined(state, request.consumer) {
        True -> run(settle.release(), request, state)
        False -> rules.fail(Refused(UnknownConsumer(request.consumer)))
      }
  }
}

/// The `Consuming` surface's lease view for this consumer id: a read, not a
/// trigger. It goes through no table, asks for no time and returns no state.
/// The surface faces a `Consumer`, so an id that never joined is refused, as
/// it is for every trigger the surface provides.
pub fn lease_view(
  state: State,
  consumer_id: String,
) -> Result(Option(LeaseView), Failure) {
  case joined(state, consumer_id) {
    True -> Ok(consuming.lease_view(state, consumer_id))
    False -> Error(Refused(UnknownConsumer(consumer_id)))
  }
}

/// Whether a consumer with this id has joined.
fn joined(state: State, consumer_id: String) -> Bool {
  list.contains(state.consumers, Consumer(consumer_id))
}

/// Runs one table as a whole outcome over a state: the row whose guard
/// holds, the events it chains, then the checker. `apply` sends every
/// trigger through here, and a test can send a table of its own.
pub fn run(table: Table(input), input: input, state: State) -> Step {
  rules.run(table, input, rules.start(state), world(), conclude)
}

/// Runs one table as a whole outcome, as `run` does, with `now` already in
/// hand: a row that asks for the time is handed `now` and reads no clock.
/// This is how a clock trigger's outcome runs, with the `now` its `Tick`
/// read.
pub fn run_at(
  table: Table(input),
  input: input,
  state: State,
  now: Timestamp,
) -> Step {
  rules.run(table, input, rules.at(rules.start(state), now), world(), conclude)
}

/// What several outcomes run one after another came to: the state the last
/// one that succeeded left, the names of the rules each outcome that
/// succeeded fired, one list per outcome in order, and the failure that
/// stopped the run, if one did.
pub type Ran {
  Ran(state: State, fired: List(List(String)), failure: Option(Failure))
}

/// Runs the outcomes in order, each over the state the one before it left,
/// each to its end and through the checker. Each must be done without asking
/// for anything: an outcome that asks for the time or for a lease id is a
/// defect. The run stops at the first outcome that fails and keeps what the
/// outcomes before it did; the rest never run.
pub fn run_due(state: State, outcomes: List(fn(State) -> Step)) -> Ran {
  run_due_from(state, outcomes, [])
}

fn run_due_from(
  state: State,
  outcomes: List(fn(State) -> Step),
  fired: List(List(String)),
) -> Ran {
  let stop = fn(failure) {
    Ran(state:, fired: list.reverse(fired), failure: Some(failure))
  }
  case outcomes {
    [] -> Ran(state:, fired: list.reverse(fired), failure: None)
    [outcome, ..rest] ->
      case outcome(state) {
        Done(Ok(applied)) ->
          run_due_from(applied.state, rest, [applied.fired, ..fired])
        Done(Error(failure)) -> stop(failure)
        NeedsNow(_) ->
          stop(Defect(
            "Tick",
            EffectFailed("Tick", "an outcome of a tick asked for the time"),
          ))
        NeedsLeaseId(_) ->
          stop(Defect(
            "Tick",
            EffectFailed("Tick", "an outcome of a tick asked for a lease id"),
          ))
      }
  }
}

/// What a tick did. `fired` is the names of the rules every outcome that
/// succeeded fired, in order. `lapsed` is the leases it lapsed, in the order
/// they lapsed, as the state keeps them once ended. `superseded` and
/// `dead_letters` are the entries that decayed, in the order they decayed,
/// as they stood.
pub type TickReport {
  TickReport(
    fired: List(String),
    lapsed: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}

/// A tick's whole result: the state it reached, what it reports, and the
/// failure that stopped it, if one did. A tick keeps every outcome that
/// succeeded before a failure.
pub type Ticked {
  Ticked(state: State, report: TickReport, failure: Option(Failure))
}

/// Everything the clock has made due at `now`, applied. First the due
/// lapses: each active lease whose `expires_at <= now`, smallest
/// `expires_at` first and equal ones in their creation order in
/// `state.leases` (G57), each its own outcome. Then, only if no lapse
/// failed, the decays due in the state the lapses left: the superseded
/// records, then the dead letters, each whose `expires_at <= now`, each
/// queue oldest first, each its own outcome. Every outcome runs with `now`
/// in hand and is checked at its end, and an entry a lapse creates with a
/// zero TTL is due in the same tick.
pub fn tick(state: State, now: Timestamp) -> Ticked {
  let due =
    state.leases
    |> list.filter(fn(lease) {
      lease.status == LeaseActive && lease.expires_at <= now
    })
    |> list.sort(fn(a, b) { int.compare(a.expires_at, b.expires_at) })
  let lapses =
    run_due(
      state,
      list.map(due, fn(lease) {
        run_at(expiry.lease_lapses(), Lapse(lease: lease.id, now:), _, now)
      }),
    )
  let lapsed = applied(due, lapses.fired)
  case lapses.failure {
    Some(_) -> report(lapses, lapsed, [], [], lapses.fired)
    None -> {
      let shed =
        list.filter(lapses.state.superseded, fn(shed) { shed.expires_at <= now })
      let letters =
        list.filter(lapses.state.dead_letters, fn(letter) {
          letter.expires_at <= now
        })
      let decays =
        run_due(
          lapses.state,
          list.append(
            list.map(shed, fn(shed) {
              run_at(decay.superseded_record_decays(), shed, _, now)
            }),
            list.map(letters, fn(letter) {
              run_at(decay.dead_letter_decays(), letter, _, now)
            }),
          ),
        )
      report(
        decays,
        lapsed,
        applied(shed, decays.fired),
        applied(letters, list.drop(decays.fired, list.length(shed))),
        list.append(lapses.fired, decays.fired),
      )
    }
  }
}

/// The inputs whose outcome ran and fired a rule, each paired with the
/// names its outcome fired. An outcome after a failure never ran, and one in
/// which nothing happened fired nothing: neither is reported.
fn applied(inputs: List(a), fired: List(List(String))) -> List(a) {
  list.zip(inputs, fired)
  |> list.filter_map(fn(pair) {
    case pair.1 {
      [] -> Error(Nil)
      _ -> Ok(pair.0)
    }
  })
}

/// The tick's result, with each lapsed lease read by its id from the state
/// the tick reached, where it is expired. A lease is never removed, so each
/// is there; were one missing, the tick would report that as its failure.
fn report(
  ran: Ran,
  lapsed: List(Lease),
  superseded: List(SupersededRecord),
  dead_letters: List(DeadLetter),
  fired: List(List(String)),
) -> Ticked {
  let found =
    list.map(lapsed, fn(lease) {
      #(lease.id, model.lease(ran.state.leases, lease.id))
    })
  let missing =
    list.find_map(found, fn(found) {
      case found.1 {
        Ok(_) -> Error(Nil)
        Error(Nil) -> Ok(found.0)
      }
    })
  let failure = case ran.failure, missing {
    Some(failure), _ -> Some(failure)
    None, Ok(id) ->
      Some(Defect("Tick", EffectFailed("LeaseLapses", "no lease " <> id)))
    None, Error(Nil) -> None
  }
  Ticked(
    state: ran.state,
    report: TickReport(
      fired: list.flatten(fired),
      lapsed: list.filter_map(found, fn(found) { found.1 }),
      superseded:,
      dead_letters:,
    ),
    failure:,
  )
}

/// The end of every outcome: the checker reads the state the whole chain
/// left, never a state part-way through it. An outcome that leaves an
/// invariant broken fails, naming each one, and so leaves no new state.
fn conclude(cursor: Cursor, answer: Option(Answer)) -> Step {
  case invariants.check(inspection.snapshot(rules.state(cursor))) {
    [] -> rules.finish(cursor, answer)
    broken -> rules.fail(BrokenInvariant(broken))
  }
}

/// Every table, as its trigger's name and its rules' names. The coverage
/// check reads the rules from here, so a rule is registered only by being a
/// row of a table the apply step can reach.
pub fn registry() -> List(#(String, List(String))) {
  [
    entry(identity.publisher_joins()),
    entry(identity.consumer_joins()),
    entry(publish.publish()),
    entry(publish.pocket_touched()),
    entry(pull.pull()),
    entry(pull.handed()),
    entry(settle.renew()),
    entry(settle.ack()),
    entry(settle.nack()),
    entry(settle.release()),
    entry(lease_end.freed()),
    entry(expiry.lease_lapses()),
    entry(expiry.lease_lapsed()),
    entry(decay.superseded_record_decays()),
    entry(decay.dead_letter_decays()),
  ]
}

fn entry(table: Table(input)) -> #(String, List(String)) {
  #(table.trigger, rules.names(table))
}

/// What the machinery needs from the domain. A position is allotted in one
/// place, the interpreter in `rules`, which reads and moves the queue's
/// `next_position` through the two functions here and overwrites whatever
/// counter a row finishes with. Nothing else reads or moves it for a rule.
fn world() -> World {
  World(
    next_position: fn(state: State) { state.queue.next_position },
    with_next_position: fn(state, next_position) {
      State(..state, queue: Queue(next_position:))
    },
    chain:,
  )
}

/// Sends an emitted event to its table.
fn chain(event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
  case event {
    PocketTouched(pocket_key) ->
      rules.run(
        publish.pocket_touched(),
        pocket_key,
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
    Handed(pocket_key, record_key) ->
      rules.run(
        pull.handed(),
        Handover(pocket_key:, record_key:),
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
    Freed(lease, record_key, ended_at) ->
      rules.run(
        lease_end.freed(),
        Freeing(lease:, record_key:, ended_at:),
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
    LeaseLapsed(lease, now) ->
      rules.run(
        expiry.lease_lapsed(),
        Lapse(lease:, now:),
        cursor,
        world(),
        fn(cursor, _answer) { then(cursor) },
      )
  }
}
