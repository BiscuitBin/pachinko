---
title: "Maintenance"
kind: "operations"
audience: [maintainer, operator, agent]
canonical_for: [maintenance_routine]
requires: []
---

# Maintenance

There is nothing running to operate. The library runs inside whoever calls it, the service
binary is built and tested by the gate but deployed nowhere, nothing accumulates between
changes, and there is no on-call. The one thing deployed is the API reference, which a
workflow publishes on every push to `main`; see
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md). What follows is upkeep of
the pins that build and check it. The routine is the one carried from libpawdoku at
`ccdd001`, with this repository's names.

## Routine

**Weekly.** Read the week's run of the `audit` workflow. It runs `just audit` against the
RustSec advisory database every Monday, as well as on every pull request, and it is never
a required check, so a failure there blocks nothing and is easy to miss. An advisory
against a crate in `Cargo.lock` is fixed by moving that crate, not by ignoring it.

**Monthly.** Move the dependency pins, one kind at a time, as
[Maintain dependencies](../how-to/maintain-dependencies.md) describes. `just lock-upgrade`
moves both lockfiles within the manifests' constraints — `Cargo.lock` within the caret
ranges, `pixi.lock` beneath the exact tool pins; run `just check` and commit both. A tool
pin itself moves in `pyproject.toml`, then `pixi update <name>`, then `pixi.lock` is
committed. A line in `tools.txt` or `tools-source.txt` moves by hand and takes effect at
the next `just install-tools`; a `tools-source.txt` line moves one release at a time,
with `just check` run on each, because rustqual is a gate. A hook `rev` or a GitHub
Action SHA moves by hand, with its version comment moved beside it.

**Monthly.** Run `just check-links-online`. It is not part of the gate because it needs
the network, so external links rot silently until someone looks.

**Per toolchain release.** Bump `channel` in `rust-toolchain.toml` and `rust-version` in
the root `Cargo.toml` together, run `just install-toolchain`, then run `just clippy`
before anything else: clippy's lint set moves with the release, and a lint that is new
under `-D warnings` fails the gate with no change to the code. `just check` follows.

**Per Allium release.** The checker's version and its checksums live in the
`biscuit-games-tooling` package, not here; moving that package's pin in `pyproject.toml`
moves the checker, and `just install-allium` installs it, as
[Moving the Allium binary](../how-to/maintain-dependencies.md#moving-the-allium-binary)
describes. A moved checker re-verifies every module under `docs/specs/`. The modules carry
no waiver today; any waiver added later is valid only against the version it was verified
on, so each is checked again.

Nothing here moves on its own: no bot opens a pull request for any of these pins until
ticket T17 brings Renovate.

## Secrets

None are stored, and there is nothing to rotate. Continuous integration uses the token
GitHub mints for each run and discards with it; the Pages deployment's write scopes come
from that token too. Both crates are `publish = false`, so there is no registry token
either.

## Related pages

- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md)
- [Troubleshooting](troubleshooting.md)
- [Security model](../explanation/security-model.md)
