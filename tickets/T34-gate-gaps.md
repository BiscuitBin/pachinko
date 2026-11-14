---
id: T34
title: "Gate gaps: default-features doctests, untracked files, quoted evidence, loadgen in CI"
status: done
depends_on: [T31]
parallel_with: [T32, T33]
branch: ticket/t34-gate-gaps
estimated_size: S
---

# T34: Gate gaps: default-features doctests, untracked files, quoted evidence, loadgen in CI

## Context

T31 integrated the MVP wave and carried four gaps in the gate to its Open points, each
owned by the maintainer. This ticket closes all four. Each is a change to how a check is
run, not to what the crates do, so the `fix-quality` and `project-check` skills govern it
and the `Justfile` stays the only interface to the checks (`AGENTS.md`). Line numbers in
this ticket are as of `4daf9df`, the head of pull request 40, which T31 merges to `main`.

| Item | Source, in `tickets/T31-mvp-integration.md`, "Open points" | The gap today |
| --- | --- | --- |
| 1 | Lines 970 to 971, "No gate runs doctests with default features" (D-T23-2) | `test` and `test-doc` (`Justfile` lines 139 to 144) pass `--all-features`, and `features` (lines 260 to 261) only builds, so a doctest that compiles only with libpachinko's `testing` feature passes the gate |
| 2 | Lines 972 to 974, "The hook gate's `prek run --all-files` reads tracked files only" (D-T23-3) | `lint` (lines 212 to 213) runs `prek run --all-files`, which prek documents as "all tracked files", so a new file's fault surfaces only at commit, as it did in T23 (`tickets/T23-library-publish-pull.md` lines 673 to 676) and T30 (`tickets/T30-performance-evidence.md` lines 792 to 796) |
| 3 | Lines 967 to 969, "`just fix` strips trailing whitespace inside quoted output in ticket hand-backs" | The builtin `trailing-whitespace` fixer in `.pre-commit-fix.yaml` (line 42) rewrites every file, so `just fix` strips the five trailing spaces quoted inside a `text` block in `tickets/T30-performance-evidence.md` (lines 587, 607, 648, 668 and 710) |
| 4 | Lines 1034 to 1035, "`just test-loadgen` runs only by hand" (D-T30-6) | No job in `.github/workflows/ci.yml` runs `just test-loadgen` (`Justfile` lines 118 to 121), and `just test` cannot reach the excluded `loadgen/` crate |

What was checked when this ticket was written, at `4daf9df`:

- **Item 1.** libpachinko declares one feature, `testing`, and no `default`
  (`crates/libpachinko/Cargo.toml` lines 38 to 39), so default features and no features
  are the same build. No workspace crate enables `testing` through a dependency
  (`crates/pachinko/Cargo.toml` lines 25 and 36 to 38), so `cargo test --doc --workspace
  --locked` builds libpachinko without it. Run by hand, it passed: 95, 4 and 27
  doctests. The doctests that need the feature already guard it, as
  `crates/libpachinko/src/queue/expiry.rs` lines 40 to 42 do around the
  `leases_examined` assertion; nothing enforces the guard.
- **Item 2.** `prek run --help` (prek 0.5.7, `pyproject.toml` line 23) describes
  `--all-files` as "Run hooks on all tracked files" and `--files` as "Run hooks on the
  specified file paths". An untracked scratch `.yml` with a trailing space passed
  `prek run editorconfig-checker --all-files` (exit 0) and failed
  `prek run editorconfig-checker --files <it>` (exit 1, "Trailing whitespace"). The
  runner `just check` uses snapshots `git status --porcelain=v1 --untracked-files=all`
  and every path `git ls-files --cached --others --exclude-standard` prints, before
  and after each recipe (the tooling package's `run_project_check.py`, `_snapshot`,
  installed under `.pixi/envs/default/lib/python3.14/site-packages/biscuit_games_tooling/`),
  so a recipe that stages anything, `git add -N` included, fails the gate as a
  worktree change. markdownlint is the exception: `.pre-commit-config.yaml` lets prek
  append the config's `**/*.md` glob (`.markdownlint-cli2.jsonc`), so an untracked
  Markdown file with a trailing space in prose already fails `just lint`. A Markdown
  probe therefore cannot show the gap.
- **Item 3.** On a scratch copy of `tickets/T30-performance-evidence.md`,
  `prek run trailing-whitespace --config .pre-commit-fix.yaml --files <copy>` removed
  all five trailing spaces. The read-only gate does not report them: `.editorconfig`
  sets `trim_trailing_whitespace = false` for `*.md`, and markdownlint's MD009 leaves
  fenced code alone, which is why `just check` is green at `4daf9df` with them present.
  MD009 still reports a trailing space in Markdown prose, and `markdownlint --fix`
  (`.pre-commit-fix.yaml` lines 44 to 53) repairs it.
- **Item 4.** The composite setup action already fetches `loadgen/Cargo.lock`
  (`.github/actions/setup/action.yml` line 77, `just sync`; `Justfile` line 65), so a
  job needs no new setup. `loadgen/` builds into its own `loadgen/target/`, gitignored
  (`.gitignore` line 68), which the setup action's Rust cache (lines 48 to 52) does not
  save, so the job compiles loadgen's dependencies cold on every run.

`tickets/CONVENTIONS.md` §11 ("Wave 2 shares files by rebasing or merging") says "A
recipe a wave-2 ticket adds sits outside `check`'s recipe list". Item 1 puts its recipe
inside, which departs from that sentence. The reason §11 gives, the fuzz crate's
`--manifest-path` lines failing before `fuzz/` existed, does not apply to a recipe over
crates that exist. This ticket does not edit `CONVENTIONS.md`. The departure is carried
in Open points.

## Goal

`just check` runs a twentieth gate, `test-doc-default`, which runs every doctest with
default features. CI's `rust` job runs the same recipe. `just lint`, and so `just check`,
reads untracked files that Git does not ignore, and stays read-only. `just fix` leaves
the quoted evidence in `tickets/` alone, and it still repairs a trailing space anywhere
else. A CI job, `loadgen`, runs `just test-loadgen` and is one of `check`'s `needs`.
Each of items 1 to 3 is shown to catch a failure the tree did not catch before. Item 4's
job is shown red in CI, with `check` red too, on a scratch-break push to the pull
request branch, which step 9's stop asks the maintainer to authorise. The handbook says
all of this.

This ticket owns no specification rule, invariant, guarantee, scenario or test
obligation, and it names no obligation id. It changes no file under `crates/`,
`loadgen/` or `docs/specs/`. What it delivers, by source:

| Item | Source | Delivers |
| --- | --- | --- |
| 1 | D-T23-2 | The `test-doc-default` recipe, gate 20 in `pyproject.toml`'s `recipes`, a step in CI's `rust` job |
| 2 | D-T23-3 | `lint` runs the hook gate a second time over untracked, unignored files |
| 3 | T31 Open points, the `just fix` bullet | The `trailing-whitespace` fixer skips `tickets/` |
| 4 | D-T30-6 | CI's `loadgen` job, joined to `check`'s `needs` |

## Non-goals

- No change to a doctest, test or crate source. If `test-doc-default` is red on the
  untouched tree at step 3, that is a failing doctest. It is handed back to the crate's
  owner and blocks this ticket. The recipe is not weakened to pass.
- No `loadgen` in `just check`. Item 4 is a CI job, as T30's Open points offered
  (`tickets/T30-performance-evidence.md` lines 897 to 900). The local gate's length and
  the excluded crate's place outside it stay as `docs/reference/quality-gates.md` lines
  45 to 49 describe.
- No change to `check-docs`'s `prek run --all-files markdownlint-cli2 typos lychee`
  (`Justfile` line 312). Its three hooks are among `lint`'s, so `lint` now covers an
  untracked file for them. Making `check-docs` see one too is in Open points.
- No change to the pre-commit hook a commit runs. It already reads the staged files.
- No renumbering of gates 1 to 19. Decision records cite gates by number (`rg -n -i
  'gate 1[0-9]' docs/decisions`), and decision records are not edited here.
- No edit to `tickets/CONVENTIONS.md`, to a decision record, or to another ticket.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Justfile` | shared, own lines | The `test-doc-default` recipe and its comment after `test-doc` (lines 143 to 144); `lint` (lines 212 to 213) with its untracked pass; `fix` (lines 200 to 208) calling a new private `_fix-untracked` recipe that runs the untracked fixer passes; the `test-loadgen` comment (lines 118 to 119) names the CI job |
| `pyproject.toml` | shared, own lines | `"test-doc-default"` after `"snapshots-check"` in `[tool.biscuit-games-tooling]` `recipes` (line 64) |
| `pixi.lock` | shared, own lines | Expected unchanged, because `[tool.biscuit-games-tooling]` is not a pixi table. Only if `just lock-check` reports it stale, regenerated by `just lock` (network, authorised by step 3), never edited by hand |
| `.pre-commit-config.yaml` | shared, own lines | The comment at lines 102 to 104 only, if the step 5 exclusion needs saying beside the gate; no hook changes |
| `.pre-commit-fix.yaml` | shared, own lines | `exclude` on the builtin `trailing-whitespace` hook (line 42), with its reason |
| `.github/workflows/ci.yml` | shared, own lines | `just test-doc-default` after `just test` in `rust` (line 55); the new `loadgen` job before `check`; `loadgen` in `check`'s `needs` (line 179) |
| `docs/reference/quality-gates.md` | shared, own lines | The gate table (rows 3, 20 and the new `check-clean` row 21); lines 45 to 49; "Outside the gate"; "The mutating counterpart"; "In continuous integration"; "On `main`" |
| `docs/reference/commands.md` | shared, own lines | Lines 24 to 26; the `test-loadgen` row (53); the `test-doc-default` row after `test-doc` (58); the `fix` row (69); the `lint` row (75); the `check` row's count (110) |
| `docs/reference/testing.md` | shared, own lines | "Framework" (lines 13 to 14); "The `testing` feature" bullet (lines 91 to 95); "Load generation" (lines 381 to 383) |
| `docs/reference/configuration.md` | shared, own lines | The `.pre-commit-fix.yaml` row (line 110) |
| `CHANGELOG.md` | shared, own lines | pachinko Unreleased, Changed: the gate's new recipe, the two hook changes and CI running loadgen's unit tests (the file has crate sections only, and none of this changes libpachinko's behaviour) |
| `tickets/T34-gate-gaps.md` | ticket | `status:` and hand-back notes |

T32 and T33 may be landing at the same time. The files they share with this ticket are
`docs/reference/testing.md` (T32), `docs/reference/configuration.md` (T33) and
`CHANGELOG.md` (both); neither edits `Justfile`, `pyproject.toml`,
`.github/workflows/ci.yml`, `docs/reference/quality-gates.md` or
`docs/reference/commands.md`. This ticket edits only its own lines in each file. Every
scratch break below is reverted before the next step and never committed.

## Steps

1. Create the worktree from `main` after T31 has merged (`tickets/README.md`, "How to
   pick up a ticket"), and run `just initialize` there (network, authorised by this
   step). Alternatively, in a clean worktree already initialized on `main` whose branch
   carries nothing, create the branch there and skip `just initialize`. Run `just check`
   on the untouched worktree and quote its last line as the baseline. A red baseline
   stops the ticket (`tickets/CONVENTIONS.md` §11, "Green").
2. **The plan.** Use the `plan-change` skill. Read the four source bullets in Context;
   the `Justfile` recipes `test`, `test-doc`, `features`, `test-loadgen`, `fix`, `lint`,
   `check-docs` and `check`; `pyproject.toml` lines 44 to 66; `.pre-commit-config.yaml`;
   `.pre-commit-fix.yaml`; `.editorconfig`; `.markdownlint-cli2.jsonc`;
   `.github/workflows/ci.yml`; `.github/actions/setup/action.yml`; and the pages in Files
   touched. Confirm the Context facts at the new `main`, and record any that moved. The
   specification is not touched, so `just plan-spec pachinko` is not run.
3. **Doctests with default features (item 1).** Use the `fix-quality` skill.
   - Red first, on a scratch break: delete the two guard lines,
     `/// # #[cfg(feature = "testing")] {` and `/// # }`, at
     `crates/libpachinko/src/queue/expiry.rs` lines 40 and 42. `just test-doc` still
     exits 0, which is the gap. Quote it. Keep the break.
   - Add the recipe after `test-doc`, with a comment saying why it exists: libpachinko
     has no default features, so this is the build a user of the crate gets. The comment
     names what keeps the `testing` feature off: each package runs on its own, so a
     sibling's dev-dependency features cannot unify into libpachinko's doctest build,
     and the manifest check below finds any manifest that enables `testing`.

     ```just
     test-doc-default:
         cargo test --doc --package libpachinko --locked
         cargo test --doc --package pachinko --locked
     ```

   - The manifest check, which fails closed: its positive control
     `printf 'x = { features = ["testing"] }\n' | rg 'features\s*=.*"testing"'` exits 0,
     then `rg -n 'features\s*=.*"testing"' Cargo.toml crates/*/Cargo.toml` prints
     nothing and exits 1. Quote both.

   - Under the break, `just test-doc-default` exits non-zero, and its first error names
     `leases_examined` as no field of `TickReport`. Quote it. Revert the break by
     restoring the two lines. `git diff --exit-code -- crates/` then exits 0.
     `just test-doc-default` exits 0 and both runs print only `test result: ok` lines.
     Quote them.
   - Append `"test-doc-default"` after `"snapshots-check"` in `pyproject.toml`'s
     `recipes`, so it runs as gate 20 and no existing gate number moves. Add
     `- run: just test-doc-default` after `- run: just test` in CI's `rust` job. Run
     `just lock-check`, which must stay green with `pixi.lock` unchanged. If it reports
     `pixi.lock` stale, run `just lock` (network, authorised by this step) and say so.
4. **Untracked files in the hook gate (item 2).** Use the `fix-quality` skill.
   - Red first: capture the tree's state with
     `git status --short >| ai_tmp/t34-pre-lint.status`. Then create two scratch files
     that Git does not ignore, `t34-probe.yml`
     and `t34 probe.yml` (a space in the name), each holding `a: 1` followed by one
     trailing space. Also create `ai_tmp/t34-probe.yml` with the same fault. The
     positive control `git ls-files --others --exclude-standard` prints the first two
     and not the third. `just lint` exits 0, which is the gap. Quote it. Not Markdown:
     markdownlint already reads untracked Markdown (Context, item 2).
   - Change `lint` so that, after `prek run --all-files`, it runs
     `prek run --files` over every path `git ls-files -z --others --exclude-standard`
     prints, and skips that second run when there are none. It must fail closed: a
     failure of `git ls-files` fails the recipe, not an empty list. A pipe's first
     command is invisible to `sh -e`, so the list goes through a file under `mktemp`,
     outside the worktree, and not through a pipe. Paths with spaces must survive (`-z`
     with `xargs -0`; BSD and GNU `xargs` both take `-0`). One shape is below. Smoke-run
     it on macOS: `sh -n` does not prove a recipe runs.

     ```just
     lint:
         #!/bin/sh
         set -eu
         prek run --all-files
         list=$(mktemp)
         trap 'rm -f "$list"' EXIT
         git ls-files -z --others --exclude-standard >"$list"
         if [ -s "$list" ]; then
             xargs -0 prek run --files <"$list"
         fi
     ```

   - Give `fix` two untracked fixer passes, matching the tracked pair (`Justfile` lines
     199 to 202). `fix` is a line recipe, so the passes go in a private shebang helper
     recipe, `_fix-untracked`, which `fix` calls after its two `--all-files` passes and
     before its cargo lines. It builds the `mktemp` list once; the
     `git ls-files -z --others --exclude-standard >"$list"` write runs under `set -e`, so
     a listing failure still fails the recipe ("fails closed in the same way" as `lint`
     covers only this listing). When `[ -s "$list" ]` is false, both passes are skipped.
     Otherwise it runs
     `xargs -0 prek run --config .pre-commit-fix.yaml --files <"$list" || true`, then
     the same line without `|| true`, which passes its failure on. The first run may
     fail because a fixer changed a file; the second must pass, as in the tracked
     passes. The existing first `--all-files` pass keeps its leading `-`.
   - With the probes present, `just lint` exits non-zero, and `editorconfig-checker`
     names both probes at the repository root and not the file under `ai_tmp/`. Quote
     it. Then `just fix` exits 0, and `rg -c '[ \t]+$' t34-probe.yml` prints nothing.
     Quote both. Until step 5 lands, that `just fix` also strips the five quoted
     trailing spaces in `tickets/T30-performance-evidence.md` (item 3). Check
     `git diff --exit-code -- tickets/T30-performance-evidence.md` before the run; when
     it exited 0, this step authorises `git restore tickets/T30-performance-evidence.md`
     for that one path afterwards. Delete the three probes.
     `git status --short | diff ai_tmp/t34-pre-lint.status -` prints nothing, and
     `just lint` exits 0. Quote both. `git status --short` then lists only the files
     steps 3 and 4 changed (`Justfile`, `pyproject.toml`, `.github/workflows/ci.yml`,
     and `pixi.lock` if `just lock` ran) and no probe.
   - The rejected mechanism, and why: `git add -N` (intent to add) makes `--all-files`
     see the file, but it changes the index and the status line the runner snapshots,
     so `just check` would fail the recipe as a worktree change, and a check that
     stages is a defect (`AGENTS.md`). A temporary `GIT_INDEX_FILE` is hidden state
     prek does not document. The trade-off of the chosen design, which the hand-back
     and `docs/reference/quality-gates.md` state: an untracked scratch file outside
     `ai_tmp/`, such as notes at the repository root, is now linted, so a fault in it
     fails `just lint` and `just check` until it moves under `ai_tmp/` or into
     `.git/info/exclude`. A hook with `pass_filenames: false` whose `files` pattern
     matches an untracked path runs twice. In CI a fresh checkout has no untracked
     file, so the second run never happens there.
5. **`just fix` and quoted evidence (item 3).** Use the `fix-quality` skill.
   - Red first, once `git diff --exit-code -- tickets/T30-performance-evidence.md`
     exits 0: the positive control `rg -c '[ \t]+$' tickets/T30-performance-evidence.md`
     prints 5. Run
     `prek run trailing-whitespace --all-files --config .pre-commit-fix.yaml`. It
     rewrites that file, and the count drops to nothing. Quote both. This step
     authorises `git restore tickets/T30-performance-evidence.md` for that one path
     only, because this file was unmodified before the run. Run it, and the count is 5
     again.
   - Add `exclude: ^tickets/.*\.md$` to the `trailing-whitespace` hook in
     `.pre-commit-fix.yaml`, with a comment: hand-backs quote command output verbatim,
     `.editorconfig` already leaves Markdown's trailing whitespace alone, and MD009 still
     reports and `markdownlint --fix` still repairs a trailing space in ticket prose.
     The read-only gate is unchanged, because `.pre-commit-config.yaml` holds no
     fixer. Excluding every `*.md` would match `.editorconfig` more closely. It is
     wider than the source asked for, so it goes to Open points.
   - Green: `just fix` exits 0, the count is still 5, and
     `git diff --exit-code -- tickets/` exits 0.
   - The gate still bites outside the exclusion. First capture
     `git diff -- Justfile docs/reference/commands.md tickets/README.md >| ai_tmp/t34-pre-fix.diff`.
     Then add one trailing space to the end of `Justfile` line 17 (`default:`), of
     `docs/reference/commands.md` line 11, and of a prose line in `tickets/README.md`.
     `just lint` exits non-zero and names all three (`editorconfig-checker` for the
     `Justfile`, MD009 for the two Markdown files).
     `just fix` exits 0, and then
     `git diff -- Justfile docs/reference/commands.md tickets/README.md | diff ai_tmp/t34-pre-fix.diff -`
     prints nothing, because each fixer restored its file byte for byte. Quote the
     three. If that comparison prints a difference, the fixer did not restore the file
     exactly: remove the added space by hand until the diff matches the captured one,
     and never run `git restore Justfile`. This step authorises `git restore` of
     `docs/reference/commands.md` and `tickets/README.md` only, and only for a path
     whose captured diff was empty. The hand-back records the difference as a finding.
6. **loadgen in CI (item 4).** Use the `fix-quality` skill.
   - Red first, locally, on a scratch break: change `2_000` to `2_001` in
     `loadgen/src/report.rs` line 98. `just test-loadgen` exits non-zero and names
     `the_rate_is_per_second_of_the_elapsed_time`. Quote it as the recipe's control,
     not the job's: the recipe fails this way before T34 too. Revert the break.
     `git diff --exit-code -- loadgen/` exits 0, and `just test-loadgen` exits 0.
   - Add a job to `.github/workflows/ci.yml` before `check`, shaped like `deny`: name
     `loadgen`, `ubuntu-latest`, a timeout that covers a cold build of loadgen's
     dependencies (start at 15 minutes and adjust it from the first run), the pinned
     checkout with `persist-credentials: false`, `./.github/actions/setup` with
     `cache-key: loadgen`, then `just test-loadgen`, then `just check-clean`. Neither
     the job nor any of its steps carries `continue-on-error` or an `if:`. Add
     `loadgen` to `check`'s `needs`. Say in a comment why it is a CI job and not in
     `just check`, and that the Rust cache does not cover `loadgen/target/`. Every new
     `run:` is one line (`docs/reference/quality-gates.md` lines 214 to 222). `just lint`
     runs actionlint over it.
   - Structural check, a static guard and not a substitute for the job's red, which
     fails closed and is required in every case. Red first:
     `git show <baseline>:.github/workflows/ci.yml | rg -n '^  loadgen:'` exits 1.
     Then `rg -n -A12 '^  loadgen:' .github/workflows/ci.yml` prints the job. Quote it, and
     quote that the job has no `if:` and no `continue-on-error`, that it runs
     `just test-loadgen`, and that `check`'s `needs` contains `loadgen`.
   - The job is seen red or green only on a push. The push and the pull request are
     separately authorised stops (`tickets/CONVENTIONS.md` §11). Step 9 reads the run.
     Showing the job red in CI needs a pushed scratch commit, which step 9 asks the
     maintainer to authorise. The job's negative control is step 9's scratch-break run.
     Without it, item 4 is not done.
7. **The handbook.** Use the `review-docs` skill. Each fact goes on the page that owns
   its topic in `docs/manifest.yml`.
   - `docs/reference/quality-gates.md`, which owns the gates: a row 20 for
     `test-doc-default` (`cargo test --doc` with default features; every doc example
     compiles and passes without the `testing` feature), with `check-clean` becoming 21;
     row 3's "Runs" and "Proves" name the untracked pass; lines 45 to 49 and "Outside
     the gate" say CI's `loadgen` job runs `test-loadgen`; "The mutating counterpart"
     says `trailing-whitespace` skips `tickets/` and why; "In continuous integration"
     counts the gate jobs again, adds `test-doc-default` to the `rust` row and a
     `loadgen` row, and says which recipe each job maps to; "On `main`" names
     `loadgen` among `check`'s `needs`; the item 2 trade-off from step 4.
   - `docs/reference/commands.md`, which owns the recipes: the `test-doc-default` row,
     Gate 20; the `lint` row, untracked files included; the `fix` row, `tickets/` left
     alone by the whitespace fixer and untracked, unignored files repaired too; the
     `test-loadgen` row and lines 24 to 26, run by CI's `loadgen` job and still outside
     `just check`; line 110, "twenty gates".
   - `docs/reference/testing.md`: "Framework", that `just test-doc-default` runs the
     doctests with default features; the `testing` feature bullet, that a doctest using
     it guards the lines with `# #[cfg(feature = "testing")]` and gate 20 enforces it;
     "Load generation" lines 381 to 383, that CI's `loadgen` job runs the unit tests
     and nothing runs the generator.
   - `docs/reference/configuration.md` line 110: the fixer's exclusion in one clause.
   - `CHANGELOG.md`, pachinko's Unreleased section, under Changed.
   - `rg -n -i 'nineteen' docs --glob '!docs/decisions/*'` prints nothing afterwards.
     Its positive control is the same search at the baseline commit, which prints
     `docs/reference/commands.md:110`. Run `just check-docs`.
8. **Adversarial review** (`tickets/CONVENTIONS.md` §14), checkpoint T34-1, before the
   push stop: the whole diff against the Goal, the item table, Files touched and the
   red and green quotes of steps 3 to 6. Ask: whether `test-doc-default` can pass with
   the `testing` feature on, for example through workspace feature unification if a
   member gains a dev-dependency on libpachinko with `testing`; whether `lint` can pass
   when `git ls-files` fails, when a path holds a space or a newline, or when the
   untracked list is empty; whether any recipe now writes inside the worktree or to the
   index, so that `just check`'s snapshot would trip or a stage would go unnoticed;
   whether the fixer's exclusion lets a real trailing-space fault through, in a ticket
   or anywhere else, and whether the gate would report it; whether the `loadgen` job
   can be skipped, cancelled or green while its test fails, whether it is in `check`'s
   `needs`, and whether it reaches the network after the setup action; whether each
   red quote came from the break it names and each break was reverted; whether a page
   still says nineteen gates, that no CI job runs the loadgen tests, or that `lint`
   reads every file; and whether any file outside Files touched changed. Two reviewer
   subagents with fresh contexts run in parallel, one lens each, `conformance` and
   `breakage`. Each runs on the harness §14 selects, the one the executing agent is not
   when it is on `PATH`: `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T34-1/findings-<lens>.md "<brief>" < /dev/null`, or, from the
   worktree, `claude -p --model claude-opus-5-5 --permission-mode plan "<brief>" >
   ai_tmp/review/T34-1/findings-<lens>.md < /dev/null` (§14's commands; `-o` writes
   only the reviewer's last message, and a read-only reviewer writes no file of its
   own). Separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T34-1/verdicts.md`, each finding valid with its fix or invalid with
   the reason. One writer applies the valid fixes one at a time, only within Files
   touched, and hands back any fix outside it. If a high-severity finding was valid, a
   second round reviews the fix diff and writes beside the first with a `-2` suffix.
   Two rounds at most. Rerun the red and green quotes of any step a fix touched.
9. **Commit, merge, push and CI.** Use the `project-check` skill: `just check`, with
   `fix-quality` for any failing recipe. Commit on the ticket branch (commit A). Run
   `git fetch origin` (network, authorised by this step) and merge `origin/main`
   (`tickets/CONVENTIONS.md` §11). T32 or T33 may have merged since this branch was cut,
   so a conflict in a shared file keeps both tickets' lines; a conflict in a file
   neither sibling touches is not sibling work, so stop and report it. A lockfile is
   regenerated with `just lock` (network, authorised by this step), never merged by
   hand. Rerun the Verification block at the merge commit and requote it. Then stop and
   ask for three authorisations together: the push, the pull request, and one
   scratch-break commit (step 6's break) pushed to the pull request branch followed by
   its revert, which is the job's negative control the breakdown requires. When
   authorised, push and open the pull request, then read its CI run. Quote the run URL,
   the tested merge commit, and the `rust` job's `just test-doc-default` step and the
   `loadgen` job, each green, and `check` green with `loadgen` among its `needs`.
   When authorised, push the scratch-break commit to the pull request branch: quote
   `loadgen` red and `check` red with the run URL and the tested merge commit, then
   push the revert and quote both green. If the maintainer declines the scratch-break
   push, quote the refusal, do not set `status: done`, and hand back with item 4's
   negative control recorded as unmet.
10. **Adversarial review** (`tickets/CONVENTIONS.md` §14), checkpoint T34-2, before the
    final commit: the whole diff of the branch against `main`, hand-back notes
    included, with the CI run of step 9 beside it. Ask: whether every acceptance
    criterion is shown by a quote headed by the commit it ran at; whether the CI quote
    names the merge commit the run tested, not the branch head; whether a quote went
    stale after a fix or the merge; whether every Open point names an owner; and the
    questions of T34-1 again for anything changed since. Two reviewer subagents, one
    lens each, each on the harness §14 selects, as in step 8: `codex exec -m
    gpt-6-astra -s read-only -C <worktree> -o ai_tmp/review/T34-2/findings-<lens>.md
    "<brief>" < /dev/null`, or, from the worktree, `claude -p --model claude-opus-5-5
    --permission-mode plan "<brief>" > ai_tmp/review/T34-2/findings-<lens>.md <
    /dev/null`;
    triage writes `ai_tmp/review/T34-2/verdicts.md`; one writer applies the valid
    fixes, only within Files touched; a second round, if a valid finding was high,
    writes beside the first with a `-2` suffix; two rounds at most. Then `just check`,
    set `status: done`, and commit B with the hand-back notes, each quoted output headed
    by the commit it ran at. Pushing commit B is separately authorised: stop and ask.
    Merging to `main` runs `pages.yml`, which deploys the API reference (decision
    0013), so the merge is a deployment and the maintainer's action.

## Acceptance criteria

- `just test-doc-default` exits 0 at the final commit, quoted, and is the twentieth entry
  of `recipes` in `pyproject.toml`. Under step 3's scratch break it exits non-zero
  while `just test-doc` exits 0, both quoted. The recipe runs libpachinko and pachinko
  as separate `cargo test --doc --package` runs, and step 3's manifest check
  `rg -n 'features\s*=.*"testing"' Cargo.toml crates/*/Cargo.toml` prints nothing at
  the final commit, after its positive control, both quoted.
- With step 4's probes present, `just lint` exits non-zero and names the two unignored
  probes, one with a space in its name, and not the ignored one. Before the change it exited 0. Both are quoted.
  After the probes are deleted, `just lint` exits 0.
- No check recipe (any recipe `just check` runs) writes to the index or inside the
  worktree; `just fix` and `just lock` are the repairing recipes and are exempt:
  `just check` ends with `All checks passed and the worktree is unchanged.` and `git status --short` prints nothing.
- `just fix` leaves `tickets/T30-performance-evidence.md`'s five trailing spaces in
  place, and `git diff --exit-code -- tickets/` exits 0. Before the change, the fixer
  removed them, quoted. A trailing space added to the `Justfile`, to
  `docs/reference/commands.md` and to ticket prose is reported by `just lint` and
  repaired by `just fix`, quoted. `just fix` also repairs a trailing space in an
  untracked, unignored file (step 4's `t34-probe.yml`), quoted.
- `.github/workflows/ci.yml` has a `loadgen` job running `just test-loadgen` and
  `just check-clean`, `loadgen` is in `check`'s `needs`, `ci.yml` holds no
  `continue-on-error`, and the `rust` job runs
  `just test-doc-default`. The pull request's CI run is green on both, with the run URL
  and the tested merge commit quoted. `just test-loadgen` failed locally under step 6's
  scratch break, quoted, as the recipe's control. For the job, the CI run of
  step 9's scratch-break push shows `loadgen` and `check` red, then the run after its
  revert shows both green, each quoted with the run URL and the tested merge commit.
- The handbook pages in Files touched say what landed. `rg -n -i 'nineteen' docs
  --glob '!docs/decisions/*'` prints nothing, after its positive control.
  `just check-docs` is green.
- No file outside Files touched changed, and no scratch break or probe is committed.
- The adversarial reviews in Steps ran (`tickets/CONVENTIONS.md` §14), and every valid
  finding is fixed or handed back.

## Verification

Run from the worktree root in `zsh` or `bash`, so that `set -o pipefail` applies. Each
line prints its exit status, and a search that must print nothing follows its positive
control (`tickets/CONVENTIONS.md` §11, "Checks fail closed").

```sh
set -o pipefail
just lock-check; echo "exit $?"
just test-doc; echo "exit $?"
just test-doc-default; echo "exit $?"
printf 'x = { features = ["testing"] }\n' | rg 'features\s*=.*"testing"'; echo "exit $?"
rg -n 'features\s*=.*"testing"' Cargo.toml crates/*/Cargo.toml; echo "exit $?"
rg -n '"test-doc-default"' pyproject.toml; echo "exit $?"
rg -n 'just test-doc-default|just test-loadgen|^  loadgen:|needs: \[' .github/workflows/ci.yml; echo "exit $?"
rg -n -A12 '^  loadgen:' .github/workflows/ci.yml; echo "exit $?"
rg -n 'continue-on-error' tickets/T34-gate-gaps.md; echo "exit $?"
rg -n 'continue-on-error' .github/workflows/ci.yml; echo "exit $?"
git ls-files --others --exclude-standard; echo "exit $?"
just lint; echo "exit $?"
rg -c '[ \t]+$' tickets/T30-performance-evidence.md; echo "exit $?"
just fix; echo "exit $?"
git diff --exit-code -- tickets/; echo "exit $?"
rg -c '[ \t]+$' tickets/T30-performance-evidence.md; echo "exit $?"
just test-loadgen; echo "exit $?"
git show <baseline>:docs/reference/commands.md | rg -n -i 'nineteen'; echo "exit $?"
rg -n -i 'nineteen' docs --glob '!docs/decisions/*'; echo "exit $?"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

`<baseline>` is the commit step 1 quoted. The `continue-on-error` search over this ticket
is the positive control for the one over `ci.yml`, which must print nothing (exit 1). The
CI negative control is step 9's scratch-break run, quoted in its hand-back section. The `git ls-files` line
prints nothing on a clean tree, and step 4's quotes are its positive control. The red
runs of steps 3 to 6 are quoted in their own hand-back sections, not here.

## Hand-back notes

### Baseline

Worktree `ticket/t34-gate-gaps`, cut from `wave-2-tickets` at `53d64b7` (which contains
`4daf9df`); by the maintainer's choice `wave-2-tickets` stands in for `main` as the merge
target and the pull request base. `just initialize` ran (network, authorised by step 1)
and exited 0.

At `53d64b7`, untouched:

```text
$ just check
exit 0
All checks passed and the worktree is unchanged.
```

### Step 2: the plan

Read the four source bullets, the recipes, `pyproject.toml`, both hook configurations,
`.editorconfig`, `.markdownlint-cli2.jsonc`, `ci.yml`, the setup action and the pages in
Files touched. At `53d64b7` every Context fact holds (the `Justfile` lines, the
`pyproject.toml` recipe list ending at `snapshots-check`, `.pre-commit-fix.yaml` line 42,
`ci.yml`'s `check.needs` and the `deny` job, the setup action's line 77 and Rust cache,
`.gitignore` line 68, prek 0.5.7, the five trailing spaces in T30, and
`docs/reference/commands.md:110` the only `nineteen`), except one that moved:

- **The step 3 break target moved.** `816af9a` ("Make libpachinko's crate root the
  caller's interface"), after `4daf9df`, moved `leases_examined` from `TickReport` to
  `testing::TickCost`, so the guard at `crates/libpachinko/src/queue/expiry.rs` lines 40
  to 42 no longer exists. The remaining `testing`-guarded doctests are in
  `crates/libpachinko/src/queue/types.rs` (lines 147, 240, 265 and 349). Step 3's break
  used the smallest, the `PocketStatus` example: its guard lines 147 and 150. The error
  it names is `could not find testing in libpachinko`, not `leases_examined`.
- libpachinko's default-features doctest count is 95, and with every feature 103.

`just plan-spec` was not run; the specification is untouched.

### Step 3: doctests with default features

The manifest check, at `53d64b7`:

```text
$ printf 'x = { features = ["testing"] }\n' | rg 'features\s*=.*"testing"'
x = { features = ["testing"] }
exit 0
$ rg -n 'features\s*=.*"testing"' Cargo.toml crates/*/Cargo.toml
exit 1
```

T34-1 (below) found that this check misses a multi-line `features` array, and that
`pachinko`'s own dev-dependencies could still turn `testing` on for its doctests. The
recipe therefore became a `#!/bin/sh` recipe that first runs
`cargo tree --edges features --package <p> --invert libpachinko --locked --offline` per
package, fails unless it lists libpachinko, and fails if it lists
`libpachinko feature "testing"`; then it runs the ticket's two
`cargo test --doc --package … --locked` lines. Triage showed it fail under a scratch
multi-line dev-dependency (reverted, `Cargo.lock` unchanged). The quotes below are the
rerun after that fix.

At `53d64b7` with uncommitted changes (the final recipe; scratch break: `types.rs`
guard lines 147 and 150 deleted):

```text
$ just test-doc
exit 0
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
$ just test-doc-default
exit 101
test crates/libpachinko/src/queue/types.rs - queue::types::PocketStatus (line 146) ... FAILED
error[E0432]: unresolved import `libpachinko::testing`
    |                  ^^^^^^^ could not find `testing` in `libpachinko`
```

The break reverted, at `53d64b7` with uncommitted changes:

```text
$ git diff --exit-code -- crates/
exit 0
$ just test-doc-default
exit 0
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`"test-doc-default"` follows `"snapshots-check"` in `pyproject.toml` (gate 20), and
`- run: just test-doc-default` follows `- run: just test` in `ci.yml`'s `rust` job.
`just lock-check` exited 0 and `pixi.lock` is unchanged, so `just lock` did not run.

### Step 4: untracked files in the hook gate

Red, at `53d64b7` with uncommitted changes (step 3's edits; `lint` unchanged; probes
`t34-probe.yml`, `t34 probe.yml` and `ai_tmp/t34-probe.yml`, each `a: 1` plus a trailing
space):

```text
$ git ls-files --others --exclude-standard
t34 probe.yml
t34-probe.yml
exit 0
$ just lint
exit 0
EditorConfig.............................................................Passed
```

`lint` now runs `prek run --all-files`, then a private shebang recipe,
`_prek-untracked <config>`, which writes `git ls-files -z --others --exclude-standard`
to a `mktemp` file under `set -eu`, skips when it is empty, and otherwise runs
`xargs -0 … prek run --config <config> --files`. `fix` calls `_fix-untracked` after its
two `--all-files` passes; it runs `_prek-untracked .pre-commit-fix.yaml 2`, whose second
parameter makes the helper run the one list twice, the first run with `|| true` and the
second without, so a listing failure still fails `fix` (T34-2's conformance-1 replaced
two calls, the first with a leading `-` that also hid a listing failure). One helper replaced the ticket's three inline copies after
T34-1, whose breakage-1 finding (a root path named `--help` reached prek as an option,
and `lint` exited 0) added a guard: a path beginning with `-` is named on stderr and
fails the recipe, and the other paths are still checked. The rejected `git add -N` and
`GIT_INDEX_FILE` mechanisms are as the ticket says; the trade-off is in
`docs/reference/quality-gates.md`, "What the hook gate contains".

With the probes present, at `d433690` with uncommitted changes (the recipes after
T34-2's fix):

```text
$ just lint
exit 1
EditorConfig.............................................................Failed
- hook id: editorconfig-checker
  t34 probe.yml:
      1: Trailing whitespace
  t34-probe.yml:
      1: Trailing whitespace
```

No line of that output names `ai_tmp/`. With a fourth probe, `./--help`, added and then
removed:

```text
$ just lint
exit 1
untracked path begins with -: --help
  t34 probe.yml:
  t34-probe.yml:
error: recipe `_prek-untracked` failed with exit code 1
```

Then, `--help` removed, at `d433690` with uncommitted changes:

```text
$ git diff --exit-code -- tickets/T30-performance-evidence.md
exit 0
$ just fix
exit 0
$ rg -c '[ \t]+$' t34-probe.yml
exit 1
$ rg -c '[ \t]+$' 't34 probe.yml'
exit 1
$ rg -c '[ \t]+$' tickets/T30-performance-evidence.md
5
exit 0
```

The first run of this step, before step 5, did strip T30's five spaces as the ticket
foresaw; its diff had exited 0 first, so `git restore tickets/T30-performance-evidence.md`
ran for that one path. The rerun above came after step 5, so T30 stayed whole. The
probes deleted:

```text
$ git status --short | diff ai_tmp/t34-pre-lint.status -
exit 0
$ just lint
exit 0
```

### Step 5: `just fix` and quoted evidence

Red, at `53d64b7` with uncommitted changes (`.pre-commit-fix.yaml` unchanged; `prek`
from `.pixi/envs/default/bin`):

```text
$ git diff --exit-code -- tickets/T30-performance-evidence.md
exit 0
$ rg -c '[ \t]+$' tickets/T30-performance-evidence.md
5
exit 0
$ prek run trailing-whitespace --all-files --config .pre-commit-fix.yaml
- hook id: trailing-whitespace
- files were modified by this hook
  Fixing tickets/T30-performance-evidence.md
exit 1
$ rg -c '[ \t]+$' tickets/T30-performance-evidence.md
exit 1
$ git restore tickets/T30-performance-evidence.md
exit 0
$ rg -c '[ \t]+$' tickets/T30-performance-evidence.md
5
exit 0
```

`exclude: ^tickets/.*\.md$` is on the `trailing-whitespace` hook, with its reason in a
comment. `.pre-commit-config.yaml` is unchanged. Green, at `53d64b7` with uncommitted
changes:

```text
$ just fix
exit 0
$ rg -c '[ \t]+$' tickets/T30-performance-evidence.md
5
exit 0
$ git diff --exit-code -- tickets/
exit 0
```

The gate still bites, rerun at `d433690` with uncommitted changes, after T34-2's fix
(one trailing space added at `Justfile:17`, `docs/reference/commands.md:11` and
`tickets/README.md:3`):

```text
$ just lint
exit 1
  Justfile:
      17: Trailing whitespace
  docs/reference/commands.md:11:86 error MD009/no-trailing-spaces Trailing spaces [Expected: 0 or 2; Actual: 1]
  tickets/README.md:3:85 error MD009/no-trailing-spaces Trailing spaces [Expected: 0 or 2; Actual: 1]
$ just fix
exit 0
$ git diff -- Justfile docs/reference/commands.md tickets/README.md | diff ai_tmp/t34-pre-fix.diff -
exit 0
$ git diff --exit-code -- tickets/
exit 0
```

Each fixer restored its file byte for byte, so no `git restore` ran.
editorconfig-checker indents its findings with a tab; the quotes in steps 4 and 5
show it as four spaces, because markdownlint forbids hard tabs.

### Step 6: loadgen in CI

The recipe's control, at `53d64b7` with uncommitted changes (scratch break:
`loadgen/src/report.rs:98`, `2_000` to `2_001`):

```text
$ just test-loadgen
exit 101
test report::tests::the_rate_is_per_second_of_the_elapsed_time ... FAILED
thread 'report::tests::the_rate_is_per_second_of_the_elapsed_time' (478522207) panicked at src/report.rs:98:9:
test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Reverted:

```text
$ git diff --exit-code -- loadgen/
exit 0
$ just test-loadgen
exit 0
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The structural check, at `53d64b7` with uncommitted changes:

```text
$ git show 53d64b7:.github/workflows/ci.yml | rg -n '^  loadgen:'
exit 1
$ rg -n 'if:|continue-on-error' .github/workflows/ci.yml
177:    if: github.event_name == 'pull_request'
194:  # `if: always()` is what makes it report on a failure instead of being
200:    if: always()
204:      - if: ${{ contains(needs.*.result, 'failure') || contains(needs.*.result, 'cancelled') || contains(needs.*.result, 'skipped') }}
exit 0
```

Lines 177, 194, 200 and 204 are `semver` and `check`, outside the `loadgen` job (lines
160 to 172, quoted under Final `just check`), so the job has no `if:` and no
`continue-on-error`. It runs `just test-loadgen`, and `check`'s `needs` holds `loadgen`
(line 199). `just lint`, which runs actionlint, exited 0. The `test-loadgen` comment in
the `Justfile` names the CI job. The job's timeout stays at 15 minutes: its first run
took 1m30s.

### Step 7: the handbook

`docs/reference/quality-gates.md`: rows 3, 20 and 21; lines 45 to 49; "Outside the
gate"; the new `test-doc-default` row under "When a gate fails"; "What the hook gate
contains" (the untracked run, the trade-off, and the `-` guard); "The mutating
counterpart"; "In continuous integration" (five gate jobs, `image` the sixth, `check`
the seventh needing six, `semver` the eighth, `test-doc-default` in `rust`'s row, a
`loadgen` row, and the mapping paragraph); and "On `main`". `docs/reference/commands.md`:
lines 24 to 26, the `test-loadgen`, `test-doc-default`, `fix`, `lint` and `check` rows.
`docs/reference/testing.md`: "Framework", the `testing` feature bullet and "Load
generation". `docs/reference/configuration.md`: the `.pre-commit-fix.yaml` row.
`CHANGELOG.md`: pachinko Unreleased, a new Changed section.

At `53d64b7` with uncommitted changes:

```text
$ git show 53d64b7:docs/reference/commands.md | rg -n -i 'nineteen'
110:| `just check` | The nineteen gates in order, proving the worktree is unchanged between each, then `check-clean`. |
exit 0
$ rg -n -i 'nineteen' docs --glob '!docs/decisions/*'
exit 1
$ just check-docs
exit 0
Validated 47 pages and 48 canonical topics.
```

A sweep for stale text outside Files touched found
`docs/project/repository-map.md:103`, which says `just test-loadgen` runs "by hand and
never in CI or the gate". It is outside Files touched, so it is in Open points.

### T34-1: the whole diff before the push (step 8)

Harness: Codex, `codex exec -m gpt-6-astra -s read-only`, two reviewers in parallel
(`conformance`, `breakage`) with fresh contexts; the executing agent is Claude Code.
Triage and apply: one fresh-context Claude subagent (`claude-opus-5-5`), the single
writer. Raw files: `ai_tmp/review/T34-1/` (not committed).

- **conformance**: no findings.
- **breakage-1** (medium, `Justfile` `lint` and `_fix-untracked`): an untracked root file
  named `--help` reached prek as an option, so `just lint` printed help and exited 0.
  **Valid.** A `./` prefix does not help, because prek normalises it away and
  editorconfig-checker takes no `--`. Fixed with a guard that names a path beginning with
  `-` and fails, still checking the rest; the executing agent then folded the three
  inline copies into `_prek-untracked`. Handed back: a *tracked* such path has the same
  exposure through `--all-files` and the commit hook (Open points).
- **breakage-2** (medium, `Justfile` `test-doc-default`): a future pachinko
  dev-dependency enabling `testing` would reach pachinko's doctests, and the manual
  manifest regex misses a multi-line array. **Valid**, shown with a scratch dev-dependency
  (reverted, `Cargo.lock` unchanged). Fixed with the `cargo tree` assertion in step 3.

No finding was high, so there was no second round. Steps 3 to 5 were rerun after the
fixes. Steps 4 and 5 were rerun again after T34-2, and their quotes above are those
reruns. Then `just check`, at `53d64b7` with
uncommitted changes, exited 0 with `All checks passed and the worktree is unchanged.`

### Step 9: commit, merge, push and CI

Commit A is `5988ea7`. `git fetch origin` ran; `origin/wave-2-tickets` and
`wave-2-tickets` were both `53d64b7`, so `git merge --no-edit origin/wave-2-tickets`
printed `Already up to date.`; there was no lockfile to regenerate. The Verification
block ran at `5988ea7` and every line matched (quoted under Final `just check`, rerun at
the final commit). The first run of the block, in a child `zsh` whose `.zshenv` puts
`~/.pixi/bin` (a global rustc 1.96.0) before rustup's proxy, failed `check-toolchain` on
the cargo lines. That was the shell, not the tree; the rerun put `~/.cargo/bin` first.

The maintainer authorised the push, the pull request and the scratch-break push
together. Pull request: `https://github.com/BiscuitBin/pachinko/pull/45`, base
`wave-2-tickets`.

Run `https://github.com/BiscuitBin/pachinko/actions/runs/43390117588`, head `5988ea7`,
tested merge commit `d0cc846` ("Merge 5988ea7… into 53d64b7…"):

```text
rust      pass  3m46s   (step "Run just test-doc-default": success)
loadgen   pass  1m30s   (steps "Run just test-loadgen" and "Run just check-clean": success)
coverage  pass; deny pass; documents pass; image pass; semver pass
check     pass  3s      (needs: [rust, coverage, deny, documents, image, loadgen])
rust / Run just test-doc-default:
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
loadgen / Run just test-loadgen:
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The scratch break, commit `0fc36a6` (`loadgen/src/report.rs:98`, `2_000` to `2_001`).
Run `https://github.com/BiscuitBin/pachinko/actions/runs/43390686273`, tested merge commit
`2211197` ("Merge 0fc36a6… into 53d64b7…"), conclusion failure:

```text
loadgen: failure
test report::tests::the_rate_is_per_second_of_the_elapsed_time ... FAILED
thread 'report::tests::the_rate_is_per_second_of_the_elapsed_time' (2987) panicked at src/report.rs:98:9:
test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
check: failure   (step "Run exit 1")
rust, coverage, deny, documents, image, semver: success
```

Its revert, commit `d433690` (`git diff --exit-code 5988ea7 -- loadgen/` exited 0, and
`git diff 5988ea7 d433690` is empty). Run
`https://github.com/BiscuitBin/pachinko/actions/runs/43391164522`, tested merge commit
`06eedd9` ("Merge d433690… into 53d64b7…"), conclusion success:

```text
loadgen: success   test result: ok. 12 passed; 0 failed; …
check: success
rust: success      test-doc-default: 95, 4 and 27 passed
coverage, deny, documents, image, semver: success
```

The scratch-break commit was made with `--no-verify`, which skipped the shared
pre-commit hook for that one commit. Commit A and the revert ran the hook, and it passed.

### T34-2: the whole diff before the final commit (step 10)

Harness: Codex, `codex exec -m gpt-6-astra -s read-only`, two reviewers in parallel
over `git diff 53d64b7` with the hand-back notes uncommitted on top of `d433690`, and the
three CI runs of step 9. Triage and apply: one fresh-context Claude subagent
(`claude-opus-5-5`), the single writer. Raw files: `ai_tmp/review/T34-2/`.

- **conformance-1** (medium, `Justfile` `_fix-untracked`): its first call,
  `-just _prek-untracked …`, ignored a failing `git ls-files` too, contrary to step 4.
  **Valid.** Fixed: `_prek-untracked config passes="1"` lists once under `set -eu` and,
  with `passes` 2, runs the list with `|| true` and then again without; `_fix-untracked`
  is one call with `2`. Smoke runs: `GIT_DIR=/nonexistent just _fix-untracked` exits 128
  (`fatal: not a git repository`), and a `passes` of 3 exits 2.
- **conformance-2** (medium, this ticket): "Final `just check`" was empty. **Invalid at
  the checkpoint.** Step 10 orders the final `just check` after triage, and §11 makes any
  earlier quote stale. It is filled below.
- **breakage-1** (medium, `docs/reference/quality-gates.md`): `check-added-large-files`
  reads staged additions only, even under `--files`, so an untracked 1 MiB file passes
  `just lint`. **Valid**, shown with a scratch probe (deleted). The hook change is outside
  this ticket's comment-only line in `.pre-commit-config.yaml`, so it is handed back to
  Open points. Gate row 3, "What the hook gate contains" and the `just lint` row in
  `docs/reference/commands.md` now state the limit.

No finding was high, so there was no second round. Steps 4 and 5 were rerun, and
`just lint` and `just check-docs` exited 0.

### Final `just check`

The Verification block, at `d433690` with uncommitted changes (this commit's content:
T34-2's fixes and these notes), with `~/.cargo/bin` first on `PATH`:

```text
$ just lock-check >/dev/null 2>&1
exit 0
$ just test-doc 2>&1 | rg "^test result"
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
exit 0
$ just test-doc-default 2>&1 | rg "^test result|FAILED|^error"
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
exit 0
$ printf 'x = { features = ["testing"] }
' | rg 'features\s*=.*"testing"'
x = { features = ["testing"] }
exit 0
$ rg -n 'features\s*=.*"testing"' Cargo.toml crates/*/Cargo.toml
exit 1
$ rg -n '"test-doc-default"' pyproject.toml
65:  "test-doc-default",
exit 0
$ rg -n 'just test-doc-default|just test-loadgen|^  loadgen:|needs: \[' .github/workflows/ci.yml
56:      - run: just test-doc-default
160:  loadgen:
171:      - run: just test-loadgen
199:    needs: [rust, coverage, deny, documents, image, loadgen]
exit 0
$ rg -n -A12 '^  loadgen:' .github/workflows/ci.yml
160:  loadgen:
161-    name: loadgen
162-    runs-on: ubuntu-latest
163-    timeout-minutes: 15
164-    steps:
165-      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
166-        with:
167-          persist-credentials: false
168-      - uses: ./.github/actions/setup
169-        with:
170-          cache-key: loadgen
171-      - run: just test-loadgen
172-      - run: just check-clean
exit 0
$ rg -n 'continue-on-error' tickets/T34-gate-gaps.md
292:     the job nor any of its steps carries `continue-on-error` or an `if:`. Add
301:     quote that the job has no `if:` and no `continue-on-error`, that it runs
423:  `continue-on-error`, and the `rust` job runs
452:rg -n 'continue-on-error' tickets/T34-gate-gaps.md; echo "exit $?"
453:rg -n 'continue-on-error' .github/workflows/ci.yml; echo "exit $?"
468:`<baseline>` is the commit step 1 quoted. The `continue-on-error` search over this ticket
740:$ rg -n 'if:|continue-on-error' .github/workflows/ci.yml
750:`continue-on-error`. It runs `just test-loadgen`, and `check`'s `needs` holds `loadgen`
exit 0
$ rg -n 'continue-on-error' .github/workflows/ci.yml
exit 1
$ git ls-files --others --exclude-standard
exit 0
$ just lint >/dev/null 2>&1
exit 0
$ rg -c '[     ]+$' tickets/T30-performance-evidence.md
5
exit 0
$ just fix >/dev/null 2>&1
exit 0
$ git diff --exit-code -- tickets/
(the diff of tickets/T34-gate-gaps.md alone: this commit's hand-back notes and status, uncommitted when the block ran)
exit 1
$ rg -c '[     ]+$' tickets/T30-performance-evidence.md
5
exit 0
$ just test-loadgen 2>&1 | rg "^test result"
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
exit 0
$ git show 53d64b7:docs/reference/commands.md | rg -n -i 'nineteen'
110:| `just check` | The nineteen gates in order, proving the worktree is unchanged between each, then `check-clean`. |
exit 0
$ rg -n -i 'nineteen' docs --glob '!docs/decisions/*'
exit 1
$ just check-docs >/dev/null 2>&1
exit 0
$ just check 2>&1 | tail -1
All checks passed and the worktree is unchanged.
exit 0
$ git status --short
 M Justfile
 M docs/reference/commands.md
 M docs/reference/quality-gates.md
 M tickets/T34-gate-gaps.md
exit 0
```

`git diff --exit-code -- tickets/` exited 1 only because these notes were uncommitted;
no other ticket file differed, and `just fix` changed nothing. Commit B holds exactly
the four files `git status --short` lists above.

## Open points

- `tickets/CONVENTIONS.md` §11 says a recipe a wave-2 ticket adds sits outside `check`'s
  recipe list. Item 1 departs from it, and the sentence wants a dated note that T34
  added gate 20. Owner: maintainer, through a `main` pull request.
- `docs/decisions/0004-hook-runner-and-checkers.md` line 42 and
  `docs/decisions/0009-rust-quality-gate.md` line 15 say `just check` runs nineteen
  recipes. They are decision records, which this ticket does not edit. Owner:
  maintainer, to amend both or rule them historical, as with D-T30-7 in T31's Open
  points.
- `check-docs` still runs its three hooks over tracked files only. `lint` covers an
  untracked file for them, so the gap is in the order a failure is named, not in what
  `just check` catches. Owner: maintainer.
- Whether the `trailing-whitespace` fixer should skip every `*.md`, which would match
  `.editorconfig`, rather than `tickets/` alone. Owner: maintainer.
- `docs/project/repository-map.md` line 103 says `just test-loadgen` runs "by hand and
  never in CI or the gate". CI's `loadgen` job now runs it. The page is outside this
  ticket's Files touched. Owner: maintainer.
- A *tracked* file whose name begins with `-` reaches prek's hooks as an option through
  `prek run --all-files` and through the pre-commit hook, as an untracked one did before
  T34-1's guard (breakage-1). This predates T34. The root fix is hooks that end their
  options with `--`, which is a hook change in `.pre-commit-config.yaml`, beyond this
  ticket's comment-only line there. Owner: maintainer.
- `check-added-large-files` reads staged files only, whatever `--files` gives it (prek
  0.5.7), so `lint`'s untracked run never rejects an untracked file over 768 KiB; the
  commit that stages it does (T34-2's breakage-1). The fix is `--enforce-all` on that
  hook or a size check of its own, a hook change in `.pre-commit-config.yaml`, beyond
  this ticket's comment-only line there. Owner: maintainer.
- T34-1 made `test-doc-default` assert with `cargo tree … --offline` that no dependency
  enables libpachinko's `testing`. That goes beyond step 3's two-line recipe, and the
  manual manifest regex that the Verification block runs misses a multi-line
  `features` array. The recipe's assertion is the one that holds. Owner: maintainer, to
  accept the stronger recipe.
