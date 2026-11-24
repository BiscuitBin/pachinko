---
id: GD08
title: "Gleam double: the front door, with the owner as the root module"
status: todo
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
body=$(git diff -M "$base" -- gleam-double/src | rg '^[+-]' | rg -v '^(\+\+\+|---) ' | rg -cv '^[+-]////' || true)
test -z "$body"

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
name list wraps is seen whatever its alias. The `body` line counts the changed lines under `gleam-double/src/` that are not `////`
lines, and expects none: `rg -c` prints nothing and exits 1 when it counts zero, which
`|| true` allows. The builder runs each control first, and corrects a pattern, saying so
under Departures, if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the red steps; the checkpoint's record; Verification at the work commit; departures; and
what was not run.

## Open points

- **A constant can refer to a function, and the maintainer was told it cannot.** What
  the maintainer was shown on 2026-11-23 gave three reasons a name is public only where
  it is defined: no re-exports, no constructors through an alias, and "a constant cannot hold
  a function". The third is wrong. The standard library's `gleam/dynamic/decode` defines
  `pub const string: Decoder(String) = Decoder(decode_string)`, a constant that holds
  one. A thin root of constants that refer to `owner.gleam`'s functions was therefore an
  option the maintainer was not shown. It would hand on the functions only: `Outcome`,
  `PartialTick` and every type of GD09 would still be public only where they are
  defined, and what a function loses when a constant refers to it was not tried here.
  The choice to move the owner whole stands unless the maintainer changes it before
  step 1.
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
