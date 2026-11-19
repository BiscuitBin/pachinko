---
id: PD02
title: "Python double: pull and the lease"
status: done
depends_on: [PD01]
parallel_with: []
branch: ticket/pd02-pull-and-the-lease
estimated_size: M
---

# PD02: Python double: pull and the lease

## Context

PD01 landed the Python double's foundation under `double/`: the entity model, `Config`,
the joins, publish, the inspection seam, the full invariant checker and the coverage test
with its pending lists. Read PD01's hand-back for the design as settled, and read
`double/SPEC-NOTES.md` before adding to it. `.scratch/python-double/README.md` is the
series' index.

This ticket lands the consumer's first half: a pull that claims the front pocket, pulls
within a lease, batches that are oldest-first and capped, `NoPocketAvailable`, the held
set and renew. The specification facts it rests on, all from
`docs/specs/pachinko.allium` (the Pull, and Renew and release, sections):

- `oldest(records, r => r.position, n)` is the `n` queued records with the smallest
  position, in that order. It returns all of them when `n` exceeds their count, and none
  when `n` is not positive. The batch size is `min(limit, config.max_pull_batch)`.
- `PullClaimsFrontPocket` needs a consumer with no active lease and at least one queued
  pocket. It takes the queued pocket with the smallest position, creates a lease with
  `granted_at = now` and `expires_at = now + lease_ttl`, and leases the pocket. It does so
  even when the batch is empty, as S031's zero limit shows.
- `PullWithinLease` extends the consumer's active lease to `now + lease_ttl`, whatever
  the batch holds.
- `HoldRecord` answers each `Handed`. It sets the record to held, `held_rank` and
  `held_payload` from the record, `mark` to unsettled, and both `nacked_at` and
  `deferred_position` to null.
- `PullFindsNoPocket` answers a consumer with no lease when no pocket is queued. Its
  outcome is `NoPocketAvailable`, an answer, not a refusal.
- `Renew` requires the lease to be active and the consumer's, and sets
  `expires_at = now + lease_ttl`. Its refusals (S052, S070) land in PD03 with fencing, but
  its guard is written in full here.
- A batch names its lease. Lease ids come from PD01's injected lease-id source.

S037, which the series plan first put here, publishes a higher rank of a key while it is
held. It needs `DeferBehindHeldVersion`, so it moved to PD04 under shared rule 2.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/pull.rs`,
`publish.rs`, `identity.rs` and `lease.rs`, found with `rg -n 'scenario: S0NN'`. The
`wire` rows here are S019, S020, S028, S031, S033 and S050; port only their library half.

## Goal

A consumer can pull, be handed the oldest queued records of the front pocket under a new
lease, pull again under that lease, renew it, and be told `NoPocketAvailable` when there
is nothing to claim. The checker holds after each of these, and every scenario below is
pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (5) | `PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord`, `PullFindsNoPocket`, `Renew` |
| Scenarios (18) | S001, S008, S012, S015, S019, S020, S022, S027, S028, S029, S030, S031, S032, S033, S036, S038, S039, S050 |

## Non-goals

- No ack, nack, release, lease end, fencing refusal, deferral, lapse or decay. PD03 to PD05
  land them.
- No change to PD01's design beyond what this ticket's rules need. A change to a shared
  seam is reviewed at this ticket's checkpoint and recorded in the hand-back.
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

  `<id>` is `PD02`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `double/pachinko_double/` | shared, own lines | Pull, the held set and renew: `pull.py`, and `settle.py` with its Renew table |
| `double/tests/` | shared, own lines | The pull and lease tests; the pending lists in `test_coverage.py` |
| `double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/python-double/PD02-pull-and-the-lease.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD02's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD01. Record the base and its full commit id in the
   hand-back. Run `just initialize` there (network, authorised by this step). Run
   `just check` and `just python-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the Pull, and Renew and release,
   sections of `docs/specs/pachinko.allium`, the `OldestFirst`, `ExclusivePocket` and
   `OneVisitPerKeyPerLease` guarantees, and the catalog rows this ticket owns. Confirm
   that each owned scenario needs only pull, renew and what PD01 landed. Move any that
   does not, under shared rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test: the Pull and `Handed` tables in `pull.py`, and the Renew table in `settle.py`,
   which this ticket creates (PD01's design). Points to watch:
   - The batch is computed before `HoldRecord` moves its records, and the checker runs
     only after every `Handed` has been answered.
   - A limit that is zero or negative (S031, S032) yields an empty batch and still claims
     the pocket.
   - A pull by an unknown consumer is refused, and the refusal is `impl-choice`.
   - S019's large ranks: record how the double's unbounded integers relate to the
     specification's `Integer`.
   - S020's payload bytes and keys round-trip unchanged under the representation PD01
     chose.
   Shrink `PENDING_SCENARIOS` and `PENDING_RULES` by exactly what landed.
4. **Spec notes.** Add a note wherever pull or renew forced an interpretation. Candidates
   are a zero-limit pull that grants a lease on a pocket with records but hands none, and
   whether `PullWithinLease`'s `requires: pocket.status = leased` can ever fail while its
   lease is active.
5. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while the batch is not
   oldest-first, while it exceeds `max_pull_batch`, or while a second consumer could be
   handed a leased pocket's record.
6. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. **Stop** before pushing or opening a pull request: both
   need authorisation (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- On exit, `PENDING_SCENARIOS` holds exactly the 64 ids PD03 to PD05 own, and
  `PENDING_RULES` the 21 rules they own.
- Every owned scenario has a tagged test, and each of the five rules has its row in the
  Pull, `Handed` or Renew table, fired in at least one test.
- A pull's batch, its lease's `granted_at` and `expires_at`, and the held fields of each
  handed record are asserted through the inspection seam.
- `double/SPEC-NOTES.md` holds this ticket's notes, including the unknown-consumer
  `impl-choice`.
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
- Base commit: `cfbc4fcfd33657c611fcd6efc9dbf385811ae8d4`, PD01's record commit.
- Work commit: `b8d4613e6ee3b3f274e779a1732c35f728800dfa`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch
  in this one worktree, two commits per ticket, as PD01's hand-back section 1 records.
  That departs from AGENTS.md's one-ticket-per-worktree rule and from step 1's "create
  the worktree on `branch:`". `ticket/pd02-pull-and-the-lease` was not created.
- `just initialize` was not run. The series runs in one worktree, whose PD01 baseline
  was green without it, and PD02's baseline is quoted from PD01.

### 2. Baseline

Quoted from PD01, not rerun. The tree under `double/` and the gate are the ones PD01
left (`ai_tmp/pd/PD02/baseline.txt`).

- `just python-double` at `bb9319d59d16d8bcfdc3d126c4690f314a4ae682` printed
  `Ran 131 tests` and `OK` (PD01's Verification).
- `just check` on the tree committed as `cfbc4fcfd33657c611fcd6efc9dbf385811ae8d4`
  ended `All checks passed and the worktree is unchanged.` (exit 0). It ran just before
  that commit, which changed only the two ticket files.

### 3. What landed

Rows per table. Each row fires in at least one test, and the coverage test's fired-rule
check passes.

| Table | Rows | When no guard holds |
| --- | --- | --- |
| Pull | `PullWithinLease`, `PullClaimsFrontPocket`, `PullFindsNoPocket` | A refusal, `PullRefused`, unreachable through `Queue` (N14) |
| `Handed` | `HoldRecord` | A defect |
| Renew | `Renew` | A refusal, `LeaseFenced` (N20) |

New files under `double/`:

- `pachinko_double/pull.py`: `Pull`, `Handed`, `Batch`, `NoPocketAvailable`, `oldest`,
  the Pull table and the `Handed` table.
- `pachinko_double/settle.py`: the `Renew` trigger and the Renew table.
- `tests/test_pull.py` and `tests/test_settle.py`.

Changed files under `double/`:

- `pachinko_double/queue.py`: `Handed` is routed in `EVENT_TABLES`. `Queue.pull(
  consumer_id, limit)` returns the answer. `Queue.renew(consumer_id, lease_id)` returns
  None. `_require_consumer` raises `UnknownConsumer` before the outcome starts.
- `pachinko_double/refusals.py`: `PullRefused`, `UnknownConsumer`, `LeaseFenced`.
- `pachinko_double/__init__.py`: exports `pull`, `settle`, `Batch`, `NoPocketAvailable`
  and the three refusals.
- `tests/fixtures.py`: `scenario_setting()`, which also returns the lease-id source
  (`scenario_queue()` now wraps it), `batch()` and `keys()`.
- `tests/test_publish.py`: `PublishThenPullTest` (S012, S015, S019, S020, S022).
- `tests/test_coverage.py`: the pending lists shrank to 64 scenarios and 21 rules.
- `SPEC-NOTES.md`: N13 to N20.

Outside `double/`, only this ticket's hand-back and `status:` and the README's PD02
Status cell are written, as Files touched says.

No scenario moved under shared rule 2. All 18 need only pull, renew and what PD01 landed.

The batch shape:

- `Batch(consumer: str, lease: str, pocket: str, records: tuple[Record, ...])`.
- The first three fields are ids and keys (N05, N06).
- `records` are the `Record` entities read before the pull, oldest first, so they still
  carry status queued. No test asserts that status.
- `Batch` and `Queue.renew` carry no `expires_at`. The Rust's batch and renew do. The
  double's tests read the expiry from the snapshot's lease (N13).
- The lease id is drawn only in `PullClaimsFrontPocket`'s effect (N15).

### 4. Red runs and probes

#### Red runs

Each red run went through `just python-double`, was saved with its exit status, and was
followed by the change that turned it green. Files are in `ai_tmp/pd/PD02/`. The tree
for each is `cfbc4fcfd33657c611fcd6efc9dbf385811ae8d4` plus the uncommitted PD02 files.
In every run the tag-run check also fails on the new tags.

| File | Red before | Key output line | Exit |
| --- | --- | --- | --- |
| `red-S027.txt` | The Pull table has no rows | `pachinko_double.refusals.PullRefused: no Pull rule answers a pull by 'c1'`; `FAILED (failures=2, errors=1)` | 1 |
| `red-S027-hold.txt` | The claim row lands, the `Handed` table is empty | `pachinko_double.rules.Defect: no rule of the Handed table answers Handed(pocket='pk', record_key='r0')` | 1 |
| `red-S033.txt` | `PullFindsNoPocket` has no row | `pachinko_double.refusals.PullRefused: no Pull rule answers a pull by 'c1'`; `FAILED (failures=2, errors=1)` | 1 |
| `red-S036.txt` | `PullWithinLease` has no row | `pachinko_double.refusals.PullRefused: no Pull rule answers a pull by 'c1'`; `FAILED (failures=2, errors=1)` | 1 |
| `red-S050.txt` | The Renew table has no rows | `FAILED (failures=2, errors=1)` (the table refuses every renew with `LeaseFenced`) | 1 |
| `red-unknown-consumer-1-import.txt` | The test ran before `UnknownConsumer` existed | `ImportError: cannot import name 'UnknownConsumer' from 'pachinko_double.refusals'`; `FAILED (errors=1)` | 1 |
| `red-unknown-consumer.txt` | `UnknownConsumer` exists, `Queue.pull` does not check | `ValueError: a snapshot holds every entity it references: lease 'lease-1' names consumer 'stranger'`; `FAILED (failures=1, errors=1)` | 1 |
| `red-pull-annotation.txt` | `Queue.pull`'s return annotation named `pull.Batch` inside the class, where `pull` is the method | `AttributeError: 'function' object has no attribute 'Batch'`; `FAILED (failures=1, errors=1)` | 1 |
| `red-renew-unknown-lease.txt` | Scratch break: `settle._lease` falls back to an active lease when no lease has the id | `AssertionError: LeaseFenced not raised`; `FAILED (failures=3)` | 1 |

Notes on the table:

- `red-unknown-consumer-1-import.txt` is weak, since it fails only on the missing class.
  `red-unknown-consumer.txt` replaces it.
- The `pull.Batch` red run was fixed by importing `Batch` and `NoPocketAvailable` by
  name. `PullSignatureTest` is its regression test.
- `red-renew-unknown-lease.txt` is the red run of the untagged `RenewRefusalTest`, which
  the lead asked for after the builder stopped. The break was reverted and its SHA-256
  matched.

#### Scenarios that could not be red on their own

14 of the 18 owned scenarios could not be red for their own reason: all but S027, S033,
S036 and S050. Each was written after the rows it exercises landed, and the publish halves are PD01's rows.
Mutant probes kill them instead. The table below names the killing probe per scenario.
The red runs above cover S027, S033, S036 and S050 directly.

#### Probes

Each probe ran through `just python-double` on the tree at
`cfbc4fcfd33657c611fcd6efc9dbf385811ae8d4` plus the uncommitted PD02 files, then was
reverted. `probe.sh` copies the file first, and every file below ends with a matching
SHA-256 before and after. The probes ran when the suite held 163 tests; the final suite
holds 167 (section 5). Every exit status is 1. All files are in `ai_tmp/pd/PD02/`.

| File | Breaks | Key output line | Exit |
| --- | --- | --- | --- |
| `probe-01-batch-unsorted.txt` | The batch keeps the snapshot's tuple order | `FAILED (failures=7)` | 1 |
| `probe-02-batch-uncapped.txt` | The batch ignores `max_pull_batch` | `FAILED (failures=6)` | 1 |
| `probe-03-front-pocket-first-in-tuple.txt` | The claim takes the first queued pocket in the tuple | `FAILED (failures=4)` | 1 |
| `probe-04-nonpositive-n-sliced.txt` | `oldest` handles zero but slices a negative `n` | `FAILED (failures=4)` | 1 |
| `probe-05-claim-any-pocket.txt` | `free` is every pocket, so a leased pocket can be claimed | `FAILED (failures=3, errors=6)` | 1 |
| `probe-06-within-lease-no-extension.txt` | `PullWithinLease` leaves `expires_at` where it was | `FAILED (failures=3)` | 1 |
| `probe-07-renew-ignores-consumer.txt` | Renew's guard drops `lease.consumer = consumer` | `FAILED (failures=2)` | 1 |
| `probe-08-no-pocket-draws-an-id.txt` | A pull that finds no pocket draws a lease id | `FAILED (failures=3)` | 1 |
| `probe-09-renew-no-extension.txt` | Renew leaves `expires_at` where it was | `FAILED (failures=3)` | 1 |
| `probe-10-empty-batch-claims-nothing.txt` | A pull whose limit is not positive claims no pocket | `FAILED (failures=2, errors=4)` | 1 |
| `probe-11-batch-from-every-record.txt` | The batch is drawn from every record of the pocket, held ones included | `FAILED (failures=3, errors=5)` | 1 |
| `probe-12-batch-newest-first.txt` | The batch takes the largest positions first | `FAILED (failures=20)` | 1 |
| `probe-13-held-payload-altered.txt` | `HoldRecord` holds `record.payload + b"!"` | `FAILED (failures=3, errors=18)` | 1 |

Which tests kill which scenarios:

| Scenario | Killed by probe |
| --- | --- |
| S001, S008, S029 | 02 |
| S028, S030 | 11, and 12 for S028 |
| S031, S032, S022 | 10; S032 also 04 |
| S033, S038, S039 | 05 |
| S012, S015 | 12 |
| S019, S020 | 13 |
| S036 | 06, and its own red run |
| S050 | 09, and its own red run |

Three limits on this evidence:

- Probe 13 shows only that S019 and S020's pull half is live (`HeldCopiesAgree` breaks).
  Ranks, keys and payloads pass through with no encoding step, and their publish rows are
  PD01's, so nothing PD02 lands can make those two red on their own assertions.
- Probes 01 and 03 kill no scenario test. In every state reachable before PD03, tuple
  order, key order and position order agree. Only the direct hand-built tests catch them
  (`OldestTest`, `PullRowsOnHandBuiltStatesTest`). Probe 07 is caught only by
  `RenewGuardTest`.
- "The batch is computed after `HoldRecord`" has no probe. Effects only emit, so the view
  the batch reads is always the pre-hold state. `BatchShapeTest` pins that the batch's
  records equal the pre-pull snapshot's.

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `cfbc4fcfd33657c611fcd6efc9dbf385811ae8d4` plus uncommitted work (steps 2 to
  4).
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command
  (`< /dev/null`, `-o last-<lens>.md`, `>|` log). One Claude (sonnet) runner subagent per
  lens. Codex cannot write under read-only, so each findings file is Codex's last
  message and its first line says `Source: last-message`. Codex could not run
  `just check` in its sandbox.
- Triage and apply: one fresh opus subagent. The mutant first ran on a copy outside the
  worktree, then in the tree with the subagent as the only writer.
- Findings: 0 high, 1 medium, 1 low.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| B1 | medium | valid | No test pinned that `PullWithinLease` renews the lease on an empty batch, though the specification sets `expires_at = now + lease_ttl` unconditionally. The mutant "renew only on a nonempty batch" passed all 165 tests | Untagged `EmptyPullWithinLeaseTest` in `test_pull.py`: a drained pocket, and limits 0, -1 and -(2**63), each advancing `FakeClock` and asserting the new `expires_at` through the snapshot |
| C1 | low | invalid | The single-branch landing is the maintainer's direction, recorded in PD01's section 1 | Nothing; section 1 above restates it |

- Evidence: `ai_tmp/review/PD02-1/apply-mutant-1.txt` (`FAILED (failures=5)`, exit 1,
  reverted with matching SHA-256), `ai_tmp/review/PD02-1/apply-final.txt`
  (`Ran 167 tests`, `OK`, exit 0) and `ai_tmp/review/PD02-1/verdicts.md`.
- Re-run after apply: `just python-double` ran 167 tests, OK; `just lint` exit 0; no
  `__pycache__`. Before the apply it ran 165 tests (`ai_tmp/pd/PD02/final.txt`).
- Second round: none, because no high-severity finding was valid.
- Handed back: nothing outside Files touched.

### 6. Spec notes added

| Id | Note |
| --- | --- |
| N13 | A batch names its lease and pocket by key, and carries no expiry. |
| N14 | The Pull table's refusal, `PullRefused`, is unreachable through `Queue`. |
| N15 | A pull draws a lease id only when it creates a lease. |
| N16 | S012, S015 and S022 assert absolute positions (`impl-choice`, on N04). |
| N17 | S019's large ranks are a 64-bit integer's extremes, not the double's. |
| N18 | A pull or a renew by a consumer that never joined is refused (`impl-choice`). |
| N19 | A pull whose limit is not positive still claims a pocket. |
| N20 | Renew's refusal is `LeaseFenced`, and an unknown lease id meets it. |

### 7. Departures for the maintainer

- The series lands on one branch in one worktree, two commits per ticket (section 1).
- The acceptance line says a pull's batch is asserted "through the inspection seam".
  The batch is Pull's answer, not state, so it is asserted through `Queue.pull`'s return
  value. The lease's `granted_at` and `expires_at` and each handed record's held fields
  are asserted through the snapshot (N13). The ticket was not reworded.
- 14 of the 18 scenarios could not be red on their own (section 4). Mutant probes kill
  them.
- Probes 01 and 03 kill no scenario test, only direct hand-built tests (section 4).
- `PullRefused` is unreachable through `Queue` (N14). Its one test dispatches the
  registered Pull table on a hand-built state that breaks `UnleasedHasNoActiveLease`,
  with a recording stand-in outcome and a private `Registry`, labelled
  `impl-choice (N14)`. It is not a before-and-after snapshot proof through `Queue`.
- The specification names no Renew refusal, so it is `LeaseFenced` (N20). Its one test
  here is the untagged never-issued lease id, added at the lead's direction under shared
  rule 4, and it compares the complete snapshot before and after. The ended-lease and
  wrong-consumer cases stay with PD03's S052 and S070.
- The hand-built row tests use `RecordingOutcome`, a test-local stand-in for `Outcome`.
- `_batch` reads `max_pull_batch` from `outcome.config`, as publish's effects read their
  TTLs, not from `view.state.config`. A hand-built test exposed the two sources.
- S001's Rust test checks only the config values. The port also publishes 101 records and
  checks that a pull caps at 100 and the lease expires at `T0 + 5 min`.
- Codex cannot write files under read-only, so each findings file is Codex's last
  message (section 5).

### 8. Verification

The Verification block ran at `b8d4613e6ee3b3f274e779a1732c35f728800dfa`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
b8d4613e6ee3b3f274e779a1732c35f728800dfa
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
.......................................................................................................................................................................
----------------------------------------------------------------------
Ran 167 tests in 0.066s

OK
…  (just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- Should `Batch` carry the lease's `expires_at`? A consumer needs it to know when to
  renew (N13).
- Are `PullWithinLease`'s and `PullClaimsFrontPocket`'s `requires: pocket.status = ...`
  guards, or restatements of `UnleasedHasNoActiveLease`? If restatements, the Pull
  table's refusal never runs (N14).
- Should the specification say that a pull and a renew require a joined consumer, as N10
  asks for a publish (N18, `impl-choice`)?
- Is a claim that hands nothing intended? A non-positive limit still takes the front
  pocket from every other consumer until its lease ends (N19).
- Should the specification bound a rank, so that S019's `2**63` extremes become the
  boundary for every implementation (N17, as N03)?
- The name `LeaseFenced` for Renew's refusal is the double's. Whether an unknown lease
  is a distinct refusal is left to PD03 (N20).
- `test_queue.py`'s docstring, which is PD01's, still says no specification row chains an
  event. `Handed` now does. It was left, since those lines are not PD02's.
- `RenewRefusalTest` is untagged, because no catalog row covers a never-issued lease id.
  The maintainer may want a catalog row for it.
