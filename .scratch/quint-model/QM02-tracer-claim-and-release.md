---
id: QM02
title: "Tracer: claim and release, decision 0022 and check-quint"
status: todo
depends_on: [QM01]
parallel_with: []
branch: ticket/qm02-tracer-claim-and-release
estimated_size: L
---

# QM02: Tracer: claim and release, decision 0022 and check-quint

## Context

`docs/specs/pachinko.allium` decides what the queue does. The Quint CLI 0.34.0 is pinned
under `tools/quint/` and Apalache 0.62.4 is pinned by checksum
(`docs/decisions/0021-project-managed-quint-cli.md`). After QM01 the Rust evaluator is
pinned by checksum too. Three things are still missing, and this ticket adds the thinnest
slice of each:

- **No model.** There is no `quint/` directory and no `.qnt` file.
- **No gate.** Decision 0021 promises one and names this change as the one that adds it.
  Its "Decision" section says, at this ticket's writing on lines 83 to 86: "There is no
  Quint gate yet, because there is no Quint module, and a gate over zero files asserts
  nothing. The change that adds the first module adds `check-quint`, running
  `quint typecheck`, which is offline, to `just check` and to the recipe list in
  `pyproject.toml`, and adds `just install-quint` to the CI job that runs it."
- **No document that says how a Quint model relates to the Allium.**
  `docs/decisions/0002-specs-are-the-source-of-truth.md` says the specifications decide
  behaviour, and that "nothing mechanical holds the code to a clause". Nothing says what
  a second formal text beside the Allium may and may not decide. `docs/decisions/README.md`
  line 68 says "The next decision is 0022".

The slice is a tracer: one path through every layer, so that each later ticket only
widens something that already works end to end. It models three rules of the Allium, at
this ticket's writing on these lines of `docs/specs/pachinko.allium`:
`PullClaimsFrontPocket` (527 to 554), `PullFindsNoPocket` (569 to 575) and
`ReleaseRequeuesPocket` (668 to 687). It checks six invariants:
`LeasedHasOneActiveLease` (149) and `UnleasedHasNoActiveLease` (153) on `Pocket`, and the
top-level `OneActiveLeasePerPocket` (1030), `OneActiveLeasePerConsumer` (1037),
`UniquePocketPosition` (1056) and `PocketPositionsBelowNext` (1068).

Five facts shape the slice. Each is taken up under "Design proposed" or "Open points".

- **The tracer's claim hands out nothing.** The series README fixes that a pull's limit
  ranges over -1 and 0 until QM05. `PullClaimsFrontPocket` does not read the limit in
  its `requires`, so a consumer with no lease still claims the front pocket, and
  `oldest(...)` yields none "when n is not positive" (lines 487 to 489). That is today's
  Allium, and the model follows it.
- **T46 would remove that claim.** `tickets/T46-no-claim-without-records.md` adds
  `requires: limit >= 1` to `PullClaimsFrontPocket`. The series README's table "The T
  tickets that touch the model" says T46 lands after QM05. If T46 is ready first, it
  stops and asks; this ticket does not model it.
- **The seed has two pockets for three consumers, so that `PullFindsNoPocket` can
  fire.** The series README fixes it: the seed joins the three consumers and queues
  `a` and `b`, each with one queued record. Every release here requeues its pocket, and
  a pull by a consumer that holds a lease is `PullWithinLease`, which QM05 owns. With
  three pockets a consumer with no lease would always find one queued. With two, the
  third consumer's pull finds none once the first two have claimed.
- **`just quint typecheck` does not notice a stale install.** The guard the `quint`
  recipe runs first is `scripts/install-apalache.sh --check`. Its `check_mode` exits 0
  for `typecheck` (line 290 at this ticket's writing) before it reaches `check_cli`,
  which is what compares the installed CLI's version with the pin and refuses a
  symlinked entry point. QM01 leaves this function alone: it adds a second guard line
  to the recipe, `scripts/install-quint-evaluator.sh --check`, which leaves early for
  `typecheck` as well (QM01, "Non-goals"). The gate therefore makes both checks itself.
- **A worktree in flight fails `just check` once this ticket is in its base**, until it
  runs `just install-quint` (network). `check-quint` needs the CLI, and only
  `just initialize` and `just install-quint` install it. The pages this ticket edits say
  so.

Read these:

- `.scratch/quint-model/README.md`, whole. It fixes the rulings, the names, the design,
  the shared rules and the review deltas this ticket carries.
- `docs/specs/pachinko.allium`: the entities `Queue` (107), `Consumer` (115), `Pocket`
  (124) and `Lease` (257); the comment that opens "Pull" (486 to 498); the three rules
  above; the comment that opens "Ack and nack" (577 to 581), which says what an action
  naming an ended lease does; the comment that opens "Renew and release" (654 to 659);
  the six invariants above; and the guarantees `ExclusivePocket` (1175) and
  `ActionsNameTheirLease` (1179). Read them at the base, not here.
- `docs/decisions/0021-project-managed-quint-cli.md` and
  `docs/decisions/0002-specs-are-the-source-of-truth.md`. `docs/decisions/README.md`,
  "Writing a new one", and `docs/manifest.yml`.
- `Justfile`: the comment over `install-quint` and the recipe (78 to 83), `check-specs`
  (451), `quint` (498, with the comment above it) and `check` (528). `pyproject.toml`,
  the `recipes` list (53 to 74).
- `scripts/install-apalache.sh`, `check_cli` and `check_mode`, and
  `scripts/check-apalache-pin.sh` for how a script here reads JSON with pixi's Python.
- `.github/workflows/ci.yml`, the `documents` job (106 to 133) and `check` (197 to 205);
  `.github/workflows/release.yml`, lines 31 and 32;
  `.github/actions/setup/action.yml`.
- `docs/reference/quality-gates.md`: the gate table (17 to 39), the paragraph on gates
  17 and 18 (57 to 62), "When a gate fails" and "In continuous integration".
  `docs/reference/commands.md`: "Documents" and "Aggregate".
- `crates/libpachinko/tests/queue/walk.rs`: the domains (38 to 43), `LeaseChoice` (71 to
  78), `WalkStep` (82 to 117), the pull's limits (134) and `resolve` (219 to 230). For
  names and domains only (shared rule 1).
- `.scratch/apalache/AP01-java-and-verify.md` and
  `.scratch/apalache/AP02-checksummed-apalache.md`, as background only. The probe of
  port 8822 and the save, edit, restore and `cmp` pattern came from them, and both are
  written out in this ticket's steps and Verification: no step depends on those files.
- `tickets/CONVENTIONS.md` §11 and §14. Where §11 says `main`, read the series base.

## Goal

After this ticket, `quint/pachinko.qnt` holds a module `pachinko` with the three rules
above over a seeded start, the four ghosts, a guard, an effect and a witness for each
rule, the six invariants, `FiredMatchesGuards`, `UnansweredStepChangesNothing`,
`Canary`, and the instance `pachinko_walk` at its foot. Decision 0022 says what the
model is for and what it may not decide. `just check-quint` typechecks every `.qnt` file
under `quint/` with the pinned CLI, fails on zero files and on a CLI that is not the
pinned version, and is the twenty-first gate of `just check`. The CI job that runs it
and the release workflow install the CLI first. `AGENTS.md`, the `spec-change` skill and
the handbook say that a specification change carries the model with it.

Nothing runs `quint verify` in a gate yet; QM03 adds that. This ticket shows ruling 4's
loop once, by hand, and quotes it in the hand-back: `Canary` violated through
`pachinko_walk`, then a bounded `verify` that passes with every property named, one
witness reached, and one mutant caught by the property it aims at.

### Owns

| Kind | Names |
| --- | --- |
| Rules | `PullClaimsFrontPocket`, `PullFindsNoPocket`, `ReleaseRequeuesPocket` |
| Invariants | `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `UniquePocketPosition`, `PocketPositionsBelowNext` |
| Guarantees | none |
| Partial or extended, not counted | `ExclusivePocket` at the lease's level, which QM05 owns, and `ActionsNameTheirLease` for `release`, which QM06 owns. The series README lets a part be a property named for what it says, and these are: the first is what the four lease invariants check, the second what `FiredMatchesGuards` and `UnansweredStepChangesNothing` check for a release, once step 4's refusal run has shown that the model takes the step they speak of ("The refusal of an ended lease"). This ticket adds no `<Guarantee>_<part>` property |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them at step 3's **Stop**, after
checkpoint 1 and before any tracked file changes, and the hand-back records each
ruling. After that the builder does not reopen them. Later tickets are written against
the recommended answers. If a ruling differs, this ticket edits the sentences it changes
in the later tickets and in the series README, and lists them in the hand-back.

### The series README's second design block

The block was added after a review and the maintainer has not ruled on it. Each point,
with what it costs:

- **Every step is total.** A step no rule answers changes nothing and leaves `fired`
  empty, and the property `UnansweredStepChangesNothing` checks it: when `fired` is
  empty the state equals `prev` and `answer` is nothing. That makes a release naming an
  ended lease a refusal a property can speak of. It is checked only once a run shows
  that `step` offers that release and the model refuses it: "The refusal of an ended
  lease", below. `step` offers each action over the whole of its domains whatever the
  state holds; no property can show that, since a step that is not offered is in no
  trace, so checkpoints 2 and 3 read `step` for it. Cost: every action has a branch
  for "no rule answers", and the model has no deadlock, so a deadlock can never be the
  way a mutant is caught.
- **Three more ghosts.** `last` is the step taken, a sum type `Step` with `Init`,
  `Pull` and `Release` here. `answer` is what the step handed back: a batch naming its
  lease and pocket with its records in order, or `NoPocketAvailable`, or nothing.
  `prev` is the state before the step. Cost: `prev` is a second copy of the whole state
  in every state, so the checker carries twice the variables and a counterexample
  prints both. Who writes `last` is proposed under "The refusal of an ended lease".
- **A guard and an effect per rule.** `guard_<Rule>` is a pure definition over a state
  and the trigger's arguments. `effect_<Rule>` is an invariant: if `fired` holds the
  rule, what the Allium's `ensures` says holds between `prev` and the state. It states
  its frame: what neither the rule nor another rule fired in the same step writes is
  equal in `prev` and the state. Cost: each rule is transcribed twice, once in its
  action and once in its effect, so a specification change edits both, and the two can
  be wrong the same way if one is written by reading the other.
- **`Rule` grows with the model.** Here it has three variants, the three rules under
  "Owns", and each later ticket adds a variant together with that rule's guard, effect
  and witness, which is shared rule 3's "one variant for each Allium rule the model has
  so far". A variant without its three definitions cannot be added early: QM03's recipe
  reads the rules from `Rule` and fails on one that has no witness.
- **`FiredMatchesGuards`.** `fired` holds exactly the rules whose guard held for the
  step `last` names. No rule here fires later in a chain, so every guard is read in
  `prev`. When `last` is `Init`, `fired` is empty.
- **Properties are named apart from everything else.** A property is a `val` with no
  parameter whose name starts with an upper-case letter, or `effect_<Rule>`. Every
  other definition starts with a lower-case letter. No definition uses Quint's
  `invariant` qualifier, which the CLI would collect by default.
- **A witness rests on those.** `witness_<Rule>` is `fired.contains(<Rule>)`.
- **The state can hold what an invariant forbids.** Leases are entities, ended ones
  included. Cost: a list in the state that grows with the trace.
- **Every invariant is transcribed whole**, conditions that are always true so far
  included.
- **`pachinko_walk` carries the walk's configuration.** Here that is the consumers `c0`
  to `c2`, the pockets, and the pull limits, -1 and 0 until QM05. If the model has a
  constant for the pocket domain at this ticket, it is `a`, `b` and `c`, as shared rule
  5 has it: the seed fills `a` and `b`, and `c` is in the domain and not in the state.
  Two pockets in the seed are not a domain of two. Each other constant enters with the
  ticket that first reads it.
- **Two step relations.** This ticket writes `step`. QM10 writes `replayStep`.
- **A canary.** `Canary` is false on purpose, and no pass on an instance is trusted
  until it was violated through that instance.

The alternative is the first block alone: the `fired` set and nothing else. The series
README says why that is not enough: "a label can be set by a step that did nothing, and
a set of labels cannot say in what order a batch was handed out". Without `prev`, a
guarantee about two states cannot be an invariant over one. Without `effect_<Rule>` and
`FiredMatchesGuards`, a witness shows only that an action wrote a label.

**Recommended:** take the block as a whole.

### How a lease is identified

The Allium keeps `Lease` entities whose `released` and `expired` states are terminal
(lines 257 to 278), and `ActionsNameTheirLease` needs an action to be able to name an
ended lease. The walk's `LeaseChoice` names `Own`, `Issued` and `Never`.

**Recommended:** `leases` is a list of leases in the order they were granted, and a
lease's identity is its index. A lease is its pocket, its consumer and its status, one
of active, released and expired. An ended lease stays in the list. The action
`release(consumer, lease)` takes an index, and `step` chooses it from the list's indices
and the one index past its end. That one index stands for every lease never issued, so
the three cases of `LeaseChoice` are covered: the consumer's own, any issued, and one
never issued. QM10 maps a `LeaseChoice` to an index.

The state grows by at most one lease a step, since only a claim creates one. The step
bound is what bounds it: a `verify` at `--max-steps=n` holds at most `n` leases.

Alternatives:

- A map from a pocket to its lease. The series README rules it out: it cannot hold two
  active leases on one pocket, so `OneActiveLeasePerPocket` would be true by
  representation, and it cannot name an ended lease.
- A set of leases with an `id` field and a counter. It can say the same things, at the
  cost of one more variable and a uniqueness property the Allium does not have.
- A constant domain of lease ids set by the instance. Use it only if step 2 shows that
  Apalache does not accept a choice from a set that depends on the state.

T43 (`tickets/T43-caller-contract.md`) rules on the lease id a caller sees. The index is
the model's own identity and changes nothing there.

### How the pocket order is written

T40 (`tickets/T40-pocket-order-model.md`) may move the pocket order from a `position` on
each pocket to an order the queue owns.

**Recommended:** one section of the module, "Pocket order", holds the only two
definitions that read or write the order. One reads it: the front pocket of a state,
the queued pocket with the smallest position, which is the Allium's
`min_by(free, p => p.position)`. One writes it: the state with a named pocket put at the
back, at `next_position`, and the counter advanced. The claim calls the first and the
release the second. QM04's `PublishIntoNewPocket` and the revivals of QM06 and QM08
call the second too. Outside that section no action names a pocket's `position`. The
two position invariants name it, and so does the effect of each rule that writes the
order, because they transcribe the Allium, and T40's follow-ups change them anyway.
Here that effect is `effect_ReleaseRequeuesPocket`. It says in its own words that the
pocket's position is `prev`'s `next_position` and that the counter is one more (lines
677 to 686), and it never calls the definition the action calls: an effect that called
it would agree with the action whatever that definition wrote.

This is the series README's "one section": two definitions, and a T40 follow-up edits
that section, the two position invariants and the effects that state a position.

Alternatives:

- The Allium's shape inline in each action. It reads line for line against the Allium,
  and a T40 follow-up then edits every action that touches the order.
- For the effects only: two more definitions in the section that read a pocket's
  position and the counter, which the effects use in place of the field names. T40's
  edit stays inside the section, and one more definition stands between each effect
  and the Allium's text.

### The refusal of an ended lease

The "Owns" table credits this ticket with `ActionsNameTheirLease` for `release`, through
`FiredMatchesGuards` and `UnansweredStepChangesNothing`. Both hold in a model whose
`step` never offers an ended lease. Both can hold where the action and
`guard_ReleaseRequeuesPocket` take an ended lease for its successor in the same way, and
then only the effect's frame stands in the way. So the credit needs a run that shows
the model takes that step and refuses it.

**Recommended:** one lower-case definition, `staleReleaseRefused`, and one more by-hand
run. The definition reads `last`, `prev` and `fired`, and no `guard_` definition. It
holds when `last` is a release by a consumer that names a lease `prev` holds as ended,
`prev` also holds an active lease of the same consumer on the same pocket, and `fired`
is empty. That is the Allium's own case: an action meant for an ended lease "is
rejected, even when the same consumer holds the same pocket again under a new lease"
(lines 577 to 581). Step 4 shows `not(staleReleaseRefused)` violated at five steps, and
`UnansweredStepChangesNothing`, in the pass, says the state then equals `prev`.

The run shows that the case is reached and refused once, for one consumer. It does
not show that every such release is refused: until QM06 that is still the two label
checks and the effect's frame. And every one of those reads `last`. An action that
turned another consumer's ended lease into its successor and wrote the successor into
`last` would leave a trace no property and no run can tell from a release of the
successor.

**Recommended, for that:** no action writes `last`. `step` writes it, in the same
conjunction that calls the action, from the arguments `step` chose, so `last` holds
the lease the release was given and never one the action worked out. A release that
then acts on another lease fails `FiredMatchesGuards`, or, where the guard is wrong
the same way, the effect's frame, since the lease `last` names was already ended and
some other lease changed. Step 2's scratch module is written the same way, so claim c
covers the shape. Alternative: the actions write `last`, and checkpoint 2 reads each
one for it; that leaves the tie between the choice and `last` to a reading. Either way
the credit under "Owns" is for one refused case and the label checks, not for the
guarantee's release half as a whole.

`staleReleaseRefused` is not a property, so QM03's recipe does not repeat the run. A
release naming the index past the end, the lease never issued, has no run of its own
here. QM06 states `ActionsNameTheirLease` over `prev`, with a mutant aimed at it, and
says whether `staleReleaseRefused` stays.

Alternative: a property `ActionsNameTheirLease_release`, the form the series README
names first for a part checked early. When `last` is a release naming a lease that is
not active in `prev`, or not the consumer's, `fired` is empty. It reads `prev` and no
guard, and every later run of `verify-quint` checks it. It still needs the run above,
because a property about a step holds where the step is never taken. If the maintainer
rules for it, the name joins the list under "The properties, and how a run names
them", the pass of step 4, `props` in Verification, the acceptance bullet on the
model and decision 0022's list, and it changes QM06's sentence that this ticket adds
no property under that name. Step 3 records each of those edits. The series README
reads more easily this way: a part checked early "is a property of its own".

### The seed

`init` is a seed, because publish is QM04's. The series README fixes what it holds: the
three consumers are joined (`ConsumerJoins` is QM04's, and QM04 adds "the consumer has
joined" to the two pull guards), and the pockets `a` and `b` are queued, each with one
queued record. `c` first exists when QM04 publishes into it. No lease exists. At
`Init`, `prev` equals the state, `fired` is empty and `answer` is nothing.

**Recommended, for what the README leaves open:** the pockets' positions are 0 and 2
and `next_position` is 4, which is what two `PublishIntoNewPocket` publishes leave (a
pocket at `n`, its record at `n + 1`), so the seed is a state the Allium reaches. The
alternative is 0 and 1 with `next_position` 2.

A pocket here is its status and its position. `consecutive_expiries` is not in the
state before QM08, which adds it to both release rules. The header's "Not yet
modelled" has a line for it that names QM08, and `effect_ReleaseRequeuesPocket` leaves
out the Allium's `pocket.consecutive_expiries = 0` until then.

A record here is its pocket, its key and its status, and the status is always queued.
It has no mark and no `deferred_position`: QM06 gives records a mark and QM07 adds
deferral. `ending_records` is one definition, and it is empty until QM06, with a "Not
yet modelled" line that names QM06 and QM07. `guard_ReleaseRequeuesPocket` is still
written as the Allium writes it, `records.count > ending_records.count`, through that
definition. So the guard is 1 > 0 in every reachable state, and QM06 changes the
definition and not the guard. QM04 replaces the seed with an empty `init` and real
publishes.

### The properties, and how a run names them

The series README fixes how a property is declared and named (its second block,
"Properties are named apart from everything else"). Nothing uses the `invariant`
qualifier, so a `verify` that names no property checks nothing (the README's claim 7).
What this ticket proposes is how a run names them: every command names its properties
with `--invariant=<a comma-separated list>` and its instance with
`--main=pachinko_walk`, and no command uses `--invariants` (claim b).

`Canary` is `last == Init`. It holds at the start and fails after any step, so its
violation shows that the instance's `init` and its `step` both ran. The alternative,
`false`, is violated by `init` alone.

The properties of this ticket, in the order the commands name them: the six invariants
under "Owns", `effect_PullClaimsFrontPocket`, `effect_PullFindsNoPocket`,
`effect_ReleaseRequeuesPocket`, `FiredMatchesGuards` and
`UnansweredStepChangesNothing`.

### The gate

- **`scripts/check-quint.sh`, run by `just check-quint`.** POSIX `sh`, offline and
  read-only, as `scripts/check-effects.sh` is to `just effects`. In order:
  1. It reads the pinned version from `tools/quint/package.json` and the installed one
     from `tools/quint/node_modules/@informalsystems/quint/package.json`, with pixi's
     Python as `scripts/check-apalache-pin.sh` reads JSON. It fails, naming
     `just install-quint`, when the installed package is missing, unreadable or another
     version. It fails the same way when any of the six paths `check_cli` in
     `scripts/install-apalache.sh` tests is a symlink, or when the entry point,
     `cli.js` under the package's `dist/src/`, is not a regular file: the `quint` recipe
     runs that file, and node follows a link to another CLI whatever the
     `package.json` beside the package says.
     `just lock-check` already holds `package.json` to its lock.
  2. It lists every regular file named `*.qnt` under `quint/`. It fails when `quint/`
     is missing, when the listing fails, and when the list is empty.
  3. It runs `just quint typecheck <file>` for each, one file a run, lets that
     command's own output through, and fails at the first that fails, naming the file.
     The file's name alone does not say why the command failed; Quint's output does.
  4. It prints one line with the number of files and the CLI's version.

  Every line it prints starts `check-quint:`. Verification matches these fragments, so
  they are part of the proposal: `no .qnt file` when the list is empty or `quint/` is
  missing; the failing file's path when a typecheck fails; and `just install-quint`
  when the CLI is missing, linked or another version.
- **It is the last recipe in the list**, gate 21, and `check-clean` becomes 22. No other
  gate's number moves. The alternative is a place beside `analyse-specs`, as gate 19.
  That renumbers `snapshots-check` and `test-doc-default` in the `Justfile`'s comments
  (lines 194 and 242), in `docs/reference/commands.md`, in `docs/reference/testing.md`
  and in `docs/reference/quality-gates.md`.
- **No hook.** `check-specs` is a pre-commit hook as well as a gate. `check-quint` is a
  gate and a CI step only.
- **Mutant patches and trace fixtures are not swept in.** Only `*.qnt` files are
  listed, and a mutant is a `.patch`.

### Decision 0022

The file is `docs/decisions/0022-quint-model-of-the-queue.md`, with this frontmatter:

```yaml
---
title: "Decision 0022: A Quint model of the queue"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_quint_model]
requires: []
---
```

Its line in `docs/manifest.yml`, after decision 0021's and with a comma added to that
one:

```json
{"path": "decisions/0022-quint-model-of-the-queue.md", "title": "Decision 0022: A Quint model of the queue", "kind": "decision", "audience": ["contributor", "maintainer", "agent"], "canonical_for": ["decision_quint_model"], "requires": []}
```

It has the sections every record has: Context, Decision, Consequences, What would
reopen this, Related pages. It states:

- The Allium decides. The model is derived from it and from nothing else, and never
  leads it: a counterexample that shows the Allium wrong or silent is a question for the
  maintainer, never a change to the model alone.
- Where the model lives: `quint/pachinko.qnt`, one module that grows, with its
  instances at its foot.
- Which properties it checks: every Allium invariant and guarantee under its Allium
  name, a guard, an effect and a witness per rule, `FiredMatchesGuards` and
  `UnansweredStepChangesNothing`.
- What "verified" means, as ruling 4 has it: a bounded `verify` at the walk's domains,
  a witness for every rule, and seeded mutants the checker catches. It says that until
  QM03 this is shown by hand, once, and that `just check-quint` proves only that the
  model typechecks.
- What is excluded and why: liveness (`ExpiryRecoversWithinTtl` is checked only as
  safety fragments); the eviction order under `NeverRefusedForCapacity`, because the
  Allium has no rule for it and bounds no memory; and both open questions.
- Which invariants are true by representation, each with the Allium's reason. This
  ticket starts the list with what its state creates, and each later ticket adds its
  own.
- Which properties stand for a guarantee's part before its owner lands, and what each
  leaves unshown: the four lease invariants for `ExclusivePocket`, and the two label
  checks with one by-hand run for the release half of `ActionsNameTheirLease`.
- What hurts: a second text to keep in step with the Allium; bounds, not proof; and a
  per-worktree install the gate needs.

## Claims this ticket checks

Numbers are the series README's. Letters are claims this ticket rests on that nobody
has run. Each is run through `just quint` and its output quoted. An answer that differs
from the one below is a design change: stop and bring it back.

| # | Claim | Check |
| --- | --- | --- |
| 5 | An instance that sets a module's `const`s is checked by naming it with `--main`, with that instance's `init` and `step`. Not read | step 2: one property over a constant passes through the instance whose value makes it true and is violated through the other; step 4 on the model |
| 6 | `typecheck` exits non-zero on an ill-typed module and needs no network. Not read | step 2 for the exit status; step 6 for the network, under §11's denial with both `curl` controls |
| 7 | The properties a `verify` checks are the ones named on its command line; with none named it falls back to the definitions declared with the `invariant` qualifier, and the model declares none, so such a run can pass having checked nothing. Read in `cliHelpers.js` (`getInvariants`) and `cliCommands.js`, not run | step 2: a `verify` naming no property passes on a module whose `Canary` is false, and is violated on the copy that declares one false `invariant`; step 4: `Canary` violated through `pachinko_walk` before any pass is read |
| 8 | Quint uses an Apalache server already on `localhost:8822` before it starts its own. Read in `apalache.js` (`connect`), not run here | steps 2 and 4: the port probe before every `verify`, and at `--verbosity=4` the line `No running Apalache server found, launching...` present and `Connecting with existing Apalache server` absent |
| a | `typecheck` over a file checks every module in it, the instance at its foot included. Not read | step 2: a scratch copy whose instance sets a `const` to a value of the wrong type fails |
| b | One `--invariant` takes several names separated by commas. Read in `cliCommands.js` ("An `--invariant` may list several invariants separated by commas"), not run. `--invariants` is an array option and can take the input file's name as a property (read in `cli.js`, the comment on `observation`), so no command here uses it | step 2: two names that hold pass, and the same list with a false name second is violated |
| c | Apalache accepts the shapes the design needs: a list in the state, `prev` holding the whole state, sum types with record payloads, and an argument chosen from a set that depends on the state. Only `typecheck` was run, on a module with neither the list nor `prev` | step 2, before the maintainer rules |
| d | Without `--main`, a `verify` of `quint/pachinko.qnt` takes the module `pachinko`, whose constants are unset. Read in `cliHelpers.js` (`guessMainModule`, lines 68 to 78): a file with more than one module gives the module named as the file is; what the run then does is not read | step 2, on a scratch file whose base module carries the file's name: record what it prints and its exit status |
| e | The by-hand `verify` at `--max-steps=5` finishes in a time QM03 can put in a CI job. Not run | step 4: record each run's wall time |

## Non-goals

- No `quint verify` in any gate, recipe or CI job. QM03 adds `just verify-quint` and the
  job, and adds the recipe to the same-change rule in `AGENTS.md` and the `spec-change`
  skill. This ticket writes that rule with `just check-quint` only: no page, and no
  line of `AGENTS.md`, names `verify-quint` yet, because it does not exist.
- No publish and no joins. A record has no rank, payload, position or mark here.
- No pull with a positive limit, no `PullWithinLease`, no held record.
- No `consecutive_expiries` in the state. QM08 adds it.
- No pre-commit hook for Quint.
- No committed mutant and no witness recipe. QM03 creates `quint/mutants/`. The by-hand
  mutant stays under `ai_tmp/qm02/`.
- No `--server-endpoint` on any command, and no `verify` while `just install-apalache`
  or `just install-quint-evaluator` runs or another worktree verifies (shared rule 10).
- No clock, timestamp or TTL (shared rule 7).
- No change to the `quint` recipe or its guard.
- No `CHANGELOG.md` entry. It records the crates' changes.

## Shared rules (QM01 to QM13)

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

This ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. These deltas
apply, and every ticket of the series carries them word for word.

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

`<id>` is `QM02`. Checkpoints:

- Checkpoint 1, in step 3: the plan and the draft of decision 0022, with a design lens,
  before any tracked file changes and before the **Stop** for the maintainer's ruling.
- Checkpoint 2, in step 5: the model, with a design lens, before the gate and the pages
  take its names.
- Checkpoint 3, in step 9: the diff, before the final commit.

## Files touched

Each page row covers every sentence on that page that this ticket makes false: that
there is no Quint module or gate, that no job needs Quint, the count of gates, and what
a specification change carries. Step 8 searches for them. No mutant patch and no trace
fixture exists yet, so shared rule 11's first sentence has nothing to re-cut or
regenerate here. That sentence asks for `just verify-quint` only "once QM03 has landed
that recipe": this ticket does not run it and does not write it. Step 4's by-hand loop
stands in its place, as shared rule 6 says ("QM02 shows the same by hand"). The rule's
last sentence is the one step 8 writes into `AGENTS.md` and the skill.

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | new | the module `pachinko` and the instance `pachinko_walk`, with a header comment that names decision 0022 and lists "Not yet modelled" |
| `scripts/check-quint.sh` | new | the gate's script, POSIX `sh` |
| `docs/decisions/0022-quint-model-of-the-queue.md` | new | decision 0022, from the draft the maintainer ruled on |
| `Justfile` | shared, own lines | `check-quint`, with a comment, in the documents section; the comment over `install-quint`, which says no job needs Quint until a gate does |
| `pyproject.toml` | shared, own lines | `"check-quint"` appended to `recipes` |
| `.github/workflows/ci.yml` | shared, own lines | the `documents` job: `just install-quint` beside `just install-allium`, with a comment, and `just check-quint` after `just analyse-specs` |
| `.github/workflows/release.yml` | shared, own lines | `just install-quint` beside `just install-allium`, before `just check` |
| `.github/actions/setup/action.yml` | shared, own lines | the description's last sentence and the last comment, which name the jobs' own install steps |
| `AGENTS.md` | shared, own lines | `quint/` in the project map; the same-change rule in the block on changing a module under `docs/specs/` |
| `.agents/skills/spec-change/SKILL.md` | shared, own lines | the same-change rule as a step of the body; the frontmatter is frozen and no bridge changes |
| `docs/manifest.yml` | shared, own lines | decision 0022's entry |
| `docs/decisions/README.md` | shared, own lines | decision 0022's row, a sentence on what it records, and "The next decision is 0023" |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | only the two sentences this ticket makes false: "no job needs Quint until one runs a Quint gate" and "There is no Quint gate yet"; each now names the gate and decision 0022 |
| `docs/reference/quality-gates.md` | shared, own lines | gate 21's row and `check-clean` as 22; the per-worktree install beside gates 17 and 18; a row in "When a gate fails"; the `documents` row of the CI table |
| `docs/reference/commands.md` | shared, own lines | a `just check-quint` row under "Documents"; the paragraph under that table on per-worktree binaries; "The twenty gates" |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | "Change behaviour": the model moves in the same change, and `just check-quint` |
| `docs/explanation/specifications.md` | shared, own lines | what the model is and that it never leads, pointing at decision 0022 |
| `docs/project/repository-map.md` | shared, own lines | `quint/` in the tree and in the table; `check-quint.sh` in the `scripts/` row |
| `docs/how-to/develop-locally.md` | shared, own lines | "Quint": the model, `just check-quint`, `--main=pachinko_walk`, and that a worktree needs `just install-quint` before `just check` passes |
| `docs/operations/troubleshooting.md` | shared, own lines | a section headed by the gate's message for a missing or stale CLI |
| `docs/reference/testing.md` | shared, own lines | only if a sentence there says what checks the design itself; the author found none |
| `.scratch/quint-model/QM02-tracer-claim-and-release.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM02's Status cell; and, only if a ruling at step 3 differs from a recommended answer, the sentences it changes there |
| `.scratch/quint-model/QM03-*.md` to `QM13-*.md` | shared, own lines | only if a ruling at step 3 differs from a recommended answer: the sentences it changes, each listed in the hand-back |
| `tickets/T39-decide-q2-q6.md`, `tickets/T40-pocket-order-model.md`, `tickets/T42-clock-model.md`, `tickets/T43-caller-contract.md`, `tickets/T45-drop-joining.md`, `tickets/T46-no-claim-without-records.md`, `tickets/T47-start-rule-and-config-bounds.md` | shared, own lines | only if a ruling at step 3 differs from a recommended answer, and only in a ticket still `todo`: the paths and commands that its "The Quint model" paragraph and its `quint/pachinko.qnt` row take from this ticket, each listed in the hand-back |

## Steps

1. **Worktree and baseline.** Create the worktree on
   `ticket/qm02-tracer-claim-and-release` from the series base the maintainer names,
   once QM01 has merged there, and record the base's full commit id as `base`. Show
   that `git ls-files .scratch/quint-model/` lists the README and this ticket, and that
   `tools/quint/evaluator.txt` (created by QM01) exists. Run `just initialize` (network,
   authorised): a new worktree has no `tools/quint/node_modules/` and no `.quint/`, and
   this ticket needs the CLI, Apalache and the evaluator, since a `verify` that reports
   a violation starts the evaluator (the series README's claim 1, QM01's). Run
   `just check` on the untouched worktree and quote its last line, headed by the commit.
   Quote `just --show quint`. This ticket's `verify` runs rely on that recipe unsetting
   `grpc_proxy`, `https_proxy` and `http_proxy`, as QM01 recommends ("The proxy
   variables"): the CLI's gRPC client would otherwise send its loopback connection
   through a proxy the port probe never sees. If the recipe's `env` does not unset
   them, QM01's ruling was the alternative, and every `verify` in this ticket, in
   steps 2 and 4 and in Verification, is run under
   `env -u grpc_proxy -u https_proxy -u http_proxy`; say which in the hand-back.
   Show that
   `docs/decisions/` holds no `0022-*` file. If one exists, take the next free number
   and read "0022" and "0023" throughout this ticket as that number and the one after
   it: in the file's name, its title, its line in `docs/manifest.yml`, its row in
   `docs/decisions/README.md` and "The next decision is". Say so in the hand-back, with
   the record's path. The slug `quint-model-of-the-queue` and the key
   `canonical_for: [decision_quint_model]` do not move with the number: later tickets
   find the record by that key.
2. **Read, probe and plan.** No tracked file changes in this step.
   - Read the Allium items Context lists, at `base`.
   - Write `ai_tmp/qm02/shape.qnt`: a module with constants, a state that is one record
     holding a list of records with a sum-typed field, the four ghosts with `prev`
     holding the whole state and `last` written by `step` beside the action it calls,
     not by the action, a `Step` sum type with record payloads, an action whose
     argument `step` chooses from the list's indices and the index past its end, a
     `Canary`, two instance modules that set the constants, and properties that hold
     and use each operator the planned ones need: equality of the whole state with
     `prev`, a quantifier over pairs of the list's entries, equality of `fired` with a
     set built from guards, and a match on `last`. Three things make the probes below
     depend on what their claims say. The base module is named `shape`, as the file
     is, and the instances `shape_walk` and `shape_other`: without `--main` the CLI
     takes the module named as the file is when a file holds more than one (claim d),
     and under any other name the run stops at "module shape does not exist" before a
     constant is read. The two instances give one constant different values, and a
     property `ConstIsWalk` holds only for `shape_walk`'s. No definition has the
     `invariant` qualifier.
     Write `ai_tmp/qm02/illtyped.qnt`, a module with one definition whose body is not
     of its annotated type; `ai_tmp/qm02/badinstance.qnt`, a copy of `shape.qnt` whose
     instance sets a constant to a value of the wrong type; and
     `ai_tmp/qm02/declared.qnt`, a copy of `shape.qnt` whose base module also has
     `invariant declaredFalse = false`.
   - Quote, with exit statuses: `just quint typecheck` on each of the three (claims 6
     and a: the first exits 0, the others do not).
   - Run the port probe as this ticket's Verification writes it: the positive control
     first, a listener the probe starts and `lsof` then lists, and after it `port_free`,
     which shows nothing listening on 8822. Repeat the silent half before every
     `verify` in this ticket.
   - On `shape.qnt`, each with `--verbosity=4` and its wall time recorded: a `verify`
     naming `--main=shape_walk` and `--invariant=Canary`, which must print
     `[violation] Found an issue`; the same naming two properties that hold, separated
     by a comma, which must print `[ok] No violation found` (claims 5, b and c); the
     same list with `Canary` put second, which must be violated, so the second name is
     read (claim b); `--invariant=ConstIsWalk`, which must pass through `shape_walk`
     and be violated through `--main=shape_other`, so the instance named is the one
     whose constants are used (claim 5); the first command naming no property, which
     must pass (claim 7); and the same without `--main` (claim d). On `declared.qnt`,
     the command with `--main=shape_walk` and no property flag must be violated, and
     its log must name `declaredFalse` on the cross-mark line: that is the fallback
     claim 7 describes, and without it the pass on `shape.qnt` shows only that nothing
     was checked. `--main` is given because the file's name is not a module's, so a
     run without it stops at "module declared does not exist" and reads no property.
     Quote the server lines for claim 8.
   - If Apalache refuses a shape, write what it refused and the smallest change that it
     accepts into the plan, as a change to "Design proposed" for the maintainer.
   - Write `ai_tmp/qm02/plan.md`. For each of the three rules it shows the Allium's
     text beside the planned action, `guard_<Rule>` and `effect_<Rule>`, with the frame.
     For each property it names the ghost it reads. It gives the by-hand mutant as a
     diff and the one property that must catch it. It lists every "Not yet modelled"
     line with the ticket that removes it: the two joins and publish (QM04); a pull
     with a positive limit, `PullWithinLease`, `HoldRecord` and the held set (QM05);
     ack, nack, renew, a record's mark, the release that idles an empty pocket and the
     revival of an idle pocket (QM06); deferral (QM07); what `ending_records` holds
     (QM06 and QM07); the lapse, parking and `consecutive_expiries` (QM08); the clock,
     every timestamp, the TTLs and decay (QM09). It gives the seed, the outline of
     `scripts/check-quint.sh`, the edit to each file in Files touched, and the outcome
     of each claim so far.
   - Write `ai_tmp/qm02/decision-draft.md`, the whole text of decision 0022.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14 with the deltas
   above: conformance, breakage and design, over `ai_tmp/qm02/plan.md` and
   `ai_tmp/qm02/decision-draft.md`, before any tracked file changes). Questions:
   - For each of the three rules, does the planned guard say what the Allium's
     `requires` say and the planned effect what its `ensures` say, no more and no less?
     Is anything in the rule left out without a "Not yet modelled" line?
   - Is each `effect_<Rule>` written from the Allium, or from the action? Could
     `FiredMatchesGuards` and every effect hold while an action does the wrong thing?
   - Can each of the six invariants be false in a state the planned types allow? Is
     one true by representation without the decision draft saying so?
   - Is `PullFindsNoPocket` reachable from the planned seed? What is the shortest trace?
   - Does the decision draft claim more than this ticket shows: a `verify` in a gate, a
     proof where there is a bound, a witness for a rule that has none?
   - Could `Canary` be violated, or a mutant "caught", by something other than the
     property: a type error, an unknown `--main`, a server this worktree did not start?

   **Stop** for the maintainer's ruling, with the findings and the probe output in
   hand, on: each heading under "Design proposed", the text of decision 0022, and each
   open point that names this **Stop**. Record each ruling. Edit what a ruling changes
   in the later tickets and the series README, and list each sentence in the hand-back.
4. **The model, and ruling 4's loop by hand.** Write `quint/pachinko.qnt` to the ruled
   plan, with every section in the Allium's order and a header comment that names
   decision 0022 and holds "Not yet modelled". It is indented by two spaces, as
   `.editorconfig` sets for a file with no entry of its own. No guard and no effect on
   the state reads a ghost. Show `just quint typecheck quint/pachinko.qnt` exits 0.
   Then run the five checks below, each with the port probe before it,
   `--main=pachinko_walk`, `--max-steps=5`, its output written under `ai_tmp/qm02/`,
   and its wall time recorded (claim e). Five steps is the shortest trace in which a
   release names an ended lease while a later lease holds the same pocket: a claim, its
   release, a claim of the other pocket, a claim that leases the first pocket again,
   and the release that names the first lease. `PullFindsNoPocket` is reached in three.
   - **Red, the canary.** `--invariant=Canary` exits non-zero and prints
     `[violation] Found an issue`. Quote the last state of the trace.
   - **Green, the pass.** With `--verbosity=4` and `--invariant=` naming every property
     under "The properties, and how a run names them", it prints
     `[ok] No violation found`, the launching line is present and the connecting line
     is absent (claim 8).
   - **The witness.** `--invariant='not(witness_ReleaseRequeuesPocket)'` is violated.
     Quote the last state: `fired` holds `ReleaseRequeuesPocket`, `last` is a release,
     and against `prev` the lease is released and the pocket is queued at the back.
   - **The mutant.** Save the diff from the plan as `ai_tmp/qm02/mutant.patch`: in the
     claim, the pocket stays queued where it should become leased. Apply it to a copy,
     `ai_tmp/qm02/mutant/pachinko.qnt`. Show that the copy differs from the model, that
     it typechecks, and that `--invariant=OneActiveLeasePerPocket` alone is violated on
     it. Quote the last state: two active leases name one pocket. The pass above
     names that property at the same bound, and that is what shows it holds on the
     unchanged model (shared rule 6). Verification also reruns the one property on
     `quint/pachinko.qnt`, as an extra.
   - **The refusal** ("The refusal of an ended lease", as ruled).
     `--invariant='not(staleReleaseRefused)'` is violated. Quote the last state: `last`
     is a release that names an ended lease, `prev` holds an active lease of the same
     consumer on the same pocket, `fired` is empty, `answer` is nothing, and the state
     equals `prev`. If the run passes, the model never takes that step, or does not
     refuse it: fix the model, never the definition.

   Each of the four red runs must be a counterexample and not a deadlock or an error:
   its log holds `[violation] Found an issue` and `found a counterexample`, the
   explanation the CLI gives a counterexample (`apalache.js` line 144, read, not run;
   line 146 gives a deadlock `reached a deadlock`, and the CLI prints `[violation]`
   for both). Each names one property, so a counterexample is that property's.

   The canary, the witness, the mutant and the refusal each report a violation, which
   starts the evaluator. They rest on the copy `just install-quint-evaluator` (created
   by QM01) installed in step 1, and on QM01's guard in the `quint` recipe. Show that
   Quint downloaded nothing: after a positive control on a line each log does hold, the
   line `Fetching Rust evaluator` (read in `rust/binaryManager.js`, not run) is in none
   of the four logs. After the last run, show that nothing listens on 8822. A
   counterexample against the unchanged model is handled as shared rule 2 says, never
   by weakening a property.
   **Stop** and ask the maintainer if a trace shows the Allium wrong or silent, or if a
   run does not finish in a time a CI job could hold.
5. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14 with the deltas
   above: conformance, breakage and design, over `quint/pachinko.qnt` and the output of
   step 4, before the gate and the pages take the model's names). A fix that changes the
   model re-cuts `ai_tmp/qm02/mutant.patch` and reruns step 4's runs. Questions:
   - Read the model against the Allium's lines. Does any guard have a condition the
     Allium lacks, or lack one it has?
   - Does a guard, or an effect on the state, read `fired`, `last`, `answer` or `prev`?
   - Is there a step in the domain that the model cannot take, or one that changes the
     state while `fired` is empty? Does `step` offer a release for every index of the
     list and for the one past its end?
   - Does the refusal run's last state show the Allium's case, the same consumer
     holding the same pocket under a later lease, and does `staleReleaseRefused` read
     `prev` and no `guard_` definition?
   - Is `last` written by `step` from the arguments it chose, as ruled, and by no
     action? Could an action act on a lease other than the one it was given without a
     property failing?
   - Does `effect_ReleaseRequeuesPocket` state the pocket's position and the counter
     from the Allium's lines, or through the definition the action calls?
   - Has `Rule` exactly the three variants this ticket owns?
   - Does the passing run's command name every property the model defines?
   - Does the mutant's last state show what `OneActiveLeasePerPocket` forbids? Does the
     pass at the same bound name that property?
   - Are the names shared rule 3's, letter for letter? Does every property start with
     an upper-case letter or `effect_`, and every other definition with a lower-case
     one?
6. **Red, then green: the gate.** Add `scripts/check-quint.sh`, the `check-quint`
   recipe and the line in `pyproject.toml`. Show `just check-quint` passing and quote
   its line. Then, quoting each message and exit status, and restoring after each.
   Each message must be the gate's own for that case, with the fragment "The gate"
   lists; a failure for another reason is not the control:
   - **Zero files.** Copy `quint/pachinko.qnt` to `ai_tmp/qm02/saved/` and remove it
     from `quint/`, leaving the directory. The gate fails and says no file was found.
     Copy it back and show `cmp` of the two exits 0.
   - **An ill-typed module.** Copy `ai_tmp/qm02/illtyped.qnt` to
     `quint/zz_illtyped.qnt`. The gate fails and names that file. Remove it.
   - **A wrong version.** Save the installed
     `tools/quint/node_modules/@informalsystems/quint/package.json` under
     `ai_tmp/qm02/saved/`, change its `version` to `0.33.0`, and show the file changed.
     The gate fails and names `just install-quint`. Restore it and show `cmp` exits 0.
   - **No install.** Move that `package.json` aside. The gate fails the same way and
     does not run another `quint` from `PATH`. Move it back and show `cmp` exits 0.
   - **A linked entry point.** Move the installed entry point, `cli.js` under the
     package's `dist/src/`, to `ai_tmp/qm02/saved/cli.js` and put a symlink to that copy
     in its place, so the version in `package.json` is still the pin. The gate fails,
     names the link and `just install-quint`, and runs no typecheck. Remove the link,
     move the file back, and show `just check-quint` passing.
   - **Offline** (claim 6). Run `just check-quint` under §11's denial for the platform,
     on macOS `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with both
     of §11's `curl` controls failing inside the same denial. Quote all three exit
     statuses.

   Show `just check-quint` passing again and `git status --short` listing only this
   ticket's files.
7. **CI and the release workflow.** In `.github/workflows/ci.yml`, add
   `just install-quint` to the `documents` job beside `just install-allium`, with a
   comment that says why it is not in `sync`, and `just check-quint` after
   `just analyse-specs`. In `.github/workflows/release.yml`, add `just install-quint`
   before `just check`. Bring the setup action's description and last comment into
   line. Then search for every other caller: quote
   `rg -n 'just check($|[^-a-z])|bg-project-check' .github scripts .pre-commit-config.yaml`
   and say, for each line, why it needs no install step or that it now has one. At this
   ticket's writing the only line that runs the gate is `release.yml`'s. Run
   `just lint`, which runs `actionlint`, `shellcheck`, `typos` and
   `editorconfig-checker` over the new files.
8. **Words.** Write `docs/decisions/0022-quint-model-of-the-queue.md` from the ruled
   draft, its line in `docs/manifest.yml` and its row in `docs/decisions/README.md`.
   Add `quint/` to the project map in `AGENTS.md`, and the same-change rule to its
   block on `docs/specs/` and to `.agents/skills/spec-change/SKILL.md`: a change to
   `docs/specs/pachinko.allium` that touches a rule, invariant or guarantee the model
   has updates `quint/pachinko.qnt` in the same change and runs `just check-quint`.
   Edit each page in Files touched; each topic has one owning page, so add to that page
   and do not restate it elsewhere (the `review-docs` skill). Then search for what
   this ticket made false. Quote the search below after its positive control, the same
   search for `quint` alone, and show that each line left is true:

   ```sh
   rg -n -i 'no quint|quint gate|needs quint|twenty gates' \
     --glob '!tickets/**' --glob '!.scratch/**' --glob '!ai_tmp/**' \
     --glob '!tools/quint/node_modules/**' .
   ```

   Run `just check-docs` and `just check-agents`.
9. **Adversarial review** (checkpoint 3, `tickets/CONVENTIONS.md` §14 with the deltas
   above: conformance and breakage, over `git diff "$base"...HEAD` plus the uncommitted
   changes, before the final commit). A fix that changes the model reruns step 4's
   runs. Questions:
   - Can `just check-quint` pass with a stale CLI, with no CLI, with zero files, or
     with an ill-typed instance module? Can it fall through to a `quint` on `PATH`?
   - Does `just check` stay offline and read-only with the new gate? Does the gate
     leave a file Git can see, and does anything else sweep the new top-level
     directory: the Docker build context, `typos`, lychee?
   - Does every caller of `just check` install the CLI first: the `documents` job,
     `release.yml`, any other workflow, hook or skill?
   - Does any page still say there is no Quint module or gate? Do the pages agree on
     the gate's number, on what "verified" means and on who shows it until QM03?
   - Does `AGENTS.md`, the skill, the decision or a page name a command that does not
     exist?
   - Could Verification pass if a `verify` errored where it should have found a
     violation, or talked to a server this worktree did not start?
   - Does `step`, as the diff leaves it, offer each action over the whole of its
     domains whatever the state holds?
   - Does `props` in Verification name every property the model defines, the ones
     step 3's rulings added included?
10. **Commit and hand back.** Commit the work. Run `git fetch origin` (network,
    authorised), merge or rebase onto the series base, rerun `just check`, and rerun
    Verification and quote it, headed by the full commit id. If the merge or rebase
    changed a file under Files touched, checkpoint 3 did not see that change: show its
    diff at the stop and ask whether it is reviewed before the push (the review deltas,
    "After the last checkpoint"). Set `status: done` here and in the index of
    `.scratch/quint-model/README.md`, write the hand-back notes, and commit the record.
    Report the five owned invariants no mutant aims at as checked and not
    mutation-tested (shared rule 6). Then run `just check` once more, on the record's
    commit: the Verification quoted in the notes ran on the commit before, and the hook
    gate has not yet read the notes. Show its last line and an empty
    `git status --short` at the stop. If it fails, fix the notes, commit and run it
    again; never amend the work's commit. **Stop** before pushing or opening a pull
    request: both need authorisation (`tickets/CONVENTIONS.md` §11, "Separately
    authorised actions").

## Acceptance criteria

- `quint/pachinko.qnt` holds the module `pachinko` and the instance `pachinko_walk`,
  with `guard_`, `effect_` and `witness_` for each of the three rules, the six
  invariants under their Allium names, `FiredMatchesGuards`,
  `UnansweredStepChangesNothing`, `Canary`, the four ghosts,
  and a header comment with "Not yet modelled" that names a ticket on every line.
  Evidence: the file, and checkpoint 2's record.
- By hand, at `--max-steps=5` on `pachinko_walk`, each quoted with its command, its
  exit status and its wall time: `Canary` is violated; every property passes, named on
  the command line, with the port free before and after and a log that shows Quint
  launching its own server and joining none; `witness_ReleaseRequeuesPocket`
  is reached; the mutant typechecks and is caught when `OneActiveLeasePerPocket` alone
  is named, a property the pass names at the same bound; and `staleReleaseRefused` is
  reached, its last state quoted. Each red run's log holds `found a counterexample`.
- `Rule` has the three variants this ticket owns and no other.
- `just check-quint` passes, and fails on zero files, on an ill-typed module, on a CLI
  whose version is not the pin, on a missing CLI and on a linked entry point, each
  quoted with the gate's own message for that case. It exits 0 under the network
  denial, with both negative controls failing.
- `"check-quint"` is in the `recipes` list of `pyproject.toml`, and `just check` runs
  it.
- The `documents` job runs `just install-quint` before `just check-quint`, and
  `release.yml` runs it before `just check`. The search of step 7 is quoted.
- Decision 0022 is registered in `docs/manifest.yml` and `docs/decisions/README.md`,
  and states each thing listed under "Decision 0022". `just check-docs` passes.
- `AGENTS.md` names `quint/` and the same-change rule, the `spec-change` skill names the
  rule, and `just check-agents` passes.
- The pages say that a worktree needs `just install-quint` before `just check` passes,
  and no page says there is no Quint module or gate.
- Each claim, 5 to 8 and a to e, has its outcome recorded with the output that shows it.
- Each ruling of step 3 is recorded, with every sentence it changed in a later ticket.
- The three checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and both hold again on the
  record's commit.

## Verification

Run from a file as one `sh` script, from the repository's root, after the commit;
`set -eu` stops it at the first command that fails. It uses the scratch files steps 2
and 4 left under `ai_tmp/qm02/`. The trap puts back the model and the installed
`package.json` if a run stops while one is moved, and a signal leaves through the same
trap. The script keeps its backups in `ai_tmp/qm02/v/`, apart from step 6's copies, and
refuses to start while one is still there: that is an interrupted run's, and it is put
back by hand. The `verify` lines are written for a `quint` recipe that unsets the proxy
variables (step 1); if it does not, each gains
`env -u grpc_proxy -u https_proxy -u http_proxy` before `just`. `props` is the list
under "The properties, and how a run names them" as step 3 ruled it: a property the
ruling added, `ActionsNameTheirLease_release` for one, is added to `props` before the
script is run, and checkpoint 3 compares `props` with the model's properties.

```sh
set -eu
git rev-parse HEAD

v=ai_tmp/qm02/v
mkdir -p "$v" ai_tmp/qm02/mutant
model=quint/pachinko.qnt
pkg=tools/quint/node_modules/@informalsystems/quint/package.json

# A leftover is an interrupted run's backup. Put it back by hand before a rerun.
for f in pachinko.qnt package.json; do
  test ! -e "$v/$f"
done
restore() {
  rm -f "${v:?}/package.tmp" quint/zz_illtyped.qnt
  [ ! -f "$v/pachinko.qnt" ] || mv "$v/pachinko.qnt" "$model"
  [ ! -f "$v/package.json" ] || mv "$v/package.json" "$pkg"
}
trap restore EXIT
trap 'exit 1' HUP INT TERM

# gate_fails <name> <text>: the gate fails, and its output holds <text>.
gate_fails() {
  if just check-quint >| "$v/$1.log" 2>&1; then exit 1; fi
  grep -F -- "$2" "$v/$1.log"
}
# counterexample <log>: a trace that is a counterexample, not a deadlock, and no download.
counterexample() {
  grep -F '[violation] Found an issue' "$1"
  grep -F 'found a counterexample' "$1"
  if grep -F 'Fetching Rust evaluator' "$1"; then exit 1; else test "$?" -eq 1; fi
}

# The gate: green, three reds each for its own reason, green.
just check-quint
mv "$model" "$v/pachinko.qnt"
gate_fails zero-files 'no .qnt file'
mv "$v/pachinko.qnt" "$model"
git diff --exit-code "$model"
cp ai_tmp/qm02/illtyped.qnt quint/zz_illtyped.qnt
gate_fails ill-typed 'quint/zz_illtyped.qnt'
grep -F 'typechecking failed' "$v/ill-typed.log"
rm -f quint/zz_illtyped.qnt
cp "$pkg" "$v/package.tmp"
mv "$v/package.tmp" "$v/package.json"
sed 's/"version": "0\.34\.0"/"version": "0.33.0"/' "$v/package.json" >| "$pkg"
if cmp -s "$pkg" "$v/package.json"; then exit 1; fi
gate_fails wrong-version 'just install-quint'
mv "$v/package.json" "$pkg"
just check-quint

# The port probe. lsof exits 1 when nothing matches, so the status is read directly.
port_free() {
  if lsof -nP -iTCP:8822 -sTCP:LISTEN; then return 1; else test "$?" -eq 1; fi
}
.pixi/envs/default/bin/python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
port_free

# Ruling 4's loop by hand: the canary, the pass, one witness, the refusal, one mutant.
props=OneActiveLeasePerPocket,OneActiveLeasePerConsumer,LeasedHasOneActiveLease,UnleasedHasNoActiveLease,UniquePocketPosition,PocketPositionsBelowNext,effect_PullClaimsFrontPocket,effect_PullFindsNoPocket,effect_ReleaseRequeuesPocket,FiredMatchesGuards,UnansweredStepChangesNothing
if just quint verify --main=pachinko_walk --max-steps=5 --invariant=Canary "$model" >| ai_tmp/qm02/canary.log 2>&1; then exit 1; fi
counterexample ai_tmp/qm02/canary.log
port_free
just quint verify --main=pachinko_walk --max-steps=5 --verbosity=4 --invariant="$props" "$model" >| ai_tmp/qm02/pass.log 2>&1
grep -F '[ok] No violation found' ai_tmp/qm02/pass.log
grep -F 'No running Apalache server found, launching' ai_tmp/qm02/pass.log
if grep -F 'Connecting with existing Apalache server' ai_tmp/qm02/pass.log; then exit 1; else test "$?" -eq 1; fi
port_free
if just quint verify --main=pachinko_walk --max-steps=5 --invariant='not(witness_ReleaseRequeuesPocket)' "$model" >| ai_tmp/qm02/witness.log 2>&1; then exit 1; fi
counterexample ai_tmp/qm02/witness.log
port_free
if just quint verify --main=pachinko_walk --max-steps=5 --invariant='not(staleReleaseRefused)' "$model" >| ai_tmp/qm02/refusal.log 2>&1; then exit 1; fi
counterexample ai_tmp/qm02/refusal.log
port_free
cp "$model" ai_tmp/qm02/mutant/pachinko.qnt
patch ai_tmp/qm02/mutant/pachinko.qnt ai_tmp/qm02/mutant.patch
if cmp -s "$model" ai_tmp/qm02/mutant/pachinko.qnt; then exit 1; fi
just quint typecheck ai_tmp/qm02/mutant/pachinko.qnt
if just quint verify --main=pachinko_walk --max-steps=5 --invariant=OneActiveLeasePerPocket ai_tmp/qm02/mutant/pachinko.qnt >| ai_tmp/qm02/mutant.log 2>&1; then exit 1; fi
counterexample ai_tmp/qm02/mutant.log
port_free
just quint verify --main=pachinko_walk --max-steps=5 --invariant=OneActiveLeasePerPocket "$model" >| ai_tmp/qm02/mutant-control.log 2>&1
grep -F '[ok] No violation found' ai_tmp/qm02/mutant-control.log
port_free

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The red tests are the three gate failures, the canary, the witness's negation, the
refusal's negation and the mutant. Each exits 1 when its command passes, and each must
also fail for its own reason. `gate_fails` looks for the gate's message for that case,
so a launcher that failed, or a gate that failed on something else, is not a pass. The
gate names the file for any failed typecheck, so the ill-typed control also looks for
`typechecking failed`, the CLI's own summary of a module that parsed and did not type
(read in `cliCommands.js` line 171, not run; step 2 quotes the real output, and a
different spelling is corrected here and recorded under Departures). The
green tests are the gate's two passes and the pass over every property. That pass names
`OneActiveLeasePerPocket` at the mutant's bound, which is what makes the mutant's
failure mean something (shared rule 6); the last `verify`, the unchanged model under
the mutant's own command, is an extra.

A `verify` that errors exits non-zero too, and the CLI prints `[violation]` for a
deadlock as for a counterexample (the series README's claim 10: read, not run, and
QM03's to check). So `counterexample` looks for `[violation] Found an issue`, which the
CLI prints only when the checker returned a trace (read in `cliReporting.js`,
`processApalacheResult`, not run), and for `found a counterexample`, the explanation
`apalache.js` gives at line 144 where a deadlock gets `reached a deadlock` at line 146
(read, not run). Each red run names one property, so a counterexample is a
counterexample to that property. If a control shows the CLI spells either line
otherwise, correct it here and record it under Departures. The lines do not show what
the trace holds: step 4 quotes each last state for that.

Four kinds of probe pass by printing nothing. `port_free` follows its positive control,
the `lsof` line that lists the control listener. The `grep` for the connecting line
follows the `grep` for the launching line, which reads the same log. Each `grep` for
`Fetching Rust evaluator` follows the two matches `counterexample` makes in the same
log. `git diff --exit-code` compares the committed model, since the script runs after
the commit, and the two `cmp -s` lines show that the edit, and the patch, changed their
files. Step 6 holds the missing-CLI and linked-entry-point controls and the network
denial; their output is quoted there.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### The by-hand runs

### Checkpoint 1: the plan and the draft of decision 0022

### Checkpoint 2: the model

### Checkpoint 3: the diff

### Departures

### Verification

## Open points

- **Two skills gain a line and are not in Files touched.**
  `.agents/skills/project-check/SKILL.md` lists what `just check` needs in a worktree,
  and `.agents/skills/fix-quality/SKILL.md` has a line for each gate. Neither says
  anything false after this ticket. `scripts/initialize.sh`'s comment over
  `just install-quint` is the same. The maintainer says, at step 3 or at the hand-back,
  whether this ticket adds the lines or a follow-up does.
- **The by-hand bound.** `--max-steps=5` is this ticket's, for the reason step 4 gives.
  QM03 sets the bound `verify-quint` uses. If a run here does not finish in a time a CI
  job could hold, stop and ask at step 4: the domains are a ruling and are not shrunk.
- **Which property a trace violates.** The exit status and the `[violation]` line do not
  say. This ticket reads each last state by hand. QM03 settles what counts as a mutant
  caught by its named property.
- **The witness's form.** This ticket shows a witness as a violation of its negation,
  the mechanism the canary already exercises. QM03 chooses between that and
  `--observation` for the recipe.
- **The partial guarantees are named for what they say.** `ExclusivePocket` and
  `ActionsNameTheirLease` appear in the model under their names only when QM05 and QM06
  state them. Until then decision 0022 says which properties stand for their part, and
  that the part of `ActionsNameTheirLease` rests on one by-hand run that no gate
  repeats ("The refusal of an ended lease"). If the maintainer wants a
  `<Guarantee>_<part>` property for either, step 3's **Stop** is where to say so. No
  record is handed out before QM05, so `ExclusivePocket`'s own words, "no other
  consumer is handed a record", have nothing to speak of here: the four lease
  invariants are its part at the lease's level and nothing more.
- **What ties `last` to the choice.** Every check of a refusal reads `last`. The
  design proposes that `step` writes it and no action does; the maintainer rules at
  step 3's **Stop**. Under the alternative nothing but checkpoint 2's reading shows
  that an action recorded the lease it was given, and QM06, which states
  `ActionsNameTheirLease` over `last` and `prev`, inherits the same gap.
- **The lease never issued.** `step` offers a release at the index past the list's
  end, and no run here shows it taken and refused. QM06's `ActionsNameTheirLease`
  covers it. Settled by QM06.
- **T46 and T43.** T46 removes the claim this model makes at a limit that is not
  positive, and lands after QM05. T43 rules on the lease id a caller sees. Whoever runs
  either applies shared rule 11 to the model.
