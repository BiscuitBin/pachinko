---
title: "Purpose and scope"
kind: "project"
audience: [user, contributor, maintainer, agent]
canonical_for: [project_purpose, project_non_goals]
requires: []
---

# Purpose and scope

This repository is the foundation for `libpachinko` and the `pachinko` service. It is a
Cargo workspace of two crates, both licensed Apache-2.0 and both `publish = false`: a std
library at `crates/libpachinko`, and a minimal service binary at `crates/pachinko` that
depends on it ([decision 0001](../decisions/0001-library-and-service.md)). What this
repository carries today is the toolchain, the gates, the handbook, the agent contract,
the CI and the Allium tooling, carried from libpawdoku at `ccdd001`, so that the
specifications and the code arrive in a repository that is already green.

## What it does

Today, very little, on purpose. `libpachinko` holds the domain once the specifications
define one; until then it holds one public item, `version()`. `pachinko`'s `main` is one
call to a tested `run()`, which writes the line `pachinko <version>`. Everything else in
the repository is there to hold the code that comes later to the same standard from its
first commit: the full Rust gate, coverage, the effects check, the specification checks,
the handbook's checks and the API reference.

What the library does will be stated first in `docs/specs/`, one rule at a time, and
built second ([decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)); see
[Specifications](../explanation/specifications.md). Today `docs/specs/` holds one
placeholder module, `pachinko.allium`, which declares nothing. A behaviour that no module
states is not the library's yet, however obvious it looks.

## What it deliberately does not do

- **No domain yet.** No module under `docs/specs/` defines an entity, a rule or a
  surface, so no code here implements one, and no page here says what the domain is.
  The first specification change that names a behaviour defines it.
- **No runtime or transport for the service.** The service is a minimal std binary with
  no async runtime, transport or HTTP stack. Choosing them before the specifications say
  what the service must do would be a guess; a spike and a further decision follow the
  specifications ([decision 0014](../decisions/0014-service-runtime-deferred.md)).
- **No publishing to a registry.** Both crates are `publish = false`, so neither reaches
  a package registry. The API reference is the one thing deployed, to GitHub Pages
  ([decision 0013](../decisions/0013-api-reference-on-github-pages.md)); the container
  image is built in CI and pushed nowhere
  ([decision 0015](../decisions/0015-container-image.md)). The release process tags a
  crate and cuts a GitHub Release with no binary assets, with `publish = false` kept
  ([decision 0016](../decisions/0016-release-process.md)).
- **No effect performed by the library.** The library performs no effect itself; the
  service is where effects happen. [Architecture](../explanation/architecture.md)
  explains the boundary, and
  [decision 0003](../decisions/0003-effects-behind-traits.md) records it.

## Who it is for

Consumers of the library, and the people and agents who change it. The `pachinko`
service is the library's one consumer today: it depends on `libpachinko`, never the
reverse, and it is where the real implementations of the library's effect traits will
live. The repository's readers are contributors, maintainers and agents working to the
[agent contract](../reference/agent-contract.md); none of them re-decides a behaviour the
specifications state.

## Related pages

- [Repository map](repository-map.md)
- [Terminology](terminology.md)
- [Architecture](../explanation/architecture.md)
- [Specifications](../explanation/specifications.md)
