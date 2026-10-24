---
id: T15
title: "Mutation testing: cargo-mutants as a weekly report"
status: done
depends_on: [T11]
parallel_with: [T12, T13, T14, T16]
branch: ticket/t15-mutation-testing
estimated_size: S
---

# T15: Mutation testing: cargo-mutants as a weekly report

## Context

L's spike S03 measured cargo-mutants on L's crate and drafted the build ticket that
adopts it: L `tickets/S03-bench-fuzz-mutants.md` lines 553-687, with the evidence above it
from line 154. This ticket is that draft, adapted to two crates. Read the draft first; it
gives the recipe text, the workflow shape, the doc changes and the reasons, and this
ticket does not restate them.

What differs: the workspace has two crates and three tiny items, so the first run is
seconds and nearly every mutant is of `version()`, `run()` or `main`; the decision
record is 0017, shared with T16 (`CONVENTIONS.md` §8); and wave-2 rules apply
(`CONVENTIONS.md` §11).

## Goal

`just mutants` runs cargo-mutants over the workspace on a copy of the tree and reports
every mutant caught or unviable; a scheduled workflow, `mutants.yml`, runs it weekly and
by hand and uploads `mutants.out/`; it is a report and never a gate; and the
quality-philosophy page says why.

## Non-goals

- No score threshold, no gate, no `--in-diff` on pull requests: the draft's non-goals.
- No `mutants` crate dependency; an exclusion goes in `.cargo/mutants.toml`, created only
  when one is needed.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `pyproject.toml` | shared, own lines | `cargo-mutants`, the draft's pin re-verified |
| `pixi.lock` | shared, regenerated | `pixi lock` |
| `Justfile` | shared, own lines | the draft's `mutants` recipe |
| `.github/workflows/mutants.yml` | new | the draft's, never in `check`'s `needs` |
| `.gitignore` | shared, own lines | the `mutants.out*/` comment names T15 as done |
| `docs/explanation/quality-philosophy.md` | shared, own lines | the paragraph the draft gives |
| `docs/reference/testing.md` | shared, own lines | "Mutation testing" |
| `docs/reference/quality-gates.md` | shared, own lines | among the checks outside the gate |
| `docs/reference/commands.md` | shared, own lines | the `mutants` row |
| `docs/operations/maintenance.md` | shared, own lines | read the weekly run |
| `docs/decisions/0017-mutation-bench-and-fuzz.md` | new or shared | the mutation section (`CONVENTIONS.md` §8) |
| `docs/manifest.yml` | shared, own lines | 0017's entry, if this ticket creates it |
| `docs/README.md` | shared, own lines | 0017's link, if this ticket creates it |
| `docs/decisions/README.md` | shared, own lines | 0017's row, if this ticket creates it |
| `CHANGELOG.md` | shared, own lines | one `[Unreleased]` line |
| `tickets/T15-mutation-testing.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged; rebase on any wave-2 ticket merged since.
   If T16 has merged, 0017 exists: add this ticket's section to it; otherwise create it.
2. Re-verify the draft's cargo-mutants pin against conda-forge on the day, and that its
   conda-forge build depends on no `rust` (`CONVENTIONS.md` §13).
3. Apply the draft's Files touched, with `crates/pawdoku` read as the workspace and L's
   names as this repository's.
4. `just mutants`; quote the summary. Every mutant must be caught or unviable; a missed
   one is a missing test, added in this ticket, in the crate it belongs to.
5. `just check`, the final adversarial review, `just check` again if fixes landed, then
   **authorisation required: push and open a pull request**. After the merge, a
   `workflow_dispatch` of `mutants.yml` (separately authorised); download and read the
   artefact.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff and the mutant summary,
   before the push. Ask: which missed mutant was explained away rather than tested;
   whether the cargo-mutants pin is the conda-forge build step 2 verified, with no `rust`
   dependency.

## Acceptance criteria

- `just mutants` output is quoted, with no missed mutant.
- `mutants.yml` is not in `check`'s `needs`, and branch protection still names `check`
  alone.
- The worktree after `just mutants` differs only by `mutants.out/`, which Git ignores.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just mutants
git status --short
gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
just check
```

## Hand-back notes

Implemented on 2026-10-23 Pacific (2026-10-24 UTC), starting at `2c93ae3`, after
T11 and T14 merged. Switched from the supplied `ticket/T15-mutation-testing` branch
to the existing `ticket/t15-mutation-testing` branch at the same baseline commit,
matching the ticket's branch field. No unrelated work was present.

### Baseline

Commit `2c93ae3`, untouched worktree, exit 0:

```text
$ just check
All checks passed and the worktree is unchanged.
```

The initial attempt lacked the tool environment. Ran `just initialize` as authorised
by §11, with rustup's cargo first on PATH, and granted supported recipes access to
existing tool caches after sandbox denials. Initialization skipped hook installation
in this secondary worktree. No gate or hook was weakened.

### Verified pin

Commit `2c93ae3`, before edits, both commands exit 0, on 2026-10-23 Pacific:

```text
$ pixi search cargo-mutants --platform linux-64
cargo-mutants-27.1.0-hf19af3b_0 (+ 1 build)
Dependencies: libgcc >=15, __glibc >=2.17,<3.0.a0
$ pixi search cargo-mutants --platform osx-arm64
cargo-mutants-27.1.0-hc2d82ec_0 (+ 1 build)
Dependencies: __osx >=11.0
```

Neither depends on Rust. `just lock` regenerated `pixi.lock`; the diff adds only those
two package records and their environment entries, with the verified build hashes.
`just sync` installed the tool. No crate dependency or exclusion was added.

### Mutation and gate evidence

Commit `2c93ae3` with uncommitted T15 changes, all commands exit 0:

```text
$ just mutants
Found 4 mutants to test
ok       Unmutated baseline in 0s build + 1s test
4 mutants tested in 4s: 4 caught
$ just check-docs
Validated 41 pages and 42 canonical topics.
$ just test
     Summary [   0.156s] 3 tests run: 3 passed, 0 skipped
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
$ just check
All checks passed and the worktree is unchanged.
```

The four caught changes replace `version` with two alternative strings, replace `run`
with `Ok(())`, and replace `main` with `Default::default()`. No missed mutant was
explained away and no new assertion was needed. The baseline log includes doctests;
`outcomes.json` shows `--all-features --locked` on builds and tests for both crates.
Before and after the repeat mutation run, `git diff --binary` and `git status --short`
were captured separately in ignored scratch files. Both comparisons exited 0, proving
the same deliverable diff and status. `git check-ignore mutants.out mutants.out.old`
then named both output directories, exit 0. The run took four seconds; every mutant
was caught again.

Commit `2c93ae3` with uncommitted T15 changes, after explicit user authorisation,
exit 0:

```text
$ gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
[{"app_id":15368,"context":"check"}]
```

The workflow is independent of `ci.yml`, which is unchanged. The only required check
is still `check`.

### Adversarial checkpoint 1

Commit `2c93ae3` with uncommitted T15 changes. Fresh-context Codex reviewers using
`gpt-6-astra` ran the conformance and breakage lenses in parallel. Neither found a code
defect: all four mutants failed real assertions, the pinned conda builds bring no Rust,
and the workflow remains outside the required aggregate. The breakage reviewer noted
that the initial evidence lacked before/after mutation snapshots; the repeat run and
comparisons above close that limitation. Post-merge dispatch remains future evidence.

The preferred alternate harness, Claude Code with `claude-opus-5-5`, was attempted
but automatic approval review rejected the potential third-party repository export.
Fresh Codex subagents were used under the explicit equivalence in §14. Raw reviews
and independent triage are under `ai_tmp/review/T15-1/`, never committed.
A separate fresh-context Codex `gpt-6-astra` triager verified both reviews: no
implementation findings, the snapshot limitation was valid and is now resolved, and
independent live reads and post-merge workflow evidence are correctly distinguished.
No code fix or second review round was needed; nothing was handed back as a defect.
The repository's `review-change` skill was also applied to the entire diff. The
requested `code-review` skill was unavailable in the installed skill catalog.
No Rust behaviour changed, so no new test seam or TDD cycle was needed.

### External follow-up and departures

Push, opening a pull request, and a post-merge workflow dispatch each require separate
authorisation under §11. None has occurred. There is no dispatch URL or downloaded
CI artifact yet; dispatch and artifact inspection remain post-merge follow-up.

The draft is adapted to both workspace crates and decision 0017. The conda-forge
version remains 27.1.0, with the builds reverified on this run's date. The action SHAs
match `ci.yml`. The ticket index is unchanged because this ticket's Files touched does
not include it; the changelog line uses the existing library unreleased section.

## Open points

None.
