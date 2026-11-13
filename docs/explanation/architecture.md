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
nothing it can do until a caller calls it. The service is that caller, today the
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

`crates/libpachinko` is the library. It holds the domain `docs/specs/pachinko.allium`
defines, in the modules [The library](#the-library) lists. A module may use only what its
specification imports, in the direction
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
read the environment or spawn a thread, and it will hand each implementation to the
library through the library's trait. The library never learns which implementation it
was given.

Two traits exist: the clock and the lease-id source, described in
[The library](#the-library). Further traits arrive with the specifications that name
the effects: a specification change that needs another effect brings its trait, its fake
and the service's implementation together
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

## The library

`libpachinko` implements `docs/specs/pachinko.allium` behind three surfaces, one for
each kind of reader: the crate root for the caller who drives a queue, one public module
per seam for whoever supplies an effect or a store, and `testing` for a test that checks
the queue from outside.

| Surface | Holds |
| --- | --- |
| The crate root | `Queue`, its operations and the values they take and return: `Publication`, `PublishOutcome`, `PullOutcome`, `LeaseView`, `SettleOutcome`, `PocketFate`, `Lease`, `LeaseStatus`, `TickReport` |
| The crate root | The names and values: `PublisherId`, `ConsumerId`, `PocketKey`, `RecordKey`, `Rank`, `Payload`, `HeldRecord`; `Timestamp`, milliseconds since the Unix epoch; `LeaseId`, an opaque `u128` written as 32 lower-case hex digits, and `ParseLeaseIdError` |
| The crate root | `Config`, the specification's five values with its defaults, and `Config::check` |
| The crate root | One `thiserror` enum per operation: `JoinError`, `PublishError`, `PullError`, `LeaseActionError`, `ConfigError` |
| The crate root | `version()` |
| `clock` | The `Clock` trait; `FakeClock` |
| `lease_ids` | The `LeaseIds` trait; `CountingLeaseIds` |
| `records` | The store seam: the `PocketRecords` trait, `MemoryRecords`, and the values only the store speaks in, `RecordView`, `RecordStatus` and `Position` |
| `testing`, behind the `testing` feature | `Snapshot`, `PocketView`, `TickCost`, `InspectRecords`, `check_contract`, and `PocketStatus`, `ShedReason`, `SupersededRecord` and `DeadLetter`, which no other surface exports |

Each item has exactly one public path, and one `use libpachinko::{…}` names `Queue`,
`Config` and every value an operation takes, returns or refuses with; the clock, the
lease-id source and the store a queue is built over come from the seam modules. `lib.rs`
re-exports the root's items from five private modules:
`queue` holds `Queue` and the values of the first row; `ids` the names and values with
`HeldRecord`, `LeaseId` and `ParseLeaseIdError`; `timestamp` holds `Timestamp`; `config`
holds `Config`; and `error` the five enums. `LeaseId` sits in `ids` rather than in a
module of its own because gate 7 measures coupling for each top-level module: with a
third value module `error` imported two modules, which made it less stable than `config`,
its dependent, and failed the stable-dependencies check, and it left `queue` exactly at
the instability cap. A seam module holds its trait and the fake or the
implementation beside it, and `records` also the values only the store speaks in.
[Layering and dependency direction](layering.md#enforcement) states the rule the
re-exports bring. The `testing` feature is additive: it adds the `testing` module and two
`Queue` methods, `snapshot` and `tick_with_cost`, and changes nothing the other two
surfaces hold. [Testing](../reference/testing.md#tests-from-the-specifications) says what
the tests do with it.

**Two effect traits, each with a fake.** `Clock` gives the queue its `now`; the service's
implementation reads the system time, and `FakeClock`, shared through an `Arc` so a test
keeps a handle, moves only when a test moves it. The queue clamps the clock: it keeps the
latest reading it has seen, so a clock that steps back stamps nothing before an earlier
stamp. `LeaseIds` hands out the lease ids that fence an ack, nack, renew or release to
its lease; the service's source draws an epoch once from the operating system's random
source and counts a serial beside it, and `CountingLeaseIds` counts 1, 2, 3. Both traits
are `Clone + Debug + Send + Sync + 'static`, so a queue over them meets invariant 3.

**The store seam.** A pocket's records live behind `PocketRecords`, the record-level
steps the rules take: look a key up, insert it queued, supersede it, hold the oldest
queued records, settle, defer, and free every held and deferred record in
O(outstanding). The queue decides every rule and asks the store for one step; the store
decides nothing. `MemoryRecords` is the one store (decision 0019). The trait holds only
the steps the queue calls: reading every record back is `testing::InspectRecords`, a
second trait a store implements for tests, as `MemoryRecords` does behind the `testing`
feature. `testing::check_contract` is the one contract every store passes, over a store
that implements both traits, and it refuses a store that could hand out a held record
twice, keep two records for one key, or share state between clones.

**`Queue<R, C, L>`** is generic over the store, the clock and the lease-id source. It is
plain data with `&mut self` methods, so its owner applies one operation at a time, which
is how decision 0019 serializes a lapse with a consumer's action. It holds the
publishers, the consumers with their active lease, the pockets with the pocket order, the
lease registry indexed by expiry, the shed queue of superseded records, and the one
`next_position` counter pockets and records share. A clone forks all of that; the two
share only what the clock and lease-id source share between clones. Behind `testing`,
`Queue::snapshot` shows every entity as values, for tests, over a store that implements
`InspectRecords`, and `Queue::tick_with_cost` returns a tick's `TickReport` beside a
`TickCost`, the entries the tick read to find what was due.

**The lease actions and fencing.** `ack`, `nack`, `renew` and `release` each name the
lease they act under, and check, in the order of the rules' `requires` and before any
effect, that the consumer has joined, that the lease is in the active registry, that it
is the consumer's, and, for ack and nack, that its pocket holds the record. A refused
action changes nothing, its lease's `expires_at` included. The registry holds only
active leases: an ended lease leaves both its id index and its expiry index and is
returned to the caller as it ended, never kept. So an ended lease is unknown rather than
remembered, one refusal covers a lease that ended and one that never existed, the
registry and a tick's lapse walk stay the size of the live leases, and an action
for an old lease cannot land on a new lease over the same pocket, because the new lease
has a new id. No action compares `expires_at` with the clock: a lease the registry
holds is active, and the owner's serialization orders a late action against a lapse
(decision 0019). A nack's dead letter sits in the shed queue beside the superseded
records, keyed by expiry and arrival for the decay.

**The tick.** `Queue::tick` is the clock's trigger. It reads `now` once, walks the
expiry index from its earliest entry and stops at the first one later than `now`, so a
tick costs the leases due, never the leases active. Each due lease ends as expired and
leaves the registry, so it lapses once; its pocket's expiry count rises, its held and
deferred records return to queued, and the pocket parks at `max_consecutive_expiries`,
keeps its position, which is the front, with records, or goes idle without. Then both
shed queues decay by removing entries from the front while their `expires_at` has
passed, stopping at the first that has not. A tick is
one operation like any other, so it never interleaves with an action.

**A rule's code may land before its owner.** A rule is owned by the ticket whose test
pins it. Every publish into an existing pocket ends in the `PocketTouched` dispatch, and
its idle and parked arms, `ReviveIdlePocket` and `ReviveParkedPocket`, landed with
publish in T23. A release first makes a pocket idle, and T25 owns `ReviveIdlePocket`
and its test; a lapse first parks one, and T26 owns `ReviveParkedPocket`.

## What crosses the boundary

| Caller | Crosses | Through |
| --- | --- | --- |
| `pachinko` | Values, and the real implementations of the library's effect traits | A Cargo path dependency on `libpachinko` |

Every public type is `Send + Sync + 'static`, `Clone` and `Debug`, and every public enum
and struct is `#[non_exhaustive]`: the service may move a value between threads, copy it
or print it, and a new field or variant must not break a caller that matched on the
old ones. `AGENTS.md` states this as invariant 3.

## The service

`pachinko serve` is an HTTP/1.1 JSON server over one in-memory queue, built as
[decision 0019](../decisions/0019-service-runtime-tokio-and-http.md) decided. `main` makes
one call, `exit_code(run(args, stdout), stderr)`: with no argument `run` writes
`pachinko <version>`; with `serve` and its flags ([Configuration](../reference/configuration.md#service-flags))
it builds a multi-thread tokio runtime and serves until SIGTERM or SIGINT.

- **The owner task.** One task owns the `Queue<MemoryRecords, SystemClock,
  EpochSerialLeaseIds>` and applies every command synchronously, in arrival order, from
  a bounded channel of 1,024 commands, each with a `oneshot` reply. There is no `Mutex`:
  a handler holds only a `QueueHandle`, a clone of the channel's sender, so a lapse
  and a consumer's action are totally ordered. A full channel makes a handler wait, never
  refuses it. A pull whose requester has gone before the owner reaches it is skipped.
- **The tick lane.** A ticker task sleeps `--sweep-ms`, sends one tick on a channel of
  its own that holds one, waits for the owner to apply it and answer, then sleeps
  again. The owner polls that lane before the commands, so a tick waits behind at most
  the command being applied. It is not the command channel because a tick queued there
  waits behind the whole backlog: PR 38's tick did, and its lapse lag grew to seconds
  under load (decision 0019). The ticker never has a second tick in flight, so commands
  get at least a full sweep between two ticks however long one takes. A tick that
  lapsed or decayed anything logs one `info` line with its three counts. The ticker
  ends when the shutdown begins.
- **The real effects.** `SystemClock` reads `SystemTime` in milliseconds since the Unix
  epoch, behind the queue's clamp, which never lets the queue's time step back;
  `EpochSerialLeaseIds` draws a 64-bit epoch from the operating system's random source
  once and pairs it with a serial from 1. `serve` refuses to start when the clock reads
  before the epoch or the random source fails, and the owner stops rather than reissue a
  lease id once the serial is exhausted.
- **The task scope.** `serve` holds the owner, the ticker, the listener and the signal task in one
  `JoinSet`. The first to end starts the shutdown: `GET /readyz` answers 503 at once, the
  listener stops accepting, lets in-flight requests finish, cuts connections still open
  after `--shutdown-grace-ms`, and drops the router, whose handle was the owner's last
  sender, so the owner ends too. A task that panicked comes back as an error, and the
  binary exits 1. [HTTP API](../reference/http-api.md#shutdown)
  owns the shutdown's contract.
- **The wire.** axum routes each request to one command and maps every refusal through
  one exhaustive table of codes; [HTTP API](../reference/http-api.md) owns the routes,
  bodies and codes. `--request-timeout-ms` answers a request still running at
  the bound with 408 `request_timeout` and closes an idle connection, and `GET /readyz` reports whether the owner is
  serving. Each request is logged at `debug`, below the default `info`, which
  `--log-level` sets.

The tests sit in three places. Unit tests live in the modules: the flag parser, the
clock, the lease ids, the owner's ordering, skipping, back-pressure and exhaustion, the
tick lane under a command backlog and the ticker's ends, the error map with one test per code, and the task scope's panic, stop and cut paths.
`tests/http.rs` starts `serve` in-process on port 0 and drives every route with a
blocking client. `tests/binary.rs` runs the built binary: its version line, a usage
error, and `serve` answering `/healthz` and exiting 0 on SIGTERM and on SIGINT.
[Test and debug](../how-to/test-and-debug.md) shows how to run each.
[Purpose and scope](../project/purpose-and-scope.md) records what this repository
deliberately does not do.

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
- [HTTP API](../reference/http-api.md)
