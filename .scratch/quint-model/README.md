# Tickets for a Quint model of the Pachinko queue

`docs/specs/pachinko.allium` decides what the queue does: 32 rules, 35 invariants and 12
surface guarantees. Two implementations follow it, the Rust crates and the Python double
under `double/`. Nothing checks the design itself exhaustively. The proptest walk in
`crates/libpachinko/tests/queue/walk.rs` samples 512 random walks, and the double is a
second derivation of the specification, not a proof of it.

These thirteen tickets write a Quint model of the queue, check it with Apalache at small
bounds, and then tie it to the Rust by replaying the model's traces. The Quint CLI 0.34.0
and Apalache 0.62.4 are already pinned and guarded (decision 0021,
`docs/decisions/0021-project-managed-quint-cli.md`, and the tickets under
`.scratch/apalache/`). No `.qnt` module exists yet, no gate runs Quint, and no document
says how a Quint model relates to the Allium. QM02 writes that document, decision 0022.

The tickets follow the format of `tickets/README.md`, "Ticket format", with three more
sections that `.scratch/apalache/AP02-checksummed-apalache.md` also has: "Design proposed
in this ticket", "Claims this ticket checks" and "Adversarial review". Each cites
`tickets/CONVENTIONS.md` by section and carries, word for word, the shared rules and the
review deltas printed below.

## Rulings these tickets rest on

The maintainer made these seven rulings on 2026-11-24 and 2026-11-25. Every ticket takes
them as fixed. A ticket that finds one cannot hold stops and asks.

1. **Purpose.** The model checks the design first. Tying the model to the
   implementations comes second.
2. **Gates.** `check-quint`, which runs `quint typecheck`, joins `just check`. A separate
   recipe, `verify-quint`, runs `quint verify` over every instance at stated bounds. It
   runs locally and in its own CI job, and that job joins `needs` of the one required
   `check` job from the day it lands.
3. **Sequencing.** The series starts now, against today's specification. A later
   specification ticket that changes modelled behaviour updates the model in the same
   change. The model never settles the Allium's two open questions.
4. **Verified.** Three things together: a bounded `quint verify` at the walk's small
   domains (2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3), a
   reachability witness for every rule, and seeded mutants that the checker catches. The
   domains are a ruling: a ticket that cannot fit them inside the CI timeout stops and
   asks, and never shrinks them. Inductive invariants are a follow-up, QM13.
5. **Lapse.** An untimed lapse lands first: any active lease may lapse at any moment.
   The clock, the timestamps, the TTLs and decay wait for the tickets T42 creates.
6. **Conformance.** The Rust replays the model's traces first. The Python double's replay
   is a follow-up, QM12.
7. **Location.** The model lives under `quint/` at the repository root.

## Before QM01 starts

- **The series base is the maintainer's choice.** The series needs the Quint CLI pin,
  AP01, AP02 and the Python double in its base. On 2026-11-25 they were on the branch
  `mk-quint-spec-tickets` (commit `cf1f3f3`) and not on `main`. If all of them have
  merged, the series base is `main`. If not, the maintainer names it before QM01's
  step 1.
- **Read "the series base" wherever `tickets/CONVENTIONS.md` §11 says `main`.** A ticket's
  worktree starts from the series base once every ticket in its `depends_on` has merged
  there. Its closing fetch, its merge or rebase and its fresh evidence are against the
  series base too.
- **The records under `.scratch/apalache/` are tracked.** A ticket of this series may
  cite them for what was run, where `tickets/CONVENTIONS.md` §11 bars a scratch
  citation. A command a step needs is still written out in the ticket.
- **A red control of the hash check is not a gate run.** Shared rule 11 has the
  fixtures regenerated before any gate runs, so that no gate is read against stale
  fixtures. A control a ticket states, which runs `just check-quint`,
  `just quint-traces check` or `just verify-quint` for its failure on stale fixtures, is
  not such a run, as in QM11's step 4. It is never quoted as a pass.
- **Commit these files first.** An untracked file does not appear in a new worktree.
  Before step 1, `git ls-files .scratch/quint-model/` in the new worktree must list this
  README and the ticket being run.
- **Decision 0022 is the next free number today** (`docs/decisions/README.md` says "The
  next decision is 0022"). If another change has taken it by the time QM02 runs, QM02
  takes the next free number and every later ticket reads "0022" as that number. A
  later ticket finds the record with
  `rg -l '^canonical_for: \[decision_quint_model\]' docs/decisions`, which prints one
  path, never by its number.
- **A worktree in flight fails `just check` after QM02 merges into its base**, until it
  runs `just install-quint` (network). `check-quint` needs the CLI, and only
  `just initialize` and `just install-quint` install it.
- **The paths are a proposal.** A rename of `quint/`, of a recipe or of a pin file
  applies to all thirteen tickets and to the shared rules.
- **Later tickets are written against recommended answers.** Several tickets put a
  design point to the maintainer (see "Design points a ticket settles"). The tickets
  after them assume the recommended answer. If the maintainer rules otherwise, the
  ruling ticket edits the sentences the ruling changes in the later tickets and in this
  README, and lists them in its hand-back. The seven T tickets that carry the model's
  clause name paths and commands that QM02, QM03, QM10 and QM12 propose; a ruling that
  moves one is carried into those tickets too, where they are still `todo`.

## Design fixed before QM01

The first block is the maintainer's, from the plan this series came from.

- **One state machine in one file that grows.** `quint/pachinko.qnt` holds one module,
  `pachinko`, whose sections follow the Allium's order. The domains and the config
  values are `const`s.
- **Bounds are instance modules.** Each sits at the foot of `quint/pachinko.qnt`, is
  named `pachinko_<bound>`, and imports `pachinko` with its constants set. The first is
  `pachinko_walk`. "Every module", for `verify-quint`, means every instance module.
- **Rule names are verbatim.** A sum type `Rule` has one variant for each Allium rule
  the model has so far, spelt as the Allium spells it. The ticket that owns a rule adds
  its variant.
- **One action per cascade.** A pull with its `HoldRecord` steps is one action. A lease
  end with its `Freed` rules is one action. A publish with the revival its
  `PocketTouched` causes is one action. The Allium claims its invariants only after the
  whole chain, so no intermediate state exists in the model.
- **A ghost set of fired rules.** The variable `fired: Set[Rule]` holds the rules the
  last step fired, inner rules included.
- **Top-level actions mirror the walk's step vocabulary.** They are named after
  `WalkStep` in `crates/libpachinko/tests/queue/walk.rs`: `publisherJoins(publisher)`,
  `consumerJoins(consumer)`, `publish`, `pull`, `ack`, `nack`, `renew`, `release`, and
  from QM09 `advance(millis)` and `tick`. An argument that is a named field in
  `WalkStep` keeps its name.
- **Mutants are patches.** Each is a unified diff against `quint/pachinko.qnt`, kept
  under `quint/mutants/` and applied to a scratch copy. No glob sweeps a mutant into
  `check-quint`.
- **No clock before QM09.** QM02 to QM08 omit `now` and every timestamp field.
- **The model never leads the specification.** A counterexample that shows the Allium is
  wrong, or silent, stops the ticket and goes to the maintainer as a question. It is
  never answered by weakening an invariant or by changing the model alone.
- **Excluded, with the reason stated in decision 0022:**
  - liveness: `ExpiryRecoversWithinTtl` is checked only as safety fragments;
  - the eviction order under `NeverRefusedForCapacity`, because the Allium has no rule
    for it and bounds no memory;
  - both open questions.

The second block was added on 2026-11-25, after Codex (Sol) reviewed this README. The
plan said the fired-rules ghost alone gives each rule its witness and turns a guarantee
about two states into an invariant over one. It does not: a label can be set by a step
that did nothing, and a set of labels cannot say in what order a batch was handed out.
QM02 is the first ticket to build on this block, and its first checkpoint puts the
block to the maintainer.

- **Every step is total.** A step in the domain that no rule answers is still a step.
  It changes nothing and leaves `fired` empty, as the Allium says of a publish at the
  stored rank: it "is rejected and changes nothing". The model then has no deadlock,
  and a refused step can be replayed. `step` offers each action over the whole of its
  domains whatever the state holds. No property can show that, since a step that is
  not offered is in no trace, so each model ticket's diff checkpoint reads `step` for
  it. A property, `UnansweredStepChangesNothing`, checks the rest: when `fired` is
  empty the state equals `prev` and `answer` is nothing. From QM09 the clock is the one
  exception, and the property states it by the step `last` names: after `advance`, and
  after a `tick` that finds nothing due, the clock may differ from `prev` as the action
  says and everything else is equal; after any other step with `fired` empty the clock
  is equal too.
- **Three more ghosts.** `last` is the step taken, with its arguments: a sum type `Step`
  with one variant per top-level action, plus `Init`. `answer` is what the step handed
  back: a batch, with its lease, its pocket and its records in order, or
  `NoPocketAvailable`, or nothing. `prev` is the state before the step. `step` writes
  `last` from the arguments it chose, and from QM10 `replayStep` does the same for the
  actions it offers. No action writes it, so that an action cannot report a step other
  than the one taken. No guard and no effect on the state reads a ghost.
- **A guard and an effect per rule.** `guard_<Rule>` is a pure definition over a state
  and the arguments of the trigger the rule answers. `effect_<Rule>` is an invariant: if
  `fired` holds the rule, then what the Allium's `ensures` says holds between `prev` and
  the state. It is transcribed from the Allium a second time, apart from the action,
  and it states its frame: what neither the rule nor another rule fired in the same
  step writes is equal in `prev` and the state. On a step that runs several stages, as
  QM09's `tick` does, "between `prev` and the state" reads "between the states before
  and after the stage that fired the rule". One more invariant,
  `FiredMatchesGuards`, says that `fired` holds exactly the rules whose guard held for
  the step `last` names: in `prev`, or, for a rule later in a chain, in the state the
  chain had reached. Where two rules of one chain write the same field, as a publish
  and a revival both advance the counter, their effects read it through one shared
  definition; the ticket that first meets the case adds the definition and says which
  earlier effect it edits.
- **Properties are named apart from everything else.** A property is a `val` with no
  parameter whose name starts with an upper-case letter, or `effect_<Rule>`. Every other
  definition starts with a lower-case letter. No definition uses Quint's `invariant`
  qualifier, which the CLI would collect by default. A part of a guarantee that a
  ticket checks before the guarantee's owner lands is a property of its own, named
  `<Guarantee>_<part>` or for what it says, and the owner keeps it.
- **A witness rests on those.** `witness_<Rule>` is `fired.contains(<Rule>)`. It means
  something because `FiredMatchesGuards` and `effect_<Rule>` hold.
- **The state can hold what an invariant forbids.** Leases are entities with an identity
  of their own, ended ones included, not a map from a pocket to its lease. The two shed
  queues are lists in creation order, not sets, so two identical stale publishes make
  two entries. An invariant may be true by representation only where the Allium's own
  notion of identity makes it so; decision 0022 lists each with the reason.
- **Every invariant is transcribed whole.** A condition that is always true in the model
  so far stays in the transcription (`HeldCopiesAgree` keeps
  `deferred_position = null` before deferral exists), so that a later ticket changes
  no earlier property.
- **`pachinko_walk` carries the walk's configuration as well as its domains.** The
  names are the walk's: publishers `p0` and `p1`, consumers `c0` to `c2`, pockets `a`,
  `b` and `c`, record keys `k0` to `k2`. Pull limits are -1 to 3. A payload is the
  publish's own tuple, as the walk builds it, so it is not a free domain.
  `max_pull_batch` is 2 and `max_consecutive_expiries` is 2. From QM09, `lease_ttl` is
  100 ms, `superseded_ttl` 150 ms and `dead_letter_ttl` 250 ms. Each constant enters
  with the ticket that first reads it.
- **Two step relations.** `step` offers every action and is what `verify-quint` checks.
  `replayStep` offers only the actions that have a `WalkStep`, and is what the trace
  fixtures are generated from. The untimed lapse of QM08 is a model-only action,
  `lapse(lease)`, in `step` alone.
- **A canary.** Every instance carries a property that is false on purpose.
  `verify-quint` requires it to be violated before it trusts a pass on that instance,
  and names every property it checks. It never relies on which properties the CLI
  finds by default.

A scratch module of this shape (a `Rule` sum type, `guard_<Rule>` definitions, one
record of state, `fired`, `last`, and an instance that imports the module with its
constants) passed `just quint typecheck` with the pinned CLI on 2026-11-25. Nothing else
about Quint was run for this series. Every other claim about the CLI's behaviour is in
"Claims the tickets check".

### Names every ticket uses

| Thing | Name | Created by |
| --- | --- | --- |
| The model | `quint/pachinko.qnt`, module `pachinko` | QM02 |
| An instance | module `pachinko_<bound>` at the foot of the model; the first is `pachinko_walk` | QM02 |
| The ghosts | `fired`, `last`, `answer`, `prev` | QM02 |
| The step relations | `step`, and `replayStep` | QM02; QM10, extended by QM11 |
| A rule's guard, effect and witness | `guard_<Rule>`, `effect_<Rule>`, `witness_<Rule>` | the rule's owner |
| The label checks | `FiredMatchesGuards`, `UnansweredStepChangesNothing` | QM02 |
| The gate's script | `scripts/check-quint.sh` | QM02 |
| An Allium invariant or guarantee as a property | its Allium name, unchanged | its owner |
| The canary | `Canary` | QM02 |
| A mutant | `quint/mutants/<ticket>-<slug>.patch`, the ticket id in lower case | the ticket that lists it; QM03 for the first |
| The typecheck gate | `just check-quint` | QM02 |
| The verify recipe | `just verify-quint`, running `scripts/verify-quint.sh` | QM03 |
| The bounds | `quint/verify.txt` | QM03 |
| The verify CI job | `quint`, in `.github/workflows/ci.yml` | QM03 |
| The evaluator's pin | `tools/quint/evaluator.txt` | QM01 |
| The evaluator's installer | `just install-quint-evaluator`, running `scripts/install-quint-evaluator.sh` | QM01 |
| The evaluator's pin check | `scripts/check-quint-evaluator-pin.sh`, in `just lock-check` | QM01 |
| The trace fixtures, their start and their recipe | `quint/traces/`, `replayInit`, `just quint-traces` | QM10 |
| A ticket's scratch | `ai_tmp/<ticket>/`, the ticket id in lower case | each ticket |

### Design points a ticket settles

These are not fixed here. The named ticket proposes an answer under "Design proposed in
this ticket", the maintainer rules, and later tickets take the ruling as fixed.

| Point | Settled in |
| --- | --- |
| The layout of the evaluator's pin file, one hash per platform, and where the stamp lives | QM01 |
| Which `just quint` commands need the evaluator, and what the guard does with `--backend typescript` | QM01 |
| The second design block above, as a whole | QM02 |
| How a lease is identified | QM02 |
| How the pocket order is written, in one section of the model | QM02 |
| How a witness is run: one `--observation` per run, or a negated invariant that must be violated | QM03 |
| What counts as a mutant caught by its named property, given that the CLI's exit status does not say | QM03 |
| The format of `quint/verify.txt`, `--max-steps` for each instance, and the CI job's `timeout-minutes` | QM03 |
| What `verify-quint` shows about the Apalache server that answers, and what it cannot | QM03 |
| What a step by a publisher or a consumer that has not joined does, the counter's first value, and which invariants are true by representation | QM04 |
| Whether a config bound is an `assume` or an invariant over the constants | QM05 |
| How the clock moves, against whatever T42's ruling lands | QM09 |
| The fixture format, the seed, how the model's hash is recorded, how a trace starts where the walk starts, and how the replay lists what it does not compare | QM10 |
| Which `tick` steps and which `advance` steps can be replayed | QM11 |

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Size | Status |
| --- | --- | --- | --- | --- | --- |
| QM01 | The Rust evaluator pinned by checksum | `QM01-evaluator-pin.md` | none | M | done |
| QM02 | Tracer: claim and release, decision 0022 and check-quint | `QM02-tracer-claim-and-release.md` | QM01 | L | todo |
| QM03 | The verify recipe and its required CI job | `QM03-verify-recipe-and-ci.md` | QM02 | M | todo |
| QM04 | Publish with nothing held | `QM04-publish.md` | QM03 | M | todo |
| QM05 | Pull and the held set | `QM05-pull-and-held-set.md` | QM04 | L | todo |
| QM06 | Ack, nack, renew and the lease's end | `QM06-ack-nack-renew-lease-end.md` | QM05 | L | todo |
| QM07 | Deferral behind the held version | `QM07-deferral.md` | QM06 | L | todo |
| QM08 | The untimed lapse and parking | `QM08-untimed-lapse-and-parking.md` | QM07 | M | todo |
| QM09 | The clock, timestamps and decay | `QM09-clock-timestamps-decay.md` | QM08, T41, and the specification ticket T42 creates | L | todo |
| QM10 | Conformance: the Rust replays the model's traces | `QM10-rust-replay.md` | QM07 | M | todo |
| QM11 | Conformance: lapses and time | `QM11-replay-lapses-and-time.md` | QM09, QM10 | S | todo |
| QM12 | Follow-up: the Python double replays the same traces | `QM12-double-replay.md` | QM10 | M | todo |
| QM13 | Follow-up: inductive lease exclusivity | `QM13-inductive-exclusivity.md` | QM08 | M | todo |

## Dependencies

```text
QM01 ─> QM02 ─> QM03 ─> QM04 ─> QM05 ─> QM06 ─> QM07 ─┬─> QM08 ─┬─> QM09 ─┬─> QM11
                                                      │         └─> QM13  │
                                                      └─> QM10 ───────────┘
                                                            └─> QM12
T41 ─> T42 ─> (the specification ticket T42 creates) ─> QM09
T41 ─────────────────────────────────────────────────> QM09
```

- **QM01 goes first** so that no later ticket sees a counterexample, a witness or a
  trace through a download nothing checked.
- **QM02 to QM08 are a chain because reachability forces it.** An idle pocket needs an
  ack and a release. A parked pocket needs a lapse. A deferred version needs a held
  record.
- **QM09's last blocker has no id yet.** T42 explores the clock model, has it ruled, and
  creates the tickets that land it. QM09 waits for the one that changes
  `docs/specs/pachinko.allium`. QM09's `depends_on` is `[QM08, T41, T42]`, where `T42`
  stands for that ticket. Whoever creates it replaces `T42` with its id, in QM09's
  frontmatter and in the index above.
- **QM11 also needs the Rust to hold the ruled clock.** If the ticket that lands the
  clock in the Rust is not the specification ticket, QM11 waits for it as well.
- **Parallel pairs.** QM08 and QM10 may run in parallel. QM12 may run in parallel with
  any ticket that is neither its ancestor nor its descendant in the graph, and so may
  QM13. Each pair shares `quint/pachinko.qnt` or `quint/traces/` and merges one at a
  time under `tickets/CONVENTIONS.md` §11, "Wave 2 shares files by rebasing or merging".
  The one that merges second rebases, reruns its verification, and applies shared rule
  11 to whatever the rebase changed.

## What each ticket delivers

| Id | Delivers | Controls and mutants, at least |
| --- | --- | --- |
| QM01 | The pin file, the installer into `.quint/`, the pin check in `just lock-check` (offline, reading only committed files, because CI runs it without the CLI), the `just quint` guard extended to every command that uses the evaluator, the installer in `scripts/initialize.sh` and in the network-recipe lists of `AGENTS.md`, `docs/reference/commands.md` and `docs/reference/quality-gates.md`, and a dated note in decision 0021 | A tampered binary is refused; `just quint run` succeeds with the external network denied |
| QM02 | Decision 0022. The model with three rules over a seeded start, the four ghosts, `Canary`, and `pachinko_walk`. `check-quint` in the `Justfile`, in the recipe list in `pyproject.toml` and in `docs/reference/quality-gates.md`; it checks that the installed CLI is the pinned version, and fails on zero files. `just install-quint` before every caller of the gate: the CI job that runs `check-quint`, and `.github/workflows/release.yml`. `quint/` in the `AGENTS.md` project map, the same-change rule in `AGENTS.md` and the `spec-change` skill, and the handbook pages. The two sentences of decision 0021 that the gate makes false. By hand: the canary violated, a bounded `verify` that passes, one witness and one mutant | An ill-typed scratch module fails the gate; the gate fails on zero files and on a CLI that is not the pinned version; one exclusivity mutant is caught by hand |
| QM03 | `just verify-quint` and `quint/verify.txt`; the CI job `quint`, which installs the CLI, Apalache and the evaluator and joins `needs` of `check`; decision 0021 amended and 0022 extended; `just verify-quint` added to the same-change rule in `AGENTS.md` and the `spec-change` skill; witnesses for QM02's three rules and the first committed mutants | An unreachable witness, a surviving mutant, a patch that does not apply, a mutant that does not typecheck and a mutant caught by another property each fail the recipe; two exclusivity mutants are caught |
| QM04 | An empty `init`, the two joins, four publish rules, records and the superseded queue | A supersede moves the position; a stale publish is dropped silently; one of two identical stale publishes is lost |
| QM05 | The full pull: positive limits, `HoldRecord`, the held set, a release that requeues | A batch exceeds the cap; a held key is handed again; a batch skips the oldest record; a second consumer is handed a record of a leased pocket |
| QM06 | Marks, the dead-letter queue, the idle pocket and its revival | A nack after an ack is accepted; an acked record is requeued; an ack naming an ended lease lands on its successor; a pocket idles with a record left |
| QM07 | Deferral, and all six rows of the lease's end | A deferred version is lost; an arrival position is reused; a publish onto a held key changes the held version; a deferred version is handed out under the same lease |
| QM08 | `lapse(lease)` for any active lease, the three expiry rules, parking and its revival, and why an untimed lapse covers every timed behaviour | The count is not reset by an ack; parking is skipped; a lapse requeues an acked record; an expiry moves the pocket to the back |
| QM09 | `now`, every timestamp, the TTLs, both decay rules, the stamps on earlier rules, `advance` and `tick` in `step`, and a second instance for zero TTLs | An expiry stamped before its grant; a lease extension that does not extend; a dead letter's expiry measured from `nacked_at` instead of its lease's end; a decay before `expires_at` |
| QM10 | `replayStep`, the fixtures under `quint/traces/`, a Rust test beside the walk that replays them and compares state and answers, `serde_json` as a dev-dependency with its decision-0007 note, the offline hash check in `check-quint`, and regeneration with a diff in `verify-quint` | A hand-edited fixture fails the test; a one-character model edit fails the hash check; zero traces fail both |
| QM11 | `replayStep`, the fixtures and the mapping extended to the `advance` and `tick` steps that can be replayed | A fixture with a wrong expiry fails |
| QM12 | A test under `double/tests/` that replays the fixtures and compares the double's snapshot and fired rules; the double README's "not a test oracle" line amended | A hand-edited trace fails; an edited `fired` set fails on the rule comparison alone |
| QM13 | An inductive invariant that implies the two exclusivity invariants, in `verify-quint`; or, by the maintainer's ruling at QM13's stop, a recorded finding of why the pinned tools cannot check one | A weakened invariant is shown not to be inductive |

Until QM05 lands, a pull's limit ranges over -1 and 0 only, so a claim hands out
nothing, which is what the Allium says of a limit that is not positive. QM02's seed
joins the three consumers and queues two pockets, `a` and `b`, each with one queued
record. The record lets `ReleaseRequeuesPocket` fire. Two pockets for three consumers
lets `PullFindsNoPocket` fire: with three pockets, a consumer with no lease always finds
one queued.

## What each ticket owns

Each of the 32 rules, 35 invariants and 12 guarantees is owned by exactly one ticket. A
ticket repeats its part of this section as an "Owns" table under its Goal.

- **A rule** is owned by the ticket that adds it to the model with its guard, its effect
  and its witness. A later ticket may extend it, and that is not ownership.
- **An invariant** is owned by the ticket that adds it as a checked property.
- **A guarantee** is owned by the ticket that states it as a property in the form it
  keeps, and checks every case the model has at that point. Owning a guarantee does not
  mean every case has been checked: "Coverage a later ticket adds" lists the rest, and a
  guarantee is checked in full only when the last ticket in its row there has landed.

Every property, once stated, runs against the whole model in every later ticket,
because `verify-quint` runs them all.

| Ticket | Rules | Invariants | Guarantees |
| --- | --- | --- | --- |
| QM01 | 0 | 0 | 0 |
| QM02 | 3 | 6 | 0 |
| QM03 | 0 | 0 | 0 |
| QM04 | 6 | 6 | 0 |
| QM05 | 3 | 4 | 4 |
| QM06 | 9 | 1 | 1 |
| QM07 | 4 | 6 | 5 |
| QM08 | 5 | 3 | 1 |
| QM09 | 2 | 9 | 1 |
| QM10 to QM13 | 0 | 0 | 0 |
| Total | 32 | 35 | 12 |

### Rules

| Ticket | Rules owned |
| --- | --- |
| QM02 | `PullClaimsFrontPocket`, `PullFindsNoPocket`, `ReleaseRequeuesPocket` |
| QM04 | `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`, `DropStalePublish` |
| QM05 | `PullWithinLease`, `HoldRecord`, `RequeueUnsettledRecord` |
| QM06 | `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats`, `Renew`, `ReleaseIdlesEmptyPocket`, `ConsumeAckedRecord`, `DeadLetterNackedRecord`, `ReviveIdlePocket` |
| QM07 | `DeferBehindHeldVersion`, `SupersedeUnsettledHeldVersion`, `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedVersionQueueDeferred` |
| QM08 | `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket` |
| QM09 | `SupersededRecordDecays`, `DeadLetterDecays` |

Extended, not owned:

- QM04 adds "the consumer has joined" to QM02's two pull guards, whose seed had every
  consumer joined.
- QM05 extends `PullClaimsFrontPocket` with its `Handed` chain and the positive limits,
  and `ReleaseRequeuesPocket` with its `Freed` chain.
- QM07 extends `Supersede` with its second case, a held record with a version already
  deferred. QM04 lands the queued case.
- QM08 calls the six lease-end rules from a lapse. QM05 to QM07 own them. It adds
  `consecutive_expiries` to the state, and so to `PublishIntoNewPocket`, the two mark
  rules that reset it, both release rules and `ReviveIdlePocket`.
- QM09 adds the timestamps to every rule that stamps one, and their effects. It
  completes `FiredMatchesGuards` and `UnansweredStepChangesNothing` for the clock and
  the tick, and reads every effect stage by stage on a tick.

### Invariants

| Ticket | Invariants owned |
| --- | --- |
| QM02 | `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `UniquePocketPosition`, `PocketPositionsBelowNext` |
| QM04 | `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`, `UniqueRecordPosition`, `RecordPositionsBelowNext` |
| QM05 | `OutstandingOnlyWhileLeased`, `HeldVersionNotNewer`, `HeldCopiesAgree`, `PullBatchPositive` |
| QM06 | `IdleHasNoRecords` |
| QM07 | `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` |
| QM08 | `ExpiriesWithinLimit`, `MarkedPocketHasNoExpiries`, `ExpiryLimitPositive` |
| QM09 | `NackedHasItsTime`, `OnlyNackedHasNackTime`, `ShedBeforeExpiry`, `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant`, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative` |

### Guarantees

| Ticket | Guarantees owned | Partial in an earlier ticket |
| --- | --- | --- |
| QM05 | `ExclusivePocket`, `OldestFirst`, `OneVisitPerKeyPerLease`, `HeldSetStableUntilLeaseEnds` | `ExclusivePocket` at the lease's level in QM02 |
| QM06 | `ActionsNameTheirLease` | for `release` in QM02 |
| QM07 | `NeverRefusedForCapacity`, `EveryDropIsVisible`, `OldestSlotNewestPayload`, `HeldRecordsOldestFirst`, `NothingLostWhileHeld` | the three publish guarantees with nothing held in QM04; `EveryDropIsVisible` and `HeldRecordsOldestFirst` for acked and nacked records in QM06 |
| QM08 | `MarksStandHoweverLeaseEnds` | its release half in QM06 |
| QM09 | `ExpiryRecoversWithinTtl`, as safety fragments | its untimed fragment in QM08 |

`NeverRefusedForCapacity` is checked as one fragment: the publish rules read no size,
and a publish by a joined publisher is answered by exactly one rule unless its rank
equals the stored rank. That shows acceptance at the model's finite domains. It does
not show unbounded capacity, and the eviction order is excluded.

### Coverage a later ticket adds

A ticket named here adds a path that an earlier property already covers. It adds a
mutant on that path which the earlier property must catch, or it shows that the new
path runs a definition a listed mutant already breaks, and names that definition.

| Property | Owner | Later ticket, and the path it adds |
| --- | --- | --- |
| `ExclusivePocket` | QM05 | QM08: a lapse frees the pocket for another consumer |
| `OldestFirst` | QM05 | QM07: a version queued at its arrival position is pulled in position order |
| `OneVisitPerKeyPerLease` | QM05 | QM06: an acked or nacked key is not handed out again. QM07: a deferred version is not handed out under the same lease |
| `HeldSetStableUntilLeaseEnds` | QM05 | QM06: only the consumer's own ack or nack sets a mark. QM07: a publish onto a held key leaves the held version alone. QM08: a lapse is a lease's end |
| `HeldVersionNotNewer`, `HeldCopiesAgree` | QM05 | QM07: with deferral, neither is trivially true any more |
| `UniqueRecordPosition`, `RecordPositionsBelowNext` | QM04 | QM07: a record takes its arrival position when its lease ends |
| `IdleHasNoRecords` | QM06 | QM07: a pocket whose acked record has a version deferred requeues and does not idle |
| `ActionsNameTheirLease` | QM06 | QM08: a lease ended by a lapse is refused as one ended by a release is |
| `EveryDropIsVisible`, `OldestSlotNewestPayload`, `HeldRecordsOldestFirst`, `NothingLostWhileHeld` | QM07 | QM08: the lease's end by lapse. QM09, for `EveryDropIsVisible`: only a decay removes a shed entry |
| `MarksStandHoweverLeaseEnds` | QM08 | QM09: a repeated ack or nack extends the lease |
| Every lease extension (`PullWithinLease`, the four mark rules, `Renew`) | QM05, QM06 | QM09: `expires_at` is exactly `now + lease_ttl` after the step. Not "later than before": two steps at one `now` write the same value, and a zero TTL is allowed |

## Claims the tickets check

Each row is a question about the pinned CLI that a ticket settles by running it. "Read"
names where the answer was read in the installed source, under
`tools/quint/node_modules/@informalsystems/quint/dist/src/`. Nothing here was run, with
half an exception. AP01 ran `verify` with port 8822 free: Quint started its own Apalache
server and stopped it, and AP01's hand-back in
`.scratch/apalache/AP01-java-and-verify.md` quotes that. The other half of row 8, that
Quint uses a server it finds already listening, was read in `apalache.js` and has never
been run; shared rule 10 forbids the run that would show it. The named ticket runs the probe
through `just quint`, quotes the output, and records the outcome. An answer that differs
from the one below is a design change: the ticket stops and brings it back to this file.

| # | Claim | Read | Checked by |
| --- | --- | --- | --- |
| 1 | A plain `verify` that passes does not start the Rust evaluator. A `verify` that names an invariant and reports a violation does, and so does an `--observation` that is reached | `verify.js`, `cliReporting.js`, `rust/invariantsReporter.js` | QM01, and QM03 for the witness path it picks |
| 2 | The executable at `<QUINT_HOME>/rust-evaluator-<version>/quint_evaluator` is used when it exists and nothing is downloaded. The CLI checks that it exists, not what it is. QM01 runs this probe before it adds the guard, which would refuse a changed binary | `rust/binaryManager.js`, `config.js` | QM01 |
| 3 | Whether the evaluator's releases publish checksums. The CLI's source cannot say; the release's asset list can | not read | QM01 |
| 4 | `run --backend typescript` evaluates in process and downloads nothing. QM01 says whether the guard reads the option or fails closed on every `run` | `cliCommands.js` | QM01 |
| 5 | An instance that sets a module's `const`s is checked by naming it with `--main`, with that instance's `init` and `step` | not read | QM02 |
| 6 | `typecheck` exits non-zero on an ill-typed module and needs no network | not read | QM02 |
| 7 | The properties a `verify` checks are the ones named on its command line; with none named it falls back to the definitions declared with the `invariant` qualifier, and the model declares none. A run can therefore pass having checked nothing, so a pass means something only after `Canary` was violated through the same instance | `cliHelpers.js`, `cliCommands.js`, `verify.js` | QM02 by hand, QM03 in the recipe |
| 8 | Quint tries an Apalache server already listening on `localhost:8822` before it starts its own, so a stamped install does not say which server answered | `apalache.js` | Not run: shared rule 10 forbids the run. QM02 by hand and QM03 in the recipe show the other half, the port free and Quint launching its own server; QM03 matches the connecting line in `apalache.js` and refuses a busy port |
| 9 | A single `--observation` passes when reached and fails when not. `--search-observations` only warns on one it does not reach, and several stopping observations pass when one is reached | `verify.js`, `cliCommands.js` | QM03 |
| 10 | A syntax error, a type error and a real counterexample all exit 1, and a deadlock, if Apalache reports one under the configuration Quint builds (QM03's claim F), is reported as a violation. The exit status therefore does not say that a mutant was caught | `cliCommands.js`, `apalache.js`, `cliReporting.js` | QM03 |
| 11 | The output names the violated invariant only when the reporter can evaluate the counterexample | `cliReporting.js` | QM03 |
| 12 | The whole of `verify-quint` (every instance, witness and mutant, with setup) finishes inside the CI job's timeout. Each run being fast is not enough | not read | QM03, and every ticket that adds a run |
| 13 | An `assume` over a `const` is not among the properties a `verify` selects, and one that is false can leave no start state, which passes vacuously | `parsing/ToIrListener.js`, `cliHelpers.js` | QM05 |
| 14 | `run --seed` repeats a trace for a fixed seed and CLI, but the ITF output carries wall-clock metadata, so two runs differ byte for byte | `cliHelpers.js`, `cliReporting.js`, `itf.js` | QM10 |
| 15 | ITF holds states and values, not a log of actions with arguments. `--mbt` adds the action's name and the named `nondet` picks, which depend on how the action is written. The ghost `last` is what a replay reads | `itf.js`, `runtime/impl/builder.js`, `runtime/impl/builtins.js`, `runtime/impl/nondet.js` | QM10 |
| 16 | `verify --inductive-invariant` checks the start and the step, and then that the invariant implies the ordinary invariants named with it. Which check failed is read from the progress lines, not from the exit status | `verify.js`, `cliHelpers.js` | QM13 |
| 17 | An `init` that no state satisfies leaves a `verify` with nothing to violate, as a false `assume` does, so a mutant that breaks a requirement of the start cannot be caught by a property | not read | T47, at its stop on a config-bound mutant, on scratch copies whose `init` requires the bounds, before the maintainer chooses; or, where T47 lands before any such mutant exists or its hand-back quotes no such run, QM05, QM08 or QM09, on a scratch module at its step 2 |

## Shared rules (QM01 to QM13)

Every ticket carries this section word for word, under the same heading.

1. **The Allium decides.** `docs/specs/pachinko.allium` is the source of truth for
   behaviour (decision 0002). The model is derived from it and from nothing else. Read
   each rule and invariant in the Allium at the ticket's base, never from a ticket's
   summary of it, a page or a chat. No ticket reads the Rust or the double to decide
   what the model does. A ticket may read them for names, for the walk's domains and
   configuration, and, in QM10 to QM12, to map steps and state.
2. **The model never leads.** A counterexample has one of three causes. The model
   transcribes the Allium wrongly: fix the model. The property transcribes the Allium
   wrongly: fix the property, against the Allium's text. The Allium is wrong or silent:
   stop, write the trace and the question under Open points, and ask the maintainer. Never
   weaken an invariant, narrow a domain, drop a rule or add a guard the Allium does not
   have to make a run pass. Never settle either `open question` in the Allium.
3. **Names are fixed.** The model is `quint/pachinko.qnt`, module `pachinko`. An
   instance is a module `pachinko_<bound>` at its foot; the first is `pachinko_walk`.
   The sum type `Rule` has one variant for each Allium rule the model has so far, spelt
   as the Allium spells it.
   A rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`. An Allium invariant
   or guarantee is a property under its Allium name; `FiredMatchesGuards` and
   `UnansweredStepChangesNothing` check the labels. The ghosts are `fired`, `last`,
   `answer` and `prev`. A mutant is `quint/mutants/<ticket>-<slug>.patch`. Top-level
   actions carry `WalkStep`'s names (`crates/libpachinko/tests/queue/walk.rs`) in lower
   camel case. The gate is `just check-quint`, the verify recipe is `just verify-quint`,
   and the trace fixtures live in `quint/traces/`.
4. **One action per cascade, and every step is total.** A pull with its `HoldRecord`
   steps is one action. A lease's end with its `Freed` rules is one action. A publish
   with the revival it causes is one action. A step that no rule answers changes
   nothing and leaves `fired` empty; from QM09, `advance` and a `tick` with nothing due
   move only the clock. No guard and no effect on the state reads a ghost.
5. **The domains are a ruling.** `pachinko_walk` has the walk's domains and
   configuration: 2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3,
   pull limits -1 to 3 (-1 and 0 until QM05), `max_pull_batch` 2 and
   `max_consecutive_expiries` 2. A ticket
   that cannot verify at these inside the CI job's timeout stops and asks. It does not
   shrink a domain or drop an instance. QM03 sets each instance's `--max-steps`; a
   later ticket may raise it and never lowers it.
6. **Verified means this.** From QM03 on, `just verify-quint` shows all of it. `Canary`
   is violated on every instance. Every property passes on every instance, each named
   on the command line. Every rule the ticket owns has its witness reached. Every
   mutant the ticket lists typechecks and is caught by the one property it names, which
   may be any property the recipe checks, and which the unchanged model's pass on that
   instance shows to hold at the same bound. Each check fails closed: zero instances, a
   witness not reached, a patch that does not apply, a mutant that survives, a mutant
   caught by another property, and an error or a deadlock read as a catch are all
   failures. QM02 shows the same by hand for one witness and one mutant. An owned
   invariant that no listed mutant aims at is reported in the hand-back as checked and
   not mutation-tested.
7. **No clock before QM09.** QM02 to QM08 add no `now`, no timestamp field and no TTL.
   While the model has no clock, a specification change that only says when something
   is due changes nothing in the model, as long as the untimed lapse still covers it.
8. **What the model lacks is written down.** A behaviour the Allium allows and the
   model does not yet have is one line in the model's header comment, under "Not yet
   modelled", naming the ticket that adds it. That ticket deletes the line. A property
   that would fail only because of such a gap carries the gap as a named exception, and
   the same ticket removes the exception. A property is transcribed whole, conditions
   included, even where a condition is always true so far.
9. **Quint runs through the recipe.** Every Quint command is `just quint <args>`, run
   from the repository's root, never a bare `quint` on `PATH`. A claim about the CLI's
   behaviour is checked by running the pinned CLI and quoting its output, never from
   memory or from a page.
10. **The network is named, and the checker is the pinned one.** `just initialize`,
    `just install-quint`, `just install-apalache` and `just install-quint-evaluator`
    reach the network, and each is a named, authorised step in the ticket that runs it
    (`tickets/CONVENTIONS.md` §11, "Network"). `just check` reaches none. No step lets
    Quint download anything itself. Before any `verify`, show that nothing is listening
    on `localhost:8822`; never pass `--server-endpoint`; never run a `verify`, or any
    other `just quint` command that needs the evaluator, while `just install-apalache`
    or `just install-quint-evaluator` runs or while another worktree verifies. A `verify` talks
    to its own server over the loopback interface, so a network denial used as evidence
    for a `verify` denies the external network and allows loopback, and its negative
    controls are §11's two `curl` commands.
11. **One change carries the model and what depends on it.** A change to
    `quint/pachinko.qnt` re-cuts every mutant patch it breaks, keeping the mutation and
    the property it names, and runs `just verify-quint` once QM03 has landed that recipe.
    Once `quint/traces/` exists it also regenerates the fixtures, before any gate runs,
    because `just check-quint` then compares the model's hash, and keeps both replays
    green: the Rust test in `just test` and, once QM12 has landed, `just python-double`. A
    field or a step the model gains is either mapped in each replay or added to that
    replay's list of what it does not compare, naming the ticket that will map it. A step
    on a replay's list ends that replay of every trace that holds it, so a ticket lists a
    step only with the maintainer's answer on which ticket maps it. The patches, the
    fixtures, their generator and the replays are in the Files touched of every ticket
    that changes the model, and the hand-back lists each patch re-cut. A change to
    `docs/specs/pachinko.allium` that touches a modelled rule, invariant or guarantee
    updates the model in the same change, runs `just check-quint`, and runs
    `just verify-quint` if the recipe exists.
12. **Files.** Edit only the files the ticket lists, plus its own `status:` line and
    hand-back notes. A shared file is listed as `shared, own lines`. A change needed
    elsewhere is handed back, not made (`tickets/CONVENTIONS.md` §11). Scratch goes in
    `ai_tmp/<ticket>/` and is never committed.
13. **Evidence, and the series base.** Each quoted output is headed by the commit it ran
    at and shows its exit status. A check that passes by printing nothing follows its
    positive control (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"
    and "Checks fail closed"). Where §11 says `main`, read the series base the
    maintainer named: for the worktree's start, for the closing fetch and merge, and
    for the evidence quoted afterwards. Pushing and opening a pull request are
    separately authorised.

## Adversarial review

Every ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. Every ticket
carries the deltas below word for word, under the heading "Adversarial review", followed
by its own list of checkpoints.

- **Reviewer.** Codex, Sol, which is `codex exec -m gpt-6.2-sol`. The maintainer named
  Sol for this series on 2026-11-25, in place of the model §14 prints. Confirm
  `codex exec --help` on the day, as §14 asks. Fall back to a fresh-context
  `claude-opus-5-5` subagent only if `codex` is not on `PATH`, and say so in the
  hand-back notes.
- **A usage limit is not a clean review.** If a Codex log ends in a usage-limit error,
  the lens is incomplete. Stop and ask the maintainer whether to wait or to use the
  fallback. Never record it as "no findings".
- **Command.** Run this from the worktree, once per lens:

  ```sh
  dir="$PWD/ai_tmp/review/<id>-<n>"
  mkdir -p "$dir"
  codex exec -m gpt-6.2-sol -s read-only -C "$PWD" \
    -o "$dir/last-<lens>.md" "<brief>" \
    < /dev/null >| "$dir/log-<lens>.txt" 2>&1 &
  pid=$!
  wait "$pid"
  echo "exit $?"
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens. A
  second round uses `<lens>-2` in all three names. `< /dev/null` closes stdin; without
  it the run waits forever. `-o` holds only the last message. `>|` is for a shell with
  `noclobber`. Under `-s read-only` Codex cannot write files, so the brief tells it to
  put its complete findings in its final message, and the runner copies
  `last-<lens>.md` to `findings-<lens>.md` under a first line `Source: last-message`.
  If §14 on the ticket's base already prints a command that redirects stdin from
  `/dev/null`, use §14's, with Sol's model.
- **Completion.** A lens is complete when its process, followed by the process id the
  command captured, has exited 0 and `last-<lens>.md` is not empty. Never wait on a
  file name or a `pgrep` pattern, which can match a run in another worktree. A lens
  that exits non-zero or leaves an empty file runs again, once; a second failure stops
  the ticket.
- **Lenses.** `conformance` and `breakage`, as §14 defines them. A checkpoint marked
  "with a design lens" adds a third reviewer, `design`: does the model say what the
  Allium says, is anything the Allium allows missing without being listed under "Not
  yet modelled", and could a property hold for a reason other than the one claimed.
- **Brief.** It gives the ticket's path, `tickets/CONVENTIONS.md`,
  `docs/specs/pachinko.allium`, the checkpoint's questions, and the artefact. At a plan
  checkpoint the artefact is the plan file. At a checkpoint on a model or a recipe it
  is the files the ticket names there, with the output of the controls run so far. At
  a diff checkpoint it is `git diff "$base"...HEAD` plus the uncommitted changes.
  `base` is the commit step 1 records. The brief never gives the author's reasoning or
  the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one
  writer at a time. Before the lead runs `just check` or `just verify-quint`, it
  confirms every writer has stopped.
- **After the last checkpoint.** A merge or a rebase onto the series base, or a fix made
  after the last checkpoint, as after a red CI run, that changes a file under the
  ticket's Files touched was not reviewed. The agent shows that diff
  at the push stop and asks whether the last checkpoint runs over it first.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

Checkpoints, by ticket. The plan this series came from asked for two per ticket, with a
design lens at QM02 and QM09. On 2026-11-25 the maintainer asked for reviews from Codex
(Sol) at the key moments, so QM02 and QM03 have a third, and the model tickets' diffs
have the design lens.

| Ticket | Checkpoints |
| --- | --- |
| QM01 | 1, the plan, before any tracked file changes; 2, the diff, before the final commit |
| QM02 | 1, the plan and the draft of decision 0022, with a design lens; 2, the model, with a design lens, before the gate and the pages take its names; 3, the diff |
| QM03 | 1, the plan; 2, the recipe and its controls, before the CI job joins `needs`; 3, the diff |
| QM04 to QM08 | 1, the plan; 2, the diff, with a design lens |
| QM09 | 1, the plan, with a design lens, before any earlier rule is changed; 2, the diff, with a design lens |
| QM10 | 1, the plan and the fixture format, with a design lens; 2, the diff |
| QM11 to QM13 | 1, the plan; 2, the diff |

## The T tickets that touch the model

Ruling 3 puts an obligation on tickets outside this series. A ticket's agent edits only
the files its ticket lists, so the obligation is written into each of these tickets as a
conditional step and a conditional row in Files touched: if the tree the ticket lands
on holds `quint/pachinko.qnt`, at its base or after its closing merge or rebase, and it
models what the ticket changes, the ticket updates the model in the same change, as
shared rule 11 says.

| Ticket | What it does | What it touches in the model |
| --- | --- | --- |
| T39 | Lands the operator listing and `new_pocket_first` in the Allium | `PublishIntoNewPocket`, the claim's choice of pocket, the rules that clear a new pocket's standing (a release, idling and a revival), the properties for the reworded Lease expiry comment and `ExpiryRecoversWithinTtl`, and, where the model has `PublishIntoNewPocket`, an instance with `new_pocket_first` on (QM04, QM08, QM09) |
| T40 | Rules on where the pocket order lives, then creates tickets | The pocket's position in claim, release and revive, and two position invariants. The model keeps the pocket order in one section, so that this is that section, the two position invariants and the effects that state a position. T40 carries the obligation into the tickets it creates |
| T41 | Investigates the order of work in a tick | Nothing. It blocks QM09 |
| T42 | Explores the clock model, has it ruled, then creates tickets | Nothing itself. The specification ticket it creates blocks QM09, and T42 carries the obligation into it. While the model has no clock, that ticket changes nothing in the model where it only says when something is due and the untimed lapse still covers it (shared rule 7); a change to a lapse rule the model already has moves the model in the same change. QM09 adds the clock |
| T43 | Rules on the caller contract, then creates tickets | Every consumer action, `answer`, and QM10's mapping: the lease's id and the named refusals. T43 carries the obligation into the tickets it creates |
| T44 | Researches the width of a rank | Nothing. A bound on ranks lies outside the small domains |
| T45 | Drops joining | `PublisherJoins`, `ConsumerJoins`, `UniquePublisherId` and `UniqueConsumerId` (QM04), the start of every fixture, the generator's witness list, and the joins in both replays |
| T46 | Adds `limit >= 1` to the claim | `guard_PullClaimsFrontPocket`, the rule or refusal T43 decided for the gap, and `OldestFirst`. It lands after QM05. Before QM05 the model's only claim is one at a limit that is not positive, which T46 removes, so a T46 that is ready first stops and asks, whether or not the model exists yet |
| T47 | States the start as a rule that requires the config bounds | `init`, the Queue's first position (in every fixture's first state and in both replays), and the five config-bound invariants. A mutant that breaks a config bound is expected to lose its start state once the bounds are the start rule's requirements (claim 17, not run), so T47 probes it and the maintainer rules on those mutants |

Only QM09 and QM11 are blocked by a T ticket: QM09 by T41 and by the specification
ticket T42 creates, and QM11 by the ticket that lands the ruled clock in the Rust, where
that is another ticket ("Dependencies"). The rest of the series models today's
specification, and each T ticket above that lands later changes the model with it.

On 2026-11-25 the maintainer ruled on three points the tickets above had left open, and
each ticket already says what its ruling says. T46 lands after QM05, and a T46 that is
ready first stops and asks. T39 verifies a second instance with `new_pocket_first` on,
owed only where the model has `PublishIntoNewPocket`, and stops and asks if that
instance does not fit the verify job's timeout. T45 and T47 stop and ask before they
touch a mutant patch that cannot stand as it is: in T45 one whose rule or property the
ticket removes, in T47 one that sets a config value out of its bound and so is expected
to lose its start state, which T47 shows on a scratch copy before it asks.

A T ticket that lands between two QM tickets changes the model and not the text of the
QM tickets still to run. Each of those reads its counts and its traces against the
base: its step 2 counts the variants of `Rule` in the base's model, names the T ticket
behind any difference from the count its text assumes, and has its plan recompute each
trace and the bound. T45 is the case in view: it removes two rules and the two joins
that open every trace.
