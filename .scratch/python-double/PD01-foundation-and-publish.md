---
id: PD01
title: "Python double: the foundation, the invariant checker and publish"
status: done
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
  advances it. `PublishIntoNewPocket` takes two: the pocket's, then the record's. In the
  double only `Queue` allots positions, from its apply step (shared rule 4).
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
checker holds all 35 invariants, runs once per outcome from `Queue`'s apply step and has a
positive control for each. The coverage test parses the catalog, the rules and the
invariants and leaves pending exactly what PD02 to PD05 own. Every scenario below is
pinned by a tagged test.

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
| `double/pachinko_double/queue.py` | `Queue`, the facade: one method per trigger a surface provides, plus `tick()` from PD05. Its one apply step runs a trigger's table, the events it chains and then the checker, once per outcome, and it is the only position allotter |
| `double/pachinko_double/rules.py` | The rule-table machinery: `Rule`, a row `(name, guard, effect)`; the table for one trigger with its "no guard holds" answer; dispatch, which evaluates every guard; `GuardOverlap`; and the record of which rules fired |
| `double/pachinko_double/identity.py` | The two joins' tables, the specification's Identity section |
| `double/pachinko_double/publish.py` | The Publish table and the `PocketTouched` table, the specification's Publish section. Landed here with four Publish rows; PD03 adds `ReviveIdlePocket`, PD04 `DeferBehindHeldVersion` and PD05 `ReviveParkedPocket` |
| `double/pachinko_double/pull.py` | The Pull and `Handed` tables, the specification's Pull section. Created in PD02 |
| `double/pachinko_double/settle.py` | The Ack, Nack, Renew and Release tables, the specification's Ack and nack, and Renew and release, sections. Created in PD02 with the Renew table; PD03 adds the rest |
| `double/pachinko_double/lease_end.py` | The lease-end module: the shared end-of-lease steps, `ending_records` and the `Freed` table, the specification's Lease end section. Created in PD03 |
| `double/pachinko_double/expiry.py` | The lapse's clock trigger and the `LeaseLapsed` table, the specification's Lease expiry section. Created in PD05 |
| `double/pachinko_double/decay.py` | The two decays, the specification's Decay section. Created in PD05 |
| `double/pachinko_double/model.py` | The entities and their enums, every field the specification declares |
| `double/pachinko_double/config.py` | `Config`, a frozen dataclass with the five defaults, refusing a value that breaks a config invariant |
| `double/pachinko_double/effects.py` | The `Clock` protocol and `FakeClock`, which counts its reads; the lease-id source protocol and its sequential fake. The double ships no real clock: a reader or a test drives time by setting the fake, and PD05's Verification checks that nothing reads the wall clock |
| `double/pachinko_double/refusals.py` | The refusals, one exception class per kind, under one base class |
| `double/pachinko_double/inspection.py` | The complete immutable snapshot that the tests and the checker read, with its derived sets computed once |
| `double/pachinko_double/invariants.py` | The registry from invariant name to check; `check(snapshot) -> violations`, which returns every violation; and the exception the apply step raises |
| `double/tests/` | `__init__.py`, a fixture helper, and one test module per area: config, identity, publish, rules, invariants, coverage |

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
   `python -B -m unittest discover` over `double/tests`. Land `rules.py`'s machinery
   first, with no specification row yet, because the rule half reads the tables and the
   fired-rule record: `Rule`, the per-trigger table, dispatch, `GuardOverlap` and the
   record. Write `test_coverage.py` with every scenario and rule pending. Its invariant
   half has no pending list, so it lands in step 4 with the checker's registry; until
   then the test asserts only the count of 35. Quote each probe below with its output and
   exit status, then revert it:
   - A `# scenario: S999` tag fails the test, naming the tag.
   - An id removed from `PENDING_SCENARIOS` with no test tagged for it fails.
   - A tag added for an id still pending fails.
   - A table row named `NoSuchRule`, which the specification does not hold, fails,
     naming it.
   - A rule removed from `PENDING_RULES` and registered as a row that never fires fails,
     naming it.
4. **The design and the checker.** Write the model, the effects, `Config`, the refusals,
   the inspection seam and `Queue` with its apply step and its joins. Write the checker
   with all 35 invariants, each a direct reading of the specification's quantifiers,
   `implies` and null cases. Give each its positive control: build a snapshot by hand,
   since most of these states are not reachable yet, and assert that `check(snapshot)`
   names that invariant. A control that would pass because a different invariant fired
   proves nothing, so assert on the name. Quote one scratch break: weaken one check so
   that it never fires, show its control failing, and revert it. Give dispatch its own
   tests, each on a scratch table:
   - a positive control: two guards that both hold raise `GuardOverlap` naming both;
   - one test per answer in shared rule 4's "no guard holds" table: a refusal, nothing
     happens, and a defect.

   **Adversarial review** (checkpoint 1: conformance, breakage and design, before any
   scenario is ported). Questions for this checkpoint:
   - Does the model hold every field the specification declares, with the specification's
     names?
   - Is each check faithful to its invariant? Look at quantifiers over pairs, `a != b`,
     `implies` with a false premise, and null on either side of a comparison.
   - Can a positive control pass while its check is wrong?
   - Does the checker run after the whole chained outcome and not part-way through it?
   - Are the guards pure, and does dispatch evaluate every guard rather than stop at the
     first that holds?
   - Does each table state its "no guard holds" answer?
   - Is the snapshot complete: does `check` need anything the snapshot does not hold?
   - Are the clock and the lease ids injected, and does anything read the wall clock or
     draw randomness?
   - Does the coverage test fail closed? Look at a count that drifts, a regex that matches
     nothing, and a tag with a typo.
5. **Port the scenarios, red first.** For each scenario this ticket owns, write its tagged
   test against the Rust test's setup, steps and assertions. See it fail, or say in the
   hand-back why it cannot be red. Then add the rule that makes it pass as a row of its
   trigger's table: the Publish table for the publish rules, and the joins' tables for
   the joins. PD04 adds `DeferBehindHeldVersion`'s row to the Publish table. Each refusal
   test compares the complete snapshot before and after. Those refusals are a second
   join, the three config refusals, the equal-rank publish and a publish by an unknown
   publisher. S005's runtime refusal of a negative TTL and the unknown-publisher refusal
   are `impl-choice` (shared rule 1).
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
- The dispatch control shows a scratch table with two overlapping guards raising
  `GuardOverlap` naming both, and each "no guard holds" answer has its test.
- The snapshot holds `next_position`, the publishers and the consumers as well as the
  pockets, records, leases and both shed queues.
- Every rule this ticket owns fired at least once, as the coverage test's fired-rule
  check shows.
- Every refusal this ticket lands has a test showing that the complete snapshot is
  unchanged.
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

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `6ebb76b393687e88656ba6594cf1c53584fc5d70`.
- Work commit: `bb9319d59d16d8bcfdc3d126c4690f314a4ae682`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch
  in this one worktree, two commits per ticket. That departs from AGENTS.md's
  one-ticket-per-worktree rule and from step 1's "create the worktree on `branch:`".
  `ticket/pd01-foundation-and-publish` was not created.
- `just initialize` was not run, because the baseline was green without it.

### 2. Baseline

`just check` at `6ebb76b393687e88656ba6594cf1c53584fc5d70` ended
`All checks passed and the worktree is unchanged.` (exit 0).
`.pixi/envs/default/bin/python --version` printed `Python 3.14.10`. The counts were 93
scenarios, 32 rules and 35 invariants.

### 3. Design as settled at checkpoint 1

All modules live under `double/pachinko_double/` and all tests under `double/tests/`.

| Module | Holds |
| --- | --- |
| `queue.py` | `Queue`, `Queue.publish`, `Outcome` and the one apply step `_apply` |
| `rules.py` | `Rule`, `Registry`, dispatch, `GuardOverlap`, the fired record |
| `identity.py` | The `PublisherJoins` and `ConsumerJoins` tables |
| `publish.py` | The `Publish` table (four rows) and the `PocketTouched` table (no rows) |
| `model.py` | The entities and enums; entities are frozen dataclasses |
| `config.py` | `Config`, five defaults, validated in `__post_init__` |
| `effects.py` | `Clock`, `FakeClock` (counts reads), the lease-id source and its fake |
| `refusals.py` | One exception class per refusal, under one base class |
| `inspection.py` | The immutable `Snapshot`, `View` and `ConfigValues` |
| `invariants.py` | `REGISTRY` of 35 checks, `check`, `Violation`, `InvariantsBroken` |
| `__init__.py` | The public names |

Departures from the ticket's design table:

- `publish.py` landed in step 5, not step 4. `identity.py` was first registered with zero
  rows, then took its two rows in step 5.
- `Rule` guards and effects take two seams, `View` (the snapshot plus `now`, read only)
  and `Outcome` (the writes: `allot_position`, `create`, `replace`, `remove`, `emit`,
  `draw_lease_id`, `now`, `config`).
- `Outcome.answer(value)` was added at checkpoint 1 (finding D1). It is a single slot that
  `_apply` returns after the check and the commit. Pull's `Batch` and `NoPocketAvailable`
  use it in PD02.
- `rules.Registry` holds the per-trigger tables; `rules.SPECIFICATION` holds the registry
  and the fired record. Dispatch records only for its own registry's tables, so dispatch
  tests use a private `Registry`.
- `inspection.ConfigValues` is an unvalidated copy of the config in the snapshot, for the
  config-invariant controls.
- `Lease` has an `id` (N05). References between entities are keys (N06).
- Clock-trigger tables are keyed by the trigger parsed from the rule's `when:`
  (`Lease.expires_at`, `SupersededRecord.expires_at`, `DeadLetter.expires_at`). PD05
  registers them under those names.
- Extra test modules beyond the ticket's list: `test_queue`, `test_effects`,
  `test_inspection`, `test_publish`, plus the helper `fixtures.py`.
- The recipe runs `unittest discover -s double/tests -t double`. Without `-t double`
  the start directory is the top level and `load_tests` is ignored (probes 06 and 07).

### 4. Probes

Each probe ran through `just python-double` on the tree at
`6ebb76b393687e88656ba6594cf1c53584fc5d70` plus the uncommitted phase-A files, and was
reverted afterwards. Files are in `ai_tmp/pd/PD01/`. Every exit status is 1.

| File | Breaks | Key output line | Exit |
| --- | --- | --- | --- |
| `probe-01-unknown-scenario-tag.txt` | A `# scenario: S999` tag above a test | `AssertionError: Lists differ: ['S999'] != []` | 1 |
| `probe-02-unpended-untagged.txt` | S011 removed from `PENDING_SCENARIOS`, no test tagged | `AssertionError: Lists differ: ['S011'] != []` | 1 |
| `probe-03-tagged-while-pending.txt` | A `# scenario: S011` tag while S011 is pending | `AssertionError: Lists differ: ['S011'] != []` | 1 |
| `probe-04-unknown-rule-row.txt` | A row named `NoSuchRule` in the registered `PublisherJoins` table | `AssertionError: Lists differ: ['NoSuchRule'] != []` | 1 |
| `probe-05-registered-never-fired.txt` | `PublisherJoins` registered and unpended, no test fires it | `AssertionError: Lists differ: ['PublisherJoins'] != [] : registered rows that never fired` | 1 |
| `probe-06-load-tests-skipped.txt` | Recipe drops `-t double`, sets `PYTHONPATH=double` | `AssertionError: just python-double ran the coverage test without tests.load_tests, so the fired-rule record cannot be trusted; run discovery with -t double` | 1 |
| `probe-07-top-level-dropped.txt` | Recipe drops `-t double` | `ModuleNotFoundError: No module named 'pachinko_double'` | 1 |
| `probe-08-old-python-refused.txt` | Version floor raised to (3, 99) | `python-double: needs Python 3.14 or newer, found 3.14.10` | 1 |
| `probe-09-scratch-break-idle-has-no-records.txt` | `IdleHasNoRecords` weakened so it never fires | `FAILED (failures=2)` (the `IdleHasNoRecords` control and the apply-step test) | 1 |
| `probe-10-checker-names-nothing.txt` | `check()` returns no violation | `FAILED (failures=38)` | 1 |
| `probe-11-typo-tags.txt` | A `#scenario: S011` tag and a control tag `IdleHasNoRecord` | `AssertionError: Lists differ: ['IdleHasNoRecord'] != []` | 1 |
| `probe-12-check-part-way.txt` | The apply step also checks before the trigger's events | `pachinko_double.invariants.InvariantsBroken: invariants broken: IdleHasNoRecords; idle pocket 'p' has 1 records` | 1 |

Probes 09, 10 and 12 are after-the-fact evidence for the checker and the apply step (see
section 8).

### 5. Red runs

Each red run went through `just python-double`, was saved with its exit status, and was
followed by the change that turned it green. In every red run the tag-run check also
fails on the new tags.

| File | Red before | Key output line | Exit |
| --- | --- | --- | --- |
| `red-S003-S004-S005.txt` | `Config` validates | `AssertionError: ConfigRefused not raised`; `FAILED (failures=7)` | 1 |
| `red-S009-S010.txt` | The join rows land, so the row-less tables refuse | `pachinko_double.refusals.PublisherAlreadyJoined: a publisher with id 'p' has joined`; `FAILED (failures=1, errors=3)` | 1 |
| `red-S011-S013.txt` | `PublishIntoNewPocket` lands | `pachinko_double.refusals.EqualRankRefused: 'one'/'k' already stores rank 1`; `FAILED (failures=1, errors=2)` | 1 |
| `red-unknown-publisher.txt` | `Queue.publish` checks the publisher | `AssertionError: UnknownPublisher not raised`; `FAILED (failures=1)` | 1 |
| `red-S014-S016-S017-S018.txt` | Only `PublishIntoNewPocket` exists; S017 already passes | `pachinko_double.refusals.EqualRankRefused: 'pk'/'a' already stores rank 4`; `FAILED (failures=1, errors=3)` | 1 |
| `red-S016-S018.txt` | `PublishNewKey` and `Supersede` landed, `DropStalePublish` not | `pachinko_double.refusals.EqualRankRefused: 'pk'/'a' already stores rank 4`; `FAILED (failures=1, errors=2)` | 1 |

S017 cannot be red for its own reason (see section 8).

### 6. Checkpoint records

#### Checkpoint 1: conformance, breakage and design (before any scenario was ported)

- Ran at: `6ebb76b393687e88656ba6594cf1c53584fc5d70` plus uncommitted work.
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command
  (`< /dev/null`, `-o last-<lens>.md`, `>|` log). One Claude (sonnet) runner subagent per
  lens. Codex cannot write under read-only (`ai_tmp/pd/codex-smoke.txt`), so each
  findings file is Codex's last message, and its first line says `Source: last-message`.
- Triage: two fresh opus subagents (coverage findings; D1), then applied one writer at a
  time, coverage first, then D1.
- Findings: 0 high, 5 medium (breakage B1; conformance C1 and C2; design D1 and D2).

| Id | Verdict | Reason | Applied |
| --- | --- | --- | --- |
| C1 | valid | a tag counted on any `def test` text, collected or not | Fix 1: `load_tests` records passing tests' def sites; a tag counts only if its test ran and passed |
| B1 | valid (duplicate of C1) | same | Fix 1 |
| D2 | valid (duplicate of C1) | a plain-class control drops four controls without failing coverage | Fix 1 |
| C2 | valid | rule coverage by name only; no check that a row sits in its `when:` trigger's table | Fix 2: trigger parsed from `when:`; a misfiled row and an unknown-trigger table fail |
| D1 | valid | the apply step had no channel for an operation's answer | `Outcome.answer(value)`, returned by `_apply` after check and commit |

- Evidence: six mutants (`ai_tmp/review/PD01-1/apply-coverage-mutant-1.txt` to
  `-6.txt`), each exit 1 naming its target, reverted with SHA-256 confirmation; D1 red
  run `ai_tmp/review/PD01-1/apply-d1-red.txt` (5 tests, 4 errors, exit 1).
- Lead's decisions during triage: clock-trigger tables keyed by the trigger parsed from
  `when:`; a tag above a decorated test fails closed.
- Applied: Fix 1, Fix 2 and D1, as above.
- Handed back (outside Files touched): PD02's acceptance line says the batch is
  "asserted through the inspection seam"; Batch is Pull's answer, so PD02 rewords its own
  line (`ai_tmp/pd/carry-forward.md`).
- Re-run: `just python-double` ran 115 tests, OK; `just lint` exit 0; no `__pycache__`.
- Second round: none, because no high-severity finding was valid.

#### Checkpoint 2: conformance and breakage (before the final commit)

- Ran at: `6ebb76b393687e88656ba6594cf1c53584fc5d70` plus uncommitted work (steps 2 to
  7).
- Harness and model: Codex `gpt-6-astra`, read-only, the corrected command; one sonnet
  runner subagent per lens; findings are Codex's last message (`Source: last-message`).
  Codex's own `just check` and `just check-docs` attempts failed on sandbox write denial;
  the tree was confirmed unchanged afterwards.
- Triage and apply: one fresh opus subagent, mutations on copies outside the worktree,
  then fixes applied as the only writer.
- Findings: 0 high, 2 medium, 2 low.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| B1 | medium | valid | an untagged class losing its `TestCase` base drops 6 tests; run still OK (124) | `tests/__init__` records `COLLECTED`; a new check walks every `def test*` by `ast` and requires collected, and passed outside `test_coverage.py` |
| C1 | medium | valid (duplicate of B1) | same, `UnknownPublisherTest`, 129 OK | same fix; mutants 1 to 4 each exit 1 |
| C2 | low | valid | S011, S014 and S016 assert absolute positions that rest on N04's initial `next_position` 0 | `# impl-choice (spec note N04)` above the three tags; N04 retitled `impl-choice` and names them |
| C3 | low | valid (a gap, not a code defect) | the three config refusals cannot compare a snapshot, because a refused `Config` never becomes a queue | `test_config` docstring and N09 say so; recorded as a departure in section 8 |

- Evidence: `ai_tmp/review/PD01-2/apply-mutant-1.txt` to `-4.txt` and `apply-final.txt`.
- Residual gaps named by triage: a test renamed off the `test` prefix is invisible to
  unittest and to the new check; a decorator-skipped test inside `test_coverage.py` is
  checked only for collection.
- Re-run after apply: `just python-double` 131 tests OK, exit 0; `just lint` exit 0; no
  `__pycache__`.
- Second round: none, because no high-severity finding.
- Handed back: nothing outside Files touched.

### 7. Spec notes added

| Id | Note |
| --- | --- |
| N01 | Timestamps and durations are whole milliseconds. |
| N02 | A payload is `bytes`. |
| N03 | A rank is an unbounded integer. |
| N04 | The queue's first position is 0 (S011, S014 and S016 are `impl-choice`). |
| N05 | A lease has an id, which the specification does not declare. |
| N06 | A reference to an entity is that entity's key. |
| N07 | `a != b` in an invariant compares entities, not their fields. |
| N08 | A field declared `when status = ...` is the entity's shape, not an invariant. |
| N09 | A config that breaks a config invariant is refused when it is made (S005 is `impl-choice`). |
| N10 | A publish by a publisher that never joined is refused (`impl-choice`). |
| N11 | The equal-rank publish is the Publish table's refusal, `EqualRankRefused`. |
| N12 | A join with an id already taken is refused. |

### 8. Departures for the maintainer

- The series lands on one branch in one worktree, two commits per ticket (section 1).
- S003 to S005's config refusals compare no snapshot, which departs from step 5 and the
  acceptance criterion. No queue exists before the refusal (checkpoint 2, C3; N09).
- The red-first evidence for the checker and the apply step came after the fact. Probes
  09, 10 and 12 break the finished code; the queue, inspection and invariants modules
  were written with their tests (`summary-A.md`, doubt 3).
- S017 could not go red on its own. Its assertion is the Publish table's no-guard
  refusal, which landed with the table before any row. Before the table existed it could
  fail only on a missing `Queue.publish`, and before `PublishIntoNewPocket` only on its
  setup publish. In `red-S014-S016-S017-S018.txt` it passes.
- The builder once used the system `python3` as a text editor on `test_rules.py`, and
  ran no test through it.
- The builder could not write to `ai_tmp/` through its Write tool. The lead saved
  `summary-A.md`, and `summary-B.md` went through a heredoc.
- Codex cannot write files under read-only, so every findings file is Codex's last
  message (section 6).

### 9. Verification

The Verification block ran at `bb9319d59d16d8bcfdc3d126c4690f314a4ae682`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
bb9319d59d16d8bcfdc3d126c4690f314a4ae682
Python 3.14.10
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
...................................................................................................................................
----------------------------------------------------------------------
Ran 131 tests in 0.024s

OK
…  (just check-docs and just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- The paths `double/`, `double/pachinko_double/` and `double/SPEC-NOTES.md` are this
  series' proposal. The maintainer may rename them before PD01 starts. A rename applies
  to all six tickets.
- `AGENTS.md`'s project map does not list `double/`. Adding it changes the agent contract
  (`writing-for-agents`, `just check-agents`), so it is the maintainer's call, here or in
  PD06.
- PD02's acceptance wording about the batch: its line says the batch is "asserted
  through the inspection seam", but Batch and `NoPocketAvailable` are Pull's answer, not
  state. PD02 rewords its own acceptance line and says so in its hand-back
  (`ai_tmp/pd/carry-forward.md`).
- Residual gaps from checkpoint 2: a test renamed off the `test` prefix is invisible to
  unittest and to the new check, and a decorator-skipped test inside `test_coverage.py`
  is checked only for collection.
- `_apply`'s `event_tables` keyword is a test-only seam. `test_queue` calls the private
  `_apply`, and `Queue` reads `outcome._state`. The maintainer may prefer a public seam
  (`summary-A.md`, doubt 1).
- A lease id drawn inside an outcome that is then refused is not given back: the fake
  source's counter advances while the snapshot stays unchanged. No PD01 rule draws one;
  PD02's pull is the first, so PD02 decides whether a refused pull may consume an id
  (`summary-A.md`, doubt 4). `docs/project/repository-map.md` now lists `double/`, so
  only `AGENTS.md`'s project map is still without it.
