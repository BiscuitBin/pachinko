---
id: GD02
title: "Gleam double: the model, the invariant checker, coverage and publish"
status: todo
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

Written at the end.

## Open points

- PD07, which asks whether the inspection shape is part of the contract, is still todo.
  Its answer may change what the snapshot lists and in which order. This ticket follows
  the rulings as they stand.
