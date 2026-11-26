---
id: GD05
title: "Gleam double: deferral behind the held version"
status: done
depends_on: [GD04]
parallel_with: []
branch: build-gleam-double
estimated_size: M
---

# GD05: Gleam double: deferral behind the held version

## Context

GD01 to GD04 landed the Gleam double under `gleam-double/`: publish, pull, renew, marks,
fencing, release, the lease-end module with its three rows that have nothing deferred,
dead letters and the idle revival, all behind the one owner. Read their hand-backs and
`gleam-double/SPEC-NOTES.md` first. `.scratch/gleam-double/README.md` is the series'
index.

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
- **A second higher rank.** `Supersede`'s held branch, written in GD02, answers a higher
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
- **Where the rows go.** The three deferred rows are rows of the lease-end module's
  `Freed` table, which GD04 created with `ending_records` already written in full, and
  `DeferBehindHeldVersion` is a row of the Publish table. This ticket changes neither the
  lease-end module's interface nor its release callers.
- **The arrival-position invariants.** `DeferredHasArrivalPosition`,
  `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `HeldCopiesAgree`,
  `ArrivalPositionUnoccupied`, `UniqueArrivalPosition` and `ArrivalPositionsBelowNext`
  become reachable here. GD02's checker already holds them.

What the maintainer has already ruled, and what the Python double already read:

- **S091's kept positions are compared by key** (the ruling on N31). The Rust's test
  lists a pocket's records in its own snapshot order, which is its inspection shape. The
  port compares positions by key.
- **A deferral answers nothing** (the ruling on N32), where the Rust answers
  `Superseded`. The port asserts the state and that the publish answered nothing.
- S091 pins the order of the entries one lease end creates, `impl-choice` (N29).
- A deferred version shed by a later publish takes the publish's `now` (N30).

This ticket covers the release path only. The lapse half of each "by release or lapse"
row lands in GD06 with `Tick`, which is why GD06 owns S080 and S088, whose Rust tests
loop over both endings.

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

- No lapse, parking or decay, and no lapse half of a "by release or lapse" scenario. GD06
  lands them.
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

  `<id>` is `GD05`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `gleam-double/src/` | shared, own lines | Deferral in the Publish table, the deferred supersede, and the three deferred rows in the lease-end module's `Freed` table |
| `gleam-double/test/` | shared, own lines | The deferral tests; the pending lists in `coverage.gleam` |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/gleam-double/GD05-deferral.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/GD06-clock-tick-lapse-parking-decay.md` | shared, own lines | Only if a scenario moves under shared rule 2 |
| `.scratch/gleam-double/README.md` | shared, own lines | GD05's Status cell, and any move |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD04, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read decision 0020, the Publish and Lease end
   sections of the specification, the guarantees `OldestSlotNewestPayload`,
   `NothingLostWhileHeld` and `EveryDropIsVisible`, N29 to N32 in `double/SPEC-NOTES.md`,
   and the catalog rows this ticket owns. Confirm that no owned scenario needs a lapse.
   Move any that does, under shared rule 2.
3. **Port the scenarios, red first.** For each owned scenario, write its tagged test, see
   it fail, then add the rule as its row in its trigger's table, fired in at least one
   test. Points to watch:
   - The three deferred rows go inside the lease-end module, beside GD04's three. If one
     seems to need a change to the module's interface or to a release caller, **stop**
     and say so in the hand-back rather than make it.
   - The arrival position is allotted when the first newer version arrives, not when the
     lease ends, and the counter is not advanced again at the end.
   - A superseded record made at the lease's end takes `shed_at` and `expires_at` from
     `ended_at`, not from the rule's own `now`.
   - S091 is ported as the ruling on N31 says, with positions compared by key. Its order
     of the entries one lease end creates, smallest held position first, is more than the
     specification states. Port that as `impl-choice` (N29).
   - The `Freed` table now has six rows. Dispatch evaluates all six guards for every
     held record, so a `GuardOverlap` here is the specification's disjoint-guards claim
     failing: record it and **stop** (shared rule 4).

   Shrink `pending_scenarios` and `pending_rules` by exactly what landed.
4. **Spec notes.** Add a note wherever deferral forced an interpretation, each citing its
   `N`-note. Candidates:
   - S091's order;
   - whether a superseded record shed from the deferred slot takes the pocket's current
     time or the lease's.
5. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks whether a test could pass while a held version was
   shed during its lease, while a deferred version lost its arrival position on a second
   supersede, or while one publish ended in both shed queues.
6. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- On exit, `pending_scenarios` holds exactly the 34 ids GD06 owns, and `pending_rules`
  the 7 rules it owns.
- Every owned scenario has a tagged test, and each of the four rules has its row in its
  trigger's table, fired in at least one test.
- This ticket's diff leaves the lease-end module's interface and its release callers
  unchanged.
- The arrival-position invariants' positive controls still pass. A test or a quoted probe
  shows that the premise of each of those seven invariants is true in at least one state
  the checker sees during this ticket's scenarios, so none of them holds only vacuously.
- `gleam-double/SPEC-NOTES.md` holds this ticket's notes, including S091's `impl-choice`,
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

Evidence files are under `ai_tmp/gd/GD05/`, and the review record under
`ai_tmp/review/GD05-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name, or keeps only the values its
assertion compared. A failing run prints the runner's panic and the recipe's `error:`
line between `coverage faults: N` and `exit status: 1`; those are left out too, with no
mark. A quote that does not open with its `$` line starts part-way through the output.
Where a fact the record should hold was never saved, the text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `5f3c430d3ce95f890acfcce317c898409a661f73`, GD04's record commit. It holds
  GD04's work, `e0eddcc8ea46e3f0b7ff729d0eb0b4285e1e879b`.
- Work commit: `d279f7a3293e80e397eaf3c1e64769af3c3df03f`. The commit after it holds this
  record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `5f3c430d3ce95f890acfcce317c898409a661f73`, on the untouched worktree
(`baseline.log`, quoted whole; it records each status as `<name>-exit=N` and holds no `$`
line, and its last line counts the lines `git status --short` printed):

```text
5f3c430d3ce95f890acfcce317c898409a661f73
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
check-exit=0
    Running pachinko_double_test.main
214 tests run, 214 passed
coverage faults: 0
gleam-double-exit=0
       0
```

### 3. The design as it stands after GD05

This section adds to section 3 of GD04's hand-back, and through it to GD03's, GD02's and
GD01's. Sources are under `gleam-double/src/pachinko_double/` and tests under
`gleam-double/test/`. Under `src/` the work commit changes only `publish.gleam` and
`lease_end.gleam` (`ev-handback-extra.txt`, part 2), so `core.gleam`, `settle.gleam`,
`owner.gleam`, `domain.gleam`, `model.gleam`, `consuming.gleam`, `refusals.gleam` and the
rest stand as GD04's section 3 describes them. In particular `chain` still routes `Freed`
to `lease_end.freed()`, `registry()` is unchanged, and `domain.Event` still has no
`LeaseLapsed`. Under `test/`, `harness.gleam` is unchanged.

- **`publish.gleam`.** Its public interface is unchanged: `Publication(publisher,
  pocket_key, record_key, rank, payload: BitArray)`, `publish() -> Table(Publication)`
  and `pocket_touched() -> Table(String)`.
  - The Publish table's rows, in the specification's order: `PublishIntoNewPocket`,
    `PublishNewKey`, `Supersede`, `DeferBehindHeldVersion` (new) and
    `DropStalePublish`. When no guard holds it refuses `EqualRankRefused(pocket_key,
    record_key, rank)`, whatever the record's status or mark (G13).
  - `DeferBehindHeldVersion`. Guard: the record exists, `rank > record.rank`, and it is
    `RecordHeld` with `deferred_position == None`. Effect: `use position <- Position`,
    then the record with the publication's rank and payload and
    `RecordHeld(..held, deferred_position: Some(position))`; its position, `held_rank`,
    `held_payload`, mark and `nacked_at` stay (G51). It emits
    `PocketTouched(pocket_key)`, asks for no time, and answers `Impossible` where the
    record is missing or queued (G53). The `PocketTouched` finds the pocket leased, so
    nothing happens, and a deferral replies `Ok(Outcome(["DeferBehindHeldVersion"],
    None))` (G54).
  - `Supersede` is unchanged since GD02. Its guard is `rank > record.rank` with the
    record queued, or held with `deferred_position != None`. It asks for `now`, sheds the
    stored rank and payload, the deferred version's for a held record, with that `now`
    (G50), and raises the record's rank and payload. The held fields and the arrival
    position stay.
  - `DropStalePublish` holds for `rank < record.rank`, whatever the status: the stored
    rank is the newest version's (S082, S083).
  - `pocket_touched()` still holds one row, `ReviveIdlePocket`, and answers
    `NothingHappens` when no guard holds. GD06's `ReviveParkedPocket` is its second row.
- **`lease_end.gleam`.** Its public interface is unchanged (`ev-handback-checks.txt` and
  `ev-handback-extra.txt`, part 1): `Ending` with `Release` and `Lapse`; `end(state:
  State, lease: Lease, ending: Ending, at: Timestamp) -> #(State, List(Event))`;
  `outlasts(state: State, pocket_key: String) -> Bool`; `Freeing(lease: String,
  record_key: String, ended_at: Timestamp)`; and `freed() -> Table(Freeing)`. GD04's
  section 3 describes `end` and `outlasts`, and both bodies are unchanged.
  - `outlasts` counts the pocket's records against `model.ending`, which keeps a held
    record whose mark is not `Unsettled` and whose `deferred_position` is `None`. So a
    record with a deferred version, of any mark, outlasts the lease, and so does an
    unsettled one (S054).
  - The `Freed` table, named `Freed`, has six rows, in the specification's order. Every
    guard is `held(state, freeing)` plus the mark and `deferred_position` below; every
    effect starts `use held <- free(state, freeing)`. When no guard holds it is
    `Defective`.

    | Row | Mark | `deferred_position` | Effect |
    | --- | --- | --- | --- |
    | `RequeueUnsettledRecord` | `Unsettled` | `None` | Queued at its own position |
    | `SupersedeUnsettledHeldVersion` | `Unsettled` | `Some` | The held version appended to `superseded` with reason `Superseded`, `shed_at: ended_at` and `expires_at: ended_at + superseded_ttl`; the record queued at its own position, the held version's, keeping the newer rank and payload (G52) |
    | `ConsumeAckedRecord` | `Acked` | `None` | Removed |
    | `ConsumeAckedVersionQueueDeferred` | `Acked` | `Some` | Queued at its arrival position; nothing recorded |
    | `DeadLetterNackedRecord` | `Nacked` | `None` | Removed; a dead letter appended |
    | `DeadLetterNackedVersionQueueDeferred` | `Nacked` | `Some` | Queued at its arrival position; a dead letter appended |

  - New private helpers, beside GD04's `Held`, `held`, `free` and `without`:
    - `arrival(held, freeing, effect)` hands `effect` the `deferred_position`, and
      answers `Impossible` where it is `None` (G53);
    - `nack_time(held, freeing, effect)` hands it `nacked_at`, and answers `Impossible`
      where it is `None` (G43);
    - `letter(state, held, freeing, nacked_at) -> DeadLetter` takes the lease's pocket
      and consumer, the held rank and payload, the nack's time, and `expires_at:
      ended_at + dead_letter_ttl`;
    - `queue_at(state, record, position)` queues the record at `position`, dropping the
      held variant's fields and keeping its rank, payload and place in the list.
  - `DeadLetterNackedRecord`'s body now uses `nack_time` and `letter`; its messages are
    unchanged (`report-a.md`).
  - No `Freed` row asks for a time or a position: each stamps from `freeing.ended_at`,
    and positions come from the record. Each row leaves `next_position` as it receives
    it, which need not be where the deferral left it: `ReleaseRequeuesPocket` asks for
    the pocket's position before it calls `lease_end.end` (`settle.gleam:433` and `435`,
    `ev-C2-positions.txt`), and a later publish may allot more (G51).
  - What GD06 needs: its lapse rules call `lease_end.end(state, lease, lease_end.Lapse,
    now)` and `lease_end.outlasts(state, pocket.key)`, put the returned events in their
    `Finish`, and need no route for `Freed`. All six rows read the mark and never how the
    lease ended: "the Freed rules read the mark and never how the lease ended" now runs
    the three new rows over a lease ended by release and by expiry (`report-a.md`).
    GD06 adds `ReviveParkedPocket` to `publish.pocket_touched()`.
- **`test/coverage.gleam`.** `pending_scenarios` holds 34 ids and `pending_rules` 7
  rules, the same sets as GD06's ownership table (`ev-handback-pending.txt`).
- **Tests.** `publish_test`, `settle_test`, `pull_test`, `lease_test` and
  `lease_end_test` changed; no module is new, and `main`'s list is unchanged. The run has
  232 tests. Helpers a later port may reuse:
  - `lease_test.deferred_and_nacked() -> #(Double, String, Int)`, S091's setup, answering
    the lease's id and the nacks' time; `positions_by_key`, a pocket's records as key and
    position in key order.
  - `publish_test.held_and_marked(mark) -> #(Double, Option(Int))`, the Rust file's
    helper; `held_as`, a held status with an arrival position; `shed_entry`; and
    `deferring(mark) -> State`, a hand-built state with the counter at 6.
  - `settle_test.held_as` and `deferred_as(rank, payload, mark, nacked_at, arrival)`,
    the Rust fixture's `held_as` without and with an arrival position, and
    `superseded_ttl()`; GD04's `deferred_with` is still there.
  - `lease_end_test.behind(record, arrival)` and `deferred_ending(ended: LeaseStatus) ->
    State`.
  - Three tests were renamed: "each Freed rule's guard holds only for a held record with
    its mark and nothing deferred behind it", "PublishNewKey and Supersede touch the
    pocket, and the other two emit nothing" and "Supersede takes a held record only with
    a version deferred behind it" now say what they cover with the new rows.

Changes to a GD01 to GD04 seam, or departures from the ticket's wording, and what each
costs:

| Before, or as the ticket words it | After GD05 | Cost |
| --- | --- | --- |
| A higher rank onto a held record with nothing deferred met `EqualRankRefused` (G13's pending input) | `DeferBehindHeldVersion` | No test had pinned the refusal; `held_record_guards` and `events` now assert the deferral (`report-a.md`) |
| `DeadLetterNackedRecord` built its dead letter inline | It uses `nack_time` and `letter`, shared with `DeadLetterNackedVersionQueueDeferred` | None recorded |
| The `Freed` table held three rows and three comments | Six rows | None |
| `pending_scenarios` held 46 ids and `pending_rules` 11 rules | 34 and 7 | None |

### 4. Steps 2 to 4

Every output in this section ran uncommitted, on base
`5f3c430d3ce95f890acfcce317c898409a661f73`.

Who did what, one writer at a time:

- The lead decided the seams and wrote them into two briefs, `brief-a.md` and
  `brief-b.md`. By its account it read N31's note text to confirm S091's port: positions
  compared by key and against the state before the release, never as absolute values.
- Agent A, from `brief-a.md`, wrote the four rows, the twelve ports, the changes for
  G13's pending input (D), the premise test (E) and the shrunk lists (`report-a.md`).
- Agent B, from `brief-b.md`, ran the mutation pass and added no test (`report-b.md`;
  section 5).
- A third agent wrote the spec notes: G48 to G55. It amended G04, G13, G14, G15, G19,
  G20, G31, G38, G43, G44 and G46 (`ev-handback-extra.txt`, part 7, maps the diff's
  hunks to these notes; G47 appears there only because the hunk that adds G48 starts on
  its trailing blank line). By the lead's account it replaced 12 `G-new` placeholders
  and added one `impl-choice` label, to S044's release (G38, as N29). None is left under
  `gleam-double/` at the work commit (`ev-handback-checks.txt`, part 6); the count of 12
  is not recorded.
- At the checkpoint one runner subagent per lens drove Codex, and one fresh triage
  subagent checked the findings and applied the fixes (section 6).
- The harness refuses report files from subagents. The lead saved the replies of A and B
  as `report-a.md` and `report-b.md`.

#### Step 2: the plan

`plan-change` was loaded in GD01 and applied here. Agent A checked each owned scenario
against its Rust test: none needs a lapse (`report-a.md`). So no scenario moved under
shared rule 2, and no other ticket's ownership table changed.

#### Step 3: the ports, red first

The scenarios, by module, each under its `Scenario` tag (`ev-handback-checks.txt`, part
5, finds one tag for each):

| Module | Scenarios |
| --- | --- |
| `publish_test` | S021, S082, S083 |
| `settle_test` | S043, S044, S045, S046, S071, S072 |
| `pull_test` | S037 |
| `lease_test` | S054, S091 |

The `impl-choice` labels, each in its test's comment:

- The absolute positions in S043, S044, S045, S046, S054, S071, S072, S082 and S083
  (G48, extending G04).
- S082's superseded-queue order (G19).
- S037's `records[0]` (G31 and N23).
- S091's order of entries, and S044's order of `Freed` in its fired list (G38 and G49,
  as N29).

The Rust assertions not ported literally, each with its reason:

- Each publish's outcome. Each port asserts the reply `Ok(Outcome([rule], None))` and the
  state through the snapshot (G54, the ruling on N32). After checkpoint C1, S021's port
  compares the whole record with the one read before the deferral (section 6).
- S054's `PocketFate::Queued`. The port asserts the reply `ReleaseRequeuesPocket` then
  `ConsumeAckedVersionQueueDeferred`, and that the pocket is queued (G54).
- S091's literal `[("b", 3), ("z", 1)]`. The port compares the kept positions by key
  against the positions before the release (G49, the ruling on N31).

More than the Rust: S046, S082 and S083 refuse through `harness.refused`, which compares
the complete snapshot, the clock's reads and the lease-id draws.

D, G13's pending input: no test pinned `EqualRankRefused` for it. `held_record_guards`
now asserts `DeferBehindHeldVersion`, and that an equal rank finds no rule; `events`
covers the deferral's `PocketTouched` (`report-a.md`).

E, the premises of the seven arrival-position invariants: one test in `lease_test`, "the
premise of each arrival-position invariant is true in a state the checker sees after a
deferral" (`arrival_premises`). It drives S091's setup through the owner and reads
`inspection.views` once, after the last nack. `pk`'s held records there are `z` and `b`,
each held at rank 1 with rank 2 deferred, and `y` and `a`, nacked with nothing deferred.
It asserts each premise and that `invariants.check` names nothing (G55).

The first red, `ev-a-red-first.txt`, with agent A's tests written and its rows not yet
in. 22 tests failed.

```text
$ just gleam-double
...
FAIL publish_test.publishing into a leased pocket neither requeues it nor escapes the lease, and sheds no held version: crashed: ...
...
FAIL lease_test.the premise of each arrival-position invariant is true in a state the checker sees after a deferral: crashed: ... #(Function, "deferred_and_nacked") ... #(Value, Error(Refused(EqualRankRefused("pk", "b", 2)))) ...
...
231 tests run, 209 passed
coverage faults: 0
exit status: 1
```

The last red, `ev-a-red-last.txt`, 12 tests failing. `report-a.md` calls them all
`Freed` defects; by `ev-handback-fail-sites.txt` 11 of the 12 `FAIL` lines carry
`Defect("Freed", NoRuleHeld)`, and the twelfth is the `Freed` guard test, `guard_holds`:

```text
$ just gleam-double
...
FAIL lease_test.a release with only an acked record and its deferred version requeues the pocket rather than idling it: crashed: ... #(Value, Error(Defect("Freed", NoRuleHeld))) ...
...
231 tests run, 219 passed
coverage faults: 0
exit status: 1
```

What the pending lists held at the red runs, and which test the green run's 232nd is, are
not recorded.

Agent A's green run, `ev-a-green.txt`: `232 tests run, 232 passed`, `coverage faults: 0`,
exit status 0.

Never red, and why. `report-a.md` names two tests as never red. The evidence differs for
one of them (`ev-handback-never-red.txt`):

| Test | Red first? | Failed by |
| --- | --- | --- |
| "DeferBehindHeldVersion's effect without its held record is a defect, and reads no clock" (`defer_without_held_record`) | Never red: it builds its states by hand | No mutant |
| "the premise of each arrival-position invariant is true in a state the checker sees after a deferral" (`arrival_premises`) | Red in the first run, but in its setup, `deferred_and_nacked`, where a publish met `EqualRankRefused`; never at a premise assertion. Green by the last red run | D1, D4, D6, D9, D12, SP4, each in its setup, `deferred_and_nacked` (`ev-handback-fail-sites.txt`) |

The mutation pass, after green (`ev-b-mutants.txt`, which holds no `$` line; runner
`b-scratch/runner.py`, GD04's copied unchanged; definitions `b-scratch/mutants.py`; one
log for each mutant under `b-scratch/logs/`). Its last lines:

```text
D6 first form (Some(record.position), leaving position unused) failed only the warnings-as-errors build (exit 1, 0 tests failed); rewritten as Some(record.position + 0 * position) and rerun: killed.
totals: 53 mutants run, 53 killed, 0 survived, 1 build-only first form rewritten (D6); 0 tests added
```

- The 53: `DeferBehindHeldVersion` D1 to D14, `Supersede`'s held branch SP1 to SP5,
  `SupersedeUnsettledHeldVersion` SU1 to SU12, `ConsumeAckedVersionQueueDeferred` CA1 to
  CA7, `DeadLetterNackedVersionQueueDeferred` DN1 to DN12, and H1 (`arrival` one short),
  O1 (`outlasts`) and O2 (`model.ending`).
- Each of the twelve scenarios fails under at least one mutant, by the per-scenario
  lines that close the file: S021 by D12, S037 by D4, S043 by D10, S044 by CA1, S045 by
  SP3, S046 by D2, S054 by O1, S071 by CA2, S072 by DN5, S082 by SP1, S083 by D12 and
  S091 by SU9, the mutants `report-b.md` names.
- Three mutants failed exactly one test: D3 and DN12, each by its "changes exactly what
  it ensures" test, and D11, by the `PocketTouched` events test. DN12, the nacked version
  ending in both shed queues, is caught by that one test alone (Open points).
- The nine guard mutants were killed through a `GuardOverlap` they caused; the
  unmutated tables return none (`report-b.md`).
- `src/` in the worktree at the work commit matches all 16 lines of
  `b-scratch/manifest-src.sha256`, which agent B took before its first mutant
  (`ev-handback-extra.txt`, part 3). The triage changed nothing under `src/`.

Agent B's final run, `ev-b-green.txt`:

```text
$ just gleam-double
...
232 tests run, 232 passed
coverage faults: 0
exit status: 0
```

No ported scenario failed against rows that follow the specification, and dispatch
returned no `GuardOverlap` on the Publish table or the six-row `Freed` table. On exit
`pending_scenarios` holds 34 ids and `pending_rules` 7 rules.

#### Step 4: spec notes

`gleam-double/SPEC-NOTES.md` gained G48 to G55. Each cites its `N`-note or says it has
none (`ev-handback-extra.txt`, part 6). None departs from its `N`-note.

| Note | Title | `N`-note |
| --- | --- | --- |
| G48 | nine of GD05's rows assert absolute positions (`impl-choice`) | No `N`-note; extends G16, G29 and G33 |
| G49 | S091's kept positions are compared by key and against the state before (the maintainer's ruling on N31), and its order of entries is `impl-choice` | Agrees with N31 and follows its ruling; agrees with N29 |
| G50 | a deferred version shed by a later publish takes the publish's `now`, and a held version shed at the lease's end takes `ended_at` | Agrees with N30 |
| G51 | the arrival position is allotted at the deferral, never at the lease's end, and the deferral changes only what it ensures | No `N`-note |
| G52 | `SupersedeUnsettledHeldVersion` queues the record at its own position, which no `ensures` names | No `N`-note |
| G53 | the arrival position the queue-deferred rows read is optional, and only their guard promises it | No `N`-note |
| G54 | a deferral answers nothing (the maintainer's ruling on N32) | Agrees with N32 and follows its ruling |
| G55 | each arrival-position invariant's premise is met in a state the rules reach | No `N`-note |

- Step 4's candidates: S091's order (G49) and the time a deferred version shed by a
  later publish takes (G50).
- The notes' `just lint`, `ev-notes-lint.txt`, exit status 0, and `just gleam-double`,
  `ev-notes-green.txt`: `232 tests run, 232 passed`, `coverage faults: 0`, exit status 0.
- The checkpoint then corrected G51 and G54 (section 6).

### 5. Agent B's reply was cut short

By `report-b.md`'s heading, agent B's reply was truncated at its section on where the
specification was silent or ambiguous. The lead read the evidence files rather than
resume the agent. What the missing section said is not recorded. The rest of its account
differs from `ev-b-mutants.txt` in two places, and this record follows the evidence:

- It says five mutants span more than one line, and names seven: D3, D5, SU1 and SP3,
  and SP1, SP2 and SP5.
- Its "killed by one test only" list includes SU1, failed by 2 tests, and SU9, failed by
  3. Only D3, D11 and DN12 failed exactly one.

### 6. Checkpoint record

One checkpoint, at step 5: conformance and breakage, before the final commit.

- **`codex exec --help` on the day.** It was checked in GD04, on 2026-11-21, and was not
  rerun for GD05. Its output is `ai_tmp/gd/GD04/ev-codex-help.txt`, stamped
  `2026-11-21T06:09:24Z`. The two review logs were last written at
  `2026-11-21T08:01:27Z` and `08:02:31Z`, the same UTC day (`ev-handback-extra.txt`,
  part 5).
- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `sandbox: read-only`, one
  fresh process per lens, by the ticket's corrected command (`log-conformance.txt` and
  `log-breakage.txt` each open with those lines). Codex was on `PATH`, so no fallback to
  a Claude reviewer was needed.
- **Recovery.** Both findings files open `Source: last-message`: each is Codex's last
  message, recovered because a read-only Codex writes no file.
- **What the reviewers ran.** Nothing. Each says its evidence was read-only inspection.
  The triage ran each claim.
- **Breakage.** Codex answered "No BREAKAGE findings." and listed what it examined. Its
  log is 424303 bytes, so the answer came after reading, and is not an empty result.
- **Triage.** One fresh subagent, neither a reviewer nor an author.
- **Rounds.** One. No finding was rated high, by a reviewer or by the triage, so no
  second round was owed.
- **Not recorded.** The model of the runner and triage subagents.

It ran uncommitted, on base `5f3c430d3ce95f890acfcce317c898409a661f73`. Before any fix
`just gleam-double` ended `232 tests run, 232 passed`, `coverage faults: 0`, exit status 0
(`ev-baseline.txt`). The reviewed tree and the tree before the triage are the same,
`257c7347e1061b4a1cdfd7f6cfb90f4036b25277` (`tree-before-review.txt` and
`tree-before-triage.txt`). The conformance lens filed C1 and C2; the breakage lens filed
none.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | S021's port asserted the deferral's rule but not the state it leaves, which the ruling on N32 asks for in place of the Rust's `Superseded`; a deferral that keeps the old payload passed it | S021's port reads `a` and `next_position` before the deferral and asserts the whole record after it, and the counter one higher, pinning no absolute position; G54 names S021 |
| C2 | low | valid | G51 said the counter stays where the deferral left it, but `ReleaseRequeuesPocket` allots the pocket's position before `Freed` runs | G51 says each `Freed` row leaves the counter as it receives it |

Both were valid, and both were applied. Nothing was handed back. The triage changed two
files, both in this ticket's Files touched: `gleam-double/test/publish_test.gleam`
(S021's port) and `gleam-double/SPEC-NOTES.md` (G51 and G54). Nothing under
`gleam-double/src/` changed, no seam changed, and the test count stayed at 232, so the
pending lists did not change.

Each C1 mutant ran through `scratch/mutant.sh`. Each evidence file ends with the
mutant's diff and a `restored:` line whose two SHA-256 values match:

| Mutant | Run | Result |
| --- | --- | --- |
| The deferral keeps the old payload (`publish.gleam:262`) | Before the fix, `ev-C1-payload-survives.txt` | `232 tests run, 228 passed`, exit 1; S021's port is not among the four failures |
| The same | After the fix, `ev-C1-payload-killed.txt` | `232 tests run, 227 passed`, exit 1; S021's port is among the five |
| The deferral keeps the old rank (`publish.gleam:261`) | Before the fix, `ev-C1-rank-survives.txt` | `232 tests run, 218 passed`, exit 1. Despite the file's name the mutant is killed, by the checker's `OnlyDeferredHasArrivalPosition` among others, and S021's port fails (`ev-handback-extra.txt`, part 4) |

`ev-green-after-tests.txt` is `just gleam-double` after the fixes: `232 tests run, 232
passed`, `coverage faults: 0`, exit status 0. `ev-lint.txt` is `just lint` after the note
edits, exit status 0.

Tree ids: as reviewed and before the triage `257c7347e1061b4a1cdfd7f6cfb90f4036b25277`,
after it `495ba504e5547518b09bfdb826eb2ad84dcfad9a` (`tree-after-triage.txt`). The work
commit's tree is that second id (`ev-handback-checks.txt`, part 4), so the re-run below
ran on the tree that was committed. Whether `just check` ran between the triage and the
commit is not recorded; it ran at the work commit (section 7).

The re-run after the checkpoint (`ev-final.txt`), uncommitted, on base
`5f3c430d3ce95f890acfcce317c898409a661f73`:

```text
$ just gleam-double
...
232 tests run, 232 passed
coverage faults: 0
exit status: 0
```

### 7. Verification

The Verification block ran at `d279f7a3293e80e397eaf3c1e64769af3c3df03f`, the work
commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's block word
for word; output in `ev-verification.txt`). It exited 0, so its last two lines held:
`git status --short` printed nothing.

```text
$ sh ai_tmp/gd/GD05/verification.sh
d279f7a3293e80e397eaf3c1e64769af3c3df03f
...
gleam 1.19.1
...
OTP 29
...
232 tests run, 232 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The run counted 214 tests at the base (`baseline.log`), 231 at both red runs, and 232 at
agent A's green run (`ev-a-green.txt`), after the mutation pass (`ev-b-green.txt`) and
after the triage (`ev-final.txt`).

### 8. The acceptance criteria against the evidence

1. `just gleam-double` exits 0 and shows the counts: `ev-verification.txt`.
2. `pending_scenarios` holds exactly GD06's 34 ids and `pending_rules` its 7 rules:
   `ev-handback-pending.txt` compares the lists at the work commit with GD06's ownership
   table, as sets. `findings-conformance.md` says the same.
3. Every owned scenario has a tagged test, and each of the four rules is a row that
   fired: `coverage faults: 0` in `ev-verification.txt`, with none of the 12 ids or the 4
   rules pending (`ev-handback-pending.txt`). `ev-handback-checks.txt`, part 5, finds one
   tag for each of the 12.
4. The diff leaves the lease-end module's interface and its release callers unchanged.
   In `ev-handback-checks.txt`,
   `git diff 5f3c430 d279f7a -- gleam-double/src/pachinko_double/lease_end.gleam | rg '^[-+]pub'`
   prints nothing (`rg` exit 1), and
   `git diff --stat 5f3c430 d279f7a -- gleam-double/src/pachinko_double/settle.gleam`
   prints nothing. Part 3, the positive control, lists the module's five `pub` lines at
   the work commit. Since `rg '^[-+]pub'` cannot see a parameter line or a constructor,
   `ev-handback-extra.txt`, part 1, also compares each public block, from its `pub` line
   to its closing brace, at both commits: the only lines that differ are the three
   comments in `freed()`'s list, each replaced by its row.
5. The arrival-position invariants' positive controls still pass: `232 tests run, 232
   passed` and `coverage faults: 0` in `ev-verification.txt`, where the coverage check
   requires an `InvariantControl` for each invariant. The premise of each of the seven is
   true in a state the checker sees: `arrival_premises` (section 4, G55) passes in
   `ev-verification.txt`. It falls short in one way: no evidence file shows one of its
   premise assertions failing, since the six mutants that fail it fail it in its setup
   (`ev-handback-fail-sites.txt`; Open points).
6. The spec notes are there, each with its `N`-note line, S091's `impl-choice` among them
   in G49: section 4, step 4, and `ev-handback-extra.txt`, part 6.
7. `just gleam-double` leaves nothing Git can see and no `erl_crash.dump`:
   `ev-verification.txt`. The script tests both under `set -eu` and exits 0.
8. The checkpoint is recorded in section 6, with the `codex exec --help` check made on
   the same day in GD04.
9. `just check` ends with its line and `git status --short` prints nothing:
   `ev-verification.txt`.

### 9. Stops

None fired. The conditions watched:

- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did.
- A `GuardOverlap` on a specification table (shared rule 4). Dispatch returned none on
  the Publish table or the six-row `Freed` table.
- A deferred row that needs a change to the lease-end module's interface or a release
  caller (step 3). None did (section 8, criterion 4).
- A scenario that needs a lapse, or another operation this ticket does not land (shared
  rule 2). None moved.
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
- The mutation pass after the checkpoint. The 53 mutants ran before the triage changed
  S021's port; `src/` is the same at the work commit (section 4), so each kill still
  names a test that exists, but the pass was not repeated.
- The lapse half of each "by release or lapse" row. GD06 owns it.

Not recorded: the model of the runner, triage and author subagents; the missing section
of agent B's reply; what the pending lists held at the red runs; which test the green
run's 232nd is; the count of `G-new` placeholders replaced; and whether `just check` ran
between the triage and the commit.

### 11. Working lessons for GD06 and GD07

- When a review answers "no findings", check the log's size and what it lists as
  examined before recording it.
- A truncated reply can be completed from the evidence files instead of waking the
  agent; where its account and the evidence disagree, follow the evidence.
- An evidence file's name records what the author expected, not what happened: read its
  tail (`ev-C1-rank-survives.txt` ends with the mutant killed).
- `rg '^[-+]pub'` over a diff cannot see a public function's parameters or a type's
  constructors; compare the public blocks whole.

## Open points

- **PD07 asks whether the order in which a pocket's records are listed is part of the
  contract.** This ticket follows the ruling on N31 as it stands (G49); PD07's answer may
  change the port of S091.
- **The questions the notes leave for the maintainer.**
  - G48, as G04, G16, G29 and G33: is `next_position`'s starting value meant to be
    unspecified, and if so should these nine rows assert absolute positions?
  - G49, N31's and N29's: is the order in which a snapshot lists a pocket's records part
    of the inspection contract? Should the specification order the entries one lease end
    creates, or say that their order is not observable?
  - G50: should a reader of the superseded queue be told that `shed_at` does not order a
    key's versions?
  - G52: should `SupersedeUnsettledHeldVersion` ensure the record's position, or say that
    it keeps it, as the other two rules that queue a deferred version ensure theirs?
  - G54, N32's: should the `Publishing` surface name what a publish answers, so that a
    double or a second implementation can port it?
- **One test was never red, and no mutant fails it.** "DeferBehindHeldVersion's effect
  without its held record is a defect, and reads no clock" pins a defect over states
  built by hand. No evidence file shows that it can fail.
- **`arrival_premises` has failed only in its setup.** It was red in the first run, and
  each of the six mutants that fail it, in `deferred_and_nacked`
  (`ev-handback-fail-sites.txt`). No evidence file shows a premise assertion failing.
  The positive controls show each conclusion checked; that each premise is true rests
  on the test passing.
- **One version ending in both shed queues is caught by one test alone.** DN12 fails
  only "DeadLetterNackedVersionQueueDeferred changes exactly what it ensures, stamps the
  entry from ended_at and asks for no time".
- **The mutation pass predates the checkpoint's fix to S021's port.** `src/` is
  unchanged since, so no kill is lost, but no mutant ran against the new assertions
  except C1's payload mutant.
- **Both reviewers ran nothing.** Their findings, and the breakage lens's "No BREAKAGE
  findings.", rest on static inspection; the triage ran C1's and C2's claims.
- **The lapse half of each "by release or lapse" row is GD06's,** with S080 and S088.
  `ReviveParkedPocket`, and what `PocketTouched` does for a parked pocket, are GD06's too.
- **A lease-id source that repeated an id would give two leases one id (G27).** Carried
  from GD04. The double still does not check that an id it is handed is new.
