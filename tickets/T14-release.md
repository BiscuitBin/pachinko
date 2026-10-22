---
id: T14
title: "Release process: cargo-release, per-crate tags, a GitHub Release, semver checks"
status: open
depends_on: [T11]
parallel_with: [T12, T13, T15, T16]
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

`cargo release` run locally bumps one crate, commits and tags it (`libpachinko-v<x.y.z>`
or `pachinko-v<x.y.z>`) without publishing or pushing; pushing that tag runs
`release.yml`, which builds, runs the gate and creates a GitHub Release with the
crate's `CHANGELOG.md` section as its body; `just semver` checks libpachinko against its
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
| `.github/workflows/release.yml` | new | on `libpachinko-v*` and `pachinko-v*` tags |
| `.github/workflows/ci.yml` | shared, own lines | a `semver` job, not in `check`'s `needs` |
| `Justfile` | shared, own lines | `semver`, and `release` if the draft's reason holds |
| `pyproject.toml` | shared, own lines | cargo-semver-checks, if conda-forge carries it |
| `pixi.lock` | shared, regenerated | `pixi lock` |
| `tools.txt` | shared, own lines | cargo-release, and cargo-semver-checks if not on conda-forge |
| `CHANGELOG.md` | shared, own lines | per-crate sections, the release discipline |
| `docs/operations/maintenance.md` | shared, own lines | "Cut a release" |
| `docs/decisions/0016-release-process.md` | new | `CONVENTIONS.md` §8 |
| `docs/manifest.yml` | shared, own lines | one entry |
| `docs/README.md` | shared, own lines | one link |
| `docs/decisions/README.md` | shared, own lines | one row |
| `docs/reference/commands.md` | shared, own lines | `semver` (and `release`) rows |
| `tickets/T14-release.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged; rebase on any wave-2 ticket merged since.
2. **Re-verify the draft's pins on the day.** cargo-release (`1.1.6` in the draft) and
   cargo-semver-checks: the newest release, its binstall support and its conda-forge
   package. The pin goes in `pyproject.toml` if conda-forge carries it, else
   `tools.txt` (`CONVENTIONS.md` §2).
3. **Configuration.** The draft's `[workspace.metadata.release]` (`publish = false`,
   `push = false`, the commit and tag messages). Per crate, `tag-prefix` so tags are
   `libpachinko-v{{version}}` and `pachinko-v{{version}}`, and the CHANGELOG replacement
   rules the draft gives. `publish.workspace = true` stays on both crates.
4. **`CHANGELOG.md` discipline.** One `CHANGELOG.md` at the root (T12 owns its shape),
   with an `[Unreleased]` subsection per crate and a release heading per crate tag
   (`## libpachinko 0.1.1`). Each crate's `pre-release-replacements` rewrites only its
   own subsection; prove it in step 7 and record it in decision 0016. If cargo-release
   cannot target one crate's subsection cleanly, stop and hand back: per-crate files are
   new paths this ticket does not list.
5. **`release.yml`.** `on: push: tags: ['libpachinko-v*', 'pachinko-v*']`;
   `permissions: contents: write` on the one job and nothing else; the composite setup
   action; `just install-allium` (the setup action leaves it out, `CONVENTIONS.md` §10);
   `just check`; then `gh release create "$GITHUB_REF_NAME" --notes-file <the
   crate's section>` with the run's `GITHUB_TOKEN`. Never in `check`'s `needs`.
6. **`semver`.** `cargo semver-checks -p libpachinko --baseline-rev <rev>`: before the
   first release the baseline is `main`'s merge base, so the recipe is a statement about
   this branch; after it, the last `libpachinko-v*` tag. Settle `CONVENTIONS.md` §12
   claim 7 by running both forms. The CI `semver` job runs it on pull requests and is not
   required, as the draft argues.
7. **The proof, in a scratch clone** under `ai_tmp/`: `cargo release patch -p libpachinko`
   dry run, then `--execute`; quote the commit, the tag and the CHANGELOG diff. Do the
   same for `pachinko`. Delete the clone.
8. **The docs.** "Cut a release" in `docs/operations/maintenance.md`: the sequence, which
   steps are local and which CI does, and which are separately authorised (the tag push).
   Decision 0016: cargo-release over release-plz (the draft's evidence), per-crate tags,
   a GitHub Release instead of crates.io, and what would reopen it.
9. `just check`, then **authorisation required: push and open a pull request**. A real
   tag push to exercise `release.yml` is a further, separate authorisation.

## Acceptance criteria

- The scratch-clone release of each crate is quoted: commit, tag name, CHANGELOG diff.
- `just semver` is green and its output quoted, in both baseline forms.
- `release.yml` passes actionlint and grants `contents: write` to its one job only.
- `just check` is green.

## Verification

```sh
just semver
cargo release --version
rg -n 'permissions' -A2 .github/workflows/release.yml
just lint
just check
```

## Hand-back notes

To be written by the executing agent: the pins and the day, the CHANGELOG shape chosen
and why, the scratch-clone transcript, the outcome of `CONVENTIONS.md` §12 claim 7, and
where this ticket departed from L's draft.

## Open points

- The first version. The draft cut `0.1.0`; both crates are `0.1.0` in their manifests
  already, so the first tag is `v0.1.0` of each with no bump, or a bump to `0.1.1`.
  The maintainer decides when the first release is cut.
