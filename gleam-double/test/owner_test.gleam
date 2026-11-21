//// The owner actor: one mailbox, one state, and a failure that changes
//// nothing. None of these tests pins a catalog row.

import gleam/erlang/process
import gleam/list
import gleam/option.{None}
import harness
import pachinko_double/config
import pachinko_double/inspection.{Snapshot}
import pachinko_double/model.{Publisher}
import pachinko_double/owner.{Outcome}
import pachinko_double/refusals.{PublisherAlreadyJoined}
import pachinko_double/rules.{Refused}
import runner.{type Test, Test}

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
  ]
}

fn refusal_keeps_state() -> List(String) {
  let double = harness.start(config.default())
  assert harness.publisher_joins(double, "q")
    == Ok(Outcome(["PublisherJoins"], None))
  let before = harness.inspect(double)
  assert before == Snapshot(0, [Publisher("q")], [])
  assert harness.publisher_joins(double, "q")
    == Error(Refused(PublisherAlreadyJoined("q")))
  assert harness.inspect(double) == before
  assert harness.publisher_joins(double, "r")
    == Ok(Outcome(["PublisherJoins"], None))
  assert harness.inspect(double)
    == Snapshot(0, [Publisher("q"), Publisher("r")], [])
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
      process.send(replies, harness.publisher_joins(double, "q"))
    })
  })
  let answers =
    list.map(list.repeat(Nil, callers), fn(_) {
      let assert Ok(reply) = process.receive(replies, owner.call_timeout)
      reply
    })
  let joined = Ok(Outcome(["PublisherJoins"], None))
  let refused = Error(Refused(PublisherAlreadyJoined("q")))
  assert list.count(answers, fn(answer) { answer == joined }) == 1
  assert list.count(answers, fn(answer) { answer == refused }) == callers - 1
  assert harness.inspect(double) == Snapshot(0, [Publisher("q")], [])
  harness.fired(double)
}
