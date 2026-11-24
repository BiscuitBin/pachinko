//// The lease's end, `lease_end`: the shared end-of-lease steps, the
//// comparison that decides the pocket's fate, and the specification's
//// "Lease end" section: the six `Freed` rules, three for a held record with
//// nothing deferred behind it and three for one with a newer version
//// deferred.
////
//// These tests pin no catalog row. They run over states built by hand:
//// `lease_end.end` and `lease_end.outlasts` are called directly, and each
//// `Freed` rule is run alone, as its own outcome through `core.run`, so the
//// checker reads the state it leaves. What a whole release does, with the
//// `Freed` rules it chains to, is in `lease_test` and `settle_test`.

import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/config
import pachinko_double/failure.{Defect, EffectFailed, NoRuleHeld}
import pachinko_double/ids.{
  type LeaseId, type RecordKey, ConsumerId, LeaseId, PocketKey, PublisherId,
  RecordKey,
}
import pachinko_double/internal/core
import pachinko_double/internal/domain.{type Step, Freed}
import pachinko_double/internal/lease_end.{Freeing, Lapse, Release}
import pachinko_double/internal/rules.{Applied, Defective, Done, Row, Table}
import pachinko_double/model.{
  type LeaseStatus, type Mark, type Record, type State, Acked, Consumer,
  DeadLetter, Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked, Pocket,
  PocketIdle, PocketLeased, Publisher, Queue, Record, RecordHeld, RecordQueued,
  Stale, State, Superseded, SupersededRecord, Unsettled,
}
import support/runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "ending a lease sets its status and ended_at and sends Freed for each held record, smallest position first, and changes nothing else",
      [],
      end_exactly,
    ),
    Test(
      "a pocket outlasts its lease when it holds a record the lease's end does not remove",
      [],
      outlasts,
    ),
    Test(
      "RequeueUnsettledRecord changes exactly what it ensures and asks for no time",
      [],
      requeue_exactly,
    ),
    Test(
      "ConsumeAckedRecord changes exactly what it ensures and asks for no time",
      [],
      consume_exactly,
    ),
    Test(
      "DeadLetterNackedRecord changes exactly what it ensures, stamps the entry from ended_at and asks for no time",
      [],
      dead_letter_exactly,
    ),
    Test(
      "with dead_letter_ttl 0 a dead letter expires at the lease's ended_at",
      [],
      dead_letter_zero_ttl,
    ),
    Test(
      "a dead letter carries the held version, not a newer one the record has",
      [],
      dead_letter_held_version,
    ),
    Test(
      "SupersedeUnsettledHeldVersion changes exactly what it ensures, stamps the entry from ended_at and asks for no time",
      [],
      supersede_held_exactly,
    ),
    Test(
      "ConsumeAckedVersionQueueDeferred changes exactly what it ensures and asks for no time",
      [],
      consume_deferred_exactly,
    ),
    Test(
      "DeadLetterNackedVersionQueueDeferred changes exactly what it ensures, stamps the entry from ended_at and asks for no time",
      [],
      dead_letter_deferred_exactly,
    ),
    Test(
      "the Freed rules read the mark and never how the lease ended",
      [],
      however_the_lease_ended,
    ),
    Test(
      "each Freed rule's guard holds only for a held record with its mark, and with a newer version deferred behind it or with none",
      [],
      freed_guards,
    ),
    Test("a Freed no rule answers is a defect", [], freed_defect),
    Test(
      "a Freed rule's effect without what its guard saw is a defect",
      [],
      freed_without_record,
    ),
  ]
}

const requeue = "RequeueUnsettledRecord"

const consume = "ConsumeAckedRecord"

const dead_letter = "DeadLetterNackedRecord"

const supersede_held = "SupersedeUnsettledHeldVersion"

const consume_deferred = "ConsumeAckedVersionQueueDeferred"

const dead_letter_deferred = "DeadLetterNackedVersionQueueDeferred"

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

/// `pk`'s five records as they are before any is held, listed in no order
/// of position: `n` at 8, `u` at 6, `a` at 3, `m` at 5 and `q` at 11.
fn plain() -> #(Record, Record, Record, Record, Record) {
  #(
    Record(PocketKey("pk"), RecordKey("n"), 45, <<"en":utf8>>, 8, RecordQueued),
    Record(PocketKey("pk"), RecordKey("u"), 44, <<"you":utf8>>, 6, RecordQueued),
    Record(PocketKey("pk"), RecordKey("a"), 46, <<"ay":utf8>>, 3, RecordQueued),
    Record(PocketKey("pk"), RecordKey("m"), 43, <<"em":utf8>>, 5, RecordQueued),
    Record(
      PocketKey("pk"),
      RecordKey("q"),
      42,
      <<"queue":utf8>>,
      11,
      RecordQueued,
    ),
  )
}

/// `other`'s two records, both held by `c2` under `lease-2`: `a`,
/// unsettled, and `n`, nacked at 520. `pk` holds both keys too, each with
/// another mark.
fn theirs() -> #(Record, Record) {
  #(
    held(
      Record(
        PocketKey("other"),
        RecordKey("a"),
        48,
        <<"oa":utf8>>,
        12,
        RecordQueued,
      ),
      Unsettled,
      None,
    ),
    held(
      Record(
        PocketKey("other"),
        RecordKey("n"),
        47,
        <<"on":utf8>>,
        10,
        RecordQueued,
      ),
      Nacked,
      Some(520),
    ),
  )
}

/// Consumers `c1` and `c2` joined.
///
/// - `c1` holds `pk` under `lease-1`, granted at 600. `pk` holds `n`,
///   nacked at 700; `u`, unsettled; `a`, acked; `m`, nacked at 650; and `q`,
///   queued.
/// - `c2` once held `pk` under `lease-0`, which ended as `ended` says.
/// - `c2` holds `other` under `lease-2`, with the two records of `theirs`.
/// - `done` is idle.
/// - The superseded queue holds a stale version of `pk`'s `a`, and the
///   dead-letter queue a record `c2` nacked out of `pk` under `lease-0`.
fn ending(ended: LeaseStatus) -> State {
  let ttl = config.defaults().lease_ttl
  let #(n, u, a, m, q) = plain()
  let #(their_a, their_n) = theirs()
  State(
    ..model.new(config.default()),
    queue: Queue(next_position: 20),
    publishers: [Publisher(PublisherId("p"))],
    consumers: [Consumer(ConsumerId("c1")), Consumer(ConsumerId("c2"))],
    pockets: [
      Pocket(PocketKey("other"), PocketLeased, 9, 0),
      Pocket(PocketKey("pk"), PocketLeased, 2, 0),
      Pocket(PocketKey("done"), PocketIdle, 1, 1),
    ],
    records: [
      held(n, Nacked, Some(700)),
      their_n,
      held(u, Unsettled, None),
      held(a, Acked, None),
      their_a,
      held(m, Nacked, Some(650)),
      q,
    ],
    leases: [
      Lease(
        id: LeaseId("lease-0"),
        pocket: PocketKey("pk"),
        consumer: ConsumerId("c2"),
        status: ended,
        granted_at: 100,
        expires_at: 100 + ttl,
      ),
      Lease(
        id: LeaseId("lease-2"),
        pocket: PocketKey("other"),
        consumer: ConsumerId("c2"),
        status: LeaseActive,
        granted_at: 500,
        expires_at: 500 + ttl,
      ),
      Lease(
        id: LeaseId("lease-1"),
        pocket: PocketKey("pk"),
        consumer: ConsumerId("c1"),
        status: LeaseActive,
        granted_at: 600,
        expires_at: 600 + ttl,
      ),
    ],
    superseded: [
      SupersededRecord(
        pocket: PocketKey("pk"),
        key: RecordKey("a"),
        rank: 40,
        payload: <<"stale":utf8>>,
        reason: Stale,
        shed_at: 300,
        expires_at: 300 + config.defaults().superseded_ttl,
      ),
    ],
    dead_letters: [
      DeadLetter(
        pocket: PocketKey("pk"),
        key: RecordKey("k0"),
        rank: 39,
        payload: <<"zero":utf8>>,
        nacked_by: ConsumerId("c2"),
        nacked_at: 120,
        expires_at: 150 + config.defaults().dead_letter_ttl,
      ),
    ],
  )
}

/// `lease-0` as it ended by release.
const released = LeaseReleased(150)

/// The time a hand-sent `Freed` says its lease ended at, unlike every time
/// in the state.
const ended_at = 777

/// One `Freed` as its own outcome: the `Freed` table, then the checker.
fn freed(state: State, lease: LeaseId, record_key: RecordKey) -> Step {
  core.run(lease_end.freed(), Freeing(lease:, record_key:, ended_at:), state)
}

/// `lease_end.end` over `c1`'s lease, by release and by lapse. The lease is
/// `released` or `expired` with `ended_at` the time it was given, and keeps
/// its `granted_at` and `expires_at`. Nothing else in the complete state
/// changes: the pocket is still leased and its records still held, since the
/// pocket's fate is the calling rule's and each record's the `Freed` rules'.
///
/// `impl-choice` (spec note G38, as N25): one `Freed` for each held record
/// of the lease's pocket, smallest position first, `a`, `m`, `u`, `n`,
/// though they are listed `n`, `u`, `a`, `m`. The queued `q` and `other`'s
/// held records get none. Each names the lease by its id and carries the
/// same `ended_at`.
fn end_exactly() -> List(String) {
  let state = ending(released)
  let assert [lease_0, lease_2, lease_1] = state.leases
  let events = fn(at) {
    list.map(["a", "m", "u", "n"], fn(key) {
      Freed(lease: LeaseId("lease-1"), record_key: RecordKey(key), ended_at: at)
    })
  }
  assert lease_end.end(state, lease_1, Release, 900)
    == #(
      State(..state, leases: [
        lease_0,
        lease_2,
        Lease(..lease_1, status: LeaseReleased(900)),
      ]),
      events(900),
    )
  assert lease_end.end(state, lease_1, Lapse, 950)
    == #(
      State(..state, leases: [
        lease_0,
        lease_2,
        Lease(..lease_1, status: LeaseExpired(950)),
      ]),
      events(950),
    )
  // A lease whose pocket holds no held record sends nothing.
  let #(_, _, _, _, q) = plain()
  let none_held = State(..state, records: [q])
  assert lease_end.end(none_held, lease_1, Release, 900).1 == []
  []
}

/// `pocket.records.count > pocket.ending_records.count`, the pocket named by
/// its key. `pk` holds a queued record and an unsettled one, so it outlasts
/// its lease, and so does `other`, whose `a` is unsettled. With only its
/// acked and nacked records `pk` does not, and with one unsettled or one
/// queued record more it does. An acked or a nacked record with a newer
/// version deferred behind it is not an ending record, so with it alone
/// `pk` outlasts its lease. A pocket with no record, and a key no pocket
/// has, do not.
fn outlasts() -> List(String) {
  let state = ending(released)
  let assert [n, their_n, u, a, their_a, m, q] = state.records
  let with = fn(records) { State(..state, records:) }
  assert lease_end.outlasts(state, PocketKey("pk"))
  assert lease_end.outlasts(state, PocketKey("other"))
  assert !lease_end.outlasts(with([n, their_n, a, their_a, m]), PocketKey("pk"))
  assert lease_end.outlasts(
    with([n, their_n, a, their_a, m]),
    PocketKey("other"),
  )
  assert lease_end.outlasts(with([n, a, m, u]), PocketKey("pk"))
  assert lease_end.outlasts(with([n, a, m, q]), PocketKey("pk"))
  let #(plain_n, _, plain_a, _, _) = plain()
  let deferred = fn(record: Record, mark, nacked_at) {
    Record(
      ..record,
      status: RecordHeld(
        held_rank: 41,
        held_payload: <<"older":utf8>>,
        mark:,
        nacked_at:,
        deferred_position: Some(14),
      ),
    )
  }
  assert lease_end.outlasts(
    with([deferred(plain_a, Acked, None)]),
    PocketKey("pk"),
  )
  assert lease_end.outlasts(
    with([deferred(plain_n, Nacked, Some(700))]),
    PocketKey("pk"),
  )
  assert lease_end.outlasts(
    with([a, deferred(plain_n, Nacked, Some(700))]),
    PocketKey("pk"),
  )
  assert !lease_end.outlasts(with([n, a, m]), PocketKey("other"))
  assert !lease_end.outlasts(with([their_n]), PocketKey("other"))
  assert !lease_end.outlasts(state, PocketKey("done"))
  assert !lease_end.outlasts(state, PocketKey("nowhere"))
  []
}

/// `Freed` for `pk`'s unsettled `u`: the record is queued, at the position
/// it had, with none of the held fields. Nothing else in the complete state
/// changes: the counter stays, no entry is made, and `other`'s records are
/// as they were. The step is done at once, so the rule asked for no time.
fn requeue_exactly() -> List(String) {
  let state = ending(released)
  let assert [n, their_n, held_u, a, their_a, m, q] = state.records
  let #(_, u, _, _, _) = plain()
  assert freed(state, LeaseId("lease-1"), RecordKey("u"))
    == Done(
      Ok(Applied(
        State(..state, records: [n, their_n, u, a, their_a, m, q]),
        [requeue],
        None,
      )),
    )
  // `other`'s unsettled `a`, under `c2`'s lease: `pk`'s `a`, which is acked,
  // is not touched.
  let #(their_plain, _) = plain_theirs()
  assert freed(state, LeaseId("lease-2"), RecordKey("a"))
    == Done(
      Ok(Applied(
        State(..state, records: [n, their_n, held_u, a, their_plain, m, q]),
        [requeue],
        None,
      )),
    )
  [requeue, requeue]
}

/// `other`'s two records as they are before either is held.
fn plain_theirs() -> #(Record, Record) {
  #(
    Record(
      PocketKey("other"),
      RecordKey("a"),
      48,
      <<"oa":utf8>>,
      12,
      RecordQueued,
    ),
    Record(
      PocketKey("other"),
      RecordKey("n"),
      47,
      <<"on":utf8>>,
      10,
      RecordQueued,
    ),
  )
}

/// `Freed` for `pk`'s acked `a`: the record is removed and recorded
/// nowhere. The records that remain keep their order, `other`'s `a`
/// included, and nothing else in the complete state changes. The step is
/// done at once, so the rule asked for no time.
fn consume_exactly() -> List(String) {
  let state = ending(released)
  let assert [n, their_n, u, _, their_a, m, q] = state.records
  assert freed(state, LeaseId("lease-1"), RecordKey("a"))
    == Done(
      Ok(Applied(
        State(..state, records: [n, their_n, u, their_a, m, q]),
        [consume],
        None,
      )),
    )
  [consume]
}

/// `Freed` for `pk`'s `m`, nacked at 650, with `ended_at` 777. The record is
/// removed, and a dead letter is added at the end of the queue: the
/// pocket's key and the record's, the held rank and payload, `nacked_by`
/// the lease's consumer, `nacked_at` the time of the nack, and `expires_at`
/// 777 plus `dead_letter_ttl`. Nothing else in the complete state changes.
/// The step is done at once: the rule asked for no time, so `ended_at` is
/// the only time it could stamp the entry from.
///
/// Then `other`'s `n`, nacked at 520 under `c2`'s lease: the pocket and the
/// consumer are that lease's, and `pk`'s `n` is not touched.
fn dead_letter_exactly() -> List(String) {
  let state = ending(released)
  let assert [n, their_n, u, a, their_a, m, q] = state.records
  let ttl = config.defaults().dead_letter_ttl
  assert freed(state, LeaseId("lease-1"), RecordKey("m"))
    == Done(
      Ok(Applied(
        State(
          ..state,
          records: [n, their_n, u, a, their_a, q],
          dead_letters: list.append(state.dead_letters, [
            DeadLetter(
              pocket: PocketKey("pk"),
              key: RecordKey("m"),
              rank: 43,
              payload: <<"em":utf8>>,
              nacked_by: ConsumerId("c1"),
              nacked_at: 650,
              expires_at: ended_at + ttl,
            ),
          ]),
        ),
        [dead_letter],
        None,
      )),
    )
  assert freed(state, LeaseId("lease-2"), RecordKey("n"))
    == Done(
      Ok(Applied(
        State(
          ..state,
          records: [n, u, a, their_a, m, q],
          dead_letters: list.append(state.dead_letters, [
            DeadLetter(
              pocket: PocketKey("other"),
              key: RecordKey("n"),
              rank: 47,
              payload: <<"on":utf8>>,
              nacked_by: ConsumerId("c2"),
              nacked_at: 520,
              expires_at: ended_at + ttl,
            ),
          ]),
        ),
        [dead_letter],
        None,
      )),
    )
  [dead_letter, dead_letter]
}

/// `expires_at: ended_at + config.dead_letter_ttl`, at the boundary
/// `DeadLetterTtlNonNegative` admits. With `dead_letter_ttl` 0, `Freed` for
/// `pk`'s `m`, nacked at 650, with `ended_at` 777, adds a dead letter that
/// expires at 777 itself.
fn dead_letter_zero_ttl() -> List(String) {
  let assert Ok(zero) =
    config.new(config.Settings(..config.defaults(), dead_letter_ttl: 0))
  let state = State(..ending(released), config: zero)
  let assert Done(Ok(Applied(after, [rule], None))) =
    freed(state, LeaseId("lease-1"), RecordKey("m"))
  assert rule == dead_letter
  assert list.last(after.dead_letters)
    == Ok(DeadLetter(
      pocket: PocketKey("pk"),
      key: RecordKey("m"),
      rank: 43,
      payload: <<"em":utf8>>,
      nacked_by: ConsumerId("c1"),
      nacked_at: 650,
      expires_at: ended_at,
    ))
  [dead_letter]
}

/// `rank: held_rank` and `payload: held_payload`. With nothing deferred the
/// held version is the record's own, so no state that holds the invariants
/// can tell the two apart. This one is built to: `m`'s record carries rank
/// 50 and another payload while its held version is at 43, with no arrival
/// position, which breaks `DeferredHasArrivalPosition` and
/// `HeldCopiesAgree` before the rule runs. The rule removes the record, so
/// the state it leaves holds every invariant, and the dead letter carries
/// the held version.
fn dead_letter_held_version() -> List(String) {
  let state = ending(released)
  let assert [n, their_n, u, a, their_a, m, q] = state.records
  let newer = Record(..m, rank: 50, payload: <<"newer":utf8>>)
  let before = State(..state, records: [n, their_n, u, a, their_a, newer, q])
  let assert Done(Ok(applied)) =
    freed(before, LeaseId("lease-1"), RecordKey("m"))
  assert applied.fired == [dead_letter]
  assert list.last(applied.state.dead_letters)
    == Ok(DeadLetter(
      pocket: PocketKey("pk"),
      key: RecordKey("m"),
      rank: 43,
      payload: <<"em":utf8>>,
      nacked_by: ConsumerId("c1"),
      nacked_at: 650,
      expires_at: ended_at + config.defaults().dead_letter_ttl,
    ))
  applied.fired
}

/// `record`, held, with a newer version deferred behind its held one: the
/// record's rank is 10 higher and its payload "newer", and the newer
/// version arrived at `arrival`. The held version, the mark and the nack
/// time stay.
fn behind(record: Record, arrival: Int) -> Record {
  case record.status {
    RecordHeld(..) as status ->
      Record(
        ..record,
        rank: record.rank + 10,
        payload: <<"newer":utf8>>,
        status: RecordHeld(..status, deferred_position: Some(arrival)),
      )
    RecordQueued -> record
  }
}

/// `superseded_ttl` and `dead_letter_ttl`, unlike each other and unlike the
/// lease's, so that a rule that reads the wrong one shows.
const shed_ttl = 5000

const letter_ttl = 7000

/// `ending(ended)` with a newer version deferred behind three of `pk`'s held
/// records: the unsettled `u`, at 6, arrived at 14; the acked `a`, at 3, at
/// 15; and `n`, at 8 and nacked at 700, at 16. Each arrival position is
/// above its record's position and below the counter, and no record stands
/// at one. `m` has nothing deferred. `other`'s `n` and `a` are listed first,
/// so that a lookup by key alone finds them. The config's two shed TTLs are
/// `shed_ttl` and `letter_ttl`.
fn deferred_ending(ended: LeaseStatus) -> State {
  let assert Ok(distinct) =
    config.new(
      config.Settings(
        ..config.defaults(),
        superseded_ttl: shed_ttl,
        dead_letter_ttl: letter_ttl,
      ),
    )
  let state = ending(ended)
  let assert [n, their_n, u, a, their_a, m, q] = state.records
  State(..state, config: distinct, records: [
    their_n,
    their_a,
    behind(n, 16),
    behind(u, 14),
    behind(a, 15),
    m,
    q,
  ])
}

/// `Freed` for `pk`'s unsettled `u`, with `v` deferred behind it, and
/// `ended_at` 777. The held version, rank 44 and payload "you", is added at
/// the end of the superseded queue with `shed_at` 777 and `expires_at` 777
/// plus `superseded_ttl`. The record is queued at the position it had, 6,
/// with the newer version's rank and payload and none of the held fields.
/// Nothing else in the complete state changes: the counter, the dead
/// letters, the other deferred records and `other`'s. The step is done at
/// once: the rule asked for no time.
fn supersede_held_exactly() -> List(String) {
  let state = deferred_ending(released)
  let assert [their_n, their_a, n, _, a, m, q] = state.records
  assert freed(state, LeaseId("lease-1"), RecordKey("u"))
    == Done(
      Ok(Applied(
        State(
          ..state,
          records: [
            their_n,
            their_a,
            n,
            Record(
              PocketKey("pk"),
              RecordKey("u"),
              54,
              <<"newer":utf8>>,
              6,
              RecordQueued,
            ),
            a,
            m,
            q,
          ],
          superseded: list.append(state.superseded, [
            SupersededRecord(
              pocket: PocketKey("pk"),
              key: RecordKey("u"),
              rank: 44,
              payload: <<"you":utf8>>,
              reason: Superseded,
              shed_at: ended_at,
              expires_at: ended_at + shed_ttl,
            ),
          ]),
        ),
        [supersede_held],
        None,
      )),
    )
  [supersede_held]
}

/// `Freed` for `pk`'s acked `a`, with a newer version deferred behind it at
/// 15. The acked version is consumed and recorded nowhere; the record is
/// queued at its arrival position, 15, not at 3, with the newer version's
/// rank and payload and none of the held fields. `other`'s `a`, listed
/// first, is not touched, and nothing else in the complete state changes:
/// neither shed queue gains an entry and the counter stays.
fn consume_deferred_exactly() -> List(String) {
  let state = deferred_ending(released)
  let assert [their_n, their_a, n, u, _, m, q] = state.records
  assert freed(state, LeaseId("lease-1"), RecordKey("a"))
    == Done(
      Ok(Applied(
        State(..state, records: [
          their_n,
          their_a,
          n,
          u,
          Record(
            PocketKey("pk"),
            RecordKey("a"),
            56,
            <<"newer":utf8>>,
            15,
            RecordQueued,
          ),
          m,
          q,
        ]),
        [consume_deferred],
        None,
      )),
    )
  [consume_deferred]
}

/// `Freed` for `pk`'s `n`, nacked at 700, with a newer version deferred
/// behind it at 16, and `ended_at` 777. The held version, rank 45 and
/// payload "en", becomes a dead letter at the end of the queue, nacked by
/// the lease's consumer at 700, expiring at 777 plus `dead_letter_ttl`. The
/// record is queued at its arrival position, 16, with the newer version's
/// rank and payload and none of the held fields. `other`'s `n`, nacked at
/// 520 and listed first, is not touched. Nothing else in the complete state
/// changes, and the rule asked for no time.
fn dead_letter_deferred_exactly() -> List(String) {
  let state = deferred_ending(released)
  let assert [their_n, their_a, _, u, a, m, q] = state.records
  assert freed(state, LeaseId("lease-1"), RecordKey("n"))
    == Done(
      Ok(Applied(
        State(
          ..state,
          records: [
            their_n,
            their_a,
            Record(
              PocketKey("pk"),
              RecordKey("n"),
              55,
              <<"newer":utf8>>,
              16,
              RecordQueued,
            ),
            u,
            a,
            m,
            q,
          ],
          dead_letters: list.append(state.dead_letters, [
            DeadLetter(
              pocket: PocketKey("pk"),
              key: RecordKey("n"),
              rank: 45,
              payload: <<"en":utf8>>,
              nacked_by: ConsumerId("c1"),
              nacked_at: 700,
              expires_at: ended_at + letter_ttl,
            ),
          ]),
        ),
        [dead_letter_deferred],
        None,
      )),
    )
  [dead_letter_deferred]
}

/// `Freed` names its lease only to reach `lease.pocket` and
/// `lease.consumer`; no rule reads the lease's status. `lease-0` is `c2`'s
/// lease over `pk`, already ended, by release or by expiry. A `Freed` that
/// names it reaches `pk`'s records all the same, each by its mark, and the
/// dead letter is nacked by that lease's consumer, `c2`, whichever way it
/// ended. The same holds for the three rules with a newer version deferred
/// behind the held one: the superseded held version, the queued record and
/// the dead letter are the same whichever way `lease-0` ended.
fn however_the_lease_ended() -> List(String) {
  list.flat_map([released, LeaseExpired(150)], fn(ended) {
    let state = ending(ended)
    let assert Done(Ok(requeued)) =
      freed(state, LeaseId("lease-0"), RecordKey("u"))
    let assert Done(Ok(consumed)) =
      freed(state, LeaseId("lease-0"), RecordKey("a"))
    let assert Done(Ok(lettered)) =
      freed(state, LeaseId("lease-0"), RecordKey("n"))
    assert #(requeued.fired, consumed.fired, lettered.fired)
      == #([requeue], [consume], [dead_letter])
    assert list.last(lettered.state.dead_letters)
      == Ok(DeadLetter(
        pocket: PocketKey("pk"),
        key: RecordKey("n"),
        rank: 45,
        payload: <<"en":utf8>>,
        nacked_by: ConsumerId("c2"),
        nacked_at: 700,
        expires_at: ended_at + config.defaults().dead_letter_ttl,
      ))
    assert lettered.state.leases == state.leases
    let deferred = deferred_ending(ended)
    let assert Done(Ok(superseded)) =
      freed(deferred, LeaseId("lease-0"), RecordKey("u"))
    let assert Done(Ok(queued)) =
      freed(deferred, LeaseId("lease-0"), RecordKey("a"))
    let assert Done(Ok(deferred_lettered)) =
      freed(deferred, LeaseId("lease-0"), RecordKey("n"))
    assert #(superseded.fired, queued.fired, deferred_lettered.fired)
      == #([supersede_held], [consume_deferred], [dead_letter_deferred])
    assert list.last(superseded.state.superseded)
      == Ok(SupersededRecord(
        pocket: PocketKey("pk"),
        key: RecordKey("u"),
        rank: 44,
        payload: <<"you":utf8>>,
        reason: Superseded,
        shed_at: ended_at,
        expires_at: ended_at + shed_ttl,
      ))
    assert model.record(
        queued.state.records,
        pocket: PocketKey("pk"),
        key: RecordKey("a"),
      )
      == Ok(Record(
        PocketKey("pk"),
        RecordKey("a"),
        56,
        <<"newer":utf8>>,
        15,
        RecordQueued,
      ))
    assert list.last(deferred_lettered.state.dead_letters)
      == Ok(DeadLetter(
        pocket: PocketKey("pk"),
        key: RecordKey("n"),
        rank: 45,
        payload: <<"en":utf8>>,
        nacked_by: ConsumerId("c2"),
        nacked_at: 700,
        expires_at: ended_at + letter_ttl,
      ))
    assert deferred_lettered.state.leases == deferred.leases
    [
      requeue,
      consume,
      dead_letter,
      supersede_held,
      consume_deferred,
      dead_letter_deferred,
    ]
  })
}

/// Whether the guard of the `Freed` rule with this name holds for this
/// record under `c1`'s lease.
fn guard_holds(rule: String, state: State, record_key: RecordKey) -> Bool {
  let assert Ok(row) =
    list.find(lease_end.freed().rows, fn(row) { row.name == rule })
  row.guard(state, Freeing(lease: LeaseId("lease-1"), record_key:, ended_at:))
}

/// Each rule's `requires: record.mark = ...` and its `requires:
/// record.deferred_position = null` or `!= null`, each shown to matter on
/// its own. `pk`'s `u` is unsettled, `a` acked and `n` nacked. With nothing
/// deferred the guard of each of the three rules with nothing deferred holds
/// for the record with its mark and for neither of the others, and no guard
/// of the three deferred rules holds for any. With a newer version deferred
/// behind each held one, at a higher rank with an arrival position, it is
/// the other way about: each deferred rule's guard holds for the record with
/// its mark alone. No guard holds for the queued `q` either way.
fn freed_guards() -> List(String) {
  let state = ending(released)
  let behind =
    State(
      ..state,
      records: list.map(state.records, fn(record) {
        case record.status {
          RecordHeld(held_rank:, held_payload:, mark:, nacked_at:, ..) ->
            Record(
              ..record,
              rank: record.rank + 10,
              payload: <<"newer":utf8>>,
              status: RecordHeld(
                held_rank:,
                held_payload:,
                mark:,
                nacked_at:,
                deferred_position: Some(record.position + 20),
              ),
            )
          RecordQueued -> record
        }
      }),
    )
  let rows = [
    #(requeue, "u", False),
    #(supersede_held, "u", True),
    #(consume, "a", False),
    #(consume_deferred, "a", True),
    #(dead_letter, "n", False),
    #(dead_letter_deferred, "n", True),
  ]
  list.each(rows, fn(own) {
    let #(rule, own_key, deferred) = own
    list.each(["u", "a", "n"], fn(key) {
      assert guard_holds(rule, state, RecordKey(key))
        == { key == own_key && !deferred }
      assert guard_holds(rule, behind, RecordKey(key))
        == { key == own_key && deferred }
    })
    assert !guard_holds(rule, state, RecordKey("q"))
    assert !guard_holds(rule, behind, RecordKey("q"))
  })
  []
}

/// A lease's end sends `Freed` only for a held record of its pocket. One
/// for a queued record, for a key the lease's pocket does not have, or
/// naming a lease id no lease has, finds no rule, which is a defect.
fn freed_defect() -> List(String) {
  let state = ending(released)
  let defect = Done(Error(Defect("Freed", NoRuleHeld)))
  assert freed(state, LeaseId("lease-1"), RecordKey("q")) == defect
  assert freed(state, LeaseId("lease-1"), RecordKey("absent")) == defect
  assert freed(state, LeaseId("lease-9"), RecordKey("u")) == defect
  assert freed(state, LeaseId("pk"), RecordKey("u")) == defect
  // `m` is held in `pk` and `other` has no `m`: the record is looked up in
  // the lease's own pocket.
  assert freed(state, LeaseId("lease-2"), RecordKey("m")) == defect
  []
}

/// Each `Freed` rule's effect reads the held record its guard saw. Where
/// there is none, which only a wrong guard could bring about, the outcome is
/// a defect that names the rule: a value, not a crash. Each rule is run
/// under a guard that always holds, for the queued `q`.
///
/// `DeadLetterNackedRecord` and `DeadLetterNackedVersionQueueDeferred` also
/// read the record's `nacked_at`, which their guards do not name and
/// `NackedHasItsTime` promises. A nacked record without one, which no rule
/// leaves, cannot become a dead letter: that is a defect too.
///
/// `ConsumeAckedVersionQueueDeferred` and
/// `DeadLetterNackedVersionQueueDeferred` queue the record at its arrival
/// position, which their guards promise. Run without their guards for a
/// record with nothing deferred, each is a defect.
fn freed_without_record() -> List(String) {
  let state = ending(released)
  let unguarded = fn(row: domain.Row(lease_end.Freeing)) {
    Table(
      "Freed",
      [Row(row.name, fn(_state, _freeing) { True }, row.effect)],
      Defective,
    )
  }
  list.each(lease_end.freed().rows, fn(row) {
    assert core.run(
        unguarded(row),
        Freeing(
          lease: LeaseId("lease-1"),
          record_key: RecordKey("q"),
          ended_at:,
        ),
        state,
      )
      == Done(
        Error(Defect(
          "Freed",
          EffectFailed(row.name, "no held record q under lease lease-1"),
        )),
      )
  })
  let assert [n, their_n, u, a, their_a, m, q] = state.records
  let #(_, _, _, plain_m, _) = plain()
  let timeless =
    State(..state, records: [
      n,
      their_n,
      u,
      a,
      their_a,
      held(plain_m, Nacked, None),
      q,
    ])
  assert m != held(plain_m, Nacked, None)
  assert freed(timeless, LeaseId("lease-1"), RecordKey("m"))
    == Done(
      Error(Defect(
        "Freed",
        EffectFailed(dead_letter, "nacked record m has no nacked_at"),
      )),
    )
  let timeless_behind =
    State(..timeless, records: [
      n,
      their_n,
      u,
      a,
      their_a,
      behind(held(plain_m, Nacked, None), 16),
      q,
    ])
  assert freed(timeless_behind, LeaseId("lease-1"), RecordKey("m"))
    == Done(
      Error(Defect(
        "Freed",
        EffectFailed(dead_letter_deferred, "nacked record m has no nacked_at"),
      )),
    )
  list.each([#(consume_deferred, "a"), #(dead_letter_deferred, "n")], fn(own) {
    let #(rule, key) = own
    let assert Ok(row) =
      list.find(lease_end.freed().rows, fn(row) { row.name == rule })
    assert core.run(
        unguarded(row),
        Freeing(
          lease: LeaseId("lease-1"),
          record_key: RecordKey(key),
          ended_at:,
        ),
        state,
      )
      == Done(
        Error(Defect(
          "Freed",
          EffectFailed(
            rule,
            "held record " <> key <> " has no deferred_position",
          ),
        )),
      )
  })
  []
}
