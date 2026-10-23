---
title: "Security model"
kind: "explanation"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [security_model]
requires: []
---

# Security model

This workspace is a library and a minimal service binary. The library holds no data
about anyone and performs no effect itself, and the service, today, writes one line to
standard output and exits. That removes most of the attack surface software usually
has, and it is worth being precise about what remains rather than claiming the problem
away: the supply chain that builds both crates, the code handed to whoever consumes the
library, and a network surface for the service that no decision has drawn yet.

## What there is to protect

Very little, and that is the point.

- **Nothing is collected or stored.** libpachinko will hold the domain once the
  specifications define one; it takes values from its caller and hands values back,
  and it has nowhere of its own to send anything.
- **There are no credentials in the repository.** No file here carries a token.
  Continuous integration uses the token GitHub mints for each run and discards with it,
  and the workflows need nothing more: the Pages deployment proves itself with the run's
  own identity token, so it stores no credential either. Every crate sets
  `publish = false`; how a release is published is the release process's to decide
  (ticket T14, in wave 2).

## What is deliberately not secure

**Nothing in the library is a secret.** Its output is computable by anyone holding the
crate and the same inputs. No specification yet asks for cryptographic randomness; until
one does, no effect trait promises it, and the fakes that sit beside the effect traits
for tests are never a source to use where unpredictability matters.

## What the build defends

- **Supply chain.** `Cargo.lock` is the pin. The recipes
  [decision 0007](../decisions/0007-dependency-policy.md) names pass `--locked`, so they
  fail rather than resolve a version the lockfile does not name, and `just lock-check`
  fails on drift between a manifest and its lockfile; only `just lock` and
  `just lock-upgrade` resolve, by design. `just deny` checks
  licences, bans and sources offline on every run of `just check`, and `just audit`
  checks the RustSec advisories, which need the network, outside `just check`: weekly
  and on every pull request in its own workflow, which is never a required check.
  `deny.toml` allows only a named list of permissive licences, refuses wildcard version
  requirements, accepts crates from the crates.io index alone and no Git source, and bans
  `openssl-sys` from the whole workspace, so no native TLS stack arrives by accident.
  The ban is on what ships: development dependencies are outside the graph, because they
  serve the tests only. `deny.toml` bans no effect crate; keeping randomness, async
  runtimes and sockets out of the library is `just effects`' job, below. Every remote
  hook and every GitHub Action is pinned to a commit SHA, not to a mutable tag. Two
  tools are the exceptions worth naming. cargo-hack, the one line in `tools.txt`, is
  taken as a release binary and never compiled. rustqual, the one line in
  `tools-source.txt`, publishes no binary at all, so it arrives as its crates.io source,
  checked against the registry index's checksum, and is built on the machine with
  `--locked`, taking the dependency versions its own lockfile names. Were
  cargo-quickinstall to publish a build of the pin, cargo-binstall would take that
  instead, and only if it is signed (`--only-signed`). A release asset rustqual itself
  published would not be taken: that pass disables cargo-binstall's `crate-meta-data`
  strategy, so nothing unsigned arrives by it
  ([decision 0012](../decisions/0012-metrics-gate.md)). It also runs with the GitHub
  token unset, so the build scripts it compiles cannot read the token.
- **Memory safety.** `#![forbid(unsafe_code)]` holds in every crate through the
  workspace lint table in the root `Cargo.toml`, which sets `unsafe_code = "forbid"` for
  every member that inherits it. `forbid`, not `deny`: no member and no item can opt back
  in.
- **No effects in the library.** libpachinko reaches the clock, randomness, the
  filesystem, the network, the environment and threads only through traits it defines,
  and `just effects` proves the crate bans today; the std-API bans join it with T10,
  and until then a direct std call is caught by review ([Architecture](architecture.md),
  [decision 0003](../decisions/0003-effects-behind-traits.md)).
- **Workflow permissions.** `ci.yml` and `audit.yml` hold `contents: read` and nothing
  else. `pages.yml` holds more, `pages: write` and `id-token: write`, on its deploy job
  alone, which checks nothing out and runs no code from the repository
  ([decision 0013](../decisions/0013-api-reference-on-github-pages.md)). The release
  workflow T14 designs will be the second to hold more, and it will live in a file of
  its own rather than widen `ci.yml`.
- **Credential leakage.** `ripsecrets` scans every commit, and its output is redacted so
  a match never copies the matched value into a log.

## What is deferred

**The service's network surface.** The service is a minimal std binary with no async
runtime, transport or HTTP stack, and so it listens on nothing. Which of those it gets,
and therefore what it exposes and to whom, waits for the specifications to say what the
service must do ([decision 0014](../decisions/0014-service-runtime-deferred.md)). This
page describes that surface, its authentication and its limits when the decision that
follows the specifications records them.

## What is out of scope

What a consumer does with the library's output, and what it passes in through the
effect traits it implements: an implementation that reads the clock or the network on
the library's behalf answers for its own sources.

## Reporting

See `SECURITY.md` at the repository root.

## Related pages

- [Architecture](architecture.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Quality gates](../reference/quality-gates.md)
- [Decision 0007](../decisions/0007-dependency-policy.md)
- [Decision 0014](../decisions/0014-service-runtime-deferred.md)
