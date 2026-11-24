//// The fakes the tests inject, and the helpers that start them. Each fake is
//// a small process of its own, so that the test and whatever it injects the
//// fake into see one clock and one lease-id source. Neither holds any queue
//// state.

import gleam/bit_array
import gleam/erlang/process.{type Subject}
import gleam/int
import gleam/list
import gleam/option.{type Option, Some}
import gleam/otp/actor
import pachinko_double.{type PartialTick, type Reply, Outcome}
import pachinko_double/config.{type Config, type Settings, Settings}
import pachinko_double/consuming.{type HeldView, type LeaseView}
import pachinko_double/core.{type TickReport}
import pachinko_double/domain.{
  type Applied, type BatchRecord, type Failure, type Step, Batch,
}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/model.{type Pocket, type Record, PocketQueued}
import pachinko_double/publish.{type Publication, Publication}
import pachinko_double/refusals.{type Refusal}
import pachinko_double/rules.{Done, NeedsLeaseId, NeedsNow, Refused}
import pachinko_double/time.{type Duration, type Timestamp}

/// How long a call to a fake may take, in milliseconds.
const call_timeout = 1000

type ClockMessage {
  Read(reply: Subject(Timestamp))
  Set(time: Timestamp, reply: Subject(Nil))
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
        Set(time, reply) -> {
          process.send(reply, Nil)
          actor.continue(ClockState(..state, time:))
        }
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

/// Sets the time. It is not a read. It waits until the clock holds the new
/// time, so a trigger sent afterwards reads it: the owner reads the clock
/// from its own process, and Erlang orders messages only between one sender
/// and one receiver.
pub fn set_time(clock: Clock, time: Timestamp) -> Nil {
  process.call(clock.subject, call_timeout, Set(time, _))
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
    owner: Subject(pachinko_double.Message),
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
    pachinko_double.start(config, clock(fake_clock), lease_ids(fake_lease_ids))
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
  fixture_with(fixture_settings())
}

/// The Rust fixture's `with_config`: the same starting point over other
/// settings.
pub fn fixture_with(settings: Settings) -> Double {
  let assert Ok(config) = config.new(settings)
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
  logged(double, pachinko_double.publisher_joins(double.owner, publisher_id))
}

pub fn consumer_joins(double: Double, consumer_id: String) -> Reply {
  logged(double, pachinko_double.consumer_joins(double.owner, consumer_id))
}

/// A publish by the fixture's publisher, `p`, with a text payload, as the
/// Rust fixture's `publish` does.
pub fn publish(
  double: Double,
  pocket_key: String,
  record_key: String,
  rank: Int,
  payload: String,
) -> Reply {
  publish_as(double, "p", pocket_key, record_key, rank, payload)
}

/// A publish by any publisher id, joined or not.
pub fn publish_as(
  double: Double,
  publisher: String,
  pocket_key: String,
  record_key: String,
  rank: Int,
  payload: String,
) -> Reply {
  publish_publication(
    double,
    Publication(
      publisher:,
      pocket_key:,
      record_key:,
      rank:,
      payload: bit_array.from_string(payload),
    ),
  )
}

/// A publish of a publication made by hand, for a payload that is not text.
pub fn publish_publication(double: Double, publication: Publication) -> Reply {
  logged(double, pachinko_double.publish(double.owner, publication))
}

/// A tick, as the Rust fixture's `tick` without its check: the owner reads
/// the clock once and applies everything due. A tick that succeeded is
/// logged like any trigger.
pub fn tick(double: Double) -> Result(TickReport, PartialTick) {
  let reply = pachinko_double.tick(double.owner)
  case reply {
    Ok(report) -> process.send(double.log, Fired(report.fired))
    Error(_) -> Nil
  }
  reply
}

/// A pull, as the Rust fixture's `try_pull`: whatever the owner replies.
pub fn pull(double: Double, consumer: String, limit: Int) -> Reply {
  logged(double, pachinko_double.pull(double.owner, consumer, limit))
}

/// A pulled batch, unpacked, as the Rust fixture's `Batch`, beside the names
/// of the rules the pull fired. `lease` is the lease's id and `pocket_key`
/// the pocket's key. It has no `expires_at`, which the Rust's has: the
/// specification's `Batch` carries none, so a test reads the expiry from the
/// snapshot's lease.
pub type Pulled {
  Pulled(
    fired: List(String),
    lease: String,
    pocket_key: String,
    records: List(BatchRecord),
  )
}

/// A pull that must return a batch, as the Rust fixture's `pull`. The test
/// fails on any other reply, and on a batch that names another consumer.
pub fn batch(double: Double, consumer: String, limit: Int) -> Pulled {
  let assert Ok(Outcome(
    fired,
    Some(Batch(consumer: named, lease:, pocket: pocket_key, records:)),
  )) = pull(double, consumer, limit)
  assert named == consumer
  Pulled(fired:, lease:, pocket_key:, records:)
}

/// The records' keys, in order, as the Rust fixture's `keys`.
pub fn keys(records: List(BatchRecord)) -> List(String) {
  list.map(records, fn(record) { record.key })
}

pub fn renew(double: Double, consumer: String, lease: String) -> Reply {
  logged(double, pachinko_double.renew(double.owner, consumer, lease))
}

/// An ack, as the Rust fixture's `ack`: whatever the owner replies.
pub fn ack(
  double: Double,
  consumer: String,
  lease: String,
  record_key: String,
) -> Reply {
  logged(double, pachinko_double.ack(double.owner, consumer, lease, record_key))
}

/// A nack, as the Rust fixture's `nack`: whatever the owner replies.
pub fn nack(
  double: Double,
  consumer: String,
  lease: String,
  record_key: String,
) -> Reply {
  logged(
    double,
    pachinko_double.nack(double.owner, consumer, lease, record_key),
  )
}

/// A release, as the Rust fixture's `release`: whatever the owner replies.
pub fn release(double: Double, consumer: String, lease: String) -> Reply {
  logged(double, pachinko_double.release(double.owner, consumer, lease))
}

/// The Rust fixture's `refused`: runs `action`, which must be refused with
/// `expected`, and checks that the complete snapshot is the same after as
/// before. It checks two things more than the Rust's: that the refusal read
/// no clock and drew no lease id.
pub fn refused(
  double: Double,
  expected: Refusal,
  action: fn() -> Reply,
) -> Nil {
  let before = inspect(double)
  let reads = clock_reads(double.clock)
  let draws = lease_id_draws(double.lease_ids)
  assert action() == Error(Refused(expected))
  assert inspect(double) == before
  assert clock_reads(double.clock) == reads
  assert lease_id_draws(double.lease_ids) == draws
  Nil
}

/// The Rust fixture's `at`: sets the clock to `millis` after `t0`, and
/// returns that time. It is not a counted read.
pub fn at(double: Double, millis: Duration) -> Timestamp {
  set_time(double.clock, t0 + millis)
  t0 + millis
}

/// The Rust fixture's `lease_view`: what the `Consuming` surface exposes to
/// this consumer, read through the owner's `LeaseView` message. `Ok(None)`
/// says the consumer has no active lease. Where the Rust's unwraps, this
/// hands back the owner's reply whole, so that a test can read a refusal.
pub fn lease_view(
  double: Double,
  consumer: String,
) -> Result(Option(LeaseView), Failure) {
  pachinko_double.lease_view(double.owner, consumer)
}

/// The lease view's records' keys, in order, as the Rust fixture's
/// `marked_keys`.
pub fn marked_keys(records: List(HeldView)) -> List(String) {
  list.map(records, fn(record) { record.key })
}

/// What a step came to when driven with one time and one lease id, and how
/// often it asked for each.
pub type Driven {
  Driven(
    result: Result(Applied, Failure),
    clock_reads: Int,
    lease_id_draws: Int,
  )
}

/// Drives a step of the pure apply step to its end, as `rules.drive` does,
/// answering every request for the time with `now` and every request for a
/// lease id with `lease_id`, and counting both. For a test over a state
/// built by hand, where no owner and no fake is involved.
pub fn drive(step: Step, now: Timestamp, lease_id: String) -> Driven {
  drive_on(step, now, lease_id, 0, 0)
}

fn drive_on(
  step: Step,
  now: Timestamp,
  lease_id: String,
  clock_reads: Int,
  lease_id_draws: Int,
) -> Driven {
  case step {
    Done(result) -> Driven(result:, clock_reads:, lease_id_draws:)
    NeedsNow(next) ->
      drive_on(next(now), now, lease_id, clock_reads + 1, lease_id_draws)
    NeedsLeaseId(next) ->
      drive_on(next(lease_id), now, lease_id, clock_reads, lease_id_draws + 1)
  }
}

pub fn inspect(double: Double) -> Snapshot {
  pachinko_double.inspect(double.owner)
}

/// The pocket with this key, read from a fresh snapshot, as the Rust
/// fixture's `pocket` does. The test fails if there is none.
pub fn pocket(double: Double, pocket_key: String) -> Pocket {
  let assert Ok(pocket) = model.pocket(inspect(double).pockets, pocket_key)
  pocket
}

/// The record with this key in this pocket, read from a fresh snapshot, as
/// the Rust fixture's `record` does: the pocket first, then its record. The
/// test fails if there is no such pocket, or no such record in it.
pub fn record(
  double: Double,
  pocket_key: String,
  record_key: String,
) -> Record {
  let snapshot = inspect(double)
  let assert Ok(pocket) = model.pocket(snapshot.pockets, pocket_key)
  let assert Ok(record) =
    model.record(snapshot.records, pocket: pocket.key, key: record_key)
  record
}

/// The pocket keys in the pocket order, front first, as the Rust fixture's
/// `order` gives them. The double stores no such order: it is the queued
/// pockets by position, smallest first, which is the specification's
/// definition of it.
pub fn order(double: Double) -> List(String) {
  inspect(double).pockets
  |> list.filter(fn(pocket) { pocket.status == PocketQueued })
  |> list.sort(fn(a, b) { int.compare(a.position, b.position) })
  |> list.map(fn(pocket) { pocket.key })
}
