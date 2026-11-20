---
id: GD04
title: "Gleam double: ack, nack, release and the lease's end"
status: todo
depends_on: [GD03]
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD04: Gleam double: ack, nack, release and the lease's end

## Context

GD01 to GD03 landed the Gleam double under `gleam-double/`: the owner actor, the pure
core, the entity model, `Config`, the joins, publish, pull, the held set, renew, the
invariant checker and the coverage check. Read their hand-backs and
`gleam-double/SPEC-NOTES.md` first. `.scratch/gleam-double/README.md` is the series'
index.

This ticket lands what a consumer does with a held record, and the lease's end by
release:

- marks, repeats and mark conflicts;
- fencing by lease id;
- a release that requeues or idles the pocket;
- the lease-end module, `lease_end.gleam`, with the three `Freed` rows that have no
  deferred version;
- dead letters;
- the revival of an idle pocket;
- the lease view, through the owner's `LeaseView` message.

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
  not even the lease's expiry, and reads no clock. The fake clock's read count shows that.
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

What the maintainer has already ruled, and what the Python double already read:

- **An ended lease stays in the state** (the ruling on N24). S053's Rust test asserts an
  empty lease registry, which is the Rust's inspection shape. The port asserts the
  specification's facts instead: the lease is `released` with `ended_at`, and no lease is
  active.
- A release and a mark answer nothing (N24).
- The refusals of a mark are the double's own names: `LeaseFenced`, `RecordNotHeld` and
  `MarkConflict`, with the lease checked first (N21).
- An ack, nack, release or lease read by a consumer that never joined is refused,
  `impl-choice` (N22).
- S090's lease view lists the held records smallest position first, `impl-choice` (N23).
- One lease end sends `Freed` smallest held position first (N25).
- The Release table's refusal past fencing is unreachable through the owner (N26).
- An idle pocket keeps its position (N27).
- A refused action reads no clock (N28).

S091 defers a newer version behind two held keys, so GD05 owns it.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/settle.rs`
and `lease.rs`, found with `rg -n 'scenario: S0NN'`. The `wire` rows here are S040, S047,
S053, S068 and S085; port only their library half.

## Goal

A consumer can ack and nack its held records, repeat a mark, be refused a conflicting mark
or a fenced action, and release its lease, all through the owner. The release reconciles
each held record by its mark, creates dead letters and requeues or idles the pocket. A
publish revives an idle pocket. A consumer can read its lease. One lease-end module owns
the lease's end, so GD05 adds rows inside it and GD06 only adds callers. The checker
holds after each operation, and every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (10) | `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `RequeueUnsettledRecord`, `ConsumeAckedRecord`, `DeadLetterNackedRecord`, `ReviveIdlePocket` |
| Scenarios (18) | S023, S035, S040, S041, S042, S047, S049, S051, S052, S053, S055, S057, S068, S070, S084, S085, S086, S090 |

## Non-goals

- No deferral and none of the three deferred lease-end rows. GD05 lands them.
- No lapse, parking or decay. GD06 lands them, with the lapse half of every "by release
  or lapse" scenario.
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

  `<id>` is `GD04`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `gleam-double/src/` | shared, own lines | Marks, fencing, release, the lease-end module with its three `Freed` rows, dead letters, the idle revival, the lease view, and the owner's `Ack`, `Nack`, `Release` and `LeaseView` messages |
| `gleam-double/test/` | shared, own lines | The settle and lease tests; the pending lists in `coverage.gleam` |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/gleam-double/GD04-ack-nack-release.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/GD05-deferral.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/gleam-double/GD06-clock-tick-lapse-parking-decay.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/gleam-double/README.md` | shared, own lines | GD04's Status cell, and any move |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD03, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the sections named in Context, the
   guarantees `ActionsNameTheirLease`, `HeldSetStableUntilLeaseEnds` and
   `MarksStandHoweverLeaseEnds`, N21 to N28 in `double/SPEC-NOTES.md`, and the catalog
   rows this ticket owns. Confirm that each owned scenario needs no deferral and no
   lapse. Move any that does, under shared rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test. Points to watch:
   - Land the lease-end module whole, as Context describes. GD05 adds its three deferred
     rows inside the module, and GD06 only adds callers from the lapse; neither changes
     its interface.
   - Every refusal compares the snapshot before and after: a fenced action, a mark
     conflict, an ack of a key not held, and a second release. S085's refusal also reads
     no clock.
   - Lease ids are compared by value. A new lease over the same pocket has a new id from
     the lease-id source.
   - A release at the grant instant is legal (S057).
   - S053 is ported as the ruling on N24 says: the ended lease stays, and the test
     asserts its status and `ended_at`.
   - S090's order of the lease view, smallest position first, is more than the surface
     states. Port it as `impl-choice` (N23).

   Shrink `pending_scenarios` and `pending_rules` by exactly what landed.
4. **Spec notes.** Add a note wherever marks, fencing or release forced an
   interpretation, each citing its `N`-note. Candidates:
   - which refusal an ack of a queued record of the same pocket is;
   - whether a refusal kind is the specification's to name at all;
   - S090's order;
   - whether `ReleaseIdlesEmptyPocket` leaving the pocket's position unchanged matters,
     given that idle pockets are not in the pocket order.
5. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while a refusal changed
   something, while a dead letter's `nacked_at` or `expires_at` came from the wrong
   timestamp, or while the checker ran part-way through a release.
6. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- On exit, `pending_scenarios` holds exactly the 46 ids GD05 and GD06 own, and
  `pending_rules` the 11 rules they own.
- Every owned scenario has a tagged test, and each of the ten rules has its row in its
  trigger's table, fired in at least one test.
- Both release rows end the lease only through the lease-end module.
- Every refusal this ticket lands has a test showing that the snapshot is unchanged, and
  S085's test shows that no clock was read.
- The lease view is read through the owner's `LeaseView` message.
- `gleam-double/SPEC-NOTES.md` holds this ticket's notes, including S090's `impl-choice`,
  each citing its `N`-note.
- `just gleam-double` leaves nothing Git can see, and no `erl_crash.dump`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
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
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Written at the end.

## Open points

- PD07 asks whether an ended lease is part of the observable state. This ticket follows
  the ruling on N24 as it stands; PD07's answer may change the port of S053.
