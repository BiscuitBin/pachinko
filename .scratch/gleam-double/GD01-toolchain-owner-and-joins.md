---
id: GD01
title: "Gleam double: the toolchain, the owner actor and the joins"
status: todo
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
- the `gleam` and `erlang` pins in `pyproject.toml`, and the relocked `pixi.lock`;
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
gleam-double-deps:
    cd gleam-double && if test -f manifest.toml; then before=$(cksum < manifest.toml); else before=new; fi; gleam deps download; [ "$before" = new ] || [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }

gleam-double:
    command -v gleam >/dev/null && command -v erl >/dev/null || { echo 'gleam-double: gleam or erl is missing; run just initialize' >&2; exit 2; }
    test -f gleam-double/build/packages/packages.toml || { echo 'gleam-double: run just gleam-double-deps (needs network)' >&2; exit 2; }
    gleam --version
    erl -noshell -eval 'io:format("OTP ~s~n", [erlang:system_info(otp_release)]), halt().'
    cd gleam-double && gleam format --check src test
    cd gleam-double && before=$(cksum < manifest.toml); ERL_CRASH_DUMP_BYTES=0 gleam build --warnings-as-errors; [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }
    cd gleam-double && before=$(cksum < manifest.toml); ERL_CRASH_DUMP_BYTES=0 gleam test; [ "$(cksum < manifest.toml)" = "$before" ] || { echo 'gleam rewrote gleam-double/manifest.toml; read the diff' >&2; exit 1; }
```

Both recipes rely on the `Justfile`'s `set shell := ["sh", "-eu", "-c"]`: under `-e` a
failing `gleam` command ends its line before the comparison runs, so the comparison's
status cannot hide it. Step 6's probes prove that.

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
| `pyproject.toml` | shared, own lines | The `gleam` and `erlang` pins, with a comment naming decision 0021 |
| `pixi.lock` | shared | Relocked by `just lock`, never by hand |
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
   - whether they join the default environment, the only one on the `Justfile`'s `PATH`,
     or a feature with its own environment, as knarr keeps its cluster tools out of its
     default one, and what either costs CI;
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
   the two pins to `pyproject.toml`. Run `just lock` (network, authorised by this step).
   It also runs `cargo update --workspace` three times, so confirm with
   `git status --short` that of the lockfiles only `pixi.lock` moved; if a `Cargo.lock`
   moved, **stop**. Run `just initialize` (network, authorised by this step) to install
   the relocked environment. Quote `gleam --version` and the OTP release `erl` reports.
   Run `just lock-check` and `just check-docs`.
4. **The package and the first green run, red first.** Write `gleam.toml` by hand rather
   than with `gleam new`, whose skeleton is not this repository's. Add the `.gitignore`
   lines and both recipes. Run `just gleam-double` before any package is fetched and
   quote its refusal. Run `just gleam-double-deps` (network, authorised by this step),
   which writes `manifest.toml`. Read it: it should pin the four packages and what they
   bring, which is expected to be `filepath` alone. Put any other package to the
   maintainer before going on. Write the smallest runner `main` with one test, and run
   `just gleam-double` until it is green.
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
- `pyproject.toml` pins `gleam` and `erlang` exactly, `just lock-check` passes, and of
  the lockfiles only `pixi.lock` moved.
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

Written at the end.

## Open points

- **The pins are unproved until step 4.** Nothing has run `gleam` 1.19.1 on conda's
  `erlang` 29.1.2. Step 4 carries the fallbacks.
- **The offline claim is unproved until step 5.** Gleam has no offline flag. knarr shows
  an offline gate on 1.19.0; step 5 shows it here.
- **`AGENTS.md` gains one recipe name.** It lists the recipes that reach the network, and
  `just gleam-double-deps` is a new one. The maintainer may prefer to leave `AGENTS.md`
  alone, as the Python double did; that leaves its list incomplete.
- **The default environment grows.** `gleam` and `erlang` in the default environment are
  installed by every CI job that installs it. Decision 0021 weighs a separate feature.
