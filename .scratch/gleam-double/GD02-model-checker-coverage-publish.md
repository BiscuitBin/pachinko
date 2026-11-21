---
id: GD02
title: "Gleam double: the model, the invariant checker, coverage and publish"
status: done
depends_on: [GD01]
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD02: Gleam double: the model, the invariant checker, coverage and publish

## Context

GD01 proved the Gleam toolchain and landed the Gleam double's skeleton under
`gleam-double/`: the rule-table machinery, the apply step with its seam for `now` and
lease ids, the owner actor, the fakes, the test runner, `Config` and the two joins. Read
GD01's hand-back for the design as settled, and read `gleam-double/SPEC-NOTES.md` before
adding to it. `.scratch/gleam-double/README.md` is the series' index.

This ticket lands everything a publish needs and everything a later ticket checks itself
against:

- the whole entity model;
- the complete inspection snapshot;
- the invariant checker, all 35 invariants with a positive control each, run by the apply
  step after every outcome;
- the coverage check with its pending lists;
- position allotment in the apply step;
- publish into a new pocket or under a new key, supersede, the stale publish and the
  equal-rank refusal;
- the superseded queue.

The specification facts it rests on, all from `docs/specs/pachinko.allium`:

- **Entities.** `Pocket` holds a key, a status (queued, leased, idle or parked), a
  position and `consecutive_expiries`. `Record` holds a key, a rank, a payload, a
  position, and a status (queued or held). When held it also holds `held_rank`,
  `held_payload`, `mark`, `nacked_at` and `deferred_position`. `SupersededRecord`,
  `DeadLetter` and `Lease` are the rest. The checker needs every field, so the whole
  entity model lands here, before any rule that sets the held fields or creates a lease.
- **Positions.** Every rule that allots a position takes `queue.next_position` and
  advances it. `PublishIntoNewPocket` takes two: the pocket's, then the record's. In the
  double only the apply step allots positions (shared rule 4). The first position is 0
  (N04).
- **Publish.** Five rules share the `Publish` trigger, with disjoint guards. No rule
  covers a rank equal to the stored rank, so that publish is rejected and changes nothing,
  whatever its payload. The stored rank is `record.rank` whatever the record's state. A
  stale publish creates a `SupersededRecord` with reason `stale`, changes no record and
  emits no `PocketTouched`.
- **Supersede's guard.** `record.status = queued or (record.status = held and
  record.deferred_position != null)`. Write the full guard here. Its held branch cannot be
  reached until GD05, whose S045 exercises it.
- **`PocketTouched`.** A publish that changes a record emits it. Its table has no row
  until GD04, and its answer when no guard holds is "nothing happens".
- **The invariants.** Fifteen sit inside entities (`LeasedHasOneActiveLease` to
  `EndedAfterGrant`) and twenty at top level (`ExpiryLimitPositive` to
  `ArrivalPositionsBelowNext`).

What the maintainer has already ruled, and what the Python double already read:

- **A publish answers nothing** (the ruling on N32). Where a Rust test checks a publish
  for an outcome, the port asserts the state through the snapshot and that the publish
  answered nothing. It does not port the Rust's outcome.
- **A publish by a publisher that never joined is refused** before any rule runs, and its
  test is `impl-choice` (N10).
- **The equal-rank publish is the Publish table's refusal**, `EqualRankRefused` (N11).
- S011, S014 and S016 assert absolute positions, which hold only because the first
  position is 0. They are `impl-choice` (N04).

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/publish.rs`,
found with `rg -n 'scenario: S0NN'`.

## Goal

A publish through the owner creates a pocket and a record, adds a record under a new key,
supersedes a queued record, or sheds a stale one, and an equal-rank publish is refused and
changes nothing. The package holds the whole entity model, and the snapshot is complete.
The checker holds all 35 invariants, runs once per outcome from the apply step and has a
positive control for each. The coverage check parses the catalog, the rules and the
invariants and leaves pending exactly what GD03 to GD06 own. Every scenario below is
pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (4) | `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`, `DropStalePublish` |
| Invariants (35) | `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `IdleHasNoRecords`, `ExpiriesWithinLimit`, `HeldVersionNotNewer`, `NackedHasItsTime`, `OnlyNackedHasNackTime`, `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `HeldCopiesAgree`, `ShedBeforeExpiry`, `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant`, `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative`, `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `OutstandingOnlyWhileLeased`, `MarkedPocketHasNoExpiries`, `UniquePocketPosition`, `UniqueRecordPosition`, `PocketPositionsBelowNext`, `RecordPositionsBelowNext`, `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` |
| Scenarios (6) | S011, S013, S014, S016, S017, S018 |

## Non-goals

- No pull, lease, ack, nack, renew, release, deferral, expiry or decay behaviour. The
  entity model holds their fields so that the checker is complete, and GD03 to GD06 land
  the rules.
- No change to GD01's design beyond what this ticket's rules need. A change to a shared
  seam is reviewed at this ticket's design checkpoint and recorded in the hand-back.
- No new package and no change to the pins.
- No change under `crates/`, `docs/specs/`, `docs/decisions/`, `double/`, `loadgen/` or
  `fuzz/`.

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

  `<id>` is `GD02`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `gleam-double/src/` | shared, own lines | The rest of the model, the complete snapshot, `invariants.gleam`, position allotment and the checker call in the apply step, `publish.gleam` with the Publish and `PocketTouched` tables, and the owner's `Publish` message |
| `gleam-double/test/` | shared, own lines | `coverage.gleam` with the pending lists, the invariant controls, the publish tests, and the runner's call to the coverage check |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `docs/reference/commands.md` | shared, own lines | The `just gleam-double` row, if it should now name the coverage check |
| `.scratch/gleam-double/GD02-model-checker-coverage-publish.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD02's Status cell |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD01, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read `docs/specs/pachinko.allium` in full,
   N01 to N12 and N32 in `double/SPEC-NOTES.md`, and the catalog rows this ticket owns.
   Confirm the three counts as GD01's step 2 does. If one has moved, **stop** and put it
   to the maintainer.
3. **The coverage check, red first.** Write `coverage.gleam` and call it from the
   runner's `main` after the last test. Start with every scenario and rule pending but
   the five scenarios and two rules GD01 landed, which must now be covered by GD01's
   tags and fired rows. Its invariant half has no pending list, so it lands in step 4
   with the checker's registry; until then the check asserts only the count of 35. Quote
   each probe below with its output and exit status, then revert it:
   - a `Scenario("S999")` tag fails the run, naming the tag;
   - an id removed from `pending_scenarios` with no test tagged for it fails;
   - a tag added for an id still pending fails;
   - a table row named `NoSuchRule`, which the specification does not hold, fails,
     naming it;
   - a rule removed from `pending_rules` and registered as a row that never fires fails,
     naming it;
   - a count constant changed to 92 fails, which shows the check compares what it parsed;
   - a tagged test that fails does not credit its tag or the rules it fired.
4. **The model, the snapshot and the checker.** Write the rest of the model and complete
   the snapshot. Write the checker with all 35 invariants, each a direct reading of the
   specification's quantifiers, `implies` and null cases. Give each its positive control:
   build a snapshot by hand, since most of these states are not reachable yet, and assert
   that `check(snapshot)` names that invariant. A control that would pass because a
   different invariant fired proves nothing, so assert on the name. Make the checker the
   last step of the apply step, and show on a scratch table that an effect which breaks
   an invariant returns a broken invariant naming it and leaves no new state. Quote one
   scratch break: weaken one check so that it never fires, show its control failing, and
   revert it.

   **Adversarial review** (checkpoint 1: conformance, breakage and design, before any
   publish scenario is ported). Questions for this checkpoint:
   - Does the model hold every field the specification declares, with the specification's
     names?
   - Is each check faithful to its invariant? Look at quantifiers over pairs, `a != b`,
     `implies` with a false premise, and an absent value on either side of a comparison.
   - Can a positive control pass while its check is wrong?
   - Does the checker run after the whole chained outcome and not part-way through it?
   - Is the snapshot complete: does `check` need anything the snapshot does not hold?
   - Does any snapshot comparison depend on a `Dict`'s iteration order?
   - Does the coverage check fail closed? Look at a count that drifts, a pattern that
     matches nothing, a tag with a typo, and a tag on a test that failed.
5. **Port the scenarios, red first.** For each scenario this ticket owns, write its tagged
   test against the Rust test's setup, steps and assertions, driving the owner. See it
   fail, or say in the hand-back why it cannot be red. Then add the rule that makes it
   pass as a row of the Publish table. GD05 adds `DeferBehindHeldVersion`'s row. Points
   to watch:
   - A publish answers nothing (the ruling on N32).
   - Each refusal test compares the complete snapshot before and after: the equal-rank
     publish and a publish by an unknown publisher, which is `impl-choice` (N10).
   - `Supersede` and `DropStalePublish` read `now`. Assert through the fake clock that
     each read it once, and that the equal-rank refusal read it not at all.

   Shrink `pending_scenarios` and `pending_rules` by exactly what landed.
6. **Spec notes.** Add this ticket's notes, each citing its `N`-note: a payload's type
   (N02), a rank's bound (N03), the first position (N04), a lease's id (N05), a reference
   as a key (N06), `a != b` over entities (N07), a field declared `when status = ...`
   (N08), the unknown publisher (N10), the equal-rank refusal (N11) and the publish that
   answers nothing (N32). Say where Gleam's types made a reading unnecessary: a field
   that only a held record has can be a field of the held variant alone.
7. **Adversarial review** (checkpoint 2: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while a superseded record
   took the wrong reason or time, while a refused publish changed a position counter, or
   while the coverage check credited a rule that never fired.
8. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- The coverage check asserts the counts 93, 32 and 35. On exit, `pending_scenarios` holds
  exactly the 82 ids GD03 to GD06 own, and `pending_rules` the 26 rules they own. No
  invariant is pending.
- All 35 invariants are in the checker's registry, and each has a passing positive control
  tagged `InvariantControl("<Name>")`.
- The step 3 and step 4 probes are quoted with their output and exit status.
- The snapshot holds `next_position`, the publishers and the consumers as well as the
  pockets, records, leases and both shed queues, each as a list in a stated order.
- Every rule GD01 and this ticket own fired at least once, as the coverage check's
  fired-rule half shows.
- Every refusal this ticket lands has a test showing that the complete snapshot is
  unchanged and that no clock was read.
- `gleam-double/SPEC-NOTES.md` holds a note for each `impl-choice` row and each
  representation choice, each citing its `N`-note.
- `just gleam-double` leaves nothing Git can see, and no `erl_crash.dump`.
- Both checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The `rg`
lines show that `owner` is the only module under `src/` that imports the process
libraries. The probe is the positive control: the pattern matches both of its lines.

```sh
set -eu
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated
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

Evidence files are under `ai_tmp/gd/GD02/`, and the review records under
`ai_tmp/review/GD02-1/` and `ai_tmp/review/GD02-2/`. None is committed. Each `ev-*.txt`
begins `$ <command>` and ends `exit status: N`, except where the text says otherwise. In
every quote below the recipe's echo of its own command lines is left out, and `...` marks
other lines left out or a line cut short: a `FAIL` line is cut after the test's name, or
keeps only the two values its assertion compared. A failing run prints the runner's panic
and the recipe's `error:` line between `coverage faults: N` and `exit status: 1`; those
are left out too, with no mark. A quote that does not open with its `$` line starts
part-way through the output. Where a quote the record should hold was never saved, the
text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`, GD01's record commit, which
  holds GD01.
- Work commit: `76782f774f34dfebf3b3f46358356d3cd83799aa`. The commit after it holds this
  record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`, on the untouched worktree
(`baseline.log`, which records each status as `<name>-exit=N` and holds no `$` line; its
last line counts the lines `git status --short` printed):

```text
a0d88fc1bd62584debb8a7e90dc1c5c47eee690f
...
All checks passed and the worktree is unchanged.
check-exit=0
...
26 tests run, 26 passed
gleam-double-exit=0
       0
```

Step 2: `plan-change` was loaded in GD01 and applied here. The three counts, 93 catalog
rows, 32 rules and 35 invariants, were confirmed in GD01 at
`5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`. `git diff --stat` from that commit to the
work commit prints nothing for `docs/specs/pachinko.allium` and
`docs/reference/testing.md`. The counts were not re-run by hand at this ticket's start.
From step 3 on the coverage check asserts all three on every run.

### 3. The design as it stands after GD02

This section is what a later ticket's author rereads. Sources are under
`gleam-double/src/pachinko_double/` and tests under `gleam-double/test/`. GD01's
section 3 still describes `rules.gleam`, `identity.gleam`, `config.gleam`, `time.gleam`
and `test/runner.gleam`: the work commit touches none of them.

- **`model.gleam`.**
  - `Queue`, `Publisher`, `Consumer`.
  - `Pocket(key, status, position, consecutive_expiries)`, with the status
    `PocketQueued`, `PocketLeased`, `PocketIdle` or `PocketParked`.
  - `Record(pocket, key, rank, payload: BitArray, position, status)`, with the status
    `RecordQueued` or `RecordHeld(held_rank, held_payload, mark, nacked_at: Option,
    deferred_position: Option)`. A `Mark` is `Unsettled`, `Acked` or `Nacked`.
  - `SupersededRecord(pocket, key, rank, payload, reason, shed_at, expires_at)`, with the
    reason `Superseded` or `Stale`.
  - `DeadLetter(pocket, key, rank, payload, nacked_by, nacked_at, expires_at)`.
  - `Lease(id, pocket, consumer, status, granted_at, expires_at)`, with the status
    `LeaseActive`, `LeaseReleased(ended_at)` or `LeaseExpired(ended_at)`.
  - `State(config, queue, publishers, consumers, pockets, records, leases, superseded,
    dead_letters)`. Every list is in creation order, and a changed member keeps its
    place.
  - Functions over plain lists: `pocket`, `record`, `records_of`, `queued`, `held`,
    `ending`, `active`, `leases_of_pocket`, `leases_of_consumer` and `replace_record`.
  - A reference to an entity is that entity's key (G09): a `pocket` field holds a pocket
    key, and a lease's `consumer` a consumer id.
- **`inspection.gleam`.**
  - `Snapshot(config: Settings, next_position, publishers, consumers, pockets, records,
    leases, superseded, dead_letters)`, with `empty(settings)` and `snapshot(state)`.
  - `views(snapshot) -> Views(pockets: List(PocketView), consumers: List(ConsumerView))`.
    A `PocketView` holds the pocket and the specification's derived sets under its names:
    `records`, `queued_records`, `held_records`, `ending_records`, `leases` and
    `active_leases`. A `ConsumerView` holds the consumer, `leases` and `active_leases`.
  - There is one view per entity, in the snapshot's order. A key several entities hold
    gets a view for each, and a member whose reference names nothing is in no view.
- **`invariants.gleam`.** `Invariant(name, holds: fn(Snapshot, Views) -> Bool)`,
  `registry()` and `check(snapshot) -> List(String)`. The registry holds the 35 checks in
  the specification's order, each under a comment quoting the specification's expression.
  `check` calls `inspection.views` once. A field read through a reference is read from
  every pocket with that key and must hold for each; with none it holds vacuously (G09).
- **`core.gleam`.**
  - `Trigger`: `PublisherJoins`, `ConsumerJoins` and `Publish(Publication)`.
  - `apply(state, trigger)`. An unknown publisher is refused there, with
    `UnknownPublisher`, before any table runs (G12).
  - `run(table, input, state)`, public so that a test can send a scratch table. Every
    outcome ends in the private `conclude`, which runs the checker once over the state
    the whole chain left, then finishes or fails with `BrokenInvariant`.
  - `registry()`, each table as its trigger's name and its rules' names. The coverage
    check reads the rules from it.
  - `chain` routes `PocketTouched` to `publish.pocket_touched()`.
- **`publish.gleam`.** `Publication(publisher, pocket_key, record_key, rank, payload)`;
  `publish()`, with four rows and the refusal `EqualRankRefused`; `pocket_touched()`,
  with no row and `NothingHappens`.
- **`domain.gleam`, `refusals.gleam`, `owner.gleam`.** `Event` has one constructor,
  `PocketTouched(pocket_key)`, and `Answer` none yet. The refusals gain
  `UnknownPublisher(publisher_id)` and `EqualRankRefused(pocket_key, record_key, rank)`.
  The owner gains `publish(owner, publication)`; `Message` is unchanged.
- **`test/coverage.gleam`.** `check(report) -> List(String)`, called from `main` after
  `runner.run`, and the pure `faults(Inputs)`; `pending_scenarios` and `pending_rules`;
  `expected_scenarios` 93, `expected_rules` 32 and `expected_invariants` 35. `main`
  prints each fault as `COVERAGE <fault>`, then `coverage faults: N`. Beyond shared
  rule 3 it has three more faults: an entry a parsed list holds more than once, a pending
  entry the sources lack, and a rule name that is a row twice (checkpoint 1). A fired
  name that no table holds is deliberately no fault, and a test pins that.
- **`test/harness.gleam`.** Adds `publish(double, pocket_key, record_key, rank,
  payload_text)`, which publishes as publisher `p`, and `publish_as`, `pocket`, `record`
  and `order`. The new test modules, all listed in `main`, are `coverage_test`,
  `invariants_test`, `core_test`, `inspection_test` and `publish_test`.

Changes to a GD01 seam, or departures from shared rule 4, and what each costs:

| Before, or as the rule words it | After GD02 | Cost |
| --- | --- | --- |
| `Snapshot(next_position, publishers, consumers)` | Nine fields. The first is `config: Settings`, which shared rule 4's list does not name and the config invariants read | `owner_test`'s three positional `Snapshot` values were rewritten onto `inspection.empty`. No other cost is recorded |
| "Derived sets [...] are computed once, in the snapshot" (shared rule 4) | `inspection.views(snapshot)` computes them once per `check`. `Snapshot` stores none (checkpoint 1, D1 and C1) | `Invariant.holds` takes the views beside the snapshot, and 31 of the 35 checks ignore them. The rule is met as the snapshot's module, not as fields of the value. Five of the eight sets have no reader under `src/` yet: GD04's lease view and lease end are their first |
| "the records in the specification's flat field names" (shared rule 4) | The snapshot holds `model.Record` unchanged, so the five held fields sit on `RecordHeld` (checkpoint 1, C2; G14) | The sentence, carried word for word by GD03 to GD07, does not describe the code. A reader reaches the held fields by matching on `record.status` (Open points) |
| `core.apply` handed `rules.finish` to `rules.run` for each trigger | `core.run` hands it `conclude`, which runs the checker first | `core.run` builds its own `World`, whose `chain` is fixed, so a test cannot hand it a chain that mends a broken state (checkpoint 1, B6; Open points) |
| `chain` routed nothing, and `Event` had no constructor | `chain` routes `PocketTouched(pocket_key)` to a table with no row | None recorded |
| `Trigger` held the two joins | `Publish(Publication)` is added, and `apply` refuses an unknown publisher itself, before any rule runs, so no table is consulted (G12) | None recorded |
| `State(config, queue, publishers, consumers)` | Nine fields | None recorded |
| `main` ran the registry and printed the counts | `main` then runs `coverage.check(report)`, prints the faults and the `coverage faults: N` line, and its panic message names the coverage check | The `just gleam-double` row of `docs/reference/commands.md` now names the coverage check |

### 4. Steps 3 to 6

Every output in this section ran uncommitted, on base
`a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`. Each subsection repeats its head.

Who did what, one writer at a time:

- The lead wrote `model.gleam`, `inspection.gleam`'s `Snapshot`, `empty` and `snapshot`,
  `domain.gleam`'s event, `core.gleam`, `publish.gleam`'s skeleton with two tables and no
  row, `invariants.gleam`'s stub, the two refusals, `owner.publish`, and
  `harness.publish` and `publish_as`.
- One subagent each wrote step 3 (`coverage.gleam`, `coverage_test.gleam`), step 4
  (`invariants.gleam`, `invariants_test.gleam`, `core_test.gleam`) and step 5 (the four
  Publish rows, `model.replace_record`, `publish_test.gleam`, and the harness's `pocket`,
  `record` and `order`).
- Step 6 had two: one drafted G07 to G19 in scratch while checkpoint 1 ran, and another
  finalised G07 to G21 against the landed code and extended the `just gleam-double` row
  of `docs/reference/commands.md`.
- At each checkpoint one runner subagent per lens drove Codex, and one fresh triage
  subagent checked the findings and applied the fixes.

#### Step 3: the coverage check

Uncommitted, on base `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`.

The red came in two batches, not one test at a time. `ev-step3-red-1-parse.txt` has the
three parsing tests failing, `29 tests run, 26 passed`, exit status 1.
`ev-step3-red-2-faults.txt` has 16 of the 19 tests then added failing, `48 tests run, 32
passed`, exit status 1; the three that expect no fault were green against the stub.
`ev-step3-green.txt` followed the probes: `48 tests run, 48 passed`,
`coverage faults: 0`, exit status 0.

The seven probes. Each was a scratch change run through `just gleam-double` and reverted.
Each evidence file ends with the scratch diff and the SHA-256 of each changed file before
the change and after the restore, which match. Probes 1 to 6 each ended
`48 tests run, 48 passed` and `coverage faults: 1`.

| Probe | Scratch change | Evidence | The line that carries the claim | Exit |
| --- | --- | --- | --- | --- |
| 1 | `Scenario("S999")` on `identity_test`'s untagged test | `ev-step3-s999-tag.txt` | `COVERAGE identity_test.a publisher and a consumer may share an id is tagged Scenario("S999"), and the catalog holds no S999` | 1 |
| 2 | `S001` taken off `pending_scenarios` | `ev-step3-unpend-untagged.txt` | `COVERAGE S001 is not in pending_scenarios, and no test tagged Scenario("S001") passed` | 1 |
| 3 | `Scenario("S011")` on the same test, with S011 still pending | `ev-step3-tag-pending.txt` | `COVERAGE S011 is in pending_scenarios, and a test tagged Scenario("S011") passed` | 1 |
| 4 | A row `NoSuchRule`, whose guard never holds, in the `PublisherJoins` table | `ev-step3-nosuchrule-row.txt` | `COVERAGE the PublisherJoins table has a row named NoSuchRule, and the specification holds no rule NoSuchRule` | 1 |
| 5 | `Renew` taken off `pending_rules` and made a row of the `ConsumerJoins` table, whose guard never holds | `ev-step3-unfired-row.txt` | `COVERAGE Renew is a row of the ConsumerJoins table and not in pending_rules, and no test that passed fired it` | 1 |
| 6 | `expected_scenarios` changed from 93 to 92 | `ev-step3-count-92.txt` | `COVERAGE the catalog holds 93 scenarios, and the check expects 92` | 1 |
| 7 | An assertion that fails inside `harness.fired` | `ev-step3-failed-test.txt` | Quoted below | 1 |

Probe 7 failed five tests, not one, because `harness.fixture()` fires both joins in every
fixture test. Neither tag and neither rule was credited:

```text
FAIL identity_test.a publisher joins once: crashed: ...
...
COVERAGE S009 is not in pending_scenarios, and no test tagged Scenario("S009") passed
COVERAGE S010 is not in pending_scenarios, and no test tagged Scenario("S010") passed
COVERAGE PublisherJoins is a row of the PublisherJoins table and not in pending_rules, and no test that passed fired it
COVERAGE ConsumerJoins is a row of the ConsumerJoins table and not in pending_rules, and no test that passed fired it
48 tests run, 43 passed
coverage faults: 4
```

#### Step 4: the model, the snapshot and the checker

Uncommitted, on base `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`.

The 35 checks were written red first in eight slices. Each invariant has a control that
asserts the exact list, `check(snapshot) == ["<Name>"]`. Every run below exits 1.

| Evidence | Controls failing | Tests run, passed |
| --- | --- | --- |
| `ev-step4-red-1.txt` | `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `IdleHasNoRecords`, `ExpiriesWithinLimit` | 55, 51 |
| `ev-step4-red-2.txt` | `HeldVersionNotNewer` to `HeldCopiesAgree`, seven | 62, 55 |
| `ev-step4-red-3.txt` | `ShedBeforeExpiry`, `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant` | 67, 63 |
| `ev-step4-red-4.txt` | The five config invariants, `ExpiryLimitPositive` to `DeadLetterTtlNonNegative` | 73, 68 |
| `ev-step4-red-5.txt` | `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket` | 77, 73 |
| `ev-step4-red-6.txt` | `OneActiveLeasePerPocket` (two controls), `OneActiveLeasePerConsumer`, `OutstandingOnlyWhileLeased`, `MarkedPocketHasNoExpiries`, and one untagged test on a pocket key two pockets have | 86, 80 |
| `ev-step4-red-7.txt` | `UniquePocketPosition`, `UniqueRecordPosition`, `PocketPositionsBelowNext`, `RecordPositionsBelowNext`, and one untagged test on repeated members | 92, 87 |
| `ev-step4-red-8.txt` | `ArrivalPositionUnoccupied` (two controls), `UniqueArrivalPosition`, `ArrivalPositionsBelowNext`, and the registry-order test | 98, 93 |

One of them, `ev-step4-red-8.txt`, with the invariant half of the coverage check on:

```text
FAIL invariants_test.UniqueArrivalPosition: two deferred versions with one arrival position: crashed: ...
...
COVERAGE the invariant UniqueArrivalPosition is not in the checker's registry
COVERAGE the invariant UniqueArrivalPosition has no test tagged InvariantControl("UniqueArrivalPosition") that passed
...
98 tests run, 93 passed
coverage faults: 6
exit status: 1
```

The scratch break. `HeldCopiesAgree` was weakened so that it never fires
(`probe-weakened.diff`, the first two lines below). Its control failed, and the coverage
check withdrew the credit (`ev-step4-weakened-check.txt`):

```text
<       implies(deferred_position == None, held_payload == record.payload)
>       implies(deferred_position == None && held_payload == record.payload, True)
...
FAIL invariants_test.HeldCopiesAgree: two payloads with nothing deferred: crashed: ... #(Left, ... #(Value, []) ...), #(Right, ... #(Value, ["HeldCopiesAgree"]) ...
COVERAGE the invariant HeldCopiesAgree has no test tagged InvariantControl("HeldCopiesAgree") that passed
102 tests run, 101 passed
coverage faults: 1
exit status: 1
```

The check was restored from its copy; `probe-weakened.notes` holds the two SHA-256
values, which are equal. `ev-step4-green.txt` then ended `102 tests run, 102 passed`,
`coverage faults: 0`, exit status 0.

The apply step. `core_test` holds four tests on scratch tables of the domain's types: an
effect that leaves an invariant broken fails, naming it; an outcome that breaks two
invariants names both, in the registry's order; an outcome that emits an event is still
checked; and a publisher join through the apply step still succeeds.

A failure carries no state, so the only state after a `BrokenInvariant` is the one the
caller passed in. A red run of these four is not recorded. That they can fail is shown by
the author's sweep: with `UniquePublisherId` made never to fire, the first three fail
(`ev-step4-mutants.txt`). Not shown: an outcome broken part-way and valid at the end
(Open points, B6).

The author's mutation sweeps hold no `$` line; each line records its own result.
`ev-step4-mutants.txt` has 35 checks made never to fire and 51 operators swapped, 86 of
86 killed. `ev-step4-mutants-structural.txt` has 12 structural changes: 11 killed, and
`OneActiveLeasePerPocket ignores the leases' status` did not build. Both ran before
checkpoint 1, whose D1 fix then rewrote four checks (section 8).

#### Step 5: the Publish rows and the ports

Uncommitted, on base `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`.

The 15 publish tests were written first and the rows added one at a time. Each run has
121 tests.

| Evidence | Rows in the Publish table | Publish tests failing | Passed | Coverage faults | Exit |
| --- | --- | --- | --- | --- | --- |
| `ev-step5-red-0-no-rows.txt` | None | 15 | 106 | 6: S011, S013, S014, S016, S017, S018 | 1 |
| `ev-step5-red-1-after-PublishIntoNewPocket.txt` | `PublishIntoNewPocket` | 10 | 111 | 3: S014, S016, S018 | 1 |
| `ev-step5-red-2a-PublishNewKey-still-pending.txt` | And `PublishNewKey`, still in `pending_rules` | 9 | 112 | 4: those three, and the rule both pending and a row | 1 |
| `ev-step5-red-2-after-PublishNewKey.txt` | And `PublishNewKey`, off the list | 9 | 112 | 3 | 1 |
| `ev-step5-red-3-after-Supersede.txt` | And `Supersede` | 5 | 116 | 2: S016, S018 | 1 |
| `ev-step5-green.txt` | All four | 0 | 121 | 0 | 0 |

The first red, `ev-step5-red-0-no-rows.txt`. With no row, every publish met the table's
refusal:

```text
FAIL publish_test.a publish into a new pocket creates a queued pocket and record: crashed: ... #(Left, ... #(Value, Error(Refused(EqualRankRefused("pk", "a", 3)))) ...), #(Right, ... #(Value, Ok(Outcome(["PublishIntoNewPocket"], None))) ...
...
COVERAGE S011 is not in pending_scenarios, and no test tagged Scenario("S011") passed
...
121 tests run, 106 passed
coverage faults: 6
exit status: 1
```

S013, S017, the unknown-publisher test and the `PublishIntoNewPocket` rule test were red
only through their setup publish, since both refusals predate the rows.

The last red, `ev-step5-red-3-after-Supersede.txt`, with `DropStalePublish` still
missing:

```text
FAIL publish_test.a stale publish is shed, not applied and not an error: crashed: ... #(Left, ... #(Value, Error(Refused(EqualRankRefused("pk", "a", 4)))) ...), #(Right, ... #(Value, Ok(Outcome(["DropStalePublish"], None))) ...
...
COVERAGE S016 is not in pending_scenarios, and no test tagged Scenario("S016") passed
COVERAGE S018 is not in pending_scenarios, and no test tagged Scenario("S018") passed
121 tests run, 116 passed
coverage faults: 2
exit status: 1
```

The mutant sweep, after green (`ev-step5-mutants.txt`, with each run's whole output in
`ev-step5-mutant-<id>.txt`). Its summary line:

```text
37 mutants, 37 killed by a named test of publish_test.
```

The 37 are P1 to P7 on `PublishIntoNewPocket`, K1 to K6 on `PublishNewKey`, U1 to U10 on
`Supersede`, D1 to D7 on `DropStalePublish`, H1 to H5 on the shed entry the last two
write, and M1 and M2 on `model.replace_record`. Every line records exit status 1.

- The first pass had 33 mutants and killed 30 (`ev-step5-mutants-first-pass.txt`). P3, U1
  and D3 died at the build on an unused import and were rerun with the import list
  adjusted, so that a test fails and not the compiler. P7, K6, U10 and D7 were added
  after the first pass.
- One fixture was added for a mutant, before it ran: a pocket `other` holding the same
  key, without which M1 has no test that supersedes a key two pockets hold.
- The sweep ran at 121 tests, before checkpoint 2 changed the tests (section 8).

The green run, `ev-step5-green.txt`:

```text
$ just gleam-double
...
121 tests run, 121 passed
coverage faults: 0
exit status: 0
```

The ports, all through the owner: S011, S013, S014, S016, S017 and S018.

- Every Rust publish outcome is ported as `Ok(Outcome([rule], None))`, with the state
  read through the snapshot (the ruling on N32; G15). Nothing the Rust asserts was
  dropped.
- S011, S014 and S016 assert absolute positions and are `impl-choice` (G04, G16). S014
  ports `invariants::shed_before_expiry` as `invariants.check` not naming
  `ShedBeforeExpiry`.
- S017's port asserts the refusal, compares the complete snapshot before and after, and
  asserts that the fake clock was not read. The unknown-publisher test is untagged and
  `impl-choice` (G12).
- Untagged tests kept beside the ports: one "changes exactly what it ensures" test per
  rule, of which `Supersede`'s and `DropStalePublish`'s read the clock once and the other
  two not at all; and four that read the Publish table directly on hand-built states, for
  the emitted events, `Supersede`'s held branch, `Supersede` shedding a deferred version,
  and the `Impossible` branch.
- `PublishIntoNewPocket` emits no `PocketTouched`, as the rule text has it (G20). The
  lead's brief had said otherwise (section 9).
- On exit `pending_scenarios` holds 82 ids and `pending_rules` 26 rules.

#### Step 6: spec notes

`gleam-double/SPEC-NOTES.md` gained G07 to G21, and no `TODO` remains in it.

Ten agree with an `N`-note, which each cites: G07, a payload is a `BitArray` (N02); G08,
a rank is an unbounded `Int` (N03); G09, a reference is the entity's key (N06); G10, a
lease has an id (N05); G11, `a != b` compares members (N07); G12, the unknown publisher,
`impl-choice` (N10); G13, `EqualRankRefused` (N11); G14, a field declared
`when status = ...` (N08); G15, a publish answers nothing (N32); and G16, the absolute
positions of S011, S014 and S016, `impl-choice` (N04). Five have no `N`-note: G17 and G18
(Open points); G19, the snapshot's lists are in creation order; G20,
`PublishIntoNewPocket` emits no `PocketTouched`; and G21, a rule changes only what its
`ensures` names.

G14 departs from shared rule 4's "flat field names" and says so. It is also where Gleam's
types made a reading unnecessary: the five held fields are fields of the held variant
alone, and `ended_at` of the two ended variants.

### 5. Checkpoint records

Both checkpoints used Codex v0.169.1 with `gpt-6-astra`, `-s read-only`, one fresh
process per lens. Each lens left `last-<lens>.md` and `log-<lens>.txt`, as the ticket's
corrected command writes them. Codex was on `PATH`, so no fallback to a Claude reviewer
was needed. All five findings files open `Source: last-message`: each is Codex's last
message, recovered because a read-only Codex writes no file. Checkpoint 1's three
reviewers traced by reading and ran nothing, and checkpoint 2's breakage reviewer says
the same of itself; the triage ran each claim. One triage subagent worked each
checkpoint, and it was neither a reviewer nor the author. No finding was rated high at
either checkpoint, by the reviewer or by the triage, so each had one round and no second
was owed. Not recorded: the `codex exec --help` check on the day, and the model of the
runner and triage subagents.

#### Checkpoint 1 (step 4): conformance, breakage and design

Ran uncommitted, on base `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`, before any publish
scenario was ported. `just gleam-double` ended `102 tests run, 102 passed` before any fix
(`ev-baseline.txt`). The conformance lens filed C1 and C2, the breakage lens B1 to B6 and
the design lens D1 to D3.

| Id | Severity | Verdict | Reason | Applied or handed back |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | Shared rule 4 says derived sets are "computed once, in the snapshot"; the snapshot held none and the checker composed them per check | Handed back with D1. The lead accepted it: `inspection.views`, below |
| C2 | medium | valid as written | Shared rule 4 asks for the snapshot's records "in the specification's flat field names"; the five held fields have the specification's names but sit inside the `status` value | Handed back. The lead ruled no code change: the departure is stated in G14 and under Open points |
| B1 | medium | valid | Two rows with one name shared one firing: `faults` returned `[]` for `["First", "First"]` | A fault for a name that is a row more than once, in one table or across tables, with a test |
| B2 | medium | valid | `mark == Acked` and `mark == Nacked` each survived in place of `mark != Unsettled` | The marked-pocket control also asserts each mark alone |
| B3 | low | valid in all 13 rows | Each of the 13 named weakenings survived the reviewed controls when run | Assertions added to the 14 controls its rows name |
| B4 | low | valid in all 3 rows | `!= next_position` survived in place of `< next_position` for pockets, records and arrivals | The three controls also assert a position above `next_position` |
| B5 | low | valid in all 6 rows | A literal bound of 3, `!= 0` and `!= -1` each survived | The expiry controls set the limit in the snapshot; the two zero controls also assert -1; the three TTL controls also assert -2 |
| B6 | low | valid | A checker call added before the chained event survived; the witness needs a `PocketTouched` row, which GD04 lands | Handed back: GD04 owes the witness (Open points) |
| D1 | medium | valid | As C1 | As C1 |
| D2 | medium | valid | As B1 | As B1 |
| D3 | low | valid | With the switch flipped to `CountOnly` the whole run stayed green; nothing pinned it | Step 3's `CountOnly` mode is gone, and the invariant half always asks for the registry entry and the passed control |

The triage changed three files, all under `gleam-double/test/`: `coverage.gleam`,
`coverage_test.gleam` and `invariants_test.gleam`. Every new assertion of B2 to B5 passed
on the unchanged checker at its first run, so none found a fault in `invariants.gleam`.
After it `ev-final.txt` ended `102 tests run, 102 passed`: one test was added for B1 and
one removed for D3.

The 24 weakenings of B2 to B5, and the four of `views`:

| Evidence | Tree | Result |
| --- | --- | --- |
| `ev-mutants-before-summary.txt` | As reviewed | 24 of 24 exit 0, each `102 tests run, 102 passed` |
| `ev-mutants-after-summary.txt` | After the triage | 24 of 24 exit 1, each at a new assertion of the control named for its invariant |
| `ev-mutants-after-d1-summary.txt` | After D1 | 24 of 24 exit 1, each at the same control and line, of 106 tests |
| `ev-D1-views-*.txt` | After D1, four weakenings of `views` | Each exit 1. Two fail one test of `inspection_test`. The other two, a pocket's and a consumer's `active_leases` made all its leases, fail 50 and 49 tests: the checker reads the views |

B1 on the real table: a second `PublisherJoins` row whose guard never holds printed
`COVERAGE PublisherJoins is a row more than once: PublisherJoins, PublisherJoins` and
`coverage faults: 1`, exit status 1 (`ev-B1-D2-real-table-probe.txt`). B6's mutant, a
checker call in `core.chain` before the chained event, is killed by nothing yet:
`102 tests run, 102 passed`, `coverage faults: 0`, exit status 0
(`ev-B6-premature-check.txt`).

The lead's rulings on the three hand-backs:

- **D1 and C1.** Accepted, and applied by the same triage subagent in its smallest
  proposed form: `inspection.views(snapshot)`, computed once per `check`. `Snapshot`
  keeps its nine fields. It was written red first: the build failed before `views`
  existed (`ev-D1-red-no-views.txt`, exit status 1), and two tests failed against a
  `views` that returned nothing (`ev-D1-red-stub.txt`, `106 tests run, 104 passed`, exit
  status 1). The fix changed four files: `inspection.gleam`, `invariants.gleam`, the new
  `inspection_test.gleam` with four tests, and `pachinko_double_test.gleam`. One mutant,
  B3-01, was rewritten for the changed line; the other 23 apply as written.
- **C2.** No code change. Spec note G14 states the departure, and Open points puts the
  choice to the maintainer.
- **B6.** Handed to GD04, which lands the first `PocketTouched` rule. GD04's ticket does
  not hold the line yet; the lead adds it before GD04's baseline.

Tree ids: before the triage `78239c7d351ac38a62ba0cb74a303c0bcf4c022b`, after it
`dd4c3fc0edec3aef0bf528ad167bee50c8b18f4b`, after D1
`10489274edfa35ef16e08f99d3164532ec524ed0`.

The re-run after the checkpoint (`ev-D1-final.txt`), uncommitted, on base
`a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`:

```text
$ just gleam-double
...
106 tests run, 106 passed
coverage faults: 0
exit status: 0
```

#### Checkpoint 2 (step 7): conformance and breakage

Ran uncommitted, on base `a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`, after step 6.
`just gleam-double` ended `121 tests run, 121 passed` before any fix (`ev-baseline.txt`).
The conformance lens filed C1 and the breakage lens B1 to B6. L1 is a comment the lead
flagged.

| Id | Severity | Verdict | Reason | Applied | Its mutants, before and after |
| --- | --- | --- | --- | --- | --- |
| C1 | medium | valid | S018's port asserted state only after its fourth publish, though the Rust checks three of them for an outcome | After each of the three publishes the port asserts the stored version, the one version shed with its reason, and the queue's count. G15 and G19 reworded | As B3 |
| B1 | medium | valid in all 3 rows | Each of the three rows could erase the superseded queue's earlier entries and every test passed | The three "changes exactly" tests start from a supersede and compare the complete snapshot | Three, each 121 of 121; then 120, 121 and 121 of 122 |
| B2 | medium | valid in all 4 mutants | Every shed entry and every asserted `PocketTouched` was for pocket `pk` and key `a`, so a literal survived | Two tests shed in pocket `other` under key `b`, and `events` asserts each emitting row in both pockets | Four, each 121 of 121; then 120, 120, 121 and 121 of 122 |
| B3 | medium | valid | A `Supersede` that kept the old rank for a negative incoming rank passed all 121 tests | As C1 | One, 121 of 121; then 121 of 122, S018's port failing at line 272 |
| B4 | medium | valid | No test took a `State` with dead letters through `inspection.snapshot`; dropping them passed | A new test, `snapshot_of_state`, compares a whole `State`'s snapshot field by field | One, 121 of 121; then 121 of 122 |
| B5 | medium | valid for both marks | Every held fixture was unsettled; a guard that refused an acked or a nacked record passed | The two held-record tests run for each of the three marks. G13 and G21 gained a clause | Two, each 121 of 121; then 120 of 122 each |
| B6 | low | valid | `harness.record` did not look the pocket up first as the Rust fixture's `record` does, so S013 passed with no pocket stored | `harness.record` finds the pocket first, then that pocket's record | One, 112 of 121 with S013's port passing; then 112 of 122 with it failing |
| L1 | low | valid | The comment said the specification orders no collection; G19 records that it orders queued pockets and a pocket's queued records by position | The comment in `publish_test.gleam` is in G19's words | None |

All eight were applied and nothing was handed back. The triage changed four files:
`publish_test.gleam`, `inspection_test.gleam`, `harness.gleam` and
`gleam-double/SPEC-NOTES.md`. Nothing under `gleam-double/src/` changed.

Each breakage finding predicted a mutant, twelve in all (`ev-before-<mutant>.txt`,
`ev-after-<mutant>.txt`). Every "before" run was taken before any fix, and every "after"
run on the final tree. The last column gives the tests passed. Eleven survived before,
exit status 0. The twelfth, B6's, already failed nine tests, exit status 1, but not
S013's port. Every "after" run exits 1, and each fails the test its verdict names. After
B6's fix its mutant also loses S013's credit: the run prints
`COVERAGE S013 is not in pending_scenarios, and no test tagged Scenario("S013") passed`.

Three runs between the fixes were green: after C1 and after B1 and B2, each
`121 tests run, 121 passed`, and after B4, B5 and B6, `122 tests run, 122 passed`
(`ev-green-after-c1.txt`, `ev-green-after-b1-b2.txt`, `ev-green-after-b4-b5-b6.txt`, exit
status 0 each).

A note from the triage, not a finding and not fixed: a publish row that emptied `leases`
or `dead_letters` is seen only for `Supersede`, by the test that compares a whole `State`
holding a lease. No publish through the owner meets a lease or a dead letter until GD03
and GD06 land the rules that create them.

Tree ids: before the triage `064faec5cfdfaf88f63fc5d0f41bf8a5d8f6e303`, after it
`430607eb495e67d4dae38138ad42e216ead700d4`. `just lint` and `just check-docs` passed
after the last Markdown edit (`ev-lint.txt`, `ev-check-docs.txt`, exit status 0 each).
`just check` did not run between the triage and the commit: it ran at the work commit
(section 6).

The re-run after the checkpoint (`ev-final-green.txt`), uncommitted, on base
`a0d88fc1bd62584debb8a7e90dc1c5c47eee690f`:

```text
$ just gleam-double
...
122 tests run, 122 passed
coverage faults: 0
exit status: 0
```

### 6. Verification

The Verification block ran at `76782f774f34dfebf3b3f46358356d3cd83799aa`, the work
commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's block word for
word; output in `ev-verification.txt`). It exited 0, so its last two lines held:
`git status --short` printed nothing.

```text
$ sh ai_tmp/gd/GD02/verification.sh
76782f774f34dfebf3b3f46358356d3cd83799aa
gleam 1.19.1
OTP 29
...
122 tests run, 122 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The 122 tests are GD01's 26 and 96 more: `coverage_test`, `invariants_test`,
`core_test`, `inspection_test` and `publish_test`. A count for each module is not
recorded.

### 7. Stops

None fired. The conditions watched:

- A count that moved (step 2). None did: the coverage check asserts 93, 32 and 35 on
  every run, and every green run from step 3 on ends `coverage faults: 0`.
- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did.
- A `GuardOverlap` on a specification table (shared rule 4). Dispatch returned none on
  the Publish table.
- A reach for the network. Nothing was fetched and no network recipe ran. The two review
  checkpoints called a model API, which each checkpoint authorises.

Step 8's stop stands: nothing was pushed and no pull request was opened.

### 8. What was not run

- Any push, pull request or issue.
- `just initialize`, `just gleam-double-deps` and `just lock`.
- A CI run.
- The three counts by hand at this ticket's start (section 2).
- A second review round at either checkpoint. None was owed.
- The step 4 sweeps after checkpoint 1. The 86 and the 12 mutants ran against
  `invariants.gleam` as it was before D1 rewrote four checks. After D1 only the 24
  weakenings of B2 to B5 and the four weakenings of `views` were run.
- The step 5 sweep after checkpoint 2. The 37 mutants ran at 121 tests. Checkpoint 2
  changed no file under `gleam-double/src/`, but it rewrote several of the tests the
  sweep names, and the sweep was not repeated at 122.
- A witness that the checker runs after the whole chain (checkpoint 1, B6).
- A test that would see a Publish row other than `Supersede` empty `leases` or
  `dead_letters` (checkpoint 2, the triage's note).

Not recorded: a red run of the four `core_test` tests; `codex exec --help` on the day;
the model of the runner and triage subagents; and a test count for each module at the
work commit.

### 9. Working lessons for GD03 to GD07

- A brief for a rule's author must quote the rule, not paraphrase it. The lead's brief
  said that a publish which adds or raises a record emits `PocketTouched`. That is wider
  than the rule, under which `PublishIntoNewPocket` emits nothing, and the author caught
  it.
- Mutation after green finds what red-first misses. Checkpoint 1's 24 surviving
  weakenings were all in controls written red first.
- Send a seam decision back to the triage subagent that proposed it. It applied D1 in one
  pass.

## Open points

- **PD07 is still todo, and is not a blocker.**
  `.scratch/python-double/PD07-inspection-contract.md` asks whether the inspection shape
  is part of the contract. Its answer may change what the snapshot lists and in which
  order. This ticket followed the rulings as they stand.
- **The snapshot's record holds the five held fields on its held variant, not flat
  (checkpoint 1, C2; spec note G14).** Shared rule 4 asks for "the records in the
  specification's flat field names". The snapshot holds the model's `Record` unchanged:
  `held_rank`, `held_payload`, `mark`, `nacked_at` and `deferred_position` keep the
  specification's names, but they are fields of `RecordHeld`, and a reader reaches them
  by matching on `record.status`. The lead kept the variant. The maintainer either
  accepts it or asks for one of two forms. The literal form is an inspection record with
  the five as `Option` fields: it rewrites the 12 checks that match on a record's status
  and every control that builds a held record, and it lets a hand-built snapshot hold a
  queued record with a `held_rank`, a state the specification excludes and no invariant
  names. The smaller form is five read functions in `inspection`, such as
  `inspection.held_rank(record) -> Option(Int)`, which give the flat names as reads and
  change no stored shape. The shared rules are carried word for word in seven tickets and
  were not edited.
- **No test shows that the checker runs after the whole chained outcome and not part-way
  (checkpoint 1, B6).** As landed, `core.conclude` runs it once, after the chain. But a
  second checker call placed in `core.chain`, before the chained event, passed all 102
  tests: the one event, `PocketTouched`, goes to a table with no row, so no outcome can
  be broken part-way and mended by its chain. GD04 owes the witness with
  `ReviveIdlePocket`. A publish of a new key into an idle pocket fires `PublishNewKey`,
  which leaves `IdleHasNoRecords` broken until the chained `ReviveIdlePocket` queues the
  pocket. That test, through the owner, must pass, and it kills the mutant. GD04's ticket
  does not hold the line yet; the lead adds it before GD04's baseline.
- **A higher rank published onto a held record with nothing deferred meets
  `EqualRankRefused` until GD05 (spec note G13).** That input is
  `DeferBehindHeldVersion`'s, and GD05 lands its row. Shared rule 4 makes the input
  pending, and no test pins it as a refusal. No publish through the owner can reach it
  until GD03 lands a pull, since no record is held before then. Between GD03 and GD05 the
  double answers it with the equal-rank refusal's name.
- **Two invariants are taken as written, and each leaves a question for the specification
  (spec notes G17 and G18).** `OneActiveLeasePerConsumer` ranges over consumers, so two
  active leases that name an id no consumer has break nothing. G17 asks whether the
  invariant is meant to bound the leases or the consumers. The position invariants
  compare within a kind, so a record's position or an arrival position equal to a
  pocket's breaks nothing, though the Queue comment says "positions are unique". G18 asks
  whether an invariant is missing or the comment should say "within each order". Both
  differences show only in a hand-built snapshot: a lease's consumer is a `Consumer` as
  the specification types it, and every position a rule is handed comes from one counter.
- **Derived sets are computed in `inspection.views`, not stored in `Snapshot`: closed by
  the lead's ruling (checkpoint 1, D1 and C1).** Shared rule 4 says derived sets are
  "computed once, in the snapshot". `inspection.views(snapshot)` computes them once per
  `check`, which meets the rule as the snapshot's module and not as fields of the value.
  The literal form, fields of `Snapshot`, was not taken: every control builds its
  snapshot by record update, so a hand-built snapshot could then hold a view that
  disagrees with its lists, and the checker would have to choose which to believe.
