---
id: T36
title: "Diagram drift: a check that the committed SVGs match the script that draws them, and where that script lives"
status: done
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

### Base, branch and the step 2 choice

- **Base:** `wave-2-tickets`, at `8f27860`, which holds T35 (pull request 49) and T33. `origin/main`, at `611a86e`, did not yet have T35. Between `be7caf3e03bfd8a4dd360470af150a697663e094` and `8f27860` the script, the `Justfile`, `commands.md`, `quality-gates.md`, `pyproject.toml` and `ci.yml` were unchanged, so Context's facts held at the base, line numbers included.
- **Branch:** the work is on `tickets/t36-diagram-drift-check.md`, the worktree's branch, not the ticket's `branch: ticket/t36-diagram-drift-check`.
- **The choice, given by the maintainer on 2026-11-18:**
  - Script home: "(b) docs/decisions/0020/ (Recommended)".
  - Check: "(1) --check in check-docs (Recommended)".
  - Base: "Yes, wave-2-tickets (Recommended)".

### Baseline, at `8f27860`

`just initialize` exited 0. `just check` on the untouched worktree ended:

```text
All checks passed and the worktree is unchanged.
```

### The move (step 3), at `8f27860` plus the uncommitted move

After `git mv scripts/diagrams.py docs/decisions/0020/diagrams.py`, `OUT = Path(__file__).resolve().parent`, `ROOT = OUT.parents[2]` and the recipe edit, `just diagrams` printed the four paths. Then:

```text
$ git status --short
 M Justfile
RM scripts/diagrams.py -> docs/decisions/0020/diagrams.py
$ git diff --stat -M HEAD
 Justfile                                     | 2 +-
 {scripts => docs/decisions/0020}/diagrams.py | 4 ++--
 2 files changed, 3 insertions(+), 3 deletions(-)
```

No SVG changed and no new folder appeared.

### Red first (step 4), at `8f27860` plus the move

The break changed `held_record`'s "pull" label to "fetch" and did not rerun the script. `just check` still ended `All checks passed and the worktree is unchanged.`, with the move and the break uncommitted.

### What landed

- **`docs/decisions/0020/diagrams.py`:**
  - `render(name)` is the one rendering. Draw mode writes it with `write_bytes`.
  - `--check` reads with `read_bytes`, `is_file` and `iterdir`. It names an SVG that differs, is missing, or is drawn by no diagram, in any letter case, and an empty `DIAGRAMS`. It exits 1 if it named anything; otherwise it prints `compared N SVGs, all match` and exits 0.
  - Any other argument prints usage and exits 2.
- **`Justfile`:**
  - `_diagrams draw|check` is the one discovery: the glob `docs/decisions/*/diagrams.py`, with an `-f` test.
  - In both modes it fails on:
    - finding no script;
    - a script that is a symlink or sits in a symlinked folder;
    - a `diagrams.py` the glob does not reach;
    - a name under `docs/decisions/` that holds a newline.
  - `check` also fails:
    - a script whose `--check` exits 0 without its count line;
    - a symlink that could hide a diagram;
    - an SVG in a folder with no discovered script.
  - The file lists are held in variables, so the recipe writes no file of its own.
  - `diagrams` runs `_diagrams draw`, and `check-docs` gains `just _diagrams check`.
- **`docs/reference/commands.md` and `docs/reference/quality-gates.md`:**
  - The `diagrams` and `check-docs` rows, and gate 15's row, say what the check proves, where it runs and what it trusts.
  - The `lint` paragraph says no hook runs the diagram check.

Commits: `21bd3e6` (the move and the check), `22b463f` (round-1 fixes), `754bffd` (round-2 fixes).

### The probes (step 5), at `754bffd`

The probes are one script, run alone through `just _diagrams check` (or `just diagrams` where stated). Each exit status was captured directly. The tree was required clean before and after each probe, and the script stops with exit 3 on any unexpected result. The output below is condensed from that run: every diagnostic line is verbatim, followed by the exit status and the first `git status --short` line, which shows the break unrewritten.

```text
commit 754bffd
1. Stale label ("pull" -> "fetch" in held_record, not rerun)
docs/decisions/0020/held-record.svg: differs from what docs/decisions/0020/diagrams.py draws; run `just diagrams`
exit 1 |  M docs/decisions/0020/diagrams.py
2. Hand-edited colour (#ffffff -> #fffffe in defect.svg)
docs/decisions/0020/defect.svg: differs from what docs/decisions/0020/diagrams.py draws; run `just diagrams`
exit 1 |  M docs/decisions/0020/defect.svg
3. Extra file (positions.svg copied to extra.svg)
docs/decisions/0020/extra.svg: drawn by no diagram in docs/decisions/0020/diagrams.py; remove it, or add a diagram that draws it and run `just diagrams`
exit 1 | ?? docs/decisions/0020/extra.svg
4. Missing file (lease-end.svg deleted)
docs/decisions/0020/lease-end.svg: missing; run `just diagrams`
exit 1 |  D docs/decisions/0020/lease-end.svg
5. Orphan (docs/decisions/9999/ holds defect.svg, no script)
docs/decisions/9999/defect.svg: no docs/decisions/*/diagrams.py draws it; add one in its folder and run `just diagrams`, or remove the SVG
exit 1 | ?? docs/decisions/9999/defect.svg
6. Nested script (docs/decisions/0020/sub/diagrams.py and its SVGs); check, then just diagrams
docs/decisions/0020/sub/diagrams.py: outside the discovery, which runs only docs/decisions/*/diagrams.py; move it to such a folder and run `just diagrams`
docs/decisions/0020/sub/positions.svg: no docs/decisions/*/diagrams.py draws it; add one in its folder and run `just diagrams`, or remove the SVG
(and the same for sub/defect.svg, sub/lease-end.svg, sub/held-record.svg)
exit 1 | ?? docs/decisions/0020/sub/defect.svg
just diagrams: the same "outside the discovery" line; exit 1
6b. Hidden folder (docs/decisions/.hidden/diagrams.py)
docs/decisions/.hidden/diagrams.py: outside the discovery, which runs only docs/decisions/*/diagrams.py; move it to such a folder and run `just diagrams`
exit 1 | ?? docs/decisions/.hidden/diagrams.py
7. Second decision (docs/decisions/9998/diagrams.py, DIAGRAMS trimmed to positions.svg)
just diagrams wrote docs/decisions/9998/positions.svg, then:
docs/decisions/0020/diagrams.py: compared 4 SVGs, all match
docs/decisions/9998/diagrams.py: compared 1 SVGs, all match
exit 0
after a hand edit of 9998/positions.svg:
docs/decisions/9998/positions.svg: differs from what docs/decisions/9998/diagrams.py draws; run `just diagrams`
exit 1 | ?? docs/decisions/9998/diagrams.py
8. No script (0020/diagrams.py moved out of the worktree)
found no docs/decisions/*/diagrams.py, so nothing draws or checks the diagrams; restore or add the script beside its SVGs and run `just diagrams`
exit 1 |  D docs/decisions/0020/diagrams.py
9. Nothing drawn (DIAGRAMS = {})
docs/decisions/0020/diagrams.py: draws nothing; add a diagram to DIAGRAMS and run `just diagrams`
docs/decisions/0020/defect.svg: drawn by no diagram in docs/decisions/0020/diagrams.py; remove it, or add a diagram that draws it and run `just diagrams`
(and the same for held-record.svg, lease-end.svg, positions.svg)
exit 1 |  M docs/decisions/0020/diagrams.py
10. Clean tree, read-only (chmod a-w on the four SVGs and docs/decisions/0020/)
modes read-only: 555 docs/decisions/0020, 444 on each SVG
negative controls: docs/decisions/0020/defect.svg refuses append, docs/decisions/0020/ refuses a new file
docs/decisions/0020/diagrams.py: compared 4 SVGs, all match
exit 0
modes after restore: 755 and 644, as before; git status before and after: identical (0 lines)
```

The probes added for the review's fixes (§14, below) held in the same run:

| Probe | Break | Result |
| --- | --- | --- |
| 11 | `docs/decisions/9999/orphan.SVG`, no script | exit 1, `orphan.SVG: no docs/decisions/*/diagrams.py draws it` |
| 12 | `docs/decisions/0020/extra.SVG` | exit 1, `extra.SVG: drawn by no diagram` |
| 13 | `9998/diagrams.py` a symlink to `../0020/diagrams.py`, beside a stale `positions.svg` | exit 1 in `check` and in `just diagrams`, `a symlink or in a symlinked folder` |
| 14 | `9999/orphan.svg` a symlink to `../0020/defect.svg` | exit 1, `a symlink to an SVG, a diagrams.py or a folder` |
| 15 | `docs/decisions/9997` a symlink to `0020` | exit 1, the script and the folder both named |
| 15b | `0020/evidence.txt` a symlink to `../README.md` | exit 0, `compared 4 SVGs, all match` |
| 15c | a folder named `99`, newline, `98`, holding the script | exit 1 in both modes, `a name under docs/decisions/ holds a newline` |
| 16 | `9998/diagrams.py` exits 0 silently, beside a stale SVG | exit 1, `--check exited 0 without saying how many SVGs it compared` |
| 16b | a later script fails with its problem on stdout | exit 1, the line still printed |
| 17 | `docs/decisions/9999/` unreadable | exit 1, `find: docs/decisions/9999: Permission denied` |
| 18 | no-write evidence | the recipe at `21bd3e6` matches `mktemp` (positive control); the recipe now matches none of `mktemp`, `>` into a file, `tee` or `touch`, and grep must exit exactly 1 |

The script ended `ALL PROBES HELD`, exit 0. The check mode, as committed, reads with `read_bytes`, `is_file` and `iterdir` and opens nothing for writing:

```python
def check():
    """Compare every SVG in this folder with what the script draws; open nothing for writing."""
    script = Path(__file__).resolve().relative_to(ROOT)
    rerun = "run `just diagrams`"
    problems = []
    if not DIAGRAMS:
        problems.append(f"{script}: draws nothing; add a diagram to DIAGRAMS and {rerun}")
    for name in DIAGRAMS:
        path = OUT / name
        if not path.is_file():
            problems.append(f"{path.relative_to(ROOT)}: missing; {rerun}")
        elif path.read_bytes() != render(name):
            problems.append(f"{path.relative_to(ROOT)}: differs from what {script} draws; {rerun}")
    for path in sorted(OUT.iterdir()):
        if path.suffix.lower() == ".svg" and path.name not in DIAGRAMS:
            problems.append(
                f"{path.relative_to(ROOT)}: drawn by no diagram in {script}; "
                f"remove it, or add a diagram that draws it and {rerun}"
            )
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"{script}: compared {len(DIAGRAMS)} SVGs, all match")
    return 0
```

The evidence for "writes nothing" has two limits:

- It is static for the recipe. A live test with `TMPDIR` does not work on macOS, whose `mktemp` ignores it.
- `just` itself writes each shebang recipe's body to its own temporary directory, so the claim is that the check's own code writes nothing.

### Adversarial review (step 7, §14): T36-1

Codex `codex-cli 0.169.1`, `-m gpt-6-astra -s read-only`, on 2026-11-18, run with the corrected command form (`< /dev/null`, `-o` at `last-<lens>.md`). Round 1 covered `8f27860..21bd3e6`. A `/code-review` pass over the same diff ran beside it, and its findings were triaged in the same way. Triage and fixes were done by one fresh-context Claude subagent, the only writer, applying fixes one at a time. The executing agent's briefs set some dispositions in advance:

- R5 deferred to step 7, R6 recorded here, and C2-1 an Open point.
- R1 and R8 handed back under the scope guard.
- The shape of the C2-2 fix proposed: narrow the rejection to links that could hide a diagram.

The subagent checked each of the others itself. Its ShellCheck and `/bin/dash` results (R1, and after each round) rest on `verdicts.md` and `verdicts-2.md`. The raw files are under `ai_tmp/review/T36-1/` and are not committed.

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| C1 / B4: the recipe wrote its SVG list to a `mktemp` file | medium | valid | The lists are held in variables; a failing `find` still fails (probe 17) |
| C2: the no-script failure did not say to run `just diagrams` | low | valid | It does |
| C3: the handbook said all of gate 15 runs inside `lint` | low | valid | The paragraph excepts the diagram check |
| B1 / R3: `*.SVG` escaped both inventories | medium | valid | The match ignores case in `find` and in `check()` (probes 11, 12) |
| B2 / R3: a symlinked script, SVG or folder bypassed comparison | high | valid | Symlinked scripts are refused, and symlinks that could hide a diagram are named (probes 13 to 15) |
| B3: a later script could exit 0 without comparing | medium | valid, partly applied | A script must print its count line (probe 16); the rest is an Open point |
| B5: the probe harness did not fail closed | medium | valid | The harness checks every step, asserts its negative controls and restores the tree on exit |
| R1: gate shell in the `Justfile` is not shellchecked | low | valid, handed back | Moving it to `scripts/` is outside Files touched. ShellCheck 0.11.0 on the body reports only the intended SC2016 |
| R2: a nested `diagrams.py` was never run and was misreported | medium | valid | It is named as outside the discovery (probes 6, 6b) |
| R4: "run `just diagrams`" could not fix an extra SVG or an empty `DIAGRAMS` | low | valid | Each failure's advice clears it |
| R5: status and hand-back not written | none | deferred | Written here |
| R6: branch name differs from `branch:` | low | recorded | Above, under Base |
| R7: `relative_to(ROOT)` could raise | low | invalid | `ROOT` is always an ancestor of `OUT`; the cases it names are refused (probes 6, 6b, 13, 15) |
| R8: `docs/project/repository-map.md` does not name the `diagrams.py` sources | low | valid, fixed after the maintainer widened scope | The `docs/decisions/` row names each record's folder, its SVGs and the `diagrams.py` that draws them |

B2 was a valid high finding, so round 2 ran over `21bd3e6..22b463f`:

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| C2-1: B3's remainder, the recipe trusts each script's count | medium | Open point | None |
| C2-2: every symlink under `docs/decisions/` failed, not only those that could hide a diagram | low | valid | Narrowed to links named like an SVG or a `diagrams.py`, and links to a folder (probe 15b) |
| B2-1: a name holding a newline was split into fragments | low | valid | Refused first, by name, in both modes (probe 15c) |
| B2-2: the harness's negative greps treated an error as no match | medium | valid | They require exit 1 exactly |

There were no valid high findings in round 2, and two rounds are the limit. The round-2 fixes, at `754bffd` (the narrowed symlink filter and the newline refusal), were therefore not reviewed again. The probes (15b, 15c, and 18's exact-status greps) and the Verification below are their only evidence.

### Verification, at `754bffd`

`git fetch origin` found `origin/wave-2-tickets` still at `8f27860` and `origin/main` still at `611a86e`. Both are already in this branch, so neither merge had anything to bring in. The Verification block ran as one `sh -eu` script and exited 0. Its output includes:

```text
commit 754bffd
just _diagrams draw
docs/decisions/0020/defect.svg
docs/decisions/0020/held-record.svg
docs/decisions/0020/lease-end.svg
docs/decisions/0020/positions.svg
prek run --all-files markdownlint-cli2 typos lychee
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 48 pages and 49 canonical topics.
just _diagrams check
docs/decisions/0020/diagrams.py: compared 4 SVGs, all match
...
All checks passed and the worktree is unchanged.
bg-project-check clean "$1"
The Git worktree is clean.
```

### Maintainer's follow-up, 2026-11-18

After the hand-back above, the maintainer answered four questions:

- Strictness: "Keep all of it (Recommended)". The `_diagrams` recipe is unchanged.
- Leftovers: "Fix the ci.yml comment here (Recommended)" and "Add R8 to repository-map here (Recommended)". R1 and B3 stay open.
- Branch name: "Keep as is." The branch is pushed as `tickets/t36-diagram-drift-check.md`.
- Push and PR: "Push + PR to wave-2-tickets (Recommended)", ready for review.

The answer widens the scope beyond Files touched to `.github/workflows/ci.yml`, whose comment above `check-specs` now says that no hook runs the diagram check inside `check-docs`, and `docs/project/repository-map.md`, whose `docs/decisions/` row now names the diagram sources (R8).

## Open points

- **B3 / C2-1, the maintainer's decision.** Each `diagrams.py` compares its own folder, and the recipe trusts a script that exits 0 and prints `compared N SVGs, all match` with N at least 1. A later script whose comparison loop broke while it kept printing its count would pass. A shared checker, which every decision's script imports and the recipe drives, would close this. It conflicts with choice (b) ("the SVG helpers move to a shared module then, not before") and needs a file outside Files touched. Gate 15's row states the trust limit.
- **R1:** the `_diagrams` shell is not covered by the shellcheck hook, which reads `scripts/` only. Moving it there is outside Files touched.
- **Not run on Linux.** The probes ran on macOS's `/bin/sh` and were smoke-run under `/bin/dash`; CI's `documents` job, on Ubuntu, is the first Linux run.
- **Push and pull request:** the maintainer authorised both on 2026-11-18, the push under the branch's local name and the pull request into `wave-2-tickets`, ready for review.
- **Branch name:** the branch keeps its local name, `tickets/t36-diagram-drift-check.md`, not the ticket's `branch:` (R6).
