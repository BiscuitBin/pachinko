---
title: "Decision 0007: Dependency policy: caret ranges and `Cargo.lock`"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_dependency_policy]
requires: []
---

# Decision 0007: Dependency policy: caret ranges and `Cargo.lock`

*Carried from libpawdoku's decision 0007 at `ccdd001`.*

## Context

The games pin every dependency exactly. Their `AGENTS.md` invariant reads "No `^`, no
`~`", in `package.json` or `pyproject.toml`, with lockfiles committed and proved by
`just lock-check`. That is right for an application, whose manifest is the last word on
what it runs. A library's manifest is a constraint every consumer inherits. A crate that
writes `=1.2.3` for a dependency forbids every other crate in a consumer's tree from
resolving a different `1.2.x`, because Cargo unifies one version per compatible range, so
two such libraries pinning different patch releases cannot share a tree at all.

This workspace holds a library and a service
([decision 0001](0001-library-and-service.md)). Today the library's one consumer is the
service, which shares its lockfile, so an exact pin would cost nothing yet; but the
library's manifest is written for the consumer that would reopen decision 0001, a crate
outside this workspace, and that consumer would inherit whatever it says.

Two further facts shape the mechanism. The gate runner snapshots the worktree between
recipes and aborts on any change, so no recipe may rewrite `Cargo.lock`. The first
external dependencies are development harnesses introduced by T16; normal
workspace dependencies still consist only of the service’s path dependency on
`libpachinko`. Cargo-shear fails the gate on a dependency nothing uses.

## Decision

Manifests write full `x.y.z` caret ranges. The first external dependency adds one
`[workspace.dependencies]` table to the root `Cargo.toml`, which every crate inherits
from; a crate manifest names an external dependency with `workspace = true` and never a
version of its own. A path dependency inside the workspace writes its range in place, as
the service's on `libpachinko` does: `version = "0.1.0"`, a caret range. `Cargo.lock` is
committed and is the pin. Every cargo invocation in a recipe that builds, tests, lints or
documents the workspace carries `--locked`, as do `just sync`, `just effects`,
`just deny` and `just audit`, so a recipe that would need to change the lockfile fails
instead, and `just lock-check`, gate 2 of `just check`, proves the lockfile matches the
manifests, offline and without writing. The snapshot recipes are the one exception among
the recipes that test. cargo-insta 1.48.0 hands trailing options to nextest's test
binaries, and cargo has no config equivalent of `--locked`, so those recipes depend on
`lock-check` and run offline, as [Configuration](../reference/configuration.md) records
under `CARGO_NET_OFFLINE`. cargo-deny checks licences, bans and sources offline at gate
13 of `just check`, and checks advisories in `just audit`, which runs outside
`just check`, in its own workflow, because the RustSec fetch can fail for reasons
unrelated to the diff. `rust-version` follows the toolchain pin until a crate is first
published. There is no `.cargo/config.toml`: `RUSTFLAGS` set there changes fingerprints
and thrashes the build directory against rust-analyzer.

A new dependency carries a note, which `AGENTS.md` calls the decision-0007 note: what the
crate is for, and why the crate that takes it cannot do without it. The reason is
recorded in the manifest beside the range and stated in the pull request that adds it.
A new version of a dependency already present needs no note; a new crate always does,
development dependencies included.

This departs from the games' invariants, and `AGENTS.md` says so: its eighth invariant
states the lockfile as the pin and cites this record, its ninth requires the note, and
its Provenance section lists this record among the deliberate deviations from the games'
conventions.
The reason is the one above. An exact pin is a promise about this repository's
resolution; in a library it becomes a demand on everyone else's.

## Harness dependencies

T16 adds divan 0.1.21 through `[workspace.dependencies]` as a library development
dependency only. It provides the benchmark harness and black box; it is outside the
graph consumers resolve and cargo-deny’s `exclude-dev` graph. Its MSRV is 1.80.0,
below the pinned compiler. Its transitive packages are committed in `Cargo.lock`.

The unpublished `fuzz/` crate is excluded from the workspace. It names libfuzzer-sys
0.4.13 directly because it cannot inherit workspace dependencies; this is the explicit
exception to inheritance above. Its libFuzzer driver supplies the byte fuzzing harness,
and `fuzz/Cargo.lock` pins its separate graph. `lock-check` proves that lock offline
alongside the workspace and pixi locks. cargo-fuzz 0.13.2 is a tool pin in `tools.txt`,
with no locked flag: `just fuzz` validates the locks first and runs offline.

Unlike the service's workspace path dependency, the unpublished fuzz harness names
`libpachinko` by path alone: its excluded manifest is not updated by cargo-release.
This exception avoids a stale version constraint; a library version bump still requires
refreshing `fuzz/Cargo.lock` before the locked checks can pass.

## Consequences

The consequences that hurt. The lockfile tests one resolution and consumers get another:
a bug in a dependency version this repository never resolves is invisible here until a
consumer hits it. Renovate ([decision 0018](0018-renovate.md)) proposes pin moves, but
holds each one until a maintainer approves it on the dependency dashboard, a
vulnerability fix excepted once alerts are on, so the lockfile ages deliberately rather
than by neglect.
Every addition is a reviewed decision with its reason recorded in the manifest beside the
range. And caret ranges make `cargo update` a real change every time it runs: a moved
lockfile is a diff to read, not noise to wave through.

The tools the gate runs are pinned the other way, exactly, because they are an
application's dependencies rather than a library's;
[decision 0011](0011-tool-manager.md) records how.

## What would reopen this

The service's binary did not reopen this, and another binary would not: one lockfile
still resolves once, and the binary ships that resolution. Publishing may add a
minimal-versions check, because a caret range is a claim that the crate builds against
its floor, and nothing here tests that today; both crates are `publish = false`, so the
trigger is the first decision to publish one. A native dependency that must be pinned
exactly would be a per-crate exception recorded in this file, not a return to exact pins
everywhere.

## Related pages

- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Configuration](../reference/configuration.md)
- [Quality gates](../reference/quality-gates.md)
- [Security model](../explanation/security-model.md)
- [Decision 0001: A library and a service in one workspace](0001-library-and-service.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
