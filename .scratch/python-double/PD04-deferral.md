---
id: PD04
title: "Python double: deferral behind the held version"
status: todo
depends_on: [PD03]
parallel_with: []
branch: ticket/pd04-deferral
estimated_size: M
---

# PD04: Python double: deferral behind the held version

## Context

PD01 to PD03 landed the Python double under `double/`: publish, pull, renew, marks,
fencing, release, the three lease-end rows with nothing deferred, dead letters and the
idle revival. Read their hand-backs and `double/SPEC-NOTES.md` first.
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
   - The specification's rules, `^rule (\w+) \{`, are each covered by a `# rule: <Name>`
     comment at the code that applies the rule, under `double/pachinko_double/`.
   - The specification's invariants, `^\s*invariant (\w+) \{`, are each covered by an
     entry in the checker's registry and by a positive control tagged
     `# invariant-control: <Name>`. Fifteen of the thirty-five sit indented inside an
     entity, so a pattern anchored at column 0 finds only twenty.
   - It asserts the three counts it parsed, 93, 32 and 35, so a pattern that silently
     matches less fails. A specification change that moves a count updates the constant
     and adds a spec note. A tag that names an id or a name the sources do not hold fails.
   - `PENDING_SCENARIOS` and `PENDING_RULES` hold what is not covered yet. An entry that is
     both pending and covered fails, so each ticket shrinks the lists by exactly what it
     lands. PD06 deletes both lists.
4. **Design constraints.** PD01 proposes the class layout and reviews it. These are fixed:
   - The clock and lease ids are injected: a `Clock` protocol with a fake whose time a test
     sets and whose reads it counts, and a lease-id source with a deterministic fake. This
     follows the house rule that effects sit behind seams (decisions 0003 and 0008). The
     double reads no wall clock and draws no randomness.
   - A refusal changes nothing, and a test proves it for each refusal by comparing the
     inspection seam's snapshot before and after.
   - The invariant checker runs after every public operation, after the whole chained
     outcome (`Handed`, `Freed`, `PocketTouched`), because the specification says the
     invariants hold after the whole of it. It raises naming each invariant that fails.
     Each of the 35 invariants has a positive control: a test that corrupts the state
     directly and shows that the checker names that invariant.
   - Tests read state through an inspection seam: each pocket's status, position and
     consecutive expiries; each record's state, positions and held fields; the leases; and
     the superseded and dead-letter queues.
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
| `double/pachinko_double/` | shared, own lines | Deferral, the deferred supersede and the three deferred lease-end rows |
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
   it fail, then land the rule with its `# rule: <Name>` comment. Points to watch:
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
- Every owned scenario has a tagged test, and each of the four rules has its `# rule:`
  comment.
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

Written at the end. They record the base and its full commit id, the baseline, any
scenario moved under shared rule 2, the checkpoint's record, the spec notes added, and
Verification quoted under its commit.

## Open points

None yet.
