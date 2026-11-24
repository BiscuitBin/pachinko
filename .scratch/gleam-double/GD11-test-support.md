---
id: GD11
title: "Gleam double: test support under test/support/"
status: todo
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
that fails. `base` is the commit step 1 records; set it before the script runs. Each
probe that passes by printing nothing follows its positive control, and no probe reads
`rg`'s status through a pipe.

```sh
set -eu
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

support=$(ls gleam-double/test/support | LC_ALL=C sort | tr '\n' ' ')
test "$support" = "coverage.gleam harness.gleam runner.gleam "
test -d gleam-double/test/support
others=$(find gleam-double/test -mindepth 1 -maxdepth 1 ! -name '*_test.gleam' -print)
test "$others" = "gleam-double/test/support"
tests=$(find gleam-double/test -mindepth 1 -maxdepth 1 -name '*_test.gleam' -print)
test -n "$tests"                        # positive control: the find sees the tree

pattern='^import (harness|runner|coverage)\b'
probe=$(printf 'import harness.{type Double}\nimport runner\nimport coverage\nimport support/runner\n' | rg -c "$pattern")
test "$probe" -eq 3
if rg -n "$pattern" gleam-double/test; then exit 1; else test "$?" -eq 1; fi
imports=$(rg -l '^import support/(harness|runner|coverage)\b' gleam-double/test)
test -n "$imports"

pattern='test/(harness|runner|coverage)\.gleam'
probe=$(printf 'test/harness.gleam\ntest/support/harness.gleam\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" gleam-double/README.md gleam-double/SPEC-NOTES.md docs/project/repository-map.md; then exit 1; else test "$?" -eq 1; fi

for module in harness runner coverage; do
  body=$(git diff -M "$base" -- "gleam-double/test/$module.gleam" "gleam-double/test/support/$module.gleam" | rg '^[+-]' | rg -v '^(\+\+\+|---) ' | rg -cv '^[+-]import ' || true)
  test -z "$body"
done
sources=$(git diff --name-only "$base" -- gleam-double/src)
test -z "$sources"

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The `body` lines count the changed lines of each moved module that are not `import`
lines, and expect none: `rg -c` prints nothing and exits 1 when it counts zero, which
`|| true` allows. If `gleam format` rewraps an import of a moved module over several
lines, that count is no longer zero; the builder then shows the same thing with a saved
diff and says so under Departures. The builder runs each control first, and corrects a
pattern, saying so under Departures, if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the red steps; the checkpoint's record; Verification at the work commit; departures; and
what was not run.

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
