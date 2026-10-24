---
title: "Architecture"
kind: "explanation"
audience: [contributor, maintainer, operator, agent]
canonical_for: [system_architecture]
requires: []
---

# Architecture

The workspace is two crates: a library, `libpachinko`, and a service, `pachinko`, built
around it. The library has no process of its own: no `main`, no clock, no effect, and
nothing it can do until a consumer calls it. The service is that consumer, today the
only one, and the one place in the workspace where an effect may happen.

That split is not a limitation waiting for a runtime to be added; it is the
architecture. The domain lives where it can be tested without effects, the service stays
thin, and both share one lockfile and one gate. See
[Decision 0001](../decisions/0001-library-and-service.md) for why there are two crates in
one workspace, [Decision 0003](../decisions/0003-effects-behind-traits.md) for the
effects pattern, and [Decision 0008](../decisions/0008-std-library.md) for why the
library is a std crate with its effects kept out mechanically. The shape was carried from
libpawdoku at `ccdd001`, a single library crate, and widened to two crates here.

## Workspace

The root `Cargo.toml` is a virtual manifest: it names the members (`crates/*`), the
package fields every crate inherits, and the `[workspace.lints]` table every crate opts
into. `Cargo.lock` beside it is the pin
([decision 0007](../decisions/0007-dependency-policy.md)).

`crates/libpachinko` is the library. It holds the domain once the specifications define
one; today `src/lib.rs` holds the crate's documentation and one item, `version()`.
Behaviour will arrive as one Rust module per specification module, and a module may use
only what its specification imports, in the direction
[Layering and dependency direction](layering.md) describes;
[Specifications](specifications.md) explains why the specification comes first.

`crates/pachinko` is the service. It depends on `libpachinko` and nothing else, and
`libpachinko` never depends on it. `src/lib.rs` holds `run()`, `src/main.rs` holds the
binary's one-line `main`, and `tests/binary.rs` runs the built binary as a user would.

## A std library

`libpachinko` is an ordinary standard-library crate. That keeps it simple to write and to
test, and it means the clock, the filesystem, the environment, the network and threads
are all nameable from inside it: nothing in the language stops a call to
`std::time::Instant::now` or `std::fs::read`. So the effects are not unreachable; they
are banned, and a gate proves both the crate bans and the std-API bans hold.
[Decision 0008](../decisions/0008-std-library.md) records that choice and the
alternative it rejected.

## The effects boundary

An effect is anything a computation reaches outside itself for. Six are named: the clock,
randomness, the filesystem, the network, the environment and threads. `libpachinko`
performs none of them itself. Where the domain needs one, the library defines a trait
that says what it needs, in its own terms, and takes an implementation of that trait from
its caller. A fake sits beside each trait in the library, so a test supplies exactly the
times, draws or contents it wants, built per test, and the same input gives the same
answer on every machine and on every run.

`pachinko` supplies the real implementations. It is the only crate allowed to read the
clock, draw from the operating system's entropy, touch the filesystem or the network,
read the environment or spawn a thread, and it will hand each implementation to the
library through the library's trait. The library never learns which implementation it
was given.

No trait exists yet. The traits arrive with the specifications that name the effects:
a specification change that needs the time, a random draw or a stored value brings its
trait, its fake and the service's implementation together
([decision 0003](../decisions/0003-effects-behind-traits.md)).

Standard input and output are not on the list. The service writes its line to standard
output, and nothing bans the library from writing to a writer it is handed.

## Gate 8: how the boundary is enforced

`just effects` is gate 8, and it runs `scripts/check-effects.sh`
([decision 0008](../decisions/0008-std-library.md)). The bans apply to `libpachinko`
only; `pachinko` is allowed every effect.

| Banned in `libpachinko` | Which |
| --- | --- |
| Crates, through its normal dependency tree | `rand`, `rand_core`, `getrandom`, `tokio`, `async-std`, `smol`, `mio` and `socket2` |
| std APIs | `std::time::SystemTime::now`, `std::time::Instant::now`, `std::time::Instant::elapsed`, `std::time::SystemTime::elapsed`; everything under `std::fs`, `std::env`, `std::net` and `std::process`; `std::thread::spawn`, `std::thread::scope`, `std::thread::sleep` and `std::thread::Builder` |

Development dependencies are exempt, so a test may use a seeded generator. Standard I/O
(`std::io::stdin` and the rest) is not banned, because no named effect covers it.

**The crate bans.** The script reads `cargo tree -p libpachinko -e normal
--all-features --target all --prefix none --locked --offline` and fails if any line
names a banned crate, matched as a whole name so that `rand` never matches `rand_core`.
That covers transitive and optional dependencies on every target, and is scoped to
`libpachinko` by construction. A positive control comes first: a tree that does not list
`libpachinko` itself proves nothing, so the script fails on one. This is the only crate
guard: `deny.toml` bans no effect crate, because its bans are workspace-wide and its
`wrappers` exempt only a banned crate's direct parents, so a ban there would also reject
the crates the service reaches transitively. Its one ban is `openssl-sys`, everywhere.

**The std-API bans.** They are clippy's `disallowed-methods` and `disallowed-types`, in
`crates/libpachinko/clippy.toml`, each with a reason that begins `effect:`. Clippy reads
the first `clippy.toml` it finds walking up from a crate's manifest, so that file governs
libpachinko alone, and the service, linted with the root file, keeps every effect.
Clippy resolves names, so an alias, a glob import, a macro body, a function value and a
method on a value passed in are all caught. Because the lists name items rather than
modules, a std item added in a later Rust is not banned until it is listed, and code
behind a target the host does not compile is not linted; review covers both.

**The probe.** A ban that cannot be shown to fire is not trusted to hold. A fixture under
`tests/fixtures/effects-violation/` breaks every ban exactly once, and the gate fails
unless every ban fires there and none fires on `crates/pachinko`.
[Quality gates](../reference/quality-gates.md#gate-8-the-effects-bans) lists the script's
checks in order.

## What crosses the boundary

| Consumer | Crosses | Through |
| --- | --- | --- |
| `pachinko` | Values, and the real implementations of the library's effect traits | A Cargo path dependency on `libpachinko` |

Every public type is `Send + Sync + 'static`, `Clone` and `Debug`, and every public enum
and struct is `#[non_exhaustive]`: the service may move a value between threads, copy it
or print it, and a new field or variant must not break a consumer that matched on the
old ones. `AGENTS.md` states this as invariant 3.

## The service today

`pachinko::run` takes any writer and writes one line, `pachinko <version>`, with the
library's version. `main` calls it on locked standard output and turns the result into
the process's exit status, and does nothing else. A unit test in `src/lib.rs` checks the
line against a buffer, and `tests/binary.rs` runs the built binary and checks its output,
its exit status and that it writes nothing to standard error.
[Test and debug](../how-to/test-and-debug.md) shows how to tell those two tests' failures
apart.

## What is deferred

The service has no async runtime, transport or HTTP stack, and none is chosen. Choosing
one before the specifications say what the service must expose would be a guess. The
trigger is the first specification that names a surface the service exposes; a spike
then compares the options and a decision record supersedes
[decision 0014](../decisions/0014-service-runtime-deferred.md). Until then the service is
`run()` over a writer. [Purpose and scope](../project/purpose-and-scope.md) records the
rest of what this repository deliberately does not do.

## What wave 2 adds

T13 will add a container image whose entrypoint is the `pachinko` binary, built in CI on
every pull request without pushing it. T14 will add the release process, which tags each
crate on its own and cuts a GitHub Release, with `publish = false` kept. Each ticket
updates its sentence here when it lands.

## Related pages

- [Layering and dependency direction](layering.md)
- [Specifications](specifications.md)
- [Security model](security-model.md)
- [Repository map](../project/repository-map.md)
- [Quality gates](../reference/quality-gates.md)
- [Test and debug](../how-to/test-and-debug.md)
- [API reference](../reference/api.md)
