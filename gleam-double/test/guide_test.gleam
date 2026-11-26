//// The walkthrough of `gleam-double/README.md`, run, and the check that keeps
//// the page true to it.
////
//// Gleam has no doctest, so the page's examples live here. Each of decision
//// 0020's Scenarios A to D is one registered test, a short session against
//// the owner with the state after each step written as an assertion. The
//// page prints only what these tests assert: every block it fences as
//// `gleam` is a run of whole lines of this file, copied verbatim. A block it
//// fences as `gleam src/...` is an excerpt of that source file instead, and
//// must appear there verbatim. `drift` checks both, and that the page has
//// one section for the setting, one for the common start and one for each
//// scenario, each holding at least one block of this file, whose blocks, in
//// order, make up the whole body of its function: `setting`,
//// `v3_deferred_behind_v1` or the scenario's session.
////
//// `impl-choice` (spec notes G04 and G48): every step asserts absolute
//// positions and counter values, from the first publish's pocket at 0, its
//// record at 1 and the counter at 2 on. They hold because the double's first
//// position is 0. The specification gives `next_position` no initial value,
//// and fixes only that positions are unique and handed out in increasing
//// order.
////
//// `lease_endings_test` holds the catalog's ports of the same scenarios,
//// each ended both ways. The guide's sessions end A and C by release and B
//// and D by lapse, from a clock at 0, so that the times read as offsets.

import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import pachinko_double.{type Reply, Outcome}
import pachinko_double/answers.{Batch, BatchRecord, TickReport}
import pachinko_double/config.{Settings}
import pachinko_double/ids.{
  type RecordKey, ConsumerId, LeaseId, PocketKey, PublisherId, RecordKey,
}
import pachinko_double/invariants
import pachinko_double/model.{
  type DeadLetter, type Lease, type Pocket, type Record, type SupersededRecord,
  Acked, DeadLetter, Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked,
  Pocket, PocketLeased, PocketQueued, Record, RecordHeld, RecordQueued,
  Superseded, SupersededRecord, Unsettled,
}
import simplifile
import support/harness.{type Double}
import support/runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "Scenario A, as the guide walks it: the acked v1 is consumed at the release and v3 queued where it arrived",
      [Scenario("S078")],
      scenario_a,
    ),
    Test(
      "Scenario B, as the guide walks it: the nacked v1 is dead-lettered at the lapse and v3 queued where it arrived",
      [Scenario("S079")],
      scenario_b,
    ),
    Test(
      "Scenario C, as the guide walks it: v3 is shed when v5 arrives, and v5 takes v3's arrival position at the release",
      [Scenario("S080")],
      scenario_c,
    ),
    Test(
      "Scenario D, as the guide walks it: the unsettled v1 is superseded at the lapse and v3 takes its position",
      [Scenario("S081")],
      scenario_d,
    ),
    Test(
      "every gleam block of the guide is verbatim in its source, and the setting's, the common start's and each scenario's section shows its whole function",
      [],
      guide_matches,
    ),
    Test(
      "a guide block that is not in its source is a fault",
      [],
      a_block_not_in_its_source_is_a_fault,
    ),
    Test(
      "a scenario section with no block, a missing one and a repeated one are faults",
      [],
      a_scenario_section_without_a_block_is_a_fault,
    ),
    Test(
      "a gleam fence the check does not recognise, or one never closed, is a fault",
      [],
      an_unrecognised_fence_is_a_fault,
    ),
    Test(
      "an excerpt of a source that cannot be read, or is not under src, is a fault",
      [],
      an_unreadable_excerpt_is_a_fault,
    ),
    Test(
      "a gleam fence in a blockquote is a fault, and a longer closing fence closes its block",
      [],
      a_quoted_fence_and_a_longer_closing_fence,
    ),
    Test(
      "a scenario's section that repeats, moves or leaves out a step of its session is a fault",
      [],
      a_section_that_is_not_its_whole_session_is_a_fault,
    ),
    Test(
      "a setting or a common start that is missing, repeated, reordered or not its whole function is a fault",
      [],
      a_setting_or_common_start_that_is_not_its_whole_function_is_a_fault,
    ),
  ]
}

// The walkthrough ---------------------------------------------

/// What the guide shows after each step: the queue's position counter,
/// pocket `pk`, the records of `pk` with the keys given, looked up by key,
/// every lease, and the superseded and dead-letter queues.
type Shown {
  Shown(
    next_position: Int,
    pocket: Pocket,
    records: List(Record),
    leases: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}

fn show(double: Double, keys: List(RecordKey)) -> Shown {
  let snapshot = harness.inspect(double)
  let assert Ok(pocket) = model.pocket(snapshot.pockets, PocketKey("pk"))
  Shown(
    next_position: snapshot.next_position,
    pocket:,
    records: list.map(keys, fn(key) {
      let assert Ok(record) =
        model.record(snapshot.records, pocket: PocketKey("pk"), key:)
      record
    }),
    leases: snapshot.leases,
    superseded: snapshot.superseded,
    dead_letters: snapshot.dead_letters,
  )
}

/// A trigger that succeeded, fired these rules and answered nothing.
fn fired(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}

/// The keys of the batch `c2` is handed when it pulls up to ten records.
fn c2_pulls(double: Double) -> List(RecordKey) {
  harness.keys(harness.batch(double, ConsumerId("c2"), 10).records)
}

fn setting() -> Double {
  let assert Ok(config) =
    config.new(
      Settings(
        ..config.defaults(),
        lease_ttl: 1000,
        superseded_ttl: 5000,
        dead_letter_ttl: 7000,
      ),
    )
  let double = harness.start(config)
  assert harness.publisher_joins(double, PublisherId("p"))
    == fired(["PublisherJoins"])
  assert harness.consumer_joins(double, ConsumerId("c1"))
    == fired(["ConsumerJoins"])
  assert harness.consumer_joins(double, ConsumerId("c2"))
    == fired(["ConsumerJoins"])

  double
}

/// The common start of the four sessions. `impl-choice` (spec notes G04 and
/// G48): its positions and counter values are absolute.
fn v3_deferred_behind_v1() -> Double {
  let double = setting()
  assert harness.publish(double, PocketKey("pk"), RecordKey("k"), 1, "v1")
    == fired(["PublishIntoNewPocket"])
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 2,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
      ],
      leases: [],
      superseded: [],
      dead_letters: [],
    )

  assert harness.publish(double, PocketKey("pk"), RecordKey("early"), 1, "e")
    == fired(["PublishNewKey"])
  assert show(double, [RecordKey("k"), RecordKey("early")])
    == Shown(
      next_position: 3,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("early"),
          rank: 1,
          payload: <<"e":utf8>>,
          position: 2,
          status: RecordQueued,
        ),
      ],
      leases: [],
      superseded: [],
      dead_letters: [],
    )

  assert harness.pull(double, ConsumerId("c1"), 1)
    == Ok(Outcome(
      ["PullClaimsFrontPocket", "HoldRecord"],
      Some(
        Batch(
          consumer: ConsumerId("c1"),
          lease: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          records: [
            BatchRecord(key: RecordKey("k"), rank: 1, payload: <<"v1":utf8>>),
          ],
        ),
      ),
    ))
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 3,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: None,
          ),
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )

  harness.set_time(double.clock, 10)
  assert harness.publish(double, PocketKey("pk"), RecordKey("k"), 3, "v3")
    == fired(["DeferBehindHeldVersion"])
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )

  double
}

fn scenario_a() -> List(String) {
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 20)
  assert harness.ack(
      double,
      ConsumerId("c1"),
      LeaseId("lease-1"),
      RecordKey("k"),
    )
    == fired(["AckMarksRecord"])
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Acked,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )

  harness.set_time(double.clock, 50)
  assert harness.release(double, ConsumerId("c1"), LeaseId("lease-1"))
    == fired(["ReleaseRequeuesPocket", "ConsumeAckedVersionQueueDeferred"])
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 4,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseReleased(ended_at: 50),
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
  assert c2_pulls(double) == [RecordKey("early"), RecordKey("k")]

  harness.fired(double)
}

fn scenario_b() -> List(String) {
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 20)
  assert harness.nack(
      double,
      ConsumerId("c1"),
      LeaseId("lease-1"),
      RecordKey("k"),
    )
    == fired(["NackMarksRecord"])
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Nacked,
            nacked_at: Some(20),
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )

  harness.set_time(double.clock, 1020)
  let expired =
    Lease(
      id: LeaseId("lease-1"),
      pocket: PocketKey("pk"),
      consumer: ConsumerId("c1"),
      status: LeaseExpired(ended_at: 1020),
      granted_at: 0,
      expires_at: 1020,
    )
  assert harness.tick(double)
    == Ok(
      TickReport(
        fired: [
          "LeaseLapses",
          "LeaseExpiryKeepsPocketAtFront",
          "DeadLetterNackedVersionQueueDeferred",
        ],
        lapsed: [expired],
        superseded: [],
        dead_letters: [],
      ),
    )
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 1,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
      ],
      leases: [expired],
      superseded: [],
      dead_letters: [
        DeadLetter(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          nacked_by: ConsumerId("c1"),
          nacked_at: 20,
          expires_at: 8020,
        ),
      ],
    )
  assert c2_pulls(double) == [RecordKey("early"), RecordKey("k")]

  harness.fired(double)
}

fn scenario_c() -> List(String) {
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 15)
  assert harness.publish(double, PocketKey("pk"), RecordKey("between"), 1, "b")
    == fired(["PublishNewKey"])
  assert show(double, [RecordKey("k"), RecordKey("between")])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("between"),
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )

  harness.set_time(double.clock, 18)
  assert harness.publish(double, PocketKey("pk"), RecordKey("k"), 5, "v5")
    == fired(["Supersede"])
  let v3 =
    SupersededRecord(
      pocket: PocketKey("pk"),
      key: RecordKey("k"),
      rank: 3,
      payload: <<"v3":utf8>>,
      reason: Superseded,
      shed_at: 18,
      expires_at: 5018,
    )
  assert show(double, [RecordKey("k"), RecordKey("between")])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("between"),
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [v3],
      dead_letters: [],
    )

  harness.set_time(double.clock, 20)
  assert harness.nack(
      double,
      ConsumerId("c1"),
      LeaseId("lease-1"),
      RecordKey("k"),
    )
    == fired(["NackMarksRecord"])
  assert show(double, [RecordKey("k"), RecordKey("between")])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Nacked,
            nacked_at: Some(20),
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("between"),
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [v3],
      dead_letters: [],
    )

  harness.set_time(double.clock, 50)
  assert harness.release(double, ConsumerId("c1"), LeaseId("lease-1"))
    == fired(["ReleaseRequeuesPocket", "DeadLetterNackedVersionQueueDeferred"])
  assert show(double, [RecordKey("k"), RecordKey("between")])
    == Shown(
      next_position: 6,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 5,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("between"),
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: LeaseId("lease-1"),
          pocket: PocketKey("pk"),
          consumer: ConsumerId("c1"),
          status: LeaseReleased(ended_at: 50),
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [v3],
      dead_letters: [
        DeadLetter(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          nacked_by: ConsumerId("c1"),
          nacked_at: 20,
          expires_at: 7050,
        ),
      ],
    )
  assert c2_pulls(double)
    == [RecordKey("early"), RecordKey("k"), RecordKey("between")]

  harness.fired(double)
}

fn scenario_d() -> List(String) {
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 1000)
  let expired =
    Lease(
      id: LeaseId("lease-1"),
      pocket: PocketKey("pk"),
      consumer: ConsumerId("c1"),
      status: LeaseExpired(ended_at: 1000),
      granted_at: 0,
      expires_at: 1000,
    )
  assert harness.tick(double)
    == Ok(
      TickReport(
        fired: [
          "LeaseLapses",
          "LeaseExpiryKeepsPocketAtFront",
          "SupersedeUnsettledHeldVersion",
        ],
        lapsed: [expired],
        superseded: [],
        dead_letters: [],
      ),
    )
  assert show(double, [RecordKey("k")])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: PocketKey("pk"),
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 1,
      ),
      records: [
        Record(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
      ],
      leases: [expired],
      superseded: [
        SupersededRecord(
          pocket: PocketKey("pk"),
          key: RecordKey("k"),
          rank: 1,
          payload: <<"v1":utf8>>,
          reason: Superseded,
          shed_at: 1000,
          expires_at: 6000,
        ),
      ],
      dead_letters: [],
    )
  assert c2_pulls(double) == [RecordKey("k"), RecordKey("early")]

  assert invariants.check(harness.inspect(double)) == []

  harness.fired(double)
}

// The drift check ---------------------------------------------

/// The guide and the walkthrough, from `gleam-double/`, where the run starts.
pub const readme_path = "README.md"

pub const walkthrough_path = "test/guide_test.gleam"

/// The scenarios whose sections the guide must have, one each.
const scenarios = ["A", "B", "C", "D"]

/// A section the guide must have once, which shows the whole body of one
/// function of the walkthrough: the key its heading gives its blocks, its
/// name in a fault, and the function's name and the type it returns.
type Part {
  Part(key: String, name: String, function: String, returns: String)
}

/// The setting, the common start and the scenarios, in the guide's order.
/// A scenario's key is its letter.
fn parts() -> List(Part) {
  [
    Part("setting", "the setting", "setting", "Double"),
    Part("common start", "the common start", "v3_deferred_behind_v1", "Double"),
    ..list.map(scenarios, fn(letter) {
      Part(
        letter,
        "Scenario " <> letter,
        "scenario_" <> string.lowercase(letter),
        "List(String)",
      )
    })
  ]
}

/// What a fence opens: a block of the walkthrough, an excerpt of a source
/// file, or a block the check does not read.
type Kind {
  Walkthrough
  Excerpt(path: String)
  Skipped
}

type Open {
  Open(line: Int, marker: String, kind: Kind, lines: List(String))
}

/// A fenced block the check reads: the line its fence opens on, the file it
/// must appear in, its text, and the key of the part whose section holds it.
type Block {
  Block(line: Int, source: String, text: String, section: Option(String))
}

type Reading {
  Reading(
    section: Option(String),
    fence: Option(Open),
    blocks: List(Block),
    sections: List(String),
    faults: List(String),
  )
}

/// The guide's faults, one line each; none means the page matches.
///
/// - Every block fenced exactly as `gleam` is a run of whole lines of the
///   walkthrough, `test/guide_test.gleam`, verbatim.
/// - Every block fenced exactly as `gleam src/<path>.gleam` is a run of whole
///   lines of that source file, verbatim.
/// - Any other fence that names `gleam`, in any case, with a space before it,
///   a trailing space, a `~~~` fence or inside a blockquote, is a fault, so a
///   block cannot slip out of the check by its spelling. So is a fence that
///   never closes, and an empty block. A fence closes, as in Markdown, on a
///   run of its mark at least as long as the one that opened it.
/// - The setting, the common start and each of Scenarios A to D has exactly
///   one section, which runs from its level-three heading to the next heading
///   of level three or above and holds at least one block of the
///   walkthrough. The setting's heading is exactly `### The setting`, the
///   common start's exactly `### The common start`, and a scenario's starts
///   `### Scenario` and its letter. A heading for any other scenario is a
///   fault.
/// - Each of those sections shows its whole function: its blocks of the
///   walkthrough, in order and joined by a blank line, are the body of
///   `setting`, of `v3_deferred_behind_v1` or of `scenario_<letter>`, less
///   the line that returns. So a reordered common start is a fault, as a
///   reordered session is.
///
/// `read` gives a file's text, or the fault that it cannot be read.
pub fn drift(
  readme: Result(String, String),
  read: fn(String) -> Result(String, String),
) -> List(String) {
  case readme {
    Error(fault) -> [fault]
    Ok(text) -> {
      let reading =
        text
        |> string.split("\n")
        |> list.index_map(fn(line, index) { #(index + 1, line) })
        |> list.fold(Reading(None, None, [], [], []), step)
      let unclosed = case reading.fence {
        Some(open) -> [
          "line " <> int.to_string(open.line) <> ": a fence that never closes",
        ]
        None -> []
      }
      let blocks = list.reverse(reading.blocks)
      let block_faults =
        list.map(blocks, fn(block) { block_faults(block, read) })
      list.flatten([
        list.reverse(reading.faults),
        unclosed,
        list.flatten(block_faults),
        list.flat_map(parts(), fn(part) {
          section_faults(part, reading.sections, blocks)
        }),
        list.flat_map(parts(), fn(part) {
          session_faults(
            part,
            reading.sections,
            list.zip(blocks, block_faults),
            read,
          )
        }),
      ])
    }
  }
}

fn step(reading: Reading, numbered: #(Int, String)) -> Reading {
  let #(number, line) = numbered
  case reading.fence {
    Some(open) ->
      case closes(line, open.marker) {
        True -> close(reading, open)
        False ->
          Reading(
            ..reading,
            fence: Some(Open(..open, lines: [line, ..open.lines])),
          )
      }
    None -> {
      let trimmed = bare(line)
      case
        string.starts_with(trimmed, "```"),
        string.starts_with(trimmed, "~~~"),
        string.starts_with(line, "#")
      {
        True, _, _ -> open(reading, number, line, trimmed, "`")
        _, True, _ -> open(reading, number, line, trimmed, "~")
        _, _, True -> heading(reading, number, line)
        _, _, _ -> reading
      }
    }
  }
}

/// A line without the spaces and the blockquote marks `>` it starts with, so
/// that a fence inside a blockquote is seen as a fence. `open` then finds
/// that its line is not exactly ```` ```gleam ````, so a blockquoted fence
/// that names `gleam` is a fault.
fn bare(line: String) -> String {
  case line {
    " " <> rest | ">" <> rest -> bare(rest)
    _ -> line
  }
}

/// Whether a line closes a fence opened with `marker`: as Markdown has it, a
/// run of the same mark at least as long as the opening one, with nothing
/// after it but spaces. A longer closing fence therefore closes the block
/// rather than leaving the rest of the page inside it.
fn closes(line: String, marker: String) -> Bool {
  let text = string.trim(bare(line))
  let mark = string.slice(marker, 0, 1)
  let length = run(text, mark)
  length >= string.length(marker) && length == string.length(text)
}

fn open(
  reading: Reading,
  number: Int,
  line: String,
  trimmed: String,
  mark: String,
) -> Reading {
  let marker = string.repeat(mark, run(trimmed, mark))
  let kind = case line {
    "```gleam" -> Ok(Walkthrough)
    "```gleam " <> path ->
      case
        string.starts_with(path, "src/")
        && string.ends_with(path, ".gleam")
        && !string.contains(path, " ")
        && !string.contains(path, "..")
      {
        True -> Ok(Excerpt(path))
        False -> Error(Nil)
      }
    _ ->
      case string.contains(string.lowercase(line), "gleam") {
        True -> Error(Nil)
        False -> Ok(Skipped)
      }
  }
  case kind {
    Ok(kind) -> Reading(..reading, fence: Some(Open(number, marker, kind, [])))
    Error(Nil) ->
      Reading(
        ..reading,
        fence: Some(Open(number, marker, Skipped, [])),
        faults: [
          "line "
            <> int.to_string(number)
            <> ": a gleam fence the check does not recognise: "
            <> line,
          ..reading.faults
        ],
      )
  }
}

/// How many times `mark` repeats at the start of `text`.
fn run(text: String, mark: String) -> Int {
  case string.starts_with(text, mark) {
    True -> 1 + run(string.drop_start(text, 1), mark)
    False -> 0
  }
}

fn close(reading: Reading, open: Open) -> Reading {
  let text = string.join(list.reverse(open.lines), "\n")
  let block = fn(source) {
    Block(line: open.line, source:, text:, section: reading.section)
  }
  case open.kind {
    Skipped -> Reading(..reading, fence: None)
    Walkthrough ->
      Reading(..reading, fence: None, blocks: [
        block(walkthrough_path),
        ..reading.blocks
      ])
    Excerpt(path) ->
      Reading(..reading, fence: None, blocks: [block(path), ..reading.blocks])
  }
}

/// A heading of level three or above ends the section it is in. A
/// level-three heading that starts `### Scenario ` opens a scenario's, and
/// `### The setting` and `### The common start` open theirs.
fn heading(reading: Reading, number: Int, line: String) -> Reading {
  case run(line, "#") <= 3 {
    False -> reading
    True ->
      case line {
        "### The setting" -> opens(reading, "setting")
        "### The common start" -> opens(reading, "common start")
        "### Scenario " <> rest -> {
          let letter = string.slice(rest, 0, 1)
          case list.contains(scenarios, letter) {
            True -> opens(reading, letter)
            False ->
              Reading(..reading, section: None, faults: [
                "line "
                  <> int.to_string(number)
                  <> ": a section for a scenario the guide does not walk: "
                  <> line,
                ..reading.faults
              ])
          }
        }
        _ -> Reading(..reading, section: None)
      }
  }
}

/// The reading inside the section of the part with this key.
fn opens(reading: Reading, key: String) -> Reading {
  Reading(..reading, section: Some(key), sections: [key, ..reading.sections])
}

fn block_faults(
  block: Block,
  read: fn(String) -> Result(String, String),
) -> List(String) {
  let at = "line " <> int.to_string(block.line) <> ": "
  case string.trim(block.text) == "", read(block.source) {
    True, _ -> [at <> "an empty gleam block"]
    False, Error(fault) -> [at <> fault]
    False, Ok(source) ->
      case string.contains("\n" <> source <> "\n", "\n" <> block.text <> "\n") {
        True -> []
        False -> [
          at <> "the gleam block is not verbatim in " <> block.source,
        ]
      }
  }
}

fn section_faults(
  part: Part,
  sections: List(String),
  blocks: List(Block),
) -> List(String) {
  let name = part.name
  let walked =
    list.count(blocks, fn(block) {
      block.section == Some(part.key) && block.source == walkthrough_path
    })
  case list.count(sections, fn(section) { section == part.key }), walked {
    0, _ -> ["the guide has no section for " <> name]
    1, 0 -> ["the section for " <> name <> " has no gleam block"]
    1, _ -> []
    count, _ -> [name <> " has " <> int.to_string(count) <> " sections"]
  }
}

/// A part's section must show its whole function, in order. Its blocks of
/// the walkthrough, joined by one blank line, are the body of the part's
/// function in the walkthrough, without the body's last line, which returns
/// the double or the rules the test saw fire, and the blank lines before it.
/// So a block repeated, moved or left out is a fault, although each block
/// alone is verbatim.
///
/// The check runs only on a part with exactly one section, holding at least
/// one block of the walkthrough, each of them verbatim; any other case
/// already has its fault.
fn session_faults(
  part: Part,
  sections: List(String),
  checked: List(#(Block, List(String))),
  read: fn(String) -> Result(String, String),
) -> List(String) {
  let mine =
    list.filter(checked, fn(pair) {
      { pair.0 }.section == Some(part.key)
      && { pair.0 }.source == walkthrough_path
    })
  let verbatim = list.all(mine, fn(pair) { pair.1 == [] })
  let one = list.count(sections, fn(section) { section == part.key }) == 1
  case one && verbatim && mine != [], read(walkthrough_path) {
    True, Ok(source) -> {
      let shown =
        string.join(list.map(mine, fn(pair) { { pair.0 }.text }), "\n\n")
      let head = "fn " <> part.function <> "() -> " <> part.returns <> " {"
      case session(source, head) == Ok(shown) {
        True -> []
        False -> [
          "the blocks of "
          <> part.name
          <> "'s section are not the whole of "
          <> part.function
          <> " in "
          <> walkthrough_path
          <> ", in order",
        ]
      }
    }
    _, _ -> []
  }
}

/// The body of the function whose head line is `head`, such as
/// `fn setting() -> Double {`, in a source, as a section shows it: every line
/// between the head line and the first `}` that closes a top-level item, less
/// the last line and the blank lines before it.
fn session(source: String, head: String) -> Result(String, Nil) {
  let head = "\n" <> head <> "\n"
  case string.split_once("\n" <> source, head) {
    Error(Nil) -> Error(Nil)
    Ok(#(_, rest)) ->
      case string.split_once(rest, "\n}\n") {
        Error(Nil) -> Error(Nil)
        Ok(#(body, _)) ->
          case list.reverse(string.split(body, "\n")) {
            [] | [_] -> Error(Nil)
            [_, ..before] ->
              Ok(
                before
                |> list.drop_while(fn(line) { string.trim(line) == "" })
                |> list.reverse
                |> string.join("\n"),
              )
          }
      }
  }
}

/// A file's text, or the fault that it could not be read.
pub fn read(path: String) -> Result(String, String) {
  case simplifile.read(path) {
    Ok(text) -> Ok(text)
    Error(error) ->
      Error("cannot read " <> path <> ": " <> simplifile.describe_error(error))
  }
}

fn guide_matches() -> List(String) {
  assert drift(read(readme_path), read) == []
  []
}

// The drift check's controls, on pages and files made by hand ---

/// A reader over these files alone.
fn files(
  held: List(#(String, String)),
) -> fn(String) -> Result(String, String) {
  fn(path) {
    case list.key_find(held, path) {
      Ok(text) -> Ok(text)
      Error(Nil) -> Error("cannot read " <> path)
    }
  }
}

/// A walkthrough that holds `step one` and `step two`, with a session for
/// each scenario whose body is that scenario's block in `page(four)`, a
/// setting and a common start whose bodies are their blocks in `page`, and a
/// source file.
fn sources() -> fn(String) -> Result(String, String) {
  files([
    #(walkthrough_path, walkthrough),
    #("src/pachinko_double/row.gleam", "fn row() {\n  guard\n}\n"),
  ])
}

const walkthrough =
  "fn walk() {
  step one
  step two
}

fn scenario_a() -> List(String) {
  step one

  []
}

fn scenario_b() -> List(String) {
  step two

  []
}

fn scenario_c() -> List(String) {
  step one
  step two

  []
}

fn scenario_d() -> List(String) {
  step one

  []
}

fn setting() -> Double {
  set up

  double
}

fn v3_deferred_behind_v1() -> Double {
  start one

  start two

  double
}
"

/// A section for a scenario, with one block of the walkthrough.
fn section(letter: String, block: String) -> String {
  "### Scenario " <> letter <> ": walked\n\n```gleam\n" <> block <> "\n```\n"
}

/// A page with all four scenarios' sections, each with a block of the
/// walkthrough, an excerpt before them, and after them the setting's
/// section and the common start's, which ends the page at line 47.
fn page(blocks: List(String)) -> String {
  let sections =
    list.zip(scenarios, blocks)
    |> list.map(fn(pair) { section(pair.0, pair.1) })
  string.join(
    [
      "# Guide\n\n```gleam src/pachinko_double/row.gleam\n  guard\n```\n",
      ..sections
    ],
    "\n",
  )
  <> "\n"
  <> shared
}

/// The setting's section and the common start's, in the walkthrough's order.
const shared =
  "### The setting

```gleam
  set up
```

### The common start

```gleam
  start one
```

```gleam
  start two
```
"

const four = ["  step one", "  step two", "  step one\n  step two", "  step one"]

fn a_block_not_in_its_source_is_a_fault() -> List(String) {
  // The control's baseline holds: a page whose blocks are all in place.
  assert drift(Ok(page(four)), sources()) == []
  // A block that is not a run of whole lines of the walkthrough.
  assert drift(
      Ok(page(["  step one", "  step three", "  step", "step one"])),
      sources(),
    )
    == [
      "line 15: the gleam block is not verbatim in test/guide_test.gleam",
      "line 21: the gleam block is not verbatim in test/guide_test.gleam",
      "line 27: the gleam block is not verbatim in test/guide_test.gleam",
    ]
  // An excerpt that is not in the file it names.
  let excerpt =
    string.replace(
      page(four),
      "```gleam src/pachinko_double/row.gleam\n  guard",
      "```gleam src/pachinko_double/row.gleam\n  effect",
    )
  assert drift(Ok(excerpt), sources())
    == [
      "line 3: the gleam block is not verbatim in src/pachinko_double/row.gleam",
    ]
  // A page that cannot be read.
  assert drift(Error("cannot read README.md"), sources())
    == ["cannot read README.md"]
  []
}

fn a_scenario_section_without_a_block_is_a_fault() -> List(String) {
  let without_c =
    string.replace(
      page(four),
      "### Scenario C: walked\n\n```gleam\n  step one\n  step two\n```\n",
      "### Scenario C: walked\n\nNo block here.\n",
    )
  assert drift(Ok(without_c), sources())
    == ["the section for Scenario C has no gleam block"]
  // The block after a new heading is no longer in the scenario's section.
  let moved =
    string.replace(
      page(four),
      "### Scenario C: walked\n\n",
      "### Scenario C: walked\n\n## Elsewhere\n\n",
    )
  assert drift(Ok(moved), sources())
    == ["the section for Scenario C has no gleam block"]
  // A heading renamed, so the section is gone.
  let renamed = string.replace(page(four), "### Scenario D", "### Case D")
  assert drift(Ok(renamed), sources())
    == ["the guide has no section for Scenario D"]
  // A section repeated, and one for a scenario the guide does not walk.
  let repeated = page(four) <> "\n" <> section("A", "  step one")
  assert drift(Ok(repeated), sources()) == ["Scenario A has 2 sections"]
  let unknown = page(four) <> "\n" <> section("E", "  step one")
  assert drift(Ok(unknown), sources())
    == [
      "line 48: a section for a scenario the guide does not walk: ### Scenario E: walked",
    ]
  []
}

fn an_unrecognised_fence_is_a_fault() -> List(String) {
  let spelled = fn(fence) {
    string.replace(
      page(four),
      "### Scenario B: walked\n\n```gleam\n",
      "### Scenario B: walked\n\n" <> fence <> "\n",
    )
  }
  // Each spelling leaves Scenario B with no block it recognises.
  list.each(["``` gleam", "```gleam ", "```Gleam", " ```gleam"], fn(fence) {
    assert drift(Ok(spelled(fence)), sources())
      == [
        "line 15: a gleam fence the check does not recognise: " <> fence,
        "the section for Scenario B has no gleam block",
      ]
  })
  let tildes =
    string.replace(
      page(four),
      "### Scenario B: walked\n\n```gleam\n  step two\n```\n",
      "### Scenario B: walked\n\n~~~gleam\n  step two\n~~~\n",
    )
  assert drift(Ok(tildes), sources())
    == [
      "line 15: a gleam fence the check does not recognise: ~~~gleam",
      "the section for Scenario B has no gleam block",
    ]
  // An excerpt that names a file outside `src/`.
  let outside =
    string.replace(
      page(four),
      "```gleam src/pachinko_double/row.gleam",
      "```gleam test/harness.gleam",
    )
  assert drift(Ok(outside), sources())
    == [
      "line 3: a gleam fence the check does not recognise: ```gleam test/harness.gleam",
    ]
  // A fence that never closes swallows the rest of the page.
  let unclosed = page(four) <> "\n```gleam\n  step one\n"
  assert drift(Ok(unclosed), sources())
    == ["line 48: a fence that never closes"]
  // A fence of another language is skipped, even with gleam-like text in it.
  let other = page(four) <> "\n```sh\njust gleam-double\n### Scenario A\n```\n"
  assert drift(Ok(other), sources()) == []
  // An empty block.
  let empty =
    string.replace(page(four), "```gleam\n  step one\n```", "```gleam\n```")
  assert drift(Ok(empty), sources())
    == ["line 9: an empty gleam block", "line 27: an empty gleam block"]
  []
}

fn an_unreadable_excerpt_is_a_fault() -> List(String) {
  let missing =
    string.replace(
      page(four),
      "src/pachinko_double/row.gleam",
      "src/pachinko_double/gone.gleam",
    )
  assert drift(Ok(missing), sources())
    == ["line 3: cannot read src/pachinko_double/gone.gleam"]
  assert drift(Ok(page(four)), files([]))
    == [
      "line 3: cannot read src/pachinko_double/row.gleam",
      "line 9: cannot read test/guide_test.gleam",
      "line 15: cannot read test/guide_test.gleam",
      "line 21: cannot read test/guide_test.gleam",
      "line 28: cannot read test/guide_test.gleam",
      "line 34: cannot read test/guide_test.gleam",
      "line 40: cannot read test/guide_test.gleam",
      "line 44: cannot read test/guide_test.gleam",
    ]
  []
}

fn a_quoted_fence_and_a_longer_closing_fence() -> List(String) {
  // A fence after a blockquote's `>` is a fence. One that names gleam is a
  // fault rather than a block the check skips.
  let quoted = page(four) <> "\n> ```gleam\n> assert False\n> ```\n"
  assert drift(Ok(quoted), sources())
    == ["line 48: a gleam fence the check does not recognise: > ```gleam"]
  // A quoted fence of another language is skipped, and closes on its
  // quoted closing fence, so the block after it is still read.
  let quoted_sh =
    page(four)
    <> "\n> ```sh\n> just gleam-double\n> ```\n\n```gleam\n  step three\n```\n"
  assert drift(Ok(quoted_sh), sources())
    == ["line 52: the gleam block is not verbatim in test/guide_test.gleam"]
  // A closing fence longer than its opening one closes the block, so the
  // gleam block after it is read, not swallowed.
  let longer =
    page(four)
    <> "\n```sh\njust gleam-double\n````\n\n```gleam\n  step three\n```\n"
  assert drift(Ok(longer), sources())
    == ["line 52: the gleam block is not verbatim in test/guide_test.gleam"]
  // A shorter one, or one of the other mark, does not close it.
  let shorter = page(four) <> "\n````sh\njust gleam-double\n```\n~~~~\n"
  assert drift(Ok(shorter), sources()) == ["line 48: a fence that never closes"]
  []
}

fn a_section_that_is_not_its_whole_session_is_a_fault() -> List(String) {
  let fault = fn(letter) {
    "the blocks of Scenario "
    <> letter
    <> "'s section are not the whole of scenario_"
    <> string.lowercase(letter)
    <> " in test/guide_test.gleam, in order"
  }
  // A walkthrough whose Scenario A takes two steps.
  let two_steps =
    files([
      #(
        walkthrough_path,
        string.replace(
          walkthrough,
          "fn scenario_a() -> List(String) {\n  step one\n",
          "fn scenario_a() -> List(String) {\n  step one\n\n  step two\n",
        ),
      ),
      #("src/pachinko_double/row.gleam", "fn row() {\n  guard\n}\n"),
    ])
  let shown = fn(steps) {
    let blocks =
      list.map(steps, fn(step) { "```gleam\n  step " <> step <> "\n```\n" })
    string.replace(
      page(four),
      "### Scenario A: walked\n\n```gleam\n  step one\n```\n",
      "### Scenario A: walked\n\n" <> string.join(blocks, "\n"),
    )
  }
  // Both steps, in order, are the whole session.
  assert drift(Ok(shown(["one", "two"])), two_steps) == []
  // A step repeated, the steps swapped, and a step left out.
  assert drift(Ok(shown(["one", "two", "two"])), two_steps) == [fault("A")]
  assert drift(Ok(shown(["two", "one"])), two_steps) == [fault("A")]
  assert drift(Ok(shown(["one"])), two_steps) == [fault("A")]
  // A walkthrough that holds every block but no function a section shows.
  let sessionless =
    files([
      #(
        walkthrough_path,
        "fn walk() {\n  step one\n  step two\n  set up\n  start one\n  start two\n}\n",
      ),
      #("src/pachinko_double/row.gleam", "fn row() {\n  guard\n}\n"),
    ])
  assert drift(Ok(page(four)), sessionless)
    == [
      "the blocks of the setting's section are not the whole of setting in test/guide_test.gleam, in order",
      "the blocks of the common start's section are not the whole of v3_deferred_behind_v1 in test/guide_test.gleam, in order",
      fault("A"),
      fault("B"),
      fault("C"),
      fault("D"),
    ]
  []
}

fn a_setting_or_common_start_that_is_not_its_whole_function_is_a_fault() -> List(
  String,
) {
  let setting =
    "the blocks of the setting's section are not the whole of setting in test/guide_test.gleam, in order"
  let start =
    "the blocks of the common start's section are not the whole of v3_deferred_behind_v1 in test/guide_test.gleam, in order"
  let start_one = "```gleam\n  start one\n```\n"
  let start_two = "```gleam\n  start two\n```\n"
  // The common start's two steps swapped, each block still verbatim.
  let swapped =
    string.replace(
      page(four),
      start_one <> "\n" <> start_two,
      start_two <> "\n" <> start_one,
    )
  assert drift(Ok(swapped), sources()) == [start]
  // A step of the common start left out, and one repeated.
  let left_out = string.replace(page(four), start_one <> "\n", "")
  assert drift(Ok(left_out), sources()) == [start]
  let twice =
    string.replace(page(four), start_two, start_two <> "\n" <> start_two)
  assert drift(Ok(twice), sources()) == [start]
  // A setting that shows a block of another function.
  let other =
    string.replace(
      page(four),
      "```gleam\n  set up\n```\n",
      "```gleam\n  start one\n```\n",
    )
  assert drift(Ok(other), sources()) == [setting]
  // A heading renamed, so the check would have nothing to compare.
  let renamed =
    string.replace(page(four), "### The common start", "### Common start")
  assert drift(Ok(renamed), sources())
    == ["the guide has no section for the common start"]
  // A section with no block, and one repeated.
  let empty =
    string.replace(
      page(four),
      "### The setting\n\n```gleam\n  set up\n```\n",
      "### The setting\n\nNo block here.\n",
    )
  assert drift(Ok(empty), sources())
    == ["the section for the setting has no gleam block"]
  let repeated = page(four) <> "\n### The setting\n\n```gleam\n  set up\n```\n"
  assert drift(Ok(repeated), sources()) == ["the setting has 2 sections"]
  []
}
