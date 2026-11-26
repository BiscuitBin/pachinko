---
id: PD08
title: "Python double: one read context for every rule"
status: done
depends_on: [PD06]
parallel_with: [PD07]
branch: ticket/pd08-one-read-context
estimated_size: S
---

# PD08: Python double: one read context for every rule

## Context

An architecture review of the finished double, two adversarial passes by Codex (Astra)
checked against the code, found that a rule reads the specification's `config` through
two routes. The review's own files are scratch, so this section restates the evidence.
The line numbers are those of commit `76e51c1442fffbd22599ff186ea53afbb31857eb`.

- **Effects read `outcome.config`.** Ten sites under `double/pachinko_double/` do:

  | File | Function | Line | Reads |
  | --- | --- | --- | --- |
  | `pull.py` | `_batch` | 96 | `max_pull_batch` |
  | `pull.py` | `_pull_within_lease` | 133 | `lease_ttl` |
  | `pull.py` | `_pull_claims_front_pocket` | 173 | `lease_ttl` |
  | `settle.py` | `_extend` | 135 | `lease_ttl` |
  | `settle.py` | `_renew` | 286 | `lease_ttl` |
  | `publish.py` | `_supersede` | 160 | `superseded_ttl` |
  | `publish.py` | `_drop_stale_publish` | 234 | `superseded_ttl` |
  | `lease_end.py` | `_supersede_unsettled_held_version` | 203 | `superseded_ttl` |
  | `lease_end.py` | `_dead_letter_nacked_record` | 265 | `dead_letter_ttl` |
  | `lease_end.py` | `_dead_letter_nacked_version_queue_deferred` | 297 | `dead_letter_ttl` |

- **The expiry guard reads `view.state.config`.** `_at_threshold` in `expiry.py`, line
  109, reads `max_consecutive_expiries` from the snapshot, as the checker does
  (`invariants.py` reads `s.config`).

`Queue` keeps the two in step. `Queue.__init__` holds the `Config` it was given as
`_config` and a copy of it as `_config_values`. `Queue._apply` gives the first to the
`Outcome` and the second to every snapshot. A caller of the rule interface other than
`Queue` has to keep them in step itself, and the single-rule tests do not:

- `test_the_claim_caps_the_batch_at_max_pull_batch` in `double/tests/test_pull.py` passes
  `Config(max_pull_batch=2)` to `run_pull`, which gives it to the `RecordingOutcome`. The
  snapshot from `scrambled()` keeps the default, 100. The test passes because the effect
  reads the recorder.
- `freed` in `double/tests/test_lease_end.py` builds `RecordingOutcome(CONFIG)`, where
  `CONFIG` sets `superseded_ttl` to 5,000 and `dead_letter_ttl` to 7,000. The snapshot
  from `state()` keeps the defaults, twelve hours each.
  `test_a_freed_naming_a_lease_the_state_lacks_is_a_defect_no_row_answers` builds a
  second `RecordingOutcome(CONFIG)`.

These are the only three places a test gives a config to the recorder. The tests in
`test_settle.py`, `test_deferral.py` and `test_expiry.py` build `RecordingOutcome()` with
no argument.

`Outcome` also offers a second route to `now`. `View.now` is the one every rule reads.
`Outcome.now` is a public property over the same reader, no rule reads it, and one
scratch effect does: `open_seeing` in `test_it_reads_the_clock_once_per_outcome`
(`double/tests/test_queue.py`).

None of this is a failure through `Queue`. It becomes one when a specification change
moves a config-dependent condition between a guard and an effect, because the tests
above would then set a value the rule no longer reads. The maintainer chose this
candidate from the review and asked for it as one ticket.

## Goal

A guard and an effect read state, config and `now` from the `View` alone. `Outcome` only
writes: it creates, replaces and removes entities, emits events, allots a position, draws
a lease id and takes the trigger's answer.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The builder does not reopen these.

- **`view.config` is the one route.** `View` gains one read-only property, `config`,
  which hands back its snapshot's config. The specification's `config.x` is then
  `view.config.x` at every site, the expiry guard included. No site keeps
  `view.state.config`, and no site binds the config to a local name first.
- **What a rule reads is `ConfigValues`**, the form the snapshot holds. `Config` still
  validates when it is made, and `Queue.__init__` still takes a `Config`.
- **`Outcome` loses `config` and `now`.** Its constructor takes the working state and the
  lease-id source, and nothing else.
- **The lazy clock reader lives in the apply step.** `Queue._apply` still makes one
  `_Now` per outcome and hands that one reader to every `View` of the outcome, through
  `Queue._dispatch`. So an outcome still reads the clock at most once, and an outcome
  whose rules never read `now` still reads none (S085, spec note N28).
- **`Queue._config` goes.** Only the `Outcome` read it. `Queue` keeps `_config_values`.
- **The recorder holds no config.** `RecordingOutcome` and `run_pull` in
  `double/tests/test_pull.py` lose their `config` parameter. A single-rule test sets a
  config in the snapshot it builds, with `fixtures.config(...)`, and nowhere else.
- **One checkpoint detail of PD01 is revised.** PD01's hand-back lists `now` and `config`
  among `Outcome`'s members, under "Departures from the ticket's design table". This
  ticket moves both to the `View`. It reopens none of the four points under "Design fixed
  before PD01" in `.scratch/python-double/README.md`, and neither decision 0002 nor
  decision 0003. PD01's hand-back is a record and is not edited.

## Non-goals

- No change under `docs/specs/` or `crates/`.
- No behaviour change through `Queue`. Every scenario test passes as it stands.
- No other candidate from the review. In particular, no shared setup for the single-rule
  tests and no change to how `Config` and `ConfigValues` divide the five constraints.
- No new test helper beyond what moving the config needs.
- No spec note. The change interprets nothing in the specification.

## Shared rules

Shared rule 1 of PD01 to PD06 binds this ticket word for word:

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

Shared rules 2 to 6 bind this ticket as `.scratch/python-double/PD06-close-out.md`
writes them under "Shared rules (PD01 to PD06)". They are not copied here. Rule 3's
coverage test stays strict, and its three counts stay 93, 32 and 35.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/python-double/PD06-close-out.md` writes under "Adversarial review". They are
not copied here. Read them with these substitutions:

- `<id>` is `PD08`, and `<n>` is 1: this ticket has one checkpoint, at step 8.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names `docs/specs/pachinko.allium`, this ticket's path and its
  "Decisions fixed in this ticket".
- `base` is the commit step 1 records.
- §14's rounds stand: a second round reviews the fix diff only if a high-severity
  finding was valid.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/pachinko_double/inspection.py` | shared, own lines | `View.config`, and `View`'s docstring |
| `double/pachinko_double/queue.py` | shared, own lines | `Outcome` without `config` and `now`; `_apply` and `_dispatch` hand the clock reader to each view; `Queue._config` removed; the docstrings |
| `double/pachinko_double/pull.py`, `settle.py`, `publish.py`, `lease_end.py` | shared, own lines | The ten effect sites read `view.config` |
| `double/pachinko_double/expiry.py` | shared, own lines | `_at_threshold` reads `view.config` |
| `double/tests/test_pull.py` | shared, own lines | `RecordingOutcome` and `run_pull` without a config; the capped-batch test |
| `double/tests/test_lease_end.py` | shared, own lines | The TTLs in `state()`'s snapshot; `CONFIG` as the recorder's argument removed |
| `double/tests/test_queue.py` | shared, own lines | The clock-once test's scratch effect; the pin of step 5 |
| `double/README.md` | shared, own lines | The `config` row and the sentence on what a guard and an effect read, only where the wording no longer matches |
| `double/SPEC-NOTES.md` | shared, own lines | Only a review fix. No note describes the `Outcome` seam today |
| `.scratch/python-double/PD08-one-read-context.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD08's Status cell |

## Steps

Each red step quotes the failing test's name and its assertion message, headed by the
commit and "with uncommitted changes".

1. **Worktree and baseline.** Create the worktree on `branch:` from the base the
   maintainer names, which holds PD06. PD01 to PD06 landed on `build-python-double` in one
   worktree at the maintainer's direction, so ask which applies here. Record the base and
   its full commit id in the hand-back. A new worktree runs `just initialize` (network,
   authorised by this step). Run `just check` and `just python-double` on the untouched
   worktree, and quote each one's last line, headed by its commit, as the baseline.
2. **Red: the capped batch.** In `test_the_claim_caps_the_batch_at_max_pull_batch`, put
   `max_pull_batch=2` in the snapshot and pass no config to `run_pull`. It fails, because
   `_batch` reads the recorder's default of 100 and answers three records.
3. **Red: the two TTLs.** In `double/tests/test_lease_end.py`, put `SUPERSEDED_TTL` and
   `DEAD_LETTER_TTL` in the snapshot `state()` builds, and build both recorders with no
   argument. The four tests that assert an entry stamped with a TTL fail, because the
   effects read the recorder's twelve-hour defaults.
4. **Green.** Add `View.config`. Switch the ten effect sites and `_at_threshold` to
   `view.config`. Steps 2 and 3 pass, and so does everything else.
5. **Red: the pin.** Add a test to `double/tests/test_queue.py` that drives `Queue._apply`
   with a scratch effect and asserts that the outcome the apply step made has neither a
   `config` nor a `now` attribute. Write the names as strings for `hasattr`, so that the
   test does not trip the probe in Verification. It fails today on both.
6. **Remove the second routes.** Take `config` and `now` off `Outcome`, with the clock
   reader moved as fixed above. Remove `Queue._config`. Remove the `config` parameter from
   `RecordingOutcome` and `run_pull`. Make `open_seeing` read `view.now`. Step 5 passes.
7. **Words.** Update what names the two seams: `View`'s and `Outcome`'s docstrings,
   `queue.py`'s module docstring if it no longer matches, and in `double/README.md` the
   `config` row of "How the package follows the specification" and the sentence after
   that table. Run `just check-docs`.
8. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Does any guard or effect still reach the config or `now` by a route other than the
     view?
   - Does any single-rule test still set a config where the rule does not read it?
   - Could step 2, 3 or 5 have failed for a reason other than the one it states?
   - Did the diff change any test under a `# scenario:` tag, or the walkthrough?
   - Does an outcome still read the clock at most once, and a refused mark none?
   - Would the pin fail if `Outcome` regained either attribute?
   - Could a probe in Verification print nothing while its claim is false?
9. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification and quote
   it, headed by the full commit id. Set `status: done` here and in
   `.scratch/python-double/README.md`, write the hand-back notes, and commit the record.
   **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- No rule and no test reads the config or `now` through an outcome:
  `rg -n 'outcome\.(config|now)' double` prints nothing.
- Every config read under `double/pachinko_double/` is `view.config`:
  `rg -n 'state\.config' double/pachinko_double -g '!inspection.py'` prints nothing, and
  Verification counts eleven reads of a config field through `view.config`, the ten
  effect sites and the expiry guard. `inspection.py` is left out of the first probe
  because `View.config` itself reads the snapshot there.
- No single-rule test gives the recorder an argument:
  `rg -n 'RecordingOutcome\([^)]' double/tests` prints nothing.
- Steps 2, 3 and 5 are each quoted red before the step that turns them green.
- `just python-double` exits 0 with the coverage test still strict at 93, 32 and 35. Its
  test count is the baseline's plus the tests step 5 adds.
- The diff changes no test under a `# scenario:` tag and nothing in
  `double/tests/test_guide.py`. The hand-back names each test function it changed.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. Each probe
that passes by printing nothing follows its positive control: the same pattern, matched
against a line it must find.

```sh
set -eu
git rev-parse HEAD
just python-double

control=$(printf 'outcome.config.lease_ttl\noutcome.now\n' | rg -c 'outcome\.(config|now)')
test "$control" -eq 2
if rg -n 'outcome\.(config|now)' double; then exit 1; else test "$?" -eq 1; fi

control=$(printf 'view.state.config.lease_ttl\n' | rg -c 'state\.config')
test "$control" -eq 1
if rg -n 'state\.config' double/pachinko_double -g '!inspection.py'; then exit 1; else test "$?" -eq 1; fi

fields='lease_ttl|max_consecutive_expiries|max_pull_batch|superseded_ttl|dead_letter_ttl'
reads=$(rg -o --no-filename "view\.config\.($fields)" double/pachinko_double | wc -l | tr -d ' ')
test "$reads" -eq 11

control=$(printf 'RecordingOutcome(CONFIG)\n' | rg -c 'RecordingOutcome\([^)]')
test "$control" -eq 1
if rg -n 'RecordingOutcome\([^)]' double/tests; then exit 1; else test "$?" -eq 1; fi

caches=$(find double -name __pycache__ -print)
test -z "$caches"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `acd466f3f5c6609be2cde98298481d91a23f9b37`, the commit after PD07's record.
- Work commit: `38f43d34a4f4d1160348e9365007c668ff9af5c6`.
- On 2026-11-20 the maintainer directed that PD08 land on this branch in this worktree, as
  PD01 to PD06 did. That departs from AGENTS.md's one-ticket-per-worktree rule and from
  step 1's "create the worktree on `branch:`". `ticket/pd08-one-read-context` was not
  created, and `just initialize` was not run.

### 2. Baseline

Both ran on the untouched worktree at `acd466f3f5c6609be2cde98298481d91a23f9b37`
(`ai_tmp/pd/PD08/baseline-python-double.txt` and `baseline-check.txt`).

- `just python-double` printed `Ran 274 tests` and ended `OK`.
- `just check` ended `All checks passed and the worktree is unchanged.`, and
  `git status --short` printed nothing.

### 3. What landed

- `inspection.py`: `View.config`, a read-only property that hands back the snapshot's
  `ConfigValues`. `View`'s docstring names the three things a rule reads there.
- `pull.py`, `settle.py`, `publish.py` and `lease_end.py`: the ten effect sites read
  `view.config`. `pull._batch` took the outcome only for the config, so it lost that
  parameter, and its two callers changed with it.
- `expiry.py`: `_at_threshold` reads `view.config`.
- `queue.py`: `Outcome.__init__` takes the working state and the lease-id source. Its
  `config` attribute, its `_now` and its `now` property are gone. `Queue._apply` still makes
  one `_Now` per outcome and passes it to `Queue._dispatch`, which builds each `View` with
  it. `Queue._config` is gone. The module docstring and `Outcome`'s docstring say so.
- `double/README.md`: the `config` row, and the paragraph after the table, which now names
  the view as the one place a rule reads and the outcome as write-only.
- The suite has 275 tests, the baseline's 274 and the pin. The coverage test stays strict
  at 93, 32 and 35.

Test functions and helpers changed. None sits under a `# scenario:` tag, and
`double/tests/test_guide.py` is untouched.

| File | Name | Change |
| --- | --- | --- |
| `test_pull.py` | `RecordingOutcome.__init__` | no `config` parameter, no `config` attribute |
| `test_pull.py` | `run_pull` | no `config` parameter |
| `test_pull.py` | `PullRowsOnHandBuiltStatesTest.scrambled` | an optional `config_values`, passed to `snapshot` as its config |
| `test_pull.py` | `test_the_claim_caps_the_batch_at_max_pull_batch` | `max_pull_batch=2` is in the snapshot |
| `test_lease_end.py` | `state` | the two TTLs are in the snapshot's config |
| `test_lease_end.py` | `freed` | the recorder takes no argument |
| `test_lease_end.py` | `test_a_freed_naming_a_lease_the_state_lacks_is_a_defect_no_row_answers` | the recorder takes no argument |
| `test_lease_end.py` | `CONFIG` and the `Config` import | removed |
| `test_queue.py` | `test_it_reads_the_clock_once_per_outcome` | its scratch effect `open_seeing` reads `view.now` |
| `test_queue.py` | `test_the_outcome_an_effect_is_handed_only_writes` | new: the pin of step 5 |

### 4. The red steps

Each ran at `acd466f3f5c6609be2cde98298481d91a23f9b37` with uncommitted changes, and its
output was saved as it happened under `ai_tmp/pd/PD08/`. Every failure is an assertion
failure, not an error. In each run the coverage test's
`test_every_test_def_was_collected_and_outside_this_module_passed` also failed, naming the
same tests: it reports any test that did not pass.

Step 2 (`red-step2.txt`), with `max_pull_batch=2` in the snapshot and no config given to
`run_pull`:

```text
FAIL: test_the_claim_caps_the_batch_at_max_pull_batch (tests.test_pull.PullRowsOnHandBuiltStatesTest.test_the_claim_caps_the_batch_at_max_pull_batch)
AssertionError: Lists differ: ['z1', 'z2', 'z3'] != ['z1', 'z2']
```

Step 3 (`red-step3.txt`), with the two TTLs in `state()`'s snapshot and both recorders
built with no argument. Exactly four tests of `test_lease_end.py` failed, each with an
entry's `expires_at` twelve hours (43,200,000 ms) past `ENDED_AT` where the test expects
the TTL it set:

```text
FAIL: test_a_nacked_held_version_is_dead_lettered_from_ended_at_and_the_newer_one_queued_at_arrival (tests.test_lease_end.FreedDeferredRowsTest.test_a_nacked_held_version_is_dead_lettered_from_ended_at_and_the_newer_one_queued_at_arrival)
AssertionError: Lists differ: [('cr[105 chars]s_at=44200005)), ('replace', Record(pocket='p'[172 chars]ne))] != [('cr[105 chars]s_at=1007005)), ('replace', Record(pocket='p',[171 chars]ne))]
FAIL: test_an_unsettled_held_version_is_shed_from_ended_at_and_the_newer_one_keeps_the_held_position (tests.test_lease_end.FreedDeferredRowsTest.test_an_unsettled_held_version_is_shed_from_ended_at_and_the_newer_one_keeps_the_held_position)
AssertionError: Lists differ: [('cr[140 chars]s_at=44200005)), ('replace', Record(pocket='p'[171 chars]ne))] != [('cr[140 chars]s_at=1005005)), ('replace', Record(pocket='p',[170 chars]ne))]
FAIL: test_a_nacked_record_becomes_a_dead_letter_stamped_from_ended_at_not_now (tests.test_lease_end.FreedTableTest.test_a_nacked_record_becomes_a_dead_letter_stamped_from_ended_at_not_now)
AssertionError: Lists differ: [('cr[106 chars]s_at=44200005)), ('remove', Record(pocket='p',[190 chars]ne))] != [('cr[106 chars]s_at=1007005)), ('remove', Record(pocket='p', [189 chars]ne))]
FAIL: test_the_dead_letter_holds_the_held_version_not_the_records_newest (tests.test_lease_end.FreedTableTest.test_the_dead_letter_holds_the_held_version_not_the_records_newest)
AssertionError: Tuples differ: ('cre[56 chars]'held', nacked_by='c', nacked_at=1000001, expires_at=44200005)) != ('cre[56 chars]'held', nacked_by='c', nacked_at=1000001, expires_at=1007005))
```

Step 2's test was still red in that run. After step 4 the suite ran 274 tests, `OK`.

Step 5 (`red-step5.txt`), with the pin added and `Outcome` not yet changed. Both subtests
failed:

```text
FAIL: test_the_outcome_an_effect_is_handed_only_writes (tests.test_queue.ApplyStepTest.test_the_outcome_an_effect_is_handed_only_writes) (name='config')
AssertionError: True is not false
FAIL: test_the_outcome_an_effect_is_handed_only_writes (tests.test_queue.ApplyStepTest.test_the_outcome_an_effect_is_handed_only_writes) (name='now')
AssertionError: True is not false
```

After step 6 `just python-double` ran 275 tests, `OK`.

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `acd466f3f5c6609be2cde98298481d91a23f9b37` plus uncommitted work (steps 2 to 7).
  HEAD was the base, so the brief gave the uncommitted diff
  (`ai_tmp/review/PD08-1/diff-uncommitted.patch`), the ticket's seven questions and the
  saved red output, and none of the author's reasoning.
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command.
  One Claude runner subagent per lens. Each findings file is Codex's last message
  (`Source: last-message`). Codex ran `just python-double` itself (275 tests, `OK`); its
  sandbox blocked `just check` and `just check-docs`. No writer or gate of the lead's ran
  meanwhile, and the tree's diff was byte-identical to the reviewed patch afterwards.
- Lenses: conformance and breakage.
- Triage: one fresh subagent, which reproduced each finding on a throwaway copy under
  `ai_tmp/review/PD08-1/` and edited no tree file (`verdicts.md`).
- Findings: 0 high, 1 medium, 1 low. No second round, since no high-severity finding was
  valid.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | Verification's count pipes `rg` into `wc` and `tr`, so `set -eu` does not see `rg` fail; a copy with a twelfth read in an unreadable file was accepted as eleven | not applied: the fix edits the Verification script, outside Files touched; handed back under Open points |
| B1 | low | valid | the probes match spellings, not routes: an alias (`snapshot = view.state`) with a comment that restores the count passes all of them, as do `getattr` on an outcome and a recorder argument on the next line | not applied, for the same reason; handed back under Open points |

Neither finding is about the tree as it stands. Triage confirmed that `rg` exits 0 there
with exactly eleven matches, all code, and that none of B1's three shapes occurs under
`double/`.

### 6. Departures

- PD08 landed on `build-python-double` in the series' worktree, with no new worktree, no
  `ticket/pd08-one-read-context` and no `just initialize` (section 1).
- `scrambled` in `test_pull.py` gained an optional `config_values` argument. That is the smallest
  way to put the cap in the snapshot the test builds, and it is the one helper change
  beyond removing the recorder's parameter.
- `fixtures.config` is imported by name into `test_pull.py` and `test_lease_end.py`, as
  those files import their other fixtures.
- The review ran on the uncommitted diff from the base, not `git diff "$base"...HEAD`,
  which was empty before the work commit.
- Both valid review findings concern the ticket's own Verification script. They were not
  applied, because the ticket's row in Files touched allows only `status:` and these
  notes. Verification below ran as the ticket writes it.
- The triage subagent left four throwaway directories under the gitignored
  `ai_tmp/review/PD08-1/` (`triage-c1`, `triage-b1`, `triage-b1-probe`, `triage-b1-tests`).
- The `/code-review` pass followed this ticket's record commit, and its fixes have their
  own commit (section 8).

### 7. Verification

The Verification block ran at `38f43d34a4f4d1160348e9365007c668ff9af5c6`, the work commit,
as one `sh` script under `set -eu`. It exited 0 (`exit=0`). The four probes printed
nothing. The opening and closing lines of its output
(`ai_tmp/pd/PD08/verify-38f43d3.log`):

```text
38f43d34a4f4d1160348e9365007c668ff9af5c6
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
...................................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 275 tests in 0.138s

OK
…  (just check-docs and just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

### 8. The code review, after the record commit

After `db3ed1aafb869043f2bc7462669ec747c3cc7d50`, the `/code-review` pass ran at medium
effort over `acd466f3f5c6609be2cde98298481d91a23f9b37..HEAD`. It found no correctness bug
and five nits.

| Nit | Outcome |
| --- | --- |
| `View`'s docstring said a rule "has no other route", which the code does not enforce | fixed: the docstring calls it the package's convention |
| `scrambled`'s new `config` parameter shadowed the `config` fixture imported into the same module | fixed: the parameter is `config_values` |
| The pin checks two attribute names on `Queue`'s `Outcome`, not its whole surface, and not the recorder | not changed: step 5 fixes the pin's form; added to Open points |
| Section 7's quoted block holds one line no command printed, the elision | not changed: it follows PD06's hand-back; the line is marked with `…` |
| PD08 landed on the series' branch, not its own | the maintainer's direction of 2026-11-20 (section 1) |

The two fixes are commit `fe5cbc3e8bdabeb066895e0f713bdfb9ac1cf6f8`. The Verification
block ran again there, as written, and exited 0: `Ran 275 tests`, `OK`, the four probes
silent, and `All checks passed and the worktree is unchanged.`
(`ai_tmp/pd/PD08/verify-review-fix.log`).

## Open points

- **C1: Verification's count does not see `rg` fail.** Proposed, replacing the line that
  assigns `reads`:

  ```sh
  found=$(rg -o --no-filename "view\.config\.($fields)" double/pachinko_double)
  reads=$(printf '%s\n' "$found" | wc -l | tr -d ' ')
  ```

  Triage ran it: exit 2 on the copy with the unreadable file, and 11 on the real tree.
- **B1: the probes match spellings, not routes.** `ai_tmp/review/PD08-1/verdicts.md` holds
  a tested, tighter block: an alias probe (any `<name>.config` that is not `view.config`,
  outside `inspection.py` and `invariants.py`; it needs `rg -P`), a count that skips
  comment lines and supersedes C1's two lines, and a recorder probe that spans a line
  break. Adopting it also means rewording the Acceptance criteria that quote the old
  patterns. It still does not catch `getattr` on an outcome; the pin in `test_queue.py`
  covers `Queue`'s own `Outcome` there, not the recorder. A syntax-aware check would need
  an exclusion list, which is the maintainer's choice.
- The pin names two attributes of `Queue`'s `Outcome`. A read route under another name, or
  one on the tests' recorder, passes it. An allow-list of the outcome's write methods,
  checked on both, would close that (the code review's nit, section 8).
- Pushing the branch and opening a pull request both need the maintainer's authorisation
  (§11, "Separately authorised actions").
