//// The specification's Identity section: S009 and S010, each through the
//// owner. A second join is refused and changes nothing (spec note G03). Each
//// port asserts what its Rust test asserts: S009 the publishers in order,
//// S010 that the consumer is there and holds no lease (spec note G06).
////
//// Then S002, a row of the Rust's identity file that `config_test`'s S001
//// stands beside: the defaults an owner starts over. Last, a test of the
//// double's own: a join that succeeds changes nothing but its new entity.

import gleam/list
import gleam/option.{None}
import pachinko_double.{Outcome}
import pachinko_double/config
import pachinko_double/failure.{Refused}
import pachinko_double/inspection.{Snapshot}
import pachinko_double/model.{Consumer, Publisher}
import pachinko_double/refusals.{ConsumerAlreadyJoined, PublisherAlreadyJoined}
import pachinko_double/time
import support/harness
import support/runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test("a publisher joins once", [Scenario("S009")], publisher_joins_once),
    Test("a consumer joins once", [Scenario("S010")], consumer_joins_once),
    Test("a publisher and a consumer may share an id", [], shared_id),
    Test(
      "the default parking threshold and decay TTLs apply",
      [Scenario("S002")],
      default_threshold_and_decay_ttls,
    ),
    Test(
      "a join that succeeds keeps the superseded records and the dead letters, and changes nothing else",
      [],
      a_join_changes_only_its_entity,
    ),
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
  // As the Rust asserts it, `(c4, None)`: `c4` is among the consumers, in no
  // stated order, and holds no lease.
  assert list.contains(after.consumers, Consumer("c4"))
  let assert Ok(c4) =
    list.find(inspection.views(after).consumers, fn(view) {
      view.consumer == Consumer("c4")
    })
  assert c4.active_leases == []
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

/// The Rust builds a queue from the default configuration and reads the
/// three values back from it. Here, as S001's port does, an owner is
/// started over the default configuration, and its snapshot holds them. No
/// rule fires.
fn default_threshold_and_decay_ttls() -> List(String) {
  let double = harness.start(config.default())
  let snapshot = harness.inspect(double)
  assert snapshot.config.max_consecutive_expiries == 3
  assert snapshot.config.superseded_ttl == 12 * time.hour
  assert snapshot.config.dead_letter_ttl == 12 * time.hour
  harness.fired(double)
}

/// The owner is driven to a state that holds something in every part a join
/// could touch: `old`, superseded by a newer `a1`, in the superseded queue,
/// and `a2`, nacked and released, in the dead-letter queue, with an ended
/// lease. A consumer then joins, and a publisher: each changes the complete
/// snapshot only by appending its entity, and reads no clock and draws no
/// lease id.
fn a_join_changes_only_its_entity() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "a", "a1", 1, "old")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  assert harness.publish(double, "a", "a2", 1, "a2")
    == Ok(Outcome(["PublishNewKey"], None))
  assert harness.publish(double, "a", "a1", 2, "new")
    == Ok(Outcome(["Supersede"], None))
  let batch = harness.batch(double, "c1", 10)
  assert harness.nack(double, "c1", batch.lease, "a2")
    == Ok(Outcome(["NackMarksRecord"], None))
  let assert Ok(_) = harness.release(double, "c1", batch.lease)
  let before = harness.inspect(double)
  assert before.superseded != []
  assert before.dead_letters != []
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  assert harness.consumer_joins(double, "c4")
    == Ok(Outcome(["ConsumerJoins"], None))
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      consumers: list.append(before.consumers, [Consumer("c4")]),
    )
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == draws
  let before = harness.inspect(double)
  assert harness.publisher_joins(double, "q")
    == Ok(Outcome(["PublisherJoins"], None))
  assert harness.inspect(double)
    == Snapshot(
      ..before,
      publishers: list.append(before.publishers, [Publisher("q")]),
    )
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}
