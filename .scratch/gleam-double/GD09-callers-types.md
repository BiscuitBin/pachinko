---
id: GD09
title: "Gleam double: the caller's types in public modules"
status: done
depends_on: [GD08]
parallel_with: []
branch: build-gleam-double
estimated_size: L
---

# GD09: Gleam double: the caller's types in public modules

## Context

After GD08 a caller starts at the root module, `gleam-double/src/pachinko_double.gleam`.
To name what it sends and what it reads back, it must still import the engine. The facts
below are those of commit `aebae0afadfb34a2d8ae762316376f70d3df08fe`, with GD08's move
read in: where this ticket says "the root", that commit still has
`gleam-double/src/pachinko_double/owner.gleam`.

| A caller names | Defined in | Kind of module |
| --- | --- | --- |
| `Failure`, with `Refused`, `GuardOverlap`, `Defect` and `BrokenInvariant` | `rules.gleam` | The rule-table machinery |
| `Cause`, with `NoRuleHeld` and `EffectFailed` | `rules.gleam` | The rule-table machinery |
| `Answer`, with `Batch` and `NoPocketAvailable`; `BatchRecord` | `domain.gleam` | The machinery bound to this domain |
| `TickReport` | `core.gleam` | The apply step |
| `LeaseView`, `HeldView` | `consuming.gleam` | A rule module |
| `Publication` | `publish.gleam` | A rule module |

The walkthrough in `gleam-double/test/guide_test.gleam` is the nearest thing the double
has to a caller: it drives the queue through the test harness and reads every reply. It
imports `pachinko_double/core` for `TickReport` and `pachinko_double/domain` for `Batch`
and `BatchRecord`.

A Gleam module cannot re-export a name, so the root cannot hand these types on. It also
cannot hold them itself: the root imports `core`, and `rules`, `core`, `consuming` and
`pull` each need one of them, which would make a cycle. They have to live in modules
below the engine in the import graph. That graph shows where there is room: `refusals`
and `time` import nothing from the package, `config` imports those two, and `model`
imports `config` and `time`.

GD10 puts the engine and the rule modules under `internal/`. This ticket has to come
first, so that no type a caller names is left behind in them.

## Goal

A caller that drives the queue and reads every reply imports only the root, the public
vocabulary modules and test support. `gleam-double/test/guide_test.gleam` is the
witness, and the pin holds it.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer fixed the first group on 2026-11-23. The builder does not reopen them.

- **`gleam-double/src/pachinko_double/failure.gleam` is new and imports nothing.** It
  holds the generic `Failure(refusal)` and `Cause`, moved from `rules.gleam` with their
  doc comments and their constructors unchanged:

  | Type | Constructors |
  | --- | --- |
  | `Failure(refusal)` | `Refused`, `GuardOverlap`, `Defect`, `BrokenInvariant` |
  | `Cause` | `NoRuleHeld`, `EffectFailed` |

- **`gleam-double/src/pachinko_double/answers.gleam` is new and imports only
  `pachinko_double/model` and `pachinko_double/time`.** It holds five types, moved with
  their doc comments and their constructors unchanged:

  | Type | Constructors | Moved from |
  | --- | --- | --- |
  | `Answer` | `Batch`, `NoPocketAvailable` | `domain.gleam` |
  | `BatchRecord` | `BatchRecord` | `domain.gleam` |
  | `TickReport` | `TickReport` | `core.gleam` |
  | `LeaseView` | `LeaseView` | `consuming.gleam` |
  | `HeldView` | `HeldView` | `consuming.gleam` |

- **`Publication` stays in `publish.gleam`.** The root's `publish` stops taking one. It
  takes the five values instead: `publisher`, `pocket_key`, `record_key`, `rank` and
  `payload`, in the order of the specification's
  `Publish(publisher, pocket_key, record_key, rank, payload)`.
- **No behaviour changes**, and no function body changes except the four named below.
- **The pin gains the two new modules' names, and the witness**: the rule that
  `gleam-double/test/guide_test.gleam` imports no engine module and no rule module.

The second group is this ticket's own design, written from a scan of the tree at the
commit Context names. The builder follows it, and reports under Departures if one of
these cannot hold.

- **The five values are plain arguments.** What the maintainer was first shown on
  2026-11-23 called them labelled, "as every other trigger function already does". The
  other trigger functions take plain positional arguments, so `publish` matches them,
  and the maintainer confirmed plain arguments the same day:

  ```gleam
  pub fn publish(
    owner: Subject(Message),
    publisher: String,
    pocket_key: String,
    record_key: String,
    rank: Int,
    payload: BitArray,
  ) -> Reply
  ```

  Its body builds the `Publication` and sends `core.Publish` as before.
- **The four bodies that change.** The root's `publish`, as above.
  `harness.publish_publication` in `gleam-double/test/harness.gleam`, which keeps its
  signature and passes the publication's five fields on. The one call of the root's
  `publish` in `gleam-double/test/owner_test.gleam`, which passes five values where it
  built a `Publication`. And `tick_stops_at_a_failed_lapse` in
  `gleam-double/test/expiry_test.gleam`, which writes `rules.BrokenInvariant` with its
  qualifier: the qualifier becomes `failure`. One signature changes the same way, and no
  body with it: `marked` in `gleam-double/test/settle_test.gleam` returns a
  `consuming.HeldView`, which becomes `answers.HeldView`. The command is in "Files
  touched".
- **The root writes the failure type out.** Today the root imports `domain` for the
  alias `Failure`, which is `rules.Failure(Refusal)`. After this ticket it imports
  `failure.{type Failure}` and `refusals.{type Refusal}` and writes `Failure(Refusal)` in
  four places: `Reply`, `PartialTick`, the reply of `Message`'s `LeaseView` constructor,
  and `lease_view`'s result. It imports `answers` for
  `Answer`, `LeaseView` and `TickReport`, and no longer imports `domain`. The types are
  the same types, so no caller changes.
- **`domain.gleam` keeps its aliases**, for the engine and its tests: `Plan`, `Row`,
  `Table`, `Failure`, `Applied`, `Step`, `Cursor` and `World`, and the type `Event`. Its
  `Failure` becomes `failure.Failure(Refusal)`, and its aliases take `Answer` from
  `answers`.
- **The defining modules import what they lost.** `rules.gleam` imports `failure` for
  `Failure`, `Cause` and their constructors. `core.gleam` imports `failure` and `answers`.
  `consuming.gleam` and `pull.gleam` import `answers`. Each keeps writing the names
  unqualified, so that no line of a function body changes.
- **Every other file follows its names.** A module under `gleam-double/test/` that took a
  moved name from its old module takes it from the new one. At the base these are:

  | Moved names | Modules | Test modules that name them |
  | --- | --- | --- |
  | `Failure`'s and `Cause`'s constructors | 13 | `branch_a_test`, `consuming_test`, `core_test`, `expiry_test`, `harness`, `identity_test`, `lease_end_test`, `lease_test`, `owner_test`, `publish_test`, `pull_test`, `rules_test`, `settle_test` |
  | `Batch`, `NoPocketAvailable`, `BatchRecord` | 7 | `competing_test`, `guide_test`, `harness`, `lease_test`, `parking_test`, `publish_test`, `pull_test` |
  | `TickReport` | 8 | `branch_a_test`, `decay_test`, `expiry_test`, `guide_test`, `harness`, `lease_endings_test`, `owner_test`, `parking_test` |
  | `LeaseView`, `HeldView` | 4 | `consuming_test`, `harness`, `lease_endings_test`, `settle_test` |

  The compiler is the check: a moved name taken from its old module does not build.
  `harness` and `pull_test` also import the alias `domain.Failure`; that alias stays, so
  those two lines may stay.
- **The header and "Where to look" follow.** The root's header says the reply types live
  in `failure` and `answers`, and no longer lists engine modules for them. The second
  row of "Where to look" in `gleam-double/README.md` becomes: `config.gleam`,
  `time.gleam`, `model.gleam`, `refusals.gleam`, `failure.gleam` and `answers.gleam`.
- **What the pin gains**, in `gleam-double/test/interface_test.gleam`, with the unit GD08
  fixed:
  - `failure`'s items are exactly the two types above with their six constructors, and
    `answers`'s the five types above with their six constructors. None is internal.
  - `failure.gleam` holds no line that starts with `import`. The lines of
    `answers.gleam` that start with `import` name exactly `pachinko_double/model` and
    `pachinko_double/time`.
  - **The witness.** Every line of `gleam-double/test/guide_test.gleam` that starts with
    `import` names one of: a module under `gleam/`; `simplifile`; the support modules
    `harness` and `runner`; the root, `pachinko_double`; or one of the eight vocabulary
    modules `pachinko_double/config`, `time`, `model`, `refusals`, `failure`, `answers`,
    `inspection` and `invariants`. The module a line names is the text after `import`,
    up to the first space or `.`.
  - The root's 17 items still stand, from GD08.

## Non-goals

- No behaviour change, and no change to what any trigger answers.
- No test under a `Scenario` tag changes beyond its imports. The two tests whose bodies
  change, `partial_tick_keeps_what_ran` in `owner_test.gleam` and
  `tick_stops_at_a_failed_lapse` in `expiry_test.gleam`, carry no tag.
- No fenced block of `gleam-double/README.md` changes: the walkthrough's blocks are
  function bodies of `guide_test.gleam`, and only its imports change.
- No module moves under `internal/`, and nothing is marked `@internal`: that is GD10.
- No rename of a moved type or constructor, and no change to a field.
- No move of `Publication`, `Trigger`, `Event`, `Snapshot` or any type this ticket does
  not list.
- No change to `gleam-double/gleam.toml`, `gleam-double/manifest.toml`, the `Justfile` or
  anything under `docs/specs/`, `docs/decisions/`, `crates/` or `double/`.
- No new spec note (shared rule 5): the change interprets nothing in the specification.

## Shared rules

Shared rules 1 to 6 bind this ticket as `.scratch/gleam-double/GD07-close-out.md` writes
them under "Shared rules (GD01 to GD07)", with rule 4's second sentence as
`.scratch/gleam-double/GD08-front-door.md` restates it under "Shared rules". They are not
copied here. Rule 3's coverage check stays strict at 93, 32 and 35. Under rule 1,
`double/pachinko_double/` and `double/tests/` stay unread.

Rule 4's "Failures are values" is untouched: `Failure` is the same type in a new module,
and the owner still answers with it and keeps its state.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/gleam-double/GD07-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `GD09`, and `<n>` is 1: this ticket has one checkpoint, at step 8.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names this ticket's path and its "Decisions fixed in this
  ticket", not `docs/specs/pachinko.allium`: the ticket changes no behaviour.
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff.
- A runner polls on `GD09-1/last-<lens>.md` or the process id, never the bare file name.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `gleam-double/src/pachinko_double/failure.gleam` | new | `Failure` and `Cause` |
| `gleam-double/src/pachinko_double/answers.gleam` | new | The five reply types |
| `gleam-double/src/pachinko_double/rules.gleam` | shared, own lines | Loses the two types; imports `failure` |
| `gleam-double/src/pachinko_double/domain.gleam` | shared, own lines | Loses `Answer` and `BatchRecord`; its aliases follow |
| `gleam-double/src/pachinko_double/core.gleam` | shared, own lines | Loses `TickReport`; imports follow |
| `gleam-double/src/pachinko_double/consuming.gleam` | shared, own lines | Loses the two views; imports `answers` |
| `gleam-double/src/pachinko_double/pull.gleam` | shared, own lines | Its import of the batch names |
| `gleam-double/src/pachinko_double.gleam` | shared, own lines | Imports, `publish`, the four written-out failure types, the header |
| `gleam-double/test/interface_test.gleam` | shared, own lines | What the pin gains |
| `gleam-double/test/harness.gleam`, `gleam-double/test/owner_test.gleam`, `gleam-double/test/expiry_test.gleam` | shared, own lines | Imports, and the three test bodies named above |
| The other test modules of the table in "Decisions fixed" | shared, own lines | Imports only, and a qualifier where a moved name was written qualified |
| `gleam-double/README.md` | shared, own lines | "Where to look", and the lines that give a moved type's module |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | Only a line that gives a moved type's old module, if one does |
| `.scratch/gleam-double/GD09-callers-types.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD09's Status cell |

The path `gleam-double/src/pachinko_double.gleam` exists once GD08 is done, and so does
`gleam-double/test/interface_test.gleam`. No other module under `gleam-double/src/`
changes: `identity`, `publish`, `settle`, `lease_end`, `expiry`, `decay`, `config`,
`time`, `model`, `refusals`, `inspection` and `invariants` name none of the moved names.

At the base, `gleam-double/README.md` gives a moved type's module on two lines, both
`rules.Failure`. The command is
`rg -n 'rules\.(Failure|Refused|Defect)|domain\.(Answer|Batch)|core\.TickReport|consuming\.(LeaseView|HeldView)' gleam-double/README.md gleam-double/SPEC-NOTES.md`,
and it prints nothing for `gleam-double/SPEC-NOTES.md`. The guide's sentence about
`consuming.lease_view`, the function, stays true and stays.

At the base a moved name is written with its old module's qualifier on three lines: the
alias in `domain.gleam`, and the two test lines "Decisions fixed" names. The command:

```sh
rg -n '\b(rules\.(Refused|GuardOverlap|Defect|BrokenInvariant|NoRuleHeld|EffectFailed|Failure|Cause)|domain\.(Answer|Batch|NoPocketAvailable|BatchRecord)|core\.TickReport|consuming\.(LeaseView|HeldView))\b' gleam-double/src gleam-double/test
```

## Steps

Each red step quotes the failing test's name and its message verbatim, or the compiler's
error, headed by the full commit id and "with uncommitted changes". Evidence goes under
`ai_tmp/gd/GD09/`.

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD08. If `.pixi/`,
   `.tools/` or `gleam-double/build/packages/` is missing, run `just initialize` and
   `just gleam-double-deps` (network, authorised by this step). Run `just check` and
   `just gleam-double` on the untouched worktree. Quote each one's last line and the
   line `N tests run, N passed`, headed by the commit.
2. **Red: the pin.** Add to `interface_test.gleam` the assertions of "What the pin
   gains". They fail: neither new file can be read, and `guide_test.gleam` imports
   `pachinko_double/core` and `pachinko_double/domain`. Run `just gleam-double-format`
   first, then quote the failures. Add a control for the witness's parser on text made
   by hand: a line `import pachinko_double/core` is named, and so is one that takes the
   module with unqualified names or under an alias. Write that text as GD08 says a
   control's text is written, so that no line of `interface_test.gleam` starts with
   `import pachinko_double/core`.
3. **Green: `failure.gleam`.** Move `Failure` and `Cause`. Fix the imports in `rules`,
   `domain` and `core`, and in the 13 test modules. `just gleam-double` passes except for
   the pin's `answers` and witness assertions.
4. **Green: `answers.gleam`.** Move the five types. Fix the imports in `domain`, `core`,
   `consuming`, `pull` and the root, and in the test modules. The root stops importing
   `domain`. The pin passes.
5. **Red, then green: `publish`.** Change the call in `owner_test.gleam` to pass five
   values, and quote the compiler's error as the red. Then change the root's `publish`
   and `harness.publish_publication`. Run `just gleam-double-format`, then
   `just gleam-double`, which passes with the baseline's count plus the pin's new tests.
6. **The moved definitions are unchanged.** For each of the seven types, show that the
   lines removed from the old module and the lines added to the new one are the same
   text. Save the comparison under `ai_tmp/gd/GD09/` and quote its result.
7. **Words.** Update the root's header, the second row of "Where to look" and the two
   `rules.Failure` lines of `gleam-double/README.md`. Run `just gleam-double`, which
   checks the guide against `guide_test.gleam`, and `just check-docs`.
8. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Is any moved type, constructor, field or doc comment different from what it was?
   - Does any public signature of the root still name a type from `rules`, `domain`,
     `core`, `consuming`, `publish`, `pull` or `settle`?
   - Does `failure.gleam` or `answers.gleam` import anything that would stop GD10 from
     leaving it public: any of the eleven modules GD10 moves?
   - Would the witness fail if `guide_test.gleam` imported `pachinko_double/core` again,
     with unqualified names, or under an alias?
   - Did any function body change other than the four named in "Decisions fixed"?
   - Did any test under a `Scenario` tag change beyond its imports?
   - Is every fenced block of `gleam-double/README.md` unchanged?
   - Does the coverage check still collect every test and stay strict at 93, 32 and 35?
   - Could a probe in Verification print nothing while its claim is false?
9. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `failure.gleam` holds `Failure` and `Cause` and imports nothing. `answers.gleam` holds
  the five types and imports only `pachinko_double/model` and `pachinko_double/time`.
  The pin asserts both.
- Each of the seven types is defined once under `gleam-double/src/`, in its new module,
  with the text it had (step 6).
- The root's `publish` takes five plain values, and the root does not import
  `pachinko_double/domain`.
- `gleam-double/test/guide_test.gleam` imports only what the witness allows, and the pin
  asserts it.
- Under `gleam-double/src/`, the diff from the base touches exactly the eight files of
  "Files touched".
- Steps 2 and 5 are each quoted red before they turn green.
- `just gleam-double` exits 0 with the coverage check strict at 93, 32 and 35. Its test
  count is the baseline's plus the tests step 2 adds.
- The diff changes no fenced block of `gleam-double/README.md` and no test under a
  `Scenario` tag beyond its imports.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script from the repository root, under `set -eu`. A pipe's status is its
last command's, so every command that reads the tree or git runs in an assignment of its
own or last in its pipe, and only `printf` comes before a `|`. `base` is the commit step
1 records; set it before the script runs. Each probe that passes by printing nothing
follows its positive control.

```sh
set -eu
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

src=gleam-double/src/pachinko_double
root=gleam-double/src/pachinko_double.gleam
test -f "$src/failure.gleam"
test -f "$src/answers.gleam"

probe=$(printf 'import gleam/list\n' | rg -c '^import ')
test "$probe" -eq 1
if rg -n '^import ' "$src/failure.gleam"; then exit 1; else test "$?" -eq 1; fi
pattern='^import [^ .]+'
probe=$(printf 'import pachinko_double/model2\n' | rg -o "$pattern")
test "$probe" = "import pachinko_double/model2"
probe=$(printf 'import pachinko_double/time.{type Timestamp}\n' | rg -o "$pattern")
test "$probe" = "import pachinko_double/time"
raw=$(rg -o "$pattern" "$src/answers.gleam")
sorted=$(printf '%s\n' "$raw" | LC_ALL=C sort)
imports=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$imports" = "import pachinko_double/model import pachinko_double/time "

# each type is defined once, with a body, in its new module; an alias has no `{`,
# and a copy left elsewhere counts whether it is public, opaque or private
probe=$(printf 'pub type TickReport {\npub opaque type TickReport {\ntype TickReport {\npub type TickReport(a1) {\npub type TickReport =\n' | rg -c '^(pub )?(opaque )?type TickReport(\([a-z0-9_, ]*\))? \{$')
test "$probe" -eq 4
for pair in failure:Failure failure:Cause answers:Answer answers:BatchRecord \
  answers:TickReport answers:LeaseView answers:HeldView; do
  module=${pair%%:*}
  name=${pair##*:}
  pattern="^pub type $name(\([a-z0-9_, ]*\))? \{\$"
  rg -q "$pattern" "$src/$module.gleam"
  homes=$(rg -l "^(pub )?(opaque )?type $name(\([a-z0-9_, ]*\))? \{\$" gleam-double/src)
  test "$homes" = "$src/$module.gleam"
done

pattern='^import pachinko_double/domain\b'
probe=$(printf 'import pachinko_double/domain.{type Failure}\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" "$root"; then exit 1; else test "$?" -eq 1; fi
pattern=': ([a-z_]+\.)?Publication\b'
probe=$(printf '  publication: Publication,\n  publication: publish.Publication,\n' | rg -c "$pattern")
test "$probe" -eq 2
if rg -n "$pattern" "$root"; then exit 1; else test "$?" -eq 1; fi

allowed='^import (gleam/[a-z_/]+|simplifile|harness|runner|pachinko_double|pachinko_double/(config|time|model|refusals|failure|answers|inspection|invariants))(\.\{| as |$)'
probe=$(printf 'import pachinko_double/core.{TickReport}\nimport harness.{type Double}\nimport pachinko_double/domain\nimport pachinko_double.{Outcome}\nimport pachinko_double/core as engine\nimport pachinko_double/answers as answer\n' | rg -c "$allowed")
test "$probe" -eq 3
all=$(rg -c '^import ' gleam-double/test/guide_test.gleam)
ok=$(rg -c "$allowed" gleam-double/test/guide_test.gleam)
test "$all" -eq "$ok"

names=$(git diff --name-only "$base" -- gleam-double/src)
sorted=$(printf '%s\n' "$names" | LC_ALL=C sort)
changed=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$changed" = "$root $src/answers.gleam $src/consuming.gleam $src/core.gleam $src/domain.gleam $src/failure.gleam $src/pull.gleam $src/rules.gleam "

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The import probe and the changed-files probe take `rg`'s and `git`'s output in an
assignment of their own, so that `set -eu` stops the script when either fails, even after
it printed the expected lines; only then is the text sorted and joined. The import
probe's module is the text after `import` up to the first space or `.`, as the witness
reads it, and its controls show a name with a digit read whole and a name list cut off.
A type's parameters may hold digits, as in `(a1)`, and the "defined once" control counts
such a line. The builder runs each control first, and corrects a pattern, saying so under
Departures, if one does not behave as written.

## Hand-back notes

Evidence files are under `ai_tmp/gd/GD09/`, and the review record under
`ai_tmp/review/GD09-1/`. None is committed. They come in three shapes. A recipe's own
record (`ev-baseline-*.txt`, `ev-fmt-step*.txt`, `ev-step7-*.txt`, the `*-run.txt` files,
`ev-departure-imports.txt` below its first two lines, and the triage's `ev-format.txt`,
`ev-final.txt` and `ev-docs.txt`) begins `$ <command>`, ends `exit status: N` and names no
commit. A step's digest (`ev-red-pin.txt`, `ev-green-failure.txt`, `ev-green-answers.txt`,
`ev-red-publish.txt`, `ev-green-publish.txt`, `ev-moved-defs.txt`, `ev-bodies.txt`,
`ev-qualifiers-after.txt`), written by the implementer, opens with the commit line and its
own prose and quotes or renders lines of its `*-run.txt`. The triage's probes
(`ev-C1-red.txt`, `ev-C2-red.txt`, `ev-B3-red.txt`, `ev-B1.txt`, `ev-P1.txt`) open with a
`#` line and give each command's `exit: N`. In every quote below the recipe's echo of its
own command lines is left out, and `...` marks other lines left out or a line cut short:
a `FAIL` line is cut after the test's name. Times are UTC, read from the files'
modification times, which are each file's last write, not always the order of the steps;
the work commit's own date is 2026-11-23T19:37:41-08:00, which is 2026-11-24T03:37:41Z.
Where a fact the record should hold was never saved, the text says "not recorded"; where
it rests only on the lead's message, the text says "by the lead's account".

### 1. Base, branch and commits

- Branch: `build-gleam-double`, in this worktree (shared rule 6).
- Base commit: `514c63341f18d77a9963551e7895251b2a8740eb` (`base.txt`, written at
  03:00:15Z), GD08's record commit. It holds GD08's work,
  `5ceb284816f432e65ef0848c7b8554185a7ddfc0`.
- Work commit: `72fe01a206936433299151562c10fa8850281649` (`work-commit.txt`), tree
  `604f560f0c0147196612e96098d7dbe87e90ea4d`, titled "Give the Gleam double's caller its
  types in public modules (GD09)". It touches 28 files (`git show --stat`): the eight
  under `gleam-double/src/` that "Files touched" lists, 19 test modules (the 18 of the
  table in "Decisions fixed" and `interface_test.gleam`), and `gleam-double/README.md`.
  `gleam-double/SPEC-NOTES.md` is not touched: no line of it gives a moved type's old
  module.
- The commit after it holds this record, the index's Status cell, and the ticket's
  corrected Verification block (section 7, departure 6). The corrected block was not in
  the committed ticket at the work commit.
- Nothing was fetched. `.pixi/`, `.tools/` and `gleam-double/build/packages/` were
  present, so step 1's `just initialize` and `just gleam-double-deps` were not run.

### 2. Baseline

At `514c63341f18d77a9963551e7895251b2a8740eb`, on the untouched worktree. Neither file
names its commit; the base id rests on `base.txt`, written before both runs.

`ev-baseline-check.txt`, written at 03:01:05Z:

```text
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

`ev-baseline-gleam.txt`, written at 03:01:09Z:

```text
$ just gleam-double
...
    Running pachinko_double_test.main
319 tests run, 319 passed
coverage faults: 0
exit status: 0
```

### 3. What landed

- **`gleam-double/src/pachinko_double/failure.gleam`**, new, imports nothing. It holds
  `Failure(refusal)`, with `Refused`, `GuardOverlap`, `Defect` and `BrokenInvariant`, and
  `Cause`, with `NoRuleHeld` and `EffectFailed`, moved from `rules.gleam`.
- **`gleam-double/src/pachinko_double/answers.gleam`**, new, imports only
  `pachinko_double/model` and `pachinko_double/time`. It holds `Answer` (`Batch`,
  `NoPocketAvailable`) and `BatchRecord` from `domain.gleam`, `TickReport` from
  `core.gleam`, and `LeaseView` and `HeldView` from `consuming.gleam`, each with its one
  constructor of its own name where it has one.
- **The moved text is unchanged** (step 6, section 4). Each new module opens with a
  `////` header of three lines (departure 2).
- **The defining modules.** `rules`, `core`, `consuming` and `pull` import what they lost
  and keep writing the names unqualified; `domain` keeps its aliases, its `Failure` now
  `failure.Failure(Refusal)` and its aliases taking `Answer` from `answers`. `rules` and
  `consuming` import less than "Decisions fixed" writes (departure 1).
- **The root.** It imports `answers.{type Answer, type LeaseView, type TickReport}`,
  `failure.{type Failure}` and `refusals.{type Refusal}`, and no longer imports
  `domain`, `consuming.{type LeaseView}` or `type TickReport` from `core`. It writes
  `Failure(Refusal)` in `Reply`, `PartialTick`, `Message`'s `LeaseView` reply and
  `lease_view`'s result. `publish` takes `publisher: String`, `pocket_key: String`,
  `record_key: String`, `rank: Int` and `payload: BitArray` after `owner`, and its body
  builds the `Publication` and sends `core.Publish` as before. Its header says that
  `failure` holds `Failure` and `Cause` and `answers` the five types, and no longer names
  an engine module for a reply type or `publish.Publication`.
- **The tests.** The 18 test modules take each moved name from its new module. The four
  changed bodies and the one changed signature are those "Decisions fixed" names, and no
  other line outside the import blocks changed (`ev-bodies.txt`, section 4).
- **The pin**, `gleam-double/test/interface_test.gleam`, gains seven tests: two controls
  of the witness's parser, "an import line names the module up to the first space or
  dot" and "the witness allows gleam/, the root, the vocabulary and test support"; and
  five assertions of the tree, "failure holds exactly Failure and Cause", "failure
  imports nothing", "answers holds exactly the five reply types", "answers imports only
  model and time" and "the walkthrough imports no engine module and no rule module". The
  root's 17 items still stand from GD08.
- **The words.** `gleam-double/README.md`'s second row of "Where to look" names
  `config.gleam`, `time.gleam`, `model.gleam`, `refusals.gleam`, `failure.gleam` and
  `answers.gleam`, and its two `rules.Failure` lines, both prose outside any fence, read
  `failure.Failure`. No fenced block changed.

The test count: 319 at the base, and 326 from step 2 on, the baseline's plus the pin's
seven; all 326 pass from step 4. The coverage check's constants stay 93, 32 and 35
(`ev-green-publish.txt`).

### 4. The red steps, and steps 3 to 7

Every output in this section ran uncommitted, on base
`514c63341f18d77a9963551e7895251b2a8740eb`. The implementing subagent ran on
`claude-opus-5-5`, by the lead's account. `just gleam-double-format` ran before each
`just gleam-double` of steps 2 to 5 and 7, each exit status 0 (`ev-fmt-step2.txt`
to `ev-fmt-step7.txt`); what it changed is in `format-diff.txt` (departure 4).

#### Step 2: red, the pin

`ev-red-pin-run.txt`, written at 03:02:19Z, is the run; `ev-red-pin.txt`, written at
03:03:57Z, is its digest, which quotes its `FAIL` lines whole. Below, each `FAIL` line is
cut after the test's name, and the indented line under it gives the assertion's `Left`
value, then its `Right`, from the same line.

```text
514c63341f18d77a9963551e7895251b2a8740eb with uncommitted changes
...
FAIL interface_test.failure holds exactly Failure and Cause: crashed: ...
  ... #(Value, Error("cannot read src/pachinko_double/failure.gleam: No such file or directory")) ... #(Value, Ok([Item("Cause", ["NoRuleHeld", "EffectFailed"], False) ...
FAIL interface_test.failure imports nothing: crashed: ...
  ... #(Value, Error("cannot read src/pachinko_double/failure.gleam: No such file or directory")) ... #(Value, Ok([])) ...
FAIL interface_test.answers holds exactly the five reply types: crashed: ...
  ... #(Value, Error("cannot read src/pachinko_double/answers.gleam: No such file or directory")) ... #(Value, Ok([Item("Answer", ["Batch", "NoPocketAvailable"], False) ...
FAIL interface_test.answers imports only model and time: crashed: ...
  ... #(Value, Error("cannot read src/pachinko_double/answers.gleam: No such file or directory")) ... #(Value, Ok(["pachinko_double/model", "pachinko_double/time"])) ...
FAIL interface_test.the walkthrough imports no engine module and no rule module: crashed: ...
  ... #(Value, Ok(["pachinko_double/core", "pachinko_double/domain"])) ... #(Value, Ok([])) ...

326 tests run, 321 passed
coverage faults: 0
exit status: 1
```

The two new controls passed: they read text made by hand. The digest counts the `FAIL`
lines of each of the eight controls, GD08's six and the two new ones, as 0.

#### Step 3: green, `failure.gleam`

`ev-green-failure-run.txt`, written at 03:06:58Z, and its digest `ev-green-failure.txt`.
The three failures are the ones step 3 expects, the pin's `answers` and witness
assertions; the digest renders each one's `Left` value after `--`.

```text
514c63341f18d77a9963551e7895251b2a8740eb with uncommitted changes
...
FAIL interface_test.answers holds exactly the five reply types -- left: Error("cannot read src/pachinko_double/answers.gleam: No such file or directory")
FAIL interface_test.answers imports only model and time -- left: Error("cannot read src/pachinko_double/answers.gleam: No such file or directory")
FAIL interface_test.the walkthrough imports no engine module and no rule module -- left: Ok(["pachinko_double/core", "pachinko_double/domain"])

326 tests run, 323 passed
coverage faults: 0
exit status: 1
```

#### Step 4: green, `answers.gleam`

`ev-green-answers-run.txt`, written at 03:07:53Z: `326 tests run, 326 passed`,
`coverage faults: 0`, exit status 0. Its digest records that a first run did not build
on two unused imports in `consuming.gleam`; that run is not saved (departure 1).

#### Step 5: red, then green, `publish`

`ev-red-publish.txt`, written at 03:08:10Z, the digest of `ev-red-publish-run.txt`
(exit status 1). The red is the compiler's error on the five-value call in
`owner_test.gleam`; the warning above it and the recipe's `error:` line are left out.

```text
514c63341f18d77a9963551e7895251b2a8740eb with uncommitted changes
...
error: Incorrect arity
    ┌─ .../gleam-double/test/owner_test.gleam:128:7
    │
128 │       pachinko_double.publish(queue, "p", key, key <> "1", 1, <<key:utf8>>)
    │       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Expected 2 arguments, got 6


error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:128:38
    │
128 │       pachinko_double.publish(queue, "p", key, key <> "1", 1, <<key:utf8>>)
    │                                      ^^^

Expected type:

    publish.Publication

Found type:

    String
...
exit status: 1
```

Then `ev-green-publish-run.txt`, written at 03:08:25Z: `326 tests run, 326 passed`,
`coverage faults: 0`, exit status 0, the baseline's 319 plus the pin's seven.

#### Step 6: the moved definitions

`ev-moved-defs.txt`, written at 03:13:18Z, from `ai_tmp/gd/GD09/moved-defs.sh`, with the
texts it compares under `ai_tmp/gd/GD09/moved/`:

```text
514c63341f18d77a9963551e7895251b2a8740eb with uncommitted changes
$ ai_tmp/gd/GD09/moved-defs.sh  (texts under ai_tmp/gd/GD09/moved/)
Failure: rules.gleam at base -> failure.gleam: 13 lines, same; gone from rules.gleam
Cause: rules.gleam at base -> failure.gleam: 7 lines, same; gone from rules.gleam
Answer: domain.gleam at base -> answers.gleam: 17 lines, same; gone from domain.gleam
BatchRecord: domain.gleam at base -> answers.gleam: 5 lines, same; gone from domain.gleam
TickReport: core.gleam at base -> answers.gleam: 13 lines, same; gone from core.gleam
LeaseView: consuming.gleam at base -> answers.gleam: 6 lines, same; gone from consuming.gleam
HeldView: consuming.gleam at base -> answers.gleam: 5 lines, same; gone from consuming.gleam
control: an altered copy of Failure is reported as different
control: Cause with a constructor dropped is reported as different
exit status: 0
```

`ev-bodies.txt`, written at 03:13:01Z, compares each test module but `interface_test`
with its import block stripped, base against the tree. It lists four places: the
qualifier in `expiry_test.gleam` (`rules.BrokenInvariant` to `failure.BrokenInvariant`),
`harness.publish_publication`'s call passing the publication's five fields, the call in
`owner_test.gleam` passing five values, and `marked`'s result in `settle_test.gleam`
(`consuming.HeldView` to `answers.HeldView`).

#### Step 7: the words

`ev-step7-gleam.txt` (written at 03:13:07Z): `326 tests run, 326 passed`,
`coverage faults: 0`, exit status 0, which includes `guide_test`'s check of the guide's
blocks. `ev-step7-docs.txt` (written at 03:13:10Z): `Validated 49 pages and 50 canonical
topics.`, exit status 0. `ev-qualifiers-after.txt` (written at 03:12:35Z): the ticket's
qualifier command and its README command each exit 1 on the tree, printing nothing.
`ev-probes-precommit.txt` (written at 03:13:14Z) ran Verification's probes from
`ai_tmp/gd/GD09/probes-precommit.sh` under `set -x`, before the review and with the
probes as they then stood, with the changed-files probe altered (departure 5); it ended
on `just check-docs`, exit status 0.

### 5. Checkpoint record

One checkpoint, step 8: conformance and breakage, before the work commit, over the base
and the uncommitted diff.

- **Codex first.** One reviewer subagent per lens started Codex, `gpt-6-astra`, in the
  form `ai_tmp/review/reviewer-subagent.md` gives: `codex exec -m gpt-6-astra -s
  read-only -C "$PWD" -o "$DIR/last-$LENS.md" "$(cat "$DIR/brief-$LENS.txt")" <
  /dev/null >| "$DIR/log-$LENS.txt" 2>&1 &`. The exact command lines are not saved. Each
  `log-<lens>.txt` opens `OpenAI Codex v0.169.1`, `model: gpt-6-astra`,
  `sandbox: read-only`, `reasoning effort: ultra`; the process ids are in
  `pid-<lens>.txt` (written at 03:13:53Z and 03:13:54Z). `codex exec --help` for the day
  is `ai_tmp/review/codex-exec-help-2026-11-23.txt` (`codex-cli 0.169.1`), last written
  at 02:59:30Z, before step 1.
- **Codex stopped on its quota.** Both logs end
  `ERROR: You’ve hit your usage limit. ... try again at Nov 25th, 2026 4:22 PM.`
  (`log-conformance.txt`, last written at 03:16:34Z; `log-breakage.txt`, 03:16:40Z), and
  both `last-<lens>.md` are empty. The partial output was recovered from the logs as
  `codex-partial-conformance.md` and `codex-partial-breakage.md`, each opening
  `Source: log`; neither is a finished review.
- **A departure from GD07's reviewer delta** (departure 7). The delta allows the Claude
  fallback only when `codex` is not on `PATH`, and it was. The lead asked the user, who
  chose, by the lead's account: fresh-context `claude-opus-5-5` reviewers now, with the
  same briefs, and an Astra pass once the quota returns, its valid fixes landing as a
  follow-up commit. `verdicts.md` records the user's approval of the
  Claude reviewers. Their form is `ai_tmp/review/claude-reviewer.md`: read-only, the two
  brief files and nothing else under the review directory, findings sent by message.
  The harness refused their `Write`, so the lead saved `findings-conformance.md` and
  `findings-breakage.md` from their messages, and each file says so; the breakage file
  is the reviewer's summary. Every subagent in this ticket ran on `claude-opus-5-5`, by
  the lead's account.
- **Brief.** `brief-<lens>.txt` is `brief-common.txt` and `lens-<lens>.txt`: the ticket,
  the series index, the shared rules with GD08's rule 4, `tickets/CONVENTIONS.md` §11 and
  §14, the base, `git diff -M` plus every untracked file read in full, the read limits
  and the step-8 questions. The conformance brief conforms to the ticket's decisions, not
  to the specification. It gives no hand-back conclusion. The reviewed tree was the
  uncommitted worktree over `514c63341f18d77a9963551e7895251b2a8740eb`. By
  `verdicts.md`, every fix was to Verification, so no reviewed file under `gleam-double/`
  changed before the work commit.
- **Triage.** One separate subagent, neither a reviewer nor the lead, checked each finding
  by a probe and applied the fixes; its verdicts are `verdicts.md`.
- **Rounds.** One. No finding was high, so no second round was owed.
- **The Astra rerun.** The user then reset Codex's quota, and both Astra lenses ran
  again on GD09, over the committed diff
  `514c63341f18d77a9963551e7895251b2a8740eb..72fe01a206936433299151562c10fa8850281649`,
  in `ai_tmp/review/GD09-astra/`, with the same briefs (rewritten for a committed diff, and
  told not to read the first round's output). Both exited normally;
  each `findings-<lens>.md` opens `Source: last-message`. A separate triage subagent
  checked and applied the findings (`ai_tmp/review/GD09-astra/verdicts.md`). None was
  high, so no further round was owed. No file under `gleam-double/` changed, so no
  follow-up commit was needed: the fixes are in this ticket's Verification block and in
  `ai_tmp/gd/GD09/verification.sh` (departure 8).

  | Id | Severity | Verdict | Reason | Applied |
  | --- | --- | --- | --- | --- |
  | C1 | low | valid | The "defined once" probe's parameter class has no digit, so `pub type Answer(a1) {` is not counted | The class takes digits; the control expects 4 |
  | B1 | medium | valid | `rg \| sort \| tr` and `git diff \| sort \| tr` take `tr`'s status, so a producer that prints and then fails passes; this revises the first round's P1 | Each producer runs in its own assignment; only `printf` feeds a pipe |
  | B2 | low | valid | `^import [a-z_/]+` reads `model2` as `model` | `^import [^ .]+`, with two new controls |

  Red evidence for each, on hand-made text, is `ev-C1-red.txt`, `ev-B1-red.txt` and
  `ev-B2-red.txt` in that directory.

| Id | Source | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- | --- |
| C1 | conformance (Claude) | low | valid | Verification's `Publication` probe misses `publication: publish.Publication` | The probe takes a qualifier; its control takes both spellings |
| C2 | conformance (Claude) | low | valid | The "defined once" probe sees only a `pub type X {` copy, not an opaque or private one | `homes` counts all three and still skips an alias; a new control shows it |
| B1 | breakage (Claude) | low | invalid (moot) | The import forms the pin's parser misses are unformatted, and `just gleam-double` runs `gleam format --check` first (`ev-B1.txt`) | None |
| B2 | breakage (Claude) | low | valid, same as C1 | The same probe | C1's fix |
| B3 | breakage (Claude) | low | valid | The witness regex's control has no ` as ` line | The control gains an aliased engine line and an aliased vocabulary line, and expects 3 |
| F1 | conformance (Codex partial) | none given; low in `verdicts.md` | valid, same as C1 | The same probe | C1's fix |
| P1 | breakage (Codex partial) | none | invalid | Each piped substitution is compared with a non-empty literal, so a failed read fails closed (`ev-P1.txt`) | None |

`codex-partial-conformance.md` says the recovering agent judged F1 low or medium. The
fixes landed in the ticket's Verification block and in `ai_tmp/gd/GD09/verification.sh`;
no file under `gleam-double/` changed in triage. The evidence, all in
`ai_tmp/review/GD09-1/` and run at `514c63341f18d77a9963551e7895251b2a8740eb` with
uncommitted changes, on hand-made text:

- `ev-C1-red.txt`: on a hand-made root whose `publish` takes
  `publication: publish.Publication,`, the old probe `exit: 1`, so it passes on the gap;
  the new one prints `4:  publication: publish.Publication,`, `exit: 0`; on the real
  root the new one `exit: 1`.
- `ev-C2-red.txt`: with an opaque and then a private copy of `TickReport` left in a
  hand-made `core.gleam`, the old `homes` pattern lists only `answers.gleam` and the new
  one both; the new one skips an alias, `exit: 1 (1 = no match)`, and on the real tree
  lists each type's new module once.
- `ev-B3-red.txt`: under a regex with ` as ` dropped, the old four-line control counts
  `2` as the ticket's regex does; the new six-line control counts 3 under the ticket's
  regex and `2` under the broken one.

The reruns after the fixes, uncommitted, on base
`514c63341f18d77a9963551e7895251b2a8740eb`: `ev-format.txt` (`just gleam-double-format`),
exit status 0; `ev-final.txt`, written at 03:36:43Z, `326 tests run, 326 passed`,
`coverage faults: 0`, exit status 0; `ev-docs.txt` (`just check-docs`), `Validated 49
pages and 50 canonical topics.`, exit status 0; and `ev-verification-smoke.txt`, the
corrected `verification.sh` under `sh -x` without its `just` lines, its changed-files
check and its clean-tree check, ending `probes passed`, `exit: 0`.

### 6. Verification at the work commit

At `72fe01a206936433299151562c10fa8850281649`, the work commit, on a clean tree:
`ev-verification-at-work.txt`, written at 03:38:19Z, is `ai_tmp/gd/GD09/verification.sh`
under `sh -x`. The script is this ticket's corrected block with two comment lines above
`set -eu` and two lines after it, `base=$(cat ai_tmp/gd/GD09/base.txt)` and
`git cat-file -e "$base^{commit}"`; a `diff` of the block against the script, made for
this record, shows nothing else. At the run the corrected block was not yet committed in
this ticket, and `git status --short` printed nothing, so the ticket's edit was set aside
for the run. `ticket-triage.patch`, written at 03:37:41Z, matches the ticket's
uncommitted diff before this record was written (`diff`, for this record). This run is
the first of the corrected probes in full.

After the Astra rerun corrected the block again (section 5), the script, again this
ticket's block plus the same lines, ran a second time at the same commit on a clean tree,
the ticket's and the index's edits set aside as `ai_tmp/gd/GD09/record-pending.patch`:
`ai_tmp/gd/GD09/ev-verification-at-work-2.txt` ends
`All checks passed and the worktree is unchanged.`, an empty `status=`, and
`exit status: 0`. That run is the record of Verification at the work commit.

It exited 0, so every `test` held: both new files present; `failure.gleam` with no
`import` line; `answers.gleam`'s imports exactly `pachinko_double/model` and
`pachinko_double/time`; each of the seven types defined with a body in its new module
and nowhere else under `src/`; the root with no import of `pachinko_double/domain` and no
argument typed `Publication`, plain or qualified; the 12 `import` lines of
`guide_test.gleam` all allowed; the changed paths under `src/` exactly the eight; no
`erl_crash.dump`. Each probe's positive control printed its expected count first. The two
`273 tests run: 273 passed, 0 skipped` lines inside `just check` are the Rust workspace's
tests, not the double's.

```text
$ sh -x ai_tmp/gd/GD09/verification.sh
+ set -eu
...
+ git rev-parse HEAD
72fe01a206936433299151562c10fa8850281649
...
gleam 1.19.1
...
OTP 29
...
326 tests run, 326 passed
coverage faults: 0
...
+ test 3 -eq 3
...
+ test 2 -eq 2
...
+ test 12 -eq 12
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

1. **Two defining modules import less than "Decisions fixed" writes.** Under
   `--warnings-as-errors`, `rules.gleam` does not import `type Cause` or
   `BrokenInvariant`, and `consuming.gleam` drops `type Mark` and its import of `time`,
   which only the moved views used. `ev-departure-imports.txt`, written at 03:12:48Z, is a
   later reproduction: with the imports as the ticket writes them, the build stops on
   `error: 4 warnings generated.`, exit status 1, and both files were restored after
   (`cmp`, by its own account). The first failing builds are not saved.
2. **The new modules' `////` headers are the implementer's prose.** The ticket fixes no
   wording for them. `failure.gleam`'s opens "How an outcome fails, as a value", and
   `answers.gleam`'s "What a reply carries".
3. **The qualifier command printed five lines at the base, not three**
   (`ev-qualifiers-after.txt`). The two the ticket did not count were the root header's
   lines 28 and 29, prose naming `rules.Failure` and `consuming.LeaseView`, which step 7
   rewrote.
4. **The formatter moved lines the ticket did not name.** `just gleam-double-format`
   sorted the new imports into place and rewrapped name lists, at steps 3 and 4 (12 and
   16 files of `format-diff.txt`); at step 5 and after step 7 it changed nothing. Every
   changed line in `format-diff.txt` is an `import` line, a line of an import's name list
   or its closing `}` (`rg`, for this record).
5. **`ev-probes-precommit.txt` alone altered the changed-files probe.** With
   `failure.gleam` and `answers.gleam` untracked, `git diff --name-only` cannot list them,
   so that run added `git ls-files --others --exclude-standard -- gleam-double/src`. The
   run at the work commit used the ticket's form.
6. **Verification was corrected at the checkpoint** (C1, B2, F1, C2, B3; `verdicts.md`).
   The `Publication` probe matches `: ([a-z_]+\.)?Publication\b`, and its control takes
   both spellings. The "defined once" probe's `homes` pattern counts a public, opaque or
   private copy and still skips an alias, with a new control. The witness regex's control
   gains an aliased engine line and an aliased vocabulary line and expects 3.
7. **The reviewers were Claude, not Codex, for want of quota** (section 5). GD07's delta
   allows that fallback only when `codex` is not on `PATH`; the user ruled on it here.
8. **Verification was corrected again after the Astra pass** (GD09-astra C1, B1, B2;
   `ai_tmp/review/GD09-astra/verdicts.md`). The import probe and the changed-files probe
   take `rg`'s and `git`'s output in an assignment of their own, so a producer that fails
   after printing the expected lines stops the script; the sentence above the block that
   said `set -eu` stops at every failure is reworded. The import probe reads the module up
   to the first space or `.` (`^import [^ .]+`), with two controls, one holding a digit.
   The "defined once" probe's parameter class takes digits (`[a-z0-9_, ]*`), and its
   control gains `pub type TickReport(a1) {` and expects 4.

### 8. What was not run, and what was not recorded

Not run:

- A push or a pull request. Both need authorisation (step 9); nothing left the worktree.
- `just initialize`, `just gleam-double-deps` or any other network recipe. The review's
  model calls are the only network use, which the checkpoint authorises.
- A second review round. None was owed.
- A review of the triage's fix diff.
- The corrected probes in full before the work commit. The smoke run left out three
  checks (section 5); section 6's run is the first full one.
- A rerun of `ai_tmp/gd/GD09/probes-precommit.sh` with the corrected probes, which
  `verdicts.md` hands back. It still holds the old probes; it is scratch, not the record,
  and the run it recorded stands as recorded.

Not recorded: the exact `codex exec` command lines, beyond the form in
`ai_tmp/review/reviewer-subagent.md`; the first failing builds behind departure 1; and
the models of the subagents, beyond the lead's account.

## Open points

- **`publish`'s five arguments have an alternative.** `Publication` could move to a
  public module instead, and `publish` keep taking one. The maintainer chose the five
  values on 2026-11-23, so that `Publication` stays with its rule module, where it is the
  input of the `Publish` table.
- **Closed: labelled or plain.** The maintainer was first shown labelled arguments, and
  this ticket writes plain ones, to match `pull`, `ack` and the rest. Three of the five
  are `String`s, so labels would guard against a swapped pair. On 2026-11-23 the
  maintainer ruled for plain arguments, over labels for `publish` alone and over
  keeping `Publication` as the argument.
- **How a caller names the failure type.** A caller writes `failure.Failure(Refusal)`,
  or uses the root's `Reply`. The root could hold an alias for it, which would be an
  eighteenth item. `domain.Failure` is that alias today, and GD10 makes `domain`
  internal.
- **`TickReport.fired` and `Outcome.fired`** stay the double's own channels for tests
  and the coverage check, as GD08's header labels them. Moving `TickReport` does not
  change that.
- **The witness is one module.** `guide_test.gleam` reaches the root through `harness`,
  which is test support and imports the engine: it drives a `Step` by hand and builds a
  `Publication`. The pin does not say that every test could be written against the
  public modules alone, and the tests of the engine could not.
