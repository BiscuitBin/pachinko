//// The invariant checker's positive controls: at least one per invariant,
//// each a snapshot built by hand, since most of these states cannot be
//// reached through the rules, and each asserting the whole list `check`
//// returns. A control that asked only whether its name was in the list
//// would pass while its own check never fired.
////
//// Every control starts from `base`, which holds at least one of every
//// entity and breaks nothing, and changes as little of it as breaks its
//// invariant. Where a state cannot break one invariant without another,
//// the control asserts both names and says why. `base` is also the near
//// miss for most checks: it holds a state on the allowed side of each
//// one, so a check that is too strict turns `the base snapshot breaks
//// nothing` red. The tests named `... breaks nothing` are the near misses
//// the base cannot hold.

import coverage
import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/config.{Settings}
import pachinko_double/inspection.{type Snapshot, Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type Lease, type Mark, type Pocket, type Record, Acked, Consumer, DeadLetter,
  Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked, Pocket, PocketIdle,
  PocketLeased, PocketParked, PocketQueued, Publisher, Record, RecordHeld,
  RecordQueued, Superseded, SupersededRecord, Unsettled,
}
import pachinko_double/time.{type Timestamp}
import runner.{type Test, InvariantControl, Test}

pub fn tests() -> List(Test) {
  [
    Test("the base snapshot breaks nothing", [], base_breaks_nothing),
    Test("an empty queue breaks nothing", [], empty_breaks_nothing),
    Test(
      "LeasedHasOneActiveLease: a leased pocket with no active lease",
      [InvariantControl("LeasedHasOneActiveLease")],
      leased_without_active_lease,
    ),
    Test(
      "UnleasedHasNoActiveLease: a pocket not leased with an active lease",
      [InvariantControl("UnleasedHasNoActiveLease")],
      unleased_with_active_lease,
    ),
    Test(
      "IdleHasNoRecords: an idle pocket with a record",
      [InvariantControl("IdleHasNoRecords")],
      idle_with_record,
    ),
    Test(
      "ExpiriesWithinLimit: one lapse more than the limit",
      [InvariantControl("ExpiriesWithinLimit")],
      expiries_over_limit,
    ),
    Test(
      "ExpiriesWithinLimit: a pocket at the limit breaks nothing",
      [],
      expiries_at_limit,
    ),
    Test(
      "HeldVersionNotNewer: a held rank above the record's rank",
      [InvariantControl("HeldVersionNotNewer")],
      held_version_newer,
    ),
    Test(
      "NackedHasItsTime: a nacked record with no nack time",
      [InvariantControl("NackedHasItsTime")],
      nacked_without_time,
    ),
    Test(
      "OnlyNackedHasNackTime: a nack time on a record not nacked",
      [InvariantControl("OnlyNackedHasNackTime")],
      nack_time_without_nack,
    ),
    Test(
      "DeferredHasArrivalPosition: a newer version with no arrival position",
      [InvariantControl("DeferredHasArrivalPosition")],
      deferred_without_arrival_position,
    ),
    Test(
      "OnlyDeferredHasArrivalPosition: an arrival position with nothing deferred",
      [InvariantControl("OnlyDeferredHasArrivalPosition")],
      arrival_position_without_deferred,
    ),
    Test(
      "DeferredArrivedAfterHeld: an arrival position below the record's own",
      [InvariantControl("DeferredArrivedAfterHeld")],
      deferred_arrived_before_held,
    ),
    Test(
      "HeldCopiesAgree: two payloads with nothing deferred",
      [InvariantControl("HeldCopiesAgree")],
      held_copies_differ,
    ),
    Test(
      "ShedBeforeExpiry: a superseded record that expires before it was shed",
      [InvariantControl("ShedBeforeExpiry")],
      shed_after_expiry,
    ),
    Test(
      "NackedBeforeExpiry: a dead letter that expires before its nack",
      [InvariantControl("NackedBeforeExpiry")],
      nacked_after_expiry,
    ),
    Test(
      "GrantedBeforeExpiry: a lease that expires before it was granted",
      [InvariantControl("GrantedBeforeExpiry")],
      granted_after_expiry,
    ),
    Test(
      "EndedAfterGrant: a lease that ended before it was granted",
      [InvariantControl("EndedAfterGrant")],
      ended_before_grant,
    ),
    Test("equal timestamps break nothing", [], equal_timestamps),
    Test(
      "ExpiryLimitPositive: an expiry limit of 0",
      [InvariantControl("ExpiryLimitPositive")],
      expiry_limit_zero,
    ),
    Test(
      "PullBatchPositive: a pull batch of 0",
      [InvariantControl("PullBatchPositive")],
      pull_batch_zero,
    ),
    Test(
      "LeaseTtlNonNegative: a lease TTL below 0",
      [InvariantControl("LeaseTtlNonNegative")],
      lease_ttl_negative,
    ),
    Test(
      "SupersededTtlNonNegative: a superseded TTL below 0",
      [InvariantControl("SupersededTtlNonNegative")],
      superseded_ttl_negative,
    ),
    Test(
      "DeadLetterTtlNonNegative: a dead-letter TTL below 0",
      [InvariantControl("DeadLetterTtlNonNegative")],
      dead_letter_ttl_negative,
    ),
    Test("the smallest config admitted breaks nothing", [], smallest_config),
    Test(
      "UniquePublisherId: two publishers with one id",
      [InvariantControl("UniquePublisherId")],
      two_publishers_one_id,
    ),
    Test(
      "UniqueConsumerId: two consumers with one id",
      [InvariantControl("UniqueConsumerId")],
      two_consumers_one_id,
    ),
    Test(
      "OnePocketPerKey: two pockets with one key",
      [InvariantControl("OnePocketPerKey")],
      two_pockets_one_key,
    ),
    Test(
      "OneRecordPerKeyPerPocket: two records with one key in one pocket",
      [InvariantControl("OneRecordPerKeyPerPocket")],
      two_records_one_key,
    ),
    Test(
      "OneActiveLeasePerPocket: two active leases over a key no pocket has",
      [InvariantControl("OneActiveLeasePerPocket")],
      two_active_leases_one_key,
    ),
    Test(
      "OneActiveLeasePerPocket: two active leases over the leased pocket",
      [
        InvariantControl("LeasedHasOneActiveLease"),
        InvariantControl("OneActiveLeasePerPocket"),
      ],
      two_active_leases_one_pocket,
    ),
    Test(
      "OneActiveLeasePerConsumer: one consumer with two active leases",
      [InvariantControl("OneActiveLeasePerConsumer")],
      two_active_leases_one_consumer,
    ),
    Test(
      "OneActiveLeasePerConsumer: an ended lease beside an active one breaks nothing",
      [],
      ended_lease_beside_active,
    ),
    Test(
      "OutstandingOnlyWhileLeased: a held record in a queued pocket",
      [InvariantControl("OutstandingOnlyWhileLeased")],
      held_in_queued_pocket,
    ),
    Test(
      "MarkedPocketHasNoExpiries: a marked record in a pocket with an expiry",
      [InvariantControl("MarkedPocketHasNoExpiries")],
      marked_in_pocket_with_expiry,
    ),
    Test(
      "MarkedPocketHasNoExpiries: unsettled records in a pocket with an expiry break nothing",
      [],
      unsettled_in_pocket_with_expiry,
    ),
    Test(
      "a record whose pocket key no pocket has breaks nothing",
      [],
      reference_names_none,
    ),
    Test(
      "a record whose pocket key two pockets have is read against both",
      [],
      reference_names_two,
    ),
    Test(
      "UniquePocketPosition: two pockets at one position",
      [InvariantControl("UniquePocketPosition")],
      two_pockets_one_position,
    ),
    Test(
      "UniqueRecordPosition: two records at one position, in two pockets",
      [InvariantControl("UniqueRecordPosition")],
      two_records_one_position,
    ),
    Test(
      "PocketPositionsBelowNext: a pocket at the next position",
      [InvariantControl("PocketPositionsBelowNext")],
      pocket_at_next_position,
    ),
    Test(
      "RecordPositionsBelowNext: a record at the next position",
      [InvariantControl("RecordPositionsBelowNext")],
      record_at_next_position,
    ),
    Test(
      "RecordPositionsBelowNext: a record one below the next position breaks nothing",
      [],
      record_below_next_position,
    ),
    Test(
      "a pocket or a record repeated in every field is still two",
      [],
      repeated_in_every_field,
    ),
    Test(
      "ArrivalPositionUnoccupied: an arrival position where a record stands",
      [InvariantControl("ArrivalPositionUnoccupied")],
      arrival_position_occupied,
    ),
    Test(
      "ArrivalPositionUnoccupied: an arrival position where its own record stands",
      [
        InvariantControl("DeferredArrivedAfterHeld"),
        InvariantControl("ArrivalPositionUnoccupied"),
      ],
      arrival_position_its_own,
    ),
    Test(
      "UniqueArrivalPosition: two deferred versions with one arrival position",
      [InvariantControl("UniqueArrivalPosition")],
      two_arrivals_one_position,
    ),
    Test(
      "ArrivalPositionsBelowNext: an arrival position at the next position",
      [InvariantControl("ArrivalPositionsBelowNext")],
      arrival_at_next_position,
    ),
    Test(
      "ArrivalPositionsBelowNext: an arrival one below the next position breaks nothing",
      [],
      arrival_below_next_position,
    ),
    Test(
      "the registry holds the specification's invariants, in its order",
      [],
      registry_follows_specification,
    ),
  ]
}

// The base's pockets, by key.
const leased = "pocket-leased"

const queued = "pocket-queued"

const idle = "pocket-idle"

/// A snapshot that breaks nothing. Positions come from one counter, as the
/// rules allot them: 0 the leased pocket, 1 to 5 its records, 6 the queued
/// pocket, 7 its record, 8 and 9 the two arrival positions, 10 the idle
/// pocket, and 11 is next. So no record stands at 0, 6 or 10.
///
/// The leased pocket holds one record of every kind: held and unsettled,
/// held and nacked, held with a newer version deferred behind it, held and
/// acked with a newer version deferred behind it, and one still queued. The
/// queued pocket's one record has the key of the leased pocket's queued
/// record. The leased pocket has an expired lease as well as its active
/// one, and both of `con-b`'s leases have ended.
fn base() -> Snapshot {
  Snapshot(
    config: config.defaults(),
    next_position: 11,
    publishers: [Publisher("pub-a"), Publisher("pub-b")],
    consumers: [Consumer("con-a"), Consumer("con-b")],
    pockets: [
      Pocket(
        key: leased,
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      Pocket(
        key: queued,
        status: PocketQueued,
        position: 6,
        consecutive_expiries: 0,
      ),
      Pocket(
        key: idle,
        status: PocketIdle,
        position: 10,
        consecutive_expiries: 0,
      ),
    ],
    records: [
      Record(
        pocket: leased,
        key: "unsettled",
        rank: 1,
        payload: <<"u1":utf8>>,
        position: 1,
        status: RecordHeld(
          held_rank: 1,
          held_payload: <<"u1":utf8>>,
          mark: Unsettled,
          nacked_at: None,
          deferred_position: None,
        ),
      ),
      Record(
        pocket: leased,
        key: "nacked",
        rank: 1,
        payload: <<"n1":utf8>>,
        position: 2,
        status: RecordHeld(
          held_rank: 1,
          held_payload: <<"n1":utf8>>,
          mark: Nacked,
          nacked_at: Some(700),
          deferred_position: None,
        ),
      ),
      Record(
        pocket: leased,
        key: "deferred",
        rank: 2,
        payload: <<"d2":utf8>>,
        position: 3,
        status: RecordHeld(
          held_rank: 1,
          held_payload: <<"d1":utf8>>,
          mark: Unsettled,
          nacked_at: None,
          deferred_position: Some(8),
        ),
      ),
      Record(
        pocket: leased,
        key: "acked",
        rank: 3,
        payload: <<"a3":utf8>>,
        position: 4,
        status: RecordHeld(
          held_rank: 2,
          held_payload: <<"a2":utf8>>,
          mark: Acked,
          nacked_at: None,
          deferred_position: Some(9),
        ),
      ),
      Record(
        pocket: leased,
        key: "waiting",
        rank: 1,
        payload: <<"w1":utf8>>,
        position: 5,
        status: RecordQueued,
      ),
      Record(
        pocket: queued,
        key: "waiting",
        rank: 1,
        payload: <<"q1":utf8>>,
        position: 7,
        status: RecordQueued,
      ),
    ],
    leases: [
      Lease(
        id: "lease-1",
        pocket: queued,
        consumer: "con-b",
        status: LeaseReleased(ended_at: 150),
        granted_at: 100,
        expires_at: 400,
      ),
      Lease(
        id: "lease-2",
        pocket: leased,
        consumer: "con-b",
        status: LeaseExpired(ended_at: 500),
        granted_at: 200,
        expires_at: 500,
      ),
      Lease(
        id: "lease-3",
        pocket: leased,
        consumer: "con-a",
        status: LeaseActive,
        granted_at: 600,
        expires_at: 900,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: queued,
        key: "waiting",
        rank: 0,
        payload: <<"q0":utf8>>,
        reason: Superseded,
        shed_at: 650,
        expires_at: 1650,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: leased,
        key: "gone",
        rank: 1,
        payload: <<"g1":utf8>>,
        nacked_by: "con-b",
        nacked_at: 300,
        expires_at: 1500,
      ),
    ],
  )
}

fn base_breaks_nothing() -> List(String) {
  assert invariants.check(base()) == []
  []
}

fn empty_breaks_nothing() -> List(String) {
  assert invariants.check(inspection.empty(config.defaults())) == []
  []
}

// The changes a control makes to the base. Each names the one member it
// changes; a change that finds no such member leaves the snapshot as it
// was, and the control then fails, since it asserts the whole list.

fn with_pocket(
  snapshot: Snapshot,
  key: String,
  change: fn(Pocket) -> Pocket,
) -> Snapshot {
  let pockets =
    list.map(snapshot.pockets, fn(pocket) {
      case pocket.key == key {
        True -> change(pocket)
        False -> pocket
      }
    })
  Snapshot(..snapshot, pockets:)
}

fn with_lease(
  snapshot: Snapshot,
  id: String,
  change: fn(Lease) -> Lease,
) -> Snapshot {
  let leases =
    list.map(snapshot.leases, fn(lease) {
      case lease.id == id {
        True -> change(lease)
        False -> lease
      }
    })
  Snapshot(..snapshot, leases:)
}

fn with_record(
  snapshot: Snapshot,
  pocket: String,
  key: String,
  change: fn(Record) -> Record,
) -> Snapshot {
  let records =
    list.map(snapshot.records, fn(record) {
      case record.pocket == pocket && record.key == key {
        True -> change(record)
        False -> record
      }
    })
  Snapshot(..snapshot, records:)
}

// One held field of a held record, changed. A queued record has none of
// them, and is returned as it was.

fn held_rank(record: Record, held_rank: Int) -> Record {
  case record.status {
    RecordHeld(..) as held ->
      Record(..record, status: RecordHeld(..held, held_rank:))
    RecordQueued -> record
  }
}

fn held_payload(record: Record, held_payload: BitArray) -> Record {
  case record.status {
    RecordHeld(..) as held ->
      Record(..record, status: RecordHeld(..held, held_payload:))
    RecordQueued -> record
  }
}

fn nacked_at(record: Record, nacked_at: Option(Timestamp)) -> Record {
  case record.status {
    RecordHeld(..) as held ->
      Record(..record, status: RecordHeld(..held, nacked_at:))
    RecordQueued -> record
  }
}

fn deferred_position(record: Record, deferred_position: Option(Int)) -> Record {
  case record.status {
    RecordHeld(..) as held ->
      Record(..record, status: RecordHeld(..held, deferred_position:))
    RecordQueued -> record
  }
}

fn mark(record: Record, mark: Mark) -> Record {
  case record.status {
    RecordHeld(..) as held ->
      Record(..record, status: RecordHeld(..held, mark:))
    RecordQueued -> record
  }
}

/// A record as `HoldRecord` leaves it: held at its own rank and payload,
/// unsettled, with no nack time and nothing deferred.
fn hold(record: Record) -> Record {
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
}

// Pocket ---------------------------------------------------

/// The leased pocket's one active lease has ended, so it has none.
fn leased_without_active_lease() -> List(String) {
  let snapshot =
    with_lease(base(), "lease-3", fn(lease) {
      Lease(..lease, status: LeaseReleased(ended_at: 800))
    })
  assert invariants.check(snapshot) == ["LeasedHasOneActiveLease"]
  []
}

/// The lease over the queued pocket is active again, then the same over
/// an idle pocket, and then with the queued pocket parked: `status !=
/// leased` is every other status. A parked pocket, which the base has
/// none of, breaks nothing while its lease stays ended.
fn unleased_with_active_lease() -> List(String) {
  let active = fn(lease) { Lease(..lease, status: LeaseActive) }
  let snapshot = with_lease(base(), "lease-1", active)
  assert invariants.check(snapshot) == ["UnleasedHasNoActiveLease"]
  let over_idle = fn(lease) {
    Lease(..lease, pocket: idle, status: LeaseActive)
  }
  let snapshot = with_lease(base(), "lease-1", over_idle)
  assert invariants.check(snapshot) == ["UnleasedHasNoActiveLease"]
  let parked =
    with_pocket(base(), queued, fn(pocket) {
      Pocket(..pocket, status: PocketParked)
    })
  assert invariants.check(parked) == []
  assert invariants.check(with_lease(parked, "lease-1", active))
    == ["UnleasedHasNoActiveLease"]
  []
}

/// The queued pocket, which has a record, is idle. Then that record is
/// held: `records` is every record, queued or held, and a held record in a
/// pocket that is not leased breaks `OutstandingOnlyWhileLeased` besides.
fn idle_with_record() -> List(String) {
  let snapshot =
    with_pocket(base(), queued, fn(pocket) {
      Pocket(..pocket, status: PocketIdle)
    })
  assert invariants.check(snapshot) == ["IdleHasNoRecords"]
  assert invariants.check(with_record(snapshot, queued, "waiting", hold))
    == ["IdleHasNoRecords", "OutstandingOnlyWhileLeased"]
  []
}

fn expiries(snapshot: Snapshot, key: String, count: Int) -> Snapshot {
  with_pocket(snapshot, key, fn(pocket) {
    Pocket(..pocket, consecutive_expiries: count)
  })
}

fn expiry_limit(snapshot: Snapshot, limit: Int) -> Snapshot {
  let config = Settings(..snapshot.config, max_consecutive_expiries: limit)
  Snapshot(..snapshot, config:)
}

/// The limit is 3. The queued pocket holds no marked record, so
/// `MarkedPocketHasNoExpiries` asks nothing of its count. Then a limit of
/// 1: the bound is the snapshot's configured limit, not the default.
fn expiries_over_limit() -> List(String) {
  assert invariants.check(expiries(base(), queued, 4))
    == ["ExpiriesWithinLimit"]
  assert invariants.check(expiries(base(), queued, 2) |> expiry_limit(1))
    == ["ExpiriesWithinLimit"]
  []
}

/// At the default limit, and at a limit of 5, above the default.
fn expiries_at_limit() -> List(String) {
  assert invariants.check(expiries(base(), queued, 3)) == []
  assert invariants.check(expiries(base(), queued, 5) |> expiry_limit(5)) == []
  []
}

// Record ---------------------------------------------------
// In the base the unsettled record's held rank equals its rank, which is
// the allowed side of `held_rank <= rank`, and its two payloads agree.

/// Nothing is deferred and the payloads agree, so no other check of the
/// record has a true premise.
/// The same of the nacked record, and of the acked one, whose rank is 3:
/// the invariant is every held record's, whatever its mark.
fn held_version_newer() -> List(String) {
  let snapshot = with_record(base(), leased, "unsettled", held_rank(_, 2))
  assert invariants.check(snapshot) == ["HeldVersionNotNewer"]
  let snapshot = with_record(base(), leased, "nacked", held_rank(_, 2))
  assert invariants.check(snapshot) == ["HeldVersionNotNewer"]
  let snapshot = with_record(base(), leased, "acked", held_rank(_, 4))
  assert invariants.check(snapshot) == ["HeldVersionNotNewer"]
  []
}

/// The nacked record loses its nack time. Then the acked record, which has
/// a newer version deferred behind it and no nack time, is nacked: a
/// deferred version excuses nothing.
fn nacked_without_time() -> List(String) {
  let snapshot = with_record(base(), leased, "nacked", nacked_at(_, None))
  assert invariants.check(snapshot) == ["NackedHasItsTime"]
  let snapshot = with_record(base(), leased, "acked", mark(_, Nacked))
  assert invariants.check(snapshot) == ["NackedHasItsTime"]
  []
}

/// `mark != nacked` is unsettled and acked alike.
fn nack_time_without_nack() -> List(String) {
  let timed = nacked_at(_, Some(700))
  assert invariants.check(with_record(base(), leased, "unsettled", timed))
    == ["OnlyNackedHasNackTime"]
  assert invariants.check(with_record(base(), leased, "acked", timed))
    == ["OnlyNackedHasNackTime"]
  []
}

/// The unsettled record's rank moves on to 2 while its held rank stays 1,
/// and nothing says where the newer version arrived. Its payloads still
/// agree, so `HeldCopiesAgree` holds. Then the same of the nacked record,
/// and of that record acked: the invariant is every held record's,
/// whatever its mark.
fn deferred_without_arrival_position() -> List(String) {
  let newer = fn(record) { Record(..record, rank: 2) }
  let snapshot = with_record(base(), leased, "unsettled", newer)
  assert invariants.check(snapshot) == ["DeferredHasArrivalPosition"]
  let snapshot = with_record(base(), leased, "nacked", newer)
  assert invariants.check(snapshot) == ["DeferredHasArrivalPosition"]
  let snapshot =
    with_record(base(), leased, "nacked", fn(record) {
      newer(record) |> mark(Acked) |> nacked_at(None)
    })
  assert invariants.check(snapshot) == ["DeferredHasArrivalPosition"]
  []
}

/// The deferred record's rank falls back to its held rank, 1, and its
/// arrival position stays. Then the acked record's rank falls back to its
/// held rank, 2: the invariant is every held record's, whatever its mark.
fn arrival_position_without_deferred() -> List(String) {
  let snapshot =
    with_record(base(), leased, "deferred", fn(record) {
      Record(..record, rank: 1)
    })
  assert invariants.check(snapshot) == ["OnlyDeferredHasArrivalPosition"]
  let snapshot =
    with_record(base(), leased, "acked", fn(record) {
      Record(..record, rank: 2)
    })
  assert invariants.check(snapshot) == ["OnlyDeferredHasArrivalPosition"]
  []
}

/// The deferred record stands at 3 and its newer version arrived at 0,
/// where no record stands. Then the acked record, which stands at 4.
fn deferred_arrived_before_held() -> List(String) {
  let snapshot =
    with_record(base(), leased, "deferred", deferred_position(_, Some(0)))
  assert invariants.check(snapshot) == ["DeferredArrivedAfterHeld"]
  let snapshot =
    with_record(base(), leased, "acked", deferred_position(_, Some(0)))
  assert invariants.check(snapshot) == ["DeferredArrivedAfterHeld"]
  []
}

/// In the base the deferred record's two payloads differ, which is allowed
/// while a newer version is deferred. Here nothing is: first of the
/// unsettled record, then of the nacked one.
fn held_copies_differ() -> List(String) {
  let snapshot =
    with_record(base(), leased, "unsettled", held_payload(_, <<"u0":utf8>>))
  assert invariants.check(snapshot) == ["HeldCopiesAgree"]
  let snapshot =
    with_record(base(), leased, "nacked", held_payload(_, <<"n0":utf8>>))
  assert invariants.check(snapshot) == ["HeldCopiesAgree"]
  []
}

// SupersededRecord, DeadLetter and Lease -----------------
// Each control sets the later timestamp one millisecond before the
// earlier, and `equal_timestamps` sets the two equal.

/// The base's one superseded record, shed at 650, with this `expires_at`.
fn shed_expiring_at(expires_at: Timestamp) -> Snapshot {
  let superseded =
    list.map(base().superseded, fn(shed) {
      SupersededRecord(..shed, expires_at:)
    })
  Snapshot(..base(), superseded:)
}

/// The base's one dead letter, nacked at 300, with this `expires_at`.
fn letter_expiring_at(expires_at: Timestamp) -> Snapshot {
  let dead_letters =
    list.map(base().dead_letters, fn(letter) {
      DeadLetter(..letter, expires_at:)
    })
  Snapshot(..base(), dead_letters:)
}

fn lease_expiring_at(id: String, expires_at: Timestamp) -> Snapshot {
  with_lease(base(), id, fn(lease) { Lease(..lease, expires_at:) })
}

fn shed_after_expiry() -> List(String) {
  assert invariants.check(shed_expiring_at(649)) == ["ShedBeforeExpiry"]
  []
}

fn nacked_after_expiry() -> List(String) {
  assert invariants.check(letter_expiring_at(299)) == ["NackedBeforeExpiry"]
  []
}

/// The active lease was granted at 600, the released one at 100 and the
/// expired one at 200: the invariant is every lease's, ended or not. The
/// expired lease still ended, at 500, after its grant.
fn granted_after_expiry() -> List(String) {
  assert invariants.check(lease_expiring_at("lease-3", 599))
    == ["GrantedBeforeExpiry"]
  assert invariants.check(lease_expiring_at("lease-1", 99))
    == ["GrantedBeforeExpiry"]
  assert invariants.check(lease_expiring_at("lease-2", 199))
    == ["GrantedBeforeExpiry"]
  []
}

/// `status in {released, expired}`: the released lease was granted at 100
/// and the expired one at 200.
fn ended_before_grant() -> List(String) {
  let released =
    with_lease(base(), "lease-1", fn(lease) {
      Lease(..lease, status: LeaseReleased(ended_at: 99))
    })
  assert invariants.check(released) == ["EndedAfterGrant"]
  let expired =
    with_lease(base(), "lease-2", fn(lease) {
      Lease(..lease, status: LeaseExpired(ended_at: 199))
    })
  assert invariants.check(expired) == ["EndedAfterGrant"]
  []
}

/// The four comparisons admit equality: a zero TTL stamps `expires_at` at
/// the timestamp it is measured from, and a lease can end in the instant
/// it was granted.
fn equal_timestamps() -> List(String) {
  assert invariants.check(shed_expiring_at(650)) == []
  assert invariants.check(letter_expiring_at(300)) == []
  assert invariants.check(lease_expiring_at("lease-3", 600)) == []
  let released =
    with_lease(base(), "lease-1", fn(lease) {
      Lease(..lease, status: LeaseReleased(ended_at: 100))
    })
  assert invariants.check(released) == []
  let expired =
    with_lease(base(), "lease-2", fn(lease) {
      Lease(..lease, status: LeaseExpired(ended_at: 200))
    })
  assert invariants.check(expired) == []
  []
}

// Config ---------------------------------------------------
// A snapshot holds the plain settings, so it can hold what `config.new`
// refuses. Each control is the largest value its invariant rejects, and
// then one further below it: the bound is an order, not one value.

/// Every pocket of the base has no consecutive expiry, so a limit of 0
/// leaves `ExpiriesWithinLimit` holding. A limit of -1 does not: every
/// pocket's count, 0, is above it.
fn expiry_limit_zero() -> List(String) {
  assert invariants.check(expiry_limit(base(), 0)) == ["ExpiryLimitPositive"]
  assert invariants.check(expiry_limit(base(), -1))
    == ["ExpiriesWithinLimit", "ExpiryLimitPositive"]
  []
}

fn pull_batch_zero() -> List(String) {
  let config = Settings(..config.defaults(), max_pull_batch: 0)
  assert invariants.check(Snapshot(..base(), config:)) == ["PullBatchPositive"]
  let config = Settings(..config.defaults(), max_pull_batch: -1)
  assert invariants.check(Snapshot(..base(), config:)) == ["PullBatchPositive"]
  []
}

fn lease_ttl_negative() -> List(String) {
  let config = Settings(..config.defaults(), lease_ttl: -1)
  assert invariants.check(Snapshot(..base(), config:))
    == ["LeaseTtlNonNegative"]
  let config = Settings(..config.defaults(), lease_ttl: -2)
  assert invariants.check(Snapshot(..base(), config:))
    == ["LeaseTtlNonNegative"]
  []
}

fn superseded_ttl_negative() -> List(String) {
  let config = Settings(..config.defaults(), superseded_ttl: -1)
  assert invariants.check(Snapshot(..base(), config:))
    == ["SupersededTtlNonNegative"]
  let config = Settings(..config.defaults(), superseded_ttl: -2)
  assert invariants.check(Snapshot(..base(), config:))
    == ["SupersededTtlNonNegative"]
  []
}

fn dead_letter_ttl_negative() -> List(String) {
  let config = Settings(..config.defaults(), dead_letter_ttl: -1)
  assert invariants.check(Snapshot(..base(), config:))
    == ["DeadLetterTtlNonNegative"]
  let config = Settings(..config.defaults(), dead_letter_ttl: -2)
  assert invariants.check(Snapshot(..base(), config:))
    == ["DeadLetterTtlNonNegative"]
  []
}

/// 1 for the two limits and 0 for each TTL.
fn smallest_config() -> List(String) {
  let config =
    Settings(
      lease_ttl: 0,
      max_consecutive_expiries: 1,
      max_pull_batch: 1,
      superseded_ttl: 0,
      dead_letter_ttl: 0,
    )
  assert invariants.check(Snapshot(..base(), config:)) == []
  []
}

// One per key ----------------------------------------------
// `a != b` is two members, not two unequal values. The base is the near
// miss for all of these: every member paired with itself has its own id,
// key and position, so a check that forgot `a != b` breaks on the base.

/// A second publisher equal to the first in every field. Compared by
/// value the two would be one publisher, and nothing would break.
fn two_publishers_one_id() -> List(String) {
  let publishers = list.append(base().publishers, [Publisher("pub-a")])
  assert invariants.check(Snapshot(..base(), publishers:))
    == ["UniquePublisherId"]
  []
}

/// A second consumer equal to the first. Each of the two has the one
/// active lease that names `con-a`, so `OneActiveLeasePerConsumer` holds.
fn two_consumers_one_id() -> List(String) {
  let consumers = list.append(base().consumers, [Consumer("con-a")])
  assert invariants.check(Snapshot(..base(), consumers:))
    == ["UniqueConsumerId"]
  []
}

/// A second pocket with the queued pocket's key, at the next position.
fn two_pockets_one_key() -> List(String) {
  let pockets =
    list.append(base().pockets, [
      Pocket(
        key: queued,
        status: PocketQueued,
        position: 11,
        consecutive_expiries: 0,
      ),
    ])
  assert invariants.check(Snapshot(..base(), next_position: 12, pockets:))
    == ["OnePocketPerKey"]
  []
}

/// A second record under the queued pocket's one key, at the next
/// position. In the base that key is in the leased pocket too, which is
/// allowed: `a.pocket = b.pocket` is false of those two. Then a second
/// record under the leased pocket's key `nacked`, first held and then
/// queued: the invariant is over every record, whatever its status.
fn two_records_one_key() -> List(String) {
  let second = fn(pocket, key, status) {
    let record =
      Record(
        pocket:,
        key:,
        rank: 2,
        payload: <<"second":utf8>>,
        position: 11,
        status: RecordQueued,
      )
    let records = list.append(base().records, [status(record)])
    Snapshot(..base(), next_position: 12, records:)
  }
  let stays_queued = fn(record) { record }
  assert invariants.check(second(queued, "waiting", stays_queued))
    == ["OneRecordPerKeyPerPocket"]
  assert invariants.check(second(leased, "nacked", hold))
    == ["OneRecordPerKeyPerPocket"]
  assert invariants.check(second(leased, "nacked", stays_queued))
    == ["OneRecordPerKeyPerPocket"]
  []
}

// Leases, and what a record asks of its pocket -------------

/// Two active leases over one pocket key always break a second invariant
/// when a pocket has that key: the pocket is leased, and
/// `LeasedHasOneActiveLease` asks for exactly one, or it is not, and
/// `UnleasedHasNoActiveLease` asks for none. This invariant breaks alone
/// only where no pocket has the key, so that no pocket counts the two
/// leases. The second lease is a third consumer's, so that each consumer
/// has one.
fn two_active_leases_one_key() -> List(String) {
  let snapshot =
    with_lease(base(), "lease-1", fn(lease) {
      Lease(..lease, pocket: "pocket-gone", status: LeaseActive)
    })
  let consumers = list.append(snapshot.consumers, [Consumer("con-c")])
  let leases =
    list.append(snapshot.leases, [
      Lease(
        id: "lease-4",
        pocket: "pocket-gone",
        consumer: "con-c",
        status: LeaseActive,
        granted_at: 600,
        expires_at: 900,
      ),
    ])
  assert invariants.check(Snapshot(..snapshot, consumers:, leases:))
    == ["OneActiveLeasePerPocket"]
  []
}

/// The leased pocket's expired lease, `con-b`'s, is active again beside
/// `con-a`'s. The pocket exists and is leased, so
/// `LeasedHasOneActiveLease` breaks with it, as the comment above says it
/// must: this is also that invariant's control for two leases, not none.
fn two_active_leases_one_pocket() -> List(String) {
  let snapshot =
    with_lease(base(), "lease-2", fn(lease) {
      Lease(..lease, status: LeaseActive)
    })
  assert invariants.check(snapshot)
    == ["LeasedHasOneActiveLease", "OneActiveLeasePerPocket"]
  []
}

/// The queued pocket is leased too, and to `con-a`, who holds the leased
/// pocket. Each pocket has its one active lease.
fn two_active_leases_one_consumer() -> List(String) {
  let snapshot =
    base()
    |> with_pocket(queued, fn(pocket) { Pocket(..pocket, status: PocketLeased) })
    |> with_lease("lease-1", fn(lease) {
      Lease(..lease, consumer: "con-a", status: LeaseActive)
    })
  assert invariants.check(snapshot) == ["OneActiveLeasePerConsumer"]
  []
}

/// The leased pocket's expired lease was `con-a`'s too. Only an active
/// lease counts, as it does in the base, where both of `con-b`'s leases
/// have ended.
fn ended_lease_beside_active() -> List(String) {
  let snapshot =
    with_lease(base(), "lease-2", fn(lease) {
      Lease(..lease, consumer: "con-a")
    })
  assert invariants.check(snapshot) == []
  []
}

/// The queued pocket's record is held, and its pocket is not leased. Then
/// that record is acked as well: the invariant is every held record's,
/// whatever its mark, and the pocket has no consecutive expiry.
fn held_in_queued_pocket() -> List(String) {
  let snapshot = with_record(base(), queued, "waiting", hold)
  assert invariants.check(snapshot) == ["OutstandingOnlyWhileLeased"]
  let snapshot =
    with_record(base(), queued, "waiting", fn(record) {
      hold(record) |> mark(Acked)
    })
  assert invariants.check(snapshot) == ["OutstandingOnlyWhileLeased"]
  []
}

/// The leased pocket holds a nacked record and an acked one, and its count
/// is 1, which is within the limit. Then each of the two alone, the other
/// unsettled: `mark != unsettled` is acked and nacked alike.
fn marked_in_pocket_with_expiry() -> List(String) {
  let lapsed = expiries(base(), leased, 1)
  assert invariants.check(lapsed) == ["MarkedPocketHasNoExpiries"]
  let nacked_alone = with_record(lapsed, leased, "acked", mark(_, Unsettled))
  assert invariants.check(nacked_alone) == ["MarkedPocketHasNoExpiries"]
  let acked_alone =
    with_record(lapsed, leased, "nacked", fn(record) {
      record |> mark(Unsettled) |> nacked_at(None)
    })
  assert invariants.check(acked_alone) == ["MarkedPocketHasNoExpiries"]
  []
}

/// The same count, with the nacked and the acked record unsettled.
fn unsettled_in_pocket_with_expiry() -> List(String) {
  let snapshot =
    expiries(base(), leased, 1)
    |> with_record(leased, "nacked", fn(record) {
      record |> mark(Unsettled) |> nacked_at(None)
    })
    |> with_record(leased, "acked", mark(_, Unsettled))
  assert invariants.check(snapshot) == []
  []
}

// A reference is a key, and in a snapshot built by hand it can name no
// entity, or several. These two tests pin what the checker does then.

/// The queued pocket's record is held and acked, and names a pocket no
/// pocket is. There is no pocket whose status or count could be wrong, so
/// neither `OutstandingOnlyWhileLeased` nor `MarkedPocketHasNoExpiries`
/// breaks, and the specification has no invariant that a reference names
/// an entity.
fn reference_names_none() -> List(String) {
  let snapshot =
    with_record(base(), queued, "waiting", fn(record) {
      Record(..hold(record), pocket: "pocket-gone") |> mark(Acked)
    })
  assert invariants.check(snapshot) == []
  []
}

/// A second pocket has the leased pocket's key, and it is queued and has
/// one consecutive expiry. Every record and lease that names the key
/// names both pockets: the second has the active lease, which a queued
/// pocket may not, and the held and the marked records, which ask a
/// leased pocket with no expiry.
fn reference_names_two() -> List(String) {
  let pockets =
    list.append(base().pockets, [
      Pocket(
        key: leased,
        status: PocketQueued,
        position: 11,
        consecutive_expiries: 1,
      ),
    ])
  assert invariants.check(Snapshot(..base(), next_position: 12, pockets:))
    == [
      "UnleasedHasNoActiveLease", "OnePocketPerKey",
      "OutstandingOnlyWhileLeased", "MarkedPocketHasNoExpiries",
    ]
  []
}

// Positions ------------------------------------------------

fn pocket_at(snapshot: Snapshot, key: String, position: Int) -> Snapshot {
  with_pocket(snapshot, key, fn(pocket) { Pocket(..pocket, position:) })
}

fn record_at(
  snapshot: Snapshot,
  pocket: String,
  key: String,
  position: Int,
) -> Snapshot {
  with_record(snapshot, pocket, key, fn(record) { Record(..record, position:) })
}

/// The idle pocket stands at 6, where the queued pocket stands.
fn two_pockets_one_position() -> List(String) {
  assert invariants.check(pocket_at(base(), idle, 6))
    == ["UniquePocketPosition"]
  []
}

/// The queued pocket's record stands at 5, where the leased pocket's
/// queued record stands: the invariant is over every record, not over one
/// pocket's. Then it stands at 1, where a held record stands, and then the
/// nacked record does, so that both are held: the invariant is over every
/// record, whatever its status.
fn two_records_one_position() -> List(String) {
  assert invariants.check(record_at(base(), queued, "waiting", 5))
    == ["UniqueRecordPosition"]
  assert invariants.check(record_at(base(), queued, "waiting", 1))
    == ["UniqueRecordPosition"]
  assert invariants.check(record_at(base(), leased, "nacked", 1))
    == ["UniqueRecordPosition"]
  []
}

/// The next position is 11. In the base the idle pocket stands at 10. Then
/// it stands at 12, above the next position and not at it.
fn pocket_at_next_position() -> List(String) {
  assert invariants.check(pocket_at(base(), idle, 11))
    == ["PocketPositionsBelowNext"]
  assert invariants.check(pocket_at(base(), idle, 12))
    == ["PocketPositionsBelowNext"]
  []
}

/// At the next position, 11, and then above it, at 12.
fn record_at_next_position() -> List(String) {
  assert invariants.check(record_at(base(), queued, "waiting", 11))
    == ["RecordPositionsBelowNext"]
  assert invariants.check(record_at(base(), queued, "waiting", 12))
    == ["RecordPositionsBelowNext"]
  []
}

/// 10 is the idle pocket's position and no record's. No invariant compares
/// a pocket's position with a record's.
fn record_below_next_position() -> List(String) {
  assert invariants.check(record_at(base(), queued, "waiting", 10)) == []
  []
}

/// The idle pocket twice, and then the queued pocket's record twice. A
/// copy shares the key and the position at once, so the two invariants
/// cannot be separated here; compared by value, each pair would be one
/// member and nothing would break.
fn repeated_in_every_field() -> List(String) {
  let pockets =
    list.append(base().pockets, [
      Pocket(
        key: idle,
        status: PocketIdle,
        position: 10,
        consecutive_expiries: 0,
      ),
    ])
  assert invariants.check(Snapshot(..base(), pockets:))
    == ["OnePocketPerKey", "UniquePocketPosition"]
  let records =
    list.append(base().records, [
      Record(
        pocket: queued,
        key: "waiting",
        rank: 1,
        payload: <<"q1":utf8>>,
        position: 7,
        status: RecordQueued,
      ),
    ])
  assert invariants.check(Snapshot(..base(), records:))
    == ["OneRecordPerKeyPerPocket", "UniqueRecordPosition"]
  []
}

// Arrival positions ----------------------------------------
// In the base the deferred record stands at 3 with its newer version's
// arrival at 8, and the acked record at 4 with its arrival at 9.

fn arrival_at(key: String, position: Int) -> Snapshot {
  with_record(base(), leased, key, deferred_position(_, Some(position)))
}

/// 7 is where the queued pocket's record stands: the invariant is over
/// every record, not over one pocket's. Then the same of the acked
/// record's arrival position: the invariant is every held record's,
/// whatever its mark.
fn arrival_position_occupied() -> List(String) {
  assert invariants.check(arrival_at("deferred", 7))
    == ["ArrivalPositionUnoccupied"]
  assert invariants.check(arrival_at("acked", 7))
    == ["ArrivalPositionUnoccupied"]
  []
}

/// `ArrivalPositionUnoccupied` has no `a != b`, so it ranges over a record
/// paired with itself: an arrival position equal to the record's own
/// position breaks it. That state is also the nearest one that breaks
/// `DeferredArrivedAfterHeld`, whose `>` rejects equality, so the two
/// cannot be separated here.
fn arrival_position_its_own() -> List(String) {
  assert invariants.check(arrival_at("deferred", 3))
    == ["DeferredArrivedAfterHeld", "ArrivalPositionUnoccupied"]
  assert invariants.check(arrival_at("acked", 4))
    == ["DeferredArrivedAfterHeld", "ArrivalPositionUnoccupied"]
  []
}

/// 9 is the acked record's arrival position, and no record stands there.
/// In the base the unsettled and the nacked record are both held with
/// nothing deferred, which is allowed: two absent positions are not one
/// position.
///
/// Then two arrivals in two pockets. The queued pocket is leased, to
/// `con-b`, whose lease over it is active again, and its record, at 7, is
/// held with a newer version deferred behind it. With that version's
/// arrival at 10, where no record stands, nothing breaks; at 8, the
/// arrival position of the leased pocket's deferred record, the invariant
/// does: it is over every record, not over one pocket's.
fn two_arrivals_one_position() -> List(String) {
  assert invariants.check(arrival_at("deferred", 9))
    == ["UniqueArrivalPosition"]
  let second_pocket = fn(arrival) {
    base()
    |> with_pocket(queued, fn(pocket) { Pocket(..pocket, status: PocketLeased) })
    |> with_lease("lease-1", fn(lease) { Lease(..lease, status: LeaseActive) })
    |> with_record(queued, "waiting", fn(record) {
      Record(..hold(record), rank: 2, payload: <<"q2":utf8>>)
      |> deferred_position(Some(arrival))
    })
  }
  assert invariants.check(second_pocket(10)) == []
  assert invariants.check(second_pocket(8)) == ["UniqueArrivalPosition"]
  []
}

/// The next position is 11. Then an arrival above it, at 12.
fn arrival_at_next_position() -> List(String) {
  assert invariants.check(arrival_at("acked", 11))
    == ["ArrivalPositionsBelowNext"]
  assert invariants.check(arrival_at("acked", 12))
    == ["ArrivalPositionsBelowNext"]
  []
}

/// 10 is the idle pocket's position and no record's.
fn arrival_below_next_position() -> List(String) {
  assert invariants.check(arrival_at("acked", 10)) == []
  []
}

// The registry ---------------------------------------------

/// The coverage check asks that each invariant is registered. This asks
/// the order too, and that none is registered twice: `check` reports in
/// the registry's order, and every control above asserts a whole list.
fn registry_follows_specification() -> List(String) {
  let assert Ok(specification) = coverage.read(coverage.specification_path)
  let registered =
    list.map(invariants.registry(), fn(invariant) { invariant.name })
  assert registered == coverage.invariant_names(specification)
  []
}
