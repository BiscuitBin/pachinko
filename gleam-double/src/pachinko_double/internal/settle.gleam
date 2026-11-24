//// The actions a consumer takes under a lease it names: the specification's
//// "Ack and nack" and "Renew and release" sections, as the Ack, Nack, Renew
//// and Release tables. Each names its lease by id and requires it to be
//// active and the consumer's. A release ends the lease through `lease_end`,
//// which owns what a lease's end does.

import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import pachinko_double/config
import pachinko_double/ids.{
  type ConsumerId, type LeaseId, type PocketKey, type RecordKey, ConsumerId,
  LeaseId, RecordKey,
}
import pachinko_double/internal/domain.{type Plan, type Row, type Table}
import pachinko_double/internal/lease_end
import pachinko_double/internal/rules.{
  Finish, Impossible, Now, Position, Refuses, Row, Table,
}
import pachinko_double/model.{
  type HeldRecord, type Lease, type Mark, type Pocket, type State, Acked, Lease,
  LeaseActive, Nacked, Pocket, PocketIdle, PocketLeased, PocketQueued, Record,
  RecordHeld, State, Unsettled,
}
import pachinko_double/refusals.{
  type Refusal, LeaseFenced, MarkConflict, RecordNotHeld, ReleaseRefused,
}
import pachinko_double/time.{type Timestamp}

/// The input of `Ack(consumer, lease, record_key)`. `consumer` is the
/// consumer's id and `lease` the lease's id.
pub type Ack {
  Ack(consumer: ConsumerId, lease: LeaseId, record_key: RecordKey)
}

/// The input of `Nack(consumer, lease, record_key)`. `consumer` is the
/// consumer's id and `lease` the lease's id.
pub type Nack {
  Nack(consumer: ConsumerId, lease: LeaseId, record_key: RecordKey)
}

/// The input of `Renew(consumer, lease)`. `consumer` is the consumer's id
/// and `lease` the lease's id.
pub type Renew {
  Renew(consumer: ConsumerId, lease: LeaseId)
}

/// The input of `Release(consumer, lease)`. `consumer` is the consumer's id
/// and `lease` the lease's id.
pub type Release {
  Release(consumer: ConsumerId, lease: LeaseId)
}

/// The lease an action names, where it meets the two requirements every
/// action under a lease shares: `requires: lease.status = active` and
/// `requires: lease.consumer = consumer`. An id that names no lease meets
/// neither.
fn own_lease(
  state: State,
  consumer: ConsumerId,
  lease_id: LeaseId,
) -> Result(Lease, Nil) {
  use lease <- result.try(model.lease(state.leases, lease_id))
  case lease.status == LeaseActive && lease.consumer == consumer {
    True -> Ok(lease)
    False -> Error(Nil)
  }
}

/// `ensures: lease.expires_at = now + config.lease_ttl`.
fn extend(state: State, lease: Lease, now: Timestamp) -> State {
  let extended =
    Lease(..lease, expires_at: now + config.settings(state.config).lease_ttl)
  State(..state, leases: model.replace_lease(state.leases, extended))
}

/// What a mark reads: the consumer's active lease, and the held record
/// `Record{pocket: lease.pocket, key: record_key}`. It is the record a mark
/// names, where the lease is active and the consumer's, the record `exists`
/// in the lease's pocket and `record.status = held`.
fn held(
  state: State,
  consumer: ConsumerId,
  lease_id: LeaseId,
  record_key: RecordKey,
) -> Result(HeldRecord, Nil) {
  use lease <- result.try(own_lease(state, consumer, lease_id))
  model.held_under(state.records, lease, record_key)
}

/// Whether the record a mark names is held under the consumer's active
/// lease with this mark: the guard the four mark rules share, each with its
/// own mark.
fn held_as(
  state: State,
  consumer: ConsumerId,
  lease_id: LeaseId,
  record_key: RecordKey,
  mark: Mark,
) -> Bool {
  case held(state, consumer, lease_id, record_key) {
    Error(Nil) -> False
    Ok(held) -> held.mark == mark
  }
}

/// A mark rule's effect over what its guard saw. Where that is missing,
/// which only a wrong guard could bring about, the outcome is a defect.
fn mark(
  state: State,
  consumer: ConsumerId,
  lease_id: LeaseId,
  record_key: RecordKey,
  effect: fn(HeldRecord) -> Plan,
) -> Plan {
  case held(state, consumer, lease_id, record_key) {
    Error(Nil) -> {
      let RecordKey(key) = record_key
      let LeaseId(lease) = lease_id
      Impossible("no held record " <> key <> " under lease " <> lease)
    }
    Ok(held) -> effect(held)
  }
}

/// `ensures: pocket.consecutive_expiries = 0`, where `pocket` is
/// `lease.pocket`: the pocket with that key. The mark rules require nothing
/// of the pocket, so it is not looked up: no other pocket changes and none
/// moves.
fn reset_expiries(state: State, pocket_key: PocketKey) -> State {
  State(
    ..state,
    pockets: list.map(state.pockets, fn(pocket) {
      case pocket.key == pocket_key {
        True -> Pocket(..pocket, consecutive_expiries: 0)
        False -> pocket
      }
    }),
  )
}

/// `ensures: record.mark = mark`, with the nack time the rule ensures or
/// leaves. The held version and the arrival position stay.
fn remark(
  state: State,
  held: HeldRecord,
  mark: Mark,
  nacked_at: Option(Timestamp),
) -> State {
  let marked =
    Record(
      ..held.record,
      status: RecordHeld(
        held_rank: held.held_rank,
        held_payload: held.held_payload,
        mark:,
        nacked_at:,
        deferred_position: held.deferred_position,
      ),
    )
  State(..state, records: model.replace_record(state.records, marked))
}

/// What a mark no rule covers is refused with. The specification rejects it
/// without naming the rejection, so the names are the double's, and so is
/// the order: the first requirement that fails, in the order the rules
/// state them. The lease first: one that is not active, is not the
/// consumer's, or was never issued is fenced, whatever key the mark names.
/// Then the record: the lease's pocket holds no held record with that key.
/// Otherwise the record is held and carries the other mark, which stands.
fn mark_refusal(
  state: State,
  action: String,
  consumer: ConsumerId,
  lease_id: LeaseId,
  record_key: RecordKey,
) -> Refusal {
  case own_lease(state, consumer, lease_id) {
    Error(Nil) -> LeaseFenced(action:, consumer_id: consumer, lease_id:)
    Ok(_lease) ->
      case held(state, consumer, lease_id, record_key) {
        Error(Nil) -> RecordNotHeld(action:, lease_id:, record_key:)
        Ok(_held) -> MarkConflict(action:, lease_id:, record_key:)
      }
  }
}

/// `Ack`. Two rules share it, split by the held record's mark: unsettled,
/// and already acked. No rule covers an ack of a nacked record, so that ack
/// is rejected, as is one whose lease or record fails the rules' shared
/// requirements.
pub fn ack() -> Table(Ack) {
  Table(
    "Ack",
    [ack_marks_record(), ack_repeats()],
    Refuses(fn(state, ack: Ack) {
      mark_refusal(state, "Ack", ack.consumer, ack.lease, ack.record_key)
    }),
  )
}

/// `AckMarksRecord`. The held record is marked acked and nothing is
/// removed: it stays in the held set until the lease ends. The lease is
/// extended, and the ack is progress, so the pocket's count of consecutive
/// expiries is reset.
fn ack_marks_record() -> Row(Ack) {
  Row(
    "AckMarksRecord",
    fn(state: State, ack: Ack) {
      held_as(state, ack.consumer, ack.lease, ack.record_key, Unsettled)
    },
    fn(state, ack) {
      use held <- mark(state, ack.consumer, ack.lease, ack.record_key)
      use now <- Now
      Finish(
        state
          |> extend(held.lease, now)
          |> reset_expiries(held.lease.pocket)
          |> remark(held, Acked, held.nacked_at),
        [],
        None,
      )
    },
  )
}

/// `AckRepeats`. The first mark stands: a repeat of it succeeds and changes
/// nothing but the lease's `expires_at`.
fn ack_repeats() -> Row(Ack) {
  Row(
    "AckRepeats",
    fn(state: State, ack: Ack) {
      held_as(state, ack.consumer, ack.lease, ack.record_key, Acked)
    },
    fn(state, ack) {
      use held <- mark(state, ack.consumer, ack.lease, ack.record_key)
      use now <- Now
      Finish(extend(state, held.lease, now), [], None)
    },
  )
}

/// `Nack`. Two rules share it, split by the held record's mark: unsettled,
/// and already nacked. No rule covers a nack of an acked record, so that
/// nack is rejected, as is one whose lease or record fails the rules'
/// shared requirements.
pub fn nack() -> Table(Nack) {
  Table(
    "Nack",
    [nack_marks_record(), nack_repeats()],
    Refuses(fn(state, nack: Nack) {
      mark_refusal(state, "Nack", nack.consumer, nack.lease, nack.record_key)
    }),
  )
}

/// `NackMarksRecord`. The held record is marked nacked, with the time the
/// nack was processed, and nothing is removed: the dead letter is made when
/// the lease ends. The lease is extended and the pocket's count of
/// consecutive expiries is reset, as for an ack.
fn nack_marks_record() -> Row(Nack) {
  Row(
    "NackMarksRecord",
    fn(state: State, nack: Nack) {
      held_as(state, nack.consumer, nack.lease, nack.record_key, Unsettled)
    },
    fn(state, nack) {
      use held <- mark(state, nack.consumer, nack.lease, nack.record_key)
      use now <- Now
      Finish(
        state
          |> extend(held.lease, now)
          |> reset_expiries(held.lease.pocket)
          |> remark(held, Nacked, Some(now)),
        [],
        None,
      )
    },
  )
}

/// `NackRepeats`. The first mark stands, and so does its time: a repeat
/// changes nothing but the lease's `expires_at`.
fn nack_repeats() -> Row(Nack) {
  Row(
    "NackRepeats",
    fn(state: State, nack: Nack) {
      held_as(state, nack.consumer, nack.lease, nack.record_key, Nacked)
    },
    fn(state, nack) {
      use held <- mark(state, nack.consumer, nack.lease, nack.record_key)
      use now <- Now
      Finish(extend(state, held.lease, now), [], None)
    },
  )
}

/// `Renew`. One rule has the trigger, so a renew its guard does not hold
/// for is rejected: the lease is not active, is not the consumer's, or was
/// never issued. The rule sets the lease's `expires_at` to `now` plus
/// `lease_ttl` and nothing else: a renew is not a new grant, and renewals do
/// not add up.
pub fn renew() -> Table(Renew) {
  Table(
    "Renew",
    [
      Row(
        "Renew",
        fn(state: State, renew: Renew) {
          result.is_ok(own_lease(state, renew.consumer, renew.lease))
        },
        fn(state, renew) {
          case own_lease(state, renew.consumer, renew.lease) {
            Error(Nil) -> {
              let LeaseId(lease) = renew.lease
              let ConsumerId(consumer) = renew.consumer
              Impossible(
                "no active lease " <> lease <> " of consumer " <> consumer,
              )
            }
            Ok(lease) -> {
              use now <- Now
              Finish(extend(state, lease, now), [], None)
            }
          }
        },
      ),
    ],
    Refuses(fn(_state, renew: Renew) {
      LeaseFenced(
        action: "Renew",
        consumer_id: renew.consumer,
        lease_id: renew.lease,
      )
    }),
  )
}

/// `Release`. Two rules share it, split by whether a record of the pocket
/// outlasts the lease. A release whose lease is not active, is not the
/// consumer's, or was never issued is fenced. In a state that holds the
/// invariants the pocket of an active lease is leased, so one of the two
/// rules holds for every other release, and the refusal past fencing is
/// never met through the owner.
pub fn release() -> Table(Release) {
  Table(
    "Release",
    [release_requeues_pocket(), release_idles_empty_pocket()],
    Refuses(fn(state, release: Release) {
      case own_lease(state, release.consumer, release.lease) {
        Error(Nil) ->
          LeaseFenced(
            action: "Release",
            consumer_id: release.consumer,
            lease_id: release.lease,
          )
        Ok(_lease) ->
          ReleaseRefused(consumer_id: release.consumer, lease_id: release.lease)
      }
    }),
  )
}

/// The consumer's active lease and `lease.pocket`, where `pocket.status =
/// leased`: what the two release rules share before they split.
fn leased(state: State, release: Release) -> Result(#(Lease, Pocket), Nil) {
  use lease <- result.try(own_lease(state, release.consumer, release.lease))
  use pocket <- result.try(model.pocket(state.pockets, lease.pocket))
  case pocket.status == PocketLeased {
    True -> Ok(#(lease, pocket))
    False -> Error(Nil)
  }
}

/// A release rule's effect over what its guard saw. Where that is missing,
/// which only a wrong guard could bring about, the outcome is a defect.
fn releasing(
  state: State,
  release: Release,
  effect: fn(Lease, Pocket) -> Plan,
) -> Plan {
  case leased(state, release) {
    Error(Nil) -> {
      let LeaseId(lease) = release.lease
      let ConsumerId(consumer) = release.consumer
      Impossible(
        "no active lease "
        <> lease
        <> " of consumer "
        <> consumer
        <> " on a leased pocket",
      )
    }
    Ok(#(lease, pocket)) -> effect(lease, pocket)
  }
}

/// `ReleaseRequeuesPocket`. A record outlasts the lease, so the pocket is
/// queued again, at a fresh position at the back of the pocket order, with
/// its count of consecutive expiries reset. The lease ends through
/// `lease_end`, which sends `Freed` to each record of the held set.
fn release_requeues_pocket() -> Row(Release) {
  Row(
    "ReleaseRequeuesPocket",
    fn(state: State, release: Release) {
      case leased(state, release) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) -> lease_end.outlasts(state, pocket.key)
      }
    },
    fn(state, release) {
      use lease, pocket <- releasing(state, release)
      use position <- Position
      use now <- Now
      let #(state, freed) = lease_end.end(state, lease, lease_end.Release, now)
      let queued =
        Pocket(
          ..pocket,
          status: PocketQueued,
          position:,
          consecutive_expiries: 0,
        )
      Finish(
        State(..state, pockets: model.replace_pocket(state.pockets, queued)),
        freed,
        None,
      )
    },
  )
}

/// `ReleaseIdlesEmptyPocket`. No record outlasts the lease: every record
/// the pocket has is held, acked or nacked, with nothing deferred behind
/// it, and the `Freed` each is sent removes it. The pocket goes idle, with
/// its count reset. It is allotted no position and keeps the one it had.
fn release_idles_empty_pocket() -> Row(Release) {
  Row(
    "ReleaseIdlesEmptyPocket",
    fn(state: State, release: Release) {
      case leased(state, release) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) -> !lease_end.outlasts(state, pocket.key)
      }
    },
    fn(state, release) {
      use lease, pocket <- releasing(state, release)
      use now <- Now
      let #(state, freed) = lease_end.end(state, lease, lease_end.Release, now)
      let idle = Pocket(..pocket, status: PocketIdle, consecutive_expiries: 0)
      Finish(
        State(..state, pockets: model.replace_pocket(state.pockets, idle)),
        freed,
        None,
      )
    },
  )
}
