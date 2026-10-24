---
title: "Decision 0016: The release process"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_release_process]
requires: []
---

# Decision 0016: The release process

## Context

Both crates have independent versions and remain private to crates.io through inherited
`publish = false`. A release records a reviewed snapshot for consumers without choosing
a registry or a binary distribution channel. Libpawdoku's S02 spike at `ccdd001`
proved cargo-release's local version, changelog and annotated-tag operations, including
tagging a merge commit after a release pull request.

The spike found that release-plz can avoid generated changelogs and does not require
conventional commits. Its release pull request nevertheless needs a stored PAT or App
credential to trigger CI, and its publisher runs on pushes to main. The local
cargo-release flow fits the existing required `check` and uses only GitHub's run token.

## Decision

Use cargo-release 1.1.7, pinned in `tools.txt` because conda-forge does not carry it.
Its upstream binaries support both repository platforms. Use cargo-semver-checks 0.51.0,
pinned in pixi for both platforms. These are development tools, with no new Rust
dependency; this records their purpose under decision 0007.

Versions are independent. Tags are `libpachinko-v<version>` and `pachinko-v<version>`.
Cargo-release bumps one selected crate and commits on `release/<crate>-v<version>`
with tagging disabled. Publishing and pushing are disabled in workspace release
metadata. The commit reaches `main` through a pull request requiring `check`; the
maintainer then updates main and makes the annotated tag on the merge commit.
Commit consolidation is disabled explicitly: with 1.1.7's default, updating a
dependent's constraint can leave crate and version placeholders literal in the commit
subject. Per-crate commits retain the intended release subject.

One root changelog has `## libpachinko [Unreleased][libpachinko-unreleased]` and
`## pachinko [Unreleased][pachinko-unreleased]`. Each release inserts a heading such as
`## libpachinko [0.1.1][libpachinko-0.1.1] - YYYY-MM-DD` immediately below that
crate's unreleased heading, preserving previous releases until the next crate heading.
Each replacement is anchored to a whole line, names its crate, and requires exactly one
match. Each crate also owns its reference-link names. The replacement writes the full
tag prefix rather than the unavailable tag-name placeholder when tagging is disabled.
The date is the UTC day the tag is pushed; a later merge requires a small pull request
to re-date the heading before tagging. Release notes end at the next level-two heading
or the reference-link block and must contain text.

The tag workflow has no permissions at its top level. Its read-only verify job runs
the complete gate and checks the tag's commit, manifest version, ancestry on main and
exactly one dated non-empty changelog section. Its publish job has only contents write,
is gated by the `release` environment's maintainer review, re-fetches the tag and
refuses a changed commit. It runs no setup action, pixi or just. It creates a GitHub
Release from that section with the run token, with no registry publish or binary asset.
The environment must be created and its required reviewer read back before merging
the workflow; its creation and every tag push are separately authorized actions.

`just semver` compares libpachinko against its latest reachable release tag. Before
the first tag it compares against the branch's merge base with `origin/main`, or local
`main` when that remote branch is absent. An explicit revision overrides discovery.
The recipe checks both locks first and runs offline because cargo-semver-checks has
no locked option. The pull-request job is advisory and outside the required aggregate.

## Consequences

The maintainer owns release timing, level and prose. Empty unreleased sections are
normal after a release, but an empty release cannot pass verification. The first real
release may tag the existing 0.1.0 or bump it; this decision chooses neither.
The service's library dependency constraint may need to change with a library release;
that does not require a service version bump or move its changelog section.
The current library unit test and doctest and both service output tests contain explicit
version assertions. A library bump requires manually updating and committing those
assertions before the release gate; the service reports the library version, not its
own. Maintenance records this release step.

There is no release wrapper recipe: the distinct preparation and tag commands expose
which action changes history. `SECURITY.md` supports only main and promises no
backports. Before libpachinko 1.0, add the semver job to the required aggregate and
reassess the compatibility policy. See [Maintenance](../operations/maintenance.md)
for the operator sequence.

## What would reopen this

Publishing to crates.io would reopen the decision: lift the selected crate's inherited
publish prohibition, check its package licence and metadata, decide the first publish
credential, then configure trusted publishing and revoke that credential. Reassess
MSRV, compatibility enforcement and version floors then. A need for binary assets,
automated release pull requests or a different versioning relationship would also
require new evidence and an updated record.

## Related pages

- [Maintenance](../operations/maintenance.md)
- [Commands](../reference/commands.md)
- [Decision 0007: Dependency policy](0007-dependency-policy.md)
- [Architecture](../explanation/architecture.md)
