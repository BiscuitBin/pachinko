---
id: T36
title: "Diagram drift: a check that the committed SVGs match the script that draws them, and where that script lives"
status: todo
depends_on: [T35]
parallel_with: [T33]
branch: ticket/t36-diagram-drift-check
estimated_size: S
---

# T36: Diagram drift: a check that the committed SVGs match the script that draws them, and where that script lives

## Context

T35 redrew decision 0020 at the maintainer's request (`tickets/T35-decide-q1-q2.md`,
hand-back, "Decision 0020 redrawn"). Four of its diagrams are SVG files that a script
draws, and nothing checks that the files and the script agree. This ticket picks a check,
lands it, and settles where the script lives. Line numbers are as of `be7caf3e03bfd8a4dd360470af150a697663e094`, the head
of pull request 49, T35's.

What exists at `be7caf3e03bfd8a4dd360470af150a697663e094`:

- `scripts/diagrams.py` draws `defect.svg`, `held-record.svg`, `lease-end.svg` and
  `positions.svg` into `docs/decisions/0020/`. It is standard-library Python, it owns
  each diagram's layout and words, and it writes every file on every run. It takes no
  arguments. Its output is deterministic: no timestamp, a fixed file order and fixed
  number formatting. `pyproject.toml` pins the interpreter to `python = "3.14.*"`.
- `just diagrams` (`Justfile` lines 420 to 424) runs `python scripts/diagrams.py`. The
  `Justfile`'s `PATH` puts the pixi environment's `python` first. The recipe writes, so
  it sits outside `just check`.
- `docs/reference/commands.md` line 100 is the recipe's row, and it says "nothing
  checks that the files match the script".
- `docs/decisions/0020-held-set-until-lease-ends.md` embeds the four files as
  `0020/<name>.svg`, and each image's alt text restates its diagram.

The gap: someone edits the script and does not rerun it, or edits an SVG by hand, and
`just check` stays green while the page and its source disagree. The reverse also goes
unseen: a stale SVG left behind when the script stops drawing it.

Facts that bear on the choice, checked at `be7caf3e03bfd8a4dd360470af150a697663e094`:

- `bg-validate-docs` registers and checks `docs/**/*.md` pages only (`validate_docs.py`
  line 279, in the installed `biscuit_games_tooling` package under
  `.pixi/envs/default/lib/python3.14/site-packages/`), and it checks that each link a
  page makes, the four SVG images included, names a file that exists, with exact case.
  No check reads an SVG's content against the script, and a `.py` or `.svg` under
  `docs/` needs no manifest entry. editorconfig-checker and typos read both through
  `just lint`.
- `just check` runs the `recipes` list in `pyproject.toml` (lines 45 onwards), twenty
  gates, then `check-clean`. A new gate would be gate 21, gates 1 to 20 keep their
  numbers, and `check-clean`, row 21 of the table in `docs/reference/quality-gates.md`
  (line 39), would become row 22. `docs/reference/commands.md` line 112 states the count.
- CI's `documents` job (`.github/workflows/ci.yml` lines 106 to 133) runs `just lint`,
  `just check-docs`, `just check-agents` and the two spec recipes, then
  `just check-clean`.
- Checks are read-only, and `just check` fails a recipe that changes the worktree
  (`AGENTS.md`; `tickets/CONVENTIONS.md` §11, "Green"). It compares each tracked file's
  mode and content hash before and after a recipe (`run_project_check.py` lines 49 to
  67), so a recipe that rewrites a file with the same bytes passes it. "Writes nothing"
  therefore needs its own evidence.
- `scripts/diagrams.py` derives its output folder from its own path (`ROOT` and `OUT`,
  near the top), writes with `Path.write_text`, and prints each path it writes.

## Goal

`just check` and CI fail when a committed SVG under `docs/decisions/` differs from what
its script draws, when a file a script draws is missing, when a folder holds an SVG its
script no longer draws, when an SVG under `docs/decisions/` has no script that draws
into its folder, and when no script is found at all. Each failure names the file and says
to run `just diagrams`. The check writes nothing. Generation and the check find their
scripts the same way, so a later decision's diagrams are checked without a new recipe.
The script lives where the maintainer chooses at step 2, and the four SVGs stay
byte-identical through any move. The handbook says what the check proves and where it
runs.

This ticket owns no specification rule and changes no file under `crates/`, `loadgen/`
or `docs/specs/`. It changes no diagram's content.

## Non-goals

- No change to what a diagram shows. Its words are T35's, checked against the
  specification there.
- No new dependency and no new tool: standard-library Python and the pinned interpreter
  only (decision 0007; `AGENTS.md`, invariant 9).
- No Mermaid conversion. The maintainer picked SVG for these four in T35.
- No generation at build or deploy time. GitHub renders committed files only, so the
  SVGs stay committed.
- No edit to decision 0020's page, whose image paths no option changes, nor to T35's
  hand-back, which records the script's path as it was.

## The choice for step 2

Two decisions are the maintainer's. The agent presents both, with the recommendations,
and stops for the answer.

**Where the script lives.**

| Option | For | Against |
| --- | --- | --- |
| (a) Stay at `scripts/diagrams.py` | The repository's tooling lives in `scripts/`; nothing moves | The drawings' source sits away from the SVGs it writes. A later decision's diagrams go in this file or in a second script under `scripts/`, either way named by hand |
| (b) Move to `docs/decisions/0020/diagrams.py` (recommended) | The source sits in the folder it writes, beside the SVGs and next to the decision page. A decision record is history, so the folder can be left alone as a unit. Scripts are found by the path `docs/decisions/*/diagrams.py`, with no list to keep | Python under `docs/`, which nothing else there is. A second decision with diagrams would copy the SVG helpers, so they move to a shared module then, not before |
| (c) Helpers in `scripts/`, drawings beside each record | No copied helpers | Two files to read for one diagram, and an import across the tree, for a second user that does not exist yet |

**The check.**

| Option | Where it runs | For | Against |
| --- | --- | --- | --- |
| (1) A `--check` mode, run by `check-docs` (recommended) | `just check` gate 15 and CI's `documents` job, both already | No new gate and no count to change; diagrams are documentation | `check-docs`'s rows in the two pages grow a clause |
| (2) A new recipe `diagrams-check` as gate 21 | `just check`; a new step in CI's `documents` job | The gate table names it on its own | `pyproject.toml`, the count and the `check-clean` row in two pages, and CI all change |
| (3) A local prek hook on the script and SVG paths | At commit, in `just lint` and CI | Fails before the commit lands | A third place names the paths. The hook needs `pixi run --frozen`, as the other script hooks do. In `just check` it adds nothing that (1) does not |
| (4) CI only | CI | Smallest change | Does not meet the Goal: a local `just check` stays green on a stale SVG. Choosing it hands the ticket back to the maintainer to rewrite the Goal and acceptance criteria first |

Under every option, the method is the same:

- **One discovery.** `just diagrams` and the check find their scripts through the same
  code: a glob under (b) or (c), the one script under (a). Finding no script is a
  failure, never an empty success; a shell glob that matches nothing stays literal, so
  the recipe tests for that.
- **One rendering.** Each diagram is drawn to bytes by one function, which the writer
  writes with `write_bytes` and the check compares with the file read in binary mode,
  so no newline or encoding conversion sits between them.
- **The comparison.** For each script, the check names every SVG in its folder that
  differs, is missing or is drawn by no diagram, and every SVG under `docs/decisions/`
  in a folder with no script. It opens nothing for writing, exits 1 if anything is named,
  and exits 0 only after comparing at least one file, printing how many.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `scripts/diagrams.py`, or `docs/decisions/0020/diagrams.py` after a move | shared, own lines | The `--check` mode, the shared rendering and discovery; the move only under (b) or (c), by `git mv`, with `ROOT` and `OUT` derived for the new place |
| `scripts/` (a helpers module) | new | Under (c) only |
| `docs/decisions/0020/*.svg` | shared, own lines | Unchanged in content; listed because the check reads them and the scratch breaks edit them |
| `Justfile` | shared, own lines | `diagrams` (lines 420 to 424) over the chosen discovery; under (1) a line in `check-docs`; under (2) a new `diagrams-check` |
| `pyproject.toml` | shared, own lines | Under (2) only, `"diagrams-check"` in `recipes` |
| `.pre-commit-config.yaml` | shared, own lines | Under (3) only, the local hook |
| `.github/workflows/ci.yml` | shared, own lines | Under (2), a step in the `documents` job; under (4), if the maintainer rewrites the Goal, the CI step |
| `docs/reference/commands.md` | shared, own lines | The `diagrams` row (line 100) loses "nothing checks"; the row of the recipe that runs the check; under (2), the count at line 112 |
| `docs/reference/quality-gates.md` | shared, own lines | What the check proves, in the row of the gate that runs it; under (2), a row 21 and `check-clean` as row 22 |
| `tickets/README.md` | shared, own lines | T36's Status cell |
| `tickets/T36-diagram-drift-check.md` | ticket | `status:` and hand-back notes |

Every scratch break below, a second decision folder included, is reverted before the
next step and never committed.

## Steps

1. Create the worktree from `main` once T35 has reached it; T35 merges first into
   `wave-2-tickets`, through pull request 49 (`tickets/README.md`, "How to pick up a
   ticket"). If the maintainer wants T36 sooner, base it on `wave-2-tickets` after T35
   has merged there, and record that base in the hand-back. Run `just initialize` there
   (network, authorised by this step). Run `just check` on the untouched worktree and
   quote its last line, headed by its commit, as the baseline.
2. **The plan and the choice.** Use the `plan-change` skill. Read the files in Context
   and confirm the facts at the new base; record any that moved. Present "The choice for
   step 2" to the maintainer with the recommendations, (b) and (1), and **stop** for the
   answer. Record it, quoted and dated, in the hand-back.
3. **The move,** under (b) or (c) only, before any probe, because the probes run the
   chosen discovery. Move the script with `git mv`, so history follows it, derive
   `ROOT` and `OUT` for its new place, and point the `diagrams` recipe at the new path;
   step 5 replaces that path with the discovery. Run `just diagrams`: `git status
   --short` shows the move and the recipe edit and no change to an SVG, no new folder
   appears, and `git diff --stat -M HEAD` shows the script as a rename. Point the
   script's docstring, the recipe comment and the pages at the new path.
4. **Red first.** On a scratch break, change one word in a label the script draws to
   another correctly spelled word, so that typos cannot fail first, without rerunning the
   script. `just check` still ends green, which is the gap. Quote its last line. Keep the
   break.
5. **The check.** Use the `fix-quality` skill. Add the shared discovery, rendering and
   `--check` mode, then wire the check in as step 2 chose. Each probe below runs the
   diagram check on its own, by the recipe or the script's `--check`, not only the
   aggregate, because a missing embedded SVG also fails lychee and the documentation
   contract, and that failure would not prove the new check fired. Each failing probe
   quotes the check's output and exit status, and then shows the break still in place,
   unrewritten, by `git diff --stat` or `git status --short`.
   - Under the break from step 4, the check exits 1 and names the SVG that holds the
     label. Revert the break.
   - A hand edit: change one colour value in one SVG. The check names that file.
     Revert it.
   - An extra file: copy one SVG to `extra.svg` in the same folder. The check names
     `extra.svg`. Remove it.
   - A missing file: delete one SVG. The check names it as missing. Restore it.
   - An orphan: create a scratch folder `docs/decisions/9999/` holding a copy of one SVG
     and no script. The check names the orphan. Remove the folder.
   - A second decision, under (b) or (c): create a scratch folder with a copy of the
     script that draws one diagram, run `just diagrams`, then hand-edit that SVG. The
     check names it, so a later decision is covered without a recipe edit. Remove the
     folder.
   - No script: with every script found by the discovery moved out of reach on a scratch
     break, the check fails and says it found none. Restore.
   - Nothing drawn: empty one script's list of diagrams on a scratch break. The check
     fails and says it compared nothing. Restore.
   - The clean tree: make the SVGs and their folder read-only (`chmod a-w`), run the
     check, and quote exit 0 with "compared 4" (or the count the discovery finds). Then
     restore the modes, and `git status --short` prints what it printed before the
     probe. A check that writes
     fails here. Also quote the check mode's lines, which open no file for writing.
6. **The handbook.** Use the `review-docs` skill. Edit the rows in Files touched. Run
   `just check-docs`.
7. **Adversarial review** (`tickets/CONVENTIONS.md` §14), conformance and breakage, a
   first round, and a second over the fix diff if any high-severity finding is valid.
   The breakage lens asks whether the check can pass while an SVG is stale: whether it
   compares at all, whether discovery can find nothing and pass, whether a script that
   draws nothing passes, whether `--check` can write, and whether a later decision's
   folder is found.
8. Commit. Run `git fetch origin` (network, authorised by this step), merge the base
   branch and `origin/main`, rerun `just check`, and rerun and requote Verification,
   each headed by its commit (§11, "Green" and "Quoted evidence names its commit").
   Set `status: done` here and in `tickets/README.md`, and write the hand-back.
   **Stop** before pushing or opening a pull request; both need authorisation.

## Acceptance criteria

- Each probe in step 5 is quoted with the check's output and exit status: exit 1 naming
  the file for the stale, hand-edited, extra, missing and orphan SVGs and the second
  decision's stale SVG; exit 1 for no script and for nothing drawn; exit 0 with the
  count on the clean, read-only tree. Each failing probe shows its break unrewritten.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.
- `just diagrams` leaves the tree clean after the change.
- The four SVGs are byte-identical to `be7caf3e03bfd8a4dd360470af150a697663e094`'s.
- No row in `docs/reference/commands.md` still says nothing checks the diagrams. The
  page that owns the gates says what the check proves.
- The step 2 choice is quoted in the hand-back with its date.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails, so it ends
at `just check-clean` only if every line held.

```sh
set -eu
just diagrams
status=$(git status --short)                          # fails the script if git fails
test -z "$status"                                     # regeneration changed nothing
git diff --exit-code be7caf3e03bfd8a4dd360470af150a697663e094 -- 'docs/decisions/0020/*.svg'   # T35's bytes
just check-docs                                       # or the chosen recipe; prints the count
before=$(git show be7caf3e03bfd8a4dd360470af150a697663e094:docs/reference/commands.md)
printf '%s\n' "$before" | rg -q 'nothing checks'      # positive control
if rg -q 'nothing checks' docs/reference/commands.md; then exit 1; else test "$?" -eq 1; fi
just check
just check-clean
```

## Hand-back notes

## Open points
