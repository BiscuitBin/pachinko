---
id: T45
title: "Drop joining for publishers and consumers; a caller-supplied consumer id stays, accepted on first use"
status: todo
depends_on: []
parallel_with: []
branch: ticket/t45-drop-joining
estimated_size: L
---

# T45: Drop joining for publishers and consumers; a caller-supplied consumer id stays, accepted on first use

## Context

`docs/specs/pachinko.allium` has two rules under "Identity", `PublisherJoins` and
`ConsumerJoins`. Each creates an entity, `Publisher` or `Consumer`, and every other
trigger takes one of those entities: `Publish(publisher, …)`, `Pull(consumer, limit)`,
and `Ack`, `Nack`, `Renew` and `Release` with a `consumer`. Nothing in the module says
what an id that never joined does. Both implementations refuse it before any rule runs,
with refusals the module does not state.

A Python double of the library, `double/`, was built from the specification alone, and
its notes, `double/SPEC-NOTES.md`, put that gap to the maintainer as finding 4 (notes
N10, N18 and N22), and a second join with a taken id as part of finding 5 (note N12).
The maintainer ruled on both on 2026-11-24. This ticket lands the ruling in the
specification, the library, the service, the double and the handbook.

Nothing in this ticket re-opens the ruling. A finding that the ruling is wrong goes to
the maintainer as a question, never into a fix.

### The ruling (2026-11-24): drop joining

Joining is dropped, for publishers and for consumers.

- **Why.** The maintainer's reason: joining was never wanted, nothing plans to use it,
  the first use is one consumer with many workers and one or a few publishers, and it
  can be added later with intent. Keep things as simple as the goals allow.
- **Removed.** The rules `PublisherJoins` and `ConsumerJoins`; `entity Publisher` and
  the publisher argument of `Publish`; the invariants `UniquePublisherId` and
  `UniqueConsumerId`; the `UnknownPublisher` and `UnknownConsumer` refusals; the join
  routes of the HTTP surface.
- **Kept.** A caller-supplied consumer id on pull, renew, ack, nack and release,
  accepted on first use with no registration. `Lease.consumer`, `DeadLetter.nacked_by`,
  and one lease per consumer id.
- **A stated consequence, which the specification must say.** Two workers that send the
  same consumer id share one lease, and nothing catches that mistake once the join's
  refusal of a taken id is gone.
- **N12 is moot.** Whether a second join with a taken id is refused or succeeds and
  changes nothing has no subject once there is no join.

### What the ruling leaves open

Step 1 puts each of these to the maintainer. None is assumed, and no step before the
answers chooses one.

1. **How a consumer exists in the specification without a join.** Not ruled. It is
   either a `Consumer` entity created on first use, or a plain id value. The answer
   decides `Lease.consumer`, `facing consumer: Consumer` on the `Consuming` surface,
   `DeadLetter.nacked_by` and the `OneActiveLeasePerConsumer` invariant. "The options
   for the consumer" below gives the specification text each implies.
2. **What the `Publishing` surface faces.** It says `facing publisher: Publisher`, and
   the ruling removes `entity Publisher`. By validation rule 28 (below) the clause
   cannot stay as written. Two forms are open: the surface has no `facing` clause, which
   the reference allows since every clause is optional; or it faces a placeholder,
   `external entity Publisher {}`, which is a declaration the ruling did not name. Each
   trial diff of step 1 says which form it used, and the question is asked whichever
   the checker accepts.
3. **The publish routes' shape.** The ruling names "the join routes". Two publish routes
   also carry a publisher id: the batch route's path,
   `POST /publishers/{publisher_id}/publish`, and the single publish's body field
   `publisher_id`. Their new shape is not ruled: the batch route's new path, and whether
   a body that still sends `publisher_id` is ignored or refused. Neither body type sets
   `deny_unknown_fields` today, so serde would ignore the field if it were only removed
   from the struct.
4. **What a never-seen consumer id is answered.** Ruled: a pull by it is accepted. Not
   ruled, and each follows from point 1: what the lease read answers for an id that
   holds no lease and never pulled (today `Queue::lease_view` refuses it with
   `PullError::UnknownConsumer`, the only variant that method can return, and the route
   answers 404); and what an ack, nack, renew or release by such an id answers (with the
   join check gone, the Rust's next checks give `LeaseNotActive` for a lease id that is
   not active and `LeaseNotYours` for another consumer's lease, and the double gives
   `LeaseFenced` for both). The names and the order of the lease refusals are not this
   ticket's: see "Other tickets" below.
5. **The scenario ids S009 and S010.** Both scenarios go with the joins. The catalog,
   `docs/reference/testing.md`, says "The 93 scenarios" and states no rule for an id
   whose scenario is removed: leave a gap, keep a row marked removed, or renumber.
6. **How decision 0019 is revised.** See "Decision 0019" below.
7. **The base branch.** See the last bullet of "What exists today".

### The options for the consumer

The facts come from `allium-skill-reference/allium/language-reference.md`; read its
sections "External entities", "Join lookups", "Existence", "Entity creation" under
"Postconditions (ensures)", "Actor declarations", "Surface structure" and "Validation
rules" before step 1.

- Validation rule 28: a type in a `facing` clause must be a declared `actor` type or an
  entity type, internal, external or imported. An actor's `identified_by` has the form
  `EntityType where condition`. So a plain `String` cannot be what a surface faces.
- Validation rule 29: every field in `exposes` must be reachable from a binding the
  surface declares with `facing`, `context` or `let`. Every clause of a surface is
  optional.
- A lookup `Consumer{id: consumer_id}` must match at most one entity; where the fields
  do not identify one instance, the specification is ambiguous.
- An external entity "defines its structure but not its lifecycle", and the checker
  warns where one is referenced.

Each sketch below was written from the reference and has not been run through the
pinned checker. Step 1 runs each and quotes the result beside it.

**Option A: a `Consumer` entity created on first use.** `entity Consumer` stays, with
its `leases` and `active_leases`. `Lease.consumer: Consumer`,
`DeadLetter.nacked_by: Consumer`, `facing consumer: Consumer` and
`OneActiveLeasePerConsumer` (`for c in Consumers: c.active_leases.count <= 1`) stay as
written. A trigger that takes a `Consumer` cannot be formed for an id with no entity,
which is the gap N10 and N18 reported, so the consumer triggers take the id and a rule
creates the entity:

```text
rule ConsumerFirstSeen {
    when: Pull(consumer_id, limit)
    requires: not exists Consumer{id: consumer_id}
    ensures: Consumer.created(id: consumer_id)
}
```

What Option A leaves for the maintainer:

- Which actions create a `Consumer`: a pull that claims a pocket only, or also a pull
  that finds none (`PullFindsNoPocket`), or a refused ack.
- The sketch shares the `Pull` trigger with the three Pull rules, whose guards the
  module states are disjoint; the creation may have to sit inside those rules instead.
- The lookup by id needs ids to be unique, which is what `UniqueConsumerId` states, and
  the ruling removes that invariant. Put this to the maintainer as a question; do not
  keep or drop the invariant on your own reading.
- No rule removes a `Consumer`, so the set grows with every id ever used, and the
  service has no authentication (`docs/explanation/security-model.md`).
- A caller that has not yet pulled has no entity for the surface to face, so what the
  lease read answers for it is still unstated.

**Option B: a plain id value.** `entity Consumer` goes, with `leases` and
`active_leases`. The fields become `Lease.consumer: String` and
`DeadLetter.nacked_by: String`. The rules keep their text: `Lease{consumer: consumer,
status: active}` and `lease.consumer = consumer` then compare ids. The invariant is
written pairwise over leases, as `OneActiveLeasePerPocket` is:

```text
invariant OneActiveLeasePerConsumer {
    for a in Leases:
        for b in Leases:
            a != b and a.status = active and b.status = active
                implies a.consumer != b.consumer
}
```

What Option B leaves for the maintainer:

- By rule 28 the `Consuming` surface cannot face a `String`. It either omits `facing`,
  or faces a placeholder (Option C).
- With no `facing` binding, rule 29 needs another binding for the surface's `exposes`
  and for `let leases = Leases where consumer = consumer and status = active`. A
  `context lease: Lease where status = active` binds one, but a surface with a
  `context` is absent when no entity matches, and a consumer with no lease must still
  be offered `Pull`.
- The field's name, `consumer` or `consumer_id`.

**Option C: an entity the module references and does not govern.** A sub-form to ask
about, not a third model the ruling named. `external entity Consumer { id: String }`
replaces `entity Consumer`; `Lease.consumer`, `nacked_by` and
`facing consumer: Consumer` keep their text, and the module creates no `Consumer` and
states no lifecycle for one, which reads close to "accepted on first use with no
registration". Unverified, for step 1 to check: whether an external entity may carry
the `leases` and `active_leases` relationships; whether `for c in Consumers` ranges
over an external entity; and whether the checker's warning on an external reference
fails `just check-specs` or `just analyse-specs`.

Under every option the specification says, in the `Consuming` surface or beside
`OneActiveLeasePerConsumer`, that two workers sending one id share one lease and that
nothing detects it.

### What exists today

Re-read each at step 2; these were true when the ticket was written. Line numbers are
of the branch `build-python-double` on 2026-11-24.

The specification, `docs/specs/pachinko.allium`:

- The header's Includes line names `Publisher` and `Consumer` (line 14); the Vocabulary
  defines both as parties (lines 53 to 55). The terms stay; the entities are in
  question.
- `entity Publisher` (line 111) and `entity Consumer` with `leases` and `active_leases`
  (lines 115 to 120). `DeadLetter.nacked_by: Consumer` (line 248) and
  `Lease.consumer: Consumer` (line 259).
- The section "Identity" with `PublisherJoins` (line 306) and `ConsumerJoins`
  (line 312).
- Five rules take `Publish(publisher, pocket_key, record_key, rank, payload)`, at lines
  338, 369, 392, 422 and 439. No rule reads `publisher`.
- `Pull(consumer, limit)` at lines 512, 528 and 570, and `lease.consumer = consumer` in
  `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats`, `Renew`,
  `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket` (lines 604 to 695).
  `nacked_by: lease.consumer` at lines 916 and 940.
- `UniquePublisherId` (line 1006), `UniqueConsumerId` (line 1012) and
  `OneActiveLeasePerConsumer` (line 1037). The module has 35 invariants: 20 at the top
  level and 15 inside entities (`rg -c '^\s*invariant ' docs/specs/pachinko.allium`
  prints 35).
- `surface Publishing` faces `publisher: Publisher` and provides
  `PublisherJoins(publisher_id)` (lines 1109 to 1114). `surface Consuming` faces
  `consumer: Consumer`, binds `let leases = Leases where consumer = consumer and status
  = active`, and provides `ConsumerJoins(consumer_id)` (lines 1150 to 1173).

The library, `crates/libpachinko/`:

- `src/queue.rs`: the registries `publishers: BTreeSet<PublisherId>` (line 112) and
  `consumers: BTreeMap<ConsumerId, Option<LeaseId>>` (line 114), which is also the index
  of each consumer's active lease; `publisher_joins` (line 190) and `consumer_joins`
  (line 217); `lease_view` returning `Result<Option<LeaseView>, PullError>` (line 253);
  `active_lease`, which raises `PullError::UnknownConsumer` (line 266).
- `src/queue/publish.rs`: `publish(&mut self, publisher_id: &PublisherId, …)` and its
  `UnknownPublisher` check (lines 62 to 69).
- `src/queue/pull.rs`: `pull` calls `active_lease` (line 63) and a claim writes the
  registry (line 119). `src/queue/renew.rs`: `authorise` checks the registry before the
  lease (lines 162 to 171), and a release writes it (line 110). `src/queue/expiry.rs`
  line 151 clears it on a lapse. `src/queue/leases.rs`: `check` answers
  `LeaseNotActive`, then `LeaseNotYours` (lines 71 to 84).
- `src/error.rs`: `JoinError` with `PublisherExists` and `ConsumerExists` (lines 16 to
  22), `PublishError::UnknownPublisher` (line 38), `PullError::UnknownConsumer`
  (line 78), `LeaseActionError::UnknownConsumer` (line 141), and their unit tests.
- `src/ids.rs`: `PublisherId` (line 73), documented by `UniquePublisherId`, and
  `ConsumerId`, documented by `UniqueConsumerId` (line 77).
- `src/queue/inspect.rs`: `Snapshot.publishers` (line 74) and `Snapshot.consumers`
  (line 76), both public fields.
- `src/lib.rs`: the interface table's two join rows and every refusal column that names
  `UnknownPublisher` or `UnknownConsumer` (lines 22 to 30); the exports of `JoinError`
  (line 124) and `PublisherId` (line 128).
- Doctests and unit tests that join before they act, in `src/lib.rs`, `src/queue.rs`,
  `src/queue/expiry.rs`, `inspect.rs`, `publish.rs`, `pull.rs`, `renew.rs`, `settle.rs`
  and `types.rs`. Find them with the removed-names search under Verification.
- `tests/queue/identity.rs`: S009 `a_publisher_joins_once` (line 92), S010
  `a_consumer_joins_once` (line 114), and two untagged tests of the unknown-id refusals
  (lines 131 and 150). `tests/queue/settle.rs`: the `"nobody"` cases of S042 (line 115)
  and S049 (line 358). `tests/queue/lease.rs`:
  `an_unknown_consumer_is_refused_before_any_lease_guard` (line 339).
- `tests/queue/fixture.rs`: the checks `unique_publisher_id` (line 379),
  `unique_consumer_id` (line 383) and `one_active_lease_per_consumer`, which also holds
  the registry to the leases (lines 488 to 506). `tests/queue/invariants.rs`: the
  "all 35 invariants" in the module doc (line 3), the obligation tags at lines 411, 412
  and 416, and the reach labels `publish
  UnknownPublisher` and `pull`, `ack`, `nack`, `renew` and `release UnknownConsumer`
  (lines 498 to 529). `tests/queue/walk.rs` and `tests/queue/walk/model.rs`: the join
  steps and the model's unknown-id predictions; `walk.rs` lines 36 to 38 keep a
  publisher and a consumer unjoined so the refusals are reachable.
- `benches/workloads.rs`: `joined()` (line 109) joins one publisher and one consumer.
- `README.md` of the crate: the signatures and the refusal table (lines 26 to 50) and
  the example (lines 117 to 126).
- No `libpachinko-v*` tag existed in the drafting worktree (`git tag --list` printed
  nothing), so `just semver` compares against the merge base with `main`. Unverified
  for the day the ticket runs; step 2 checks.

The service, `crates/pachinko/`:

- `src/http.rs`: the routes `POST /publishers/{publisher_id}` and
  `POST /consumers/{consumer_id}` (lines 95 and 96) and their handlers (lines 511 to
  529); the batch route `POST /publishers/{publisher_id}/publish` (lines 98 to 101);
  `PublishBody.publisher_id` (line 397); the `Refusal` variants `PublisherExists`,
  `ConsumerExists`, `UnknownPublisher` and `UnknownConsumer`, their statuses and their
  codes `publisher_exists`, `consumer_exists`, `unknown_publisher` and
  `unknown_consumer` (lines 132 to 305); and the unit tests of each.
- `src/http/batch.rs`: the batch handler, which takes the publisher from the path.
- `src/actor.rs`: the commands `PublisherJoins` and `ConsumerJoins` (lines 62 to 71),
  the handle's `publisher_joins` and `consumer_joins` (lines 236 to 254), the
  `publisher_id` each publish command carries, `publish_each` (line 543), and unit
  tests that use a join as their filler command.
- `src/serve.rs`: tests that send `POST /publishers/p` and a body with `publisher_id`
  (lines 429 and 761).
- `tests/http.rs`: the `join` helper (line 77) and `joined` (line 156); the 409 and 404
  assertions (lines 333 to 344); the four `…_by_an_unjoined_consumer_is_unknown_consumer`
  tests (lines 617 to 654). `tests/batch_publish.rs`: `join` (line 85), `joined`
  (line 191) and `an_unjoined_publishers_batch_lands_nothing_and_shape_checks_come_first`
  (line 410).

Outside the workspace:

- `loadgen/src/client.rs`: `join_publisher` (line 108), `join_consumer` (line 114), the
  `publisher_id` body field (line 128) and the batch path (line 148).
  `loadgen/src/workloads.rs` (lines 183 to 195) and `loadgen/src/probe.rs` (lines 67,
  68, 147 and 188) call them. `loadgen/src/main.rs` line 26 prints the batch path.
  `loadgen/` is outside the workspace; `just test-loadgen` runs its tests and
  `just check` does not.
- `fuzz/fuzz_targets/` names no publisher, consumer or join.
- `.github/`, `.agents/`, `.claude/`, `.codex/`, `scripts/`, `Dockerfile` and `Justfile`
  name no join, no removed refusal, no `/publishers/` or `/consumers/` path and neither
  S009 nor S010 (`rg --hidden` over them printed nothing).

The double, `double/` (on `build-python-double` only):

- `pachinko_double/identity.py`: both joins, their triggers and tables.
  `pachinko_double/queue.py`: `publisher_joins` and `consumer_joins` (lines 239 to 243),
  the `UnknownPublisher` gate (line 250), the `UnknownConsumer` gate (line 342), and the
  `publishers` and `consumers` maps (lines 71 and 72).
- `pachinko_double/refusals.py`: `PublisherAlreadyJoined`, `ConsumerAlreadyJoined`,
  `UnknownPublisher` and `UnknownConsumer`; `exceptions.py` re-exports them.
- `pachinko_double/model.py`: `Publisher` and `Consumer` (lines 60 to 66).
  `pachinko_double/inspection.py`: `Snapshot.publishers` and `consumers` (lines 73 and
  74), `active_leases_by_consumer` indexed from `consumers` (line 105), and
  `_refuse_dangling_references`, which refuses a lease or a dead letter that names a
  consumer the snapshot does not hold (lines 110 to 126).
- `pachinko_double/invariants.py`: the header's "35 invariants" (line 3),
  `UniquePublisherId` (line 231), `UniqueConsumerId` (line 238) and
  `OneActiveLeasePerConsumer`, which ranges over `s.consumers` (line 266).
- `tests/test_identity.py` (S009, S010), `tests/fixtures.py` (line 169 and the joins at
  187 to 189), `tests/test_interface.py` (lines 43 to 46), `tests/test_invariants.py`
  (the "35" at line 1 and the controls at lines 183, 188 and 212), and the
  `impl-choice` tests the notes name: `tests/test_publish.py` line 47 (N10),
  `tests/test_pull.py` line 77 and `tests/test_settle.py` line 60 (N18),
  `tests/test_settle.py` line 207 and `tests/test_ack_nack.py` lines 91, 118 and 288
  (N22).
- `README.md`: the guide's apply-flow text and figure (lines 67 to 80), the exceptions
  table (line 102), the rule cascade (lines 320 and 321), the invariant table (lines 395
  to 400), the walkthrough's joins (lines 500 to 502) and "the 35 invariants" (line 26).
- `SPEC-NOTES.md`: findings 4 and 5, notes N10, N12, N18 and N22, the gate figure under
  N10, the figure under N15 that begins with "consumer joined?", N20's sentence
  comparing an unknown lease id with an unknown consumer, N28's example of a join, and
  the rulings table's row for finding 4.

The handbook and the root:

- `docs/reference/testing.md`: "all 35 invariants" (lines 134 and 469), "two
  publishers, three consumers" in the walk's domains (line 128), "The 93 scenarios"
  (the catalog's first line), rows S009 and S010 (lines 203 and 204), rows S042 and
  S049 (lines 236 and 243), and "The joins and the fill run before the timed window"
  (line 417). The catalog's descriptions never mention an unknown id, so the rows are
  found from the tests: each assertion of an unknown-id refusal was mapped to the
  `// scenario:` tag above it. Only S042 and S049 hold one (`"nobody"` in
  `tests/queue/settle.rs`, and `double/tests/test_ack_nack.py` lines 91 and 118). Every
  other such assertion is in a test with no scenario tag: `identity.rs` lines 131 and
  150, `lease.rs` line 339 (tagged `// obligation: guarantee.ActionsNameTheirLease`
  only), the `refusals` function of `crates/pachinko/tests/http.rs`, its four
  unjoined-consumer tests, the batch test, and the double's own unknown-id tests. Step
  2 repeats the mapping.
- `docs/reference/http-api.md`: the two join rows (lines 24 and 25), every
  `unknown_publisher` and `unknown_consumer` in the route table (lines 26 to 33), the
  lease routes' check order (line 132), the batch section (lines 158 and 189) and the
  code table (lines 258 to 261).
- `docs/reference/api.md` (lines 72 and 82), `docs/explanation/architecture.md`
  (lines 147, 149 and 218), `docs/explanation/security-model.md` (line 117, "can join
  under any id"), `docs/how-to/test-and-debug.md` (lines 83 to 103),
  `docs/project/repository-map.md` (line 110, "all 35 invariants") and
  `docs/project/terminology.md` (the Consumer and Publisher rows, lines 33 and 34).
- `README.md`: the sequence diagram's two joins (lines 44 and 45) and the "Run it"
  transcript (lines 125 to 131).
- `CHANGELOG.md`: both crates' sections are Unreleased. Lines 101, 113, 119, 148 and 158
  describe the joins, `UnknownConsumer`, the batch route's path and "all 35 invariants",
  all inside the two Unreleased sections.
- `tickets/T23-library-publish-pull.md` and `tickets/T28-invariant-walk.md` list S009
  and S010. They are history and are not edited.
- `main` did not hold `double/`, `double/SPEC-NOTES.md`, or the specification edits of
  2026-11-24 (the `HeldRecordsOldestFirst` guarantee, `a != b` in
  `ArrivalPositionUnoccupied`, the labels on restating `requires`) when this ticket was
  written. They were on the branch `build-python-double`.

### Today's refusals, and the contract after this ticket

Today, in the Rust (`crates/libpachinko/src/error.rs`): a join, `JoinError::PublisherExists`
or `ConsumerExists`; a publish, `PublishError::UnknownPublisher` or `EqualRank`; a pull
or lease read, `PullError::UnknownConsumer`, and for a pull that claims,
`PullError::LeaseId` over `LeaseIdError::Exhausted` or `StillActive`; a renew or
release, `LeaseActionError::UnknownConsumer`, then `LeaseNotActive`, then
`LeaseNotYours`; an ack or nack, those three, then `RecordNotHeld`, then `MarkConflict`;
and `ConfigError`. `UnknownConsumer` is checked before the lease guards.

Today, in the double (`double/pachinko_double/refusals.py`): `PublisherAlreadyJoined`,
`ConsumerAlreadyJoined`, `UnknownPublisher`, `UnknownConsumer`, `EqualRankRefused`,
`LeaseFenced` where the Rust has two lease refusals, `RecordNotHeld`, `MarkConflict`,
`ConfigRefused`, and `PullRefused` and `ReleaseRefused`, which only a hand-built state
reaches.

After this ticket neither implementation has a join refusal, `UnknownPublisher` or
`UnknownConsumer`, and no wire code `publisher_exists`, `consumer_exists`,
`unknown_publisher` or `unknown_consumer`. Every other refusal keeps its name, its
order and its status code.

### Decision 0019

`docs/decisions/0019-service-runtime-tokio-and-http.md` fixes the wire as "PR 39's
routes, plus a batch publish route". It does not list the routes. Today's routes
include the two joins; that PR 39's did is an inference from that sentence and was not
checked against the pull request (step 2 checks, at the commit `cf1a6b2` the decision
cites). Removing routes that the decision fixed revises it. The handbook marks a
superseded decision and does not delete it (`docs/decisions/README.md`): decision 0011
supersedes 0004 in part, and decision 0020 carries a dated sentence for a ruling of
2026-11-24 that overtook one of its consequences. Which form this revision takes, a
dated note in 0019 or a new record that supersedes it in part, is point 6 of the stop.
A new record needs its entries in `docs/manifest.yml` and `docs/decisions/README.md`.

### Other tickets

- **T39** (`tickets/T39-decide-q2-q6.md`). Its undecided point 3 asks how an Operator
  exists in the specification and tells its executor to read what this ticket merged.
  The answer to this ticket's point 1 is the model T39 reads. T39 also says its
  suggested model adds "one more invariant than today's 35" and numbers new scenarios
  from S094; this ticket moves the invariant count and may add scenarios. Whichever of
  the two lands second re-reads the other's merged text and counts.
- **T43** (`tickets/T43-caller-contract.md`) names the specification's refusals and
  fixes their order, including one lease refusal or two. This ticket removes
  `UnknownPublisher`, `UnknownConsumer` and the two already-joined refusals from the
  list T43 works on, and names no refusal itself. If T43's follow-up has merged first,
  this ticket uses its names.
- **T42** (`tickets/T42-clock-model.md`) and T43's follow-up each revise decision 0019
  too: T42 for a lapsed lease, T43 for the lease id. Whichever lands later re-reads
  the decision as merged and adds to it without undoing the others.
- **T46** (`tickets/T46-no-claim-without-records.md`) changes the Pull rules and S031
  and S032. If it merged first, re-read the Pull rules before editing their trigger.

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The module passes
  `just check-specs` and `just analyse-specs` before any test or code changes.
- **A test edited to pass without a clause moving is a finding.** Every test this
  ticket changes names the clause that moved: a removed rule, a removed invariant, a
  changed trigger, or an answer from step 1. A test whose only change is that its
  setup no longer joins asserts what it asserted before.
- **A public API shrinks.** `Queue::publisher_joins`, `Queue::consumer_joins`,
  `JoinError`, three error variants, `Snapshot.publishers` and the `publisher_id`
  parameter of `Queue::publish` are public and go. `PublisherId` goes too unless step
  1's answer on the publish routes leaves a publisher id in use. `just semver` reports each
  removal; the report is quoted and each removal is in `CHANGELOG.md`. Each public item
  that stays or changes keeps `AGENTS.md` invariant 3 and has a doctest.
- **A wire break.** Removed routes answer 404 or 405 to a client that still joins. The
  load generator is such a client and moves in the same commit as the service.
- **Ids without registration are unbounded.** Any caller can name any consumer id. If
  the library or the double keeps an entry per id ever seen, its state grows without
  bound; say in the hand-back what each keeps and when an entry leaves.
- **The mistake nothing catches.** Two workers with one id share a lease. A test pins
  it, and the specification and `docs/reference/http-api.md` say it.
- **The small denominator.** Removing code removes tested lines; `just coverage` stays
  at or above 90%.
- **A gate that cannot fire.** The walk's reach test fails when a label is never
  reached; its label list loses the unknown-id labels in the same change that makes
  them unreachable, and no other label is removed.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links.

## Goal

The specification, the library, the service, the load generator, the double and the
handbook have no join: no `PublisherJoins` or `ConsumerJoins`, no `Publisher` entity, no
publisher argument on a publish, no `UniquePublisherId` or `UniqueConsumerId`, no
`UnknownPublisher` or `UnknownConsumer`, and no join route. `Queue::publish` takes a
publication and no publisher. A consumer id is supplied by
the caller on pull, renew, ack, nack and release and is accepted on first use. One
consumer id holds at most one active lease, and the specification says that two workers
sending one id share it. Everything else a caller can observe is unchanged.

## Non-goals

- No authentication, and no other way to tell two callers with one id apart.
- No names or order for the remaining refusals, and no change to what an action
  answers; those are T43's.
- No lease id in the specification (T43), no clock model (T42), no `limit >= 1` rule
  (T46), no start rule (T47).
- No Operator entity or actor; that is T39's point 3.
- No rewrite of a released changelog entry, a merged ticket or a decision record's
  history.
- No change to the publish rules' guards or outcomes beyond the removed argument.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The Identity section removed; `entity Publisher` removed; the consumer as step 1 decides; the `Publish` trigger without its publisher; the two invariants removed; both surfaces' `facing` and `provides`; the shared-lease consequence; the header's Includes line |
| `crates/libpachinko/src/queue.rs`, `queue/publish.rs`, `queue/pull.rs`, `queue/renew.rs`, `queue/settle.rs`, `queue/expiry.rs`, `queue/inspect.rs`, `queue/types.rs`, `error.rs`, `ids.rs`, `lib.rs` | shared, own lines | The joins, the registries, the checks, the error variants, the publish parameter, `PublisherId` if step 1 leaves it no use, the snapshot fields, the doctests |
| `crates/libpachinko/tests/queue/*.rs`, `tests/queue/walk/model.rs`, `benches/workloads.rs`, `README.md` | shared, own lines | The scenarios, the walk and its model, the invariant checks, the bench set-up, the signatures |
| `crates/pachinko/src/http.rs`, `http/batch.rs`, `actor.rs`, `serve.rs`, `tests/http.rs`, `tests/batch_publish.rs` | shared, own lines | The routes, the refusals and codes, the commands, the wire tests |
| `loadgen/src/client.rs`, `workloads.rs`, `probe.rs`, `main.rs` | shared, own lines | No join calls; the publish paths and bodies as step 1 decides |
| `double/pachinko_double/*.py`, `double/tests/*.py`, `double/README.md`, `double/SPEC-NOTES.md`, `double/diagrams/*` | shared, own lines, if the base holds it | The double's rules, model, snapshot, checker, tests and guide; findings 4 and 5 and notes N10, N12, N18 and N22 marked applied |
| `docs/reference/testing.md`, `http-api.md`, `api.md` | shared, own lines | The catalog rows and counts; the routes and codes; the refusal enums |
| `docs/explanation/architecture.md`, `security-model.md`, `docs/how-to/test-and-debug.md`, `docs/project/repository-map.md`, `terminology.md` | shared, own lines | The check order, "can join under any id", the transcript, the invariant count, the two terms |
| `docs/decisions/0019-service-runtime-tokio-and-http.md`, or a new record with `docs/manifest.yml` and `docs/decisions/README.md` | shared, own lines, or new | The revision, in the form step 1 decides |
| `README.md`, `CHANGELOG.md` | shared, own lines | The diagram and transcript; both Unreleased sections |
| `quint/pachinko.qnt`, `quint/mutants/*.patch`, `quint/traces/*`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines, if the tree this ticket lands on holds `quint/pachinko.qnt`, at the base or after the closing merge or rebase, and each path only where that tree holds it | `PublisherJoins`, `ConsumerJoins`, `UniquePublisherId` and `UniqueConsumerId` removed, the publish and consumer actions without a join, and the start every fixture is generated from; each mutant patch the edit breaks, re-cut, and a patch whose rule or property is gone, as the maintainer rules at this step's stop; the generator's witness list and its start; the trace fixtures, regenerated if they exist; the joins, the two rules and the two steps taken out of each replay |
| `tickets/README.md` | shared, own lines | T45's Status cell |
| `tickets/T45-drop-joining.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch and its worktree from `main`
   (`CONVENTIONS.md` §11). Run `just initialize` (network, authorised by this step).
   Run `just check` before any edit and quote its last line as the baseline. Check
   whether the base holds `double/` and the specification edits of 2026-11-24; if it
   does not, that is point 7 of step 1, and a different base means this step is run
   again there.
1. **Authorisation required: what the ruling leaves open.** Read the language
   reference sections Context names. For each of Options A, B and C, write the full
   specification diff, apply it to the module, run `just check-specs` and
   `just analyse-specs`, save the diff and both outputs under `ai_tmp/`, and put the
   module back with `git restore docs/specs/pachinko.allium`, which this step
   authorises for these trial edits only. Then ask the maintainer, quoting this
   ticket's Context and each option's diff and checker output, the seven points of
   "What the ruling leaves open", in order.

   **Adversarial review** (`CONVENTIONS.md` §14), before asking: the three diffs and
   the question list. Does each option remove everything the ruling removes and keep
   everything it keeps? Does any option's text decide a point the list says is open?
   Is a checker result quoted for each?

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these points is a new design: record
   it and stop again.
2. **The plan.** Use the `plan-change` skill. Re-read everything "What exists today"
   names, run the three searches under Verification that must match at this step, and run
   `just plan-spec pachinko`. Write down, before any edit: each clause that moves; each
   test that changes and the clause that moves it; each scenario removed, changed or
   added; each public item removed; each page to change.
3. **The specification.** Use the `spec-change` skill. Land the ruling with step 1's
   answers, and state the shared-lease consequence. Run `just check-specs` and
   `just analyse-specs`, then `just plan-spec pachinko` and note the obligations that
   went and came.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   ruling and step 1's answers. Is anything the ruling keeps gone? Does any rule's
   guard or outcome differ beyond the removed argument and the consumer's type? Is the
   consequence stated?
4. **The tests, red first.** Change the tests the plan lists and add one for each new
   answer: a pull by a never-seen id claims a pocket; two callers with one id share one
   lease; the lease read and each lease action by a never-seen id answer what step 1
   decided. Show each new or changed assertion failing against the unchanged code and
   quote the failures. Remove the tests whose rule is gone.
5. **The code.** Use the `rust-change` skill. The library first, then the service and
   the load generator, in one commit or in commits that each leave `just check` green.
   Run `just fmt-check`, `just clippy`, `just test`, `just coverage`, `just doc`,
   `just effects`, `just semver`, `just test-loadgen` and `just bench` (a build and one
   run; its timings are no gate) as the change needs them.
6. **The double**, if the base holds it: the same change, its tests, its guide, its
   figures and `double/SPEC-NOTES.md`, with `just python-double` green. In the notes,
   mark findings 4 and 5's N12 applied as finding 1 is marked, and update N10, N12, N18
   and N22, their `impl-choice` labels and each figure that draws the join gate.

   **The Quint model**, if the tree this ticket lands on holds `quint/pachinko.qnt`, at
   the base or after the closing merge or rebase onto `main`: that model is derived
   from this specification, so the same change updates it wherever it models what this
   ticket changes. Read the condition again after that merge: if it brought in the
   model, `quint/traces/` or a replay the base did not hold, this paragraph is done
   then, before any gate runs, each stop in it included. The rule is the same-change
   rule in `AGENTS.md` and the decision record under `docs/decisions/` that introduced
   the model; both exist wherever the model does.
   Here the two join rules and the two id invariants go, with their guards,
   effects, witnesses and the two join actions, and every action that required a join
   loses the requirement. A mutant patch cannot be re-cut as it stands when the rule
   it mutates or the property it names is one this ticket removes from the
   specification. **Stop and ask** before touching such a patch: name it, name the rule
   or the property that is gone, and put the two choices to the maintainer, to remove
   the patch or to aim its mutation at a property that remains. Do neither on your
   own, and never leave it failing.

   If `quint/traces/` exists, more than the fixtures moves, because they were generated
   from a start that had taken three joins and by a generator that asks for a trace
   reaching each join rule. In `scripts/quint-traces.py`: the two join rules leave the
   witness list, the two join steps leave the steps a fixture may hold, and the start
   becomes the model's `init`, since the joined start, `replayInit`, goes with the joins
   it was built from. In the Rust replay, `crates/libpachinko/tests/queue/walk/replay.rs`
   with `walk/model/trace.rs` beside it: the three joins it runs before a trace, the
   mapping of the two join steps and the two rules in the list its rule test requires
   all go. In the double's replay,
   `double/tests/test_replay.py`, if it exists: the joins it makes to reach a fixture's
   first state and the mapping of the two steps go.

   In this order: edit the model; re-cut each mutant patch under `quint/mutants/` that
   the edit breaks, keeping its mutation and the property it names, and list each in
   the hand-back; change the generator and run `just quint-traces`; change the replays
   until `just test` and, where the double's replay exists, `just python-double` are
   green; then run `just check-quint`, and `just verify-quint` if the recipe exists. The
   fixtures come before the two gates because both fail on a model whose hash the
   fixtures do not record. Where the model's header lists the behaviour under "Not yet
   modelled", change nothing there and say so in the hand-back.
7. **The handbook.** Use the `review-docs` skill for every page in Files touched, and
   revise decision 0019 as step 1 decided. Update every invariant count: 33 if the two
   invariants go and none is added, otherwise the count the module has. Run
   `just check-docs`.
8. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Then run
   `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- The maintainer's answers at step 1 are quoted in the hand-back with their date, and
  nothing before them chose an option.
- The module has no `PublisherJoins`, `ConsumerJoins`, `entity Publisher`,
  `UniquePublisherId` or `UniqueConsumerId`, and no `Publish` trigger takes a
  publisher. It states that a consumer id is accepted on first use, that one id holds
  at most one active lease, and that two workers sending one id share that lease
  undetected.
- `just check-specs` and `just analyse-specs` pass, and every `// obligation:` id in
  `crates/` is in `just plan-spec pachinko`'s output or names a `@guarantee`.
- The library has no join method, `JoinError`, `UnknownPublisher` or `UnknownConsumer`,
  and `Queue::publish` has no publisher parameter;
  the service has no join route and none of the four wire codes; the load generator
  joins nothing; the double has neither join, nor the four refusals.
- A library test and a wire test each show a pull by an id never seen before claiming
  a pocket, and two callers with one id sharing one lease. Each was red before the code
  changed.
- The lease read and each lease action by a never-seen id answer what step 1 decided,
  each pinned by a test.
- Every changed test is listed in the hand-back with the clause that moved it. No
  scenario outside that list has a changed assertion.
- The catalog's rows, its scenario count and every invariant count match the module
  and the tests; S009 and S010 are handled as step 1 decided.
- `double/SPEC-NOTES.md` marks finding 4 and finding 5's N12 applied, and no note or
  test still labels an unknown-id refusal `impl-choice`.
- Decision 0019 is revised in the form step 1 decided.
- The `just semver` report is quoted, and `CHANGELOG.md` lists each removed public item
  and each removed route and code under both Unreleased sections.
- Every public item that stays or changed keeps invariant 3 and has a doctest;
  `just coverage` reports at or above 90%; `just effects` passes.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on `rg`'s status 1, never on an error, and
follows a positive control over the same files. The searches name identifiers, never
the bare word "join": a record joins the held set, a pocket rejoins the order, and
`JoinSet`, `tokio::join!` and `.join(` are unrelated. The three searches that must
find nothing at the close are each run at step 2, before any edit, and each must match
then: the removed names in `docs/specs`, `crates`, `loadgen/src` and `docs/reference`,
the join routes, and the prose. That run is the positive control, and its matches are
quoted.

The searches leave out what is history: `docs/decisions/`, the tickets other than this
one, and `CHANGELOG.md`. In `double/SPEC-NOTES.md` the removed names stay where a note
or a ruling records what was asked and decided; read each match there instead of
requiring none.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
# the removed names; at step 2 this must match (exit 0), at the close it must not
names='PublisherJoins|ConsumerJoins|publisher_joins|consumer_joins|join_publisher|join_consumer'
names="$names|JoinError|AlreadyJoined|PublisherExists|ConsumerExists|publisher_exists|consumer_exists"
names="$names|UnknownPublisher|UnknownConsumer|unknown_publisher|unknown_consumer"
names="$names|UniquePublisherId|UniqueConsumerId|has (not|already) joined|never joined"
# if the base holds the double, add double/pachinko_double double/tests double/README.md
# none of the removed names is left: rg must exit 1, so this prints "exit 0"
rg -n "$names" docs/specs crates loadgen/src README.md \
  docs/reference docs/explanation docs/how-to docs/project
[ "$?" -eq 1 ]; echo "exit $?"
# no join route is left: rg must exit 1
rg -n 'POST "?(http://[^/ ]+)?/(publishers|consumers)/\{?[a-z_0-9]+\}?( |`|"|$)' \
  crates loadgen/src README.md docs/reference docs/how-to
[ "$?" -eq 1 ]; echo "exit $?"
# no prose still tells a reader to join: rg must exit 1
rg -n -i 'join as |join a publisher|can join|the joins ' README.md docs/reference docs/explanation docs/how-to
[ "$?" -eq 1 ]; echo "exit $?"
# the module's invariant count, to compare with every count in prose
rg -c '^\s*invariant ' docs/specs/pachinko.allium; echo "exit $?"
rg -n '\b3[0-9] invariants' crates docs double README.md; echo "exit $?"
just test; echo "exit $?"
just test-loadgen; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just semver; echo "exit $?"
# only if the base holds the double
just python-double; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

`PublisherId` is searched for separately once step 1 has said whether any publisher id
is left on the wire; if none is, it joins `names`.

## Hand-back notes

None yet.

## Open points

- The seven points of "What the ruling leaves open" are step 1's to settle.
- Whether the double moves in this ticket depends on the base step 1 names.
- Whether the `PublisherId` type goes follows from the answer on the publish routes.
- The checker's verdict on each option was not known when the ticket was written; step
  1 produces it.
