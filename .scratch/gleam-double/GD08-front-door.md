---
id: GD08
title: "Gleam double: the front door, with the owner as the root module"
status: done
depends_on: []
parallel_with: []
branch: build-gleam-double
estimated_size: M
---

# GD08: Gleam double: the front door, with the owner as the root module

## Context

GD01 to GD07 are done. The double works, and nothing in it says which part is its
interface. The facts below are those of commit
`aebae0afadfb34a2d8ae762316376f70d3df08fe`.

- The package has no root module. `gleam-double/src/` holds only the directory
  `pachinko_double/`, with 18 modules side by side, and `gleam-double/gleam.toml` marks
  none of them internal.
- A caller starts at `gleam-double/src/pachinko_double/owner.gleam`. It holds 17 `pub`
  items: `start`, the eight trigger functions, `tick`, the two reads, and the five names
  around them.
- Seventeen modules under `gleam-double/test/` import `pachinko_double/owner`. Five of
  them call it by its qualifier, 31 times in all: `harness.gleam` (13), `owner_test.gleam`
  (13), `branch_a_test.gleam` (2), `competing_test.gleam` (2) and `decay_test.gleam` (1).
  The other twelve import only `type Reply`, `Outcome` or `type PartialTick` from it.
- No module under `gleam-double/src/` imports `owner`: it is the top of the import graph.

The maintainer asked for one place a reviewer goes to see the production interface, with
the test-only parts, the test harness and the other concerns easy to tell apart. The
Python double got that in PD09, which is on `build-python-double` at commit
`ba06b5e6aeb0b33f5590a043296c879bc5262653` and not on this branch. PD09 is named here for
its shape only. It could re-export names. A Gleam module has no re-export, and a type
alias carries no constructors, so a type a caller builds or matches on is public only
where it is defined. The Gleam answer is therefore four tickets of moves, GD08 to GD11,
and this is the first: the owner becomes the package's root module, the one file a
reader opens first. Open points says what the alternative for the functions alone would
have been.

Every count in this ticket is given beside the command that prints it, so the builder
reruns the command at the base and uses what it prints.

## Goal

`import pachinko_double` gives `start`, every trigger, `tick` and the two reads. The root
module's header is the map of the package. A new test, the pin, holds the root's 17
names, and holds that the root is the only module under `gleam-double/src/` that imports
`gleam/erlang` or `gleam/otp`.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer fixed the first group on 2026-11-23. The builder does not reopen them.

- **The root module is the owner, moved there whole.**
  `gleam-double/src/pachinko_double/owner.gleam` moves with `git mv` to
  `gleam-double/src/pachinko_double.gleam`. No file named `owner.gleam` is left. "The
  owner" stays the name of the concept: the one process that holds the queue's state.
- **Its 17 items are unchanged**: the same names, the same types and the same bodies.

  | Group | Items |
  | --- | --- |
  | Starting | `start`, `Message`, `call_timeout` |
  | The `Publishing` surface | `publisher_joins`, `publish` |
  | The `Consuming` surface | `consumer_joins`, `pull`, `renew`, `ack`, `nack`, `release`, and the read `lease_view` |
  | The clock | `tick` |
  | What they answer | `Reply`, `Outcome`, `PartialTick` |
  | The inspection seam | `inspect` |

  `Outcome` and `PartialTick` each have one constructor, of the type's own name.
  `Message` is opaque and `Reply` is an alias, so neither has a constructor a caller can
  name.
- **The header is the map.** The root's `////` header groups the 17 items as the table
  above does, and names each one in backticks. It labels three things as the double's
  own channels for tests and for the coverage check, not something the specification
  answers: `inspect`, `Outcome.fired` and `TickReport.fired`. It then says where a
  caller's other names live, as they stand at this ticket's commit: `config`, `time`,
  `model` and `refusals` for what a caller names, `inspection` and `invariants` for the
  snapshot and the checker, and the types a caller sends or a reply carries that still
  sit in engine modules (`rules.Failure`, `domain.Answer`, `core.TickReport`, `consuming.LeaseView` and
  `publish.Publication`). The first four are what a reply carries, and GD09 moves them.
  The last is what a caller sends, and GD09 takes it out of the root's arguments. The
  header states what is true at the commit and names no ticket.
- **Shared rule 4 is restated.** See "Shared rules".
- **Tests call the root by its name.** A module that imported `pachinko_double/owner`
  imports `pachinko_double`, with the same unqualified names, and writes
  `pachinko_double.start` where it wrote `owner.start`. No import takes the root under
  an alias such as `as owner`.
- **The guide gains "Where to look".** `gleam-double/README.md` gets a table with that
  heading, placed before "The specification's parts and where they live". Its rows, at
  this ticket's commit:

  | A reader looking for | Opens |
  | --- | --- |
  | The interface: starting the owner, the triggers, the clock and the two reads | `src/pachinko_double.gleam` |
  | What a caller names when it sends a trigger or reads a reply | `config.gleam`, `time.gleam`, `model.gleam` and `refusals.gleam`, and for now the reply types in `rules.gleam`, `domain.gleam`, `core.gleam` and `consuming.gleam`, and `Publication` in `publish.gleam` |
  | The snapshot and the invariant checker | `inspection.gleam` and `invariants.gleam` |
  | The engine: the rule-table machinery and the apply step | `rules.gleam`, `domain.gleam` and `core.gleam` |
  | The specification's rules | `identity.gleam`, `publish.gleam`, `pull.gleam`, `settle.gleam`, `lease_end.gleam`, `expiry.gleam`, `decay.gleam` and `consuming.gleam` |
  | The tests | `test/*_test.gleam`, run from `test/pachinko_double_test.gleam` |
  | Test support: the fakes, the runner and the coverage check | `test/harness.gleam`, `test/runner.gleam` and `test/coverage.gleam` |

  GD09, GD10 and GD11 each change the rows their moves touch.
- **The pin is `gleam-double/test/interface_test.gleam`**, in the style of
  `gleam-double/test/coverage.gleam`: a pure parser over source text, written with line
  prefixes and the standard library's string functions, and a reader that hands it files
  through `simplifile`. It adds no package. Its parser has controls: tests that run it on
  source text made by hand and show that its result changes when a `pub` item is added,
  when one is removed, and when an `@internal` line is dropped.

The second group is this ticket's own design, written from a scan of the tree at the
commit Context names. The builder follows it, and reports under Departures if one of
these cannot hold.

- **The pin is registered.** It is imported and listed in
  `gleam-double/test/pachinko_double_test.gleam`, as the runner requires of every
  `*_test.gleam`.
- **The pin's unit.** GD09 to GD11 extend this parser, so its unit is fixed once, here.
  - An *item* is a line that starts at column 0 with `pub fn`, `pub const`, `pub type` or
    `pub opaque type`. Its name is the word that follows, which ends at the first
    character that is not a letter, a digit or `_`.
  - A `pub type` line that ends in `{` opens a *body*, which runs to the next line that
    is `}`. A `pub type` line that does not end in `{` is an alias and has no body.
  - A type's *constructors* are the lines of its body that start with exactly two spaces
    and an uppercase letter. An opaque type and an alias have none.
  - An item is *internal* when the line directly above its `pub` line is `@internal`. A
    constructor is internal when its type is.
  - The parser returns a module's items in the file's order, each with its constructors
    and its internal mark.
- **One more control, and how controls are written.** A fourth control shows the
  parser's result changes when a constructor is added. Nothing in `gleam-double/src/`
  carries `@internal` at this ticket's commit, so the control for the mark is the only
  place it is exercised until GD10. The text a control reads is written with `\n`
  escapes or indented, here and in GD09 to GD11, so that no line of
  `interface_test.gleam` other than its own imports starts with `import`: later
  Verification scripts search `gleam-double/test` for import lines.
- **What the pin asserts of the tree.**
  - The root's items are exactly the 17 above, with those constructors and none
    internal. The comparison ignores order: the file's order is not the header's
    grouping, and moving a function is not a change of interface.
  - The modules under `gleam-double/src/` that hold a line starting with
    `import gleam/erlang` or `import gleam/otp` are exactly `src/pachinko_double.gleam`.
  - `gleam-double/src/` holds, at its top level, exactly `pachinko_double.gleam` and the
    directory `pachinko_double`.
- **`owner_test.gleam` keeps its name.** It tests the owner, which is still the concept's
  name. Variables and record fields named `owner`, such as `double.owner`, stay.

## Non-goals

- No behaviour change. No function body changes, and no item of the root changes its
  name or its type.
- No test under a `Scenario` tag changes beyond its import line and the qualifier of a
  call.
- No type moves between modules: that is GD09. No module moves under `internal/`: that
  is GD10. No file under `gleam-double/test/` moves: that is GD11.
- `inspect`, `Outcome.fired` and `TickReport.fired` are labelled, not changed.
- No change to `gleam-double/gleam.toml`, `gleam-double/manifest.toml`, the `Justfile` or
  anything under `docs/specs/`, `docs/decisions/`, `crates/` or `double/`.
- No new spec note. The change interprets nothing in the specification (shared rule 5).

## Shared rules

Shared rules 1 to 6 bind this ticket as `.scratch/gleam-double/GD07-close-out.md` writes
them under "Shared rules (GD01 to GD07)". They are not copied here. Read them with these
three notes.

- **Rule 4, "A pure core", is restated.** Its second sentence names the module `owner`.
  From this ticket's commit on, it reads: "The root module,
  `gleam-double/src/pachinko_double.gleam`, which is the owner, is the only module under
  `gleam-double/src/` that imports `gleam/erlang` or `gleam/otp`." Nothing else in rule 4
  changes. GD09 to GD11 cite this restatement.
- **Rule 3.** GD07 deleted the pending lists, so the coverage check is strict. Its three
  counts stay 93, 32 and 35.
- **Rule 1.** `double/pachinko_double/` and `double/tests/` stay unread. PD09 is named in
  Context for its shape; the builder does not open it.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/gleam-double/GD07-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `GD08`, and `<n>` is 1: this ticket has one checkpoint, at step 6.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names this ticket's path and its "Decisions fixed in this
  ticket". It does not name `docs/specs/pachinko.allium` as the thing to conform to: the
  ticket changes no behaviour.
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff, as PD09's did.
- A runner polls on `GD08-1/last-<lens>.md` or the process id, never the bare file name:
  a Codex run in another worktree matches the bare name.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `gleam-double/src/pachinko_double/owner.gleam` | moved away | `git mv` to the path below |
| `gleam-double/src/pachinko_double.gleam` | new, by the move | The owner, and the header of "Decisions fixed" |
| `gleam-double/test/interface_test.gleam` | new | The pin |
| `gleam-double/test/pachinko_double_test.gleam` | shared, own lines | Imports and lists `interface_test` |
| The 17 modules under `gleam-double/test/` that import `pachinko_double/owner` | shared, own lines | The import, the qualifier of each call, and any comment that names `owner.gleam` or `owner.<name>` |
| `gleam-double/README.md` | shared, own lines | "Where to look", and the lines that name the module |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | The lines that name the module |
| `docs/project/repository-map.md` | shared, own lines | The `gleam-double/` row's `owner.gleam` |
| `.scratch/gleam-double/GD08-front-door.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD08's Status cell |

The 17 modules are `branch_a_test`, `competing_test`, `config_test`, `consuming_test`,
`core_test`, `decay_test`, `expiry_test`, `guide_test`, `harness`, `identity_test`,
`lease_endings_test`, `lease_test`, `owner_test`, `parking_test`, `publish_test`,
`pull_test` and `settle_test`. The command is
`rg -l '^import pachinko_double/owner' gleam-double/src gleam-double/test`.

The lines that name the module, with the command for each:

| File | Lines at the base | Command |
| --- | --- | --- |
| `gleam-double/README.md` | 9 | `rg -n '\bowner\.[a-z_]+\b' gleam-double/README.md` |
| `gleam-double/SPEC-NOTES.md` | 4: `owner.gleam` twice, `owner.tick` once, and the sentence that opens "`owner` is the only module under `src/`" | The command below the table |
| `docs/project/repository-map.md` | 1 | `rg -n 'owner\.gleam' docs/project/repository-map.md` |

```sh
rg -n '\bowner\.(gleam|tick)\b|`owner`' gleam-double/SPEC-NOTES.md
```

In `gleam-double/SPEC-NOTES.md` the wording changes and no note changes what it claims.
A sentence that says "the owner" for the process stays as it is.

## Steps

Each red step quotes the failing test's name and its message verbatim, headed by the
full commit id and "with uncommitted changes". Evidence goes under `ai_tmp/gd/GD08/`.

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). The base is the commit that holds this ticket, GD09 to GD11 and the
   series README's rows for them: all five files are committed before this step, so the
   worktree is clean. It is a later commit than the one Context names, and holds the
   same tree under `gleam-double/`. Record its full commit id. If `.pixi/`, `.tools/` or
   `gleam-double/build/packages/` is missing, run `just initialize` and
   `just gleam-double-deps` (network, authorised by this step). Run `just check` and
   `just gleam-double` on the untouched worktree. Quote each one's last line and the
   line `N tests run, N passed`, headed by the commit.
2. **Red: the pin.** Add `gleam-double/test/interface_test.gleam` with the parser, its
   controls and the three assertions of "What the pin asserts of the tree", and register
   it in `gleam-double/test/pachinko_double_test.gleam`. Run `just gleam-double-format`
   first, since the recipe checks the format before it runs a test. The controls pass,
   because they read text made by hand. The three assertions fail: `src/pachinko_double.gleam` cannot
   be read, the importer is `src/pachinko_double/owner.gleam`, and the top level of
   `src/` holds no `pachinko_double.gleam`. Quote the failures.
3. **Green: the move.** `git mv` the file. In the 17 test modules change the import and
   the qualifier. Run `just gleam-double-format`, then `just gleam-double`. The pin
   passes and the count is the baseline's plus the pin's tests.
   `git diff -M "$base" -- gleam-double/src` shows the root as a rename with no changed
   line.
4. **The header.** Write the root's header as "Decisions fixed" describes it. Only `////`
   lines change in the file.
5. **Words.** Add "Where to look" to `gleam-double/README.md` and reword the lines of the
   table under "Files touched", in the three files and in any comment under
   `gleam-double/src/` and `gleam-double/test/`. Use the `review-docs` skill for
   `docs/project/repository-map.md`. Run `just gleam-double`, which checks the guide's
   blocks against `guide_test.gleam`, and `just check-docs`.
6. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Does any file under `gleam-double/` or `docs/` still name a module `owner`,
     `owner.gleam` or `owner.<name>` as if it existed?
   - Would the pin fail if a `pub` item were added to the root, removed from it or
     renamed, if a constructor were added to `Outcome`, or if a second module under
     `src/` imported `gleam/otp`?
   - Did the move change a function body, or the type of any of the 17 items?
   - Did any test under a `Scenario` tag change beyond its import and its qualifier?
   - Does the header say anything that is untrue at this commit, such as naming a module
     for a type that module does not hold?
   - Does the coverage check still collect every test, the pin's included, and stay
     strict at 93, 32 and 35?
   - Could a probe in Verification print nothing while its claim is false?
7. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions"), and a push would add
   to the open pull request for this branch.

## Acceptance criteria

- `gleam-double/src/pachinko_double.gleam` exists and
  `gleam-double/src/pachinko_double/owner.gleam` does not.
- The root holds exactly the 17 items of "Decisions fixed", and the pin asserts it.
- The root's header names each of the 17 in backticks, and labels `inspect`,
  `Outcome.fired` and `TickReport.fired` as the double's own channels.
- The root is the only module under `gleam-double/src/` that imports `gleam/erlang` or
  `gleam/otp`, and the pin asserts it.
- No file under `gleam-double/src/` or `gleam-double/test/` imports
  `pachinko_double/owner` or takes the root under an alias. No tracked file under
  `gleam-double/`, and not `docs/project/repository-map.md`, names `owner.gleam` or
  `owner.<item>`. Verification's probes print nothing.
- Under `gleam-double/src/`, the diff from the base is the move and the header: no line
  that is not a `////` line changes.
- Step 2 is quoted red before step 3 turns it green.
- `just gleam-double` exits 0 with the coverage check strict at 93, 32 and 35. Its test
  count is the baseline's plus the pin's tests.
- `gleam-double/README.md` holds the "Where to look" table with the seven rows above.
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

root=gleam-double/src/pachinko_double.gleam
test -f "$root"
test ! -e gleam-double/src/pachinko_double/owner.gleam
items=$(rg -c '^pub ' "$root")
test "$items" -eq 17
for name in start Message call_timeout publisher_joins publish consumer_joins pull renew \
  ack nack release lease_view tick Reply Outcome PartialTick inspect; do
  rg -q "^pub (fn|const|type|opaque type) $name\b" "$root"
  rg -q "^////.*\`$name\`" "$root"
done

pattern='^import gleam/(erlang|otp)'
probe=$(printf 'import gleam/erlang/process\nimport gleam/otp/actor\n' | rg -c "$pattern")
test "$probe" -eq 2
importers=$(rg -l "$pattern" gleam-double/src)
test "$importers" = "$root"

pattern='^import pachinko_double/owner\b|^import pachinko_double(\.\{[^}]*\})? as '
probe=$(printf 'import pachinko_double/owner.{Outcome}\nimport pachinko_double as owner\nimport pachinko_double.{\n  type Reply, Outcome,\n} as queue\nimport pachinko_double.{type Reply}\n' | rg -U --count-matches "$pattern")
test "$probe" -eq 3
if rg -nU "$pattern" gleam-double/src gleam-double/test; then exit 1; else test "$?" -eq 1; fi

names='start|publisher_joins|consumer_joins|publish|pull|renew|ack|nack|release|lease_view|tick|inspect|call_timeout|Outcome|Reply|PartialTick|Message'
pattern="\bowner\.(gleam|$names)\b"
probe=$(printf 'owner.gleam\nowner.start(config)\nowner.Outcome\n' | rg -c "$pattern")
test "$probe" -eq 3
if rg -n "$pattern" gleam-double/src gleam-double/test gleam-double/README.md \
  gleam-double/SPEC-NOTES.md docs/project/repository-map.md; then exit 1; else test "$?" -eq 1; fi

changed=$(git diff --no-renames --name-only "$base" -- gleam-double/src | LC_ALL=C sort | tr '\n' ' ')
test "$changed" = "gleam-double/src/pachinko_double.gleam gleam-double/src/pachinko_double/owner.gleam "
patch=$(git diff --no-ext-diff --no-color -M "$base" -- gleam-double/src)
printf '%s\n' "$patch" | rg -q '^rename to gleam-double/src/pachinko_double\.gleam$'
pattern='^(?!(\+\+\+|---) (a/|b/|/dev/null))[+-](?!////)'
probe=$(printf -- '--- a/x.gleam\n+++ b/x.gleam\n+//// a header line\n-  let x = 1\n+  let x = 2\n' | rg -Pc "$pattern")
test "$probe" -eq 2
if printf '%s\n' "$patch" | rg -nP "$pattern"; then exit 1; else test "$?" -eq 1; fi

registered=$(rg -c 'interface_test' gleam-double/test/pachinko_double_test.gleam)
test "$registered" -eq 2
rg -q '^## Where to look$' gleam-double/README.md
dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The alias probe runs `rg` with `-U`, so that `[^}]*` crosses lines and an import whose
name list wraps is seen whatever its alias. The patch probe takes the diff under
`gleam-double/src/` in an assignment of its own, so that `set -eu` stops the script if
`git diff` fails, and its positive control is the rename line. Its pattern matches a
changed line that is not a `////` line and not a file header, and its control shows it
counting the two changed body lines of a hand-made patch and passing over the `////`
line. The search then prints any such line and fails, and an `rg` error, which exits 2,
fails it too. The builder runs each control first, and corrects a pattern, saying so
under Departures, if one does not behave as written.

## Hand-back notes

Evidence files are under `ai_tmp/gd/GD08/`, and the review record under
`ai_tmp/review/GD08-1/`. None is committed. Each `ev-*.txt` begins `$ <command>` and ends
`exit status: N`, except where the text says otherwise; a file that names its commit does
so on its first line, before the `$` line. In every quote below the recipe's echo of its
own command lines is left out, and `...` marks other lines left out or a line cut short.
Times are UTC, read from the files' modification times; the work commit's own date is
2026-11-23T18:52:41-08:00, which is 2026-11-24T02:52:41Z. Where a fact the record should
hold was never saved, the text says "not recorded".

### 1. Base, branch and commits

- Branch: `build-gleam-double`, in this worktree (shared rule 6).
- Base commit: `52f54df6999da7c6b2223d0fe5cda3eabadbed07` (`base.txt`, written at
  02:36:55Z), the commit that records the maintainer's three rulings in GD08, GD09 and
  GD10. It holds this ticket, GD09 to GD11 and the series README's rows for them.
- Work commit: `5ceb284816f432e65ef0848c7b8554185a7ddfc0` (`work-commit.txt`), tree
  `311ca05a7aefaf8e7da08817e196ddf0905f7217`, titled "Make the Gleam double's owner its
  root module, with the map and the pin (GD08)". It touches 23 files: the move, the pin,
  the runner's registration, the 17 importing test modules, `gleam-double/README.md`,
  `gleam-double/SPEC-NOTES.md` and `docs/project/repository-map.md` (`git show --stat`).
- The commit after it holds this record, the index's Status cell, and the ticket's
  corrected Verification block (section 7, departure 3). The corrected block was not in
  the committed ticket at the work commit.
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `52f54df6999da7c6b2223d0fe5cda3eabadbed07`, on the untouched worktree. Neither file
names its commit; the base id rests on `base.txt`, written before both runs.

`ev-baseline-check.txt`, written at 02:38:06Z:

```text
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

`ev-baseline-gleam.txt`, written at 02:38:11Z:

```text
$ just gleam-double
...
    Running pachinko_double_test.main
310 tests run, 310 passed
coverage faults: 0
exit status: 0
```

### 3. What landed

- **The move.** `gleam-double/src/pachinko_double/owner.gleam` is now
  `gleam-double/src/pachinko_double.gleam`, by `git mv`; no `owner.gleam` is left. Before
  the header was written the move was a pure rename. At `52f54df6999da7c6b2223d0fe5cda3eabadbed07` with uncommitted
  changes (`ev-move-diff.txt`, written at 02:40:23Z with the green run, quoted whole; the
  file holds no `$` line and names no commit):

  ```text
  diff --git a/gleam-double/src/pachinko_double/owner.gleam b/gleam-double/src/pachinko_double.gleam
  similarity index 100%
  rename from gleam-double/src/pachinko_double/owner.gleam
  rename to gleam-double/src/pachinko_double.gleam
  ```

- **The header.** The root's `////` header names the 17 items in backticks, grouped as
  "Decisions fixed" groups them; labels `inspect`, `Outcome.fired` and `TickReport.fired`
  as the double's own channels, for tests and for the coverage check; and says where a
  caller's other names live, naming `rules.Failure`, `domain.Answer`, `core.TickReport`,
  `consuming.LeaseView` and `publish.Publication`. It says this is the only module under
  `src/` that imports `gleam/erlang` or `gleam/otp`. Only `////` lines changed under
  `src/` from the base (Verification's patch probe, section 6).
- **The pin**, `gleam-double/test/interface_test.gleam`, registered in
  `gleam-double/test/pachinko_double_test.gleam` (an `import interface_test` and a
  `Module("interface_test", interface_test.tests())` entry). It is a pure parser over
  source text with the unit "Decisions fixed" gives, and a `simplifile` reader. It adds
  no package. Its nine tests:
  - "an item is a pub line at column 0, with its constructors and its mark";
  - the four controls: "adding a pub item changes the items", "removing a pub item
    changes the items", "dropping an @internal line changes the items" and "adding a
    constructor changes the items";
  - "an effect import starts with import gleam/erlang or gleam/otp";
  - the three assertions of the tree: "the root holds exactly the interface's items",
    "the root is the only module under src/ that imports an effect" and "src/ holds only
    the root and its directory".
- **The tests.** The 17 modules that imported `pachinko_double/owner` import
  `pachinko_double` with the same unqualified names, and call `pachinko_double.<name>`
  where they called `owner.<name>`. No import takes the root under an alias.
  `owner_test.gleam` keeps its name.
- **The words.** `gleam-double/README.md` gains "Where to look", before "The
  specification's parts and where they live", with the ticket's seven rows. Its opening
  line says the package is "under `src/`" (departure 2). `gleam-double/SPEC-NOTES.md`
  names `pachinko_double.gleam` and `pachinko_double.tick` where it named `owner.gleam`
  and `owner.tick`, restates rule 4's sentence, and cites `interface_test` for the
  importer check (C2). `docs/project/repository-map.md`'s `gleam-double/` row names the
  root module as the interface and the owner. Whether the `review-docs` skill was used for
  the map is not recorded.

The test count: 310 at the base, and 319 from step 2 on, the baseline's plus the pin's
nine; all 319 pass from step 3.

### 4. The red step, and steps 3 to 5

Every output in this section ran uncommitted, on base
`52f54df6999da7c6b2223d0fe5cda3eabadbed07`. The model of the implementing subagent is not
recorded.

#### Step 2: red, the pin

`ev-red-format.txt` (`just gleam-double-format`, written at 02:39:41Z): exit status 0. Then
`ev-red-pin.txt`, written at 02:39:49Z. Its first lines are the commit and
`git status --short`. Each `FAIL` line is cut after the test's name; the indented line
below it holds the assertion's message and its `Left` and `Right` values, cut from the same
line. The runner's panic and the recipe's `error:` line, between `coverage faults: 0` and
`exit status: 1`, are left out with no mark.

```text
52f54df6999da7c6b2223d0fe5cda3eabadbed07 with uncommitted changes

 M gleam-double/test/pachinko_double_test.gleam
?? gleam-double/test/interface_test.gleam

$ just gleam-double
...
FAIL interface_test.the root holds exactly the interface's items: crashed: ...
  ... #(Message, "Assertion failed.") ... #(Left, dict.from_list([... #(Value, Error("cannot read src/pachinko_double.gleam: No such file or directory")) ...
FAIL interface_test.the root is the only module under src/ that imports an effect: crashed: ...
  ... #(Message, "Assertion failed.") ... #(Value, ["src/pachinko_double/owner.gleam"]) ... #(Value, ["src/pachinko_double.gleam"]) ...
FAIL interface_test.src/ holds only the root and its directory: crashed: ...
  ... #(Message, "Assertion failed.") ... #(Value, Ok(["pachinko_double"])) ... #(Value, Ok(["pachinko_double", "pachinko_double.gleam"])) ...
319 tests run, 316 passed
coverage faults: 0
exit status: 1
```

The first assertion's `Right` value is the 17 items, each `Item(name, constructors,
False)`, with `Outcome` and `PartialTick` carrying one constructor of their own name. The
six other pin tests, the parser's and the controls, passed: they read text made by hand.

#### Step 3: green, the move

`ev-green-format.txt` (written at 02:40:12Z), exit status 0, then `ev-green-move.txt`,
written at 02:40:23Z:

```text
52f54df6999da7c6b2223d0fe5cda3eabadbed07 with uncommitted changes

$ just gleam-double
...
319 tests run, 319 passed
coverage faults: 0
exit status: 0
```

What the formatter changed is in `format-diff.txt` (departure 1).

#### Steps 4 and 5: the header and the words

`ev-step5-gleam.txt` (written at 02:44:29Z): `319 tests run, 319 passed`,
`coverage faults: 0`, exit status 0, which includes `guide_test`'s check of the guide's
blocks. `ev-step5-docs.txt` (written at 02:41:57Z): `Validated 49 pages and 50 canonical
topics.`, exit status 0. `ev-probes-precommit.txt` (written at 02:44:34Z) ran
Verification without `git rev-parse`, `just check` and the `git status` test, under
`set -x`, before the review. It ended `PROBES DONE`, exit status 0; it ran the old `body`
probe, which printed `+ body=`, and section 5 shows that probe proved nothing.

### 5. Checkpoint record

One checkpoint, step 6: conformance and breakage, before the work commit, over the base
and the uncommitted diff. The conformance lens filed C1 and C2, the breakage lens B1.

- **Harness and model.** Codex v0.169.1 with `gpt-6-astra`, `sandbox: read-only`,
  reasoning effort `ultra`, one fresh process per lens; each `log-<lens>.txt` opens with
  those lines. By the lead's account the command was `codex exec -m gpt-6-astra -s
  read-only ... < /dev/null`; its exact text is not saved in a file. That is the shape of
  GD07's corrected command, which the series README says GD08 to GD11 take from
  `GD07-close-out.md`, and each `last-<lens>.md` exists, as its `-o` writes it. Codex was
  on `PATH`, so no fallback ran.
- **`codex exec --help` on the day.** The lead ran it before the implementer started and
  read its usage, unsaved; the same run was saved after the checkpoint, with
  `codex --version` (`codex-cli 0.169.1`), as `ai_tmp/review/codex-exec-help-2026-11-23.txt`.
- **Who ran it.** One reviewer subagent per lens started Codex; each Codex process id is
  in `pid-<lens>.txt`. Every subagent in this ticket ran on `claude-opus-5-5`.
- **Brief.** `brief-<lens>.txt`: `brief-common.txt` (the ticket, the series index, the
  shared rules, `tickets/CONVENTIONS.md` §11 and §14, the base, `git diff -M` plus the
  untracked files, the read limits, the step-6 questions) and `lens-<lens>.txt`. The
  conformance brief conforms to the ticket's decisions, not to the specification. It gives
  no hand-back conclusion. The reviewed tree was the uncommitted worktree over
  `52f54df6999da7c6b2223d0fe5cda3eabadbed07`; the work commit holds it plus C2's fix.
- **Recovery.** Each `findings-<lens>.md` opens `Source: last-message`, and below that line
  matches its `last-<lens>.md` byte for byte (checked with `diff` for this record).
- **What the reviewers ran.** Read-only commands and probes. Both lenses reproduced the
  `body` probe passing with `git diff` failing; neither ran a recipe.
- **Triage.** One separate subagent, neither a reviewer nor the lead, checked each finding
  and applied the fixes; its verdicts are `verdicts.md`.
- **Rounds.** One. No finding was high, so no second round was owed.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium | valid | Verification's `body` probe passes when `git diff` fails, and has no positive control | The patch probe (departure 3), in the ticket and in `verification.sh` |
| C2 | low | valid | `SPEC-NOTES.md` cited GD07's Verification for the importer check, which compares against the old path | The clause cites `interface_test` |
| B1 | medium | valid | The same defect as C1 | C1's fix |

C1's evidence, all in `ai_tmp/review/GD08-1/` and run at `52f54df6999da7c6b2223d0fe5cda3eabadbed07` with
uncommitted changes. `ev-C1-red.txt`, the old two lines with `git` failing:

```text
$ sh ai_tmp/review/GD08-1/red-C1.sh
simulated: git diff fails
old body probe PASSED although git diff failed
exit status: 0
```

After the fix: `ev-C1-green-gitfail.txt`, the new block with the patch step failing, stops
at the assignment, `exit status: 128`; `ev-C1-green-handmade.txt`, a hand-made rename patch
with one changed body line, prints `8:-  actor.send(owner, Tick)` and
`9:+  actor.send(owner, Tock)`, then `probe FAILED: a changed body line`, `exit status: 1`;
and `ev-C1-green-tree.txt`, the new block on the tree, `body block PASSED on the tree`,
`exit status: 0`.

The reruns after the fixes, uncommitted, on base `52f54df6999da7c6b2223d0fe5cda3eabadbed07`:
`ev-format.txt` (`just gleam-double-format`), exit status 0; `ev-final.txt`, written at
02:51:52Z, `319 tests run, 319 passed`, `coverage faults: 0`, exit status 0; and
`ev-docs.txt` (`just check-docs`), `Validated 49 pages and 50 canonical topics.`, exit
status 0.

### 6. Verification at the work commit

At `5ceb284816f432e65ef0848c7b8554185a7ddfc0`, the work commit, on a clean tree:
`ev-verification-at-work.txt`, written at 02:53:16Z, is `ai_tmp/gd/GD08/verification.sh`
under `sh -x`. The script is this ticket's corrected block with two lines added after
`set -eu`: `base=$(cat ai_tmp/gd/GD08/base.txt)` and `git cat-file -e "$base^{commit}"`
(`diff` of the two shows nothing else). At the run the corrected block was not yet
committed in this ticket, and `git status --short` printed nothing, so the ticket's edit
was set aside for the run. `ticket-triage.patch`, written at 02:52:41Z, matches the
ticket's uncommitted diff before this record was written.

It exited 0, so every `test` held: 17 `pub` items, each named in the header; the root the
one importer of `gleam/erlang` and `gleam/otp`; no import of `pachinko_double/owner` and
no alias; no `owner.gleam` or `owner.<item>` in the files searched; the only changed paths
under `src/` the move's two; the rename line present and no changed line under `src/`
that is not a `////` line; `interface_test` registered twice; "Where to look" present; no
`erl_crash.dump`. Each probe's positive control printed its expected count first. The two
`273 tests run: 273 passed, 0 skipped` lines inside `just check` are the Rust workspace's
tests, not the double's.

```text
$ sh -x ai_tmp/gd/GD08/verification.sh
+ set -eu
...
+ git rev-parse HEAD
5ceb284816f432e65ef0848c7b8554185a7ddfc0
...
gleam 1.19.1
...
OTP 29
...
319 tests run, 319 passed
coverage faults: 0
...
+ test 17 -eq 17
...
+ test gleam-double/src/pachinko_double.gleam = gleam-double/src/pachinko_double.gleam
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

1. **The formatter moved lines the ticket did not name.** `just gleam-double-format` put
   the root's import above the `pachinko_double/` imports in the 17 test modules, and
   rewrapped seven call lines that the longer qualifier pushed past the line width
   (`format-diff.txt`, the interdiff before and after the formatter). Each is an import
   or a qualified call, the lines step 3 changes.
2. **`gleam-double/README.md` line 5.** "under `src/pachinko_double/`", the end of the
   sentence that says the double is written in Gleam, became "under `src/`". The command under Files touched did not list the line, but it
   stopped being true when the root moved out of that directory.
3. **Verification's `body` probe was replaced** after C1 and B1 showed it passes when
   `git diff` fails. The patch probe takes the diff in its own assignment, checks the
   rename line, runs a control on a hand-made patch, and fails on any match or `rg` error
   (`verdicts.md`). The ticket's paragraph under the block says so. The step-5 probe run,
   `ev-probes-precommit.txt`, ran the old probe.
4. **The header's wording.** It calls `inspect`, `Outcome.fired` and `TickReport.fired`
   "three things a caller reads", not things of the root's: `TickReport` is in `core`.

### 8. What was not run, and what was not recorded

Not run:

- A push or a pull request. Both need authorisation (step 7); nothing left the worktree.
- `just initialize`, `just gleam-double-deps` or any other network recipe. The review's
  model calls are the only network use, which the checkpoint authorises.
- A second review round. None was owed.
- A review of the triage's fix diff.

Not recorded: the exact `codex exec` command lines, beyond the form in
`ai_tmp/review/reviewer-subagent.md`; and whether `review-docs` was used for
`docs/project/repository-map.md`.

## Open points

- **Closed: a constant can refer to a function, and the maintainer was told it
  cannot.** What the maintainer was first shown on 2026-11-23 gave three reasons a name
  is public only where it is defined: no re-exports, no constructors through an alias,
  and "a constant cannot hold a function". The third is wrong. The standard library's `gleam/dynamic/decode` defines
  `pub const string: Decoder(String) = Decoder(decode_string)`, a constant that holds
  one. A thin root of constants that refer to `owner.gleam`'s functions was therefore an
  option the maintainer was not shown. It would hand on the functions only: `Outcome`,
  `PartialTick` and every type of GD09 would still be public only where they are
  defined, and what a function loses when a constant refers to it was not tried here.
  Shown the correction later the same day, the maintainer ruled that the owner still
  moves whole.
- **`gleam docs build` and `gleam export package-interface`** would give the compiler's
  own view of the public list, where the pin reads source text. Each needs a new
  `Justfile` recipe, so neither is in scope for GD08 to GD11.
- **The pin reads text, not the compiler's result.** A `pub` item that does not start at
  column 0, or a constructor that does not start at two spaces, is not seen.
  `gleam format --check`, which `just gleam-double` runs, keeps both layouts.
- **`build-python-double` has moved since this branch was cut.** It now holds PD08 and
  PD09. Merging it into this branch is separate work.
- **The series' shared rule 4 is restated here, not edited in GD01 to GD07.** Those
  seven tickets are records of work done and keep the word `owner`.
