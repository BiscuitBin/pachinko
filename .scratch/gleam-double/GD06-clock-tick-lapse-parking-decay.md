---
id: GD06
title: "Gleam double: the clock, with Tick, lapse, parking and decay"
status: todo
depends_on: [GD05]
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD06: Gleam double: the clock, with Tick, lapse, parking and decay

## Context

GD01 to GD05 landed every consumer and publisher action of the Gleam double under
`gleam-double/`, and the lease's end by release in every row of the lease-end table, all
behind the one owner. Read their hand-backs and `gleam-double/SPEC-NOTES.md` first.
`.scratch/gleam-double/README.md` is the series' index.

So far a lease that is never released lasts forever, no pocket parks, and nothing
decays. This ticket adds the clock's side:

- `Tick`, an ordinary message in the owner's mailbox;
- the lapse and the three expiry rules;
- parking, and the revival of a parked pocket;
- the cases where an action comes after `expires_at` but before the lapse is processed;
- the decay of both shed queues;
- the lapse half of every "by release or lapse" scenario.

The specification facts it rests on, all from `docs/specs/pachinko.allium` (the Lease
expiry, Lease end and Decay sections, and the `ExpiryRecoversWithinTtl` guarantee):

- **Temporal triggers.** `LeaseLapses` fires for an active lease whose
  `expires_at <= now`. `SupersededRecordDecays` and `DeadLetterDecays` fire for an entry
  whose `expires_at <= now`. The double has no background time and no timer. It processes
  these only when the owner takes a `Tick`, which reads `now` once from the injected
  clock and applies everything due. `Tick` runs the apply step once for each due lapse
  and once for each due decay, so the checker runs after each outcome.
- **One serialized stream.** The Lease expiry comment says the implementation "applies
  the lapse and every action from one serialized stream (decision 0019), so the guards
  above decide in either order". In this double that stream is the owner's mailbox:
  `Tick` waits its turn among the actions, and there is no timer per lease that could
  act on its own.
- **Branch A** (decision 0019). A pull, ack, nack, renew or release processed after
  `expires_at` but before the lapse succeeds, as long as its other guards hold. A pull,
  ack, nack or renew moves `expires_at` on. A release ends the lease. The lapse then
  stops at its guards, unless the new `expires_at` has also passed, as it always has
  when `lease_ttl` is 0 (S006, S066, S067). So an action never lapses a lease itself, and
  only `Tick` does.
- **The three expiry rules.** Each needs the lease active, `expires_at <= now` and the
  pocket leased. Each ends the lease `expired` at `now`, sends every held record `Freed`
  and raises `consecutive_expiries` by one. They are the rows of the `LeaseLapsed` table,
  and each one's effect ends the lease through GD04's lease-end module, which already
  answers every `Freed`.
  - `LeaseExpiryParksPocket`: at `count + 1 >= max_consecutive_expiries`, the pocket
    parks, whether or not records remain.
  - `LeaseExpiryKeepsPocketAtFront`: below the threshold, with a record that outlasts the
    lease, the pocket is queued at its old position, the front.
  - `LeaseExpiryIdlesEmptyPocket`: below the threshold, with none, the pocket goes idle.
- **Revival.** `ReviveParkedPocket` answers `PocketTouched` on a parked pocket, as
  `ReviveIdlePocket` does on an idle one. A stale publish and a refused equal-rank publish
  touch nothing, so a parked pocket stays parked with its count (S025, S092). Whether
  anything other than a publish can revive a parked pocket is open question 7. S026 pins
  it as the rules stand, labelled `oq:7`, and leaves it open.
- **Decay.** A superseded record decays at `shed_at + superseded_ttl`, and a dead letter
  at its lease's end plus `dead_letter_ttl`.

What the maintainer has already ruled, and what the Python double already read:

- **The three ports stand as the Rust's inspection shape** (the ruling on N38). S066's
  release half asserts the specification's facts about the released lease, as S053 does.
  S093 compares a pocket's records by key, as S091 does. S075 asserts the position
  invariants and an empty `check`, since the double stores no pocket order.
- The tick is the double's only clock, and it reports what it did (N33).
- One tick lapses in `expires_at` order, then decays in queue order (N34).
- An entry a lapse creates with a zero TTL decays in the same tick (N35).
- Only a publish revives a parked pocket, as the rules stand, `oq:7` (N36).
- The expiry rules' answer when no guard holds is unreachable through a tick (N37).

One question is new with the actor, and this ticket answers it in the double and records
it. A tick is one message but several outcomes. If an outcome part-way through a tick
returns an `Error`, the owner either keeps the outcomes that succeeded before it or
returns to the state it had before the `Tick`. Finding 9 of `double/SPEC-NOTES.md` asks
whether a tick should be atomic as a whole; the Python double commits each outcome as it
goes.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/expiry.rs`,
`decay.rs`, `lease.rs` and `identity.rs`, found with `rg -n 'scenario: S0NN'`. A Rust test
that loops over `ENDINGS` runs once by release and once by lapse. Port both halves, each
as its own registered test carrying the scenario's tag, so that the runner's count shows
both ran. S066 and S067 each have two halves in two Rust tests; tag both Gleam tests. The
`wire` rows here are S064 and S077; port only their library half.

## Goal

A `Tick` processes everything the clock has made due and reports it. A lease lapses at
`expires_at`, and once. A pocket parks on its `max_consecutive_expiries`-th consecutive
lapse, and a publish revives it. Superseded records and dead letters decay at their
`expires_at`. Branch A holds in both orders, with the order decided by the mailbox and by
nothing else. Every scenario below is pinned by a tagged test, and on exit nothing is
pending.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (7) | `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket`, `SupersededRecordDecays`, `DeadLetterDecays` |
| Scenarios (34) | S002, S006, S007, S024, S025, S026, S034, S048, S056, S058, S059, S060, S061, S062, S063, S064, S065, S066, S067, S069, S073, S074, S075, S076, S077, S078, S079, S080, S081, S087, S088, S089, S092, S093 |

## Non-goals

- No background process, timer, sleep or wall clock. Time moves only when a test sets the
  fake clock, and due work happens only when the owner takes a `Tick`.
- No resolution of open question 7, or of any other.
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

  `<id>` is `GD06`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
| `gleam-double/src/` | shared, own lines | The owner's `Tick` message and the core's tick, `expiry.gleam` with the lapse and the three expiry rules, the parked revival, and `decay.gleam` with the two decays |
| `gleam-double/test/` | shared, own lines | The expiry, decay, both-endings and branch A tests; the pending lists in `coverage.gleam` |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/gleam-double/GD06-clock-tick-lapse-parking-decay.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD06's Status cell |

## Steps

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD05, in the hand-back.
   If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the sections named in Context, decision
   0019's lapse ordering (`docs/decisions/0019-service-runtime-tokio-and-http.md`), N33
   to N38 and finding 9 in `double/SPEC-NOTES.md`, and the catalog rows this ticket owns.
3. **`Tick` and the lapse, red first.** Write S059, S065 and S006 first. A lease lapses
   exactly at `expires_at`, once per lease, and at the instant of grant when `lease_ttl`
   is 0. Then add `LeaseLapses` and the three expiry rules as rows of their tables, each
   fired in at least one test. Each expiry row's effect calls the lease-end module; this
   ticket adds no `Freed` row and no second copy of the end-of-lease steps. The tick
   runs the apply step once per due lapse and once per due decay, so the checker runs
   after each lapse's whole outcome and after each decay. A tick reads the fake clock
   once, however much is due, and a tick with nothing due still reads it once.
4. **Branch A, both orders.** Port S066 and S067. From one test process, whose messages
   the owner takes in the order they were sent, show both orders:
   - an action sent after `expires_at` and before the `Tick` succeeds and extends the
     lease, and the `Tick` behind it lapses nothing;
   - a `Tick` sent first lapses the lease, and the same action behind it is refused.

   Then one untagged test with competing callers: one process sends the action and
   another sends the `Tick`, at once. In either order the snapshot passes `check`, and
   the test asserts that the action's answer, the tick's report and the final state
   agree. Quote a scratch break: make an action lapse its own lease, show S066 failing,
   and revert it.
5. **A failure part-way through a tick.** Decide which state the owner keeps when an
   outcome part-way through a tick returns an `Error`: the outcomes that succeeded, or
   the state before the `Tick`. Pin the choice with a test on a scratch table, and record
   it as a spec note against N33 and finding 9.
6. **Port the rest, red first.** Port parking, revival, decay and the "by release or
   lapse" scenarios, with both halves where the Rust loops over `ENDINGS`. Each new rule
   is a row of its trigger's table, fired in at least one test. Port S066's release
   half, S093 and S075 as the ruling on N38 says. Shrink `pending_scenarios` and
   `pending_rules` to empty. Leave the lists in place; GD07 deletes them.
7. **Spec notes.** Record, each against its `N`-note: the order in which one tick applies
   several due lapses and decays (N34); whether an entry a lapse creates in a tick decays
   in the same tick when its TTL is 0 (N35, S007); what a tick reports (N33); S026's
   `oq:7` (N36); and the expiry rules' unreachable answer (N37). Add one note on the
   Lease expiry comment's "the owner may apply a lapse ahead of an already-queued
   action": in this double the owner takes messages in mailbox order and a lapse happens
   only inside a `Tick`, so say what that sentence permits that the double does not do.
8. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks these:
   - Could a test pass while an action lapsed its own lease?
   - Could a tick skip a lease that was due, or lapse one twice?
   - Could a "by release or lapse" test run only one half?
   - Could a parked pocket be revived by a stale publish?
   - Could anything but a `Tick` message make time pass?
9. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (§11, "Separately authorised actions").

## Acceptance criteria

- `just gleam-double` exits 0, and its output shows the number of tests run and passed.
- On exit, `pending_scenarios` and `pending_rules` are both empty.
- Every owned scenario has a tagged test. Each scenario whose Rust test loops over
  `ENDINGS` has one registered test by release and one by lapse. Each of the seven rules
  has its row in its trigger's table, fired in at least one test.
- This ticket adds no `Freed` row and no second copy of the end-of-lease steps: each
  expiry row ends the lease through the lease-end module.
- The step 4 break is quoted with its output and exit status, and the competing-callers
  test passes.
- Step 5's choice is pinned by a test and recorded as a spec note.
- No code under `gleam-double/src/` reads the time except through the injected clock,
  arms a timer or sleeps, and only a `Tick` applies a temporal trigger.
- `gleam-double/SPEC-NOTES.md` holds the notes step 7 lists, and S026 is labelled `oq:7`
  in its test and its note.
- `just gleam-double` leaves nothing Git can see, and no `erl_crash.dump`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The first
`rg` lines show that `owner` is the only module under `src/` that imports the process
libraries. The second show that nothing under `src/` arms a timer, sleeps, imports a
time library or reaches Erlang through an `@external`, which is the only other way to a
wall clock. Each probe is its pattern's positive control: every alternative matches one
of its lines.

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
clock='send_after|process\.sleep|system_time|monotonic_time|import gleam/time|@external'
probe=$(printf 'process.send_after(s, 5, Tick)\nprocess.sleep(5)\nos.system_time()\nerlang.monotonic_time()\nimport gleam/time/timestamp\n@external(erlang, "os", "timestamp")\n' | rg -c "$clock")
test "$probe" -eq 6
if rg -n "$clock" gleam-double/src; then exit 1; else test "$?" -eq 1; fi
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Written at the end.

## Open points

- PD07 asks whether the order of the queued pockets is stored or only derived. This
  ticket follows the ruling on N38 as it stands; PD07's answer may change the ports of
  S066, S075 and S093.
- Whether a tick should be atomic as a whole is finding 9 of `double/SPEC-NOTES.md`.
  Step 5 answers it for this double only.
