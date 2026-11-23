//// The specification's Pull section: the Pull table and the `Handed`
//// table.

import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import pachinko_double/config
import pachinko_double/domain.{
  type Plan, type Row, type Table, Batch, BatchRecord, Handed, NoPocketAvailable,
}
import pachinko_double/model.{
  type Lease, type Pocket, type Record, type State, Lease, LeaseActive, Pocket,
  PocketLeased, PocketQueued, Record, RecordHeld, RecordQueued, State, Unsettled,
}
import pachinko_double/refusals.{PullRefused}
import pachinko_double/rules.{
  Defective, Finish, Impossible, LeaseId, Now, Refuses, Row, Table,
}

/// The input of `Pull(consumer, limit)`. `consumer` is the consumer's id.
pub type Pull {
  Pull(consumer: String, limit: Int)
}

/// The input of `Handed(pocket, record_key)`. `pocket_key` is the pocket's
/// key.
pub type Handover {
  Handover(pocket_key: String, record_key: String)
}

/// `Pull`. Three rules share it. In a state that holds the invariants
/// exactly one of their guards holds, so the refusal here is never met
/// through the owner.
pub fn pull() -> Table(Pull) {
  Table(
    "Pull",
    [pull_within_lease(), pull_claims_front_pocket(), pull_finds_no_pocket()],
    Refuses(fn(_state, pull: Pull) { PullRefused(pull.consumer) }),
  )
}

/// `Lease{consumer: consumer, status: active}`.
fn active_lease(state: State, pull: Pull) -> Result(Lease, Nil) {
  state.leases
  |> model.leases_of_consumer(pull.consumer)
  |> model.active
  |> list.first
}

/// `Pockets where status = queued`.
fn free(state: State) -> List(Pocket) {
  list.filter(state.pockets, fn(pocket) { pocket.status == PocketQueued })
}

/// `min_by(free, p => p.position)`: the queued pocket with the smallest
/// position, which is the front of the pocket order.
fn front(state: State) -> Result(Pocket, Nil) {
  free(state)
  |> list.sort(fn(a, b) { int.compare(a.position, b.position) })
  |> list.first
}

/// `oldest(pocket.queued_records, r => r.position, min(limit,
/// config.max_pull_batch))`: that many of the pocket's queued records, the
/// smallest positions first; all of them when there are fewer, and none
/// when the number is not positive.
fn batch(state: State, pocket: Pocket, limit: Int) -> List(Record) {
  state.records
  |> model.records_of(pocket.key)
  |> model.queued
  |> model.oldest_first
  |> list.take(int.min(limit, config.settings(state.config).max_pull_batch))
}

/// The end of a pull that hands out a batch: `Handed` for each record of
/// the batch, in the batch's order, and the `Batch` answer, which names the
/// lease the records were handed under; the consumer and the pocket are that
/// lease's. `state` is the state the rule's other `ensures` lines left. The
/// batch was computed before any `Handed` is answered, so its records carry
/// the rank and payload they had then.
fn hand_out(state: State, lease: Lease, batch: List(Record)) -> Plan {
  Finish(
    state,
    list.map(batch, fn(record) {
      Handed(pocket_key: lease.pocket, record_key: record.key)
    }),
    Some(Batch(
      consumer: lease.consumer,
      lease: lease.id,
      pocket: lease.pocket,
      records: list.map(batch, fn(record) {
        BatchRecord(key: record.key, rank: record.rank, payload: record.payload)
      }),
    )),
  )
}

/// The consumer's active lease, and `lease.pocket`.
fn lease_and_pocket(state: State, pull: Pull) -> Result(#(Lease, Pocket), Nil) {
  use lease <- result.try(active_lease(state, pull))
  use pocket <- result.map(model.pocket(state.pockets, lease.pocket))
  #(lease, pocket)
}

/// `PullWithinLease`. A consumer that holds a lease pulls from that
/// lease's pocket and from no other. The lease is extended to `now` plus
/// `lease_ttl` whatever the batch holds, an empty batch included, and its
/// `granted_at` stays.
fn pull_within_lease() -> Row(Pull) {
  Row(
    "PullWithinLease",
    fn(state: State, pull: Pull) {
      case lease_and_pocket(state, pull) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) -> pocket.status == PocketLeased
      }
    },
    fn(state, pull) {
      case lease_and_pocket(state, pull) {
        Error(Nil) ->
          Impossible(
            "no active lease of consumer " <> pull.consumer <> " on a pocket",
          )
        Ok(#(lease, pocket)) -> {
          let batch = batch(state, pocket, pull.limit)
          use now <- Now
          let extended =
            Lease(
              ..lease,
              expires_at: now + config.settings(state.config).lease_ttl,
            )
          hand_out(
            State(..state, leases: model.replace_lease(state.leases, extended)),
            extended,
            batch,
          )
        }
      }
    },
  )
}

/// `PullClaimsFrontPocket`. A consumer with no active lease claims the
/// queued pocket with the smallest position: a new lease, granted now, and
/// the pocket leased. It does so whatever the batch holds, an empty batch
/// included. It is the one rule that creates a lease, so the one that asks
/// for a lease id.
fn pull_claims_front_pocket() -> Row(Pull) {
  Row(
    "PullClaimsFrontPocket",
    fn(state: State, pull: Pull) {
      result.is_error(active_lease(state, pull))
      && free(state) != []
      && case front(state) {
        Error(Nil) -> False
        Ok(pocket) -> pocket.status == PocketQueued
      }
    },
    fn(state, pull) {
      case front(state) {
        Error(Nil) -> Impossible("no queued pocket")
        Ok(pocket) -> {
          let batch = batch(state, pocket, pull.limit)
          use now <- Now
          use lease_id <- LeaseId
          let lease =
            Lease(
              id: lease_id,
              pocket: pocket.key,
              consumer: pull.consumer,
              status: LeaseActive,
              granted_at: now,
              expires_at: now + config.settings(state.config).lease_ttl,
            )
          let leased = Pocket(..pocket, status: PocketLeased)
          hand_out(
            State(
              ..state,
              pockets: model.replace_pocket(state.pockets, leased),
              leases: list.append(state.leases, [lease]),
            ),
            lease,
            batch,
          )
        }
      }
    },
  )
}

/// `PullFindsNoPocket`. The consumer holds no lease and no pocket is queued.
/// It answers `NoPocketAvailable`, which is not a refusal, and changes
/// nothing. It ensures nothing that needs `now`, so it reads no clock.
fn pull_finds_no_pocket() -> Row(Pull) {
  Row(
    "PullFindsNoPocket",
    fn(state: State, pull: Pull) {
      result.is_error(active_lease(state, pull)) && free(state) == []
    },
    fn(state, pull) {
      Finish(state, [], Some(NoPocketAvailable(consumer: pull.consumer)))
    },
  )
}

/// `Handed(pocket, record_key)`. A pull emits it only for a record of the
/// batch it has just computed, so a `Handed` no rule answers is a defect.
pub fn handed() -> Table(Handover) {
  Table("Handed", [hold_record()], Defective)
}

/// `Record{pocket: pocket, key: record_key}`.
fn record(state: State, handover: Handover) -> Result(Record, Nil) {
  model.record(
    state.records,
    pocket: handover.pocket_key,
    key: handover.record_key,
  )
}

/// `HoldRecord`. The queued record becomes held: the held version is the
/// record's own rank and payload, nothing has been said of it, and nothing
/// is deferred behind it.
fn hold_record() -> Row(Handover) {
  Row(
    "HoldRecord",
    fn(state: State, handover: Handover) {
      case record(state, handover) {
        Error(Nil) -> False
        Ok(record) -> record.status == RecordQueued
      }
    },
    fn(state, handover) {
      case record(state, handover) {
        Error(Nil) ->
          Impossible(
            "no record "
            <> handover.record_key
            <> " in pocket "
            <> handover.pocket_key,
          )
        Ok(record) -> {
          let held =
            Record(
              ..record,
              status: RecordHeld(
                held_rank: record.rank,
                held_payload: record.payload,
                mark: Unsettled,
                nacked_at: None,
                deferred_position: None,
              ),
            )
          Finish(
            State(..state, records: model.replace_record(state.records, held)),
            [],
            None,
          )
        }
      }
    },
  )
}
