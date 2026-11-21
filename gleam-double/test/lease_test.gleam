//// The specification's "Renew and release" section: `Renew`, and the two
//// release rules, with the revival of the idle pocket a release leaves and
//// the fencing every action under a lease shares.
////
//// First ten catalog rows of `crates/libpachinko/tests/queue/lease.rs`,
//// S023, S035, S050, S051, S052, S053, S055, S057, S068 and S070, each a
//// port of its Rust test, driving the owner. A port asserts what its Rust
//// test asserts, with these changes.
////
//// - The Rust's renew answers the lease's new `expires_at`, its ack and nack
////   a `SettleOutcome`, its release the pocket's fate and the ended lease,
////   and its publish a `PublishOutcome`. The specification's answer nothing,
////   so a port asserts that the reply is `Ok(Outcome(fired, None))`, with the
////   names of the rules it fired, and reads the state from the snapshot (N13
////   and the rulings on N24 and N32 in `double/SPEC-NOTES.md`).
//// - An ended lease stays in the state (the ruling on N24). Where the Rust
////   reads the lease its release returned, a port reads that lease from the
////   snapshot; where S053's Rust asserts an empty lease registry, its port
////   asserts that the lease is there, released, and that none is active.
//// - The Rust's refusals are `LeaseNotActive` and `LeaseNotYours`. The
////   specification names none. The double's is `LeaseFenced` for both, the
////   lease checked before the record (N21). The refusal's name and shape are
////   `impl-choice` (spec note G28).
//// - Every refusal goes through `harness.refused`, which compares the
////   complete snapshot, the clock's read count and the lease-id draws before
////   and after.
////
//// S053 and S068 are `wire` rows: only the library half is ported.
////
//// Then the double's own tests, which pin no catalog row. For `Renew`: what
//// it changes, the refusal of a consumer that never joined, the fencing of
//// a renew that names another consumer's lease or no lease, and the rule's
//// guard and effect over states built by hand. For the release: what each
//// of the two rules changes with the `Freed` rules it chains to, over
//// states built by hand; the two rules' guards; and the refusals of a
//// release, of an action that names a lease that has ended, and of a
//// release by a consumer that never joined.

import gleam/list
import gleam/option.{type Option, None, Some}
import harness.{type Double, Driven}
import pachinko_double/config.{Settings}
import pachinko_double/core
import pachinko_double/domain.{NoPocketAvailable}
import pachinko_double/inspection.{Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type LeaseStatus, type Mark, type PocketStatus, type Record, type State, Acked,
  Consumer, DeadLetter, Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked,
  Pocket, PocketIdle, PocketLeased, PocketParked, PocketQueued, Publisher, Queue,
  Record, RecordHeld, RecordQueued, Stale, State, SupersededRecord, Unsettled,
}
import pachinko_double/owner.{type Reply, Outcome}
import pachinko_double/refusals.{LeaseFenced, ReleaseRefused, UnknownConsumer}
import pachinko_double/rules.{
  Applied, Defect, Done, EffectFailed, Refused, Refuses, Row, Table,
}
import pachinko_double/settle.{type Release, type Renew, Release, Renew}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a renew extends the lease to now plus lease_ttl",
      [Scenario("S050")],
      renew_extends,
    ),
    Test(
      "a publish revives an idle pocket at the back",
      [Scenario("S023")],
      revives_idle,
    ),
    Test(
      "a consumer with a lease never gets NoPocketAvailable or another pocket",
      [Scenario("S035")],
      keeps_its_pocket,
    ),
    Test("renewals do not stack", [Scenario("S051")], renewals_do_not_stack),
    Test(
      "a renew of an ended lease is rejected and moves nothing",
      [Scenario("S052")],
      renew_of_ended_lease,
    ),
    Test(
      "a release requeues the pocket at the back and its unsettled records in place, and consumes the acked ones",
      [Scenario("S053")],
      release_requeues,
    ),
    Test("a release idles an empty pocket", [Scenario("S055")], release_idles),
    Test(
      "a second release is rejected; a release at the grant instant is legal",
      [Scenario("S057")],
      second_release,
    ),
    Test(
      "a consumer that re-claims the same pocket cannot use its old lease",
      [Scenario("S068")],
      old_lease_fenced,
    ),
    Test(
      "a consumer cannot act on another consumer's lease",
      [Scenario("S070")],
      another_consumers_lease,
    ),
    Test(
      "Renew changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      renew_exactly,
    ),
    Test(
      "a renew by a consumer that never joined is refused",
      [],
      unknown_consumer,
    ),
    Test(
      "a renew that names another consumer's lease or no lease is fenced",
      [],
      renew_fenced,
    ),
    Test(
      "Renew's guard holds only for an active lease of the consumer's",
      [],
      renew_guard,
    ),
    Test(
      "Renew leaves a retained version and a dead letter as they were",
      [],
      renew_keeps_shed_queues,
    ),
    Test(
      "ReleaseRequeuesPocket changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      requeues_exactly,
    ),
    Test(
      "a release frees the held records smallest position first, each by its mark, and stamps every entry from the lease's end",
      [],
      requeues_by_mark,
    ),
    Test(
      "ReleaseIdlesEmptyPocket changes exactly what it ensures, reads the clock once and draws no lease id",
      [],
      idles_exactly,
    ),
    Test(
      "each release rule's guard holds only for the consumer's active lease on a leased pocket, by whether a record outlasts it",
      [],
      release_guards,
    ),
    Test(
      "a release that names another consumer's lease or no lease is fenced",
      [],
      release_fenced,
    ),
    Test(
      "an ack, a nack and a release that name a lease that has ended are fenced",
      [],
      ended_lease_fenced,
    ),
    Test(
      "a release by a consumer that never joined is refused",
      [],
      release_by_unknown_consumer,
    ),
    Test(
      "no Release rule holds for an active lease on a pocket that is not leased",
      [],
      release_on_unleased_pocket,
    ),
    Test(
      "a release rule's effect without its lease on a leased pocket is a defect, and asks for no time",
      [],
      release_without_lease,
    ),
  ]
}

/// What a trigger that fired this one rule and answered nothing replies.
fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

/// What a trigger that fired these rules, in this order, and answered
/// nothing replies: a release, with the `Freed` rules it chained to, or a
/// publish that revived its pocket.
fn fired_all(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}

const claims = "PullClaimsFrontPocket"

const within = "PullWithinLease"

const hold = "HoldRecord"

const ack_marks_record = "AckMarksRecord"

const requeues = "ReleaseRequeuesPocket"

const idles = "ReleaseIdlesEmptyPocket"

const requeue = "RequeueUnsettledRecord"

const consume = "ConsumeAckedRecord"

/// The Rust test file's `two_pockets`: pocket `a` with `a1` and `a2`, then
/// pocket `b` with `b1`.
fn two_pockets() -> Double {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "a2", 1, "a2") == fired("PublishNewKey")
  assert harness.publish(double, "b", "b1", 1, "b1")
    == fired("PublishIntoNewPocket")
  double
}

/// The Rust test file's `short_lease`: a fixture with a 10 ms lease and one
/// held record under `c1`'s lease, pulled at `t0`, and that lease's id.
fn short_lease() -> #(Double, String) {
  let double =
    harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: 10))
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  #(double, batch.lease)
}

// The catalog rows -----------------------------------------

fn renew_extends() -> List(String) {
  let #(double, lease) = short_lease()
  harness.set_time(double.clock, harness.t0 + 6)
  assert harness.renew(double, "c1", lease) == fired("Renew")
  let snapshot = harness.inspect(double)
  let assert [active] = snapshot.leases
  assert active.expires_at == harness.t0 + 16
  // "A renew is not a new grant."
  assert active.granted_at == harness.t0
  // The Rust's `invariants::granted_before_expiry`, by the checker.
  assert !list.contains(invariants.check(snapshot), "GrantedBeforeExpiry")
  harness.fired(double)
}

/// The Rust's last publish answers `NewKey`; the specification's answers
/// nothing, and the reply names the two rules it fired (the ruling on N32).
/// The positions are read against the counter, so none is absolute: the
/// revival answers the `PocketTouched` that `PublishNewKey` emitted, so it
/// reads the counter `PublishNewKey` left, and the pocket stands one above
/// its new record.
fn revives_idle() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  assert harness.ack(double, "c1", batch.lease, "a1") == fired(ack_marks_record)
  assert harness.ack(double, "c1", batch.lease, "a2") == fired(ack_marks_record)
  assert harness.release(double, "c1", batch.lease)
    == fired_all([idles, consume, consume])
  assert harness.pocket(double, "a").status == PocketIdle
  assert harness.publish(double, "c", "c1", 1, "c")
    == fired("PublishIntoNewPocket")
  let next = harness.inspect(double).next_position
  assert harness.publish(double, "a", "a3", 1, "a3")
    == fired_all(["PublishNewKey", "ReviveIdlePocket"])
  let pocket = harness.pocket(double, "a")
  assert pocket.status == PocketQueued
  // "PublishNewKey takes its position first."
  assert harness.record(double, "a", "a3").position == next
  // "Then the revive: the back."
  assert pocket.position == next + 1
  assert pocket.consecutive_expiries == 0
  assert harness.order(double) == ["b", "c", "a"]
  assert harness.publish(double, "a", "a4", 1, "a4") == fired("PublishNewKey")
  // "A queued pocket stays put."
  assert harness.pocket(double, "a").position == next + 1
  // The Rust's `invariants::all`, by the checker, which the apply step has
  // also run after each outcome.
  assert invariants.check(harness.inspect(double)) == []
  harness.fired(double)
}

/// `harness.batch` fails the test on any reply but a batch, so each pull
/// by `c1` below shows it was not told `NoPocketAvailable`. Lease ids are
/// compared by value: the lease `c1` is granted over `b` at the end has
/// another id.
fn keeps_its_pocket() -> List(String) {
  let double = two_pockets()
  let first = harness.batch(double, "c1", 1)
  assert first.fired == [claims, hold]
  let lease = first.lease
  let again = harness.batch(double, "c1", 1)
  assert #(again.fired, again.lease, again.pocket_key)
    == #([within, hold], lease, "a")
  let theirs = harness.batch(double, "c2", 10)
  assert theirs.fired == [claims, hold]
  // "No pocket is queued."
  assert harness.order(double) == []
  let empty = harness.batch(double, "c1", 10)
  assert #(empty.fired, empty.lease, empty.pocket_key)
    == #([within], lease, "a")
  assert empty.records == []
  // The Rust unwraps each release's answer; the specification's release
  // answers nothing.
  assert harness.release(double, "c2", theirs.lease)
    == fired_all([requeues, requeue])
  let still = harness.batch(double, "c1", 10)
  assert #(still.fired, still.lease, still.pocket_key)
    == #([within], lease, "a")
  assert harness.release(double, "c1", lease)
    == fired_all([requeues, requeue, requeue])
  let other = harness.batch(double, "c1", 10)
  assert other.fired == [claims, hold]
  assert other.lease != lease
  assert other.pocket_key == "b"
  harness.fired(double)
}

/// The Rust compares the two expiries its renews answer; the specification's
/// renew answers nothing, so the port reads the lease's `expires_at` from
/// the snapshot after each (N13).
fn renewals_do_not_stack() -> List(String) {
  let #(double, lease) = short_lease()
  let acked = harness.advance(double.clock, harness.t0, 4)
  assert harness.ack(double, "c1", lease, "k") == fired(ack_marks_record)
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.expires_at == harness.t0 + 14
  let _ = harness.advance(double.clock, acked, 2)
  assert harness.renew(double, "c1", lease) == fired("Renew")
  let assert Ok(first) = model.lease(harness.inspect(double).leases, lease)
  assert harness.renew(double, "c1", lease) == fired("Renew")
  let assert Ok(second) = model.lease(harness.inspect(double).leases, lease)
  assert #(first.expires_at, second.expires_at)
    == #(harness.t0 + 16, harness.t0 + 16)
  assert list.length(harness.inspect(double).leases) == 1
  harness.fired(double)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape. The
/// Rust's is `LeaseNotActive`. The released lease stays in the state (the
/// ruling on N24), and a renew that names it is fenced, before `c1` claims
/// the pocket again and after: the second refusal is of the old lease's id
/// while `c1` holds the same pocket under a new one.
fn renew_of_ended_lease() -> List(String) {
  let #(double, lease) = short_lease()
  assert harness.release(double, "c1", lease) == fired_all([requeues, requeue])
  let _ = harness.advance(double.clock, harness.t0, 3)
  let ended = LeaseFenced("Renew", "c1", lease)
  harness.refused(double, ended, fn() { harness.renew(double, "c1", lease) })
  let again = harness.batch(double, "c1", 10)
  assert again.fired == [claims, hold]
  harness.refused(double, ended, fn() { harness.renew(double, "c1", lease) })
  harness.fired(double)
}

/// The Rust's release answers the pocket's fate and the ended lease; the
/// specification's answers nothing, so the port reads both from the
/// snapshot.
///
/// The ruling on N24: the Rust asserts that the snapshot's lease list is
/// empty, "an ended lease leaves the registry", and that no consumer has a
/// lease. That is its inspection shape. The port asserts the specification's
/// facts instead: the lease is still in the state, released, with its
/// `ended_at`; no lease is active; and no consumer has a lease view.
///
/// `impl-choice` (spec note G38, as N25): the order of the two `Freed` rules
/// in the release's reply, `a1` at the smaller position first. Here position
/// order, key order and the order of creation agree.
fn release_requeues() -> List(String) {
  let double = two_pockets()
  let a2 = harness.record(double, "a", "a2").position
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  let lease = batch.lease
  assert harness.ack(double, "c1", lease, "a1") == fired(ack_marks_record)
  // "An acked record stays until the lease ends."
  assert harness.record(double, "a", "a1").status
    == RecordHeld(
      held_rank: 1,
      held_payload: <<"a1":utf8>>,
      mark: Acked,
      nacked_at: None,
      deferred_position: None,
    )
  let ended_at = harness.advance(double.clock, harness.t0, 7)
  let next = harness.inspect(double).next_position
  assert harness.release(double, "c1", lease)
    == fired_all([requeues, consume, requeue])
  let snapshot = harness.inspect(double)
  let assert [ended] = snapshot.leases
  assert ended.status == LeaseReleased(ended_at)
  assert #(ended.id, ended.consumer) == #(lease, "c1")
  // The Rust's `invariants::ended_after_grant`, by the checker.
  assert !list.contains(invariants.check(snapshot), "EndedAfterGrant")
  let pocket = harness.pocket(double, "a")
  assert #(pocket.status, pocket.position, pocket.consecutive_expiries)
    == #(PocketQueued, next, 0)
  let record = harness.record(double, "a", "a2")
  assert #(record.status, record.position) == #(RecordQueued, a2)
  // "The acked a1 is consumed."
  assert list.length(model.records_of(snapshot.records, "a")) == 1
  assert harness.order(double) == ["b", "a"]
  assert model.active(snapshot.leases) == []
  list.each(["c1", "c2", "c3"], fn(consumer) {
    assert harness.lease_view(double, consumer) == Ok(None)
  })
  assert invariants.check(snapshot) == []
  assert harness.batch(double, "c1", 10).pocket_key == "b"
  harness.fired(double)
}

/// The Rust's release answers the fate `Idle` and the ended lease; the port
/// reads the pocket and the lease from the snapshot. The idle pocket keeps
/// the position it had (N27).
fn release_idles() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  let lease = batch.lease
  list.each(["a1", "a2"], fn(key) {
    assert harness.ack(double, "c1", lease, key) == fired(ack_marks_record)
  })
  let position = harness.pocket(double, "a").position
  assert harness.release(double, "c1", lease)
    == fired_all([idles, consume, consume])
  let snapshot = harness.inspect(double)
  let assert Ok(ended) = model.lease(snapshot.leases, lease)
  assert ended.status == LeaseReleased(harness.t0)
  let pocket = harness.pocket(double, "a")
  assert #(pocket.status, pocket.consecutive_expiries, pocket.position)
    == #(PocketIdle, 0, position)
  // The Rust's `invariants::idle_has_no_records` and `invariants::all`, by
  // the checker.
  assert !list.contains(invariants.check(snapshot), "IdleHasNoRecords")
  assert invariants.check(snapshot) == []
  assert harness.order(double) == ["b"]
  assert harness.batch(double, "c1", 10).pocket_key == "b"
  assert harness.pull(double, "c2", 10)
    == Ok(Outcome(["PullFindsNoPocket"], Some(NoPocketAvailable("c2"))))
  harness.fired(double)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape. The
/// Rust's is `LeaseNotActive`. The release is at `t0`, the instant of the
/// grant, and is legal: the lease read back from the snapshot is released
/// with `ended_at` equal to its `granted_at`. A second release of it is
/// fenced.
fn second_release() -> List(String) {
  let #(double, lease) = short_lease()
  assert harness.release(double, "c1", lease) == fired_all([requeues, requeue])
  let snapshot = harness.inspect(double)
  let assert Ok(ended) = model.lease(snapshot.leases, lease)
  assert ended.granted_at == harness.t0
  assert ended.status == LeaseReleased(harness.t0)
  // The Rust's `invariants::ended_after_grant`, by the checker.
  assert !list.contains(invariants.check(snapshot), "EndedAfterGrant")
  harness.refused(double, LeaseFenced("Release", "c1", lease), fn() {
    harness.release(double, "c1", lease)
  })
  harness.fired(double)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape. The
/// Rust's is `LeaseNotActive`. `c1` holds the same pocket again under a new
/// lease, and the same key at a newer rank; lease ids are compared by value,
/// and the new lease's is another. A renew, an ack, a nack and a release
/// that name the old lease are each fenced and change nothing, the new
/// lease's expiry included, and the new lease still acks the key.
fn old_lease_fenced() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired("PublishIntoNewPocket")
  let first = harness.batch(double, "c1", 10)
  assert first.fired == [claims, hold]
  let old = first.lease
  assert harness.release(double, "c1", old) == fired_all([requeues, requeue])
  assert harness.publish(double, "pk", "k", 2, "v2") == fired("Supersede")
  let new = harness.batch(double, "c1", 10)
  assert new.fired == [claims, hold]
  assert new.lease != old
  assert new.pocket_key == "pk"
  let assert [handed, ..] = new.records
  assert handed.rank == 2
  let _ = harness.advance(double.clock, harness.t0, 5)
  let refused = fn(action: String, act: fn() -> Reply) {
    harness.refused(double, LeaseFenced(action, "c1", old), act)
  }
  refused("Renew", fn() { harness.renew(double, "c1", old) })
  refused("Ack", fn() { harness.ack(double, "c1", old, "k") })
  refused("Nack", fn() { harness.nack(double, "c1", old, "k") })
  refused("Release", fn() { harness.release(double, "c1", old) })
  // The Rust's ack answers `Acked`; the specification's answers nothing.
  assert harness.ack(double, "c1", new.lease, "k") == fired(ack_marks_record)
  harness.fired(double)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape. The
/// Rust tells `LeaseNotYours`, while `c1`'s lease is active, from
/// `LeaseNotActive`, once `c1` has released it; both are `LeaseFenced` here,
/// since the specification draws no line between the two `requires`.
///
/// `impl-choice` (spec note G34, as N21): the order. The ack by `c3` names a
/// key the lease's pocket does not have, and is fenced, not "not held": the
/// lease is checked first.
fn another_consumers_lease() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == [claims, hold, hold]
  let mine = batch.lease
  assert harness.batch(double, "c2", 10).fired == [claims, hold]
  let refused = fn(consumer: String, action: String, act: fn() -> Reply) {
    harness.refused(double, LeaseFenced(action, consumer, mine), act)
  }
  let by_c2 = fn() {
    refused("c2", "Ack", fn() { harness.ack(double, "c2", mine, "a1") })
    refused("c2", "Nack", fn() { harness.nack(double, "c2", mine, "a1") })
    refused("c2", "Renew", fn() { harness.renew(double, "c2", mine) })
    refused("c2", "Release", fn() { harness.release(double, "c2", mine) })
  }
  by_c2()
  refused("c3", "Ack", fn() { harness.ack(double, "c3", mine, "absent") })
  assert harness.release(double, "c1", mine)
    == fired_all([requeues, requeue, requeue])
  by_c2()
  harness.fired(double)
}

// What the rule changes --------------------------------------

/// `c1` holds `pk` and `c2` holds `other`, and the superseded queue holds an
/// earlier version of `pk`'s record. Each renew of `c1`'s lease sets its
/// `expires_at` to that renew's `now` plus `lease_ttl`, so renewals do not
/// add up, and changes nothing else in the complete snapshot: not its
/// `granted_at`, not `c2`'s lease, no pocket, no record and not the retained
/// version. Each reads the clock once, and none draws a lease id.
fn renew_exactly() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "other", "o", 1, "o")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "k", 2, "v2") == fired("Supersede")
  let theirs = harness.batch(double, "c2", 1)
  assert theirs.pocket_key == "pk"
  let mine = harness.batch(double, "c1", 1)
  assert mine.pocket_key == "other"
  let before = harness.inspect(double)
  assert list.length(before.superseded) == 1
  let assert [other_lease, lease] = before.leases
  assert lease.id == mine.lease
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  let ttl = harness.fixture_settings().lease_ttl
  let renewed = fn(now) {
    Snapshot(..before, leases: [
      other_lease,
      Lease(..lease, expires_at: now + ttl),
    ])
  }

  let first = harness.advance(double.clock, harness.t0, 300)
  assert harness.renew(double, "c1", mine.lease) == fired("Renew")
  assert harness.inspect(double) == renewed(first)
  assert harness.clock_reads(double.clock) == reads + 1

  assert harness.renew(double, "c1", mine.lease) == fired("Renew")
  assert harness.inspect(double) == renewed(first)
  assert harness.clock_reads(double.clock) == reads + 2

  let second = harness.advance(double.clock, first, 450)
  assert harness.renew(double, "c1", mine.lease) == fired("Renew")
  assert harness.inspect(double) == renewed(second)
  assert harness.clock_reads(double.clock) == reads + 3
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}

// The refusal of a consumer that never joined ----------------

/// `impl-choice` (spec note G26, as N18): the specification states no
/// such refusal. `cx` never joined, and `p` joined as a publisher only.
/// Each renew names `c1`'s active lease or a lease never issued, and each
/// is refused: the complete snapshot is unchanged, no clock was read and no
/// lease id was drawn.
fn unknown_consumer() -> List(String) {
  let #(double, lease) = short_lease()
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  harness.set_time(double.clock, harness.t0 + 3)
  let refused = fn(consumer) { Error(Refused(UnknownConsumer(consumer))) }
  assert harness.renew(double, "cx", lease) == refused("cx")
  assert harness.renew(double, "cx", "never-issued") == refused("cx")
  assert harness.renew(double, "p", lease) == refused("p")
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}

// The fencing of a renew -------------------------------------

/// `impl-choice` (spec note G28, as N20 and N21): the refusal's name and
/// shape. `pk` holds `k` and `other` holds `x`; `c1` has pulled `pk` and
/// `c2` `other`. A renew by `c1`, which has joined, that names `c2`'s active
/// lease, or an id no lease has, is fenced. The complete snapshot is
/// unchanged, `c2`'s lease's expiry included, no clock was read and no lease
/// id was drawn.
fn renew_fenced() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "other", "x", 1, "x")
    == fired("PublishIntoNewPocket")
  assert harness.batch(double, "c1", 1).pocket_key == "pk"
  let theirs = harness.batch(double, "c2", 1)
  assert theirs.pocket_key == "other"
  let _ = harness.at(double, 300)
  harness.refused(double, LeaseFenced("Renew", "c1", theirs.lease), fn() {
    harness.renew(double, "c1", theirs.lease)
  })
  harness.refused(double, LeaseFenced("Renew", "c1", "lease-99"), fn() {
    harness.renew(double, "c1", "lease-99")
  })
  harness.fired(double)
}

// The guard, over states built by hand ------------------------

/// Consumers `c1` and `c2` joined, and one lease, `lease-1`, of `c1` over
/// pocket `pk`, in this status. The pocket is leased while the lease is
/// active and queued once it has ended. The superseded queue holds a stale
/// version of the pocket's record, and the dead-letter queue a record `c2`
/// once nacked out of it.
fn with_lease(status: LeaseStatus) -> State {
  let pocket_status = case status {
    LeaseActive -> PocketLeased
    LeaseReleased(_) | LeaseExpired(_) -> PocketQueued
  }
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 2),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1"), Consumer("c2")],
    pockets: [Pocket("pk", pocket_status, 0, 0)],
    records: [Record("pk", "k", 1, <<"v":utf8>>, 1, RecordQueued)],
    leases: [
      Lease(
        id: "lease-1",
        pocket: "pk",
        consumer: "c1",
        status:,
        granted_at: 100,
        expires_at: 400,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "k",
        rank: 0,
        payload: <<"stale":utf8>>,
        reason: Stale,
        shed_at: 50,
        expires_at: 50 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "pk",
        key: "gone",
        rank: 3,
        payload: <<"dead":utf8>>,
        nacked_by: "c2",
        nacked_at: 60,
        expires_at: 60 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// The rule the Renew table sends this renew to over this state, if any.
fn rule(state: State, renew: Renew) -> option.Option(String) {
  let assert Ok(row) = rules.dispatch(settle.renew(), state, renew)
  option.map(row, fn(row) { row.name })
}

/// `requires: lease.status = active` and `requires: lease.consumer =
/// consumer`, each shown to matter on its own: the consumer's own lease once
/// it has been released or has expired, another consumer's active lease,
/// and a lease id that names no lease. Only what dispatch finds is asserted,
/// not what the table then answers.
fn renew_guard() -> List(String) {
  assert rule(with_lease(LeaseActive), Renew("c1", "lease-1")) == Some("Renew")
  assert rule(with_lease(LeaseReleased(300)), Renew("c1", "lease-1")) == None
  assert rule(with_lease(LeaseExpired(400)), Renew("c1", "lease-1")) == None
  assert rule(with_lease(LeaseActive), Renew("c2", "lease-1")) == None
  assert rule(with_lease(LeaseActive), Renew("c1", "lease-2")) == None
  assert rule(with_lease(LeaseActive), Renew("c1", "pk")) == None
  []
}

/// `Renew` through the apply step, over the state built by hand: the
/// lease's `expires_at` is this renew's `now` plus `lease_ttl`, and the
/// rest of the complete state is as it was, the retained version and the
/// dead letter included.
fn renew_keeps_shed_queues() -> List(String) {
  let state = with_lease(LeaseActive)
  let assert [lease] = state.leases
  let now = 250
  assert rules.drive(
      core.apply(state, core.Renew(Renew("c1", "lease-1"))),
      fn() { now },
      fn() { "unused" },
    )
    == Ok(Applied(
      State(..state, leases: [
        Lease(..lease, expires_at: now + config.defaults().lease_ttl),
      ]),
      ["Renew"],
      None,
    ))
  ["Renew"]
}

// The release, over states built by hand ----------------------
//
// Through the owner a pocket's `consecutive_expiries` is 0 until a lease can
// lapse, a list's order and the positions agree, and no two pockets hold one
// key. These tests run the apply step over states built by hand in which
// none of that holds, with the checker reading what each whole outcome
// leaves: the release and every `Freed` it sent.

/// A record held at its own rank and payload with this mark and this nack
/// time, and nothing deferred behind it.
fn held(record: Record, mark: Mark, nacked_at: Option(Int)) -> Record {
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

/// `other`'s two records, both held by `c2` under `lease-2`: `a`, acked, and
/// `n`, nacked at 520. `pk`'s records share both keys where a test says so.
fn theirs() -> #(Record, Record) {
  #(
    held(Record("other", "a", 48, <<"oa":utf8>>, 12, RecordQueued), Acked, None),
    held(
      Record("other", "n", 47, <<"on":utf8>>, 10, RecordQueued),
      Nacked,
      Some(520),
    ),
  )
}

/// The time `c1`'s lease, `lease-1`, was granted at.
const granted = 600

/// Consumers `c1` and `c2` joined.
///
/// - `c1` holds `pk` under `lease-1`, granted at 600. `pk` is in this
///   status, has seen `expiries` consecutive expiries, and holds `records`.
/// - `c1` once held `pk` under `lease-0`, since released.
/// - `c2` holds `other` under `lease-2`; `other`'s records are in `records`
///   too, wherever the test put them.
/// - `done` is idle, and has seen one expiry.
/// - The superseded queue holds a stale version of a `pk` record, and the
///   dead-letter queue a record `c1` nacked out of `pk` under `lease-0`.
///
/// The counter stands at 20. No pocket list is in position order.
fn releasing(
  status: PocketStatus,
  expiries: Int,
  records: List(Record),
) -> State {
  let ttl = config.defaults().lease_ttl
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 20),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1"), Consumer("c2")],
    pockets: [
      Pocket("other", PocketLeased, 9, 0),
      Pocket("pk", status, 2, expiries),
      Pocket("done", PocketIdle, 1, 1),
    ],
    records:,
    leases: [
      Lease(
        id: "lease-0",
        pocket: "pk",
        consumer: "c1",
        status: LeaseReleased(150),
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
        key: "a",
        rank: 40,
        payload: <<"stale":utf8>>,
        reason: Stale,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "pk",
        key: "k0",
        rank: 39,
        payload: <<"zero":utf8>>,
        nacked_by: "c1",
        nacked_at: 120,
        expires_at: 150 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// `c1` releases `lease-1` at this time, through the apply step, with the
/// clock reads and the lease-id draws the outcome asked for.
fn released(state: State, now: Int) -> harness.Driven {
  harness.drive(
    core.apply(state, core.Release(Release("c1", "lease-1"))),
    now,
    "unused",
  )
}

/// `pk` has seen two expiries and holds two records, both held and
/// unsettled, the one listed first at the larger position. `c1` releases at
/// 900. The lease is `released` with `ended_at` 900, and its `granted_at`
/// and `expires_at` stay. The pocket is queued at the counter's value, 20,
/// with its count 0, and the counter is 21. Each record is queued where it
/// stood, the smaller position freed first. Nothing else in the complete
/// state changes: not `other` or its records, not `lease-0` or `lease-2`,
/// not the idle pocket, and neither shed queue. The outcome reads the clock
/// once, though four rules fired.
fn requeues_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let late = Record("pk", "a", 44, <<"late":utf8>>, 6, RecordQueued)
  let early = Record("pk", "n", 45, <<"early":utf8>>, 3, RecordQueued)
  let state =
    releasing(PocketLeased, 2, [
      their_a,
      held(late, Unsettled, None),
      their_n,
      held(early, Unsettled, None),
    ])
  let assert [other, pk, done] = state.pockets
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 900
  let fired = [
    "ReleaseRequeuesPocket", "RequeueUnsettledRecord", "RequeueUnsettledRecord",
  ]
  assert released(state, now)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          queue: Queue(next_position: 21),
          pockets: [
            other,
            Pocket(
              ..pk,
              status: PocketQueued,
              position: 20,
              consecutive_expiries: 0,
            ),
            done,
          ],
          records: [their_a, late, their_n, early],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, status: LeaseReleased(now)),
          ],
        ),
        fired,
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  fired
}

/// `pk` holds five records, listed in no order of position: `n`, nacked at
/// 700, at position 8; `u`, unsettled, at 6; `a`, acked, at 3; `m`, nacked
/// at 650, at 5; and `q`, queued, at 11. `other` holds an `a` and an `n` of
/// its own under `c2`'s lease. `c1` releases at 900.
///
/// `impl-choice` (spec note G38, as N25): `Freed` goes to the held records
/// smallest position first, so the rules fire in the order `a`, `m`, `u`,
/// `n`, and the two dead letters are added in the order `m`, `n`, though `n`
/// is listed first.
///
/// The acked record is removed and recorded nowhere. Each nacked record is
/// removed and becomes a dead letter at its held rank and payload, nacked by
/// the lease's consumer, with the time of its own nack as `nacked_at` and
/// `expires_at` at the lease's end, 900, plus `dead_letter_ttl`. The
/// unsettled record is queued where it stood. `q` and `other`'s records are
/// as they were, and the records that remain keep their order. Since `u`
/// and `q` outlast the lease the pocket is queued at the counter's value.
fn requeues_by_mark() -> List(String) {
  let #(their_a, their_n) = theirs()
  let n = Record("pk", "n", 45, <<"en":utf8>>, 8, RecordQueued)
  let u = Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued)
  let a = Record("pk", "a", 46, <<"ay":utf8>>, 3, RecordQueued)
  let m = Record("pk", "m", 43, <<"em":utf8>>, 5, RecordQueued)
  let q = Record("pk", "q", 42, <<"queue":utf8>>, 11, RecordQueued)
  let state =
    releasing(PocketLeased, 0, [
      held(n, Nacked, Some(700)),
      their_n,
      held(u, Unsettled, None),
      held(a, Acked, None),
      their_a,
      held(m, Nacked, Some(650)),
      q,
    ])
  let assert [other, pk, done] = state.pockets
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = 900
  let letter = fn(record: Record, nacked_at) {
    DeadLetter(
      pocket: "pk",
      key: record.key,
      rank: record.rank,
      payload: record.payload,
      nacked_by: "c1",
      nacked_at:,
      expires_at: now + config.defaults().dead_letter_ttl,
    )
  }
  let fired = [
    "ReleaseRequeuesPocket", "ConsumeAckedRecord", "DeadLetterNackedRecord",
    "RequeueUnsettledRecord", "DeadLetterNackedRecord",
  ]
  assert released(state, now)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          queue: Queue(next_position: 21),
          pockets: [
            other,
            Pocket(..pk, status: PocketQueued, position: 20),
            done,
          ],
          records: [their_n, u, their_a, q],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, status: LeaseReleased(now)),
          ],
          dead_letters: list.append(state.dead_letters, [
            letter(m, 650),
            letter(n, 700),
          ]),
        ),
        fired,
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  fired
}

/// Twice. First `pk` holds no record at all and has seen two expiries, a
/// state no pull through the owner leaves, which shows the reset: the lease
/// is `released` with `ended_at` 900, and the pocket is idle with its count
/// 0. It keeps its position, 2, and the counter stays at 20: an idle pocket
/// is allotted nothing (N27). No `Freed` is sent.
///
/// Then `pk` holds an acked record and a nacked one, the nacked one listed
/// first at the larger position, and `other` holds both keys under `c2`'s
/// lease. Both of `pk`'s records are removed, the nacked one into a dead
/// letter, and the pocket is idle where it stood. Nothing else in either
/// complete state changes.
fn idles_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let now = 900
  let empty = releasing(PocketLeased, 2, [their_a, their_n])
  let assert [other, pk, done] = empty.pockets
  let assert [lease_0, lease_2, lease_1] = empty.leases
  let ended = [lease_0, lease_2, Lease(..lease_1, status: LeaseReleased(now))]
  assert released(empty, now)
    == Driven(
      Ok(Applied(
        State(
          ..empty,
          pockets: [
            other,
            Pocket(..pk, status: PocketIdle, consecutive_expiries: 0),
            done,
          ],
          leases: ended,
        ),
        ["ReleaseIdlesEmptyPocket"],
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )

  let n = Record("pk", "n", 45, <<"en":utf8>>, 8, RecordQueued)
  let a = Record("pk", "a", 46, <<"ay":utf8>>, 3, RecordQueued)
  let marked =
    releasing(PocketLeased, 0, [
      held(n, Nacked, Some(700)),
      their_a,
      held(a, Acked, None),
      their_n,
    ])
  let assert [other, pk, done] = marked.pockets
  let fired = [
    "ReleaseIdlesEmptyPocket", "ConsumeAckedRecord", "DeadLetterNackedRecord",
  ]
  assert released(marked, now)
    == Driven(
      Ok(Applied(
        State(
          ..marked,
          pockets: [other, Pocket(..pk, status: PocketIdle), done],
          records: [their_a, their_n],
          leases: ended,
          dead_letters: list.append(marked.dead_letters, [
            DeadLetter(
              pocket: "pk",
              key: "n",
              rank: 45,
              payload: <<"en":utf8>>,
              nacked_by: "c1",
              nacked_at: 700,
              expires_at: now + config.defaults().dead_letter_ttl,
            ),
          ]),
        ),
        fired,
        None,
      )),
      clock_reads: 1,
      lease_id_draws: 0,
    )
  ["ReleaseIdlesEmptyPocket", ..fired]
}

/// The rule the Release table sends this release to over this state, if
/// any.
fn release_rule(state: State, release: Release) -> Option(String) {
  let assert Ok(row) = rules.dispatch(settle.release(), state, release)
  option.map(row, fn(row) { row.name })
}

/// Each `requires` line shown to matter on its own. The two rules split on
/// whether a record outlasts the lease, `pocket.records.count` against
/// `pocket.ending_records.count`: one queued record, or one held and
/// unsettled, beside any number acked or nacked, requeues the pocket; with
/// every record acked or nacked, or with none, it goes idle. An acked or a
/// nacked record with a newer version deferred behind it outlasts the lease,
/// alone as beside an ending one. `other`'s
/// records, which are not `pk`'s, do not count. Neither rule holds under
/// `c1`'s own lease once it has ended, under another consumer's active
/// lease, under an id that names no lease, or for `c2` naming `c1`'s. Only
/// what dispatch finds is asserted.
fn release_guards() -> List(String) {
  let #(their_a, their_n) = theirs()
  let a =
    held(Record("pk", "a", 46, <<"ay":utf8>>, 3, RecordQueued), Acked, None)
  let n =
    held(
      Record("pk", "n", 45, <<"en":utf8>>, 8, RecordQueued),
      Nacked,
      Some(700),
    )
  let u =
    held(
      Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued),
      Unsettled,
      None,
    )
  let q = Record("pk", "q", 42, <<"queue":utf8>>, 11, RecordQueued)
  let leased = fn(records) {
    releasing(PocketLeased, 0, [their_a, their_n, ..records])
  }
  let mine = Release("c1", "lease-1")
  let requeues = Some("ReleaseRequeuesPocket")
  let idles = Some("ReleaseIdlesEmptyPocket")
  assert release_rule(leased([a, n, q]), mine) == requeues
  assert release_rule(leased([a, n, u]), mine) == requeues
  assert release_rule(leased([u]), mine) == requeues
  assert release_rule(leased([q]), mine) == requeues
  let deferred = fn(record: Record) {
    let assert RecordHeld(mark:, nacked_at:, ..) = record.status
    Record(
      ..record,
      status: RecordHeld(
        held_rank: 41,
        held_payload: <<"older":utf8>>,
        mark:,
        nacked_at:,
        deferred_position: Some(14),
      ),
    )
  }
  assert release_rule(leased([deferred(a)]), mine) == requeues
  assert release_rule(leased([deferred(n)]), mine) == requeues
  assert release_rule(leased([a, deferred(n)]), mine) == requeues
  assert release_rule(leased([a, n]), mine) == idles
  assert release_rule(leased([n]), mine) == idles
  assert release_rule(leased([]), mine) == idles
  let state = leased([a, u])
  assert release_rule(state, Release("c1", "lease-0")) == None
  assert release_rule(state, Release("c1", "lease-2")) == None
  assert release_rule(state, Release("c2", "lease-1")) == None
  assert release_rule(state, Release("c1", "lease-9")) == None
  assert release_rule(state, Release("c1", "pk")) == None
  assert release_rule(state, Release("c2", "lease-2")) == idles
  []
}

// The refusals of a release ----------------------------------

/// `pk` holds `k` and `other` holds `x`; `c1` has pulled `pk` and `c2`
/// `other`. A retained version sits in the superseded queue. It answers
/// `c1`'s lease id and `c2`'s.
fn two_leases() -> #(Double, String, String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "k", 2, "v2") == fired("Supersede")
  assert harness.publish(double, "other", "x", 1, "x")
    == fired("PublishIntoNewPocket")
  let mine = harness.batch(double, "c1", 1)
  assert mine.pocket_key == "pk"
  let theirs = harness.batch(double, "c2", 1)
  assert theirs.pocket_key == "other"
  #(double, mine.lease, theirs.lease)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape.
/// A release by `c1` that names `c2`'s active lease, or an id no lease has,
/// is fenced. The complete snapshot is unchanged, no clock was read and no
/// lease id was drawn: `c2` still holds `other`, and `c1` still holds `pk`.
fn release_fenced() -> List(String) {
  let #(double, _mine, theirs) = two_leases()
  let _ = harness.at(double, 300)
  harness.refused(double, LeaseFenced("Release", "c1", theirs), fn() {
    harness.release(double, "c1", theirs)
  })
  harness.refused(double, LeaseFenced("Release", "c1", "lease-99"), fn() {
    harness.release(double, "c1", "lease-99")
  })
  harness.fired(double)
}

/// `impl-choice` (spec note G28, as N21): the refusal's name and shape. `c1`
/// acks `k` and releases its lease, which idles `pk`: the lease has ended,
/// and it stays in the state (the ruling on N24). An ack, a nack and a
/// second release that name it are each fenced, and each leaves the complete
/// snapshot, the clock's read count and the lease-id draws as they were.
fn ended_lease_fenced() -> List(String) {
  let #(double, lease, _theirs) = two_leases()
  assert harness.ack(double, "c1", lease, "k")
    == Ok(Outcome(["AckMarksRecord"], None))
  let ended = harness.at(double, 200)
  assert harness.release(double, "c1", lease)
    == Ok(Outcome(["ReleaseIdlesEmptyPocket", "ConsumeAckedRecord"], None))
  let assert Ok(released) = model.lease(harness.inspect(double).leases, lease)
  assert released.status == LeaseReleased(ended)
  let _ = harness.at(double, 400)
  harness.refused(double, LeaseFenced("Ack", "c1", lease), fn() {
    harness.ack(double, "c1", lease, "k")
  })
  harness.refused(double, LeaseFenced("Nack", "c1", lease), fn() {
    harness.nack(double, "c1", lease, "k")
  })
  harness.refused(double, LeaseFenced("Release", "c1", lease), fn() {
    harness.release(double, "c1", lease)
  })
  harness.fired(double)
}

/// `impl-choice` (spec note G36, as N22; G26 is the same for a pull and
/// a renew): the specification states no such refusal. `cx` never joined,
/// and `p` joined as a publisher only. Each release names `c1`'s active
/// lease or a lease never issued, and each is refused before any rule runs:
/// the complete snapshot is unchanged, no clock was read and no lease id was
/// drawn.
fn release_by_unknown_consumer() -> List(String) {
  let #(double, lease, _theirs) = two_leases()
  let _ = harness.at(double, 300)
  list.each([#("cx", lease), #("cx", "lease-99"), #("p", lease)], fn(names) {
    let #(consumer, lease) = names
    harness.refused(double, UnknownConsumer(consumer), fn() {
      harness.release(double, consumer, lease)
    })
  })
  harness.fired(double)
}

/// `impl-choice` (spec note G39, as N26; G23 is the same for a pull): a
/// state no release through the owner reaches, since it breaks
/// `UnleasedHasNoActiveLease`. `c1`'s lease is active and its own, and its
/// pocket is queued, idle or parked. Both rules require the pocket leased,
/// so no guard holds, and the table's refusal past fencing is
/// `ReleaseRefused`. The step is done at once: no clock is read, no lease id
/// is drawn and no state is handed back.
fn release_on_unleased_pocket() -> List(String) {
  list.each([PocketQueued, PocketIdle, PocketParked], fn(status) {
    let state = releasing(status, 0, [])
    assert release_rule(state, Release("c1", "lease-1")) == None
    assert core.apply(state, core.Release(Release("c1", "lease-1")))
      == Done(Error(Refused(ReleaseRefused("c1", "lease-1"))))
  })
  []
}

/// Each release rule's effect reads the lease and the leased pocket its
/// guard saw. Where they are missing, which only a wrong guard could bring
/// about, the outcome is a defect that names the rule: a value, not a
/// crash. The step is done without asking for the time or a position. Each
/// rule is run under a guard that always holds, for a lease that has ended.
fn release_without_lease() -> List(String) {
  let state = releasing(PocketLeased, 0, [])
  list.each(settle.release().rows, fn(row) {
    let unguarded =
      Table(
        "Release",
        [Row(row.name, fn(_state, _release) { True }, row.effect)],
        Refuses(fn(_state, _release) { UnknownConsumer("unused") }),
      )
    assert core.run(unguarded, Release("c1", "lease-0"), state)
      == Done(
        Error(Defect(
          "Release",
          EffectFailed(
            row.name,
            "no active lease lease-0 of consumer c1 on a leased pocket",
          ),
        )),
      )
  })
  []
}
