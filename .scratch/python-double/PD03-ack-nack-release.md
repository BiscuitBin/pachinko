---
id: PD03
title: "Python double: ack, nack, release and the lease's end"
status: done
depends_on: [PD02]
parallel_with: []
branch: ticket/pd03-ack-nack-release
estimated_size: L
---

# PD03: Python double: ack, nack, release and the lease's end

## Context

PD01 and PD02 landed the Python double under `double/`: the entity model, `Config`, the
joins, publish, pull, the held set, renew, the invariant checker and the coverage test.
Read their hand-backs and `double/SPEC-NOTES.md` first. `.scratch/python-double/README.md`
is the series' index.

This ticket lands what a consumer does with a held record, and the lease's end by
release:

- marks, repeats and mark conflicts;
- fencing by lease id;
- a release that requeues or idles the pocket;
- the lease-end module, `lease_end.py`, with the three `Freed` rows that have no deferred
  version;
- dead letters;
- the revival of an idle pocket;
- the lease view.

The specification facts it rests on, all from `docs/specs/pachinko.allium` (the Ack and
nack, Renew and release, and Lease end sections, and the `Consuming` surface):

- **Fencing.** Ack, nack, renew and release name their lease and require it to be active
  and the consumer's. An action naming an ended lease is refused, even when the same
  consumer holds the same pocket again under a new lease, with the same key at a newer
  rank (S068).
- **Marks.** `AckMarksRecord` and `NackMarksRecord` need a held, unsettled record. Each
  extends the lease to `now + lease_ttl`, resets the pocket's `consecutive_expiries` to 0,
  and sets the mark; a nack also stamps `nacked_at = now`. `AckRepeats` and `NackRepeats`
  extend the lease and change nothing else. No rule covers an ack of a nacked record or a
  nack of an acked one, so that action is refused. S085 says the refusal changes nothing,
  not even the lease's expiry, and reads no clock. `FakeClock`'s read count shows that.
- **Release.** `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket` split on
  `pocket.records.count` against `pocket.ending_records.count`. Ending records are held,
  acked or nacked, and have nothing deferred. A requeue gives the pocket a fresh position
  at the back and resets its count. An idle pocket keeps its position, and its count is
  reset. Either way the lease ends `released` at `now` and every held record is sent
  `Freed`. Both release rules are rows of the Release table, and each one's effect ends
  the lease through the lease-end module.
- **The lease-end module.** The lease's end is one table of six rows (the Lease end
  section), and none of its rules reads how the lease ended. So one module owns it, and
  this ticket creates it whole. Its interface is: end this lease, how it ended (release
  or lapse), and when. It owns:
  - the shared end-of-lease steps: set the lease's status and `ended_at`, then send
    `Freed` for each held record;
  - the comparison that decides the pocket's fate, its record count against
    `ending_records`, which the release and lapse guards read;
  - `ending_records`, written in full: held, marked acked or nacked, and
    `deferred_position = null`;
  - the `Freed` table, holding this ticket's three rows with their full guards;
  - the `ended_at` stamping, so that each entry a `Freed` row creates takes its time from
    `ended_at`.
- **The lease's end, with nothing deferred.** `RequeueUnsettledRecord` queues an
  unsettled record at its own position and clears its held fields. `ConsumeAckedRecord`
  removes an acked record. `DeadLetterNackedRecord` removes a nacked record and creates a
  `DeadLetter`: the held rank and payload, `nacked_by` the lease's consumer, `nacked_at`
  from the record, and `expires_at = ended_at + dead_letter_ttl`. The checker runs after
  the release and every `Freed` it caused.
- **Revival.** `ReviveIdlePocket` answers `PocketTouched` on an idle pocket. It queues the
  pocket at a fresh position and resets its count.
- **The lease view.** The `Consuming` surface exposes, for the consumer's active lease,
  the pocket key, `expires_at` and each held record's key, `held_rank`, `held_payload`
  and mark.

S091, which the series plan first put here, publishes a higher rank while its key is
held. It needs `DeferBehindHeldVersion`, so it moved to PD04 under shared rule 2.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/settle.rs`
and `lease.rs`, found with `rg -n 'scenario: S0NN'`. The `wire` rows here are S040, S047,
S053, S068 and S085; port only their library half.

## Goal

A consumer can ack and nack its held records, repeat a mark, be refused a conflicting mark
or a fenced action, and release its lease. The release reconciles each held record by its
mark, creates dead letters and requeues or idles the pocket. A publish revives an idle
pocket. A consumer can read its lease. One lease-end module owns the lease's end, so PD04
adds rows inside it and PD05 only adds callers. The checker holds after each operation,
and every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (10) | `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `RequeueUnsettledRecord`, `ConsumeAckedRecord`, `DeadLetterNackedRecord`, `ReviveIdlePocket` |
| Scenarios (18) | S023, S035, S040, S041, S042, S047, S049, S051, S052, S053, S055, S057, S068, S070, S084, S085, S086, S090 |

## Non-goals

- No deferral and none of the three deferred lease-end rows. PD04 lands them.
- No lapse, parking or decay. PD05 lands them, with the lapse half of every "by release
  or lapse" scenario.
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

  `<id>` is `PD03`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `double/pachinko_double/` | shared, own lines | Marks, fencing, release, the lease-end module with its three `Freed` rows, dead letters, the idle revival and the lease view |
| `double/tests/` | shared, own lines | The settle and lease tests; the pending lists in `test_coverage.py` |
| `double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/python-double/PD03-ack-nack-release.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/PD04-deferral.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/python-double/PD05-clock-lapse-parking-decay.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/python-double/README.md` | shared, own lines | PD03's Status cell, and any move |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD02. Record the base and its full commit id in the
   hand-back. Run `just initialize` there (network, authorised by this step). Run
   `just check` and `just python-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the sections named in Context, the
   guarantees `ActionsNameTheirLease`, `HeldSetStableUntilLeaseEnds` and
   `MarksStandHoweverLeaseEnds`, and the catalog rows this ticket owns. Confirm that each
   owned scenario needs no deferral and no lapse. Move any that does, under shared rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test. Points to watch:
   - Land the lease-end module whole, as Context describes. PD04 adds its three deferred
     rows inside the module, and PD05 only adds callers from the lapse; neither changes
     its interface.
   - Every refusal compares the snapshot before and after: a fenced action, a mark
     conflict, an ack of a key not held, and a second release. S085's refusal also reads
     no clock.
   - Lease ids are compared by value. A new lease over the same pocket has a new id from
     the lease-id source.
   - A release at the grant instant is legal (S057).
   - S090's order of the lease view, smallest position first, is more than the surface
     states. Port it as `impl-choice`.
   Shrink `PENDING_SCENARIOS` and `PENDING_RULES` by exactly what landed.
4. **Spec notes.** Add a note wherever marks, fencing or release forced an
   interpretation. Candidates:
   - which refusal an ack of a queued record of the same pocket is;
   - whether a refusal kind is the specification's to name at all;
   - S090's order;
   - whether `ReleaseIdlesEmptyPocket` leaving the pocket's position unchanged matters,
     given that idle pockets are not in the pocket order.
5. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while a refusal changed
   something, while a dead letter's `nacked_at` or `expires_at` came from the wrong
   timestamp, or while the checker ran part-way through a release.
6. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. **Stop** before pushing or opening a pull request: both
   need authorisation (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- On exit, `PENDING_SCENARIOS` holds exactly the 46 ids PD04 and PD05 own, and
  `PENDING_RULES` the 11 rules they own.
- Every owned scenario has a tagged test, and each of the ten rules has its row in its
  trigger's table, fired in at least one test.
- Both release rows end the lease only through the lease-end module.
- Every refusal this ticket lands has a test showing that the snapshot is unchanged, and
  S085's test shows that no clock was read.
- `double/SPEC-NOTES.md` holds this ticket's notes, including S090's `impl-choice`.
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
- Base commit: `091f0e568e1b2fcba2af33a028fa79123c57c882`, PD02's record commit.
- Work commit: `c5d0ff3897e4ccd25fd60de1286c470ac0a28e43`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch
  in this one worktree, as PD01's hand-back section 1 records. That departs from
  AGENTS.md's one-ticket-per-worktree rule and from step 1's "create the worktree on
  `branch:`". `ticket/pd03-ack-nack-release` was not created.
- `just initialize` was not run. The series runs in one worktree, whose PD01 baseline
  was green without it, and PD03's baseline is quoted from PD02.

### 2. Baseline

Quoted from PD02, not rerun (`ai_tmp/pd/PD03/baseline.txt`).

- `just python-double` at `b8d4613e6ee3b3f274e779a1732c35f728800dfa` printed
  `Ran 167 tests` and `OK` (PD02's Verification).
- `just check` on the tree committed as `091f0e568e1b2fcba2af33a028fa79123c57c882` ended
  `All checks passed and the worktree is unchanged.` (exit 0). It ran just before that
  commit, which changed only the two ticket files.

### 3. What landed

Rows per table. Each row fires in at least one test, and the coverage test's fired-rule
check passes. Both release rows end the lease only through `lease_end.end_lease`.

| Table | Rows | When no guard holds |
| --- | --- | --- |
| Ack | `AckMarksRecord`, `AckRepeats` | A refusal: `LeaseFenced`, `RecordNotHeld` or `MarkConflict`, the lease checked first (N21) |
| Nack | `NackMarksRecord`, `NackRepeats` | The same three refusals |
| Release | `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket` | A refusal: `LeaseFenced`, else `ReleaseRefused`, unreachable through `Queue` (N26) |
| `Freed` (new, in `lease_end.py`) | `RequeueUnsettledRecord`, `ConsumeAckedRecord`, `DeadLetterNackedRecord` | A defect |
| `PocketTouched` | `ReviveIdlePocket` | Nothing happens |

New files under `double/`:

- `pachinko_double/lease_end.py`: `Freed`, `end_lease`, `ending_records`,
  `a_record_outlasts_the_lease`, `no_record_outlasts_the_lease` and the `FREED` table.
- `pachinko_double/consuming.py`: `LeaseView`, `HeldRecordView` and
  `lease_view(snapshot, consumer_id)`.
- `tests/test_ack_nack.py` and `tests/test_lease_end.py`.

Changed files under `double/`:

- `pachinko_double/settle.py`: the Ack, Nack and Release triggers and tables beside
  Renew, with shared heads `_acting_lease`, `_held_record`, `_marked` and `_releasing`.
- `pachinko_double/queue.py`: `Queue.ack`, `nack`, `release` and `lease_view`. `Freed` is
  routed in `EVENT_TABLES`. The clock is read lazily through `_Now` (N28).
- `pachinko_double/refusals.py`: `RecordNotHeld`, `MarkConflict`, `ReleaseRefused`.
- `pachinko_double/rules.py`: a refusal builder takes `(view, trigger)`, where it took
  `trigger`. This lets one table name three refusals.
- `pachinko_double/inspection.py`: `View(state, clock_read)` with a `now` property.
  `View(state, T0)` still works.
- `pachinko_double/publish.py`: the `ReviveIdlePocket` row in the `PocketTouched` table.
- `pachinko_double/identity.py` and `pull.py`: the refusal lambdas follow the new
  builder signature.
- `pachinko_double/__init__.py`: exports `consuming`, `lease_end`, the two view classes
  and the three refusals.
- `tests/fixtures.py`, `test_publish.py` (S023), `test_settle.py`, `test_pull.py`,
  `test_queue.py`, `test_rules.py` and `test_coverage.py` (the pending lists).
- `SPEC-NOTES.md`: N21 to N28.

Outside `double/`, only this ticket's hand-back and `status:` and the README's PD03
Status cell are written, as Files touched says.

No scenario moved under shared rule 2. The Rust tests confirm that none of the 18
publishes onto a held key or ticks. S068's rank-2 publish lands after the release, so it
is a plain supersede of a queued record.

Changes to PD01's and PD02's machinery. There are two:

- The refusal builder takes `(view, trigger)`. Every existing table and the scratch
  tables in `test_rules.py` and `test_queue.py` were updated.
- The apply step reads the clock lazily, at most once per outcome (N28). A committed
  outcome whose rules never read `now`, such as a join, no longer reads the clock. PD01's
  `test_it_reads_the_clock_once_per_outcome` was rewritten so that its scratch rules read
  `now` four times, and it still asserts exactly one read. A new test asserts zero reads
  when no rule reads `now`.

The lease-end interface, for PD04 and PD05:

- `end_lease(view, outcome, lease, ended, ended_at)` replaces the lease with its status
  and `ended_at`, then emits `Freed(lease.id, record_key, ended_at)` for each held
  record, smallest position first. It raises `ValueError` for an active status. The
  caller writes the pocket's own fields.
- `a_record_outlasts_the_lease` and `no_record_outlasts_the_lease` compare
  `records.count` against `ending_records.count`. The release guards read them now, and
  PD05's lapse guards will.
- `ending_records` reads the snapshot's `ending_by_pocket`. Its where clause (held, mark
  not unsettled, `deferred_position = null`) is written in full in `inspection.py`'s
  `_is_ending`, so the set is computed once, in the snapshot. The ticket words
  `ending_records` as owned by the lease-end module. This one piece stays in
  `inspection.py`.
- Each of the three `Freed` rows guards `deferred_position is None`, so PD04's rows sit
  beside them without a `GuardOverlap`. A hand-built test asserts, for each mark, that
  none of the three answers a deferred record. It checks the three rows by name, so it
  survives PD04's rows.
- The rows stamp from `event.ended_at`, never from `now`. That includes the dead letter's
  `expires_at`.

### 4. Red runs and probes

#### Red runs

Each red run went through `just python-double`, was saved with its exit status, and was
followed by the change that turned it green. Files are in `ai_tmp/pd/PD03/`. The tree for
each is `091f0e568e1b2fcba2af33a028fa79123c57c882` plus the uncommitted PD03 files. In
every run the coverage module's tag checks also fail on the new tags.

| File | Red before | Key output line | Exit |
| --- | --- | --- | --- |
| `red-S023.txt` | The `PocketTouched` table has no rows | `InvariantsBroken: invariants broken: IdleHasNoRecords; idle pocket 'a' has 1 records`; `FAILED (failures=3, errors=1)` | 1 |
| `red-S040.txt` | `Queue.ack` and the lease view exist, the Ack table has no rows | `LeaseFenced: Ack by 'c1': lease 'lease-1' is not active and theirs`; `FAILED (failures=3, errors=1)` | 1 |
| `red-S040-import.txt` | `consuming.py` did not exist (weak) | `ModuleNotFoundError: No module named 'pachinko_double.consuming'`; `FAILED (errors=1)` | 1 |
| `red-S042.txt` | `RecordNotHeld` exists, the Ack table refuses everything with `LeaseFenced`, no `AckRepeats` row | `LeaseFenced: Ack by 'c1': lease 'lease-1' is not active and theirs`; `FAILED (failures=3, errors=4)` | 1 |
| `red-S047-S086.txt` | The `Freed` table has no `DeadLetterNackedRecord` row | `Defect: no rule of the Freed table answers Freed(lease='lease-1', record_key='k', ended_at=1000030)`; `FAILED (failures=3, errors=2)` | 1 |
| `red-S049.txt` | `Queue.nack` missing (weak) | `AttributeError: 'Queue' object has no attribute 'nack'. Did you mean: 'ack'?`; `FAILED (failures=3, errors=2)` | 1 |
| `red-S049-table.txt` | `Queue.nack` exists, the Nack table has no rows. Only the nack-mark test fails; S049 itself passes | `LeaseFenced: Nack by 'c1': lease 'lease-1' is not active and theirs`; `FAILED (failures=2, errors=1)` | 1 |
| `red-S053-method.txt` | `Queue.release` missing (weak) | `AttributeError: 'Queue' object has no attribute 'release'`; `FAILED (failures=3, errors=1)` | 1 |
| `red-S055.txt` | The Release table has only the requeue row | `ReleaseRefused: no Release rule answers a release of 'lease-1' by 'c1'`; `FAILED (failures=3, errors=1)` | 1 |
| `red-S085.txt` | `MarkConflict` exists, the classifier answers `LeaseFenced` for a held record, the clock is read eagerly | `LeaseFenced: Ack by 'c1': lease 'lease-1' is not active and theirs`; `FAILED (failures=4, errors=2)` | 1 |
| `red-S085-clock.txt` | The classifier is fixed, the clock is still read eagerly | `AssertionError: 11 != 9 : a refused mark reads no clock`; `FAILED (failures=4)` | 1 |

Notes on the table:

- `red-S040-import.txt`, `red-S049.txt` and `red-S053-method.txt` are weak, since they
  fail only on a missing module or method.
- `red-S049-table.txt` shows that S049 cannot go red through a row-less Nack table. Its
  refusals are that table's answer when no row holds.
- `red-S085.txt` and `red-S085-clock.txt` are two steps of one scenario. The second is
  the one that needed the lazy clock read (N28).

#### Scenarios with their own red run

7 of the 18 owned scenarios went red for their own reason: S023, S040, S042, S047, S055,
S085 and S086.

#### Scenarios that could not be red on their own

11 of the 18 could not: S035, S041, S049, S051, S052, S053, S057, S068, S070, S084 and
S090.

- S049 and S053 went red only on a missing method.
- S049 passed against a row-less Nack table (above).
- The rest were written after the rows they exercise.

Mutant probes kill all 11. The 7 plus the 11 are 18, the number the ticket owns.

#### Probes

Each probe ran through `just python-double` on the tree at
`091f0e568e1b2fcba2af33a028fa79123c57c882` plus the uncommitted PD03 files, then was
reverted. `probe.sh` copies the file first, and every file below ends with a matching
SHA-256 before and after. Probes 01 to 18 ran when the suite held 207 tests, and
probe 19 at 208. The final suite holds 209 (section 5). Every exit status is 1. All
files are in `ai_tmp/pd/PD03/`.

| File | Breaks | Key output line | Exit |
| --- | --- | --- | --- |
| `probe-01-ack-no-extension.txt` | `AckMarksRecord` does not extend the lease | `FAILED (failures=4)` | 1 |
| `probe-02-nack-repeat-restamps.txt` | `NackRepeats` re-stamps `nacked_at` | `FAILED (failures=4)` | 1 |
| `probe-03-record-checked-before-lease.txt` | The mark refusal checks the record before the lease | `FAILED (failures=2, errors=11)` | 1 |
| `probe-04-fencing-ignores-status.txt` | `_acting_lease` ignores the lease's status | `FAILED (failures=3, errors=3)` | 1 |
| `probe-05-fencing-ignores-consumer.txt` | `_acting_lease` ignores the consumer | `FAILED (failures=6, errors=2)` | 1 |
| `probe-06-renew-ignores-status.txt` | Renew's guard ignores the lease's status | `FAILED (failures=5)` | 1 |
| `probe-07-renew-stacks.txt` | Renewals stack | `FAILED (failures=5)` | 1 |
| `probe-08-requeue-keeps-old-position.txt` | A requeue keeps the pocket's old position | `FAILED (failures=4)` | 1 |
| `probe-09-acked-requeued-not-consumed.txt` | `ConsumeAckedRecord` requeues the record instead of removing it | `FAILED (failures=5, errors=2)` | 1 |
| `probe-10-nacked-at-from-ended-at.txt` | A dead letter's `nacked_at` is `ended_at` | `FAILED (failures=6)` | 1 |
| `probe-11-dead-letter-expires-from-now.txt` | A dead letter's `expires_at` counts from `now` | `FAILED (failures=3)` | 1 |
| `probe-12-lease-view-by-key.txt` | The lease view sorts by key | `FAILED (failures=3)` | 1 |
| `probe-13-freed-in-tuple-order.txt` | `Freed` goes out in tuple order | `FAILED (failures=2)` | 1 |
| `probe-14-refusal-reads-now.txt` | The mark refusal reads `now` | `FAILED (failures=3)` | 1 |
| `probe-15-check-part-way.txt` | The apply step also checks after the trigger's row, before its events | `FAILED (failures=21)` | 1 |
| `probe-16-idle-takes-a-position.txt` | An idle release allots a position | `FAILED (failures=3)` | 1 |
| `probe-17-revive-keeps-old-position.txt` | A revival keeps the pocket's old position | `FAILED (failures=3)` | 1 |
| `probe-18-ack-keeps-the-count.txt` | `AckMarksRecord` leaves `consecutive_expiries` | `FAILED (failures=2)` | 1 |
| `probe-19-release-skips-consumer-check.txt` | `Queue.release` skips the unknown-consumer check | `FAILED (failures=1, errors=1)` | 1 |

Which tests kill which scenarios, from `summary.md`:

| Scenario | Killed by probe |
| --- | --- |
| S041, S051 | 01; S051 also 07 |
| S084 | 02 (S086 too) |
| S042, S049, S070 | 03 and 05; S068 also 03 |
| S057, S068 | 04 |
| S052 | 06 |
| S035, S053 | 08 |
| S090 | 12 |
| S023 | 09 and 17 |
| S055 | 16 |
| S085 | 14 |
| Every scenario that reaches a release or `Freed` | 15 |

Four limits on this evidence:

- Probe 11 is killed only by `test_lease_end.py`, on a hand-built state. Through `Queue`,
  `now` equals `ended_at`, so no scenario can tell the two apart.
- Probe 13 is killed only by `test_lease_end.py`. With nothing deferred, no scenario can
  tell the order of the `Freed` events (N25).
- Probe 18 is killed only by the hand-built reset test. The count is always 0 before
  PD05.
- Probe 19 is killed only by the untagged N22 release test, since no catalog row covers a
  release by a consumer that never joined.

Probe 15 answers the breakage lens's question about the checker running part-way through
a release. Probes 10 and 11 answer its question about a dead letter's timestamps coming
from the wrong source.

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `091f0e568e1b2fcba2af33a028fa79123c57c882` plus uncommitted work (steps 2 to
  4).
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command
  (`< /dev/null`, `-o last-<lens>.md`, `>|` log). One Claude (sonnet) runner subagent per
  lens. Codex cannot write under read-only, so each findings file is Codex's last
  message and its first line says `Source: last-message`. Codex could not run
  `just check` in its sandbox.
- Triage and apply: one fresh opus subagent. The mutants first ran on a copy outside the
  worktree, then in the tree with the subagent as the only writer.
- Findings: 0 high, 2 medium, and the two are one defect.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | S084 compares one snapshot after both repeats at one instant, so either `AckRepeats` or `NackRepeats` could stop extending the lease. Both mutants passed all 208 tests | Untagged `test_each_repeat_on_its_own_extends_the_lease_from_its_own_time` beside S084 in `test_ack_nack.py`: ack alone at `T0 + 30`, nack alone at `T0 + 50`, the complete snapshot compared after each. S084's tagged test is unchanged |
| B1 | medium | valid, duplicate of C1 | The same defect | The same fix |

- Conformance also judged the S053 port faithful to the catalog row and the
  specification, and N21 to N28 complete.
- Evidence: `ai_tmp/review/PD03-1/apply-mutant-1.txt` (`AckRepeats` does not extend:
  exit 1, 209 tests, `FAILED (failures=2)`, the `k2` subtest, reverted with matching
  SHA-256), `apply-mutant-2.txt` (`NackRepeats` does not extend: exit 1, `FAILED
  (failures=2)`, the `k1` subtest, reverted with matching SHA-256),
  `apply-final.txt` and `verdicts.md`, all in `ai_tmp/review/PD03-1/`.
- Re-run after apply: `just python-double` ran 209 tests, OK; `just lint` exit 0; no
  `__pycache__`. Before the apply it ran 208 tests (`ai_tmp/pd/PD03/final.txt`).
- Second round: none, because no high-severity finding was valid.
- Handed back: nothing outside Files touched.

### 6. Spec notes added

| Id | Note |
| --- | --- |
| N21 | The mark refusals and their order. `LeaseFenced` is reused for all four actions, the Rust's `NotActive` and `NotYours` are not split, and an ack of a queued key in the same pocket is `RecordNotHeld`. |
| N22 | An ack, nack, release or lease read by a consumer that never joined is refused (`impl-choice`, extends N18). |
| N23 | S090's lease view lists the held records smallest position first (`impl-choice`). The view carries no lease id. |
| N24 | An ended lease stays in the state, and a release and a mark answer nothing (S053's "leaves the registry"). |
| N25 | One lease end sends `Freed` smallest held position first. |
| N26 | The Release table's refusal past fencing, `ReleaseRefused`, is unreachable. |
| N27 | An idle pocket keeps its position. |
| N28 | A refused action reads no clock (S085): the lazy clock read. |

### 7. Departures and rulings

#### Maintainer's ruling on S053

Recorded in `ai_tmp/pd/PD03/ruling-S053.txt`. On 2026-11-18, through AskUserQuestion, the
maintainer chose "Accept N24's port (Recommended)".

- The Rust's S053 asserts that the ended lease leaves the registry
  (`snapshot.leases.is_empty()`). A double that follows the specification keeps released
  leases, so that assertion fails against it.
- The maintainer ruled that the empty registry is the Rust's inspection shape, not
  behaviour.
- The double ports the specification's facts: the lease is `released`, `ended_at` is set,
  and the pocket has no active lease. N24 keeps the question open.
- The builder ported the specification's facts and wrote N24 rather than stopping under
  shared rule 1, and flagged the point to the lead as uncertain.

#### Other departures

- The series lands on one branch in one worktree (section 1).
- Step 1 asks for `just initialize` and for `just check` and `just python-double` on the
  untouched worktree. Neither was rerun. The baseline is quoted from PD02 (section 2).
- The refusal builder's signature and the lazy clock read change PD01's machinery
  (section 3). Both were needed so that one table can name three refusals and so that a
  refusal reads no clock.
- 11 of the 18 scenarios could not be red on their own (section 4). Mutant probes kill
  them.
- Probes 11, 13, 18 and 19 are killed only by hand-built or untagged tests (section 4).
- `ReleaseRefused` is unreachable through `Queue` (N26). It is tested on a hand-built
  state only, as PD02's `PullRefused` is.
- Two untagged tests have no catalog row: the N22 release by a consumer that never
  joined, and the C1 repeat test beside S084.
- S040 does not assert the absolute position 1, which rests on N04. It asserts that the
  ack leaves the record's position unchanged, and a comment says why.
- The Rust S053 and the other ported rows were read for their setup, steps and
  assertions only. No behaviour was taken from `crates/libpachinko/src/`.
- `RecordingOutcome`, PD02's test-local class in `test_pull.py`, gained `remove` and
  `allot_position`, so that the hand-built `Freed` and Release tests reuse it.
- Codex cannot write files under read-only, so each findings file is Codex's last
  message (section 5).

### 8. Verification

The Verification block ran at `c5d0ff3897e4ccd25fd60de1286c470ac0a28e43`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
c5d0ff3897e4ccd25fd60de1286c470ac0a28e43
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
.................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 209 tests in 0.083s

OK
…  (the remaining output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- Should an ended lease leave the specification's lease set, as the Rust's inspection
  shows, or stay with its end recorded? The double keeps it and the maintainer accepted
  that port (N24, S053).
- Should the specification name the refusals of an ack, a nack and a release? The double
  reuses `LeaseFenced` for a fenced action and adds `RecordNotHeld` and `MarkConflict`,
  and the Rust splits a lease that is not active from one that is not the consumer's
  (N21).
- Should the specification say that an ack, nack or release requires a joined consumer?
  This extends PD02's N18 (N22, `impl-choice`).
- Is S090's smallest-position-first order of the lease view intended, or is the order
  unspecified? The surface does not state it (N23, `impl-choice`).
- Should the specification fix the order in which one lease end sends `Freed`? With
  nothing deferred, no scenario can tell (N25).
- Is the `ReleaseRefused` branch reachable? If the two release guards are exhaustive past
  fencing, the Release table's second refusal never runs (N26).
- Should a pocket that goes idle keep its position, given that idle pockets are not in
  the pocket order (N27)?
- The lazy clock read (N28) changes PD01's apply step. PD05's `tick()` reads `now` once
  and may want to hand `_apply` a fixed instant. `_Now` takes a clock, so a clock that
  answers one value would do.
- `ending_records` is computed in the snapshot (`inspection.py`'s `_is_ending`), not in
  `lease_end.py` as the ticket words it. The where clause is written in full there.
- The untagged repeat test (C1) and the untagged N22 release test have no catalog row.
  The maintainer may want rows for them.
