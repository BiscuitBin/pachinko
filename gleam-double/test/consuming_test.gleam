//// The `Consuming` surface's `exposes` block: the lease view.
////
//// These tests pin no catalog row; S033, S036, S040, S041, S042, S047,
//// S049, S085 and S090 read the view through the owner in `pull_test` and
//// `settle_test`. Here: what the view holds and the order of its records,
//// over a state built by hand; that the read through the owner changes
//// nothing and reads no clock; and the refusal of a consumer that never
//// joined.

import gleam/option.{None, Some}
import pachinko_double.{Outcome}
import pachinko_double/answers.{HeldView, LeaseView}
import pachinko_double/config
import pachinko_double/failure.{Refused}
import pachinko_double/internal/consuming
import pachinko_double/model.{
  type State, Acked, Consumer, Lease, LeaseActive, LeaseExpired, LeaseReleased,
  Nacked, Pocket, PocketLeased, Publisher, Queue, Record, RecordHeld,
  RecordQueued, State, Unsettled,
}
import pachinko_double/refusals.{UnknownConsumer}
import support/harness
import support/runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "the lease view shows the pocket of the consumer's active lease, its expiry, and each held record at its held version, smallest position first",
      [],
      view_exactly,
    ),
    Test(
      "a consumer with no active lease has no lease view",
      [],
      no_active_lease,
    ),
    Test(
      "a lease read through the owner changes nothing and reads no clock",
      [],
      read_changes_nothing,
    ),
    Test(
      "a lease read by a consumer that never joined is refused",
      [],
      unknown_consumer,
    ),
  ]
}

/// The time `c1`'s active lease expires at.
const expires = 4600

/// Consumers `c1` to `c4` joined, and no list is in position order.
///
/// - `c1` holds `pk` under `lease-1`, and once held it under `lease-0`,
///   since released. `pk` holds `a`, acked, at position 8; `q`, queued, at
///   5; and `z`, nacked, at 3. A newer version of `a`, at rank 9, is
///   deferred behind the held one, which is at rank 4: the view shows the
///   held version. No checker reads this state, and no rule runs over it.
/// - `c2` holds `other` under `lease-2`; its one record, `x`, is held at
///   position 2, smaller than any of `pk`'s.
/// - `c3` held `other` under `lease-3`, which expired. `c4` never held a
///   lease.
fn viewed() -> State {
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 20),
    publishers: [Publisher("p")],
    consumers: [Consumer("c1"), Consumer("c2"), Consumer("c3"), Consumer("c4")],
    pockets: [
      Pocket("other", PocketLeased, 9, 0),
      Pocket("pk", PocketLeased, 1, 0),
    ],
    records: [
      Record(
        "pk",
        "a",
        9,
        <<"newer":utf8>>,
        8,
        RecordHeld(
          held_rank: 4,
          held_payload: <<"held":utf8>>,
          mark: Acked,
          nacked_at: None,
          deferred_position: Some(15),
        ),
      ),
      Record(
        "other",
        "x",
        7,
        <<"ex":utf8>>,
        2,
        RecordHeld(
          held_rank: 7,
          held_payload: <<"ex":utf8>>,
          mark: Unsettled,
          nacked_at: None,
          deferred_position: None,
        ),
      ),
      Record("pk", "q", 6, <<"queue":utf8>>, 5, RecordQueued),
      Record(
        "pk",
        "z",
        5,
        <<"zed":utf8>>,
        3,
        RecordHeld(
          held_rank: 5,
          held_payload: <<"zed":utf8>>,
          mark: Nacked,
          nacked_at: Some(700),
          deferred_position: None,
        ),
      ),
    ],
    leases: [
      Lease("lease-0", "pk", "c1", LeaseReleased(150), 100, 400),
      Lease("lease-3", "other", "c3", LeaseExpired(450), 200, 450),
      Lease("lease-2", "other", "c2", LeaseActive, 500, 800),
      Lease("lease-1", "pk", "c1", LeaseActive, 600, expires),
    ],
  )
}

/// `c1`'s view is of `pk`, the pocket of its active lease and not of the
/// one it released, with that lease's `expires_at`. It lists `pk`'s held
/// records and no other: not the queued `q`, and not `other`'s `x`.
///
/// `impl-choice` (spec note G31, as N23): the order, smallest position
/// first, `z` then `a`, though `a` is listed first and sorts first by key.
///
/// Each record shows `held_rank` and `held_payload`, the version the
/// consumer was handed, where a newer one is deferred behind it. `c2`'s
/// view is of `other`.
fn view_exactly() -> List(String) {
  let state = viewed()
  assert consuming.lease_view(state, "c1")
    == Some(
      LeaseView(pocket_key: "pk", expires_at: expires, records: [
        HeldView("z", 5, <<"zed":utf8>>, Nacked),
        HeldView("a", 4, <<"held":utf8>>, Acked),
      ]),
    )
  assert consuming.lease_view(state, "c2")
    == Some(
      LeaseView(pocket_key: "other", expires_at: 800, records: [
        HeldView("x", 7, <<"ex":utf8>>, Unsettled),
      ]),
    )
  []
}

/// `c3`'s only lease has expired and `c4` never held one: neither has a
/// view. Once `c1`'s lease is released it has none either. The pure read
/// answers the same for an id that never joined; the refusal of one is the
/// owner's.
fn no_active_lease() -> List(String) {
  let state = viewed()
  assert consuming.lease_view(state, "c3") == None
  assert consuming.lease_view(state, "c4") == None
  assert consuming.lease_view(state, "cx") == None
  let ended =
    State(..state, leases: [
      Lease("lease-1", "pk", "c1", LeaseReleased(700), 600, expires),
    ])
  assert consuming.lease_view(ended, "c1") == None
  []
}

/// `c1` holds `pk` and `c2` holds nothing. Each reads its lease, twice, with
/// the clock moved on: the view is of the committed state, the complete
/// snapshot is as it was, no clock was read and no lease id drawn.
fn read_changes_nothing() -> List(String) {
  let double = harness.fixture()
  assert harness.publish(double, "pk", "k", 1, "v")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  let _ = harness.at(double, 300)
  let view =
    Ok(
      Some(
        LeaseView(
          pocket_key: "pk",
          expires_at: harness.t0 + harness.fixture_settings().lease_ttl,
          records: [HeldView("k", 1, <<"v":utf8>>, Unsettled)],
        ),
      ),
    )
  assert harness.lease_view(double, "c1") == view
  assert harness.lease_view(double, "c2") == Ok(None)
  assert harness.lease_view(double, "c1") == view
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}

/// `impl-choice` (spec note G36, as N22; G26 is the same for a pull and
/// a renew): the specification states no such refusal. `cx` never joined,
/// and `p` joined as a publisher only. Each read is refused, over an empty
/// queue and with `c1` holding a lease, and the complete snapshot, the
/// clock's read count and the lease-id draws are as they were.
fn unknown_consumer() -> List(String) {
  let double = harness.fixture()
  let refused = fn(consumer) { Error(Refused(UnknownConsumer(consumer))) }
  assert harness.lease_view(double, "cx") == refused("cx")
  assert harness.publish(double, "pk", "k", 1, "v")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  let batch = harness.batch(double, "c1", 10)
  assert batch.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  let before = harness.inspect(double)
  let reads = harness.clock_reads(double.clock)
  let draws = harness.lease_id_draws(double.lease_ids)
  assert harness.lease_view(double, "cx") == refused("cx")
  assert harness.lease_view(double, "p") == refused("p")
  assert harness.inspect(double) == before
  assert harness.clock_reads(double.clock) == reads
  assert harness.lease_id_draws(double.lease_ids) == draws
  harness.fired(double)
}
