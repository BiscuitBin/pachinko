---
id: PD03
title: "Python double: ack, nack, release and the lease's end"
status: todo
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

Written at the end. They record the base and its full commit id, the baseline, any
scenario moved under shared rule 2, the checkpoint's record, the spec notes added, and
Verification quoted under its commit.

## Open points

None yet.
