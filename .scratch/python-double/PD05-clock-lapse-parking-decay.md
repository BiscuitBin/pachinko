---
id: PD05
title: "Python double: the clock, with lapse, parking and decay"
status: todo
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
  everything due.
- **Branch A** (decision 0019). The lapse and the actions are one serialized stream. A
  pull, ack, nack, renew or release processed after `expires_at` but before the lapse
  succeeds, as long as its other guards hold. A pull, ack, nack or renew moves
  `expires_at` on. A release ends the lease. The lapse then stops at its guards, unless
  the new `expires_at` has also passed, as it always has when `lease_ttl` is 0 (S006,
  S066, S067). So an action never lapses a lease itself, and only `tick()` does.
- **The three expiry rules.** Each needs the lease active, `expires_at <= now` and the
  pocket leased. Each ends the lease `expired` at `now`, sends every held record `Freed`
  and raises `consecutive_expiries` by one.
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
   - The specification's rules, `^rule (\w+) \{`, are each covered by a `# rule: <Name>`
     comment at the code that applies the rule, under `double/pachinko_double/`.
   - The specification's invariants, `^\s*invariant (\w+) \{`, are each covered by an
     entry in the checker's registry and by a positive control tagged
     `# invariant-control: <Name>`. Fifteen of the thirty-five sit indented inside an
     entity, so a pattern anchored at column 0 finds only twenty.
   - It asserts the three counts it parsed, 93, 32 and 35, so a pattern that silently
     matches less fails. A specification change that moves a count updates the constant
     and adds a spec note. A tag that names an id or a name the sources do not hold fails.
   - `PENDING_SCENARIOS` and `PENDING_RULES` hold what is not covered yet. An entry that is
     both pending and covered fails, so each ticket shrinks the lists by exactly what it
     lands. PD06 deletes both lists.
4. **Design constraints.** PD01 proposes the class layout and reviews it. These are fixed:
   - The clock and lease ids are injected: a `Clock` protocol with a fake whose time a test
     sets and whose reads it counts, and a lease-id source with a deterministic fake. This
     follows the house rule that effects sit behind seams (decisions 0003 and 0008). The
     double reads no wall clock and draws no randomness.
   - A refusal changes nothing, and a test proves it for each refusal by comparing the
     inspection seam's snapshot before and after.
   - The invariant checker runs after every public operation, after the whole chained
     outcome (`Handed`, `Freed`, `PocketTouched`), because the specification says the
     invariants hold after the whole of it. It raises naming each invariant that fails.
     Each of the 35 invariants has a positive control: a test that corrupts the state
     directly and shows that the checker names that invariant.
   - Tests read state through an inspection seam: each pocket's status, position and
     consecutive expiries; each record's state, positions and held fields; the leases; and
     the superseded and dead-letter queues.
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
| `double/pachinko_double/` | shared, own lines | `tick()`, the lapse, the three expiry rules, the parked revival and the two decays |
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
   is 0. Then land `LeaseLapses` and the three expiry rules, each with its `# rule:`
   comment. The checker runs after each lapse's whole outcome, and again after the tick.
4. **Branch A, both orders.** Port S066 and S067. Show that an action processed after
   `expires_at` and before `tick()` succeeds and extends the lease, and that a `tick()`
   processed first lapses it so that the same action is then refused. Quote a scratch
   break: make an action lapse its own lease, show S066 failing, and revert it.
5. **Port the rest, red first.** Port parking, revival, decay and the "by release or
   lapse" scenarios, with both halves where the Rust loops over `ENDINGS`. Shrink
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
  `ENDINGS` is tested by release and by lapse. Each of the seven rules has its `# rule:`
  comment.
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

Written at the end. They record the base and its full commit id, the baseline, the step 4
break, the checkpoint's record, the spec notes added, and Verification quoted under its
commit.

## Open points

None yet.
