//// The competing callers: several consumers pull through one owner at the
//// same moment. Each caller is a process of its own, so the owner's mailbox
//// orders their pulls, in an order no test fixes. A test here asserts only
//// what holds in every such order: how many callers are handed a batch,
//// that no two of them share a pocket, a lease or a consumer, that the
//// pockets are claimed front first, and that each batch is its own pocket's
//// oldest records. It never asserts which consumer won, which pocket a
//// consumer got, or the number in a lease id. None of these tests pins a
//// catalog row.
////
//// Every assertion runs in the test's process, which waits for one reply
//// from each caller it starts. A caller is linked to it, so a caller that
//// crashes takes the test down, and one that never replies fails it by
//// timeout.
////
//// The scheduler mostly lets the callers reach the mailbox in the order
//// they were started, so one starting order would show little more than one
//// mailbox order. Each test therefore runs once for every caller, with that
//// caller started first, and every run must pass.

import gleam/bit_array
import gleam/erlang/process
import gleam/int
import gleam/list
import gleam/option.{Some}
import gleam/string
import harness.{type Double}
import pachinko_double/domain.{
  type BatchRecord, Batch, BatchRecord, NoPocketAvailable,
}
import pachinko_double/inspection.{type Snapshot}
import pachinko_double/invariants
import pachinko_double/model.{Lease, LeaseActive}
import pachinko_double/owner.{type Reply, Outcome}
import runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "eight consumers pull at once while one pocket is queued: one is handed its records under a lease and seven are told there is none",
      [],
      one_pocket,
    ),
    Test(
      "six consumers pull at once while six pockets are queued: each is handed a different pocket, and the pockets are claimed front first",
      [],
      a_pocket_each,
    ),
    Test(
      "nine consumers pull at once while four pockets are queued: four are handed a pocket each and five are told there is none",
      [],
      more_consumers_than_pockets,
    ),
  ]
}

const claims = "PullClaimsFrontPocket"

const finds_none = "PullFindsNoPocket"

const hold = "HoldRecord"

/// One pocket of three records, and a limit above that count: whichever
/// consumer the mailbox puts first is handed all three, oldest first.
fn one_pocket() -> List(String) {
  every_turn(consumers(8), compete(_, ["only"], 3, 10))
}

/// A pocket for each consumer, two records each.
fn a_pocket_each() -> List(String) {
  every_turn(consumers(6), compete(
    _,
    ["north", "east", "south", "west", "up", "down"],
    2,
    10,
  ))
}

/// Fewer pockets than consumers, and a limit below a pocket's three
/// records: each batch is its pocket's two oldest.
fn more_consumers_than_pockets() -> List(String) {
  every_turn(consumers(9), compete(_, ["north", "east", "south", "west"], 3, 2))
}

/// The consumer ids `c1` to `c<count>`.
fn consumers(count: Int) -> List(String) {
  int.range(from: count, to: 0, with: [], run: fn(ids, n) {
    ["c" <> int.to_string(n), ..ids]
  })
}

/// Runs a test's body once for each caller, on a new owner each time: the
/// callers in the order given, then turned so that the second is started
/// first and the first last, and so on round. It returns what every run
/// fired.
fn every_turn(
  callers: List(String),
  body: fn(List(String)) -> List(String),
) -> List(String) {
  list.index_map(callers, fn(_, turn) {
    body(list.append(list.drop(callers, turn), list.take(callers, turn)))
  })
  |> list.flatten
}

/// The keys and ranks of the records a test publishes into every pocket, in
/// the order it publishes them. Neither the keys nor the ranks ascend, so a
/// batch ordered by either is not oldest first.
const versions = [#("m", 5), #("z", 9), #("a", 1)]

/// What the test published into a pocket, oldest first, as a pull hands it
/// out. Every pocket holds the same keys at the same ranks, and each payload
/// names its pocket, so a record handed out of another pocket shows.
fn stocked(pocket_key: String, stock: Int) -> List(BatchRecord) {
  list.map(list.take(versions, stock), fn(version) {
    BatchRecord(
      key: version.0,
      rank: version.1,
      payload: bit_array.from_string(pocket_key <> " " <> version.0),
    )
  })
}

/// Publishes `stock` records into each pocket, one record of every pocket
/// before the next of any, so that the pockets enter the pocket order in the
/// order given and one pocket's records do not hold neighbouring positions.
fn publish(double: Double, pocket_keys: List(String), stock: Int) -> Nil {
  list.each(list.take(versions, stock), fn(version) {
    list.each(pocket_keys, fn(pocket_key) {
      let assert Ok(_) =
        harness.publish(
          double,
          pocket_key,
          version.0,
          version.1,
          pocket_key <> " " <> version.0,
        )
    })
  })
}

/// One caller's batch: the consumer that pulled, and the lease, the pocket
/// and the records its answer named.
type Grant {
  Grant(
    consumer: String,
    lease: String,
    pocket_key: String,
    records: List(BatchRecord),
  )
}

/// Starts one process for each caller, in the order given, each of which
/// pulls once and sends back whatever the owner replied, and collects one
/// reply from each. A caller waits at a gate of its own until every caller
/// has been started and has reported its gate; only then is each let go. So
/// all of them are running before any pulls, none waits for another, and
/// their pulls reach the owner's mailbox in whatever order the scheduler
/// gives them.
///
/// A caller reads the log before it sends its reply. The log is a process,
/// and `harness.pull` sends it the names a pull fired from the caller's own
/// process, so the read returns only once the log holds them: when the test
/// has every reply, the log holds every caller's names.
fn race(
  double: Double,
  callers: List(String),
  limit: Int,
) -> List(#(String, Reply)) {
  let replies = process.new_subject()
  let ready = process.new_subject()
  list.each(callers, fn(caller) {
    process.spawn(fn() {
      let gate = process.new_subject()
      process.send(ready, gate)
      process.receive_forever(gate)
      let reply = harness.pull(double, caller, limit)
      let _ = harness.fired(double)
      process.send(replies, #(caller, reply))
    })
  })
  let gates =
    list.map(callers, fn(_) {
      let assert Ok(gate) = process.receive(ready, owner.call_timeout)
      gate
    })
  list.each(gates, process.send(_, Nil))
  let answers =
    list.map(callers, fn(_) {
      let assert Ok(answer) = process.receive(replies, owner.call_timeout)
      answer
    })
  // One reply from each caller, and none besides.
  assert list.sort(list.map(answers, fn(answer) { answer.0 }), string.compare)
    == list.sort(callers, string.compare)
  assert process.receive(replies, 0) == Error(Nil)
  answers
}

/// The batches among the answers. The test fails on an answer that is
/// neither a batch nor `NoPocketAvailable`, on one that names another
/// consumer than its caller, and on one whose pull fired anything but the
/// claim and one `HoldRecord` for each record, or `PullFindsNoPocket` alone.
/// So every answer that is not a batch here is a `NoPocketAvailable`.
fn grants(answers: List(#(String, Reply))) -> List(Grant) {
  list.filter_map(answers, fn(answer) {
    let #(caller, reply) = answer
    case reply {
      Ok(Outcome(fired, Some(Batch(consumer:, lease:, pocket:, records:)))) -> {
        assert consumer == caller
        assert fired == [claims, ..list.map(records, fn(_) { hold })]
        Ok(Grant(consumer:, lease:, pocket_key: pocket, records:))
      }
      _ -> {
        assert reply
          == Ok(Outcome([finds_none], Some(NoPocketAvailable(caller))))
        Error(Nil)
      }
    }
  })
}

/// Whether no value occurs twice.
fn distinct(values: List(a)) -> Bool {
  list.unique(values) == values
}

/// One run of a test's body. The fixture's publisher stocks the pockets, the
/// callers that are not the fixture's consumers join, and then every caller
/// pulls at once with the same limit. The clock stands at `t0` throughout.
fn compete(
  callers: List(String),
  pocket_keys: List(String),
  stock: Int,
  limit: Int,
) -> List(String) {
  let double = harness.fixture()
  list.each(callers, fn(caller) {
    case list.contains(["c1", "c2", "c3"], caller) {
      True -> Nil
      False -> {
        let assert Ok(_) = harness.consumer_joins(double, caller)
        Nil
      }
    }
  })
  publish(double, pocket_keys, stock)
  // The setup, before any pull: every pocket queued, in the order given, no
  // lease, and no lease id drawn.
  assert harness.order(double) == pocket_keys
  assert harness.inspect(double).leases == []
  assert harness.lease_id_draws(double.lease_ids) == 0
  let reads = harness.clock_reads(double.clock)

  let answers = race(double, callers, limit)
  let grants = grants(answers)
  let granted = list.length(grants)
  let handed = list.flat_map(grants, fn(grant) { grant.records })

  // As many batches as there were pockets to claim or consumers to claim
  // them, whichever is fewer. Every other caller was told
  // `NoPocketAvailable`, which `grants` has asserted.
  assert granted == int.min(list.length(callers), list.length(pocket_keys))
  // No two batches name the same pocket, the same lease or the same
  // consumer, and the pockets they name are the front ones: all of them
  // where there was a consumer for each.
  assert distinct(list.map(grants, fn(grant) { grant.pocket_key }))
  assert distinct(list.map(grants, fn(grant) { grant.lease }))
  assert distinct(list.map(grants, fn(grant) { grant.consumer }))
  assert list.sort(
      list.map(grants, fn(grant) { grant.pocket_key }),
      string.compare,
    )
    == list.sort(list.take(pocket_keys, granted), string.compare)
  // Each batch is its own pocket's records, oldest first, up to the limit.
  list.each(grants, fn(grant) {
    assert grant.records == list.take(stocked(grant.pocket_key, stock), limit)
  })

  let after = harness.inspect(double)
  assert invariants.check(after) == []
  leases_match(after, grants, pocket_keys)
  // The pockets that were claimed are leased: none of them is still queued.
  assert harness.order(double) == list.drop(pocket_keys, granted)
  // The held records are the ones the batches handed out, pocket by pocket.
  // `impl-choice` (spec note G19): their order. The snapshot lists records
  // in the order they were created, which within one pocket here is the
  // order of their positions, and so the batch's.
  list.each(grants, fn(grant) {
    assert after.records
      |> model.records_of(grant.pocket_key)
      |> model.held
      |> list.map(fn(record) { record.key })
      == harness.keys(grant.records)
  })
  assert list.length(model.held(after.records)) == list.length(handed)
  // One lease id for each lease granted, and one clock read: a pull that
  // finds no pocket draws no id and reads no clock.
  assert harness.lease_id_draws(double.lease_ids) == granted
  assert harness.clock_reads(double.clock) == reads + granted

  // The log holds what every caller's pull fired, whichever process made
  // it. Its order across callers is the log's mailbox order, not the
  // owner's, so only the counts are asserted.
  let fired = harness.fired(double)
  assert list.count(fired, fn(name) { name == claims }) == granted
  assert list.count(fired, fn(name) { name == finds_none })
    == list.length(callers) - granted
  assert list.count(fired, fn(name) { name == hold }) == list.length(handed)
  fired
}

/// The snapshot's leases are the batches' leases, one for one: each batch's
/// lease is in the snapshot under the id the answer named, active, on the
/// answer's pocket, held by the answer's consumer, granted at `t0` for the
/// fixture's `lease_ttl`; and the snapshot holds no other lease. No two of
/// them share a pocket or a consumer. The snapshot lists leases in the order
/// they were created (spec note G19), and each claim takes the front queued
/// pocket, so that order is the pocket order whichever consumer came first.
fn leases_match(
  after: Snapshot,
  grants: List(Grant),
  pocket_keys: List(String),
) -> Nil {
  list.each(grants, fn(grant) {
    assert model.lease(after.leases, grant.lease)
      == Ok(Lease(
        id: grant.lease,
        pocket: grant.pocket_key,
        consumer: grant.consumer,
        status: LeaseActive,
        granted_at: harness.t0,
        expires_at: harness.t0 + harness.fixture_settings().lease_ttl,
      ))
  })
  assert list.length(after.leases) == list.length(grants)
  assert distinct(list.map(after.leases, fn(lease) { lease.pocket }))
  assert distinct(list.map(after.leases, fn(lease) { lease.consumer }))
  assert list.map(after.leases, fn(lease) { lease.pocket })
    == list.take(pocket_keys, list.length(grants))
}
