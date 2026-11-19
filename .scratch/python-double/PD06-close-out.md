---
id: PD06
title: "Python double: close-out and the specification reader's guide"
status: todo
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

Written at the end. They record the base and its full commit id, the baseline, the step 2
and step 3 probes, the checkpoint's record, where to start reading the findings, and
Verification quoted under its commit.

## Open points

- Whether the double later becomes an oracle for the Rust tests, or joins `just check`, is
  the maintainer's decision. This series does neither.
- `AGENTS.md`'s project map, if PD01 left it for here.
