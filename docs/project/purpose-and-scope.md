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
library at `crates/libpachinko`, and a service binary at `crates/pachinko` that
depends on it ([decision 0001](../decisions/0001-library-and-service.md)). What this
repository carries today is the toolchain, the gates, the handbook, the agent contract,
the CI and the Allium tooling, carried from libpawdoku at `ccdd001`, so that the
specifications and the code arrive in a repository that is already green.

## What it does

`libpachinko` implements the domain the specifications define as a synchronous
`Queue`, which reaches the clock and its lease-id source only through the traits
`Clock` and `LeaseIds`. `pachinko`'s `main` is one call to a tested `run()`: with no
argument it writes the line `pachinko <version>`, and `pachinko serve` runs the queue in
memory behind the [HTTP API](../reference/http-api.md)
([decision 0019](../decisions/0019-tokio-and-axum-service.md)). Everything else in the
repository holds that code to one standard: the full Rust gate, coverage, the effects
check, the specification checks, the handbook's checks and the API reference.

What the library does will be stated first in `docs/specs/`, one rule at a time, and
built second ([decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)); see
[Specifications](../explanation/specifications.md). Today `docs/specs/` holds one
module, `pachinko.allium`, which specifies the domain: a queue of pockets, each holding
one record per key with a higher rank superseding in place, consumed under exclusive,
renewable leases. The library implements all of it, and the service exposes its two
surfaces. A behaviour that no module states is not the library's, however obvious it
looks.

## What it deliberately does not do

- **No answer to an open question.** The root module under `docs/specs/` leaves nine
  product decisions open, and the code decides none of them: a publish whose rank equals
  the stored record's is refused as a stated placeholder, and no route reads the
  superseded records or the dead letters. [Terminology](terminology.md) holds the
  domain's words.
- **No persistence, TLS or authentication.** The service holds the queue in one
  process's memory, so a restart empties it, and it speaks plain HTTP to whoever reaches
  its listener, with no authentication or rate limiting
  ([decision 0019](../decisions/0019-tokio-and-axum-service.md),
  [Security model](../explanation/security-model.md)).
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

Callers of the library, and the people and agents who change it. The `pachinko`
service is the library's one caller today: it depends on `libpachinko`, never the
reverse, and it is where the real implementations of the library's effect traits
live. The repository's readers are contributors, maintainers and agents working to the
[agent contract](../reference/agent-contract.md); none of them re-decides a behaviour the
specifications state.

## Related pages

- [Repository map](repository-map.md)
- [Terminology](terminology.md)
- [Architecture](../explanation/architecture.md)
- [Specifications](../explanation/specifications.md)
