---
title: "Maintenance"
kind: "operations"
audience: [maintainer, operator, agent]
canonical_for: [maintenance_routine]
requires: []
---

# Maintenance

There is nothing running to operate. The library runs inside whoever calls it, the service
binary is built and tested by the gate but deployed nowhere, nothing accumulates between
changes, and there is no on-call. The one thing deployed is the API reference, which a
workflow publishes on every push to `main`; see
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md). What follows is upkeep of
the pins that build and check it. The routine is the one carried from libpawdoku at
`ccdd001`, with this repository's names.

## Routine

**Weekly.** Read the week's run of the `audit` workflow. It runs `just audit` against the
RustSec advisory database every Monday, as well as on every pull request, and it is never
a required check, so a failure there blocks nothing and is easy to miss. An advisory
against a crate in `Cargo.lock` is fixed by moving that crate, not by ignoring it.

Read the week's `mutants` workflow too. Download its `mutants` artifact and inspect
`outcomes.json`, the missed-mutant list and the individual logs in `mutants.out/`.
Use [Testing](../reference/testing.md#mutation-testing) to interpret the outcomes and
[Quality philosophy](../explanation/quality-philosophy.md) to resolve survivors.
The report runs Monday at 07:00 UTC and by hand. If its mutation step takes more than
fifteen minutes, raise the workflow's thirty-minute timeout before it becomes a limit.

**Monthly.** Move the dependency pins, one kind at a time, as
[Maintain dependencies](../how-to/maintain-dependencies.md) describes. `just lock-upgrade`
moves both lockfiles within the manifests' constraints — `Cargo.lock` within the caret
ranges, `pixi.lock` beneath the exact tool pins; run `just check` and commit both. A tool
pin itself moves in `pyproject.toml`, then `pixi update <name>`, then `pixi.lock` is
committed. A line in `tools.txt` or `tools-source.txt` moves by hand and takes effect at
the next `just install-tools`; a `tools-source.txt` line moves one release at a time,
with `just check` run on each, because rustqual is a gate. A hook `rev` or a GitHub
Action SHA moves by hand, with its version comment moved beside it.

**Monthly.** Run `just check-links-online`. It is not part of the gate because it needs
the network, so external links rot silently until someone looks.

**Per toolchain release.** Bump `channel` in `rust-toolchain.toml` and `rust-version` in
the root `Cargo.toml` together, run `just install-toolchain`, then run `just clippy`
before anything else: clippy's lint set moves with the release, and a lint that is new
under `-D warnings` fails the gate with no change to the code. `just check` follows.

**Per Allium release.** The checker's version and its checksums live in the
`biscuit-games-tooling` package, not here; moving that package's pin in `pyproject.toml`
moves the checker, and `just install-allium` installs it, as
[Moving the Allium binary](../how-to/maintain-dependencies.md#moving-the-allium-binary)
describes. A moved checker re-verifies every module under `docs/specs/`. The modules carry
no waiver today; any waiver added later is valid only against the version it was verified
on, so each is checked again.

Nothing here moves on its own: no bot opens a pull request for any of these pins until
ticket T17 brings Renovate.

## Cut a release

Each crate is released independently. [Decision 0016](../decisions/0016-release-process.md)
records the mechanism; `CHANGELOG.md` owns its release notes. Neither crate goes to
crates.io, and the GitHub Release has no binary assets. Only `main` is supported, with
no backports, as [SECURITY.md](../../SECURITY.md) states.

Before the workflow merges, separately authorize creation of the GitHub environment
`release`, with the maintainer as its one required reviewer. Read its protection rules
back and record them in the pull request. Without this, GitHub would create an
unprotected environment on the first run. No real release is authorized by implementing
the release process.

The following example prepares a library patch. Substitute `pachinko` for a service
release and choose the next version deliberately. For the first release the maintainer
may instead tag the existing 0.1.0 without a bump; omit `patch` during preparation.

1. Locally update `main` with `git pull --ff-only` and create
   `release/libpachinko-v0.1.1`. Write the release notes under
   `## libpachinko [Unreleased][libpachinko-unreleased]`, leaving the other crate's section alone.
   Commit the notes locally with `git add CHANGELOG.md` and
   `git commit -m "Prepare libpachinko release notes"`. Confirm `git status --short`
   prints nothing before continuing: cargo-release requires a clean worktree.
2. Run `just check` and `just semver`. Before the first library tag, semver compares
   this branch with main's merge base; after it, with the last reachable library tag.
   `just semver <revision>` chooses a specific baseline. This report is advisory on
   pull requests; before libpachinko 1.0, make it a required job in `check`'s needs.
3. Run `cargo release patch -p libpachinko` for the dry run. Then run
   `cargo release patch -p libpachinko --no-tag --execute`. This bumps the selected
   crate, rewrites only its changelog section and commits locally; it does not publish
   or push. Review the whole diff, including any dependent constraint update and the
   lockfile. After a library version bump, update the explicit version assertions in
   the unit test and doctest in `crates/libpachinko/src/lib.rs`, the service unit test
   in `crates/pachinko/src/lib.rs`, and the binary test in
   `crates/pachinko/tests/binary.rs`. The service prints the library version, so its
   output assertions follow a library bump even when the service version stays the
   same; a service-only bump does not change those expectations. Commit these assertion
   updates on the release branch before running `just check` on the resulting commit
   and opening the release pull request.
4. Separately authorize the branch push and opening the pull request. Merge only after
   the required `check` passes. The release commit must reach main through that pull
   request. No tag is made on the release branch.
5. Locally switch to `main` and run `git pull --ff-only`. Compare `date -u +%F` with
   the date in `## libpachinko [0.1.1][libpachinko-0.1.1] - YYYY-MM-DD`. It must be today's
   UTC date, and the notes must be non-empty. If the day changed, merge a one-line
   re-dating pull request first, then pull main again. Repeat this check if the UTC
   day changes before the tag push. Before the first tag, review `SECURITY.md`, the
   changelog and the README for claims the release would make false.
6. Separately authorize the real local tag. Run
   `cargo release tag -p libpachinko --execute` on the updated main merge commit.
   Confirm `git cat-file -t libpachinko-v0.1.1` prints `tag` and
   `git rev-parse libpachinko-v0.1.1^{commit} HEAD` prints the same commit twice.
7. Separately authorize `git push origin libpachinko-v0.1.1`. Push that one tag.
   CI's read-only verify job runs setup, installs Allium, runs `just check`, and
   checks the tag's commit, crate version, ancestry on main and changelog section.
   The date guard accepts any correctly shaped date so reruns work on later days;
   the local check above is what enforces the UTC push day.
8. Review the waiting deployment in the `release` environment. Approving that
   deployment is a separate authorization to create the GitHub Release. The privileged
   job runs no setup or build tools, fetches the tag again, refuses a changed commit,
   and creates the Release from that crate's changelog section with `--verify-tag`.
   A rerun after a Release already exists fails rather than rewriting it silently;
   inspect the existing Release before choosing any further action.

## Secrets

None are stored, and there is nothing to rotate. Continuous integration uses the token
GitHub mints for each run and discards with it; the Pages deployment's write scopes come
from that token too. Both crates are `publish = false`, so there is no registry token
either.

## Related pages

- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md)
- [Troubleshooting](troubleshooting.md)
- [Security model](../explanation/security-model.md)
