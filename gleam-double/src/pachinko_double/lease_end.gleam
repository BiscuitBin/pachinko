//// The lease's end: the one module that owns it. A lease ends by release or
//// by lapse, and either way the same things happen: the lease takes its
//// final status and its `ended_at`, and `Freed` is sent for each record of
//// its held set. The rules that answer `Freed`, the specification's "Lease
//// end" section, read each record's mark and never how the lease ended.
////
//// The rule that ends a lease decides its pocket's fate, and reads
//// `outlasts` to do so; this module leaves the pocket alone. Every entry a
//// `Freed` rule creates takes its times from the `ended_at` the event
//// carries, the time the ending rule wrote to the lease, so no rule here
//// asks for the time.

import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import pachinko_double/config
import pachinko_double/domain.{type Event, type Row, type Table, Freed}
import pachinko_double/model.{
  type Lease, type Mark, type Record, type State, Acked, DeadLetter, Lease,
  LeaseExpired, LeaseReleased, Nacked, Record, RecordHeld, RecordQueued, State,
  Unsettled,
}
import pachinko_double/rules.{Defective, Finish, Impossible, Row, Table}
import pachinko_double/time.{type Timestamp}

/// How a lease ended. No rule that answers `Freed` reads it.
pub type Ending {
  /// The consumer released it: the lease ends `released`.
  Release
  /// Its `expires_at` passed: the lease ends `expired`.
  Lapse
}

/// The steps every rule that ends a lease shares:
///
/// ```text
/// ensures: lease.status = released        -- or expired
/// ensures: lease.ended_at = now
/// ensures:
///     for record in pocket.held_records:
///         Freed(lease: lease, record_key: record.key, ended_at: now)
/// ```
///
/// `at` is the calling rule's `now`. The state it answers differs from
/// `state` in that lease alone; the events are for the caller to emit, one
/// for each held record of the lease's pocket. The specification's
/// `held_records` is a set; the events are in order of the records'
/// positions, smallest first, which is the order the entries one lease end
/// creates are added in. The pocket's fate is the calling rule's.
pub fn end(
  state: State,
  lease: Lease,
  ending: Ending,
  at: Timestamp,
) -> #(State, List(Event)) {
  let status = case ending {
    Release -> LeaseReleased(ended_at: at)
    Lapse -> LeaseExpired(ended_at: at)
  }
  let ended = Lease(..lease, status:)
  let freed =
    state.records
    |> model.records_of(lease.pocket)
    |> model.held
    |> list.sort(fn(a, b) { int.compare(a.position, b.position) })
    |> list.map(fn(record) {
      Freed(lease: lease.id, record_key: record.key, ended_at: at)
    })
  #(State(..state, leases: model.replace_lease(state.leases, ended)), freed)
}

/// `pocket.records.count > pocket.ending_records.count`: whether a record of
/// this pocket outlasts its lease. The ending records are the ones the
/// lease's end removes, so a pocket that outlasts its lease is queued again
/// and one that does not has no record left. The rules that end a lease
/// split on it; the ending records are some of the pocket's records, so
/// where this does not hold the two counts are equal.
pub fn outlasts(state: State, pocket_key: String) -> Bool {
  let records = model.records_of(state.records, pocket_key)
  list.length(records) > list.length(model.ending(records))
}

/// The input of `Freed(lease, record_key, ended_at)`. `lease` is the lease's
/// id.
pub type Freeing {
  Freeing(lease: String, record_key: String, ended_at: Timestamp)
}

/// `Freed(lease, record_key, ended_at)`. The specification gives it six
/// rules with disjoint guards, one for each mark with and without a newer
/// version deferred behind the held one. The three here are the ones with
/// nothing deferred, in the specification's order. A lease's end emits
/// `Freed` only for a held record of its pocket, so a `Freed` no rule
/// answers is a defect.
pub fn freed() -> Table(Freeing) {
  Table(
    "Freed",
    [
      requeue_unsettled_record(),
      // `SupersedeUnsettledHeldVersion` goes here.
      consume_acked_record(),
      // `ConsumeAckedVersionQueueDeferred` goes here.
      dead_letter_nacked_record(),
      // `DeadLetterNackedVersionQueueDeferred` goes here.
    ],
    Defective,
  )
}

/// What a `Freed` rule reads: the lease the event names, for `lease.pocket`
/// and `lease.consumer`, and the held record `Record{pocket: lease.pocket,
/// key: record_key}` with the five fields only a held record has.
type Held {
  Held(
    lease: Lease,
    record: Record,
    held_rank: Int,
    held_payload: BitArray,
    mark: Mark,
    nacked_at: Option(Timestamp),
    deferred_position: Option(Int),
  )
}

/// The record a `Freed` names, where it `exists` in the lease's pocket and
/// `record.status = held`. The lease is the one with the id the event
/// carries, whatever its status: the rule that ended it has already run.
fn held(state: State, freeing: Freeing) -> Result(Held, Nil) {
  use lease <- result.try(model.lease(state.leases, freeing.lease))
  use record <- result.try(model.record(
    state.records,
    pocket: lease.pocket,
    key: freeing.record_key,
  ))
  case record.status {
    RecordQueued -> Error(Nil)
    RecordHeld(held_rank:, held_payload:, mark:, nacked_at:, deferred_position:) ->
      Ok(Held(
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

/// A `Freed` rule's effect over what its guard saw. Where that is missing,
/// which only a wrong guard could bring about, the outcome is a defect.
fn free(
  state: State,
  freeing: Freeing,
  effect: fn(Held) -> domain.Plan,
) -> domain.Plan {
  case held(state, freeing) {
    Error(Nil) ->
      Impossible(
        "no held record "
        <> freeing.record_key
        <> " under lease "
        <> freeing.lease,
      )
    Ok(held) -> effect(held)
  }
}

/// `ensures: not exists record`: the records without this one, the others
/// in the order they had.
fn without(state: State, record: Record) -> State {
  State(
    ..state,
    records: list.filter(state.records, fn(stored) {
      !{ stored.pocket == record.pocket && stored.key == record.key }
    }),
  )
}

/// `RequeueUnsettledRecord`. The held version was neither acked nor nacked
/// and nothing newer arrived: the record is queued again where it stood,
/// with none of the held fields.
fn requeue_unsettled_record() -> Row(Freeing) {
  Row(
    "RequeueUnsettledRecord",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Unsettled && held.deferred_position == None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      let queued = Record(..held.record, status: RecordQueued)
      Finish(
        State(..state, records: model.replace_record(state.records, queued)),
        [],
        None,
      )
    },
  )
}

/// `ConsumeAckedRecord`. The held version was acked and nothing newer
/// arrived: it is consumed, and recorded nowhere.
fn consume_acked_record() -> Row(Freeing) {
  Row(
    "ConsumeAckedRecord",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Acked && held.deferred_position == None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      Finish(without(state, held.record), [], None)
    },
  )
}

/// `DeadLetterNackedRecord`. The held version was nacked and nothing newer
/// arrived: the record is removed, and the held version becomes a dead
/// letter, added at the end of the dead-letter queue. It keeps the time its
/// nack was processed as `nacked_at`, and decays `dead_letter_ttl` after the
/// lease's end: `expires_at` is stamped from the event's `ended_at`, not
/// from a `now` of this rule's own.
fn dead_letter_nacked_record() -> Row(Freeing) {
  Row(
    "DeadLetterNackedRecord",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Nacked && held.deferred_position == None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      case held.nacked_at {
        // `NackedHasItsTime` promises the time, and a dead letter cannot be
        // made without one.
        None ->
          Impossible(
            "nacked record " <> freeing.record_key <> " has no nacked_at",
          )
        Some(nacked_at) -> {
          let letter =
            DeadLetter(
              pocket: held.lease.pocket,
              key: freeing.record_key,
              rank: held.held_rank,
              payload: held.held_payload,
              nacked_by: held.lease.consumer,
              nacked_at:,
              expires_at: freeing.ended_at
                + config.settings(state.config).dead_letter_ttl,
            )
          let state = without(state, held.record)
          Finish(
            State(
              ..state,
              dead_letters: list.append(state.dead_letters, [letter]),
            ),
            [],
            None,
          )
        }
      }
    },
  )
}
