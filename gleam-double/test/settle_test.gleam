//// The specification's "Ack and nack" section: the Ack and Nack tables.
////
//// First nine catalog rows of `crates/libpachinko/tests/queue/settle.rs`,
//// S040, S041, S042, S047, S049, S084, S085, S086 and S090, each a port of
//// its Rust test, driving the owner. A port asserts what its Rust test
//// asserts, with these changes.
////
//// - The Rust's ack and nack answer `SettleOutcome` and its release a fate
////   and the ended lease. The specification's answer nothing, so a port
////   asserts that the reply is `Ok(Outcome(fired, None))`, with the names of
////   the rules it fired, and reads the state from the snapshot (the ruling on
////   N24 in `double/SPEC-NOTES.md`).
//// - The Rust's batch carries its lease's `expires_at`; a port reads it from
////   the snapshot's lease (N13).
//// - The Rust's refusals are `LeaseNotActive`, `LeaseNotYours`,
////   `RecordNotHeld` and `MarkConflict`. The specification names none. The
////   double's are `LeaseFenced`, which is both of the first two,
////   `RecordNotHeld` and `MarkConflict`, the lease checked first (N21). Each
////   refusal's name and shape is `impl-choice`: spec note G28 for
////   `LeaseFenced`, G34 for `RecordNotHeld` and the order, and G35 for
////   `MarkConflict`.
//// - Every refusal goes through `harness.refused`, which compares the
////   complete snapshot, the clock's read count and the lease-id draws before
////   and after.
//// - The lease view is read through the owner's `LeaseView` message.
////
//// S040, S047 and S085 are `wire` rows: only the library half is ported.
//// S047 and S086 end their lease by release, as their Rust tests do.
////
//// Then six catalog rows of the same file that publish onto a held key,
//// S043, S044, S045, S046, S071 and S072, ported the same way. Where the
//// Rust checks a publish for an outcome, the port asserts that it answered
//// nothing and names the rule it fired, and asserts the state through the
//// snapshot (the ruling on N32). Each that ends its lease ends it by
//// release, as its Rust test does. Each asserts absolute positions, which
//// hold only because the queue's first position is 0: `impl-choice` (spec
//// note G04, and G48).
////
//// Then the double's own tests, which pin no catalog row: what each of the
//// four mark rules changes, how often it reads the clock and that it draws
//// no lease id, each rule's guard, and the refusals in the order they are
//// checked, all over states built by hand.

import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import harness.{type Double, Driven}
import pachinko_double/config.{Settings}
import pachinko_double/consuming.{HeldView}
import pachinko_double/core
import pachinko_double/inspection.{Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type LeaseStatus, type Mark, type Record, type State, Acked, Consumer,
  DeadLetter, Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked, Pocket,
  PocketIdle, PocketLeased, Publisher, Queue, Record, RecordHeld, RecordQueued,
  State, Superseded, SupersededRecord, Unsettled,
}
import pachinko_double/owner.{type Reply, Outcome}
import pachinko_double/refusals.{
  type Refusal, EqualRankRefused, LeaseFenced, MarkConflict, RecordNotHeld,
  UnknownConsumer,
}
import pachinko_double/rules.{
  Applied, Defect, Done, EffectFailed, Refused, Refuses, Row, Table,
}
import pachinko_double/settle.{type Ack, type Nack, Ack, Nack, Renew}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "an ack marks a held record and removes nothing",
      [Scenario("S040")],
      ack_marks,
    ),
    Test("an ack extends the lease", [Scenario("S041")], ack_extends),
    Test(
      "rejected acks change nothing, not even the lease's expiry; a repeat ack is not a refusal",
      [Scenario("S042")],
      rejected_acks,
    ),
    Test(
      "a nack marks the held version without redelivery, and dead-letters it when the lease ends",
      [Scenario("S047")],
      nack_marks,
    ),
    Test(
      "rejected nacks change nothing and create no dead letter",
      [Scenario("S049")],
      rejected_nacks,
    ),
    Test(
      "a repeat ack or nack answers like the first, extends the lease and changes nothing else",
      [Scenario("S084")],
      repeats,
    ),
    Test(
      "a changed mark is refused as a mark conflict and changes nothing, not even the lease's expiry, and reads no clock",
      [Scenario("S085")],
      changed_mark,
    ),
    Test(
      "a repeat nack keeps the first nack's time",
      [Scenario("S086")],
      repeat_nack_keeps_time,
    ),
    Test(
      "the lease read shows every held record with its mark, smallest position first",
      [Scenario("S090")],
      lease_read,
    ),
    Test(
      "a supersede while the key is held is deferred behind the held version",
      [Scenario("S043")],
      deferred_behind_held,
    ),
    Test(
      "a version deferred behind an acked version is queued at its arrival position",
      [Scenario("S044")],
      deferred_behind_acked,
    ),
    Test(
      "two supersedes while held: only the deferred version is shed, and the held version never is",
      [Scenario("S045")],
      two_supersedes_while_held,
    ),
    Test(
      "a stale check while held compares with the latest rank, not the held one",
      [Scenario("S046")],
      stale_against_latest,
    ),
    Test(
      "a key acked, then published again within the same lease, waits behind the held version until the lease ends",
      [Scenario("S071")],
      acked_then_published,
    ),
    Test(
      "a key nacked, then published again within the same lease, waits until the lease ends; the dead letter appears then",
      [Scenario("S072")],
      nacked_then_published,
    ),
    Test(
      "AckMarksRecord changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      ack_marks_exactly,
    ),
    Test(
      "AckRepeats changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      ack_repeats_exactly,
    ),
    Test(
      "NackMarksRecord changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      nack_marks_exactly,
    ),
    Test(
      "NackRepeats changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      nack_repeats_exactly,
    ),
    Test(
      "each mark rule's guard holds only for a held record of the consumer's active lease, by its mark",
      [],
      mark_guards,
    ),
    Test(
      "a mark keeps the held version and the deferred one's arrival position, and a deferred version does not change which mark rule holds",
      [],
      deferred_marks,
    ),
    Test(
      "with lease_ttl 0 a mark, its repeat and a renew each set the lease's expires_at to now",
      [],
      zero_lease_ttl,
    ),
    Test(
      "a refused mark names the first requirement that fails, the lease first, and asks for no time",
      [],
      mark_refusals,
    ),
    Test(
      "a mark rule's effect without its held record is a defect, and asks for no time",
      [],
      mark_without_record,
    ),
  ]
}

const claims = "PullClaimsFrontPocket"

const within = "PullWithinLease"

const hold = "HoldRecord"

const ack_marks_record = "AckMarksRecord"

const ack_repeats = "AckRepeats"

const nack_marks_record = "NackMarksRecord"

const nack_repeats = "NackRepeats"

/// What a trigger that fired these rules and answered nothing replies. A
/// publish, an ack, a nack, a renew and a release all answer nothing.
fn fired(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}

/// The Rust fixture's `LEASE_TTL`.
fn lease_ttl() -> Int {
  harness.fixture_settings().lease_ttl
}

/// The Rust fixture's `DEAD_LETTER_TTL`.
fn dead_letter_ttl() -> Int {
  harness.fixture_settings().dead_letter_ttl
}

/// The Rust test file's `held`: a fixture whose pocket `pk` holds `k` at
/// rank 1, payload `v1`, pulled by `c1`, and that lease's id.
fn held() -> #(Double, String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired(["PublishIntoNewPocket"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold]
  #(double, batch.lease)
}

/// The Rust test file's `marked_both_ways`: `pk` with `k1` and `k2`, both
/// held by `c1` from `t0`, then `k1` nacked and `k2` acked at 10 ms.
fn marked_both_ways() -> #(Double, String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k1", 1, "k1")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "pk", "k2", 1, "k2")
    == fired(["PublishNewKey"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  let _ = harness.at(double, 10)
  assert harness.nack(double, "c1", batch.lease, "k1")
    == fired([nack_marks_record])
  assert harness.ack(double, "c1", batch.lease, "k2")
    == fired([ack_marks_record])
  #(double, batch.lease)
}

/// The status of a record held at `rank` and `payload` with this mark and
/// this nack time, and nothing deferred behind it: the Rust fixture's
/// `held_as` with no arrival position.
fn held_as(
  rank: Int,
  payload: String,
  mark: Mark,
  nacked_at: Option(Int),
) -> model.RecordStatus {
  RecordHeld(
    held_rank: rank,
    held_payload: <<payload:utf8>>,
    mark:,
    nacked_at:,
    deferred_position: None,
  )
}

/// The Rust fixture's `held_as` with an arrival position: the status of a
/// record held at `rank` and `payload`, with this mark, this nack time and
/// this `deferred_position`.
fn deferred_as(
  rank: Int,
  payload: String,
  mark: Mark,
  nacked_at: Option(Int),
  arrival: Option(Int),
) -> model.RecordStatus {
  RecordHeld(
    held_rank: rank,
    held_payload: <<payload:utf8>>,
    mark:,
    nacked_at:,
    deferred_position: arrival,
  )
}

/// The Rust fixture's `SUPERSEDED_TTL`.
fn superseded_ttl() -> Int {
  harness.fixture_settings().superseded_ttl
}

const defer = "DeferBehindHeldVersion"

const release_requeues = "ReleaseRequeuesPocket"

const requeue_unsettled = "RequeueUnsettledRecord"

const consume_deferred = "ConsumeAckedVersionQueueDeferred"

const dead_letter_deferred = "DeadLetterNackedVersionQueueDeferred"

/// The Rust test file's `marked`: a lease view's record for `key` at rank 1.
fn marked(key: String, payload: String, mark: Mark) -> consuming.HeldView {
  HeldView(key:, held_rank: 1, held_payload: <<payload:utf8>>, mark:)
}

// The catalog rows -----------------------------------------

fn ack_marks() -> List(String) {
  let #(double, lease) = held()
  assert harness.publish(double, "pk", "other", 1, "o")
    == fired(["PublishNewKey"])
  // The Rust's ack answers `Acked`; the specification's answers nothing.
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  let record = harness.record(double, "pk", "k")
  assert record.status == held_as(1, "v1", Acked, None)
  // `impl-choice` (spec notes G04 and G33): an absolute
  // position, which holds only because the queue's first position is 0.
  assert record.position == 1
  let snapshot = harness.inspect(double)
  // "Nothing is removed": the pocket's record keys, by key, as the Rust
  // lists them.
  assert model.records_of(snapshot.records, "pk")
    |> list.map(fn(record) { record.key })
    |> list.sort(string.compare)
    == ["k", "other"]
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.records == [marked("k", "v1", Acked)]
  assert snapshot.superseded == []
  assert snapshot.dead_letters == []
  // The Rust's `invariants::all`, by the checker, which the apply step has
  // also run after each outcome.
  assert invariants.check(snapshot) == []
  harness.fired(double)
}

fn ack_extends() -> List(String) {
  let double =
    harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: 10))
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired(["PublishIntoNewPocket"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold]
  // The Rust asserts the batch's `expires_at`; the lease's is the same fact
  // (N13).
  let assert [granted] = harness.inspect(double).leases
  assert granted.expires_at == harness.t0 + 10
  let _ = harness.advance(double.clock, harness.t0, 4)
  assert harness.ack(double, "c1", batch.lease, "k")
    == fired([ack_marks_record])
  let assert [lease] = harness.inspect(double).leases
  assert lease.expires_at == harness.t0 + 14
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.expires_at == harness.t0 + 14
  harness.fired(double)
}

/// The Rust test file's setup for S042 and S049: `pk` holds `k` and
/// `queued`, and `other` holds `x`; `c1` pulls one record, `k` of `pk`, and
/// `c2` one, `x` of `other`; then 500 ms pass. It answers `c1`'s lease id
/// and `c2`'s.
fn two_leases() -> #(Double, String, String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "pk", "queued", 1, "q")
    == fired(["PublishNewKey"])
  assert harness.publish(double, "other", "x", 1, "x")
    == fired(["PublishIntoNewPocket"])
  let mine = harness.batch(double, "c1", 1)
  assert #(mine.fired, mine.pocket_key) == #([claims, hold], "pk")
  let theirs = harness.batch(double, "c2", 1)
  assert #(theirs.fired, theirs.pocket_key) == #([claims, hold], "other")
  let _ = harness.advance(double.clock, harness.t0, 500)
  #(double, mine.lease, theirs.lease)
}

/// A lease id the fake source never hands out here, for the Rust's
/// `LeaseId::from_u128(99)`.
const never = "lease-99"

/// Each refusal's name and shape is `impl-choice`: `LeaseFenced` (spec note
/// G28, as N21), and `RecordNotHeld` and the lease being checked first
/// (spec note G34, as N21). `nobody` never joined, which is refused before
/// any rule runs, `impl-choice` too (spec note G36, as N22).
/// The Rust tells a lease that is not active, `never`, from one that is not
/// the consumer's, `theirs`; both are `LeaseFenced` here, and the ack by
/// `c1` under `theirs` names `x`, which that lease does hold: the lease is
/// checked first. Then a key the pocket does not have, a queued key of the
/// leased pocket, and `x`, held in another pocket, are each not held.
fn rejected_acks() -> List(String) {
  let #(double, lease, theirs) = two_leases()
  let refused = fn(expected: Refusal, consumer, lease, key) {
    harness.refused(double, expected, fn() {
      harness.ack(double, consumer, lease, key)
    })
  }
  refused(UnknownConsumer("nobody"), "nobody", lease, "k")
  refused(LeaseFenced("Ack", "c1", never), "c1", never, "k")
  refused(LeaseFenced("Ack", "c1", theirs), "c1", theirs, "x")
  list.each(["absent", "queued"], fn(key) {
    refused(RecordNotHeld("Ack", lease, key), "c1", lease, key)
  })
  refused(RecordNotHeld("Ack", lease, "x"), "c1", lease, "x")
  // "No refusal moved the expiry."
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.expires_at == harness.t0 + lease_ttl()
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  // "A repeat is not a refusal." The Rust's answers `Acked` again; here it
  // succeeds, answers nothing, and is the repeat's rule.
  assert harness.ack(double, "c1", lease, "k") == fired([ack_repeats])
  harness.fired(double)
}

/// A "by release or lapse" row in its ending: this port, like its Rust
/// test, ends the lease by release.
fn nack_marks() -> List(String) {
  let #(double, lease) = held()
  let nacked = harness.at(double, 20)
  // The Rust's nack answers `Nacked`; the specification's answers nothing.
  assert harness.nack(double, "c1", lease, "k") == fired([nack_marks_record])
  // "The nack extended it."
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.expires_at == nacked + lease_ttl()
  assert harness.record(double, "pk", "k").status
    == held_as(1, "v1", Nacked, Some(nacked))
  // "Not before the end."
  assert harness.inspect(double).dead_letters == []
  let again = harness.batch(double, "c1", 10)
  assert again.fired == [within]
  assert again.lease == lease
  // "No redelivery."
  assert again.records == []
  // "The pull within the lease extends it": the Rust reads the batch's
  // `expires_at`, and the lease's is the same fact (N13).
  let assert [active] = harness.inspect(double).leases
  assert active.expires_at == nacked + lease_ttl()
  let ended = harness.at(double, 30)
  // The Rust unwraps the release's answer; the specification's release
  // answers nothing.
  assert harness.release(double, "c1", lease)
    == fired(["ReleaseIdlesEmptyPocket", "DeadLetterNackedRecord"])
  let snapshot = harness.inspect(double)
  assert snapshot.dead_letters
    == [
      DeadLetter(
        pocket: "pk",
        key: "k",
        rank: 1,
        payload: <<"v1":utf8>>,
        nacked_by: "c1",
        nacked_at: nacked,
        expires_at: ended + dead_letter_ttl(),
      ),
    ]
  // The Rust's `invariants::nacked_before_expiry`, by the checker.
  assert !list.contains(invariants.check(snapshot), "NackedBeforeExpiry")
  // The pocket is still there, and holds no record.
  let pocket = harness.pocket(double, "pk")
  assert model.records_of(snapshot.records, pocket.key) == []
  harness.fired(double)
}

/// As `rejected_acks`, for a nack: each refusal's name and shape is
/// `impl-choice` (spec note G28 for `LeaseFenced`, and G34 for
/// `RecordNotHeld`, both as N21), and `nobody`'s (spec note G36, as N22).
fn rejected_nacks() -> List(String) {
  let #(double, lease, theirs) = two_leases()
  let refused = fn(expected: Refusal, consumer, lease, key) {
    harness.refused(double, expected, fn() {
      harness.nack(double, consumer, lease, key)
    })
  }
  refused(UnknownConsumer("nobody"), "nobody", lease, "k")
  refused(LeaseFenced("Nack", "c1", never), "c1", never, "k")
  refused(LeaseFenced("Nack", "c1", theirs), "c1", theirs, "x")
  list.each(["absent", "queued", "x"], fn(key) {
    refused(RecordNotHeld("Nack", lease, key), "c1", lease, key)
  })
  assert harness.inspect(double).dead_letters == []
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.expires_at == harness.t0 + lease_ttl()
  harness.fired(double)
}

fn repeats() -> List(String) {
  let #(double, lease) = marked_both_ways()
  let now = harness.at(double, 30)
  let before = harness.inspect(double)
  let assert [active] = before.leases
  let expected =
    Snapshot(..before, leases: [
      Lease(..active, expires_at: now + lease_ttl()),
    ])
  // The Rust's repeats answer `Acked` and `Nacked`, like the first; here
  // each succeeds, answers nothing, and is its repeat's rule.
  assert harness.ack(double, "c1", lease, "k2") == fired([ack_repeats])
  assert harness.nack(double, "c1", lease, "k1") == fired([nack_repeats])
  // "A repeat moves the lease's expiry and nothing else."
  assert harness.inspect(double) == expected
  // "The first nack's time stands."
  assert harness.record(double, "pk", "k1").status
    == held_as(1, "k1", Nacked, Some(harness.t0 + 10))
  harness.fired(double)
}

/// `MarkConflict`'s name and shape are `impl-choice` (spec note G35, which
/// departs from N21 in the shape). The order of the lease view's records is
/// `impl-choice` too (spec note G31, as N23). That a refusal reads no clock
/// is shown twice: by the fake clock's read count, inside
/// `harness.refused` and again across both refusals and the lease read, and
/// as the Rust shows it, by a renew at an earlier time, which would extend
/// from 500 had a refusal's time been kept (spec note G05, as N28).
fn changed_mark() -> List(String) {
  let #(double, lease) = marked_both_ways()
  let _ = harness.at(double, 500)
  let reads = harness.clock_reads(double.clock)
  harness.refused(double, MarkConflict("Ack", lease, "k1"), fn() {
    harness.ack(double, "c1", lease, "k1")
  })
  harness.refused(double, MarkConflict("Nack", lease, "k2"), fn() {
    harness.nack(double, "c1", lease, "k2")
  })
  // "The first marks stand."
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.records == [marked("k1", "k1", Nacked), marked("k2", "k2", Acked)]
  assert harness.clock_reads(double.clock) == reads
  let earlier = harness.at(double, 100)
  // The Rust's renew answers the new expiry; the specification's answers
  // nothing, and the lease holds it (N13).
  assert harness.renew(double, "c1", lease) == fired(["Renew"])
  let assert [active] = harness.inspect(double).leases
  assert active.expires_at == earlier + lease_ttl()
  harness.fired(double)
}

fn repeat_nack_keeps_time() -> List(String) {
  let #(double, lease) = held()
  let first = harness.at(double, 10)
  assert harness.nack(double, "c1", lease, "k") == fired([nack_marks_record])
  let _ = harness.at(double, 30)
  assert harness.nack(double, "c1", lease, "k") == fired([nack_repeats])
  let ended = harness.at(double, 40)
  assert harness.release(double, "c1", lease)
    == fired(["ReleaseIdlesEmptyPocket", "DeadLetterNackedRecord"])
  assert harness.inspect(double).dead_letters
    == [
      DeadLetter(
        pocket: "pk",
        key: "k",
        rank: 1,
        payload: <<"v1":utf8>>,
        nacked_by: "c1",
        nacked_at: first,
        expires_at: ended + dead_letter_ttl(),
      ),
    ]
  harness.fired(double)
}

/// `impl-choice` (spec note G31, as N23): the order. The surface exposes the
/// held records as a set; the view lists them smallest position first, which
/// here is the order they were published in, `z`, `a`, `m`, and not key
/// order.
fn lease_read() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "z", 1, "z")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "pk", "a", 1, "a") == fired(["PublishNewKey"])
  assert harness.publish(double, "pk", "m", 1, "m") == fired(["PublishNewKey"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold, hold]
  assert harness.ack(double, "c1", batch.lease, "a")
    == fired([ack_marks_record])
  assert harness.nack(double, "c1", batch.lease, "m")
    == fired([nack_marks_record])
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.records
    == [
      marked("z", "z", Unsettled),
      marked("a", "a", Acked),
      marked("m", "m", Nacked),
    ]
  harness.fired(double)
}

/// The Rust's publish answers `Superseded`; the specification's answers
/// nothing, and the deferral is asserted through the state (the ruling on
/// N32). `impl-choice` (spec note G04, and G48): the position 1.
fn deferred_behind_held() -> List(String) {
  let #(double, lease) = held()
  let next = harness.inspect(double).next_position
  assert harness.publish(double, "pk", "k", 3, "v3") == fired([defer])
  let record = harness.record(double, "pk", "k")
  // "The newer version is deferred at the counter's value."
  assert record.status == deferred_as(1, "v1", Unsettled, None, Some(next))
  assert #(record.rank, record.payload, record.position)
    == #(3, <<"v3":utf8>>, 1)
  assert harness.inspect(double).next_position == next + 1
  // "Nothing is shed."
  assert harness.inspect(double).superseded == []
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  assert harness.record(double, "pk", "k").status
    == deferred_as(1, "v1", Acked, None, Some(next))
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  // "The lease read shows the held version."
  assert view.records == [marked("k", "v1", Acked)]
  // The Rust's `invariants::all`, by the checker.
  assert invariants.check(harness.inspect(double)) == []
  harness.fired(double)
}

/// `impl-choice` (spec note G04, and G48): the arrival position 3 and the
/// position 1 are absolute. That `k` takes the counter's value when `v3`
/// arrives, after `early` and before `between`, is the specification's.
/// The release frees `k`, `early` and `between`, smallest position first,
/// against a key order that is the reverse: `impl-choice` (spec note G38,
/// as N29).
fn deferred_behind_acked() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "pk", "early", 1, "e")
    == fired(["PublishNewKey"])
  let first = harness.batch(double, "c1", 1)
  assert first.fired == [claims, hold]
  let lease = first.lease
  assert harness.publish(double, "pk", "k", 3, "v3") == fired([defer])
  let arrival = 3
  assert harness.publish(double, "pk", "between", 1, "b")
    == fired(["PublishNewKey"])
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  // "A held record keeps its position until the lease ends."
  assert harness.record(double, "pk", "k").position == 1
  let again = harness.batch(double, "c1", 10)
  assert again.fired == [within, hold, hold]
  assert again.lease == lease
  // "The deferred key is not handed out again under this lease."
  assert harness.keys(again.records) == ["early", "between"]
  assert harness.release(double, "c1", lease)
    == fired([
      release_requeues,
      consume_deferred,
      requeue_unsettled,
      requeue_unsettled,
    ])
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.position, record.rank)
    == #(RecordQueued, arrival, 3)
  let next = harness.batch(double, "c1", 10)
  assert next.fired == [claims, hold, hold, hold]
  assert next.lease != lease
  assert harness.keys(next.records) == ["early", "k", "between"]
  harness.fired(double)
}

/// The Rust's second publish answers `Superseded`; the specification's
/// answers nothing (the ruling on N32). The deferred `v3` is shed with the
/// second publish's time (N30). `impl-choice` (spec note G04, and G48):
/// the arrival position 2.
fn two_supersedes_while_held() -> List(String) {
  let #(double, lease) = held()
  let _ = harness.at(double, 10)
  assert harness.publish(double, "pk", "k", 3, "v3") == fired([defer])
  // "v1 is held."
  assert harness.inspect(double).superseded == []
  let twenty = harness.at(double, 20)
  assert harness.publish(double, "pk", "k", 5, "v5") == fired(["Supersede"])
  let v3 =
    SupersededRecord(
      pocket: "pk",
      key: "k",
      rank: 3,
      payload: <<"v3":utf8>>,
      reason: Superseded,
      shed_at: twenty,
      expires_at: twenty + superseded_ttl(),
    )
  // "v3 is shed at once."
  assert harness.inspect(double).superseded == [v3]
  let record = harness.record(double, "pk", "k")
  // "v5 keeps v3's arrival position."
  assert #(record.status, record.rank)
    == #(deferred_as(1, "v1", Unsettled, None, Some(2)), 5)
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  let _ = harness.at(double, 30)
  assert harness.release(double, "c1", lease)
    == fired([release_requeues, consume_deferred])
  let snapshot = harness.inspect(double)
  // "The acked v1 is consumed."
  assert snapshot.superseded == [v3]
  assert snapshot.dead_letters == []
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.position, record.rank) == #(RecordQueued, 2, 5)
  harness.fired(double)
}

/// The Rust's stale publish answers `Stale`; the specification's answers
/// nothing (the ruling on N32). The equal rank is refused, and the Rust's
/// `EqualRank` carries the record key and the rank where the double's names
/// the pocket as well (spec note G13). `impl-choice` (spec note G04, and
/// G48): the arrival position 2.
fn stale_against_latest() -> List(String) {
  let #(double, _) = held()
  assert harness.publish(double, "pk", "k", 3, "v3") == fired([defer])
  assert harness.publish(double, "pk", "k", 2, "v2")
    == fired(["DropStalePublish"])
  // "An equal rank is rejected, against the latest rank."
  harness.refused(
    double,
    EqualRankRefused(pocket_key: "pk", record_key: "k", rank: 3),
    fn() { harness.publish(double, "pk", "k", 3, "again") },
  )
  let record = harness.record(double, "pk", "k")
  assert record.rank == 3
  assert record.status == deferred_as(1, "v1", Unsettled, None, Some(2))
  harness.fired(double)
}

/// `impl-choice` (spec note G04, and G48): the arrival position 2.
fn acked_then_published() -> List(String) {
  let #(double, lease) = held()
  assert harness.ack(double, "c1", lease, "k") == fired([ack_marks_record])
  assert harness.publish(double, "pk", "k", 2, "v2") == fired([defer])
  let record = harness.record(double, "pk", "k")
  // "The record stays held and acked, with v2 deferred."
  assert #(record.status, record.rank)
    == #(deferred_as(1, "v1", Acked, None, Some(2)), 2)
  let again = harness.batch(double, "c1", 10)
  assert again.fired == [within]
  assert again.lease == lease
  // "Not pullable again in this lease."
  assert again.records == []
  assert harness.release(double, "c1", lease)
    == fired([release_requeues, consume_deferred])
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.position) == #(RecordQueued, 2)
  let next = harness.batch(double, "c1", 10)
  assert next.fired == [claims, hold]
  assert next.lease != lease
  assert harness.keys(next.records) == ["k"]
  let assert [handed] = next.records
  assert handed.rank == 2
  harness.fired(double)
}

/// `impl-choice` (spec note G04, and G48): the arrival position 2. The
/// dead letter keeps the nack's time and decays `dead_letter_ttl` after the
/// release.
fn nacked_then_published() -> List(String) {
  let #(double, lease) = held()
  assert harness.nack(double, "c1", lease, "k") == fired([nack_marks_record])
  assert harness.publish(double, "pk", "k", 2, "v2") == fired([defer])
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.rank)
    == #(deferred_as(1, "v1", Nacked, Some(harness.t0), Some(2)), 2)
  let again = harness.batch(double, "c1", 10)
  assert again.fired == [within]
  // "Not pullable again in this lease."
  assert again.records == []
  assert harness.inspect(double).dead_letters == []
  let ended = harness.at(double, 5)
  assert harness.release(double, "c1", lease)
    == fired([release_requeues, dead_letter_deferred])
  // "The dead letter appears at the release."
  assert harness.inspect(double).dead_letters
    == [
      DeadLetter(
        pocket: "pk",
        key: "k",
        rank: 1,
        payload: <<"v1":utf8>>,
        nacked_by: "c1",
        nacked_at: harness.t0,
        expires_at: ended + dead_letter_ttl(),
      ),
    ]
  let record = harness.record(double, "pk", "k")
  // "v2 is queued at its arrival position."
  assert #(record.status, record.position, record.rank) == #(RecordQueued, 2, 2)
  let next = harness.batch(double, "c1", 10)
  assert next.fired == [claims, hold]
  assert harness.keys(next.records) == ["k"]
  harness.fired(double)
}

// The rules themselves, over states built by hand ------------
//
// Through the owner a pocket's `consecutive_expiries` is 0 until a lease can
// lapse, so the reset the two marking rules ensure would not show there; and
// a mark by key alone would pass every scenario above, since no two pockets
// there hold one key. These tests run the apply step over a state built by
// hand, with the checker reading what each outcome leaves.

/// The time `c1`'s lease was granted at.
const granted = 600

/// A record held at its own rank and payload with this mark and this nack
/// time, and nothing deferred behind it.
fn held_with(record: Record, mark: Mark, nacked_at: Option(Int)) -> Record {
  Record(
    ..record,
    status: RecordHeld(
      held_rank: record.rank,
      held_payload: record.payload,
      mark:,
      nacked_at:,
      deferred_position: None,
    ),
  )
}

/// Consumers `c1` and `c2` joined, and no list is in position order.
///
/// - `c1` holds `pk` under `lease-1`, granted at 600. `pk` holds `k2` and
///   `k1`, both held, and `k3`, queued. `k1` is as `k1` says and `k2`
///   unsettled. `pk` has seen `expiries` consecutive expiries.
/// - `c1` once held `pk` under `lease-0`, which ended as `ended` says.
/// - `c2` holds `other` under `lease-2`. `other` holds a `k1` of its own,
///   held and unsettled, and `kx`, held, which `pk` does not have.
/// - `done` is idle.
/// - The superseded queue holds an earlier version of `pk`'s `k1`, and the
///   dead-letter queue a record `c1` nacked out of `pk` under `lease-0`.
///
/// Each rank and payload is unlike its position and unlike the others.
fn marking(
  expiries: Int,
  k1: fn(Record) -> Record,
  ended: LeaseStatus,
) -> State {
  let ttl = config.defaults().lease_ttl
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 20),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1"), Consumer("c2")],
    pockets: [
      Pocket("other", PocketLeased, 9, 0),
      Pocket("pk", PocketLeased, 2, expiries),
      Pocket("done", PocketIdle, 1, 1),
    ],
    records: [
      unsettled(Record("other", "k1", 47, <<"theirs":utf8>>, 10, RecordQueued)),
      unsettled(Record("pk", "k2", 45, <<"two":utf8>>, 6, RecordQueued)),
      k1(Record("pk", "k1", 44, <<"one":utf8>>, 3, RecordQueued)),
      Record("pk", "k3", 46, <<"three":utf8>>, 8, RecordQueued),
      unsettled(Record("other", "kx", 48, <<"x":utf8>>, 12, RecordQueued)),
    ],
    leases: [
      Lease(
        id: "lease-0",
        pocket: "pk",
        consumer: "c1",
        status: ended,
        granted_at: 100,
        expires_at: 100 + ttl,
      ),
      Lease(
        id: "lease-2",
        pocket: "other",
        consumer: "c2",
        status: LeaseActive,
        granted_at: 500,
        expires_at: 500 + ttl,
      ),
      Lease(
        id: "lease-1",
        pocket: "pk",
        consumer: "c1",
        status: LeaseActive,
        granted_at: granted,
        expires_at: granted + ttl,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "k1",
        rank: 43,
        payload: <<"older":utf8>>,
        reason: Superseded,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "pk",
        key: "k0",
        rank: 42,
        payload: <<"zero":utf8>>,
        nacked_by: "c1",
        nacked_at: 120,
        expires_at: 150 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

fn unsettled(record: Record) -> Record {
  held_with(record, Unsettled, None)
}

fn acked(record: Record) -> Record {
  held_with(record, Acked, None)
}

/// The time the hand-built `k1` was nacked at, where it is nacked.
const nacked_at = 700

fn nacked(record: Record) -> Record {
  held_with(record, Nacked, Some(nacked_at))
}

/// `lease-0` as it ended by release.
const released = LeaseReleased(150)

/// One trigger through the apply step at this time, with the clock reads
/// and the lease-id draws it asked for.
fn applied(state: State, trigger: core.Trigger, now: Int) -> harness.Driven {
  harness.drive(core.apply(state, trigger), now, "unused")
}

/// `c1` acks `pk`'s unsettled `k1` at 900, `pk` having seen two expiries.
/// The lease's `expires_at` is 900 plus `lease_ttl`, the pocket's count is
/// 0 and the record is acked, with no nack time. Nothing else in the
/// complete state changes: not `other`'s `k1`, not `k2`, not the lease's
/// `granted_at`, not `lease-0` or `lease-2`, not the idle pocket's count,
/// no position, and neither shed queue.
fn ack_marks_exactly() -> List(String) {
  let state = marking(2, unsettled, released)
  let assert [other, pk, done] = state.pockets
  let assert [other_k1, k2, k1, k3, kx] = state.records
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 900
  assert applied(state, core.Ack(Ack("c1", "lease-1", "k1")), now)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          pockets: [other, Pocket(..pk, consecutive_expiries: 0), done],
          records: [other_k1, k2, acked(k1), k3, kx],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, expires_at: now + config.defaults().lease_ttl),
          ],
        ),
        [ack_marks_record],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  [ack_marks_record]
}

/// `c1` acks `k1` again at 1100. Only the lease's `expires_at` changes: the
/// record stays acked.
fn ack_repeats_exactly() -> List(String) {
  let state = marking(0, acked, released)
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 1100
  assert applied(state, core.Ack(Ack("c1", "lease-1", "k1")), now)
    == Driven(
      Ok(Applied(
        State(..state, leases: [
          lease_0,
          lease_2,
          Lease(..lease_1, expires_at: now + config.defaults().lease_ttl),
        ]),
        [ack_repeats],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  [ack_repeats]
}

/// `c1` nacks `pk`'s unsettled `k1` at 950, `pk` having seen two expiries.
/// The lease's `expires_at` is 950 plus `lease_ttl`, the pocket's count is
/// 0, and the record is nacked with `nacked_at` 950: the time of the nack,
/// not the lease's grant or its expiry. No dead letter is made, and nothing
/// else in the complete state changes.
fn nack_marks_exactly() -> List(String) {
  let state = marking(2, unsettled, released)
  let assert [other, pk, done] = state.pockets
  let assert [other_k1, k2, k1, k3, kx] = state.records
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 950
  assert applied(state, core.Nack(Nack("c1", "lease-1", "k1")), now)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          pockets: [other, Pocket(..pk, consecutive_expiries: 0), done],
          records: [other_k1, k2, held_with(k1, Nacked, Some(now)), k3, kx],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, expires_at: now + config.defaults().lease_ttl),
          ],
        ),
        [nack_marks_record],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  [nack_marks_record]
}

/// `c1` nacks `k1` again at 1200, having nacked it at 700. Only the lease's
/// `expires_at` changes: the record keeps the first nack's time.
fn nack_repeats_exactly() -> List(String) {
  let state = marking(0, nacked, released)
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 1200
  assert applied(state, core.Nack(Nack("c1", "lease-1", "k1")), now)
    == Driven(
      Ok(Applied(
        State(..state, leases: [
          lease_0,
          lease_2,
          Lease(..lease_1, expires_at: now + config.defaults().lease_ttl),
        ]),
        [nack_repeats],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  [nack_repeats]
}

/// A record held at an older version than its own, rank 40 and payload
/// "held", with this mark and this nack time, and its own rank and payload
/// deferred behind it at arrival position 15: above its position, below the
/// counter and unlike every other position.
fn deferred_with(record: Record, mark: Mark, nacked_at: Option(Int)) -> Record {
  Record(
    ..record,
    status: RecordHeld(
      held_rank: 40,
      held_payload: <<"held":utf8>>,
      mark:,
      nacked_at:,
      deferred_position: Some(15),
    ),
  )
}

/// `ensures: record.mark = mark`, and nothing of the held version or the
/// deferred one. With nothing deferred the held version is the record's
/// own, so a mark that rewrote it from the record would pass the tests
/// above. Here `k1` holds rank 40 and "held" with rank 44 and "one" deferred
/// at 15. An ack at 900 and a nack at 950, `pk` having seen two expiries,
/// leave the held version, the deferred position and the record's own rank
/// and payload as they were. The four mark rules require nothing of a
/// deferred version, so dispatch finds the same rule for each mark as with
/// nothing deferred.
fn deferred_marks() -> List(String) {
  let deferred = fn(record) { deferred_with(record, Unsettled, None) }
  let state = marking(2, deferred, released)
  let assert [other, pk, done] = state.pockets
  let assert [other_k1, k2, k1, k3, kx] = state.records
  let assert [lease_0, lease_2, lease_1] = state.leases
  let after = fn(marked, now) {
    State(
      ..state,
      pockets: [other, Pocket(..pk, consecutive_expiries: 0), done],
      records: [other_k1, k2, marked, k3, kx],
      leases: [
        lease_0,
        lease_2,
        Lease(..lease_1, expires_at: now + config.defaults().lease_ttl),
      ],
    )
  }
  assert applied(state, core.Ack(Ack("c1", "lease-1", "k1")), 900)
    == Driven(
      Ok(Applied(
        after(deferred_with(k1, Acked, None), 900),
        [ack_marks_record],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  assert applied(state, core.Nack(Nack("c1", "lease-1", "k1")), 950)
    == Driven(
      Ok(Applied(
        after(deferred_with(k1, Nacked, Some(950)), 950),
        [nack_marks_record],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  let marked = fn(mark, nacked_at) {
    marking(0, fn(record) { deferred_with(record, mark, nacked_at) }, released)
  }
  assert mark_rules(state, "c1", "lease-1", "k1")
    == #(Some(ack_marks_record), Some(nack_marks_record))
  assert mark_rules(marked(Acked, None), "c1", "lease-1", "k1")
    == #(Some(ack_repeats), None)
  assert mark_rules(marked(Nacked, Some(nacked_at)), "c1", "lease-1", "k1")
    == #(None, Some(nack_repeats))
  [ack_marks_record, nack_marks_record]
}

/// `ensures: lease.expires_at = now + config.lease_ttl`, at the boundary
/// `LeaseTtlNonNegative` admits. With `lease_ttl` 0 each of the four mark
/// rules and `Renew` leaves `lease-1` expiring at the time it read, and
/// reads the clock once.
fn zero_lease_ttl() -> List(String) {
  let assert Ok(zero) = config.new(Settings(..config.defaults(), lease_ttl: 0))
  let with = fn(k1) { State(..marking(0, k1, released), config: zero) }
  let expiry = fn(state, trigger, now) {
    let assert Driven(
      Ok(Applied(after, fired, None)),
      clock_reads: 1,
      lease_id_draws: 0,
    ) = applied(state, trigger, now)
    let assert Ok(lease) = model.lease(after.leases, "lease-1")
    #(fired, lease.expires_at)
  }
  assert expiry(with(unsettled), core.Ack(Ack("c1", "lease-1", "k1")), 900)
    == #([ack_marks_record], 900)
  assert expiry(with(acked), core.Ack(Ack("c1", "lease-1", "k1")), 1000)
    == #([ack_repeats], 1000)
  assert expiry(with(unsettled), core.Nack(Nack("c1", "lease-1", "k1")), 1100)
    == #([nack_marks_record], 1100)
  assert expiry(with(nacked), core.Nack(Nack("c1", "lease-1", "k1")), 1200)
    == #([nack_repeats], 1200)
  assert expiry(with(unsettled), core.Renew(Renew("c1", "lease-1")), 1300)
    == #(["Renew"], 1300)
  [ack_marks_record, ack_repeats, nack_marks_record, nack_repeats, "Renew"]
}

/// The rule the Ack table sends this ack to over this state, if any.
fn ack_rule(state: State, ack: Ack) -> Option(String) {
  let assert Ok(row) = rules.dispatch(settle.ack(), state, ack)
  option.map(row, fn(row) { row.name })
}

/// The rule the Nack table sends this nack to over this state, if any.
fn nack_rule(state: State, nack: Nack) -> Option(String) {
  let assert Ok(row) = rules.dispatch(settle.nack(), state, nack)
  option.map(row, fn(row) { row.name })
}

/// The rules an ack and a nack with these three names are sent to.
fn mark_rules(
  state: State,
  consumer: String,
  lease: String,
  key: String,
) -> #(Option(String), Option(String)) {
  #(
    ack_rule(state, Ack(consumer, lease, key)),
    nack_rule(state, Nack(consumer, lease, key)),
  )
}

/// Each `requires` line shown to matter on its own. Under `c1`'s active
/// lease the record's mark picks the rule: unsettled, the marking rule;
/// already marked the same way, the repeat; marked the other way, none. No
/// rule holds for a queued record of the leased pocket, a key the pocket
/// does not have, or a key held only in another pocket. None holds under
/// `c1`'s own lease once it has been released or has expired, though `c1`
/// holds the same pocket and the same key again; under another consumer's
/// active lease, though that lease holds the key; or under an id that names
/// no lease. `c2` under its own lease is sent to the marking rules. Only
/// what dispatch finds is asserted.
fn mark_guards() -> List(String) {
  let state = marking(0, unsettled, released)
  assert mark_rules(state, "c1", "lease-1", "k1")
    == #(Some(ack_marks_record), Some(nack_marks_record))
  assert mark_rules(marking(0, acked, released), "c1", "lease-1", "k1")
    == #(Some(ack_repeats), None)
  assert mark_rules(marking(0, nacked, released), "c1", "lease-1", "k1")
    == #(None, Some(nack_repeats))
  assert mark_rules(state, "c1", "lease-1", "k3") == #(None, None)
  assert mark_rules(state, "c1", "lease-1", "absent") == #(None, None)
  assert mark_rules(state, "c1", "lease-1", "kx") == #(None, None)
  assert mark_rules(state, "c1", "lease-0", "k1") == #(None, None)
  assert mark_rules(
      marking(0, unsettled, LeaseExpired(150)),
      "c1",
      "lease-0",
      "k1",
    )
    == #(None, None)
  assert mark_rules(state, "c1", "lease-2", "k1") == #(None, None)
  assert mark_rules(state, "c2", "lease-1", "k1") == #(None, None)
  assert mark_rules(state, "c1", "lease-9", "k1") == #(None, None)
  assert mark_rules(state, "c1", "pk", "k1") == #(None, None)
  assert mark_rules(state, "c2", "lease-2", "k1")
    == #(Some(ack_marks_record), Some(nack_marks_record))
  assert mark_rules(state, "c2", "lease-2", "kx")
    == #(Some(ack_marks_record), Some(nack_marks_record))
  []
}

/// What an ack and a nack with these three names are refused with: each
/// step is done at once, so neither asked for the time or a lease id, and a
/// failure hands back no state.
fn mark_refusals_of(
  state: State,
  consumer: String,
  lease: String,
  key: String,
  expected: fn(String) -> Refusal,
) -> Nil {
  assert core.apply(state, core.Ack(Ack(consumer, lease, key)))
    == Done(Error(Refused(expected("Ack"))))
  assert core.apply(state, core.Nack(Nack(consumer, lease, key)))
    == Done(Error(Refused(expected("Nack"))))
  Nil
}

/// `impl-choice` (spec note G28 for `LeaseFenced`; G34, G35 and G36 for the
/// rest, as N21 and N22): the refusals' names, their shapes and the order
/// they are checked in. A consumer that never joined is refused before any
/// table is read. Then the lease: one that has ended, by release
/// or by expiry, another consumer's, or an id that names none is fenced
/// whatever key the action names, a key that lease holds included. Then the
/// record: a key the lease's pocket does not have, a queued one, or one held
/// only in another pocket is not held. Then the mark: the other mark already
/// stands.
fn mark_refusals() -> List(String) {
  let state = marking(0, unsettled, released)
  let fenced = fn(consumer, lease) {
    fn(action) { LeaseFenced(action, consumer, lease) }
  }
  let not_held = fn(lease, key) {
    fn(action) { RecordNotHeld(action, lease, key) }
  }
  mark_refusals_of(state, "cx", "lease-1", "k1", fn(_) { UnknownConsumer("cx") })
  mark_refusals_of(state, "p", "lease-1", "k1", fn(_) { UnknownConsumer("p") })
  mark_refusals_of(state, "c1", "lease-0", "k1", fenced("c1", "lease-0"))
  mark_refusals_of(
    marking(0, unsettled, LeaseExpired(150)),
    "c1",
    "lease-0",
    "k1",
    fenced("c1", "lease-0"),
  )
  mark_refusals_of(state, "c1", "lease-2", "k1", fenced("c1", "lease-2"))
  mark_refusals_of(state, "c2", "lease-1", "k1", fenced("c2", "lease-1"))
  mark_refusals_of(state, "c1", "lease-9", "k1", fenced("c1", "lease-9"))
  mark_refusals_of(state, "c1", "lease-9", "absent", fenced("c1", "lease-9"))
  mark_refusals_of(
    state,
    "c1",
    "lease-1",
    "absent",
    not_held("lease-1", "absent"),
  )
  mark_refusals_of(state, "c1", "lease-1", "k3", not_held("lease-1", "k3"))
  mark_refusals_of(state, "c1", "lease-1", "kx", not_held("lease-1", "kx"))
  assert core.apply(
      marking(0, nacked, released),
      core.Ack(Ack("c1", "lease-1", "k1")),
    )
    == Done(Error(Refused(MarkConflict("Ack", "lease-1", "k1"))))
  assert core.apply(
      marking(0, acked, released),
      core.Nack(Nack("c1", "lease-1", "k1")),
    )
    == Done(Error(Refused(MarkConflict("Nack", "lease-1", "k1"))))
  []
}

/// Each mark rule's effect reads the held record its guard saw. Where there
/// is none, which only a wrong guard could bring about, the outcome is a
/// defect that names the rule: a value, not a crash. The step is done
/// without asking for the time. Each rule is run under a guard that always
/// holds, for a queued record of the leased pocket.
fn mark_without_record() -> List(String) {
  let state = marking(0, unsettled, released)
  let reason = "no held record k3 under lease lease-1"
  list.each(settle.ack().rows, fn(row) {
    let unguarded =
      Table(
        "Ack",
        [Row(row.name, fn(_state, _ack) { True }, row.effect)],
        Refuses(fn(_state, _ack) { UnknownConsumer("unused") }),
      )
    assert core.run(unguarded, Ack("c1", "lease-1", "k3"), state)
      == Done(Error(Defect("Ack", EffectFailed(row.name, reason))))
  })
  list.each(settle.nack().rows, fn(row) {
    let unguarded =
      Table(
        "Nack",
        [Row(row.name, fn(_state, _nack) { True }, row.effect)],
        Refuses(fn(_state, _nack) { UnknownConsumer("unused") }),
      )
    assert core.run(unguarded, Nack("c1", "lease-1", "k3"), state)
      == Done(Error(Defect("Nack", EffectFailed(row.name, reason))))
  })
  []
}
