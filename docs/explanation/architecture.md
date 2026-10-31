---
title: "Architecture"
kind: "explanation"
audience: [contributor, maintainer, operator, agent]
canonical_for: [system_architecture]
requires: []
---

# Architecture

The workspace is two crates: a library, `libpachinko`, and a service, `pachinko`, built
around it. The library has no process of its own: no `main`, no clock of its own, no
effect, and nothing it can do until a caller calls it. The service is that caller, today
the only one, and the one place in the workspace where an effect may happen.

That split is not a stage on the way to something else; it is the architecture, and it
held when the service gained a runtime. The domain lives where it can be tested without
effects, the runtime and the transport stay in the service, and both crates share one
lockfile and one gate. See
[Decision 0001](../decisions/0001-library-and-service.md) for why there are two crates in
one workspace, [Decision 0003](../decisions/0003-effects-behind-traits.md) for the
effects pattern, [Decision 0008](../decisions/0008-std-library.md) for why the
library is a std crate with its effects kept out mechanically, and
[Decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md) for the service's
runtime and transport. The shape was carried from libpawdoku at `ccdd001`, a single
library crate, and widened to two crates here.

## Workspace

The root `Cargo.toml` is a virtual manifest: it names the members (`crates/*`), the
package fields every crate inherits, and the `[workspace.lints]` table every crate opts
into. `Cargo.lock` beside it is the pin
([decision 0007](../decisions/0007-dependency-policy.md)).

`crates/libpachinko` is the library. It holds the queue `docs/specs/pachinko.allium`
specifies, in five modules: `clock` (the time, the `Clock` effect trait and its fake),
`config` (the specification's five figures), `error` (the rejections), `records` (one pocket's
records, behind the `PocketRecords` trait) and `queue` (the `Queue`, with one method per
trigger). All five implement the one specification module, and the order among them is
the one [Layering and dependency direction](layering.md) describes;
[Specifications](specifications.md) explains why the specification comes first.

`crates/pachinko` is the service. It depends on `libpachinko` and on the runtime,
transport and Arrow crates decisions
[0019](../decisions/0019-service-runtime-tokio-and-flight.md) and
[0020](../decisions/0020-records-as-arrow-batches.md) chose, and `libpachinko` never
depends on it. `src/lib.rs` holds `run()` and `serve()`, `src/cli.rs` reads the command
line, `src/clock.rs` and `src/arrow.rs` hold what the service hands the library,
`src/actor.rs` the queue's owner task and ticker, and `src/flight.rs` the Flight service.
`src/main.rs` holds the binary's `main`, which calls `run()` and reports its error.

## A std library

`libpachinko` is an ordinary standard-library crate. That keeps it simple to write and to
test, and it means the clock, the filesystem, the environment, the network and threads
are all nameable from inside it: nothing in the language stops a call to
`std::time::Instant::now` or `std::fs::read`. So the effects are not unreachable; they
are banned, and a gate holds both the crate bans and the std-API bans, with review
covering what the gate cannot see.
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
read the environment or spawn a thread, and it hands each implementation to the
library through the library's trait. The library never learns which implementation it
was given.

One effect trait exists: `Clock`, in `libpachinko::clock`. The queue reads the time
through it and nowhere else; `FakeClock` beside it is moved by hand in the library's
tests, and the service's `SystemClock` reads the system's wall clock. No rule of the
specification needs another effect, so no other trait exists. Each arrives with the
specification change that names its effect, bringing its trait, its fake and the
service's implementation together
([decision 0003](../decisions/0003-effects-behind-traits.md)).

`PocketRecords` is a trait the library owns too, but not an effect trait: a pocket's
records are held in memory either way. It exists so that the service's store can sit
outside the library. The maintainer asked for each pocket's records to be an Arrow
`RecordBatch`, and `arrow-array` turns on `getrandom`, a crate gate 8 bans from
libpachinko's tree, so `ArrowRecords` lives in `pachinko` and `MemoryRecords`, the
trait's reference, lives beside the trait
([decision 0020](../decisions/0020-records-as-arrow-batches.md)).

Standard input and output are not on the list. The service writes its lines to standard
output and its errors to standard error, and nothing bans the library from writing to a
writer it is handed.

## Gate 8: how the boundary is enforced

`just effects` is gate 8, and it runs `scripts/check-effects.sh`
([decision 0008](../decisions/0008-std-library.md)). The bans apply to `libpachinko`
only; `pachinko` is allowed every effect.

| Banned in `libpachinko` | Which |
| --- | --- |
| Crates, through its normal dependency tree | `rand`, `rand_core`, `getrandom`, `tokio`, `async-std`, `smol`, `mio` and `socket2` |
| std APIs | `std::time::SystemTime::now`, `std::time::Instant::now`, `std::time::Instant::elapsed`, `std::time::SystemTime::elapsed`; everything under `std::fs`, `std::env`, `std::net` and `std::process`; `std::thread::spawn`, `std::thread::scope`, `std::thread::sleep` and `std::thread::Builder` |

The table names what is banned; `crates/libpachinko/clippy.toml` is the list, 121 item
paths. Where another path reaches the same effect it is banned too: the `Path`
filesystem queries, the unix `fs` and `net` extensions, `std::hash::RandomState`,
`std::thread::available_parallelism` and the timed `std::sync` waits. A constant cannot
be banned, so `std::env::consts` stays reachable; it holds compile-time values, not an
effect.

Development dependencies are exempt from the crate bans, so a test may use a seeded
generator; the std-API bans still reach libpachinko's own tests, by the maintainer's
choice on 2026-10-24, and
[Quality gates](../reference/quality-gates.md#gate-8-the-effects-bans) says where a test
that needs a real effect goes. Standard I/O
(`std::io::stdin` and the rest) is not banned, because no named effect covers it.
`std::backtrace::Backtrace::capture` reads `RUST_BACKTRACE` but stays allowed, by the
maintainer's choice: it changes diagnostics, not results, so libpachinko should not
branch on its `status()`.

**The crate bans.** The script reads `cargo tree -p libpachinko -e normal
--all-features --target all --prefix none --locked --offline` and fails if any line
names a banned crate, matched as a whole name so that `rand` never matches `rand_core`.
That covers transitive and optional dependencies on every target, and is scoped to
`libpachinko` by construction. A positive control comes first: a tree that does not list
`libpachinko` itself proves nothing, so the script fails on one. This is the only crate
guard: `deny.toml` bans no effect crate, because its bans are workspace-wide and its
`wrappers` exempt only a banned crate's direct parents, so a ban there would also reject
the crates the service reaches transitively. Its one ban is `openssl-sys`, everywhere.
The crate bans stay a denylist of the eight names above rather than an allowlist, by the
maintainer's choice on 2026-10-24, so a crate outside the list is left to review.

**The std-API bans.** They are clippy's `disallowed-methods` and `disallowed-types`, in
`crates/libpachinko/clippy.toml`, each with a reason that begins `effect:`. Clippy reads
the first `clippy.toml` it finds walking up from a crate's manifest, so that file governs
libpachinko alone, and the service, linted with the root file, keeps every effect.
Clippy resolves names, so an alias, a glob import, a macro body, a function value and a
method on a value passed in are all caught. Because the lists name items rather than
modules, a std item added in a later Rust is not banned until it is listed. Clippy lints
only what it builds, so code behind a target the host does not compile, under a mix of
features none of gate 8's passes sets, or under `cfg(not(clippy))` or a custom `cfg` is
not linted. Review covers all of these. Leaving another target's `cfg` to review and
`std::env::consts` reachable are the maintainer's choice on 2026-10-24
([decision 0008](../decisions/0008-std-library.md)).

**The probe.** A ban that cannot be shown to fire is not trusted to hold. A fixture under
`tests/fixtures/effects-violation/` breaks every ban exactly once, and the gate fails
unless every ban fires there and none fires on `crates/pachinko`.
[Quality gates](../reference/quality-gates.md#gate-8-the-effects-bans) lists the script's
checks in order.

## What crosses the boundary

| Caller | Crosses | Through |
| --- | --- | --- |
| `pachinko` | Values, the real implementation of the library's effect trait (`SystemClock` for `Clock`) and its record store (`ArrowRecords` for `PocketRecords`) | A Cargo path dependency on `libpachinko` |

Every public type is `Send + Sync + 'static`, `Clone` and `Debug`, and every public enum
and struct is `#[non_exhaustive]`: the service may move a value between threads, copy it
or print it, and a new field or variant must not break a caller that matched on the
old ones. `AGENTS.md` states this as invariant 3.

## The service today

`pachinko::run` takes the command-line arguments and any writer. With no arguments it
writes one line, `pachinko <version>`, with the library's version, and builds no
runtime. With `serve` it builds a multi-thread tokio runtime, binds its listener
(`127.0.0.1:0` unless `--listen` names another address), writes
`listening on <address>`, and runs `pachinko::serve` until an interrupt; its other
options are the queue's five figures. `main` calls `run` on locked standard output with
the arguments after the program's name; an error becomes one line on standard error and
exit status 1.

`serve` is the shape [decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md)
chose. One task owns the service's `Queue`, over `ArrowRecords` and `SystemClock`, and
every trigger reaches it as a command on one bounded channel, with a one-shot reply: the
Flight handlers send the parties' commands, and a ticker task sends `Tick`, which
processes lapses and decay, once a second. A lapse and a consumer's action on the same
lease therefore meet in one stream, in arrival order, as the specification's expiry
comment requires, and a lapse is processed up to one tick after its lease's expiry. Both
tasks live in a `JoinSet` that `serve` owns and joins before it returns, so no task
outlives it. On shutdown tonic finishes its graceful shutdown first, then the ticker
stops and the owner drains the commands already sent and ends; a request whose handler
has not begun, or an upload still open, is answered `UNAVAILABLE` rather than holding
the shutdown open.

The Flight service carries the specification's two surfaces over gRPC: `do_put` is a
publish, `do_get` a pull, and `do_action` carries the joins, the consumer's four lease
actions and a `status` read, each body and reply an Arrow IPC stream. The
[API reference](../reference/api.md) lists the mapping and its status codes, and the
[Security model](security-model.md) says what an unauthenticated listener means.

Unit tests in `src/lib.rs` check `run` against a buffer; `tests/binary.rs` runs the
built binary, `tests/flight.rs` drives `serve` with a Flight client over loopback, and
`tests/arrow_records.rs` runs the library's records contract over `ArrowRecords`.
[Test and debug](../how-to/test-and-debug.md) shows how to tell their failures apart.

## What is deferred

The prototype answers what [decision 0014](../decisions/0014-service-runtime-deferred.md)
deferred, and [decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md)
supersedes it; that record names what would reopen the choice: more than one queue per
process, a queue that outgrows one core, a client that cannot speak Flight, or a
deployment beyond one host, which also needs authentication and TLS. Nothing persists:
the queue lives in the owner task's memory and ends with the process, and nothing bounds
that memory. Superseded records and dead letters decay on their TTLs; what more a bound
should do is the specification's capacity open question. No surface reads the
superseded records or the dead letters, which waits on the question of who reads them.
Comparing `ArrowRecords` with `MemoryRecords`, or with any other store behind the trait,
is a later ticket ([decision 0020](../decisions/0020-records-as-arrow-batches.md)).
[Purpose and scope](../project/purpose-and-scope.md) records the rest of what this
repository deliberately does not do.

## What wave 2 added

The container image's entrypoint is the `pachinko` binary, built in CI on every pull
request without pushing it
([decision 0015](../decisions/0015-container-image.md)). The release process tags each crate on its own
and cuts a GitHub Release, with `publish = false` kept
([decision 0016](../decisions/0016-release-process.md)).

## Related pages

- [Layering and dependency direction](layering.md)
- [Specifications](specifications.md)
- [Security model](security-model.md)
- [Repository map](../project/repository-map.md)
- [Quality gates](../reference/quality-gates.md)
- [Test and debug](../how-to/test-and-debug.md)
- [API reference](../reference/api.md)
- [Decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md)
- [Decision 0020](../decisions/0020-records-as-arrow-batches.md)
