---
id: T14
title: "Release process: cargo-release, per-crate tags, a GitHub Release, semver checks"
status: open
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

To be written by the executing agent: the pins and the day, the CHANGELOG shape chosen
and why, the scratch-clone transcript, the outcome of `CONVENTIONS.md` §12 claim 7, the
review record for each checkpoint (`CONVENTIONS.md` §14), and where this ticket departed
from L's draft.

## Open points

- The first version. The draft cut `0.1.0`; both crates are `0.1.0` in their manifests
  already, so the first tag is `v0.1.0` of each with no bump, or a bump to `0.1.1`.
  The maintainer decides when the first release is cut.
