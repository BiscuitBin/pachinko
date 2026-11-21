---
id: GD01
title: "Gleam double: the toolchain, the owner actor and the joins"
status: done
depends_on: []
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD01: Gleam double: the toolchain, the owner actor and the joins

## Context

The Python double under `double/` re-implemented the domain `docs/specs/pachinko.allium`
defines from the specification alone, so that the maintainer could judge the
specification (`.scratch/python-double/README.md`). The maintainer now wants a second
double, in Gleam, built as a pure core behind one owner actor. It is a series of seven
tickets, GD01 to GD07, run one after another. Each lands the operations its ownership
table names, ports the scenarios it owns from the 93 in the catalog in
`docs/reference/testing.md`, and shrinks the coverage check's pending lists.
`.scratch/gleam-double/README.md` is the index, and its "Design fixed before GD01"
section is this ticket's brief.

The double lives under `gleam-double/`: the package `pachinko_double`, its sources under
`gleam-double/src/`, its tests under `gleam-double/test/`, and the spec-notes file
`gleam-double/SPEC-NOTES.md`. The HTTP service is out of scope, and so is a real ticker:
time moves only when a test sets the fake clock, and due work happens only when a test
sends `Tick`.

**Why one actor.** The specification's Lease expiry comment says the implementation
"applies the lapse and every action from one serialized stream (decision 0019)", and
decision 0019 rejects partitioning by pocket key, because positions and
`PullClaimsFrontPocket` need one linear order. So one owner process is faithful to the
specification, and a process per pocket, per lease or per consumer would not be. The
actor holds only the serialization. The rules live in a pure core the actor calls.

This ticket proves the toolchain on the thinnest real path and lays what every later
ticket builds on:

- decision 0021, which records the Gleam toolchain and the hex packages;
- the `gleam` and `erlang` pins in `pyproject.toml`, in a `gleam` feature with its own
  environment, and the relocked `pixi.lock`;
- the package, with `gleam.toml` and the committed `manifest.toml`;
- `just gleam-double` and `just gleam-double-deps`, with their rows in
  `docs/reference/commands.md`;
- the gate kept green with `gleam-double/build/` populated;
- the rule-table machinery: rows, tables, dispatch and `GuardOverlap`;
- the pure apply step, with chained events and the seam through which an effect asks for
  `now` and for a lease id;
- the owner actor, the fake clock and the fake lease-id source;
- the test runner;
- `Config` with its validation, and the two joins;
- the inspection snapshot for what this ticket holds;
- the spec-notes file.

No Gleam toolchain has run in this repository. The facts below come from documentation
checked on 2026-11-19 and from `BiscuitBin/knarr`, the maintainer's Gleam project, read
on 2026-11-20 at commit `d7cf7b41416446f1924f144702b74c385471561d`. Each is proved here
before anything is built on it.

| Fact | Source | Proved in |
| --- | --- | --- |
| conda-forge packages `gleam` 1.19.1 and `erlang` 29.1.2 for linux-64 and osx-arm64. Its `gleam` has no run dependency on `erlang`, so both are pinned | `api.anaconda.org/package/conda-forge/gleam`, `.../erlang` and the gleam feedstock's recipe | Step 3 |
| conda's `erlang` 29.1.2 runs conda's `gleam`. knarr pins `gleam = "==1.19.0"` and `erlang = "==29.1.2"` through pixi, so 1.19.0 is proved there and 1.19.1 is not proved anywhere | knarr's `pixi.toml` | Step 4 |
| Gleam has no frozen, locked or offline flag. A `manifest.toml` that disagrees with `gleam.toml` is rewritten, and the command exits 0 | knarr's decision 0003 and its `manifest-check` recipe | Step 5 |
| With `build/packages` populated and `manifest.toml` unchanged, a build and a test run reach no network | knarr's bootstrap evidence 03, on gleam 1.19.0; gleam's `dependencies.rs` | Step 5 |
| `gleam build --warnings-as-errors` compiles `test/` as well as `src/` | knarr's `docs/reference/testing.md` | Step 6 |
| `gleam test` runs `main` of `<package>_test`, and a panic in `main` exits 1 | `gleam.run/documentation/command-line-reference` and the compiler's `gleam@@main.erl` template | Step 6 |
| gleeunit runs through EUnit, which fixes no order and never runs a test in the caller's process, and then halts the VM | gleeunit's source and EUnit's documentation | Not used: the double has its own runner |
| `gleam_otp` 1.3.0's builder is `actor.new(state)`, `actor.on_message(handler)` and `actor.start`. `process.call` panics on a timeout or a dead callee | `gleam-otp.hexdocs.pm` and `gleam-erlang.hexdocs.pm` | Step 8 |
| A bare `gleam_otp` 1.3.0 actor cannot be stopped with `gen_server:stop` on OTP 29. It dies on an exit signal | knarr's `docs/reference/testing.md`, "Stopping a bare actor in a test" | Step 8 |
| `gleam_stdlib` has no regular-expression module, and `Dict`'s iteration order is not to be relied on | `gleam-stdlib.hexdocs.pm` | Shared rules 3 and 4 |
| Every package below publishes `build_tools = ["gleam"]`, so `rebar3` is not needed. knarr needs it only for `prometheus` | hex.pm's release metadata; knarr's decision 0003 | Step 5 |

The specification facts this ticket rests on, all from `docs/specs/pachinko.allium`:

- **Config.** Five fields with defaults: `lease_ttl` 5 minutes, `max_consecutive_expiries`
  3, `max_pull_batch` 100, `superseded_ttl` 12 hours and `dead_letter_ttl` 12 hours. Five
  invariants bound them: `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
  `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`. The double refuses a config
  that breaks one when it is made (N09), so the owner cannot be started with one. The
  checker's five entries for them land in GD02.
- **Identity.** `Publisher` and `Consumer` hold an id. `PublisherJoins` requires that no
  publisher has the id and creates one, and `ConsumerJoins` does the same for a consumer.
  A join with an id already taken is refused (N12).
- **Queue.** `Queue` holds `next_position`. Nothing in this ticket allots a position.
- **The surfaces.** `Publishing` provides `PublisherJoins` and `Publish`. `Consuming`
  provides `ConsumerJoins`, `Pull`, `Ack`, `Nack`, `Renew` and `Release`, and exposes the
  lease view. Those eight triggers are the owner's messages, beside `Inspect`,
  `LeaseView` and `Tick`. This ticket lands the two joins and `Inspect`.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/identity.rs`,
found with `rg -n 'scenario: S0NN'`, plus S005's doctest in
`crates/libpachinko/src/config.rs`.

## Goal

`just gleam-double` checks the format, builds with warnings as errors and runs the tests,
offline, and exits 0. A publisher and a consumer join through the owner actor, a second
join with a taken id is refused and changes nothing, and a config that breaks a config
invariant is refused when it is made. The rule-table machinery, the apply step, the seam
for `now` and lease ids, the owner, the fakes and the test runner are in place and
proved on scratch tables. `just check` is green with `gleam-double/build/` populated.
Every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (2) | `PublisherJoins`, `ConsumerJoins` |
| Scenarios (5) | S003, S004, S005, S009, S010 |

## Non-goals

- No HTTP, no wire format, no real clock, no ticker and no supervisor. The double is not
  an OTP application: a test starts the owner itself.
- No use of the double as an oracle for the Rust or for the Python double.
- No publish, pull, lease, ack, nack, renew, release, deferral, expiry or decay
  behaviour, no entity beyond `Queue`, `Publisher` and `Consumer`, no invariant checker
  and no coverage check. GD02 to GD06 land them.
- No change under `crates/`, `docs/specs/`, `double/`, `loadgen/` or `fuzz/`. A doubt
  about the specification becomes a spec note, never an edit. This ticket does add one
  record under `docs/decisions/`, which no other ticket in the series touches.
- No gleeunit, no property-test or snapshot package and no linter package.
- No place in `just check`'s recipe list.

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

  `<id>` is `GD01`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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

## The design to propose

The design checkpoint (step 8) reviews this layout and may change it. Record what changed
in the hand-back. The type shapes below come from a design pass that ran no Gleam, so
treat them as a starting point, not as code that compiles.

| Path | Holds |
| --- | --- |
| `gleam-double/gleam.toml` | The package `pachinko_double`, target `erlang`, with its dependencies as ranges |
| `gleam-double/manifest.toml` | The pin: written only by `gleam`, committed, never edited by hand |
| `gleam-double/src/pachinko_double/core.gleam` | The apply step: a trigger's table, the events it chains, then the checker from GD02, once per outcome. It returns the names of the rules that fired, and is the only position allotter |
| `gleam-double/src/pachinko_double/rules.gleam` | The rule-table machinery: a row `(name, guard, effect)`; the table for one trigger with its "no guard holds" answer; dispatch, which evaluates every guard; and the failure type, whose variants are a refusal, `GuardOverlap`, a defect and a broken invariant |
| `gleam-double/src/pachinko_double/owner.gleam` | The owner actor, its messages and its start function. The only module under `src/` that imports `gleam/erlang` or `gleam/otp` |
| `gleam-double/src/pachinko_double/identity.gleam` | The two joins' tables, the specification's Identity section |
| `gleam-double/src/pachinko_double/publish.gleam` | The Publish table and the `PocketTouched` table, the specification's Publish section. Created in GD02 with four Publish rows; GD04 adds `ReviveIdlePocket`, GD05 `DeferBehindHeldVersion` and GD06 `ReviveParkedPocket` |
| `gleam-double/src/pachinko_double/pull.gleam` | The Pull and `Handed` tables, the specification's Pull section. Created in GD03 |
| `gleam-double/src/pachinko_double/settle.gleam` | The Ack, Nack, Renew and Release tables, the specification's Ack and nack, and Renew and release, sections. Created in GD03 with the Renew table; GD04 adds the rest |
| `gleam-double/src/pachinko_double/lease_end.gleam` | The lease-end module: the shared end-of-lease steps, `ending_records` and the `Freed` table, the specification's Lease end section. Created in GD04 |
| `gleam-double/src/pachinko_double/expiry.gleam` | The lapse's clock trigger and the `LeaseLapsed` table, the specification's Lease expiry section. Created in GD06 |
| `gleam-double/src/pachinko_double/decay.gleam` | The two decays, the specification's Decay section. Created in GD06 |
| `gleam-double/src/pachinko_double/model.gleam` | The entities and their variants. This ticket lands `Queue`, `Publisher` and `Consumer`; GD02 lands the rest, every field the specification declares |
| `gleam-double/src/pachinko_double/config.gleam` | `Config`, an opaque type with the five defaults, whose constructor refuses a value that breaks a config invariant |
| `gleam-double/src/pachinko_double/refusals.gleam` | The refusals, one variant per kind, named as the `N`-notes name them where one does (N11, N20, N21), so that the two doubles' findings line up |
| `gleam-double/src/pachinko_double/inspection.gleam` | The complete immutable snapshot that the tests and the checker read, with its derived sets computed once |
| `gleam-double/src/pachinko_double/invariants.gleam` | The registry from invariant name to check, and `check(snapshot) -> violations`. Created in GD02 |
| `gleam-double/test/pachinko_double_test.gleam` | The runner: `main` lists every test module, runs each test in its own monitored process, then runs the coverage check from GD02 |
| `gleam-double/test/harness.gleam` | The fake clock, the fake lease-id source and the helpers that start an owner and call it |
| `gleam-double/test/` | One `*_test.gleam` module per area, each exporting its list of tests: config, identity, rules, core, owner. GD02 adds `coverage.gleam` |

The seam for `now` and lease ids, as proposed:

```gleam
pub type Effect(t) {
  Plain(fn(View, t, Draft) -> Draft)
  Timed(fn(View, t, Timestamp, Draft) -> Draft)
  Granting(fn(View, t, Timestamp, LeaseId, Draft) -> Draft)
}

pub type Step {
  Done(Result(Applied, Failure))
  NeedsNow(fn(Timestamp) -> Step)
  NeedsLeaseId(fn(LeaseId) -> Step)
}
```

`apply` returns a `Step`. The core asks for `now` only on reaching a row whose effect
reads it, so at most once per outcome and never on a refusal, and for a lease id only on
reaching a row that creates a lease. The owner drives the `Step` with its two injected
functions. The fallback, if `Step` proves awkward, is to pass a clock function into
`apply`; it is simpler, but the table then no longer shows which rows read time.

Decide and record in the spec notes or the hand-back:

- How a timestamp and a duration are represented. N01 chose whole milliseconds.
- `Step`, or the fallback.
- How a test reports the rules it fired to the runner. The constraint is that a rule is
  credited only by a test that passed.
- The timeout of a call to the owner, and the timeout of one test.
- Whether a test stops its owner. If it does, it uses an exit signal, not
  `gen_server:stop`, which a bare `gleam_otp` actor ignores on OTP 29.

The two recipes, as proposed and not yet run:

```just
gleam_bin := justfile_directory() / ".pixi" / "envs" / "gleam" / "bin"

gleam-double-deps:
    pixi install --locked -e gleam
    cd gleam-double && export PATH="{{ gleam_bin }}:$PATH"; if test -f manifest.toml; then before=$(cksum < manifest.toml); else before=new; fi; gleam deps download; [ "$before" = new ] || [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }

gleam-double:
    test -x "{{ gleam_bin }}/gleam" && test -x "{{ gleam_bin }}/erl" || { echo 'gleam-double: the gleam environment is missing; run just gleam-double-deps (needs network)' >&2; exit 2; }
    test -f gleam-double/build/packages/packages.toml || { echo 'gleam-double: the hex packages are missing; run just gleam-double-deps (needs network)' >&2; exit 2; }
    export PATH="{{ gleam_bin }}:$PATH"; gleam --version
    export PATH="{{ gleam_bin }}:$PATH"; erl -noshell -eval 'io:format("OTP ~s~n", [erlang:system_info(otp_release)]), halt().'
    cd gleam-double && export PATH="{{ gleam_bin }}:$PATH"; gleam format --check src test
    cd gleam-double && export PATH="{{ gleam_bin }}:$PATH"; before=$(cksum < manifest.toml); ERL_CRASH_DUMP_BYTES=0 gleam build --warnings-as-errors; [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }
    cd gleam-double && export PATH="{{ gleam_bin }}:$PATH"; before=$(cksum < manifest.toml); ERL_CRASH_DUMP_BYTES=0 gleam test; [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }
```

Both recipes rely on the `Justfile`'s `set shell := ["sh", "-eu", "-c"]`: under `-e` a
failing `gleam` command ends its line before the comparison runs, so the comparison's
status cannot hide it. Step 6's probes prove that.

`gleam` and `erlang` sit in their own pixi environment, `gleam`, not the default one, so
the `Justfile`'s exported `PATH` does not hold them and stays as it is. Each recipe line
is its own `sh`, so each line that runs `gleam` or `erl` puts `.pixi/envs/gleam/bin`
first on its own `PATH`; `gleam` finds `erl`, `erlc` and `escript` there by name. The
guard tests the two binaries at that path, not `command -v`, so that a `gleam` installed
elsewhere on the machine cannot stand in for the pinned one. `just gleam-double-deps` is
the double's one network recipe: it installs the `gleam` environment exactly as
`pixi.lock` pins it, then fetches the hex packages under the `cksum` guard.

knarr also runs a pre-check that compares `manifest.toml`'s `[requirements]` with
`gleam.toml` before any `gleam` command, so that drift fails before the rewrite. The
guard above fails after it. Record at the design checkpoint whether the double needs the
pre-check too.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/0021-gleam-double-toolchain.md` | new | Decision 0021: the Gleam toolchain and the hex packages |
| `docs/decisions/README.md` | shared, own lines | Decision 0021's row |
| `docs/manifest.yml` | shared, own lines | Decision 0021's entry |
| `pyproject.toml` | shared, own lines | A `gleam` feature holding the `gleam` and `erlang` pins, and a `gleam` environment with `no-default-feature = true`, with a comment naming decision 0021 |
| `pixi.lock` | shared | Relocked by `just lock`, never by hand. It gains the `gleam` environment; the default environment's packages do not move |
| `gleam-double/gleam.toml` | new | The package manifest |
| `gleam-double/manifest.toml` | new | Written by `just gleam-double-deps` |
| `gleam-double/src/` | new | The package, laid out as the design checkpoint settles |
| `gleam-double/test/` | new | The runner, the harness and the test modules |
| `gleam-double/SPEC-NOTES.md` | new | A header saying what the file is for, then this ticket's notes |
| `Justfile` | shared, own lines | The `gleam-double` and `gleam-double-deps` recipes, beside `python-double`, with a comment saying they are outside `just check` |
| `.gitignore` | shared, own lines | `/gleam-double/build/` and `erl_crash.dump`, with the reason |
| `taplo.toml` | shared, own lines | `gleam-double/build/**` and `gleam-double/manifest.toml`, only if step 5's red run asks for them |
| `.markdownlint-cli2.jsonc` | shared, own lines | `gleam-double/build`, only if step 5's red run asks for it |
| `_typos.toml` | shared, own lines | `gleam-double/manifest.toml`, only if step 5's red run asks for it |
| `.editorconfig` | shared, own lines | An entry for `*.gleam` or `manifest.toml`, only if editorconfig-checker objects |
| `renovate.json` | shared, own lines | Only if decision 0021 finds that Renovate must ignore or group the hex pins |
| `docs/reference/commands.md` | shared, own lines | Rows for `just gleam-double` and `just gleam-double-deps` |
| `docs/project/repository-map.md` | shared, own lines | A row for `gleam-double/`, the page that owns the repository's layout |
| `AGENTS.md` | shared, own lines | `just gleam-double-deps` added to the list of recipes that reach the network, and nothing else |
| `.scratch/gleam-double/GD01-toolchain-owner-and-joins.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD01's Status cell |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id in the hand-back. The worktree was
   initialized on 2026-11-20; if `.pixi/` or `.tools/` is missing, run `just initialize`
   (network, authorised by this step). Run `just check` and `just python-double` on the
   untouched worktree, and quote each one's last line, headed by its commit, as the
   baseline.
2. **The plan.** Use the `plan-change` skill. Read `docs/specs/pachinko.allium` in full,
   the findings and N01 to N12 in `double/SPEC-NOTES.md`, the index's "Design fixed
   before GD01" and "Where this departs from knarr", and the catalog rows this ticket
   owns. Confirm the three counts:
   `rg -c '^\| S[0-9]{3} ' docs/reference/testing.md` prints 93,
   `rg -c '^rule ' docs/specs/pachinko.allium` prints 32, and
   `rg -c '^\s*invariant ' docs/specs/pachinko.allium` prints 35. Confirm that
   `docs/decisions/` holds no record numbered 0021. If a count has moved or the number is
   taken, **stop** and put it to the maintainer.
3. **Decision 0021 and the pins.** Use the `review-docs` skill. Write the record first,
   then make the change it records. The record covers:
   - `gleam` and `erlang` from conda-forge through pixi, pinned exactly as decision 0011
     pins a tool, and why both are pinned;
   - that they sit in a feature, `gleam`, with its own environment and
     `no-default-feature = true`, as the maintainer directed on 2026-11-20 and as knarr
     keeps its cluster tools out of its default environment; that the default
     environment, the only one on the `Justfile`'s `PATH` and the one CI installs, is
     unchanged; and what the choice costs: a second environment in `pixi.lock`, whose
     hash keys CI's pixi and prek caches, a `PATH` set by each recipe line, and one more
     install step for anyone who runs the double. Confirm from
     `.github/actions/setup/action.yml` that it names no environment, and say that
     installing only the default one is then `setup-pixi`'s and `just sync`'s behaviour;
   - each hex package and what the double cannot do without it, which is the note
     decision 0007 asks of a crate, since that decision's own note is Cargo-specific;
   - ranges in `gleam.toml` with `manifest.toml` as the pin, the Gleam counterpart of
     decision 0007's `Cargo.lock`;
   - that Gleam has no frozen mode, and the guard the recipes carry instead;
   - why the double takes no gleeunit;
   - how the pins move: whether Renovate (decision 0018) reads the two pixi pins and
     `gleam-double/gleam.toml`, and whether `just audit` covers a hex package. If neither
     does, the record says the pins move by hand;
   - what would reopen it.

   Add its row to `docs/decisions/README.md` and its entry to `docs/manifest.yml`. Add
   the `gleam` feature, with the two pins, and the `gleam` environment to
   `pyproject.toml`. Run `just lock` (network, authorised by this step). It also runs
   `cargo update --workspace` three times, so confirm with `git status --short` that of
   the lockfiles only `pixi.lock` moved; if a `Cargo.lock` moved, **stop**. Confirm from
   `pixi.lock`'s diff that the default environment's packages did not move, so the
   installed default environment is still the locked one and `just initialize` is not
   run. Run `just lock-check` and `just check-docs`. The `gleam` environment is installed
   in step 4, by `just gleam-double-deps`.
4. **The package and the first green run, red first.** Write `gleam.toml` by hand rather
   than with `gleam new`, whose skeleton is not this repository's. Add the `.gitignore`
   lines and both recipes. Run `just gleam-double` before the `gleam` environment is
   installed and quote its refusal, which names the missing environment. Run
   `just gleam-double-deps` (network, authorised by this step), which installs the
   `gleam` environment and writes `manifest.toml`. Quote `gleam --version` and the OTP
   release `erl` reports, both from `.pixi/envs/gleam/bin`. Read `manifest.toml`: it
   should pin the four packages and what they bring, which is expected to be `filepath`
   alone. Put any other package to the maintainer before going on. One recipe now
   installs the environment and fetches the packages, so the second refusal, for missing
   packages, has no natural red: prove it by moving `gleam-double/build/` aside into
   `ai_tmp/`, quoting the refusal, and moving it back. Write the smallest runner `main`
   with one test, and run `just gleam-double` until it is green.
   - If conda's `erlang` 29.1.2 does not run `gleam` 1.19.1, pin `gleam` 1.19.0, the pair
     knarr proves, relock, and amend decision 0021.
   - If that fails too, **stop** and put the remaining fallback to the maintainer: the
     release tarball, checksum-pinned into `.tools/bin` as `just install-allium` installs
     its binary.
5. **The offline, manifest and gate probes.** Quote each probe with its output and exit
   status. Copy a file before a scratch change, restore it from the copy afterwards, and
   show a matching SHA-256.
   - **Offline.** With `gleam-double/build/` populated, `just gleam-double` passes with
     the network denied: on macOS under
     `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, on Linux under
     `unshare -rn`. The positive control is a `curl` to `https://hex.pm` that fails under
     the same denial. If the run reaches for the network, **stop**.
   - **The manifest guard.** Widen one range in `gleam.toml`. `just gleam-double` fails,
     naming `manifest.toml`. Restore both files.
   - **No `rebar3`.** `command -v rebar3` finds nothing, and the build is green.
   - **The gate with `build/` populated, red first.** Run `just toml-check` and
     `just lint` before any exclude is added, and quote what each reports under
     `gleam-double/`. Add only the entries a red run asks for, in `taplo.toml`,
     `.markdownlint-cli2.jsonc` and `_typos.toml`, then show both green. Check that
     lychee, editorconfig-checker, typos and ripsecrets pass over `gleam.toml`,
     `manifest.toml` and the sources, and that `gleam.toml` is in the layout
     `just toml-check` expects. Then run `just check`.
6. **The test runner, red first.** Write the registry, `Test(name, tags, run)` with the
   tags `Scenario` and `InvariantControl`, and the runner's `main`. It runs each test in
   its own monitored process with a timeout, prints each failure by name, prints the
   number of tests run and passed, and exits non-zero on any failure. Every
   `*_test.gleam` file on disk must be listed in `main`. Keep each test function private,
   so that one left out of its module's list is an unused function and fails the build.
   Quote each probe below with its output and exit status, then revert it:
   - a test whose assertion fails;
   - a test that panics;
   - a test that never returns;
   - a `*_test.gleam` module that `main` does not list;
   - a test function left out of its module's list, which fails the build and so shows
     that `--warnings-as-errors` covers `test/`;
   - a registry with no test in it.
7. **The rule tables, the apply step and the fakes.** Write the fake clock and the fake
   lease-id source in the harness. Write the row, the table, dispatch, the failure type
   and the apply step with its chained events and its seam for `now` and lease ids. No
   specification row exists yet, so each test below runs on a scratch table:
   - two guards that both hold return `GuardOverlap` naming both, with the second of
     them the last row, so that a dispatch that stops at the first guard fails the test;
   - one test per answer in shared rule 4's "no guard holds" table: a refusal, nothing
     happens, and a defect;
   - an effect that emits an event runs that event's table within the same outcome, and
     the fired names list both rows in order;
   - a row that reads `now` reads the fake clock once, a refusal reads it not at all, and
     two timed rows in one outcome still read it once;
   - a row that creates a lease draws one lease id, and no other row draws one.
8. **The owner.** Write the owner: started with a `Config`, a clock function and a
   lease-id function, taking `PublisherJoins`, `ConsumerJoins` and `Inspect`. Write the model's
   three entities, `Config`, the refusals this ticket needs and the snapshot for
   `next_position`, the publishers and the consumers. Settle each point "The design to
   propose" lists under "Decide and record". Give the owner its own tests:
   - a refusal is answered, the snapshot after it equals the snapshot before it, and the
     owner answers the next message;
   - several processes send the same join at once, exactly one is answered with success
     and the rest are refused, and the snapshot holds one publisher.

   **Adversarial review** (checkpoint 1: conformance, breakage and design, before any
   scenario is ported). Questions for this checkpoint:
   - Is the core pure, and is `owner` the only module under `src/` that imports
     `gleam/erlang` or `gleam/otp`?
   - Can anything under `src/` panic, and does the owner's handler have exactly one
     branch for an `Error`, which keeps the old state?
   - Are the guards pure, and does dispatch evaluate every guard rather than stop at the
     first that holds?
   - Does each table state its "no guard holds" answer?
   - Are the clock and the lease ids injected, can a guard or a refusal read the clock,
     and does anything read the wall clock or draw randomness?
   - Does the runner fail closed? Look at a crashed test, a timeout, an unlisted module,
     an unlisted function and an empty registry.
   - Can a tag be credited by a test that failed or never ran?
   - Could `just gleam-double` rewrite a tracked file, reach the network or leave a file
     Git can see?
9. **Port the scenarios, red first.** For each scenario this ticket owns, write its
   tagged test against the Rust test's setup, steps and assertions. See it fail, or say
   in the hand-back why it cannot be red. Then add the rule that makes it pass as a row
   of its join's table. Each test asserts the names of the rules it fired. Each refusal
   test compares the complete snapshot before and after: a second join of each kind, and
   the three config refusals, which leave no owner to inspect and so assert the refusal
   itself. S005's run-time refusal of a negative TTL is `impl-choice` (N09).
10. **Spec notes.** Write `gleam-double/SPEC-NOTES.md`: a header saying what the file is
    for and how a `G`-note cites its `N`-note, then this ticket's notes. They cover the
    representation of a timestamp and a duration (N01), the config refused when it is
    made (N09), the second join (N12), and the seam for `now` and lease ids (N15, N28).
11. **The handbook and the agent contract.** Use the `review-docs` skill. Add the rows
    for both recipes to `docs/reference/commands.md` and the `gleam-double/` row to
    `docs/project/repository-map.md`. Add `just gleam-double-deps` to the list of recipes
    that reach the network in `AGENTS.md`, and change nothing else there. Run
    `just check-docs` and `just check-agents`.
12. **Adversarial review** (checkpoint 2: conformance and breakage, before the final
    commit). The breakage lens asks whether `just gleam-double` could exit 0 while a test
    failed or never ran, while the build reached the network, or while `manifest.toml`
    was rewritten.
13. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
    commit and quote it, headed by the full commit id. Set `status: done` here and in
    `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
    record. **Stop** before pushing or opening a pull request: both need authorisation
    (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the version of `gleam`, the OTP
  release and the number of tests run and passed.
- Decision 0021 is registered in `docs/manifest.yml` and listed in
  `docs/decisions/README.md`, and it answers each point step 3 lists.
- `pyproject.toml` pins `gleam` and `erlang` exactly, in the `gleam` feature;
  `just lock-check` passes; of the lockfiles only `pixi.lock` moved; and the default
  environment's locked packages did not move.
- `gleam-double/manifest.toml` is committed and pins only the packages decision 0021
  records and what they bring.
- Step 5's probes are quoted with their output and exit status: the offline run with its
  positive control, the manifest guard, and the gate red before the excludes and green
  after them.
- Step 6's six runner probes are quoted with their output and exit status.
- The dispatch control shows a scratch table with two overlapping guards returning
  `GuardOverlap` naming both, each "no guard holds" answer has its test, and the chained
  event, the single clock read and the single lease-id draw each have theirs.
- A quoted `rg`, with its positive control, shows no `panic`, `todo`, `assert` or
  `let assert` in code under `gleam-double/src/`.
- Both joins go through the owner. Each rule this ticket owns fired in at least one test
  that asserts its name.
- Every refusal this ticket lands has a test showing that the complete snapshot is
  unchanged, and the owner's concurrent-join test passes.
- The fake clock counts its reads and the fake lease-id source counts its draws, though
  no specification rule here reads either.
- `gleam-double/SPEC-NOTES.md` holds a note for S005's `impl-choice` and for each
  representation choice this ticket made, each citing its `N`-note.
- `just gleam-double` leaves nothing Git can see, and no `erl_crash.dump`.
- Both checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.` with
  `gleam-double/build/` populated, and `git status --short` prints nothing after the
  commit.

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
just lock-check
just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Evidence files are under `ai_tmp/gd/GD01/`, and the review records under
`ai_tmp/review/GD01-1/` and `ai_tmp/review/GD01-2/`. None is committed. In every quote
below the recipe's echo of its own command lines is left out, and `...` marks other lines
left out or a line cut short. Where a quote the record should hold was never saved, the
text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, the amendment that put `gleam`
  and `erlang` in their own pixi feature. It sits on
  `cca486f29c0149be575def14d44055b56ae84181`, which added the seven tickets.
- Work commit: `0db1adb86e7d02c0a39f97c2fcebc931c9dc9529`. The commit after it holds this
  record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- `just initialize` was not run. It was not needed: step 3 left the default environment's
  locked packages as they were.

### 2. Baseline

At `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, on the untouched worktree
(`baseline.log`):

```text
5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae
...
All checks passed and the worktree is unchanged.
check-exit=0
...
Ran 274 tests in 0.771s

OK
python-double-exit=0
```

Step 2 confirmed the three counts, 93 catalog rows, 32 rules and 35 invariants, and that
`docs/decisions/` held no record numbered 0021. The output of the three `rg -c` commands
is not recorded.

### 3. The design as landed

This section is what a later ticket's author rereads. Sources are under
`gleam-double/src/pachinko_double/` and tests under `gleam-double/test/`.

| Module | Holds |
| --- | --- |
| `rules.gleam` | The engine, generic in state, event, answer and refusal: rows, tables, dispatch, the interpreter, the failure type, `Step` and `drive` |
| `domain.gleam` | `Event` and `Answer`, with no constructors yet, and the aliases that fix the engine to the domain: `Plan`, `Row(input)`, `Table(input)`, `Failure`, `Applied`, `Step`, `Cursor`, `World` |
| `core.gleam` | `Trigger`, one variant per trigger; `apply(state, trigger) -> Step`; `world()`; `chain`, which routes nothing yet |
| `identity.gleam` | The two join tables |
| `model.gleam` | `State(config, queue, publishers, consumers)` and `model.new(config)` |
| `config.gleam` | `Settings`, the opaque `Config`, `defaults()`, `default()`, `new`, `broken`, `settings` |
| `refusals.gleam` | `ConfigRefused`, `PublisherAlreadyJoined`, `ConsumerAlreadyJoined` |
| `inspection.gleam` | `Snapshot(next_position, publishers, consumers)` and `snapshot(state)` |
| `time.gleam` | `Timestamp`, `Duration` and the constants |
| `owner.gleam` | `start(config, now, lease_id)`; `Message`, which is `Act(trigger, reply)` or `Inspect(reply)`; `Outcome(fired, answer)`; `Reply`; `publisher_joins`, `consumer_joins`, `inspect`; `call_timeout` |
| `test/runner.gleam` | `Test(name, tags, run)`; `Tag`, which is `Scenario` or `InvariantControl`; `Module`; `Report`, `Ran`, `Outcome`; `run`, `run_one`, `registry_faults`, `failures`, `counts` |
| `test/pachinko_double_test.gleam` | `main` and the module list |
| `test/harness.gleam` | `start_clock`, `clock`, `set_time`, `advance`, `clock_reads`; `start_lease_ids`, `lease_ids`, `lease_id_draws`; `Double`, `start`, `fixture`, `fixture_settings`, `t0`, `fired`, `publisher_joins`, `consumer_joins`, `inspect` |
| `test/*_test.gleam` | `config_test`, `identity_test`, `owner_test`, `rules_test` and `runner_test`, each exporting `tests()` |

The engine's types, in `rules.gleam`:

- `Plan`, what a row's effect returns: `Finish(state, events, answer: Option)`,
  `Now(next)`, `LeaseId(next)`, `Position(next)` and `Impossible(reason)`. A row writes
  `use now <- Now`, and likewise for the other two requests.
- `Row(name, guard: fn(state, input) -> Bool, effect: fn(state, input) -> Plan)`.
- `Otherwise`, a table's "no guard holds" answer: `Refuses(fn(state, input) -> refusal)`,
  `NothingHappens` or `Defective`.
- `Table(trigger, rows, otherwise)`, and `rules.names(table)`.
- `Failure`: `Refused(refusal)`, `GuardOverlap(trigger, rules)`, `Defect(trigger, cause)`
  and `BrokenInvariant(invariants)`. A `Cause` is `NoRuleHeld` or
  `EffectFailed(rule, reason)`.
- `Applied(state, fired, answer)`.
- `Step`: `Done(result)`, `NeedsNow(next)` and `NeedsLeaseId(next)`.
- `Cursor`, opaque: `rules.start(state)`, `rules.state(cursor)`,
  `rules.finish(cursor, answer)`, `rules.fail(failure)` and `rules.at(cursor, now)`. `at`
  is for a trigger that carries the `now` it read: a `Tick` returns `NeedsNow` itself,
  then calls `at`.
- `World(next_position, with_next_position, chain)`. `chain(event, cursor, then)` routes
  an emitted event to its table.
- `rules.run(table, input, cursor, world, then)`, `rules.dispatch`, and
  `rules.drive(step, now, lease_id)`.

The checker, from GD02, goes where `core` calls `rules.finish`.

Departures from "The design to propose", and what each costs:

| Proposed | Landed | Cost |
| --- | --- | --- |
| `Effect(t)` with `Plain`, `Timed` and `Granting` | A row's effect returns a `Plan`. `Step` is kept as the seam between `core` and `owner` | The table no longer shows statically which rows read time; a row shows it by opening with `use now <- Now`. In return a guard still cannot reach a clock, since nothing hands it one, one outcome reads the clock once, and one row can ask for time, an id and positions together |
| `core` is the only position allotter | The interpreter in `rules` allots, through the two accessors `core.world` hands it, and overwrites whatever counter a row finishes with | Shared rule 4's sentence no longer describes the code word for word (Open points). A scratch test shows a row cannot move the counter |
| `rules.gleam` holds the domain's machinery | `rules.gleam` is generic, so the apply step is tested on scratch tables as step 7 asks; `domain.gleam` and `time.gleam` are new to the layout | Two more modules. Neither holds behaviour |
| The owner drives the `Step` | `rules.drive` runs a `Step` with the two injected functions, and the owner is its only caller under `src/` | Effect driving sits beside `Step` in `rules`, not in `owner`. `drive` performs no effect of its own, and `owner` is still the only module that imports `gleam/erlang` or `gleam/otp` |
| One owner message per trigger | The mailbox is `Act(Trigger, reply)` and `Inspect`; the per-trigger variants are in `core.Trigger` | The message type no longer lists the triggers. The handler has one `Error` branch for every trigger |
| A failure type of four variants | The same four; `Defect` carries a `Cause`, and `Plan` has `Impossible(reason)` (checkpoint 1, D2) | `Defect(trigger)` is now `Defect(trigger, cause)` |
| `Config`, an opaque type whose constructor refuses | `Config` is opaque over a public `Settings` record, and `config.new(Settings)` refuses | None recorded |
| The runner in `pachinko_double_test.gleam`; test modules for config, identity, rules, core and owner | The runner's mechanics are `test/runner.gleam`, and the entry module only lists. There is a `runner_test` and no `core_test`: the apply step's controls are in `rules_test`, because the interpreter is in `rules` | None recorded |
| Two recipes | A third, `just gleam-double-format`: `gleam format --check` fails on unformatted sources, and `AGENTS.md` forbids a bare `gleam` | One more row in `docs/reference/commands.md`. The recipe has no pin check (checkpoint 2, B1) |
| `just gleam-double` with two precondition lines and the `cksum` guard | Five precondition lines, in this order: the pinned binaries exist; `build/packages/packages.toml` exists; `gleam.toml` and `manifest.toml` agree on the requirements (step 5); `build/packages` holds the versions `manifest.toml` pins (checkpoint 1, B3); the `gleam` environment's `conda-meta` holds the versions `pyproject.toml` pins (checkpoint 2, B1). Then the two version prints, the format check, and the guarded build and test | Three lines of Python run before any `gleam` command. The fifth reads the pins from `pyproject.toml`, not from `pixi.lock` |

The "Decide and record" answers:

- **Timestamp and duration.** Whole milliseconds, as `Int` (G01, which agrees with N01).
- **`Step`, or the fallback.** `Step` is kept. The effect kinds are replaced by `Plan`,
  as above.
- **How a test reports the rules it fired.** `Test.run` returns the list of names, so
  only a test that returned, and so passed, credits a rule. `harness.Double` logs the
  `fired` of every successful owner call, and a test returns `harness.fired(double)`.
- **Timeouts.** A call to the owner waits 1000 ms (`owner.call_timeout`). One test has
  5000 ms (`runner.test_timeout`).
- **Whether a test stops its owner.** It does not. The owner is linked to the test's
  process, because `actor.start` uses `process.spawn`, so it dies with a test that
  crashes or is killed on timeout; otherwise it ends with the VM. No `gen_server:stop` is
  used anywhere.
- **The pre-check.** Needed, and added in step 5. Checkpoint 1 added a second, for the
  package cache (B3), and checkpoint 2 a third, for the pixi pins (B1).

### 4. Steps 3 to 9

Every output in this section ran uncommitted, on base
`5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, except the last subsection's, which ran at
the work commit. Each subsection repeats its head.

#### Step 3: decision 0021 and the pins

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

Decision 0021 was written first. Then `pyproject.toml` gained
`[tool.pixi.feature.gleam.dependencies]` with `gleam = "==1.19.1"` and
`erlang = "==29.1.2"`, and the `gleam` environment with `no-default-feature = true`.
`just lock` ran once (`lock.log`, which holds no exit status):

```text
cargo update --workspace
     Locking 0 packages to highest Rust 1.99 compatible versions
...
pixi lock
✔ Updated lock file
Environment: gleam
...
Platform: gleam:osx-arm64
  + (conda) erlang           29.1.2 pl5321h4aabdd8_0
  + (conda) gleam            1.19.1 hc2d82ec_0
...
Platform: gleam:linux-64
  + (conda) erlang           29.1.2 pl5321h8f2c242_0
  + (conda) gleam            1.19.1 hf19af3b_0
```

- Of the lockfiles only `pixi.lock` moved: the work commit holds `pixi.lock` and no
  `Cargo.lock`. The `git status --short` output taken after the relock is not recorded.
- The default environment's package list was byte-identical before and after, by a diff
  of the `default:` block against the base's `pixi.lock`. That diff's output is not
  recorded.
- `.github/actions/setup/action.yml` passes `setup-pixi` no `environments` input, and
  `just sync` runs `pixi install --frozen` with no `-e`, so both install only the default
  environment.
- The step's own `just lock-check` and `just check-docs` runs are not recorded. Both pass
  at the work commit (section 6).
- The local pixi, 0.84.1, wrote the lock. Open points holds what that leaves.

#### Step 4: the package and the first green run

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`. The first three files
record their status as `exit=N` and hold no `$` line.

Before the `gleam` environment was installed, `just gleam-double` refused
(`ev-step4-red-env.txt`):

```text
gleam-double: the gleam environment is missing; run just gleam-double-deps (needs network)
error: recipe `gleam-double` failed on line 458 with exit code 2
exit=2
```

`just gleam-double-deps`, with the network (`ev-step4-deps.txt`):

```text
✔ The gleam environment has been installed.
  Resolving versions
Downloading packages
 Downloaded 5 packages in 0.11s
      Added filepath v1.1.2
      Added gleam_erlang v1.3.0
      Added gleam_otp v1.3.0
      Added gleam_stdlib v1.1.0
      Added simplifile v2.7.0
exit=0
```

`manifest.toml` pins `filepath` 1.1.2, `gleam_erlang` 1.3.0, `gleam_otp` 1.3.0,
`gleam_stdlib` 1.1.0 and `simplifile` 2.7.0, each from hex with
`build_tools = ["gleam"]`: the four packages decision 0021 records, and `filepath`, which
`simplifile` brings. No other package came, so nothing was put to the maintainer.

With `gleam-double/build/` moved aside into `ai_tmp/`, the second refusal
(`ev-step4-red-packages.txt`); the directory was then moved back:

```text
gleam-double: the hex packages are missing; run just gleam-double-deps (needs network)
error: recipe `gleam-double` failed on line 459 with exit code 2
exit=2
```

The first green run, with both versions from `.pixi/envs/gleam/bin`
(`ev-step4-green.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
   Compiled in 0.18s
   Compiled in 0.19s
    Running pachinko_double_test.main
1 tests run, 1 passed
exit status: 0
```

Conda's `erlang` 29.1.2 runs `gleam` 1.19.1, so neither fallback was needed.

#### Step 5: the offline, manifest and gate probes

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

**Offline.** Two runs under the denial passed. `ev-step5-offline.txt` is an incremental
run, `1 tests run, 1 passed`, exit status 0. `ev-step5-offline-clean.txt` ran with
`build/dev` moved aside, so every package recompiled:

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) just gleam-double
gleam 1.19.1
OTP 29
  Compiling gleam_stdlib
  Compiling filepath
  Compiling gleam_erlang
  Compiling gleam_otp
  Compiling simplifile
  Compiling pachinko_double
   Compiled in 0.40s
...
1 tests run, 1 passed
exit status: 0
```

The positive control, under the same denial (`ev-step5-offline-control.txt`):

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) curl -sS -m 10 -o /dev/null https://hex.pm
curl: (6) Could not resolve host: hex.pm
exit status: 6
```

The same shell had the network minutes earlier, when `just gleam-double-deps` fetched
from hex.pm in it. No `curl` was run without the denial, since one `curl` was authorised.
Both offline runs predate the recipe's pre-check lines; section 6 repeats the run at the
work commit.

**The manifest guard.** It did not behave as the ticket assumed. With one range widened
in `gleam.toml` and the network denied, `gleam` did not rewrite `manifest.toml`. It tried
to resolve over the network and failed (`ev-step5-drift-without-precheck.txt`):

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) just gleam-double
gleam 1.19.1
OTP 29
  Resolving versions
error: Dependency resolution failed

An error occurred while choosing the version of gleam_otp: error sending
request for url (https://repo.hex.pm/packages/gleam_otp)

error: recipe `gleam-double` failed on line 463 with exit code 1
exit status: 1
```

So a drifted `gleam.toml` sends `just gleam-double` to the network. That is the case
knarr's pre-check exists for, and the pre-check was added: the recipe compares the
requirements of `gleam.toml` and `manifest.toml` before any `gleam` command. Red, under
the denial, with the same widened range (`ev-step5-precheck-red.txt`):

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) just gleam-double
gleam-double: gleam.toml and manifest.toml disagree on the requirements; gleam would resolve them over the network and rewrite manifest.toml
error: recipe `gleam-double` failed on line 464 with exit code 1
exit status: 1
```

The `cksum` backstop was proved by appending a line to `manifest.toml` while
`gleam build` ran (`ev-step5-cksum-guard.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
  Compiling gleam_stdlib
...
  Compiling pachinko_double
   Compiled in 0.41s
gleam rewrote gleam-double/manifest.toml; read the diff
error: recipe `gleam-double` failed on line 468 with exit code 1
exit status: 1
```

Not shown here: that `gleam` itself rewrites a drifted manifest and exits 0. That stays
knarr's finding (Open points).

Both files were restored from their copies. Their SHA-256 before the probes
(`ev-step5-guard-sha-before.txt`):

```text
b1769d2578b742d0b9671e0a883cf2d73d971de88127fe02a91c8b1fbb999058  gleam-double/gleam.toml
38143e080cb1ede330cf37abcc65a0537aaa9a306ce1af3360d85d568392bb15  gleam-double/manifest.toml
```

The hashes taken after the restore are not recorded as output. The same two values stand
in `ai_tmp/review/GD01-2/sha-before.txt`, taken at checkpoint 2, and the run after the
restore was green: `1 tests run, 1 passed`, exit status 0 (`ev-step5-after-restore.txt`).

**No `rebar3`.** `command -v rebar3` exited 1, and `.pixi/envs/gleam/bin` holds none. The
output is not recorded. Every run above built without it.

**The gate with `build/` populated, red first.** Before any exclude,
`just toml-check` (`ev-step5-toml-red.txt`):

```text
$ just toml-check
...
ERROR taplo:format_files: the file is not properly formatted path=".../gleam-double/build/packages/gleam_otp/gleam.toml"
ERROR taplo:format_files: the file is not properly formatted path=".../gleam-double/manifest.toml"
ERROR operation failed error=some files were not properly formatted
error: recipe `toml-check` failed on line 304 with exit code 1
exit status: 1
```

and `just lint` (`ev-step5-lint-red.txt`), in which two hooks failed and markdownlint
named only the packages' `README.md` files:

```text
$ just lint
...
taplo format and lint....................................................Failed
- hook id: toml-check
- exit code: 1
...
markdownlint.............................................................Failed
- hook id: markdownlint-cli2
...
  gleam-double/build/packages/gleam_erlang/README.md:8 error MD012/no-multiple-blanks Multiple consecutive blank lines [Expected: 1; Actual: 2]
...
error: recipe `lint` failed on line 295 with exit code 1
exit status: 1
```

Added, and nothing else: `gleam-double/build/**` and `gleam-double/manifest.toml` in
`taplo.toml`, and `gleam-double/build` in `.markdownlint-cli2.jsonc`. `_typos.toml` and
`.editorconfig` needed nothing and are unchanged. `gleam.toml` is not excluded, so
`just toml-check` still reads it. After the excludes `just toml-check` exited 0
(`ev-step5-toml-green.txt`), and so did `just lint` (`ev-step5-lint-green.txt`). Its
second pass runs the hooks over the untracked files, which `gleam-double/` then was. The
lines below are from that pass:

```text
$ just lint
...
taplo format and lint....................................................Passed
...
EditorConfig.............................................................Passed
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
...
ripsecrets...............................................................Passed
exit status: 0
```

Then `just check`, with `build/` populated (`ev-step5-check.txt`):

```text
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

#### Step 6: the six runner probes

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`. Each probe is
`just gleam-double` with one scratch change, reverted afterwards. At the time line 468 of
the `Justfile` was the build and line 469 the test run.

A test whose assertion fails (`ev-step6-assertion-fails.txt`). A first try used
`assert 1 == 2`, which the compiler rejects as a warning; the recorded probe asserts on a
computed value:

```text
FAIL runner_test.one is two: crashed: ProcessDown(... #(Function, "one_is_two"), #(Line, 9), #(Message, "Assertion failed."), ...
1 tests run, 0 passed
...
the Gleam double's tests failed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

A test that panics (`ev-step6-panics.txt`):

```text
FAIL runner_test.it panics: crashed: ProcessDown(... #(Function, "it_panics"), #(Line, 8), #(Message, "probe"), ...
1 tests run, 0 passed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

A test that never returns (`ev-step6-never-returns.txt`):

```text
FAIL runner_test.it never returns: timed out after 5000 ms
1 tests run, 0 passed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

A `*_test.gleam` module that `main` does not list, as the check stood at step 6
(`ev-step6-unlisted-module.txt`):

```text
FAULT orphan_test.gleam is on disk and not listed in main
1 tests run, 1 passed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

Checkpoint 1's second round replaced that check with one that reads `main`'s import
lines. The fault as it stands, from the "Round 2" section of
`ai_tmp/review/GD01-1/verdicts.md`, with a probe module on disk that `main` listed and
did not import:

```text
FAULT probe_test.gleam is on disk and main does not import it
19 tests run, 19 passed
error: recipe `gleam-double` failed on line 474 with exit code 1
```

A test function left out of its module's list (`ev-step6-unlisted-function.txt`). The
build fails, which shows that `--warnings-as-errors` covers `test/`:

```text
warning: Unused private function
...
11 │ fn left_out() -> List(String) {
   │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This private function is never used
...
error: 1 warning generated.
...
error: recipe `gleam-double` failed on line 468 with exit code 1
exit status: 1
```

A registry with no test in it (`ev-step6-empty-registry.txt`):

```text
FAULT the registry holds no test
FAULT runner_test lists no test
0 tests run, 0 passed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

The two files the probes edited, before them (`ev-step6-sha-before.txt`):

```text
cb33781d0363727078690311b96c391fdbfaca440ff3588884676a9d85a10053  gleam-double/test/runner_test.gleam
301ca7a998c42ceb40e78b08af3460d6760d4883054347e58a7bb388176fe387  gleam-double/test/pachinko_double_test.gleam
```

They were restored from copies. The hashes taken after the restore are not recorded as
output. The run after it was green: `1 tests run, 1 passed`, exit status 0
(`ev-step6-after-restore.txt`).

#### Step 7: the rule tables, the apply step and the fakes

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

The engine's tests were written after the engine, so their red is by mutation of
`rules.gleam`, restored after each. Three mutants each failed one test:

| File | Mutant | Key output lines | Exit |
| --- | --- | --- | --- |
| `ev-step7-mutant-first-guard-wins.txt` | The first guard that holds wins | `FAIL rules_test.two guards that hold are an overlap naming both: crashed: ...`; `11 tests run, 10 passed` | 1 |
| `ev-step7-mutant-clock-not-cached.txt` | The clock's reading is not cached | `FAIL rules_test.two timed rows in one outcome read the clock once: crashed: ...`; `11 tests run, 10 passed` | 1 |
| `ev-step7-mutant-counter-not-overwritten.txt` | The counter a row finishes with is not overwritten | `FAIL rules_test.a row is allotted positions and cannot move the counter: crashed: ...`; `11 tests run, 10 passed` | 1 |

A fourth mutant, events not chained, failed no test. The recorded run stopped at the
format check, before the build (`ev-step7-mutant-events-not-chained.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
error: These files have not been formatted

  - src/pachinko_double/rules.gleam

error: recipe `gleam-double` failed on line 467 with exit code 1
exit status: 1
```

The lead's notes say this mutant failed the build on an unused variable. The evidence
file shows the format check, and this record follows the file. Either way the test
"an emitted event runs its table in the same outcome" has no mutation red on record.

`rules.gleam` before the mutants (`ev-step7-sha-before.txt`); the hash after the last
restore is not recorded as output:

```text
b4d4e8d0e278e296f93afed4efebca8df8e3503ea96cd8639b45487ce4a17aba  gleam-double/src/pachinko_double/rules.gleam
```

Green (`ev-step7-green.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
...
11 tests run, 11 passed
exit status: 0
```

The scratch-table tests step 7 asks for, by their registered names in `rules_test`:
"two guards that hold are an overlap naming both"; "no guard holds: a refusal",
"no guard holds: nothing happens" and "no guard holds: a defect"; "an emitted event runs
its table in the same outcome"; "a row that reads now reads the clock once", "a refusal
reads no clock and draws no lease id" and "two timed rows in one outcome read the clock
once"; and "only a row that creates a lease draws a lease id". The fake clock counts its
reads (`harness.clock_reads`) and the fake lease-id source its draws
(`harness.lease_id_draws`).

#### Step 8: the owner

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

The owner's two tests were red against join tables with no row
(`ev-step8-owner-red.txt`). The first got a refusal where it expected the join to fire,
and the second counted no successful join where it expected one:

```text
FAIL owner_test.a refusal is answered, changes nothing and leaves the owner serving: crashed: ... #(Value, Error(Refused(PublisherAlreadyJoined("q")))), ... #(Value, Ok(Outcome(["PublisherJoins"], None))), ...
FAIL owner_test.the same join from several processes succeeds exactly once: crashed: ... #(Value, 0), ... #(Value, 1), ...
13 tests run, 11 passed
...
error: recipe `gleam-double` failed on line 469 with exit code 1
exit status: 1
```

Green after the `PublisherJoins` row (`ev-step8-owner-green.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
...
13 tests run, 13 passed
exit status: 0
```

The second test sends the same join from eight processes. Checkpoint 1 ran here
(section 5).

#### Step 9: the scenarios

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

All five scenario tests were red first (`ev-step9-red.txt`). S003, S004 and S005 failed
because `config.new` admitted everything. S009 and S010 failed in the fixture, which
joins consumers, because `ConsumerJoins` had no row; the untagged shared-id test failed
there too. S009 could not be red for `PublisherJoins` itself, whose row landed in step 8.

```text
FAIL config_test.a zero expiry limit is refused: crashed: ... #(Function, "zero_expiry_limit"), #(Line, 36), #(Message, "Assertion failed."), ...
FAIL config_test.a zero pull batch is refused: crashed: ... #(Function, "zero_pull_batch"), #(Line, 44), #(Message, "Assertion failed."), ...
FAIL config_test.a negative TTL is refused: crashed: ... #(Function, "negative_ttls"), #(Line, 58), #(Message, "Assertion failed."), ...
FAIL identity_test.a publisher joins once: crashed: ... #(Function, "fixture"), #(Line, 165), #(Message, "Pattern match failed, no pattern matched the value."), ... #(Value, Error(Refused(ConsumerAlreadyJoined("c1")))), ...
FAIL identity_test.a consumer joins once: crashed: ... #(Function, "fixture"), #(Line, 165), ...
FAIL identity_test.a publisher and a consumer may share an id: crashed: ... #(Function, "fixture"), #(Line, 165), ...
25 tests run, 19 passed
...
error: recipe `gleam-double` failed on line 474 with exit code 1
exit status: 1
```

Green, with the config validation and the `ConsumerJoins` row (`ev-step9-green.txt`):

```text
$ just gleam-double
gleam 1.19.1
OTP 29
...
25 tests run, 25 passed
exit status: 0
```

The tags are S003, S004 and S005 in `config_test`, and S009 and S010 in `identity_test`.
The three config refusals leave no owner to inspect, so they assert the refusal itself,
as step 9 says. Checkpoint 2 changed what four of the ports assert, S003, S004, S005 and
S010, and added the twenty-sixth test (section 5).

#### Steps 10 and 11: the spec notes, the handbook and the agent contract

Uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`.

`gleam-double/SPEC-NOTES.md` holds six notes. G06 was added at checkpoint 2 by the
triage, and the lead accepted it.

| Id | Note | `N`-note |
| --- | --- | --- |
| G01 | Timestamps and durations are whole milliseconds | Agrees with N01 |
| G02 | A config that breaks a config invariant is refused when it is made (S005 is `impl-choice`) | Agrees with N09 |
| G03 | A join with an id already taken is refused | Agrees with N12; the competing callers have no `N`-note |
| G04 | The queue's first position is 0 | Agrees with N04 |
| G05 | A rule asks for `now` and for a lease id; a guard and a refusal cannot | Agrees with N28, N15 and N05 |
| G06 | The snapshot lists publishers and consumers in the order they joined (S009's order is `impl-choice`) | No `N`-note |

`docs/reference/commands.md` has three rows, for `just gleam-double`,
`just gleam-double-deps` and `just gleam-double-format`. `docs/project/repository-map.md`
has the `gleam-double/` row, and `AGENTS.md` names `just gleam-double-deps` among the
recipes that reach the network.

```text
$ just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
Validated 49 pages and 50 canonical topics.
exit status: 0
```

```text
$ just check-agents
Validated AGENTS.md, 2 adapters, and 14 skills.
exit status: 0
```

`just lint` exited 0 (`ev-step11-lint.txt`).

#### No `panic`, `todo`, `assert` or `let assert` under `src/`

At `0db1adb86e7d02c0a39f97c2fcebc931c9dc9529` (`ev-no-panic.txt`, which quotes its script,
`no-panic.sh`). The pattern's `assert` also matches `let assert`. The control prints 4,
one for each of its four lines, and the search over `gleam-double/src` prints no line and
exits 1:

```text
0db1adb86e7d02c0a39f97c2fcebc931c9dc9529
set -u
pattern='\b(panic|todo|assert)\b'
printf 'panic\ntodo\nassert x\nlet assert y\n' | rg -c "$pattern"   # positive control: 4 of 4 lines
rg -n "$pattern" gleam-double/src
echo "rg exit status over gleam-double/src: $? (1 means no match)"
--- output
4
rg exit status over gleam-double/src: 1 (1 means no match)
```

### 5. Checkpoint records

Both checkpoints used Codex v0.169.1 with `gpt-6-astra`, `-s read-only`, through the
ticket's corrected command (`< /dev/null`, `-o last-<lens>.md`, a `>|` log). Codex was on
`PATH`, so no fallback to a Claude reviewer was needed. Every findings file opens
`Source: last-message`: it is Codex's last message, recovered because a read-only Codex
writes no file. One triage subagent worked each round, and it was neither a reviewer nor
the author. Not recorded: the `codex exec --help` check on the day, and the model of the
reviewer and triage subagents.

In the tables, "Filed" is the reviewer's severity and "Triaged" the triager's.

#### Checkpoint 1, round 1: conformance, breakage and design

Ran uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, after step 8 and
before any scenario was ported. `just gleam-double` ended `13 tests run, 13 passed`
before any fix.

The conformance lens filed C1 to C12. The breakage lens filed B1 to B5, and B6 is the
item it named as unverified. The design lens reported no supported finding; D1 and D2 are
the two observations in its message that the triage took up.

| Id | Filed | Triaged | Verdict | Reason | Applied or handed back |
| --- | --- | --- | --- | --- | --- |
| C1 | medium | low | valid | Shared rule 6's two bullets disagree only on the failing path, and the guard is the one the rule prescribes: the check can leave `manifest.toml` rewritten before it fails | Nothing applied. Handed back (Open points) |
| C2 | medium | low | valid | The `erl` line of `gleam-double` did not set `ERL_CRASH_DUMP_BYTES=0`, and each recipe line is its own `sh` | The variable is on the `erl` line |
| C3 | low | low | invalid | `rules.drive` performs no effect of its own; it calls the two functions it is handed, and `owner` is its only caller under `src/` | Nothing. The delta stands |
| C4 | low | low | valid in part | There is one allotter and a row cannot move the counter; a comment in `core` claimed the increment, which is in `rules` | The comment on `core.world` reworded |
| C5 | low | low | valid | Decision 0021 said the double adds no pre-check, and the recipe has one | "The guard" rewritten; the pre-check dropped from "What would reopen this" |
| C6 | low | low | valid in part | The record opened "The pins move by hand." as if settled; what Renovate reads cannot be answered without the network | The paragraph now says the question is open. The question is handed back (Open points) |
| C7 | low | low | valid | `just audit` runs one `cargo deny` over the root workspace; it does not read three lockfiles | Decision 0021 says "the workspace's `Cargo.lock`" |
| C8 | low | low | invalid | The seam is still `Step`, and every clock and lease-id constraint of shared rule 4 holds and has a test | Nothing. The delta stands |
| C9 | low | low | invalid | Neither `domain` nor `time` holds behaviour, and no shared rule fixes the module list | Nothing. The delta stands |
| C10 | low | low | valid in part | Each trigger is still one message through one mailbox; a comment read as one variant per trigger | The comment on `Message` reworded |
| C11 | low | low | invalid | Only `Passed` carries fired names; the apply step's controls test `rules.run`, so they sit in `rules_test` | Nothing. A note for GD02, below |
| C12 | low | low | valid in part | The third recipe stands; decision 0021 did not name it | Decision 0021 names it. The third `commands.md` row was handed to step 11, where it landed |
| B1 | high | low | valid in part | A clock or lease-id function that crashes takes the owner down. That is outside the four failures shared rule 4 makes values, cannot be converted without FFI under `src/`, and fails the test that injected it | The contract is stated on `owner.start` (Open points) |
| B2 | medium | medium | valid | The registry checked names against files, so a name could be paired with another module's `tests()` | A "shared tests" fault in `registry_faults`. Round 2 removed it |
| B3 | medium | medium | valid | Nothing compared the versions `manifest.toml` pins with those `build/packages` holds, and `gleam` fetches what is missing | A fourth precondition line, exit 2, before any `gleam` command |
| B4 | medium | low | valid in part | `process.call` panics in the caller, a test's process; the ticket records that as known and asks only for the timeout | A comment on `call_timeout` says what happens to a caller with no reply |
| B5 | low | low | valid | The same line and the same fix as C2 | As C2 |
| B6 | none | none | invalid | The reason given was that a test's process cannot fail after reporting a pass. Round 2's C2-1 replaced that reason: `Passed` means `run` returned | Nothing |
| D1 | observation | observation | invalid | The same question as C3 | Nothing. The delta stands |
| D2 | none | medium | valid | A `Plan` could not return a failure from inside a matched effect, and from GD04 an effect looks up what its guard promised | `Impossible(reason)`, `Defect(trigger, cause)` and three scratch tests |

Applied, one writer, in this order: C2 and B5; B3; D2; B2; the four comments (B1, B4, C4,
C10) as one edit; the four changes to decision 0021 (C5, C6, C7, C12) as one edit.
`just gleam-double` ended green after each of the first five, and `just check-docs` and
`just lint` passed after the last.

B3's evidence, from the verdicts file. `manifest.toml`'s `filepath` pin was edited from
1.1.2 to 1.1.1 for the probe and restored. Under the denial, with both pre-checks of the
time passing, the recipe as it was reached for the network:

```text
Downloading packages
error: HTTP error
    error sending request for url (https://repo.hex.pm/tarballs/filepath-1.1.1.tar)
error: recipe `gleam-double` failed on line 468 with exit code 1
```

With the fix, the same probe under the same denial ran no `gleam` command:

```text
gleam-double: gleam-double/build/packages does not hold the versions manifest.toml pins, and gleam would fetch them; run just gleam-double-deps (needs network)
error: recipe `gleam-double` failed on line 469 with exit code 2
```

Handed on from round 1:

- **Checkpoint question 2, "can anything under `src/` panic".** No code under `src/`
  holds the four words. Two things can still crash a process, both by design and both
  failing the test: an injected function that crashes, in the owner (B1), and a call that
  gets no reply, in the caller (B4). A call to the owner waits 1000 ms; a caller with no
  reply in that time, or whose owner has died, crashes in `process.call`, and the trigger
  it sent may still be applied. The B1 chain was read from the pinned sources and the
  runner. No crashing fake was run through an owner, because no row that reads the clock
  goes through the owner yet.
- **For GD02.** The coverage check must credit a tag only from a `Ran` whose outcome is
  `Passed` (C11).

A second round ran because B1 was filed high and was valid in part. The re-run that
closed round 1, uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, exit 0:

```text
$ just gleam-double
   Compiled in 0.23s
   Compiled in 0.20s
    Running pachinko_double_test.main
17 tests run, 17 passed
```

#### Checkpoint 1, round 2: conformance and breakage, over the fix diff

Ran uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, with a fresh
triager. The breakage lens filed B2-1 and B2-2. The conformance lens reported no
confirmed finding; its remark on round 1's B6 verdict was taken as C2-1.

| Id | Filed | Triaged | Verdict | Reason | Applied or handed back |
| --- | --- | --- | --- | --- | --- |
| B2-1 | medium | medium | valid | `==` on `Test` values does not say which module a list came from. With a probe module that `main` listed beside an inline stand-in and did not import, the gate was green, `20 tests run, 20 passed`, and the module's own test never ran | The registry check reads the entry module's source and names every `*_test.gleam` on disk that it does not import. The reading is a pure function, `import_faults(source, on_disk)` |
| B2-2 | medium | low | valid | Neither shared rule 4 nor step 6 forbids two modules listing one `Test`, and round 1's fault fired on it | The "shared tests" fault and its test are removed. Nothing replaces it |
| C2-1 | low | low | valid in part | Round 1's B6 wording claimed more than the source proves: a helper the test started and linked can kill the test's process after its send. What holds is that `Passed` means `t.run()` returned | A paragraph on `run_one`: `Passed` means `run` returned, and a test waits for what it starts |

B2-1's new test was red first, against a stub that returned no fault:
`19 tests run, 18 passed`. The gate failing on the probe is quoted in step 6 above. The
other half was run too: with `import probe_test` added to `main` and the stand-in kept,
the build failed on `Unused imported module`. B2-2's test was red against round 1's
fault: `32 tests run, 31 passed`.

Handed on from round 2:

- **Two cases the import check does not catch**, and the linked-helper case: Open points.
- **A second new fault**, `cannot read test/pachinko_double_test.gleam: <reason>`, for an
  entry module that cannot be read. It was read from the code and not run.
- **Superseded.** Round 1 told authors that a test module must not reuse another module's
  `Test` value. That no longer holds: two modules may list one test.
- **For GD02 to GD07.** A test waits for every process it starts whose failure should
  fail it (section 9).

No finding in round 2 was high, and §14 allows two rounds at most. The re-run that closed
the checkpoint, uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, exit 0:

```text
$ just gleam-double
   Compiled in 0.17s
    Running pachinko_double_test.main
18 tests run, 18 passed
```

Eighteen is the 17, less round 1's test of the removed fault, plus the two new ones.

#### Checkpoint 2: conformance and breakage, before the final commit

Ran uncommitted, on base `5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae`, after step 11.
`just gleam-double` ended `25 tests run, 25 passed` before any fix. One round: no finding
was high. All six findings are valid, and every fix is inside Files touched. Nothing
under `gleam-double/src/` changed.

| Id | Filed | Triaged | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- | --- |
| C1 | low | low | valid | The three config ports asserted `Snapshot(0, [], [])`, where the Rust asserts only that a queue is built, so they pinned G04's first position in rows that do not name it | `serves` asserts that the settings are admitted and that an owner over them answers an inspection with no publisher and no consumer. G04 says what the ports assert, and that the two owner tests do pin the 0 |
| C2 | low | low | valid | `harness.fixture` used the default config and a clock at 0; the Rust fixture has three short TTLs and starts at 1,000,000 ms | `harness.t0`, `harness.fixture_settings()`, and a `fixture()` that uses them. G01 mentions the fixture's start. `harness.start` still starts the clock at 0 |
| C3 | low | low | valid | The Rust S010 asserts that `c4` is among the consumers, and the port asserted the ordered list of four; the snapshot's join order, which S009's literal rests on, had no spec note | S010 asserts membership. S009 keeps the Rust's ordered literal, labelled `impl-choice`. G06 is new |
| B1 | medium | medium | valid | `just lock` installs nothing, decision 0021's procedure never reinstalled the `gleam` environment, and the recipe only tested that the two binaries exist, so an environment installed before a pin moved passed as the old pair | A fifth precondition line, exit 2. Decision 0021 and the two `commands.md` rows say so |
| B2 | medium | low | valid | `shared_id` asserted two replies and no state, so a join that answered and added nothing passed | The test inspects before and after, and asserts membership and that each list grew by one |
| B3 | medium | medium | valid | Every limit the suite tried was 0, 1 or a default, so `!= 0` in place of `>= 1` passed while admitting -1 | A new untagged test, "a negative limit is refused" |

The evidence, from `ai_tmp/review/GD01-2/`. Its files hold no exit-status line, so each
status below is the one the verdicts file states.

| Finding | Probe | Result |
| --- | --- | --- |
| C1 | `model.new` starting at `next_position: 10`, after the fix (`ev-c1-after-fix-at-10.txt`) | `25 tests run, 23 passed`; only the two owner tests fail, and neither pins a catalog row |
| B1 | A scratch copy of `pyproject.toml` with `gleam = "==1.19.2"`, the recipe as it was (`ev-b1-old-recipe-pin-moved.txt`) | `gleam 1.19.1`, `OTP 29`, `26 tests run, 26 passed`, exit 0 |
| B1 | The same, with the new recipe (`ev-b1-new-recipe-pin-moved.txt`) | The refusal quoted below, exit 2 |
| B2 | A mutant join that answers and adds nothing, before the fix (`ev-b2-mutant-before-fix.txt`) | `25 tests run, 25 passed`, exit 0: the mutant survived |
| B2 | The same mutant, after the fix (`ev-b2-mutant-after-fix.txt`) | `FAIL identity_test.a publisher and a consumer may share an id`; `25 tests run, 24 passed`, exit 1 |
| B3 | `!= 0` for both `>= 1` in `config.gleam`, after the fix (`ev-b3-mutant-after-fix.txt`) | `FAIL config_test.a negative limit is refused`; `26 tests run, 25 passed`, exit 1 |

```text
gleam-double: the gleam environment does not hold the gleam and erlang that pyproject.toml pins; run just gleam-double-deps (needs network)
error: recipe `gleam-double` failed on line 474 with exit code 2
```

Read from the terminal, with no evidence file: C1's probe before the fix
(`25 tests run, 20 passed`); C3's two runs with the consumers reversed (`24 passed`
before the fix, `25 passed` after); B3's mutant before the fix (`25 tests run, 25
passed`) and its run with only the pull-batch line mutated. C2 has no red in the suite,
since the snapshot shows neither the config nor the time; a one-off assertion on the
fixture's clock stood in, and is not recorded either.

Handed on from checkpoint 2. Nothing was left unapplied.

- **C3's label is the triager's reading**, not a ruling. S009's ordered assertion is
  labelled `impl-choice` because shared rule 1 asks for the label on a row that asserts
  more than the specification states. S009 is not among the known cases the shared rules
  list. If the maintainer reads an inspection order as shape, the label comes out of the
  test comment and of G06's heading, and the note stands without it.
- **B1's limits.** The fifth line reads the pins from `pyproject.toml`, not from
  `pixi.lock`, so it rests on the house rule that a pin is written `==x.y.z`, which
  `just lock-check` proves the lock agrees with. `just gleam-double-format` has no such
  line; a stale `gleam` there could lay the sources out differently, and
  `just gleam-double` would then fail its format check under the pinned one.
- **For a later ticket.** `harness.t0` and `harness.fixture_settings()` are there for the
  port of the Rust's `Fixture::with_config`.

The re-run after the fixes, uncommitted, on base
`5e4083d544a4cfc6466ea38b71dd9d9d7cbf74ae` (`ev-final-gleam-double.txt`), exit 0 by the
verdicts file; `just lint` also exited 0 (`ev-lint.txt`):

```text
gleam 1.19.1
OTP 29
   Compiled in 0.18s
   Compiled in 0.18s
    Running pachinko_double_test.main
26 tests run, 26 passed
```

The final checkpoint's `just check` is the Verification in section 6.

### 6. Verification

The Verification block ran at `0db1adb86e7d02c0a39f97c2fcebc931c9dc9529`, the work
commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's block word for
word; output in `ev-verification.txt`). It exited 0, so its last two lines held:
`git status --short` printed nothing.

```text
$ sh ai_tmp/gd/GD01/verification.sh
0db1adb86e7d02c0a39f97c2fcebc931c9dc9529
gleam 1.19.1
OTP 29
...
    Running pachinko_double_test.main
26 tests run, 26 passed
...
✔ Lock-file was already up-to-date
...
Validated 49 pages and 50 canonical topics.
...
Validated AGENTS.md, 2 adapters, and 14 skills.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The 26 tests: the five tagged ones (S003, S004 and S005 in `config_test`; S009 and S010
in `identity_test`), and 21 untagged: the runner's own, the engine's on scratch tables,
the owner's two, the defaults, a negative limit and a shared id.

The offline run, repeated at `0db1adb86e7d02c0a39f97c2fcebc931c9dc9529` with the recipe's
five precondition lines in place (`ev-final-offline.txt`, which names its commit on its
last line):

```text
$ sandbox-exec -p (version 1)(allow default)(deny network*) just gleam-double
gleam 1.19.1
OTP 29
   Compiled in 0.19s
   Compiled in 0.18s
    Running pachinko_double_test.main
26 tests run, 26 passed
exit status: 0
0db1adb86e7d02c0a39f97c2fcebc931c9dc9529
```

Its positive control is the one `curl` of step 5, which ran before the commit and was not
repeated.

### 7. Stops

One of the plan's named stops had its literal condition met twice: "`just gleam-double`
reaches for the network with `build/` populated".

- With a `gleam.toml` range edited away from `manifest.toml`, `gleam` went to
  `repo.hex.pm`, which the probe denied (step 5).
- With `build/packages` holding an older version than `manifest.toml` pins, `gleam` tried
  to fetch the tarball, which the triage's probe denied (checkpoint 1, B3).

In both the files disagreed, which is not the case the offline claim is about, and each
is now refused by a pre-check before any `gleam` command. The run was not paused for
either. The maintainer should judge whether that was right.

The other named stops did not fire: a count that moved or a taken 0021; a `Cargo.lock`
moving; a package beyond the four and `filepath`; neither `gleam` version running; a
ported scenario failing; `GuardOverlap` on a specification table.

### 8. What was not run

- Any push, pull request or issue.
- `just initialize`, `just sync` and `just install-allium`.
- A CI run. In particular, pixi v0.83.0, which CI pins, was not run against the
  `pixi.lock` that pixi 0.84.1 wrote.
- A `gleam` command over a drifted manifest with the network allowed. That `gleam` then
  rewrites `manifest.toml` and exits 0 needs a hex.pm request this run was not authorised
  to make.
- A second `curl`: none without the denial, and not §11's pair of negative controls. The
  ticket's step 5 names one `curl` to `https://hex.pm`, and that one ran.
- The fallback to `gleam` 1.19.0, which was not needed.
- The restoring form of the manifest guard (checkpoint 1, C1), a crashing fake through an
  owner (B1), and the fault for an unreadable entry module (round 2).

Not recorded, though the lead's notes state each result: step 2's three counts; step 3's
`just lock` exit status, its `git status --short`, the default-environment diff and its
`just lock-check` and `just check-docs` runs; `command -v rebar3`; the SHA-256 taken
after each restore in steps 5, 6 and 7; `codex exec --help` on the day; the reviewer and
triage subagents' model; and the checkpoint 2 probes that section 5 lists as read from
the terminal.

### 9. Working lessons for GD02 to GD07

- After restoring a `.gleam` file from a copy, `touch` it. `gleam` trusts modification
  times and will reuse a stale result.
- Before a triage edits anything, snapshot the tree
  (`GIT_INDEX_FILE=<scratch> git add -A . && git write-tree`), so that a second round has
  a real diff.
- A pipe into `tail` reports `tail`'s status. Evidence is taken with `ai_tmp/gd/run.sh`,
  which records the command's own.
- A literal assertion such as `assert 1 == 2` is a compile-time warning, so a probe must
  assert on a computed value.
- A test waits for every process it starts whose failure should fail it.
- Checkpoint 2's six findings were of three kinds: a port asserting more or less than its
  Rust test, a fixture unlike the Rust fixture, and a test that survives a mutant.

## Open points

- **`pixi.lock` was written by pixi 0.84.1, and CI pins v0.83.0.** The local pixi wrote
  the relocked file, and also rewrote its two `virtual-packages` lines from `__unix=0=0`
  to `__unix=0`. `.github/actions/setup/action.yml` pins pixi v0.83.0 with
  `locked: true`. `just lock-check` passes locally. Whether 0.83.0 accepts the file was
  not run, and only a push can settle it.
- **Shared rule 6 says two things about a rewritten `manifest.toml` (checkpoint 1, C1).**
  Its first bullet says `just gleam-double` writes only under `gleam-double/build/`. Its
  second says each recipe compares `manifest.toml` before and after and fails if it
  changed. On the failing path the file is left rewritten, for `git diff` to show. The
  recipe does what the second bullet says, and its pre-checks stop the two drifts that
  occur in practice before `gleam` runs. The maintainer either accepts "fails after the
  rewrite", or asks for the guarded lines to restore the file: copy `manifest.toml` to
  `build/manifest.toml.before` first; on a mismatch copy the rewrite to
  `build/manifest.toml.rewritten`, copy the saved file back and exit 1; with a `trap` on
  `EXIT`, to cover a `gleam` that fails after rewriting. That form is untested, because
  no drift that passes the pre-checks could be made offline. The shared rules are carried
  word for word in seven tickets and were not edited.
- **Shared rule 4 says `core` is the only place that allots from `next_position`.** As
  landed, the interpreter in `rules.gleam` allots, through the two accessor functions
  `core` hands it, and overwrites whatever counter a row finishes with. There is still
  one allotter, and a scratch test shows that a row cannot move the counter. The rule's
  wording was not edited, for the same reason. The maintainer either accepts the reading
  or rewords the rule in the seven tickets.
- **Whether Renovate reads the Gleam pins (checkpoint 1, C6).** Step 3 asks whether
  Renovate reads the two pixi pins in the `gleam` feature and `gleam-double/gleam.toml`.
  Answering needs Renovate's documentation or its dashboard, over the network, which the
  triage could not use.
  Decision 0021 says the question is open and that the pins move by hand until it is
  settled. `renovate.json` is untouched. If Renovate reads `gleam.toml`, it needs a rule
  for the hex pins.
- **An injected clock or lease-id function that crashes kills the owner (checkpoint 1,
  B1).** The owner calls both in its own process. Turning the crash into a value would
  need FFI under `src/` or a fifth failure kind, and the fixed design has neither. It is
  stated as a contract on `owner.start`: both functions must return. A fake that crashes
  fails the test that injected it, because the owner is linked to that test's process.
  The double ships no real clock, so only test fakes are ever injected.
- **That `gleam` rewrites a drifted manifest and exits 0 is not shown here.** Under the
  network denial `gleam` failed instead of rewriting, and showing the rewrite needs a
  hex.pm request this run was not authorised to make. It stays knarr's finding. The
  `cksum` backstop was proved by editing `manifest.toml` while `gleam build` ran.
- **What the runner's registry check still misses (checkpoint 1, round 2).** Two cases
  pass it: a `main` that imports a test module and uses it some other way than by listing
  its `tests()`, such as calling `x_test.tests()` and dropping the list; and a string in
  `main` holding a line that starts `import x_test`. Separately, a test that starts a
  linked helper and does not wait for it passes even when the helper then crashes: of 300
  such runs in a probe, 300 passed. The runner was not changed for that, because a check
  on the test's exit would pass or fail by timing. The rule for authors is that a test
  waits for what it starts.
- **Two network reaches were not treated as stops.** Section 7 of the hand-back has both.
  Each met the literal words of a named stop while the files disagreed, and each is now
  refused before `gleam` runs. The maintainer judges whether the run should have paused.
- **The pins: closed by step 4.** `gleam` 1.19.1 runs on conda's `erlang` 29.1.2, so no
  fallback was needed.
- **The offline claim: shown by step 5 for files that agree, on one control.** With
  `gleam.toml`, `manifest.toml`, `build/packages` and the `gleam` environment in
  agreement, `just gleam-double` passes with the network denied, at the work commit too.
  Where they disagree the recipe now refuses before `gleam` runs. The denial's evidence
  is one negative control, the hostname `curl` to `https://hex.pm`, which fails on name
  resolution. `tickets/CONVENTIONS.md` §11 asks for a pair, the hostname control and
  `curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1`, both inside the denial and
  both quoted. The direct-IP control was not run, because the plan authorised one `curl`,
  so the claim stays open until it is (series review, 2026-11-21, finding C4).
- **`AGENTS.md` gains one recipe name.** It lists the recipes that reach the network, and
  `just gleam-double-deps` is a new one. The maintainer approved the addition on
  2026-11-20.
- **A separate pixi feature.** Settled by the maintainer on 2026-11-20, before step 1:
  `gleam` and `erlang` sit in a `gleam` feature with its own environment, so the default
  environment, and every CI job that installs it, is unchanged. Step 3, the recipe
  sketches and Files touched were amended to say so. Decision 0021 records the choice and
  what it costs.
