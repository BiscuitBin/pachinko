//// The specification's "Decay" section: `SupersededRecordDecays` and
//// `DeadLetterDecays`.
////
//// First the three catalog rows of
//// `crates/libpachinko/tests/queue/decay.rs`, S007, S073 and S074, each a
//// port of its Rust test, driving the owner through `harness`. A port
//// asserts what its Rust test asserts, with these changes:
////
//// - The Rust's tick answers counts of the entries it decayed; a port
////   asserts the double's whole `TickReport`, which lists the entries as
////   they stood (N33). The Rust's publish, nack and release answers are
////   ported as the rules each fired (the rulings on N24 and N32).
//// - `impl-choice` (spec note G38, as N25 and N29): where one end frees
////   records of different marks, or creates two entries, the order of the
////   `Freed` rules in the fired names and of the entries, smallest held
////   position first.
////
//// Then the double's own tests, which pin no catalog row. Each rule runs alone over a state built by hand, through `core.run_at`,
//// with the tick's `now` in hand: what it changes, compared with the whole
//// state, the rules fired, and the clock reads and lease-id draws, which are
//// none. Then two equal entries: one decay removes one of them, the first,
//// and keeps the others in order; and through the owner, two identical
//// stale versions both decay in one tick, and the report names both.

import gleam/option.{None}
import harness.{type Double, Driven}
import pachinko_double/config.{Settings}
import pachinko_double/core.{type TickReport, TickReport}
import pachinko_double/decay
import pachinko_double/invariants
import pachinko_double/model.{
  type DeadLetter, type State, type SupersededRecord, Consumer, DeadLetter,
  Pocket, PocketQueued, Publisher, Queue, Record, RecordQueued, Stale, State,
  Superseded, SupersededRecord,
}
import pachinko_double/owner.{Outcome}
import pachinko_double/rules.{Applied, Done}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "zero superseded and dead-letter TTLs decay at once, the dead letter at its lease's end",
      [Scenario("S007")],
      zero_ttls_decay_at_once,
    ),
    Test(
      "superseded entries decay at shed_at plus superseded_ttl",
      [Scenario("S073")],
      superseded_decay_at_ttl,
    ),
    Test(
      "dead letters decay at the lease's end plus dead_letter_ttl",
      [Scenario("S074")],
      dead_letters_decay_at_ttl,
    ),
    Test(
      "SupersededRecordDecays removes exactly the entry it is given and reads no clock",
      [],
      superseded_decays_exactly,
    ),
    Test(
      "DeadLetterDecays removes exactly the entry it is given and reads no clock",
      [],
      dead_letter_decays_exactly,
    ),
    Test(
      "a decay of one of two equal entries removes the first and keeps the other",
      [],
      one_of_two_equal,
    ),
    Test(
      "two identical stale versions both decay in one tick, and the report names both",
      [],
      two_identical_decay_together,
    ),
    Test(
      "two distinct superseded entries due in one tick decay in queue order",
      [],
      superseded_decay_in_queue_order,
    ),
  ]
}

fn fired(rules: List(String)) -> owner.Reply {
  Ok(Outcome(rules, None))
}

/// The Rust fixture's `tick`: one tick, which must not fail, after which
/// every invariant holds.
fn tick(double: Double) -> TickReport {
  let assert Ok(report) = harness.tick(double)
  assert invariants.check(harness.inspect(double)) == []
  report
}

const nothing = TickReport([], [], [], [])

// The catalog rows ------------------------------------------

/// Both TTLs are zero. `old` is shed at `t0` by the supersede, and the
/// nacked `a2` becomes a dead letter only at the release, both due at
/// once; one tick decays both.
fn zero_ttls_decay_at_once() -> List(String) {
  let double =
    harness.fixture_with(
      Settings(
        ..harness.fixture_settings(),
        superseded_ttl: 0,
        dead_letter_ttl: 0,
      ),
    )
  assert harness.publish(double, "a", "a1", 1, "old")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "a", "a2", 1, "a2") == fired(["PublishNewKey"])
  assert harness.publish(double, "a", "a1", 2, "new") == fired(["Supersede"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord", "HoldRecord"]
  assert harness.nack(double, "c1", batch.lease, "a2")
    == fired(["NackMarksRecord"])
  // Not before the end.
  assert harness.inspect(double).dead_letters == []
  assert harness.release(double, "c1", batch.lease)
    == fired([
      "ReleaseRequeuesPocket", "RequeueUnsettledRecord",
      "DeadLetterNackedRecord",
    ])
  let snapshot = harness.inspect(double)
  let assert [shed] = snapshot.superseded
  let assert [letter] = snapshot.dead_letters
  assert shed.expires_at == harness.t0
  assert letter.expires_at == harness.t0
  // `impl-choice` (spec note G57): the superseded record decays before the
  // dead letter. The specification gives no order among decays due at once.
  assert tick(double)
    == TickReport(["SupersededRecordDecays", "DeadLetterDecays"], [], [shed], [
      letter,
    ])
  let snapshot = harness.inspect(double)
  assert snapshot.superseded == []
  assert snapshot.dead_letters == []
  harness.fired(double)
}

/// The stale `a1` is shed at 3 ms and the kept one at 4 ms, each due
/// `superseded_ttl` later, not before, and each decays once.
fn superseded_decay_at_ttl() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 2, "kept")
    == fired(["PublishIntoNewPocket"])
  let stale_at = harness.at(double, 3)
  assert harness.publish(double, "a", "a1", 1, "stale")
    == fired(["DropStalePublish"])
  let _ = harness.at(double, 4)
  assert harness.publish(double, "a", "a1", 3, "newer") == fired(["Supersede"])
  // `impl-choice` (spec note G19): the superseded queue in the order its
  // entries were created, the stale `a1` first. The specification gives that
  // queue no order.
  let assert [stale, kept] = harness.inspect(double).superseded
  assert stale.reason == Stale
  assert stale.shed_at == stale_at
  let Settings(superseded_ttl: ttl, ..) = harness.fixture_settings()
  let _ = harness.at(double, 3 + ttl - 1)
  assert tick(double) == nothing
  assert harness.inspect(double).superseded == [stale, kept]
  let _ = harness.at(double, 3 + ttl)
  assert tick(double) == TickReport(["SupersededRecordDecays"], [], [stale], [])
  let left = harness.inspect(double).superseded
  assert left == [kept]
  assert kept.reason == Superseded
  let _ = harness.at(double, 4 + ttl)
  assert tick(double) == TickReport(["SupersededRecordDecays"], [], [kept], [])
  // A decay fires once.
  assert tick(double) == nothing
  assert harness.inspect(double).superseded == []
  harness.fired(double)
}

/// The setup of S073's port, with both entries left until both are
/// overdue: one tick at `4 + superseded_ttl` decays the stale `a1`, shed
/// first, then the kept one, and reports them in that order. `impl-choice`
/// (spec note G57, as N34): the specification gives no order among
/// several decays due at once.
fn superseded_decay_in_queue_order() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 2, "kept")
    == fired(["PublishIntoNewPocket"])
  let _ = harness.at(double, 3)
  assert harness.publish(double, "a", "a1", 1, "stale")
    == fired(["DropStalePublish"])
  let _ = harness.at(double, 4)
  assert harness.publish(double, "a", "a1", 3, "newer") == fired(["Supersede"])
  let assert [stale, kept] = harness.inspect(double).superseded
  assert stale != kept
  let Settings(superseded_ttl: ttl, ..) = harness.fixture_settings()
  let _ = harness.at(double, 4 + ttl)
  assert tick(double)
    == TickReport(
      ["SupersededRecordDecays", "SupersededRecordDecays"],
      [],
      [stale, kept],
      [],
    )
  assert harness.inspect(double).superseded == []
  harness.fired(double)
}

/// Each dead letter keeps its nack's time and expires `dead_letter_ttl`
/// after the lease's end, not after its nack.
fn dead_letters_decay_at_ttl() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 1, "a1")
    == fired(["PublishIntoNewPocket"])
  assert harness.publish(double, "a", "a2", 1, "a2") == fired(["PublishNewKey"])
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord", "HoldRecord"]
  let two = harness.at(double, 2)
  assert harness.nack(double, "c1", batch.lease, "a1")
    == fired(["NackMarksRecord"])
  let three = harness.at(double, 3)
  assert harness.nack(double, "c1", batch.lease, "a2")
    == fired(["NackMarksRecord"])
  // Not before the end.
  assert harness.inspect(double).dead_letters == []
  let ended = harness.at(double, 10)
  assert harness.release(double, "c1", batch.lease)
    == fired([
      "ReleaseIdlesEmptyPocket", "DeadLetterNackedRecord",
      "DeadLetterNackedRecord",
    ])
  let Settings(dead_letter_ttl: ttl, ..) = harness.fixture_settings()
  let letter = fn(key, nacked_at) {
    DeadLetter(
      pocket: "a",
      key:,
      rank: 1,
      payload: <<key:utf8>>,
      nacked_by: "c1",
      nacked_at:,
      expires_at: ended + ttl,
    )
  }
  let letters = [letter("a1", two), letter("a2", three)]
  assert harness.inspect(double).dead_letters == letters
  let _ = harness.at(double, 3 + ttl)
  // Not at `nacked_at` plus the TTL.
  assert tick(double) == nothing
  let _ = harness.at(double, 10 + ttl - 1)
  assert tick(double) == nothing
  let _ = harness.at(double, 10 + ttl)
  assert tick(double)
    == TickReport(["DeadLetterDecays", "DeadLetterDecays"], [], [], letters)
  // A decay fires once.
  assert tick(double) == nothing
  assert harness.inspect(double).dead_letters == []
  harness.fired(double)
}

// The double's own tests ------------------------------------

/// The tick's `now` in these tests.
const now = 50_000

fn shed(key: String, expires_at: Int) -> SupersededRecord {
  SupersededRecord(
    pocket: "pk",
    key:,
    rank: 3,
    payload: <<key:utf8>>,
    reason: Superseded,
    shed_at: expires_at - 100,
    expires_at:,
  )
}

fn letter(key: String, expires_at: Int) -> DeadLetter {
  DeadLetter(
    pocket: "other",
    key:,
    rank: 4,
    payload: <<key:utf8>>,
    nacked_by: "c1",
    nacked_at: expires_at - 200,
    expires_at:,
  )
}

/// Two queued pockets with a record each, and both shed queues holding
/// these entries.
fn shedding(
  superseded: List(SupersededRecord),
  dead_letters: List(DeadLetter),
) -> State {
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 8),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1")],
    pockets: [
      Pocket("pk", PocketQueued, 5, 0),
      Pocket("other", PocketQueued, 2, 1),
    ],
    records: [
      Record("pk", "a", 5, <<"x":utf8>>, 7, RecordQueued),
      Record("other", "b", 5, <<"y":utf8>>, 3, RecordQueued),
    ],
    superseded:,
    dead_letters:,
  )
}

fn decayed(step) -> harness.Driven {
  harness.drive(step, now + 1, "unused")
}

/// Of three superseded records, `m`, due, is offered: it alone is removed,
/// and `e` before it and `l` after it, neither yet decayed, keep their
/// order. Neither dead letter nor any other part of the state changes. An
/// entry that is not in the queue meets no guard, and nothing happens.
fn superseded_decays_exactly() -> List(String) {
  let e = shed("e", now - 5)
  let m = shed("m", now)
  let l = shed("l", now + 5)
  let letters = [letter("d", now), letter("f", now + 9)]
  let state = shedding([e, m, l], letters)
  let fired = ["SupersededRecordDecays"]
  assert decayed(core.run_at(decay.superseded_record_decays(), m, state, now))
    == Driven(
      Ok(Applied(State(..state, superseded: [e, l]), fired, None)),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  assert core.run_at(
      decay.superseded_record_decays(),
      shed("gone", now),
      state,
      now,
    )
    == Done(Ok(Applied(state, [], None)))
  fired
}

/// The same for the dead-letter queue: `m` alone is removed, and the
/// superseded records are untouched.
fn dead_letter_decays_exactly() -> List(String) {
  let e = letter("e", now - 5)
  let m = letter("m", now)
  let l = letter("l", now + 5)
  let superseded = [shed("s", now), shed("t", now + 9)]
  let state = shedding(superseded, [e, m, l])
  let fired = ["DeadLetterDecays"]
  assert decayed(core.run_at(decay.dead_letter_decays(), m, state, now))
    == Driven(
      Ok(Applied(State(..state, dead_letters: [e, l]), fired, None)),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  assert core.run_at(
      decay.dead_letter_decays(),
      letter("gone", now),
      state,
      now,
    )
    == Done(Ok(Applied(state, [], None)))
  fired
}

/// Each queue holds `x`, then `y`, then an entry equal to `x`. One decay of
/// `x` removes the first and leaves `y` and the second `x`, in that order:
/// a rule that removed every equal entry would leave `y` alone, and one that
/// removed the last would leave `x` before `y`.
/// `impl-choice` (spec note G64): that one occurrence goes is the rule's
/// `not exists shed`; that it is the first, which the order `[y, x]` shows,
/// is the double's, since the specification cannot tell equal entries
/// apart.
fn one_of_two_equal() -> List(String) {
  let x = shed("x", now)
  let y = shed("y", now + 5)
  let state = shedding([x, y, x], [])
  assert decayed(core.run_at(decay.superseded_record_decays(), x, state, now))
    == Driven(
      Ok(Applied(
        State(..state, superseded: [y, x]),
        ["SupersededRecordDecays"],
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  let dx = letter("x", now)
  let dy = letter("y", now + 5)
  let state = shedding([], [dx, dy, dx])
  assert decayed(core.run_at(decay.dead_letter_decays(), dx, state, now))
    == Driven(
      Ok(Applied(
        State(..state, dead_letters: [dy, dx]),
        ["DeadLetterDecays"],
        None,
      )),
      clock_reads: 0,
      lease_id_draws: 0,
    )
  ["SupersededRecordDecays", "DeadLetterDecays"]
}

/// Two stale publishes of `k` at one rank and one instant shed two
/// identical superseded records. Once `superseded_ttl` has passed, one tick
/// decays both, each its own outcome, and reports both; the queue is empty.
fn two_identical_decay_together() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 5, "v")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  assert harness.publish(double, "pk", "k", 1, "old")
    == Ok(Outcome(["DropStalePublish"], None))
  assert harness.publish(double, "pk", "k", 1, "old")
    == Ok(Outcome(["DropStalePublish"], None))
  let assert [first, second] = harness.inspect(double).superseded
  assert first == second
  assert first.reason == Stale
  let Settings(superseded_ttl:, ..) = harness.fixture_settings()
  let _ = harness.at(double, superseded_ttl)
  assert harness.tick(double)
    == Ok(
      TickReport(
        ["SupersededRecordDecays", "SupersededRecordDecays"],
        [],
        [first, second],
        [],
      ),
    )
  let snapshot = harness.inspect(double)
  assert snapshot.superseded == []
  assert invariants.check(snapshot) == []
  harness.fired(double)
}
