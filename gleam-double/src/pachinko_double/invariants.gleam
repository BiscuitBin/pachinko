//// The invariant checker: the registry from each invariant's name to its
//// check, and `check`, which the apply step calls once per outcome.
////
//// Each check is a direct reading of the specification's expression, which
//// is quoted above it. It reads the snapshot and the snapshot's derived
//// sets, and nothing else. `check` computes the derived sets once, with
//// `inspection.views`, and hands them to every check; a check that reads
//// one, such as a pocket's `active_leases`, reads it from there and never
//// builds its own.
////
//// How the expressions are read:
////
//// - `x implies y` is true when `x` is false.
//// - `a != b` between two members of one collection means two members, not
////   two unequal values: members are told apart by their place in the
////   snapshot's list, so two publishers with equal fields are still two.
////   An expression with no `a != b` ranges over every pair, a member
////   paired with itself included.
//// - The fields declared `when status = held` belong to the held variant
////   alone, so `status = held` is the match on that variant and a queued
////   record has no such field to compare. `ended_at` is the same for a
////   lease that has ended.
//// - `= null` and `!= null` are `None` and `Some`.
//// - A reference is a key: a record's `pocket` is the pocket's key and a
////   lease's `consumer` the consumer's id. Two references are equal when
////   their keys are. An entity's own collection, such as a pocket's
////   `records` or a consumer's `leases`, is every member whose reference
////   holds that entity's key; `inspection.views` builds them so, one view
////   for each entity. Reading a field through a reference, as in
////   `r.pocket.status`, reads it from every entity holding that key, and
////   the check must hold for each: one entity in every state the rules can
////   reach, several in a hand-built snapshot where a key is repeated, and
////   none where the reference names nothing. So a field read through a
////   reference that names nothing asks nothing, and the invariant holds of
////   that record; and a lease whose pocket or consumer does not exist is
////   counted for no pocket and no consumer. Two references are still
////   compared as keys whether or not anything holds the key: two active
////   leases over one key break `OneActiveLeasePerPocket` even where no
////   pocket has it. The specification has no invariant that a reference
////   names an entity, so the checker reports none.

import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/inspection.{type Snapshot, type Views}
import pachinko_double/model.{
  type Pocket, type Record, LeaseActive, LeaseExpired, LeaseReleased, Nacked,
  PocketIdle, PocketLeased, RecordHeld, RecordQueued, Unsettled,
}

/// One invariant: its name in the specification, and whether it holds of a
/// snapshot and that snapshot's derived sets.
pub type Invariant {
  Invariant(name: String, holds: fn(Snapshot, Views) -> Bool)
}

/// Every invariant of the specification, in the specification's order.
pub fn registry() -> List(Invariant) {
  [
    Invariant("LeasedHasOneActiveLease", leased_has_one_active_lease),
    Invariant("UnleasedHasNoActiveLease", unleased_has_no_active_lease),
    Invariant("IdleHasNoRecords", idle_has_no_records),
    Invariant("ExpiriesWithinLimit", expiries_within_limit),
    Invariant("HeldVersionNotNewer", held_version_not_newer),
    Invariant("NackedHasItsTime", nacked_has_its_time),
    Invariant("OnlyNackedHasNackTime", only_nacked_has_nack_time),
    Invariant("DeferredHasArrivalPosition", deferred_has_arrival_position),
    Invariant(
      "OnlyDeferredHasArrivalPosition",
      only_deferred_has_arrival_position,
    ),
    Invariant("DeferredArrivedAfterHeld", deferred_arrived_after_held),
    Invariant("HeldCopiesAgree", held_copies_agree),
    Invariant("ShedBeforeExpiry", shed_before_expiry),
    Invariant("NackedBeforeExpiry", nacked_before_expiry),
    Invariant("GrantedBeforeExpiry", granted_before_expiry),
    Invariant("EndedAfterGrant", ended_after_grant),
    Invariant("ExpiryLimitPositive", expiry_limit_positive),
    Invariant("PullBatchPositive", pull_batch_positive),
    Invariant("LeaseTtlNonNegative", lease_ttl_non_negative),
    Invariant("SupersededTtlNonNegative", superseded_ttl_non_negative),
    Invariant("DeadLetterTtlNonNegative", dead_letter_ttl_non_negative),
    Invariant("UniquePublisherId", unique_publisher_id),
    Invariant("UniqueConsumerId", unique_consumer_id),
    Invariant("OnePocketPerKey", one_pocket_per_key),
    Invariant("OneRecordPerKeyPerPocket", one_record_per_key_per_pocket),
    Invariant("OneActiveLeasePerPocket", one_active_lease_per_pocket),
    Invariant("OneActiveLeasePerConsumer", one_active_lease_per_consumer),
    Invariant("OutstandingOnlyWhileLeased", outstanding_only_while_leased),
    Invariant("MarkedPocketHasNoExpiries", marked_pocket_has_no_expiries),
    Invariant("UniquePocketPosition", unique_pocket_position),
    Invariant("UniqueRecordPosition", unique_record_position),
    Invariant("PocketPositionsBelowNext", pocket_positions_below_next),
    Invariant("RecordPositionsBelowNext", record_positions_below_next),
    Invariant("ArrivalPositionUnoccupied", arrival_position_unoccupied),
    Invariant("UniqueArrivalPosition", unique_arrival_position),
    Invariant("ArrivalPositionsBelowNext", arrival_positions_below_next),
  ]
}

/// The names of the invariants the snapshot breaks, in the registry's order.
/// The derived sets are computed here, once, for all of them.
pub fn check(snapshot: Snapshot) -> List(String) {
  let views = inspection.views(snapshot)
  registry()
  |> list.filter(fn(invariant) { !invariant.holds(snapshot, views) })
  |> list.map(fn(invariant) { invariant.name })
}

/// `premise implies conclusion`.
fn implies(premise: Bool, conclusion: Bool) -> Bool {
  !premise || conclusion
}

/// `members.count`.
fn count(members: List(member)) -> Int {
  list.length(members)
}

// Pocket ---------------------------------------------------

// `active_leases` and `records` are the pocket's derived sets, read from its
// view.

// status = leased implies active_leases.count = 1
fn leased_has_one_active_lease(_snapshot: Snapshot, views: Views) -> Bool {
  use view <- list.all(views.pockets)
  implies(view.pocket.status == PocketLeased, count(view.active_leases) == 1)
}

// status != leased implies active_leases.count = 0
fn unleased_has_no_active_lease(_snapshot: Snapshot, views: Views) -> Bool {
  use view <- list.all(views.pockets)
  implies(view.pocket.status != PocketLeased, count(view.active_leases) == 0)
}

// status = idle implies records.count = 0
fn idle_has_no_records(_snapshot: Snapshot, views: Views) -> Bool {
  use view <- list.all(views.pockets)
  implies(view.pocket.status == PocketIdle, count(view.records) == 0)
}

// consecutive_expiries <= config.max_consecutive_expiries
fn expiries_within_limit(snapshot: Snapshot, _views: Views) -> Bool {
  use pocket <- list.all(snapshot.pockets)
  pocket.consecutive_expiries <= snapshot.config.max_consecutive_expiries
}

// Record ---------------------------------------------------
// `status = held` is the `RecordHeld` branch. A queued record makes the
// premise false, so the invariant holds of it.

// status = held implies held_rank <= rank
fn held_version_not_newer(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(held_rank:, ..) -> held_rank <= record.rank
  }
}

// status = held and mark = nacked implies nacked_at != null
fn nacked_has_its_time(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(mark:, nacked_at:, ..) ->
      implies(mark == Nacked, nacked_at != None)
  }
}

// status = held and mark != nacked implies nacked_at = null
fn only_nacked_has_nack_time(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(mark:, nacked_at:, ..) ->
      implies(mark != Nacked, nacked_at == None)
  }
}

// status = held and held_rank < rank implies deferred_position != null
fn deferred_has_arrival_position(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(held_rank:, deferred_position:, ..) ->
      implies(held_rank < record.rank, deferred_position != None)
  }
}

// status = held and held_rank = rank implies deferred_position = null
fn only_deferred_has_arrival_position(
  snapshot: Snapshot,
  _views: Views,
) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(held_rank:, deferred_position:, ..) ->
      implies(held_rank == record.rank, deferred_position == None)
  }
}

/// A record's `deferred_position` where `status = held and
/// deferred_position != null`, and `None` where that is false.
fn arrival(record: Record) -> Option(Int) {
  case record.status {
    RecordQueued -> None
    RecordHeld(deferred_position:, ..) -> deferred_position
  }
}

// status = held and deferred_position != null
//     implies deferred_position > position
fn deferred_arrived_after_held(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case arrival(record) {
    None -> True
    Some(deferred_position) -> deferred_position > record.position
  }
}

// status = held and deferred_position = null implies held_payload = payload
fn held_copies_agree(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case record.status {
    RecordQueued -> True
    RecordHeld(held_payload:, deferred_position:, ..) ->
      implies(deferred_position == None, held_payload == record.payload)
  }
}

// SupersededRecord, DeadLetter and Lease -----------------

// shed_at <= expires_at
fn shed_before_expiry(snapshot: Snapshot, _views: Views) -> Bool {
  use shed <- list.all(snapshot.superseded)
  shed.shed_at <= shed.expires_at
}

// nacked_at <= expires_at
fn nacked_before_expiry(snapshot: Snapshot, _views: Views) -> Bool {
  use letter <- list.all(snapshot.dead_letters)
  letter.nacked_at <= letter.expires_at
}

// granted_at <= expires_at
fn granted_before_expiry(snapshot: Snapshot, _views: Views) -> Bool {
  use lease <- list.all(snapshot.leases)
  lease.granted_at <= lease.expires_at
}

// status in {released, expired} implies ended_at >= granted_at
fn ended_after_grant(snapshot: Snapshot, _views: Views) -> Bool {
  use lease <- list.all(snapshot.leases)
  case lease.status {
    LeaseActive -> True
    LeaseReleased(ended_at:) | LeaseExpired(ended_at:) ->
      ended_at >= lease.granted_at
  }
}

// Config ---------------------------------------------------
// These read the snapshot's plain settings. A duration is whole
// milliseconds, so `0.seconds` is 0.

// config.max_consecutive_expiries >= 1
fn expiry_limit_positive(snapshot: Snapshot, _views: Views) -> Bool {
  snapshot.config.max_consecutive_expiries >= 1
}

// config.max_pull_batch >= 1
fn pull_batch_positive(snapshot: Snapshot, _views: Views) -> Bool {
  snapshot.config.max_pull_batch >= 1
}

// config.lease_ttl >= 0.seconds
fn lease_ttl_non_negative(snapshot: Snapshot, _views: Views) -> Bool {
  snapshot.config.lease_ttl >= 0
}

// config.superseded_ttl >= 0.seconds
fn superseded_ttl_non_negative(snapshot: Snapshot, _views: Views) -> Bool {
  snapshot.config.superseded_ttl >= 0
}

// config.dead_letter_ttl >= 0.seconds
fn dead_letter_ttl_non_negative(snapshot: Snapshot, _views: Views) -> Bool {
  snapshot.config.dead_letter_ttl >= 0
}

// One per key ----------------------------------------------

/// `for a in members: for b in members: a != b implies holds(a, b)`. Two
/// members are distinct when they stand at different places in the list,
/// whatever their fields.
fn distinct(members: List(member), holds: fn(member, member) -> Bool) -> Bool {
  let placed = list.index_map(members, fn(member, place) { #(place, member) })
  use #(place_a, a) <- list.all(placed)
  use #(place_b, b) <- list.all(placed)
  implies(place_a != place_b, holds(a, b))
}

// for a in Publishers: for b in Publishers:
//     a != b implies a.id != b.id
fn unique_publisher_id(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.publishers)
  a.id != b.id
}

// for a in Consumers: for b in Consumers:
//     a != b implies a.id != b.id
fn unique_consumer_id(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.consumers)
  a.id != b.id
}

// for a in Pockets: for b in Pockets:
//     a != b implies a.key != b.key
fn one_pocket_per_key(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.pockets)
  a.key != b.key
}

// for a in Records: for b in Records:
//     a != b and a.pocket = b.pocket implies a.key != b.key
fn one_record_per_key_per_pocket(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.records)
  implies(a.pocket == b.pocket, a.key != b.key)
}

// Leases, and what a record asks of its pocket -------------

// for a in Leases: for b in Leases:
//     a != b and a.status = active and b.status = active
//         implies a.pocket != b.pocket
fn one_active_lease_per_pocket(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.leases)
  implies(
    a.status == LeaseActive && b.status == LeaseActive,
    a.pocket != b.pocket,
  )
}

// for c in Consumers: c.active_leases.count <= 1
// `active_leases` is the consumer's derived set, read from its view.
fn one_active_lease_per_consumer(_snapshot: Snapshot, views: Views) -> Bool {
  use view <- list.all(views.consumers)
  count(view.active_leases) <= 1
}

/// The pockets a record's `pocket` names: every pocket holding that key.
/// See the module's header for none and for several.
fn pockets_of(snapshot: Snapshot, record: Record) -> List(Pocket) {
  list.filter(snapshot.pockets, fn(pocket) { pocket.key == record.pocket })
}

// for r in Records: r.status = held implies r.pocket.status = leased
fn outstanding_only_while_leased(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  use pocket <- list.all(pockets_of(snapshot, record))
  case record.status {
    RecordQueued -> True
    RecordHeld(..) -> pocket.status == PocketLeased
  }
}

// for r in Records: r.status = held and r.mark != unsettled
//     implies r.pocket.consecutive_expiries = 0
fn marked_pocket_has_no_expiries(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  use pocket <- list.all(pockets_of(snapshot, record))
  case record.status {
    RecordQueued -> True
    RecordHeld(mark:, ..) ->
      implies(mark != Unsettled, pocket.consecutive_expiries == 0)
  }
}

// Positions ------------------------------------------------
// `queue.next_position` is the snapshot's `next_position`.

// for a in Pockets: for b in Pockets:
//     a != b implies a.position != b.position
fn unique_pocket_position(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.pockets)
  a.position != b.position
}

// for a in Records: for b in Records:
//     a != b implies a.position != b.position
fn unique_record_position(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.records)
  a.position != b.position
}

// for p in Pockets: p.position < queue.next_position
fn pocket_positions_below_next(snapshot: Snapshot, _views: Views) -> Bool {
  use pocket <- list.all(snapshot.pockets)
  pocket.position < snapshot.next_position
}

// for r in Records: r.position < queue.next_position
fn record_positions_below_next(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  record.position < snapshot.next_position
}

// Arrival positions ----------------------------------------
// `arrival`, above, is `Some` exactly where `status = held and
// deferred_position != null`.

/// `for a in members: for b in members: holds(a, b)`: every pair, a member
/// paired with itself included.
fn every_pair(
  members: List(member),
  holds: fn(member, member) -> Bool,
) -> Bool {
  use a <- list.all(members)
  use b <- list.all(members)
  holds(a, b)
}

// for a in Records: for b in Records:
//     a.status = held and a.deferred_position != null
//         implies a.deferred_position != b.position
fn arrival_position_unoccupied(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- every_pair(snapshot.records)
  case arrival(a) {
    None -> True
    Some(deferred_position) -> deferred_position != b.position
  }
}

// for a in Records: for b in Records:
//     a != b and a.status = held and b.status = held
//         and a.deferred_position != null and b.deferred_position != null
//         implies a.deferred_position != b.deferred_position
fn unique_arrival_position(snapshot: Snapshot, _views: Views) -> Bool {
  use a, b <- distinct(snapshot.records)
  case arrival(a), arrival(b) {
    Some(deferred_position_a), Some(deferred_position_b) ->
      deferred_position_a != deferred_position_b
    _, _ -> True
  }
}

// for r in Records: r.status = held and r.deferred_position != null
//     implies r.deferred_position < queue.next_position
fn arrival_positions_below_next(snapshot: Snapshot, _views: Views) -> Bool {
  use record <- list.all(snapshot.records)
  case arrival(record) {
    None -> True
    Some(deferred_position) -> deferred_position < snapshot.next_position
  }
}
