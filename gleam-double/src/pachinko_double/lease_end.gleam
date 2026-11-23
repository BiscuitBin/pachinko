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

import gleam/list
import gleam/option.{None, Some}
import gleam/result
import pachinko_double/config
import pachinko_double/domain.{type Event, type Row, type Table, Freed}
import pachinko_double/model.{
  type DeadLetter, type HeldRecord, type Lease, type Record, type State, Acked,
  DeadLetter, Lease, LeaseExpired, LeaseReleased, Nacked, Record, RecordQueued,
  State, Superseded, SupersededRecord, Unsettled,
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
    |> model.oldest_first
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
/// version deferred behind the held one, here in the specification's
/// order. A lease's end emits `Freed` only for a held record of its pocket,
/// so a `Freed` no rule answers is a defect.
pub fn freed() -> Table(Freeing) {
  Table(
    "Freed",
    [
      requeue_unsettled_record(),
      supersede_unsettled_held_version(),
      consume_acked_record(),
      consume_acked_version_queue_deferred(),
      dead_letter_nacked_record(),
      dead_letter_nacked_version_queue_deferred(),
    ],
    Defective,
  )
}

/// What a `Freed` rule reads: the lease the event names, for `lease.pocket`
/// and `lease.consumer`, and the held record `Record{pocket: lease.pocket,
/// key: record_key}`. It is the record a `Freed` names, where it `exists` in
/// the lease's pocket and `record.status = held`. The lease is the one with
/// the id the event carries, whatever its status: the rule that ended it has
/// already run.
fn held(state: State, freeing: Freeing) -> Result(HeldRecord, Nil) {
  use lease <- result.try(model.lease(state.leases, freeing.lease))
  model.held_under(state.records, lease, freeing.record_key)
}

/// A `Freed` rule's effect over what its guard saw. Where that is missing,
/// which only a wrong guard could bring about, the outcome is a defect.
fn free(
  state: State,
  freeing: Freeing,
  effect: fn(HeldRecord) -> domain.Plan,
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

/// `let arrival_position = record.deferred_position`: the arrival position
/// the guard promised, for `effect`. Where there is none, which only a
/// wrong guard could bring about, the outcome is a defect.
fn arrival(held: HeldRecord, effect: fn(Int) -> domain.Plan) -> domain.Plan {
  case held.deferred_position {
    None ->
      Impossible(
        "held record " <> held.record.key <> " has no deferred_position",
      )
    Some(arrival_position) -> effect(arrival_position)
  }
}

/// `let nacked_at = record.nacked_at`, for `effect`. `NackedHasItsTime`
/// promises the time, and a dead letter cannot be made without one.
fn nack_time(
  held: HeldRecord,
  effect: fn(Timestamp) -> domain.Plan,
) -> domain.Plan {
  case held.nacked_at {
    None ->
      Impossible("nacked record " <> held.record.key <> " has no nacked_at")
    Some(nacked_at) -> effect(nacked_at)
  }
}

/// `DeadLetter.created(...)` for the held version: the lease's pocket and
/// consumer, the held rank and payload, the time of the nack, and
/// `expires_at` stamped from the event's `ended_at`.
fn letter(
  state: State,
  held: HeldRecord,
  freeing: Freeing,
  nacked_at: Timestamp,
) -> DeadLetter {
  DeadLetter(
    pocket: held.lease.pocket,
    key: freeing.record_key,
    rank: held.held_rank,
    payload: held.held_payload,
    nacked_by: held.lease.consumer,
    nacked_at:,
    expires_at: freeing.ended_at + config.settings(state.config).dead_letter_ttl,
  )
}

/// `record.status = queued` at `position`, with `held_rank`, `held_payload`,
/// `mark`, `nacked_at` and `deferred_position` all null: the held variant's
/// fields go with it. The record keeps its rank and payload, the newest
/// version's, and its place in the list.
fn queue_at(state: State, record: Record, position: Int) -> State {
  let queued = Record(..record, position:, status: RecordQueued)
  State(..state, records: model.replace_record(state.records, queued))
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
      Finish(queue_at(state, held.record, held.record.position), [], None)
    },
  )
}

/// `SupersedeUnsettledHeldVersion`. The held version was neither acked nor
/// nacked and a newer version is deferred behind it: the held version is
/// shed, added at the end of the superseded queue with `shed_at` the
/// event's `ended_at`, not a `now` of this rule's own, and the newer
/// version is queued at the held version's position.
fn supersede_unsettled_held_version() -> Row(Freeing) {
  Row(
    "SupersedeUnsettledHeldVersion",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Unsettled && held.deferred_position != None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      let shed =
        SupersededRecord(
          pocket: held.lease.pocket,
          key: freeing.record_key,
          rank: held.held_rank,
          payload: held.held_payload,
          reason: Superseded,
          shed_at: freeing.ended_at,
          expires_at: freeing.ended_at
            + config.settings(state.config).superseded_ttl,
        )
      let state = queue_at(state, held.record, held.record.position)
      Finish(
        State(..state, superseded: list.append(state.superseded, [shed])),
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

/// `ConsumeAckedVersionQueueDeferred`. The held version was acked and a
/// newer version is deferred behind it: the acked version is consumed and
/// recorded nowhere, and the newer version, a new arrival, is queued at its
/// arrival position. The counter does not move.
fn consume_acked_version_queue_deferred() -> Row(Freeing) {
  Row(
    "ConsumeAckedVersionQueueDeferred",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Acked && held.deferred_position != None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      use arrival_position <- arrival(held)
      Finish(queue_at(state, held.record, arrival_position), [], None)
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
      use nacked_at <- nack_time(held)
      let letter = letter(state, held, freeing, nacked_at)
      let state = without(state, held.record)
      Finish(
        State(..state, dead_letters: list.append(state.dead_letters, [letter])),
        [],
        None,
      )
    },
  )
}

/// `DeadLetterNackedVersionQueueDeferred`. The held version was nacked and
/// a newer version is deferred behind it: the held version becomes a dead
/// letter, as `DeadLetterNackedRecord` makes one, and the newer version, a
/// new arrival, is queued at its arrival position. The counter does not
/// move.
fn dead_letter_nacked_version_queue_deferred() -> Row(Freeing) {
  Row(
    "DeadLetterNackedVersionQueueDeferred",
    fn(state: State, freeing: Freeing) {
      case held(state, freeing) {
        Error(Nil) -> False
        Ok(held) -> held.mark == Nacked && held.deferred_position != None
      }
    },
    fn(state, freeing) {
      use held <- free(state, freeing)
      use arrival_position <- arrival(held)
      use nacked_at <- nack_time(held)
      let letter = letter(state, held, freeing, nacked_at)
      let state = queue_at(state, held.record, arrival_position)
      Finish(
        State(..state, dead_letters: list.append(state.dead_letters, [letter])),
        [],
        None,
      )
    },
  )
}
