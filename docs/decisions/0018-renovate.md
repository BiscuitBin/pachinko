---
title: "Decision 0018: Dependency updates through Renovate"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_dependency_updates]
requires: []
---

# Decision 0018: Dependency updates through Renovate

## Context

Every pin here moved by hand. Libpawdoku's S01 spike, on 2026-09-26, compared Dependabot,
Renovate and no updater against the same kinds of pins this repository holds, and drafted
both configurations. It found Dependabot stronger than expected: it has `rust-toolchain`
and `pre-commit` ecosystems, and its pre-commit updater reads the `rev: <sha> # vX.Y.Z`
form. What Renovate alone reaches is the one-line pins outside any ecosystem: the
`tools.txt` lines, `rust-version` grouped with the toolchain, the tooling package's tag,
setup-pixi's `pixi-version` input and the markdownlint rev repeated in
`.pre-commit-fix.yaml`. Neither bot relocks `pixi.lock` on its own, but Renovate can at
least list the pixi pins. S01 recommended Renovate through the Mend app, on condition
that the maintainer accepts the app's write access. The maintainer accepted it for this
repository.

Wave 2 added pins S01 never saw: the `Dockerfile`'s two base images by tag and digest
(decision 0015), cargo-release and cargo-fuzz in `tools.txt`, the fuzz crate's own
manifest and lockfile (decision 0017), and `nextest-version`, which must equal the
cargo-nextest pin.

## Decision

Renovate, through the Mend-hosted app installed on `BiscuitBin/pachinko` alone, configured
by `renovate.json` at the repository root. Native managers read the crates (both cargo
roots), the actions, the pixi pins and the `Dockerfile`. Regex managers read the rest:
the toolchain channel, `rust-version`, `tools.txt` and `tools-source.txt`,
`nextest-version`, the tooling tag, the pixi bootstrap pair and the hook revs. The native
`pre-commit` manager stays off because it reads a SHA rev only as `# frozen:`, and the
native `rust-toolchain` manager is off for the reason below.

The configuration lands held. `:dependencyDashboardApproval` in `extends` makes every
update wait for a click on the dependency dashboard. Vulnerability-fix pull requests are
the exception: Renovate's `vulnerabilityAlerts` settings skip dashboard approval by
default, so once vulnerability alerts are enabled on the repository such a pull request
opens unasked. `automerge` is `false`, so nothing merges itself. A later change may
remove the `extends` line so the crates and actions groups flow weekly; every other rule
carries its own approval and stays held.

The toolchain moves in one pull request, in three places, and all three take their
versions from the `rust` image's `-slim-bookworm` tags on Docker Hub, the variant the
`Dockerfile` builds on. `channel` and `rust-version` are read by regex managers under the
`docker` datasource and depName `rust`, each keeping only the `-slim-bookworm` tags, the
`Dockerfile` is read natively under the same depName, and one rule groups them. The
official image pushes `x.y-slim-bookworm` and `x.y.z-slim-bookworm` from one build. A
channel read from Rust's own release manifest, or from the bare tags, could move to a
release whose image is not yet published, and the required `image` job would fail on it.
Docker versioning keeps the component count, so the two-part `rust-version` moves on a
minor release only, as it should.

setup-pixi's `pixi-version` is read by a regex manager, not by the github-actions manager,
under the depName and datasource of the `requires-pixi` floor, so the pair moves in one
pull request.

## Consequences

The app holds read access to Dependabot alerts, administration and metadata, and read
and write access to checks, contents, commit statuses, issues, pull requests and
workflows, which the [security model](../explanation/security-model.md) records. That is
the one trust it is given that a contributor's branch is not. The `check` context is bound
to the GitHub Actions app and no review is required, so "a person merges" rests on
`automerge` staying off.

Three moves stay manual and the how-to says why: lychee, whose `LYCHEE_VERSION` argument
must match its rev and whose tags carry a `lychee-v` prefix; the allium binary, which moves
with the tooling tag; and the `pixi.lock` relock after a pixi pin moves, until the hosted
app is shown to run `pixi lock`. A toolchain pull request arrives red when the lint set
moves, and a person finishes it with `just clippy`. `requires-python` in `pyproject.toml`
is disabled for Renovate's `pep621` manager, because pixi owns Python; it moves by hand
with the `python` pin.

Each regex manager is proved only by Renovate itself, from its dependency dashboard or a
real pull request, after the app is installed.

## What would reopen this

The app's permissions changing, or Mend's hosted service ending. Dependabot gaining the
one-line pins above, or a way to read a pixi manifest. Evidence that the hosted app
relocks `pixi.lock`, which would move that relock out of the manual list, or that it
cannot, which would turn the pixi rules into a notice for good. The `rust` image ceasing
to publish `-slim-bookworm` tags for new releases, or to track Rust releases closely,
which would stop the toolchain group moving.

## Related pages

- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Maintenance](../operations/maintenance.md)
- [Security model](../explanation/security-model.md)
- [Decision 0007: Dependency policy](0007-dependency-policy.md)
- [Decision 0015: The container image](0015-container-image.md)
