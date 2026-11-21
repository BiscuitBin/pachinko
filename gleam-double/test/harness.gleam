//// The fakes the tests inject, and the helpers that start them. Each fake is
//// a small process of its own, so that the test and whatever it injects the
//// fake into see one clock and one lease-id source. Neither holds any queue
//// state.

import gleam/erlang/process.{type Subject}
import gleam/int
import gleam/list
import gleam/otp/actor
import pachinko_double/config.{type Config, type Settings, Settings}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/owner.{type Reply}
import pachinko_double/time.{type Duration, type Timestamp}

/// How long a call to a fake may take, in milliseconds.
const call_timeout = 1000

type ClockMessage {
  Read(reply: Subject(Timestamp))
  Set(time: Timestamp)
  Reads(reply: Subject(Int))
}

type ClockState {
  ClockState(time: Timestamp, reads: Int)
}

/// A clock whose time a test sets, and which counts its reads.
pub opaque type Clock {
  Clock(subject: Subject(ClockMessage))
}

/// Starts a fake clock at time 0, with no read counted.
pub fn start_clock() -> Clock {
  let assert Ok(started) =
    actor.new(ClockState(time: 0, reads: 0))
    |> actor.on_message(fn(state: ClockState, message) {
      case message {
        Read(reply) -> {
          process.send(reply, state.time)
          actor.continue(ClockState(..state, reads: state.reads + 1))
        }
        Set(time) -> actor.continue(ClockState(..state, time:))
        Reads(reply) -> {
          process.send(reply, state.reads)
          actor.continue(state)
        }
      }
    })
    |> actor.start
  Clock(started.data)
}

/// The function to inject: each call is one counted read.
pub fn clock(clock: Clock) -> fn() -> Timestamp {
  fn() { process.call(clock.subject, call_timeout, Read) }
}

/// Sets the time. It is not a read.
pub fn set_time(clock: Clock, time: Timestamp) -> Nil {
  process.send(clock.subject, Set(time))
}

/// Moves the time on by a duration, and returns the new time. It is not a
/// counted read: the test already knows the time it set.
pub fn advance(clock: Clock, from: Timestamp, by: Duration) -> Timestamp {
  set_time(clock, from + by)
  from + by
}

/// How many times the injected function has been called.
pub fn clock_reads(clock: Clock) -> Int {
  process.call(clock.subject, call_timeout, Reads)
}

type LeaseIdsMessage {
  Draw(reply: Subject(String))
  Draws(reply: Subject(Int))
}

/// A lease-id source that hands out `lease-1`, `lease-2` and so on, and
/// counts its draws.
pub opaque type LeaseIds {
  LeaseIds(subject: Subject(LeaseIdsMessage))
}

/// Starts a fake lease-id source with no draw counted.
pub fn start_lease_ids() -> LeaseIds {
  let assert Ok(started) =
    actor.new(0)
    |> actor.on_message(fn(draws, message) {
      case message {
        Draw(reply) -> {
          process.send(reply, "lease-" <> int.to_string(draws + 1))
          actor.continue(draws + 1)
        }
        Draws(reply) -> {
          process.send(reply, draws)
          actor.continue(draws)
        }
      }
    })
    |> actor.start
  LeaseIds(started.data)
}

/// The function to inject: each call is one counted draw.
pub fn lease_ids(lease_ids: LeaseIds) -> fn() -> String {
  fn() { process.call(lease_ids.subject, call_timeout, Draw) }
}

/// How many ids have been drawn.
pub fn lease_id_draws(lease_ids: LeaseIds) -> Int {
  process.call(lease_ids.subject, call_timeout, Draws)
}

/// A started owner with its fakes, and a log of the rules its triggers fired.
pub type Double {
  Double(
    owner: Subject(owner.Message),
    clock: Clock,
    lease_ids: LeaseIds,
    log: Subject(LogMessage),
  )
}

pub opaque type LogMessage {
  Fired(names: List(String))
  Log(reply: Subject(List(String)))
}

/// Starts an owner over an empty queue, with a fake clock at 0 and a fake
/// lease-id source. The owner is linked to the calling test's process, so it
/// dies with a test that crashes or is killed; otherwise it ends with the VM.
pub fn start(config: Config) -> Double {
  let fake_clock = start_clock()
  let fake_lease_ids = start_lease_ids()
  let assert Ok(log) =
    actor.new([])
    |> actor.on_message(fn(fired, message) {
      case message {
        Fired(names) -> actor.continue(list.append(fired, names))
        Log(reply) -> {
          process.send(reply, fired)
          actor.continue(fired)
        }
      }
    })
    |> actor.start
  let assert Ok(started) =
    owner.start(config, clock(fake_clock), lease_ids(fake_lease_ids))
  Double(
    owner: started.data,
    clock: fake_clock,
    lease_ids: fake_lease_ids,
    log: log.data,
  )
}

/// The instant the Rust fixture starts at, `T0` in
/// `crates/libpachinko/tests/queue/fixture.rs`.
pub const t0: Timestamp = 1_000_000

/// The Rust fixture's settings: the specification's two limits, and three
/// short TTLs, each unlike the others so that a mix-up shows.
pub fn fixture_settings() -> Settings {
  Settings(
    ..config.defaults(),
    lease_ttl: 1000 * time.millisecond,
    superseded_ttl: 5000 * time.millisecond,
    dead_letter_ttl: 7000 * time.millisecond,
  )
}

/// The Rust fixture's starting point: its settings, the clock at `t0`, and
/// publisher `p` and consumers `c1`, `c2` and `c3`, all joined. Setting the
/// time is not a counted read.
pub fn fixture() -> Double {
  let assert Ok(config) = config.new(fixture_settings())
  let double = start(config)
  set_time(double.clock, t0)
  let assert Ok(_) = publisher_joins(double, "p")
  let assert Ok(_) = consumer_joins(double, "c1")
  let assert Ok(_) = consumer_joins(double, "c2")
  let assert Ok(_) = consumer_joins(double, "c3")
  double
}

/// The names of the rules fired by every trigger sent through this double
/// that succeeded, in order. A test returns it to the runner.
pub fn fired(double: Double) -> List(String) {
  process.call(double.log, call_timeout, Log)
}

fn logged(double: Double, reply: Reply) -> Reply {
  case reply {
    Ok(outcome) -> process.send(double.log, Fired(outcome.fired))
    Error(_) -> Nil
  }
  reply
}

pub fn publisher_joins(double: Double, publisher_id: String) -> Reply {
  logged(double, owner.publisher_joins(double.owner, publisher_id))
}

pub fn consumer_joins(double: Double, consumer_id: String) -> Reply {
  logged(double, owner.consumer_joins(double.owner, consumer_id))
}

pub fn inspect(double: Double) -> Snapshot {
  owner.inspect(double.owner)
}
