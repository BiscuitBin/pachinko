---
title: "Architecture decisions"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_index]
requires: []
---

# Architecture decisions

This index will be the record of what was chosen for this repository, what it cost, and
what would have to change for each choice to be revisited. The series is this
repository's own and starts at 0001; records carried from the repository it was adapted
from say so under their heading and keep the topic slug they had. Lane T07 writes the
full index and every record below; until then each record is a stub that names what it
will say.

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

Records from 0015 onwards are written by the wave-2 tickets that take those decisions,
each adding its file, its manifest entry and its row here.
