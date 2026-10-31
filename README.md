# pachinko

[![CI](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml/badge.svg)](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml)

A Rust workspace with two crates: `libpachinko`, the library, and `pachinko`, a service
built around it. The library performs no effects itself: it reaches the clock and its
lease-id source through the traits `Clock` and `LeaseIds`, with fakes beside them, and
the service supplies their real implementations.

The domain is a single-deployment queue of pockets: publishers publish records into
pockets, a higher rank supersedes a record in place, and a consumer leases one pocket at
a time, pulls its oldest records in batches and acks, nacks, renews or releases them.
The Allium module `docs/specs/pachinko.allium` decides that behaviour, and the library
implements it as a synchronous `Queue`. `pachinko serve` runs it as an in-memory HTTP
service ([HTTP API](docs/reference/http-api.md)); with no argument, `pachinko` writes
`pachinko <version>` and exits. Neither crate is published to crates.io.

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
