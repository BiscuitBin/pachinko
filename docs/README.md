---
title: "Documentation map"
kind: "project"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [documentation_navigation]
requires: []
---

# Documentation map

Every page below is registered in `manifest.yml`, owns at least one topic, and is
reachable from here. That is the whole of the arrangement; the rules behind it are in
the documentation contract, listed under Look up.

This repository is a Cargo workspace of two crates: `crates/libpachinko`, a std library,
and `crates/pachinko`, a minimal service binary built around it. What their domain is,
the specifications will define; the handbook describes the machinery that is already
here.

Behaviour is specified separately, in Allium, under `docs/specs/`. There is one module
so far, the placeholder root [`pachinko.allium`](specs/pachinko.allium), and it states
no behaviour yet; each behaviour arrives there as a specification change before any code
implements it. That file is not part of this handbook; it is its subject. Start at
Specifications, under Understand, to see how the two relate.

## Start here

- [Purpose and scope](project/purpose-and-scope.md) — what the workspace is for, and what it is not.
- [Repository map](project/repository-map.md) — where everything lives.
- [Terminology](project/terminology.md) — the words this repository uses precisely.
- [Make your first change](tutorials/first-change.md) — clone to green gate, through one small change and its tests.

## How to

- [Develop locally](how-to/develop-locally.md)
- [Test and debug](how-to/test-and-debug.md)
- [Work with the specifications](how-to/work-with-the-specs.md)
- [Maintain dependencies](how-to/maintain-dependencies.md)
- [Build the container image](how-to/build-the-container-image.md)
- [Deploy to GitHub Pages](how-to/deploy-to-github-pages.md)

## Understand

- [Architecture](explanation/architecture.md) — how a library and the service around it are put together, and where effects live.
- [Layering and dependency direction](explanation/layering.md) — which crate and module may depend on which.
- [Specifications](explanation/specifications.md) — why behaviour is written down before it is built.
- [Security model](explanation/security-model.md) — what a library and a minimal service binary do and do not defend.
- [Quality philosophy](explanation/quality-philosophy.md) — why each gate exists.

## Look up

- [Commands](reference/commands.md)
- [Configuration](reference/configuration.md)
- [Testing](reference/testing.md)
- [Quality gates](reference/quality-gates.md)
- [Documentation contract](reference/documentation-contract.md)
- [Agent contract](reference/agent-contract.md)
- [API reference](reference/api.md)

## Run it

- [Maintenance](operations/maintenance.md)
- [Troubleshooting](operations/troubleshooting.md)

## Decisions

- [Architecture decisions](decisions/README.md) — the record of what was chosen and why.
- [Decision 0015: The container image](decisions/0015-container-image.md)
- [Decision 0016: The release process](decisions/0016-release-process.md)
- [Decision 0017: Mutation testing, benchmarks and fuzzing](decisions/0017-mutation-bench-and-fuzz.md)
- [Decision 0018: Dependency updates through Renovate](decisions/0018-renovate.md)
