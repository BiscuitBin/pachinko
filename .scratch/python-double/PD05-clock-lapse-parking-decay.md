---
id: PD05
title: "Python double: the clock, with lapse, parking and decay"
status: done
depends_on: [PD04]
parallel_with: []
branch: ticket/pd05-clock-lapse-parking-decay
estimated_size: L
---

# PD05: Python double: the clock, with lapse, parking and decay

## Context

PD01 to PD04 landed every consumer and publisher action of the Python double under
`double/`, and the lease's end by release in every row of the lease-end table. Read their
hand-backs and `double/SPEC-NOTES.md` first. `.scratch/python-double/README.md` is the
series' index.

So far a lease that is never released lasts forever, no pocket parks, and nothing
decays. This ticket adds the clock's side:

- `tick()`;
- the lapse and the three expiry rules;
- parking, and the revival of a parked pocket;
- the cases where an action comes after `expires_at` but before the lapse is processed;
- the decay of both shed queues;
- the lapse half of every "by release or lapse" scenario.

The specification facts it rests on, all from `docs/specs/pachinko.allium` (the Lease
expiry, Lease end and Decay sections, and the `ExpiryRecoversWithinTtl` guarantee):

- **Temporal triggers.** `LeaseLapses` fires for an active lease whose
  `expires_at <= now`. `SupersededRecordDecays` and `DeadLetterDecays` fire for an entry
  whose `expires_at <= now`. The double has no background time. It processes these only
  when `tick()` is called, which reads `now` once from the injected clock and applies
  everything due. `tick()` runs `Queue`'s apply step once for each due lapse and once for
  each due decay, so the checker runs after each outcome.
- **Branch A** (decision 0019). The lapse and the actions are one serialized stream. A
  pull, ack, nack, renew or release processed after `expires_at` but before the lapse
  succeeds, as long as its other guards hold. A pull, ack, nack or renew moves
  `expires_at` on. A release ends the lease. The lapse then stops at its guards, unless
  the new `expires_at` has also passed, as it always has when `lease_ttl` is 0 (S006,
  S066, S067). So an action never lapses a lease itself, and only `tick()` does.
- **The three expiry rules.** Each needs the lease active, `expires_at <= now` and the
  pocket leased. Each ends the lease `expired` at `now`, sends every held record `Freed`
  and raises `consecutive_expiries` by one. They are the rows of the `LeaseLapsed` table,
  and each one's effect ends the lease through PD03's lease-end module, which already
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

Questions the specification leaves to the implementation, which this ticket answers in
the double and records as spec notes:

- the order in which one tick applies several due lapses and decays;
- whether an entry that a lapse creates in a tick decays in the same tick when its TTL is
  0 (S007);
- what `tick()` reports.

The Rust tests this ticket ports from are in `crates/libpachinko/tests/queue/expiry.rs`,
`decay.rs`, `lease.rs` and `identity.rs`, found with `rg -n 'scenario: S0NN'`. A Rust test
that loops over `ENDINGS` runs once by release and once by lapse. Port both halves, for
example with `subTest`. S066 and S067 each have two halves in two Rust tests; tag both
Python tests. The `wire` rows here are S064 and S077; port only their library half.

## Goal

`tick()` processes everything the clock has made due and reports it. A lease lapses at
`expires_at`, and once. A pocket parks on its `max_consecutive_expiries`-th consecutive
lapse, and a publish revives it. Superseded records and dead letters decay at their
`expires_at`. Branch A holds in both orders. Every scenario below is pinned by a tagged
test, and on exit nothing is pending.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (7) | `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket`, `SupersededRecordDecays`, `DeadLetterDecays` |
| Scenarios (34) | S002, S006, S007, S024, S025, S026, S034, S048, S056, S058, S059, S060, S061, S062, S063, S064, S065, S066, S067, S069, S073, S074, S075, S076, S077, S078, S079, S080, S081, S087, S088, S089, S092, S093 |

## Non-goals

- No background thread, timer or wall clock. Time moves only when a test sets the fake
  clock, and due work happens only in `tick()`.
- No resolution of open question 7, or of any other.
- No change under `crates/`, `docs/specs/`, `docs/decisions/`, `loadgen/` or `fuzz/`.

## Shared rules (PD01 to PD06)

Every ticket in this series carries these rules word for word. They bind this ticket as
written here; nothing outside it amends them.

1. **Where the behaviour comes from.** The double is derived from
   `docs/specs/pachinko.allium` alone.
   - `crates/libpachinko/src/` is never read for behaviour. The tests under
     `crates/libpachinko/tests/` are read only to port a scenario: its setup, its steps
     and what it asserts. S005's owning test is a doctest in
     `crates/libpachinko/src/config.rs`; that doctest is the one part of `src/` a ticket
     reads.
   - If a ported scenario fails against a double that follows the specification, record
     it in `double/SPEC-NOTES.md` and **stop**: put it to the maintainer. Never make it
     pass by copying the Rust.
   - A row that asserts more than the specification states is still ported, and its test
     and its spec note are labelled `impl-choice`. Known cases: S005, whose `compile_fail`
     doctest becomes a runtime refusal; S091's pinned order of the entries one lease end
     creates; and the refusals for an unknown publisher or consumer.
   - For a `wire` row, port only the library half. The HTTP half is out of scope.
   - Never silently resolve an `open question` (`AGENTS.md`). An `oq:N` row is ported as
     the rules stand, and its test and its spec note name the question.
2. **Scenario ownership.** Each S-id is owned by exactly one ticket and listed in that
   ticket's ownership table under Goal. Each test that pins one carries a
   `# scenario: S0NN` comment on the line above its `def`.
   - A scenario that turns out to need an operation this ticket does not land moves to the
     ticket that lands it. Update both tickets' ownership tables, the pending list and
     `.scratch/python-double/README.md`, and say so in the hand-back. Never drop one.
3. **Coverage tests.** `double/tests/test_coverage.py`, landed in PD01, parses three lists
   and fails unless each entry is covered or pending.
   - The catalog's rows, `^\| (S[0-9]{3}) \|` in `docs/reference/testing.md`, are each
     covered by a `# scenario:` tag under `double/tests/`.
   - The specification's rules, `^rule (\w+) \{`, are each covered when two things hold:
     the rule has a row in its trigger's table, and those tables are the registry; and
     the dispatcher recorded that row as fired during the run. The rule half fails on a
     registry name the specification does not hold, on a specification rule that has no
     row and is not pending, and on a row, not pending, that never fired. The fired-rule
     check reads the record only after every other test module has run, since `discover`
     would otherwise run `test_coverage.py` first; PD01 settles how.
   - The specification's invariants, `^\s*invariant (\w+) \{`, are each covered by an
     entry in the checker's registry and by a positive control tagged
     `# invariant-control: <Name>`. Fifteen of the thirty-five sit indented inside an
     entity, so a pattern anchored at column 0 finds only twenty.
   - It asserts the three counts it parsed, 93, 32 and 35, so a pattern that silently
     matches less fails. A specification change that moves a count updates the constant
     and adds a spec note. A tag that names an id or a name the sources do not hold fails.
   - `PENDING_SCENARIOS` and `PENDING_RULES` hold what is not covered yet. An entry that is
     both pending and covered fails, where a rule counts as covered once it is registered,
     so each ticket shrinks the lists by exactly what it lands. PD06 deletes both lists.
4. **Design constraints.** PD01 proposes the class layout and reviews it. These are fixed:
   - The clock and lease ids are injected: a `Clock` protocol with a fake whose time a test
     sets and whose reads it counts, and a lease-id source with a deterministic fake. This
     follows the house rule that effects sit behind seams (decisions 0003 and 0008). The
     double reads no wall clock and draws no randomness.
   - A refusal changes nothing, and a test proves it for each refusal by comparing the
     inspection seam's complete snapshot before and after.
   - **Tables per trigger.** The rules that share a trigger are the rows of one table for
     that trigger, each row `(name, guard, effect)`.
     - Guards are pure: a guard allots nothing and changes nothing.
     - Dispatch evaluates every guard of the table. If two hold, it raises
       `GuardOverlap` naming both. That is a specification finding, since the
       specification claims disjoint guards for Publish, the lapse and `Freed`: record
       it in `double/SPEC-NOTES.md` and **stop**.
     - Each table states what happens when no guard holds:

       | Trigger | When no guard holds |
       | --- | --- |
       | Publish, Pull, Ack, Nack, Renew, Release and the two joins | A refusal |
       | `PocketTouched` | Nothing happens |
       | `LeaseLapsed` | Nothing happens, under branch A (decision 0019) |
       | `Freed` and `Handed` | A defect |
       | The lapse's clock trigger and the two decays | Nothing happens: `tick()` offers only what is due |

     - An input whose only matching rule is still pending is itself pending. No test pins
       it as a refusal.
   - **One apply step.** `Queue` has one internal apply step. It runs a trigger's table,
     then the events that outcome emits (`Handed`, `Freed`, `PocketTouched`,
     `LeaseLapsed`) through their own tables, then the checker, once per outcome. It is
     the only place that allots from `next_position`, so `Queue` is the only module that
     allots positions; a row's effect asks it for one.
   - **The checker.** Its whole interface is `check(snapshot) -> violations`. The apply
     step calls it after the whole chained outcome, never part-way through, because the
     specification says the invariants hold after the whole of it, and raises naming each
     invariant that fails. Each of the 35 invariants has a positive control: a test that
     builds a snapshot by hand and shows that `check` names that invariant.
   - **The inspection seam.** Tests and the checker read state through one complete,
     immutable snapshot: the queue's `next_position`, the publishers, the consumers, the
     pockets with their status, position and consecutive expiries, the records in the
     specification's flat field names, the leases, and the superseded and dead-letter
     queues. Derived sets, such as a pocket's held records, are computed once, in the
     snapshot.
   - Standard library only: `unittest`, `dataclasses`, `enum`, `typing` and the like. No
     third-party package, so no decision-0007 note is needed. Python 3.14 from the pixi
     environment.
5. **Spec notes.** Every ticket appends to `double/SPEC-NOTES.md` wherever the
   specification was silent, ambiguous or forced an interpretation, and once for every
   `impl-choice` row. Each note gives an id (`N01`, `N02`, and so on), the ticket, the
   rule, invariant or comment it concerns, by name, what the specification says, what the
   double does, and the question it leaves for the maintainer. This file is the
   deliverable the maintainer uses to judge the specification.
6. **Worktree hygiene.**
   - `just python-double` runs `python -B`, so it writes no `__pycache__/`. `.gitignore`
     does not ignore one, and an untracked one fails `just check`.
   - The Python and Markdown files pass the existing hooks: editorconfig-checker, typos,
     markdownlint and lychee. `.editorconfig` defaults to `indent_size = 2`;
     `scripts/diagrams.py` is the four-space precedent, and it passes. Add a `[*.py]`
     entry only if editorconfig-checker objects, and say so in the hand-back.
   - Quoted evidence names the full commit id from `git rev-parse HEAD`. Until T37 lands,
     a seven-character prefix can fail typos (`_typos.toml`).
   - Green means three things: `just python-double` exits 0, `just check` ends with
     `All checks passed and the worktree is unchanged.`, and `git status --short` prints
     nothing after the commit (`tickets/CONVENTIONS.md` §11, "Green").
   - `just python-double` sits outside `just check`. Its owning page is
     `docs/reference/commands.md`, checked with `just check-docs`.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14: its phases, its lenses, its two rounds
at most, its scope guard and its record. The deltas below are shared by PD01 to PD06.

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

  `<id>` is `PD05`, `<n>` is the checkpoint's number in Steps, and `<lens>` is the
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
    table. At PD01's design checkpoint and at PD06, a third lens, `design`, reviews the
    object-oriented architecture. It asks whether each class has one job, whether a reader
    can find a rule's code from the rule's name, and whether the seams are the ones the
    shared rules fix.
  - The brief gives `git diff "$base"...HEAD` plus the uncommitted changes. Here `base`
    is this ticket's base branch, which is not `main` when the branches stack. The brief
    never gives the author's reasoning or the hand-back's conclusions.
- **Phase 2, triage.** Separate fresh subagents, neither the reviewers nor the lead,
  check each finding by running a command or reading the specification. Each finding is
  marked valid, with a fix written, or invalid, with the reason, in
  `ai_tmp/review/<id>-<n>/verdicts.md`. The subagents pass file paths, not pasted text.
- **Phase 3, apply.** The triage subagents apply the valid fixes one writer at a time. A
  fix edits only this ticket's Files touched; any other valid fix is handed back. Before
  the lead reruns `just python-double` or `just check`, it confirms that every writer has
  stopped and is running nothing in the tree.
- **Rounds and record.** As §14: if any high-severity finding was valid, a second round
  reviews the fix diff, and there are two rounds at most. Anything still open goes under
  Open points. The checkpoint authorises the review's model call (§11, "Network").

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/pachinko_double/` | shared, own lines | `tick()`, `expiry.py` with the lapse and the three expiry rules, the parked revival, and `decay.py` with the two decays |
| `double/tests/` | shared, own lines | The expiry, decay and both-endings tests; the pending lists in `test_coverage.py` |
| `double/SPEC-NOTES.md` | shared, own lines | This ticket's notes |
| `.scratch/python-double/PD05-clock-lapse-parking-decay.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD05's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD04. Record the base and its full commit id in the
   hand-back. Run `just initialize` there (network, authorised by this step). Run
   `just check` and `just python-double` on the untouched worktree, and quote each one's
   last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Read the sections named in Context, decision
   0019's lapse ordering (`docs/decisions/0019-service-runtime-tokio-and-http.md`), and
   the catalog rows this ticket owns.
3. **`tick()` and the lapse, red first.** Write S059, S065 and S006 first. A lease lapses
   exactly at `expires_at`, once per lease, and at the instant of grant when `lease_ttl`
   is 0. Then add `LeaseLapses` and the three expiry rules as rows of their tables, each
   fired in at least one test. Each expiry row's effect calls the lease-end module; this
   ticket adds no `Freed` row and no second copy of the end-of-lease steps. `tick()` runs
   the apply step once per due lapse and once per due decay, so the checker runs after
   each lapse's whole outcome and after each decay.
4. **Branch A, both orders.** Port S066 and S067. Show that an action processed after
   `expires_at` and before `tick()` succeeds and extends the lease, and that a `tick()`
   processed first lapses it so that the same action is then refused. Quote a scratch
   break: make an action lapse its own lease, show S066 failing, and revert it.
5. **Port the rest, red first.** Port parking, revival, decay and the "by release or
   lapse" scenarios, with both halves where the Rust loops over `ENDINGS`. Each new rule
   is a row of its trigger's table, fired in at least one test. Shrink
   `PENDING_SCENARIOS` and `PENDING_RULES` to empty. Leave the lists in place; PD06
   deletes them.
6. **Spec notes.** Answer the three questions in Context in the double, and record each.
   Add a note for S026's `oq:7` and for anything else the expiry rules left to
   interpretation.
7. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). The breakage lens asks these:
   - Could a test pass while an action lapsed its own lease?
   - Could a tick skip a lease that was due, or lapse one twice?
   - Could a "by release or lapse" test run only one half?
   - Could a parked pocket be revived by a stale publish?
8. **Commit and hand back.** Commit on `branch:`. Rerun Verification and quote it, headed
   by the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   and write the hand-back notes. **Stop** before pushing or opening a pull request: both
   need authorisation (§11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0, and its output shows the number of tests run.
- On exit, `PENDING_SCENARIOS` and `PENDING_RULES` are both empty.
- Every owned scenario has a tagged test. Each scenario whose Rust test loops over
  `ENDINGS` is tested by release and by lapse. Each of the seven rules has its row in its
  trigger's table, fired in at least one test.
- This ticket adds no `Freed` row and no second copy of the end-of-lease steps: each
  expiry row ends the lease through the lease-end module.
- The step 4 break is quoted with its output and exit status.
- No code under `double/pachinko_double/` reads the time except through the injected
  clock, and only `tick()` applies a temporal trigger.
- `double/SPEC-NOTES.md` answers the three questions in Context, and S026 is labelled
  `oq:7` in its test and its note.
- `just python-double` leaves no `__pycache__/` under `double/`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The `rg`
lines show that nothing in the package reads the wall clock, since the double ships only
the fake clock (PD01). The probe is the positive control: each of the pattern's four
alternatives matches one of its four lines.

```sh
set -eu
git rev-parse HEAD
just python-double
pattern='^\s*(import time|from time import)|datetime\.now|time\.time'
probe=$(printf 'import time\nfrom time import monotonic\nx = datetime.now()\ny = t.time.time()\n' | rg -c "$pattern")
test "$probe" -eq 4
if rg -n "$pattern" double/pachinko_double; then exit 1; else test "$?" -eq 1; fi
sources=$(find double -name '*.py' -print)
test -n "$sources"                      # positive control: the find sees the tree
caches=$(find double -name __pycache__ -print)
test -z "$caches"
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `1296beaff69fdfcf649de15e150859c6741bffc7`, the commit that added the PD07
  follow-up ticket. It was made after PD04's record commit
  `50d7c291ae27fe5181ec10c17658ecc7e27d27f3`.
- Work commit: `15fffd6fc14816413fdf15c432c0be5a3ab52204`.
- At the maintainer's direction the whole PD01 to PD06 series lands on this one branch
  in this one worktree, as PD01's hand-back section 1 records. That departs from
  AGENTS.md's one-ticket-per-worktree rule and from step 1's "create the worktree on
  `branch:`". `ticket/pd05-clock-lapse-parking-decay` was not created.
- `just initialize` was not run. The series runs in one worktree, whose PD01 baseline
  was green without it, and PD05's baseline is quoted from PD04.

### 2. Baseline

Quoted from PD04, not rerun (`ai_tmp/pd/PD05/baseline.txt`).

- `just python-double` at `252e98f7f5bee89c7b36554d12c0fceb26eb72d6` printed
  `Ran 228 tests` and `OK` (PD04's Verification).
- `just check` on the tree committed as `1296beaff69fdfcf649de15e150859c6741bffc7` ended
  `All checks passed and the worktree is unchanged.` (exit 0). It ran just before that
  commit, which added only `.scratch/python-double/PD07-inspection-contract.md` and its
  README row, after PD04's record commit.

### 3. What landed

Rows per table. Each row fires in at least one test through `Queue`, and the coverage
test's fired-rule check passes. No table raised a `GuardOverlap`.

| Table | Rows added | Rows now |
| --- | --- | --- |
| `Lease.expires_at` (new) | `LeaseLapses` | 1 |
| `LeaseLapsed` (new) | `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket` | 3 |
| `PocketTouched` | `ReviveParkedPocket` | 2 |
| `SupersededRecord.expires_at` (new) | `SupersededRecordDecays` | 1 |
| `DeadLetter.expires_at` (new) | `DeadLetterDecays` | 1 |

That is the seven rules the ticket owns. This ticket adds no `Freed` row and no second
copy of the end-of-lease steps. Each expiry row calls
`lease_end.end_lease(view, outcome, lease, LeaseStatus.EXPIRED, view.now)` and then writes
only the pocket.

How `tick()` works:

- It reads the clock once, and hands that one `now` to every outcome through `_Instant`,
  a one-value clock.
- It applies each due lapse as its own `_apply`, in `expires_at` order and then grant
  order. It then re-reads the state and applies each due superseded record, then each due
  dead letter.
- Its `TickReport` holds the leases that went from active to expired and the entries the
  decay rows answered.
- Only `tick()` applies a temporal trigger. No action calls it.

Files under `double/`:

- `pachinko_double/expiry.py` (new): `LeaseExpiresAt`, `LeaseLapsed`, `due_lapses`, the
  `Lease.expires_at` table and the `LeaseLapsed` table.
- `pachinko_double/decay.py` (new): the two trigger classes, `due_superseded` and
  `due_dead_letters`, and the `SupersededRecord.expires_at` and `DeadLetter.expires_at`
  tables.
- `pachinko_double/queue.py`: `TickReport`, `_Instant` and `Queue.tick()`; `_apply` gained
  a keyword `clock=` (`event_tables` is unchanged); `LeaseLapsed` is routed in
  `EVENT_TABLES`.
- `pachinko_double/publish.py`: the `ReviveParkedPocket` row in `POCKET_TOUCHED`, and the
  docstring.
- `pachinko_double/__init__.py`: exports `decay`, `expiry` and `TickReport`.
- `tests/test_expiry.py` (new): the lapse, branch A, the expiry rules, the counts,
  parking, S075 and the hand-built table tests.
- `tests/test_decay.py` (new): S002, S007, S073, S074 and the tick tests.
- `tests/test_endings.py` (new): the both-endings scenarios and S076.
- `tests/test_coverage.py`: both pending lists are now `frozenset()`, kept in place with a
  comment for PD06.
- `SPEC-NOTES.md`: N33 to N38.

Outside `double/`, only this ticket's hand-back and `status:` and the README's PD05
Status cell are written, as Files touched says.

Unchanged, confirmed by `git diff --quiet`: `lease_end.py` and `settle.py`.

Pending lists:

- `PENDING_SCENARIOS` held 34 ids and now holds none.
- `PENDING_RULES` held 7 rules and now holds none.

No scenario moved under shared rule 2.

The suite now runs 274 tests, up from 228.

### 4. Red runs and probes

#### Red runs

Each red run went through `just python-double`, was saved with its exit status, and was
followed by the change that turned it green. Files are in `ai_tmp/pd/PD05/`. The tree for
each is `1296beaff69fdfcf649de15e150859c6741bffc7` plus the uncommitted PD05 files. In
every run after the first, the coverage module's tag-run and test-def checks also fail,
on the new tags and test defs.

| File | Red before | Scenarios | Key output line | Exit |
| --- | --- | --- | --- | --- |
| `red-S006-S059-S065.txt` | No `tick()` and no `TickReport` | the whole suite, by an `ImportError` | `FAILED (errors=1)`; 1 test | 1 |
| `red-S006-S059-S065-no-expiry-row.txt` | `tick()` and `LeaseLapses` in, the `LeaseLapsed` table empty (a temporary edit, restored, SHA-256 matched) | S006, S059, S065 | `FAILED (failures=7)`; 231 tests | 1 |
| `red-expiry-before-idle-park-revive-rows.txt` | Only `LeaseExpiryKeepsPocketAtFront` in the `LeaseLapsed` table | S024, S025, S026, S034, S056, S061, S062, S063, S075, S077, S092 | `FAILED (failures=13)`; 251 tests | 1 |
| `red-expiry-before-park-revive-rows.txt` | `LeaseExpiryIdlesEmptyPocket` added; the park row and `ReviveParkedPocket` absent | the same, less S061 | `FAILED (failures=12)`; 251 tests | 1 |
| `red-expiry-before-revive-row.txt` | `LeaseExpiryParksPocket` added; `ReviveParkedPocket` absent | S024, S062 | `FAILED (failures=4)`; 251 tests | 1 |
| `red-S002-S007-S073-S074.txt` | No decay table, and `tick()` applies no decay | S002, S007, S073, S074 and four untagged tick tests | `FAILED (failures=10)`; 260 tests | 1 |

Notes on the table:

- The scenario column is the builder's reading of the failing test names in each file.
- The `ImportError` in the first file is `cannot import name 'TickReport' from
  'pachinko_double.queue'`, which stops discovery, so it names no scenario.
- The third file's failures also include untagged tests: the idle-pocket test, the
  threshold test, the resets and the checker-order tests.

#### Scenarios with their own red run

9 of the 34 owned scenarios went red for their own reason: S006, S059, S061, S062, S063,
S024, S007, S073 and S074.

#### Scenarios that could not be red on their own

25 of the 34. Nine and 25 make the 34 owned, with no id in both lists. Each was written
after the row it tests, or was red only in a setup step, or asserts what holds by
construction. A mutant probe kills each one (probe files below).

| Scenario | Why it could not be red alone | Killed by probe |
| --- | --- | --- |
| S065 | Failed only at the `lapse()` helper's setup ("0 != 1 : one lease lapsed"), not at "a lease lapsed twice" | 05 |
| S058 | Asserts a non-event | 05 |
| S060 | Passed once keep-at-front landed | 07, 08 |
| S048 | Passed once keep-at-front landed | 27, 06 |
| S064 | Passed once keep-at-front landed | 24 |
| S069 | Passed once keep-at-front landed | 23 |
| S066 (both halves) | Holds branch A by construction | 01, 23 |
| S067 (both halves) | Holds branch A by construction | 01, 23 |
| S025 | Red only in its `parked()` setup | 19 |
| S026 | Red only in its `parked()` setup | 17 |
| S034 | Red only in its `parked()` setup | 18 |
| S092 | Red only in its `parked()` setup | 12 |
| S075 | Red only through parking, in its setup | 28, 18 |
| S056 | Red only at the parking step; the resets predate PD05 | 26 |
| S077 | Red only at the parking step; the resets predate PD05 | 25 |
| S002 | The Rust's config-only assertions already held; only the port's behavioural extension was red | 29, 15 |
| S076 | Needs no PD05 operation | 22 |
| S078 | Written after the rows; its release half predates PD05 | 20, 21, 10 |
| S079 | As S078 | 20, 21, 10 |
| S080 | As S078 | 20, 21, 10 |
| S081 | As S078 | 20, 21, 10 |
| S087 | As S078 | 20, 21, 10, 30 |
| S088 | As S078 | 20, 21, 10 |
| S089 | Written after the rows | 09, 10 |
| S093 | Written after the rows | 09, 10, 27 |

Probes 20 and 21 each kill only their own half of the "by release or lapse" tests: 20
breaks the lapse half and 21 the release half.

#### Step 4's scratch break

Probe 01 is the scratch break step 4 asks for. Every action calls `self.tick()` first, so
an action processed after `expires_at` lapses its own lease. It is a `sed` on
`double/pachinko_double/queue.py`, for example:

```text
<         return self._apply(pull.PULL, pull.Pull(consumer_id, limit))
>         self.tick(); return self._apply(pull.PULL, pull.Pull(consumer_id, limit))
```

Run on the final tree, it failed S066 (both halves), S067 (both halves) and S085's
no-clock-read test, with the key line `FAILED (failures=4, errors=5)` at 274 tests and
exit status 1. The file ends with `# reverted: sha256 before
e83bf8bd84eb08430c2f45e15340ea0c374691a1ddad06f87b8fd7ba9b7ad2b4 after
e83bf8bd84eb08430c2f45e15340ea0c374691a1ddad06f87b8fd7ba9b7ad2b4`. A first run at 235
tests, before the later tests existed, had the same S066 and S067 failures; the final-tree
rerun overwrote it.

#### Probes

Each probe ran through `just python-double` on the tree at
`1296beaff69fdfcf649de15e150859c6741bffc7` plus the uncommitted PD05 files, then was
reverted. `probe.sh` copies the file first. All 32 probe files end with a matching SHA-256
before and after. `probes.sh` drives probes 02 to 30; 01 and 31 ran on their own. All
files are in `ai_tmp/pd/PD05/`, named `probe-NN-<name>.txt`. Every numbered probe exits 1.

| Probe | Breaks | Key output line | Exit |
| --- | --- | --- | --- |
| 01 | An action lapses its own lease (step 4's break) | `FAILED (failures=4, errors=5)` | 1 |
| 02 | A lapse is due at `<`, not `<=` | `FAILED (failures=23)` | 1 |
| 03 | A lapse is one ms early | `FAILED (failures=4)` | 1 |
| 04 | A tick skips the later due leases | `FAILED (failures=4)` | 1 |
| 05 | An ended lease lapses again | `FAILED (failures=9, errors=14)` | 1 |
| 06 | A lapse ends the lease released | `FAILED (failures=44)` | 1 |
| 07 | A lapse does not raise the count | `FAILED (failures=26)` | 1 |
| 08 | Keep-at-front allots a position | `FAILED (failures=21, errors=1)` | 1 |
| 09 | Parking is one lapse late | `FAILED (failures=18, errors=1)` | 1 |
| 10 | `ended_at` is taken from `expires_at` | `FAILED (failures=12)` | 1 |
| 11 | The clock is re-read per lapse | `FAILED (failures=2)` | 1 |
| 12 | A stale publish emits `PocketTouched` | `FAILED (failures=3)` | 1 |
| 13 | The revive keeps the count | `FAILED (failures=4)` | 1 |
| 14 | The revive keeps the position | `FAILED (failures=4)` | 1 |
| 15 | A decay is due at `<` | `FAILED (failures=9)` | 1 |
| 16 | The decays are read before the lapses | `FAILED (failures=2)` | 1 |
| 17 | A tick touches every pocket (a cooldown) | `FAILED (failures=3)` | 1 |
| 18 | A pull claims a parked pocket | `FAILED (failures=2, errors=4)` | 1 |
| 19 | An equal rank is taken as stale | `FAILED (failures=17)` | 1 |
| 20 | A lapse emits no `Freed` | `FAILED (failures=10, errors=36)` | 1 |
| 21 | A release emits no `Freed` | `FAILED (failures=4, errors=32)` | 1 |
| 22 | The view shows the newest version | `FAILED (failures=5)` | 1 |
| 23 | A lapsed lease is not fenced | `FAILED (failures=4, errors=5)` | 1 |
| 24 | A nack keeps the count | `FAILED (failures=3, errors=2)` | 1 |
| 25 | An ack keeps the count | `FAILED (failures=3, errors=2)` | 1 |
| 26 | A release keeps the count | `FAILED (failures=4)` | 1 |
| 27 | A nacked deferred version keeps the held position | `FAILED (failures=10)` | 1 |
| 28 | The revive does not advance `next_position` | `FAILED (failures=2, errors=3)` | 1 |
| 29 | The default expiry limit is 4 | `FAILED (failures=13)` | 1 |
| 30 | The idle row queues the pocket | `FAILED (failures=5)` | 1 |
| 31 | A nack alone ticks first | `FAILED (failures=3, errors=2)` | 1 |
| 31a | The probe 31 mutation, run before its test existed | `FAILED (failures=3, errors=1)` | 1 |

Which tests kill which breakages, from the builder's `summary.md`:

- Probes 04 and 11 are killed only by the tick tests.
- Probe 16 is killed only by the zero-TTL test.
- Probe 12 is killed only by S092, and probe 17 only by S026.
- Probes 20 and 21 each kill only their own half, as above.

The four breakage-lens questions of step 7:

- A test passing while an action lapsed its own lease: probe 01.
- A tick skipping a due lease, or lapsing one twice: probes 02, 04 and 05.
- A "by release or lapse" test running one half only: probes 20 and 21.
- A stale publish reviving a parked pocket: probe 12.

How probe 31a was handled. Probe 31 mutates `nack` alone to tick first. Run before any test
pinned that order, as `probe-31a-...txt`, it exited 1 but killed only S007 and S085
incidentally. The builder then added the untagged
`test_a_nack_after_expires_at_but_before_the_lapse_succeeds` to `BranchATest`, in both
orders, and reran the probe as `probe-31-nack-lapses-its-own-lease.txt`, which fails that
test. The 31a file is kept as the record.

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `1296beaff69fdfcf649de15e150859c6741bffc7` plus uncommitted work (steps 2 to 6).
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command.
  One Claude (sonnet) runner subagent per lens. Codex cannot write under read-only, so each
  findings file is Codex's last message and its first line says `Source: last-message`.
  Codex could not run `just check` in its sandbox.
- Triage and apply: one fresh opus subagent. It ran mutants first on copies outside the
  worktree, then in the tree as the only writer.
- Findings: 0 high, 1 medium, 3 low (two duplicates).

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| B1 | medium | valid | Both-ending coverage could vanish: `ENDINGS = ()`, `(RELEASE,)` and `(RELEASE, RELEASE)` each still ran 274 tests, OK | `EndingsTest` records each ended lease's status and, after each of the six loops (S078 to S081, S087, S088), asserts `assertCountEqual` against `[RELEASED, EXPIRED]`; in-tree mutants 1 to 4 exit 1, reverts SHA-256 matched |
| C2 | low | valid (duplicate of B1's docstring half) | `test_endings.py` claimed a loop that ran one ending twice would fail, which the release-twice mutant disproves | The module docstring describes the new check |
| B2 | low | valid | N38's Concerns named no specification anchor | N38 names the Pocket and Record comments, the four position invariants, `ReleaseRequeuesPocket`, the `Lease` terminal transitions, `OldestSlotNewestPayload`, `PullWithinLease`'s `Batch`, `DeferBehindHeldVersion` and the Publishing surface, `Queue.next_position` and `Position` |
| C1 | low | valid (duplicate of B2) | The same gap | The same fix |

- Mutants run in the tree through `just python-double` (all exit 1, 274 tests):

  | Mutation of `double/tests/test_endings.py` | Key output line |
  | --- | --- |
  | `ENDINGS = ()` | `FAILED (failures=8)` |
  | `ENDINGS = (Ending.RELEASE,)` | `FAILED (failures=8)` |
  | `ENDINGS = (Ending.RELEASE, Ending.RELEASE)` | `FAILED (failures=8)` |
  | S081's loop alone over `ENDINGS[:1]` | `FAILED (failures=3)` |

  Each revert matched the fixed file's SHA-256
  `89f415ef23fbc24e2aaa17dd7b2fb051e44befa79456a217f0b1d95c94087f99`.
- Evidence: `ai_tmp/review/PD05-1/verdicts.md`, `findings-conformance.md`,
  `findings-breakage.md`, `phase2-mutants.txt` and `apply-final.txt`.
- Re-run after apply: `just python-double` ran 274 tests, OK, exit 0; `just lint` exit 0;
  no `__pycache__`; the wall-clock `rg` found nothing (exit 1).
- Second round: none, because no high-severity finding was valid.
- Handed back: nothing outside Files touched.

### 6. Spec notes added

| Id | Note |
| --- | --- |
| N33 | `tick()` is the double's only clock and reports what it did. The specification names no process that notices a due trigger and no surface that reports one (Context's question 3). |
| N34 | One tick lapses in `expires_at` order, then decays in queue order. The order changes no pocket or record and shows only in the report and in the order of the entries two lapses create (question 1; extends N29). |
| N35 | An entry a lapse creates with a zero TTL decays in the same tick, so nothing is due once a tick returns (question 2). |
| N36 | S026, labelled `oq:7`: only a publish revives a parked pocket, as the rules stand. Open question 7 is left open. |
| N37 | The expiry rules' answer when no guard holds is reached only by hand-built dispatch, and the decay guards read from the trigger binding. |
| N38 | PD05's ports of the Rust's inspection shapes, under N04, N13, N24, N31 and N32. It carries the maintainer's ruling below. |

S026 is named `oq:7` in its test comment, which cites open question 7 and N36, and in
N36's title and Concerns. Neither resolves the question. N36 says the double adds no
cooldown and no operator action, and that if anything else may revive a parked pocket it
needs a rule and S026 changes with it.

### 7. Departures and rulings

#### Maintainer's ruling on N38

Recorded in `ai_tmp/pd/PD05/ruling-N38.txt`. On 2026-11-19, through AskUserQuestion, the
maintainer chose "Accept, add to PD07 (Recommended)" for N38.

- Three ports follow the Rust's inspection shape rather than behaviour:
  - S066's release half asserts in the Rust that the released lease leaves the registry.
    The double keeps it, `released` with its `ended_at`, and asserts that the pocket has
    no active lease (N24).
  - S093 lists the pocket's records in the Rust's snapshot order. The double compares
    them by key (N31).
  - S075 asserts in the Rust that the stored pocket order is the queued pockets sorted
    by position. The double stores no pocket order, so it asserts the rest of that step.
- The ports stand as the Rust's inspection shape. PD07's scope gains S066, S093 and
  S075's stored pocket order.
- PD07's ticket is updated in its own commit after this ticket's record. This ticket
  does not edit it.

#### Other departures

- The series lands on one branch in one worktree (section 1).
- Step 1 asks for `just initialize` and for `just check` and `just python-double` on the
  untouched worktree. Neither was rerun. The baseline is quoted from PD04 (section 2).
- The builder edited three stale forward references in lines earlier tickets wrote. Each
  said PD05's work was still to come, and each was stale once it landed:
  - `double/pachinko_double/queue.py`: the comment above `EVENT_TABLES` ("LeaseLapsed's
    table follows in PD05"), PD01's.
  - `double/pachinko_double/publish.py`: the docstring sentence "PD05 adds
    `ReviveParkedPocket` beside it", PD03's.
  - `double/tests/test_settle.py`: the docstring of `ResetsOnHandBuiltStatesTest`
    ("Before PD05 lands the lapse..."), PD03's.

  This reads "own lines" as allowing it, as PD04 did. The three edits revert cleanly if
  the maintainer reads it otherwise.
- The branch-A nack test is untagged: `test_a_nack_after_expires_at_but_before_the_lapse_succeeds`,
  in `BranchATest`. It was added after probe 31a showed that no tagged test pinned branch A
  for the nack alone. The same class holds the ack, release, renew and pull tests in both orders.
- A tick is not atomic as a whole. It commits each outcome as it goes, so a later
  `InvariantsBroken` would leave the earlier lapses committed. Each lapse and each decay
  is checked on its own, as step 3 asks.
- The decay guards read from the trigger binding. The specification gives the two decay
  rules no `requires`, so each guard holds while the entry its trigger binds is still in
  its queue. This is the double's reading (N37).
- S002 extends the Rust's config-only assertions with the port's behavioural check, as
  PD02's S001 port did.
- The checker-count test uses `mock.patch(wraps=check)`.
- S075's step asserts what the Rust's `one_strict_order` asserts (`fixture.rs:766-781`):
  the four position invariants from the snapshot, distinct record positions per pocket,
  then `check`. Its `order_is_the_queued_pockets` has no counterpart, since the double
  stores no pocket order (N38).
- Probes 04, 11, 12, 16, 17, 20 and 21 are each killed only by the tests named in
  section 4. Probe 31 first survived its own lens as probe 31a records.
- S064 and S077 are `wire` rows. Only their library halves are ported.
- No behaviour was taken from `crates/libpachinko/src/`; the Rust tests were read for
  their setup, steps and assertions.
- Codex cannot write files under read-only, so each findings file is Codex's last
  message (section 5).
- `just lint` skipped markdownlint in its second pass ("no files to check") but passed it
  over all files in its first. `just check` is the gate that covers `double/SPEC-NOTES.md`.

### 8. Verification

The Verification block ran at `15fffd6fc14816413fdf15c432c0be5a3ab52204`, the work commit, as one `sh` script under
`set -eu`. It exited 0 (`exit=0`). The opening and closing lines of its output:

```text
15fffd6fc14816413fdf15c432c0be5a3ab52204
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
..................................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 274 tests in 0.126s

OK
…  (the remaining output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

## Open points

- Should the specification, or a surface, name how the temporal triggers are processed and
  what that processing reports? Today the double's `tick()` and its `TickReport` are the
  implementation's choice (N33).
- Should the specification order what several lapses in one tick create, or say that the
  order is not observable? This is N29's question again, for lapses (N34).
- Is "nothing is due once a tick returns" intended? An entry with a zero TTL that a lapse
  creates decays in the tick that made it; the other reading also satisfies every rule
  (N35).
- Can anything other than a publish revive a parked pocket, such as an operator action or
  a cooldown? This is open question 7, and S026 ports it as the rules stand (N36).
- Are the expiry rules' `lease.status = active` and `lease.expires_at <= now` guards in
  their own right, or restatements of what `LeaseLapses` established in the same chain?
  The same question applies to the decays, which have no `requires` (N37, as N14 and N26).
- Should a tick be atomic as a whole? The double commits each lapse and each decay as it
  goes, so a later invariant failure leaves the earlier outcomes committed.
- The three stale forward references edited in PD01's and PD03's lines (section 7) are the
  builder's reading of "own lines". The maintainer may want a different reading.
- S066, S075 and S093 wait on PD07's ruling on the inspection contract. The maintainer
  accepted the ports and added them to PD07's scope (N38).
