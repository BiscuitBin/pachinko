//// The specification's "Renew and release" section, as far as it has
//// landed: `Renew`.
////
//// First the catalog row S050, a port of its Rust test in
//// `crates/libpachinko/tests/queue/lease.rs`, driving the owner. The Rust's
//// renew answers the lease's new `expires_at`; the specification's `Renew`
//// answers nothing, so the port asserts that and reads the expiry from the
//// snapshot's lease (N13 in `double/SPEC-NOTES.md`).
////
//// Then the double's own tests, which pin no catalog row: what `Renew`
//// changes, the refusal of a consumer that never joined, and the rule's
//// guard and effect over states built by hand. What the Renew table answers
//// when the guard does not hold is not pinned here: its catalog rows, S052
//// and S070, belong to the ticket that lands fencing.

import gleam/list
import gleam/option.{None, Some}
import harness.{type Double}
import pachinko_double/config.{Settings}
import pachinko_double/core
import pachinko_double/inspection.{Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  type LeaseStatus, type State, Consumer, DeadLetter, Lease, LeaseActive,
  LeaseExpired, LeaseReleased, Pocket, PocketLeased, PocketQueued, Publisher,
  Queue, Record, RecordQueued, Stale, State, SupersededRecord,
}
import pachinko_double/owner.{type Reply, Outcome}
import pachinko_double/refusals.{UnknownConsumer}
import pachinko_double/rules.{Applied, Refused}
import pachinko_double/settle.{type Renew, Renew}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a renew extends the lease to now plus lease_ttl",
      [Scenario("S050")],
      renew_extends,
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
      "Renew's guard holds only for an active lease of the consumer's",
      [],
      renew_guard,
    ),
    Test(
      "Renew leaves a retained version and a dead letter as they were",
      [],
      renew_keeps_shed_queues,
    ),
  ]
}

/// What a trigger that fired this one rule and answered nothing replies.
fn fired(rule: String) -> Reply {
  Ok(Outcome([rule], None))
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

// The catalog row ------------------------------------------

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

// The guard, over states built by hand ------------------------

/// Consumers `c1` and `c2` joined, and one lease, `lease-1`, of `c1` over
/// pocket `pk`, in this status. The pocket is leased while the lease is
/// active and queued once it has ended. The superseded queue holds a stale
/// version of the pocket's record, and the dead-letter queue a record `c2`
/// once nacked out of it, which no renew through the owner can meet yet.
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
