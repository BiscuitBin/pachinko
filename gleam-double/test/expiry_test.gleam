//// The clock's side: the owner's `Tick`, the specification's "Lease expiry"
//// section and the revival of a parked pocket.
////
//// First three catalog rows of `crates/libpachinko/tests/queue/expiry.rs`,
//// S059, S065 and S006, each a port of its Rust test, driving the owner. The
//// Rust's fixture checks every invariant after each tick; `tick` here does
//// the same with the checker. The Rust's report gives the lapsed leases; the
//// double's `TickReport` gives them too, as the state keeps them once
//// ended, beside the rules fired and the entries that decayed (spec note
//// N33). The Rust's batch carries `expires_at`; the double's does not, so a
//// port reads it from the snapshot's lease (G22, N13).
////
//// Then the double's own tests of a tick, which pin no catalog row: the
//// clock is read once whatever is due, a lease lapses once, several lapses
//// go in `expires_at` order with ties in grant order (N34), and an entry a
//// lapse creates with a zero TTL decays in the same tick (N35).
////
//// Then each of the five rules here run alone over states built by hand,
//// through `core.run_at`, which runs an outcome with the tick's `now` in
//// hand: what each changes, compared with the whole state, the rules fired,
//// and the clock reads and lease-id draws, which are none.

import gleam/list
import gleam/option.{type Option, None, Some}
import harness.{type Double, Driven}
import pachinko_double.{type Reply, Outcome}
import pachinko_double/answers.{type TickReport, TickReport}
import pachinko_double/config.{Settings}
import pachinko_double/failure
import pachinko_double/inspection
import pachinko_double/internal/core
import pachinko_double/internal/domain.{LeaseLapsed}
import pachinko_double/internal/expiry.{Lapse}
import pachinko_double/internal/publish.{Publication}
import pachinko_double/internal/rules.{Applied, Done, Finish, Now}
import pachinko_double/invariants
import pachinko_double/model.{
  type Mark, type Record, type State, Acked, Consumer, DeadLetter, Lease,
  LeaseActive, LeaseExpired, LeaseReleased, Nacked, Pocket, PocketIdle,
  PocketLeased, PocketParked, PocketQueued, Publisher, Queue, Record, RecordHeld,
  RecordQueued, Stale, State, Superseded, SupersededRecord, Unsettled,
}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a lease lapses exactly at expires_at",
      [Scenario("S059")],
      lapses_exactly_at_expiry,
    ),
    Test("a lapse fires once per lease", [Scenario("S065")], lapse_fires_once),
    Test(
      "a zero lease_ttl lapses at the instant of grant",
      [Scenario("S006")],
      zero_ttl_lapses_at_grant,
    ),
    Test(
      "a tick reads the clock exactly once, with nothing due and with several things due",
      [],
      tick_reads_once,
    ),
    Test(
      "a lease lapses once: a second tick at the same instant lapses nothing and changes nothing",
      [],
      lapses_once,
    ),
    Test(
      "a tick lapses in expires_at order, with equal expires_at in grant order",
      [],
      lapse_order,
    ),
    Test(
      "a tie in expires_at lapses in creation order, even against granted_at after the clock stepped back",
      [],
      tie_by_creation_not_granted_at,
    ),
    Test(
      "an entry a lapse creates with a zero TTL decays in the same tick",
      [],
      zero_ttl_entries_decay_in_the_lapse_tick,
    ),
    Test(
      "LeaseLapses changes nothing, emits LeaseLapsed with the tick's now, and holds only for an active lease",
      [],
      lease_lapses_exactly,
    ),
    Test(
      "LeaseExpiryParksPocket changes exactly what it ensures and reads no clock",
      [],
      parks_exactly,
    ),
    Test(
      "LeaseExpiryKeepsPocketAtFront changes exactly what it ensures, keeps the pocket's position and reads no clock",
      [],
      keeps_at_front_exactly,
    ),
    Test(
      "LeaseExpiryIdlesEmptyPocket changes exactly what it ensures and reads no clock",
      [],
      idles_exactly,
    ),
    Test(
      "the LeaseLapsed rows hold only for an active lease past expires_at at the event's now on a leased pocket, and otherwise nothing happens",
      [],
      lapse_guards,
    ),
    Test(
      "a tick whose second lapse fails keeps the first, stops there, and decays nothing",
      [],
      tick_stops_at_a_failed_lapse,
    ),
    Test(
      "ReviveParkedPocket changes exactly what it ensures and reads no clock",
      [],
      revive_parked_exactly,
    ),
  ]
}

const keeps = "LeaseExpiryKeepsPocketAtFront"

const requeue = "RequeueUnsettledRecord"

fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
}

/// The Rust test file's `fixture`: a 10 ms lease.
fn short() -> Double {
  harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: 10))
}

/// The Rust test file's `two_pockets`: pocket `a` holding `a1` and `a2`,
/// then pocket `b` holding `b1`, with a 10 ms lease.
fn two_pockets() -> Double {
  let double = short()
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "a2", 1, "a2") == fired("PublishNewKey")
  assert harness.publish(double, "b", "b1", 1, "b1")
    == fired("PublishIntoNewPocket")
  double
}

/// The Rust fixture's `tick`: one tick, which must not fail, after which
/// every invariant holds.
fn tick(double: Double) -> TickReport {
  let assert Ok(report) = harness.tick(double)
  assert invariants.check(harness.inspect(double)) == []
  report
}

/// The lease with this id, from a fresh snapshot.
fn lease(double: Double, id: String) -> model.Lease {
  let assert Ok(lease) = model.lease(harness.inspect(double).leases, id)
  lease
}

/// What a lapse of `a` under `c1`'s lease fires, with both of `a`'s records
/// held and unsettled.
const lapse_of_a = ["LeaseLapses", keeps, requeue, requeue]

// The catalog rows -----------------------------------------

fn lapses_exactly_at_expiry() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  assert batch.pocket_key == "a"
  assert lease(double, batch.lease).expires_at == harness.t0 + 10
  harness.at(double, 9)
  let before = harness.inspect(double)
  assert tick(double) == TickReport([], [], [], [])
  // A tick before expiry changed the queue.
  assert harness.inspect(double) == before
  let now = harness.at(double, 10)
  let report = tick(double)
  let assert [lapsed] = report.lapsed
  assert lapsed.id == batch.lease
  assert lapsed.status == LeaseExpired(ended_at: now)
  assert lapsed.expires_at == now
  assert report == TickReport(lapse_of_a, [lapsed], [], [])
  // The Rust's `invariants::ended_after_grant`, by the checker, which `tick`
  // ran over the snapshot that holds the lapsed lease.
  assert lease(double, batch.lease) == lapsed
  harness.fired(double)
}

fn lapse_fires_once() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  let _ = harness.at(double, 10)
  let assert [lapsed] = tick(double).lapsed
  assert lapsed.id == batch.lease
  let after = harness.inspect(double)
  list.each([10, 20, 1000], fn(later) {
    harness.at(double, later)
    // A lease lapsed twice.
    assert tick(double).lapsed == []
  })
  assert harness.inspect(double) == after
  assert harness.pocket(double, "a").consecutive_expiries == 1
  harness.fired(double)
}

fn zero_ttl_lapses_at_grant() -> List(String) {
  let double =
    harness.fixture_with(Settings(..harness.fixture_settings(), lease_ttl: 0))
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired("PublishIntoNewPocket")
  let batch = harness.batch(double, "c1", 10)
  assert lease(double, batch.lease).expires_at == harness.t0
  // The clock has not moved since the grant.
  let report = tick(double)
  let assert [lapsed] = report.lapsed
  assert lapsed.id == batch.lease
  assert lapsed.status == LeaseExpired(ended_at: harness.t0)
  assert report.fired == ["LeaseLapses", keeps, requeue]
  assert harness.lease_view(double, "c1") == Ok(None)
  harness.fired(double)
}

// The double's own tests of a tick -------------------------

/// With nothing due, a tick still reads the clock once, and changes
/// nothing. With two leases due and a superseded record due, it reads it
/// once too, though it runs three outcomes and fires nine rules.
fn tick_reads_once() -> List(String) {
  let double = two_pockets()
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  assert tick(double) == TickReport([], [], [], [])
  assert harness.clock_reads(double.clock) == reads + 1
  assert harness.inspect(double) == before

  // `a` and `b` leased at t0, and a stale version of `a1` shed at t0, whose
  // superseded_ttl is 5000 ms.
  let first = harness.batch(double, "c1", 10)
  let second = harness.batch(double, "c2", 10)
  assert harness.publish(double, "a", "a1", 0, "stale")
    == fired("DropStalePublish")
  let assert [shed] = harness.inspect(double).superseded
  let _ = harness.at(double, 5000)
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  let report = tick(double)
  assert harness.clock_reads(double.clock) == reads + 1
  assert harness.lease_id_draws(double.lease_ids) == draws
  assert list.map(report.lapsed, fn(lease) { lease.id })
    == [first.lease, second.lease]
  assert report.superseded == [shed]
  assert report.fired
    == list.flatten([
      lapse_of_a,
      ["LeaseLapses", keeps, requeue],
      ["SupersededRecordDecays"],
    ])
  harness.fired(double)
}

/// A lease that lapsed is expired, so the next tick, at the same instant or
/// later, offers it nothing: the report is empty, no rule fires, and the
/// complete state is as the lapse left it.
fn lapses_once() -> List(String) {
  let double = two_pockets()
  let batch = harness.batch(double, "c1", 10)
  let _ = harness.at(double, 10)
  let report = tick(double)
  assert list.map(report.lapsed, fn(lease) { lease.id }) == [batch.lease]
  assert list.count(report.fired, fn(name) { name == "LeaseLapses" }) == 1
  let after = harness.inspect(double)
  let logged = harness.fired(double)
  assert tick(double) == TickReport([], [], [], [])
  assert harness.inspect(double) == after
  assert harness.fired(double) == logged
  logged
}

/// Four leases, granted in the order `lease-1` to `lease-4` with a 10 ms
/// lease. `lease-1` (`a`, `c1`), `lease-2` (`b`, `c3`) and `lease-3` (`c`,
/// `c2`) are granted at t0; `lease-4` (`d`, `c4`) at t0 + 3; and `c1`
/// renews `lease-1` at t0 + 5. So they expire at t0 + 15, 10, 10 and 13.
/// One tick at t0 + 20 lapses `lease-2`, then `lease-3`, the two granted in
/// that order with equal `expires_at`, then `lease-4`, then `lease-1`. The
/// consumers of the tied pair are not in id order, so a tie broken by
/// consumer would show. `impl-choice` (spec note G57, as N34): the
/// specification gives no order among several lapses due at once, so the
/// order of the report and of `fired`, the tie's above all, is the
/// double's.
fn lapse_order() -> List(String) {
  let double = short()
  assert harness.consumer_joins(double, "c4") == fired("ConsumerJoins")
  list.each(["a", "b", "c", "d"], fn(pocket_key) {
    assert harness.publish(double, pocket_key, "k", 1, pocket_key)
      == fired("PublishIntoNewPocket")
  })
  let l1 = harness.batch(double, "c1", 10)
  let l2 = harness.batch(double, "c3", 10)
  let l3 = harness.batch(double, "c2", 10)
  let _ = harness.at(double, 3)
  let l4 = harness.batch(double, "c4", 10)
  let _ = harness.at(double, 5)
  assert harness.renew(double, "c1", l1.lease) == fired("Renew")
  assert list.map([l1, l2, l3, l4], fn(pulled) { pulled.pocket_key })
    == ["a", "b", "c", "d"]
  assert list.map(harness.inspect(double).leases, fn(lease) { lease.id })
    == [l1.lease, l2.lease, l3.lease, l4.lease]
  let now = harness.at(double, 20)
  let report = tick(double)
  assert list.map(report.lapsed, fn(lease) { lease.id })
    == [l2.lease, l3.lease, l4.lease, l1.lease]
  assert list.map(report.lapsed, fn(lease) { lease.status })
    == list.repeat(LeaseExpired(ended_at: now), 4)
  assert report.fired
    == list.flatten(list.repeat(["LeaseLapses", keeps, requeue], 4))
  harness.fired(double)
}

/// A test may set the fake clock back, and the double does not clamp it, so
/// creation order and `granted_at` can disagree. `c1` claims `a` at t0 + 10; the clock steps
/// back to t0, and `c2` claims `b`, expiring at t0 + 10; at t0 + 10 `c2`
/// renews, to t0 + 20, where `c1`'s lease expires too. The tied pair's
/// `granted_at` puts `c2`'s first; the tick at t0 + 20 lapses `c1`'s first,
/// the order they were created in. `impl-choice` (spec note G57, as N34).
fn tie_by_creation_not_granted_at() -> List(String) {
  let double = short()
  list.each(["a", "b"], fn(pocket_key) {
    assert harness.publish(double, pocket_key, "k", 1, pocket_key)
      == fired("PublishIntoNewPocket")
  })
  let _ = harness.at(double, 10)
  let first = harness.batch(double, "c1", 10)
  let _ = harness.at(double, 0)
  let second = harness.batch(double, "c2", 10)
  let _ = harness.at(double, 10)
  assert harness.renew(double, "c2", second.lease) == fired("Renew")
  assert list.map([first, second], fn(pulled) { pulled.pocket_key })
    == ["a", "b"]
  let created = lease(double, first.lease)
  let renewed = lease(double, second.lease)
  assert created.granted_at == harness.t0 + 10
  assert renewed.granted_at == harness.t0
  assert created.expires_at == renewed.expires_at
  let _ = harness.at(double, 20)
  let report = tick(double)
  assert list.map(report.lapsed, fn(lease) { lease.id })
    == [first.lease, second.lease]
  harness.fired(double)
}

/// With both shed TTLs 0, `c1` holds `a`'s two records: `n`, which it
/// nacks, and `u`, unsettled, with a newer version deferred behind it. The
/// lapse makes a dead letter of `n` and a superseded record of `u`'s held
/// version, each with `expires_at` the lapse's `now`, so each is due at once
/// and decays in the same tick (N35). Once the tick returns nothing is due
/// at its `now`.
fn zero_ttl_entries_decay_in_the_lapse_tick() -> List(String) {
  let double =
    harness.fixture_with(
      Settings(
        ..harness.fixture_settings(),
        lease_ttl: 10,
        superseded_ttl: 0,
        dead_letter_ttl: 0,
      ),
    )
  assert harness.publish(double, "a", "n", 1, "n")
    == fired("PublishIntoNewPocket")
  assert harness.publish(double, "a", "u", 1, "u") == fired("PublishNewKey")
  let batch = harness.batch(double, "c1", 10)
  assert harness.nack(double, "c1", batch.lease, "n")
    == fired("NackMarksRecord")
  assert harness.publish(double, "a", "u", 2, "newer")
    == fired("DeferBehindHeldVersion")
  let now = harness.at(double, 10)
  let report = tick(double)
  assert report.superseded
    == [
      SupersededRecord(
        pocket: "a",
        key: "u",
        rank: 1,
        payload: <<"u":utf8>>,
        reason: Superseded,
        shed_at: now,
        expires_at: now,
      ),
    ]
  assert report.dead_letters
    == [
      DeadLetter(
        pocket: "a",
        key: "n",
        rank: 1,
        payload: <<"n":utf8>>,
        nacked_by: "c1",
        nacked_at: harness.t0,
        expires_at: now,
      ),
    ]
  assert report.fired
    == [
      "LeaseLapses", keeps, "DeadLetterNackedRecord",
      "SupersedeUnsettledHeldVersion", "SupersededRecordDecays",
      "DeadLetterDecays",
    ]
  let snapshot = harness.inspect(double)
  assert snapshot.superseded == []
  assert snapshot.dead_letters == []
  assert tick(double) == TickReport([], [], [], [])
  harness.fired(double)
}

// The rules over states built by hand ----------------------

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
/// `n`, nacked at 720.
fn theirs() -> #(Record, Record) {
  #(
    held(Record("other", "a", 48, <<"oa":utf8>>, 12, RecordQueued), Acked, None),
    held(
      Record("other", "n", 47, <<"on":utf8>>, 10, RecordQueued),
      Nacked,
      Some(720),
    ),
  )
}

/// When `c1`'s lease, `lease-1`, granted at 600 for the default
/// `lease_ttl`, is found due: seven milliseconds after its `expires_at`, so
/// a row that stamped `expires_at` instead of `now` would show.
fn due_at() -> Int {
  600 + config.defaults().lease_ttl + 7
}

/// Consumers `c1` and `c2` joined, with the default config, whose
/// `max_consecutive_expiries` is 3.
///
/// - `c1` holds `pk` under `lease-1`, granted at 600, due at `due_at()`.
///   `pk` is leased at position 2, has seen `expiries` consecutive expiries,
///   and holds `records`.
/// - `c1` once held `pk` under `lease-0`, since released.
/// - `c2` holds `other` under `lease-2`, granted at 700 and not yet due;
///   `other`'s records are in `records` too.
/// - `done` is idle and has seen one expiry.
/// - The superseded queue holds a stale version of a `pk` record, and the
///   dead-letter queue a record `c1` nacked out of `pk` under `lease-0`.
///
/// The counter stands at 20.
fn lapsing(expiries: Int, records: List(Record)) -> State {
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
        granted_at: 700,
        expires_at: 700 + ttl,
      ),
      Lease(
        id: "lease-1",
        pocket: "pk",
        consumer: "c1",
        status: LeaseActive,
        granted_at: 600,
        expires_at: 600 + ttl,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "u",
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

/// `lease-1`'s lapse at `due_at()`, as the `LeaseLapsed` table's own
/// outcome with the tick's `now` in hand, driven with a clock that answers
/// a time no row should stamp.
fn lapsed(state: State) -> harness.Driven {
  let now = due_at()
  harness.drive(
    core.run_at(expiry.lease_lapsed(), Lapse("lease-1", now), state, now),
    now + 1,
    "unused",
  )
}

/// The rule the `LeaseLapsed` table sends `lease-1`'s lapse at `now` to,
/// if any.
fn expiry_rule_at(state: State, lease: String, now: Int) -> Option(String) {
  let assert Ok(row) =
    rules.dispatch(expiry.lease_lapsed(), state, Lapse(lease, now))
  option.map(row, fn(row) { row.name })
}

fn expiry_rule(state: State) -> Option(String) {
  expiry_rule_at(state, "lease-1", due_at())
}

/// The row alone: over `lapsing`, its guard holds, and its effect asks for
/// `now`, changes nothing and emits `LeaseLapsed` for the lease with that
/// `now`. As a whole outcome, with the tick's `now` in hand, it reads no
/// clock and goes on to the expiry rule. For a lease that is released or
/// expired, or for an id no lease has, nothing happens.
fn lease_lapses_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let u = Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued)
  let state = lapsing(1, [their_a, held(u, Unsettled, None), their_n])
  let now = due_at()
  let lapse = Lapse("lease-1", now)
  let assert [row] = expiry.lease_lapses().rows
  assert row.name == "LeaseLapses"
  assert row.guard(state, lapse)
  let assert Now(next) = row.effect(state, lapse)
  assert next(now) == Finish(state, [LeaseLapsed("lease-1", now)], None)

  let fired = ["LeaseLapses", keeps, requeue]
  let assert Driven(Ok(applied), clock_reads: 0, lease_id_draws: 0) =
    harness.drive(
      core.run_at(expiry.lease_lapses(), lapse, state, now),
      now + 1,
      "unused",
    )
  assert applied.fired == fired
  let assert Ok(ended) = model.lease(applied.state.leases, "lease-1")
  assert ended.status == LeaseExpired(ended_at: now)

  let untouched = fn(state: State, lease: String) {
    assert !row.guard(state, Lapse(lease, now))
    assert core.run_at(expiry.lease_lapses(), Lapse(lease, now), state, now)
      == Done(Ok(Applied(state, [], None)))
  }
  untouched(state, "lease-0")
  untouched(state, "nowhere")
  untouched(applied.state, "lease-1")
  fired
}

/// `pk` has seen two expiries, so this lapse is its third and it parks
/// though `u`, unsettled, and `q`, queued, outlast the lease. The lease is
/// `expired` at the tick's `now`, `u` is queued where it stood, and `pk` is
/// parked with its count 3 at the position it had. No position is allotted.
/// Nothing else in the complete state changes.
fn parks_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let u = Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued)
  let q = Record("pk", "q", 42, <<"queue":utf8>>, 11, RecordQueued)
  let state = lapsing(2, [their_a, held(u, Unsettled, None), q, their_n])
  let assert [other, pk, done] = state.pockets
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = due_at()
  let fired = ["LeaseExpiryParksPocket", requeue]
  assert expiry_rule(state) == Some("LeaseExpiryParksPocket")
  assert lapsed(state)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          pockets: [
            other,
            Pocket(..pk, status: PocketParked, consecutive_expiries: 3),
            done,
          ],
          records: [their_a, u, q, their_n],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, status: LeaseExpired(now)),
          ],
        ),
        fired,
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  fired
}

/// `pk` has seen one expiry and holds two unsettled records, the one listed
/// first at the larger position. Both outlast the lease: each is queued
/// where it stood, the smaller position freed first. The lease is `expired`
/// at the tick's `now`, and `pk` is queued with its count 2 at the position
/// it had, 2, the front: the counter stays at 20. Nothing else changes.
fn keeps_at_front_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let late = Record("pk", "a", 44, <<"late":utf8>>, 6, RecordQueued)
  let early = Record("pk", "n", 45, <<"early":utf8>>, 3, RecordQueued)
  let state =
    lapsing(1, [
      their_a,
      held(late, Unsettled, None),
      their_n,
      held(early, Unsettled, None),
    ])
  let assert [other, pk, done] = state.pockets
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = due_at()
  let fired = [keeps, requeue, requeue]
  assert expiry_rule(state) == Some(keeps)
  assert lapsed(state)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          pockets: [
            other,
            Pocket(..pk, status: PocketQueued, consecutive_expiries: 2),
            done,
          ],
          records: [their_a, late, their_n, early],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, status: LeaseExpired(now)),
          ],
        ),
        fired,
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  fired
}

/// `pk` has seen no expiry and holds an acked record and a nacked one, the
/// nacked one listed first at the larger position, so none outlasts the
/// lease. The acked one is consumed and the nacked one becomes a dead letter
/// nacked by `c1` at its own nack time, expiring `dead_letter_ttl` after
/// the tick's `now`. `pk` is idle with its count 1 at the position it had.
/// Nothing else changes.
fn idles_exactly() -> List(String) {
  let #(their_a, their_n) = theirs()
  let n = Record("pk", "n", 45, <<"en":utf8>>, 8, RecordQueued)
  let a = Record("pk", "a", 46, <<"ay":utf8>>, 3, RecordQueued)
  let state =
    lapsing(0, [
      held(n, Nacked, Some(700)),
      their_a,
      held(a, Acked, None),
      their_n,
    ])
  let assert [other, pk, done] = state.pockets
  let assert [lease_0, lease_2, lease_1] = state.leases
  let now = due_at()
  let fired = [
    "LeaseExpiryIdlesEmptyPocket", "ConsumeAckedRecord",
    "DeadLetterNackedRecord",
  ]
  assert expiry_rule(state) == Some("LeaseExpiryIdlesEmptyPocket")
  assert lapsed(state)
    == Driven(
      Ok(Applied(
        State(
          ..state,
          pockets: [
            other,
            Pocket(..pk, status: PocketIdle, consecutive_expiries: 1),
            done,
          ],
          records: [their_a, their_n],
          leases: [
            lease_0,
            lease_2,
            Lease(..lease_1, status: LeaseExpired(now)),
          ],
          dead_letters: list.append(state.dead_letters, [
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
      clock_reads: 0,
      lease_id_draws: 0,
    )
  fired
}

/// The three requirements the expiry rules share, each shown failing alone
/// over `lapsing`, where `lease-1`'s lapse otherwise keeps `pk` at the
/// front: the event's `now` a millisecond before `expires_at` (and at it, a
/// rule holds); the lease released or expired; `pk` queued, idle or parked;
/// and an id no lease has. A tick never offers these, so they are reached
/// only by hand (N37). The table answers that nothing happens.
fn lapse_guards() -> List(String) {
  let #(their_a, their_n) = theirs()
  let u = Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued)
  let state = lapsing(1, [their_a, held(u, Unsettled, None), their_n])
  let assert Ok(lease_1) = model.lease(state.leases, "lease-1")
  let expires_at = lease_1.expires_at
  assert expiry_rule_at(state, "lease-1", expires_at - 1) == None
  assert expiry_rule_at(state, "lease-1", expires_at) == Some(keeps)
  assert core.run_at(
      expiry.lease_lapsed(),
      Lapse("lease-1", expires_at - 1),
      state,
      expires_at - 1,
    )
    == Done(Ok(Applied(state, [], None)))
  list.each([LeaseReleased(700), LeaseExpired(700)], fn(status) {
    let ended =
      State(
        ..state,
        leases: model.replace_lease(state.leases, Lease(..lease_1, status:)),
      )
    assert expiry_rule(ended) == None
  })
  list.each([PocketQueued, PocketIdle, PocketParked], fn(status) {
    let unleased =
      State(
        ..state,
        pockets: list.map(state.pockets, fn(pocket) {
          case pocket.key == "pk" {
            True -> Pocket(..pocket, status:)
            False -> pocket
          }
        }),
      )
    assert expiry_rule(unleased) == None
  })
  assert expiry_rule_at(state, "nowhere", due_at()) == None
  []
}

/// Step 5's choice through the real tables, on a state built by hand. Both
/// `lease-1` and `lease-2` are due, `lease-1` first, and so is one
/// superseded record. `other`, which `lease-2` holds, has already seen
/// `max_consecutive_expiries` expiries, a state no tick leaves, so its
/// lapse parks it with a count above the limit and the checker fails that
/// outcome with `ExpiriesWithinLimit`. The tick keeps `lease-1`'s lapse,
/// reports it, stops there, and runs no decay: the due entry is still in
/// its queue.
fn tick_stops_at_a_failed_lapse() -> List(String) {
  let u = Record("pk", "u", 44, <<"you":utf8>>, 6, RecordQueued)
  let o = Record("other", "o", 41, <<"oh":utf8>>, 10, RecordQueued)
  let base = lapsing(1, [held(o, Unsettled, None), held(u, Unsettled, None)])
  let now = 700 + config.defaults().lease_ttl + 1
  let due =
    SupersededRecord(
      pocket: "pk",
      key: "u",
      rank: 30,
      payload: <<"due":utf8>>,
      reason: Stale,
      shed_at: now - 9,
      expires_at: now,
    )
  let state =
    State(
      ..base,
      pockets: list.map(base.pockets, fn(pocket) {
        case pocket.key == "other" {
          True -> Pocket(..pocket, consecutive_expiries: 3)
          False -> pocket
        }
      }),
      superseded: [due, ..base.superseded],
    )
  assert invariants.check(inspection.snapshot(state)) == []
  let assert Driven(Ok(first), clock_reads: 0, lease_id_draws: 0) =
    harness.drive(
      core.run_at(expiry.lease_lapses(), Lapse("lease-1", now), state, now),
      now,
      "unused",
    )
  let assert Ok(lapsed) = model.lease(first.state.leases, "lease-1")
  assert lapsed.status == LeaseExpired(now)
  let ticked = core.tick(state, now)
  assert ticked
    == core.Ticked(
      state: first.state,
      report: TickReport(["LeaseLapses", keeps, requeue], [lapsed], [], []),
      failure: Some(failure.BrokenInvariant(["ExpiriesWithinLimit"])),
    )
  assert list.contains(ticked.state.superseded, due)
  first.fired
}

/// Publisher `p` joined, and the counter stands at 9. `parked` is parked at
/// position 0 with its count 3 and one queued record, `r`. `pk` is queued
/// with a record. The superseded queue holds a version of `pk`'s record, and
/// the dead-letter queue one of `parked`'s.
fn with_parked() -> State {
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 9),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1")],
    pockets: [
      Pocket("pk", PocketQueued, 6, 0),
      Pocket("parked", PocketParked, 0, 3),
    ],
    records: [
      Record("pk", "a", 5, <<"x":utf8>>, 7, RecordQueued),
      Record("parked", "r", 2, <<"r":utf8>>, 4, RecordQueued),
    ],
    superseded: [
      SupersededRecord(
        pocket: "pk",
        key: "a",
        rank: 4,
        payload: <<"older":utf8>>,
        reason: Superseded,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: "parked",
        key: "gone",
        rank: 3,
        payload: <<"dead":utf8>>,
        nacked_by: "c1",
        nacked_at: 350,
        expires_at: 400 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// `PocketTouched` for the parked pocket, as its own outcome: it is queued
/// at the counter's value, 9, with its count 0, and the counter is 10.
/// Nothing else changes. Then a publish of a new key into it, through the
/// apply step: the record takes 9 and the pocket 10, behind `pk`, and the
/// counter is 11. Neither reads a clock or draws a lease id. A queued
/// pocket touched is left alone.
fn revive_parked_exactly() -> List(String) {
  let state = with_parked()
  let assert [pk, parked] = state.pockets
  let revive = "ReviveParkedPocket"
  assert harness.drive(
      core.run(publish.pocket_touched(), "parked", state),
      harness.t0,
      "unused",
    )
    == Driven(
      Ok(Applied(
        State(..state, queue: Queue(next_position: 10), pockets: [
          pk,
          Pocket(
            ..parked,
            status: PocketQueued,
            position: 9,
            consecutive_expiries: 0,
          ),
        ]),
        [revive],
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  let fired = ["PublishNewKey", revive]
  assert harness.drive(
      core.apply(
        state,
        core.Publish(Publication("p", "parked", "k", 1, <<"v":utf8>>)),
      ),
      harness.t0,
      "unused",
    )
    == Driven(
      Ok(Applied(
        State(
          ..state,
          queue: Queue(next_position: 11),
          pockets: [
            pk,
            Pocket(
              ..parked,
              status: PocketQueued,
              position: 10,
              consecutive_expiries: 0,
            ),
          ],
          records: list.append(state.records, [
            Record("parked", "k", 1, <<"v":utf8>>, 9, RecordQueued),
          ]),
        ),
        fired,
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  assert core.run(publish.pocket_touched(), "pk", state)
    == Done(Ok(Applied(state, [], None)))
  [revive, ..fired]
}
