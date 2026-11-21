//// The specification's Identity section: S009 and S010, each through the
//// owner. A second join is refused and changes nothing (spec note G03). Each
//// port asserts what its Rust test asserts: S009 the publishers in order,
//// S010 that the consumer is there (spec note G06).

import gleam/list
import gleam/option.{None}
import harness
import pachinko_double/model.{Consumer, Publisher}
import pachinko_double/owner.{Outcome}
import pachinko_double/refusals.{ConsumerAlreadyJoined, PublisherAlreadyJoined}
import pachinko_double/rules.{Refused}
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test("a publisher joins once", [Scenario("S009")], publisher_joins_once),
    Test("a consumer joins once", [Scenario("S010")], consumer_joins_once),
    Test("a publisher and a consumer may share an id", [], shared_id),
  ]
}

fn publisher_joins_once() -> List(String) {
  let double = harness.fixture()
  assert harness.publisher_joins(double, "q")
    == Ok(Outcome(["PublisherJoins"], None))
  let before = harness.inspect(double)
  assert harness.publisher_joins(double, "q")
    == Error(Refused(PublisherAlreadyJoined("q")))
  let after = harness.inspect(double)
  assert after == before
  // `impl-choice`: the Rust asserts this order, and the specification gives
  // the publishers none. It holds because the snapshot lists them in the
  // order they joined (spec note G06).
  assert after.publishers == [Publisher("p"), Publisher("q")]
  // UniquePublisherId, by hand until the checker lands.
  assert list.unique(after.publishers) == after.publishers
  assert harness.clock_reads(double.clock) == 0
  assert harness.lease_id_draws(double.lease_ids) == 0
  harness.fired(double)
}

fn consumer_joins_once() -> List(String) {
  let double = harness.fixture()
  assert harness.consumer_joins(double, "c4")
    == Ok(Outcome(["ConsumerJoins"], None))
  let before = harness.inspect(double)
  assert harness.consumer_joins(double, "c4")
    == Error(Refused(ConsumerAlreadyJoined("c4")))
  let after = harness.inspect(double)
  assert after == before
  // As the Rust asserts it: `c4` is among the consumers, in no stated order.
  assert list.contains(after.consumers, Consumer("c4"))
  // UniqueConsumerId, by hand until the checker lands.
  assert list.unique(after.consumers) == after.consumers
  assert harness.clock_reads(double.clock) == 0
  assert harness.lease_id_draws(double.lease_ids) == 0
  harness.fired(double)
}

/// Each join is answered, and each adds its own entity: an id the other kind
/// holds neither refuses a join nor swallows it.
fn shared_id() -> List(String) {
  let double = harness.fixture()
  let before = harness.inspect(double)
  assert harness.publisher_joins(double, "c1")
    == Ok(Outcome(["PublisherJoins"], None))
  assert harness.consumer_joins(double, "p")
    == Ok(Outcome(["ConsumerJoins"], None))
  let after = harness.inspect(double)
  assert list.contains(after.publishers, Publisher("c1"))
  assert list.contains(after.consumers, Consumer("p"))
  assert list.length(after.publishers) == list.length(before.publishers) + 1
  assert list.length(after.consumers) == list.length(before.consumers) + 1
  harness.fired(double)
}
