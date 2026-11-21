//// The rest of `crates/libpachinko/tests/queue/lease.rs`: the consumer's
//// view (S076), and the lease's end by release or by lapse, through the six
//// rules that answer `Freed`, with the pocket's fate (S078, S079, S080,
//// S081, S087, S088, S089 and S093). Each test is a port of its Rust test,
//// driving the owner through `harness`.
////
//// A port asserts what its Rust test asserts, with these changes:
////
//// - The Rust loops S078, S079, S080, S081, S087 and S088 over `ENDINGS`,
////   ending the lease by a release at 50 ms or by a tick at 2,000 ms. Each
////   half is its own registered test carrying the scenario's tag. The lapse
////   half moves the clock past `expires_at` and ticks; nothing else ends
////   the lease.
//// - The Rust's publish answers `Superseded` or `NewKey`, its ack and nack
////   a `SettleOutcome`, and its release the pocket's fate and the ended
////   lease. The specification's answer nothing, so a port asserts the rules
////   each fired and reads the state from the snapshot (the rulings on N24
////   and N32, G15, G37). The ended lease stays in the state, released with
////   its `ended_at`.
//// - The Rust's tick answers a `TickReport` of its own shape; a port
////   asserts the lapsed lease's id from the double's `TickReport`.
//// - `impl-choice` (spec notes G04 and G48): the absolute positions the
////   Rust asserts, which hold because the double's first position is 0.
////   Each port also reads each position from the state before the step and
////   compares it with that (N38).
//// - `impl-choice` (spec note G38, as N25 and N29): where one end frees
////   records of different marks, the order of the `Freed` rules in the
////   fired names, and of the entries they create, smallest held position
////   first.
//// - S093 lists the pocket's records in the Rust's snapshot order; the port
////   compares them by key (the ruling on N38, as N31).

import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import harness.{type Double}
import pachinko_double/config.{Settings}
import pachinko_double/consuming.{HeldView, LeaseView}
import pachinko_double/core.{type TickReport}
import pachinko_double/invariants
import pachinko_double/model.{
  type Mark, type Record, type RecordStatus, Acked, DeadLetter, LeaseReleased,
  Nacked, PocketIdle, PocketParked, PocketQueued, RecordHeld, RecordQueued,
  Superseded, SupersededRecord, Unsettled,
}
import pachinko_double/owner.{type Reply, Outcome}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "the consumer view exposes only the active lease and its held versions",
      [Scenario("S076")],
      consumer_view,
    ),
    Test(
      "Scenario A: an acked version is consumed and the deferred one queued at its arrival, by release",
      [Scenario("S078")],
      fn() { scenario_a(ByRelease) },
    ),
    Test(
      "Scenario A: an acked version is consumed and the deferred one queued at its arrival, by lapse",
      [Scenario("S078")],
      fn() { scenario_a(ByLapse) },
    ),
    Test(
      "Scenario B: a nacked version is dead-lettered and the deferred one queued at its arrival, by release",
      [Scenario("S079")],
      fn() { scenario_b(ByRelease) },
    ),
    Test(
      "Scenario B: a nacked version is dead-lettered and the deferred one queued at its arrival, by lapse",
      [Scenario("S079")],
      fn() { scenario_b(ByLapse) },
    ),
    Test(
      "Scenario C: v3 is shed when v5 arrives and v5 takes v3's arrival position, by release",
      [Scenario("S080")],
      fn() { scenario_c(ByRelease) },
    ),
    Test(
      "Scenario C: v3 is shed when v5 arrives and v5 takes v3's arrival position, by lapse",
      [Scenario("S080")],
      fn() { scenario_c(ByLapse) },
    ),
    Test(
      "Scenario D: an unsettled version is superseded at the end and v3 takes its position, by release",
      [Scenario("S081")],
      fn() { scenario_d(ByRelease) },
    ),
    Test(
      "Scenario D: an unsettled version is superseded at the end and v3 takes its position, by lapse",
      [Scenario("S081")],
      fn() { scenario_d(ByLapse) },
    ),
    Test(
      "a pocket whose records were all acked or nacked idles, by release",
      [Scenario("S087")],
      fn() { all_marked_idles(ByRelease) },
    ),
    Test(
      "a pocket whose records were all acked or nacked idles, by lapse",
      [Scenario("S087")],
      fn() { all_marked_idles(ByLapse) },
    ),
    Test(
      "a pocket whose records were all acked, one with a deferred version, is queued, by release",
      [Scenario("S088")],
      fn() { all_acked_one_deferred(ByRelease) },
    ),
    Test(
      "a pocket whose records were all acked, one with a deferred version, is queued, by lapse",
      [Scenario("S088")],
      fn() { all_acked_one_deferred(ByLapse) },
    ),
    Test(
      "at an expiry limit of one a pocket of nacks parks with no records, and its dead letters are created",
      [Scenario("S089")],
      limit_one_nacks_park,
    ),
    Test(
      "at an expiry limit of one a marked version with a deferred one parks with it queued",
      [Scenario("S093")],
      limit_one_marked_with_deferred_parks,
    ),
  ]
}

fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

fn fired_all(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}

const claims = "PullClaimsFrontPocket"

const hold = "HoldRecord"

const defers = "DeferBehindHeldVersion"

const requeues = "ReleaseRequeuesPocket"

const release_idles = "ReleaseIdlesEmptyPocket"

const keeps = "LeaseExpiryKeepsPocketAtFront"

const lapse_idles = "LeaseExpiryIdlesEmptyPocket"

const parks = "LeaseExpiryParksPocket"

const consume = "ConsumeAckedRecord"

const consume_deferred = "ConsumeAckedVersionQueueDeferred"

const dead_letter = "DeadLetterNackedRecord"

const dead_letter_deferred = "DeadLetterNackedVersionQueueDeferred"

const supersede_held = "SupersedeUnsettledHeldVersion"

/// The Rust fixture's `tick`: one tick, which must not fail, after which
/// every invariant holds, `ExpiriesWithinLimit` among them.
fn tick(double: Double) -> TickReport {
  let assert Ok(report) = harness.tick(double)
  assert invariants.check(harness.inspect(double)) == []
  report
}

/// The Rust test file's `two_pockets`: pocket `a` with `a1` and `a2`, then
/// pocket `b` with `b1`, over the fixture's settings.
fn two_pockets() -> Double {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "a2", 1, "a2") == fired("PublishNewKey")
  assert harness.publish(double, "b", "b1", 1, "b1")
    == fired("PublishIntoNewPocket")
  double
}

/// A pull by `consumer` that claims a pocket, holding each record of the
/// batch. Answers the lease's id.
fn claim(double: Double, consumer: String, limit: Int) -> String {
  let batch = harness.batch(double, consumer, limit)
  assert batch.fired
    == [claims, ..list.repeat(hold, list.length(batch.records))]
  batch.lease
}

/// The Rust fixture's `held_as`.
fn held_as(
  rank: Int,
  payload: String,
  mark: Mark,
  nacked_at: Option(Int),
  arrival: Option(Int),
) -> RecordStatus {
  RecordHeld(
    held_rank: rank,
    held_payload: <<payload:utf8>>,
    mark:,
    nacked_at:,
    deferred_position: arrival,
  )
}

/// How a test ends `c1`'s lease, as the Rust's `Ending`: released at
/// 50 ms, or lapsed by a tick at 2,000 ms, past every expiry these tests
/// reach and before any entry they shed decays.
type Ending {
  ByRelease
  ByLapse
}

/// The Rust's `Ending::end`: ends `lease` this way and answers the instant
/// it ended. A release fires `release_rule` and a lapse `LeaseLapses` and
/// `lapse_rule`, each followed by `freed`, the rules that answer its
/// `Freed`.
fn end(
  double: Double,
  ending: Ending,
  lease: String,
  rules: #(String, String),
  freed: List(String),
) -> Int {
  case ending {
    ByRelease -> {
      let ended = harness.at(double, 50)
      assert harness.release(double, "c1", lease)
        == fired_all([rules.0, ..freed])
      let assert Ok(released) =
        model.lease(harness.inspect(double).leases, lease)
      assert released.status == LeaseReleased(ended_at: ended)
      ended
    }
    ByLapse -> lapse(double, lease, rules.1, freed)
  }
}

/// The Rust's `Ending::Lapse.end`: the clock at 2,000 ms and a tick, which
/// lapses `lease` alone, firing `LeaseLapses`, `rule` and `freed`.
fn lapse(
  double: Double,
  lease: String,
  rule: String,
  freed: List(String),
) -> Int {
  let ended = harness.at(double, 2000)
  let report = tick(double)
  // The lease lapsed.
  assert list.map(report.lapsed, fn(lapsed) { lapsed.id }) == [lease]
  assert report.fired == ["LeaseLapses", rule, ..freed]
  ended
}

/// The two ending rules of a pocket a record outlasts, and of one none does.
const queued_fate = #(requeues, keeps)

const idle_fate = #(release_idles, lapse_idles)

/// The Rust's `Ending::count`: the pocket's consecutive-expiry count after
/// this end of a lease that started at 0.
fn count(ending: Ending) -> Int {
  case ending {
    ByRelease -> 0
    ByLapse -> 1
  }
}

/// The Rust test file's `v3_deferred_behind_v1`: `pk` with `k` at rank 1,
/// payload `v1`, then `early`; `c1` pulls `k` alone at `t0`, and `v3`
/// arrives at 10 ms, deferred behind `v1`. Answers the lease and `v3`'s
/// arrival position.
fn v3_deferred_behind_v1() -> #(Double, String, Int) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "early", 1, "e")
    == fired("PublishNewKey")
  let lease = claim(double, "c1", 1)
  let arrival = harness.inspect(double).next_position
  let _ = harness.at(double, 10)
  assert harness.publish(double, "pk", "k", 3, "v3") == fired(defers)
  let record = harness.record(double, "pk", "k")
  // "v3 is deferred behind the held v1", at the counter's value when it
  // arrived. `impl-choice` (G04): that value is 3.
  assert #(record.status, record.rank)
    == #(held_as(1, "v1", Unsettled, None, Some(arrival)), 3)
  assert arrival == 3
  #(double, lease, arrival)
}

/// The Rust test file's `after_the_end`: `pk` is queued with the count
/// `ending` leaves, `k` is queued at `rank` and `position`, and `c2` is
/// handed `order`.
fn after_the_end(
  double: Double,
  ending: Ending,
  rank_and_position: #(Int, Int),
  order: List(String),
) -> Nil {
  let pocket = harness.pocket(double, "pk")
  assert #(pocket.status, pocket.consecutive_expiries)
    == #(PocketQueued, count(ending))
  let record = harness.record(double, "pk", "k")
  assert #(record.status, record.rank, record.position)
    == #(RecordQueued, rank_and_position.0, rank_and_position.1)
  let batch = harness.batch(double, "c2", 10)
  assert batch.fired == [claims, ..list.repeat(hold, list.length(order))]
  assert harness.keys(batch.records) == order
  Nil
}

fn shed(pocket: String, key: String, rank: Int, payload: String, shed_at: Int) {
  SupersededRecord(
    pocket:,
    key:,
    rank:,
    payload: <<payload:utf8>>,
    reason: Superseded,
    shed_at:,
    expires_at: shed_at + harness.fixture_settings().superseded_ttl,
  )
}

fn letter(
  pocket: String,
  key: String,
  rank: Int,
  payload: String,
  nacked_at: Int,
  ended: Int,
) {
  DeadLetter(
    pocket:,
    key:,
    rank:,
    payload: <<payload:utf8>>,
    nacked_by: "c1",
    nacked_at:,
    expires_at: ended + harness.fixture_settings().dead_letter_ttl,
  )
}

// The catalog rows ------------------------------------------

fn consumer_view() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "queued", 1, "q")
    == fired("PublishNewKey")
  assert harness.publish(double, "other", "x", 1, "x")
    == fired("PublishIntoNewPocket")
  let lease = claim(double, "c1", 1)
  let _ = claim(double, "c2", 10)
  assert harness.publish(double, "pk", "k", 3, "v3") == fired(defers)
  // "v1 is held, not shed."
  assert harness.inspect(double).superseded == []
  let assert Ok(Some(view)) = harness.lease_view(double, "c1")
  assert view.pocket_key == "pk"
  assert view.expires_at == harness.t0 + harness.fixture_settings().lease_ttl
  // Only the held records, and the held version, not the latest, with its
  // mark.
  assert harness.marked_keys(view.records) == ["k"]
  assert view.records == [HeldView("k", 1, <<"v1":utf8>>, Unsettled)]
  assert harness.release(double, "c1", lease)
    == fired_all([requeues, supersede_held])
  assert harness.lease_view(double, "c1") == Ok(None)
  let assert Ok(Some(LeaseView(records: theirs, ..))) =
    harness.lease_view(double, "c2")
  assert harness.marked_keys(theirs) == ["x"]
  harness.fired(double)
}

fn scenario_a(ending: Ending) -> List(String) {
  let #(double, lease, arrival) = v3_deferred_behind_v1()
  let _ = harness.at(double, 20)
  assert harness.ack(double, "c1", lease, "k") == fired("AckMarksRecord")
  let _ = end(double, ending, lease, queued_fate, [consume_deferred])
  let snapshot = harness.inspect(double)
  // v1 is recorded nowhere.
  assert snapshot.superseded == []
  assert snapshot.dead_letters == []
  after_the_end(double, ending, #(3, arrival), ["early", "k"])
  harness.fired(double)
}

fn scenario_b(ending: Ending) -> List(String) {
  let #(double, lease, arrival) = v3_deferred_behind_v1()
  let nacked = harness.at(double, 20)
  assert harness.nack(double, "c1", lease, "k") == fired("NackMarksRecord")
  assert harness.inspect(double).dead_letters == []
  let ended = end(double, ending, lease, queued_fate, [dead_letter_deferred])
  let snapshot = harness.inspect(double)
  assert snapshot.dead_letters == [letter("pk", "k", 1, "v1", nacked, ended)]
  assert snapshot.superseded == []
  after_the_end(double, ending, #(3, arrival), ["early", "k"])
  harness.fired(double)
}

fn scenario_c(ending: Ending) -> List(String) {
  let #(double, lease, arrival) = v3_deferred_behind_v1()
  let _ = harness.at(double, 15)
  assert harness.publish(double, "pk", "between", 1, "b")
    == fired("PublishNewKey")
  let arrived = harness.at(double, 18)
  assert harness.publish(double, "pk", "k", 5, "v5") == fired("Supersede")
  let v3 = shed("pk", "k", 3, "v3", arrived)
  // v3 is shed at once.
  assert harness.inspect(double).superseded == [v3]
  let nacked = harness.at(double, 20)
  assert harness.nack(double, "c1", lease, "k") == fired("NackMarksRecord")
  let ended = end(double, ending, lease, queued_fate, [dead_letter_deferred])
  let snapshot = harness.inspect(double)
  assert snapshot.superseded == [v3]
  assert snapshot.dead_letters == [letter("pk", "k", 1, "v1", nacked, ended)]
  // v5 takes v3's arrival position.
  after_the_end(double, ending, #(5, arrival), ["early", "k", "between"])
  harness.fired(double)
}

fn scenario_d(ending: Ending) -> List(String) {
  let #(double, lease, _) = v3_deferred_behind_v1()
  let own = harness.record(double, "pk", "k").position
  // `impl-choice` (G04): `k`'s own position is 1.
  assert own == 1
  assert harness.inspect(double).superseded == []
  let ended = end(double, ending, lease, queued_fate, [supersede_held])
  let snapshot = harness.inspect(double)
  // v1 is shed at the end.
  assert snapshot.superseded == [shed("pk", "k", 1, "v1", ended)]
  assert snapshot.dead_letters == []
  // v3 takes v1's position.
  after_the_end(double, ending, #(3, own), ["k", "early"])
  harness.fired(double)
}

fn all_marked_idles(ending: Ending) -> List(String) {
  let double = two_pockets()
  let lease = claim(double, "c1", 10)
  let nacked = harness.at(double, 20)
  assert harness.ack(double, "c1", lease, "a1") == fired("AckMarksRecord")
  assert harness.nack(double, "c1", lease, "a2") == fired("NackMarksRecord")
  assert harness.inspect(double).dead_letters == []
  let ended = end(double, ending, lease, idle_fate, [consume, dead_letter])
  let pocket = harness.pocket(double, "a")
  let records = model.records_of(harness.inspect(double).records, "a")
  assert #(pocket.status, pocket.consecutive_expiries, list.length(records))
    == #(PocketIdle, count(ending), 0)
  assert harness.inspect(double).dead_letters
    == [letter("a", "a2", 1, "a2", nacked, ended)]
  assert harness.order(double) == ["b"]
  harness.fired(double)
}

fn all_acked_one_deferred(ending: Ending) -> List(String) {
  let double = two_pockets()
  let lease = claim(double, "c1", 10)
  let arrival = harness.inspect(double).next_position
  let _ = harness.at(double, 10)
  assert harness.publish(double, "a", "a2", 2, "newer") == fired(defers)
  // `impl-choice` (G04): the arrival position is 5.
  assert arrival == 5
  assert harness.record(double, "a", "a2").status
    == held_as(1, "a2", Unsettled, None, Some(arrival))
  let _ = harness.at(double, 20)
  list.each(["a1", "a2"], fn(key) {
    assert harness.ack(double, "c1", lease, key) == fired("AckMarksRecord")
  })
  let _ = end(double, ending, lease, queued_fate, [consume, consume_deferred])
  let pocket = harness.pocket(double, "a")
  assert #(pocket.status, pocket.consecutive_expiries)
    == #(PocketQueued, count(ending))
  let snapshot = harness.inspect(double)
  // Only a2's newer version is left, at its arrival position.
  assert list.map(model.records_of(snapshot.records, "a"), fn(record: Record) {
      #(record.key, record.status, record.rank, record.position)
    })
    == [#("a2", RecordQueued, 2, arrival)]
  assert #(snapshot.superseded, snapshot.dead_letters) == #([], [])
  harness.fired(double)
}

fn limit_one() -> Double {
  harness.fixture_with(
    Settings(..harness.fixture_settings(), max_consecutive_expiries: 1),
  )
}

fn limit_one_nacks_park() -> List(String) {
  let double = limit_one()
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "a2", 1, "a2") == fired("PublishNewKey")
  let lease = claim(double, "c1", 10)
  let nacked = harness.at(double, 20)
  list.each(["a1", "a2"], fn(key) {
    assert harness.nack(double, "c1", lease, key) == fired("NackMarksRecord")
  })
  // Not before the lapse.
  assert harness.inspect(double).dead_letters == []
  let ended = lapse(double, lease, parks, [dead_letter, dead_letter])
  let pocket = harness.pocket(double, "a")
  let records = model.records_of(harness.inspect(double).records, "a")
  assert #(pocket.status, pocket.consecutive_expiries, list.length(records))
    == #(PocketParked, 1, 0)
  // `impl-choice` (G38, as N29): the letters in held-position order.
  assert harness.inspect(double).dead_letters
    == [
      letter("a", "a1", 1, "a1", nacked, ended),
      letter("a", "a2", 1, "a2", nacked, ended),
    ]
  harness.fired(double)
}

/// The Rust loops over the two marks; each runs on an owner of its own.
fn limit_one_marked_with_deferred_parks() -> List(String) {
  list.flat_map([Acked, Nacked], marked_with_deferred_parks)
}

fn marked_with_deferred_parks(mark: Mark) -> List(String) {
  let double = limit_one()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "pk", "early", 1, "e")
    == fired("PublishNewKey")
  assert harness.publish(double, "other", "o", 1, "o")
    == fired("PublishIntoNewPocket")
  let lease = claim(double, "c1", 1)
  let early = harness.record(double, "pk", "early").position
  let arrival = harness.inspect(double).next_position
  let _ = harness.at(double, 10)
  assert harness.publish(double, "pk", "k", 3, "v3") == fired(defers)
  // `impl-choice` (G04): early's position is 2 and the arrival position 5.
  assert #(early, arrival) == #(2, 5)
  assert harness.record(double, "pk", "k").status
    == held_as(1, "v1", Unsettled, None, Some(arrival))
  let nacked = harness.at(double, 20)
  let #(marks, frees) = case mark {
    Acked -> #(harness.ack(double, "c1", lease, "k"), consume_deferred)
    _ -> #(harness.nack(double, "c1", lease, "k"), dead_letter_deferred)
  }
  assert marks
    == fired(case mark {
      Acked -> "AckMarksRecord"
      _ -> "NackMarksRecord"
    })
  let ended = lapse(double, lease, parks, [frees])
  let pocket = harness.pocket(double, "pk")
  assert #(pocket.status, pocket.consecutive_expiries) == #(PocketParked, 1)
  assert harness.order(double) == ["other"]
  let snapshot = harness.inspect(double)
  // v3 is kept, queued, at its arrival position; the records compared by
  // key (the ruling on N38).
  let by_key =
    model.records_of(snapshot.records, "pk")
    |> list.sort(fn(a: Record, b: Record) { string.compare(a.key, b.key) })
  assert list.map(by_key, fn(record) {
      #(record.key, record.status, record.rank, record.payload)
    })
    == [
      #("early", RecordQueued, 1, <<"e":utf8>>),
      #("k", RecordQueued, 3, <<"v3":utf8>>),
    ]
  assert list.map(by_key, fn(record) { #(record.key, record.position) })
    == [#("early", early), #("k", arrival)]
  assert snapshot.superseded == []
  assert snapshot.dead_letters
    == case mark {
      Nacked -> [letter("pk", "k", 1, "v1", nacked, ended)]
      _ -> []
    }
  harness.fired(double)
}
