---
id: GD03
title: "Gleam double: pull and the lease"
status: done
depends_on: [GD02]
parallel_with: []
branch: build-gleam-double
estimated_size: M
---

# GD03: Gleam double: pull and the lease

## Context

GD01 and GD02 landed the Gleam double's foundation under `gleam-double/`: the owner
actor, the pure core, the entity model, `Config`, the joins, publish, the inspection
seam, the full invariant checker and the coverage check with its pending lists. Read
their hand-backs for the design as settled, and read `gleam-double/SPEC-NOTES.md` before
adding to it. `.scratch/gleam-double/README.md` is the series' index.

This ticket lands the consumer's first half: a pull that claims the front pocket, pulls
within a lease, batches that are oldest-first and capped, `NoPocketAvailable`, the held
set and renew. It is also the first ticket in which two callers compete for one thing,
the front pocket, so it is where the single owner first earns its place. The
specification facts it rests on, all from `docs/specs/pachinko.allium` (the Pull, and
Renew and release, sections):

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
  `expires_at = now + lease_ttl`. Its refusals (S052, S070) land in GD04 with fencing, but
  its guard is written in full here.
- A batch names its lease. A lease has an id, which the specification does not declare
  (N05), drawn from the injected lease-id source.

What the Python double already read, which this ticket takes unless the specification
contradicts it:

- A batch names its lease and its pocket by key, and carries no expiry (N13).
- The Pull table's refusal is unreachable through the owner (N14).
- A pull draws a lease id only when it creates a lease (N15).
- A pull or a renew by a consumer that never joined is refused, `impl-choice` (N18).
- A pull whose limit is not positive still claims a pocket (N19).
- Renew's refusal is `LeaseFenced`, and an unknown lease id meets it (N20).

S037, which publishes a higher rank of a key while it is held, needs
`DeferBehindHeldVersion`, so GD05 owns it.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/pull.rs`,
`publish.rs`, `identity.rs` and `lease.rs`, found with `rg -n 'scenario: S0NN'`. The
`wire` rows here are S019, S020, S028, S031, S033 and S050; port only their library half.

## Goal

A consumer can pull through the owner, be handed the oldest queued records of the front
pocket under a new lease, pull again under that lease, renew it, and be told
`NoPocketAvailable` when there is nothing to claim. Consumers that pull at the same
moment are each given a different pocket or none, never the same one. The checker holds
after each of these, and every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (5) | `PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord`, `PullFindsNoPocket`, `Renew` |
| Scenarios (18) | S001, S008, S012, S015, S019, S020, S022, S027, S028, S029, S030, S031, S032, S033, S036, S038, S039, S050 |

## Non-goals

- No ack, nack, release, lease end, fencing refusal, deferral, lapse or decay. GD04 to
  GD06 land them.
- No change to GD01's or GD02's design beyond what this ticket's rules need. A change to
  a shared seam is reviewed at this ticket's checkpoint and recorded in the hand-back.
- No new process. The concurrency tests add callers, not owners.
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

  `<id>` is `GD03`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `gleam-double/src/` | shared, own lines | Pull, the held set and renew: `pull.gleam`, `settle.gleam` with its Renew table, and the owner's `Pull` and `Renew` messages |
| `gleam-double/test/` | shared, own lines | The pull, lease and concurrent-pull tests; the pending lists in `coverage.gleam` |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/gleam-double/GD03-pull-and-the-lease.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/GD05-deferral.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/gleam-double/README.md` | shared, own lines | GD03's Status cell, and any move |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD02, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the Pull, and Renew and release,
   sections of `docs/specs/pachinko.allium`, the `OldestFirst`, `ExclusivePocket` and
   `OneVisitPerKeyPerLease` guarantees, N13 to N20 in `double/SPEC-NOTES.md`, and the
   catalog rows this ticket owns. Confirm that each owned scenario needs only pull, renew
   and what GD01 and GD02 landed. Move any that does not, under shared rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test: the Pull and `Handed` tables in `pull.gleam`, and the Renew table in
   `settle.gleam`, which this ticket creates. Points to watch:
   - The batch is computed before `HoldRecord` moves its records, and the checker runs
     only after every `Handed` has been answered.
   - A limit that is zero or negative (S031, S032) yields an empty batch and still claims
     the pocket (N19).
   - `PullClaimsFrontPocket` is the first row that draws a lease id. Assert through the
     fake lease-id source that it drew one, and that `PullWithinLease`,
     `PullFindsNoPocket` and a refused pull drew none (N15).
   - A pull by an unknown consumer is refused, and the refusal is `impl-choice` (N18).
   - S019's large ranks: record how Erlang's unbounded integers relate to the
     specification's `Integer` (N03, N17).
   - S020's payload bytes and keys round-trip unchanged under the representation GD02
     chose.
   - `PullWithinLease` renews the lease on an empty batch too. The Python series missed
     this until its review; pin it with a drained pocket and with limits of 0 and -1.

   Shrink `pending_scenarios` and `pending_rules` by exactly what landed.
4. **The competing callers.** These tests carry no scenario tag, since no catalog row
   covers them. Each starts one owner and several caller processes that send at once, and
   asserts only what holds in every order the mailbox could take:
   - several consumers pull while one pocket is queued: exactly one is handed a batch
     under a lease, and each of the others is told `NoPocketAvailable`;
   - as many consumers as queued pockets pull at once: each holds a different pocket, no
     two leases share a pocket or a consumer, and the pockets claimed are the queued ones;
   - the snapshot after each such test passes `check`.

   Quote one scratch break: make the owner answer a `Pull` from the state it held before
   the previous message, show a competing-callers test failing, and revert it.
5. **Spec notes.** Add a note wherever pull or renew forced an interpretation, each
   citing its `N`-note. Candidates are a zero-limit pull that grants a lease on a pocket
   with records but hands none, and whether `PullWithinLease`'s
   `requires: pocket.status = leased` can ever fail while its lease is active. Add one
   note on what the competing-callers tests show about the specification's "one
   serialized stream": which guarantees they exercise, and what they cannot show.
6. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while the batch is not
   oldest-first, while it exceeds `max_pull_batch`, while a second consumer could be
   handed a leased pocket's record, or while a competing-callers test asserts something
   true in only one mailbox order.
7. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- On exit, `pending_scenarios` holds exactly the 64 ids GD04 to GD06 own, and
  `pending_rules` the 21 rules they own.
- Every owned scenario has a tagged test, and each of the five rules has its row in the
  Pull, `Handed` or Renew table, fired in at least one test.
- A pull's batch is asserted through the owner's answer. Its lease's `granted_at` and
  `expires_at`, and the held fields of each handed record, are asserted through the
  snapshot.
- The fake lease-id source's draw count shows that only `PullClaimsFrontPocket` draws.
- Step 4's tests pass, and its scratch break is quoted with its output and exit status.
- `gleam-double/SPEC-NOTES.md` holds this ticket's notes, including the unknown-consumer
  `impl-choice`, each citing its `N`-note.
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

Evidence files are under `ai_tmp/gd/GD03/`, and the review record under
`ai_tmp/review/GD03-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name, or keeps only the values its
assertion compared. A failing run prints the runner's panic and the recipe's `error:`
line between `coverage faults: N` and `exit status: 1`; those are left out too, with no
mark. A quote that does not open with its `$` line starts part-way through the output.
Where a fact the record should hold was never saved, the text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`, GD02's record commit, which
  holds GD02.
- Work commit: `8e6907e2c89b9291b0aeb96cbd6d1433b3e04985`. The commit after it holds this
  record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`, on the untouched worktree
(`baseline.log`, which records each status as `<name>-exit=N` and holds no `$` line; its
last line counts the lines `git status --short` printed):

```text
cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3
...
All checks passed and the worktree is unchanged.
check-exit=0
...
122 tests run, 122 passed
coverage faults: 0
gleam-double-exit=0
       0
```

### 3. The design as it stands after GD03

This section adds to section 3 of GD02's hand-back, which a later ticket's author still
reads first. Sources are under `gleam-double/src/pachinko_double/` and tests under
`gleam-double/test/`. The work commit touches none of `rules.gleam`, `identity.gleam`,
`config.gleam`, `time.gleam`, `publish.gleam`, `inspection.gleam`, `invariants.gleam` or
`test/runner.gleam`: GD01's and GD02's sections still describe them.

- **`domain.gleam`.**
  - `Event` gains `Handed(pocket_key, record_key)`: one for each record of a batch, in
    the batch's order.
  - `Answer`, which had no constructor, has two.
    - `Batch(consumer, lease, pocket, records: List(BatchRecord))`. `consumer` is the
      consumer's id, `lease` the lease's id and `pocket` the pocket's key. It carries no
      expiry (G22). `records` is smallest position first.
    - `NoPocketAvailable(consumer)`.
  - `BatchRecord(key, rank, payload: BitArray)`: a record's key, and the rank and payload
    it had when the batch was computed.
- **`refusals.gleam`.** Three more refusals.
  - `UnknownConsumer(consumer_id)`.
  - `PullRefused(consumer_id)`, the Pull table's refusal (G23).
  - `LeaseFenced(action, consumer_id, lease_id)`. `action` is the trigger's name as a
    `String`, `"Renew"` here, so GD04's fencing can use the same refusal for ack, nack
    and release (G28).
- **`pull.gleam`**, new.
  - `Pull(consumer: String, limit: Int)` and `pull() -> Table(Pull)`. The table is named
    `Pull`, its rows are `PullWithinLease`, `PullClaimsFrontPocket` and
    `PullFindsNoPocket` in that order, and when no guard holds it refuses with
    `PullRefused`.
  - `Handover(pocket_key, record_key)` and `handed() -> Table(Handover)`. The table is
    named `Handed`, its one row is `HoldRecord`, and when no guard holds it is
    `Defective`.
  - The lookups are private. The consumer's active lease is the first active lease of
    that consumer in the state's order. The front pocket is the queued pocket with the
    smallest position. The batch is the pocket's queued records sorted by position, cut
    to `min(limit, max_pull_batch)`; a number that is not positive gives none.
  - Both pulling rows compute the batch first, then ask for `now`. Only
    `PullClaimsFrontPocket` then asks for a lease id; it appends the new lease at the end
    of `leases` and replaces the pocket in place. `PullFindsNoPocket` asks for neither.
  - A pull that hands out a batch finishes with one `Handed` for each record and the
    `Batch` answer. The batch is fixed before any `Handed` is answered.
- **`settle.gleam`**, new: "the actions a consumer takes under a lease it names". It
  holds `Renew(consumer: String, lease: String)` and `renew() -> Table(Renew)`. The table
  is named `Renew` and has one row, `Renew`. Its guard finds the lease by id with
  `model.lease` and holds when that lease is active and its consumer is the caller. Its
  effect asks for `now`, sets `expires_at` to `now + lease_ttl` with
  `model.replace_lease`, emits nothing and answers nothing. When the guard does not hold
  the table refuses with `LeaseFenced("Renew", consumer, lease)`: a lease that is not
  active, not the consumer's, or was never issued.
- **`core.gleam`.**
  - `Trigger` gains `Pull(pull: Pull)` and `Renew(renew: Renew)`.
  - `apply` refuses a pull or a renew by a consumer that never joined itself, with
    `UnknownConsumer`, before any table runs (G26), as it does the unknown publisher.
  - `chain` routes `Handed(pocket_key, record_key)` to `pull.handed()` as a `Handover`.
  - `registry()` adds `pull.pull()`, `pull.handed()` and `settle.renew()` at the end.
- **`owner.gleam`.** `pull(owner, consumer: String, limit: Int) -> Reply` and
  `renew(owner, consumer: String, lease: String) -> Reply`. `Message` is unchanged: both
  send the existing `Act` with the new trigger.
  - A pull that claims replies `Ok(Outcome(["PullClaimsFrontPocket", "HoldRecord", ...],
    Some(Batch(...))))`, with one `HoldRecord` for each record handed.
  - A pull that finds nothing replies `Ok(Outcome(["PullFindsNoPocket"],
    Some(NoPocketAvailable(consumer))))`.
  - A renew replies `Ok(Outcome(["Renew"], None))`.
- **`model.gleam`.** Three functions over plain lists.
  - `replace_pocket(pockets, pocket)` replaces by key.
  - `lease(leases, id) -> Result(Lease, Nil)` answers the first lease with that id (G24).
  - `replace_lease(leases, lease)` replaces by id, so every lease with that id (G27).
- **`test/harness.gleam`.**
  - `fixture_with(settings)`; `fixture()` is now that over `fixture_settings()`.
  - `publish_publication(double, publication)`, for a payload that is not text.
    `publish_as` now goes through it.
  - `pull(double, consumer, limit) -> Reply` and `renew(double, consumer, lease) ->
    Reply`, both logged like `publish`.
  - `batch(double, consumer, limit) -> Pulled`, with `Pulled(fired, lease, pocket_key,
    records)`. It fails the test on any reply but a batch, and on a batch that names
    another consumer.
  - `keys(records)`, the batch's keys in order.
  - `LeaseView(pocket_key, expires_at, records: List(MarkedRecord))`,
    `MarkedRecord(key, rank, payload, mark)`, `lease_view(double, consumer) ->
    Option(LeaseView)` and `marked_keys(records)`.
  - `lease_view` is a stand-in (G31). It takes one snapshot and reads
    `inspection.views`: the consumer's `active_leases`, and that lease's pocket's
    `held_records` sorted by position. It fails the test if the consumer never joined or
    holds more than one active lease. **GD04 replaces its body with the owner's
    `LeaseView` message** and keeps its signature, so S033's and S036's ports do not
    change.
- **`test/coverage.gleam`.** `pending_scenarios` holds 64 ids and `pending_rules` 21
  rules.
- **Test modules.** `pull_test`, `lease_test` and `competing_test` are new and listed in
  `main`; `competing_test` sits after `owner_test`, and the other two after
  `publish_test`. The run has 155 tests.

Changes to a GD01 or GD02 seam, or departures from the ticket's wording, and what each
costs:

| Before, or as the ticket words it | After GD03 | Cost |
| --- | --- | --- |
| `Answer` had no constructor, so every reply's answer was `None` | Two constructors; a pull's reply holds `Some(answer)` | None recorded |
| `Event` had one constructor, routed to a table with no row | `Handed` is added, and `chain` routes it to the `Handed` table | `core_test`'s header comment was reworded. No state between a pull's row and its `HoldRecord`s breaks an invariant, so `Handed` still gives no witness for GD02's B6 (Open points) |
| `Trigger` held the two joins and `Publish` | `Pull` and `Renew` are added, and `apply` refuses an unknown consumer itself, before any rule runs (G26) | That refusal reads no clock and draws no id, and no table is consulted |
| "the owner's `Pull` and `Renew` messages" (Files touched) | Two functions, `owner.pull` and `owner.renew`, over the existing `Act` message | None: `owner.publish` took the same form in GD02 |
| The lease view is "through the owner's `LeaseView` message", from GD04 (shared rule 4) | S033 and S036 read `harness.lease_view`, a helper over the snapshot | GD04 replaces the helper's body. S036's record order is `impl-choice` until S090 separates the orders (G31) |
| `harness.fixture()` built its own config | It calls `fixture_with(fixture_settings())` | None recorded |
| `pending_scenarios` held 82 ids and `pending_rules` 26 rules | 64 and 21 | None |

### 4. Steps 2 to 5

Every output in this section ran uncommitted, on base
`cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`. Each subsection repeats its head.

Who did what, one writer at a time but for the lapse in section 5:

- The lead decided the seams and wrote them into the step-3 brief (`brief-step3.md`).
- One subagent built the seams and step 3. A second wrote step 4 (`brief-step4.md`). A
  third wrote the spec notes.
- The lead read `pull.gleam` against the specification, and replaced four `G-new`
  placeholders in test comments with their note ids: G31, G26, G23 and G26.
- At the checkpoint one runner subagent per lens drove Codex, and one fresh triage
  subagent checked the findings and applied the fixes.

#### Step 2: the plan

`plan-change` was loaded in GD01 and applied here. The step-3 author checked each owned
scenario against its Rust test: none needed an operation this ticket does not land. So no
scenario moved under shared rule 2, and no other ticket's ownership table changed. One
thing falls short of a move: S033 and S036 read the consumer's lease view, whose owner
message is GD04's, and read it through `harness.lease_view` for now (section 3).

#### Step 3: the ports, red first

Uncommitted, on base `cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`.

The scenarios, by module, each under its `Scenario` tag:

| Module | Scenarios |
| --- | --- |
| `config_test` | S001, S008 |
| `publish_test` | S012, S015, S019, S020, S022 |
| `pull_test` | S027, S028, S029, S030, S031, S032, S033, S036, S038, S039 |
| `lease_test` | S050 |

The `impl-choice` labels, each in its test's comment:

- The absolute positions in S012, S015 and S022 (G04 and G29; N16).
- The order of the superseded queue in S015 and S019 (G19).
- S027's `"lease-1"`: the port's comment cites G10, and G27 records it.
- The record order of S036's lease view (G31).
- That a new lease is added at the end of the leases (G19).
- The refusal of a consumer that never joined, for a pull and for a renew (G26, as N18).
- The hand-built state that reaches the Pull table's refusal (G23, as N14).
- In `competing_test`, the two assertions that rest on the snapshot's order (G19; G32).

The Rust assertions not ported literally, each with its reason:

- `batch.expires_at` in S027 and S036, and renew's returned expiry in S050, are read
  from the snapshot's lease. A batch carries no expiry and `Renew` answers nothing (G22).
- Publish outcomes are ported as the ruling on N32 says.
- S022's `invariants::order_is_the_queued_pockets` has nothing to compare:
  `harness.order` is that derivation.
- S050's `granted_before_expiry` is `invariants.check` not naming `GrantedBeforeExpiry`.
- S027's pairing of consumer and lease is the consumer view's `active_leases`.

The first red, `ev-step3-red-first-no-rows.txt`, with the seams in and the three tables
empty. Every pull met the Pull table's refusal, and 25 tests failed:

```text
...
FAIL pull_test.a pull claims the front pocket, leases it and holds the batch: crashed: ... #(Value, Error(Refused(PullRefused("c1")))) ...
...
COVERAGE S001 is in pending_scenarios, and a test tagged Scenario("S001") passed
150 tests run, 125 passed
coverage faults: 1
exit status: 1
```

The last red, `ev-step3-red-last-renew.txt`, before the Renew row. Three tests failed.
What the pending lists held at that run is not recorded.

```text
FAIL lease_test.a renew extends the lease to now plus lease_ttl: crashed: ... #(Left, ... #(Value, Error(Refused(LeaseFenced("Renew", "c1", "lease-1")))) ...), #(Right, ... #(Value, Ok(Outcome(["Renew"], None))) ...
FAIL lease_test.Renew changes exactly what it ensures, reads the clock once and draws no lease id: crashed: ...
FAIL lease_test.Renew's guard holds only for an active lease of the consumer's: crashed: ...
150 tests run, 147 passed
coverage faults: 0
exit status: 1
```

Never red, and why:

- S001's port asserts the configuration only. It passed against the empty tables, as the
  `COVERAGE` line above shows.
- The pull by a consumer that never joined, and the hand-built `PullRefused` test: their
  refusals were in the scaffold.
- The renew by a consumer that never joined was red only through its setup pull.

The mutants below cover the refusals: K1 and K2 for the two of a consumer that never
joined, and W1 for the hand-built state. No mutant in the pass names S001's port.

The mutation pass, after green (`ev-step3-mutants.txt`, which holds no `$` line; each
mutant's whole output is `mutants/<id>.txt`). Its summary line:

```text
44 mutants, 44 killed by their named test in the last pass, every file restored to its SHA-256.
```

- The 44 are W1 to W7 on `PullWithinLease`, C1 to C14 on `PullClaimsFrontPocket`, H1 to
  H6 on `HoldRecord`, F1 to F5 on `PullFindsNoPocket`, R1 to R7 on `Renew`, A1 to A3 on
  the lookups the three Pull rules share, and K1 and K2 on `core.apply`'s refusal of a
  consumer that never joined. Every line records exit status 1.
- Each changes one line of `pull.gleam`, `settle.gleam` or `core.gleam`. Three add a
  line, four delete one, and H3 carries a second edit, to an import its change left
  unused.
- A mutant counts as killed only if the run reached the tests, exited non-zero, and its
  named test is among the `FAIL` lines.
- One survived its first run. A1 deletes the `status: active` filter from the lookup of
  the consumer's lease: `150 tests run, 150 passed`, exit status 0
  (`mutants/A1-survived.txt`). No lease ends through the owner yet, and no hand-built
  pull state held an ended lease. The test "a consumer whose lease has ended pulls as one
  with no lease" was added, and kills A1 in the last pass.
- Six mutants, C1, C14, F1, F3, A2 and A3, first left a variable unused. The
  warnings-as-errors build stopped those runs before any test ran, which is no kill.
  They were rewritten to keep the variable in use and run again.
- Not mutated, as equivalent: either of `PullClaimsFrontPocket`'s two `requires` lines.
  Not mutated, as not observable: the order in which one pull's `Handed` events are
  answered.
- The pass ran at 151 tests, before step 4 and before the checkpoint (section 10).

The green run, `ev-step3-green.txt`:

```text
$ just gleam-double
...
151 tests run, 151 passed
coverage faults: 0
exit status: 0
```

No ported scenario failed against rows that follow the specification, and dispatch
returned no `GuardOverlap` on the Pull, `Handed` or Renew table. On exit
`pending_scenarios` holds 64 ids and `pending_rules` 21 rules.

#### Step 4: the competing callers

Uncommitted, on base `cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`.

`test/competing_test.gleam` is new. Its three tests carry no tag and share one body:

| Test | Callers | Queued pockets |
| --- | --- | --- |
| "eight consumers pull at once while one pocket is queued: one is handed its records under a lease and seven are told there is none" | 8 | 1 |
| "six consumers pull at once while six pockets are queued: each is handed a different pocket, and the pockets are claimed front first" | 6 | 6 |
| "nine consumers pull at once while four pockets are queued: four are handed a pocket each and five are told there is none" | 9 | 4 |

What the body asserts, and nothing about which consumer won, which pocket a consumer got
or the number in a lease id:

- From the answers: as many batches as the fewer of the callers and the pockets, and
  `NoPocketAvailable` for every other caller. No two batches name one pocket, one lease
  or one consumer. The pockets named are the front ones. Each batch is its own pocket's
  records, oldest first, up to the limit; every payload names its pocket.
- From the snapshot afterwards: `invariants.check` names nothing. The leases are the
  batches' leases, one for one, each active, and no two share a pocket or a consumer. The
  pockets claimed are no longer queued. The held records are the ones handed out.
- From the fakes: one lease id drawn and one clock read for each lease granted.

Departures from the ticket's wording:

- The callers do not simply "send at once". Each waits at a gate of its own until every
  caller has been started, and only then is each let go.
- Each test runs its body once for each caller, with that caller started first, on a new
  owner each time. One starting order mostly gives one mailbox order.

The scratch breaks. Each evidence file opens with the break's name, what it does and its
diff, holds one `just gleam-double` run, and ends with a line that says the source was
restored and its SHA-256 is the same. Each run has 154 tests.

| Break | Scratch change | Evidence | Competing tests failing | Other tests failing | Passed | Coverage faults | Exit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `stale` | The owner applies a `Pull` to the state it held before the previous message | `ev-step4-stale.txt` | 3 | 14 | 137 | 11 | 1 |
| `unleased` | `PullClaimsFrontPocket` creates the lease and leaves the pocket queued | `ev-step4-unleased.txt` | 3, at the reply: the apply step's checker refuses every claim | 24 | 127 | 21 | 1 |
| `unleased-unchecked` | The same, with the apply step accepting a broken invariant | `ev-step4-unleased-unchecked.txt` | 3, on their own counts | 18 | 133 | 11 | 1 |
| `notfront` | The claim takes the queued pocket with the largest position | `ev-step4-notfront.txt` | 2: the second and third fail on the grant order, which rests on G19. The first passes, since one pocket has no back | 8 | 144 | 3 | 1 |
| `offstream` | The owner applies a `Pull` in a process of its own, from the state it held when it read the message, and takes the new state back as a second message | `ev-step4-offstream.txt` | 3 | 0 | 151 | 0 | 1 |

`stale` is the scratch break the ticket asks for. The first test's assertion compared two
batches with the one pocket there was to claim (`ev-step4-stale.txt`):

```text
FAIL competing_test.eight consumers pull at once while one pocket is queued: one is handed its records under a lease and seven are told there is none: crashed: ... #(Function, "compete"), #(Line, 263), #(Message, "Assertion failed.") ... #(Left, ... #(Value, 2) ...), #(Right, ... #(Value, 1) ...
FAIL competing_test.six consumers pull at once while six pockets are queued: each is handed a different pocket, and the pockets are claimed front first: crashed: ...
FAIL competing_test.nine consumers pull at once while four pockets are queued: four are handed a pocket each and five are told there is none: crashed: ...
...
154 tests run, 137 passed
coverage faults: 11
exit status: 1
restored and touched: owner.gleam sha256 ... (same)
```

`stale` also fails 14 other tests, so it does not show that the competing tests earn
their place. `offstream` does: it passes the 151 other tests and fails only these three
(`ev-step4-offstream.txt`):

```text
FAIL competing_test.eight consumers pull at once while one pocket is queued: one is handed its records under a lease and seven are told there is none: crashed: ...
FAIL competing_test.six consumers pull at once while six pockets are queued: each is handed a different pocket, and the pockets are claimed front first: crashed: ...
FAIL competing_test.nine consumers pull at once while four pockets are queued: four are handed a pocket each and five are told there is none: crashed: ...
154 tests run, 151 passed
coverage faults: 0
exit status: 1
restored and touched: owner.gleam sha256 ... (same)
```

`step4-report.md` says `offstream` failed the three in each of eight runs, seven of them
against the tests as they then stood. The evidence file holds the last run only; the
output of the other seven is not recorded.

The repeat (`ev-step4-repeat.txt`). A scratch edit added three tests, each running one
test's function 25 more times, and printed each run's grant order. The run ended
`157 tests run, 157 passed`, `coverage faults: 0`, exit status 0, and the edit was
restored. It holds 598 runs of the body, none failing:

| Callers, pockets | Runs | Different grant orders | Runs granted in another order than started |
| --- | --- | --- | --- |
| 8, 1 | 208 | 8 | 8 |
| 6, 6 | 156 | 24 | 21 |
| 9, 4 | 234 | 27 | 22 |

The green runs: `ev-step4-green.txt`, and `ev-after-notes.txt` after the note ids were
put into the four test comments. Each ends:

```text
154 tests run, 154 passed
coverage faults: 0
exit status: 0
```

#### Step 5: spec notes

`gleam-double/SPEC-NOTES.md` gained G22 to G32. Each cites its `N`-note or says it has
none.

| Note | Title | `N`-note |
| --- | --- | --- |
| G22 | a batch names its lease by id and its pocket by key, and carries no expiry | Agrees with N13 |
| G23 | the Pull table's refusal is unreachable through the owner | Agrees with N14 |
| G24 | a lookup that names one lease, and `min_by` over a tie, take the first in the snapshot's order | No `N`-note |
| G25 | a pull whose limit is not positive still claims a pocket | Agrees with N19 |
| G26 | a pull or a renew by a consumer that never joined is refused (`impl-choice`) | Agrees with N18, but not with its reason |
| G27 | a pull draws a lease id only when it creates a lease | Agrees with N15, and differs in one assertion |
| G28 | Renew's refusal is `LeaseFenced`, and an unknown lease id meets it | Agrees with N20, and differs in what is tested |
| G29 | S012, S015 and S022 assert absolute positions (`impl-choice`) | Agrees with N16 |
| G30 | S019's large ranks are a 64-bit integer's extremes, not the double's | Agrees with N17 |
| G31 | the lease view is read from the snapshot for now (S036's record order is `impl-choice`) | Takes N23's reading |
| G32 | what the competing callers show of "one serialized stream" | No `N`-note |

- Step 5's two candidates are G25 and G23, and its note on the competing callers is G32.
  Step 3's point on S019 is G30.
- Seven earlier notes were amended to stay true: G04, G08, G10, G13, G15, G17 and G19.
- One finding comes from the code and has no test, in G27: the double never checks that
  a lease id it is handed is new (Open points).
- The checkpoint then reworded G24, G28 and G32 (section 6).

### 5. A lapse in procedure

The lead answered a question from the step-3 subagent after that subagent had already
reported. The message woke it. It made comment-only edits to
`gleam-double/test/harness.gleam` and `gleam-double/test/pull_test.gleam`, and ran
`just gleam-double` once, while the step-4 subagent was meant to be the only writer. That
run was green at 151 tests, and it re-recorded `ev-step3-green.txt`.

By the file clock those edits landed at 20:38:39, before the step-4 subagent's first
scratch mutant went live at about 20:46. The step-4 subagent was told. It ran every
evidence run again as sole writer and re-recorded its final green run afterwards, and its
results did not change.

The three sources the step-3 sweep mutated kept their SHA-256, so `ev-step3-mutants.txt`
stands: `pull.gleam`, `settle.gleam` and `core.gleam` at the work commit equal the copies
the sweep restored from (`ev-handback-checks.txt`, a check the writer of this record
ran). The two times above are the lead's; no evidence file holds them.

### 6. Checkpoint record

One checkpoint, at step 6: conformance and breakage, before the final commit.

- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `-s read-only`, one fresh
  process per lens, by the ticket's corrected command. Each lens left `last-<lens>.md`
  and `log-<lens>.txt`. Codex was on `PATH`, so no fallback to a Claude reviewer was
  needed.
- **Recovery.** Both findings files open `Source: last-message`: each is Codex's last
  message, recovered because a read-only Codex writes no file.
- **What the reviewers ran.** Nothing. Each says its evidence was read-only inspection.
  The triage ran each claim.
- **Triage.** One fresh subagent, neither a reviewer nor an author.
- **Rounds.** One. No finding was rated high, by a reviewer or by the triage, so no
  second round was owed.
- **Not recorded.** The `codex exec --help` check on the day, and the model of the runner
  and triage subagents.

It ran uncommitted, on base `cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`, after step 5.
`just gleam-double` ended `154 tests run, 154 passed` before any fix (`ev-baseline.txt`).
The conformance lens filed C1 to C3 and the breakage lens B1 and B2. L1 is a point the
lead raised.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid | `competing_test` compares a pocket's held records, in the snapshot's order, with the batch's order: a second G19 dependency that G32 did not name | G32 says two assertions rest on an `impl-choice` and names both. The assertion's comment is labelled `impl-choice` (G19). The assertion is unchanged |
| C2 | low | valid | G24 said the invariants make every lookup name one entity, but no invariant compares lease ids, so `model.lease` is not covered, as G27 says | G24's claim is limited to the consumer's active lease, `min_by` and `oldest`, and points to G27 for `model.lease` |
| C3 | low | valid | The ports of S012, S015 and S022 cited G16, which records S011, S014 and S016; G29 records these three | The three comments cite G04 and G29, and the module's header G04, G16 and G29 |
| B1 | medium | valid, and wider than filed | A mutant that empties `superseded` at the start of each of the five effects passed all 154 tests. So did one that empties `dead_letters` | Tests only. The hand-built states of `pull_test` and `lease_test` hold a retained version and a dead letter. The owner-driven "changes exactly" tests publish one supersede first. One test is new, "Renew leaves a retained version and a dead letter as they were". G28 names it |
| B2 | medium | valid | `expires_at = max(now + lease_ttl, old expires_at + 1)` in `PullWithinLease` passed all 154 tests: no pull within a lease was checked with the clock standing still | `within_lease_exactly` pulls a second time before the clock moves, and asserts the same complete snapshot and one more clock read |
| L1 | low | invalid | G32 states no number of runs, so there is no "seven" to correct, and its sentence agrees with the evidence | Nothing |

Five of the six were valid, and all five were applied. Nothing was handed back. The
triage changed five files, all in this ticket's Files touched: `SPEC-NOTES.md` (G24, G28
and G32), a comment in `competing_test.gleam`, four comments in `publish_test.gleam`,
`pull_test.gleam` and `lease_test.gleam`. Nothing under `gleam-double/src/` changed, and
no ported scenario's assertion changed.

On the questions step 6 puts to the breakage lens, the reviewer lists what it examined
with no further finding: the pocket and record order where storage order differs from
position order, both batch caps, pocket exclusivity, and the competing callers'
assertions as independent of the mailbox order.

The eleven mutants B1 and B2 predict (`mutants.py`; one file for each run,
`ev-mutant-before-<id>.txt` and `ev-mutant-after-<id>.txt`, with its diff):

| Evidence | Tree | Result |
| --- | --- | --- |
| `ev-mutants-before.txt`, a summary with no `$` line | As reviewed | 11 of 11 survived, each `154 tests run, 154 passed`, exit 0 |
| `ev-mutants-after.txt`, the same | After the triage | 11 of 11 killed, exit 1 each, of 155 tests. The file names the tests that fail |

- Five mutants empty `superseded`, one for each rule. Each is now killed by at least one
  owner-driven test and one hand-built one.
- Five empty `dead_letters`. Each is killed by a hand-built test alone: no rule creates a
  dead letter yet (Open points).
- B2's mutant is killed by "PullWithinLease changes exactly what it ensures, on an empty
  batch too, reads the clock once and draws no lease id".
- Not covered, and not part of B2: `max(now + lease_ttl, old expires_at)` differs from
  the rule only when the clock goes backwards, which no test does, for `PullWithinLease`
  or for `Renew`.

`ev-after-test-fixes.txt` is `just gleam-double` after the test fixes:
`155 tests run, 155 passed`, exit status 0. `ev-after-mutants-green.txt` holds a second
run with the same two lines; `verdicts.md` does not list it, and what it followed is not
recorded.

Tree ids: as reviewed and before the triage `0b060a9f9be39c8fd5d7c4610c86bb3cea64351d`
(`tree-before-triage.txt`, and `tree-before-review-1.txt` beside the other evidence),
after it `175f5d20c32486a9bc6b8db8c715d3fe3b4d17ac` (`tree-after-triage.txt`). The work
commit's tree is that second id (`ev-handback-checks.txt`), so the re-run below and the
eleven "after" runs ran on the tree that was committed.

`verdicts.md` says `just lint` passed after the Markdown edits; no evidence file holds
that run. `just check` did not run between the triage and the commit: it ran at the work
commit (section 7).

The re-run after the checkpoint (`ev-final.txt`), uncommitted, on base
`cf1d96e21c2d40d62a9d8b9a19a0ab7e5343bcf3`:

```text
$ just gleam-double
...
155 tests run, 155 passed
coverage faults: 0
exit status: 0
```

### 7. Verification

The Verification block ran at `8e6907e2c89b9291b0aeb96cbd6d1433b3e04985`, the work
commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's block word
for word; output in `ev-verification.txt`). It exited 0, so its last two lines held:
`git status --short` printed nothing.

```text
$ sh ai_tmp/gd/GD03/verification.sh
8e6907e2c89b9291b0aeb96cbd6d1433b3e04985
gleam 1.19.1
OTP 29
...
155 tests run, 155 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The 155 tests are GD02's 122 and 33 more: 18 in `pull_test`, 5 in `lease_test`, 3 in
`competing_test`, 2 in `config_test` and 5 in `publish_test`. Those counts are read from
the modules' registries at the work commit; no evidence file holds them.

### 8. The acceptance criteria against the evidence

1. `just gleam-double` exits 0 and shows the counts: `ev-verification.txt`.
2. The pending lists hold exactly the 64 ids and the 21 rules GD04 to GD06 own:
   `ev-handback-checks.txt`, which compares the lists at the work commit with the three
   tickets' ownership tables. `findings-conformance.md` says the same.
3. Every owned scenario has a tagged test, and each of the five rules is a row that
   fired: `coverage faults: 0` in `ev-verification.txt`, with none of the 18 ids or the 5
   rules pending.
4. The batch is asserted through the owner's answer, and the lease's two times and the
   held fields through the snapshot: `ev-step3-mutants.txt`. C14 (the batch does not name
   its lease), C5 and C6 (the lease's two times) and H1 to H5 (the five held fields) are
   each killed, and S027's owner-driven port is among the tests that fail for C5, C6, C14
   and H3 (`mutants/<id>.txt`).
5. Only `PullClaimsFrontPocket` draws: `ev-step3-mutants.txt`, where W6 and R6, a draw by
   `PullWithinLease` and by `Renew`, are killed, and the competing tests' count of one
   draw for each lease granted. That `PullFindsNoPocket` and a refused pull draw none is
   asserted by passing tests; no mutant shows those two assertions can fail.
6. Step 4's tests pass, and its scratch break is quoted with its output and exit status:
   `ev-step4-green.txt` and `ev-verification.txt`; `ev-step4-stale.txt`, in section 4.
7. The spec notes are there, the unknown consumer's `impl-choice` among them (G26), each
   with its `N`-note line: `ev-handback-checks.txt` counts eleven such lines from G22 on.
8. `just gleam-double` leaves nothing Git can see and no `erl_crash.dump`:
   `ev-verification.txt`. The script tests both under `set -eu` and exits 0.
9. The checkpoint is recorded in section 6. One thing §14 asks for cannot be shown: the
   `codex exec --help` check on the day is not recorded.
10. `just check` ends with its line and `git status --short` prints nothing:
    `ev-verification.txt`.

### 9. Stops

None fired. The conditions watched:

- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did.
- A `GuardOverlap` on a specification table (shared rule 4). Dispatch returned none on
  the Pull, `Handed` or Renew table.
- A scenario that needs an operation this ticket does not land (shared rule 2). None
  moved.
- A count that moved. The coverage check asserts 93, 32 and 35 on every run, and every
  green run ends `coverage faults: 0`.
- A reach for the network. Nothing was fetched and no network recipe ran. The review
  checkpoint called a model API, which the checkpoint authorises.

Step 7's stop stands: nothing was pushed and no pull request was opened.

### 10. What was not run

- Any push, pull request or issue.
- `just initialize`, `just gleam-double-deps` and `just lock`.
- A CI run.
- A second review round. None was owed.
- The step 3 sweep after step 4 or after the checkpoint. The 44 mutants ran at 151
  tests. The three sources they change are the same at the work commit
  (`ev-handback-checks.txt`), but the checkpoint rewrote several of the tests the sweep
  names, and the sweep was not repeated at 155.
- The step 4 breaks and the repeat after the checkpoint. They ran at 154 tests. The
  checkpoint changed a comment in `competing_test.gleam` and nothing under
  `gleam-double/src/`.
- A witness that the checker runs only after every `Handed` has been answered (Open
  points).
- A test that pins `LeaseFenced` through the owner (Open points).
- A test with a lease-id source that repeats an id (G27; Open points).
- A test in which the clock goes backwards (checkpoint, B2).
- `just check` between the triage and the commit (section 6).

Not recorded: `codex exec --help` on the day; the model of the runner, triage and author
subagents; the output of seven of `offstream`'s eight runs; an evidence file for the
triage's `just lint`; what the pending lists held at the last red run; and an evidence
file for the two times in section 5 or for the test count of each module.

### 11. Working lessons for GD04 to GD07

- A message to a subagent that has already reported resumes it, and it acts on the
  message. Answer a question mid-task at once or not at all, and never send edit
  instructions while another writer is live (section 5).
- A brief can live in a file, and the spawn prompt can point at it.
- Ask the concurrency author for a mutant that only its tests catch. `offstream` is what
  shows that the competing tests earn their place; the ticket's own break, `stale`, fails
  14 other tests too.
- A "changes exactly what it ensures" test shows nothing for a list that is empty before
  and after. Seed every list of the state the rule should leave alone: the checkpoint's
  ten surviving mutants each emptied a queue that was already empty. Once a rule creates
  a dead letter, the owner-driven tests can seed one as they now seed a retained version.
- A mutant that leaves a variable or an import unused fails the warnings-as-errors build,
  not a test. Write it so that a test fails.

## Open points

- **Should a batch carry its lease's `expires_at`? Still open.** N13 already asks. This
  ticket takes N13's reading, in G22, and adds nothing to the answer: the ports read the
  expiry from the snapshot's lease.
- **The lease view in S033 and S036 is read from the snapshot (G31).**
  `harness.lease_view` stands in for the `Consuming` surface's view. GD04 lands the
  owner's `LeaseView` message and replaces the helper's body. S036's record order is
  `impl-choice` until then, and S090, which GD04 owns, is the row that tells position
  order from key order.
- **No test shows that the checker runs only after every `Handed` has been answered.**
  Step 3 asks for that order, and `core.conclude` gives it. But no state between a
  pull's own row and its `HoldRecord`s breaks an invariant, so a checker call placed
  part-way would pass every test. This is GD02's B6 again, and GD04 owes the witness
  with `ReviveIdlePocket`. As GD04's ticket stands at this record, its step 5 asks the
  breakage lens whether a test could pass "while the checker ran part-way through a
  release", and no step asks for the witness test GD02's Open points describe.
- **`LeaseFenced` is pinned by no test through the owner until GD04.** Renew's guard is
  written in full and tested on hand-built states. Its two refusal rows, S052 and S070,
  are GD04's.
- **A lease-id source that repeated an id would give two leases one id (G27).** The
  double does not check that an id it is handed is new, and no invariant compares two
  ids. `model.lease` would answer the first such lease and `model.replace_lease` would
  replace both. The fake never repeats. G10 and G27 ask whether the specification should
  declare a lease's identity and say that two leases never share one.
- **The competing-callers tests sample the scheduler's orders, and race only `Pull`
  against `Pull` (G32).** They cannot show that the properties hold in every
  interleaving, and they say nothing of the case the specification's sentence is written
  for, a lapse in the same stream as a consumer's action. GD06 owns that race.
- **Carried from GD02's hand-back: a higher rank published onto a held record with
  nothing deferred meets `EqualRankRefused` until GD05 (G13).** GD02 recorded that no
  publish through the owner could reach that input "until GD03 lands a pull". This ticket
  lands the pull and changes nothing in `publish.gleam`, so by GD02's record the input
  can now be reached through the owner. The input is `DeferBehindHeldVersion`'s, which is
  pending, so no test pins it as a refusal. No test here publishes onto a held record,
  and no evidence file shows the answer.
- **No owner-driven test would see a pull or a renew empty the dead-letter queue
  (checkpoint, B1).** Only hand-built states hold a dead letter. Once GD04 lands the rule
  that creates one, the owner-driven "changes exactly" tests can seed it.
- **PD07 is still todo, and is not a blocker.** The double follows the rulings as they
  stand.
