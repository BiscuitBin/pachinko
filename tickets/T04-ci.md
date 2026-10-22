---
id: T04
title: "CI workflows: the setup action, the gate jobs, `check`, audit and Pages"
status: open
depends_on: [T00, T01]
parallel_with: [T02, T03, T05, T06, T07, T08, T09, T10]
branch: ticket/t04-ci
estimated_size: M
---

# T04: CI workflows: the setup action, the gate jobs, `check`, audit and Pages

## Context

T00 left a one-job `ci.yml` named `check` running `just check`, and T01 made `check` the
one required status check on `main` and set the Pages source to GitHub Actions. This
lane writes the real workflows: L's, at `ccdd001`, with the deltas `CONVENTIONS.md` §10
names.

L `tickets/T04-ci.md` is the procedure, and its hand-back notes record what L's runs
proved about pixi caching, the prek cache and rust-cache. L
`tickets/T33-api-reference-on-github-pages.md` is the procedure for `pages.yml`. This lane
re-proves only what moves with the organisation and the repository: B's git dependency
installing with no credential (`CONVENTIONS.md` §12 claim 5), the action pins on the day,
and Pages on the free organisation plan.

## Goal

A pull request on `BiscuitBin/pachinko` runs the gate jobs and the aggregate `check`,
all green; `audit.yml` runs and is not required; a push to `main` deploys both crates'
rustdoc to Pages; and every remote `uses:` is pinned to a full SHA with a version comment read
on the day.

## Non-goals

- No change to the required check's name or to branch protection.
- No container, release, mutants or fuzz workflow (wave 2).
- No `Justfile` change; a recipe CI needs differently is a T00 follow-up.
- No secret. `GITHUB_TOKEN` is the run's own.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `.github/actions/setup/action.yml` | final | L's composite action |
| `.github/workflows/ci.yml` | final | L's jobs minus `wasm`, `effects` added to `rust` |
| `.github/workflows/audit.yml` | new | L's |
| `.github/workflows/pages.yml` | new | L's, publishing `just site` |
| `tickets/T04-ci.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 and T01 have merged.
2. Copy L `.github/actions/setup/action.yml`, L `.github/workflows/ci.yml`, L
   `.github/workflows/audit.yml` and L `.github/workflows/pages.yml`. Apply
   `CONVENTIONS.md` §10: delete the `wasm` job and its entry in `check`'s `needs`; add
   `- run: just effects` after `- run: just metrics` in `rust`; change nothing else.
3. Re-read every action's tag SHA (`gh api repos/<owner>/<repo>/git/ref/tags/<tag>`,
   dereferencing annotated tags). Keep L's versions unless one is withdrawn; a newer
   version is Renovate's to propose (T17).
4. `just lint` (actionlint runs over the workflows), then `just check`.

   **Adversarial review** (`CONVENTIONS.md` §14): the workflows and the setup action,
   before step 5 pushes them. Ask: which delta from L goes beyond `CONVENTIONS.md` §10;
   whether every SHA is its tag's commit, with annotated tags dereferenced; whether
   `check`'s `needs` lists every gate job and nothing optional; whether every job that
   runs an allium recipe runs `just install-allium` first; whether any job's permissions
   exceed L's.
5. **Authorisation required: push and open a pull request.** Stop and ask. On approval,
   push the branch and open a pull request against `main`. Wait for the run.
6. Read the result: every gate job and `check` green; `gh pr checks` lists `check`;
   `audit` ran and is not required. Read the setup step's log for B's install and quote
   the line that proves no credential was used (`CONVENTIONS.md` §12 claim 5).
7. After the merge, which the maintainer performs, read the `pages.yml` run on `main` and
   fetch the Pages URL it reports; both crates' pages load from the root index.
8. Rerun the pull request's workflow once and confirm the pixi, prek and rust caches
   restore (L's T04 hand-back notes name the log lines to look for).

## Acceptance criteria

- The pull request's checks are green and `check` is the required one.
- `pages.yml` deployed on `main`, and the root index links to `libpachinko` and `pachinko`.
- Every remote `uses:` names a 40-character SHA with a version comment; the local
  `uses: ./.github/actions/setup` has none.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just lint
gh pr checks <number> --repo BiscuitBin/pachinko
gh run list --repo BiscuitBin/pachinko --workflow pages.yml --limit 1
gh api repos/BiscuitBin/pachinko/pages --jq .html_url
rg -n 'uses: ' .github | rg -v 'uses: \./' | rg -v '@[0-9a-f]{40} # '
just check
```

The `rg` pipeline must print nothing.

## Hand-back notes

To be written by the executing agent: the pins and the day they were read, the run URLs,
the outcome of `CONVENTIONS.md` §12 claim 5, the cache evidence, the Pages URL, and the
review record for each checkpoint (`CONVENTIONS.md` §14).

## Open points

- If the free organisation plan blocks Pages for this repository, `pages.yml` stays and
  is disabled with `if: false` and a comment, and the maintainer decides.
