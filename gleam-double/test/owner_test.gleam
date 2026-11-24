//// The owner actor: one mailbox, one state, and a failure that changes
//// nothing. None of these tests pins a catalog row.

import gleam/erlang/process
import gleam/list
import gleam/option.{None}
import pachinko_double.{Outcome, PartialTick}
import pachinko_double/answers.{TickReport}
import pachinko_double/config.{Settings}
import pachinko_double/failure.{BrokenInvariant, Refused}
import pachinko_double/ids.{
  ConsumerId, LeaseId, PocketKey, PublisherId, RecordKey,
}
import pachinko_double/inspection.{type Snapshot, Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{
  LeaseActive, LeaseExpired, PocketLeased, PocketQueued, Publisher,
}
import pachinko_double/refusals.{PublisherAlreadyJoined}
import support/harness
import support/runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a refusal is answered, changes nothing and leaves the owner serving",
      [],
      refusal_keeps_state,
    ),
    Test(
      "the same join from several processes succeeds exactly once",
      [],
      competing_joins,
    ),
    Test(
      "a tick whose outcome fails is answered PartialTick, and the owner keeps what the outcomes before it did",
      [],
      partial_tick_keeps_what_ran,
    ),
    Test(
      "a clock set back after a nack leaves the lease's end before nacked_at, so the lapse and a release fail NackedBeforeExpiry",
      [],
      clock_set_back_after_a_nack,
    ),
  ]
}

fn refusal_keeps_state() -> List(String) {
  let double = harness.start(config.default())
  assert harness.publisher_joins(double, PublisherId("q"))
    == Ok(Outcome(["PublisherJoins"], None))
  let before = harness.inspect(double)
  assert before
    == Snapshot(..inspection.empty(config.defaults()), publishers: [
      Publisher(PublisherId("q")),
    ])
  assert harness.publisher_joins(double, PublisherId("q"))
    == Error(Refused(PublisherAlreadyJoined(PublisherId("q"))))
  assert harness.inspect(double) == before
  assert harness.publisher_joins(double, PublisherId("r"))
    == Ok(Outcome(["PublisherJoins"], None))
  assert harness.inspect(double)
    == Snapshot(..inspection.empty(config.defaults()), publishers: [
      Publisher(PublisherId("q")),
      Publisher(PublisherId("r")),
    ])
  assert harness.clock_reads(double.clock) == 0
  assert harness.lease_id_draws(double.lease_ids) == 0
  harness.fired(double)
}

/// Eight processes send the same join. The mailbox orders them, in an order
/// the test does not fix: whichever is first joins, and every other is
/// refused.
fn competing_joins() -> List(String) {
  let double = harness.start(config.default())
  let callers = 8
  let replies = process.new_subject()
  list.each(list.repeat(Nil, callers), fn(_) {
    process.spawn(fn() {
      process.send(replies, harness.publisher_joins(double, PublisherId("q")))
    })
  })
  let answers =
    list.map(list.repeat(Nil, callers), fn(_) {
      let assert Ok(reply) =
        process.receive(replies, pachinko_double.call_timeout)
      reply
    })
  let joined = Ok(Outcome(["PublisherJoins"], None))
  let refused = Error(Refused(PublisherAlreadyJoined(PublisherId("q"))))
  assert list.count(answers, fn(answer) { answer == joined }) == 1
  assert list.count(answers, fn(answer) { answer == refused }) == callers - 1
  assert harness.inspect(double)
    == Snapshot(..inspection.empty(config.defaults()), publishers: [
      Publisher(PublisherId("q")),
    ])
  harness.fired(double)
}

/// One way a tick through the owner can fail with the specification's
/// tables: a lease-id source that hands out an id twice. Its third draw
/// repeats its second, so pockets `b` and `c` are leased under two leases
/// both named `lease-2`. A tick that lapses `lease-1` and then the first
/// `lease-2` ends both leases of that id, which leaves `c` leased with no
/// active lease, and the checker fails that second outcome. The owner
/// answers `PartialTick` with what the first outcome did, keeps the state it
/// left, and reads the clock once.
fn partial_tick_keeps_what_ran() -> List(String) {
  let clock = harness.start_clock()
  let ids = harness.start_lease_ids()
  let draw = harness.lease_ids(ids)
  let lease_id = fn() {
    case draw() {
      LeaseId("lease-3") -> LeaseId("lease-2")
      id -> id
    }
  }
  let assert Ok(config) =
    config.new(Settings(..config.defaults(), lease_ttl: 10))
  let assert Ok(started) =
    pachinko_double.start(config, harness.clock(clock), lease_id)
  let queue = started.data
  let assert Ok(_) = pachinko_double.publisher_joins(queue, PublisherId("p"))
  list.each(["c1", "c2", "c3"], fn(consumer) {
    let assert Ok(_) =
      pachinko_double.consumer_joins(queue, ConsumerId(consumer))
    Nil
  })
  list.each(["a", "b", "c"], fn(key) {
    let assert Ok(_) =
      pachinko_double.publish(
        queue,
        PublisherId("p"),
        PocketKey(key),
        RecordKey(key <> "1"),
        1,
        <<
          key:utf8,
        >>,
      )
    Nil
  })
  let assert Ok(_) = pachinko_double.pull(queue, ConsumerId("c1"), 1)
  harness.set_time(clock, 5)
  let assert Ok(_) = pachinko_double.pull(queue, ConsumerId("c2"), 1)
  let assert Ok(_) = pachinko_double.pull(queue, ConsumerId("c3"), 1)
  let before = pachinko_double.inspect(queue)
  let twins = fn(snapshot: Snapshot) {
    list.filter(snapshot.leases, fn(lease) { lease.id == LeaseId("lease-2") })
  }
  assert list.map(twins(before), fn(lease) { #(lease.pocket, lease.status) })
    == [#(PocketKey("b"), LeaseActive), #(PocketKey("c"), LeaseActive)]
  harness.set_time(clock, 15)
  let reads = harness.clock_reads(clock)
  let assert Error(PartialTick(report, BrokenInvariant(broken))) =
    pachinko_double.tick(queue)
  assert harness.clock_reads(clock) == reads + 1
  assert list.contains(broken, "LeasedHasOneActiveLease")
  let after = pachinko_double.inspect(queue)
  let assert Ok(first) = model.lease(after.leases, LeaseId("lease-1"))
  assert first.status == LeaseExpired(ended_at: 15)
  assert report.lapsed == [first]
  assert report.fired
    == [
      "LeaseLapses",
      "LeaseExpiryKeepsPocketAtFront",
      "RequeueUnsettledRecord",
    ]
  assert report.superseded == [] && report.dead_letters == []
  // The second outcome left nothing: both leases named `lease-2` stand as
  // they were, and their pockets are still leased.
  assert twins(after) == twins(before)
  assert list.map(after.pockets, fn(pocket) { #(pocket.key, pocket.status) })
    == [
      #(PocketKey("a"), PocketQueued),
      #(PocketKey("b"), PocketLeased),
      #(PocketKey("c"), PocketLeased),
    ]
  // A second tick meets the same failure and changes nothing more.
  let assert Error(PartialTick(again, BrokenInvariant(_))) =
    pachinko_double.tick(queue)
  assert again.lapsed == []
  assert pachinko_double.inspect(queue) == after
  []
}

/// Another way, with unique lease ids: a clock that goes back. The double
/// does not clamp the injected clock (G57). `c1` nacks `k` at 100 and then,
/// with the clock set back, renews at 1, so the lease expires at 11. Ended
/// at 11, the lease's `Freed` makes a dead letter with `nacked_at` 100 and
/// `expires_at` 11 + `dead_letter_ttl`, which is 11, as
/// `DeadLetterNackedRecord` states, and `NackedBeforeExpiry` is broken. So
/// the due lapse fails, and so does a release, and the owner keeps the state
/// it had. This pins what the double does; whether the specification means
/// the clock never to go back is the maintainer's question (G59).
fn clock_set_back_after_a_nack() -> List(String) {
  let assert Ok(config) =
    config.new(Settings(..config.defaults(), lease_ttl: 10, dead_letter_ttl: 0))
  let double = harness.start(config)
  let assert Ok(_) = harness.publisher_joins(double, PublisherId("p"))
  let assert Ok(_) = harness.consumer_joins(double, ConsumerId("c1"))
  let assert Ok(_) =
    harness.publish(double, PocketKey("pk"), RecordKey("k"), 1, "v1")
  let lease = harness.batch(double, ConsumerId("c1"), 1).lease
  harness.set_time(double.clock, 100)
  assert harness.nack(double, ConsumerId("c1"), lease, RecordKey("k"))
    == Ok(Outcome(["NackMarksRecord"], None))
  harness.set_time(double.clock, 1)
  assert harness.renew(double, ConsumerId("c1"), lease)
    == Ok(Outcome(["Renew"], None))
  harness.set_time(double.clock, 11)
  let before = harness.inspect(double)
  assert invariants.check(before) == []
  let assert Error(PartialTick(report, BrokenInvariant(broken))) =
    harness.tick(double)
  assert broken == ["NackedBeforeExpiry"]
  assert report == TickReport([], [], [], [])
  assert harness.inspect(double) == before
  assert harness.release(double, ConsumerId("c1"), lease)
    == Error(BrokenInvariant(["NackedBeforeExpiry"]))
  assert harness.inspect(double) == before
  harness.fired(double)
}
