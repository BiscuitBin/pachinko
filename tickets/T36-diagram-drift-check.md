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

- `bg-validate-docs` reads only `docs/**/*.md` (`validate_docs.py` line 279, in the
  installed `biscuit_games_tooling` package under
  `.pixi/envs/default/lib/python3.14/site-packages/`). lychee and markdownlint read
  Markdown only. A `.py` or `.svg` under `docs/` is therefore outside the documentation
  contract. editorconfig-checker and typos still read it, through `just lint`.
- `just check` runs the `recipes` list in `pyproject.toml` (lines 45 onwards), twenty
  gates, then `check-clean`. Adding a gate renumbers nothing but changes the count that
  `docs/reference/commands.md` line 112 and `docs/reference/quality-gates.md` state.
- CI's `documents` job (`.github/workflows/ci.yml` lines 106 to 133) runs `just lint`,
  `just check-docs`, `just check-agents` and the two spec recipes, then
  `just check-clean`.
- Checks are read-only, and `just check` fails a recipe that changes the worktree
  (`AGENTS.md`; `tickets/CONVENTIONS.md` §11, "Green").

## Goal

`just check` and CI fail when any committed SVG under `docs/decisions/` differs from what
its script draws, when one is missing, or when the directory holds an SVG the script no
longer draws. The failure names each such file and says to run `just diagrams`. The
check writes nothing. The script lives where the maintainer chooses at step 2, and the
four SVGs stay byte-identical through any move. The handbook says what the check proves
and where it runs.

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
- No edit to decision 0020's text unless a move changes the image paths. The default
  move below keeps them.

## The choice for step 2

Two decisions are the maintainer's. The agent presents both, with the recommendation,
and stops for the answer.

**Where the script lives.**

| Option | For | Against |
| --- | --- | --- |
| (a) Stay at `scripts/diagrams.py` | The repository's tooling lives in `scripts/` | The record's drawings live apart from the record; a later decision with diagrams grows one file for all of them |
| (b) Move to `docs/decisions/0020/diagrams.py` (recommended) | The decision, its drawings and their source sit in one folder and change together. A decision record is history, so a self-contained folder can be left alone as a unit. Image paths are unchanged | Python under `docs/`, which no other page has. A second decision with diagrams would copy the SVG helpers, so they move to a shared module then, not before |
| (c) Helpers in `scripts/`, drawings beside each record | No copied helpers | Two files to read for one diagram, and an import path across the tree, for a second user that does not exist yet |

Under (b), `just diagrams` runs every `docs/decisions/*/diagrams.py`, so a later record
needs no recipe edit. Each script writes into its own folder.

**The check.**

| Option | Where it runs | For | Against |
| --- | --- | --- | --- |
| (1) A `--check` mode, run by `check-docs` (recommended) | `just check` gate 15 and CI's `documents` job, both already | No new gate and no count to change; diagrams are documentation | `check-docs`'s row and gate description grow a clause |
| (2) A new recipe `diagrams-check` as gate 21 | `just check`; CI's `documents` job needs a new step | The gate table names it on its own | `pyproject.toml`, the gate count in two pages and CI all change |
| (3) A local prek hook on the script and SVG paths | At commit, in `just lint` and CI | Fails before the commit lands | A third place names the paths. The hook needs `pixi run --frozen`, as the other script hooks do. In `just check` it adds nothing that (1) does not |
| (4) CI only | CI | Smallest change | A local `just check` stays green on a stale SVG, which breaks the rule that the gate is the check |

Under every option, the comparison works the same way. Each script draws in memory and
compares the result with the file on disk, byte for byte. It names each stale, missing
or extra `.svg` in its folder, writes nothing and exits 1 if any differ. It exits 0
only after comparing at least one file, so a script that draws nothing fails.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `scripts/diagrams.py` → `docs/decisions/0020/diagrams.py` | shared, own lines | The `--check` mode; the move only under (b) or (c), by `git mv` |
| `docs/decisions/0020/*.svg` | shared, own lines | Unchanged in content; listed because the check reads them, and a scratch break edits one |
| `Justfile` | shared, own lines | `diagrams` (lines 420 to 424) over the chosen location; under (1) a line in `check-docs`; under (2) a new `diagrams-check` |
| `pyproject.toml` | shared, own lines | Under (2) only, `"diagrams-check"` in `recipes` |
| `.pre-commit-config.yaml` | shared, own lines | Under (3) only, the local hook |
| `.github/workflows/ci.yml` | shared, own lines | Under (2) only, a step in the `documents` job |
| `docs/reference/commands.md` | shared, own lines | The `diagrams` row (line 100) loses "nothing checks"; the row of whichever recipe runs the check; under (2), the count at line 112 |
| `docs/reference/quality-gates.md` | shared, own lines | What the check proves, in the row of the gate that runs it, or a new row under (2) |
| `tickets/README.md` | shared, own lines | T36's Status cell |
| `tickets/T36-diagram-drift-check.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree from pull request 49's base, `wave-2-tickets`, after T35 has
   merged there, or from `main` once that branch has landed (`tickets/README.md`, "How
   to pick up a ticket"), and run `just initialize` there (network, authorised by this
   step). Run `just check` on the untouched worktree and quote its last line as the
   baseline.
2. **The plan and the choice.** Use the `plan-change` skill. Read the files in Context
   and confirm the facts at the new base; record any that moved. Present "The choice for
   step 2" to the maintainer with the recommendations, (b) and (1), and **stop** for the
   answer. Record it, quoted and dated, in the hand-back.
3. **Red first.** On a scratch break, change one word in a label the script draws,
   without rerunning it. `just check` still ends green, which is the gap. Quote its last
   line. Keep the break.
4. **The check.** Use the `fix-quality` skill. Add the `--check` mode, then wire it in
   as step 2 chose.
   - Under the break from step 3, the chosen recipe exits non-zero and names the SVG
     that holds the label. Quote it. Revert the break.
   - A second break: change one colour value in one SVG by hand. The check fails and
     names that file. Revert it.
   - A third break: copy one SVG to `extra.svg` in the same folder. The check fails and
     names `extra.svg`. Remove it.
   - On the clean tree, the check exits 0 and says how many files it compared.
5. **The move,** under (b) or (c) only. Move the script with `git mv`, so history
   follows it. Then run `just diagrams`; `git status --short` shows the move and the
   recipe edit and no change to an SVG, and `git diff --stat -M HEAD` shows the script
   as a rename. Point the script's docstring, the recipe comment and the pages at the
   new path.
6. **The handbook.** Use the `review-docs` skill. Edit the rows in Files touched. Run
   `just check-docs`.
7. **Adversarial review** (`tickets/CONVENTIONS.md` §14), one round, conformance and
   breakage. The breakage lens asks whether the check can pass while an SVG is stale:
   whether it compares at all; whether a script that draws nothing passes; whether
   `--check` can write; whether a new folder's script is found.
8. Run `just check` and confirm that `git status --short` is empty after the commit.
   Set `status: done` here and in `tickets/README.md`, and write the hand-back.
   **Stop** before pushing or opening a pull request; both need authorisation.

## Acceptance criteria

- The chosen recipe fails on a stale, a hand-edited and an extra SVG, naming each, and
  passes on the clean tree having compared four files. Each run is quoted with its exit
  status.
- The check writes nothing: `just check` ends with `All checks passed and the worktree
  is unchanged.`
- `just diagrams` leaves the tree clean after the change.
- The four SVGs are byte-identical to `be7caf3e03bfd8a4dd360470af150a697663e094`'s:
  `git diff --exit-code be7caf3e03bfd8a4dd360470af150a697663e094 -- 'docs/decisions/0020/*.svg'` exits 0.
- No row in `docs/reference/commands.md` still says nothing checks the diagrams. The
  page that owns the gates says what the check proves.
- The step 2 choice is quoted in the hand-back with its date.

## Verification

```sh
just diagrams && git status --short          # prints nothing
git diff --exit-code be7caf3e03bfd8a4dd360470af150a697663e094 -- 'docs/decisions/0020/*.svg'; echo "exit $?"
just check-docs                              # or the chosen recipe; exit 0, four compared
rg -n 'nothing checks' docs/reference/commands.md; echo "exit $?"   # exit 1
just check
git status --short
```

## Hand-back notes

## Open points
