---
id: GD06
title: "Gleam double: the clock, with Tick, lapse, parking and decay"
status: done
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
clock='send_after|\bsleep|system_time|monotonic_time|import gleam/time|@external'
probe=$(printf 'process.send_after(s, 5, Tick)\nprocess.sleep(5)\nsleep(5)\nsleep_forever()\nos.system_time()\nerlang.monotonic_time()\nimport gleam/time/timestamp\n@external(erlang, "os", "timestamp")\n' | rg -c "$clock")
test "$probe" -eq 8
if rg -n "$clock" gleam-double/src; then exit 1; else test "$?" -eq 1; fi
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Evidence files are under `ai_tmp/gd/GD06/`, and the review record under
`ai_tmp/review/GD06-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name. A failing run prints the runner's
panic and the recipe's `error:` line between `coverage faults: N` and `exit status: 1`;
those are left out too, with no mark. A quote that does not open with its `$` line starts
part-way through the output. Where a fact the record should hold was never saved, the
text says "not recorded". Three read-only checks were added for this record:
`ev-handback-checks.txt` (parts 1 to 12), `ev-handback-sha.txt` and
`ev-handback-endings.txt`, each run at the work commit on a clean tree.

### 1. Base, branch and commits

- Branch: `build-gleam-double`.
- Base commit: `bdd8fc18784dcf547b773c13bad289484192582d`, GD05's record commit. It holds
  GD05's work, `d279f7a3293e80e397eaf3c1e64769af3c3df03f`.
- Work commit: `98107a03b5374ee243f1caeade773ce16087e755`, tree
  `31c47f544a56e189bd4435a5e46f4d66514267e7` (`ev-handback-checks.txt`, part 1). The
  commit after it holds this record.
- At the maintainer's direction the whole GD01 to GD07 series lands on this one branch in
  this one worktree, two commits per ticket. That departs from `AGENTS.md`'s one ticket
  per worktree (shared rule 6).
- Nothing was fetched. Step 1's `just initialize` and `just gleam-double-deps` were not
  run; the baseline ran green without them.

### 2. Baseline

At `bdd8fc18784dcf547b773c13bad289484192582d`, on the untouched worktree
(`baseline.log`, quoted whole; it records each status as `<name>-exit=N` and holds no `$`
line, and its last line counts the lines `git status --short` printed):

```text
bdd8fc18784dcf547b773c13bad289484192582d
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
check-exit=0
    Running pachinko_double_test.main
232 tests run, 232 passed
coverage faults: 0
gleam-double-exit=0
       0
```

### 3. The design as it stands after GD06

This section adds to section 3 of GD05's hand-back, and through it to GD04's, GD03's,
GD02's and GD01's. Sources are under `gleam-double/src/pachinko_double/` and tests under
`gleam-double/test/`. Under `src/` the work commit adds `expiry.gleam` and `decay.gleam`
and changes `core.gleam`, `domain.gleam`, `owner.gleam` and `publish.gleam`
(`ev-handback-checks.txt`, part 6). `lease_end.gleam` and `settle.gleam` are byte for
byte as GD05 left them (part 5), and so are `pull.gleam`, `model.gleam`, `rules.gleam`,
`inspection.gleam`, `invariants.gleam`, `identity.gleam`, `consuming.gleam`,
`config.gleam`, `refusals.gleam` and `time.gleam`, which part 6 does not list.

- **`domain.gleam`.** `Event` gains a fourth variant, `LeaseLapsed(lease: String, now:
  Timestamp)`, beside `PocketTouched`, `Handed` and `Freed`. The specification's
  `LeaseLapsed(lease)` carries no time; `now` is the instant of the outcome that emitted
  it, which the expiry guards compare against (G60).
- **`expiry.gleam`** (new), the specification's "Lease expiry" section.
  - `pub type Lapse { Lapse(lease: String, now: Timestamp) }`, the input of both tables:
    the lease's id and the tick's `now`.
  - `pub fn lease_lapses() -> Table(Lapse)`, the table named `LeaseLapses`, with one row,
    `LeaseLapses`. Guard: the lease exists and is active; the `expires_at <= now` of the
    clock trigger is `core.tick`'s filter, not a guard (G60). Effect: `use now <- Now`,
    then `Finish(state, [LeaseLapsed(lease: lease.id, now:)], None)`; it answers
    `Impossible` where the lease is missing. When no guard holds, `NothingHappens`.
  - `pub fn lease_lapsed() -> Table(Lapse)`, the table named `LeaseLapsed`, with three
    rows in the specification's order: `LeaseExpiryParksPocket`,
    `LeaseExpiryKeepsPocketAtFront` and `LeaseExpiryIdlesEmptyPocket`. When no guard
    holds, `NothingHappens` (branch A, N37, G61).
  - Private helpers. `lapsing(state, lapse) -> Result(#(Lease, Pocket), Nil)` is the
    three shared `requires`: the lease active, `lease.expires_at <= lapse.now`, and its
    pocket leased. `parks(state, pocket)` is `consecutive_expiries + 1 >=
    max_consecutive_expiries`. The guards are `parks`, `!parks && lease_end.outlasts`
    and `!parks && !lease_end.outlasts`.
  - `expire(state, lapse, status) -> Plan` is the one effect of all three: `use now <-
    Now`, then `lease_end.end(state, lease, lease_end.Lapse, now)`, then the pocket with
    the new status and `consecutive_expiries + 1`, keeping its position; it finishes
    with the `Freed` events `lease_end.end` returned. It answers `Impossible` where
    `lapsing` fails. No expiry row allots a position, and none has a `Freed` route of
    its own: `core.chain` sends each `Freed` to GD04's `lease_end.freed()`.
- **`decay.gleam`** (new), the "Decay" section.
  - `pub fn superseded_record_decays() -> Table(SupersededRecord)` and
    `pub fn dead_letter_decays() -> Table(DeadLetter)`, each a table named after its one
    row. The input is the entry itself. The guard is `list.contains` on its queue; the
    effect removes the first equal entry with the private `without_one(entries, entry)`
    and keeps the others in order (G64). Neither asks for the time. When no guard holds,
    `NothingHappens`.
- **`publish.gleam`.** `pocket_touched()` now holds two rows, `ReviveIdlePocket` and
  `ReviveParkedPocket`, built by the private `revive(name: String, from: PocketStatus)
  -> Row(String)`; they differ only in the status they require. Both queue the pocket at
  a fresh position with its count reset. The Publish table is unchanged.
- **`core.gleam`.**
  - `Trigger` is unchanged: its eight variants are the triggers a surface provides, and
    no clock trigger is among them. `apply` is unchanged.
  - `pub fn run_at(table: Table(input), input: input, state: State, now: Timestamp) ->
    Step` runs one table as a whole outcome, as `run` does, with `now` already in hand
    (`rules.at`), so a row that asks for the time reads no clock.
  - `pub type Ran { Ran(state: State, fired: List(List(String)), failure:
    Option(Failure)) }`.
  - `pub fn run_due(state: State, outcomes: List(fn(State) -> Step)) -> Ran` runs the
    outcomes in order, each over the state the one before left, and stops at the first
    failure, keeping what came before. An outcome that asks for the time or a lease id
    is `Defect("Tick", EffectFailed("Tick", "an outcome of a tick asked for the time"))`
    or `"... asked for a lease id"`.
  - `pub type TickReport { TickReport(fired: List(String), lapsed: List(Lease),
    superseded: List(SupersededRecord), dead_letters: List(DeadLetter)) }` and
    `pub type Ticked { Ticked(state: State, report: TickReport, failure:
    Option(Failure)) }`.
  - `pub fn tick(state: State, now: Timestamp) -> Ticked`. First the active leases with
    `expires_at <= now`, sorted by `expires_at` with the stable `list.sort`, so equal
    ones keep their place in `state.leases`, which is creation order, not `granted_at`
    order (G57). Each is one `run_at(expiry.lease_lapses(), Lapse(lease.id, now), _,
    now)`. Then, only if no lapse failed, over the state the lapses left, the superseded
    records and then the dead letters with `expires_at <= now`, each queue oldest first,
    each one `run_at` of its table. An input whose outcome fired nothing is not
    reported. Each lapsed lease is read back by id from the state reached; a missing one
    would be `Defect("Tick", EffectFailed("LeaseLapses", "no lease " <> id))`, which no
    test reaches (Open points).
  - `chain` routes `LeaseLapsed(lease, now)` to `expiry.lease_lapsed()` with
    `Lapse(lease:, now:)`. Only `LeaseLapses`'s effect builds a `LeaseLapsed`, and only
    `core.tick` reaches `LeaseLapses` and the two decays (`ev-handback-checks.txt`,
    parts 8 and 12).
  - `registry()` lists 15 tables: `PublisherJoins`, `ConsumerJoins`, `Publish`,
    `PocketTouched`, `Pull`, `Handed`, `Renew`, `Ack`, `Nack`, `Release`, `Freed`,
    `LeaseLapses`, `LeaseLapsed`, `SupersededRecordDecays` and `DeadLetterDecays`. The
    names `LeaseLapses`, `SupersededRecordDecays` and `DeadLetterDecays` are the
    double's own: the specification names no temporal trigger, so each takes its one
    rule's name (G56).
- **`owner.gleam`.**
  - `pub type PartialTick { PartialTick(report: TickReport, failure: Failure) }`.
  - The opaque `Message` gains `Tick(reply: Subject(Result(TickReport, PartialTick)))`
    beside `Act`, `Inspect` and `LeaseView`.
  - On `Tick` the owner calls `core.tick(owner.state, owner.now())`, so it reads the
    clock once whatever is due, answers `Ok(report)` or `Error(PartialTick(report,
    failure))`, and keeps `ticked.state` either way (G59).
  - `pub fn tick(owner: Subject(Message)) -> Result(TickReport, PartialTick)`, a
    `process.call` with `call_timeout`.
  - `owner` is still the only module under `src/` that imports `gleam/erlang` or
    `gleam/otp` (`ev-verification.txt`).
- **`test/harness.gleam`.** `pub fn tick(double: Double) -> Result(TickReport,
  PartialTick)` calls `owner.tick` and logs `report.fired` only on `Ok`, so a
  `PartialTick`'s rules never reach `harness.fired`. A lapse is driven with
  `harness.at(double, millis)`, which predates GD06 and sets the clock to `t0 + millis` without
  counting a read, then `harness.tick(double)`.
- **`test/coverage.gleam`.** `pending_scenarios` and `pending_rules` are both `[]`
  (`ev-handback-checks.txt`, part 9). The sites GD07 removes with them:
  - the constants, `coverage.gleam:34` and `:38`;
  - the `Inputs` fields `pending_scenarios` and `pending_rules`, `:51` and `:52`, set at
    `:68` and `:69`;
  - the scenario half's pending branches and `unknown_pending`, `:122` to `:154`, and
    the rule half's, `:194` to `:233`;
  - in `coverage_test.gleam` the hand-built `Inputs` (`:104` and `:105`) and the tests
    that read the lists: "a scenario neither covered nor pending is a fault", "a
    scenario both pending and covered is a fault", "a pending entry the sources lack is
    a fault", "a rule with no row and not pending is a fault", "a rule both pending and
    a row is a fault" and "a row, not pending, that never fired is a fault".
- **Tests.** The run has 296 tests. New modules: `expiry_test`, `decay_test`,
  `branch_a_test`, `parking_test` and `lease_endings_test`. Changed: `core_test`,
  `owner_test`, `identity_test`, `publish_test` (comments), `harness`, `coverage` and
  `pachinko_double_test`. `main`'s list, in order: `rules_test`, `core_test`,
  `owner_test`, `competing_test`, `config_test`, `identity_test`, `publish_test`,
  `pull_test`, `lease_test`, `settle_test`, `lease_end_test`, `lease_endings_test`,
  `consuming_test`, `expiry_test`, `decay_test`, `branch_a_test`, `parking_test`,
  `coverage_test`, `inspection_test`, `invariants_test`.
  - Two registered tests share one tag for each of S066 and S067 (`branch_a_test`) and
    for each of S078, S079, S080, S081, S087 and S088 (`lease_endings_test`, ", by
    release" and ", by lapse"; part 3).
  - Helpers are private to their modules, and several are copied: a `tick` helper that
    unwraps the report sits in `expiry_test`, `decay_test`, `branch_a_test`,
    `parking_test` and `lease_endings_test`, and `fired`, `short` and `two_pockets`
    recur. Others a reader meets: `expiry_test`'s `lapsing`, `theirs`, `held` and
    `with_parked`; `branch_a_test`'s `late_one`, `late`, `tick_then_refused` and
    `compete`; `parking_test`'s `lapse`, `claim_and_lapse`, `parked`, `forked` and
    `strict_order`; `lease_endings_test`'s `end`, `lapse`, `held_as` and
    `after_the_end`. GD07's reader's guide may want them gathered.

Changes to a GD01 to GD05 seam, or departures from the ticket's wording, and what each
costs:

| Before, or as the ticket words it | After GD06 | Cost |
| --- | --- | --- |
| `domain.Event` had three variants | `LeaseLapsed(lease, now)` added; the specification's event carries no time (G60) | The expiry guards read `now` from the event, not from a clock |
| Only `core.run` ran a table | `run_at`, public, with `now` in hand | None recorded |
| The owner took `Act`, `Inspect` and `LeaseView` | `Tick` added; on a `PartialTick` it answers an `Error` and keeps the state the tick changed | Departs from shared rule 4 for a tick as a whole (G59) |
| `pocket_touched()` held one row | Two rows over the `revive` helper | None recorded |
| N34 and `brief-a.md`: lapse ties "in grant order" | Ties keep creation order in `state.leases`, which differs from `granted_at` once the clock steps back (C3) | `core.tick`'s doc comment still says "in the order they were granted" (Open points) |
| `pending_scenarios` held 34 ids and `pending_rules` 7 rules | Both `[]`, kept for GD07 | None |

### 4. Steps 2 to 7

Every output in this section ran uncommitted, on base
`bdd8fc18784dcf547b773c13bad289484192582d`.

Who did what, one writer at a time:

- The lead decided the seams and wrote them into `brief-a.md`: the temporal input
  carries the tick's `now`; `LeaseLapsed(lease, now)`; `expiry.gleam` and
  `decay.gleam`; `core.run_due` and `core.tick`; the owner's `Tick` reads the clock
  once; and no `core.Trigger` carries a temporal trigger. It made step 5's choice. By
  its account it asked the advisor before spawning and folded in six points:
  `run_due` generic and `tick` building the report; a decay removing one occurrence,
  with duplicates tested; step 5's test as the witness that the checker runs once per
  outcome; a defect using an existing constructor; no temporal `Trigger`; and a green
  run recorded mid-way. It wrote the port brief, `brief-port.md`, and the mutation
  brief, `brief-mut.md`.
- Agent A, from `brief-a.md`: the seams, the seven rows, S006, S059 and S065, step 4
  and step 5 (`report-a.md`).
- Agent B, from `brief-port.md`: 16 ports from `expiry.rs` in the new `parking_test`
  (`report-b.md`).
- Agent C, from `brief-port.md`: 13 ports from `decay.rs`, `lease.rs` and
  `identity.rs`, in the new `lease_endings_test` and in `decay_test` and
  `identity_test` (`report-c.md`).
- The mutation agent, from `brief-mut.md`: 74 mutants, and one test in `owner_test`
  (`report-m.md`).
- The notes agent: G56 to G67 and the amendments (step 7).
- At the checkpoint one runner subagent per lens drove Codex, and one fresh triage
  subagent checked the findings and applied the fixes (section 6).
- The harness refuses report files from subagents. The lead saved the replies as
  `report-a.md`, `report-b.md`, `report-c.md` and `report-m.md`.

#### Step 2: the plan

`plan-change` was loaded in GD01 and applied here. No scenario moved under shared rule 2,
and no other ticket's ownership table changed: each of the 34 owned ids carries a tag at
the work commit (`ev-handback-checks.txt`, part 2).

#### Step 3: `Tick` and the lapse

Agent A wrote the rows before the tests, and made its red runs afterwards by holding rows
back (section 5). Each red run's scratch diff is printed at the foot of its file, and
each was restored afterwards.

`ev-a-red-first.txt`: the `LeaseLapses` table's rows held back with
`list.take([lease_lapses_rule()], 0)`. 8 tests failed.

```text
$ just gleam-double
...
FAIL expiry_test.a lease lapses exactly at expires_at: crashed: ...
FAIL expiry_test.a lapse fires once per lease: crashed: ...
FAIL expiry_test.a zero lease_ttl lapses at the instant of grant: crashed: ...
...
COVERAGE S006 is not in pending_scenarios, and no test tagged Scenario("S006") passed
COVERAGE S059 is not in pending_scenarios, and no test tagged Scenario("S059") passed
COVERAGE S065 is not in pending_scenarios, and no test tagged Scenario("S065") passed
COVERAGE LeaseLapses is not in pending_rules and is a row of no table
248 tests run, 240 passed
coverage faults: 4
exit status: 1
```

`ev-a-red-last.txt`: both decay guards held back with `&& False`. 6 tests failed.

```text
$ just gleam-double
...
FAIL decay_test.SupersededRecordDecays removes exactly the entry it is given and reads no clock: crashed: ...
...
COVERAGE SupersededRecordDecays is a row of the SupersededRecordDecays table and not in pending_rules, and no test that passed fired it
COVERAGE DeadLetterDecays is a row of the DeadLetterDecays table and not in pending_rules, and no test that passed fired it
248 tests run, 242 passed
coverage faults: 2
exit status: 1
```

`ev-a-red-rows.txt`, at 257 tests: the three expiry guards and `ReviveParkedPocket`'s
held back. 19 tests failed, among them every `branch_a_test` port.

```text
$ just gleam-double
...
FAIL expiry_test.LeaseExpiryParksPocket changes exactly what it ensures and reads no clock: crashed: ...
...
FAIL branch_a_test.a renew after expires_at but before the lapse succeeds (branch A): crashed: ...
...
COVERAGE S066 is not in pending_scenarios, and no test tagged Scenario("S066") passed
COVERAGE S067 is not in pending_scenarios, and no test tagged Scenario("S067") passed
COVERAGE ReviveParkedPocket is a row of the PocketTouched table and not in pending_rules, and no test that passed fired it
...
257 tests run, 238 passed
coverage faults: 10
exit status: 1
```

Green runs: `ev-a-mid-green.txt`, the mid-way run the lead asked for, `248 tests run, 248
passed`; `ev-a-green.txt`, `257 tests run, 257 passed`. Each `coverage faults: 0`, exit
status 0.

Agent A's own mutants: five, in eight runs (`ev-a-mutants.txt`, which holds no `$` line).
Four runs failed only `gleam format --check` or the warnings-as-errors build and were
rewritten. "decay removes every equal entry", "run_due skips a failed outcome and goes
on", "decays read the state before the lapses (N35)" and "ties in reverse grant order"
were killed there. The fifth, "expiry stamps expires_at, not now", never reached the
tests in that file; `ev-a-mutant-stamp.txt` holds its killed run, `257 tests run, 247
passed`.

#### Step 4: branch A in both orders

S066 and S067 each have two registered tests in `branch_a_test`: a renew and a release
for S066, an ack and a pull for S067 (G63). Each half builds the same setup on two owners,
asserts their snapshots equal, and drives both from the one test process: on one the
`Tick` goes first and the action behind it is fenced, or for the pull claims under a new
lease; on the other the action goes first and the `Tick` lapses nothing. After the
checkpoint (B1) an untagged late-nack test runs the same two orders.

The competing-callers test, "a renew and a tick sent at once by two callers agree
whichever the owner takes first", untagged, passes in `ev-verification.txt`. It runs
twenty times, ten with each process started first, and accepts either consistent
result. It cannot show that both orders happened. By agent A's account a scratch print
showed each order ten times in twenty runs; that run was not saved, so the count is not
recorded.

The break, `ev-a-break.txt`: `Renew` emits `LeaseLapsed` for a lease past its
`expires_at`, so the renew lapses its own lease.

```text
$ just gleam-double
...
FAIL branch_a_test.a renew after expires_at but before the lapse succeeds (branch A): crashed: ...
FAIL branch_a_test.a renew and a tick sent at once by two callers agree whichever the owner takes first: crashed: ...
253 tests run, 251 passed
coverage faults: 0
exit status: 1
--- scratch diff for this run (restored afterwards) ---
...
-              Finish(extend(state, lease, now), [], None)
+              case lease.expires_at <= now {
+                True -> Finish(state, [domain.LeaseLapsed(lease.id, now)], None)
+                False -> Finish(extend(state, lease, now), [], None)
+              }
...
restored: gleam-double/src/pachinko_double/settle.gleam: OK
```

The `restored:` line is `shasum -c` against `settle.sha`, which still matches at the
work commit (`ev-handback-sha.txt`). S066's renew half fails; its release half is not
touched by a break in `Renew`. By agent A's account an earlier attempt ticked inside
every `Act` and failed mostly on clock-read counts; its evidence file was overwritten,
so its output is not recorded. The mutation pass's AL1, the same break, was killed too
(below).

#### Step 5: a failure part-way through a tick

The lead's choice: a tick keeps each outcome that succeeded before a failure, stops at
the failure and reports it, and does not roll back. It matches the Python double
(finding 9). The owner answers `Error(PartialTick(report, failure))` and keeps the state
the tick reached. That departs from shared rule 4 for a tick as a whole; each outcome
still changes nothing when it fails (G59).

The tests that pin it:

- `core_test`'s "a run of due outcomes keeps each outcome that succeeded before a
  failure, stops at the failure, and checks each outcome at its end", on scratch tables:
  the first outcome is kept, the second fails with
  `BrokenInvariant(["UniquePublisherId"])`, the third never runs. It is also the witness
  that the checker runs once per outcome.
- `core_test`'s "a due outcome that asks for a lease id or for the time is a defect of
  the tick".
- `expiry_test`'s "a tick whose second lapse fails keeps the first, stops there, and
  decays nothing", through the real tables over a hand-built state.
- `owner_test`'s "a tick whose outcome fails is answered PartialTick, and the owner
  keeps what the outcomes before it did", added by the mutation agent. A lease-id source
  that repeats an id gives two active leases one id (G27); the second lapse fails
  `LeasedHasOneActiveLease`, and the owner answers `PartialTick`, keeps the first lapse,
  and reads the clock once. A second tick meets the same failure and changes nothing.

#### Step 6: the rest, ported

Agent B's 16 ports, each one registered test in `parking_test`: S024, S025, S026, S034,
S048, S056, S058, S060, S061, S062, S063, S064 (library half), S069, S075, S077 (library
half) and S092. None of their Rust tests loops over `ENDINGS`. S026 is labelled `oq:7`
in its name and its module comment (`ev-handback-checks.txt`, part 7). S024 and S063
build two identical owners where the Rust forks one queue.

Agent C's 13 ports: S002 in `identity_test`; S007, S073 and S074 in `decay_test`; and in
`lease_endings_test` S076, S089 (a lapse half only, as in the Rust), S093 (one test,
each mark on an owner of its own), and S078, S079, S080, S081, S087 and S088, each as two
registered tests, by release and by lapse. Those six are exactly the Rust tests that loop
over `ENDINGS` (`ev-handback-endings.txt`: six `for ending in ENDINGS` lines in
`lease.rs`, under S078, S079, S080, S081, S087 and S088).

The ruling on N38: S066's release half keeps the released lease in the state; S093
compares a pocket's records by key; S075 asserts the position invariants and an empty
`check` at each step, not a stored pocket order (G67).

The `impl-choice` labels, each in its test's comment:

- The absolute positions in S048, S078 to S081, S088 and S093 (G48, extending G04). Each
  of those ports also compares the position with the state before.
- The order of mixed `Freed` rules in the fired lists, and of the entries one lease end
  creates, in S048, S064, S075, S077, S007, S074, S087, S088 and S089 (G38, as N25 and
  N29).
- After the checkpoint (C2, C3 and B2): the lapse tie order, which of two equal entries
  decays, and the superseded queue's decay order (G57, G64).

Never red, and why: every one of the 29 ports passed on its first run, because the rows
already existed. Each fails under at least one recorded one-line mutant, every mutant run
ending exit status 1:

- Agent B, 13 mutants in `ev-b-mutant-<id>.txt`, `273 tests run` in each. `report-b.md`
  marks the kills of S048 by `nacked-arrival` and S058 by `release-ends` as the lead's
  inference from the names; the evidence confirms both, since each file's `FAIL` lines
  name that port.
- Agent C, 9 mutants in `ev-c-mutant-<id>.txt`, `292 tests run` in each. Both halves of
  each `ENDINGS` port fail under its mutant: `acked-arrival` for S078 and S088,
  `nacked-arrival` for S079 and S080, `held-shed-at` for S081 and `ending-nacked` for
  S087.
- No `ev-b-mutant-*` or `ev-c-mutant-*` file holds a `restored:` line, so each restore
  rests on the agents' word. `src/` at the work commit matches every line of
  `c-before.sha` that names it (18 files) and of `m-scratch/manifest-src.sha256`, which
  the mutation agent took after agent C's green run (`ev-handback-sha.txt`).

Green runs: `ev-b-green.txt`, `273 tests run, 273 passed`; `ev-c-green.txt`, `292 tests
run, 292 passed`. Each `coverage faults: 0`, exit status 0.

The mutation pass (`ev-m-mutants.txt`, which holds no `$` line; definitions and runner
under `m-scratch/`). Its last four lines, of which the middle two are left out here
because the owner mutants' ids, two capitals and a digit, fail `typos`; they say that run
1 ran 73, of which 21 failed only the warnings-as-errors build and were rewritten and 4
survived (T6, T15, T16 and the fourth owner mutant), and that the new `owner_test` test
kills that fourth one and T17 was added in run 2:

```text
totals: 74 mutants ran (final run, m-scratch/run3.tsv); 71 killed, 3 survived, all three equivalent.
...
every restore matched its copy's SHA-256, and src/ matched m-scratch/manifest-src.sha256 after each mutant.
```

- The 74 cover the seven rows, the shared expiry guard and effect, `core.tick` (T1 to
  T17), `run_due` (RD1 to RD3), four mutants of the owner's `Tick` handler, and AL1, the
  step 4 break in `renew`, which failed nine tests.
- T6 (each due lease offered twice), T15 (dead letters paired with the wrong fired
  lists) and T16 (decays run at another `now`) survive as equivalent: the second offer
  stops at `LeaseLapses`' guard, every due decay fires, and no decay reads the time.
- The fourth owner mutant (the owner answers `Ok` on a failed tick) survived run 1 and is
  killed by the new `owner_test` test, the first to make the owner's `PartialTick` path
  fail.
- `report-a.md` names `core_test`'s "a due outcome that asks for a lease id or for the
  time is a defect of the tick" never red. RD2 fails it (`m-scratch/run3.tsv`, line 67);
  `ev-m-mutants.txt` shows only "+41 more" for RD2.
- The mutation agent's final run, `ev-m-green.txt`: `293 tests run, 293 passed`,
  `coverage faults: 0`, exit status 0.

On exit `pending_scenarios` and `pending_rules` are both empty, and dispatch returned no
`GuardOverlap` on the `LeaseLapsed` or `PocketTouched` table in any green run.

#### Step 7: spec notes

`gleam-double/SPEC-NOTES.md` gained G56 to G67; it holds 67 notes
(`ev-handback-checks.txt`, part 10). The notes agent amended G04, G05, G19, G24, G27,
G32, G38, G40, G45, G46, G47 and G48. Part 10 also lists G55, only because the hunk that
adds G56 starts on G55's trailing blank line. No `G-new` placeholder is left under
`gleam-double/`.

| Note | Title, in short | `N`-note |
| --- | --- | --- |
| G56 | a tick is the double's only clock, and it reports what it did | Agrees with N33; departs in adding `fired` to the report |
| G57 | one tick lapses in `expires_at` order, then decays in queue order | Agrees with N34, and states the tie within one instant |
| G58 | an entry a lapse creates with a zero TTL decays in the same tick (S007) | Agrees with N35 |
| G59 | a tick keeps each outcome that succeeded before a failure | Agrees with N33 and finding 9; departs from shared rule 4 for a tick as a whole; `PartialTick` has none |
| G60 | the clock trigger's `<= now` is the tick's filter, and the trigger carries `now` | Agrees with N37; the event's time has none |
| G61 | the expiry rules' answer when no guard holds is unreachable through the owner | Agrees with N37 |
| G62 | what "the owner may apply a lapse ahead of an already-queued action" permits | No `N`-note |
| G63 | branch A in both orders, competing callers, and the break | Agrees with N33 and N38's ruling |
| G64 | a decay names its entry by value and removes one occurrence | No `N`-note |
| G65 | only a publish revives a parked pocket (`oq:7`) | Agrees with N36 |
| G66 | a port of a test that loops over `ENDINGS` is two registered tests | No `N`-note |
| G67 | S066's release half, S093 and S075 as the Rust's inspection shape | Agrees with N38 and follows its ruling |

G48's GD06 half departs from N38: the seven ports keep the Rust's absolute positions
beside the comparison with the state before. The step 7 list maps to the notes: the order
(G57, N34), the zero-TTL entry (G58, N35, S007), the report (G56, N33), S026's `oq:7`
(G65, N36), the unreachable answer (G61, N37), and the Lease expiry sentence (G62).

The notes' `just lint`, `ev-notes-lint.txt`, exit status 0; `just gleam-double`,
`ev-notes-green.txt`, `293 tests run, 293 passed`, `coverage faults: 0`, exit status 0.
The checkpoint then corrected G27, G57, G58, G61, G63 and G64 (section 6).

### 5. Incidents

**The rows came before the tests.** Agent A wrote all seven rows before any of S006,
S059 and S065, against the brief's red-first order (`brief-a.md`, item B) and step 3's
"Write S059, S065 and S006 first". `report-a.md` says so. Its three red runs were made
afterwards, by holding rows back in a scratch diff: they show that each test can fail
without its row, not that it was written first. The lead did not have the rows removed
and rewritten; the 74-mutant pass and the per-port mutants stand in for the missing
order.

**Three replies were truncated.** The lead's account is three; the evidence files show
two:

- Agent B's reply was cut off in its mutant list. The lead read the
  `ev-b-mutant-*.txt` files instead of resuming the agent; its sections on the
  specification's silences and on the next agent are not recorded (`report-b.md`). The
  two kills the lead inferred from the mutants' names hold (section 4).
- Agent C's reply was cut off at "Rows co…". The lead checked `c-before.sha` against the
  tree and found `src/` unchanged, so no row was corrected (`report-c.md`). The rest of
  the reply, its rows section, the specification's silences and its advice, is not
  recorded. When `c-before.sha` was taken is not recorded either: it was last written at
  08:53:45Z, 30 seconds before `ev-c-green.txt`, and it lists `lease_endings_test.gleam`,
  which `report-c.md` calls new (`ev-handback-sha.txt`).
- Which the third reply was, and how it was completed, is not recorded. `report-a.md`
  was saved "from the agent's two replies", and whether the first of them was cut off
  is not recorded.

The cause, by the lead's account: idle replies longer than about 4 KB are cut off.

### 6. Checkpoint record

One checkpoint, at step 8: conformance and breakage, before the final commit.

- **`codex exec --help` on the day.** It was checked in GD04 and not rerun.
  `ai_tmp/gd/GD04/ev-codex-help.txt` was last written at `2026-11-21T06:09:24Z`; the two
  review logs at `2026-11-21T09:24:12Z` and `09:24:30Z`, the same UTC day.
- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `sandbox: read-only`,
  reasoning effort `ultra`, one fresh process per lens, by the ticket's corrected
  command (`log-conformance.txt` and `log-breakage.txt` each open with those lines).
  Codex was on `PATH`, so no fallback to a Claude reviewer was needed.
- **Recovery.** Both findings files open `Source: last-message`, and below that line
  each matches its `last-<lens>.md`: each is Codex's last message, recovered because a
  read-only Codex writes no file.
- **What the reviewers ran.** The conformance lens ran nothing: "Evidence was read-only
  source and diff inspection." The breakage lens ran no recipe or mutant, and B3's
  evidence is "An in-memory regex probe". B1 and B2 were assessed statically. The triage
  ran each claim.
- **Brief.** The diff since the base, with the untracked files, and four known points
  not to report again: the rows before the tests, G27, the three equivalent mutants and
  G35 (`head.txt`).
- **Triage.** One fresh subagent, neither a reviewer nor an author.
- **Rounds.** One. B1 was medium and the rest low; no finding was high, so no second
  round was owed.
- **Not recorded.** The model of the runner and triage subagents.

It ran uncommitted, on base `bdd8fc18784dcf547b773c13bad289484192582d`. The reviewed tree
is `084a2775f4c8ce7509c11381667ed2f3f31a50cd` (`tree-before-review.txt`). Before any fix
`just gleam-double` ended `293 tests run, 293 passed`, `coverage faults: 0`, exit status
0 (`ev-baseline.txt`). The conformance lens filed C1 to C6, the breakage lens B1 to B3.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid | The Rust's forks share one clock, so S063's acked fork lapses at t0 + 40; the port lapsed it at t0 + 30 | `parking_test`: the acked owner lapses at `3 * ttl`; the fork comment says each owner's clock stands where the Rust's shared clock does |
| C2 | low | valid | The lapse tie order and the first of two equal entries decaying assert more than the specification, unlabelled | `impl-choice` in both tests' comments; G57 and G64 carry the label |
| C3 | low | valid | With the clock set back, creation order and `granted_at` disagree; G57 said they agree in every reachable state | New `expiry_test` test, "a tie in expires_at lapses in creation order, even against granted_at after the clock stepped back", `impl-choice`; G57 corrected |
| C4 | low | valid | G58's "once a tick returns nothing is due" is false for a `PartialTick` | G58 says "returns `Ok`" |
| C5 | low | valid | G27 said no other owner-driven test asserts a literal lease id; the partial-tick test does | G27 names it |
| C6 | low | valid | At a zero TTL a renewed lease is due again at once; G61 said it was not yet due | G61 corrected |
| B1 | medium | valid | A nack extending from `min(now, expires_at)` survived every test | New `branch_a_test` test, "a nack after expires_at but before the lapse succeeds and extends the lease from its own time (branch A)", untagged; G63 names it |
| B2 | low | valid | Reversing the due superseded records survived every test | New `decay_test` test, "two distinct superseded entries due in one tick decay in queue order", untagged, `impl-choice`; G57 names it |
| B3 | low | valid | The Verification clock pattern misses an unqualified `sleep(…)`, which `gleam_erlang` exports | Handed back: the fix is in this ticket, outside the triage's files |

Eight were applied by the triage, to `parking_test.gleam`, `expiry_test.gleam`,
`decay_test.gleam`, `branch_a_test.gleam` and `gleam-double/SPEC-NOTES.md` (G27, G57,
G58, G61, G63 and G64), all in this ticket's Files touched. Nothing under
`gleam-double/src/` changed (`ev-handback-sha.txt`). The three new tests raised the
count from 293 to 296; no pending list changed.

Each mutant ran through `scratch/mutant.sh`, and each file ends with a `restored:` line
whose two SHA-256 values match:

| Mutant | Run | Result |
| --- | --- | --- |
| `LeaseExpiryIdlesEmptyPocket` holds only at exactly `expires_at` (C1) | Before, `ev-C1-idle-exact-only-survives.txt` | `293 tests run, 291 passed`; S063's port not among the failures |
| The same | After, `ev-C1-idle-exact-only-killed.txt` | `296 tests run, 293 passed`; S063's port among them |
| Ties broken by `granted_at` (C3) | After, `ev-C3-tie-by-granted-killed.txt` | `296 tests run, 295 passed`; the new test the one failure |
| The nack extends from `min(now, expires_at)` (B1) | Before, `ev-B1-nack-min-survives.txt` | `293 tests run, 293 passed` |
| The same | After, `ev-B1-nack-min-killed.txt` | `296 tests run, 295 passed`; the new test the one failure |
| The due superseded records reversed (B2) | Before, `ev-B2-shed-reverse-survives.txt` | `293 tests run, 293 passed` |
| The same | After, `ev-B2-shed-reverse-killed.txt` | `296 tests run, 295 passed`; the new test the one failure |
| The due dead letters reversed (B2's sibling) | Before, `ev-B2-letters-reverse-killed.txt` | `293 tests run, 292 passed`; S074's port fails |

`ev-B3-probe.txt`, the triage's B3 check, exit status 0: the old pattern matched neither
unqualified spelling, the installed API exports `sleep` and `sleep_forever`, and nothing
under `src/` mentions `sleep`.

B3, handed back by the triage and applied by the lead to this ticket's Verification
block, the only change at the work commit outside the double and its notes. It goes
beyond what Files touched lists for this file, `status:` and the hand-back notes; the
commit message says so. The triage suggested `\bsleep\b`; the lead applied `\bsleep`,
which also matches `sleep_forever`. The diff, from `git diff` between the base and the
work commit:

```text
-clock='send_after|process\.sleep|system_time|monotonic_time|import gleam/time|@external'
-probe=$(printf 'process.send_after(s, 5, Tick)\nprocess.sleep(5)\nos.system_time()\nerlang.monotonic_time()\nimport gleam/time/timestamp\n@external(erlang, "os", "timestamp")\n' | rg -c "$clock")
-test "$probe" -eq 6
+clock='send_after|\bsleep|system_time|monotonic_time|import gleam/time|@external'
+probe=$(printf 'process.send_after(s, 5, Tick)\nprocess.sleep(5)\nsleep(5)\nsleep_forever()\nos.system_time()\nerlang.monotonic_time()\nimport gleam/time/timestamp\n@external(erlang, "os", "timestamp")\n' | rg -c "$clock")
+test "$probe" -eq 8
```

The lead's check of the new lines, uncommitted, on base
`bdd8fc18784dcf547b773c13bad289484192582d` (`ev-b3-pattern.txt`, quoted whole;
`b3-check.sh` holds the four lines above it, from `clock=` to the `rg -n` over `src`):

```text
$ sh -c set -eu; . ai_tmp/gd/GD06/b3-check.sh; echo "probe=$probe"
probe=8
exit status: 0
```

After the fixes, `ev-tests-green.txt`: `296 tests run, 296 passed`, `coverage faults: 0`,
exit status 0. `ev-lint.txt` is `just lint` after the note edits, exit status 0. The
re-run after the checkpoint (`ev-final.txt`), uncommitted, on base
`bdd8fc18784dcf547b773c13bad289484192582d`:

```text
$ just gleam-double
...
296 tests run, 296 passed
coverage faults: 0
exit status: 0
```

`ev-final.txt` was last written at 09:29:58Z, before `ev-b3-pattern.txt` (09:30:34Z);
the B3 edit touches no file the run reads. The tree after the triage is not recorded.
By the lead's account the work commit's tree, `31c47f544a56e189bd4435a5e46f4d66514267e7`,
is that tree plus the ticket edit. Whether `just check` ran between the triage and the
commit is not recorded; it ran at the work commit (section 7).

### 7. Verification

The Verification block, as B3 left it, ran at `98107a03b5374ee243f1caeade773ce16087e755`,
the work commit, as one `sh` script under `set -eu` (`verification.sh`, the ticket's
block word for word; output in `ev-verification.txt`). It exited 0, so every `test` in it
held, the widened clock pattern's probe counted 8 and matched nothing under `src/`, and
`git status --short` printed nothing. The two lines `273 tests run: 273 passed, 0
skipped` inside `just check` are the Rust workspace's test runs, not the double's.

```text
$ sh ai_tmp/gd/GD06/verification.sh
98107a03b5374ee243f1caeade773ce16087e755
...
gleam 1.19.1
...
OTP 29
...
296 tests run, 296 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

The run counted 232 tests at the base (`baseline.log`); 248 at the first two red runs and
the mid-way green run; 253 at the break; 257 at the rows' red run and agent A's green
run; 273 after agent B; 292 after agent C; 293 after the mutation pass and the notes; and
296 after the triage (`ev-final.txt`) and at the work commit.

### 8. The acceptance criteria against the evidence

1. `just gleam-double` exits 0 and shows the counts: `ev-verification.txt`.
2. `pending_scenarios` and `pending_rules` are both empty: `ev-handback-checks.txt`,
   part 9, with `coverage faults: 0` in `ev-verification.txt`.
3. Every owned scenario has a tagged test: part 2 counts at least one tag for each of
   the 34 (S002's four include three hand-built inputs in `coverage_test`; its one test
   is in `identity_test`). Each of the six `ENDINGS` scenarios has a release and a lapse
   test: part 3, against `ev-handback-endings.txt`. Each of the seven rules is a row of
   its trigger's table (part 4) and fired in a passing test: `coverage faults: 0`, since
   the rule half faults on a row, not pending, that never fired.
4. No `Freed` row and no second copy of the end-of-lease steps: part 5 shows
   `lease_end.gleam` and `settle.gleam` unchanged since the base, and `expire`'s one call
   to `lease_end.end(state, lease, lease_end.Lapse, now)`.
5. The step 4 break is quoted with its output and exit status (section 4,
   `ev-a-break.txt`), and the competing-callers test passes in `ev-verification.txt`.
   It falls short in one way: no saved run shows both orders occurring (Open points).
6. Step 5's choice is pinned by four tests and recorded as G59 (section 4).
7. Nothing under `src/` reads the time except through the injected clock, arms a timer or
   sleeps: the Verification's clock pattern, widened by B3, matches nothing under `src/`
   in `ev-verification.txt`. Only a `Tick` applies a temporal trigger: `core.Trigger`
   holds no clock trigger, and `LeaseLapses` and the two decays are reached only from
   `core.tick`, and `LeaseLapsed` is built only by `LeaseLapses` (parts 8 and 12).
8. The notes step 7 lists are G56, G57, G58, G61, G62 and G65 (section 4), and S026 is
   labelled `oq:7` in its test and in G65 (part 7).
9. `just gleam-double` leaves nothing Git can see and no `erl_crash.dump`:
   `ev-verification.txt`, whose script tests both under `set -eu` and exits 0.
10. The checkpoint is recorded in section 6, with the `codex exec --help` check made on
    the same day in GD04.
11. `just check` ends with its line, and `git status --short` prints nothing:
    `ev-verification.txt`.

### 9. Stops

None fired. The conditions watched:

- A ported scenario failing against rows that follow the specification, with no ruling
  to cover it (shared rule 1). None did.
- A `GuardOverlap` on a specification table (shared rule 4). Dispatch returned none on
  the `LeaseLapsed` or `PocketTouched` table; KF4, KF5, IE2 and IE3, which widen one
  expiry guard into another's, were each killed.
- A scenario that needs an operation this ticket does not land (shared rule 2). None
  moved.
- A count that moved. The coverage check asserts 93, 32 and 35 on every run, and every
  green run ends `coverage faults: 0`.
- A reach for the network. Nothing was fetched and no network recipe ran. The review
  checkpoint called a model API, which the checkpoint authorises.

Step 9's stop stands: nothing was pushed and no pull request was opened.

### 10. What was not run, and what was not recorded

Not run:

- Any push, pull request or issue.
- `just initialize`, `just gleam-double-deps` and `just lock`.
- A CI run.
- A second review round. None was owed.
- The mutation pass after the checkpoint. The 74 mutants ran at 293 tests, before the
  triage's three; `src/` is the same at the work commit, so no kill is lost, but only
  the triage's own mutants ran against the new tests.

Not recorded: the model of every subagent; the competing-callers scratch run and its
count of orders; the first break attempt's output; the missing sections of agents B's and
C's replies; which the third truncated reply was; when `c-before.sha` was taken; the tree
after the triage; and whether `just check` ran between the triage and the commit.

### 11. Working lessons for GD07

- Idle replies longer than about 4 KB are cut off. Ask for the totals first, or read the
  evidence.
- A port agent over rows that already exist reports never-red ports. Ask for one killing
  mutant per port, recorded with its diff and a `restored:` line.
- A brief's red-first order does not hold by itself: ask for the first red run before
  the rows, and check its time against the rows' manifest.
- A name in a brief ("grant order") can outlive the finding that corrects it: after a
  triage, search `src/` comments for the corrected wording.

## Open points

- **PD07 asks whether the order of the queued pockets is stored or only derived.** This
  ticket follows the ruling on N38 as it stands (G67); PD07's answer may change the ports
  of S066, S075 and S093.
- **Whether a tick should be atomic as a whole is finding 9 of `double/SPEC-NOTES.md`.**
  Step 5 answers it for this double only (G59).
- **G59 departs from shared rule 4 for a tick as a whole.** The owner answers
  `PartialTick` and keeps the state the tick changed. A failure that stays in the state
  stops every later tick at the same outcome, so what is due after it never applies.
  G59's own comment says the repeated lease id is the one way a tick through the owner
  can fail with the specification's tables, and that nothing proves it.
- **The questions the notes leave for the maintainer.**
  - G48, as G04, G16, G29 and G33: should sixteen rows assert absolute positions if
    `next_position`'s start is unspecified?
  - G56, N33's: should the specification or a surface name how the temporal triggers
    are processed and what that reports?
  - G57, N34's and N29's: should the specification order what several lapses in one tick
    create, or say the order is not observable?
  - G58, N35's: is "nothing is due once a tick succeeds" intended?
  - G59, finding 9's: should a tick be atomic, and should one outcome that cannot apply
    hold back those due after it at every later tick?
  - G60: should a rule answering an event read the `now` of the outcome that emitted it?
  - G61, N37's: are the expiry rules' `active` and `expires_at <= now` guards in their
    own right, or restatements?
  - G62: is a lapse overtaking a queued action a freedom of the implementation, or
    something every implementation must be able to show?
  - G63, as G32: should the catalog hold a row in which an action and a lapse compete?
  - G64: if PD07 or open question 2 makes an entry something a reader names, should it
    have an identity?
  - G65: open question 7 itself.
  - G67: should S075's Rust comment say which pocket parks?
- **Three mutants survive as equivalent.** T6, T15 and T16 (section 4). Each rests on an
  argument, not a test.
- **The competing-callers run was not saved.** The test accepts either order and cannot
  show both happened; the count of ten each in twenty runs is agent A's account only.
- **`core.tick`'s doc comment says lapse ties go "in the order they were granted".** The
  code keeps creation order in `state.leases`, and C3 showed the two differ once the
  clock steps back (G57). The comment was not corrected, since the triage changed nothing
  under `src/`.
- **`core.report`'s missing-lease branch has no test and no mutant.** A lease is never
  removed, so it is unreachable; `report-a.md` names it never red.
- **A lease-id source that repeated an id would give two leases one id (G27).** Carried
  from GD04. The double still does not check that an id it is handed is new, and the
  `owner_test` partial-tick test depends on that.
- **The rows were written before their tests** (section 5). The red runs and mutants show
  each test can fail; no evidence shows a test written first.
