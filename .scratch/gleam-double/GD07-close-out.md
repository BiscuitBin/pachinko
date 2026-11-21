---
id: GD07
title: "Gleam double: close-out and the reader's guide"
status: done
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

Evidence files are under `ai_tmp/gd/GD07/`, and the review record under
`ai_tmp/review/GD07-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise; `ev-notes-ids.txt` and
`ev-guide-table.txt` do neither. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name. A failing run prints the runner's
panic and the recipe's `error:` line between `coverage faults: N` and `exit status: 1`;
those are left out too, with no mark. A quote that does not open with its `$` line starts
part-way through the output. Times are UTC, read from the files' modification times.
Where a fact the record should hold was never saved, the text says "not recorded". One
read-only check was added for this record, `ev-handback-checks.txt` (parts 1 to 4, from
`handback-checks.sh`), run at `25008d690be03314fdec7875325addac01da314e` on a clean tree.

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `5bc619b08375872c685aca78cc95ddd4eee2b8a2`, GD06's record commit. It holds
  GD06's work, `98107a03b5374ee243f1caeade773ce16087e755`.
- Work commit: `bedb7af6b0873335ca0dfe51136bccfcad6508a7`, tree
  `5760747f02ef1eafcf888df0e4fffb743ab13646`, committed at 10:45:23Z.
- Review fix commit: `25008d690be03314fdec7875325addac01da314e`, tree
  `20ccb54e8c726868e12a50db77aa022e6cb9036c`, committed at 10:52:18Z. It applies two of
  the series code review's findings (section 7) and is the tree this record describes.
- The commit after them holds this record (`ev-handback-checks.txt`, part 1, for the
  trees and the files each commit touches).
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree. That departs from `AGENTS.md`'s one ticket per worktree (shared rule
  6). GD07 has three commits, not two: the work, the review fix and the record.
- Nothing was fetched. Step 1's `just initialize` and `just gleam-double-deps` were not
  run; the baseline ran green without them.

### 2. Baseline

At `5bc619b08375872c685aca78cc95ddd4eee2b8a2`, on the untouched worktree, written at
09:44:05Z (`baseline.log`, quoted whole; it records each status as `<name>-exit=N` and
holds no `$` line, and its last line counts the lines `git status --short` printed):

```text
5bc619b08375872c685aca78cc95ddd4eee2b8a2
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
check-exit=0
    Running pachinko_double_test.main
296 tests run, 296 passed
coverage faults: 0
gleam-double-exit=0
       0
```

### 3. The design as it stands at the series' end

The design is section 3 of GD06's hand-back, and through it GD05's to GD01's. GD07 adds
no behaviour. What changed since, in both commits (`ev-handback-checks.txt`, parts 1 and
4):

- **`src/`.** One change, in `core.gleam`: `core.tick`'s doc comment now says equal
  `expires_at` lapse "in their creation order in `state.leases` (G57)", not "in the order
  they were granted". It closes GD06's open point on that comment (GD06's C3). It is a doc
  comment, not code, and not a review fix, which is all Files touched allows under
  `src/`; the strict agent made it in step 2. No other file under `src/` changed in the
  series' last two commits.
- **`test/coverage.gleam`.** `pending_scenarios`, `pending_rules`, the `Inputs` fields
  that held them and the branches that read them are gone. Every catalog id needs a
  tagged test that passed; every rule must be a row of a registered table and fired by a
  test that passed; every invariant needs a checker entry and a control that passed; and
  the counts are still 93, 32 and 35.
- **`test/coverage_test.gleam`.** The six tests of the pending mechanism are replaced by
  four of the strict check: "a catalog id with no test is a fault", "a rule that is a row
  of no table is a fault", "a fired name credits no rule that is not a row" and "a
  registered rule that never fired is a fault" (`git diff` between the base and the work
  commit).
- **`test/guide_test.gleam`** (new), registered in `pachinko_double_test`'s `modules()`
  after `parking_test`. It holds the guide's four walkthrough tests, tagged S078 to S081 beside
  the ports in `lease_endings_test`; the drift check, "every gleam block of the guide is
  verbatim in its source, and each scenario's section shows its whole session"; and six
  controls of the drift check, four from step 3 and two from the review (B2, B3).
- **`test/runner.gleam`.** `runner.run` adds `runner.runner_faults()` to the registry's
  faults: two checks run outside the registry, through `run_one`, that a test that kills
  itself and one that never returns each come back `Failed` (B1).
- **`test/runner_test.gleam`.** Two controls, "a test that crashes fails" and "a test that
  does not return in time fails" (B1).
- **`test/owner_test.gleam`.** "a clock set back after a nack leaves the lease's end
  before nacked_at, so the lapse and a release fail NackedBeforeExpiry" (B4, G59).
- **`test/harness.gleam`**, in the review fix commit. `set_time` is a `process.call`: the
  clock's `Set` message carries a reply, so a trigger sent after `set_time` reads the new
  time. Before, `set_time` was a bare send, and the owner reads the clock from its own
  process, so nothing ordered the two.
- **`README.md`** (new) and **`SPEC-NOTES.md`**, regrouped: section 4.

The run counted 296 tests at the base; 294 after step 2; 303 after step 3, which the
findings run kept; 305, then 307 and 308 during the triage; and 308 at both commits.

### 4. Steps 2 to 5

Every output in this section ran uncommitted, on base
`5bc619b08375872c685aca78cc95ddd4eee2b8a2`.

Who did what, one writer at a time:

- The lead wrote three briefs, `brief-strict.md`, `brief-guide.md` and
  `brief-findings.md`. It ran the row-in-table probe itself, and updated the two
  `SPEC-NOTES.md` descriptions in `docs/project/repository-map.md` to name the findings
  (`ev-lead-docs.txt`: `just check-docs`, exit status 0).
- The strict agent, from `brief-strict.md`: steps 2 and 5 and the `core.tick` comment.
- The guide agent, from `brief-guide.md`: step 3.
- The findings agent, from `brief-findings.md`: step 4.
- The model of each subagent is not recorded.

#### Step 2: strict coverage

The two probes the strict agent ran. Neither file holds a `restored:` line, so each
restore rests on the agent's word and on the green runs after it.

`ev-probe-tag.txt`: S001's tag removed from `config_test`.

```text
$ sh -c git diff -- gleam-double/test/config_test.gleam; just gleam-double
...
-      [Scenario("S001")],
+      [],
...
    Running pachinko_double_test.main
COVERAGE S001 has no test tagged Scenario("S001") that passed
294 tests run, 294 passed
coverage faults: 1
exit status: 1
```

`ev-probe-row.txt`: the brief asked for one table row removed. The agent removed a whole
table, `DeadLetterDecays`, from `core.registry()`.

```text
$ sh -c git diff -- gleam-double/src/pachinko_double/core.gleam | sed -n '/registry/,$p'; just gleam-double
...
-    entry(decay.dead_letter_decays()),
...
COVERAGE DeadLetterDecays is a row of no table
294 tests run, 294 passed
coverage faults: 1
exit status: 1
```

**The lead's row-in-table probe, and why it was needed.** `DeadLetterDecays` is a table
with one row, so the probe above shows that a table left out of the registry faults; it
does not show that a row dropped from a table that stays registered faults. The lead ran
that probe, `ev-probe-table-row.txt` (its first lines are the lead's header and a `diff`
of `publish.gleam`, then the `$` line). `ReviveParkedPocket` is taken out of the
`PocketTouched` table, and kept referenced through `list.drop(.., 1)` so that the build,
with warnings as errors, stays clean:

```text
=== probe row-in-table: ReviveParkedPocket removed from the PocketTouched table
315c315
<     [revive_idle_pocket(), revive_parked_pocket()],
---
>     [revive_idle_pocket(), ..list.drop([revive_parked_pocket()], 1)],
$ just gleam-double
...
FAIL expiry_test.ReviveParkedPocket changes exactly what it ensures and reads no clock: crashed: ...
FAIL parking_test.three consecutive lapses park the pocket, and a publish revives it: crashed: ...
FAIL parking_test.a supersede revives a parked pocket: crashed: ...
FAIL parking_test.pockets and records form one strict, deterministic order: crashed: ...
COVERAGE S024 has no test tagged Scenario("S024") that passed
COVERAGE S062 has no test tagged Scenario("S062") that passed
COVERAGE S075 has no test tagged Scenario("S075") that passed
COVERAGE ReviveParkedPocket is a row of no table
294 tests run, 290 passed
coverage faults: 4
exit status: 1
restored: sha256 before 2fb1586e1d3e3b24f2f0877d493afa3b867036c319f540b93f1c7b5ac4d3d587 after 2fb1586e1d3e3b24f2f0877d493afa3b867036c319f540b93f1c7b5ac4d3d587
```

The strict agent's green run, `ev-strict-green.txt`: `294 tests run, 294 passed`,
`coverage faults: 0`, exit status 0. "Strict coverage also means that every registered
rule fired": no probe shows that half, or an invariant left uncovered. Both rest on
`coverage_test`'s scratch-input tests, "a registered rule that never fired is a fault",
"an invariant outside the registry is a fault" and "an invariant with no passed control
is a fault", which pass in every green run.

#### Step 3: the reader's guide

`gleam-double/README.md` holds "How to run it"; "The specification's parts and where
they live", with tables of the 8 entities, 32 rules and 35 invariants; "Time, identity
and inspection"; "Reading a row: the Gleam you need", with a custom type, a row, a
`Result` and a `case`, and `use`; "Walkthrough: Scenarios A to D", with the setting, the
common start and one section per scenario; and "Keeping this page true". The guide agent
wrote it from `README.template.md` with `build_readme.py`, both under `ai_tmp/gd/GD07/`;
the triage kept the template in step with every README edit.

**The table check.** `check_table.py` reads the README's tables and greps each name in
the code and the specification. `ev-guide-table.txt`, at the README the guide agent
wrote (SHA-256 `832076ae…`): `faults: 0`, exit status 0, then ten mutated copies, each
`exit status: 1`. Two of them:

```text
== control rule-row-dropped: replace [| `Renew` | `Renew` | `Renew` | `renew()` | `src/pachinko_double/settle.gleam` |
] with []
entities 8, rules 32, invariants 35 in the specification
FAULT rule Renew has 0 rows
faults: 1
exit status: 1
...
== control checker-entry-the-code-lacks: replace [`held_copies_agree`] with [`held_copies_match`]
entities 8, rules 32, invariants 35 in the specification
FAULT invariant HeldCopiesAgree: no Invariant("HeldCopiesAgree", held_copies_match) in src/pachinko_double/invariants.gleam
FAULT invariant HeldCopiesAgree: no fn held_copies_match in src/pachinko_double/invariants.gleam
faults: 2
exit status: 1
```

The triage then changed the README (D1, D2). This record reran the plain check on the
committed README (`ev-handback-checks.txt`, part 2, at
`25008d690be03314fdec7875325addac01da314e`): SHA-256 `6f102043…`, `entities 8, rules 32,
invariants 35 in the specification`, `faults: 0`, exit 0. Its ten controls were not
rerun. The script lives under `ai_tmp/`, so it is no part of the gate (Open points).

**The drift check.** "every gleam block of the guide is verbatim in its source, and each
scenario's section shows its whole session" reads the README and `guide_test.gleam`
through `simplifile`. Every fenced `gleam` block must appear verbatim in
`guide_test.gleam`, and each scenario's section must hold at least one. After the review
(B3) each scenario's blocks, joined by one blank line, must make up its test's body in
order, less the final `harness.fired(double)`. The state after each step is an
assertion inside the block, so the page prints only what its test asserts.

**The departure (C1).** Four blocks fenced `gleam src/<path>`, the excerpts that "Reading
a row" shows from `model.gleam`, `identity.gleam` and `publish.gleam`, are checked
against the source file they name, not against `guide_test.gleam`
(`ai_tmp/review/GD07-1/ev-C1-excerpts-absent.txt`). Step 3 says "every fenced `gleam`
block ... appears verbatim in that file". The lead keeps the departure: a source excerpt
checked against `src/` fails when `src/` changes, and a copy in the test file would be a
second copy that can drift.

**The break**, `ev-guide-break.txt`: in Scenario A's first block, the ack's fired rule
`AckMarksRecord` changed to `AckRepeats`.

```text
$ just gleam-double
...
FAIL guide_test.every gleam block of the guide is verbatim in its source, and each scenario's section has one: crashed: ...
303 tests run, 302 passed
coverage faults: 0
exit status: 1

HEAD 5bc619b08375872c685aca78cc95ddd4eee2b8a2
README sha256 before the break: 832076ae8f520498132ec23208a3d894ae184af9a7add7e8ca5f6dc4ab87270c
$ diff -u ai_tmp/gd/GD07/README.before-break.md gleam-double/README.md
...
-  assert harness.ack(double, "c1", "lease-1", "k") == fired(["AckMarksRecord"])
+  assert harness.ack(double, "c1", "lease-1", "k") == fired(["AckRepeats"])
...
diff exit status: 1
restored: cp ai_tmp/gd/GD07/README.before-break.md gleam-double/README.md; touch gleam-double/README.md
README sha256 after the revert: 832076ae8f520498132ec23208a3d894ae184af9a7add7e8ca5f6dc4ab87270c
sha256 matches: yes
```

The crash report in that run names the fault: `line 696: the gleam block is not verbatim
in test/guide_test.gleam`. The drift check was renamed during the triage, after the
review's last mutant runs, which still show the old name.

Lint: two defects were planted in the README to show the hooks see it. In
`ev-guide-lint-plant.txt` markdownlint reports `MD009/no-trailing-spaces` and
`MD018/no-missing-space-atx` at `gleam-double/README.md:1220` and `:1221`, and the run
exits 1. In `ev-guide-lint-plant-typos.txt` typos fails on line 1220, exit status 1: of the two
misspellings planted, it names one, and only in the pass over untracked files. Then
`ev-guide-lint.txt` (`just lint`) and `ev-guide-docs.txt` (`just check-docs`), each exit
status 0, and `ev-guide-green.txt`, `303 tests run, 303 passed`, `coverage faults: 0`,
exit status 0. The nine new tests are the four walkthroughs, the drift check and four
controls.

#### Step 4: the findings

`gleam-double/SPEC-NOTES.md` now opens with "Findings": the rulings followed, how the
notes are grouped, where the readings agree, where they depart, what no `N`-note holds,
what the actor showed about the serialized stream, and the open questions left
untouched. The header gained one sentence: agreement is weak evidence, since shared rule
1 had this double take an `N`-note's reading wherever the specification did not
contradict it.

`ev-notes-ids.txt`, quoted in part:

```text
before (git HEAD 5bc619b08375872c685aca78cc95ddd4eee2b8a2): 67 ids G01 to G67
after  (worktree gleam-double/SPEC-NOTES.md): 67 ids G01 to G67
same set of ids: True
...
notes whose text changed: none
...
Where it departs from an `N`-note: 2: G14 G35; in id order: True
...
```

The four groups hold 34, 2, 21 and 10 notes. A note that fits another heading is listed
there from the findings section; G09, G17 and G26 against N18 are the departures listed
that way. The check predates the triage's edits to G27 and G59. At the record, 67 note
headings and 67 `N`-note lines (`ev-handback-checks.txt`, part 3).

The serialized stream, seven findings:

1. Competing pulls, in the orders that occurred: `competing_test`'s three tests, with
   GD03's scratch owner that failed them (G32).
2. Competing joins, in the one order that occurred: `owner_test`'s "the same join from
   several processes succeeds exactly once" (G03).
3. An action against a lapse: `branch_a_test`'s competing renew and tick, and S066's and
   S067's halves in fixed orders (G63).
4. No rule needed more than one message, and no process but the owner holds queue state:
   a reading of `owner.gleam`, with `core_test`'s three witness tests (G47).
5. A lapse ahead of an already-queued action: a single mailbox never does it. A reading
   of `owner.gleam`; no test (G62).
6. A failure part-way through a tick, set against finding 9: `core_test`, `expiry_test`
   and the two `owner_test` tests (G59).
7. Whether one owner made a question easier or harder: findings 6, 9 and 10, drawn from
   the notes and items 5 and 6, with no test of its own.

The four rulings are listed with the ticket that applied each (N32 in GD02, N24 in GD04,
N31 in GD05, N38 in GD06). The six open questions are quoted, and S026 (`oq:7`) is the
one `oq:N` row. Lint, `ev-findings-lint.txt`, exit status 0; `ev-findings-green.txt`,
`303 tests run, 303 passed`, `coverage faults: 0`, exit status 0.

#### Step 5: the handbook

- `docs/project/repository-map.md`: the `gleam-double/` row names `README.md` as the
  reader's guide, and the row and the tree line both say `SPEC-NOTES.md` holds the
  findings first.
- `docs/reference/commands.md`: the `just gleam-double` row describes the strict check and
  no longer mentions pending lists.
- `ev-check-docs.txt` (the strict agent) and `ev-lead-docs.txt` (the lead): `just
  check-docs`, `Validated 49 pages and 50 canonical topics.`, exit status 0. No page was
  added to `docs/manifest.yml`.

### 5. Checkpoint record

One checkpoint, step 6: conformance, breakage and design, over the whole of
`gleam-double/` and this ticket's diff, before the final commit. The conformance lens filed C1 and C2, the breakage lens B1 to
B4, and the design lens D1 and D2.

- **`codex exec --help` on the day.** It was checked in GD04 and not rerun.
  `ai_tmp/gd/GD04/ev-codex-help.txt` was last written at 2026-11-21T06:09:24Z, and the
  three review logs at 10:26:31Z, 10:27:37Z and 10:29:43Z the same UTC day. It was the
  previous day in local time.
- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `sandbox: read-only`,
  reasoning effort `ultra`, one fresh process per lens, by the ticket's corrected
  command; each `log-<lens>.txt` opens with those lines. Codex was on `PATH`, so no
  fallback was needed.
- **Recovery.** Each findings file opens `Source: last-message`, and below that line it
  matches its `last-<lens>.md`: Codex's last message, since a read-only Codex writes no
  file.
- **What the reviewers ran.** Read-only commands. The breakage lens's B1 evidence is a
  mutant applied in memory to the compiled Erlang. Its "Verification" item says "The
  script was not run."
- **Brief.** `head.txt`, then each lens's questions (`lens-<lens>.txt`). It names the
  ticket, `tickets/CONVENTIONS.md`, the base, the whole of `gleam-double/` and the
  series' diff, and gives seven known points not to report again: G35, G27, G59, the
  equivalent mutants, GD06's rows written before their tests, the walkthrough tags beside
  the ports, and the table check under `ai_tmp/`. It gives no hand-back conclusion.
- **Reviewed tree.** `766b97f049e344609f85eec9888aa1afe00f953d`
  (`tree-before-review.txt`, written at 10:22:03Z). Before any fix `just gleam-double`
  ended `303 tests run, 303 passed`, `coverage faults: 0`, exit status 0
  (`ev-baseline.txt`).
- **Triage.** The verdicts are `verdicts.md`. Who triaged, how many subagents, their
  model and their independence from the reviewers and the authors are not recorded.
- **Rounds.** One. No finding was high, so no second round was owed.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid, handed back | The four `gleam src/<path>` excerpts are checked against their source, not `guide_test.gleam` | Kept by the lead as a departure (section 4) |
| C2 | low | valid | The four walkthrough tests depend on the literal `lease-1`; G27 named only two exceptions to N15 | G27 lists them and says "It differs in six tests" |
| B1 | medium | valid | A runner that took a crash or a timeout for a pass survived every test | `runner.runner_faults()`, outside the registry, and two `runner_test` controls |
| B2 | medium | valid | A blockquoted `gleam` fence and a longer closing fence escaped the drift check | Fences parsed after spaces and `>` marks, closed as CommonMark closes them; one new control |
| B3 | medium | valid | A repeated block passed | Each scenario's blocks must make up its test body, in order; one new control |
| B4 | medium | valid, an open point | A clock set back after a nack makes the lapse and a release fail `NackedBeforeExpiry`; the rows follow the rules' text | A new `owner_test` test pins it; G59 and finding 6 name it and ask the question |
| D1 | low | valid | Owner calls time out after 1,000 ms and crash the caller; the guide said they block and nothing panics | The guide's "Its mailbox" item and the dispatch paragraph corrected |
| D2 | low | valid | Rules read `model.State`, the checker a `Snapshot`; the guide's table gave both `Snapshot` | The table row split |

Applied by the triage to `runner.gleam`, `runner_test.gleam`, `guide_test.gleam`,
`owner_test.gleam`, `README.md` and `SPEC-NOTES.md` (G27, G59 and finding 6), all in
Files touched. Nothing under `src/` changed. B4's new test pins behaviour on an open
point; the lead kept it, as the `oq:7` precedent keeps S026's port. C1 was handed back
and kept. One limit is left: a fence inside a list item is not parsed (Open points).

Each mutant ran through `scratch/mutant.sh`, GD06's script with its keep path moved and
its filter widened to `FAULT` and `COVERAGE` lines. Each file ends with a `restored:`
line whose two SHA-256 values match, then `exit status: 0`, which is the script's own
status, not the recipe's. A run the mutant fails shows a `FAIL` or `FAULT` line and the
recipe's `error: recipe gleam-double failed ... exit code 1`.

| Mutant | Run | Result |
| --- | --- | --- |
| A crash taken for a pass (B1) | Before, `ev-B1-crash-passes-survives.txt` | `303 tests run, 303 passed`, no `error:` line |
| The same | After, `ev-B1-crash-passes-killed.txt` | `FAULT the runner took a test that crashed for a pass`, `305 tests run, 305 passed`, the recipe fails |
| A timeout taken for a pass (B1) | Before, `ev-B1-timeout-passes-survives.txt` | `303 tests run, 303 passed` |
| The same | After, `ev-B1-timeout-passes-killed.txt` | `FAIL runner_test.a test that does not return in time fails`, `305 tests run, 304 passed` |
| A blockquoted `gleam` block holding `assert False` (B2) | Before, `ev-B2-blockquote-survives.txt` | `305 tests run, 305 passed` |
| The same | After, `ev-B2-blockquote-killed.txt` | `307 tests run, 306 passed`; the drift check fails |
| An `sh` block closed by a longer fence, then a `gleam` block (B2) | Before, `ev-B2-long-close-survives.txt` | `305 tests run, 305 passed` |
| The same | After, `ev-B2-long-close-killed.txt` | `307 tests run, 306 passed`; the drift check fails |
| A repeated block in Scenario A (B3) | Before, `ev-B3-duplicate-survives.txt` | `305 tests run, 305 passed` |
| The same | After, `ev-B3-duplicate-killed.txt` | `307 tests run, 306 passed`; the drift check fails |

The crash mutant survived the two new `runner_test` controls as well, because the
mutated runner also took the control's own failed assertion for a pass; that run, `305
tests run, 305 passed`, was overwritten by the rerun and is not recorded. That is why
`runner_faults()` runs outside the registry. `ev-B1-controls-green.txt`, `305 tests run,
305 passed`. `ev-B2B3-firstrun.txt` failed four existing controls, `305 tests run, 301
passed`, exit status 1, before they were adapted; `ev-B2B3-green.txt`, `307 tests run,
307 passed`.

B4's evidence: `ev-B4-spec-time.txt`, a `grep` of the specification for `monoton`, `goes
back`, `backward`, `clock` and `Timestamp`, which finds field declarations and comments
and no requirement that `now` advance; and `ev-B4-backward-clock.txt`, `308 tests run,
308 passed`, exit status 0, with the new test.

The re-run after the checkpoint, `ev-final.txt`, uncommitted, on base
`5bc619b08375872c685aca78cc95ddd4eee2b8a2`, written at 10:44:48Z:

```text
$ just gleam-double
...
308 tests run, 308 passed
coverage faults: 0
exit status: 0
```

`ev-lint.txt` (`just lint`) and `ev-docs.txt` (`just check-docs`): exit status 0 each.
The tree after the triage is not recorded; by the lead's account the work commit's tree,
`5760747f02ef1eafcf888df0e4fffb743ab13646`, is that tree.

### 6. Verification at the work commit

The Verification block ran at `bedb7af6b0873335ca0dfe51136bccfcad6508a7` as one `sh`
script under `set -eu` (`verification.sh`, the ticket's block word for word; output in
`ev-verification.txt`, written at 10:45:53Z). It exited 0, so every `test` in it held:
no pending list is left under `src/` or `test/`, `owner` is still the only importer of
`gleam/erlang` and `gleam/otp` under `src/`, no `erl_crash.dump` was left, and `git
status --short` printed nothing. The two lines `273 tests run: 273 passed, 0 skipped`
inside `just check` are the Rust workspace's tests, not the double's.

```text
$ sh ai_tmp/gd/GD07/verification.sh
bedb7af6b0873335ca0dfe51136bccfcad6508a7
...
gleam 1.19.1
...
OTP 29
...
308 tests run, 308 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The offline run at the work commit, `ev-offline.txt` (its first line is the commit,
before the `$` line):

```text
bedb7af6b0873335ca0dfe51136bccfcad6508a7
$ sandbox-exec -p (version 1)(allow default)(deny network*) just gleam-double
...
308 tests run, 308 passed
coverage faults: 0
exit status: 0
```

Its negative control, `ev-offline-control.txt`, quoted whole, written at 10:46:01Z with
`ev-offline.txt`; it names no commit:

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) curl -sS --max-time 10 -o /dev/null https://hex.pm
curl: (6) Could not resolve host: hex.pm
exit status: 6
```

This `curl` was a second control, beyond the plan's list, which authorised one, in GD01.
It ran under the denial, failed to resolve the host and reached nothing. Nothing left the
machine.

### 7. The series code review, and Verification at the review fix commit

`code-review.md` holds the result and the lead's triage.

- **How it ran.** Once, after the work commit and before this record, over
  `main...build-gleam-double`: the whole series. The plan asked for the default effort.
  The lead passed no level, and the skill reused "high", the level typed last. **This
  departs from the plan.** The lead sent the series' read limits as a message after
  launch, and the skill reports that it kept to them. It ran no recipe.
- **The result.** No blocking bug and six nits. The skill's full output is in the lead's
  conversation, not in a file under `ai_tmp/`, so it is not recorded beyond
  `code-review.md`'s summary.

| Nit | Where | Triage |
| --- | --- | --- |
| 1 | `pull.gleam:173`: a repeated lease id fences the consumer out of its new lease | Open point: G27's question |
| 2 | `harness.gleam:72`: `set_time` was asynchronous | Fixed in `25008d690be03314fdec7875325addac01da314e`: `Set` carries a reply and `set_time` waits on it |
| 3 | `core.gleam:225`: `TickReport.lapsed` can name a still-active lease on a hand-built state | Open point: the owner cannot reach it, and a fix would change `src/` after the checkpoint |
| 4 | `owner.gleam:90`: a clock that goes back | Already B4 and G59 |
| 5 | `docs/decisions/README.md:67`: "The next decision is 0021." | Fixed in `25008d6`, then reverted in the commit after this record's first version, because this ticket's Non-goals exclude `docs/decisions/`. Left as an open point (31) |
| 6 | `.scratch/` holds committed tickets the project map does not list | Reported, not changed: the tickets arrived in `cca486f`, before this run, and `.scratch/python-double/` predates it |

The fix to nit 5 edited `docs/decisions/README.md`. This ticket's Non-goals say "No
change under ... `docs/decisions/`", and Files touched does not list the file; the series
README dropped that non-goal for GD01 only. So the lead reverted that hunk in a later
commit, and the sentence stays as GD01 left it, an open point (31). The tree at the end
of the series has no change under `docs/decisions/` from GD07. The fix to nit 2
edits `harness.gleam`, under `gleam-double/test/`, which Files touched lists for this
ticket's own lines. No review ran over the fix commit's diff.

`ev-cr-green.txt` (`308 tests run, 308 passed`, `coverage faults: 0`), `ev-cr-lint.txt`
and `ev-cr-docs.txt`, each exit status 0. `code-review.md` places them at the fix commit;
they name no commit and were written at 10:52:02Z to 10:52:10Z, before that commit at
10:52:18Z, so they ran on the fix uncommitted, on `bedb7af6b0873335ca0dfe51136bccfcad6508a7`.

Verification at the review fix commit, the same script (`ev-verification-after-cr.txt`,
written at 10:52:44Z):

```text
$ sh ai_tmp/gd/GD07/verification.sh
25008d690be03314fdec7875325addac01da314e
...
gleam 1.19.1
...
OTP 29
...
308 tests run, 308 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The offline run after it, `ev-offline-after-cr.txt`, under the same denial: `308 tests
run, 308 passed`, `coverage faults: 0`, exit status 0. Unlike `ev-offline.txt` it does
not name its commit; it was written at 10:52:45Z, one second after the Verification
above, and `code-review.md` places it at the fix commit. No `curl` control ran with it.

### 8. The acceptance criteria against the evidence

1. `just gleam-double` exits 0 and shows the counts: `ev-verification.txt` and
   `ev-verification-after-cr.txt`, `308 tests run, 308 passed`.
2. No pending list: the Verification's `rg` over `src/` and `test/`, with its positive
   control, in both runs. Step 2's two probes and the lead's third are quoted with their
   output and exit status (section 4). The scenario and row halves are shown by probes;
   the never-fired and invariant halves only by `coverage_test`'s scratch-input tests.
3. The guide maps every entity, rule and invariant to a name that exists:
   `ev-guide-table.txt`, with ten failing controls, on the page before the review, and
   `ev-handback-checks.txt`, part 2, on the committed page, without controls. The
   walkthrough runs as four tagged tests in every green run, and step 3's break is
   quoted.
4. `SPEC-NOTES.md` opens with the findings; every note sits under one heading
   (`ev-notes-ids.txt`); 67 notes and 67 `N`-note lines (part 3); the rulings and the
   open questions are listed (section 4). That each `N`-note line names a note or says it
   has none was not checked by a script; the conformance lens was asked and filed no
   finding on it.
5. The findings say what the actor showed. Findings 1, 2, 3 and 6 name their tests, and
   finding 4 names the witness tests beside its reading of `owner.gleam`. Findings 5 and
   7 name no test: finding 5 says no test can show it, and finding 7 rests on the other
   findings. **Not fully met** as the criterion is worded.
6. `just gleam-double` leaves nothing Git can see and no `erl_crash.dump`: both
   Verification runs test both under `set -eu` and exit 0.
7. The checkpoint is recorded in section 5, with the `codex exec --help` check made the
   same UTC day in GD04.
8. `just check` ends with its line and `git status --short` prints nothing: both
   Verification runs.

### 9. Stops

None fired. The conditions watched:

- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did; GD07 ported none.
- A `GuardOverlap` on a specification table (shared rule 4). None in any green run.
- A count that moved: every green run asserts 93, 32 and 35 and ends `coverage faults:
  0`.
- A finding that needs a behaviour change. B4 would, if the maintainer rules that `now`
  never goes back; it is an open point, and nothing under `src/` changed for it.
- A reach for the network. No network recipe ran. The review called a model API, which
  the checkpoint authorises. The code review was the plan's, and the ticket records no
  authorisation for it. The second `curl` control ran under the denial (section 6).

Step 7's stop stands: nothing was pushed and no pull request was opened.

### 10. What was not run, and what was not recorded

Not run:

- Any push, pull request or issue.
- `just initialize`, `just gleam-double-deps` and `just lock`.
- A CI run.
- A second review round. None was owed.
- A review of the fix commit's diff, and a mutation pass in GD07 beyond the triage's ten
  mutant runs.
- §11's two named negative controls, `curl` to `static.rust-lang.org` and to `1.1.1.1`
  with `--noproxy '*'`. The one control run was to `hex.pm`, at the work commit only.
- The table check's ten controls on the committed README.

Not recorded: the model of every subagent and how many triage subagents ran; the restores
after the strict agent's two probes; the first run of the B1 crash mutant against the
two new controls; the tree after the triage; the commit of `ev-offline-control.txt`
and `ev-offline-after-cr.txt`, beyond their times; and the code review's full output.

### 11. What the maintainer should read first

In `gleam-double/SPEC-NOTES.md`, "Findings":

- **The departures, G14 and G35** ("Where the second reading departs"). G14: is a
  snapshot whose record carries the held fields on its held variant acceptable, or
  should it show the flat record, with the five fields optional? G35, finding 5: is a
  refusal's kind, and what it carries, the specification's to name at all?
- **The serialized stream** ("What the actor showed about the serialized stream", items 1
  to 7). G32: should the specification state that actions are applied one at a time,
  each to the state the one before left? Item 4: does "one serialized stream" require
  one owner of the state, or only ordered applications? G62: is a lapse overtaking a
  queued action a freedom of the implementation, or something every implementation must
  be able to show?
- **G59 and B4, the clock going back** (item 6). Finding 9: is a tick atomic as a whole?
  Should one outcome that cannot be applied hold back the outcomes due after it, at this
  tick and every later one? And does the specification mean `now` never to go back? If
  not, a lease that ends before a `nacked_at` stamped under it cannot end at all while the
  clock stays there.
- **G27, lease identity.** Should the specification declare a lease's identity, and if
  it does, say that two leases never share one? The code review's nit 1 and the
  `PartialTick` test both turn on it.

## Open points

Every point still standing from GD01's to GD07's hand-backs, merged where two tickets
carried the same one, each naming the ticket where it began. Points a later ticket closed
are left out: GD02's checker witness (GD04, G47), GD02's derived sets, the
`EqualRankRefused` input (GD05), GD03's lease view and `LeaseFenced` (GD04),
`PocketTouched` for a parked pocket and the lapse halves (GD06), GD04's release of a
deferred record end to end (GD06's section 4, the by-release halves of S078 to S081),
`core.tick`'s comment (GD07), and GD01's pins, offline claim, `AGENTS.md` line and pixi
feature.

1. **PD07 is still todo** (GD02; carried by GD04, GD05, GD06 and this ticket).
   `.scratch/python-double/PD07-inspection-contract.md` asks whether the inspection shape
   is part of the contract. The double follows the rulings as they stand (G37, G49, G67).
   PD07 was not answered before this ticket started, so the findings say nothing of how
   an answer bears on the ports of S053, S066, S075, S091 and S093; each answer may
   change them.
2. **Whether the Gleam double should then be used as an oracle**, for the Rust or against
   the Python double, is not decided here (GD07).
3. **`pixi.lock` was written by pixi 0.84.1, and CI pins v0.83.0** (GD01). Whether 0.83.0
   accepts the file was not run; only a push can settle it.
4. **Shared rule 6 says two things about a rewritten `manifest.toml`** (GD01, C1). The
   recipe fails after the rewrite and leaves the file rewritten. The maintainer accepts
   that or asks for the guarded lines that restore it. Whether `gleam` rewrites a drifted
   manifest and exits 0 is knarr's finding, not shown here (GD01).
5. **Shared rule 4 says `core` is the only place that allots from `next_position`**
   (GD01). The interpreter in `rules.gleam` allots through two accessors `core` hands it.
   The maintainer accepts the reading or rewords the rule in the seven tickets.
6. **Whether Renovate reads the Gleam pins** in the `gleam` feature and `gleam.toml`, and
   whether anything audits a hex package (GD01, C6; the series README). Decision 0021
   says the pins move by hand until it is settled.
7. **An injected clock or lease-id function that crashes kills the owner** (GD01, B1). It
   is a contract on `owner.start`; the double ships no real clock.
8. **What the runner's checks still miss** (GD01, round 2). A `main` that imports a test
   module and drops its `tests()`, a string in `main` that holds an `import` line, and a
   test whose linked helper crashes after the test returns all pass. GD07's
   `runner_faults()` now catches a runner that takes a crash or a timeout for a pass (B1).
9. **Two network reaches in GD01 were not treated as stops** (GD01, section 7 of its
   hand-back). The maintainer judges whether the run should have paused.
10. **The snapshot's record holds the held fields on its held variant (G14)** (GD02, C2).
    It departs from N08 and from shared rule 4's "flat field names"; the maintainer
    accepts it or asks for the flat form or five read functions.
11. **`MarkConflict` carries no mark (G35)** (GD04). It departs from N21.
12. **Two invariants are taken as written (G17, G18)** (GD02).
    `OneActiveLeasePerConsumer` ranges over consumers, and the position invariants compare
    within a kind.
13. **No test shows that the checker runs only after every `Handed` is answered** (GD03).
    GD04's witness tests (G47) cover the publish and release chains; no later hand-back
    records one for a pull, where no state between the claim and its `HoldRecord`s breaks
    an invariant.
14. **No owner-driven test would see a pull or a renew empty the dead-letter queue**
    (GD03, B1). Only hand-built states hold a dead letter in those tests; no later
    hand-back records the seeding GD03 proposed.
15. **The competing-callers tests sample the scheduler's orders** (GD03; GD06). They
    cannot show a property holds in every interleaving. GD06's renew-and-tick test cannot
    show both orders occurred, and the scratch run that counted ten of each was not saved.
16. **A tick keeps what succeeded before a failure, and departs from shared rule 4 for a
    tick as a whole (G59)** (GD06). Whether a tick should be atomic is finding 9 of
    `double/SPEC-NOTES.md`. A failure that stays in the state stops every later tick at
    the same outcome.
17. **A clock set back after a nack leaves a lease unable to end (B4, G59)** (GD07, and
    the code review's nit 4). The lapse and a release fail `NackedBeforeExpiry`, and the
    owner keeps its state. The specification states no monotonic clock; the maintainer
    decides whether it means one.
18. **Two leases can share an id (G27)** (GD03; carried by GD04, GD05 and GD06; the code
    review's nit 1, and C2). The double does not check that an id it is handed is new,
    and a repeated id fences a consumer out of its new lease. The `PartialTick` test
    depends on that, and six owner-driven tests depend on literal ids.
19. **The questions the notes leave for the maintainer** (GD04, GD05 and GD06), now
    gathered in the findings section of `gleam-double/SPEC-NOTES.md`: positions and
    numeric domains (G04, G16, G29, G33, G48, G18, G45, G52); refusals and their order
    (G34, G36); orders the specification does not state (G06, G19, G38, G49, G57, G64);
    restated guards and comments (G39, G40, G41, G42, G43, G46, G47, G61); time (G05,
    G56, G58, G60, G62, G63); `shed_at` (G50); what a publish answers (G54); open
    question 7 (G65); and S075's Rust comment (G67).
20. **Equivalent mutants survive** (GD04: D13 and L3e; GD06: T6, T15 and T16). Each rests
    on an argument, not a test.
21. **Four tests were never red, and no mutant fails them** (GD04: three; GD05: one).
    Each pins a defect over a state built by hand.
22. **`arrival_premises` has failed only in its setup** (GD05), and **one version ending
    in both shed queues is caught by one test alone, DN12's** (GD05).
23. **The mutation passes predate their checkpoints' fixes** (GD05, GD06). `src/` was
    unchanged by those fixes, so no kill is lost, but no mutant ran against the new
    assertions beyond the triages' own. GD07 ran no mutation pass beyond the triage's
    ten runs.
24. **Two reviewers ran nothing in GD05** (GD05). Their findings rest on static
    inspection.
25. **`core.tick`'s report on states the owner cannot reach** (GD06; the code review's
    nit 3). `core.report`'s missing-lease branch has no test and no mutant, and
    `TickReport.lapsed` can name a still-active lease on a hand-built state. A fix would
    change `src/` after the checkpoint.
26. **GD06's rows were written before their tests** (GD06). The red runs and mutants show
    each test can fail; no evidence shows a test written first.
27. **The guide's source excerpts are checked against `src/`, not `guide_test.gleam`
    (C1)** (GD07). A departure from step 3's wording that the lead kept; the maintainer
    accepts it or rewords the step.
28. **The drift check does not parse a fence inside a list item** (GD07, B2). A `gleam`
    block there in the setting or the common start would go unchecked.
29. **The guide's table check is a script under `ai_tmp/`** (GD07). It is no part of
    `just gleam-double` or the gate, so a later rename in `src/` can leave the table
    wrong unnoticed.
30. **`.scratch/` holds committed tickets that the project map does not list** (GD07, the
    code review's nit 6). Reported, not changed.
31. **`docs/decisions/README.md:67` still says "The next decision is 0021."**, though
    decision 0021 exists (GD01; the code review's nit 5). GD07 may not change
    `docs/decisions/`, so its fix was reverted. The one-line fix: "Decision 0021 records
    the Gleam double's toolchain (GD01). The next decision is 0022." Also, the work
    commit's one change under `src/` is a doc comment, not a review fix (GD07, sections 3
    and 7).
