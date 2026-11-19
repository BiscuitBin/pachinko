---
id: PD04
title: "Python double: deferral behind the held version"
status: done
depends_on: [PD03]
parallel_with: []
branch: ticket/pd04-deferral
estimated_size: M
---

# PD04: Python double: deferral behind the held version

## Context

PD01 to PD03 landed the Python double under `double/`: publish, pull, renew, marks,
fencing, release, the lease-end module with its three rows that have nothing deferred,
dead letters and the idle revival. Read their hand-backs and `double/SPEC-NOTES.md` first.
`.scratch/python-double/README.md` is the series' index.

This ticket lands what happens when a newer version of a held key arrives. It covers
deferral with its arrival position, the supersede of a deferred version, and the three
deferred rows of the lease-end table, by release. The design is decision 0020
(`docs/decisions/0020-held-set-until-lease-ends.md`). The specification facts it rests
on, all from `docs/specs/pachinko.allium`:

- **Deferral.** `DeferBehindHeldVersion` answers a publish of a higher rank onto a held
  record with nothing deferred. It sets `rank` and `payload` to the new version and
  `deferred_position` to `queue.next_position`, the arrival position, and advances the
  counter. The held version, `held_rank` and `held_payload`, does not change, whatever
  its mark.
- **A second higher rank.** `Supersede`'s held branch, written in PD01, answers a higher
  rank onto a held record whose version is already deferred. The deferred version is shed
  at once as a `SupersededRecord`. The newer version inherits the arrival position, and
  the held version is never shed (S045).
- **Judged against the stored rank.** The stored rank is `record.rank`, the newest
  version's. An equal rank is refused and a lower one is stale, whatever the mark (S082).
  Publishing again at the held rank is refused while nothing is deferred, and stale once
  a newer version is (S083).
- **The deferred lease-end rows.**
  - `SupersedeUnsettledHeldVersion`: the held version is shed with
    `shed_at = ended_at`, and the newer version is queued at the held version's position.
  - `ConsumeAckedVersionQueueDeferred`: the acked version is consumed, and the newer
    version is queued at its arrival position.
  - `DeadLetterNackedVersionQueueDeferred`: the nacked version becomes a dead letter, and
    the newer version is queued at its arrival position.
  - A record with a deferred version is not an ending record, so a release with one
    requeues the pocket rather than idling it (S054).
- **Where the rows go.** The three deferred rows are rows of the lease-end module's
  `Freed` table, which PD03 created with `ending_records` already written in full, and
  `DeferBehindHeldVersion` is a row of the Publish table. This ticket changes neither the
  lease-end module's interface nor its release callers.
- **The arrival-position invariants.** `DeferredHasArrivalPosition`,
  `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `HeldCopiesAgree`,
  `ArrivalPositionUnoccupied`, `UniqueArrivalPosition` and `ArrivalPositionsBelowNext`
  become reachable here. PD01's checker already holds them.

This ticket covers the release path only. The lapse half of each "by release or lapse"
row lands in PD05 with `tick()`. That is why S080 and S088, which the series plan first
put here and whose Rust tests loop over both endings, moved to PD05 under shared rule 2.
S091 moved here from PD03, because it defers a version behind each of two held keys, and
S037 from PD02, because it publishes a higher rank of a key while it is held.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/settle.rs`,
`publish.rs`, `pull.rs` and `lease.rs`, found with `rg -n 'scenario: S0NN'`.

## Goal

A newer version published onto a held key is deferred behind the held version and never
lost. A still newer one sheds the deferred version and keeps its arrival position. The
release reconciles every row of the lease-end table, and the checker holds throughout.
Every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (4) | `DeferBehindHeldVersion`, `SupersedeUnsettledHeldVersion`, `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedVersionQueueDeferred` |
| Scenarios (12) | S021, S037, S043, S044, S045, S046, S054, S071, S072, S082, S083, S091 |

## Non-goals

- No lapse, parking or decay, and no lapse half of a "by release or lapse" scenario. PD05
  lands them.
- No change under `crates/`, `docs/specs/`, `docs/decisions/`, `loadgen/` or `fuzz/`.

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

  `<id>` is `PD04`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `double/pachinko_double/` | shared, own lines | Deferral in the Publish table, the deferred supersede, and the three deferred rows in the lease-end module's `Freed` table |
| `double/tests/` | shared, own lines | The deferral tests; the pending lists in `test_coverage.py` |
| `double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/python-double/PD04-deferral.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/PD05-clock-lapse-parking-decay.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/python-double/README.md` | shared, own lines | PD04's Status cell, and any move |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD03. Record the base and its full commit id in the
   hand-back. Run `just initialize` there (network, authorised by this step). Run
   `just check` and `just python-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read decision 0020, the Publish and Lease end
   sections of the specification, the guarantees `OldestSlotNewestPayload`,
   `NothingLostWhileHeld` and `EveryDropIsVisible`, and the catalog rows this ticket
   owns. Confirm that no owned scenario needs a lapse. Move any that does, under shared
   rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test. Points to watch:
   - The three deferred rows go inside the lease-end module, beside PD03's three. If one
     seems to need a change to the module's interface or to a release caller, **stop**
     and say so in the hand-back rather than make it.
   - The arrival position is allotted when the first newer version arrives, not when the
     lease ends, and the counter is not advanced again at the end.
   - A superseded record made at the lease's end takes `shed_at` and `expires_at` from
     `ended_at`, not from the rule's own `now`.
   - S091's order of the entries one lease end creates, smallest held position first, is
     more than the specification states. Port it as `impl-choice`.
   Shrink `PENDING_SCENARIOS` and `PENDING_RULES` by exactly what landed.
4. **Spec notes.** Add a note wherever deferral forced an interpretation. Candidates:
   - S091's order;
   - whether a superseded record shed from the deferred slot takes the pocket's current
     time or the lease's.
5. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while a held version was
   shed during its lease, while a deferred version lost its arrival position on a second
   supersede, or while one publish ended in both shed queues.
6. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. **Stop** before pushing or opening a pull request: both
   need authorisation (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- On exit, `PENDING_SCENARIOS` holds exactly the 34 ids PD05 owns, and `PENDING_RULES`
  the 7 rules it owns.
- Every owned scenario has a tagged test, and each of the four rules has its row in its
  trigger's table, fired in at least one test.
- This ticket's diff leaves the lease-end module's interface and its release callers
  unchanged.
- The arrival-position invariants' positive controls still pass. A test or a quoted probe
  shows that the premise of each of those seven invariants is true in at least one state
  the checker sees during this ticket's scenarios, so none of them holds only vacuously.
- `double/SPEC-NOTES.md` holds this ticket's notes, including S091's `impl-choice`.
- `just python-double` leaves no `__pycache__/` under `double/`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails.

```sh
set -eu
git rev-parse HEAD
just python-double
sources=$(find double -name '*.py' -print)
test -n "$sources"                      # positive control: the find sees the tree
caches=$(find double -name __pycache__ -print)
test -z "$caches"
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `dab253a7a02737a7d42806eee7146b1b6a1e2bd1`, PD03's record commit.
- Work commit: `252e98f7f5bee89c7b36554d12c0fceb26eb72d6`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch
  in this one worktree, as PD01's hand-back section 1 records. That departs from
  AGENTS.md's one-ticket-per-worktree rule and from step 1's "create the worktree on
  `branch:`". `ticket/pd04-deferral` was not created.
- `just initialize` was not run. The series runs in one worktree, whose PD01 baseline
  was green without it, and PD04's baseline is quoted from PD03.

### 2. Baseline

Quoted from PD03, not rerun (`ai_tmp/pd/PD04/baseline.txt`).

- `just python-double` at `c5d0ff3897e4ccd25fd60de1286c470ac0a28e43` printed
  `Ran 209 tests` and `OK` (PD03's Verification).
- `just check` on the tree committed as `dab253a7a02737a7d42806eee7146b1b6a1e2bd1` ended
  `All checks passed and the worktree is unchanged.` (exit 0). It ran just before that
  commit, which changed only the two ticket files.

### 3. What landed

Rows per table. Each row fires in at least one test, and the coverage test's fired-rule
check passes. No table raised a `GuardOverlap`.

| Table | Rows added | Rows now |
| --- | --- | --- |
| Publish | `DeferBehindHeldVersion` | 5 |
| `Freed` | `SupersedeUnsettledHeldVersion`, `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedVersionQueueDeferred` | 6 |

PD01's `Supersede` held branch now fires, in S045 and S082.

Files under `double/`:

- `pachinko_double/publish.py`: the `DeferBehindHeldVersion` row in `PUBLISH`, between
  `Supersede` and `DropStalePublish`, and the module docstring.
- `pachinko_double/lease_end.py`: the three deferred `Freed` rows, with the helpers
  `_freed_deferred` (the sibling of `_freed_with`) and `_queued_at_arrival`, and the
  docstring.
- `tests/test_deferral.py` (new): the 12 owned scenarios and three untagged tests.
- `tests/test_lease_end.py`: `CONFIG` gained a distinct `superseded_ttl`, and four new
  tests.
- `tests/test_coverage.py`: the pending lists.
- `SPEC-NOTES.md`: N29 to N32.

Outside `double/`, only this ticket's hand-back and `status:` and the README's PD04
Status cell are written, as Files touched says.

Unchanged, confirmed by `git diff --quiet` before the commit: `settle.py` (the release
callers), `queue.py`, `inspection.py` and `invariants.py`. PD03's three `Freed` rows,
`_freed_with`, `end_lease`, `Freed`, `ending_records` and the two predicates are
byte-for-byte unchanged, so the lease-end module's interface and its release callers are
as PD03 left them.

Pending lists, each checked by `diff` against PD05's ownership table:

- `PENDING_SCENARIOS` holds 34 ids (the ticket asks for 34).
- `PENDING_RULES` holds 7 rules (the ticket asks for 7).

No scenario moved under shared rule 2. None of the 12 Rust tests ticks. S054 uses a short
`lease_ttl` but only releases. S080 and S088 had already moved to PD05 before this
ticket.

The three untagged tests in `test_deferral.py`:

- The Publish guards on a held record, per mark, deferred or not.
- `ArrivalPositionInvariantsAreReachedTest`, the non-vacuity test below.
- `DeferralTouchesItsPocketTest`, a hand-built dispatch that asserts the replace, the
  `PocketTouched` emit and one allotment.

The four new tests in `test_lease_end.py`:

- One that exactly one of the six rows answers each (mark, deferred) pair.
- `FreedDeferredRowsTest`: three hand-built rows where `now != ended_at`. Each asserts
  the exact writes and `allotted == 0`.

Non-vacuity of the seven arrival-position invariants. S091's committed state before the
release has `b` and `z` with a deferred version and `y` and `a` without. All seven
premises are true there, including `UniqueArrivalPosition`'s, which needs two deferred
records, and `check(state) == ()`. PD01's positive controls are unchanged and pass.

### 4. Red runs and probes

#### Red runs

Each red run went through `just python-double`, was saved with its exit status, and was
followed by the change that turned it green. Files are in `ai_tmp/pd/PD04/`. The tree
for each is `dab253a7a02737a7d42806eee7146b1b6a1e2bd1` plus the uncommitted PD04 files.
In every run the coverage module's tag-run check and test-def check also fail, on the new
tags and test defs.

| File | Red before | Scenarios | Key output line | Exit |
| --- | --- | --- | --- | --- |
| `red-S021-S037-S043-S046-S082-S083.txt` | No `DeferBehindHeldVersion` row in the Publish table | S021, S037, S043, S046, S082 (three marks), S083 | `FAILED (failures=2, errors=8)`; 215 tests | 1 |
| `red-S044-S045-S054-S071-S072-S091.txt` | No deferred row in the `Freed` table | S044, S045, S054, S071, S072, S091 | `FAILED (failures=2, errors=6)`; 221 tests | 1 |
| `red-S072-S091.txt` | `ConsumeAckedVersionQueueDeferred` added, the other two rows absent | S072, S091 | `FAILED (failures=2, errors=2)`; 221 tests | 1 |
| `red-S091.txt` | `DeadLetterNackedVersionQueueDeferred` added, `SupersedeUnsettledHeldVersion` absent | S091 | `FAILED (failures=2, errors=1)`; 221 tests | 1 |

Notes on the table:

- The last two files are steps of the second run, taken as each `Freed` row went in.
  They show S072 and S091 still red after the acked row, and S091 alone after the nacked
  row.
- The first file's errors were `EqualRankRefused` where the specification defers. The
  second file's were `Defect: no rule of the Freed table answers Freed(...)`.
- The scenario column is the builder's reading of the failing test names in each file.
  S082 appears as three subtests, one per mark.

#### Scenarios with their own red run

12 of the 12 owned scenarios went red for their own reason: S021, S037, S043, S044, S045,
S046, S054, S071, S072, S082, S083 and S091. The first file holds six of them and the
second holds the other six.

#### Scenarios that could not be red on their own

None. Mutant probes still ran against all of them (below).

#### Probes

Each probe ran through `just python-double` on the tree at
`dab253a7a02737a7d42806eee7146b1b6a1e2bd1` plus the uncommitted PD04 files, then was
reverted. `probe.sh` copies the file first, and every file ends with a matching SHA-256
before and after. `probes.sh` drives `probe.sh`. The sweep ran at 228 tests. Every
numbered probe exits 1. All files are in `ai_tmp/pd/PD04/`.

| File | Breaks | Key output line | Exit |
| --- | --- | --- | --- |
| `probe-01-defer-overwrites-held.txt` | Defer overwrites the held version | `FAILED (failures=4, errors=18)` | 1 |
| `probe-02-defer-advances-nothing.txt` | Defer does not advance the counter | `FAILED (failures=4, errors=18)` | 1 |
| `probe-03-supersede-held-reallots.txt` | `Supersede`'s held branch re-allots the arrival position | `FAILED (failures=6)` | 1 |
| `probe-04-unsettled-queues-at-arrival.txt` | The unsettled row queues at the arrival position | `FAILED (failures=4)` | 1 |
| `probe-05-acked-keeps-held-position.txt` | The acked row keeps the held position | `FAILED (failures=7)` | 1 |
| `probe-06-nacked-keeps-held-position.txt` | The nacked row keeps the held position | `FAILED (failures=4)` | 1 |
| `probe-07-acked-allots-at-end.txt` | The acked row allots at the lease's end | `FAILED (failures=7)` | 1 |
| `probe-08-shed-newest-version.txt` | The newest version is shed, not the held one | `FAILED (failures=4)` | 1 |
| `probe-09-shed-at-now.txt` | `shed_at` comes from `now` | `FAILED (failures=2)` | 1 |
| `probe-10-shed-expires-from-now.txt` | A shed record's `expires_at` counts from `now` | `FAILED (failures=2)` | 1 |
| `probe-11-nacked-deferred-no-letter.txt` | The nacked deferred row makes no dead letter | `FAILED (failures=4)` | 1 |
| `probe-12-nacked-deferred-in-both-queues.txt` | The nacked deferred row also sheds the held version | `FAILED (failures=4)` | 1 |
| `probe-13-defer-guard-ignores-deferred.txt` | The Defer guard ignores a deferred version | `FAILED (failures=5, errors=4)` | 1 |
| `probe-14-supersede-drops-held-branch.txt` | `Supersede` loses its held branch | `FAILED (failures=5, errors=4)` | 1 |
| `probe-15-stale-against-held-rank.txt` | Stale is judged against the held rank | `FAILED (failures=5, errors=5)` | 1 |
| `probe-16-ending-ignores-deferred.txt` | `ending_records` counts a deferred record | `FAILED (failures=5, errors=3)` | 1 |
| `probe-17-freed-order-by-key.txt` | `Freed` is sent in key order | `FAILED (failures=4)` | 1 |
| `probe-18-defer-sheds-held.txt` | Defer sheds the held version at once | `FAILED (failures=15)` | 1 |
| `probe-19-defer-touches-nothing.txt` | Defer emits no `PocketTouched` | `FAILED (failures=2)` | 1 |
| `probe-19a-defer-touches-nothing-before-its-test.txt` | The probe 19 mutation, run before its test existed | `OK` (`Ran 227 tests`) | 0 |

Which tests kill which breakages, from `summary.md`:

| Probe | Killed by |
| --- | --- |
| 01, 02 | All 12 scenarios, the guards test and the premises test |
| 03 | S045, S082 |
| 04, 08 | S091 and the hand-built `Freed` tests |
| 05, 07 | S044, S045, S054, S071 and the hand-built tests |
| 06, 11, 12 | S072 and the hand-built tests |
| 09, 10 | The hand-built `Freed` tests only |
| 13, 14 | S045, S082 and the guards test |
| 15 | S046, S082, S083 and the guards test |
| 16 | S045, S054, S071, S072 and two `ending_records` tests |
| 17 | S091 through `Queue`, and PD03's hand-built order test |
| 18 | S021, S043, S044, S045, S046, S072, S082, S083, S091 |
| 19 | `DeferralTouchesItsPocketTest` only |

Probe 13's log shows `GuardOverlap: Publish: the guards of Supersede,
DeferBehindHeldVersion all hold`, so the table's overlap check catches that mutation too.

How the surviving probe was handled. Probe 19 first ran as `probe-19a-...txt`, before
any test pinned the `PocketTouched` emit. That run exited 0 with 227 tests `OK`: the
mutant survived. The builder then added `DeferralTouchesItsPocketTest`, the hand-built
dispatch that asserts the replace, the emit and one allotment, and reran the probe as
`probe-19-defer-touches-nothing.txt`, which exits 1. The 19a file is kept as the record
of the survivor.

Limits on this evidence:

- Probes 09 and 10 are killed only by hand-built tests. Through `Queue`, `now` equals
  `ended_at`, so no scenario can tell the two apart.
- Probe 19 is killed only by a hand-built dispatch. The pocket of a held record is always
  leased, so through `Queue` the `PocketTouched` event is a no-op.

The three breakage-lens questions of step 5:

- A held version shed during its lease: probes 01 and 18.
- A deferred version that loses its arrival position on a second supersede: probe 03.
- One publish that ends in both shed queues: probe 12.

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `dab253a7a02737a7d42806eee7146b1b6a1e2bd1` plus uncommitted work (steps 2 to
  4).
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command
  (`< /dev/null`, `-o last-<lens>.md`, `>|` log). One Claude (sonnet) runner subagent per
  lens. Codex cannot write under read-only, so each findings file is Codex's last
  message and its first line says `Source: last-message`. Codex could not run
  `just check` in its sandbox.
- Triage and apply: one fresh opus subagent, the only writer.
- Findings: breakage 0; conformance 2 low. 0 high, 0 medium.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid | N30's Question said shed timestamps order a queued key's ranks, but `DropStalePublish` also creates `SupersededRecord` entries (reason `stale`, `shed_at: now`) | N30's Question limits the claim to the entries `Supersede` sheds while one record stays queued, with the rank-10 example of publishes at 5, 3 and 4 |
| C2 | low | valid | N31's Concerns named no specification anchor, which shared rule 5 requires | N31 names the `Record` comment and `OldestSlotNewestPayload`, and says no surface of the specification names the inspection seam |

- Evidence: `ai_tmp/review/PD04-1/verdicts.md`, `findings-conformance.md`,
  `findings-breakage.md` and `apply-final.txt`.
- Re-run after apply: `just python-double` ran 228 tests, OK, exit 0; `just lint` exit 0;
  no `__pycache__`. The triage subagent's `just lint` skipped markdownlint and lychee
  ("no files to check"), so `double/SPEC-NOTES.md` was not linted as Markdown there. That
  is left to `just check`.
- Second round: none, because no high-severity finding was valid.
- Handed back: nothing outside Files touched.
- After triage the lead added N31's Ruling line (section 7).

### 6. Spec notes added

| Id | Note |
| --- | --- |
| N29 | S091 pins the order of the entries one lease end creates (`impl-choice`). It extends N25: superseded records and dead letters come out smallest held position first. |
| N30 | A deferred version shed by a later publish takes the publish's `now`, while a held version shed at the lease's end takes `ended_at`. It asks whether a reader should be told that `shed_at` does not order a key's versions. |
| N31 | S091's record order is the Rust's inspection shape, not behaviour. The test compares the kept positions by key. The note carries the maintainer's Ruling line. |
| N32 | A deferral answers nothing, where the Rust (and decision 0020) answers `Superseded`. The specification names no outcome. |

### 7. Departures and rulings

#### Maintainer's ruling on S091

Recorded in `ai_tmp/pd/PD04/ruling-S091.txt`. On 2026-11-18, through AskUserQuestion, the
maintainer chose "For now, go with option one, but add a note or even a ticket to
investigate further".

- The Rust's S091 ends with a literal list of the pocket's records, `[("b", 3), ("z", 1)]`.
  The double lists them `[("z", 1), ("b", 3)]`, because its snapshot keeps records in
  insertion order and a replaced record keeps its place. The ported assertion fails
  against it.
- Option one is N31's port. The test compares the kept positions as a map from key to
  position, and the listing order is the Rust's inspection shape, not behaviour.
- The builder ported by key and wrote N31 rather than stopping under shared rule 1. The
  lead and the maintainer had to accept it, as S053's was in PD03.
- N31 now carries a Ruling line that records the acceptance and the request for further
  investigation.
- A follow-up ticket, `.scratch/python-double/PD07-inspection-contract.md`, takes up N31
  together with N24 (S053's kept ended lease). It lands in its own commit right after
  this ticket's record.

#### Other departures

- The series lands on one branch in one worktree (section 1).
- Step 1 asks for `just initialize` and for `just check` and `just python-double` on the
  untouched worktree. Neither was rerun. The baseline is quoted from PD03 (section 2).
- The builder edited three stale forward references in lines PD01 and PD03 wrote. Each
  said PD04's rows were future work, and each was stale once the rows landed:
  - `double/pachinko_double/publish.py`: the module docstring ("PD04 adds
    `DeferBehindHeldVersion`").
  - `double/pachinko_double/lease_end.py`: the docstring paragraph that said the deferred
    rows land later.
  - `double/tests/test_lease_end.py`: one test comment ("join the same table in PD04").

  This reads "own lines" as allowing it. The three edits revert cleanly if the maintainer
  reads it otherwise.
- The builder extracted a shared `_dead_letter` builder from PD03's
  `DeadLetterNackedRecord`, then reverted it under the own-lines rule. The new nacked row
  repeats the specification's `DeadLetter.created`, as the specification itself does.
- Probes 09, 10 and 19 are killed only by hand-built tests (section 4). Probe 19 first
  survived, as `probe-19a-...txt` records, and its test was written afterwards.
- Seven tests have no catalog row: the three untagged tests in `test_deferral.py` and the
  four new tests in `test_lease_end.py` (section 3).
- A deferral answers nothing, where decision 0020's publish table answers `superseded`.
  The tests assert the state the deferral leaves instead (N32).
- S091's order is ported as `impl-choice`, as shared rule 1 and step 3 ask (N29).
- The Rust S021, S037 and the other ported rows were read for their setup, steps and
  assertions only. No behaviour was taken from `crates/libpachinko/src/`.
- Codex cannot write files under read-only, so each findings file is Codex's last
  message (section 5).

### 8. Verification

The Verification block ran at `252e98f7f5bee89c7b36554d12c0fceb26eb72d6`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
252e98f7f5bee89c7b36554d12c0fceb26eb72d6
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
....................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 228 tests in 0.097s

OK
…  (the remaining output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- Should the specification order the entries one lease end creates, or say that their
  order is not observable? S091 is the first scenario that can tell, and the double pins
  smallest held position first as `impl-choice` (N29, extends N25).
- Should a reader of the superseded queue be told that `shed_at` does not order a key's
  versions? A deferred version shed by a publish takes the publish's `now`, and a held
  version shed at the lease's end takes `ended_at`, so a key's lowest rank can be shed
  last (N30).
- Is the order in which a snapshot lists a pocket's records part of the inspection
  contract? The maintainer accepted the port by key for now. PD07 takes this up with
  N24's kept ended lease (N31, S091, S053).
- Should the Publishing surface name what a publish answers? Decision 0020 gives a
  deferral the answer `superseded`, and the specification gives it none, so the double
  answers nothing (N32).
- `just lint` did not lint `double/SPEC-NOTES.md` as Markdown after the checkpoint's
  apply, since markdownlint reported no files to check. Only `just check` covers it.
- The three stale forward references edited in PD01's and PD03's lines (section 7) are
  the builder's reading of "own lines". The maintainer may want a different reading.
- The seven untagged tests have no catalog row. The maintainer may want rows for them.
