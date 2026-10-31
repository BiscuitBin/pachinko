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
depends on it ([decision 0001](../decisions/0001-library-and-service.md)). The
toolchain, the gates, the handbook, the agent contract, the CI and the Allium tooling
were carried from libpawdoku at `ccdd001`, so that the specification and the code
arrived in a repository that was already green; both have now arrived, the code as a
prototype.

## What it does

It is a prototype of a queue, built to be measured. `libpachinko` implements the queue
the specification defines: pockets, each holding one record per key with a higher rank
superseding in place, consumed under exclusive, renewable leases, with expiry, parking
and the decay of superseded records and dead letters. `pachinko` runs that queue behind
an Arrow Flight API on tokio, with each pocket's records held as one Arrow `RecordBatch`
([decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md),
[decision 0020](../decisions/0020-records-as-arrow-batches.md)): `pachinko serve`
starts it, and the binary with no arguments still writes the line `pachinko <version>`.
[Architecture](../explanation/architecture.md) explains how the pieces fit, and the
[API reference](../reference/api.md) lists the public items and the wire. The rest of
the repository holds that code to the standard it was built to from the first commit:
the full Rust gate, coverage, the effects check, the specification checks, the
handbook's checks and the published API reference.

What the library does is stated first in `docs/specs/`, one rule at a time, and built
second ([decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)); see
[Specifications](../explanation/specifications.md). Today `docs/specs/` holds one
module, `pachinko.allium`, and the library implements every rule it states. A
behaviour that no module states is not the library's yet, however obvious it looks.

## What it deliberately does not do

- **No answer to the specification's open questions.** The module leaves nine questions
  open, and the code answers none of them. A publish whose rank equals the stored rank
  is rejected, as the module's placeholder says, and no surface reads the superseded
  records or the dead letters: they decay unread, seen only by the tests' inspection
  seam. [Terminology](terminology.md) holds the domain's words.
- **No persistence, capacity bound or deployment beyond one host.** The queue lives in
  memory, which nothing bounds, and keeps nothing across a restart: the module's
  Excludes leave storage, snapshots, replication and recovery to the implementation, and
  the prototype provides none of them. The Flight service has no authentication,
  authorisation or TLS, and listens on loopback unless `--listen` says otherwise;
  [Security model](../explanation/security-model.md) says what that exposes, and
  [decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md) says what
  would reopen it.
- **No publishing to a registry.** Both crates are `publish = false`, so neither reaches
  a package registry. The API reference is the one thing deployed, to GitHub Pages
  ([decision 0013](../decisions/0013-api-reference-on-github-pages.md)); the container
  image is built in CI and pushed nowhere
  ([decision 0015](../decisions/0015-container-image.md)). The release process tags a
  crate and cuts a GitHub Release with no binary assets, with `publish = false` kept
  ([decision 0016](../decisions/0016-release-process.md)).
- **No effect performed by the library.** The library performs no effect itself; it
  reads the time through its clock trait, and the service is where effects happen.
  [Architecture](../explanation/architecture.md) explains the boundary, and
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
