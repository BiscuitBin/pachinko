---
id: PD01
title: "Python double: the foundation, the invariant checker and publish"
status: todo
depends_on: []
parallel_with: []
branch: ticket/pd01-foundation-and-publish
estimated_size: L
---

# PD01: Python double: the foundation, the invariant checker and publish

## Context

The maintainer writes Python, not Rust. They want a minimal but correct Python
implementation of the domain `docs/specs/pachinko.allium` defines: one queue of pockets
under exclusive leases, with publish, pull, ack, nack, renew, release, lease end, expiry,
parking and decay. They will read it to judge whether the specification is right. The
HTTP service is out of scope. So, for now, is using the double as a test oracle for the
Rust.

The double lives under `double/`: the package `double/pachinko_double/`, its tests under
`double/tests/`, and the spec-notes file `double/SPEC-NOTES.md`. Those paths are this
series' proposal, and the maintainer may rename them before this ticket starts. It is a
series of six tickets, PD01 to PD06, run one after another. Each one lands the
operations its ownership table names, ports the scenarios it owns from the 93 in the
catalog in `docs/reference/testing.md`, and shrinks the coverage test's pending lists.
`.scratch/python-double/README.md` is the index.

This ticket lays the foundation every later one builds on:

- the package and its design;
- `Config` with its validation;
- joins;
- publish into a new pocket or under a new key, supersede, the stale publish and the
  equal-rank refusal;
- the superseded queue;
- the inspection seam;
- the full invariant checker, all 35 invariants with a positive control each;
- the coverage tests with their pending lists;
- the spec-notes file;
- `just python-double` and its row in `docs/reference/commands.md`.

The specification facts it rests on, all from `docs/specs/pachinko.allium`:

- **Config.** Five fields with defaults: `lease_ttl` 5 minutes, `max_consecutive_expiries`
  3, `max_pull_batch` 100, `superseded_ttl` 12 hours and `dead_letter_ttl` 12 hours. Five
  invariants bound them: `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
  `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`.
- **Entities.** `Queue` holds `next_position`. `Publisher` and `Consumer` hold an id.
  `Pocket` holds a key, a status (queued, leased, idle or parked), a position and
  `consecutive_expiries`. `Record` holds a key, a rank, a payload, a position, and a status
  (queued or held). When held it also holds `held_rank`, `held_payload`, `mark`,
  `nacked_at` and `deferred_position`. `SupersededRecord`, `DeadLetter` and `Lease` are the
  rest. The checker needs every field, so the whole entity model lands here, before any
  rule that sets the held fields or creates a lease.
- **Positions.** Every rule that allots a position takes `queue.next_position` and
  advances it. `PublishIntoNewPocket` takes two: the pocket's, then the record's.
- **Publish.** Five rules share the `Publish` trigger, with disjoint guards. No rule
  covers a rank equal to the stored rank, so that publish is rejected and changes nothing,
  whatever its payload. The stored rank is `record.rank` whatever the record's state. A
  stale publish creates a `SupersededRecord` with reason `stale`, changes no record and
  emits no `PocketTouched`.
- **Supersede's guard.** `record.status = queued or (record.status = held and
  record.deferred_position != null)`. Write the full guard here. Its held branch cannot be
  reached until PD04, whose S045 exercises it.
- **The invariants.** Fifteen sit inside entities (`LeasedHasOneActiveLease` to
  `EndedAfterGrant`) and twenty at top level (`ExpiryLimitPositive` to
  `ArrivalPositionsBelowNext`).

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/identity.rs`
and `crates/libpachinko/tests/queue/publish.rs`, found with `rg -n 'scenario: S0NN'`,
plus S005's doctest in `crates/libpachinko/src/config.rs`.

## Goal

`just python-double` runs the double's tests on pixi's Python 3.14 and exits 0. The
package holds the whole entity model, `Config`, the joins and the publish rules below. The
checker holds all 35 invariants, runs after every operation and has a positive control for
each. The coverage test parses the catalog, the rules and the invariants and leaves
pending exactly what PD02 to PD05 own. Every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (6) | `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`, `DropStalePublish` |
| Invariants (35) | `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `IdleHasNoRecords`, `ExpiriesWithinLimit`, `HeldVersionNotNewer`, `NackedHasItsTime`, `OnlyNackedHasNackTime`, `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `HeldCopiesAgree`, `ShedBeforeExpiry`, `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant`, `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative`, `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `OutstandingOnlyWhileLeased`, `MarkedPocketHasNoExpiries`, `UniquePocketPosition`, `UniqueRecordPosition`, `PocketPositionsBelowNext`, `RecordPositionsBelowNext`, `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` |
| Scenarios (11) | S003, S004, S005, S009, S010, S011, S013, S014, S016, S017, S018 |

## Non-goals

- No HTTP, no wire format and no use of the double as an oracle for the Rust.
- No change under `crates/`, `docs/specs/`, `docs/decisions/`, `loadgen/` or `fuzz/`. A
  doubt about the specification becomes a spec note, never an edit.
- No pull, lease, ack, nack, renew, release, deferral, expiry or decay behaviour. The
  entity model holds their fields so that the checker is complete, and PD02 to PD05 land
  the rules.
- No third-party package and no pytest.
- No place in `just check`'s recipe list.

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

  `<id>` is `PD01`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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

## The design to propose

The design checkpoint (step 4) reviews this layout and may change it. Record what changed
in the hand-back.

| Path | Holds |
| --- | --- |
| `double/pachinko_double/__init__.py` | The public names a test or a reader imports |
| `double/pachinko_double/config.py` | `Config`, a frozen dataclass with the five defaults, refusing a value that breaks a config invariant |
| `double/pachinko_double/effects.py` | The `Clock` protocol and `FakeClock`, which counts its reads; the lease-id source protocol and its sequential fake. The double ships no real clock: a reader or a test drives time by setting the fake, and PD05's Verification checks that nothing reads the wall clock |
| `double/pachinko_double/model.py` | The entities and their enums, every field the specification declares |
| `double/pachinko_double/queue.py` | `Queue`, the facade: one method per trigger a surface provides, each running its chained outcome and then the checker |
| `double/pachinko_double/refusals.py` | The refusals, one exception class per kind, under one base class |
| `double/pachinko_double/invariants.py` | The registry from invariant name to check, and `check`, which returns every violation, and the exception the queue raises |
| `double/pachinko_double/inspection.py` | The immutable snapshot the tests read |
| `double/tests/` | `__init__.py`, a fixture helper, and one test module per area: config, identity, publish, invariants, coverage |

Decide and record in the spec notes:

- How a timestamp and a duration are represented: integer milliseconds, or `datetime` and
  `timedelta`.
- Whether a payload is `bytes`, as the specification's vocabulary says the implementation
  stores it, or `str`, as Allium 3 types it.
- How a rank is bounded. Python integers are unbounded, and the specification's `Integer`
  states no bound.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/pachinko_double/` | new | The package, laid out as the design checkpoint settles |
| `double/tests/` | new | The test package, including `test_coverage.py` |
| `double/SPEC-NOTES.md` | new | A header saying what the file is for, then this ticket's notes |
| `Justfile` | shared, own lines | The `python-double` recipe, beside `diagrams`, with a comment saying it is outside `just check` |
| `docs/reference/commands.md` | shared, own lines | A `just python-double` row in the Develop table |
| `docs/project/repository-map.md` | shared, own lines | A row for `double/`, the page that owns the repository's layout |
| `.editorconfig` | shared, own lines | A `[*.py]` entry, only if editorconfig-checker objects |
| `.scratch/python-double/PD01-foundation-and-publish.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD01's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, and record the base and its full commit id in the hand-back. Run
   `just initialize` there (network, authorised by this step). Run `just check` on the
   untouched worktree and quote its last line, headed by its commit, as the baseline.
   Quote `.pixi/envs/default/bin/python --version`, which must print 3.14.
2. **The plan.** Use the `plan-change` skill. Read `docs/specs/pachinko.allium` in full,
   and the catalog rows this ticket owns. Confirm the three counts:
   `rg -c '^\| S[0-9]{3} ' docs/reference/testing.md` prints 93,
   `rg -c '^rule ' docs/specs/pachinko.allium` prints 32, and
   `rg -c '^\s*invariant ' docs/specs/pachinko.allium` prints 35. If one has moved,
   **stop** and put it to the maintainer.
3. **The recipe and the coverage test, red first.** Add the recipe. It refuses a Python
   older than 3.14, printing the version it found, then runs
   `python -B -m unittest discover` over `double/tests`. Write `test_coverage.py` with
   every scenario and rule pending. Its invariant half has no pending list, so it lands in
   step 4 with the checker's registry; until then the test asserts only the count of 35.
   Quote each probe below with its output and exit status, then revert it:
   - A `# scenario: S999` tag fails the test, naming the tag.
   - An id removed from `PENDING_SCENARIOS` with no test tagged for it fails.
   - A tag added for an id still pending fails.
   - A `# rule: NoSuchRule` comment fails.
4. **The design and the checker.** Write the model, the effects, `Config`, the refusals,
   the inspection seam and `Queue` with its joins. Write the checker with all 35
   invariants, each a direct reading of the specification's quantifiers, `implies` and
   null cases. Give each its positive control: build or corrupt the state directly,
   since most of these states are not reachable yet, and assert that `check` names that
   invariant. A control that would pass because a different invariant fired proves
   nothing, so assert on the name. Quote one scratch break: weaken one check so that it
   never fires, show its control failing, and revert it.

   **Adversarial review** (checkpoint 1: conformance, breakage and design, before any
   scenario is ported). Questions for this checkpoint:
   - Does the model hold every field the specification declares, with the specification's
     names?
   - Is each check faithful to its invariant? Look at quantifiers over pairs, `a != b`,
     `implies` with a false premise, and null on either side of a comparison.
   - Can a positive control pass while its check is wrong?
   - Does the checker run after the whole chained outcome and not part-way through it?
   - Are the clock and the lease ids injected, and does anything read the wall clock or
     draw randomness?
   - Does the coverage test fail closed? Look at a count that drifts, a regex that matches
     nothing, and a tag with a typo.
5. **Port the scenarios, red first.** For each scenario this ticket owns, write its tagged
   test against the Rust test's setup, steps and assertions. See it fail, or say in the
   hand-back why it cannot be red. Then land the rule that makes it pass, with a
   `# rule: <Name>` comment at the code that applies it. Each refusal test compares the
   snapshot before and after. Those refusals are a second join, the three config refusals,
   the equal-rank publish and a publish by an unknown publisher. S005's runtime refusal of
   a negative TTL and the unknown-publisher refusal are `impl-choice` (shared rule 1).
   Shrink `PENDING_SCENARIOS` and `PENDING_RULES` by exactly what landed.
6. **Spec notes.** Write `double/SPEC-NOTES.md`: a header, then this ticket's notes. The
   notes cover the three representation choices in "The design to propose", every
   `impl-choice` row, and anything the publish rules left to interpretation. Examples are
   which refusal an equal rank is and whether a publish names a publisher that must have
   joined, since the trigger takes a `Publisher` entity.
7. **The handbook.** Use the `review-docs` skill. Add the `just python-double` row to
   `docs/reference/commands.md` and the `double/` row to `docs/project/repository-map.md`.
   Run `just check-docs`.
8. **Adversarial review** (checkpoint 2: conformance and breakage, before the final
   commit). The breakage lens asks whether `just python-double` could exit 0 while a test
   failed or was never collected. Check a module that fails to import, a test class
   `discover` skips, and a `subTest` that swallows a failure.
9. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. **Stop** before pushing or opening a pull request: both
   need authorisation (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- `test_coverage.py` asserts the counts 93, 32 and 35. On exit, `PENDING_SCENARIOS` holds
  exactly the 82 ids PD02 to PD05 own, and `PENDING_RULES` the 26 rules they own. No
  invariant is pending.
- All 35 invariants are in the checker's registry, and each has a passing positive control
  tagged `# invariant-control: <Name>`.
- The step 3 and step 4 probes are quoted with their output and exit status.
- Every refusal this ticket lands has a test showing that the snapshot is unchanged.
- `FakeClock` counts its reads, and the lease-id source has a deterministic fake, though
  no rule here draws a lease id yet.
- `double/SPEC-NOTES.md` holds a note for each `impl-choice` row and each representation
  choice.
- `just python-double` leaves no `__pycache__/` under `double/`.
- Both checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails.

```sh
set -eu
git rev-parse HEAD
.pixi/envs/default/bin/python --version
just python-double
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

Written at the end. They record the base and its full commit id, the baseline, the design
as settled at checkpoint 1, each checkpoint's record, the probes, the spec notes added,
and Verification quoted under its commit.

## Open points

- The paths `double/`, `double/pachinko_double/` and `double/SPEC-NOTES.md` are this
  series' proposal. The maintainer may rename them before PD01 starts. A rename applies
  to all six tickets.
- `AGENTS.md`'s project map does not list `double/`. Adding it changes the agent contract
  (`writing-for-agents`, `just check-agents`), so it is the maintainer's call, here or in
  PD06.
