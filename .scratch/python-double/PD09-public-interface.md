---
id: PD09
title: "Python double: a small top level, with harness and exceptions namespaces"
status: todo
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

Written at the end, in the order PD08's hand-back uses: base and branch, baseline, what
landed, the red steps, the checkpoint's record, departures, and Verification.

## Open points

- `Clock` and `LeaseIds` are the two protocols `Queue`'s constructor takes. This ticket
  puts them in `harness`, beside the fakes. The alternative is to keep them at the top
  level, which would then hold ten names. The maintainer has not ruled on it.
- The tests keep importing fakes and exceptions from the defining modules. Whether they
  should later import them from `harness` and `exceptions`, so that the suite shows the
  intended paths, is left for a later ticket.
