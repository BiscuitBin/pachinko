//// The rest of `crates/libpachinko/tests/queue/expiry.rs`: the lapse's three
//// fates, parking and its revival, the resets of the consecutive-expiry
//// count, fencing after a lapse, and the one strict order. Each test is a
//// port of its Rust test, driving the owner through `harness`.
////
//// A port asserts what its Rust test asserts, with these changes:
////
//// - The Rust's publish answers `NewKey`, `Superseded`, `Created` or `Stale`;
////   the specification's publish answers nothing, so a port asserts the
////   rules it fired (the ruling on N32, G15, G54). S077's mid-lease publish
////   is `Superseded` in the Rust and a deferral here.
//// - The Rust's ack answers `Acked` and its release the pocket's fate and the
////   ended lease; the specification's answer nothing, so a port asserts the
////   rules fired and reads the fate from the snapshot (the ruling on N24,
////   G37).
//// - The Rust's fencing refusal is `LeaseNotActive`, its equal-rank one
////   `EqualRank` and its not-held one `RecordNotHeld`; the double's are
////   `LeaseFenced` (G28, G34), `EqualRankRefused` (G13) and
////   `RecordNotHeld`. Every refusal goes through `harness.refused`, which
////   compares the complete snapshot and the clock's reads before and after.
//// - The Rust forks one queue into two (S024, S063). The double cannot fork
////   an owner, so the port builds the same setup on two owners and checks
////   that their snapshots agree before they part. The Rust's forks share a
////   clock, so the port sets each owner's clock to where that shared clock
////   stands: S063's acked owner lapses at t0 + 40, as the Rust's does.
//// - S048's Rust asserts absolute positions; the port compares them with the
////   state before the step (N38).
//// - S075 checks the Rust's stored pocket order at each step; the double
////   stores none, so the port asserts the position invariants from the
////   snapshot and an empty `check` (the ruling on N38).
//// - S064 and S077 are `wire` rows; only their library halves are here.

import gleam/list
import gleam/option.{None, Some}
import pachinko_double.{type Reply, Outcome}
import pachinko_double/answers.{type TickReport, BatchRecord, NoPocketAvailable}
import pachinko_double/config.{Settings}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type PocketStatus, DeadLetter, LeaseExpired, Nacked, PocketIdle, PocketLeased,
  PocketParked, PocketQueued, RecordHeld, RecordQueued, Stale, SupersededRecord,
}
import pachinko_double/refusals.{EqualRankRefused, LeaseFenced, RecordNotHeld}
import support/harness.{type Double, type Pulled}
import support/runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a released lease never lapses afterwards",
      [Scenario("S058")],
      released_never_lapses,
    ),
    Test(
      "a lapse keeps the pocket at the front and requeues its records",
      [Scenario("S060")],
      lapse_keeps_at_front,
    ),
    Test(
      "a lapse with no records idles the pocket",
      [Scenario("S061")],
      lapse_idles_empty,
    ),
    Test(
      "three consecutive lapses park the pocket, and a publish revives it",
      [Scenario("S062")],
      three_lapses_park,
    ),
    Test(
      "a supersede revives a parked pocket",
      [Scenario("S024")],
      supersede_revives_parked,
    ),
    Test(
      "a stale publish into a parked pocket leaves it parked with its count",
      [Scenario("S092")],
      stale_leaves_parked,
    ),
    Test(
      "an equal-rank publish does not revive a parked pocket",
      [Scenario("S025")],
      equal_rank_leaves_parked,
    ),
    Test(
      "only a publish revives a parked pocket (oq:7)",
      [Scenario("S026")],
      only_a_publish_revives,
    ),
    Test(
      "idle, parked and leased pockets are never claimed",
      [Scenario("S034")],
      never_claimed,
    ),
    Test(
      "a version deferred behind a nacked one is queued at its arrival position by the lapse",
      [Scenario("S048")],
      deferred_behind_nacked_lapse,
    ),
    Test(
      "a nack resets the expiry count, and a pull or a renew does not",
      [Scenario("S064")],
      nack_resets_count,
    ),
    Test(
      "an ack resets the consecutive-expiry count",
      [Scenario("S077")],
      ack_resets_count,
    ),
    Test(
      "a release resets the consecutive-expiry count",
      [Scenario("S056")],
      release_resets_count,
    ),
    Test(
      "at the threshold an empty pocket parks rather than idling",
      [Scenario("S063")],
      threshold_parks_empty,
    ),
    Test(
      "after a lapse the old lease is fenced even when the key has a newer rank",
      [Scenario("S069")],
      fenced_after_lapse,
    ),
    Test(
      "pockets and records form one strict, deterministic order",
      [Scenario("S075")],
      one_strict_order,
    ),
  ]
}

/// The lease every test here uses, in milliseconds, as the Rust's `TTL`.
const ttl = 10

const keeps = "LeaseExpiryKeepsPocketAtFront"

const parks = "LeaseExpiryParksPocket"

const idles = "LeaseExpiryIdlesEmptyPocket"

const requeue = "RequeueUnsettledRecord"

fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

fn fired_all(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}

/// The Rust test file's `fixture`: a 10 ms lease.
fn short() -> Double {
  harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: ttl))
}

/// The Rust test file's `two_pockets`: pocket `a` holding `a1` and `a2`,
/// then pocket `b` holding `b1`, with a 10 ms lease.
fn two_pockets() -> Double {
  two_pockets_on(short())
}

fn two_pockets_on(double: Double) -> Double {
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "a2", 1, "a2") == fired("PublishNewKey")
  assert harness.publish(double, "b", "b1", 1, "b1")
    == fired("PublishIntoNewPocket")
  double
}

/// The Rust fixture's `tick`: one tick, which must not fail, after which
/// every invariant holds, `ExpiriesWithinLimit` among them.
fn tick(double: Double) -> TickReport {
  let assert Ok(report) = harness.tick(double)
  check(double)
  report
}

/// The Rust fixture's `check`.
fn check(double: Double) -> Nil {
  assert invariants.check(harness.inspect(double)) == []
}

/// A pull by `consumer` that claims a pocket: it fired the claim and one
/// `HoldRecord` for each record of the batch.
fn claim(double: Double, consumer: String) -> Pulled {
  let batch = harness.batch(double, consumer, 10)
  assert batch.fired
    == [
      "PullClaimsFrontPocket",
      ..list.repeat("HoldRecord", list.length(batch.records))
    ]
  batch
}

/// The Rust test file's `lapse`: the clock moved one lease on from `from`,
/// to `from + ttl` after t0, and a tick, which lapses exactly one lease and
/// fires `rules`. Answers the report.
fn lapse(double: Double, from: Int, rules: List(String)) -> TickReport {
  let _ = harness.at(double, from + ttl)
  let report = tick(double)
  // One lease lapsed.
  let assert [_] = report.lapsed
  assert report.fired == rules
  report
}

/// The Rust test file's `claim_and_lapse`, at `from` after t0: `c1` claims
/// the front pocket, which must be `pocket`, and the lease lapses a lease
/// later, firing `LeaseLapses`, `rule` and one `RequeueUnsettledRecord` for
/// each record of the batch, which the test leaves unsettled.
fn claim_and_lapse(
  double: Double,
  pocket: String,
  from: Int,
  rule: String,
) -> TickReport {
  let batch = claim(double, "c1")
  assert batch.pocket_key == pocket
  lapse(double, from, [
    "LeaseLapses",
    rule,
    ..list.repeat(requeue, list.length(batch.records))
  ])
}

/// The Rust test file's `parked`: pocket `a` parked by three lapses in a
/// row, at t0, t0 + 10 and t0 + 20, with `b` queued behind it. The clock is
/// left at t0 + 30.
fn parked() -> Double {
  let double = two_pockets()
  let _ = claim_and_lapse(double, "a", 0, keeps)
  let _ = claim_and_lapse(double, "a", ttl, keeps)
  let _ = claim_and_lapse(double, "a", 2 * ttl, parks)
  assert harness.pocket(double, "a").status == PocketParked
  double
}

/// The pocket's status and consecutive-expiry count.
fn count(double: Double, pocket: String) -> #(PocketStatus, Int) {
  let pocket = harness.pocket(double, pocket)
  #(pocket.status, pocket.consecutive_expiries)
}

/// Two owners built alike, standing in for the Rust's `fork`: their
/// snapshots agree.
fn forked(build: fn() -> Double) -> #(Double, Double) {
  let first = build()
  let second = build()
  assert harness.inspect(second) == harness.inspect(first)
  #(first, second)
}

fn both(first: Double, second: Double) -> List(String) {
  list.append(harness.fired(first), harness.fired(second))
}

// The catalog rows ------------------------------------------

fn released_never_lapses() -> List(String) {
  let double = two_pockets()
  let batch = claim(double, "c1")
  assert harness.release(double, "c1", batch.lease)
    == fired_all(["ReleaseRequeuesPocket", requeue, requeue])
  let after = harness.inspect(double)
  list.each([10, 11, 1000], fn(later) {
    let _ = harness.at(double, later)
    // A released lease lapsed.
    let report = tick(double)
    assert report.lapsed == []
    assert report.fired == []
  })
  assert harness.inspect(double) == after
  assert harness.pocket(double, "a").consecutive_expiries == 0
  harness.fired(double)
}

fn lapse_keeps_at_front() -> List(String) {
  let double = two_pockets()
  let positions =
    list.map(["a1", "a2"], fn(key) { harness.record(double, "a", key).position })
  let position = harness.pocket(double, "a").position
  let first = claim(double, "c1")
  assert harness.keys(first.records) == ["a1", "a2"]
  let _ = lapse(double, 0, ["LeaseLapses", keeps, requeue, requeue])
  let pocket = harness.pocket(double, "a")
  assert #(pocket.status, pocket.position, pocket.consecutive_expiries)
    == #(PocketQueued, position, 1)
  assert harness.order(double) == ["a", "b"]
  list.zip(["a1", "a2"], positions)
  |> list.each(fn(pair) {
    let #(key, position) = pair
    let record = harness.record(double, "a", key)
    assert #(record.status, record.position) == #(RecordQueued, position)
  })
  let second = claim(double, "c2")
  // The lapsed pocket kept the front.
  assert second.pocket_key == "a"
  assert second.lease != first.lease
  assert harness.keys(second.records) == ["a1", "a2"]
  harness.fired(double)
}

fn lapse_idles_empty() -> List(String) {
  let double = two_pockets()
  let batch = claim(double, "c1")
  assert harness.ack(double, "c1", batch.lease, "a1") == fired("AckMarksRecord")
  assert harness.ack(double, "c1", batch.lease, "a2") == fired("AckMarksRecord")
  let report =
    lapse(double, 0, [
      "LeaseLapses", idles, "ConsumeAckedRecord", "ConsumeAckedRecord",
    ])
  let assert [lapsed] = report.lapsed
  assert lapsed.status == LeaseExpired(ended_at: harness.t0 + ttl)
  assert count(double, "a") == #(PocketIdle, 1)
  assert harness.order(double) == ["b"]
  harness.fired(double)
}

fn three_lapses_park() -> List(String) {
  let double = two_pockets()
  let _ = claim_and_lapse(double, "a", 0, keeps)
  assert count(double, "a") == #(PocketQueued, 1)
  let _ = claim_and_lapse(double, "a", ttl, keeps)
  assert count(double, "a") == #(PocketQueued, 2)
  let report = claim_and_lapse(double, "a", 2 * ttl, parks)
  let assert [lapsed] = report.lapsed
  let assert LeaseExpired(_) = lapsed.status
  assert count(double, "a") == #(PocketParked, 3)
  assert harness.record(double, "a", "a1").status == RecordQueued
  assert claim(double, "c2").pocket_key == "b"
  let next = harness.inspect(double).next_position
  assert harness.publish(double, "a", "a3", 1, "a3")
    == fired_all(["PublishNewKey", "ReviveParkedPocket"])
  let pocket = harness.pocket(double, "a")
  // Revived at the back, after a3's own position, with the count reset
  // (G45).
  assert #(pocket.status, pocket.position, pocket.consecutive_expiries)
    == #(PocketQueued, next + 1, 0)
  check(double)
  harness.fired(double)
}

fn supersede_revives_parked() -> List(String) {
  let #(fork, other) = forked(parked)
  let next = harness.inspect(fork).next_position
  assert harness.publish(fork, "a", "a1", 2, "x")
    == fired_all(["Supersede", "ReviveParkedPocket"])
  let pocket = harness.pocket(fork, "a")
  // A supersede revives at the back.
  assert #(pocket.status, pocket.position, pocket.consecutive_expiries)
    == #(PocketQueued, next, 0)
  assert harness.order(fork) == ["b", "a"]
  check(fork)
  // A publish into a pocket that does not exist creates it; nothing is
  // revived.
  assert harness.publish(other, "z", "z1", 1, "z")
    == fired("PublishIntoNewPocket")
  assert harness.pocket(other, "a").status == PocketParked
  both(fork, other)
}

fn stale_leaves_parked() -> List(String) {
  let double = parked()
  let before = harness.inspect(double)
  let position = harness.pocket(double, "a").position
  let now = harness.t0 + 3 * ttl
  assert harness.publish(double, "a", "a1", 0, "x") == fired("DropStalePublish")
  let pocket = harness.pocket(double, "a")
  // A stale publish touches nothing.
  assert #(pocket.status, pocket.position, pocket.consecutive_expiries)
    == #(PocketParked, position, 3)
  assert harness.order(double) == ["b"]
  let snapshot = harness.inspect(double)
  assert snapshot.next_position == before.next_position
  // The arrival is recorded as stale.
  assert snapshot.superseded
    == [
      SupersededRecord(
        pocket: "a",
        key: "a1",
        rank: 0,
        payload: <<"x":utf8>>,
        reason: Stale,
        shed_at: now,
        expires_at: now + harness.fixture_settings().superseded_ttl,
      ),
    ]
  check(double)
  harness.fired(double)
}

fn equal_rank_leaves_parked() -> List(String) {
  let double = parked()
  harness.refused(double, EqualRankRefused("a", "a1", 1), fn() {
    harness.publish(double, "a", "a1", 1, "x")
  })
  assert harness.pocket(double, "a").status == PocketParked
  harness.fired(double)
}

/// `oq:7` (open question 7, spec notes G65 and N36): only a publish revives a
/// parked pocket, as the rules stand. An operator action or a cooldown would
/// be a new rule. Time, ticks, and another pocket's lease, publish and
/// release leave `a` parked with its count.
fn only_a_publish_revives() -> List(String) {
  let double = parked()
  let lease = claim(double, "c2").lease
  assert harness.pocket(double, "b").status == PocketLeased
  // A publish into a pocket that is not parked leaves it where it is.
  assert harness.publish(double, "b", "b2", 1, "b2") == fired("PublishNewKey")
  assert harness.pocket(double, "b").status == PocketLeased
  assert harness.release(double, "c2", lease)
    == fired_all(["ReleaseRequeuesPocket", requeue])
  list.each([1000, 1_000_000], fn(later) {
    let _ = harness.at(double, later)
    assert tick(double).fired == []
  })
  let lease = claim(double, "c3").lease
  assert harness.release(double, "c3", lease)
    == fired_all(["ReleaseRequeuesPocket", requeue, requeue])
  assert claim(double, "c1").pocket_key == "b"
  assert harness.pull(double, "c2", 10)
    == Ok(Outcome(["PullFindsNoPocket"], Some(NoPocketAvailable("c2"))))
  assert count(double, "a") == #(PocketParked, 3)
  harness.fired(double)
}

fn never_claimed() -> List(String) {
  let double = parked()
  let lease = claim(double, "c1").lease
  assert harness.ack(double, "c1", lease, "b1") == fired("AckMarksRecord")
  assert harness.release(double, "c1", lease)
    == fired_all(["ReleaseIdlesEmptyPocket", "ConsumeAckedRecord"])
  assert harness.pocket(double, "b").status == PocketIdle
  assert harness.publish(double, "c", "c1", 1, "c1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "d", "d1", 1, "d1")
    == fired("PublishIntoNewPocket")
  assert claim(double, "c2").pocket_key == "c"
  // Now `a` is parked, `b` idle and `c` leased: only `d` is claimable.
  assert claim(double, "c3").pocket_key == "d"
  assert harness.consumer_joins(double, "c4") == fired("ConsumerJoins")
  assert harness.pull(double, "c4", 10)
    == Ok(Outcome(["PullFindsNoPocket"], Some(NoPocketAvailable("c4"))))
  check(double)
  harness.fired(double)
}

/// The Rust asserts the held record at position 1 and the arrival position
/// 5; the port compares both with the state before the publish (N38), and
/// keeps the absolute values as `impl-choice` (spec notes G04 and G48).
/// `impl-choice` (spec note G38, as N25): the order of the two `Freed` rules
/// the lapse fires, smallest held position first.
fn deferred_behind_nacked_lapse() -> List(String) {
  let double = two_pockets()
  let lease = claim(double, "c1").lease
  let own = harness.record(double, "a", "a1").position
  let arrival = harness.inspect(double).next_position
  assert #(own, arrival) == #(1, 5)
  assert harness.publish(double, "a", "a1", 2, "newer")
    == fired("DeferBehindHeldVersion")
  assert harness.nack(double, "c1", lease, "a1") == fired("NackMarksRecord")
  let held = harness.record(double, "a", "a1")
  // Still held at its own position, the arrival position beside it.
  assert #(held.status, held.position)
    == #(
      RecordHeld(
        held_rank: 1,
        held_payload: <<"a1":utf8>>,
        mark: Nacked,
        nacked_at: Some(harness.t0),
        deferred_position: Some(arrival),
      ),
      own,
    )
  assert harness.inspect(double).dead_letters == []
  let _ =
    lapse(double, 0, [
      "LeaseLapses", keeps, "DeadLetterNackedVersionQueueDeferred", requeue,
    ])
  let record = harness.record(double, "a", "a1")
  // Queued at the arrival position the publish gave it.
  assert #(record.status, record.position) == #(RecordQueued, arrival)
  // Dead-lettered at the lapse.
  assert harness.inspect(double).dead_letters
    == [
      DeadLetter(
        pocket: "a",
        key: "a1",
        rank: 1,
        payload: <<"a1":utf8>>,
        nacked_by: "c1",
        nacked_at: harness.t0,
        expires_at: harness.t0
          + ttl
          + harness.fixture_settings().dead_letter_ttl,
      ),
    ]
  let batch = claim(double, "c2")
  assert batch.pocket_key == "a"
  assert harness.keys(batch.records) == ["a2", "a1"]
  let assert [_, second] = batch.records
  assert second == BatchRecord(key: "a1", rank: 2, payload: <<"newer":utf8>>)
  harness.fired(double)
}

/// The Rust test file's `lapsed_once`: `two_pockets`, then `a3` published
/// into `a`, and `a` claimed at t0 and lapsed at t0 + 10. The clock is left
/// at t0 + 10.
fn lapsed_once() -> Double {
  let double = two_pockets()
  assert harness.publish(double, "a", "a3", 1, "a3") == fired("PublishNewKey")
  let _ = claim_and_lapse(double, "a", 0, keeps)
  double
}

/// The library half of a `wire` row. `impl-choice` (spec note G38, as
/// N25): the order of the `Freed` rules the lapse fires, smallest held
/// position first.
fn nack_resets_count() -> List(String) {
  let double = lapsed_once()
  let count = fn() { harness.pocket(double, "a").consecutive_expiries }
  assert count() == 1
  let lease = claim(double, "c1").lease
  // After the pull.
  assert count() == 1
  assert harness.renew(double, "c1", lease) == fired("Renew")
  // After the renew.
  assert count() == 1
  let again = harness.batch(double, "c1", 10)
  // A pull within the lease.
  assert again.fired == ["PullWithinLease"]
  assert again.lease == lease
  // After that pull.
  assert count() == 1
  assert harness.nack(double, "c1", lease, "a1") == fired("NackMarksRecord")
  // After the nack.
  assert count() == 0
  let _ =
    lapse(double, ttl, [
      "LeaseLapses", keeps, "DeadLetterNackedRecord", requeue, requeue,
    ])
  // The lapse counts from the reset.
  assert count() == 1
  harness.fired(double)
}

/// The library half of a `wire` row. The Rust's mid-lease publish answers
/// `Superseded`; here it defers behind the held version and answers nothing
/// (N32, N38). `impl-choice` (spec note G38, as N25): the order of the
/// `Freed` rules each of the first two lapses fires, smallest held position
/// first.
fn ack_resets_count() -> List(String) {
  let double = lapsed_once()
  let lease = claim(double, "c1").lease
  assert count(double, "a") == #(PocketLeased, 1)
  harness.refused(double, RecordNotHeld("Ack", lease, "b1"), fn() {
    harness.ack(double, "c1", lease, "b1")
  })
  // A refused ack.
  assert count(double, "a") == #(PocketLeased, 1)
  assert harness.ack(double, "c1", lease, "a1") == fired("AckMarksRecord")
  // An ack.
  assert count(double, "a") == #(PocketLeased, 0)
  let _ =
    lapse(double, ttl, [
      "LeaseLapses", keeps, "ConsumeAckedRecord", requeue, requeue,
    ])
  assert count(double, "a") == #(PocketQueued, 1)
  // Kept at the front.
  assert harness.order(double) == ["a", "b"]
  // An ack of a record with a newer version deferred behind it resets the
  // count too.
  let lease = claim(double, "c1").lease
  assert harness.publish(double, "a", "a2", 2, "newer")
    == fired("DeferBehindHeldVersion")
  assert harness.ack(double, "c1", lease, "a2") == fired("AckMarksRecord")
  // An ack with a version deferred.
  assert count(double, "a") == #(PocketLeased, 0)
  // From there it parks on the third lapse, not the second.
  let _ =
    lapse(double, 2 * ttl, [
      "LeaseLapses", keeps, "ConsumeAckedVersionQueueDeferred", requeue,
    ])
  assert count(double, "a") == #(PocketQueued, 1)
  let _ = claim_and_lapse(double, "a", 3 * ttl, keeps)
  assert count(double, "a") == #(PocketQueued, 2)
  let _ = claim_and_lapse(double, "a", 4 * ttl, parks)
  assert count(double, "a") == #(PocketParked, 3)
  check(double)
  harness.fired(double)
}

/// The Rust's release answers the pocket's fate, `Queued`; here the release
/// answers nothing, so the port asserts the rules fired and the pocket's
/// status from the snapshot (N24).
fn release_resets_count() -> List(String) {
  let double = lapsed_once()
  let _ = claim_and_lapse(double, "a", ttl, keeps)
  assert harness.pocket(double, "a").consecutive_expiries == 2
  let lease = claim(double, "c1").lease
  assert harness.release(double, "c1", lease)
    == fired_all(["ReleaseRequeuesPocket", requeue, requeue, requeue])
  assert harness.pocket(double, "a").status == PocketQueued
  assert harness.pocket(double, "a").consecutive_expiries == 0
  // `a` went to the back on release: `b` is claimed first, and released.
  let batch = claim(double, "c1")
  assert batch.pocket_key == "b"
  assert harness.release(double, "c1", batch.lease)
    == fired_all(["ReleaseRequeuesPocket", requeue])
  let _ = claim_and_lapse(double, "a", 2 * ttl, keeps)
  assert count(double, "a") == #(PocketQueued, 1)
  let _ = claim_and_lapse(double, "a", 3 * ttl, keeps)
  assert count(double, "a") == #(PocketQueued, 2)
  let _ = claim_and_lapse(double, "a", 4 * ttl, parks)
  assert harness.pocket(double, "a").status == PocketParked
  harness.fired(double)
}

/// The pocket's status, its count and how many records it has.
fn fate(double: Double) -> #(PocketStatus, Int, Int) {
  let #(status, expiries) = count(double, "a")
  let records = model.records_of(harness.inspect(double).records, "a")
  #(status, expiries, list.length(records))
}

/// Two owners in the state the Rust forks: `two_pockets`, `a` lapsed twice,
/// at t0 and t0 + 10, and claimed again at t0 + 20 by `c1`.
fn lapsed_twice_and_claimed() -> Double {
  let double = two_pockets()
  let _ = claim_and_lapse(double, "a", 0, keeps)
  let _ = claim_and_lapse(double, "a", ttl, keeps)
  let batch = claim(double, "c1")
  assert batch.pocket_key == "a"
  double
}

fn threshold_parks_empty() -> List(String) {
  // At a limit of 1 every lapse is at the threshold, so even a pocket whose
  // one record was acked parks.
  let once =
    harness.fixture_with(
      Settings(
        ..harness.fixture_settings(),
        lease_ttl: ttl,
        max_consecutive_expiries: 1,
      ),
    )
  assert harness.publish(once, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  let lease = claim(once, "c1").lease
  assert harness.ack(once, "c1", lease, "a1") == fired("AckMarksRecord")
  let _ = lapse(once, 0, ["LeaseLapses", parks, "ConsumeAckedRecord"])
  // Limit 1, acked.
  assert fate(once) == #(PocketParked, 1, 0)
  // At the default 3: two lapses, then a third lease whose records are all
  // nacked or all acked. Either resets the count, so the lapse idles the
  // pocket.
  let #(nacked, acked) = forked(lapsed_twice_and_claimed)
  let assert [active] = model.active(harness.inspect(nacked).leases)
  let lease = active.id
  list.each(["a1", "a2"], fn(key) {
    assert harness.nack(nacked, "c1", lease, key) == fired("NackMarksRecord")
    assert harness.ack(acked, "c1", lease, key) == fired("AckMarksRecord")
  })
  let _ =
    lapse(nacked, 2 * ttl, [
      "LeaseLapses", idles, "DeadLetterNackedRecord", "DeadLetterNackedRecord",
    ])
  // Nacked.
  assert fate(nacked) == #(PocketIdle, 1, 0)
  assert list.length(harness.inspect(nacked).dead_letters) == 2
  // The Rust's forks share one clock, so its acked fork lapses a lease
  // after the nacked one, at t0 + 40, past its `expires_at` at t0 + 30.
  let _ =
    lapse(acked, 3 * ttl, [
      "LeaseLapses", idles, "ConsumeAckedRecord", "ConsumeAckedRecord",
    ])
  // Acked.
  assert fate(acked) == #(PocketIdle, 1, 0)
  list.flatten([harness.fired(once), both(nacked, acked)])
}

fn fenced_after_lapse() -> List(String) {
  let double = two_pockets()
  let old = claim(double, "c1").lease
  let _ = lapse(double, 0, ["LeaseLapses", keeps, requeue, requeue])
  assert harness.publish(double, "a", "a1", 2, "newer") == fired("Supersede")
  let new = claim(double, "c2")
  assert new.pocket_key == "a"
  assert new.lease != old
  harness.refused(double, LeaseFenced("Ack", "c1", old), fn() {
    harness.ack(double, "c1", old, "a1")
  })
  harness.refused(double, LeaseFenced("Renew", "c1", old), fn() {
    harness.renew(double, "c1", old)
  })
  harness.refused(double, LeaseFenced("Release", "c1", old), fn() {
    harness.release(double, "c1", old)
  })
  harness.fired(double)
}

/// The Rust's `invariants::one_strict_order` without its check of the
/// stored pocket order, which the double does not store (the ruling on
/// N38): the pocket positions distinct and below `next_position`, the record
/// positions distinct within each pocket and over all pockets and below
/// `next_position`, and an empty `check`.
fn strict_order(snapshot: Snapshot) -> Nil {
  let below = fn(position) { position < snapshot.next_position }
  let pockets = list.map(snapshot.pockets, fn(pocket) { pocket.position })
  assert distinct(pockets)
  assert list.all(pockets, below)
  list.each(snapshot.pockets, fn(pocket) {
    let records =
      model.records_of(snapshot.records, pocket.key)
      |> list.map(fn(record) { record.position })
    assert distinct(records)
  })
  let records = list.map(snapshot.records, fn(record) { record.position })
  assert distinct(records)
  assert list.all(records, below)
  assert invariants.check(snapshot) == []
}

fn distinct(positions: List(Int)) -> Bool {
  list.length(list.unique(positions)) == list.length(positions)
}

/// The Rust's comment says that `a` lapses three times and parks. Traced
/// through the rules, `b` keeps the front after its first lapse, so the loop
/// lapses `b` twice more, which parks it, and then `a` once. The last two
/// publishes are a new key into queued `a` and a supersede that revives
/// `b`. The Rust asserts no fate; the port asserts the rules each step
/// fired, as they trace. `impl-choice` (spec note G38, as N25): the order of
/// the two `Freed` rules the release fires, smallest held position first.
fn one_strict_order() -> List(String) {
  let double = two_pockets()
  let step = fn() { strict_order(harness.inspect(double)) }
  let first = harness.batch(double, "c1", 1)
  assert first.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let lease = first.lease
  step()
  assert harness.ack(double, "c1", lease, "a1") == fired("AckMarksRecord")
  // The acked a1 is still held at rank 1.
  harness.refused(double, EqualRankRefused("a", "a1", 1), fn() {
    harness.publish(double, "a", "a1", 1, "again")
  })
  assert harness.publish(double, "a", "a1", 2, "again")
    == fired("DeferBehindHeldVersion")
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullWithinLease", "HoldRecord"]
  step()
  assert harness.publish(double, "a", "a2", 2, "newer")
    == fired("DeferBehindHeldVersion")
  // a1 waits for the lease's end.
  assert harness.keys(batch.records) == ["a2"]
  assert harness.nack(double, "c1", lease, "a2") == fired("NackMarksRecord")
  step()
  assert harness.release(double, "c1", lease)
    == fired_all([
      "ReleaseRequeuesPocket", "ConsumeAckedVersionQueueDeferred",
      "DeadLetterNackedVersionQueueDeferred",
    ])
  step()
  // `b` is now the front. It lapses, keeps the front, and lapses twice
  // more, which parks it; then `a` lapses once.
  let _ = claim_and_lapse(double, "b", 0, keeps)
  step()
  list.each([#(1, keeps), #(2, parks), #(3, keeps)], fn(pair) {
    let #(n, rule) = pair
    let assert [front, ..] = harness.order(double)
    let _ = claim_and_lapse(double, front, n * ttl, rule)
    step()
  })
  assert harness.publish(double, "a", "a9", 1, "revive")
    == fired("PublishNewKey")
  assert harness.publish(double, "b", "b1", 2, "revive")
    == fired_all(["Supersede", "ReviveParkedPocket"])
  step()
  check(double)
  harness.fired(double)
}
