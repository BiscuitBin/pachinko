---
title: "Architecture decisions"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_index]
requires: []
---

# Architecture decisions

A record of what was chosen, what it cost, and what would have to change for the choice
to be revisited. Each entry is numbered and never renumbered; a decision that is
superseded is marked rather than deleted, because the reasoning is what makes the
successor legible.

These are decisions about *how* this repository is built. Decisions about *what* it does
belong in the specifications under `docs/specs/`, and unresolved ones are recorded there
as `open question` blocks — see [Specifications](../explanation/specifications.md).

## The record

| Number | Decision |
| --- | --- |
| [0001](0001-library-and-service.md) | A library and a service in one workspace |
| [0002](0002-specs-are-the-source-of-truth.md) | Specifications decide behaviour |
| [0003](0003-effects-behind-traits.md) | Effects behind traits |
| [0004](0004-hook-runner-and-checkers.md) | Hook runner and checkers |
| [0005](0005-project-managed-allium-cli.md) | A project-managed Allium binary |
| [0006](0006-apache-2-0.md) | Apache-2.0 |
| [0007](0007-dependency-policy.md) | Dependency policy: caret ranges and `Cargo.lock` |
| [0008](0008-std-library.md) | A std library, with effects kept out mechanically |
| [0009](0009-rust-quality-gate.md) | The Rust quality gate |
| [0010](0010-skill-reference-material.md) | Skill reference material beside the skills |
| [0011](0011-tool-manager.md) | Tool manager |
| [0012](0012-metrics-gate.md) | The metrics gate |
| [0013](0013-api-reference-on-github-pages.md) | The API reference on GitHub Pages |
| [0014](0014-service-runtime-deferred.md) | The service runtime waits for the specifications |
| [0015](0015-container-image.md) | The container image |
| [0016](0016-release-process.md) | The release process |
| [0017](0017-mutation-bench-and-fuzz.md) | Mutation testing, benchmarks and fuzzing |
| [0018](0018-renovate.md) | Dependency updates through Renovate |

Decision 0004 is superseded in part by decision 0011, which names the consequences of
0004 it replaces and the ones that stand. Two recipes in the gate list decision 0009
records have records of their own: decision 0012 holds gate 7, `just metrics`, and
decision 0008 holds gate 8, `just effects`.

## The numbering

This series is the repository's own and starts at 0001. Eleven entries were carried from
libpawdoku's decision records at `ccdd001` when this repository was adapted from it
(0002 to 0007 and 0009 to 0013); each says so under its heading, with the number it had
there, and keeps the topic slug it had, because the slug is what a cross-repository
reference names. Three entries are new here: 0001, 0008 and 0014. Nothing here is
rendered from a template, so no frozen inventory of numbers applies.

Numbers 0015 to 0018 were reserved for the wave-2 tickets that took those decisions: 0015
for the container image (T13), 0016 for the release process (T14), 0017 for mutation
testing, benchmarks and fuzzing (T15 and T16) and 0018 for dependency updates through
Renovate (T17). Each added its file, its manifest entry and its row above. The next
decision is 0019.

## Writing a new one

Copy the shape of an existing entry: context, the decision, the consequences including
the ones that hurt, and what would reopen it. Add the file, add a manifest entry, add a
row above. A decision nobody can find is not recorded.

## Related pages

- [Documentation map](../README.md)
- [Architecture](../explanation/architecture.md)
- [Specifications](../explanation/specifications.md)
