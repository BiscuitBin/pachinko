---
id: T01
title: "GitHub repository and CI: settings, branch protection, Pages, workflows"
status: done
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

Times are UTC. The settings evidence ran against the live repository between
2026-10-23T00:03Z and 01:22Z. The workflow evidence ran at `abb9588`, the commit that
lands the workflows. The commit that adds these notes changes only this file.

### State before (step 2, 2026-10-23T00:03:59Z)

- `check` on `main` (`8b01c0c`): `["check","success",15368,...]`, run
  `https://github.com/BiscuitBin/pachinko/actions/runs/39874411413`.
- `.permissions.admin`: `true`.
- `gh repo view`: public, default branch `main`, `deleteBranchOnMerge: false`, merge,
  squash and rebase merges all allowed, `hasWikiEnabled: true`,
  `hasProjectsEnabled: true`.
- Protection: HTTP 404, `"message":"Branch not protected"`. There is no
  `protection-before.json`, so step 6 relies on the read-back alone.
- Pages: HTTP 404 `Not Found`, read after the admin check.
- Organisation: `{"members_can_create_pages":true,"plan":"free"}`.
- Private vulnerability reporting: `{"enabled":false}` (HTTP 200).
- The baseline `just check` on the untouched worktree was not run locally before the
  edits. The stub run above ran `just check` green on `8b01c0c`, the same commit.

### Dry run (step 3)

Command: `sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check --hygiene
--no-pages`, using T's script at `2283589c` copied into `ai_tmp/`. `--no-pages` was
passed so that step 5 could be authorised on its own; the script's usage says the flag
omits step 1 without renumbering the rest. It announced three changes and exited 0:

- Step 2: a protection PUT with `strict: false`, `checks: [{context: check}]`,
  `enforce_admins: false`, no reviews, no restrictions, and `allow_force_pushes` and
  `allow_deletions` both false.
- Step 4: `PUT private-vulnerability-reporting`. This settles the open point; the script
  enables it.
- Step 6: `gh repo edit --delete-branch-on-merge --enable-wiki=false
  --enable-projects=false`. Merge methods are left alone.

Step 3 (the Chromatic secret) was skipped because no token was given. Step 5 (package
access) only prints instructions. The summary line was `changed: 0 (dry run; 3 would
change)`.

`enforce_admins: false` follows L's T01 ("administrators not bound", L
`tickets/T01-github-repository.md` line 112).

### Authorisations

| Action | Authorised | Ran |
| --- | --- | --- |
| Step 4: `--apply` with the dry run's flags | 2026-10-23T01:11:43Z | `changed: 3`, exit 0 |
| Step 5: `POST repos/BiscuitBin/pachinko/pages -f build_type=workflow` | 2026-10-23T01:18:55Z | returned the Pages object, exit 0 |
| Step 10: push, open the pull request, one follow-up push | see the PR section | PR #3 |

Admin access and the `Branch not protected` message were rechecked at 00:18:58Z, just
before asking.

### State after (step 6) and claim 6

The second dry run (same flags) reported `already` for steps 2, 4 and 6 and
`changed: 0 (dry run; 0 would change)`, exit 0. Run at `abb9588` without `--no-pages`, it
also reports `1. Pages source / state: workflow / already`.

Read back at 01:11:57Z, from `ai_tmp/protection-after.json`:

```text
{"checks":[{"app_id":15368,"context":"check"}],"strict":false,"contexts":["check"],
 "enforce_admins":false,"lock_branch":false,"fp":false,"del":false,"linear":false,
 "sigs":false,"conv":false,"fork_sync":false,"reviews":null,"restr":null}
```

`block_creations` also reads `false`. Every field the PUT left out is at GitHub's
default; none blocks a merge.

- Private vulnerability reporting: `{"enabled":true}`.
- `gh repo view`: `deleteBranchOnMerge: true`, `hasWikiEnabled: false`,
  `hasProjectsEnabled: false`. Merge, squash and rebase are still allowed and issues are
  still on.
- There is no protection diff, because the before state was a 404. Every changed field
  is one the dry run announced: the protection object was created, vulnerability
  reporting went from false to true, and hygiene went from `false true true` to
  `true false false`.
- The required check's `app_id` is `15368`, GitHub Actions. That is the app the step 1
  `check` reported (`.app.id` = `15368`). T's PUT names no `app_id`, and GitHub bound
  the app that had last reported `check`.
- Pages read back: `build_type: workflow`, `html_url:
  https://biscuitbin.github.io/pachinko/`, `public: true`, `https_enforced: true`. The
  `github-pages` environment's deployment branch policy is custom, allowing `main` only.

Claim 6 holds. T's script works on an organisation-owned repository with repository
admin rights, and Pages works on the free organisation plan for a public repository.
The organisation blocked nothing, and no organisation-owner action was needed.

### Claim 5: B's git dependency with no credential

Stub run `39874411413` (`8b01c0c`), job `check`, from `ai_tmp/stub-run.log`:

- The checkout ran with `persist-credentials: false`. Its last line before the setup
  action is `Removing credentials config
  '/home/runner/work/_temp/git-credentials-776068f5-...config'` (23:50:45.75Z).
- The setup step that follows passes no token to setup-pixi; its only `env:` is the
  four cargo variables. `pixi.lock` pins
  `git+https://github.com/steven-cutting/biscuit_games_tooling?tag=v0.3.0#6c5c07f6bec86e86b3930dfa41392e4b440e8c85`,
  with no credential in the URL.
- `pixi install --locked` printed `✔ The default environment has been installed.`
  (23:50:49.83Z). `pixi list --locked` then lists `biscuit_games_tooling 0.3.0 pypi`.
  Its Source column reads `https://pypi.org/simple`; that is how pixi displays the
  column, and the lock line above is authoritative.

The full run's evidence is in the PR section.

### Workflows (step 7)

- `ci.yml`: L's, with three changes. The `wasm` job is deleted, and `check.needs` is
  `[rust, coverage, deny, documents]`. `- run: just effects` follows `- run: just
  metrics`. The comment above `jobs:` was rewritten so it stays true: "crate" became
  "crates", "five gate jobs" became "four", and the sentences about windows-latest,
  `crates/pawdoku-cli` and maturin's wheel matrix were dropped. The maintainer approved
  the rewrite as part of the `wasm` delta. It goes beyond §10's literal list (review
  finding C01).
- `audit.yml` and `pages.yml`: L's, verbatim. `pages.yml` publishes `target/doc` from
  `just site`, whose root index links `libpachinko/index.html` and
  `pachinko/index.html`.
- `.github/actions/setup/action.yml`: unchanged in this ticket. T00 left it equal to L's
  except `name:`, which reads "Set up the pachinko toolchain" where L's reads
  "libpawdoku".
- Only the `documents` job runs an allium recipe (`lint`, `check-specs`,
  `analyse-specs`), and it runs `just install-allium` first. Every job's permissions
  equal L's: `contents: read` at the top level, and `pages: write` plus `id-token: write`
  only on `pages.yml`'s `deploy` job.

### Action pins (step 8), read 2026-10-23T00:16Z

Each was read with `gh api repos/<o>/<r>/git/ref/tags/<tag>`, and the annotated tag was
dereferenced through `git/tags/<sha>`. Every one equals L's pin, so none changed.

| Action | Tag | Commit |
| --- | --- | --- |
| `actions/checkout` | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `actions/upload-artifact` | v7.0.1 | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` |
| `actions/upload-pages-artifact` | v5.0.0 | `fc324d3547104276b827a68afc52ff2a11cc49c9` |
| `actions/deploy-pages` | v5.0.1 | `368f82528645a54fb793d4d04e342629a3f51346` |
| `prefix-dev/setup-pixi` | v0.10.2 | `d3f436a425481402e6a95a1d1fc10331c708cd9e` |
| `Swatinem/rust-cache` | v2.9.2 (annotated `63fed3e2…`) | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` |
| `actions/cache` | v6.1.0 | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` |

No pinned tag is withdrawn. Newer releases exist (upload-artifact v7.0.2, setup-pixi
v0.11.2, rust-cache v2.9.3); proposing them is Renovate's job (T17).

### Verification at `abb9588` (2026-10-23T01:21:59Z)

```text
$ gh api .../protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
[{"app_id":15368,"context":"check"}]                                  exit 0
$ gh api repos/BiscuitBin/pachinko/pages --jq .build_type
workflow                                                              exit 0
$ gh repo view ... --json deleteBranchOnMerge,mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed
{"deleteBranchOnMerge":true,"mergeCommitAllowed":true,"rebaseMergeAllowed":true,"squashMergeAllowed":true}
$ sh ai_tmp/bootstrap_repo.sh BiscuitBin/pachinko --checks check --hygiene
changed: 0 (dry run; 0 would change)                                  exit 0
$ rg -n 'uses: ' .github > ai_tmp/uses.txt
exit 0, 19 lines
$ rg -v 'uses: \./' ai_tmp/uses.txt | rg -v '@[0-9a-f]{40} # '
(nothing)                                                             exit 1
$ just lint   (run standalone on the identical uncommitted tree; also inside just check)
... Lint GitHub Actions workflow files....Passed                      exit 0
$ just check
All checks passed and the worktree is unchanged.                      exit 0
```

`git status --short` printed nothing after `just check`. `git fetch origin` showed
`origin/main` = `8b01c0c`, the branch's base, so no rebase was needed.

### Pull request (steps 10 and 11)

Push and PR authorised at 2026-10-23T01:27:58Z, covering the push, `gh pr create` and one
follow-up evidence commit to this branch. PR: `https://github.com/BiscuitBin/pachinko/pull/3`,
head `bd624d0`. GitHub tested the merge commit `ea27892` (`HEAD is now at ea27892 Merge
bd624d0... into 8b01c0c...` in each job's checkout).

```text
$ gh pr checks 3 --repo BiscuitBin/pachinko
audit      pass  30s  .../actions/runs/39884148258/job/120188268856
check      pass   3s  .../actions/runs/39884148193/job/120188461816
coverage   pass  34s  .../actions/runs/39884148193/job/120188268921
deny       pass  33s  .../actions/runs/39884148193/job/120188269241
documents  pass  37s  .../actions/runs/39884148193/job/120188269280
rust       pass  29s  .../actions/runs/39884148193/job/120188269392
exit 0
$ gh pr checks 3 --repo BiscuitBin/pachinko --required
check      pass   3s  .../actions/runs/39884148193/job/120188461816
exit 0
```

- CI run `https://github.com/BiscuitBin/pachinko/actions/runs/39884148193`; Audit run
  `https://github.com/BiscuitBin/pachinko/actions/runs/39884148258`, which ran and is not
  required.
- The `rust` job's log shows every step: `check-toolchain`, `lock-check`, `fmt-check`,
  `toml-check`, `clippy`, `metrics`, `effects`, `features`, `test`, `snapshots-check`,
  `doc`, `deps-unused`. `documents` runs `install-allium` before `lint`, `check-docs`,
  `check-agents`, `check-specs` and `analyse-specs`. `coverage` printed `TOTAL ... 100.00%`
  lines under `--fail-under-lines 90`.
- The jobs are short, about 30s each, because they restore the caches `main`'s stub run
  wrote: `Cache hit for: pixi-linux-64-d9e27b6b...`, plus the tools and prek keys.

Claim 5, the full run (job `rust`): `Removing credentials config
'/home/runner/work/_temp/git-credentials-a3429715-...config'` (01:28:08.59Z), then
`✔ The default environment has been installed.` (01:28:11.59Z). The pixi environment
came from the cache here, so B was not fetched again in this run. The stub run above,
which had a `Cache miss`, is the run where B's git dependency was fetched and installed
with no credential. Claim 5 holds.

### Pages after the merge (step 12)

The maintainer merged PR #3 as `49890ae`. Checked at 2026-10-23T02:44Z:

- The `pages.yml` run on `main` was
  `https://github.com/BiscuitBin/pachinko/actions/runs/39891100019` (push, `49890ae`). Its
  `build` and `deploy` jobs both concluded `success`, and the `github-pages` deployment
  is at `49890ae`, created 02:43:52Z. CI on `49890ae` also passed.
- Pages URL: `https://biscuitbin.github.io/pachinko/` (`build_type: workflow`). The root
  index returns 200 and holds exactly the `just site` index, linking
  `libpachinko/index.html` and `pachinko/index.html`. Both return 200, with titles
  `libpachinko - Rust` and `pachinko - Rust`.
- The required check is still `[{"app_id":15368,"context":"check"}]`.

### Review records (CONVENTIONS.md §14)

The harness for both checkpoints was codex-cli 0.163.0, `codex exec -m gpt-6-astra -s
read-only`: one conformance reviewer and one breakage reviewer, then a separate Claude
Code subagent (Opus 5.5) for triage. Nothing was applied to a file at either checkpoint.

**Checkpoint 1, the dry run, before step 4** (live state as of 00:18Z; no commit
involved):

| id | Verdict | Reason |
| --- | --- | --- |
| C1 = B01 | valid, medium | The PUT names no `app_id` and the "already" compare reads names only. Action: step 6 read `app_id` (`15368`). |
| B02 (and C2's `lock_branch` half) | valid, low | The compare ignores `lock_branch` and the other fields outside the body. Protection was absent, so this apply was unaffected. Action: step 6 asserted each of them false. |
| B03 | valid, low | Any 404 reads as absence. Action: rechecked admin and `Branch not protected` before apply; required vulnerability reporting `enabled: true` after. |
| C2, C3, C4 = B04 | valid, low | The request quoted `--checks check ... --no-pages --apply` and the three changes, and said no organisation-owner action was needed. |
| B05 | invalid | Both the stub and the new `ci.yml` produce `check`. The new jobs gate only this PR (steps 9 to 11). |

**Checkpoint 2, the workflows, before step 10** (`abb9588` with the diff still
uncommitted when reviewed; the committed diff is identical):

| id | Verdict | Reason |
| --- | --- | --- |
| C01 | valid, low; no change | The rewrite of the comment above `jobs:` goes beyond §10's list. It was maintainer-approved and is recorded above. The triage also asked that the `action.yml` `name:` difference be named, and it is. |
| B01 | valid, medium; handed back | No workflow runs `check-clean`, so the split jobs cannot catch a gate that mutates the worktree, which `just check` would. The gap is inherited from L. Fixing it is a delta beyond §10, so it is handed back (Open points). |

No valid finding was high, so neither checkpoint needed a second round. After
checkpoint 2, `just check` was rerun at `abb9588`, as above.

## Open points

- Whether administrators are included in protection (`enforce_admins`). L's choice is in
  L's T01 hand-back notes; follow it unless the maintainer says otherwise.
- Private vulnerability reporting: enable it (it is free for public repositories) if
  the script does not; `SECURITY.md` (T12) points at it.
- If the free organisation plan blocks Pages for this repository, `pages.yml` stays and
  is disabled with `if: false` and a comment, and the maintainer decides.
- CI does not run `check-clean`, so a gate job that rewrites a tracked file passes
  (review finding T01-2 B01). L's `ci.yml` has the same gap; T00's stub, which ran
  `just check`, did not. The fix, for example a final `just check-clean` step in each
  gate job, is a delta beyond `CONVENTIONS.md` §10, so it is the maintainer's to decide
  as a follow-up.

The first three were answered in this ticket's own notes: `enforce_admins` is `false`,
following L (Dry run); step 4's apply enabled private vulnerability reporting
(`{"enabled":true}`, State after), and T12's `SECURITY.md` links to it; Pages works on
the free plan and deployed `49890ae` (step 12), so no `if: false` was needed. T11
confirmed all three on `main`. The rest:

- `check-clean` in CI: closed on 2026-10-24 on branch `review-and-finish-bootstrap`
  (commit `b0c931f`), by the maintainer's choice. Each `ci.yml` gate job that runs
  recipes ends with `just check-clean`.
- T's `bootstrap_repo.sh` defects (checkpoint 1, C1 and B02): still open. They close
  when the maintainer authorises an upstream report, or declines one.
- The newer action releases (Action pins): still open. They sit in Renovate's
  `github actions` group on Dependency Dashboard #19, under Pending Approval, and close
  when the maintainer approves that entry and merges its pull request.
