---
id: GD10
title: "Gleam double: the engine and the rules under internal/"
status: done
depends_on: [GD09]
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD10: Gleam double: the engine and the rules under internal/

## Context

After GD08 and GD09 the double has a front door,
`gleam-double/src/pachinko_double.gleam`, and every type a caller names sits in a module
that the engine does not own. A listing of `gleam-double/src/pachinko_double/` still does
not show that: it holds nineteen modules side by side, and the interface's vocabulary,
the rule-table machinery and the specification's rules look alike. The facts below are
those of commit `aebae0afadfb34a2d8ae762316376f70d3df08fe`, with GD08's and GD09's
changes read in.

Three kinds of `pub` name are mixed in those modules:

- **What a caller names.** `config.new`, `model.Pocket`, `refusals.LeaseFenced`,
  `invariants.check` and the rest.
- **What only the engine needs.** `model.State`, `model.replace_record`,
  `inspection.snapshot`, and everything in `rules`, `domain`, `core` and the rule
  modules. These are `pub` because another module under `gleam-double/src/` uses them.
- **What is `pub` only for the tests.** `core.run`, `core.registry`, `rules.dispatch`,
  `invariants.registry` and a few more. No other module under `gleam-double/src/` names
  them, and nothing says so.

Gleam has two tools for this. A module whose path is `<package>/internal` or lies under
it is internal by convention and by the compiler's default: `gleam.toml`'s
`internal_modules` defaults to those paths. And the attribute `@internal`, written on the
line above a `pub` definition, marks one name as no part of the interface. Neither stops
code in the same package from using the name, and the tests are in the same package, so
no test loses access.

The lists below come from a textual scan of the tree at that commit: it matched
`module.name` and the names each `import` lists. The builder checks each list against
the compiler and the tree, as the steps say.

## Goal

A listing of `gleam-double/src/pachinko_double/` shows the split: eight vocabulary
modules, and one directory, `internal/`, that holds the engine and the specification's
rules. In a public module, `@internal` marks each name that is not interface. The guide
lists the names that are `pub` only for the tests. The pin holds the public module list,
each public module's names, and that list of test-only names.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer fixed the first group on 2026-11-23. The builder does not reopen them.

- **Eleven modules move** with `git mv` to `gleam-double/src/pachinko_double/internal/`:

  | Kind | Modules |
  | --- | --- |
  | The engine | `rules`, `domain`, `core` |
  | The specification's rules | `identity`, `publish`, `pull`, `settle`, `lease_end`, `expiry`, `decay`, `consuming` |

  Each is then imported as `pachinko_double/internal/<module>`. Its qualifier is still
  its last segment, so `core.apply` is still written `core.apply`.
- **Nine modules stay public**: the root, and `config`, `time`, `model`, `refusals`,
  `failure`, `answers`, `inspection` and `invariants` directly under
  `gleam-double/src/pachinko_double/`.
- **`@internal` marks what a public module keeps `pub` for the engine or for the
  tests.** The exact names are in the second group.
- **The guide lists the test-only names**, and the pin holds that list against the
  source in both directions.
- **The checkpoint adds the `design` lens.**

The second group is this ticket's own design, written from that scan. The builder
follows it, and reports under Departures if one of these cannot hold.

- **The rule for a mark.** In a public module a `pub` name is marked `@internal` when it
  is one of these: `State`, or a function that takes or returns one; a helper that no
  module under `gleam-double/test/` names; or a name of the test-only table below. The
  lists that follow apply that rule, and where the rule and a list disagree the list
  wins.

  | Module | Marked `@internal` | Count |
  | --- | --- | --- |
  | `model` | `State`, `new`, `HeldRecord`, `held_under`, `replace_record`, `replace_pocket`, `replace_lease`, `queued`, `oldest_first`, `ending`, `leases_of_pocket`, `leases_of_consumer` | 12 |
  | `inspection` | `snapshot` | 1 |
  | `invariants` | `Invariant`, `registry` | 2 |
  | The root, `config`, `time`, `refusals`, `failure`, `answers` | none | 0 |

  `replace_lease` is named by one test module, `expiry_test`, which builds a state by
  hand. It is marked with its two siblings. `inspection.empty` is not marked: no module
  under `gleam-double/src/` names it, but `owner_test.gleam` uses it to write the
  snapshot it expects, as a caller's own test would, and on 2026-11-23 the maintainer
  ruled that it is part of the interface.
- **What stays unmarked, which is the interface.** With the root's 17 items from GD08:

  | Module | Items | Constructors |
  | --- | --- | --- |
  | `config` | `Settings`, `Config`, `defaults`, `default`, `new`, `settings` | `Settings`; `Config` is opaque |
  | `time` | `Timestamp`, `Duration`, `millisecond`, `second`, `minute`, `hour` | none: both types are aliases |
  | `refusals` | `Refusal` | `ConfigRefused`, `PublisherAlreadyJoined`, `ConsumerAlreadyJoined`, `UnknownPublisher`, `EqualRankRefused`, `UnknownConsumer`, `PullRefused`, `LeaseFenced`, `RecordNotHeld`, `MarkConflict`, `ReleaseRefused` |
  | `failure` | `Failure`, `Cause` | As GD09 lists them |
  | `answers` | `Answer`, `BatchRecord`, `TickReport`, `LeaseView`, `HeldView` | As GD09 lists them |
  | `model`, the entities | `Queue`, `Publisher`, `Consumer`, `PocketStatus`, `Pocket`, `Mark`, `RecordStatus`, `Record`, `Reason`, `SupersededRecord`, `DeadLetter`, `LeaseStatus`, `Lease` | `PocketStatus`: `PocketQueued`, `PocketLeased`, `PocketIdle`, `PocketParked`. `Mark`: `Unsettled`, `Acked`, `Nacked`. `RecordStatus`: `RecordQueued`, `RecordHeld`. `Reason`: `Superseded`, `Stale`. `LeaseStatus`: `LeaseActive`, `LeaseReleased`, `LeaseExpired`. Each other type has one constructor of its own name |
  | `model`, the lookups over a snapshot's lists | `pocket`, `record`, `lease`, `records_of`, `held`, `active` | none |
  | `inspection` | `Snapshot`, `empty`, `PocketView`, `ConsumerView`, `Views`, `views` | Each type has one constructor of its own name |
  | `invariants` | `check` | none |

  `model` has 31 `pub` items: 19 unmarked and 12 marked. `Queue` stays unmarked because
  it is an entity of the specification, though only a `State` holds one.
- **The test-only table.** A name is *`pub` only for the tests* when it is a `pub fn` or
  a `pub const` that sits in a module under `internal/` or is marked `@internal`, and no
  other module under `gleam-double/src/` names it. There are six:

  | Name | What the tests use it for |
  | --- | --- |
  | `core.run` | Running one table, a test's own included, as a whole outcome |
  | `core.run_at` | The same with `now` already in hand, as a clock trigger's outcome runs |
  | `core.run_due` | Running outcomes a test made by hand, one after another, as a tick does |
  | `core.registry` | The coverage check reads the rules from it |
  | `rules.dispatch` | Asking which row's guard holds, without running its effect |
  | `invariants.registry` | The coverage check, and the positive controls, read the checker's entries |

  The table lists functions. A type only a test names, such as `core.Ran`, follows the
  function that returns it and has no row. The guide carries this table under the
  heading `### What is pub only for the tests`, inside "Where to look", with each name
  in backticks as the first cell of its row.
- **What "names it" means**, for the pin and for a reader. A module names `m.f` when a
  line of it that is not a comment holds the text `m.f`, with no letter, digit or `_`
  directly before `m` or directly after `f`, where `m` is the module's last segment. It
  also names it when its `import` of the module lists `f` unqualified. The text match
  would miss a use through an aliased import, so the pin forbids one, below.
- **What the pin gains**, in `gleam-double/test/interface_test.gleam`, with the unit GD08
  fixed:
  - `gleam-double/src/pachinko_double/` holds, at its top level, exactly the eight
    vocabulary modules and the directory `internal`. `internal/` holds exactly the eleven
    modules. `gleam-double/src/` holds exactly the root and the directory.
  - For each of the eight vocabulary modules, its unmarked items with their
    constructors are the list above, and its marked items are the list above. The
    comparison ignores order.
  - No vocabulary module holds a line that starts with
    `import pachinko_double/internal/`. The root may.
  - No module under `gleam-double/src/` imports a module of the package under an alias.
    None does at the base: `rg -n '^import pachinko_double.* as |^\} as ' gleam-double/src`
    prints nothing.
  - The guide's test-only table and the pin's own list of six hold the same names.
  - Each of the six is a `pub fn` of its module, and no other module under
    `gleam-double/src/` names it.
  - Every other `pub fn` and `pub const` that sits under `internal/` or is marked is
    named by some other module under `gleam-double/src/`. So a new test-only name fails
    the pin until the table lists it.
  - GD09's witness still stands: `gleam-double/test/guide_test.gleam` imports no module
    under `pachinko_double/internal/`.
- **The parser's controls grow with it.** Tests on text made by hand show that the pin's
  result changes when a marked item loses its mark, when an unmarked item gains one,
  when a row is added to or removed from the table's text, when a second module starts
  to name a listed function, and when a module imports another under an alias. The text
  a control reads is written as GD08 says, so that no line of `interface_test.gleam`
  other than its own imports starts with `import`.
- **Three facts about Gleam are probed first**, each on the thinnest real change, with
  `just gleam-double`, which builds with warnings as errors. None has been run in this
  repository.

  | Fact | Probe | If it does not hold |
  | --- | --- | --- |
  | `@internal` is accepted above a `pub type` and above a `pub fn`, `gleam format` keeps it on its own line directly above `pub`, and a test can still name the marked item | Mark `invariants.Invariant` and `invariants.registry`; `coverage.gleam` and `invariants_test.gleam` call `registry` and read its entries | The kind that cannot take the attribute carries, as the last line of its doc comment, `/// Internal: not part of the interface.` The pin reads that line as the mark for that kind, and the hand-back says so |
  | A module under `internal/` needs no entry in `gleam.toml` | Move `decay`, which only `core` and `decay_test` import, and build | **Stop** and report: an edit to `gleam.toml` is outside this ticket |
  | No `pub` signature left unmarked in a public module draws a warning for naming a marked type or a type of an `internal/` module | The full build after the last mark | **Stop** and report the signature. It means a caller's type was left behind, which is GD09's subject. Do not suppress the warning |

  The list marks no constant, so `@internal` on a `pub const` is not probed.
- **The words follow the files.** The root's header, the guide's "Where to look" and
  every path the guide gives for a moved module change to the new layout. "Where to
  look" becomes:

  | A reader looking for | Opens |
  | --- | --- |
  | The interface: starting the owner, the triggers, the clock and the two reads | `src/pachinko_double.gleam` |
  | What a caller names when it sends a trigger or reads a reply | `config.gleam`, `time.gleam`, `model.gleam`, `refusals.gleam`, `failure.gleam` and `answers.gleam`, under `src/pachinko_double/` |
  | The snapshot and the invariant checker | `inspection.gleam` and `invariants.gleam`, under `src/pachinko_double/` |
  | The engine: the rule-table machinery and the apply step | `rules.gleam`, `domain.gleam` and `core.gleam`, under `src/pachinko_double/internal/` |
  | The specification's rules | `identity.gleam`, `publish.gleam`, `pull.gleam`, `settle.gleam`, `lease_end.gleam`, `expiry.gleam`, `decay.gleam` and `consuming.gleam`, under `src/pachinko_double/internal/` |
  | What is `pub` only for the tests | The table below this one |
  | The tests | As GD08 or GD11 left the row |
  | Test support | As GD08 or GD11 left the row |

  This ticket does not edit the last two rows.

## Non-goals

- No behaviour change. Outside `import` statements and `@internal` lines, no line of a
  module under `gleam-double/src/` changes, except the root's `////` header. If the
  first probe's fallback is taken for a kind, its doc line counts as that kind's
  `@internal` line here, in the checkpoint's questions and in Acceptance criteria.
- No name becomes private, and none is renamed or removed. A test-only name stays `pub`.
- No test under a `Scenario` tag changes beyond its imports.
- No fenced `gleam` block of the guide changes beyond the path on a fence's first line.
- No file under `gleam-double/test/` moves: that is GD11.
- No change to `gleam-double/gleam.toml`, `gleam-double/manifest.toml`, the `Justfile` or
  anything under `docs/specs/`, `docs/decisions/`, `crates/` or `double/`.
- No `gleam docs build` and no `gleam export package-interface`: each needs a new
  recipe.
- No new spec note (shared rule 5): the change interprets nothing in the specification.

## Shared rules

Shared rules 1 to 6 bind this ticket as `.scratch/gleam-double/GD07-close-out.md` writes
them under "Shared rules (GD01 to GD07)", with rule 4's second sentence as
`.scratch/gleam-double/GD08-front-door.md` restates it under "Shared rules". They are not
copied here. Rule 3's coverage check stays strict at 93, 32 and 35. Under rule 1,
`double/pachinko_double/` and `double/tests/` stay unread.

Rule 4 names modules by their bare names, `core` and `rules`. Those names still hold:
only the directory changes.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/gleam-double/GD07-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `GD10`, and `<n>` is 1: this ticket has one checkpoint, at step 9.
- The lenses are conformance, breakage and `design`. The design lens reads the whole of
  `gleam-double/src/` and asks the design questions of step 9, in place of the questions
  GD07's deltas give it.
- The conformance brief names this ticket's path and its "Decisions fixed in this
  ticket", not `docs/specs/pachinko.allium`: the ticket changes no behaviour.
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff.
- A runner polls on `GD10-1/last-<lens>.md` or the process id, never the bare file name.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| The eleven modules under `gleam-double/src/pachinko_double/` | moved | `git mv` to `gleam-double/src/pachinko_double/internal/`; their imports of each other |
| `gleam-double/src/pachinko_double.gleam` | shared, own lines | Its imports of moved modules, and the header |
| `gleam-double/src/pachinko_double/model.gleam`, `inspection.gleam`, `invariants.gleam` | shared, own lines | The `@internal` lines |
| `gleam-double/test/interface_test.gleam` | shared, own lines | What the pin gains |
| Every module under `gleam-double/test/` that imports a moved module | shared, own lines | The import |
| `gleam-double/README.md` | shared, own lines | "Where to look", the test-only table, and each path of a moved module, the three fences' included |
| `docs/project/repository-map.md` | shared, own lines | The `gleam-double/` row, if its wording no longer matches |
| `.scratch/gleam-double/GD10-internal-modules.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD10's Status cell |

`gleam-double/src/pachinko_double/failure.gleam` and `answers.gleam` exist once GD09 is
done, and `gleam-double/test/interface_test.gleam` once GD08 is. `config.gleam`,
`time.gleam`, `refusals.gleam`, `failure.gleam` and `answers.gleam` are not edited.

Counts at commit `aebae0afadfb34a2d8ae762316376f70d3df08fe`, each with its command. GD08
and GD09 change the first, so the builder reruns each at the base.

| What | Count | Command |
| --- | --- | --- |
| Import lines that name a moved module | 87, in 29 files | The first below |
| Paths of a moved module in the guide | 35, of which 3 open a fence | The second below |

```sh
moved='rules|domain|core|identity|publish|pull|settle|lease_end|expiry|decay|consuming'
rg -c "^import pachinko_double/($moved)\b" gleam-double/src gleam-double/test
rg -o "src/pachinko_double/($moved)\.gleam" gleam-double/README.md
```

The three fences are excerpts of `identity.gleam` and, twice, `publish.gleam`. The check
in `guide_test.gleam` reads the file a fence names, so each fence's path must change
with the move or the run fails. The guide also names modules by bare file name, such as
`rules.gleam`; those stay true. `gleam-double/SPEC-NOTES.md` gives no path under `src/`.

## Steps

Each red step quotes the failing test's name and its message verbatim, headed by the
full commit id and "with uncommitted changes". Evidence goes under `ai_tmp/gd/GD10/`.

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD09. If `.pixi/`,
   `.tools/` or `gleam-double/build/packages/` is missing, run `just initialize` and
   `just gleam-double-deps` (network, authorised by this step). Run `just check` and
   `just gleam-double` on the untouched worktree. Quote each one's last line and the
   line `N tests run, N passed`, headed by the commit.
2. **Red: the layout and the names.** Add to the pin the layout assertions, the lists of
   unmarked and marked items, the rule that no vocabulary module imports
   `pachinko_double/internal/` and the rule against an aliased import, with their
   controls. Run `just gleam-double-format`, then `just gleam-double`. The layout and
   list assertions fail: there is no `internal` directory, and nothing is marked. Quote
   those failures. The two import rules pass already, and so do the controls.
3. **The probes.** Mark `invariants.Invariant` and `invariants.registry`. Move `decay`
   and fix the imports in `core` and `decay_test`. Run `just gleam-double-format`, then
   `just gleam-double`. What the first two rows of the probe table ask is that the
   format check and the build pass, and that every test but the pin's passes. The pin's
   layout and list assertions still fail, as step 2 left them, and nothing else may.
   Quote the result, and take the fallback or stop as the table says.
4. **The moves.** Move the other ten modules and fix every import. In the guide, change
   the paths on the three fences that name `identity.gleam` and `publish.gleam`, since
   the run reads the file a fence names. Run `just gleam-double-format`, then
   `just gleam-double`. The layout assertions pass now. The list assertions still fail,
   for the thirteen marks not yet written, and nothing else may. Save
   `git diff -M "$base" -- gleam-double/src` and its `--stat`, and show that each of the
   eleven is a rename whose changed lines all belong to `import` statements.
5. **Green: the marks.** Add the other thirteen `@internal` lines. `just gleam-double`
   builds with no warning, which is the third probe; quote it. Every assertion of step 2
   passes, and the run is green.
6. **Red, then green: the test-only table.** Add the pin's assertions about the six
   names, with their controls, and run `just gleam-double-format`. They fail: the guide
   has no such table. Quote the failure. Add the table to the guide; the run is green.
   Then show the scan can fail: in a scratch change, call `core.registry()` from the
   root and quote the pin's failure, then revert it.
7. **Words.** Update "Where to look", the root's header and the other 32 paths in the
   guide. Use the `review-docs` skill for `docs/project/repository-map.md`. Run
   `just gleam-double`, which checks the guide's excerpts against the moved files, and
   `just check-docs`.
8. **The lists against the tree.** For each marked name, save the output of a search
   that shows where `gleam-double/test/` names it, and say in the hand-back which marked
   names the tests use. A marked name that turns out to be named by
   `gleam-double/test/guide_test.gleam` is a finding: stop and report it, because the
   walkthrough is the caller.
9. **Adversarial review** (checkpoint 1: conformance, breakage and design, before the
   final commit). Questions for this checkpoint:
   - Is any name marked that a caller of the root needs, or left unmarked that only the
     engine needs?
   - Does any unmarked `pub` signature of a public module name a marked type or a type
     from `internal/`?
   - Would the pin fail if a module were added to `internal/` or beside it, if an item
     gained or lost a mark, or if a new `pub fn` under `internal/` were used only by a
     test?
   - Is any of the six test-only names in fact named by a module under
     `gleam-double/src/`, or is any other name `pub` only for the tests?
   - Outside `import` statements, `@internal` lines and the root's header, did any line
     under `gleam-double/src/` change?
   - Does the guide give a path that does not exist, or an excerpt that is no longer
     verbatim?
   - Design: does each public module have one job a reader can state? Can a reader tell
     the interface from the engine from the listing alone? Does anything public depend
     on `internal/` other than the root? Is the test-only table the right place for
     each of its rows, or does a row belong in the interface?
   - Does the coverage check still collect every test and stay strict at 93, 32 and 35?
   - Could a probe in Verification print nothing while its claim is false?
10. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
    commit and quote it, headed by the full commit id. Set `status: done` here and in
    `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
    record. **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `gleam-double/src/pachinko_double/` holds exactly the eight vocabulary modules and
  `internal/`, which holds exactly the eleven moved modules. The pin asserts it.
- `model`, `inspection` and `invariants` carry exactly the 12, 1 and 2 marks of
  "Decisions fixed", and no other module under `gleam-double/src/` carries one. The pin
  asserts each module's unmarked and marked lists.
- No vocabulary module imports a module under `pachinko_double/internal/`, and no module
  under `gleam-double/src/` imports another under an alias. The pin asserts both.
- The guide's test-only table lists exactly the six names, and the pin holds the table
  against the source in both directions. Step 6's scratch break is quoted.
- No file under `gleam-double/src/` or `gleam-double/test/` imports a moved module by its
  old path. `gleam-double/README.md`, `gleam-double/SPEC-NOTES.md` and
  `docs/project/repository-map.md` give no old path of one. The tickets under
  `.scratch/gleam-double/` are records and keep the paths they were written with.
- `gleam-double/gleam.toml` and `gleam-double/manifest.toml` are unchanged.
- The three probes are quoted with their results, and any fallback taken is named.
- Steps 2 and 6 are each quoted red before they turn green.
- `just gleam-double` exits 0 with the coverage check strict at 93, 32 and 35. Its test
  count is the baseline's plus the tests this ticket adds to the pin.
- The diff changes no test under a `Scenario` tag beyond its imports.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script from the repository root; `set -eu` stops it at the first command
that fails. `base` is the commit step 1 records; set it before the script runs. The
script's first check stops it when `base` is unset or names no commit. Each probe that
passes by printing nothing follows its positive control. `sh` reads only the status of
a pipeline's last command, so every command whose failure matters runs alone in an
assignment or last in its pipeline: `ls`, `sort`, `rg -l` and `git diff` each fill a
variable of their own, only `printf` feeds a pipe, and `rg` never does. If the first
probe's fallback was taken for a kind, types or functions, the builder changes the
script for that kind only and says so under Departures: each `rg -qU` line for an item
of that kind looks for the doc line above its `pub`, each count of marks adds the doc
lines to the `@internal` lines, and `counts` takes an added doc line as a mark too.

```sh
set -eu
git cat-file -e "$base^{commit}"   # base is set before the script runs
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

src=gleam-double/src/pachinko_double
root=gleam-double/src/pachinko_double.gleam
entries=$(ls gleam-double/src)
sorted=$(printf '%s\n' "$entries" | LC_ALL=C sort)
outer=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$outer" = "pachinko_double pachinko_double.gleam "
entries=$(ls "$src")
sorted=$(printf '%s\n' "$entries" | LC_ALL=C sort)
top=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$top" = "answers.gleam config.gleam failure.gleam inspection.gleam internal invariants.gleam model.gleam refusals.gleam time.gleam "
entries=$(ls "$src/internal")
sorted=$(printf '%s\n' "$entries" | LC_ALL=C sort)
inner=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$inner" = "consuming.gleam core.gleam decay.gleam domain.gleam expiry.gleam identity.gleam lease_end.gleam publish.gleam pull.gleam rules.gleam settle.gleam "

moved='rules|domain|core|identity|publish|pull|settle|lease_end|expiry|decay|consuming'
pattern="^import pachinko_double/($moved)\b"
probe=$(printf 'import pachinko_double/core.{type Trigger}\nimport pachinko_double/decay\nimport pachinko_double/internal/core\nimport pachinko_double/core_x\n' | rg -c "$pattern")
test "$probe" -eq 2
if rg -n "$pattern" gleam-double/src gleam-double/test; then exit 1; else test "$?" -eq 1; fi

pattern='^import pachinko_double/internal/'
probe=$(printf 'import pachinko_double/internal/core\nimport pachinko_double/internal/rules.{type Table}\n  import pachinko_double/internal/core\n' | rg -c "$pattern")
test "$probe" -eq 2
for module in answers config failure inspection invariants model refusals time; do
  if rg -n "$pattern" "$src/$module.gleam"; then exit 1; else test "$?" -eq 1; fi
done
if rg -n "$pattern" gleam-double/test/guide_test.gleam; then exit 1; else test "$?" -eq 1; fi

for item in 'type State' 'fn new' 'type HeldRecord' 'fn held_under' 'fn replace_record' \
  'fn replace_pocket' 'fn replace_lease' 'fn queued' 'fn oldest_first' 'fn ending' \
  'fn leases_of_pocket' 'fn leases_of_consumer'; do
  rg -qU "^@internal\npub $item\b" "$src/model.gleam"
done
rg -qU '^@internal\npub fn snapshot\b' "$src/inspection.gleam"
pattern='^@internal\npub fn empty\b'
probe=$(printf '@internal\npub fn empty(settings: Settings) -> Snapshot {\n' | rg -U --count-matches "$pattern")
test "$probe" -eq 1
if rg -nU "$pattern" "$src/inspection.gleam"; then exit 1; else test "$?" -eq 1; fi
rg -qU '^@internal\npub type Invariant\b' "$src/invariants.gleam"
rg -qU '^@internal\npub fn registry\b' "$src/invariants.gleam"
marks=$(rg -c '^@internal$' "$src/model.gleam")
test "$marks" -eq 12
marks=$(rg -c '^@internal$' "$src/inspection.gleam")
test "$marks" -eq 1
marks=$(rg -c '^@internal$' "$src/invariants.gleam")
test "$marks" -eq 2
files=$(rg -l '^@internal' gleam-double/src)
sorted=$(printf '%s\n' "$files" | LC_ALL=C sort)
marked=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$marked" = "$src/inspection.gleam $src/invariants.gleam $src/model.gleam "

seams='core\.run|core\.run_at|core\.run_due|core\.registry|rules\.dispatch|invariants\.registry'
rows=$(rg -c "^ *\| *\`($seams)\` *\|" gleam-double/README.md)
test "$rows" -eq 6
pattern='^ *\| *`inspection\.empty` *\|'
probe=$(printf '| `inspection.empty` | a row |\n | `inspection.empty` | indented |\n|`inspection.empty`|tight|\n' | rg -c "$pattern")
test "$probe" -eq 3
if rg -n "$pattern" gleam-double/README.md; then exit 1; else test "$?" -eq 1; fi

pattern="src/pachinko_double/($moved)\.gleam"
probe=$(printf 'src/pachinko_double/publish.gleam\nsrc/pachinko_double/internal/publish.gleam\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" gleam-double/README.md gleam-double/SPEC-NOTES.md docs/project/repository-map.md; then exit 1; else test "$?" -eq 1; fi

pattern='^import pachinko_double.* as |^\} as '
probe=$(printf 'import pachinko_double/internal/core as engine\n} as engine\nimport pachinko_double/model\n' | rg -c "$pattern")
test "$probe" -eq 2
if rg -n "$pattern" gleam-double/src; then exit 1; else test "$?" -eq 1; fi

# The control: the same command names a file that did change.
seen=$(git diff --name-only "$base" -- "$src/model.gleam")
test "$seen" = "$src/model.gleam"
pins=$(git diff --name-only "$base" -- gleam-double/gleam.toml gleam-double/manifest.toml)
test -z "$pins"
untouched=$(git diff --name-only "$base" -- "$src/config.gleam" "$src/time.gleam" "$src/refusals.gleam" "$src/failure.gleam" "$src/answers.gleam")
test -z "$untouched"

# The marked modules change only by added `@internal` lines: every changed
# line of their patch is one of the six file headers or one of the 15 marks.
# A removed line that starts with `--` reads as a header, so it moves the
# header count too. The control runs the same counts on a hand-made patch
# that also changes a body line, and must see it.
counts() {
  headers=$(printf '%s\n' "$1" | rg -c '^(\+\+\+|---) ' || test "$?" -eq 1)
  changed=$(printf '%s\n' "$1" | rg -c '^[+-]' || test "$?" -eq 1)
  added=$(printf '%s\n' "$1" | rg -c '^\+@internal$' || test "$?" -eq 1)
}
control=$(printf -- '--- a/m.gleam\n+++ b/m.gleam\n@@ -1,2 +1,3 @@\n+@internal\n-pub fn f() {\n+pub fn f(x) {\n')
counts "$control"
test "$headers" -eq 2
test "$added" -eq 1
test "$changed" -ne "$((headers + added))"
patch=$(git diff "$base" -- "$src/model.gleam" "$src/inspection.gleam" "$src/invariants.gleam")
counts "$patch"
test "$headers" -eq 6
test "$added" -eq 15
test "$changed" -eq "$((headers + added))"

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The table probes allow spaces before a row and inside its first cell, as Markdown
does. The `seen` line is the control for the two `git diff --name-only` probes after
it: the same command names a file that did change. `counts` reads a patch of the
three marked modules: its file headers, its changed lines and its added `@internal`
lines. The patch passes when every changed line is a header or a mark, with six headers
and fifteen marks; the control runs the same counts on a hand-made patch that also
changes a body line, and must see the change. A removed line that starts with `--`
reads as a header, so it moves the header count too. `rg -c` prints nothing and exits 1
when it counts zero; `|| test "$?" -eq 1` lets that through and stops on any other
status, and an empty count then fails the `test` that reads it. The script does not
show that a moved module changed only in its imports, because `gleam format` may rewrap
an import; step 4's saved diff shows that. The builder runs each control first, and
corrects a pattern, saying so under Departures, if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the probes; the red steps; the checkpoint's record; Verification at the work commit;
departures; and what was not run.

Evidence files are under `ai_tmp/gd/GD10/`, and the review record under
`ai_tmp/review/GD10-1/`. None is committed. The implementer's files open with the commit
line, `6e87e357c28fc9dbb59b7097435e0c34930b2ad9` alone at the baseline and with
"(untouched)" or "with uncommitted changes" after it, except the five records of
`just gleam-double-format` (`ev-red-format.txt`, `ev-probe-format-3.txt`,
`ev-step4-format.txt`, `ev-step5-format.txt`, `ev-step6-format.txt`), which begin
`$ <command>` and name no commit. The triage's recipe records (`ev-C1-red.txt`,
`ev-B1-red.txt`, `ev-B2-red.txt`, the four `ev-*format.txt`, `ev-final.txt`,
`ev-docs.txt`, `ev-B1-nolead-docs.txt`) also begin `$ <command>` and name no commit; its
`ev-verif-controls.txt` opens with the commit line. In every quote below the recipe's
echo of its own command lines is left out, and `...` marks other lines left out or a line
cut short: a `FAIL` line is cut after the test's name, and the indented line under it,
where one is given, gives the assertion's `Left` value, then its `Right`, from the same
line. Times are UTC,
read from the files' modification times, which are each file's last write; the work
commit's own date is 2026-11-23T20:28:56-08:00, which is 2026-11-24T04:28:56Z. Where a
fact the record should hold was never saved, the text says "not recorded"; where it rests
only on the lead's message, the text says "by the lead's account".

### 1. Base, branch and commits

- Branch: `build-gleam-double`, in this worktree (shared rule 6).
- Base commit: `6e87e357c28fc9dbb59b7097435e0c34930b2ad9` (`base.txt`, written at
  03:52:09Z), GD09's record commit. It holds GD09's work,
  `72fe01a206936433299151562c10fa8850281649`.
- Work commit: `21791e62978c60909a9c3602e278c23f45c99eb2` (`work-commit.txt`), tree
  `a466c704b7557a0acc5a27e6cb70e431bd294bc9`, titled "Put the Gleam double's engine and
  rules under internal/, and mark the rest (GD10)". It touches 30 files
  (`git show --stat`): the eleven renames under `gleam-double/src/pachinko_double/`, the
  root, `model.gleam`, `inspection.gleam` and `invariants.gleam`; 13 test modules
  (`interface_test.gleam` and the 12 that import a moved module); `gleam-double/README.md`
  and `docs/project/repository-map.md`. `gleam-double/gleam.toml`,
  `gleam-double/manifest.toml` and `gleam-double/SPEC-NOTES.md` are not touched.
- The commit after it holds this record, the index's Status cell, and the ticket's
  corrected Verification block and the paragraphs around it (section 8, departure 11).
  The corrected block was not in the committed ticket at the work commit.
- Nothing was fetched. Step 1's `just initialize` and `just gleam-double-deps` were not
  run, by the lead's account; the baseline's `just gleam-double` passed its checks that
  the gleam environment and `gleam-double/build/packages/` are present.

### 2. Baseline

At `6e87e357c28fc9dbb59b7097435e0c34930b2ad9`, on the untouched worktree.

`ev-baseline-check.txt`, written at 03:53:19Z:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

`ev-baseline-gleam.txt`, written at 03:53:19Z:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9
$ just gleam-double
...
gleam 1.19.1
...
OTP 29
...
    Running pachinko_double_test.main
326 tests run, 326 passed
coverage faults: 0
exit status: 0
```

The counts at the base, `ev-counts-base.txt` (written at 03:53:24Z): 67 import lines
that name a moved module, in 22 files (departure 1); 35 paths of a moved module in the
guide; and the aliased-import search, `rg exit 1`, printing nothing.

### 3. What landed

- **The layout.** The eleven modules moved with `git mv` to
  `gleam-double/src/pachinko_double/internal/`: `rules`, `domain`, `core`, `identity`,
  `publish`, `pull`, `settle`, `lease_end`, `expiry`, `decay` and `consuming`.
  `src/pachinko_double/` now holds the eight vocabulary modules and `internal`, and
  `gleam-double/src/` the root and the directory. Each move is a rename, R092 to R100,
  whose changed lines are all import lines (`ev-moves-imports-only.txt`, section 5).
- **The marks.** Fifteen `@internal` lines, each directly above its `pub`: the twelve of
  `model` ("Decisions fixed"), `inspection.snapshot`, and `invariants.Invariant` and
  `invariants.registry`. No other module under `gleam-double/src/` carries one.
  `inspection.empty` stays unmarked.
- **The root.** Its imports take the moved modules from `pachinko_double/internal/`, and
  its `////` header gains closing sentences on the split (departure 8). No other line of
  it changed (`ev-tests-imports-only.txt`, its last part).
- **The tests.** Each of the 12 test modules that imported a moved module changes only in
  its import statements (`ev-tests-imports-only.txt`). No test under a `Scenario` tag
  changed beyond its imports.
- **The pin**, `gleam-double/test/interface_test.gleam`, gains 21 tests: 13 at step 2, 6
  at step 6 and 2 at the checkpoint (B2). Its tree assertions: "src/pachinko_double/
  holds the eight vocabulary modules and internal/", "internal/ holds exactly the eleven
  modules", the six "... holds its interface, and ..." tests of `config`, `time`,
  `model`, `refusals`, `inspection` and `invariants`, "only model, inspection and
  invariants carry marks: 12, 1 and 2", "no vocabulary module imports a module under
  internal/", "no module under src/ imports a module under an alias", "the guide's table
  lists exactly the names pub only for the tests" and "the names pub only for the tests
  are the six, and no other name is". The rest are controls on text made by hand. Its
  `////` header describes the new assertions and the parser's new rules. `failure` and
  `answers` keep GD09's item tests, and the marks test asserts that they, the root and
  `internal/` carry no mark. GD09's witness still stands.
- **The words.** `gleam-double/README.md`'s "Where to look" takes the ticket's rows, with
  "What is `pub` only for the tests" pointing to a new table under the heading
  `### What is pub only for the tests`, which lists the six names and opens with a short
  paragraph (departure 8). All 35 paths of a moved module, the three fences' included,
  now give `src/pachinko_double/internal/`. The last two rows are not edited.
  `docs/project/repository-map.md`'s `gleam-double/` row names the vocabulary, the
  snapshot and the checker, and the `internal/` directory (departure 8).

The test count: 326 at the base, 339 from step 2, 345 from step 6, and 347 after the
checkpoint, the baseline's plus the pin's 21. The coverage check's constants stay 93, 32
and 35, and every run below reports `coverage faults: 0`.

### 4. The probes

Every run in this section is uncommitted, on base
`6e87e357c28fc9dbb59b7097435e0c34930b2ad9`. All three held; no fallback was taken and no
stop was owed.

- **Probe 1: `@internal` above a `pub type` and a `pub fn`.** `ev-probe-1.txt`, written at
  03:59:13Z. After `just gleam-double-format` (`ev-probe-format-3.txt`, exit status 0),
  the diff of `invariants.gleam` is two added lines, `+@internal` directly above
  `pub type Invariant {` and above `pub fn registry() -> List(Invariant) {`. The run is
  `ev-probe-gleam-3.txt` (written at 03:59:05Z): the format check and the build with
  `--warnings-as-errors` pass, and `coverage.gleam:59` and `invariants_test.gleam:1292`
  still call `invariants.registry()`, unchanged. The four `FAIL` lines are the pin's
  layout and list assertions step 2 left red, less `invariants`', which the two marks
  turned green:

  ```text
  6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
  ...
  FAIL interface_test.src/pachinko_double/ holds the eight vocabulary modules and internal/: ...
  FAIL interface_test.internal/ holds exactly the eleven modules: ...
  FAIL interface_test.model holds its interface, and its twelve marked items: ...
  FAIL interface_test.inspection holds its interface, and its one marked item: ...
  339 tests run, 335 passed
  coverage faults: 0
  ...
  exit status: 1
  ```

- **Probe 2: a module under `internal/` needs no entry in `gleam.toml`.**
  `ev-probe-2.txt`, written at 03:59:14Z, from the same run. `git status --short` shows
  `R  gleam-double/src/pachinko_double/decay.gleam -> gleam-double/src/pachinko_double/internal/decay.gleam`
  and `core.gleam` and `decay_test.gleam` modified; `git diff --stat` of `gleam.toml` and
  `manifest.toml` is empty, and `rg -n "internal" gleam-double/gleam.toml` ends
  `rg exit 1`. The build passed with `decay` under `internal/`. Its file ends
  `Result: holds.`
- **Probe 3: no unmarked signature draws a warning.** `ev-probe-3.txt` is step 5's run,
  after the last mark; the probe's note was appended after step 6, and the file was last
  written at 04:10:14Z.

  ```text
  6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
  $ just gleam-double
  ...
    Compiling pachinko_double
     Compiled in 2.01s
  ...
  339 tests run, 339 passed
  coverage faults: 0
  exit status: 0

  --- Probe 3 (appended): the full build after the last mark, with --warnings-as-errors
  warning lines in the run above: 0
  The build is clean. That shows no warning was drawn; it does not by itself show that no unmarked signature names a marked type.
  ```

  The file adds a textual check of each unmarked `pub` item of the eight vocabulary
  modules and the root for the words `State`, `HeldRecord` and `Invariant`, which ends
  `hits: 0`. The script it ran, `leak.py`, is not saved under `ai_tmp/gd/GD10/`. The
  probe's claim is therefore what its note says: no warning was drawn. The design lens
  found no unmarked signature that names a marked or internal type (section 6).

### 5. The red steps, and steps 4 to 8

Every output in this section ran uncommitted, on base
`6e87e357c28fc9dbb59b7097435e0c34930b2ad9`. The implementing subagent ran on
`claude-opus-5-5`, by the lead's account. `just gleam-double-format` ran before each
`just gleam-double` of steps 2 to 6, each exit status 0; what it changed at steps 3 and 4
is in `format-diff.txt` (departure 2).

#### Step 2: red, the layout and the names

`ev-red-layout.txt`, written at 03:56:36Z, after `ev-red-format.txt` (exit status 0).
The two layout assertions and the three lists with marks fail; the two import rules and
every control pass.

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
FAIL interface_test.src/pachinko_double/ holds the eight vocabulary modules and internal/: crashed: ...
  ... #(Value, Ok(["answers.gleam", "config.gleam", "consuming.gleam", "core.gleam", "decay.gleam", ... #(Value, Ok(["answers.gleam", "config.gleam", "failure.gleam", "inspection.gleam", "internal", "invariants.gleam", "model.gleam", "refusals.gleam", "time.gleam"])) ...
FAIL interface_test.internal/ holds exactly the eleven modules: crashed: ...
  ... #(Value, Error(Enoent)) ... #(Value, Ok(["consuming.gleam", "core.gleam", "decay.gleam", "domain.gleam", "expiry.gleam", "identity.gleam", "lease_end.gleam", "publish.gleam", "pull.gleam", "rules.gleam", "settle.gleam"])) ...
FAIL interface_test.model holds its interface, and its twelve marked items: crashed: ...
FAIL interface_test.inspection holds its interface, and its one marked item: crashed: ...
FAIL interface_test.invariants holds its interface, and its two marked items: crashed: ...
339 tests run, 334 passed
coverage faults: 0
...
exit status: 1
```

In each of the three list failures `Left` is `Ok(#([...], []))`, the module's items
with nothing marked, and `Right` holds the marked list of "Decisions fixed", opening
`Item("HeldRecord", ...`, `Item("snapshot", ...` and `Item("Invariant", ...`.

#### Step 3: the probes

Section 4.

#### Step 4: the moves

`ev-step4-gleam.txt`, written at 03:59:43Z. The layout assertions pass; the two lists
still missing marks fail, and nothing else:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
FAIL interface_test.model holds its interface, and its twelve marked items: crashed: ...
FAIL interface_test.inspection holds its interface, and its one marked item: crashed: ...
339 tests run, 337 passed
coverage faults: 0
...
exit status: 1
```

`ev-moves-diff.txt` and `ev-moves-stat.txt` (written at 03:59:55Z) hold
`git diff -M "$base" -- gleam-double/src` and its `--stat`; `ev-moves-diff-final.txt`
is the same diff at the final tree of steps 1 to 8. `ev-moves-imports-only.txt` (last
written at 04:10:34Z) strips every import statement from each module at the base and
after the move and compares the rest with `cmp`:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
rules      ok    rename: R100 ...; non-import lines: 258 of 258 identical
domain     ok    rename: R098 ...; non-import lines: 47 of 47 identical
core       ok    rename: R096 ...; non-import lines: 351 of 351 identical
identity   ok    rename: R092 ...; non-import lines: 46 of 46 identical
publish    ok    rename: R098 ...; non-import lines: 352 of 352 identical
pull       ok    rename: R098 ...; non-import lines: 248 of 248 identical
settle     ok    rename: R098 ...; non-import lines: 429 of 429 identical
lease_end  ok    rename: R098 ...; non-import lines: 344 of 344 identical
expiry     ok    rename: R097 ...; non-import lines: 187 of 187 identical
decay      ok    rename: R095 ...; non-import lines: 78 of 78 identical
consuming  ok    rename: R100 ...; non-import lines: 36 of 36 identical

failures: 0

Control: the check sees a change to a non-import line. A copy of core.gleam with one body line altered:
control: cmp found the difference, as it must
...
failures: 0
```

The last `failures: 0` is its rerun at the final tree of steps 1 to 8.

#### Step 5: green, the marks

`ev-probe-3.txt` (section 4): `339 tests run, 339 passed`, `coverage faults: 0`,
exit status 0, the baseline's 326 plus step 2's 13.

#### Step 6: red, then green, the test-only table

`ev-red-testonly.txt`, written at 04:02:33Z, after `ev-step6-format.txt` (exit status 0).
The guide has no table yet:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
FAIL interface_test.the guide's table lists exactly the names pub only for the tests: crashed: ...
  ... #(Value, Ok([])) ... #(Value, Ok(["core.registry", "core.run", "core.run_at", "core.run_due", "invariants.registry", "rules.dispatch"])) ...
345 tests run, 344 passed
coverage faults: 0
...
exit status: 1
```

Only the guide-table assertion failed: the scan assertion, "the names pub only for the
tests are the six, and no other name is", and every control passed, since the pin's own
list of six already matched the source.

With the table in the guide, `ev-green-testonly.txt` (written at 04:02:47Z):
`345 tests run, 345 passed`, `coverage faults: 0`, exit status 0.

The scratch break, `ev-scan-break.txt` (written at 04:02:58Z): the root's `inspect`
gains `let _ = core.registry()`, and the scan fails:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
FAIL interface_test.the names pub only for the tests are the six, and no other name is: crashed: ...
  ... #(Value, Ok(["core.registry is named by src/pachinko_double.gleam"])) ... #(Value, Ok([])) ...
345 tests run, 344 passed
coverage faults: 0
...
exit status: 1

--- Reverted: the file restored from the copy taken before the break.
...
cmp: identical
matches of 'registry' in git diff of the root: 0
```

#### Step 7: the words

`ev-step7-gleam.txt` (written at 04:03:40Z): `345 tests run, 345 passed`,
`coverage faults: 0`, exit status 0, which includes `guide_test`'s check of the guide's
excerpts against the moved files. `ev-step7-docs.txt` (written at 04:03:40Z):
`Validated 49 pages and 50 canonical topics.`, exit status 0. By the lead's account the
`review-docs` skill was used for `docs/project/repository-map.md`, but its first step's
`docs/reference/documentation-contract.md` was not opened (section 9).

#### Step 8: the lists against the tree

`ev-marked-names.txt`, written at 04:04:17Z, from the search script `marked.sh` quoted
at its end, with controls on hand-made text first. Its summary:

```text
6e87e357c28fc9dbb59b7097435e0c34930b2ad9 with uncommitted changes
...
model.State                  consuming_test core_test decay_test expiry_test inspection_test lease_end_test lease_test publish_test pull_test settle_test
model.new                    consuming_test core_test decay_test expiry_test lease_end_test lease_test publish_test pull_test settle_test
model.HeldRecord             none
...
model.replace_lease          expiry_test
...
inspection.snapshot          expiry_test inspection_test
invariants.Invariant         none
invariants.registry          coverage interface_test invariants_test
```

Every other marked name is `none`. The tests use five marked names: `model.State`,
`model.new`, `model.replace_lease`, `inspection.snapshot` and `invariants.registry`.
`interface_test`'s hit is the string `"invariants.registry"` in the pin's own list, not
a use (departure 9). `guide_test.gleam` names no marked name: its section lists the
fifteen headings with no hit, and its positive control finds `model.pocket` at line 128.
No stop was owed.

`ev-probes-precommit.txt` (written at 04:10:26Z) ran Verification's probes as they then
stood, before the checkpoint, without `git rev-parse HEAD`, `just check` and the
`git status` test, under `sh -x`; it ended on `just check-docs`, exit status 0. The lines
it removed sit at the line numbers they hold in
`ai_tmp/review/GD10-1/verification.sh.before`, the triage's copy of the script before
its fixes.

### 6. Checkpoint record

One checkpoint, step 9: conformance, breakage and design, before the work commit, over
the base and the uncommitted diff.

- **Reviewer and model.** One reviewer subagent per lens started Codex, `gpt-6-astra`,
  in the form `ai_tmp/review/reviewer-subagent.md` gives: `codex exec -m gpt-6-astra -s
  read-only -C "$PWD" -o "$DIR/last-$LENS.md" "$(cat "$DIR/brief-$LENS.txt")" <
  /dev/null >| "$DIR/log-$LENS.txt" 2>&1 &`. The exact command lines are not saved. Each
  `log-<lens>.txt` opens `OpenAI Codex v0.169.1`, `model: gpt-6-astra`,
  `sandbox: read-only`, `reasoning effort: ultra`; the process ids are in
  `pid-<lens>.txt` (written at 04:11:31Z and 04:11:35Z). `codex exec --help` for the
  day is `ai_tmp/review/codex-exec-help-2026-11-23.txt` (`codex-cli 0.169.1`), last
  written at 02:59:30Z, for GD09, the same UTC day; it was not rerun. Codex was on
  `PATH`, so no fallback was needed. Every subagent in this ticket ran on
  `claude-opus-5-5`, by the lead's account.
- **Exit and recovery.** All three Codex processes exited normally, by the lead's
  account; each log ends on the review's closing lines (`log-conformance.txt` at
  04:15:43Z, `log-design.txt` at 04:16:21Z, `log-breakage.txt` at 04:17:53Z). Each
  `findings-<lens>.md` opens `Source: last-message`.
- **Brief.** `brief-<lens>.txt` is `brief-common.txt` followed by `lens-<lens>.txt`
  (`cmp`, for this record). It names the ticket, the series index, the shared rules,
  the base, `git diff -M 6e87e357c28fc9dbb59b7097435e0c34930b2ad9` plus every untracked
  file read in full, the read limits, the step-9 questions and the uncommitted
  `verification.sh`. The conformance lens conforms to "Decisions fixed", not to the
  specification, and the design lens asks step 9's design questions. The brief says what
  is still to come, and gives no hand-back conclusion. The three `lens-<lens>.txt` were
  last written at 03:38:59Z, before `base.txt`.
- **Triage.** By the lead's account, one separate subagent, neither a reviewer nor the
  lead, checked each finding by a probe and applied the fixes; its verdicts are
  `verdicts.md` (written at 04:28:39Z).
- **Rounds.** One. No finding was high, so no second round was owed.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | medium, lowered to low | valid | The pin's import parser joined a comment line inside a name list into the list, so an unqualified use could escape the scan; the pinned formatter moves such comments out, so the gate cannot reach it today | `statement` drops each line from its first `//`; three controls in `reads_names` |
| C2 | low | valid | The corrected script read `base` from ignored scratch | The ticket's block opens with `git cat-file -e "$base^{commit}"` and keeps `base` set before the script |
| B1 | medium | valid | A table row with a leading space, or no spaces in its cells, escaped `table_names` and both Verification table probes, and markdownlint accepts it | `table_names` trims rows and reads the first cell, returning a stray spelling raw; both probes allow spaces before a row and inside its first cell |
| B2 | low | valid | No pin assertion said that a module under `internal/`, the root, `failure` or `answers` carries no mark | `mark_lines`, its control `reads_mark_lines`, and `marks_per_module` |
| B3 | low | valid | `sort` sat in the middle of four pipelines, so its failure was lost under `sh` | `sort` fills `sorted` in its own assignment at all four sites |
| D | none | none | The design lens reported no findings | None |

The evidence, all in `ai_tmp/review/GD10-1/` and run at
`6e87e357c28fc9dbb59b7097435e0c34930b2ad9` with uncommitted changes; the recipe records
among them name no commit:

- `ev-C1-red.txt` (04:22:56Z), the control on the unchanged parser:
  `FAIL interface_test.a module names m.f qualified on a line that is no comment, or
  unqualified in its import: ...`, `#(Function, "reads_names"), #(Line, 1129)`,
  `345 tests run, 344 passed`, exit status 1. `ev-C1-formatter.txt`: gleam 1.19.1's
  `gleam format --stdin` moves the comment out of the braces, for a short list and a
  wrapped one, `exit 0` each.
- `ev-B1-red.txt` (04:23:16Z): `FAIL interface_test.a table's names are its rows' first
  cells, under its heading: ...`, `Left` `["core.run", "rules.dispatch"]`, `Right`
  `["core.run", "rules.dispatch", "core.indented", "core.tight", "core.bare"]`,
  `345 tests run, 344 passed`, exit status 1. `ev-B1-nolead-docs.txt` (04:28:17Z): a
  scratch row with no leading pipe fails `just check-docs` on
  `MD055/table-pipe-style ... Missing leading pipe`, exit status 1; the README was then
  restored from `README.backup`, byte-identical by `cmp`, by `verdicts.md`'s account.
- `ev-B2-red.txt` (04:23:52Z), with `@internal` inserted above `pub fn run` in
  `internal/core.gleam` (`ev-B2-scratch-diff.txt`: `124a125`, `> @internal`):
  `FAIL interface_test.only model, inspection and invariants carry marks: 12, 1 and 2:
  ...`, `Left` lists `#("src/pachinko_double/internal/core.gleam", 1)` beside the three,
  `347 tests run, 346 passed`, exit status 1. The file was restored from
  `core.gleam.backup`, byte-identical by `cmp`, by `verdicts.md`'s account.
- `ev-verif-controls.txt` (04:24:42Z): C2's script stops with `base: unbound variable`
  (exit status 1) when `base` is unset and with `fatal: Not a valid object name ...`
  (exit status 128) on a missing commit; B1's row count is 4 under the old pattern and 6
  under the new, and the `inspection.empty` control 1 and 3; B3's script with a failing
  `sort` passes under the old form (exit status 0) and stops under the new (exit
  status 2).
- `ev-C2-block-diff.txt` (04:25:11Z): `tail -n +7 ai_tmp/gd/GD10/verification.sh`
  against the ticket's block, `diff exit status: 0`.

Applied by the triage to `gleam-double/test/interface_test.gleam`, the ticket's
Verification block and the paragraphs around it, and `ai_tmp/gd/GD10/verification.sh`.
By `verdicts.md`'s account no file under `gleam-double/src/` and not the guide changed
in triage; for the two files its scratch edits touched, `README.backup` and
`core.gleam.backup`, taken before those edits, match the work commit's files (`cmp`,
for this record). Nothing was handed back.
The reruns after the fixes, uncommitted, on base
`6e87e357c28fc9dbb59b7097435e0c34930b2ad9`: `ev-format.txt`
(`just gleam-double-format`), exit status 0; `ev-final.txt`, written at 04:25:32Z,
`347 tests run, 347 passed`, `coverage faults: 0`, exit status 0; `ev-docs.txt`
(`just check-docs`), `Validated 49 pages and 50 canonical topics.`, exit status 0; and
`ev-verification-smoke.txt` (04:24:21Z), the corrected `verification.sh` under `sh -x`
without `just gleam-double`, `just check-docs`, `just check` and the `git status` test,
ending `exit status: 0`.

### 7. Verification at the work commit

At `21791e62978c60909a9c3602e278c23f45c99eb2`, the work commit, on a clean tree:
`ev-verification-at-work.txt`, written at 04:29:25Z, is `ai_tmp/gd/GD10/verification.sh`
under `sh -x`. The script is four header lines (`#!/bin/sh` and three comment lines),
`set -eu` and `base=$(cat ai_tmp/gd/GD10/base.txt)`, followed by this ticket's corrected
block verbatim, which opens with its own `set -eu` and `git cat-file -e`; a `diff` of the
block against the script, made for this record before it was written, shows nothing
else. At the run the corrected block was not yet committed in this ticket, and
`git status --short` printed nothing, so the ticket's edit was set aside for the run as
`ai_tmp/gd/GD10/ticket-triage.patch` (written at 04:28:56Z). That patch matches the
ticket's uncommitted diff before this record was written (`cmp`, for this record).

It exited 0, so every `test` held: `gleam-double/src/` holds the root and the
directory, `src/pachinko_double/` the eight and `internal`, and `internal/` the eleven;
no import of a moved module by its old path under `src/` or `test/`; no vocabulary module
and not `guide_test.gleam` imports `pachinko_double/internal/`; the fifteen marks each
directly above its `pub`, `inspection.empty` unmarked, the counts 12, 1 and 2, and only
those three files marked; six table rows, and no `inspection.empty` row; no old path in
the guide, `SPEC-NOTES.md` or the repository map; no aliased import under `src/`;
`gleam.toml`, `manifest.toml` and the five untouched vocabulary modules unchanged; the
marked modules' patch only headers and marks; no `erl_crash.dump`. Each probe's positive
control printed its expected count first. The two `273 tests run: 273 passed, 0 skipped`
lines inside `just check` are the Rust workspace's tests, not the double's.

```text
$ sh -x ai_tmp/gd/GD10/verification.sh
+ set -eu
++ cat ai_tmp/gd/GD10/base.txt
+ base=6e87e357c28fc9dbb59b7097435e0c34930b2ad9
+ set -eu
+ git cat-file -e '6e87e357c28fc9dbb59b7097435e0c34930b2ad9^{commit}'
+ git rev-parse HEAD
21791e62978c60909a9c3602e278c23f45c99eb2
+ just gleam-double
...
gleam 1.19.1
...
OTP 29
...
347 tests run, 347 passed
coverage faults: 0
...
+ marks=12
+ test 12 -eq 12
...
+ rows=6
+ test 6 -eq 6
...
+ test 2 -eq 2
+ test 1 -eq 1
+ test 5 -ne 3
...
+ test 6 -eq 6
+ test 15 -eq 15
+ test 21 -eq 21
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

### 8. Departures

1. **The import count at the base was 67 lines in 22 files, not 87 in 29**
   (`ev-counts-base.txt`). Its recorded command is
   `rg -c "^import pachinko_double/($moved)" gleam-double/src gleam-double/test`, without
   the ticket's `\b`. The guide's count, 35, matched the ticket.
2. **The formatter moved lines the ticket did not name.** `just gleam-double-format`
   re-sorted imports and rewrapped name lists after the hand edits of step 3 (one file,
   `core.gleam`) and step 4 (20 files: the root, nine moved modules and ten test modules)
   (`format-diff.txt`). Every changed line in it is an import line, a line of an import's
   name list or its closing `}` (`grep`, for this record).
3. **The implementer ran each producer alone** before the checkpoint, in
   `verification.sh`. As first written, `ls` and `sort` fed pipes whose last status
   alone was read; each now fills a variable of its own (`ev-verif-fix-producer.txt`: the
   old form `exit status: 0` on a producer that fails, the new `exit status: 2`).
4. **The `body` line became `counts`.** Its `|| true` hid a failing `git diff`; `counts`
   allows only `rg`'s exit 1 and checks itself against a hand-made patch that changes a
   body line (`ev-verif-fix-body.txt`: the old line `exit status: 0` on a bad base; the
   control sees `headers=2 changed=5 added=1`, and a removed line starting `--` reads as
   a header, `headers=3`).
5. **The old-import probe's control gained two spellings**, the bare
   `import pachinko_double/decay` and `core_x`, and expects 2; the internal-import probe's
   control gained a name-list line and an indented line, and expects 2
   (`ev-verif-fix-import.txt`: without `\b` the new control counts 3, with `\.` it
   counts 1).
6. **The two `git diff --name-only` probes gained the `seen` control**: the same command
   names `model.gleam`, which did change (`ev-verif-fix-seen.txt`: on an unchanged path
   the control stops, exit status 1).
7. **The pin goes past the ticket in three places**, each read in
   `git diff 6e87e357c28fc9dbb59b7097435e0c34930b2ad9 21791e62978c60909a9c3602e278c23f45c99eb2
   -- gleam-double/test/interface_test.gleam` for this record: `reads_witness` also
   asserts that `pachinko_double/internal/core` is refused; `test_only_faults` also faults
   a listed name that is neither under `internal/` nor marked; and the header's sentence
   on a control's text now reads "a list of lines or a string written with `\n` escapes".
8. **Some prose is the implementer's.** The ticket fixes no wording for the paragraph
   under "What is pub only for the tests" in the guide, the root header's closing
   sentences, or the repository map's `gleam-double/` sentence.
9. **The step-8 search counts a string literal.** Its hit at `interface_test.gleam:913`,
   `"invariants.registry",`, as the file stood at step 8, is the pin's own list of
   test-only names, not a use (`ev-marked-names.txt`).
10. **The checkpoint corrected Verification and the pin again** (`verdicts.md`). The
    block checks that the supplied `base` names a commit before it runs anything (C2);
    this differs from the lead's note, which put the check above the block. Both table
    probes allow spaces before a row and inside its first cell (B1). `sort` runs last in
    a pipeline of its own (B3). In the pin, an import's name list is read without its
    comments (C1); the guide's table rows are read after trimming, and a row spelled
    otherwise yields its raw cell (B1), while markdownlint's MD055 rejects a row with no
    leading pipe; and a new test asserts that only `model`, `inspection` and `invariants`
    carry marks, 12, 1 and 2, counting raw `@internal` lines across `src/` (B2). Each was
    shown red on its gap first. The pin adds 21 tests, not the implementer's 19.
11. **The ticket's Verification block and the paragraphs around it were rewritten** by
    the triage, though "Files touched" lists this ticket only for `status:` and the
    hand-back. The paragraph before the block describes the `base` check and the rule
    that every command whose failure matters runs alone in an assignment or last in its
    pipeline; the paragraph after it describes the table probes' spacing, the `seen`
    control and `counts`.

### 9. What was not run, and what was not recorded

Not run:

- A push or a pull request. Both need authorisation (step 10); nothing left the worktree.
- `just initialize`, `just gleam-double-deps` or any other network recipe. The review's
  model calls are the only network use, which the checkpoint authorises.
- A second review round. None was owed.
- A review of the triage's fix diff.
- The corrected Verification in full before the work commit. The smoke run left out four
  checks (section 6); section 7's run is the first full one.
- `docs/reference/documentation-contract.md` was not opened for the `review-docs`
  skill's first step at step 7; `just check-docs` passed.
- `gleam docs build` and `gleam export package-interface`, as Non-goals say.

Not recorded: the exact `codex exec` command lines, beyond the form in
`ai_tmp/review/reviewer-subagent.md`; probe 3's `leak.py`; the Codex processes' exit
statuses, beyond the lead's account; and the models of the subagents, beyond the lead's
account.

## Open points

- **The compiler's own view is not shown.** That `internal/` hides a module from the
  package's documentation, and `@internal` a name, is Gleam's documented behaviour. This
  ticket shows only that the build is clean and that the pin holds the layout.
  `gleam docs build` and `gleam export package-interface` would show the rest, and each
  needs a new recipe.
- **Closed: `inspection.empty`.** The maintainer was first shown it as a test-only
  name. Besides the checker's controls, `owner_test.gleam` uses it to write the snapshot
  it expects, which is what a caller's own test would do. On 2026-11-23 the maintainer
  ruled that it stays unmarked, as part of `inspection`'s interface.
- **`Queue` is unmarked and `State` is marked.** A caller never holds either: the
  snapshot gives `next_position` as a field. `Queue` stays in the interface as an entity
  of the specification.
- **The marks bind nothing in this package.** A test can name a marked name, and ten
  test modules build a `State` by hand. The marks and the pin say what the interface is;
  they do not stop a later change from leaning on the engine.
- **The pin's use scan is textual.** It would miss a use through an alias, which is why
  the pin fails on an aliased import under `gleam-double/src/`, and it would count a
  local variable named like a module whose field is named like a listed function. No
  such variable exists at the base.
- **The script behind GD07's open point 29**, which checks the guide's table against
  `src/`, is not committed and is not updated here.
- **The six lookups left in `model`'s interface take lists, not a `State`.** Whether a
  caller's vocabulary should include them, or a caller should search a snapshot's lists
  itself, was not asked. The walkthrough uses `model.pocket` and `model.record`.
