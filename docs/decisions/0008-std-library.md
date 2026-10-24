---
title: "Decision 0008: A std library, with effects kept out mechanically"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_std_library]
requires: []
---

# Decision 0008: A std library, with effects kept out mechanically

## Context

[Decision 0003](0003-effects-behind-traits.md) puts every effect behind a trait the
library owns. That rule holds only if something stops library code from reaching an
effect directly, and there are two ways to stop it: build the library without the
standard library, so that the clock, the filesystem, the network, the environment and
threads cannot be named from inside it; or keep the standard library and ban each effect
by name, with a gate that proves every ban fires.

The library this repository was carried from took the first way. It was a `no_std` core,
proved by a check build for a wasm target with no standard library, because it was
consumed from a browser and from other languages, and those consumers asked that of its
types. It paid for that in `alloc` collections, an ordered map in place of a hashed one,
float maths from a separate crate, a step budget wherever a timeout would have been
simpler, lints against every `std` path, and a gate of its own for the target check. Even
then the target check was not the whole guard: `rand` and `getrandom` were banned by
cargo-deny beside it, and the seed came from the caller.

Here no consumer asks for any of that. The library's one consumer is the `pachinko`
service, a std binary ([decision 0001](0001-library-and-service.md)), and whatever
runtime it is given later will be std too
([decision 0014](0014-service-runtime-deferred.md)). `rust-toolchain.toml` names no
target beyond the host's. The maintainer decided on 2026-10-21 that libpachinko is a std
library: no `#![no_std]`, no wasm targets and no wasm check. Without `no_std` the
effects are nameable, so keeping them out has to be a check rather than a property of
the build.

## Decision

- `libpachinko` is an ordinary std crate. It carries no `#![no_std]`, the toolchain pins
  no extra target, and the `Justfile` has no target-check recipe.
- The effects are named and banned in `libpachinko` alone, by gate 8: `just effects`,
  which runs `scripts/check-effects.sh`. It is one of `just check`'s recipes and a step
  of CI's `rust` job. `pachinko` is allowed every effect.
- The bans:
  - **Crates**, through libpachinko's normal dependency tree: `rand`, `rand_core`,
    `getrandom`, `tokio`, `async-std`, `smol`, `mio` and `socket2`. Development
    dependencies are exempt, so a test may use a seeded generator.
  - **std APIs**: `std::time::SystemTime::now`, `std::time::Instant::now`,
    `std::time::Instant::elapsed`, `std::time::SystemTime::elapsed`, everything under
    `std::fs`, `std::env`, `std::net` and `std::process`, `std::thread::spawn`,
    `std::thread::scope`, `std::thread::sleep` and `std::thread::Builder`. Standard I/O
    is not banned, because no named effect covers it.
- The crate bans are one line in the script, `cargo tree -p libpachinko -e normal
  --all-features --target all --prefix none --locked --offline`, which covers transitive
  and optional dependencies on every target and is scoped to libpachinko by
  construction. It is the only crate guard. `deny.toml` cannot hold these bans: its bans
  apply to the whole workspace, and its `wrappers` exemption names a banned crate's
  direct parents only, so it cannot say "anything under `pachinko`". A runtime crate the
  service reaches through a framework it depends on would still be denied. `deny.toml`
  therefore bans no effect crate.
- The std-API bans are clippy's `disallowed-methods` and `disallowed-types`, listed in
  `crates/libpachinko/clippy.toml` with reasons that begin `effect:`. Clippy reads the
  first `clippy.toml` it finds walking up from a crate's manifest and merges nothing, so
  that file governs libpachinko alone, restates every line of the root file, and leaves
  `crates/pachinko`, where every effect is allowed, under the root file. Clippy resolves
  names, so an alias, a glob import, a macro body, a function value and a method on a
  value passed in are caught as surely as a full path. rustqual's `forbid_path_prefix`
  rules, the alternative, were rejected on evidence: they match text, missed
  `use std as s`, `use std::*`, a macro body and a renamed import, and could ban
  `elapsed` only by name, on every type.
- Clippy needs item paths, not module prefixes, so "everything under" a module is a list
  of its stable items, 121 of them on the pinned toolchain. A std item added in a later
  Rust is not banned until it is listed, and a constant cannot be banned at all, so
  `std::env::consts` stays reachable: it holds compile-time values, like `cfg!`, not an
  effect.
- A ban that is not shown to fire is not trusted to hold. The script also runs its checks
  over `tests/fixtures/effects-violation/`, a fixture that breaks every ban exactly once,
  in the forms code writes it, and fails unless each ban fires there; then it checks that
  none fires on `crates/pachinko`. The crate bans are probed by the same `cargo tree`
  line over the fixture, which is its own workspace with a committed `Cargo.lock` and an
  empty offline path dependency named for each banned crate, one of them reached only
  transitively.

## Consequences

The library is written in ordinary Rust: `std` collections and `std::error::Error`, no
`alloc` substitution and no lints against `std` paths. `HashMap` and `HashSet` are still
disallowed by `clippy.toml`, for their iteration order rather than for any target. A
timeout needs the clock, which is banned, so it comes through a clock trait rather than
being impossible.

The effects are nameable, so a direct call compiles and gates 6 and 8 reject it. The
protection is as good as the list: a new kind of effect needs a decision record first
(`AGENTS.md`), and with it a ban in the script or in `crates/libpachinko/clippy.toml`
and a line in the fixture that breaks it. What the lists cannot see is caught by review
alone, since clippy lints only what it builds: a std item added after the inventory,
code behind a target the host does not compile, code under a mix of features none of
gate 8's three passes sets, and code under `cfg(not(clippy))` or a custom `cfg`. That is
why the `review-change` skill still tells a reviewer to read libpachinko's diff for a
direct std call.

Two mechanisms hold the boundary where a target check held most of it before: the crate
line and clippy's disallowed paths, each with its probe. Both run offline once
`just sync` has fetched the tree, so `just check` stays offline. Every change to the
root `clippy.toml` is mirrored in libpachinko's, and gate 8 fails when the two drift.

## What would reopen this

A target that needs `no_std`: an embedded consumer of the library, or a wasm consumer
running where no standard library exists. The question would then be whether the whole
library goes without `std` or a sibling crate holds the part that consumer needs, and
that is the maintainer's to answer when the consumer is named.

Or a ban that cannot be made to fire: if no mechanism makes one of the named bans fire on
the fixture in the form code writes it, that ban is a convention, not a check, and the
choice between a reviewed rule and building without `std` returns.

## Related pages

- [Architecture](../explanation/architecture.md)
- [Quality gates](../reference/quality-gates.md)
- [Configuration](../reference/configuration.md)
- [Decision 0003: Effects behind traits](0003-effects-behind-traits.md)
- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
