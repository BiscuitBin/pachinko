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
depends on it ([decision 0001](../decisions/0001-library-and-service.md)). The toolchain,
the gates, the handbook, the agent contract, the CI and the Allium tooling were carried
from libpawdoku at `ccdd001`, so that the specification and the code arrived in a
repository that was already green.

## What it does

`libpachinko` holds one queue of pockets. Publishers publish keyed, ranked records into
pockets, each pocket holding one record per key with a higher rank superseding in place;
a consumer leases one pocket at a time, pulls its oldest records in bounded batches and
acks, nacks, renews or releases them under the lease; a lease that lapses returns its
pocket to the queue, and superseded records and dead letters decay after their TTLs.
`pachinko serve` serves that queue as an HTTP/1.1 JSON API on tokio, one owner task
holding it in memory ([HTTP API](../reference/http-api.md)); with no argument the binary
writes `pachinko <version>`. Everything else in the repository holds the code to one
standard: the full Rust gate, coverage, the effects check, the specification checks, the
handbook's checks and the API reference.

What the library does will be stated first in `docs/specs/`, one rule at a time, and
built second ([decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)); see
[Specifications](../explanation/specifications.md). Today `docs/specs/` holds one
module, `pachinko.allium`, which specifies the domain, and the library and the service
implement it. A behaviour that no module states is not the library's yet, however obvious
it looks; the module's open questions mark the decisions nobody has made.

## What it deliberately does not do

- **No answer to an open question.** Equal-rank publishes, who reads the superseded and
  dead-letter queues, capacity beyond shedding them and the other open questions in the
  root module are the maintainer's to decide; the code answers none of them.
  [Terminology](terminology.md) holds the domain's words.
- **No persistence, authentication or TLS.** The queue lives in the service's memory and
  is lost when it stops; the HTTP surface is unauthenticated and binds loopback by
  default ([decision 0019](../decisions/0019-service-runtime-tokio-and-http.md);
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
