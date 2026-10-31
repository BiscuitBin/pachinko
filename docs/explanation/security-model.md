---
title: "Security model"
kind: "explanation"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [security_model]
requires: []
---

# Security model

This workspace is a library and a service binary. The library holds no data about
anyone and performs no effect itself, and the service, with no argument, writes one line
to standard output and exits; `pachinko serve` holds a queue in memory and answers HTTP
on one listener. It is worth being precise about what that leaves rather than claiming
the problem away: the supply chain that builds both crates, the code handed to whoever
consumes the library, and the listener.

## What there is to protect

Very little, and that is the point.

- **Nothing is stored beyond the process.** libpachinko holds the queue the
  specifications define; it takes values from its caller and hands values back, and it
  has nowhere of its own to send anything. The service keeps the payloads publishers
  send in memory, unread, until they are acked, decay or the process ends; nothing is
  written to disk.
- **There are no credentials in the repository.** No file here carries a token.
  Continuous integration uses the token GitHub mints for each run and discards with it,
  and the workflows need nothing more: the Pages deployment proves itself with the run's
  own identity token, so it stores no credential either. Every crate sets
  `publish = false`, and a release cuts a GitHub Release with the run's own token
  ([decision 0016](../decisions/0016-release-process.md)).

## What is deliberately not secure

**Nothing in the library is a secret.** Its output is computable by anyone holding the
crate and the same inputs. No specification asks for cryptographic randomness, so no
effect trait promises it: `LeaseIds` asks only for ids that do not repeat, and the
queue draws again if one does. The
service's `RandomLeaseIds` draws 128 bits from the operating system, so a lease id is
hard to guess, but it is a fencing token against a stale consumer, not a credential
([decision 0019](../decisions/0019-tokio-and-axum-service.md)). The fakes that sit
beside the effect traits for tests are never a source to use where unpredictability
matters.

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

## The listener

**The service's network surface is `pachinko serve`'s listener.** It binds
`127.0.0.1:8080` unless `--listen` says otherwise, and answers the routes of the
[HTTP API](../reference/http-api.md) in plain HTTP/1.1
([decision 0019](../decisions/0019-tokio-and-axum-service.md)). Anyone who can reach it
can join as a publisher or a consumer, publish, pull and act on a lease whose id they
hold. An action on a lease is refused unless it names the consumer that holds it, but a
consumer id is a name the client chose, not a proof.

**What is deferred.** There is no TLS, no authentication and no rate limit, and
nothing bounds the queue's memory but decay: capacity is an open question in the
module. Binding beyond localhost is the operator's
choice, and serving it beyond a trusted network belongs behind a proxy that terminates
TLS and authenticates; `deny.toml` keeps `openssl-sys` out, so the service grows no
native TLS stack by accident.

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
- [HTTP API](../reference/http-api.md)
- [Decision 0014](../decisions/0014-service-runtime-deferred.md)
- [Decision 0019](../decisions/0019-tokio-and-axum-service.md)
