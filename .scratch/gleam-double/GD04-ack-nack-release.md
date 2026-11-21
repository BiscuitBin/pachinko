---
id: GD04
title: "Gleam double: ack, nack, release and the lease's end"
status: done
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
  - the one reading of `ending_records` among the rules. GD02 landed the derivation in
    full as `model.ending` (held, marked acked or nacked, and
    `deferred_position = null`), where the invariant checker's views read it too, so it
    stays there and this module does not repeat it;
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
   - `harness.lease_view`, GD03's stand-in over the snapshot (G31), has its body replaced
     by the owner's `LeaseView` message, so S033's and S036's ports read the view through
     the real seam. What they assert does not change.
   - GD02's checkpoint (B6) and GD03's hand-back leave a witness owing: a test showing
     that the checker runs after the whole chained outcome and not part-way. Two states
     part-way through an outcome are candidates: a publish of a new key into an idle
     pocket, before `ReviveIdlePocket` answers, and a release, before its `Freed` rows
     answer. Write a test for each that passes through the owner, and show with a scratch
     change that runs the checker before each chained event that both fail. If neither
     state breaks an invariant, say so in the hand-back.

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
- The lease view is read through the owner's `LeaseView` message, by S090's port and by
  S033's and S036's.
- The witness is quoted with its scratch change, its output and its exit status.
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

Evidence files are under `ai_tmp/gd/GD04/`, and the review record under
`ai_tmp/review/GD04-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name, or keeps only the values its
assertion compared. A failing run prints the runner's panic and the recipe's `error:`
line between `coverage faults: N` and `exit status: 1`; those are left out too, with no
mark. A quote that does not open with its `$` line starts part-way through the output.
Where a fact the record should hold was never saved, the text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `b366530168a07a5c2389f5026afc195d0e5ae83d`, which amends this ticket
  before it starts. It sits on GD03's record commit,
  `5f4a4e59f9914c196df67320792ac454bf18cc15`, which holds GD03's work,
  `8e6907e2c89b9291b0aeb96cbd6d1433b3e04985`. The amendment added the witness owed since
  GD02's checkpoint (B6), the replacement of `harness.lease_view`'s body by the owner's
  `LeaseView` message, the ruling that `ending_records` stays in `model.ending`, and two
  acceptance criteria: the lease view read through the owner by S033, S036 and S090, and
  the witness quoted.
- Work commit: `e0eddcc8ea46e3f0b7ff729d0eb0b4285e1e879b`. The commit after it holds this
  record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `b366530168a07a5c2389f5026afc195d0e5ae83d`, on the untouched worktree
(`baseline.log`, which records each status as `<name>-exit=N` and holds no `$` line; its
last line counts the lines `git status --short` printed):

```text
b366530168a07a5c2389f5026afc195d0e5ae83d
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
check-exit=0
...
155 tests run, 155 passed
coverage faults: 0
gleam-double-exit=0
       0
```

### 3. The design as it stands after GD04

This section adds to section 3 of GD03's hand-back, and through it to GD02's and GD01's,
which a later ticket's author still reads first. Sources are under
`gleam-double/src/pachinko_double/` and tests under `gleam-double/test/`. The work commit
touches none of `config.gleam`, `identity.gleam`, `inspection.gleam`, `invariants.gleam`,
`model.gleam`, `pull.gleam`, `rules.gleam` or `time.gleam`: the earlier sections still
describe them.

- **`lease_end.gleam`**, new: the one module that owns the lease's end. Its public
  interface is four items, and GD05 and GD06 are meant to add to it without changing any.
  - `Ending`, with `Release` and `Lapse`: how the lease ended.
  - `end(state: State, lease: Lease, ending: Ending, at: Timestamp) -> #(State,
    List(Event))`. It replaces the lease by id with `model.replace_lease`, its status
    `LeaseReleased(ended_at: at)` for `Release` or `LeaseExpired(ended_at: at)` for
    `Lapse`, and changes nothing else in the state. It returns one
    `Freed(lease: lease.id, record_key:, ended_at: at)` for each record of the lease's
    pocket whose status is not queued (`model.held`), sorted by position, smallest
    first (G38). It does not emit them and does not touch the pocket: the calling rule
    puts the events in its `Finish` and decides the pocket's fate. `at` is the calling
    rule's `now`, which the caller asks for. The ended lease is built from the `lease`
    argument, so the caller passes the lease as it stands in `state`. These are the only places under `src/` that
    build `LeaseReleased` or `LeaseExpired` (`ev-handback-checks.txt`).
  - `outlasts(state: State, pocket_key: String) -> Bool`: the pocket's records counted
    against `model.ending` of them, `records.count > ending_records.count`. It takes the
    pocket's key, not the pocket. Where it is `False` the counts are equal. It is the
    one reader of `model.ending` among the rules (G46).
  - `Freeing(lease: String, record_key: String, ended_at: Timestamp)`, the `Freed`
    event's input, where `lease` is the lease's id, and `freed() -> Table(Freeing)`. The
    table is named `Freed`. Its rows, in order, are `RequeueUnsettledRecord`,
    `ConsumeAckedRecord` and `DeadLetterNackedRecord`, with a comment after each where
    GD05's row goes: `SupersedeUnsettledHeldVersion` after the first,
    `ConsumeAckedVersionQueueDeferred` after the second and
    `DeadLetterNackedVersionQueueDeferred` after the third. When no guard holds it is
    `Defective`.
  - Private, for GD05's rows to reuse: `Held(lease, record, held_rank, held_payload,
    mark, nacked_at, deferred_position)`; `held(state, freeing) -> Result(Held, Nil)`,
    which finds the lease by the event's id with `model.lease`, whatever its status, then
    `model.record(pocket: lease.pocket, key: record_key)`, and fails on a queued record;
    `free(state, freeing, effect)`, which runs an effect over `held` and answers the
    defect `Impossible` where it fails; and `without(state, record)`, which removes the
    record and keeps the others' order.
  - Each guard is `held` plus the row's mark and `held.deferred_position == None`.
    GD05's three rows are the same with `deferred_position` set. No row asks for the
    time; `DeadLetterNackedRecord` stamps `expires_at = freeing.ended_at +
    dead_letter_ttl`, and appends the dead letter at the end of `dead_letters` (G19).
    It answers `Impossible` for a nacked record with no `nacked_at` (G43).
  - Nothing in the interface stops a row from reading how the lease ended: `Held`
    carries the whole lease, `status` included. The three rows read only its `pocket` and
    `consumer` (checkpoint C1, G46). The test "the Freed rules read the mark and never how
    the lease ended" covers these three rows only, so a GD05 row needs its own case.
- **`settle.gleam`.**
  - `Ack(consumer, lease, record_key)`, `Nack(consumer, lease, record_key)` and
    `Release(consumer, lease)` join `Renew(consumer, lease)`. In each, `consumer` is the
    consumer's id and `lease` the lease's id.
  - `ack()`, `nack()` and `release()` are tables named `Ack`, `Nack` and `Release`. The
    rows are `AckMarksRecord` and `AckRepeats`; `NackMarksRecord` and `NackRepeats`;
    `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket`.
  - Private helpers. `own_lease`, the lease by id where it is active and the consumer's,
    is shared by all four tables; `Renew`'s row now uses it, and `extend`, which sets
    `expires_at = now + lease_ttl`. The marks share `held` and `held_as`. `reset_expiries`
    sets `consecutive_expiries` to 0 on the pocket with the lease's pocket key (G42).
    `remark` changes the mark and `nacked_at` and keeps the held version and
    `deferred_position`.
  - The Ack and Nack tables refuse through `mark_refusal`, which reads the state as a
    guard does: `LeaseFenced`, then `RecordNotHeld`, then `MarkConflict` (G34). The
    Release table refuses `LeaseFenced` where `own_lease` fails and `ReleaseRefused`
    otherwise (G39).
  - Both release rows find the lease and its leased pocket (`leased`), split on
    `lease_end.outlasts(state, pocket.key)`, ask for `now`, and end the lease with
    `lease_end.end(state, lease, lease_end.Release, now)`. `ReleaseRequeuesPocket` asks
    for a position first, then the time. It queues the pocket at that position with its
    count reset. `ReleaseIdlesEmptyPocket` asks for no position: the pocket goes idle
    with its count reset and keeps its position (G40). Each finishes with `end`'s events.
- **`publish.gleam`.** `pocket_touched()` now holds one row, `ReviveIdlePocket`. Its guard
  is the pocket with that key being idle. Its effect asks for a position and queues the
  pocket there with its count reset. The table's answer when no guard holds is still
  `NothingHappens`. GD06's `ReviveParkedPocket` is the second row of this table.
- **`consuming.gleam`**, new: the `Consuming` surface's view.
  - `LeaseView(pocket_key, expires_at, records: List(HeldView))` and
    `HeldView(key, held_rank, held_payload: BitArray, mark: Mark)`. The view carries no
    lease id.
  - `lease_view(state, consumer) -> Option(LeaseView)`. It reads `inspection.views`: the
    consumer's first `active_leases` entry, and that pocket's `held_records`, smallest
    position first (G31, `impl-choice`). `None` when the consumer has no active lease.
- **`domain.gleam`.** `Event` gains `Freed(lease: String, record_key: String, ended_at:
  Timestamp)`. `Answer` is unchanged: an ack, a nack and a release answer nothing.
- **`refusals.gleam`.** Three more refusals, and `LeaseFenced` is now used by all four
  actions under a lease, with `action` the trigger's name.
  - `RecordNotHeld(action, lease_id, record_key)`.
  - `MarkConflict(action, lease_id, record_key)`, which carries no mark (G35).
  - `ReleaseRefused(consumer_id, lease_id)`.
- **`core.gleam`.**
  - `Trigger` gains `Ack(ack: Ack)`, `Nack(nack: Nack)` and `Release(release: Release)`.
  - `apply` refuses each of the three by a consumer that never joined with
    `UnknownConsumer`, before any table runs (G36).
  - `lease_view(state, consumer_id) -> Result(Option(LeaseView), Failure)` is new. It is
    a read: it refuses a consumer that never joined with
    `Error(Refused(UnknownConsumer(id)))`, and otherwise answers `consuming.lease_view`.
  - `chain` routes three events: `PocketTouched` to `publish.pocket_touched()`, `Handed`
    to `pull.handed()`, and `Freed` to `lease_end.freed()` as a `Freeing`. GD05's three
    rows need no routing of their own. GD06 calls `lease_end.end` and `outlasts` from its
    lapse rules, emits the returned events, and adds no route for `Freed`. Neither
    `domain.Event` nor `chain` has `LeaseLapsed` yet.
  - `conclude` is unchanged: the checker runs once, after the whole chain (G47).
  - `registry()` adds `settle.ack()`, `settle.nack()`, `settle.release()` and
    `lease_end.freed()` at the end.
- **`owner.gleam`.**
  - `Message` gains a third constructor, `LeaseView(consumer, reply)`, a read beside
    `Inspect`. The handler answers `core.lease_view(owner.state, consumer)` and keeps the
    state.
  - `ack(owner, consumer, lease, record_key) -> Reply`, `nack(...)` and
    `release(owner, consumer, lease) -> Reply` send `Act` with the new triggers. Each
    success replies `Ok(Outcome(fired, None))`. A release's `fired` is its own rule, then
    one `Freed` rule for each held record, smallest position first.
  - `lease_view(owner, consumer) -> Result(Option(LeaseView), Failure)`.
- **`test/harness.gleam`.**
  - `ack`, `nack` and `release`, each logged like `publish`.
  - `refused(double, expected, action)`. It runs `action`, requires
    `Error(Refused(expected))`, and compares the complete snapshot, the clock's read count
    and the lease-id draws before and after.
  - `at(double, millis) -> Timestamp`, which sets the clock to `t0 + millis` without
    counting a read.
  - `lease_view(double, consumer) -> Result(Option(LeaseView), Failure)` now calls
    `owner.lease_view`. Its signature changed: it hands back the owner's reply whole, so
    a test can read a refusal, and S033's and S036's ports match `Ok(None)` and
    `Ok(Some(..))`. The harness's own `LeaseView` and `MarkedRecord` are gone, replaced by
    `consuming`'s; `marked_keys` takes `List(HeldView)`.
  - `Driven(result, clock_reads, lease_id_draws)` and `drive(step, now, lease_id) ->
    Driven`, which drive a pure step for a state built by hand, answering every time with
    `now` and every lease id with `lease_id`, and count both.
- **`test/coverage.gleam`.** `pending_scenarios` holds 46 ids and `pending_rules` 11
  rules, exactly what GD05's and GD06's ownership tables list (`ev-handback-checks.txt`).
- **Test modules.** `settle_test`, `lease_end_test` and `consuming_test` are new, and
  `main` lists them in that order after `lease_test`. `core_test` holds the three witness
  tests. The run has 214 tests.

Changes to a GD01 to GD03 seam, or departures from the ticket's wording, and what each
costs:

| Before, or as the ticket words it | After GD04 | Cost |
| --- | --- | --- |
| `harness.lease_view(double, consumer) -> Option(LeaseView)` over the snapshot (G31) | The same name, over the owner's `LeaseView` message, returning `Result(Option(LeaseView), Failure)` | S033's and S036's ports match on `Ok(..)`; what they assert is unchanged |
| The lease view's types were the harness's `LeaseView` and `MarkedRecord(key, rank, payload, mark)` | `consuming.LeaseView` and `HeldView(key, held_rank, held_payload, mark)` | The field names follow the surface's `exposes` block |
| The lease read is "the owner's `LeaseView` message" | `owner.lease_view` sends it, and `core.lease_view` holds the unknown-consumer check | None recorded |
| N21's `MarkConflict` carries the mark that stands | It carries `action` instead (G35) | `refusals` cannot import `model` without a cycle |
| `Renew`'s row had its own lookup and extension | It uses `own_lease` and `extend`, shared with the marks and the release | Its impossible branch's text changed; no test pinned it |
| `PocketTouched`'s table had no row | `ReviveIdlePocket` | A revived pocket's position is above its new record's (G45) |
| `pending_scenarios` held 64 ids and `pending_rules` 21 rules | 46 and 11 | None |

### 4. Steps 2 to 4

Every output in this section ran uncommitted, on base
`b366530168a07a5c2389f5026afc195d0e5ae83d`.

Who did what, one writer at a time:

- The lead decided the seams and wrote them into two briefs, `brief-a.md` and
  `brief-b.md`. It read `lease_end.gleam` and `settle.gleam` against the rule text. It
  renamed one test helper in `lease_test.gleam` to `another_consumers_lease`, after
  `just lint`'s typos hook rejected the old name (`ev-notes-lint.txt`, exit status 1),
  and reran `just lint` (`ev-lead-lint.txt`, exit status 0) and `just gleam-double`
  (`ev-lead-green.txt`, 210 tests, exit status 0).
- Agent A, from `brief-a.md`, built the seams, the ten rows, the refusal tests, the nine
  `settle.rs` scenarios and the harness, and shrank the pending lists to 55 and 11
  (`report-a.md`).
- Agent B, from `brief-b.md`, ported the nine `lease.rs` scenarios and recorded the
  witness, then stopped at a usage limit before its mutation pass (section 5). A third
  agent, B2, wrote the runner and ran the mutation pass (`report-b.md`).
- A fourth agent wrote the spec notes: G33 to G47. It amended G04, G05, G15, G19, G22,
  G28 and G31, and put note ids in place of the `G-new` placeholders in test comments.
  None is left under `gleam-double/`, by a search that finds two in `brief-a.md`
  (`ev-handback-checks.txt`).
- At the checkpoint one runner subagent per lens drove Codex, and one fresh triage
  subagent checked the findings and applied the fixes (section 6).
- The harness refused report files from the subagents. The lead saved the replies of A
  and B2 as `report-a.md` and `report-b.md`; the first says it is word for word.

#### Step 2: the plan

`plan-change` was loaded in GD01 and applied here. Each writer checked its nine owned
scenarios against their Rust tests: none needed deferral or a lapse. So no scenario moved
under shared rule 2, and no other ticket's ownership table changed. S047's port takes the
release half, as its Rust test does (`report-a.md`).

#### Step 3: the ports, red first

Uncommitted, on base `b366530168a07a5c2389f5026afc195d0e5ae83d`.

The scenarios, by module, each under its `Scenario` tag:

| Module | Scenarios |
| --- | --- |
| `settle_test` | S040, S041, S042, S047, S049, S084, S085, S086, S090 |
| `lease_test` | S023, S035, S051, S052, S053, S055, S057, S068, S070 |

The `impl-choice` labels, each in its test's comment:

- S040's absolute position (G33).
- The refusals' names and shapes, and the order the lease is checked first, in S042,
  S049, S052, S057, S068, S070 and S085 (G28, G34 and G35).
- The refusal of a consumer that never joined, for an ack, a nack, a release and the
  lease read: S042, S049 and three untagged tests (G36).
- The lease view's record order in S036, S085 and S090 (G31).
- The order of one lease end's `Freed`, where two hand-built tests assert it (G38).
- The hand-built state that reaches `ReleaseRefused` (G39).

The Rust assertions not ported literally, each with its reason:

- The answers of a mark and a release. Each port asserts `Ok(Outcome(fired, None))` and
  reads the state from the snapshot (G37, the ruling on N24).
- S053's empty lease registry. The port asserts the lease is in the snapshot, released,
  with its `ended_at`, that no lease is active, and that no consumer has a lease view
  (G37).
- S055's and S057's ended lease is read from the snapshot, not from a release's return.
- The Rust's `LeaseNotActive` and `LeaseNotYours` are both `LeaseFenced` (G34).
- S040's `keys_of` is the pocket's keys, sorted by key (`report-a.md`).

The scaffold run, `ev-a-scaffold.txt`, with the seams in and no new test. S036's port
failed: its `let assert Ok(Some(before)) = harness.lease_view(...)` met `Ok(None)`.

```text
$ just gleam-double
...
FAIL pull_test.a pull under a lease extends the lease: crashed: ... #(Value, Ok(None)) ...
COVERAGE S036 is not in pending_scenarios, and no test tagged Scenario("S036") passed
155 tests run, 154 passed
coverage faults: 1
exit status: 1
```

The first red, `ev-a-red-first-scaffold.txt`, with agent A's tests written and its rows
not yet in. 34 tests failed; `red1-fails.txt` lists them.

```text
$ just gleam-double
...
FAIL consuming_test.a lease read by a consumer that never joined is refused: crashed: ...
COVERAGE S036 is not in pending_scenarios, and no test tagged Scenario("S036") passed
192 tests run, 158 passed
coverage faults: 1
exit status: 1
```

A middle red, `ev-a-red-mid-marks.txt`, with the mark rows in and the `Freed` rows not:
`192 tests run, 176 passed`, `coverage faults: 7`, exit status 1. The seven faults are
S040, S041, S042, S049, S084, S085 and S090, each still pending and now passing.

The last red, `ev-a-red-last-revive.txt`, before `ReviveIdlePocket`:

```text
$ just gleam-double
...
FAIL publish_test.ReviveIdlePocket changes exactly what it ensures and reads no clock: crashed: ...
192 tests run, 191 passed
coverage faults: 0
exit status: 1
```

What the pending lists held at the last red run is not recorded.

Agent A's green run, `ev-a-green.txt`: `196 tests run, 196 passed`, `coverage faults: 0`,
exit status 0.

Never red, and why. `report-a.md` names agent A's eight by short descriptions; the full
names below are the registry's tests that match them. The mutants named are those whose
log under `b-scratch/logs/` holds the test's `FAIL` line (`ev-handback-checks.txt`).

| Test | Why never red | Failed by |
| --- | --- | --- |
| "a consumer with no active lease has no lease view" | Green on the empty scaffold | L1, L2 |
| "a Freed no rule answers is a defect" | Green on the empty scaffold | No mutant |
| "a release that names another consumer's lease or no lease is fenced" | Green on the empty scaffold | F4, F5, RL1, V3 |
| "a touched pocket that is queued or leased, and a key no pocket has, are left as they were" | Green on the empty scaffold | V3 |
| "a mark rule's effect without its held record is a defect, and asks for no time" | Written after its rows | No mutant |
| "a release rule's effect without its lease on a leased pocket is a defect, and asks for no time" | Written after its rows | F6, RL1 |
| "a Freed rule's effect without what its guard saw is a defect" | Written after its rows | RU4 |
| "ReviveIdlePocket's effect without its pocket is a defect" | Written after its row | No mutant |

Agent B's nine ports were written against rows already landed, so none was red. Each is
failed by the mutant `report-b.md` names, which `ev-handback-checks.txt` confirms from
the logs: S023 by V1, S035 by RQ4, S051 by R1, S052 by F6, S053 by RQ8, S055 by RI5,
S057 by F4, S068 by K1 and S070 by F5.

The mutation pass, after green (`ev-b-mutants.txt`, which holds no `$` line; runner
`b-scratch/runner.py`, definitions `b-scratch/mutants.py`, one log for each mutant under
`b-scratch/logs/`). Its totals:

```text
Totals: 113 run; 111 killed by a named test; 2 survived (D13, L3e); 0 killed only by the build or the coverage check.
First pass: 104 killed, 2 survived (D13, L3e), 7 failed only the warnings-as-errors build (F3, F4, F7, E6, RU4, U8, RQ11), rewritten to compile and rerun: all killed.
```

- The 113 are the 108 agent B defined and five B2 added: RQ11, RI9, RL1, K1 and V8. They
  cover the ten rows, the refusals' order, `lease_end.end`, `lease_end.outlasts`,
  `consuming.lease_view` and the unknown-consumer refusals.
- Every one of the 113 logs records `210 tests run`.
- The two survivors are equivalent. D13: `model.record` finds the record by
  `pocket = lease.pocket`, so `held.record.pocket` always equals `held.lease.pocket`.
  L3e: `held_records` is `model.held(records)`, which is what the view's `filter_map`
  keeps.
- Every file under `src/` at the work commit has the SHA-256 that
  `b-scratch/manifest-src.sha256` records before the first mutant
  (`ev-handback-checks.txt`).

The witness. GD02's checkpoint (B6) and GD03's hand-back left one owing: a test showing
that the checker runs after the whole chained outcome and not part-way. Three tests in
`core_test`, each driving the owner and carrying no tag, are the witness (G47). Two
scratch changes to `core.gleam` run the checker part-way. `ev-b-witness.txt` holds no `$`
line of its own: it is the agent's digest, with each change's diff, each `FAIL` line cut
to its assertion, a count of `FAIL` lines and the recipe's last lines. Each whole output
is in `ev-b-witness-<change>-full.txt`. Both ran at 209 tests, before agent B's last
test.

`broad` runs the checker before every chained event's table:

```text
=== scratch change: broad ===
...
-/// Sends an emitted event to its table.
+/// SCRATCH (the witness): the checker also runs before each chained event's
+/// table, over the state part-way through the outcome.
 fn chain(event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
+  case invariants.check(inspection.snapshot(rules.state(cursor))) {
+    [] -> route(event, cursor, then)
+    broken -> rules.fail(BrokenInvariant(broken))
+  }
+}
...
+/// Sends an emitted event to its table.
+fn route(event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
   case event {
     PocketTouched(pocket_key) ->
       rules.run(

$ just gleam-double   (whole output: ai_tmp/gd/GD04/ev-b-witness-broad-full.txt)
FAIL core_test.a publish of a new key into an idle pocket stands, though the pocket is idle with a record until the revival answers: assertion failed in idle at line 186; left value: Error(BrokenInvariant(["IdleHasNoRecords", "OutstandingOnlyWhileLeased"]))
FAIL core_test.a release that requeues its pocket stands, though the pocket is queued with a held record until Freed is answered: assertion failed in freed_mends_the_requeue at line 214; left value: Error(BrokenInvariant(["OutstandingOnlyWhileLeased"]))
FAIL core_test.a release that idles its pocket stands, though the pocket is idle with a held record until Freed is answered: assertion failed in idle at line 186; left value: Error(BrokenInvariant(["IdleHasNoRecords", "OutstandingOnlyWhileLeased"]))
FAIL lines in all: 18
...
exit status: 1
```

Under `broad` the publish test fails in its setup, `idle`, whose release breaks both
invariants, and not at the publish. The whole output ends `209 tests run, 191 passed`,
`coverage faults: 13`. So `touched` runs the checker before `PocketTouched`'s table
alone:

```text
=== scratch change: touched ===
...
+/// SCRATCH (the witness): the checker also runs before `PocketTouched`'s
+/// table, and before no other event's.
 fn chain(event: Event, cursor: Cursor, then: fn(Cursor) -> Step) -> Step {
+  case event, invariants.check(inspection.snapshot(rules.state(cursor))) {
+    PocketTouched(_), [_, ..] as broken -> rules.fail(BrokenInvariant(broken))
+    _, _ -> route(event, cursor, then)
+  }
+}
...

$ just gleam-double   (whole output: ai_tmp/gd/GD04/ev-b-witness-touched-full.txt)
FAIL core_test.a publish of a new key into an idle pocket stands, though the pocket is idle with a record until the revival answers: assertion failed in revival_mends_the_publish at line 199; left value: Error(BrokenInvariant(["IdleHasNoRecords"]))
PASSED under this change: core_test.a release that requeues its pocket stands
PASSED under this change: core_test.a release that idles its pocket stands
FAIL lines in all: 3
...
exit status: 1

core.gleam restored from its copy and touched; sha256 9254a79d6dc8def3a15638d9a3460d31e7ed3bb7fe688a19c2d6ceb4223a9958 before and after
```

The whole output ends `209 tests run, 206 passed`, `coverage faults: 2`. So the publish
witness stands on `touched`, and the two release witnesses on `broad`. `core.gleam` at
the work commit has the same SHA-256 (`ev-handback-checks.txt`).

The refusals (`report-b.md`, section A2). B2 checked that each refusal this ticket lands
has a test through `harness.refused`, and added none:

- `LeaseFenced` for an ended lease, for another consumer's lease and for an id no lease
  has, for each of ack, nack, renew and release;
- `RecordNotHeld` for an absent key, a queued one and a key held in another pocket;
- `MarkConflict` both ways, in S085, with no clock read;
- `UnknownConsumer` for an ack, a nack, a release and the lease read;
- a second release, in S057.

`ReleaseRefused` is the exception. It is reached only over hand-built states through the
apply step, where `core.apply` answers `Done` at once with no state, no clock read and
no draw (G39). No test reaches it through the owner, as none reaches `PullRefused`.

The green run before the checkpoint, `ev-b-green.txt`, and again after the lead's lint fix,
`ev-lead-green.txt`, each end:

```text
210 tests run, 210 passed
coverage faults: 0
exit status: 0
```

No ported scenario failed against rows that follow the specification, and dispatch
returned no `GuardOverlap` on the Ack, Nack, Release, `Freed` or `PocketTouched` table. On
exit `pending_scenarios` holds 46 ids and `pending_rules` 11 rules.

#### Step 4: spec notes

`gleam-double/SPEC-NOTES.md` gained G33 to G47. Each cites its `N`-note or says it has
none: the notes from G33 on hold fifteen `N`-note lines (`ev-handback-checks.txt`).

| Note | Title | `N`-note |
| --- | --- | --- |
| G33 | S040 asserts an absolute position (`impl-choice`) | No `N`-note |
| G34 | the refusals of a mark are `LeaseFenced`, `RecordNotHeld` and `MarkConflict`, the lease checked first | Agrees with N21, but for one field of `MarkConflict` |
| G35 | `MarkConflict` carries no mark | Departs from N21 |
| G36 | an ack, nack, release or lease read by a consumer that never joined is refused (`impl-choice`) | Agrees with N22 |
| G37 | an ended lease stays in the state, and a mark and a release answer nothing (the maintainer's ruling on N24) | Agrees with N24 and follows its ruling |
| G38 | one lease end sends `Freed` smallest held position first (`impl-choice` where a test asserts it) | Agrees with N25 |
| G39 | the Release table's refusal past fencing is unreachable through the owner | Agrees with N26 |
| G40 | an idle pocket keeps its position | Agrees with N27 |
| G41 | `ReleaseIdlesEmptyPocket` holds for a leased pocket with no record | No `N`-note |
| G42 | the mark rules require nothing of `lease.pocket` | No `N`-note |
| G43 | a nacked record without `nacked_at` cannot become a dead letter | No `N`-note |
| G44 | a dead letter's `held_rank` and the record's `rank` cannot be told apart in a state that holds the invariants | No `N`-note |
| G45 | a revived pocket's position is above its new record's | No `N`-note |
| G46 | one module owns the lease's end, and `ending_records` stays in `model` | No `N`-note |
| G47 | the witness that the checker runs once, after the whole chained outcome | No `N`-note |

- Step 4's candidates: which refusal an ack of a queued record is (G34); whether a
  refusal kind is the specification's to name (G34 and G35); S090's order (G31, whose
  title now names S036, S085 and S090); and whether the idle pocket's kept position
  matters (G40).
- N28, that a refusal reads no clock, needs no note of its own: `harness.refused` checks
  it for every refusal, and G05 was amended to say so.
- The notes' `just lint` failed on the test helper's name (`ev-notes-lint.txt`), which
  the lead then fixed (above). `ev-notes-green.txt` is `just gleam-double` at 210
  tests, exit status 0.
- The checkpoint then corrected G40, G43 and G46 (section 6).

### 5. An agent stopped at a usage limit, and the model changed

**Agent B stopped part-way.** By `report-b.md`, the lead's account, agent B stopped at a
usage limit after it had ported the nine `lease.rs` scenarios and recorded the witness
(`ev-b-witness.txt`). It had written 108 mutant definitions, and no runner and no
results. The limit's message and the time it stopped are not recorded.

- Agent B kept its backups and scripts in the session's scratch directory, outside the
  worktree: the witness's diff names `.../scratchpad/gd04b/backup/src/pachinko_double/core.gleam`.
  The lead copied what was there to `ai_tmp/gd/GD04/b-scratch/`. Of what is there now,
  `mutants.py`, `witness.py` and two manifests of `src/` and `test/`, `manifest.sha256`
  and `manifest-c.sha256`, date from the copy. The two manifests differ only in
  `test/lease_end_test.gleam`. Which of the two is agent B's and which the lead's check
  is not recorded. `runner.py`, `manifest-src.sha256`, `run1.tsv`, `run2.tsv` and
  `logs/` are B2's, written later.
- Agent B's last edit was the test "each Freed rule's guard holds only for a held record
  with its mark and nothing deferred behind it", in `lease_end_test.gleam`. It was kept.
  B2's pass shows it is the only test that kills RU3, C4 and D11, three mutants that drop
  `deferred_position = null` from a `Freed` guard.
- The lead then ran `just gleam-double` once over the tree as agent B left it
  (`ev-lead-after-b-stop.txt`), uncommitted, on base
  `b366530168a07a5c2389f5026afc195d0e5ae83d`:

  ```text
  $ just gleam-double
  ...
  210 tests run, 210 passed
  coverage faults: 0
  exit status: 0
  ```

- A fresh agent, B2, was briefed from what was left. It wrote the runner, added five
  mutants, ran the 113 and checked the refusals. It changed nothing under
  `gleam-double/` (`report-b.md`). `src/` at the work commit matches both
  `manifest-src.sha256`, which B2 took before its first mutant, and the `src/` lines of
  the copied manifests, so agent B's `src/` and the mutated `src/` are the same
  (`ev-handback-checks.txt`).

**The model changed.** By the lead's account, the user switched the session's model from
Claude Fable 5.2 to Claude Opus 5.5 after agent B stopped. The only evidence is the
commits' `Co-Authored-By` trailers (`ev-handback-checks.txt`): GD03's work and record
commits and this ticket's base carry Claude Fable 5.2, and the work commit
`e0eddcc8ea46e3f0b7ff729d0eb0b4285e1e879b` carries Claude Opus 5.5. The time of the
switch, and which agents ran on which model, are not recorded.

### 6. Checkpoint record

One checkpoint, at step 5: conformance and breakage, before the final commit.

- **`codex exec --help` on the day.** `ev-codex-help.txt` holds its output, which lists
  `-m, --model`, `-s, --sandbox` with `read-only`, `-C, --cd` and
  `-o, --output-last-message`, the flags the corrected command uses, and reads the prompt
  from stdin when none is given. It ends:

  ```text
  ...
    -V, --version
            Print version
  exit status: 0
  2026-11-21T06:09:24Z
  ```

- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `-s read-only`, one fresh
  process per lens, by the ticket's corrected command (`log-conformance.txt` and
  `log-breakage.txt` each open with those lines). Codex was on `PATH`, so no fallback to a
  Claude reviewer was needed.
- **Recovery.** Both findings files open `Source: last-message`: each is Codex's last
  message, recovered because a read-only Codex writes no file.
- **What the reviewers ran.** Nothing. Each says its evidence was read-only inspection.
  The triage ran each claim.
- **Triage.** One fresh subagent, neither a reviewer nor an author.
- **Rounds.** One. No finding was rated high, by a reviewer or by the triage, so no
  second round was owed.
- **Not recorded.** The model of the runner and triage subagents.

It ran uncommitted, on base `b366530168a07a5c2389f5026afc195d0e5ae83d`. Before any fix
`just gleam-double` ended `210 tests run, 210 passed`, `coverage faults: 0`, exit status 0
(`ev-baseline.txt`). The reviewed tree and the triaged tree are the same,
`0db7735c1eecbc640a47a554041b697e28601f60` (`tree-before-review.txt` and
`tree-before-triage.txt`). The conformance lens filed C1 to C3 and the breakage lens B1
to B5.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid | `lease_end.held` hands each row the whole `Lease`, status included, so `Freeing`'s lack of a field does not stop a row from reading how the lease ended | G46 says the interface does not prevent the read and the rows do not make it |
| C2 | low | valid | S055's port asserts the pocket's position but never `next_position`; only `idles_exactly` compares the counter | G40 attributes each assertion to its test |
| C3 | low | valid | `DeadLetterNackedRecord` has four `requires`, not three | G43 names all four |
| B1 | medium | valid | Two mutants, the held payload rewritten from the record and a mark guard requiring nothing deferred, passed all 210 tests | New `settle_test` test `deferred_marks`, with a helper `deferred_with` |
| B2 | medium | valid | An `outlasts` that counts only queued and unsettled records passed all 210 tests | Three deferred cases in `lease_end_test`'s `outlasts` and `lease_test`'s `release_guards`, through dispatch only |
| B3 | low | valid | A lease extension that turns a zero `lease_ttl` into 1 passed all 210 tests | New `settle_test` test `zero_lease_ttl` |
| B4 | low | valid | A dead letter stamped with `int.max(1, dead_letter_ttl)` passed all 210 tests | New `lease_end_test` test `dead_letter_zero_ttl` |
| B5 | low | valid | `ReviveIdlePocket`'s guard widened to parked pockets passed all 210 tests | New `publish_test` test `revive_guard_not_parked`, on the guard alone |

All eight were valid, and all eight were applied. Nothing was handed back. The triage
changed five files, all in this ticket's Files touched: `SPEC-NOTES.md` (G40, G43 and
G46), `settle_test.gleam`, `lease_end_test.gleam`, `lease_test.gleam` and
`publish_test.gleam`. Nothing under `gleam-double/src/` changed, no seam changed, and no
ported scenario's assertion changed. No new test carries a tag, so the pending lists did
not change.

On the questions step 5 puts to the breakage lens, the reviewer lists what it examined
with no further finding. It found every refusal compared over the complete snapshot and
both counters, and a dead letter's `nacked_at` and `expires_at` told from every other
time. On the checker running part-way through a release, it cites the witness.

Each mutant ran once on the reviewed tree and once after the fixes, through
`scratch/mutant.sh`. Each evidence file ends with the mutant's diff and a SHA-256 line
for the restored file, and the two hashes match in all twelve runs (`verdicts.md`):

| Finding | Before the fixes | After the fixes |
| --- | --- | --- |
| B1, payload | `ev-B1-payload-survives.txt`: 210 of 210 passed, exit 0 | `ev-B1-payload-killed.txt`: 213 of 214, exit 1 |
| B1, guard | `ev-B1-guard-survives.txt`: 210 of 210, exit 0 | `ev-B1-guard-killed.txt`: 213 of 214, exit 1 |
| B2 | `ev-B2-outlasts-survives.txt`: 210 of 210, exit 0 | `ev-B2-outlasts-killed.txt`: 212 of 214, exit 1 |
| B3 | `ev-B3-lease-ttl-survives.txt`: 210 of 210, exit 0 | `ev-B3-lease-ttl-killed.txt`: 213 of 214, exit 1 |
| B4 | `ev-B4-dead-letter-ttl-survives.txt`: 210 of 210, exit 0 | `ev-B4-dead-letter-ttl-killed.txt`: 213 of 214, exit 1 |
| B5 | `ev-B5-revive-survives.txt`: 210 of 210, exit 0 | `ev-B5-revive-killed.txt`: 213 of 214, exit 1 |

- The first B2, B3 and B5 attempts failed the recipe's format check before any test ran.
  They were rerun with `just gleam-double-format` first, and the rerun overwrote their
  `*-survives.txt` files.
- B2's new cases go through `outlasts` and dispatch only. A release through the apply step
  would chain a `Freed` for a deferred record, whose row is GD05's and pending (Open
  points).
- B3's first form used a pocket with two expiries, and the checker rejected the repeat's
  input with `MarkedPocketHasNoExpiries` (`verdicts.md`).
- C1's read is shown by `ev-C1-lease-reads.txt`, a `grep` of `lease_end.gleam` for
  `lease.` and `held.lease`, exit status 0.

`ev-green-after-tests.txt` is `just gleam-double` after the test fixes and before the kill
runs: `214 tests run, 214 passed`, `coverage faults: 0`, exit status 0. `ev-lint.txt` is
`just lint` after the note edits, exit status 0.

Tree ids: as reviewed and before the triage `0db7735c1eecbc640a47a554041b697e28601f60`,
after it `c61a8d46ce717b9571fa04674fd15e72a6a1f203` (`tree-after-triage.txt`). The work
commit's tree is that second id (`ev-handback-checks.txt`), so the re-run below ran on
the tree that was committed. `just check` did not run between the triage and the commit:
it ran at the work commit (section 7).

The re-run after the checkpoint (`ev-final.txt`), uncommitted, on base
`b366530168a07a5c2389f5026afc195d0e5ae83d`:

```text
$ just gleam-double
...
214 tests run, 214 passed
coverage faults: 0
exit status: 0
```

### 7. Verification

The Verification block ran at `e0eddcc8ea46e3f0b7ff729d0eb0b4285e1e879b`, the work
commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's block word
for word; output in `ev-verification.txt`). It exited 0, so its last two lines held:
`git status --short` printed nothing.

```text
$ sh ai_tmp/gd/GD04/verification.sh
e0eddcc8ea46e3f0b7ff729d0eb0b4285e1e879b
...
gleam 1.19.1
...
OTP 29
...
214 tests run, 214 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The run counted 155 tests at the base (`baseline.log`), 196 at agent A's green run
(`ev-a-green.txt`), 210 after agent B stopped (`ev-lead-after-b-stop.txt`) and 214 after
the triage's four (`ev-final.txt`).

### 8. The acceptance criteria against the evidence

1. `just gleam-double` exits 0 and shows the counts: `ev-verification.txt`.
2. The pending lists hold exactly the 46 ids and the 11 rules GD05 and GD06 own:
   `ev-handback-checks.txt`, which compares the lists at the work commit with the two
   tickets' ownership tables. `findings-conformance.md` says the same.
3. Every owned scenario has a tagged test, and each of the ten rules is a row that
   fired: `coverage faults: 0` in `ev-verification.txt`, with none of the 18 ids or the
   10 rules pending. `ev-handback-checks.txt` finds one tag for each of the 18.
4. Both release rows end the lease only through the lease-end module: both call
   `lease_end.end` (section 3), and `ev-handback-checks.txt` finds `LeaseReleased(` and
   `LeaseExpired(` built under `src/` only in `lease_end.gleam`. The same search also finds
   the constructors' definition in `model.gleam` and a pattern in `invariants.gleam`, so
   it sees the tree. Mutants RQ1 and RQ2, which leave the lease active or end it expired, are
   killed (`ev-b-mutants.txt`).
5. Every refusal this ticket lands has a test showing the snapshot unchanged, and S085's
   test shows no clock read: section 4, "The refusals", through `harness.refused`, which
   compares the snapshot, the clock's reads and the lease-id draws. One falls short:
   `ReleaseRefused` is shown over hand-built states through the apply step, which hands
   back no state, not through the owner's snapshot (G39).
6. The lease view is read through the owner's `LeaseView` message by S090's port and by
   S033's and S036's: `ev-handback-checks.txt` finds `harness.lease_view` calling
   `owner.lease_view`, and the three ports calling `harness.lease_view`. Mutants L1 to
   L10 of `consuming.lease_view` are killed, but for the equivalent L3e.
7. The witness is quoted with its scratch changes, its output and its exit status:
   section 4.
8. The spec notes are there, each with its `N`-note line, S090's `impl-choice` among them
   in G31: section 4, step 4, and `ev-handback-checks.txt`.
9. `just gleam-double` leaves nothing Git can see and no `erl_crash.dump`:
   `ev-verification.txt`. The script tests both under `set -eu` and exits 0.
10. The checkpoint is recorded in section 6, with the `codex exec --help` check on the
    day.
11. `just check` ends with its line and `git status --short` prints nothing:
    `ev-verification.txt`.

### 9. Stops

None fired. The conditions watched:

- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did.
- A `GuardOverlap` on a specification table (shared rule 4). Dispatch returned none.
- A scenario that needs an operation this ticket does not land (shared rule 2). None
  moved.
- A count that moved. The coverage check asserts 93, 32 and 35 on every run, and every
  green run ends `coverage faults: 0`.
- A reach for the network. Nothing was fetched and no network recipe ran. The review
  checkpoint called a model API, which the checkpoint authorises.

Step 6's stop stands: nothing was pushed and no pull request was opened.

### 10. What was not run

- Any push, pull request or issue.
- `just initialize`, `just gleam-double-deps` and `just lock`.
- A CI run.
- A second review round. None was owed.
- The mutation pass after the checkpoint. The 113 mutants ran at 210 tests. `src/` is the
  same at the work commit (`ev-handback-checks.txt`), and the triage only added tests,
  so each kill still names a test that exists; the pass was not repeated at 214.
- The witness after agent B's last test or after the checkpoint. It ran at 209 tests.
  `core.gleam` is the same at the work commit.
- A release of a held record with a deferred version through the apply step (checkpoint,
  B2; Open points).
- A test of what `PocketTouched` does for a parked pocket (checkpoint, B5; Open points).
- `ReleaseRefused` through the owner (G39).
- `just check` between the triage and the commit (section 6).

Not recorded: the usage-limit message and the time agent B stopped; the time of the
model switch; the model of the runner, triage and author subagents; which of the two
copied manifests is agent B's; and what the pending lists held at the last red run.

### 11. Working lessons for GD05 to GD07

- A subagent can stop part-way at a usage limit. Scratch kept in the session's scratch
  directory can be lost with the session, so a writer's backups and scripts belong under
  `ai_tmp/`. To recover, check the tree against the backups, run the recipe once, and
  brief a fresh agent from what is left.
- A subagent cannot write a report file. The lead saves its reply.
- An idle notice can cut a reply short. Ask the finished agent for the rest, telling it
  to edit and run nothing, and only when no other writer is live.
- GD05's three `Freed` rows can reuse `held`, `free` and `without` in `lease_end.gleam`,
  and need a case of their own in "the Freed rules read the mark and never how the lease
  ended" (section 3).

## Open points

- **PD07 asks whether an ended lease is part of the observable state.** This ticket
  follows the ruling on N24 as it stands (G37); PD07's answer may change the port of
  S053. PD07 is still todo, and is not a blocker.
- **`MarkConflict` departs from N21 (G35).** N21's carries the mark that stands; this
  double's carries the action, because `refusals` cannot import `model` without a cycle.
  A caller learns the same from either. The maintainer may want the two doubles to agree.
- **The questions the notes leave for the maintainer.**
  - G33, as G04, G16 and G29: is `next_position`'s starting value meant to be
    unspecified, and if so should S040 assert an absolute position?
  - G34, N21's: is a refusal's kind the specification's to name, with the order it is
    checked in? And is an ack of a queued record of the leased pocket refused as a key
    not held, or as a refusal of its own?
  - G36, N22's: should the specification say that each action and the lease read
    require a joined consumer?
  - G38, N25's: should the specification order the entries one lease end creates, or say
    that their order is not observable?
  - G39, N26's: is the release rules' `requires: pocket.status = leased` a guard, or a
    restatement of `UnleasedHasNoActiveLease`?
  - G40, N27's: does an idle pocket's position mean anything?
  - G41: should the comment above `ReleaseIdlesEmptyPocket` say that a pocket with no
    record idles too?
  - G42: are the release rules' `requires: pocket.status = leased` and the mark rules'
    silence meant to differ?
  - G43: should `DeadLetterNackedRecord` state `requires: record.nacked_at != null`, or
    is `NackedHasItsTime` meant to carry it?
  - G45: is it intended that a revived pocket's position is above its new record's?
  - G46: is the record count against `ending_records` meant to be the same comparison in
    each of the four rules that write it out?
  - G47: should the Lease end comment's "one outcome" sentence also cover a publish and
    the revival it causes, whose part-way state breaks `IdleHasNoRecords`?
- **Two mutants survive as equivalent (D13, L3e).** No test can tell either from the
  code, so none was added: D13 because `model.record` finds the record by the lease's
  pocket, L3e because `held_records` already holds only held records. A change to
  `model.record` or to the snapshot's derived sets would make them observable.
- **Three tests were never red, and no mutant fails them.** "a Freed no rule answers is
  a defect", "a mark rule's effect without its held record is a defect, and asks for no
  time" and "ReviveIdlePocket's effect without its pocket is a defect" fail under none of
  the 113 mutants (section 4). Each pins a defect over a state built by hand, and no
  evidence file shows that any of the three can fail.
- **No release of a deferred record runs end to end (checkpoint, B2).** The deferred
  cases of `outlasts` and the release guards go through dispatch only, because the
  `Freed` rows for a deferred record are GD05's. GD05 can drive those releases through
  the owner.
- **What `PocketTouched` does for a parked pocket is pinned by no test (checkpoint,
  B5).** Only `ReviveIdlePocket`'s guard is shown to fail for one. `ReviveParkedPocket`
  is GD06's.
- **A higher rank published onto a held record still meets `EqualRankRefused` until
  GD05 (G13).** `report-a.md` says so and keeps such publishes out of every setup. The
  input is `DeferBehindHeldVersion`'s, which is pending, so no test pins it as a refusal.
- **A lease-id source that repeated an id would give two leases one id (G27).** Carried
  from GD03. The double still does not check that an id it is handed is new, and this
  ticket's fencing and `Freed` rows find their lease with `model.lease`, as `Renew` did.
  No test repeats an id. G10 and G27 ask whether the specification should say that two
  leases never share one.
