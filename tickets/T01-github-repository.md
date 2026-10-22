---
id: T01
title: "GitHub repository: settings, branch protection, Pages source"
status: open
depends_on: [T00]
parallel_with: [T02, T03, T05, T06, T07, T08, T09, T10]
branch: ticket/t01-github-repository
estimated_size: S
---

# T01: GitHub repository: settings, branch protection, Pages source

## Context

`BiscuitBin/pachinko` already exists: public, on the free organisation plan, with
`origin` set and one `first commit` on `main`. Unlike L's T01, nothing is created and
there is no first push to make here; T00 reaches `main` through its own pull request.
What is missing is everything a file cannot carry: branch protection with `check`
required, merge hygiene, and the Pages source.

L `tickets/T01-github-repository.md` is the procedure: dry run, then apply, each apply
separately authorised. T `scripts/bootstrap_repo.sh` does the work through `gh api`; read
its usage block and its step list before running it. Its default `--checks` names the
games' three checks; this repository passes exactly `--checks check` (`CONVENTIONS.md`
§10).

`BiscuitBin` is an organisation. Branch protection and the Pages source may need an
organisation owner rather than a repository admin (`CONVENTIONS.md` §13), and the
organisation may restrict Pages. This ticket finds out before applying anything.

## Goal

`main` on `BiscuitBin/pachinko` is protected with `check` the one required status check,
the merge-hygiene settings T's script applies are in force, and Pages builds from GitHub
Actions, each change authorised before it was made and each read back afterwards.

## Non-goals

- No file in the repository changes except this ticket's `status:` and hand-back notes.
- No workflow (T04), no Pages deployment (T04's `pages.yml`), no secret, no GitHub App.
- No change to `BiscuitBin`'s organisation settings. If one is needed, it is handed back
  to the maintainer as a named setting with the reason.
- No edit to T's script. A flag it lacks is recorded, not patched.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/T01-github-repository.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged to `main`, so that the `check` job exists on
   `main` and its name can be required. Confirm it exists and passed with
   `gh api repos/BiscuitBin/pachinko/commits/main/check-runs --jq '.check_runs[] | [.name, .conclusion]'`.
   That run is the first time B's git dependency installed on a `BiscuitBin` runner
   (`CONVENTIONS.md` §12 claim 5); quote its setup step's log. If `check` did not
   conclude `success`, stop: requiring a red check blocks every lane (`CONVENTIONS.md`
   §13). Hand the failure back as a T00 follow-up.
2. Read the current state, read-only, and quote it: `gh repo view BiscuitBin/pachinko
   --json visibility,defaultBranchRef,deleteBranchOnMerge,mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed`,
   `gh api repos/BiscuitBin/pachinko/branches/main/protection` (a 404 means none),
   `gh api repos/BiscuitBin/pachinko/pages` (a 404 means none), and
   `gh api orgs/BiscuitBin --jq '{plan: .plan.name, members_can_create_pages}'`.
3. Copy T `scripts/bootstrap_repo.sh` at `2283589c` into `ai_tmp/` (never run it from the
   clone) and dry-run it: `sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check
   --hygiene`. Pages is set separately in step 5, so decide from the script's usage
   whether to pass `--no-pages`; quote the whole output.
4. **Authorisation required: apply.** Stop and ask, showing the dry run. On approval,
   rerun with `--apply`, then rerun the dry run and confirm it reports no change
   (`CONVENTIONS.md` §12 claim 6, first half).
5. **Authorisation required: the Pages source.** Stop and ask. On approval:
   `gh api -X POST repos/BiscuitBin/pachinko/pages -f build_type=workflow`, or `-X PUT`
   if Pages already exists. Read it back with `gh api repos/BiscuitBin/pachinko/pages`
   (`CONVENTIONS.md` §12 claim 6, second half).
6. Read every setting back (step 2's commands) and quote the output.

## Acceptance criteria

- `gh api repos/BiscuitBin/pachinko/branches/main/protection --jq '.required_status_checks.contexts'`
  prints `["check"]`.
- The hygiene settings the script applies read back as applied, and a second dry run
  reports no change.
- `gh api repos/BiscuitBin/pachinko/pages --jq .build_type` prints `workflow`.
- Every apply was authorised before it ran; the hand-back notes say when.

## Verification

```sh
gh api repos/BiscuitBin/pachinko/branches/main/protection
gh api repos/BiscuitBin/pachinko/pages
gh repo view BiscuitBin/pachinko --json deleteBranchOnMerge,mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed
sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check --hygiene
```

## Hand-back notes

To be written by the executing agent: the state before, the dry run, each authorisation,
the state after, the stub run's first-half evidence for `CONVENTIONS.md` §12 claim 5,
the outcome of claim 6, including any setting
the organisation blocked and who can change it.

## Open points

- Whether administrators are included in protection (`enforce_admins`). L's choice is in
  L's T01 hand-back notes; follow it unless the maintainer says otherwise.
- Private vulnerability reporting: enable it (it is free for public repositories) if
  the script does not; `SECURITY.md` (T12) points at it.
