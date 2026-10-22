---
id: T15
title: "Mutation testing: cargo-mutants as a weekly report"
status: open
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
5. `just check`, then **authorisation required: push and open a pull request**. After the
   merge, a `workflow_dispatch` of `mutants.yml` (separately authorised); download and
   read the artefact.

## Acceptance criteria

- `just mutants` output is quoted, with no missed mutant.
- `mutants.yml` is not in `check`'s `needs`, and branch protection still names `check`
  alone.
- The worktree after `just mutants` differs only by `mutants.out/`, which Git ignores.
- `just check` is green.

## Verification

```sh
just mutants
git status --short
gh api repos/BiscuitBin/pachinko/branches/main/protection --jq '.required_status_checks.contexts'
just check
```

## Hand-back notes

To be written by the executing agent: the pin and the day, the mutant summary, any test
added for a missed mutant, the dispatch run URL, and where this ticket departed from L's
draft.

## Open points

None.
