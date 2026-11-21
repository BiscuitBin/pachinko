//// The specification's "Lease expiry" section: `LeaseLapses`, the rule the
//// clock triggers for an active lease whose `expires_at` has passed, and the
//// three rules that answer the `LeaseLapsed` it emits.
////
//// Only a `Tick` reaches these tables as a trigger. It reads `now` once and
//// offers `LeaseLapses` only the active leases whose `expires_at <= now`, so
//// that comparison is the tick's filter, not a guard here. Every outcome a
//// tick makes runs with that `now` already in hand, so a row here that asks
//// for `now` is answered without a read.
////
//// Each expiry rule ends the lease through `lease_end`, which owns what a
//// lease's end does and sends `Freed` for each held record, and decides the
//// pocket's fate with `lease_end.outlasts`. No expiry rule allots a
//// position: a pocket kept at the front keeps the position it had.

import gleam/option.{None}
import gleam/result
import pachinko_double/config
import pachinko_double/domain.{type Plan, type Row, type Table, LeaseLapsed}
import pachinko_double/lease_end
import pachinko_double/model.{
  type Lease, type Pocket, type PocketStatus, type State, LeaseActive, Pocket,
  PocketIdle, PocketLeased, PocketParked, PocketQueued, State,
}
import pachinko_double/rules.{
  Finish, Impossible, NothingHappens, Now, Row, Table,
}
import pachinko_double/time.{type Timestamp}

/// The input of both tables: the lease's id, and the `now` of the tick that
/// found it due. For `LeaseLapses` it is the clock trigger `lease:
/// Lease.expires_at <= now`; for the expiry rules it is `LeaseLapsed(lease)`
/// with the instant of the outcome that emitted it.
pub type Lapse {
  Lapse(lease: String, now: Timestamp)
}

/// The clock trigger `lease: Lease.expires_at <= now`. Its one rule,
/// `LeaseLapses`, holds while the lease is active. A tick offers only active
/// leases whose `expires_at` has passed, so when no guard holds nothing
/// happens (spec note N37).
pub fn lease_lapses() -> Table(Lapse) {
  Table("LeaseLapses", [lease_lapses_rule()], NothingHappens)
}

/// `LeaseLapsed(lease)`. The specification gives it three rules with
/// disjoint guards, here in its order. Under branch A (decision 0019) a
/// lapse processed after an action that ended the lease or moved its
/// `expires_at` on stops at its guards, so when no guard holds nothing
/// happens.
pub fn lease_lapsed() -> Table(Lapse) {
  Table(
    "LeaseLapsed",
    [
      lease_expiry_parks_pocket(),
      lease_expiry_keeps_pocket_at_front(),
      lease_expiry_idles_empty_pocket(),
    ],
    NothingHappens,
  )
}

/// `LeaseLapses`: `requires: lease.status = active`, `ensures:
/// LeaseLapsed(lease: lease)`. The event carries the outcome's `now`.
fn lease_lapses_rule() -> Row(Lapse) {
  Row(
    "LeaseLapses",
    fn(state: State, lapse: Lapse) {
      case model.lease(state.leases, lapse.lease) {
        Error(Nil) -> False
        Ok(lease) -> lease.status == LeaseActive
      }
    },
    fn(state, lapse) {
      case model.lease(state.leases, lapse.lease) {
        Error(Nil) -> Impossible("no lease " <> lapse.lease)
        Ok(lease) -> {
          use now <- Now
          Finish(state, [LeaseLapsed(lease: lease.id, now:)], None)
        }
      }
    },
  )
}

/// What the three expiry rules share before they split:
///
/// ```text
/// requires: lease.status = active
/// requires: lease.expires_at <= now
/// let pocket = lease.pocket
/// requires: pocket.status = leased
/// ```
///
/// with `now` the instant the event carries.
fn lapsing(state: State, lapse: Lapse) -> Result(#(Lease, Pocket), Nil) {
  use lease <- result.try(model.lease(state.leases, lapse.lease))
  use pocket <- result.try(model.pocket(state.pockets, lease.pocket))
  case
    lease.status == LeaseActive
    && lease.expires_at <= lapse.now
    && pocket.status == PocketLeased
  {
    True -> Ok(#(lease, pocket))
    False -> Error(Nil)
  }
}

/// `pocket.consecutive_expiries + 1 >= config.max_consecutive_expiries`:
/// whether this lapse is the pocket's parking one.
fn parks(state: State, pocket: Pocket) -> Bool {
  pocket.consecutive_expiries + 1
  >= config.settings(state.config).max_consecutive_expiries
}

/// `LeaseExpiryParksPocket`. At the threshold the pocket parks, whether or
/// not a record outlasts the lease.
fn lease_expiry_parks_pocket() -> Row(Lapse) {
  Row(
    "LeaseExpiryParksPocket",
    fn(state: State, lapse: Lapse) {
      case lapsing(state, lapse) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) -> parks(state, pocket)
      }
    },
    fn(state, lapse) { expire(state, lapse, PocketParked) },
  )
}

/// `LeaseExpiryKeepsPocketAtFront`. Below the threshold, with a record that
/// outlasts the lease, the pocket is queued again at the position it had,
/// which is the front.
fn lease_expiry_keeps_pocket_at_front() -> Row(Lapse) {
  Row(
    "LeaseExpiryKeepsPocketAtFront",
    fn(state: State, lapse: Lapse) {
      case lapsing(state, lapse) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) ->
          !parks(state, pocket) && lease_end.outlasts(state, pocket.key)
      }
    },
    fn(state, lapse) { expire(state, lapse, PocketQueued) },
  )
}

/// `LeaseExpiryIdlesEmptyPocket`. Below the threshold, with no record that
/// outlasts the lease, the pocket goes idle. It keeps the position it had.
fn lease_expiry_idles_empty_pocket() -> Row(Lapse) {
  Row(
    "LeaseExpiryIdlesEmptyPocket",
    fn(state: State, lapse: Lapse) {
      case lapsing(state, lapse) {
        Error(Nil) -> False
        Ok(#(_lease, pocket)) ->
          !parks(state, pocket) && !lease_end.outlasts(state, pocket.key)
      }
    },
    fn(state, lapse) { expire(state, lapse, PocketIdle) },
  )
}

/// The effect the three expiry rules share, with the pocket's new status:
///
/// ```text
/// ensures: lease.status = expired
/// ensures: lease.ended_at = now
/// ensures:
///     for record in pocket.held_records:
///         Freed(lease: lease, record_key: record.key, ended_at: now)
/// ensures: pocket.consecutive_expiries = pocket.consecutive_expiries + 1
/// ensures: pocket.status = parked        -- or queued, or idle
/// ```
///
/// The lease ends through `lease_end.end`, at the outcome's `now`. The
/// pocket keeps its position whatever its status.
fn expire(state: State, lapse: Lapse, status: PocketStatus) -> Plan {
  case lapsing(state, lapse) {
    Error(Nil) ->
      Impossible(
        "no active, due lease " <> lapse.lease <> " on a leased pocket",
      )
    Ok(#(lease, pocket)) -> {
      use now <- Now
      let #(state, freed) = lease_end.end(state, lease, lease_end.Lapse, now)
      let ended =
        Pocket(
          ..pocket,
          status:,
          consecutive_expiries: pocket.consecutive_expiries + 1,
        )
      Finish(
        State(..state, pockets: model.replace_pocket(state.pockets, ended)),
        freed,
        None,
      )
    }
  }
}
