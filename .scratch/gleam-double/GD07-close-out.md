---
id: GD07
title: "Gleam double: close-out and the reader's guide"
status: todo
depends_on: [GD06]
parallel_with: []
branch: build-gleam-double
estimated_size: M
---

# GD07: Gleam double: close-out and the reader's guide

## Context

GD01 to GD06 landed the whole Gleam double under `gleam-double/`. Every rule of
`docs/specs/pachinko.allium` is a row of its trigger's table and has fired, every
invariant has a checker entry and a positive control, and every one of the 93 catalog
scenarios has a tagged test. The coverage check's pending lists are empty but still
present. `gleam-double/SPEC-NOTES.md` holds the notes each ticket appended, in the order
they were written. `.scratch/gleam-double/README.md` is the series' index.

The Gleam double exists for two reasons. It is a second reading of the specification,
made after the Python double's 38 notes and the maintainer's four rulings. And it is the
first implementation in this repository whose shape is the one the specification asks
for in words: one owner applying one serialized stream. This ticket turns the work into
what the maintainer reads:

- a guide that maps the specification's concepts to the double's modules;
- a walkthrough of decision 0020's Scenarios A to D, the catalog's S078 to S081;
- the spec notes consolidated into findings, each `G`-note set against its `N`-note;
- what the actor showed about the serialized-stream claim;
- one adversarial review of the whole double and its findings.

## Goal

The coverage check is strict, with no pending mechanism left. `gleam-double/README.md`
lets a reader find, for any rule, invariant or entity in the specification, the table,
checker entry or type that implements it, and walk Scenarios A to D through the owner.
`gleam-double/SPEC-NOTES.md` opens with a findings section the maintainer can act on,
which says where the second reading agreed with the first, where it departed, and what
only the actor could show. The whole double has passed the review below.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Non-goals

- No new behaviour. A finding that needs a behaviour change is fixed only if it brings the
  double back to the specification, and the hand-back names it.
- No answer to any `open question` and no edit under `docs/specs/`. A finding about the
  specification is written up for the maintainer, never applied.
- No edit to `double/SPEC-NOTES.md` or anything else under `double/`. The comparison
  lives in `gleam-double/SPEC-NOTES.md`.
- No use of the double as an oracle for the Rust or the Python, and no place in
  `just check`.
- No change under `crates/`, `docs/decisions/`, `loadgen/` or `fuzz/`.

## Shared rules (GD01 to GD07)

Every ticket in this series carries these rules word for word. They bind this ticket as
written here; nothing outside it amends them.

1. **Where the behaviour comes from.** The double is derived from
   `docs/specs/pachinko.allium`, read with the maintainer's rulings on the Python double.
   - `double/SPEC-NOTES.md` is input. Its four rulings bind this series, and a case one of
     them covers is ported as the ruling says, with no new stop:

     | Ruling | Rows | Note | First applies in |
     | --- | --- | --- | --- |
     | A publish answers nothing, as the specification states. A ported row asserts that answer and does not port the Rust's outcome. | Every publish row the Rust checks for an outcome | N32 | GD02 |
     | An ended lease stays in the state. The Rust's empty lease registry is its inspection shape. | S053 | N24 | GD04 |
     | S091's kept positions are compared by key. The Rust's record order is its inspection shape. | S091 | N31 | GD05 |
     | The three ports stand as the Rust's inspection shape. | S066's release half, S093 and S075's stored pocket order | N38 | GD06 |

   - Its notes, `N01` to `N38`, are the Python double's readings, not rulings. Where the
     specification is silent or ambiguous and an `N`-note records a reading, take that
     reading unless the specification contradicts it, and do not put the same question to
     the maintainer again. Where this double reads the specification differently, keep
     its own reading, say so in its spec note and in the hand-back, and go on.
   - `double/pachinko_double/` and `double/tests/` are never read: not for behaviour, not
     for design and not to port a scenario. `double/README.md` is read only in GD07, for
     the shape of a reader's guide.
   - `crates/libpachinko/src/` is never read for behaviour. The tests under
     `crates/libpachinko/tests/` are read only to port a scenario: its setup, its steps
     and what it asserts. S005's owning test is a doctest in
     `crates/libpachinko/src/config.rs`; that doctest is the one part of `src/` a ticket
     reads.
   - If a ported scenario fails against a double that follows the specification, and no
     ruling above covers it, record it in `gleam-double/SPEC-NOTES.md` and **stop**: put
     it to the maintainer. Never make it pass by copying the Rust or the Python.
   - A row that asserts more than the specification states is still ported, and its test
     and its spec note are labelled `impl-choice`. The known cases are the notes
     `double/SPEC-NOTES.md` lists under "The `impl-choice` rows": N04, N09, N10, N16,
     N18, N22, N23 and N29.
   - For a `wire` row, port only the library half. The HTTP half is out of scope.
   - Never silently resolve an `open question` (`AGENTS.md`). An `oq:N` row is ported as
     the rules stand, and its test and its spec note name the question.
2. **Scenario ownership.** Each S-id is owned by exactly one ticket and listed in that
   ticket's ownership table under Goal. Each test that pins one carries a
   `Scenario("S0NN")` tag in its entry in the test registry. A tag is data, not a
   comment, so it cannot sit on a test that never ran.
   - A scenario that turns out to need an operation this ticket does not land moves to the
     ticket that lands it. Update both tickets' ownership tables, the pending list and
     `.scratch/gleam-double/README.md`, and say so in the hand-back. Never drop one.
3. **The coverage check.** The coverage check, landed in GD02, runs from the test
   runner's `main` after the last test. It parses three lists and fails the run unless
   each entry is covered or pending. The run starts in `gleam-double/`, so it reads the
   two sources as `../docs/reference/testing.md` and `../docs/specs/pachinko.allium`.
   - The catalog's rows are the lines of `docs/reference/testing.md` that start with
     `| S` and three digits, followed by a space and a `|`. Each is covered by a
     `Scenario` tag on a registered test that ran and passed.
   - The specification's rules are the lines that start at column 0 with the word `rule`
     and a space, and end with a space and `{`. Each is covered when two things hold: the
     rule has a row in its trigger's table, and those tables are the registry; and a test
     that passed fired that row.
     The rule half fails on a registry name the specification does not hold, on a
     specification rule that has no row and is not pending, and on a row, not pending,
     that never fired.
   - The specification's invariants are the lines whose first word, after any leading
     spaces, is `invariant` and which end with a space and `{`. Each is covered by an
     entry in the checker's registry and by a positive control tagged
     `InvariantControl("<Name>")`. Fifteen of the thirty-five sit indented inside an
     entity, so a prefix test on the untrimmed line finds only twenty.
   - The three patterns are line prefixes and suffixes, written with the standard
     library's string functions. The double takes no regular-expression package.
   - It asserts the three counts it parsed, 93, 32 and 35, so a pattern that silently
     matches less fails. A specification change that moves a count updates the constant
     and adds a spec note. A tag that names an id or a name the sources do not hold fails.
   - The constants `pending_scenarios` and `pending_rules` hold what is not covered yet.
     An entry that is both pending and covered fails, where a rule counts as covered once
     it is registered, so each ticket shrinks the lists by exactly what it lands. GD07
     deletes both lists.
   - GD01 lands before the check exists. Its tests carry their tags from the start and
     assert the names of the rules they fired directly.
4. **Design constraints.** GD01 proposes the module layout and reviews it. These are
   fixed:
   - **A pure core.** Every rule, the apply step, the checker and the snapshot are pure
     functions over values. `owner` is the only module under `gleam-double/src/` that
     imports `gleam/erlang` or `gleam/otp`.
   - **One owner, one mailbox.** One `gleam_otp` actor holds the queue's state, the
     config, and the injected clock and lease-id functions. It takes one message per
     trigger a surface provides, plus `Inspect`, `LeaseView` (from GD04) and `Tick` (from
     GD06), and applies them one at a time through the core. No other process holds queue
     state: there is no process per pocket, per lease or per consumer, and no timer. The
     test runner's processes and the fakes' processes hold no queue state. This follows
     the specification's Lease expiry comment and decision 0019, which ask for one
     serialized stream and reject partitioning by pocket key.
   - **Failures are values.** A refusal, a guard overlap, a defect and a broken invariant
     are each an `Error` of one failure type, never a panic, because a panic would kill
     the owner. Code under `gleam-double/src/` uses no `panic`, `todo`, `assert` or
     `let assert`. On an `Error` the owner replies with it and keeps the state it had.
   - **The clock and lease ids are injected.** The owner is started with a clock function
     and a lease-id function. The fakes live under `gleam-double/test/`: a clock whose
     time a test sets and whose reads it counts, and a deterministic lease-id source that
     counts its draws. This follows the house rule that effects sit behind seams
     (decisions 0003 and 0008). The double ships no real clock, reads no wall clock and
     draws no randomness. A guard never reads the clock. The guards of the lapse, the
     three expiry rules and the two decays compare against the `now` their `Tick` read,
     which the trigger carries; no other guard sees a time. The clock is read at most
     once per outcome and never on a refusal (N28), and a lease id is drawn only when a
     lease is created (N15).
   - A refusal changes nothing, and a test proves it for each refusal by comparing the
     inspection seam's complete snapshot before and after, through the owner.
   - **Tables per trigger.** The rules that share a trigger are the rows of one table for
     that trigger, each row `(name, guard, effect)`.
     - Guards are pure: a guard allots nothing and changes nothing.
     - Dispatch evaluates every guard of the table. If two hold, it returns a guard
       overlap, `GuardOverlap`, naming both. That is a specification finding, since the
       specification claims disjoint guards for Publish, the lapse and `Freed`: record
       it in `gleam-double/SPEC-NOTES.md` and **stop**.
     - Each table states what happens when no guard holds:

       | Trigger | When no guard holds |
       | --- | --- |
       | Publish, Pull, Ack, Nack, Renew, Release and the two joins | A refusal |
       | `PocketTouched` | Nothing happens |
       | `LeaseLapsed` | Nothing happens, under branch A (decision 0019) |
       | `Freed` and `Handed` | A defect |
       | The lapse's clock trigger and the two decays | Nothing happens: `Tick` offers only what is due |

     - An input whose only matching rule is still pending is itself pending. No test pins
       it as a refusal.
   - **One apply step.** `core` has one apply step. It runs a trigger's table, then the
     events that outcome emits (`Handed`, `Freed`, `PocketTouched`, `LeaseLapsed`) through
     their own tables, then the checker, from GD02, once per outcome. It returns the
     names of the rules that fired. It is the only place that allots from
     `next_position`; a row's effect asks it for one.
   - **The checker.** The checker, from GD02, has one interface:
     `check(snapshot) -> violations`. The apply step calls it after the whole chained
     outcome, never part-way through, because the specification says the invariants hold
     after the whole of it, and returns a broken invariant naming each one that fails.
     Each of the 35 invariants has a positive control: a test that builds a snapshot by
     hand and shows that `check` names that invariant.
   - **The inspection seam.** Tests and the checker read state through one complete,
     immutable snapshot. GD01 lands it with the queue's `next_position`, the publishers
     and the consumers. GD02 completes it with the pockets with their status, position
     and consecutive expiries, the records in the specification's flat field names, the
     leases, and the superseded and dead-letter queues. Every collection in it is a list
     in a stated order, never a `Dict` in iteration order, so two snapshots compare with
     `==`. Derived sets, such as a pocket's held records, are computed once, in the
     snapshot.
   - **The test runner.** The double takes no gleeunit: EUnit fixes no order and halts
     the VM, so no check could run last. The test module's own `main` runs a typed
     registry of tests, `Test(name, tags, run)`, each in its own monitored process with a
     timeout, then the coverage check, and exits non-zero on any failure. A crash, a
     timeout and a test module that `main` does not list are failures.
   - **Dependencies.** `gleam_stdlib`, `gleam_erlang` and `gleam_otp`, with `simplifile`
     for the tests' file reads, written as ranges in `gleam.toml` and pinned by the
     committed `manifest.toml`. Decision 0021, from GD01, records each one and the
     `gleam` and `erlang` pins in `pyproject.toml`. No other package is added without
     amending that record. No Erlang FFI: if the runner or the fakes cannot be written
     without it, it goes in one named `*_ffi.erl` under `gleam-double/test/`, and the
     hand-back says why.
5. **Spec notes.** Every ticket appends to `gleam-double/SPEC-NOTES.md` wherever the
   specification was silent, ambiguous or forced an interpretation, and once for every
   `impl-choice` row. Each note gives an id (`G01`, `G02`, and so on, so that it cannot be
   mistaken for an `N`-note), the ticket, the rule, invariant or comment it concerns, by
   name, what the specification says, what the double does, the `N`-note it agrees with
   or departs from, or "no `N`-note", and the question it leaves for the maintainer. This
   file is the deliverable the maintainer sets beside `double/SPEC-NOTES.md` to judge the
   specification.
6. **Worktree hygiene.**
   - `just gleam-double` writes only under `gleam-double/build/`, which `.gitignore`
     ignores. It sets `ERL_CRASH_DUMP_BYTES=0`, so a crashed VM leaves no
     `erl_crash.dump`.
   - Gleam has no frozen mode: a `gleam.toml` that disagrees with `manifest.toml` makes
     `gleam build`, `gleam test` and `gleam deps download` rewrite `manifest.toml` and
     exit 0. Each recipe compares `manifest.toml` before and after and fails if it
     changed.
   - `just gleam-double` never reaches the network. `just gleam-double-deps` and
     `just lock` do, and run only where a step authorises them.
   - `just check` must be green with `gleam-double/build/` populated, since that
     directory holds the dependencies' own TOML, Markdown and sources.
   - The Gleam and Markdown files pass the existing hooks: editorconfig-checker, typos,
     markdownlint and lychee. `gleam format` owns Gleam layout. Add an `.editorconfig`
     entry only if editorconfig-checker objects, and say so in the hand-back.
   - Quoted evidence names the full commit id from `git rev-parse HEAD`. Until T37 lands,
     a seven-character prefix can fail typos (`_typos.toml`).
   - Green means three things: `just gleam-double` exits 0, `just check` ends with
     `All checks passed and the worktree is unchanged.`, and `git status --short` prints
     nothing after the commit (`tickets/CONVENTIONS.md` §11, "Green").
   - `just gleam-double` sits outside `just check`. Its owning page is
     `docs/reference/commands.md`, checked with `just check-docs`.
   - The series runs on one branch, `build-gleam-double`, in one worktree, two commits
     per ticket: the work, then the hand-back record. That is the maintainer's direction,
     as for PD01 to PD06, and it departs from `AGENTS.md`'s one ticket per worktree. Each
     hand-back says so.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14: its phases, its lenses, its two rounds
at most, its scope guard and its record. The deltas below are shared by GD01 to GD07.

- **Reviewer.** Codex, Astra, which is `codex exec -m gpt-6-astra`, is preferred. Fall
  back to a fresh-context `claude-opus-5-5` subagent only if `codex` is not on `PATH`,
  and say so in the hand-back notes. Confirm `codex exec --help` on the day, as §14 asks.
- **Command.** T37 is still todo, so §14's printed command misbehaves in two ways. It
  hangs, because it waits on stdin, and its `-o` overwrites the findings with Codex's
  last message. Use this form instead, run from the worktree:

  ```sh
  codex exec -m gpt-6-astra -s read-only -C "$PWD" \
    -o ai_tmp/review/<id>-<n>/last-<lens>.md "<brief naming findings-<lens>.md>" \
    < /dev/null >| ai_tmp/review/<id>-<n>/log-<lens>.txt 2>&1 &
  ```

  `<id>` is `GD07`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
  lens. `< /dev/null` closes stdin. `-o` holds only the last message. The log is what
  recovers a findings file that was never written. `>|` is for a shell with `noclobber`.
  A second round writes beside the first with a `-2` suffix.
- **Phase 1, review.**
  - The lead spawns one reviewer subagent per lens, in parallel. Each starts Codex
    detached with the command above, then polls in bounded foreground loops, each shorter
    than its tool's timeout, until the Codex process has exited
    (`pgrep -f last-<lens>.md` finds nothing) and `findings-<lens>.md` exists. It never
    idles waiting on a background job, because that job's end does not resume it. The
    lead also arms its own file watch on each findings file as a backstop.
  - The poll fails closed. If Codex has exited and `findings-<lens>.md` is missing or
    empty, the reviewer recovers the findings from `last-<lens>.md` or `log-<lens>.txt`,
    and the record says so. If neither holds findings, the lens runs again. An empty
    result is never recorded as "no findings".
  - The lenses are §14's conformance and breakage. The conformance brief names
    `docs/specs/pachinko.allium`, this ticket's path, its shared rules and its ownership
    table. At GD01's and GD02's design checkpoints and at GD07, a third lens, `design`,
    reviews the architecture. It asks whether each module has one job, whether a reader
    can find a rule's code from the rule's name, whether the seams are the ones the
    shared rules fix, and whether anything but the owner holds queue state.
  - The brief gives `git diff "$base"...HEAD` plus the uncommitted changes. Here `base`
    is this ticket's base commit, which is not `main`, because the series stacks on one
    branch. The brief never gives the author's reasoning or the hand-back's conclusions.
- **Phase 2, triage.** Separate fresh subagents, neither the reviewers nor the lead,
  check each finding by running a command or reading the specification. Each finding is
  marked valid, with a fix written, or invalid, with the reason, in
  `ai_tmp/review/<id>-<n>/verdicts.md`. The subagents pass file paths, not pasted text.
- **Phase 3, apply.** The triage subagents apply the valid fixes one writer at a time. A
  fix edits only this ticket's Files touched; any other valid fix is handed back. Before
  the lead reruns `just gleam-double` or `just check`, it confirms that every writer has
  stopped and is running nothing in the tree.
- **Rounds and record.** As §14: if any high-severity finding was valid, a second round
  reviews the fix diff, and there are two rounds at most. Anything still open goes under
  Open points. The checkpoint authorises the review's model call (§11, "Network").

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `gleam-double/test/coverage.gleam` | shared, own lines | The pending lists deleted; every entry must be covered |
| `gleam-double/README.md` | new | The reader's guide and the walkthrough |
| `gleam-double/test/` | shared, own lines | `guide_test.gleam`, which runs the walkthrough, and the check that the guide's examples match it |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | The findings section, and the notes regrouped under it |
| `gleam-double/src/` | shared, own lines | Only fixes the review finds valid |
| `docs/reference/commands.md` | shared, own lines | The `just gleam-double` row, if its wording no longer matches |
| `docs/project/repository-map.md` | shared, own lines | The `gleam-double/` row, naming the guide |
| `.scratch/gleam-double/GD07-close-out.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD07's Status cell |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD06, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **Strict coverage.** Delete `pending_scenarios` and `pending_rules` and the code that
   reads them. Quote two probes with their output and exit status, then revert each: a
   test's `Scenario` tag removed fails the coverage check, and a table row removed fails
   it. Strict coverage also means that every registered rule fired.
3. **The reader's guide.** Read `double/README.md` for the shape of a reader's guide, and
   for nothing else. Write `gleam-double/README.md` for a reader who has the
   specification open beside it and may not know Gleam. It holds:
   - how to run the double (`just gleam-double`, and `just gleam-double-deps` once);
   - a table from each entity, rule and invariant of the specification to what holds it
     and its file: for an entity its type, for a rule its trigger's table and module, and
     for an invariant its checker entry;
   - how the owner, its mailbox, the clock, the lease ids, `Tick` and the inspection seam
     stand in for what the specification leaves to time and to the implementation;
   - enough Gleam to read a rule's row: a custom type, a `case`, a `Result` and `use`;
   - a walkthrough of Scenarios A to D, each as a short session against the owner with
     the state after each step.

   Keep the walkthrough from drifting. Gleam has no doctest, so hold each scenario as a
   registered test in `guide_test.gleam`, and add a check that every fenced `gleam`
   block in `gleam-double/README.md` appears verbatim in that file and that each
   scenario's section has at least one. Show the state inside the block as an assertion,
   so that the page prints only what the test asserts. Quote a scratch break of one
   example failing the recipe, then revert it.
4. **The findings.** Use the `review-docs` skill for the prose. Open
   `gleam-double/SPEC-NOTES.md` with a findings section, and regroup every note under one
   of these headings, keeping its id:
   - where the second reading agrees with an `N`-note;
   - where it departs from one, with the specification text that decided it;
   - where it raises something no `N`-note holds;
   - the `impl-choice` rows.

   Then write what the actor showed about the serialized-stream claim, from the evidence
   GD03 to GD06 gathered:
   - which guarantees the competing-callers tests exercised, and what they cannot show;
   - whether any rule needed more than one message, or any state outside the owner;
   - whether the Lease expiry comment's "the owner may apply a lapse ahead of an
     already-queued action" describes anything a single mailbox does;
   - what GD06 chose for a failure part-way through a tick, against finding 9 of
     `double/SPEC-NOTES.md`;
   - whether one owner made any question in `double/SPEC-NOTES.md` easier or harder to
     answer.

   Each finding states the question the maintainer decides. List the specification's six
   open questions, and each `oq:N` scenario, as untouched. List the four rulings as
   followed, with the ticket that applied each.
5. **The handbook.** Update the `gleam-double/` row in `docs/project/repository-map.md`
   to name the guide, and the `just gleam-double` row in `docs/reference/commands.md` if
   its wording no longer matches. Run `just check-docs`.
6. **Adversarial review** (checkpoint 1: conformance, breakage and design, over the whole
   double and `gleam-double/SPEC-NOTES.md`, before the final commit). The brief gives the
   whole of `gleam-double/` as well as this ticket's diff. Questions for this
   checkpoint:
   - Is any rule applied in a way its guards do not allow?
   - Is any invariant checked more weakly than it is stated?
   - Was any scenario ported more weakly than its Rust test asserts, beyond what a ruling
     covers?
   - Did any note silently resolve an open question?
   - Does any finding overstate what the evidence shows, above all a finding about
     concurrency drawn from tests that fix no order?
   - Does any note claim to agree with an `N`-note that says something else?
   - Does the guide's table name a type, table or module that does not exist?
   - Does anything but the owner hold queue state?
7. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. The notes say what the maintainer should read first in the findings. **Stop**
   before pushing or opening a pull request: both need authorisation (§11, "Separately
   authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- The coverage check has no pending lists. It fails when any catalog id, rule or invariant
  is uncovered or any registered rule never fired, and step 2's two probes are quoted
  with their output and exit status.
- `gleam-double/README.md` maps every entity to its type, every rule to its table and
  module, and every invariant to its checker entry, all of which exist. Its walkthrough
  of Scenarios A to D runs as tests under `just gleam-double`, and step 3's break is
  quoted.
- `gleam-double/SPEC-NOTES.md` opens with the findings section. Every note sits under one
  of its headings and names its `N`-note or says it has none, the four rulings are listed
  as followed, and every open question is listed as untouched.
- The findings say what the actor showed about the serialized-stream claim, and each
  such finding names the test that is its evidence.
- `just gleam-double` leaves nothing Git can see, and no `erl_crash.dump`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The first
`rg` lines show that no pending list is left. The probe is the positive control: the
pattern matches each list's name. The second show that `owner` is still the only module
under `src/` that imports the process libraries.

```sh
set -eu
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated
probe=$(printf 'pub const pending_scenarios = []\npub const pending_rules = []\n' | rg -c 'pending_(scenarios|rules)')
test "$probe" -eq 2
if rg -n 'pending_(scenarios|rules)' gleam-double/src gleam-double/test; then exit 1; else test "$?" -eq 1; fi
sources=$(find gleam-double/src gleam-double/test -name '*.gleam' -print)
test -n "$sources"                      # positive control: the find sees the tree
dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
pattern='^import gleam/(erlang|otp)'
probe=$(printf 'import gleam/erlang/process\nimport gleam/otp/actor\n' | rg -c "$pattern")
test "$probe" -eq 2
importers=$(rg -l "$pattern" gleam-double/src)
test "$importers" = "gleam-double/src/pachinko_double/owner.gleam"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Written at the end.

## Open points

- PD07 is still todo. If the maintainer answers it before this ticket starts, the
  findings say how each answer bears on the Gleam double's ports of S053, S066, S075,
  S091 and S093.
- Whether the Gleam double should then be used as an oracle, for the Rust or against the
  Python double, is not decided here.
