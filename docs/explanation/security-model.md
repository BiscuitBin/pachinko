---
title: "Security model"
kind: "explanation"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [security_model]
requires: []
---

# Security model

This workspace is a library and a prototype service built around it. The library
performs no effect itself, and the service holds one queue in memory and serves it over
Arrow Flight to anyone who can reach its port. That removes most of the attack surface
software usually has, and it is worth being precise about what remains rather than
claiming the problem away: the supply chain that builds both crates, the code handed to
the library's callers, and a network surface that authenticates no one.

## What there is to protect

Very little, and that is the point.

- **Payloads, in memory only.** The queue holds what publishers send it: record keys,
  pocket keys, ranks and payloads, which are opaque bytes it never reads. libpachinko
  keeps them only in the `Queue` value its caller owns, and has nowhere of its own to
  send anything. The service keeps that queue in process memory and writes none of it
  to disk or to a log, so it ends with the process. Nothing here encrypts a payload, and
  anyone who reaches the port can pull one, so a deployment that sends sensitive
  payloads answers for that choice.
- **There are no credentials in the repository.** No file here carries a token.
  Continuous integration uses the token GitHub mints for each run and discards with it,
  and the workflows need nothing more: the Pages deployment proves itself with the run's
  own identity token, so it stores no credential either. Every crate sets
  `publish = false`, and a release cuts a GitHub Release with the run's own token
  ([decision 0016](../decisions/0016-release-process.md)).

## What is deliberately not secure

**Nothing in the library is a secret.** Its output is computable by anyone holding the
crate and the same inputs. The clock is its one effect trait, and no specification asks
for cryptographic randomness; until one does, no effect trait promises it, and the fakes
that sit beside the effect traits for tests are never a source to use where
unpredictability matters.

**The service authenticates no one and encrypts nothing.** Every request names its
publisher or consumer id in its body, ticket or descriptor, and nothing checks who sent
it: an id is a name the client asserts, not a credential. Anyone who reaches the port can
therefore claim an id no one has joined, and publish, pull, ack, nack, renew, release or
read the `status` as any party that has joined. A lease id is a fencing token, not a secret: it comes from
the queue's counter, so it is predictable, and it stops a delayed consumer acting on a
lease that has ended, not a hostile party acting on one that has not. The traffic is
unencrypted gRPC: no TLS feature of `tonic` or `arrow-flight` is enabled, so
`deny.toml`'s `openssl-sys` ban holds
([decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md)).
[API reference](../reference/api.md) lists what each Flight method and action does.

**Loopback is the default, and `--listen` is the operator's.** `pachinko serve` binds
`127.0.0.1:0` unless told otherwise, so only the host it runs on can reach it, and it
prints the address it bound as `listening on <addr>`. `--listen` binds any address the
operator names; an address other hosts can reach gives every one of them the access
above.

**Memory has no bound.** Queued and held records stay until a consumer settles them,
however many a publisher sends; superseded records and dead letters stay until
`superseded_ttl` or `dead_letter_ttl` decays them. A party that can reach the port can
therefore grow the process's memory until it fails. What, if anything, bounds it is the
module's third open question, on capacity, which no code answers.

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
  hook and every GitHub Action is pinned to a commit SHA, not to a mutable tag. The
  tools `tools.txt` and `tools-source.txt` pin are the exceptions worth naming.
  cargo-hack, cargo-release and cargo-fuzz, the three lines in `tools.txt`, are taken as
  release binaries and never compiled. rustqual, the one line in `tools-source.txt`,
  publishes no binary at all, so it arrives as its crates.io source,
  checked against the registry index's checksum, and is built on the machine with
  `--locked`, taking the dependency versions its own lockfile names. Were
  cargo-quickinstall to publish a build of the pin, cargo-binstall would take that
  instead, and only if it is signed (`--only-signed`). A release asset rustqual itself
  published would not be taken: that pass disables cargo-binstall's `crate-meta-data`
  strategy, so nothing unsigned arrives by it
  ([decision 0012](../decisions/0012-metrics-gate.md)). It also runs with the GitHub
  token unset, so the build scripts it compiles cannot read the token.
- **Dependency updates.** Renovate proposes pin moves through the Mend-hosted Renovate
  app, installed on 2026-10-24 for every repository in the `BiscuitBin` organisation, not
  this one alone, and the one party outside the maintainer and GitHub Actions that holds
  write access here ([decision 0018](../decisions/0018-renovate.md)). The installation
  holds read access to administration, Dependabot alerts, metadata, packages and the
  organisation's members, and read and write access to checks, contents, commit
  statuses, issues, pull requests and workflows, as the organisation's installation
  record lists them (`gh api orgs/BiscuitBin/installations`). Because the installation is
  organisation-wide, those permissions reach every `BiscuitBin` repository, including any
  added later.
  The app never merges: `renovate.json` sets `"automerge": false`, and `main` takes nothing
  without a green `check`, a context bound to the GitHub Actions app, so no other app can
  report it. Branch protection requires no review, so that the maintainer merges is
  policy resting on automerge staying off, not something protection enforces. Every
  update waits for approval on Renovate's dependency dashboard, with one exception: once
  vulnerability alerts are enabled on the repository, Renovate's vulnerability-fix pull
  requests skip that approval by default, so one can open unasked. A Renovate branch runs
  the same jobs as anyone's, with `contents: read` and no secret.
- **Memory safety.** `#![forbid(unsafe_code)]` holds in every crate through the
  workspace lint table in the root `Cargo.toml`, which sets `unsafe_code = "forbid"` for
  every member that inherits it. `forbid`, not `deny`: no member and no item can opt back
  in.
- **No effects in the library.** libpachinko reaches the clock, randomness, the
  filesystem, the network, the environment and threads only through traits it defines,
  and `just effects` holds both its crate bans and its std-API bans, each with a probe
  that shows every ban fires. Code clippy does not compile and crates outside the ban
  list are left to review ([Architecture](architecture.md),
  [decision 0003](../decisions/0003-effects-behind-traits.md)).
- **Workflow permissions.** `ci.yml`, `audit.yml`, `mutants.yml` and `fuzz.yml` hold
  `contents: read` and nothing else. `pages.yml` holds more, `pages: write` and
  `id-token: write`, on its deploy job alone, which checks nothing out and runs no code
  from the repository
  ([decision 0013](../decisions/0013-api-reference-on-github-pages.md)). `release.yml`
  is the second to hold more, in a file of its own rather than widening `ci.yml`: it
  grants nothing at the top level, its `verify` job, which runs the gate on the tag,
  holds `contents: read`, and its `publish` job alone holds `contents: write`, behind
  the `release` environment's required reviewer. That job runs no setup or build tool,
  and checks out the commit `verify` validated and refuses a tag that has moved since
  ([decision 0016](../decisions/0016-release-process.md)).
- **Credential leakage.** `ripsecrets` scans every commit, and its output is redacted so
  a match never copies the matched value into a log.

## What is deferred

**Authentication, encryption and a memory bound.** The prototype has none of the three,
by the choices above. Authentication and TLS wait for a deployment beyond one host, which
reopens [decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md) and with
it `deny.toml`'s TLS ban; a memory bound waits for the specification's answer on
capacity. No surface reads the superseded records or the dead letters yet, which is the
module's second open question, so what an operator may see of them, and who may, waits
for its answer too. This page describes each when its decision
records it.

## What is out of scope

What a caller does with the library's output, and what it passes in through the effect
traits it implements: an implementation that reads the clock or the network on the
library's behalf answers for its own sources. What a consumer does with a payload it
pulls is its own too: a lease's fencing token protects the queue only from actions that
reach it, so a consumer that writes elsewhere needs that store to check the token
itself.

## Reporting

See `SECURITY.md` at the repository root.

## Related pages

- [Architecture](architecture.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Quality gates](../reference/quality-gates.md)
- [API reference](../reference/api.md)
- [Decision 0007](../decisions/0007-dependency-policy.md)
- [Decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md)
