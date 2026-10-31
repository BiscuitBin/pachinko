# pachinko

[![CI](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml/badge.svg)](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml)

A Rust workspace with two crates: `libpachinko`, the library, and `pachinko`, a service
built around it. Together they are a queue: publishers publish records into pockets, a
pocket keeps one record per record key and a higher rank supersedes in place, and a
consumer leases one pocket at a time and pulls its oldest records in bounded batches.

The Allium specification `docs/specs/pachinko.allium` decides that behaviour, and both
crates implement it as a prototype. The library holds the queue and performs no effect
itself: it reads the time through its `Clock` trait, with a fake beside it, and keeps
each pocket's records behind a record-store trait. The service supplies the system clock
and an Arrow record store, and serves the queue over Arrow Flight on tokio
([decision 0019](docs/decisions/0019-service-runtime-tokio-and-flight.md)). Neither crate
is published to crates.io.

## Quick start

After cloning, follow the prerequisites in `docs/how-to/develop-locally.md`
([Develop locally](docs/how-to/develop-locally.md)), including putting rustup's
`~/.cargo/bin` ahead of `~/.pixi/bin` on `PATH`. From the repository root:

```console
just initialize
just check
```

Initialization needs the network. It installs the pinned tools and dependencies and,
from the primary checkout, the shared pre-commit hook. In a secondary worktree it skips
hook installation. The full check is read-only. It runs offline after this worktree's
tools are installed and the primary checkout has prepared the shared hook environments.

## Running the service

```console
just build
./target/debug/pachinko serve
```

It prints `listening on <address>` and serves until an interrupt (Ctrl-C). It listens on
a free loopback port unless `--listen` names another address, and the Flight service has
no authentication and no encryption; read the
[security model](docs/explanation/security-model.md) before you listen beyond loopback. With no
arguments the binary prints `pachinko <version>` and exits.
[The service's README](crates/pachinko/README.md) lists the options.

## Working on a ticket

Read [AGENTS.md](AGENTS.md), the ticket under `tickets/`, and the sections of
`tickets/CONVENTIONS.md` it cites. Use the ticket's branch in its own worktree, initialize
that worktree, and run `just check` before editing. Install the shared hook from the
primary checkout only. Each ticket names its verification and review requirements.

`just fix` applies automatic repairs; `just check` verifies the result. `just --list`
lists the recipes, and [Commands](docs/reference/commands.md) explains them.

## Documentation

Start at the [handbook](docs/README.md) for development, architecture, specifications
and operations. The [API reference for main](https://biscuitbin.github.io/pachinko/)
documents both crates; `just doc` builds it locally.

See [CHANGELOG.md](CHANGELOG.md) for changes and [SECURITY.md](SECURITY.md) for private
vulnerability reporting.

## Licence

Both crates are licensed under Apache-2.0. See [LICENSE](LICENSE).
