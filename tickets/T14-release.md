---
id: T14
title: "Release process: cargo-release, per-crate tags, a GitHub Release, semver checks"
status: done
depends_on: [T11, T12]
parallel_with: [T13, T15, T16]
branch: ticket/t14-release
estimated_size: M
---

# T14: Release process: cargo-release, per-crate tags, a GitHub Release, semver checks

## Context

L's spike S02 weighed cargo-release against release-plz, proved cargo-release's local
flow in a scratch clone, and drafted a build ticket for it: L
`tickets/S02-release-and-publishing.md` lines 648-866, with the evidence above it from
line 178. This ticket adapts that draft. Read the draft and its evidence in full first.

What differs here: two crates, each versioned and tagged on its own; no crates.io while
`publish = false` stays (`CONVENTIONS.md` §1 decision 5), so CI on a tag cuts a GitHub
Release instead of publishing, and needs `contents: write`; and a `semver` recipe that
runs cargo-semver-checks on libpachinko, whose public API is the one that can break a
consumer.

Wave-2 rules apply (`CONVENTIONS.md` §11).

## Goal

`cargo release <level> -p <crate> --no-tag --execute`, run on a
`release/<crate>-v<x.y.z>` branch, bumps one crate and commits it without publishing or
pushing; the release commit reaches `main` through a pull request that requires `check`;
after the merge, `git pull --ff-only` and `cargo release tag` tag the merge commit
(`libpachinko-v<x.y.z>` or `pachinko-v<x.y.z>`); pushing that one tag runs `release.yml`,
which verifies the tag with a read-only token and then, in a separate job, creates a
GitHub Release with the crate's `CHANGELOG.md` section as its body; `just semver` checks libpachinko against its
last release, or against a named baseline before there is one; and decision 0016 and the
maintenance page say how a release is cut.

## Non-goals

- No crates.io publish and no lifting of `publish = false`. The draft's trusted-publishing
  half is recorded as what would reopen decision 0016.
- No release cut by this ticket beyond the proof in a scratch clone. The first real tag is
  separately authorised and the maintainer's call.
- No binary artefact on the Release (T13's image is not published either).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `[workspace.metadata.release]` |
| `crates/libpachinko/Cargo.toml` | shared, own lines | `[package.metadata.release]`, tag prefix |
| `crates/pachinko/Cargo.toml` | shared, own lines | `[package.metadata.release]`, tag prefix |
| `.github/workflows/release.yml` | new | on `libpachinko-v<x.y.z>` and `pachinko-v<x.y.z>` tags; a `verify` job and a `publish` job |
| `.github/workflows/ci.yml` | shared, own lines | a `semver` job, not in `check`'s `needs` |
| `Justfile` | shared, own lines | `semver`, and `release` if the draft's reason holds |
| `pyproject.toml` | shared, own lines | cargo-semver-checks, if conda-forge carries it |
| `pixi.lock` | shared, regenerated | `pixi lock` |
| `tools.txt` | shared, own lines | cargo-release, and cargo-semver-checks if not on conda-forge |
| `CHANGELOG.md` | shared, own lines | per-crate sections, the release discipline |
| `docs/operations/maintenance.md` | shared, own lines | "Cut a release" |
| `docs/explanation/architecture.md` | shared, own lines | T14's wave-2 sentence (T07 step 6) made present tense, citing decision 0016 |
| `SECURITY.md` | shared, own lines | "Supported versions", once a release process exists |
| `README.md` | shared, own lines | only a sentence the first tag makes false, if step 9's review finds one |
| `docs/decisions/0016-release-process.md` | new | `CONVENTIONS.md` §8 |
| `docs/manifest.yml` | shared, own lines | one entry |
| `docs/README.md` | shared, own lines | one link |
| `docs/decisions/README.md` | shared, own lines | one row |
| `docs/reference/commands.md` | shared, own lines | `semver` (and `release`) rows |
| `tickets/T14-release.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 and T12 have merged, so that `CHANGELOG.md` and
   `SECURITY.md` hold T12's final text; rebase on any wave-2 ticket merged since.
2. **Re-verify the draft's pins on the day.** cargo-release (`1.1.6` in the draft) and
   cargo-semver-checks: the newest release, its binstall support and its conda-forge
   package. The pin goes in `pyproject.toml` if conda-forge carries it, else
   `tools.txt` (`CONVENTIONS.md` §2).
3. **Configuration.** The draft's `[workspace.metadata.release]` (`publish = false`,
   `push = false`, the commit and tag messages) and the draft's
   `allow-branch = ["main", "release/*"]`. Per crate, `tag-prefix` so tags are
   `libpachinko-v{{version}}` and `pachinko-v{{version}}`, and the CHANGELOG replacement
   rules the draft gives, with the tag name written in full in each
   `pre-release-replacements` entry, because `{{tag_name}}` is not rendered under
   `--no-tag` (L `tickets/S02-release-and-publishing.md` lines 536-540).
   `publish.workspace = true` stays on both crates.
4. **`CHANGELOG.md` discipline.** One `CHANGELOG.md` at the root, in T12's shape,
   whose single `[Unreleased]` section this ticket splits into an `[Unreleased]`
   subsection per crate, with a dated release heading per crate tag
   (`## libpachinko 0.1.1 - YYYY-MM-DD`, or a shape settled here and recorded in decision
   0016). The date is the UTC day the tag is pushed, not the day the release pull
   request was prepared; if the merge lands on a later UTC day, a one-line pull request
   re-dates the heading before tagging. Each crate's `pre-release-replacements` rewrites
   only its own subsection; prove it in step 7 and record it in decision 0016. If cargo-release
   cannot target one crate's subsection cleanly, stop and hand back: per-crate files are
   new paths this ticket does not list.

   **Adversarial review** (`CONVENTIONS.md` §14): the CHANGELOG shape and each crate's
   `pre-release-replacements`, before `release.yml` and step 7's proof depend on them.
   Ask: whether a release of one crate can rewrite the other's subsection; whether the
   shape survives a release of each crate in turn.
5. **`release.yml`.** Triggered on tags matching `libpachinko-v[0-9]+.[0-9]+.[0-9]+` and
   `pachinko-v[0-9]+.[0-9]+.[0-9]+`, not `-v*` globs. The top level sets
   `permissions: {}` and one `release` concurrency group with `cancel-in-progress: false`.
   Two jobs:
   - `verify` (`contents: read`; checkout with `fetch-depth: 0` and
     `persist-credentials: false`): the composite setup action, `just install-allium`
     (`CONVENTIONS.md` §10) and `just check`, then four guards, the middle two
     from the draft (L
     `tickets/S02-release-and-publishing.md` lines 717-725), each failing with
     `::error::`: `HEAD` is the tag's commit; the tag equals the crate's prefix plus that
     crate's version from `cargo metadata --locked --no-deps`; after
     `git fetch --no-tags origin main`, `git merge-base --is-ancestor <commit> FETCH_HEAD`
     succeeds; and `CHANGELOG.md` has exactly one line equal to the crate's release
     heading with a `YYYY-MM-DD` date and a non-empty section under it. The guard checks
     the date's shape, not that it is the run's day, so a rerun on a later day passes;
     step 8's local check keeps the date current. It outputs the commit.
   - `publish` (`needs: verify`, `environment: release`, `permissions: contents: write`
     and nothing else). It runs no setup action, pixi or `just`. It checks out with
     `persist-credentials: false`, fetches the tag again and stops unless the tag still
     points at `verify`'s commit, extracts the crate's section, and runs
     `gh release create "$TAG" --verify-tag --notes-file <that section>` with the run's
     `github.token`.

   The `release` environment, with the maintainer as required reviewer, must exist
   before this file merges, or GitHub creates it unprotected on the first run. Creating
   it is a repository setting, separately authorised at step 9 (`CONVENTIONS.md` §11).
   Never in `check`'s `needs`.
6. **`semver`.** `cargo semver-checks -p libpachinko --baseline-rev <rev>`: before the
   first release the baseline is `main`'s merge base, so the recipe is a statement about
   this branch; after it, the last `libpachinko-v*` tag. Settle `CONVENTIONS.md` §12
   claim 7 by running both forms. The CI `semver` job runs it on pull requests and is not
   required, as the draft argues.
7. **The proof, in a scratch clone** under `ai_tmp/`, on `release/libpachinko-v<next>`:
   a `cargo release patch -p libpachinko` dry run, then `--no-tag --execute`; then a
   `--no-ff` merge into the clone's `main` and `cargo release tag -p libpachinko --execute`.
   Quote the commit, the CHANGELOG diff, and `git cat-file -t <tag>` with
   `git rev-parse <tag>^{commit} HEAD`, showing an annotated tag on the merge commit. Do
   the same for `pachinko`. Delete the clone.
8. **The docs.** "Cut a release" in `docs/operations/maintenance.md`: release branch,
   `--no-tag --execute`, pull request, merge, `git pull --ff-only`, a check that the
   heading is dated today in UTC (re-date if not), `cargo release tag`, then the tag push;
   which steps are local and which CI does, and which are separately authorised (the
   `release` environment, the pull request, the tag push). The tag is made on `main`
   after the merge, never before. Before the first tag push, update `SECURITY.md`
   "Supported versions", whose "No version has been released" the tag makes false: keep
   `main` as the supported line, cite decision 0016, and promise no backports. Make
   T14's sentence in `docs/explanation/architecture.md` present tense.
   Decision 0016: cargo-release over release-plz (the draft's evidence), per-crate tags,
   a GitHub Release instead of crates.io, and what would reopen it.
9. `just check`, the final adversarial review, `just check` again if fixes landed, then
   **authorisation required: push and open a pull request**. Then **authorisation
   required: the `release` environment.** Stop and ask. On approval, create it with the
   maintainer as its one required reviewer
   (`gh api -X PUT repos/BiscuitBin/pachinko/environments/release`), read it back with
   `gh api repos/BiscuitBin/pachinko/environments/release` and quote its
   `protection_rules`. The pull request does not merge until that read-back is quoted. A
   real tag push to exercise `release.yml` is a further, separate authorisation.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff, before the push. Ask:
   whether any step that runs the setup action, pixi or `just check` can see a write
   token; whether a tag moved between `verify` and `publish` is refused; whether a tag on
   a commit not on `main`, or whose version disagrees with the manifest, is refused;
   whether its trigger can fire on anything but a crate tag; how the token is used, and
   whether it can reach a log; whether the semver baseline before and after the first
   release is the one step 6 states; whether `SECURITY.md`, `CHANGELOG.md` or
   `README.md` still says something the first tag makes false.

## Acceptance criteria

- The scratch-clone release of each crate is quoted: commit, CHANGELOG diff, and an
  annotated tag whose commit is the merge commit.
- `just semver` is green and its output quoted, in both baseline forms.
- `release.yml` passes actionlint; the top level grants nothing, `verify` grants
  `contents: read` only, and `publish` grants `contents: write` only, behind
  `environment: release`.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just semver
cargo release --version
rg -n 'permissions|environment|tags:|concurrency' -A3 .github/workflows/release.yml
just lint
just check
```

## Hand-back notes

Implemented on 2026-10-23 Pacific (2026-10-24 UTC), from `7dbed57`, after T11
and T12 merged. The existing Supacode branch is `ticket/T14-release`, kept under the
implement instruction to commit on the current branch, rather than renaming it to the
lowercase frontmatter name. `git fetch origin` confirmed `origin/main` is `7dbed57`.

### Baseline and pins

Commit `7dbed57`, untouched worktree, exit 0:

```text
$ just check
All checks passed and the worktree is unchanged.
```

The first attempt could not start because this worktree had no tools. Provisioned it
with `just initialize`, authorized by §11; it skipped hooks in the secondary worktree.
Sandbox cache denials were resolved by granting the supported recipes access to shared
caches. No gate was weakened and no shared hook was installed here.

Reverified upstream release metadata and conda-forge on the day: cargo-release 1.1.7,
published 2026-10-21, has both platform archives and installed successfully through
binstall. Neither supported platform has a conda-forge cargo-release package.
Cargo-semver-checks 0.51.0, published 2026-10-03, has upstream archives and conda-forge
packages on both platforms. The generated lock diff adds only those two packages and
their environment references. The pins are in `tools.txt` and `pyproject.toml`.

Commit `7dbed57` with uncommitted changes, exit 0:

```text
$ cargo release --version
cargo-release 1.1.7
```

### Changelog and scratch proof

The chosen headings are `## libpachinko [Unreleased][libpachinko-unreleased]` and its
service counterpart. Dated headings use explicit crate-qualified version references,
such as `## libpachinko [0.1.1][libpachinko-0.1.1] - 2026-10-24`. This keeps one root
file, usable Markdown links and exactly one match per crate. Replacements spell full
tag names, preserving the other crate and earlier releases.

A remote-free scratch clone under `ai_tmp/` held candidate commit `3484f6f`, copied from
`7dbed57` with the implementation changes. Each command below exited 0. No registry
publish, remote push or real repository tag occurred. Cargo-release's public registry
lookup required network access even with publishing disabled. Its `Publishing` display
is a phase label; inherited `publish = false` and release `publish = false` stayed set.

Scratch commit `2acaa3c`, prepared on `release/libpachinko-v0.1.1`:

```text
$ cargo release patch -p libpachinko
   Upgrading libpachinko from 0.1.0 to 0.1.1
    Updating pachinko's dependency from 0.1.0 to 0.1.1
   Replacing in ../../CHANGELOG.md
--- ../../CHANGELOG.md original
+++ ../../CHANGELOG.md replaced
@@ -11,6 +11,8 @@

 ## libpachinko [Unreleased][libpachinko-unreleased]

+## libpachinko [0.1.1][libpachinko-0.1.1] - 2026-10-24
+
 ### Added

 - An Apache-2.0 library, `libpachinko`, exposing its version. The domain awaits its
@@ -33,5 +35,6 @@
 - Independent crate releases through cargo-release and a guarded GitHub Release workflow.
   The crate is not published to crates.io.

-[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
+[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/libpachinko-v0.1.1...HEAD
+[libpachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/libpachinko-v0.1.1
 [pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/

  Publishing libpachinko
warning: aborting release due to dry run; re-run with `--execute`
$ cargo release patch -p libpachinko --no-tag --execute --no-confirm
   Upgrading libpachinko from 0.1.0 to 0.1.1
    Updating pachinko's dependency from 0.1.0 to 0.1.1
[release/libpachinko-v0.1.1 2acaa3c] Release libpachinko 0.1.1
 4 files changed, 7 insertions(+), 4 deletions(-)
  Publishing libpachinko
```

Scratch commit `2acaa3c`, changelog diff (the service section is unchanged):

```diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index ab54745..9cba00b 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -11,6 +11,8 @@ and version, dated on the UTC day its tag is pushed. The release process is desc

 ## libpachinko [Unreleased][libpachinko-unreleased]

+## libpachinko [0.1.1][libpachinko-0.1.1] - 2026-10-24
+
 ### Added

 - An Apache-2.0 library, `libpachinko`, exposing its version. The domain awaits its
@@ -33,5 +35,6 @@ and version, dated on the UTC day its tag is pushed. The release process is desc
 - Independent crate releases through cargo-release and a guarded GitHub Release workflow.
   The crate is not published to crates.io.

-[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
+[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/libpachinko-v0.1.1...HEAD
+[libpachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/libpachinko-v0.1.1
 [pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
```

Scratch merge commit `0e799ec`, all commands exit 0:

```text
$ git switch main
$ git merge --no-ff release/libpachinko-v0.1.1 -m "Merge library release proof"
$ cargo release tag -p libpachinko --execute --no-confirm
$ git cat-file -t libpachinko-v0.1.1
tag
$ git rev-parse libpachinko-v0.1.1^{commit} HEAD
0e799ec630c41b6055bf0935d3ce4e93868a90f4
0e799ec630c41b6055bf0935d3ce4e93868a90f4
```

Scratch commit `f35cda4`, prepared on `release/pachinko-v0.1.1` after the library merge:

```text
$ cargo release patch -p pachinko
   Upgrading pachinko from 0.1.0 to 0.1.1
   Replacing in ../../CHANGELOG.md
--- ../../CHANGELOG.md original
+++ ../../CHANGELOG.md replaced
@@ -29,6 +29,8 @@

 ## pachinko [Unreleased][pachinko-unreleased]

+## pachinko [0.1.1][pachinko-0.1.1] - 2026-10-24
+
 ### Added

 - An Apache-2.0 service, `pachinko`, that writes its name and the library version.
@@ -37,4 +39,5 @@

 [libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/libpachinko-v0.1.1...HEAD
 [libpachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/libpachinko-v0.1.1
-[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
+[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/pachinko-v0.1.1...HEAD
+[pachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/pachinko-v0.1.1

  Publishing pachinko
warning: aborting release due to dry run; re-run with `--execute`
$ cargo release patch -p pachinko --no-tag --execute --no-confirm
   Upgrading pachinko from 0.1.0 to 0.1.1
[release/pachinko-v0.1.1 f35cda4] Release pachinko 0.1.1
 3 files changed, 6 insertions(+), 3 deletions(-)
  Publishing pachinko
```

Scratch commit `f35cda4`, changelog diff (the library section is unchanged):

```diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 9cba00b..7668621 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -29,6 +29,8 @@ and version, dated on the UTC day its tag is pushed. The release process is desc

 ## pachinko [Unreleased][pachinko-unreleased]

+## pachinko [0.1.1][pachinko-0.1.1] - 2026-10-24
+
 ### Added

 - An Apache-2.0 service, `pachinko`, that writes its name and the library version.
@@ -37,4 +39,5 @@ and version, dated on the UTC day its tag is pushed. The release process is desc

 [libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/libpachinko-v0.1.1...HEAD
 [libpachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/libpachinko-v0.1.1
-[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
+[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/compare/pachinko-v0.1.1...HEAD
+[pachinko-0.1.1]: https://github.com/BiscuitBin/pachinko/releases/tag/pachinko-v0.1.1
```

Scratch merge commit `ac84c02`, all commands exit 0:

```text
$ git switch main
$ git merge --no-ff release/pachinko-v0.1.1 -m "Merge service release proof"
$ cargo release tag -p pachinko --execute --no-confirm
$ git cat-file -t pachinko-v0.1.1
tag
$ git rev-parse pachinko-v0.1.1^{commit} HEAD
ac84c02a2fdf53daa70167cadc7d7a9cc62a56d8
ac84c02a2fdf53daa70167cadc7d7a9cc62a56d8
```

### Semver and claim 7

Commit `7dbed57` with uncommitted changes, exit 0, before any real library tag:

```text
$ just semver
cargo update --workspace --locked --offline
     Locking 0 packages to highest Rust 1.99 compatible versions
pixi lock --check --offline --dry-run
i Dry-run: lock file would not change
semver: libpachinko baseline 7dbed57085d177530ca1b034a593527f373f1528
     Cloning 7dbed57085d177530ca1b034a593527f373f1528
    Building libpachinko v0.1.0 (current)
       Built [   0.145s] (current)
     Parsing libpachinko v0.1.0 (current)
      Parsed [   0.000s] (current)
    Building libpachinko v0.1.0 (baseline)
       Built [   0.173s] (baseline)
     Parsing libpachinko v0.1.0 (baseline)
      Parsed [   0.000s] (baseline)
    Checking libpachinko v0.1.0 -> v0.1.0 (no change; assume minor)
     Checked [   0.003s] 202 checks: 202 pass, 58 skip
     Summary no semver update required
    Finished [   0.581s] libpachinko
```

Scratch merge commit `0e799ec`, exit 0, with a reachable library release tag:

```text
$ just semver
cargo update --workspace --locked --offline
     Locking 0 packages to highest Rust 1.99 compatible versions
pixi lock --check --offline --dry-run
i Dry-run: lock file would not change
semver: libpachinko baseline libpachinko-v0.1.1
     Cloning libpachinko-v0.1.1
    Building libpachinko v0.1.1 (current)
       Built [   0.350s] (current)
     Parsing libpachinko v0.1.1 (current)
      Parsed [   0.001s] (current)
    Building libpachinko v0.1.1 (baseline)
       Built [   0.162s] (baseline)
     Parsing libpachinko v0.1.1 (baseline)
      Parsed [   0.000s] (baseline)
    Checking libpachinko v0.1.1 -> v0.1.1 (no change; assume minor)
     Checked [   0.004s] 202 checks: 202 pass, 58 skip
     Summary no semver update required
    Finished [   0.807s] libpachinko
```

Claim 7 holds: a Git revision baseline works before the first release, and discovery
selects the library tag afterwards. An explicit revision is also accepted by the
recipe. The semver job runs only on pull requests, outside `check`'s needs. The recipe
checks locks and runs offline because this semver-checks version has no locked option.

### Reviews and verification

Checkpoint 1 used native Codex fresh-context gpt-6-astra reviewers for conformance and
breakage, followed by a third independent gpt-6-astra triage agent. Conformance found
none. T14-C1-B1 (low) found that headings did not reference their crate-qualified link
definitions. Triage confirmed the finding by source inspection and the MD053 gate
failure, then fixed both manifests, the changelog and its documentation. Rerunning
`just check-docs` passed: `Validated 40 pages and 41 canonical topics.` The scratch
proof above then exercised the corrected replacements for each crate in turn.

The prescribed alternative-harness Claude review was rejected by automatic approval
review as an external transfer of the repository content. No Claude review ran. The
same-harness fresh-context alternative allowed by §14 was used, with the prescribed
model. Raw review and triage files stay under `ai_tmp/review/T14-1/`.

Checkpoint 2 used two fresh-context native Codex gpt-6-astra reviewers, followed by
an independent gpt-6-astra triage agent. Conformance found none. T14-F-B1 (also
recorded as T14-C2-B1, medium) identified hard-coded version expectations that fail
after a library bump. Independent `just test` at scratch commit `ac84c02` exited 100:
three tests ran and all three failed; the doctest was not reached. The scratch proof
above proves release mechanics, not a passing full gate on the bumped tree.
Triage fixed the in-scope documentation: Maintenance now requires updating and
committing all four assertions before the release gate, and decision 0016 records
that consequence. Version-independent Rust assertions are handed back outside this
ticket's allowed paths. Raw reports and verdicts stay in `ai_tmp/review/T14-2/`.
Neither checkpoint had a high finding, so no second round was required. The repository
`review-change` skill and these independent reviews supplied the review workflow;
no separate `code-review` skill is installed.

The scratch clone was deleted after retaining the quoted evidence and review logs.
The final gate below was rerun after the documentation fix.

Commit `7dbed57` with uncommitted changes, exit 0 for each:

```text
$ rg -n 'permissions|environment|tags:|concurrency' -A3 .github/workflows/release.yml
5:    tags:
6-      - 'libpachinko-v[0-9]+.[0-9]+.[0-9]+'
7-      - 'pachinko-v[0-9]+.[0-9]+.[0-9]+'
8-
9:permissions: {}
10-
11:concurrency:
12-  group: release
13-  cancel-in-progress: false
14-
--
19:    permissions:
20-      contents: read
21-    outputs:
22-      commit: ${{ steps.guards.outputs.commit }}
--
99:    environment: release
100:    permissions:
101-      contents: write
102-    steps:
103-      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
$ just lint
Lint GitHub Actions workflow files.......................................Passed
$ just check
All checks passed and the worktree is unchanged.
```

### Departures and authorization boundary

The L draft published to crates.io; this implementation keeps both publish prohibitions
and creates only a GitHub Release, with verification and writing in separate jobs. Pins
are newer, versions and changelog sections are independent, triggers are constrained to
stable crate tags, and pre-release semver uses main's merge base rather than skipping.
No release wrapper recipe was added: preparation and post-merge tagging are distinct
operator actions. The README contains no sentence made false by a GitHub Release;
SECURITY now keeps main supported, cites decision 0016 and promises no backports.

The first scratch run exposed a 1.1.7 difference from the draft: default commit
consolidation left the crate/version placeholders literal when updating a dependent.
Explicit `consolidate-commits = false` fixed this; both corrected proofs are quoted
above, with the rendered commit subjects. The library bump updates the service's
constraint but not its version or changelog.

Push, pull-request creation, the protected `release` environment, a real local tag,
tag push and deployment approval remain separate authorizations. The environment must
be created with the maintainer as its one required reviewer and its protection rules
read back into the pull request before merging. No repository setting was changed.
The first real version remains the maintainer's decision.

## Open points

- The first version. The draft cut `0.1.0`; both crates are `0.1.0` in their manifests
  already, so the first tag is `v0.1.0` of each with no bump, or a bump to `0.1.1`.
  The maintainer decides when the first release is cut.

- Version-independent assertions in the library unit test/doctest and both service
  output tests would remove the manual release step. This Rust cleanup is outside
  T14; until then, Maintenance explicitly requires updating them after library bumps.
