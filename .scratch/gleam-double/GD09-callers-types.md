---
id: GD09
title: "Gleam double: the caller's types in public modules"
status: todo
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

- **The five values are plain arguments.** What the maintainer was shown on 2026-11-23
  called them labelled, "as every other trigger function already does". The other trigger functions
  take plain positional arguments, so `publish` matches them:

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

Run as one `sh` script from the repository root; `set -eu` stops it at the first command
that fails. `base` is the commit step 1 records; set it before the script runs. Each
probe that passes by printing nothing follows its positive control, and no probe reads
`rg`'s status through a pipe.

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
imports=$(rg -o '^import [a-z_/]+' "$src/answers.gleam" | LC_ALL=C sort | tr '\n' ' ')
test "$imports" = "import pachinko_double/model import pachinko_double/time "

# each type is defined once, with a body, in its new module; an alias has no `{`
for pair in failure:Failure failure:Cause answers:Answer answers:BatchRecord \
  answers:TickReport answers:LeaseView answers:HeldView; do
  module=${pair%%:*}
  name=${pair##*:}
  pattern="^pub type $name(\([a-z_, ]*\))? \{\$"
  rg -q "$pattern" "$src/$module.gleam"
  homes=$(rg -l "$pattern" gleam-double/src)
  test "$homes" = "$src/$module.gleam"
done

pattern='^import pachinko_double/domain\b'
probe=$(printf 'import pachinko_double/domain.{type Failure}\n' | rg -c "$pattern")
test "$probe" -eq 1
if rg -n "$pattern" "$root"; then exit 1; else test "$?" -eq 1; fi
probe=$(printf '  publication: Publication,\n' | rg -c ': Publication\b')
test "$probe" -eq 1
if rg -n ': Publication\b' "$root"; then exit 1; else test "$?" -eq 1; fi

allowed='^import (gleam/[a-z_/]+|simplifile|harness|runner|pachinko_double|pachinko_double/(config|time|model|refusals|failure|answers|inspection|invariants))(\.\{| as |$)'
probe=$(printf 'import pachinko_double/core.{TickReport}\nimport harness.{type Double}\nimport pachinko_double/domain\nimport pachinko_double.{Outcome}\n' | rg -c "$allowed")
test "$probe" -eq 2
all=$(rg -c '^import ' gleam-double/test/guide_test.gleam)
ok=$(rg -c "$allowed" gleam-double/test/guide_test.gleam)
test "$all" -eq "$ok"

changed=$(git diff --name-only "$base" -- gleam-double/src | LC_ALL=C sort | tr '\n' ' ')
test "$changed" = "$root $src/answers.gleam $src/consuming.gleam $src/core.gleam $src/domain.gleam $src/failure.gleam $src/pull.gleam $src/rules.gleam "

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The builder runs each control first, and corrects a pattern, saying so under Departures,
if one does not behave as written.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the red steps; the checkpoint's record; Verification at the work commit; departures; and
what was not run.

## Open points

- **`publish`'s five arguments have an alternative.** `Publication` could move to a
  public module instead, and `publish` keep taking one. The maintainer chose the five
  values on 2026-11-23, so that `Publication` stays with its rule module, where it is the
  input of the `Publish` table.
- **Labelled or plain.** The maintainer was shown labelled arguments on 2026-11-23, and
  this ticket writes plain ones, to match `pull`, `ack` and the rest. Three of the five
  are `String`s, so labels would guard against a swapped pair. Labelling one function and not the others is a
  choice for the maintainer.
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
