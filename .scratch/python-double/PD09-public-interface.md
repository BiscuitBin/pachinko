---
id: PD09
title: "Python double: a small top level, with harness and exceptions namespaces"
status: done
depends_on: [PD08]
parallel_with: []
branch: ticket/pd09-public-interface
estimated_size: S
---

# PD09: Python double: a small top level, with harness and exceptions namespaces

## Context

`double/pachinko_double/__init__.py` exports 55 names in one flat `__all__`. The line
numbers below are those of commit `7a25a7ff37176631673753d2b0a07130e7f8550a`, where
`__all__` is lines 49 to 105.

| Kind | Count | Names |
| --- | --- | --- |
| The facade and its config | 2 | `Queue`, `Config` |
| What `Queue` answers | 6 | `Batch`, `NoPocketAvailable`, `TickReport`, `LeaseView`, `HeldRecordView`, `Snapshot` |
| Entities, enums and durations from `model.py` | 16 | `Pocket`, `Record`, `Lease`, `Mark`, `MINUTE` and the rest |
| The effect protocols and their fakes | 4 | `Clock`, `LeaseIds`, `FakeClock`, `SequentialLeaseIds` |
| The invariant checker and hand-built state | 3 | `check`, `Violation`, `ConfigValues` |
| Exception classes | 15 | the twelve of `refusals.py`, `Defect` and `GuardOverlap` of `rules.py`, `InvariantsBroken` of `invariants.py` |
| Submodules | 9 | `consuming`, `decay`, `expiry`, `identity`, `lease_end`, `publish`, `pull`, `rules`, `settle` |

A reader of that list cannot tell which names are the primary path, which are for tests,
and which are errors. The maintainer asked for the three to be told apart.

Almost nothing uses the flat list:

- One line imports names from the top level: the walkthrough in `double/README.md`, line
  240, `from pachinko_double import Config, FakeClock, Queue, SequentialLeaseIds, check`.
- Three test modules import a submodule from the package: `test_rules.py` (`rules`),
  `test_coverage.py` (`invariants`, `rules`) and `test_expiry.py` (`decay`). Those imports
  work whether or not `__init__.py` re-exports the submodule.
- Every other test import names the defining module, such as
  `from pachinko_double.refusals import ReleaseRefused`.

The exception classes are of two kinds. The twelve under `Refusal` are outcomes the
specification defines, which a caller is meant to catch. `Defect`, `GuardOverlap` and
`InvariantsBroken` mean the double itself is wrong.

The whole package is a test double, so "primary interface" here means what a reader
drives: the queue, its config and what it answers.

## Goal

The package shows three interfaces, each in its own namespace, and a test pins each
namespace's list of names.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer chose these on 2026-11-22. The builder does not reopen them.

- **Three namespaces, and `model` as it is.**

  | Namespace | Names |
  | --- | --- |
  | `pachinko_double` (8) | `Queue`, `Config`, `Batch`, `NoPocketAvailable`, `TickReport`, `LeaseView`, `HeldRecordView`, `Snapshot` |
  | `pachinko_double.harness` (7) | `Clock`, `LeaseIds`, `FakeClock`, `SequentialLeaseIds`, `check`, `Violation`, `ConfigValues` |
  | `pachinko_double.exceptions` (15) | The refusals: `Refusal`, `ConfigRefused`, `PublisherAlreadyJoined`, `ConsumerAlreadyJoined`, `UnknownPublisher`, `UnknownConsumer`, `EqualRankRefused`, `PullRefused`, `LeaseFenced`, `RecordNotHeld`, `MarkConflict`, `ReleaseRefused`. The broken double: `Defect`, `GuardOverlap`, `InvariantsBroken` |
  | `pachinko_double.model` (unchanged) | The entities, enums and durations, where they already live |

- **The namespaces are thin.** `harness.py` and `exceptions.py` are new modules that only
  import and re-export. Each has an `__all__` and a docstring that says what the namespace
  is for. No class or function moves, and the defining modules (`effects.py`,
  `refusals.py`, `rules.py`, `invariants.py`, `inspection.py`) are not edited.
- **`exceptions` keeps its two groups apart.** Its docstring names them: what the
  specification refuses, and what means the double itself is wrong. Its `__all__` lists
  the refusals first, then the other three.
- **The top level drops the entities and the submodules.** The sixteen names of
  `model.py` are reached as `pachinko_double.model`. The nine submodules stay importable
  as `pachinko_double.<module>`; `__init__.py` no longer lists them.
- **`Clock` and `LeaseIds` sit in `harness`**, beside the fakes that satisfy them. Open
  points holds the alternative.
- **Tests keep their imports.** A test goes on importing from the defining module. Only
  the walkthrough's import changes, because it is the one line that names removed
  top-level names.
- **The package docstring names the three namespaces and `model`**, so the first thing a
  reader opens says where to look.

## Non-goals

- No change under `docs/specs/` or `crates/`.
- No behaviour change. Every scenario test passes as it stands.
- No class or function moves between modules, and no rename of `refusals.py`.
- No switch of the tests' imports to the new namespaces.
- No compatibility aliases for the removed top-level names. Nothing outside `double/`
  imports the package.
- No spec note. The change interprets nothing in the specification.

## Shared rules

Shared rule 1 of PD01 to PD06 binds this ticket, as
`.scratch/python-double/PD08-one-read-context.md` copies it under "Shared rules". Shared
rules 2 to 6 bind it as `.scratch/python-double/PD06-close-out.md` writes them under
"Shared rules (PD01 to PD06)". They are not copied here. Rule 3's coverage test stays
strict, and its three counts stay 93, 32 and 35.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/python-double/PD06-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `PD09`, and `<n>` is 1: this ticket has one checkpoint, at step 6.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names this ticket's path and its "Decisions fixed in this ticket".
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff, as PD08's did.
- A runner polls on `PD09-1/last-<lens>.md` or the process id, never the bare file name.
  At PD08 the bare name also matched a Codex run in another worktree.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/pachinko_double/__init__.py` | shared, own lines | The eight names, and the docstring |
| `double/pachinko_double/harness.py` | new | The seven re-exports |
| `double/pachinko_double/exceptions.py` | new | The fifteen re-exports, in two groups |
| `double/tests/test_interface.py` | new | The pin of step 2 |
| `double/README.md` | shared, own lines | The walkthrough's import, and a short table of the namespaces under "How the package follows the specification" |
| `.scratch/python-double/PD09-public-interface.md` | ticket | `status:` and the hand-back notes |
| `.scratch/python-double/README.md` | shared, own lines | PD09's Status cell |

## Steps

Each red step quotes the failing test's name and its message verbatim, headed by the
commit and "with uncommitted changes".

1. **Worktree and baseline.** PD01 to PD08 landed on `build-python-double` in one worktree
   at the maintainer's direction, so ask which applies here. Record the base and its full
   commit id. Run `just check` and `just python-double` on the untouched worktree, and
   quote each one's last line and the test count, headed by the commit.
2. **Red: the pin.** Add `double/tests/test_interface.py`. For each of the three
   namespaces it asserts that `__all__` equals the list in "Decisions fixed", and that
   each listed name is the same object as the one in its defining module. It also asserts
   that the three lists share no name. It fails: `harness` and `exceptions` do not exist,
   and the top level's `__all__` holds 55 names.
3. **Green: the namespaces.** Add `harness.py` and `exceptions.py`, and trim
   `__init__.py` to the eight names. The pin passes. The walkthrough now fails on its
   import, because `FakeClock`, `SequentialLeaseIds` and `check` have left the top level:
   quote that failure as this step's second red.
4. **The walkthrough.** In `double/README.md` the walkthrough imports `Config` and `Queue`
   from `pachinko_double`, and `FakeClock`, `SequentialLeaseIds` and `check` from
   `pachinko_double.harness`. `just python-double` passes.
5. **Words.** Write the package docstring. Add the table of namespaces to
   `double/README.md`, and change any sentence there that says where a moved name is
   imported from. Run `just check-docs`.
6. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Is any name reachable from two of the three namespaces, or from none?
   - Would the pin fail if a name were added to a namespace, removed from one, or bound
     to a different object than its defining module's?
   - Does any import under `double/`, or any doctest, still take a removed name from the
     top level?
   - Did the diff move or edit a class, change a test under a `# scenario:` tag, or change
     the walkthrough beyond its import?
   - Does the coverage test still collect every test, the new module's included, and stay
     strict at 93, 32 and 35?
   - Does dropping the submodule re-exports break `import pachinko_double` followed by
     `pachinko_double.<module>` anywhere it is used?
   - Could a probe in Verification print nothing while its claim is false?
7. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in `.scratch/python-double/README.md`,
   write the hand-back notes, and commit the record. **Stop** before pushing or opening a
   pull request: both need authorisation (`tickets/CONVENTIONS.md` §11, "Separately
   authorised actions").

## Acceptance criteria

- `pachinko_double.__all__`, `pachinko_double.harness.__all__` and
  `pachinko_double.exceptions.__all__` hold exactly the 8, 7 and 15 names of "Decisions
  fixed", and the pin asserts it.
- No name is in two of the three lists.
- No file under `double/` imports a removed name from the top level. Verification's probe
  prints nothing.
- `git diff --stat "$base"` lists no defining module: `effects.py`, `refusals.py`,
  `rules.py`, `invariants.py`, `inspection.py`, `model.py` and `queue.py` are unchanged.
- Steps 2 and 3 are each quoted red before the step that turns them green.
- `just python-double` exits 0 with the coverage test still strict at 93, 32 and 35. Its
  test count is the baseline's plus the tests step 2 adds.
- The diff changes no test under a `# scenario:` tag, and nothing in
  `double/tests/test_guide.py`.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. The probe
that passes by printing nothing follows its positive control. No probe reads `rg`'s
status through a pipe (PD08's finding C1).

```sh
set -eu
git rev-parse HEAD
just python-double

kept='Queue|Config|Batch|NoPocketAvailable|TickReport|LeaseView|HeldRecordView|Snapshot'
pattern="^(>>> )?from pachinko_double import (?!(($kept)(, |\$))+\$)"
control=$(printf 'from pachinko_double import Config, FakeClock\n' | rg -cP "$pattern")
test "$control" -eq 1
if printf 'from pachinko_double import Config, Queue\n' | rg -P "$pattern"; then exit 1; else test "$?" -eq 1; fi
if rg -nP "$pattern" double -g '*.py' -g '*.md' -g '!test_rules.py' -g '!test_coverage.py' -g '!test_expiry.py'; then exit 1; else test "$?" -eq 1; fi

test -f double/pachinko_double/harness.py
test -f double/pachinko_double/exceptions.py
changed=$(git diff --name-only "$base" -- double/pachinko_double)
for module in effects refusals rules invariants inspection model queue; do
  case "$changed" in *"pachinko_double/$module.py"*) exit 1 ;; esac
done

caches=$(find double -name __pycache__ -print)
test -z "$caches"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

`base` is the commit step 1 records; set it before the script runs. The three test
modules the probe leaves out import a submodule from the package (`rules`, `invariants`,
`decay`), which stays allowed. The builder runs the two controls first and corrects the
pattern, saying so under Departures, if either does not behave as written.

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `00f230051ebcc2730342c39a0eeb92bc2707f1cf`, the commit that added this ticket.
- Work commit: `197050043fd9197ca2aeece6d516cb40d54e9a41`.
- On 2026-11-23 the maintainer directed that PD09 land on this branch in this worktree, as
  PD01 to PD08 did. That departs from AGENTS.md's one-ticket-per-worktree rule and from
  this ticket's `branch:`. `ticket/pd09-public-interface` was not created, and
  `just initialize` was not run.
- The same day the maintainer ruled on the first Open point: `Clock` and `LeaseIds` stay
  in `harness`. "Decisions fixed" stands as written.

### 2. Baseline

Both ran on the untouched worktree at `00f230051ebcc2730342c39a0eeb92bc2707f1cf`
(`ai_tmp/pd/PD09/baseline-python-double.txt` and `baseline-check.txt`).

- `just python-double` printed `Ran 275 tests` and ended `OK`.
- `just check` ended `All checks passed and the worktree is unchanged.`, and
  `git status --short` printed nothing.

### 3. What landed

- `__init__.py`: the eight names, in the order of "Decisions fixed". The sixteen names of
  `model.py`, the fifteen exception classes, the seven harness names and the nine
  submodule re-exports are gone from it. Its docstring names the three namespaces and
  `model`.
- `harness.py` (new): a docstring, three import lines and an `__all__` of seven names.
- `exceptions.py` (new): a docstring that names the two groups, three import statements
  and an `__all__` of fifteen names, the twelve refusals first.
- `tests/test_interface.py` (new): the pin, five tests. For each namespace, `__all__`
  equals the list, in order; the counts are 8, 7 and 15; each name is the same object as
  its defining module's, and its `__module__` is that module; the namespace binds no
  other public name and has no module `__getattr__`; and the three lists share no name.
- `double/README.md`: the walkthrough's one import line is two, and "How the package
  follows the specification" gains the table of namespaces, after the table of what
  happens when no guard holds and before `### Entities`. No other sentence there
  said where a moved name is imported from: the page names defining modules
  (`refusals.py`, `effects.py`), which is still true.
- The suite has 280 tests, the baseline's 275 and the pin's five. The coverage test stays
  strict at 93, 32 and 35.
- After a bare `import pachinko_double`, `sys.modules` holds the same seventeen
  `pachinko_double` modules at the base and at the work commit
  (`ai_tmp/pd/PD09/sysmodules-before.txt` and `sysmodules-after.txt`): `queue.py`
  imports every rule module, so the registry is still complete without the re-exports.

No defining module is edited, no test under a `# scenario:` tag is changed, and
`double/tests/test_guide.py` is untouched.

### 4. The red steps

Each ran at `00f230051ebcc2730342c39a0eeb92bc2707f1cf` with uncommitted changes, and its
output was saved as it happened under `ai_tmp/pd/PD09/`. In each run the coverage test's
`test_every_test_def_was_collected_and_outside_this_module_passed` also failed, naming the
same tests: it reports any test that did not pass.

Step 2 (`red-step2.txt`), with `test_interface.py` added and the package untouched. The
run ended `Ran 280 tests` and `FAILED (failures=3, errors=26)`. The two failures are the
top level's 55 names:

```text
FAIL: test_each_namespace_lists_exactly_its_names_in_order (tests.test_interface.NamespaceTest.test_each_namespace_lists_exactly_its_names_in_order) (namespace='pachinko_double')
AssertionError: Lists differ: ['HOUR', 'MILLISECOND', 'MINUTE', 'SECOND',[712 chars]tle'] != ['Queue', 'Config', 'Batch', 'NoPocketAvail[58 chars]hot']
FAIL: test_each_namespace_binds_no_public_name_it_does_not_list (tests.test_interface.NamespaceTest.test_each_namespace_binds_no_public_name_it_does_not_list) (namespace='pachinko_double')
AssertionError: Lists differ: ['Batch', 'Clock', 'Config', 'ConfigRefused', 'Config[607 chars]eck'] != ['Batch', 'Config', 'HeldRecordView', 'LeaseView', 'N[48 chars]ort']
```

The 26 errors are the two namespaces that did not exist. Four are these, and the other
22 are `test_each_name_is_its_defining_modules_object`, once for each of the 7 and 15
names, with the same two messages:

```text
ERROR: test_each_namespace_lists_exactly_its_names_in_order (tests.test_interface.NamespaceTest.test_each_namespace_lists_exactly_its_names_in_order) (namespace='pachinko_double.harness')
ModuleNotFoundError: No module named 'pachinko_double.harness'
ERROR: test_each_namespace_lists_exactly_its_names_in_order (tests.test_interface.NamespaceTest.test_each_namespace_lists_exactly_its_names_in_order) (namespace='pachinko_double.exceptions')
ModuleNotFoundError: No module named 'pachinko_double.exceptions'
ERROR: test_each_namespace_binds_no_public_name_it_does_not_list (tests.test_interface.NamespaceTest.test_each_namespace_binds_no_public_name_it_does_not_list) (namespace='pachinko_double.harness')
ModuleNotFoundError: No module named 'pachinko_double.harness'
ERROR: test_each_namespace_binds_no_public_name_it_does_not_list (tests.test_interface.NamespaceTest.test_each_namespace_binds_no_public_name_it_does_not_list) (namespace='pachinko_double.exceptions')
ModuleNotFoundError: No module named 'pachinko_double.exceptions'
```

Step 3 (`red-step3.txt`), with the two namespaces added, `__init__.py` trimmed and
`double/README.md` untouched. The pin's five tests passed. The run ended `Ran 280 tests`
and `FAILED (failures=2)`: the walkthrough and the coverage test's report of it. The
walkthrough's message opens with the import, and the other 44 failed examples are
`NameError`s that follow from it:

```text
FAIL: test_the_guides_walkthrough_runs_as_written (tests.test_guide.GuideTest.test_the_guides_walkthrough_runs_as_written)
AssertionError: 45 != 0 : **********************************************************************
File "/Users/scutting/.supacode/repos/pachinko/build-python-double/double/README.md", line 240, in README.md
Failed example:
    from pachinko_double import Config, FakeClock, Queue, SequentialLeaseIds, check
Exception raised:
    Traceback (most recent call last):
      File "<doctest README.md[1]>", line 1, in <module>
        from pachinko_double import Config, FakeClock, Queue, SequentialLeaseIds, check
    ImportError: cannot import name 'FakeClock' from 'pachinko_double' (/Users/scutting/.supacode/repos/pachinko/build-python-double/double/pachinko_double/__init__.py)
```

Steps 4 and 5 went in together. After them `just python-double` ran 280 tests, `OK`
(`ai_tmp/pd/PD09/green-step5.txt`).

### 5. Checkpoint 1 record: conformance and breakage (before the final commit)

- Ran at: `00f230051ebcc2730342c39a0eeb92bc2707f1cf` plus uncommitted work (steps 2 to 5).
  HEAD was the base, so the brief gave the uncommitted diff
  (`ai_tmp/review/PD09-1/diff-uncommitted.patch`), the three untracked files by name
  (`scope.md`), the ticket's seven questions and the saved red output, and none of the
  author's reasoning.
- Harness and model: Codex `gpt-6-astra`, `-s read-only`, through the corrected command.
  Each findings file is Codex's last message (`Source: last-message`). Codex ran
  `just python-double` itself (280 tests, `OK`); its sandbox blocked `just check`. No
  writer or gate of the lead's ran meanwhile, and the tree's diff was byte-identical to
  the reviewed patch afterwards.
- Lenses: conformance and breakage.
- Triage: one fresh subagent, which checked each finding by command, mutated only a
  throwaway copy under `ai_tmp/review/PD09-1/` and edited no tree file (`verdicts.md`).
  It had stopped before the lead applied a fix or ran a gate.
- Findings: 0 high, 2 medium (one reported by both lenses), 2 low. No second round,
  since no high-severity finding was valid.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1, B2 | medium | valid | Verification's import probe matches only a line that starts with the import. It prints nothing for an indented import, for two spaces after `from`, for `import pachinko_double as p` then `p.FakeClock`, and for anything in the three excluded test files | not applied: the fix edits the Verification script, outside Files touched; handed back under Open points |
| B1 | medium | valid | the pin read `vars(namespace)` and passed over every module, so a module `__getattr__` that answers `FakeClock` at the top level, or an unrelated module bound in `harness`, left all five tests passing | applied in `test_interface.py`: only the namespace's own submodule is passed over, and a module `__getattr__` fails the test |
| C2 | low | valid | `exceptions.py` said "every exception class the double raises", but the package also raises the builtin `ValueError` | applied: the three docstrings say "defines" |
| C3 | low | valid | the package docstring gave `Queue.lease_view` one answer, and it also answers `None` | applied in `__init__.py` |

C1 and B2 are about the script, not the tree: the triage's parsed scan of every `.py`
file and every doctest under `double/` found only `rules`, `invariants`, `decay`, `Config`
and `Queue` imported from the top level. B1's two shapes do not occur in the tree. After
B1's fix the pin failed on each of six in-memory mutations and passed when each was
undone (`ai_tmp/pd/PD09/pin-probes.txt`): a `__getattr__` alias, an unrelated module in
`harness`, a sibling module in `harness`, a plain alias at the top level, a name added
to an `__all__`, and a name rebound.

### 6. Departures

- PD09 landed on `build-python-double` in the series' worktree, with no new worktree, no
  `ticket/pd09-public-interface` and no `just initialize` (section 1).
- The pin has two assertions beyond step 2's three. It checks each name's `__module__`,
  and it checks that a namespace binds no public name it does not list and has no module
  `__getattr__`. Without the second, a leftover import in `__init__.py` would pass the
  pin while the name stayed reachable from two namespaces, which is step 6's first
  question. Review finding B1 tightened it after step 2's red was saved, so the pin that
  ran red is the earlier form.
- The lead started the two Codex runs itself, each as its own background job that
  reports its exit, where PD06's deltas name one runner subagent per lens. Nothing
  polled on a bare file name.
- The review ran on the uncommitted diff from the base, not `git diff "$base"...HEAD`,
  which was empty before the work commit.
- The lead applied the three in-scope fixes itself after the triage subagent had stopped,
  where PD06's deltas have the triage subagents apply them.
- The valid finding C1 (B2) concerns the ticket's own Verification script. It was not
  applied, because the ticket's row in Files touched allows only `status:` and these
  notes. Verification below ran as the ticket writes it. Both of its controls behaved as
  written, so the pattern was not corrected.
- The triage subagent left a throwaway directory under the gitignored
  `ai_tmp/review/PD09-1/` (`triage-b1`).
- The `/code-review` pass followed this ticket's record commit, and its fixes have their
  own commit (section 8). One of them adds a line to `__init__.py` that "Decisions fixed"
  does not name: `from . import exceptions, harness`.

### 7. Verification

The Verification block ran at `197050043fd9197ca2aeece6d516cb40d54e9a41`, the work commit,
as one `sh` script under `set -eu`, with `base` set to
`00f230051ebcc2730342c39a0eeb92bc2707f1cf`. It exited 0 (`exit=0`). The two controls
behaved as written, and the probe printed nothing. The opening and closing lines of its
output (`ai_tmp/pd/PD09/verify-1970500.log`):

```text
197050043fd9197ca2aeece6d516cb40d54e9a41
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
........................................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 280 tests in 0.146s

OK
…  (just check-docs and just check output elided; full log kept in ai_tmp)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

### 8. The code review, after the record commit

After `5e13a83acc2727f145748eeb118e45d568561c27`, the `/code-review` pass ran at medium
effort over `00f230051ebcc2730342c39a0eeb92bc2707f1cf..HEAD`. It found one bug and five
nits.

| Finding | Outcome |
| --- | --- |
| After a bare `import pachinko_double`, `pachinko_double.harness` and `pachinko_double.exceptions` raised `AttributeError`, while `pachinko_double.model` answered | fixed: `__init__.py` imports both modules; neither is in `__all__`, and the pin passes over a namespace's own submodules |
| Nothing pins that the nine rule modules are attributes after a bare import; they are bound only because `queue.py` imports them | not changed: "Decisions fixed" drops those re-exports; added to Open points |
| The pin's docstring said a name cannot be reachable from a namespace that does not list it, yet a namespace answers its own submodules, and a re-added `from . import rules` passes | fixed in the docstring, which now says what is checked; the gap is in Open points with the one above |
| The count test and the no-shared-name test read only the test file's lists | fixed: both read each namespace's `__all__` |
| `ConfigValues` and `Violation` are types a reader's code can name (`Snapshot.config`, `InvariantsBroken.violations`), and they sit in `harness` | not changed: the lists are the maintainer's; added to Open points |
| Section 7's quoted block holds one line no command printed, the elision; section 3 said the README table ends its section | the elision follows PD06's and PD08's hand-backs and is marked with `…`; section 3 is corrected |

The fixes are commit `fb836020c21221af5b955f11f27759df61ebc665`. The Verification block
ran again there, as written, and exited 0: `Ran 280 tests`, `OK`, the probe silent, and
`All checks passed and the worktree is unchanged.`
(`ai_tmp/pd/PD09/verify-review-fix.log`). The pin still failed on each of the six
mutations of section 5, and the no-shared-name test failed with `Refusal` added to
`harness.__all__`.

## Open points

- **Closed.** `Clock` and `LeaseIds` are the two protocols `Queue`'s constructor takes.
  This ticket puts them in `harness`, beside the fakes. The alternative was to keep them
  at the top level, which would then hold ten names. On 2026-11-23 the maintainer ruled
  that they stay in `harness`.
- The tests keep importing fakes and exceptions from the defining modules. Whether they
  should later import them from `harness` and `exceptions`, so that the suite shows the
  intended paths, is left for a later ticket.
- **C1 (B2): Verification's import probe sees only an import that starts its line.** It
  misses an indented import, a second space after `from`, an alias of the package
  followed by an attribute, and the three test files it leaves out. The tree holds none
  of these today (section 5). Proposed for a later ticket: replace the `rg` probe with a
  parsed check, either in the script or as a test in `test_interface.py`, that walks
  every `.py` file and every doctest under `double/` and allows, from the top level,
  only the eight names and the package's submodules.
- **The submodules as attributes are not pinned.** `pachinko_double.<module>` answers
  after a bare import only because `queue.py` imports every rule module, and the pin
  cannot tell that from a re-export put back in `__init__.py`. A pin would run a fresh
  interpreter, since any test that imports a submodule binds it.
- **`ConfigValues` and `Violation` in `harness`.** A reader who annotates
  `snapshot.config` or handles `InvariantsBroken.violations` names a type that only
  `harness` exports. The ruling of 2026-11-23 covered `Clock` and `LeaseIds`; the
  maintainer has not been asked about these two.
