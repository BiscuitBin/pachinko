---
id: PD06
title: "Python double: close-out and the specification reader's guide"
status: done
depends_on: [PD05]
parallel_with: []
branch: ticket/pd06-close-out
estimated_size: M
---

# PD06: Python double: close-out and the specification reader's guide

## Context

PD01 to PD05 landed the whole Python double under `double/`. Every rule of
`docs/specs/pachinko.allium` is a row of its trigger's table and has fired, every invariant
has a checker entry and a positive control, and every one of the 93 catalog scenarios has
a tagged test. The coverage
test's pending lists are empty but still present. `double/SPEC-NOTES.md` holds the notes
each ticket appended, in the order they were written. `.scratch/python-double/README.md`
is the series' index.

The double exists so that the maintainer, who reads Python rather than Rust, can judge
whether the specification is right. This ticket turns the work into what they read:

- a guide that maps the specification's concepts to the double's classes;
- a walkthrough of decision 0020's Scenarios A to D, the catalog's S078 to S081;
- the spec notes consolidated into findings, with every open question left open;
- one adversarial review of the whole double and its findings.

## Goal

The coverage test is strict, with no pending mechanism left. `double/README.md` lets a
Python reader find, for any rule, invariant or entity in the specification, the table,
checker entry or class that implements it, and walk Scenarios A to D through the double's
API.
`double/SPEC-NOTES.md` opens with a findings section the maintainer can act on. The whole
double has passed the review below.

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
- No use of the double as an oracle for the Rust, and no place in `just check`.
- No change under `crates/`, `docs/decisions/`, `loadgen/` or `fuzz/`.

## Shared rules (PD01 to PD06)

Every ticket in this series carries these rules word for word. They bind this ticket as
written here; nothing outside it amends them.

1. **Where the behaviour comes from.** The double is derived from
   `docs/specs/pachinko.allium` alone.
   - `crates/libpachinko/src/` is never read for behaviour. The tests under
     `crates/libpachinko/tests/` are read only to port a scenario: its setup, its steps
     and what it asserts. S005's owning test is a doctest in
     `crates/libpachinko/src/config.rs`; that doctest is the one part of `src/` a ticket
     reads.
   - If a ported scenario fails against a double that follows the specification, record
     it in `double/SPEC-NOTES.md` and **stop**: put it to the maintainer. Never make it
     pass by copying the Rust.
   - A row that asserts more than the specification states is still ported, and its test
     and its spec note are labelled `impl-choice`. Known cases: S005, whose `compile_fail`
     doctest becomes a runtime refusal; S091's pinned order of the entries one lease end
     creates; and the refusals for an unknown publisher or consumer.
   - For a `wire` row, port only the library half. The HTTP half is out of scope.
   - Never silently resolve an `open question` (`AGENTS.md`). An `oq:N` row is ported as
     the rules stand, and its test and its spec note name the question.
2. **Scenario ownership.** Each S-id is owned by exactly one ticket and listed in that
   ticket's ownership table under Goal. Each test that pins one carries a
   `# scenario: S0NN` comment on the line above its `def`.
   - A scenario that turns out to need an operation this ticket does not land moves to the
     ticket that lands it. Update both tickets' ownership tables, the pending list and
     `.scratch/python-double/README.md`, and say so in the hand-back. Never drop one.
3. **Coverage tests.** `double/tests/test_coverage.py`, landed in PD01, parses three lists
   and fails unless each entry is covered or pending.
   - The catalog's rows, `^\| (S[0-9]{3}) \|` in `docs/reference/testing.md`, are each
     covered by a `# scenario:` tag under `double/tests/`.
   - The specification's rules, `^rule (\w+) \{`, are each covered when two things hold:
     the rule has a row in its trigger's table, and those tables are the registry; and
     the dispatcher recorded that row as fired during the run. The rule half fails on a
     registry name the specification does not hold, on a specification rule that has no
     row and is not pending, and on a row, not pending, that never fired. The fired-rule
     check reads the record only after every other test module has run, since `discover`
     would otherwise run `test_coverage.py` first; PD01 settles how.
   - The specification's invariants, `^\s*invariant (\w+) \{`, are each covered by an
     entry in the checker's registry and by a positive control tagged
     `# invariant-control: <Name>`. Fifteen of the thirty-five sit indented inside an
     entity, so a pattern anchored at column 0 finds only twenty.
   - It asserts the three counts it parsed, 93, 32 and 35, so a pattern that silently
     matches less fails. A specification change that moves a count updates the constant
     and adds a spec note. A tag that names an id or a name the sources do not hold fails.
   - `PENDING_SCENARIOS` and `PENDING_RULES` hold what is not covered yet. An entry that is
     both pending and covered fails, where a rule counts as covered once it is registered,
     so each ticket shrinks the lists by exactly what it lands. PD06 deletes both lists.
4. **Design constraints.** PD01 proposes the class layout and reviews it. These are fixed:
   - The clock and lease ids are injected: a `Clock` protocol with a fake whose time a test
     sets and whose reads it counts, and a lease-id source with a deterministic fake. This
     follows the house rule that effects sit behind seams (decisions 0003 and 0008). The
     double reads no wall clock and draws no randomness.
   - A refusal changes nothing, and a test proves it for each refusal by comparing the
     inspection seam's complete snapshot before and after.
   - **Tables per trigger.** The rules that share a trigger are the rows of one table for
     that trigger, each row `(name, guard, effect)`.
     - Guards are pure: a guard allots nothing and changes nothing.
     - Dispatch evaluates every guard of the table. If two hold, it raises
       `GuardOverlap` naming both. That is a specification finding, since the
       specification claims disjoint guards for Publish, the lapse and `Freed`: record
       it in `double/SPEC-NOTES.md` and **stop**.
     - Each table states what happens when no guard holds:

       | Trigger | When no guard holds |
       | --- | --- |
       | Publish, Pull, Ack, Nack, Renew, Release and the two joins | A refusal |
       | `PocketTouched` | Nothing happens |
       | `LeaseLapsed` | Nothing happens, under branch A (decision 0019) |
       | `Freed` and `Handed` | A defect |
       | The lapse's clock trigger and the two decays | Nothing happens: `tick()` offers only what is due |

     - An input whose only matching rule is still pending is itself pending. No test pins
       it as a refusal.
   - **One apply step.** `Queue` has one internal apply step. It runs a trigger's table,
     then the events that outcome emits (`Handed`, `Freed`, `PocketTouched`,
     `LeaseLapsed`) through their own tables, then the checker, once per outcome. It is
     the only place that allots from `next_position`, so `Queue` is the only module that
     allots positions; a row's effect asks it for one.
   - **The checker.** Its whole interface is `check(snapshot) -> violations`. The apply
     step calls it after the whole chained outcome, never part-way through, because the
     specification says the invariants hold after the whole of it, and raises naming each
     invariant that fails. Each of the 35 invariants has a positive control: a test that
     builds a snapshot by hand and shows that `check` names that invariant.
   - **The inspection seam.** Tests and the checker read state through one complete,
     immutable snapshot: the queue's `next_position`, the publishers, the consumers, the
     pockets with their status, position and consecutive expiries, the records in the
     specification's flat field names, the leases, and the superseded and dead-letter
     queues. Derived sets, such as a pocket's held records, are computed once, in the
     snapshot.
   - Standard library only: `unittest`, `dataclasses`, `enum`, `typing` and the like. No
     third-party package, so no decision-0007 note is needed. Python 3.14 from the pixi
     environment.
5. **Spec notes.** Every ticket appends to `double/SPEC-NOTES.md` wherever the
   specification was silent, ambiguous or forced an interpretation, and once for every
   `impl-choice` row. Each note gives an id (`N01`, `N02`, and so on), the ticket, the
   rule, invariant or comment it concerns, by name, what the specification says, what the
   double does, and the question it leaves for the maintainer. This file is the
   deliverable the maintainer uses to judge the specification.
6. **Worktree hygiene.**
   - `just python-double` runs `python -B`, so it writes no `__pycache__/`. `.gitignore`
     does not ignore one, and an untracked one fails `just check`.
   - The Python and Markdown files pass the existing hooks: editorconfig-checker, typos,
     markdownlint and lychee. `.editorconfig` defaults to `indent_size = 2`;
     `scripts/diagrams.py` is the four-space precedent, and it passes. Add a `[*.py]`
     entry only if editorconfig-checker objects, and say so in the hand-back.
   - Quoted evidence names the full commit id from `git rev-parse HEAD`. Until T37 lands,
     a seven-character prefix can fail typos (`_typos.toml`).
   - Green means three things: `just python-double` exits 0, `just check` ends with
     `All checks passed and the worktree is unchanged.`, and `git status --short` prints
     nothing after the commit (`tickets/CONVENTIONS.md` §11, "Green").
   - `just python-double` sits outside `just check`. Its owning page is
     `docs/reference/commands.md`, checked with `just check-docs`.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14: its phases, its lenses, its two rounds
at most, its scope guard and its record. The deltas below are shared by PD01 to PD06.

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

  `<id>` is `PD06`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
    table. At PD01's design checkpoint and at PD06, a third lens, `design`, reviews the
    object-oriented architecture. It asks whether each class has one job, whether a reader
    can find a rule's code from the rule's name, and whether the seams are the ones the
    shared rules fix.
  - The brief gives `git diff "$base"...HEAD` plus the uncommitted changes. Here `base`
    is this ticket's base branch, which is not `main` when the branches stack. The brief
    never gives the author's reasoning or the hand-back's conclusions.
- **Phase 2, triage.** Separate fresh subagents, neither the reviewers nor the lead,
  check each finding by running a command or reading the specification. Each finding is
  marked valid, with a fix written, or invalid, with the reason, in
  `ai_tmp/review/<id>-<n>/verdicts.md`. The subagents pass file paths, not pasted text.
- **Phase 3, apply.** The triage subagents apply the valid fixes one writer at a time. A
  fix edits only this ticket's Files touched; any other valid fix is handed back. Before
  the lead reruns `just python-double` or `just check`, it confirms that every writer has
  stopped and is running nothing in the tree.
- **Rounds and record.** As §14: if any high-severity finding was valid, a second round
  reviews the fix diff, and there are two rounds at most. Anything still open goes under
  Open points. The checkpoint authorises the review's model call (§11, "Network").

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/tests/test_coverage.py` | shared, own lines | The pending lists deleted; every entry must be covered |
| `double/README.md` | new | The reader's guide and the walkthrough |
| `double/tests/` | shared, own lines | The test that runs the walkthrough's examples, if step 3 uses one |
| `double/SPEC-NOTES.md` | shared, own lines | The findings section, and the notes regrouped under it |
| `double/pachinko_double/` | shared, own lines | Only fixes the review finds valid |
| `docs/reference/commands.md` | shared, own lines | The `just python-double` row, if its wording no longer matches |
| `docs/project/repository-map.md` | shared, own lines | The `double/` row, naming the guide |
| `.scratch/python-double/PD06-close-out.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD06's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD05. Record the base and its full commit id in the
   hand-back. Run `just initialize` there (network, authorised by this step). Run
   `just check` and `just python-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **Strict coverage.** Delete `PENDING_SCENARIOS` and `PENDING_RULES` and the code that
   reads them. Quote two probes with their output and exit status, then revert each: a
   test's `# scenario:` tag removed fails the coverage test, and a table row removed
   fails it. Strict coverage also means that every registered rule fired.
3. **The reader's guide.** Write `double/README.md` for a reader who knows Python and has
   the specification open beside it. It holds:
   - how to run the double (`just python-double`);
   - a table from each entity, rule and invariant of the specification to what holds it
     and its file: for an entity its class, for a rule its trigger's table and module, and
     for an invariant its checker entry;
   - how the clock, the lease ids, `tick()` and the inspection seam stand in for what the
     specification leaves to time and to the implementation;
   - a walkthrough of Scenarios A to D, each as a short session against the double's
     API with the state after each step.

   Keep the walkthrough from drifting: run its examples as tests, for example as
   `doctest` examples loaded by a `unittest` test through `doctest.DocFileSuite`, which
   is standard library. Quote a scratch break of one example failing the recipe, then
   revert it.
4. **The findings.** Use the `review-docs` skill for the prose. Open
   `double/SPEC-NOTES.md` with a findings section, and regroup every note under one of
   these headings, keeping its id:
   - where the double and a ported scenario disagreed with the specification;
   - where the specification is ambiguous;
   - where it is silent;
   - the `impl-choice` rows.

   Each finding states the question the maintainer decides. List the specification's six
   open questions, and each `oq:N` scenario, as untouched.
5. **The handbook.** Update the `double/` row in `docs/project/repository-map.md` to name
   the guide, and the `just python-double` row in `docs/reference/commands.md` if its
   wording no longer matches. Run `just check-docs`.
6. **Adversarial review** (checkpoint 1: conformance, breakage and design, over the whole
   double and `double/SPEC-NOTES.md`, before the final commit). The brief gives the whole
   of `double/` as well as this ticket's diff. Questions for this checkpoint:
   - Is any rule applied in a way its guards do not allow?
   - Is any invariant checked more weakly than it is stated?
   - Was any scenario ported more weakly than its Rust test asserts?
   - Did any note silently resolve an open question?
   - Does any finding overstate what the evidence shows?
   - Does the guide's table name a class, table or module that does not exist?
7. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. These say what the maintainer should read first in the
   findings. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- `test_coverage.py` has no pending lists. It fails when any catalog id, rule or invariant
  is uncovered or any registered rule never fired, and step 2's two probes are quoted
  with their output and exit status.
- `double/README.md` maps every entity to its class, every rule to its table and module,
  and every invariant to its checker entry, all of which exist, and its
  walkthrough of Scenarios A to D runs as tests under `just python-double`.
- `double/SPEC-NOTES.md` opens with the findings section. Every note sits under one of its
  headings, and every open question is listed as untouched.
- `just python-double` leaves no `__pycache__/` under `double/`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The `rg`
lines show that no pending list is left. The probe is the positive control: the pattern
matches each list's name.

```sh
set -eu
git rev-parse HEAD
just python-double
probe=$(printf 'PENDING_SCENARIOS = set()\nPENDING_RULES = set()\n' | rg -c 'PENDING_(SCENARIOS|RULES)')
test "$probe" -eq 2
if rg -n 'PENDING_(SCENARIOS|RULES)' double; then exit 1; else test "$?" -eq 1; fi
sources=$(find double -name '*.py' -print)
test -n "$sources"                      # positive control: the find sees the tree
caches=$(find double -name __pycache__ -print)
test -z "$caches"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `7905ad61ac98e61eab2c0bb61cd3450785f6f239`, PD07's scope commit. It was made
  after PD05's record commit `fb23225cfe82f09b92b477a6bfdac100c411989d`.
- Work commit: `690a27bbfdcad551b6f676698ebbf1296b63e645`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch in
  this one worktree, as PD01's hand-back section 1 records. That departs from AGENTS.md's
  one-ticket-per-worktree rule and from step 1's "create the worktree on `branch:`".
  `ticket/pd06-close-out` was not created.
- `just initialize` was not run. The series runs in one worktree, whose PD01 baseline was
  green without it, and PD06's baseline is quoted from PD05.

### 2. Baseline

Quoted from PD05, not rerun (`ai_tmp/pd/PD06/baseline.txt`).

- `just python-double` at `15fffd6fc14816413fdf15c432c0be5a3ab52204` printed
  `Ran 274 tests` and `OK` (PD05's Verification).
- `just check` on the tree committed as `7905ad61ac98e61eab2c0bb61cd3450785f6f239` ended
  `All checks passed and the worktree is unchanged.` (exit 0). It ran just before that
  commit, which changed only `.scratch/python-double/PD07-inspection-contract.md`, after
  PD05's record commit.

### 3. What landed

This ticket owns no rule and no scenario, adds no table row and moves no scenario under
shared rule 2. Nothing under `double/pachinko_double/` changed: every probe edit there was
reverted and its SHA-256 matched. The suite runs 271 tests: 274 at PD05, less four that
existed only for the pending lists, plus one guide test.

Strict coverage, in `double/tests/test_coverage.py`:

- `PENDING_SCENARIOS`, `PENDING_RULES` and every read of them are deleted.
  `rg -n 'PENDING_(SCENARIOS|RULES)' double` finds nothing.
- Four tests that existed only for the lists are gone:
  `test_every_pending_scenario_is_a_catalog_row`,
  `test_no_scenario_is_both_tagged_and_pending`,
  `test_every_pending_rule_is_a_specification_rule` and
  `test_no_rule_is_both_registered_and_pending`.
- Two are renamed: `test_every_scenario_is_tagged` and `test_every_rule_is_registered`.
  The fired check is now `registered - fired`, so every registered rule must have fired.

The guide, in `double/README.md` (new) and `double/tests/test_guide.py` (new):

- The guide says how to run the double, maps the specification to the package, and lists
  the 8 entities, the 32 rules (trigger, table constant, module) and the 35 invariants
  (where each is declared, and its checker function). It also covers the clock, the lease
  ids, `tick()` and the inspection seam, and flags positions and lease ids as the double's
  own (N04, N05). It reads records by key, never by snapshot order (N31).
- Its walkthrough of Scenarios A to D is one doctest session. A and C end by release, B
  and D by lapse. It builds its own setting, with the clock at 0 and TTLs 1000, 5000 and
  7000, so a reader sees the whole setting. It cites the catalog rows S078 to S081 under
  decision 0020.
- `GuideTest.test_the_guides_walkthrough_runs_as_written` runs the page through
  `doctest.DocFileSuite` with a recording runner. It fails unless every collected example
  ran, every `pycon` block holds an example that ran, and each of Scenarios A to D holds a
  block. The review tightened it to that form (section 5).
- `ai_tmp/pd/PD06/guide-names-check.py` resolves every name the guide's tables use by
  import. It found 8 entities, 32 rules and 35 invariants, each equal to the specification's
  count, and problems: 0, exit 0. Its positive control, two wrong rows, was caught.

The findings, in `double/SPEC-NOTES.md`:

- The file opens with a findings section: a rulings table, 15 numbered questions, the six
  open questions quoted verbatim and left untouched, the one `oq:N` row (S026, `oq:7`),
  and the notes that leave no question.
- The 38 notes follow under the ticket's four headings, demoted one level, ids kept. A
  note-body diff against `7905ad61ac98e61eab2c0bb61cd3450785f6f239` shows only the added
  Ruling lines on N24 and N38 (N31 already had one), plus N32's, which the lead added
  after the review (section 7).
- Placement (`ai_tmp/pd/PD06/notes-regroup-check.txt`):
  - disagreed: N13, N24, N31, N32 and N38;
  - ambiguous: N02, N06, N07, N08, N12, N14, N21, N26, N28, N35 and N37;
  - silent: N01, N03, N05, N11, N15, N17, N19, N20, N25, N27, N30, N33, N34 and N36;
  - `impl-choice`: N04, N09, N10, N16, N18, N22, N23 and N29.
- The builder chose the heading for some notes where more than one fits. N13 and N32 sit
  under "disagreed" because the Rust's literal assertion (the batch's `expires_at`, the
  publish's `Superseded`) fails against the double. N14 and N26 sit under "ambiguous"
  though one test each is `impl-choice`. N04 sits under `impl-choice` though it is also
  silent. The group introductions cross-reference these.

The handbook:

- `docs/project/repository-map.md`: the tree lists `double/README.md` and redescribes
  `SPEC-NOTES.md`, and the `double/` row names the guide.
- `docs/reference/commands.md` is unchanged: its `just python-double` wording still matches.
- `AGENTS.md` is untouched, as instructed.

Also changed in the review's apply step (section 5): `double/tests/fixtures.py`,
`test_deferral.py`, `test_endings.py`, `test_expiry.py`, `test_publish.py` and
`test_pull.py`.

### 4. Probes

Each probe went through `just python-double`, was saved with its exit status, and was
reverted. The tree for each is `7905ad61ac98e61eab2c0bb61cd3450785f6f239` plus PD06's
uncommitted work. Files are in `ai_tmp/pd/PD06/`. In probes 2 and 4 the coverage module's
tag-run and test-def checks also fail, on the tests the probe broke.

| File | Probe | Key line | Exit |
| --- | --- | --- | --- |
| `probe-01-scenario-tag-removed.txt` | step 2: `# scenario: S078` removed from `test_endings.py` | `AssertionError: Lists differ: ['S078'] != []` in `test_every_scenario_is_tagged`; `FAILED (failures=1)` | 1 |
| `probe-02-table-row-removed.txt` | step 2: the `ReviveParkedPocket` row removed from `POCKET_TOUCHED` | `AssertionError: Lists differ: ['ReviveParkedPocket'] != []` in `test_every_rule_is_registered`; `FAILED (failures=5)` | 1 |
| `probe-03-row-never-fired.txt` | `Registry.record_fired` skips `DeadLetterDecays` | `AssertionError: Lists differ: ['DeadLetterDecays'] != [] : registered rows that never fired`; `FAILED (failures=1)` | 1 |
| `probe-04-guide-example-broken.txt` | step 3: Scenario A's expected pull order changed to `['k', 'early']` | `Expected:` `['k', 'early']` `Got:` `['early', 'k']`, at README.md line 364; `FAILED (failures=2)` | 1 |

Probe 3 shows that the fired-rule check holds with no pending list. Probe 4's second
failure is the coverage module's test-def check on `test_the_guides_walkthrough_runs_as_written`.
Probes 1 to 4 ran before the review tightened the guide test, so the guide test's own
mutants are in section 5. Each revert matched its SHA-256 (the files record it).

### 5. Checkpoint 1 record: conformance, breakage and design (before the final commit)

- Ran at: `7905ad61ac98e61eab2c0bb61cd3450785f6f239` plus uncommitted work (steps 2 to 5).
  The brief named every file under `double/` to read in full, with
  `docs/specs/pachinko.allium`.
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command.
  One Claude (sonnet) runner subagent per lens. Each findings file is Codex's last message
  (`Source: last-message`), since Codex cannot write under read-only. Codex could not run
  `just check` in its sandbox, and the tree was confirmed unchanged afterwards.
- Lenses: conformance, breakage and design, over the whole double and `SPEC-NOTES.md`.
- Triage and apply: two fresh opus subagents, one for the tests and one for the notes.
  They wrote verdicts in parallel, read-only, then applied one writer at a time: the
  notes first, then the tests in two parts.
- Findings: 0 high, 6 medium, 3 low.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| B1 | medium | valid | S036 never read `lease_view()`, so a view that caches its first answer passed | S036 reads the consumer's lease view before the first pull and after each pull; the cached-view mutant fails |
| B2 | medium | valid | the guide test required only one example in the whole README | the recording runner of section 3; five mutants fail |
| C3 | medium | valid (duplicate of B2) | the same | the same fix |
| D2 | low | valid (duplicate of B2) | the same | the same fix |
| C1 | medium | valid in part | nothing pinned N32's "a publish answers nothing"; porting the Rust's `PublishOutcome` would change the package and answer N32 | the maintainer's ruling (section 7); `fixtures.publish_answers_nothing` at all 37 Rust outcome asserts (32 call sites); four mutants fail |
| D1 | medium | valid | finding 6 suggested dropping expiry guards that the specification needs for a renewal processed before a pending lapse | finding 6 split: Pull's and Release's guards keep "say so or drop them"; the expiry guards are unreachable only under the double's scheduling (N37) |
| C2 | medium | valid (duplicate of D1) | the same | the same fix |
| C4 | low | valid | finding 7 attributed lapse ordering to S091, which never ticks | finding 7's evidence split: one lease end's order (S091, decision 0020, N25, N29) and the order within one tick (the untagged decay test, N34) |
| C5 | low | invalid | the single branch is the maintainer's direction, recorded in every hand-back | nothing |

- Triage also checked S033 for B1's gap and disproved it. S033 reads the snapshot's active
  lease where the Rust reads `lease_view("c2")`, but the view is a pure function of the
  snapshot and S033 already asserts the refused pull leaves it unchanged. Mutant
  `m10-view-leaks-other-lease` already fails two tests. S033's port is left as it is.
- Mutants run in the tree through `just python-double`, all exit 1. Each touched file was
  hashed before and after, and every revert matched.

  | n | Mutant | File | First failure |
  | --- | --- | --- | --- |
  | 1 | `m1-cached-view` | `queue.py` | S036: `Tuples differ: ('pk', 1001000) != ('pk', 1001400)` |
  | 2 | `m2-scenario-c-prose` | `README.md` | `a pycon block holds no example that ran` (two blocks) |
  | 3 | `m3-scenario-d-prose` | `README.md` | `a pycon block holds no example that ran` |
  | 4 | `m4-scenario-a-skipped` | `README.md` | `48 != 50 : every collected example runs` |
  | 5 | `m6-scenario-b-text` | `README.md` | `Scenario B holds no pycon block` |
  | 6 | `m10-view-leaks-other-lease` | `consuming.py` | the S033 disproof: two tests fail |
  | 7 | `m8-publish-answers` | `queue.py` | every covered publish row, 39 failures |
  | 8 | `m9-stale-publish-answers` | `queue.py` | 12 failures (it passed the whole suite before the ruling's fix) |
  | 9 | `m11-new-pocket-answers` | `queue.py` | S011, S013, S024 and the guide |
  | 10 | `m12-new-key-answers` | `queue.py` | S012, S021, S023, S026, S062 and the guide |

- Evidence: `ai_tmp/review/PD06-1/verdicts-tests.md`, `verdicts-notes.md`,
  `findings-conformance.md`, `findings-breakage.md`, `findings-design.md`,
  `apply-tests-final.txt` and `apply-notes-final.txt`.
- Re-run after apply: `just python-double` ran 271 tests, OK, exit 0; `just lint` exit 0;
  `just check-docs` exit 0 (`Validated 48 pages and 49 canonical topics.`); no
  `__pycache__`.
- Second round: none, because no high-severity finding was valid.
- Handed back: nothing outside Files touched. The lead added N32's Ruling line and its row
  in the findings section's rulings table.

### 6. What the maintainer should read first in the findings

Start with the "Questions for the maintainer" list in `double/SPEC-NOTES.md`. Five
findings carry the most weight, because each decides what an implementation must show or
accept.

1. **Finding 1, the inspection shape (N24, N31, N38).** Five Rust tests assert facts about
   the Rust's snapshot that the specification does not state: S053 and S066 (the empty
   lease registry), S091 and S093 (the record order), and S075 (a stored pocket order).
   Ported literally they fail against a double that follows the specification. The
   maintainer has accepted the ports for now and PD07 follows up. Read it first because it
   is the one disagreement that already changed what the double asserts.
2. **Finding 2, what each trigger answers (N13, N24, N32, N33).** The specification
   answers only a pull. A batch carries no `expires_at`, so a consumer cannot know when to
   renew (N13). A publish answers nothing, where decision 0020 gives a deferral the answer
   `superseded` (N32). A release and a mark answer nothing (N24), and what a tick reports
   is the implementation's choice (N33). It decides what a caller sees.
3. **Finding 3, lease identity (N05, N15, N23).** Fencing has to tell an ended lease from
   its successor over the same pocket and consumer, so the double adds `Lease.id`. The
   `Consuming` surface's lease view exposes no id. Read it because the double had to
   invent a field to make fencing work.
4. **Finding 4, joined publishers and consumers (N10, N18, N22).** Every trigger takes a
   `Publisher` or a `Consumer` that only a join creates, and nothing says what an id that
   never joined does. The double refuses it before any rule runs. Every test of that
   refusal is `impl-choice`, so the specification cannot confirm it.
5. **Finding 6, guards that restate (N14, N26, N37).** Pull's and Release's `requires`
   cannot fail while `UnleasedHasNoActiveLease` holds, so the refusals past fencing are
   reached only by hand-built dispatch. The review narrowed this finding: the expiry guards
   restate `LeaseLapses`, not an invariant, and the specification gives them a job when a
   lapse is processed after a release or a renewal. Read it with N37's scoping in view.

Findings 7 to 15 are narrower. The six open questions are listed under "Open questions
left untouched", and no note answers one.

### 7. Departures and rulings

#### Maintainer's ruling on N32

Recorded in `ai_tmp/pd/PD06/ruling-N32.txt`. On 2026-11-19, through AskUserQuestion, the
maintainer chose "Pin None, follow up (Recommended)" for review finding C1.

- The double keeps the specification's behaviour: a publish hands nothing back.
- The ported rows (S021, S043, S045 and the other publish rows the Rust checks for an
  outcome) assert that each publish answers `None`. They do not port the Rust's
  `PublishOutcome`.
- N32 records the ruling, and the findings section's rulings table has its row. Whether
  the specification should name a publish answer stays a finding (finding 2).

#### Other departures

- The series lands on one branch in one worktree (section 1).
- Step 1 asks for `just initialize` and for `just check` and `just python-double` on the
  untouched worktree. Neither was rerun. The baseline is quoted from PD05 (section 2).
- The final `just check` was not part of the builder's runs. The lead's Verification in
  section 8 carries it.
- Placement of notes under the four headings is the builder's judgement (section 3).
- The builder added Ruling lines to N24 and N38, worded from PD05's hand-back and pointing
  to PD07. No ruling is stated as a resolved question about the specification.
- The review changed six test files beyond the ones the builder wrote: it ported more of
  what the Rust asserts (S036's view, the publish answers). It added no scenario and no
  rule.
- The walkthrough's doctests also feed the fired-rule record. The 270-test green run after
  step 2 and before the guide existed shows all 32 rows fired without them.
- Finding 9 cites PD05's hand-back for tick atomicity, which no note holds.
- The Codex lenses were run through the corrected command from the ticket, not §14's
  printed one (section 5).
- `AGENTS.md`'s project map still omits `double/`. That point stays the maintainer's, as
  PD01's Open points say.
- The final `/code-review` runs after this ticket's record commit, and any fixes it brings
  get their own commit.

### 8. Verification

The Verification block ran at `690a27bbfdcad551b6f676698ebbf1296b63e645`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
690a27bbfdcad551b6f676698ebbf1296b63e645
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
...............................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 271 tests in 0.138s

OK
…  (just check-docs and just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

### 9. The final review, after the record commit

After `1e2962835235e71b04bd050cff915745f9bb06a5`, the plan's final review ran over the
whole series, `6ebb76b393687e88656ba6594cf1c53584fc5d70...HEAD`.

- **Harness:** Claude Code's `/code-review` at medium effort. Its brief carried shared
  rule 1 and the maintainer's rulings: N24, N31 and N38's inspection-shape ports, and
  N32's answer.
- **Result:** no important finding. It checked all 32 rules, the 35 invariants, the
  guards, the ensures, `Freed`'s stamping and the allotting of positions against the
  specification, and found no divergence.
- **Five nits and one convention point:**

| Finding | Disposition |
| --- | --- |
| `FakeClock.set()` accepts an earlier time, while `advance()` refuses a negative duration | Dropped and documented. A guard broke S085's port, which sets the clock back to keep the Rust's proof that a renew extends from its own time. `set()`'s docstring now says so. |
| A drawn lease id survives an outcome that is not committed | Documented in `Outcome.draw_lease_id`. It remains PD01's open point (N15). |
| A `Freed` naming a lease the state lacks raised a bare `ValueError` | Fixed test-first. No row answers it, so the table's `Defect` applies. |
| `Outcome.remove` of a shed entry the state lacks raised a bare `ValueError` | Fixed test-first. It raises `Defect`, as the keyed branch does. |
| `recording_run` never recorded a pass for a test run without a result | Fixed test-first. It makes a result, as `TestCase.run` does. |
| `AGENTS.md`'s project map omits `double/` | Not applied. It is the maintainer's open point, as PD01's Open points say. |

The fixes are in `10571fc6d3cb64c6de7b96d02b1da7c4a11a82fc`, committed after a green
`just check`. The fixer once used the system `python3` to apply a test edit; it ran no
test that way. It also removed an untracked `double/pachinko_double/__pycache__/` that an
earlier run without `-B` had left after the record commit.

The Verification block ran again at `10571fc6d3cb64c6de7b96d02b1da7c4a11a82fc` and
exited 0. The opening and closing lines of its output:

```text
10571fc6d3cb64c6de7b96d02b1da7c4a11a82fc
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
..................................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 274 tests in 0.129s

OK
…  (just check-docs and just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- Whether the double later becomes an oracle for the Rust tests, or joins `just check`, is
  the maintainer's decision. This series does neither.
- `AGENTS.md`'s project map, if PD01 left it for here.
- Should the specification name what a publish answers? The maintainer pinned `None` for
  now; whether a deferral answers `superseded`, as decision 0020 gives it, stays open
  (N32, finding 2).
- The four headings are the builder's placement for N13, N32, N14, N26 and N04. The
  maintainer may want a different one for any of them (section 3).
- S033 still reads the snapshot's active lease where the Rust reads `lease_view("c2")`.
  Triage disproved a coverage gap there. Porting the line would be a one-line fidelity
  change that catches nothing alone, so the lead may choose to leave it.
- Finding 9 cites PD05's hand-back for tick atomicity, because no note holds it. A note
  for it would make the finding self-contained.
- The final `/code-review` follows this ticket's record commit. Any fix it brings gets its
  own commit and is not part of the work commit above.
- Pushing the branch and opening a pull request both need the maintainer's authorisation
  (§11, "Separately authorised actions").
