---
id: GD10
title: "Gleam double: the engine and the rules under internal/"
status: todo
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
  `inspection.empty` and a few more. No other module under `gleam-double/src/` names
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
  | `inspection` | `snapshot`, `empty` | 2 |
  | `invariants` | `Invariant`, `registry` | 2 |
  | The root, `config`, `time`, `refusals`, `failure`, `answers` | none | 0 |

  `replace_lease` is named by one test module, `expiry_test`, which builds a state by
  hand. It is marked with its two siblings.
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
  | `inspection` | `Snapshot`, `PocketView`, `ConsumerView`, `Views`, `views` | Each type has one constructor of its own name |
  | `invariants` | `check` | none |

  `model` has 31 `pub` items: 19 unmarked and 12 marked. `Queue` stays unmarked because
  it is an entity of the specification, though only a `State` holds one.
- **The test-only table.** A name is *`pub` only for the tests* when it is a `pub fn` or
  a `pub const` that sits in a module under `internal/` or is marked `@internal`, and no
  other module under `gleam-double/src/` names it. There are seven:

  | Name | What the tests use it for |
  | --- | --- |
  | `core.run` | Running one table, a test's own included, as a whole outcome |
  | `core.run_at` | The same with `now` already in hand, as a clock trigger's outcome runs |
  | `core.run_due` | Running outcomes a test made by hand, one after another, as a tick does |
  | `core.registry` | The coverage check reads the rules from it |
  | `rules.dispatch` | Asking which row's guard holds, without running its effect |
  | `inspection.empty` | The snapshot of a new queue, to build a state by hand on |
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
  - The guide's test-only table and the pin's own list of seven hold the same names.
  - Each of the seven is a `pub fn` of its module, and no other module under
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
   for the fourteen marks not yet written, and nothing else may. Save
   `git diff -M "$base" -- gleam-double/src` and its `--stat`, and show that each of the
   eleven is a rename whose changed lines all belong to `import` statements.
5. **Green: the marks.** Add the other fourteen `@internal` lines. `just gleam-double`
   builds with no warning, which is the third probe; quote it. Every assertion of step 2
   passes, and the run is green.
6. **Red, then green: the test-only table.** Add the pin's assertions about the seven
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
   - Is any of the seven test-only names in fact named by a module under
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
- `model`, `inspection` and `invariants` carry exactly the 12, 2 and 2 marks of
  "Decisions fixed", and no other module under `gleam-double/src/` carries one. The pin
  asserts each module's unmarked and marked lists.
- No vocabulary module imports a module under `pachinko_double/internal/`, and no module
  under `gleam-double/src/` imports another under an alias. The pin asserts both.
- The guide's test-only table lists exactly the seven names, and the pin holds the table
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
that fails. `base` is the commit step 1 records; set it before the script runs. Each
probe that passes by printing nothing follows its positive control, and no probe reads
`rg`'s status through a pipe. If the first probe's fallback was taken for a kind, types
or functions, the builder changes the script for that kind only and says so under
Departures: each `rg -qU` line for an item of that kind looks for the doc line above its
`pub`, each count of marks adds the doc lines to the `@internal` lines, and the `body`
line also passes over an added doc line.

```sh
set -eu
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

src=gleam-double/src/pachinko_double
root=gleam-double/src/pachinko_double.gleam
outer=$(ls gleam-double/src | LC_ALL=C sort | tr '\n' ' ')
test "$outer" = "pachinko_double pachinko_double.gleam "
top=$(ls "$src" | LC_ALL=C sort | tr '\n' ' ')
test "$top" = "answers.gleam config.gleam failure.gleam inspection.gleam internal invariants.gleam model.gleam refusals.gleam time.gleam "
inner=$(ls "$src/internal" | LC_ALL=C sort | tr '\n' ' ')
test "$inner" = "consuming.gleam core.gleam decay.gleam domain.gleam expiry.gleam identity.gleam lease_end.gleam publish.gleam pull.gleam rules.gleam settle.gleam "

moved='rules|domain|core|identity|publish|pull|settle|lease_end|expiry|decay|consuming'
pattern="^import pachinko_double/($moved)\b"
probe=$(printf 'import pachinko_double/core.{type Trigger}\nimport pachinko_double/internal/core\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" gleam-double/src gleam-double/test; then exit 1; else test "$?" -eq 1; fi

pattern='^import pachinko_double/internal/'
probe=$(printf 'import pachinko_double/internal/core\n' | rg -c "$pattern")
test "$probe" -eq 1
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
rg -qU '^@internal\npub fn empty\b' "$src/inspection.gleam"
rg -qU '^@internal\npub type Invariant\b' "$src/invariants.gleam"
rg -qU '^@internal\npub fn registry\b' "$src/invariants.gleam"
marks=$(rg -c '^@internal$' "$src/model.gleam")
test "$marks" -eq 12
marks=$(rg -c '^@internal$' "$src/inspection.gleam")
test "$marks" -eq 2
marks=$(rg -c '^@internal$' "$src/invariants.gleam")
test "$marks" -eq 2
marked=$(rg -l '^@internal' gleam-double/src | LC_ALL=C sort | tr '\n' ' ')
test "$marked" = "$src/inspection.gleam $src/invariants.gleam $src/model.gleam "

seams='core\.run|core\.run_at|core\.run_due|core\.registry|rules\.dispatch|inspection\.empty|invariants\.registry'
rows=$(rg -c "^\| \`($seams)\` \|" gleam-double/README.md)
test "$rows" -eq 7

pattern="src/pachinko_double/($moved)\.gleam"
probe=$(printf 'src/pachinko_double/publish.gleam\nsrc/pachinko_double/internal/publish.gleam\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" gleam-double/README.md gleam-double/SPEC-NOTES.md docs/project/repository-map.md; then exit 1; else test "$?" -eq 1; fi

pattern='^import pachinko_double.* as |^\} as '
probe=$(printf 'import pachinko_double/internal/core as engine\n} as engine\nimport pachinko_double/model\n' | rg -c "$pattern")
test "$probe" -eq 2
if rg -n "$pattern" gleam-double/src; then exit 1; else test "$?" -eq 1; fi

pins=$(git diff --name-only "$base" -- gleam-double/gleam.toml gleam-double/manifest.toml)
test -z "$pins"
untouched=$(git diff --name-only "$base" -- "$src/config.gleam" "$src/time.gleam" "$src/refusals.gleam" "$src/failure.gleam" "$src/answers.gleam")
test -z "$untouched"
body=$(git diff "$base" -- "$src/model.gleam" "$src/inspection.gleam" "$src/invariants.gleam" | rg '^[+-]' | rg -v '^(\+\+\+|---) ' | rg -cv '^\+@internal$' || true)
test -z "$body"

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The `body` line counts the changed lines of the three marked modules that are not an
added `@internal`, and expects none: `rg -c` prints nothing and exits 1 when it counts
zero, which `|| true` allows. The script does not show that a moved module changed only
in its imports, because `gleam format` may rewrap an import; step 4's saved diff shows
that. The builder runs each control first, and corrects a pattern, saying so under
Departures, if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the probes; the red steps; the checkpoint's record; Verification at the work commit;
departures; and what was not run.

## Open points

- **The compiler's own view is not shown.** That `internal/` hides a module from the
  package's documentation, and `@internal` a name, is Gleam's documented behaviour. This
  ticket shows only that the build is clean and that the pin holds the layout.
  `gleam docs build` and `gleam export package-interface` would show the rest, and each
  needs a new recipe.
- **`inspection.empty` is listed as test-only, as the maintainer was shown it on
  2026-11-23.** Besides the checker's controls, `owner_test.gleam` uses it to write the
  snapshot it expects, which is what a caller's own test would do. The maintainer may prefer it unmarked, as part of
  `inspection`'s interface.
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
