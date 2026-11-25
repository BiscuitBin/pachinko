---
id: QM13
title: "Follow-up: inductive lease exclusivity"
status: todo
depends_on: [QM08]
parallel_with: [QM09, QM10, QM11, QM12]
branch: ticket/qm13-inductive-exclusivity
estimated_size: M
---

# QM13: Follow-up: inductive lease exclusivity

## Context

**This ticket may end with "not achievable as modelled, and why", and that is an
acceptable outcome.** It is the "option 2" the maintainer deferred in ruling 4 and asked
to try later. The model's state grows without bound: ended leases stay in it, the two
shed queues are lists that only grow before QM09's decay, and the ghost `prev` holds a
whole earlier state. `--inductive-invariant` takes one `val` and uses it both as the
step's start state and as what the step must preserve, so it has to describe every
such state without bounding its size, in a form Apalache can start from. Whether the
pinned tools accept that was neither read nor run. So step 2 probes it on scratch
modules before any tracked file changes, and **stops** for the maintainer if the probe
fails. The details are at the end of this section.

After QM08 the model in `quint/pachinko.qnt` has every rule that grants or ends a lease:
the claim, the release and the untimed lapse. `just verify-quint` checks every property
on `pachinko_walk` with a bounded `quint verify`, so `OneActiveLeasePerPocket` and
`OneActiveLeasePerConsumer` are known to hold on every run of at most `--max-steps`
steps, and no further. Ruling 4 of `.scratch/quint-model/README.md` says so and defers
the alternative: "Inductive invariants are a follow-up, QM13."

An inductive invariant removes the bound. It is a predicate that holds in every start
state and that every step preserves, so it holds after any number of steps. If it also
implies the two exclusivity invariants, they hold after any number of steps too.

The two invariants, in `docs/specs/pachinko.allium` at commit `cf1f3f3` (cite by name
if the lines have moved):

- `OneActiveLeasePerPocket` (lines 1030 to 1035): no two distinct active leases name
  one pocket.
- `OneActiveLeasePerConsumer` (lines 1037 to 1040): a consumer's `active_leases.count`
  is at most 1.

Neither is inductive alone. `PullClaimsFrontPocket` (lines 527 to 554) is the only rule
with `Lease.created` (line 537). It requires that the consumer has no active lease
(line 529) and claims a pocket whose `status` is `queued` (lines 530 to 535). It does not
ask whether that pocket already has an active lease: `UnleasedHasNoActiveLease` (lines
153 to 155) is what says a pocket that is not leased has none. Start the step from a
state where a queued pocket has an active lease, which `OneActiveLeasePerPocket` alone
allows, and one claim makes two. So the inductive invariant is stronger than the two it
proves, and finding how much stronger is this ticket's work. The `Lease` entity (lines
257 to 278) has `terminal: released, expired` (line 268), so an ended lease never
becomes active again; `LeasedHasOneActiveLease` is at lines 149 to 151.

The pinned CLI has a flag for this. Read in the installed source under
`tools/quint/node_modules/@informalsystems/quint/dist/src/`, version 0.34.0, and not
run:

- `cli.js`, lines 345 to 348: `verify` takes `--inductive-invariant`, a string, "Can be
  used together with ordinary invariants."
- `cliCommands.js`, lines 516 to 517: the flag's text is spliced into the module as
  `val q::inductiveInv = <text>`. It is a `val`: a definition's name or an expression,
  not an action.
- `verify.js`, lines 198 to 228, runs up to three Apalache checks in a row:
  1. line 205: the start. `maxSteps` 0, the invariant `q::inductiveInv`, from the
     ordinary `init`.
  2. line 215: the step. `maxSteps` 1, the invariant `q::inductiveInv`, and
     `q::inductiveInv` itself as the initial predicate (the fifth argument of
     `createConfig`, `apalache.js` line 98). The check starts from any state that
     satisfies the invariant, not from `init`.
  3. line 222: the implication. `maxSteps` 0, the invariant `q::inv`, the ordinary
     invariants, again from `q::inductiveInv`. It runs only when
     `invariantsList.length > 0` (lines 203 to 204 and 217).

  `--max-steps` on the command line is overwritten in all three. `--observation` and
  `--search-observations` are refused with the flag (lines 199 to 202).
- `cliHelpers.js`, lines 117 to 125: with no `--invariant` and no `--invariants`, the
  ordinary invariants are the `invariant` definitions the main module declares (lines
  91 to 96), which may be none. "No invariant named" is therefore not the same as "no
  third check" unless the main module declares none.
- `cliReporting.js`, lines 435 to 448: before each check the CLI prints
  `> [1/N] Checking whether the inductive invariant '<name>' holds in the initial
  state(s) defined by '<init>'...`, `> [2/N] Checking whether '<step>' preserves the
  inductive invariant '<name>'...` and `> [3/3] Checking whether the inductive invariant
  '<name>' implies '<invariants>'...`, where N is 2 or 3. They print only when
  `verbosity.hasProgress` holds, at level 2 or above (`verbosity.js`, lines 65 to 67).
  `verify` declares `--verbosity` with no default (`cli.js`, lines 367 to 370), and its
  handler sets it to the default level, 2, for the Apalache backend when the flag is
  absent (lines 407 to 410).
- There is no line that says which check failed. A failed check prints
  `[violation] Found an issue` only when Apalache returned a trace (`cliReporting.js`,
  lines 243 to 246); an error prints no such line. A pass prints `[ok] No violation
  found` once, after the last check (line 234). For the first two checks the violated
  invariant is not named, because no expression is handed to the reporter (`verify.js`,
  lines 211 and 218); for the third it is (line 224).

Two things follow, and they shape the ticket.

- **The invariant is the checker's start state for the step.** A Quint type does not
  bound a value: a set of leases may hold a lease whose pocket was never created, and an
  integer may be anything. Unless the invariant itself says what each state variable
  ranges over, the step starts from states no run can reach and fails for reasons that
  have nothing to do with leases. That part of the invariant is its *type-correctness
  conjunct*: for every `var` of the model, the four ghosts `fired`, `last`, `answer` and
  `prev` included, a clause that puts it in a set.
- **The model's state is not finite.** Ended leases stay in the state ("Leases are
  entities with an identity of their own, ended ones included", README, second design
  block). The two shed queues are lists that only grow before QM09's decay.
  `next_position` only grows, and `prev` holds a whole earlier state. A clause that
  bounds a collection's size is broken by the step that adds one entry, so it is not
  inductive. A clause that does not bound it may not be one Apalache can start from.
  Which clause shapes the pinned checker accepts as a start was neither read nor run;
  claim B below checks it before anything is built on it.

What is missing: no property of the model is proved beyond the bound, and no recipe
runs `--inductive-invariant`.

What the base must hold, besides QM08: QM01's pinned Rust evaluator
(`tools/quint/evaluator.txt`, installed by `just install-quint-evaluator`, which
`just initialize` runs). A `verify` that reports a violation starts the evaluator
(claim 1 of the series README), and every red control in this ticket is such a run.
The chain QM01 to QM08 implies it; step 1 confirms it.

Read these:

- `.scratch/quint-model/README.md`, whole; rulings 4 and 5, "Design fixed before QM01",
  claims 7, 8, 10, 11, 12 and 16, and shared rules 2, 5, 6, 10 and 11 above all.
- `docs/specs/pachinko.allium`: the `Lease`, `Pocket` and `Consumer` entities, the two
  exclusivity invariants, `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, and
  every rule that reads or writes a lease's or a pocket's `status`: `PullWithinLease`,
  `PullClaimsFrontPocket`, the two release rules, `LeaseLapses` and the three expiry
  rules, `PublishIntoNewPocket` and the two revive rules.
- `quint/pachinko.qnt`, `quint/verify.txt`, `scripts/verify-quint.sh` and
  `quint/mutants/` (created by QM02 and QM03), at the base.
- `.scratch/quint-model/QM03-verify-recipe-and-ci.md` and
  `.scratch/quint-model/QM08-untimed-lapse-and-parking.md` (created by the series):
  their "Design proposed in this ticket" and hand-back "Design rulings", for the format
  of `quint/verify.txt`, how a property is named on the command line, how the recipe
  makes sure the server that answers is its own, and the measured time.
- The CLI source cited above, and `builtin.qnt` lines 921 to 933 (`apalache::generate`,
  which "must be used together with `nondet`").
- `.scratch/apalache/AP01-java-and-verify.md`, Verification, for the port probe.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

The model gains one inductive invariant, `LeaseExclusivityInductive`, built from named
conjuncts: the type-correctness conjunct `StateInRange`, the two exclusivity
invariants, and whatever else the step needs. `just verify-quint` runs it on
`pachinko_walk` with `--inductive-invariant`, with `OneActiveLeasePerPocket` and
`OneActiveLeasePerConsumer` named as the ordinary invariants, and passes only when the
output shows all three checks ran and the third names both. The recipe also shows the
run can fail: a weakened invariant, one conjunct removed, is not preserved by the step,
and the output shows it was the step that failed.

What the result means, and the pages say so: the two invariants hold after any number
of steps of `step`, the untimed lapse included, at `pachinko_walk`'s domains. It is
not a proof for more publishers, consumers, pockets or keys than the instance has, and
it covers no other property.

What the shared rules mean for this ticket:

- *Shared rule 6, and this ticket's one departure from it.* Shared rule 6 speaks of
  mutants of the model, each "caught by the one property it names". This ticket's
  control is not a mutant of the model. It is a weakened property: the inductive
  invariant with one conjunct removed, over the unchanged actions. No named property
  catches it. It is rejected at a phase of the inductive run, the step, and which phase
  rejected it is read from the progress lines, as claim 16 says, never from the exit
  status. That is the stated departure from "the property it names"; everything else
  in rule 6 holds as written. The inductive run names its three properties on the
  command line and is trusted only after `Canary` was violated on `pachinko_walk` in
  the same recipe run. It fails closed on each of: a missing `> [1/3]`, `> [2/3]` or
  `> [3/3]` line; a `> [3/3]` line that does not name both invariants; no `[ok] No violation
  found`; a weakening patch that does not apply or does not typecheck, or whose header
  does not name the one conjunct it removes; a weakened run that passes, whose last
  progress line is not the `> [2/3]` one, that fails with no `[violation]` line, or
  that fails on a deadlock and not on a counterexample.
- *Shared rule 10.* One inductive run starts up to three checks against the Apalache
  server. The port probe runs before each `just quint verify`, never between its
  checks, and the log must show that the server every check used is the one this run
  started ("Design proposed"). A run that reports a violation starts the Rust
  evaluator (claim 1 of the
  README), so the weakened runs need the pinned evaluator that `just initialize`
  installed.
- *Shared rule 11.* This ticket changes `quint/pachinko.qnt`. It re-cuts every patch
  under `quint/mutants/` that the change breaks and runs `just verify-quint`. If the
  tree this ticket lands on holds `quint/traces/`, at the base or after the closing
  merge or rebase, it regenerates the fixtures with `just quint-traces`, before any
  gate runs, and keeps both replays green: the Rust test in `just test` and, if that
  tree holds `double/tests/test_replay.py`, `just python-double`. The new definitions
  are `val`s, not `var`s, so the fixtures
  gain no field and no step, and neither replay's not-compared list changes.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | `OneActiveLeasePerPocket` and `OneActiveLeasePerConsumer`, which QM02 owns, are proved for any number of steps. `LeaseExclusivityInductive` and `StateInRange` are model-only definitions with no Allium name |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them after checkpoint 1 and
before step 4, and the hand-back records the ruling. The series README names no design
point for QM13, so every item below is this ticket's own.

- **Names.** `LeaseExclusivityInductive` is the inductive invariant and `StateInRange`
  its type-correctness conjunct. The README fixes neither. Every other conjunct is a
  named definition: an Allium invariant under its Allium name, already in the model, or
  a new one whose comment says which Allium text implies it.
- **In the model's file, in a section of its own.** The definitions go in module
  `pachinko`, after the Allium's invariants, under a comment that says what the section
  is, what a new `var` must add to `StateInRange`, and how to read a failure of each
  check. Reasons: the README fixes one file that grows; `StateInRange` sits beside the
  `var`s it must cover; `check-quint` and every instance see it with no new glob. The
  cost: the file's hash changes, so on a tree that holds `quint/traces/` the fixtures
  are regenerated under shared rule 11, although no trace changes. Alternative: a
  second file beside
  the model that imports `pachinko_walk`. It leaves the hash alone. It also breaks "one
  file", needs `check-quint` and `verify-quint` to learn a second file, and rests on
  `--init` and `--step` resolving through an import, which was not read. Not
  recommended.
- **It runs on `pachinko_walk`.** The domains are a ruling (shared rule 5), and the two
  invariants are checked there already. Alternative: a smaller instance for this run
  alone. Every instance module is verified in full by `verify-quint`, so a new one adds
  every property's run, and a proof at smaller domains says less. Not recommended; if
  `pachinko_walk` does not fit the time, the ticket stops and asks (step 8).
- **The ordinary invariants are named, and the output is the evidence.** The command
  names `OneActiveLeasePerPocket` and `OneActiveLeasePerConsumer`, spelt the way
  `scripts/verify-quint.sh` names properties: QM03 proposes `--invariants` and the
  names, last on the line, with the model's path first. Read, not run: one
  `--invariant=A,B` is checked as well, but a violated invariant is named only when
  each entry of the list parses as an expression on its own (`verify.js`, lines 190 to
  196), and `A,B` is one entry that does not, so the third check would fail without
  saying which invariant broke. The command passes the verbosity the recipe passes on
  every run, 4 as QM03 proposes, and never less than 2, the level that prints the
  progress lines. A
  pass is accepted only when the log holds the `> [1/3]`, `> [2/3]` and `> [3/3]`
  lines, the `> [3/3]` line holds both names, and `[ok] No violation found` follows. A
  run that printed `[2/2]` proved neither invariant, whatever its exit status.
- **The server that answers each check is this run's.** QM03's recipe requires of
  every run's log the two lines that say Quint launched its own server, and refuses
  the line that says it joined one already running (QM03, "Design proposed", point 4).
  One inductive run makes up to three checks, and each connects afresh (`verify.js`,
  lines 172 to 175). Read, not run: the first check launches the server, which lives
  until Quint exits, so the second and the third join it and print
  `Connecting with existing Apalache server`, the line QM03's rule refuses. So the
  inductive run cannot use that rule as it stands, and without a rule the three
  banners and a free port beforehand do not show which server answered. Recommended,
  for the inductive run and the weakened run only: the log holds
  `No running Apalache server found, launching...` and exactly one
  `Started Apalache server on pid=` line, both before the `> [2/` line, and no
  `Connecting with existing Apalache server` line before the `Started` line. Claim F
  probes the lines. What the lines cannot show is under Open points: the joining line
  carries no process id. Alternative: three plain `verify` runs, one per check, each under
  QM03's rule unchanged; that is the bounded-start deliverable below under another
  name, since a plain run cannot start from the invariant. Not recommended.
- **A starting hypothesis for the strengthening, derived by reading the Allium and not
  run.** `StateInRange`; the two exclusivity invariants; `UnleasedHasNoActiveLease`;
  and that every lease names a pocket and a consumer that exist, which the Allium says
  by typing `Lease.pocket` as a `Pocket` and `Lease.consumer` as a `Consumer`. The
  reasoning is in Context. `LeasedHasOneActiveLease` is not expected to be needed. The
  checker's counterexamples to the step, not this list, decide the conjuncts.
- **A conjunct is never invented.** Each conjunct is one of: `StateInRange`; an Allium
  invariant, transcribed whole as the model already has it; or a statement the Allium
  implies, with the text that implies it quoted in its comment. A counterexample that
  can only be excluded by a statement the Allium neither makes nor implies stops the
  ticket (shared rule 2): the statement is put to the maintainer as a candidate
  invariant for the Allium, and is not added to the model alone.
- **`StateInRange` gives every `var` a set and bounds no growing collection's size.**
  For a finite domain it is membership in that domain. For a counter it is membership
  in an unbounded set, if the checker accepts one as a start. For a collection that
  grows (the leases, the two shed queues, and their copies in `prev`) it says what each
  element is made of and nothing about how many there are. Whether Apalache can start
  from such clauses is claim B. If it cannot, the fallback is not chosen here: a start
  bounded by `apalache::generate` needs a `nondet`, so it needs an action, and
  `--inductive-invariant` takes a `val`. That would mean three plain `verify` runs with
  `--init` naming a start action, and a proof only for states of bounded size. It is a
  different deliverable, and the ticket **stops** for a ruling rather than build it.
- **The weakening is a patch.** `quint/mutants/qm13-<slug>.patch` removes one conjunct
  from `LeaseExclusivityInductive` and changes nothing else. The recipe applies it to a
  scratch copy, typechecks the copy, runs the same inductive command on it and requires
  the step to fail on a counterexample. The evidence is QM03's for a mutant (its
  "Design proposed", point 2.4) less the line that names a property, which the step
  check does not print: the last progress line in the log is the `> [2/3]` one, and
  after it the log holds `[violation] Found an issue` and
  `error: found a counterexample`, and does not hold `error: reached a deadlock`. A
  deadlock is reported with a trace and the same `[violation]` line (`apalache.js`,
  lines 141 to 146; `cliReporting.js`, lines 243 to 246), and it says the step could
  not move, not that the weakened invariant was broken. Claim G probes the two lines.
  The conjunct removed is never one of the two exclusivity invariants.
  `UnleasedHasNoActiveLease` is the expected one. The phase that fails does not say
  which conjunct a patch removed. Read, not run: with `OneActiveLeasePerPocket` taken
  out, two active leases on one leased pocket are a start, a release of one leaves an
  unleased pocket with an active lease, and the step fails just as it does for the
  expected patch. So the patch's header names the conjunct, and the recipe checks the
  header against the patch (next bullet). Alternative: no patch, and the
  weakened invariant given as an expression to `--inductive-invariant`. It needs no
  scratch copy, and it drifts silently when a conjunct is added to the real invariant,
  where a patch that no longer applies fails the recipe. Not recommended.
- **How the run and the patch fit QM03's inventory.** QM03's recipe, as its ticket
  proposes it, knows one kind of line in `quint/verify.txt` (an instance and its
  `max-steps`) and one kind of patch under `quint/mutants/` (a mutant of the model,
  with `# mutant:`, `# instance:` and `# property:` lines, caught by a bounded run),
  and it takes every `val` with no parameter and an upper-case first letter for a
  property of every instance. Nothing of this ticket fits as it stands. An inductive
  line is malformed. The weakening would be run as a bounded mutant, which it
  survives, since a weaker invariant still holds from `init`. `StateInRange` and
  `LeaseExclusivityInductive` would join every instance's bounded pass. Step 2 reads
  what QM03 landed and the plan gives the exact lines. Recommended:
  1. `quint/verify.txt` gains a second kind of line,
     `<instance> inductive=<name> implies=<name>,<name>`, at most one for an instance.
     The recipe refuses it if the instance has no `max-steps` line, or if the
     invariant or an implied property is not in the model's list of properties.
  2. The weakening keeps its place and its name under `quint/mutants/`. Its header has
     `# weakening:`, `# instance:` and `# removes:` where a mutant's has `# mutant:`,
     `# instance:` and `# property:`. The recipe tells the two kinds apart by the first
     header line, refuses a patch that mixes them, runs a weakening through the
     inductive command and never through the bounded one, and refuses a weakening
     whose instance has no inductive line. `# removes:` names the conjunct taken out.
     The recipe checks it against the parsed model and the parsed copy: the names
     `LeaseExclusivityInductive` refers to differ by that one name and by nothing
     else, the name is not one of the implied properties, and every other definition
     of the copy is the model's, compared as QM03's point 2.6 compares one definition,
     ids and source positions aside. Without the last, a patch could take the conjunct
     out and also change an action, and the step's counterexample would say nothing
     about the conjunct. QM03's check that a
     patch leaves the property it names defined as the model defines it (its "Design
     proposed", point 2.6, `changes the property it names`) applies to the
     `# property:` kind only: a weakening changes `LeaseExclusivityInductive` on
     purpose and names no property, and the comparison above is its check in that
     one's place.
  3. `StateInRange` and `LeaseExclusivityInductive` stay properties, as the README
     names one, so every instance's bounded pass checks them too. An inductive
     invariant holds in every reachable state, so the pass is a second check on the
     conjuncts and never a wrong failure; what it costs is in claim 12's timing.

  Which file changes depends on where QM03 put the header check and the lists:
  `scripts/verify-quint.sh`, or its helper `scripts/verify-quint-model.py`, which
  QM03's ticket proposes. Alternatives. For 2: a directory of its own,
  `quint/weakenings/`, which QM03's sweep never reads; it leaves QM03's header rule
  alone and puts a patch outside the place shared rule 3 names. For 3: the helper
  leaves the two names out of the bounded pass, as it leaves `Canary` out; that is a
  second list of exceptions for no gain in what is proved. The maintainer rules at
  the **Stop** in step 4.
- **Each conjunct is shown necessary once, by hand.** In step 6 each conjunct other than
  `StateInRange` and the two exclusivity invariants is removed in turn. One whose
  removal still passes is deleted from the invariant, or kept with a reason in the
  hand-back. Only the patch above stays in the recipe, because each extra run costs CI
  time (claim 12).
- **When a later model ticket breaks the run, that ticket repairs it.** The inductive
  run is in `verify-quint`, so a model change that adds a `var` without a clause in
  `StateInRange`, or a rule that the conjuncts do not survive, turns the recipe red.
  The ticket that makes the change extends the invariant in the same change, as it
  re-cuts a mutant, and stops and asks if it cannot. It never removes the run, never
  drops one of the two ordinary invariants from it, and never deletes a conjunct to
  make it pass. For QM09 and this ticket, whichever merges second does the work: if
  QM09 is in the base, this invariant covers `now` and every timestamp from the start;
  if this ticket merges first, QM09 adds their clauses. Shared rule 11 has every model
  change run `just verify-quint`, which holds this run once this ticket lands; the
  comment above the invariant says what repairing it means.
  Alternative: the inductive run leaves the required job when it breaks. Not
  recommended: it would be dropped at the first clock change.

## Claims this ticket checks

Row 16 is the series README's and keeps its number, and row 12 is the README's too,
since this ticket adds a run. The lettered rows are claims this ticket rests on that
nobody has run. Every probe runs through `just quint`, on a scratch module under
`ai_tmp/qm13/`, with the port probe first.

| # | Claim | Check |
| --- | --- | --- |
| 16 | `verify --inductive-invariant` checks the start and the step, and then that the invariant implies the ordinary invariants named with it. Which check failed is read from the progress lines, not from the exit status. Read in `verify.js` lines 198 to 228 and `cliHelpers.js` lines 117 to 125, not run. An answer that differs is a design change: stop and bring it back to the README | step 2, probes 1 to 5 |
| 12 | The whole of `verify-quint`, with the inductive run and the weakened run added, finishes inside the `quint` CI job's timeout. Not read | step 8: the whole recipe timed before and after |
| A | The three progress lines print at `--verbosity=2`, and at the default, which is 2 for the Apalache backend. Read in `cliReporting.js` lines 435 to 448 and `cli.js` lines 367 to 370 and 407 to 410, not run. The recipe passes the flag whatever the answer, so that its evidence does not rest on a default | step 2, probe 1, run twice: with the flag and without |
| B | Apalache accepts as the step's start an invariant whose range clauses have the shapes `StateInRange` needs: membership in a finite set, membership in an unbounded integer set, a set of records described element by element, a list, a sum type, and a nested record such as `prev`. A `var` with no clause is an error, not a pass. Neither read nor run | step 2, probe 6, one scratch `var` per shape |
| C | A start that fails prints the `> [1/N]` line and `[violation] Found an issue` and no `> [2/N]` line; a step that fails prints `> [2/N]` and `[violation]` and no `> [3/3]`; an implication that fails prints `> [3/3]`, `[violation]` and, for each violated invariant, a line of two spaces, the cross mark U+274C, a space and its name. Read in `verify.js` lines 209 to 226 and `cliReporting.js` lines 165 to 174 and 243 to 252, not run | step 2, probes 2 to 4 |
| D | With no `--invariant`, the run has two checks, because the model declares no definition with the `invariant` qualifier (claim 7 of the README); a module that declared one would have three. Read in `cliHelpers.js` lines 91 to 96 and 117 to 125, not run | step 2, probe 5 |
| E | The spelling `scripts/verify-quint.sh` uses to name two properties, `--invariants A B` as QM03 proposes, gives a `> [3/3]` line that holds both names and a third check whose violation names each violated invariant. One `--invariant=A,B` gives the line and no name. Read in `cliHelpers.js` lines 101 to 124, `cliCommands.js` lines 493 to 503 and `verify.js` lines 187 to 196 and 220 to 224, not run; the recipe did not exist when this ticket was written | step 2, probe 4, in both spellings |
| F | At the recipe's verbosity, one inductive run of three checks prints `No running Apalache server found, launching...` and one `Started Apalache server on pid=` line during its first check, and `Connecting with existing Apalache server` in its second and third. Read in `verify.js` lines 172 to 175 and `apalache.js` lines 490 to 516, not run | step 2, probe 1, run once more at that verbosity, the log quoted whole |
| G | A step check that ends in a counterexample prints `error: found a counterexample`, and one that ends in a deadlock prints `error: reached a deadlock`; both print `[violation] Found an issue`. Read in `apalache.js` lines 141 to 146, `cliReporting.js` lines 243 to 254 and `cliCommands.js` lines 665 to 668, not run | step 2, probes 3 and 7 |

## Non-goals

- No inductive proof of any other invariant or guarantee. Each would be its own
  follow-up.
- No proof beyond `pachinko_walk`'s domains, and no parameterised proof.
- No change to a rule, a guard, an effect, an action or an existing property of the
  model, and no change to `docs/specs/pachinko.allium`.
- No lower bound, smaller domain or dropped instance anywhere in `quint/verify.txt`.
- No change to the `quint` CI job's `timeout-minutes` without the maintainer's ruling.
- No liveness, and no use of `--temporal`.
- No TLC backend.

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

`<id>` is `QM13`. Checkpoints:

- Checkpoint 1, the plan: step 3, before any tracked file changes.
- Checkpoint 2, the diff: step 10, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | the section "Inductive invariants": its comment, `StateInRange`, any conjunct the Allium implies and the model lacks, and `LeaseExclusivityInductive`. No other line |
| `quint/mutants/qm13-<slug>.patch` | new | the weakening: one conjunct removed from `LeaseExclusivityInductive` |
| `quint/mutants/` | shared, own lines | only the patches this change breaks, each re-cut with its mutation and its named property kept (shared rule 11) |
| `quint/verify.txt` | shared, own lines | the inductive line for `pachinko_walk` with its two implied properties, in the form ruled at step 4, and the comment that describes the second kind of line |
| `scripts/verify-quint.sh` | shared, own lines | the inductive run, the evidence it requires from the log, the server lines it requires, the weakened run and its fail-closed cases |
| `scripts/verify-quint-model.py` | shared, own lines | only if QM03 landed the helper and reads the lines of `quint/verify.txt` or a patch's header there: the second kind of line, the weakening's header and the check of `# removes:` against the parsed copy. Created by QM03 |
| `quint/traces/` | shared, own lines | only if the tree this ticket lands on holds `quint/traces/`, at the base or after the closing merge or rebase: the fixtures regenerated by `just quint-traces`, with the model's new hash, before any gate runs |
| `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines | on the same condition for the generator and the Rust replay, and only if that tree holds `double/tests/test_replay.py` for the double's; no edit is expected: shared rule 11 lists the replays for every ticket that changes the model, and this one adds no `var` and no step. An edit needed here means the new section changed a trace, which is step 7's **Stop** |
| `docs/decisions/0022-<slug>.md` | shared, own lines | created by QM02, which chooses the slug: one paragraph on what the inductive run proves, what it does not, and what a ticket that changes the model owes it |
| `docs/reference/quality-gates.md` | shared, own lines | only if the page lists what `just verify-quint` runs: the inductive run and what it proves |
| `docs/reference/commands.md` | shared, own lines | only if the `just verify-quint` row lists its runs |
| `.scratch/quint-model/QM13-inductive-exclusivity.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM13's Status cell |

Decision 0022 defines "verified" for the series, which is why step 9 adds to it. If the
record's number is not 0022, read "Before QM01 starts" in the series README; the
hand-back names the file as it is.

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm13-inductive-exclusivity`
   from the series base the maintainer names, once QM08 has merged there. Confirm the
   base holds QM01's `tools/quint/evaluator.txt`. Record the
   base's full commit id as `base`. Confirm `git ls-files .scratch/quint-model/` lists
   the series README and this ticket. Run `just initialize` (network, authorised). Run
   `just check` on the untouched worktree and quote its last line, headed by the commit.
   Then, with the port probe showing nothing on `localhost:8822`, time
   `just verify-quint` on the untouched worktree and quote its last lines, its exit
   status and the seconds it took: that is the baseline for claim 12. Record whether
   the base holds `quint/traces/`, whether it holds `double/tests/test_replay.py`, and
   whether it holds QM09 (a `now` in the model). Step 11 asks the three again of the
   tree after the closing merge or rebase.
2. **Read before writing, and probe.** No tracked file changes in this step. Read the
   list under Context. Write a scratch module, `ai_tmp/qm13/probe.qnt`: one counter that
   a step raises, a second `var` the step leaves alone, a start, and three definitions:
   one that is inductive and implies a bound, one that holds at the start and that the
   step breaks, and one that is inductive and implies nothing useful. Run each probe
   through `just quint verify`, each after the port probe, and quote the whole output
   and the exit status:
   1. the inductive definition with one ordinary invariant named, at `--verbosity=2`
      and again with no `--verbosity` (claims 16 and A): three progress lines, `[ok]`;
      and once more at the verbosity `scripts/verify-quint.sh` passes, for the server
      lines of each check (claim F);
   2. `--inductive-invariant` given an expression that is false at the start (claim C,
      the start);
   3. the definition the step breaks (claim C, the step);
   4. the definition that implies nothing useful, with two ordinary invariants, one of
      which it does not imply, named in the spelling `scripts/verify-quint.sh` uses
      for two properties, and again as one `--invariant` value with a comma (claims C
      and E, the implication): which run names the violated invariant;
   5. the inductive definition with no ordinary invariant named (claims 16 and D): the
      lines read `[1/2]` and `[2/2]`;
   6. one scratch `var` for each shape in claim B, in a second scratch module, each
      with its range clause, and once more with one clause removed;
   7. a third scratch module whose step cannot move from one state its invariant
      admits, run as probe 3 is (claim G): quote the lines that tell a deadlock in the
      step from probe 3's counterexample. If the checker reports no deadlock there at
      all, record that; the recipe's refusal of the deadlock line stays.

   **Stop** and bring it back to the README if claim 16's answer differs. **Stop** and
   ask if claim B fails for a shape the model has: the fallback in "Design proposed" is
   a different deliverable. Before asking, run checkpoint 1 (step 3) over the probe
   modules and their output in place of a plan, with one more question: is the shape
   refused by the checker, or is the probe at fault, through a clause written wrongly,
   an option missing or a module that does not say what it claims to? A ticket that
   closes on a faulty probe closes on a false finding, so the maintainer rules with
   those findings in hand. If the ruling is that the ticket closes as "not achievable
   as modelled", write the finding under Open points and in the hand-back: each shape
   probed, the probe's output, which `var` of the model needs the shape that failed,
   and what would make the proof possible. Steps 4 to 8 then do not run and no file
   under `quint/` or `scripts/` changes. Steps 9 to 11 run in the form each gives for
   this outcome. Otherwise list every `var`
   of `quint/pachinko.qnt` with its type, and write `ai_tmp/qm13/plan.md` with:
   - each `var` beside the clause `StateInRange` gives it, and the probe that showed the
     checker accepts that shape;
   - each conjunct of `LeaseExclusivityInductive` beside the Allium's text: the
     invariant it transcribes, or the text that implies it;
   - for each rule that creates or ends a lease or changes a pocket's `status`, the
     Allium's text beside one sentence on why the conjuncts survive it;
   - the weakening, the conjunct it removes, and the check that must fail;
   - the lines for `quint/verify.txt`, the weakening's header and the change to
     `scripts/verify-quint.sh` and, if it reads either, `scripts/verify-quint-model.py`,
     against what QM03 landed ("Design proposed", "How the run and the patch fit QM03's
     inventory"), with the exact evidence the recipe greps for, the server lines
     included, copied from the probes' output;
   - which patches under `quint/mutants/` the new section may break, and whether the
     fixtures are regenerated;
   - no "Not yet modelled" line is added or removed; say so, or say which and why.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qm13/plan.md` with the probes' output, before any tracked file changes;
   `tickets/CONVENTIONS.md` §14 with the deltas above). Questions:
   - Can the inductive run pass while proving neither invariant: two checks instead of
     three, a `> [3/3]` line that names something else, or a `StateInRange` so narrow
     that no state satisfies it, so that the step and the implication pass with nothing
     to check?
   - Is every conjunct something the Allium states or implies? Is any of them true of
     the model only because of how the model is written?
   - Does `StateInRange` bound the size of anything that grows, and would the step
     check have caught it?
   - Would the weakened run still fail if the recipe read only the exit status? Could
     it fail in the start or the implication, or on a deadlock, and be counted? Could a
     patch that removes another conjunct than its header names be counted?
   - Does QM03's recipe, as landed, refuse the inductive line or run the weakening as a
     bounded mutant, and does the plan change exactly the file that decides it?
   - What does a later ticket that adds a `var` see, and does the comment tell it what
     to do?
4. **Red: the range alone.** **Stop** first for the maintainer's ruling on "Design
   proposed", with the checkpoint's findings in hand, and record it. Then add the
   section and `StateInRange` to `quint/pachinko.qnt`, and run
   `just quint typecheck quint/pachinko.qnt`. Until step 7, `just check-quint` and
   `just verify-quint` are not run: on a tree that holds `quint/traces/`, the model's
   new hash fails both until the fixtures are regenerated. With the port probe first, run the
   inductive command with `--inductive-invariant=StateInRange` and the two ordinary
   invariants. Quote the output. It must show the start and the step passing and the
   implication failing:
   the last progress line is the `> [3/3]` one, `[violation]` follows it, and a
   cross-mark line names at least one of the two invariants. If no line names one, the
   spelling is not the one claim E describes: **stop** and settle it before going on,
   since the recipe's evidence rests on the name. The range alone is inductive and
   proves nothing. If the step fails
   instead, `StateInRange` bounds something a step moves past; fix the range, never
   the model. If the implication
   passes, the range is too narrow: it admits only states in which both invariants
   already hold, so it is doing a conjunct's work under the wrong name. Find out what
   it excludes before going on. The violation this run reports is itself a state in
   the range, which shows the range is not empty.
5. **Find the strengthening.** Add `LeaseExclusivityInductive` with the hypothesis from
   "Design proposed" and run the inductive command. For each failure of the step, save
   the output as `ai_tmp/qm13/step-<n>.log` and classify the first of its two states:
   - no run of the model can reach it, and an Allium invariant or a statement the Allium
     implies excludes it: add that conjunct, with its comment;
   - no run can reach it, and nothing the Allium states or implies excludes it:
     **Stop**; write the state and the missing statement under Open points and ask the
     maintainer (shared rule 2);
   - a run can reach it, and the second state breaks a conjunct other than the two
     exclusivity invariants: that conjunct is false of the model, so it was derived
     wrongly. Remove or correct it, against the Allium's text;
   - a run can reach it, and the second state breaks one of the two exclusivity
     invariants: the model breaks it beyond the bound. **Stop**; write the trace under
     Open points and ask. Do not weaken the invariant.

   A failure of the start means a conjunct is false in `init`: the conjunct is wrong,
   since every Allium invariant holds there. Never remove one of the two exclusivity
   invariants, add a guard to an action, or narrow a domain to make the run pass.
6. **Green, and its controls.** With the port probe before each run, and each output
   quoted with its exit status, headed by the commit and "with uncommitted changes":
   - the inductive command passes: `> [1/3]`, `> [2/3]`, `> [3/3]` naming both
     invariants, and `[ok] No violation found`;
   - cut `quint/mutants/qm13-<slug>.patch`. On a scratch copy under `ai_tmp/qm13/weak/`
     with the patch applied, `just quint typecheck` passes and the inductive command
     fails in the step, on a counterexample: the last progress line is `> [2/3]`,
     `[violation] Found an issue` and `error: found a counterexample` follow, and
     `error: reached a deadlock` is absent;
   - `--inductive-invariant='not(LeaseExclusivityInductive)'` on the unchanged model
     fails in the start: the last progress line is `> [1/3]`, and `[violation]`
     follows;
   - the inductive command with no ordinary invariant named, if claim D's answer makes
     that a run of two checks: quote the `[2/2]` line. This is the run the recipe must
     never accept;
   - each conjunct other than `StateInRange` and the two exclusivity invariants removed
     in turn on a scratch copy: record which check fails, and delete from the model any
     conjunct whose removal changes nothing, or keep it and say why;
   - each exclusivity mutant QM02 and QM03 committed under `quint/mutants/`, applied to
     a scratch copy: the inductive command fails in the step. A patch that no longer
     applies waits for step 7 and is run then. A mutant that breaks an
     exclusivity invariant and passes the inductive run means the run is not checking
     what it claims: **Stop** and ask.
7. **What depends on the model** (shared rule 11), before any gate runs. For each
   patch under `quint/mutants/`, this ticket's own included, show it still applies to
   the changed model; re-cut each that does not, keeping its mutation and the property
   it names, and list them. If the tree holds `quint/traces/`, at the base or, when
   step 11 comes back here, after the closing merge or rebase: regenerate the fixtures
   with `just quint-traces`, show the diff is
   the recorded hash and nothing else, and run `just test`, and `just python-double` if
   `double/tests/test_replay.py` exists. A fixture whose trace changed means the new
   section changed the model's behaviour, which it must not: **Stop** and find out why.
   Then run `just check-quint` and quote it.
8. **The recipe.** Add the inductive line, the weakening's header and the two runs to
   `quint/verify.txt`, `scripts/verify-quint.sh` and, where the plan says so,
   `scripts/verify-quint-model.py`, in the form ruled at step 4 and in QM03's style,
   with the evidence greps from the plan. Then show the recipe failing closed, each on
   an edit restored afterwards from a copy under `ai_tmp/qm13/`, with `cmp` exiting 0
   after the last restore, and each message quoted:
   - the two implied properties removed from the inductive line: the recipe fails on
     the line, before any run;
   - the weakening's header given `# property:` in place of `# removes:`: the recipe
     fails on the header, and does not run the patch as a bounded mutant;
   - the weakening replaced by one that removes `OneActiveLeasePerPocket`, its
     `# removes:` line saying so: the recipe fails on the header, before any run,
     because the conjunct is an implied property. Run by hand, the same patch is
     expected to fail in the step as the real one does, which is why the phase cannot
     be the check; quote that run, whatever it shows, beside the recipe's message;
   - the same patch with `# removes:` still naming the real weakening's conjunct: the
     recipe fails, because the parsed copy lacks another name than the header gives;
   - the model given, for this control only, a `val` that is `true` and that `val` as
     one more conjunct of the invariant, and the weakening replaced by one that removes
     exactly it, its header saying so. A conjunct that is `true` cannot matter, so the
     invariant is still inductive and the weakening is still inductive too. First show
     by hand that the inductive command passes on the edited model; then the recipe
     fails, because the weakened run passed. If the tree holds `quint/traces/`, the
     recipe reports the edited model's stale fixtures as well; the message to quote is
     the weakened run's. Restore the model;
   - the weakening given a second hunk that changes one action and nothing in the
     invariant, its header unchanged: the recipe fails before any run, because a
     definition other than the invariant differs from the model's;
   - the weakening patch given a hunk that does not apply: the recipe fails.

   Then, with the port probe first, time the whole of `just verify-quint` and quote its
   exit status and the seconds beside step 1's (claim 12). **Stop** and ask if the
   whole recipe takes more than half of the `quint` job's `timeout-minutes`, read from
   `.github/workflows/ci.yml`: a local run is not the CI runner, the CI figure needs a
   push, and a push is separately authorised. Do not shrink a domain, drop a run or
   raise the timeout to get under the bar.
9. **Words.** If the proof landed, add the paragraph to decision 0022: what an
   inductive invariant is, that
   `LeaseExclusivityInductive` implies the two exclusivity invariants, that the proof
   is for any number of steps at `pachinko_walk`'s domains and for nothing else, and
   what a ticket that changes the model owes the run. Edit the two reference pages if
   their rows need it. If the ticket closed as "not achievable as modelled", add one
   sentence to decision 0022 in place of all that: the inductive check was tried, on
   which shapes, and why it did not land. It says nothing was proved, and the
   reference pages do not change. The `review-docs` skill applies. Run
   `just check-docs`.
10. **Adversarial review** (checkpoint 2: conformance and breakage, over
    `git diff "$base"...HEAD` plus the uncommitted changes, with the output of steps 4
    to 8, before the final commit; `tickets/CONVENTIONS.md` §14 with the deltas above).
    Questions:
    - Does the recipe accept a run on any evidence weaker than three checks, both names
      and `[ok]`? Does it read an exit status where it should read a line?
    - Is every probe that passes by printing nothing preceded by a positive control
      from a log of the same recipe run?
    - Does the diff touch any line of the model outside the new section?
    - Does each conjunct's comment quote Allium text that says what the conjunct says?
    - Do the pages claim more than "any number of steps, at these domains, for these
      two invariants"?
    - What happens to this run when QM09 adds `now`, and is that written where QM09's
      agent will read it?

    If the ticket closed as "not achievable as modelled", steps 4 to 8 left no output
    and the questions above do not apply. The artefact is the diff and the probes'
    output, and the questions are two: does the sentence in decision 0022 claim
    anything the probes did not show, and does the hand-back's finding name each shape
    probed, the `var` that needs the one that failed, and what would make the proof
    possible?
11. **Commit and hand back.** Confirm every writer has stopped. Fetch, and merge or
    rebase onto the series base (shared rule 13). If the ticket closed as "not
    achievable as modelled", the merge owes the model nothing: this ticket leaves no
    definition under `quint/` to extend, steps 4 to 8 stay unrun whatever landed, and
    only the gates below run. Otherwise read each condition of this ticket
    again against the merged tree: if it now holds `quint/traces/` or
    `double/tests/test_replay.py` and the base did not, or the merge changed
    `quint/pachinko.qnt`, do step 7 again, before any gate; and if it now holds QM09's
    `now`, give `StateInRange` the clock's clauses and rerun step 6's green run. Show
    the merge's diff at the push stop (the review deltas, "After the last
    checkpoint"). Run `just check`, and,
    with the port probe first, `just verify-quint`. Commit the work. Rerun Verification
    and quote it, headed by the full commit id. Set `status: done` here and in
    `.scratch/quint-model/README.md`, write the hand-back notes, and commit the record.
    Then run `just check` once more, on the record's commit, and show its last line and
    an empty `git status --short`: the quoted Verification ran on the commit before,
    and the hook gate has not yet read the notes. If it fails, fix the notes, commit
    and run it again; the work's commit is not amended. **Stop** before pushing or
    opening a pull request: both need authorisation (`tickets/CONVENTIONS.md` §11,
    "Separately authorised actions").

## Acceptance criteria

The ticket has two acceptable outcomes, and the hand-back says which it reached.

*Not achievable as modelled.* The maintainer ruled so at the stop in step 2 or step 5.
Then these hold, and the list after them does not apply except for its last three
items. Checkpoint 1 ran before the ruling, over the probes if the ticket stopped in
step 2, and checkpoint 2 ran in the form step 10 gives for this outcome; both are
recorded:

- The hand-back records why: each claim probed with its output, the shape or the
  conjunct that failed, the `var` or the rule of the model that needs it, and what
  would make the proof possible (a bounded start, another checker, or a change to how
  the model holds ended leases, the shed queues or `prev`).
- The maintainer's ruling is quoted with its date.
- Decision 0022 says, in one sentence, that the inductive check was tried and why it
  did not land.
- No file under `quint/` or `scripts/` differs from `base`: what steps 4 and 5 added
  to the worktree, if they ran, is taken out again before the commit.

*The proof lands.* Then all of these hold:

- `just verify-quint` exits 0, and its output shows the inductive run on
  `pachinko_walk` with the `> [1/3]`, `> [2/3]` and `> [3/3]` lines, both invariant
  names in the third, and `[ok] No violation found`, quoted.
- The same recipe run shows the weakened invariant failing in the step, quoted: its
  last progress line is the `> [2/3]` one, `[violation] Found an issue` and
  `error: found a counterexample` follow, and no deadlock is reported. The patch's
  header names the conjunct it removes, and the recipe checked the two against each
  other.
- Both runs' logs show the server lines "Design proposed" requires: one server
  started, by this run, before its second check.
- Step 4's run is quoted: `StateInRange` alone passes the start and the step and fails
  the implication, with `[violation]` and the violated invariant named.
- Step 6's start control and each necessity run are quoted, and the hand-back lists
  every conjunct with the Allium text behind it and the check that fails without it.
- Each of step 8's seven edits makes the recipe fail, each message quoted, and the
  edited files are byte for byte as committed afterwards.
- `git diff "$base"...HEAD -- quint/pachinko.qnt` shows added lines in one section and
  no changed or removed line.
- Every patch under `quint/mutants/` applies to the committed model, and the hand-back
  lists each one re-cut. If the tree this ticket landed on holds `quint/traces/`, at
  the base or after the closing merge or rebase, the fixtures were
  regenerated, the diff is the hash alone, and `just test` passes.
- Claim 12 is recorded with both timings and the job's timeout, and the whole recipe
  is under the bar step 8 names, or the maintainer's ruling on it is recorded.
- Claims 16 and A to G each have their outcome recorded with the probe's output.
- Decision 0022 says what the run proves and what it does not.
- Both checkpoints are recorded as `tickets/CONVENTIONS.md` §14's Record asks.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.
- The record's commit passes `just check` too, run after it was made.

## Verification

Run from a file as one `sh` script, from the repository's root, after the commit, with
no other `verify` running in any worktree; `set -eu` stops it at the first command that
fails. Set `budget_seconds` to half of the `quint` job's `timeout-minutes`, in seconds,
before it runs. The two ordinary invariants are named as QM03's recipe names
properties: `--invariants` and the names, last on the line, with the model's path
first, because the option takes every word after it. They are the script's positional
parameters, set once by `set --`; a function call does not disturb them. `v` is the
verbosity the recipe passes, 4 unless the ruling at step 4 changed it. If step 2's
probe 4 showed the recipe needs another spelling, change the `set --` line and the
three commands and say so under Departures. The grep patterns are the lines read in
`cliReporting.js`, `cliCommands.js` and `apalache.js`; if the probes showed other
words, the patterns are corrected and Departures says so.

```sh
set -eu
git rev-parse HEAD
mkdir -p ai_tmp/qm13
set -- OneActiveLeasePerPocket OneActiveLeasePerConsumer
v=${v:-4}
test "$budget_seconds" -gt 0
py=.pixi/envs/default/bin/python
test -x "$py"
cross=$(printf '\342\235\214')

port_free() {
  if lsof -nP -iTCP:8822 -sTCP:LISTEN; then return 1; else test "$?" -eq 1; fi
}

# The server every check of a run used is the one that run started: one start, the
# launch and the start both before the second check's banner, and no line that joins
# a server before the start. The count is the positive control for the silent probe.
own_server() {
  test "$(grep -c -F 'Started Apalache server on pid=' "$1")" -eq 1
  sed '/> \[2\//q' "$1" >| ai_tmp/qm13/first-check.txt
  grep -F 'No running Apalache server found, launching...' ai_tmp/qm13/first-check.txt
  grep -F 'Started Apalache server on pid=' ai_tmp/qm13/first-check.txt
  if sed '/Started Apalache server on pid=/q' "$1" |
    grep -F 'Connecting with existing Apalache server'; then return 1; fi
}

# The port probe's positive control, then the probe.
"$py" -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
port_free

just check-quint

# Green: three checks, both names, ok.
FORCE_COLOR=0 just quint verify quint/pachinko.qnt --main=pachinko_walk \
  --verbosity="$v" --inductive-invariant=LeaseExclusivityInductive \
  --invariants "$@" >| ai_tmp/qm13/green.log 2>&1
grep -F '> [1/3] Checking whether the inductive invariant' ai_tmp/qm13/green.log
grep -F '> [2/3] Checking whether' ai_tmp/qm13/green.log
grep -F '> [3/3] Checking whether the inductive invariant' ai_tmp/qm13/green.log >| ai_tmp/qm13/third.txt
grep -F 'OneActiveLeasePerPocket' ai_tmp/qm13/third.txt
grep -F 'OneActiveLeasePerConsumer' ai_tmp/qm13/third.txt
grep -F '[ok] No violation found' ai_tmp/qm13/green.log
own_server ai_tmp/qm13/green.log
port_free

# Red: the range alone is inductive and implies neither invariant. The third check
# names what it violated, one cross-mark line for each.
if FORCE_COLOR=0 just quint verify quint/pachinko.qnt --main=pachinko_walk \
  --verbosity="$v" --inductive-invariant=StateInRange \
  --invariants "$@" >| ai_tmp/qm13/range.log 2>&1; then exit 1; fi
grep -F '> [' ai_tmp/qm13/range.log | tail -n 1 | grep -F '> [3/3]'
grep -F '[violation] Found an issue' ai_tmp/qm13/range.log
grep -F 'error: found a counterexample' ai_tmp/qm13/range.log
grep -E "^  $cross (OneActiveLeasePerPocket|OneActiveLeasePerConsumer)\$" ai_tmp/qm13/range.log
own_server ai_tmp/qm13/range.log
port_free

# Red: each weakening fails in the step, on a counterexample, on a scratch copy.
weak=ai_tmp/qm13/weak
n=0
for p in quint/mutants/qm13-*.patch; do
  test -f "$p"
  n=$((n + 1))
  grep -E '^# removes: [A-Za-z_]+$' "$p"
  rm -rf "${weak:?}"
  mkdir -p "$weak"
  cp quint/pachinko.qnt "$weak/pachinko.qnt"
  patch "$weak/pachinko.qnt" < "$p"
  if cmp -s quint/pachinko.qnt "$weak/pachinko.qnt"; then exit 1; fi
  just quint typecheck "$weak/pachinko.qnt"
  if FORCE_COLOR=0 just quint verify "$weak/pachinko.qnt" --main=pachinko_walk \
    --verbosity="$v" --inductive-invariant=LeaseExclusivityInductive \
    --invariants "$@" >| "ai_tmp/qm13/weak-$n.log" 2>&1; then exit 1; fi
  grep -F '> [' "ai_tmp/qm13/weak-$n.log" | tail -n 1 | grep -F '> [2/3]'
  grep -F '[violation] Found an issue' "ai_tmp/qm13/weak-$n.log"
  grep -F 'error: found a counterexample' "ai_tmp/qm13/weak-$n.log"
  if grep -F 'error: reached a deadlock' "ai_tmp/qm13/weak-$n.log"; then exit 1; else test "$?" -eq 1; fi
  own_server "ai_tmp/qm13/weak-$n.log"
  port_free
done
test "$n" -ge 1

# The green log holds no violation; the weakened logs above are the positive control.
if grep -F '[violation]' ai_tmp/qm13/green.log; then exit 1; else test "$?" -eq 1; fi

# The model's other dependants, the recipe and its time, and the gate.
git diff --exit-code quint/
start=$(date +%s)
just verify-quint
end=$(date +%s)
echo "verify-quint seconds: $((end - start))"
test "$((end - start))" -le "$budget_seconds"
port_free
just check
status=$(git status --short)
test -z "$status"
```

The `StateInRange` run and each weakened run are the red tests: each must exit non-zero
and show its `[violation]` line and the line that says a counterexample was found.
Which check failed is read from the last progress
line, which is positive evidence: `> [3/3]` for the range alone and `> [2/3]` for a
weakening. The range run must also name a violated invariant, on a cross-mark line
written as `printf` bytes so that the script stays ASCII, as QM03's recipe does. The
first inductive run and `just verify-quint` are the green tests. `own_server` reads
each log for the one server the run started. Each weakening must change the copy it
is applied to. Which conjunct a patch removes is the recipe's check, through the
parsed model; the script only requires the header line that names it. Four
probes pass by printing nothing. `port_free` follows the control listener that `lsof`
saw earlier in the script. The probe inside `own_server` follows the count of
`Started` lines in the same log. The deadlock probe follows the counterexample line
matched in the same log, the line before it. The `[violation]` probe in `green.log`
follows the same
pattern matching in `range.log` and in every weakened log. The count test after the
loop fails the script when no weakening patch exists, and `test -f` fails it when the
glob matched nothing. Step 8 holds the recipe's own fail-closed edits, and steps 4 and
6 the start control and the necessity runs, whose output is quoted there. If the ticket
closed as "not achievable as modelled", only the first two lines and the last three
lines of the script apply, and Departures says so.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Departures

### Verification

## Open points

- **Whether an inductive invariant of this model can be checked by the pinned tools at
  all.** The state holds collections that only grow, and `--inductive-invariant` uses
  one `val` both as the step's start and as what the step must preserve. Claim B is not
  read and not run. If it fails, the choices are a bounded start through three plain
  `verify` runs with a start action, which proves less, or closing this ticket as "not
  achievable as modelled, and why", which the Acceptance criteria allow. The maintainer
  settles it at the stop in step 2.
- **QM03's recipe has no place for an inductive run or a weakening.** Its
  `quint/verify.txt` has one kind of line and its patches one kind of header, and it
  forbids the line the second and third checks print when they join the first check's
  server. "Design proposed" recommends a second kind of line, a second kind of header
  and a server rule for this run. Each amends what QM03 ruled, so the maintainer rules
  at step 4, and decision 0022 records it for the next ticket that adds a kind of run.
- **A server that changes between two checks of one run.** The recipe's evidence for
  an inductive run is one launch and one start before the second check, and no joining
  line before the start. The second and third checks then print
  `Connecting with existing Apalache server`, which carries no process id, so the log
  cannot tell this run's server from a listener that took the port between two checks.
  QM03's own Open point says Quint's connection is not tied to the process it spawned.
  The maintainer settles at the ruling in step 4, with claim F's log in hand, whether
  that window is accepted, as it is between the port probe and a plain run's launch,
  or whether the run is split into plain runs, which the design bullet sets aside.
- **What repairing the run means is written only in the model's comment.** Shared rule
  11 has a model change run `just verify-quint`, so a later ticket sees this run fail.
  It does not say the invariant is extended and never dropped. The maintainer settles
  at the ruling in step 4 whether the comment is enough.
- **QM09 and this ticket.** They run in parallel and both change `quint/pachinko.qnt`.
  The one that merges second gives `StateInRange` the clock's clauses and reruns the
  inductive run. Timestamps and `now` are unbounded integers, and decay makes the shed
  queues shrink; neither is expected to touch lease exclusivity, and neither was
  checked. The maintainer names the order when the second of the two is ready to merge.
- **The CI figure for claim 12.** Step 8 measures the recipe locally. The `quint` job's
  time is known only after a push, which is separately authorised. The maintainer
  settles the bar at the ruling in step 4, and the timeout after the first CI run.
- **Statements the Allium implies and does not state.** If the strengthening needs
  "every lease names a pocket and a consumer that exist", or anything like it, the
  hand-back lists each with the text that implies it. Whether any becomes an Allium
  invariant is the maintainer's, after this ticket.
- **Which other properties deserve an inductive proof.** `ExclusivePocket` and
  `OutstandingOnlyWhileLeased` are the nearest. The maintainer decides after reading
  what this ticket cost.
