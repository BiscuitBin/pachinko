---
id: T01
title: "GitHub repository and CI: settings, branch protection, Pages, workflows"
status: open
depends_on: [T00]
parallel_with: [T02, T05, T06, T07, T10]
branch: ticket/t01-github-and-ci
estimated_size: M
---

# T01: GitHub repository and CI: settings, branch protection, Pages, workflows

## Context

`BiscuitBin/pachinko` already exists: public, on the free organisation plan, with
`origin` set and one `first commit` on `main`. Unlike L's T01, nothing is created and
there is no first push to make here; T00 reaches `main` through its own pull request.
What is missing is everything a file cannot carry: branch protection with `check`
required, merge hygiene, and the Pages source. Steps 1 to 6 set those.

L `tickets/T01-github-repository.md` is the procedure for the settings: dry run, then
apply, each apply separately authorised. T `scripts/bootstrap_repo.sh` does the work
through `gh api`; read its usage block and its step list before running it. Its default
`--checks` names the games' three checks; this repository passes exactly `--checks check`
(`CONVENTIONS.md` §10).

`BiscuitBin` is an organisation. Branch protection and the Pages source may need an
organisation owner rather than a repository admin (`CONVENTIONS.md` §13), and the
organisation may restrict Pages. This ticket finds out before applying anything.

T00 left a one-job `ci.yml` named `check` running `just check`. Once steps 1 to 6 have
made `check` the one required status check on `main` and set the Pages source to GitHub
Actions, steps 7 to 12 write the real workflows: L's, at `ccdd001`, with the deltas
`CONVENTIONS.md` §10 names.

L `tickets/T04-ci.md` is the procedure for the workflows, and its hand-back notes record
what L's runs proved about pixi caching, the prek cache and rust-cache. L
`tickets/T33-api-reference-on-github-pages.md` is the procedure for `pages.yml`. This lane
re-proves only what moves with the organisation and the repository: B's git dependency
installing with no credential (`CONVENTIONS.md` §12 claim 5), the action pins on the day,
and Pages on the free organisation plan.

## Goal

`main` on `BiscuitBin/pachinko` is protected with `check` the one required status check,
the merge-hygiene settings T's script applies are in force, and Pages builds from GitHub
Actions, each change authorised before it was made and each read back afterwards. A pull
request runs the gate jobs and the aggregate `check`, all green; `audit.yml` runs and is
not required; a push to `main` deploys both crates' rustdoc to Pages; and every remote
`uses:` is pinned to a full SHA with a version comment read on the day.

## Non-goals

- No repository file changes beyond the setup action, the workflows and this ticket's
  `status:` and hand-back notes.
- No required check but `check`, and no change to its name (`CONVENTIONS.md` §10).
- No secret and no GitHub App. `GITHUB_TOKEN` is the run's own.
- No change to `BiscuitBin`'s organisation settings. If one is needed, it is handed back
  to the maintainer as a named setting with the reason.
- No edit to T's script. A flag it lacks is recorded, not patched.
- No container, release, mutants or fuzz workflow (wave 2).
- No `Justfile` change; a recipe CI needs differently is a T00 follow-up.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `.github/actions/setup/action.yml` | final | L's composite action |
| `.github/workflows/ci.yml` | final | L's jobs minus `wasm`, `effects` added to `rust` |
| `.github/workflows/audit.yml` | new | L's |
| `.github/workflows/pages.yml` | new | L's, publishing `just site` |
| `tickets/T01-github-and-ci.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged to `main`, so that the `check` job exists on
   `main` and its name can be required. Confirm it exists and passed with
   `gh api repos/BiscuitBin/pachinko/commits/main/check-runs --jq '.check_runs[] | [.name, .conclusion]'`.
   That run is the first time B's git dependency installed on a `BiscuitBin` runner
   (`CONVENTIONS.md` §12 claim 5); quote its setup step's log. If `check` did not
   conclude `success`, stop: requiring a red check blocks every lane (`CONVENTIONS.md`
   §13). Hand the failure back as a T00 follow-up.
2. Read the current state, read-only, and quote it. First confirm
   `gh api repos/BiscuitBin/pachinko --jq .permissions.admin` prints `true`; if not, stop,
   because GitHub answers 404 to a token that may not read protection. Then
   `gh repo view BiscuitBin/pachinko
   --json visibility,defaultBranchRef,deleteBranchOnMerge,mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed`,
   `gh api repos/BiscuitBin/pachinko/branches/main/protection | jq -S . > ai_tmp/protection-before.json`
   (only an HTTP 404 with the message `Branch not protected` means none; any other error
   stops the step), `gh api repos/BiscuitBin/pachinko/pages` (only a 404 means none,
   after the admin check), and
   `gh api orgs/BiscuitBin --jq '{plan: .plan.name, members_can_create_pages}'`.
3. Copy T `scripts/bootstrap_repo.sh` at `2283589c` into `ai_tmp/` (never run it from the
   clone) and dry-run it: `sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check
   --hygiene`. Pages is set separately in step 5, so decide from the script's usage
   whether to pass `--no-pages`; quote the whole output.

   **Adversarial review** (`CONVENTIONS.md` §14): the dry run, before step 4 asks to
   apply it. The settings steps change no repository file, so a valid finding changes
   the request, not a file, and the review goes into the authorisation request. Ask:
   whether the script will require `check` and nothing else; what in it would lock a lane
   out of merging; which setting needs an organisation owner; whether passing
   `--no-pages` or not is right, given step 5.
4. **Authorisation required: apply.** Stop and ask, showing the dry run. On approval,
   rerun with `--apply`, then rerun the dry run and confirm it reports no change
   (`CONVENTIONS.md` §12 claim 6, first half).
5. **Authorisation required: the Pages source.** Stop and ask. On approval:
   `gh api -X POST repos/BiscuitBin/pachinko/pages -f build_type=workflow`, or `-X PUT`
   if Pages already exists. Read it back with `gh api repos/BiscuitBin/pachinko/pages`
   (`CONVENTIONS.md` §12 claim 6, second half).
6. Read every setting back (step 2's commands) and quote the output. Save this
   protection response the way step 2 saved its own, as `ai_tmp/protection-after.json`,
   diff it against `ai_tmp/protection-before.json`, and list every field that changed; each
   change must be one the dry run announced. T's apply is a full PUT, so a field it
   leaves out returns to GitHub's default (a true `lock_branch` blocks every merge). If
   step 2 returned a 404, say so and rely on the read-back alone. Read the required check
   from `.checks`, not the deprecated `.contexts` (T `scripts/bootstrap_repo.sh` calls it
   deprecated), and record the `app_id` that ran step 1's `check`:
   `gh api repos/BiscuitBin/pachinko/commits/main/check-runs --jq '.check_runs[] | select(.name=="check") | .app.id'`.
   If the required `check` has a null `app_id` or another app's, hand that back; do not
   patch T's script.
7. Copy L `.github/actions/setup/action.yml`, L `.github/workflows/ci.yml`, L
   `.github/workflows/audit.yml` and L `.github/workflows/pages.yml`. Apply
   `CONVENTIONS.md` §10: delete the `wasm` job and its entry in `check`'s `needs`; add
   `- run: just effects` after `- run: just metrics` in `rust`; change nothing else.
8. Re-read every action's tag SHA (`gh api repos/<owner>/<repo>/git/ref/tags/<tag>`,
   dereferencing annotated tags). Keep L's versions unless one is withdrawn; a newer
   version is Renovate's to propose (T17).
9. `just lint` (actionlint runs over the workflows), then `just check`.

   **Adversarial review** (`CONVENTIONS.md` §14): the workflows and the setup action,
   before step 10 pushes them. Ask: which delta from L goes beyond `CONVENTIONS.md` §10;
   whether every SHA is its tag's commit, with annotated tags dereferenced; whether
   `check`'s `needs` lists every gate job and nothing optional; whether every job that
   runs an allium recipe runs `just install-allium` first; whether any job's permissions
   exceed L's.
10. **Authorisation required: push and open a pull request.** Stop and ask. On approval,
    push the branch and open a pull request against `main`. Wait for the run.
11. Read the result: every gate job and `check` green; `gh pr checks` lists `check`;
    `audit` ran and is not required. Read the setup step's log for B's install and quote
    the line that proves no credential was used (`CONVENTIONS.md` §12 claim 5).
12. After the merge, which the maintainer performs, read the `pages.yml` run on `main` and
    fetch the Pages URL it reports; both crates' pages load from the root index.

## Acceptance criteria

- `gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'`
  prints exactly one entry, `check`, whose `app_id` is GitHub Actions', as step 6
  recorded it.
- The protection diff of step 6 lists only changes the dry run announced.
- The hygiene settings the script applies read back as applied, and a second dry run
  reports no change.
- `gh api repos/BiscuitBin/pachinko/pages --jq .build_type` prints `workflow`.
- Every apply was authorised before it ran; the hand-back notes say when.
- The pull request's checks are green and `check` is the required one.
- `pages.yml` deployed on `main`, and the root index links to `libpachinko` and `pachinko`.
- Every remote `uses:` names a 40-character SHA with a version comment; the local
  `uses: ./.github/actions/setup` has none.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
gh api repos/BiscuitBin/pachinko/branches/main/protection
gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
diff ai_tmp/protection-before.json ai_tmp/protection-after.json
gh api repos/BiscuitBin/pachinko/pages
gh repo view BiscuitBin/pachinko --json deleteBranchOnMerge,mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed
sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check --hygiene
just lint
gh pr checks <number> --repo BiscuitBin/pachinko
gh run list --repo BiscuitBin/pachinko --workflow pages.yml --limit 1
gh api repos/BiscuitBin/pachinko/pages --jq .html_url
rg -n 'uses: ' .github > ai_tmp/uses.txt; echo "exit $?"; wc -l < ai_tmp/uses.txt
rg -v 'uses: \./' ai_tmp/uses.txt | rg -v '@[0-9a-f]{40} # '
just check
```

The first `rg` line must exit 0 with a count above zero; the second must print nothing.

## Hand-back notes

To be written by the executing agent: the state before, the dry run, each authorisation,
the state after and the protection diff; the required check's `app_id`; the stub run's evidence and the full run's for `CONVENTIONS.md` §12
claim 5; the outcome of claim 6, including any setting the organisation blocked and who
can change it; the action pins and the day they were read, the run URLs and the Pages
URL; and the review record for each checkpoint (`CONVENTIONS.md` §14).

## Open points

- Whether administrators are included in protection (`enforce_admins`). L's choice is in
  L's T01 hand-back notes; follow it unless the maintainer says otherwise.
- Private vulnerability reporting: enable it (it is free for public repositories) if
  the script does not; `SECURITY.md` (T12) points at it.
- If the free organisation plan blocks Pages for this repository, `pages.yml` stays and
  is disabled with `if: false` and a comment, and the maintainer decides.
