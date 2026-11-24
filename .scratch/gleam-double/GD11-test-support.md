---
id: GD11
title: "Gleam double: test support under test/support/"
status: done
depends_on: [GD08]
parallel_with: []
branch: build-gleam-double
estimated_size: S
---

# GD11: Gleam double: test support under test/support/

## Context

`gleam-double/test/` holds 26 files side by side at commit
`aebae0afadfb34a2d8ae762316376f70d3df08fe`: 22 test modules, the entry module
`pachinko_double_test.gleam`, and three modules that are neither.

| Module | What it is |
| --- | --- |
| `harness.gleam` | The fakes the tests inject, a clock and a lease-id source, and the helpers that start the owner and send it triggers |
| `runner.gleam` | The double's own test runner, and its checks that every test module is listed and imported |
| `coverage.gleam` | The coverage check, which `main` runs after the last test |

GD08 adds a 23rd test module, `interface_test.gleam`. A reader who lists the directory
cannot tell the three support modules from the tests except by the missing `_test`. The
maintainer asked for the test harness to be easy to tell apart from the tests and from
the interface.

GD08 to GD10 sort `gleam-double/src/`. This ticket sorts `gleam-double/test/`. It needs
only GD08, which creates the pin this ticket extends. It is written to run last, after
GD10, because the series shares one worktree; "Decisions fixed" says what differs if it
runs earlier.

## Goal

A listing of `gleam-double/test/` shows the test modules and one directory, `support/`,
which holds the fakes, the runner and the coverage check. The pin holds both lists.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer fixed the first group on 2026-11-23. The builder does not reopen them.

- **The three modules move as they are**, with `git mv`, to
  `gleam-double/test/support/harness.gleam`, `gleam-double/test/support/runner.gleam` and
  `gleam-double/test/support/coverage.gleam`. Nothing in them is split, merged or
  renamed.
- **The imports follow.** `import harness` becomes `import support/harness`, and
  likewise for `runner` and `coverage`. A module's qualifier is its last segment, so
  `harness.start` and `runner.run` are written as before.
- **The pin holds the top level of `gleam-double/test/` and the contents of
  `support/`.**

The second group is this ticket's own design. The builder follows it, and reports under
Departures if one of these cannot hold.

- **How many lines.** At commit `aebae0afadfb34a2d8ae762316376f70d3df08fe`, 45 lines
  in 24 files under `gleam-double/test/` start with `import harness`, `import runner` or
  `import coverage`. The pattern's `\b` matters: without it the count takes in
  `import coverage_test` and `import runner_test` in the entry module, which are test
  modules and stay. The command is
  `rg -c '^import (harness|runner|coverage)\b' gleam-double/test`. GD08 adds at least
  one, in `interface_test.gleam`, so the builder reruns the command at the base and
  changes what it prints.
- **One of those lines is not an import.** `gleam-double/test/runner_test.gleam` holds a
  fixture, a string of source text that `unimported_module_is_a_fault` hands to
  `runner.import_faults`. Its line `import runner.{type Module, Module}` starts at column
  0. It changes with the rest, to `import support/runner.{type Module, Module}`, so that
  the fixture still reads like the entry module. The test's result does not depend on
  that line.
- **The runner's checks are reread, and changed only where a red run asks.** The
  expectation is that none changes:
  - `runner.run` is still handed the directory `"test"`, and the entry module is still
    `test/pachinko_double_test.gleam`.
  - `runner.registry_faults` lists the directory with `simplifile.get_files`, which
    descends into `support/`. It keeps only files that end in `_test.gleam`, so the
    three support modules are passed over, as they are today.
  - A file `test/support/extra_test.gleam` would be found, under the module name
    `support/extra_test`, and reported as on disk and not listed in `main`. That is the
    right answer: `support/` holds no test.
  - `runner.import_faults` reads the module an import line names up to the first space
    or `.`, so `import support/runner.{type Module, Module}` reads as `support/runner`.

  Step 3 shows the third point red, on a scratch file.
- **What the pin gains**, in `gleam-double/test/interface_test.gleam`:
  - The top level of `gleam-double/test/` holds only files whose names end in
    `_test.gleam`, and the directory `support`.
  - `gleam-double/test/support/` holds exactly `coverage.gleam`, `harness.gleam` and
    `runner.gleam`.

  The pin reads each directory with `simplifile.read_directory`, which does not descend.
  It does not list the test modules by name: the runner already fails on a test module
  that `main` does not list.
- **The witness follows, if GD09 is done.** GD09's witness lets
  `gleam-double/test/guide_test.gleam` import the support modules `harness` and
  `runner`. If GD09 is done at this ticket's base, those two entries of the pin's allowed
  list become `support/harness` and `support/runner`, in step 3, before its green run.
  If GD09 is not done, its builder reads `support/harness` and `support/runner` where
  GD09 writes `harness` and `runner`: in its pin, in the `allowed` pattern of its
  Verification, and in that pattern's control, whose line `import harness.{type Double}`
  becomes `import support/harness.{type Double}`. That builder says so under
  Departures.
- **The words follow.** In `gleam-double/README.md`, `test/harness.gleam`, twice, and
  `test/coverage.gleam`, once, gain `support/`. The last two rows of "Where to look"
  become:

  | A reader looking for | Opens |
  | --- | --- |
  | The tests | `test/*_test.gleam`, run from `test/pachinko_double_test.gleam` |
  | Test support: the fakes, the runner and the coverage check | `test/support/harness.gleam`, `test/support/runner.gleam` and `test/support/coverage.gleam` |

  The root's header names no test file, so it does not change.

## Non-goals

- No behaviour change, and no change to what any test asserts.
- No test under a `Scenario` tag changes beyond its imports.
- No change inside `harness.gleam`, `runner.gleam` or `coverage.gleam` beyond their own
  `import` lines, unless step 3 shows a runner check must change.
- No test module moves into a directory, and none is renamed. The engine's tests stay
  beside the others.
- No split of `harness.gleam` into fakes and helpers.
- No change under `gleam-double/src/`, and none to `gleam-double/gleam.toml`,
  `gleam-double/manifest.toml`, the `Justfile` or anything under `docs/specs/`,
  `docs/decisions/`, `crates/` or `double/`.
- No new spec note (shared rule 5).

## Shared rules

Shared rules 1 to 6 bind this ticket as `.scratch/gleam-double/GD07-close-out.md` writes
them under "Shared rules (GD01 to GD07)", with rule 4's second sentence as
`.scratch/gleam-double/GD08-front-door.md` restates it under "Shared rules". They are not
copied here. Rule 3's coverage check stays strict at 93, 32 and 35. Under rule 1,
`double/pachinko_double/` and `double/tests/` stay unread.

Rule 4 says the fakes live under `gleam-double/test/`. They still do, one directory
down. Its test-runner paragraph is unchanged: a test module that `main` does not list is
still a failure.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/gleam-double/GD07-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `GD11`, and `<n>` is 1: this ticket has one checkpoint, at step 6.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names this ticket's path and its "Decisions fixed in this
  ticket", not `docs/specs/pachinko.allium`: the ticket changes no behaviour.
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff.
- A runner polls on `GD11-1/last-<lens>.md` or the process id, never the bare file name.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `gleam-double/test/harness.gleam`, `runner.gleam`, `coverage.gleam` | moved | `git mv` to `gleam-double/test/support/`; `coverage.gleam`'s import of `runner` |
| Every module under `gleam-double/test/` that imports one of the three | shared, own lines | The import |
| `gleam-double/test/runner_test.gleam` | shared, own lines | Its imports, and the fixture's one line |
| `gleam-double/test/interface_test.gleam` | shared, own lines | What the pin gains; the witness's two entries, if GD09 is done |
| `gleam-double/README.md` | shared, own lines | Three paths, and the last two rows of "Where to look" |
| `docs/project/repository-map.md` | shared, own lines | The `gleam-double/` row and the tree's `test/` line, if either's wording no longer matches |
| `.scratch/gleam-double/GD11-test-support.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD11's Status cell |

`gleam-double/test/interface_test.gleam` and the guide's "Where to look" exist once GD08
is done. `gleam-double/test/guide_test.gleam` holds a fixture that names
`test/harness.gleam` as a fence its check does not recognise; that is text the test
reads, and it stays.

## Steps

Each red step quotes the failing test's name and its message verbatim, headed by the
full commit id and "with uncommitted changes". Evidence goes under `ai_tmp/gd/GD11/`.

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, and say which of GD09 and GD10 it
   holds. If `.pixi/`, `.tools/` or `gleam-double/build/packages/` is missing, run
   `just initialize` and `just gleam-double-deps` (network, authorised by this step). Run
   `just check` and `just gleam-double` on the untouched worktree. Quote each one's last
   line and the line `N tests run, N passed`, headed by the commit.
2. **Red: the pin.** Add the two assertions of "What the pin gains", with a control on
   listings made by hand: a top level that holds `helpers.gleam` fails, and a `support/`
   that holds a fourth module fails. Run `just gleam-double-format` first. On the tree
   they fail: the top level holds three
   files that do not end in `_test.gleam`, and `test/support/` cannot be read. Quote the
   failures.
3. **Green: the move.** `git mv` the three files. Change the imports. If GD09 is done,
   change the two entries of the witness's allowed list in the same step, since
   `guide_test.gleam` now imports `support/harness` and `support/runner`. Run
   `just gleam-double-format`, then `just gleam-double`. The pin passes and the count is
   the baseline's plus the pin's new tests. Then show the runner still guards the new
   directory: add a scratch `gleam-double/test/support/extra_test.gleam` that holds one
   test, quote the run failing with
   `support/extra_test.gleam is on disk and not listed in main`, and delete the file. If
   the run does not fail that way, change the runner's check so that it does, and say so
   under Departures.
4. **The moved modules are unchanged.** Save `git diff -M "$base"` for the three
   modules and show that each is a rename whose changed lines are `import` lines.
5. **Words.** Change the three paths and the two rows in `gleam-double/README.md`. Use
   the `review-docs` skill for `docs/project/repository-map.md` if its wording needs to
   change. Run `just gleam-double` and `just check-docs`.
6. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Does any file under `gleam-double/` still import `harness`, `runner` or `coverage`
     by its old name, or give one's old path as if it existed?
   - Would the pin fail if a module that is not a test were added to the top level of
     `gleam-double/test/`, or a fourth module to `support/`?
   - Does the runner still fail on a test module that `main` does not list, at the top
     level and under `support/`? Did the move weaken any check of
     `runner.registry_faults` or `runner.import_faults`?
   - Did any line of the three moved modules change beyond an `import`?
   - Did any test under a `Scenario` tag change beyond its imports?
   - Does the coverage check still collect every test and stay strict at 93, 32 and 35?
   - Could a probe in Verification print nothing while its claim is false?
7. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `gleam-double/test/support/` holds exactly `coverage.gleam`, `harness.gleam` and
  `runner.gleam`, and the top level of `gleam-double/test/` holds only `*_test.gleam`
  files and `support/`. The pin asserts both.
- No line under `gleam-double/test/` imports `harness`, `runner` or `coverage` by that
  whole module name. `import runner_test` and `import coverage_test`, in the entry
  module, are other modules and stay. Verification's probe prints nothing.
- Each of the three moved modules differs from its text at the base only in `import`
  lines.
- Step 2 is quoted red before step 3 turns it green, and step 3's scratch test module is
  quoted failing the run.
- `just gleam-double` exits 0 with the coverage check strict at 93, 32 and 35. Its test
  count is the baseline's plus the tests step 2 adds.
- The diff changes nothing under `gleam-double/src/` and no test under a `Scenario` tag
  beyond its imports.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script from the repository root; `set -eu` stops it at the first command
that fails. `base` is the commit step 1 records; set it before the script runs. The
script's first check stops it when `base` is unset or names no commit. Each probe that
passes by printing nothing follows its positive control on every spelling it must
catch. `sh` reads only the status of a pipeline's last command, so every command whose
failure matters runs alone in an assignment or last in its pipeline: `ls`, `sort`,
`rg -l`, `git diff` and `find` each fill a variable of their own, only `printf` feeds a
pipe, and `rg` never does.

```sh
set -eu
git cat-file -e "$base^{commit}"   # base is set before the script runs
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

# The layout: support/ holds exactly the three modules, and the top level
# holds only *_test.gleam files and support/. `ls -Ap` ends each directory's
# name with `/`, so a directory named like a test module is one of the others.
test -d gleam-double/test/support
entries=$(ls gleam-double/test/support)
sorted=$(printf '%s\n' "$entries" | LC_ALL=C sort)
support=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$support" = "coverage.gleam harness.gleam runner.gleam "
pattern='^[^/]+_test\.gleam$'
probe=$(printf 'a_test.gleam\nsupport/\nextra_test.gleam/\nhelpers.gleam\n' | rg -cv "$pattern")
test "$probe" -eq 3
entries=$(ls -Ap gleam-double/test)
others=$(printf '%s\n' "$entries" | rg -v "$pattern")
test "$others" = "support/"
tests=$(printf '%s\n' "$entries" | rg -c "$pattern")
test "$tests" -gt 0                     # positive control: the listing sees the tree

# No import of a support module by its old name. The control holds each
# whole name bare and with a name list, the fixture's spelling, and three
# spellings that must not count: the new path, two test modules whose names
# only start alike, and a name with a digit.
pattern='^import (harness|runner|coverage)\b'
probe=$(printf 'import harness.{type Double}\nimport runner\nimport coverage\nimport runner.{type Module, Module}\nimport support/runner\nimport runner_test\nimport coverage_test\nimport harness2\n' | rg -c "$pattern")
test "$probe" -eq 4
if rg -n "$pattern" gleam-double/test; then exit 1; else test "$?" -eq 1; fi
imports=$(rg -l '^import support/(harness|runner|coverage)\b' gleam-double/test)
test -n "$imports"
rg -q '^import support/coverage$' gleam-double/test/pachinko_double_test.gleam
rg -q '^import support/runner\.\{type Module, Module\}$' gleam-double/test/pachinko_double_test.gleam
rg -q '^import runner_test$' gleam-double/test/pachinko_double_test.gleam
rg -q '^import coverage_test$' gleam-double/test/pachinko_double_test.gleam

# No prose gives a support module's old path. The control takes each of the
# three old paths and must not take the new one.
pattern='test/(harness|runner|coverage)\.gleam'
probe=$(printf 'test/harness.gleam\ntest/runner.gleam\n`test/coverage.gleam`,\ntest/support/harness.gleam\n' | rg -c "$pattern")
test "$probe" -eq 3
if rg -n "$pattern" gleam-double/README.md gleam-double/SPEC-NOTES.md gleam-double/src docs/project/repository-map.md; then exit 1; else test "$?" -eq 1; fi

# Each moved module is a rename whose changed lines are all import lines.
# `counts` reads a patch's file headers, its changed lines and its changed
# import lines; a pure rename has none of any. A removed line that starts with
# `-- ` reads as a header, so it moves the header count too. The control runs
# the same counts on a hand-made patch that also changes a body line, and must
# see it. `--text` keeps git from reporting a module as binary, which hides
# its changed lines; the second control's line is what git prints then, and
# the probe stops the script if a patch holds it anyway.
counts() {
  headers=$(printf '%s\n' "$1" | rg -c '^(\+\+\+|---) ' || test "$?" -eq 1)
  changed=$(printf '%s\n' "$1" | rg -c '^[+-]' || test "$?" -eq 1)
  imported=$(printf '%s\n' "$1" | rg -c '^[+-]import ' || test "$?" -eq 1)
}
control=$(printf -- '--- a/m.gleam\n+++ b/m.gleam\n@@ -1,2 +1,2 @@\n-import runner\n+import support/runner\n-pub fn f() {\n+pub fn f(x) {\n')
counts "$control"
test "$headers" -eq 2
test "$imported" -eq 2
test "$changed" -ne "$((headers + imported))"
pattern='^Binary files '
probe=$(printf 'rename from m.gleam\nrename to n.gleam\nBinary files a/m.gleam and b/n.gleam differ\n' | rg -c "$pattern")
test "$probe" -eq 1
for module in harness runner coverage; do
  patch=$(git diff --text -M "$base" -- "gleam-double/test/$module.gleam" "gleam-double/test/support/$module.gleam")
  printf '%s\n' "$patch" | rg -q "^rename from gleam-double/test/$module\.gleam\$"
  printf '%s\n' "$patch" | rg -q "^rename to gleam-double/test/support/$module\.gleam\$"
  if printf '%s\n' "$patch" | rg -n "$pattern"; then exit 1; else test "$?" -eq 1; fi
  counts "$patch"
  test "${changed:-0}" -eq "$((${headers:-0} + ${imported:-0}))"
done

# Nothing under src/ and neither pin changed. The control: the same command
# names a file that did change.
seen=$(git diff --name-only "$base" -- gleam-double/test/interface_test.gleam)
test "$seen" = "gleam-double/test/interface_test.gleam"
sources=$(git diff --name-only "$base" -- gleam-double/src)
test -z "$sources"
pins=$(git diff --name-only "$base" -- gleam-double/gleam.toml gleam-double/manifest.toml)
test -z "$pins"

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The layout reads `ls -Ap`, which ends a directory's name with `/`, so a directory whose
name ends in `_test.gleam` counts among the others, as it does in the pin. The `counts`
lines expect each moved module's changed lines to be its two file headers and its
import lines; a rename with no change has none of any, which `${changed:-0}` and its
neighbours read as zero. `git diff --text` keeps git from reporting a module as binary,
which would hide its changed lines, and the `Binary files` probe stops the script if a
patch holds that line anyway. If `gleam format` rewraps an import of a moved module
over several lines, its continuation lines do not start with `import` and the count
fails; the builder then shows the same thing with a saved diff and says so under
Departures. The builder runs each control first, and corrects a pattern, saying so
under Departures, if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the red steps; the checkpoint's record; Verification at the work commit; departures; and
what was not run.

Evidence files are under `ai_tmp/gd/GD11/`, and the review record under
`ai_tmp/review/GD11-1/`. None is committed. The implementer's files open with the commit
line, `d37b481b30afc31393f98223ab9eef1ec89f6e79` with "(untouched worktree)" or "with
uncommitted changes" after it; its two `just gleam-double-format` records do too.
`ev-verification-at-work.txt` opens `$ sh -x ...` and names its commit through
`git rev-parse HEAD`. The triage's records in `ai_tmp/review/GD11-1/` open with
`$ <command>` or a `#` comment and name no commit. In every quote below the recipe's echo
of its own command lines is left out, and `...` marks other lines left out or a line cut
short: a `FAIL` line is cut after the test's name, and the indented line under it, where
one is given, gives the assertion's `Left` value, then its `Right`, from the same line.
Times are UTC, read from the files' modification times, which are each file's last
write. Several of the implementer's files were last written after the runs they record,
so the times do not give the order of the steps; that order rests on the files' headers
and counts and on the lead's account. The work commit's own date is
2026-11-23T21:06:05-08:00, which is 2026-11-24T05:06:05Z. Where a fact the record should
hold was never saved, the text says "not recorded"; where it rests only on the lead's
message, the text says "by the lead's account".

### 1. Base, branch and commits

- Branch: `build-gleam-double`, in this worktree (shared rule 6).
- Base commit: `d37b481b30afc31393f98223ab9eef1ec89f6e79` (`base.txt`, last written at
  04:42:47Z), GD10's record commit. It holds GD09 and GD10, both done, so step 3's
  witness edit applies.
- Work commit: `421e24682999e8bf7a5813f6af1e95f859dfa5fc` (`work-commit.txt`), tree
  `4f36f1e78a4296cd0c081c6a474f6f9b23e44301`, titled "Move the Gleam double's test
  support under test/support/ (GD11)". It touches 28 files (`git show --stat`): the
  three renames to `gleam-double/test/support/`, 24 test modules (`interface_test.gleam`
  and the 23 that import a support module, the entry module among them) and
  `gleam-double/README.md`. Nothing under `gleam-double/src/`, and not
  `gleam-double/gleam.toml`, `gleam-double/manifest.toml` or
  `docs/project/repository-map.md`, is touched.
- The commit after it holds this record, the index's Status cell, and the ticket's
  corrected Verification block and the paragraphs around it (departure 8). The corrected
  block was not in the committed ticket at the work commit.
- Nothing was fetched. Step 1's `just initialize` and `just gleam-double-deps` were not
  run, by the lead's account; the baseline's `just gleam-double` passed its checks that
  the gleam environment and `gleam-double/build/packages/` are present.

### 2. Baseline

At `d37b481b30afc31393f98223ab9eef1ec89f6e79`, on the untouched worktree. Both files
were last written at 04:48:07Z, the same second as step 5's two records and after the
red run, so the time does not date the run. Each is headed "(untouched worktree)", and
its count, 347, holds none of the pin's four new tests.

`ev-baseline-check.txt`:

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 (untouched worktree)
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

`ev-baseline-gleam.txt`:

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 (untouched worktree)
$ just gleam-double
...
gleam 1.19.1
...
OTP 29
...
    Running pachinko_double_test.main
347 tests run, 347 passed
coverage faults: 0
exit status: 0
```

The import count at the base is not recorded. By the lead's account the implementer saw
47 lines in 25 files that start with an old name, and did not save it;
`ev-import-counts.txt` holds that figure only as a comment. What the file does record,
after the move, is `47 lines in 25 files` for the new spelling, `import support/...`.

### 3. What landed

- **The moves.** `harness.gleam`, `runner.gleam` and `coverage.gleam` moved with
  `git mv` to `gleam-double/test/support/`, whole: nothing split, merged or renamed.
  `harness` and `runner` are R100 renames; `coverage` is a rename at 99% similarity
  whose changed lines are its import of `runner` and the formatter's re-sorting of
  `simplifile` (`ev-moves-diff.txt`, section 4).
- **The imports.** Every `import harness`, `import runner` and `import coverage` became
  `import support/...`, in 23 test modules, `coverage.gleam` and the pin; qualifiers are
  unchanged. `runner_test.gleam`'s fixture line became
  `import support/runner.{type Module, Module}`, as "Decisions fixed" says. The entry
  module's `import runner_test` and `import coverage_test` stay
  (`ev-import-counts.txt`: lines 11 and 30). No test module other than the pin changed
  beyond its import lines (`ev-import-counts.txt`, its last part: `exit status: 1 (1 =
  none)`), so no test under a `Scenario` tag did.
- **The runner's checks** did not change. The scratch module of step 3 was found under
  `support/` and reported (section 4), so no runner change was owed.
- **The pin**, `gleam-double/test/interface_test.gleam`, gains 4 tests: the two tree
  assertions, "test/ holds at its top level only test modules and support/" and
  "test/support/ holds exactly coverage, harness and runner", and their two controls on
  listings made by hand, "a top level that holds a module that is not a test, a
  directory named like one, or no support, differs" (its name since the checkpoint,
  C1) and "a support listing with a fourth module, or one missing, differs". The
  top-level assertion also asserts that `test/support` is a directory. Each directory is
  read with `simplifile.read_directory`; since the checkpoint the top level's entries
  are read with whether each is a file (C1). The `////` header gains a sentence on the
  layout. GD09's witness now allows `support/harness` and `support/runner`, and its
  control asserts that bare `harness` and `runner` are refused (departure 2).
- **The words.** In `gleam-double/README.md`, `test/harness.gleam`, twice, and
  `test/coverage.gleam`, once, now read `test/support/...`, and the "Test support" row of
  "Where to look" names the three new paths. The "The tests" row already read as the
  ticket asks at the base and is unchanged. `docs/project/repository-map.md` did not
  change: its `gleam-double/` row and its tree's `test/` line say that `test/` holds the
  runner, the fakes and the tests, which stays true, so the `review-docs` skill was not
  owed.

The test count: 347 at the base, 351 from step 2 on, the baseline's plus the pin's 4. The
coverage check's constants stay 93, 32 and 35, and every green run below reports
`coverage faults: 0`.

### 4. The red steps, and steps 3 to 5

Every output in this section ran uncommitted, on base
`d37b481b30afc31393f98223ab9eef1ec89f6e79`. The implementing subagent ran on
`claude-opus-5-5`, by the lead's account. `just gleam-double-format` ran before the
`just gleam-double` of steps 2 and 3 (`ev-step2-format.txt`, `ev-step3-format.txt`,
each `exit status: 0`, both last written at 04:51:20Z). At step 2 it changed nothing; at
step 3 it moved lines the hand edits did not (`format-diff.txt`, departure 1).

#### Step 2: red, the pin

`ev-red-pin.txt`, last written at 04:46:35Z. Its second line records that both controls
were registered; exactly the two tree assertions fail, and both controls pass.

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 with uncommitted changes
...
FAIL interface_test.test/ holds at its top level only test modules and support/: crashed: ...
  ... #(Value, Ok(["coverage.gleam", "harness.gleam", "runner.gleam"])) ... #(Value, Ok(["support"])) ...
FAIL interface_test.test/support/ holds exactly coverage, harness and runner: crashed: ...
  ... #(Value, Error(Enoent)) ... #(Value, Ok(["coverage.gleam", "harness.gleam", "runner.gleam"])) ...
351 tests run, 349 passed
coverage faults: 0
...
exit status: 1
```

The first fails at `interface_test.gleam:1516`, `test_top_level`, on the three support
modules still at the top level; the second at line 1523, `support_top_level`, because
`test/support/` cannot be read. At this point the first control was still named "a top
level that holds a module that is not a test, or no support, differs".

#### Step 3: green, the move

`ev-green-move.txt`, last written at 04:47:15Z:

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 with uncommitted changes
# baseline 347 tests + the pin's 4 new tests = 351
$ just gleam-double
...
351 tests run, 351 passed
coverage faults: 0
exit status: 0
```

The scratch module, `ev-extra-test.txt` (last written at 04:47:30Z):
`gleam-double/test/support/extra_test.gleam` held one test and was listed nowhere.

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 with uncommitted changes
...
FAIL interface_test.test/support/ holds exactly coverage, harness and runner: crashed: ...
FAULT support/extra_test.gleam is on disk and not listed in main
FAULT support/extra_test.gleam is on disk and main does not import it
351 tests run, 350 passed
coverage faults: 0
...
exit status: 1
```

The one failing test is the pin's `support/` assertion, whose `Left` lists
`extra_test.gleam` beside the three, as it must with a fourth module there; the scratch
test itself was never registered, so it did not run. The runner names it by its path
under `test/`, `support/extra_test.gleam`. The file was then deleted: the `git status --short`
that follows shows no trace of it, `ls` of `test/support/` lists the three modules, and
the file ends `no file under gleam-double/ (outside build/) names extra_test`.

#### Step 4: the moved modules

`ev-moves-diff.txt` (last written at 04:47:39Z) holds
`git diff -M d37b481b30afc31393f98223ab9eef1ec89f6e79` for the three modules, old and new
paths. `ev-moves-imports-only.txt`, from the same second:

```text
d37b481b30afc31393f98223ab9eef1ec89f6e79 with uncommitted changes
harness: rename=yes changed_lines=0 headers=0 non_import_changed_lines=[]
runner: rename=yes changed_lines=0 headers=0 non_import_changed_lines=[]
coverage: rename=yes changed_lines=6 headers=2 non_import_changed_lines=[]
```

`coverage`'s four changed lines are `-import runner.{`, `+import simplifile`,
`+import support/runner.{` and `-import simplifile`; the name list's continuation line
and `}` are context lines, unchanged.

#### Step 5: words

`ev-step5-gleam.txt`: `351 tests run, 351 passed`, `coverage faults: 0`, exit status 0,
which includes `guide_test`'s check of the guide's excerpts. `ev-step5-docs.txt`:
`Validated 49 pages and 50 canonical topics.`, exit status 0. Both were last written at
04:48:07Z.

`ev-probes-precommit.txt` (last written at 04:48:54Z) ran `probes-precommit.sh`, the
implementer's Verification without `git rev-parse HEAD`, `just check` and the
`git status` test, under `sh -x`; it ends on `just check-docs`,
`Validated 49 pages and 50 canonical topics.`, exit status 0. That script predates the
checkpoint's fixes.

### 5. Checkpoint record

One checkpoint, step 6: conformance and breakage, before the work commit, over the base
and the uncommitted diff. There was no `design` lens.

- **Reviewer and model.** One reviewer subagent per lens started Codex, `gpt-6-astra`,
  in the form `ai_tmp/review/reviewer-subagent.md` gives: `codex exec -m gpt-6-astra -s
  read-only -C "$PWD" -o "$DIR/last-$LENS.md" "$(cat "$DIR/brief-$LENS.txt")" <
  /dev/null >| "$DIR/log-$LENS.txt" 2>&1 &`. The exact command lines are not saved. Each
  `log-<lens>.txt` opens `OpenAI Codex v0.169.1`, `model: gpt-6-astra`,
  `sandbox: read-only`, `reasoning effort: ultra`; the process ids are in
  `pid-<lens>.txt` (last written at 04:52:09Z and 04:52:10Z). `codex exec --help` for the
  day is `ai_tmp/review/codex-exec-help-2026-11-23.txt` (`codex-cli 0.169.1`), last
  written at 2026-11-24T02:59:30Z, for GD09, the same UTC day; it was not rerun. Codex
  was on `PATH`, so no fallback was needed. Every subagent in this ticket ran on
  `claude-opus-5-5`, by the lead's account.
- **Exit and recovery.** Both Codex processes exited normally, by the lead's account;
  `last-conformance.md` was last written at 04:55:08Z and `last-breakage.md` at
  04:57:08Z. Each `findings-<lens>.md` opens `Source: last-message`, and the rest of it
  matches its `last-<lens>.md` (`diff`, for this record).
- **Brief.** `brief-<lens>.txt` is `brief-common.txt` followed by `lens-<lens>.txt`
  (`cmp`, for this record). It names the ticket, the series index, the shared rules,
  §11 and §14, the base, `git diff -M d37b481b30afc31393f98223ab9eef1ec89f6e79` plus
  every untracked file read in full, the read limits, the step-6 questions and the
  uncommitted `verification.sh` as Verification. The conformance lens conforms to
  "Decisions fixed", "What the pin gains", "Files touched" and "Acceptance criteria", not
  to the specification. The brief says what is still to come and gives no hand-back
  conclusion. The two `lens-<lens>.txt` were last written at 03:52:58Z, before
  `base.txt`'s last write.
- **Triage.** By the lead's account, one separate subagent, neither a reviewer nor the
  lead, checked each finding by a probe and applied the fixes; its verdicts are
  `verdicts.md` (last written at 05:03:52Z).
- **Rounds.** One. No finding was high, so no second round was owed.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | The pin and Verification told entries apart by name alone, so an empty directory `test/extra_test.gleam/` passed the run, Verification's `find` and the runner | `not_tests` reads each entry with whether it is a file, filled by a new helper `kinds`; the control gains such a directory; Verification reads the top level with `ls -Ap`, with a control |
| B2 | low | valid | The same gap as C1, in the shape of a directory `test/extra_test.gleam/` that holds `notes.txt` | C1's fix |
| B1 | medium, taken as low | valid | A rename that git reports as `Binary files ... differ` has no `+` or `-` lines, so the imports-only count passes on a changed body; it needs a NUL in a Gleam source | The loop's `git diff` takes `--text`, and a probe with its control stops the script on a `Binary files` line |

Handed back, by `verdicts.md`: `counts` still reads a removed line that starts with `--`
and a space as a header, so such a body change would pass its equality. The script's
comment says so, no line of the three modules starts with `--` or `++` and a space at
the base (`rg -c '^(-- |\+\+ )'` printed nothing), and no reviewer raised it, so it was
not changed.

The evidence, all in `ai_tmp/review/GD11-1/` and run at
`d37b481b30afc31393f98223ab9eef1ec89f6e79` with uncommitted changes, by `verdicts.md`'s
account; the records themselves name no commit:

- `ev-C1-gap-tree.txt` (04:58:33Z), the gap before any fix, with the empty directory
  made by `mkdir` by `verdicts.md`'s account (the file opens `$ just gleam-double`):
  `351 tests run, 351 passed`, `coverage faults: 0`, exit status 0.
- `ev-C1-red.txt` (05:01:15Z), the new control on the unchanged filter:
  `FAIL interface_test.a top level that holds a module that is not a test, a directory
  named like one, or no support, differs: crashed: ...`, at `#(Line, 1525)` in
  `reads_test_top_level`, `Left` `["support"]`, `Right` `["extra_test.gleam",
  "support"]`, `351 tests run, 350 passed`, exit status 1. After the fix,
  `ev-C1-green.txt` (05:01:24Z): `351 tests run, 351 passed`, exit status 0.
- `ev-C1-red-tree.txt` (05:01:31Z), B2's shape on the tree after the fix:
  `FAIL interface_test.test/ holds at its top level only test modules and support/:
  crashed: ...`, at `#(Line, 1549)`, `Left` `Ok(["extra_test.gleam", "support"])`,
  `Right` `Ok(["support"])`, `351 tests run, 350 passed`, exit status 1. The scratch
  file and directory were then removed, by `verdicts.md`'s account.
  `ev-B2-red.txt` (05:05:47Z, after `verdicts.md`) is byte-identical to it (`cmp`, for
  this record), is not cited by `verdicts.md`, and is no separate run.
- `ev-B1-red.txt` (05:02:45Z), reproduced only in a scratch git repository outside the
  tree, by its first line: `runner.gleam` renamed to `support/`, a NUL in line 1 and
  `test_timeout` changed from 5000 to 5001. The loop as it stood reads
  `Binary files a/gleam-double/test/runner.gleam and b/gleam-double/test/support/runner.gleam differ`
  and passes, `body check passed: changed=0 headers=0 imported=0`, `exit 0`; the fixed
  loop prints `-pub const test_timeout = 5000` and `+pub const test_timeout = 5001`,
  `exit 1`; the `Binary files` probe alone, without `--text`, `exit 1`.
- `ev-verif-controls.txt` (05:02:37Z): on a hand-made top level that holds
  `extra_test.gleam/`, `others=extra_test.gleam/ support/` and the new probe exits 1; the
  old name-only filter on the new control's text exits 1, and the new control counts 3
  and exits 0; on a hand-made binary patch the old body check exits 0, the new one
  exits 1, and the `Binary files` control counts 1 and exits 0.
- `ev-block-match.txt` (05:02:56Z): `diff ticket-block.sh script-tail.sh`,
  `exit status: 0`, the ticket's block against `verification.sh` from its line 7.
  `verification-before.sh` is the triage's copy of the script before its fixes.

Applied by the triage to `gleam-double/test/interface_test.gleam`, the ticket's
Verification block and the paragraphs around it, and `ai_tmp/gd/GD11/verification.sh`.
No test was added in triage, so the count stays 351. The reruns after the fixes, on base
`d37b481b30afc31393f98223ab9eef1ec89f6e79` with uncommitted changes: `ev-format.txt`
(`just gleam-double-format`), exit status 0; `ev-final.txt` (05:03:16Z),
`351 tests run, 351 passed`, `coverage faults: 0`, exit status 0; `ev-docs.txt`
(`just check-docs`), `Validated 49 pages and 50 canonical topics.`, exit status 0; and
`ev-verif-smoke.txt` (05:03:03Z), the corrected `verification.sh` without its `just`
lines and its final `git status` test, under `sh`. That file holds only the run's tail,
which shows coverage's `imported=4`, the `seen`, `sources`, `pins` and `dumps` checks,
and `exit 0`; its `others=support/` and `tests=24` are `verdicts.md`'s account.

### 6. Verification at the work commit

At `421e24682999e8bf7a5813f6af1e95f859dfa5fc`, the work commit, on a clean tree:
`ev-verification-at-work.txt`, last written at 05:06:33Z, is
`ai_tmp/gd/GD11/verification.sh` under `sh -x`. The script is `#!/bin/sh`, four comment
lines and `base=$(cat ai_tmp/gd/GD11/base.txt)`, followed from line 7 by this ticket's
corrected block verbatim, which opens with `set -eu` and `git cat-file -e`; a `diff` of
the block against the script's tail, made for this record before it was written, shows
nothing else. At the run the corrected block was not yet committed in this ticket, and
`git status --short` printed nothing, so the ticket's edit was set aside for the run as
`ai_tmp/gd/GD11/ticket-triage.patch` (last written at 05:06:05Z). That patch matches the
ticket's uncommitted diff before this record was written (`cmp`, for this record).

It exited 0, so every `test` held: `test/support/` holds exactly the three modules; the
top level, read with `ls -Ap`, holds besides `*_test.gleam` files only `support/`, and
24 entries named like test modules, the entry module among them; no import of a support
module by its old name under `gleam-double/test/`, and the entry module's four imports
spelt as expected; no old path in the guide, `SPEC-NOTES.md`, `src/` or the repository
map; `harness` and `runner` renames with no changed line, and `coverage` a rename whose
changed lines are its two headers and four import lines; no `Binary files` line;
nothing under `gleam-double/src/`, and neither `gleam.toml` nor `manifest.toml`,
changed; no `erl_crash.dump`. Each probe's positive control printed its expected count
first. The two `273 tests run: 273 passed, 0 skipped` lines inside `just check` are the
Rust workspace's tests, not the double's.

```text
$ sh -x ai_tmp/gd/GD11/verification.sh
++ cat ai_tmp/gd/GD11/base.txt
+ base=d37b481b30afc31393f98223ab9eef1ec89f6e79
+ set -eu
+ git cat-file -e 'd37b481b30afc31393f98223ab9eef1ec89f6e79^{commit}'
+ git rev-parse HEAD
421e24682999e8bf7a5813f6af1e95f859dfa5fc
+ just gleam-double
...
351 tests run, 351 passed
coverage faults: 0
...
+ support='coverage.gleam harness.gleam runner.gleam '
...
+ others=support/
+ test support/ = support/
...
+ tests=24
+ test 24 -gt 0
...
+ probe=4
+ test 4 -eq 4
...
+ test 6 -ne 4
...
+ headers=2
...
+ changed=6
...
+ imported=4
+ test 6 -eq 6
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
++ git status --short
+ status=
+ test -z ''
exit status: 0
```

### 7. Departures

1. **The formatter moved lines the ticket did not name** (`format-diff.txt`, the
   interdiff of the uncommitted diff before and after step 3's
   `just gleam-double-format`). It moved each `import support/...` to its sorted place
   among the imports, below `import simplifile` where a module imports it, and rewrapped the string list of the
   pin's `import_lines()` fixture. At step 2 it changed nothing.
2. **The witness change went past "two entries".** Besides the allowed list's two
   entries, the pin's `import_lines()` fixture and the expected list of `reads_imports`
   now spell `support/harness` and `support/runner`, `witness_allows`' doc comment names
   them, and `reads_witness` gains two asserts that bare `harness` and `runner` are
   refused.
3. **The pin is 4 tests and a header sentence**: the two assertions and their two
   controls, and a sentence in the `////` header on the layout. The top-level assertion
   also asserts that `test/support` is a directory with `simplifile.is_directory`.
4. **The implementer corrected Verification before the checkpoint**, in
   `verification.sh`. Each producer fills a variable of its own
   (`ev-verif-fix-producer.txt`: the old form `exit status: 0` on a failing `ls`, the
   new `exit status: 1`). The `body` line became `counts`, which lets only `rg`'s exit 1
   through and checks itself on a hand-made patch (`ev-verif-fix-body.txt`: the old line
   `exit status: 0` on a base that names no commit, the new form `exit status: 128`).
   The loop also asserts each module's `rename from` and `rename to` lines, and reads an
   empty count as zero with `${changed:-0}` and its neighbours. The old-import control gained the fixture's spelling, `import runner_test`,
   `import coverage_test` and `import harness2`, and expects 4
   (`ev-verif-fix-import.txt`: without `\b` it counts 7). The old-path control takes all
   three old paths and expects 3, and the probe also reads `gleam-double/src`
   (`ev-verif-fix-path.txt`). The two `git diff --name-only` probes gained the `seen`
   control (`ev-verif-fix-seen.txt`: on an unchanged path it stops, exit status 1), and
   a probe that neither pin file changed. The block checks `base` first, and checks the
   entry module's four imports.
5. **The layout pin reads each entry's kind** (C1, B2). The ticket's design read names
   only. `not_tests` now takes each entry with whether it is a file, and only a file
   whose name ends in `_test.gleam` counts as a test module. Its control holds such a
   directory, and its name gains "a directory named like one". In Verification, the top
   level is read with `ls -Ap` instead of `find`, with a control on a hand-made listing.
6. **Verification's imports-only check forces a text diff** (B1). The loop's `git diff`
   takes `--text`, and a probe with its control stops the script on a `Binary files`
   line.
7. **The import count at the base differs from the ticket's**: 47 lines in 25 files by
   the lead's account, not 45 in 24, as the ticket foresaw for GD08's additions. The
   base count is not recorded (section 2).
8. **The ticket's Verification block and the paragraphs around it were rewritten**, by
   the implementer's corrections and the triage's two fixes, though "Files touched"
   lists this ticket only for `status:` and the hand-back. The paragraph before the
   block describes the `base` check and the rule that every command whose failure
   matters runs alone in an assignment or last in its pipeline; the paragraph after it
   describes the `ls -Ap` layout, the `counts` equality, `--text` and the
   `Binary files` probe.

### 8. What was not run, and what was not recorded

Not run:

- A push or a pull request. Both need authorisation (step 7); nothing left the worktree.
- `just initialize`, `just gleam-double-deps` or any other network recipe. The review's
  model calls are the only network use, which the checkpoint authorises.
- A second review round. None was owed.
- A review of the triage's fix diff.
- The corrected Verification in full before the work commit. The smoke run left out its
  `just` lines and the `git status` test (section 5); section 6's run is the first full
  one.
- The `review-docs` skill, since `docs/project/repository-map.md` did not change.

Not recorded: the import count at the base; the exact `codex exec` command lines, beyond
the form in `ai_tmp/review/reviewer-subagent.md`; the Codex processes' exit statuses,
beyond the lead's account; the head of the triage's smoke run; and the models of the
subagents, beyond the lead's account.

## Open points

- **The engine's tests sit beside the caller's.** After GD10 a reader can tell the
  engine from the interface under `gleam-double/src/`. Under `gleam-double/test/`,
  `core_test.gleam` and `guide_test.gleam` are still neighbours. Sorting the test
  modules the same way was not asked for.
- **`harness.gleam` holds two things**: the fakes a caller would inject, and helpers
  that reach into the engine, such as `drive`. Splitting it is left out; the maintainer
  chose to move the three modules as they are.
- **`support` is a short module prefix.** A test module is then `support/harness` to the
  compiler. No package the double takes has a module of that name. If one ever does,
  the prefix has to change.
- **GD07's open point 8 stands.** What the runner's checks miss, a `main` that drops a
  module's `tests()` among them, is not changed by the move.
