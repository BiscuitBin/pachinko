---
title: "Decision 0019: The service runs on tokio and serves HTTP with axum"
kind: "decision"
audience: [contributor, maintainer, operator, agent]
canonical_for: [decision_service_runtime]
requires: []
---

# Decision 0019: The service runs on tokio and serves HTTP with axum

## Context

[Decision 0014](0014-service-runtime-deferred.md) left the service without a runtime,
transport or HTTP stack until the first specification named a surface the service
exposes. `docs/specs/pachinko.allium` now names two, `Publishing` and `Consuming`, so the
trigger has fired. The maintainer asked on 2026-10-31 for a prototype that a publisher and
a consumer can drive over HTTP, and chose tokio; this record runs the spike decision 0014
names, records why tokio, and chooses the HTTP stack.

Two readings shaped the choice:

- Martin Kleppmann, "How to do distributed locking",
  `https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html`. A lease is
  a fencing token. The service mints an opaque lease id on every claim, a batch names it,
  and every ack, nack, renew and release carries it back, so an action naming an ended
  lease is refused rather than landing on a successor lease over the same pocket (the
  module's `ActionsNameTheirLease`). Expiry is judged by the service's clock alone.
- Nathaniel J. Smith, "Notes on structured concurrency, or: Go statement considered
  harmful", `https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/`.
  No task outlives the function that started it.

The module's Excludes leave transport, wire format and client authentication to this
spike; the behaviour stays the module's.

## The spike

Each option was judged on the four things decision 0014 names: the dependency tree
`just deny` must accept, the test story, maintenance, and fit with an effects boundary
that keeps the runtime out of the library.

| Option | Dependency tree | Test story | Maintenance | Effects fit |
| --- | --- | --- | --- | --- |
| tokio with axum | 61 crates in the service's normal tree; `just deny` passes with the existing licence allow-list and the `openssl-sys` ban, with no TLS | An in-process server on port 0 driven by a blocking client; `#[tokio::test]`; timers and signals in the runtime | The most used Rust runtime and its HTTP router, released together with hyper | The runtime, the listener, the sweep's timer and the signals live in `pachinko`; `just effects` already bans `tokio` from libpachinko's tree |
| Plain threads with a hand-written HTTP/1.1 loop | No new crate | Blocking and simple, but graceful shutdown needs a non-blocking accept or a wake-up connection, and every parsing case is ours to test | Request parsing, keep-alive, limits and timeouts written and kept here | Fits: std threads and `std::net` in the service only |
| `async-std` or `smol` | `async-std` is deprecated in favour of `smol` on crates.io; `smol` with hyper needs glue crates and has no router | As tokio, with fewer test helpers | A smaller ecosystem around the HTTP stack, and the glue is ours | Fits as tokio does |

tokio with axum wins on everything but tree size, and the tree is the cost this record
accepts.

## Decision

The service runs on a multi-thread tokio runtime and serves HTTP/1.1 with axum. Each
crate enters through `[workspace.dependencies]` with its
[decision 0007](0007-dependency-policy.md) note: `tokio` (runtime, listener, timer,
signals, shutdown channel), `axum` (`http1`, `json` and `tokio` features only), `serde`
(the bodies), `base64` (payloads are bytes and JSON has none), `getrandom` (the lease-id
source draws 128 bits from the operating system; a smaller tree than `rand` for one
call), `tracing` and `tracing-subscriber` (the log lines, since the lint table forbids
`eprintln!`), and, for tests only, `serde_json`, `ureq` (a blocking client, without TLS)
and `proptest` in the library. `thiserror` derives both crates' error enums
(`AGENTS.md`, invariant 10).

The task scope is a rule. `pachinko::serve` starts the listener, the sweep and the
signal handling in one `JoinSet`; the first to end starts the shutdown, every task is
joined before `serve` returns, and the first error is the one returned. axum serves each
connection on a task of its own that the listener waits for, so a connection still open
after a bounded grace period is cut, and that wait ends too. Nothing is aborted: a
connection busy in code that never reads or writes holds `serve` until it finishes. No task is spawned anywhere else, and a task whose handle nobody joins is a
defect in review.

The lease id is a fencing token: 128 random bits, never reissued, named by every action
on the lease. The library takes it through its `LeaseIds` trait and the clock through
`Clock` ([decision 0003](0003-effects-behind-traits.md)); the service supplies
`RandomLeaseIds` and `SystemClock`. The library stays synchronous; the service holds the
queue behind a `std::sync::Mutex` that no handler holds across an `.await`.

The wire shape belongs to [HTTP API](../reference/http-api.md): the routes, the bodies and
the mapping from each refusal to its status. It is a prototype's choice, and a later
record can replace it without touching the library.

## Consequences

- The service's dependency tree is now the audit surface: 61 crates where there was one,
  each judged by `just deny` and `just deps-unused`.
- No native TLS. The listener speaks plain HTTP; `openssl-sys` stays denied, and serving
  beyond localhost is an operator's choice to put behind a proxy
  ([Security model](../explanation/security-model.md)).
- The bare binary and the container image are unchanged: with no argument, `pachinko`
  still writes `pachinko <version>` and exits 0. `pachinko serve` is the new command.
- The library is still synchronous, still free of every runtime, and still tested through
  fakes; `just effects` proves it.
- The queue lives in one process's memory; a restart empties it. Persistence is outside
  the module's scope.

## What would reopen this

A specification that needs a surface plain HTTP/1.1 cannot carry well (a streaming
consumer, HTTP/2, a binary protocol), a requirement for TLS in the service itself, an
advisory or licence change that `just deny` cannot accept, or the tree's cost outweighing
what axum saves.

## Related pages

- [HTTP API](../reference/http-api.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
- [Decision 0007: Dependency policy: caret ranges and `Cargo.lock`](0007-dependency-policy.md)
