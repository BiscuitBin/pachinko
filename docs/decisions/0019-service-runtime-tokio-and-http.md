---
title: "Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_service_runtime]
requires: []
---

# Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON

This record supersedes [decision 0014](0014-service-runtime-deferred.md), which deferred
the runtime until a specification named a surface the service exposes.

## Context

Decision 0014 chose no async runtime, transport or HTTP stack and named its trigger: the
first specification that names a surface the service exposes. T19 wrote the domain as
`docs/specs/pachinko.allium`, and its two surfaces, `Publishing` and `Consuming`, are
that trigger. The spike 0014 called for then ran as three prototypes, each building the
domain a different way on an unmerged branch with its own ticket, and a fourth branch
stacked on the first:

| Source | Branch | Head |
| --- | --- | --- |
| PR 38: an Arrow Flight service over gRPC with a library-owned store trait, an owner task and batch publish | `tickets/t20-pachinko-prototype-complete` | `cb77f07` |
| PR 39: an HTTP/JSON service behind a global mutex, with an exhaustive error map, signal handling and a guard-predicting proptest walk | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` |
| PR 36: a worker-pool spike partitioned by pocket key | `tickets/t20-worker-pool-spike-complete` | `0fa614e` |
| T21: a log-backed store stacked on PR 38 | `ticket/t21-pocket-log-records` (local to the maintainer's checkout, not pushed) | `94fc9b8` |

A comparison of the three concluded: merge neither PR 38 nor PR 39, and build a new
service that grafts PR 38's seams (the `PocketRecords` trait and its `check_contract`,
the expiry-indexed tick, the owner task and the batch semantics) onto PR 39's HTTP/JSON
wire, error map, signal handling and guard-predicting walk. This record writes that
conclusion down, row by row.

## Decision

**Wire.** The service speaks HTTP/1.1 with JSON bodies through axum, on PR 39's routes,
plus a batch publish route that answers with an outcome per row.

**Executor.** One tokio task owns the queue. Every handler sends a command on a bounded
mpsc channel and awaits the reply on a oneshot; there is no `Mutex`. The sweep's `Tick`
is a command the same owner applies, serialized with every other command, and it arrives
on a separate lane that a `tokio::select!` with `biased;` polls first. Because every
command and every tick are applied by one task, no borrow of the owner's state crosses an
`.await`, and a lapse and a consumer's action are totally ordered: whichever the owner
applies first, the rules' guards decide the second. Branch A of the specification's
eighth open question follows, below.

**Store.** The queue keeps its records in `MemoryRecords` behind the library-owned
`PocketRecords` trait, with one `check_contract` every implementation passes.
`ArrowRecords` is never used. `LogRecords` (T21) stays a reopen option.

**Open question 8, closed with branch A.** A late action, one the owner applies after
the lease's `expires_at` but before the lapse, succeeds as long as its other guards
hold, because the owner serializes the lapse and the action: a pull, ack, nack or renew
extends the lease, and a release ends it and frees its records. Arrival order does not
decide; the order in which the owner applies them does. The tick lane is polled first,
so a ready tick can apply a lapse ahead of an action queued earlier, which the owner
then applies as if it came after the lapse: an ack, nack, renew or release is rejected
because the lease it names has ended, and a pull is handled as one from a consumer with
no active lease, claiming the front queued pocket or finding none. Recovery after a dead
consumer therefore takes `lease_ttl`, plus up to one sweep interval, plus the
application of the tick in progress and of the command the owner is applying. A tick
samples `now` once, so a lease that expires during a long tick waits for the next one.
Tests pin the outcome in both orders, the lapse first and the action first.

**Open question 3 and the limits.** Open question 3, capacity, stays open. The service
adds only request-shape limits (rows, bytes, timeouts), never a capacity refusal: a
request refused for its shape says nothing about how full the deployment is, and
`NeverRefusedForCapacity` stands.

**Batch limits.** Three flags bound a request: 1,000 rows per request, 1 MiB per payload
and 16 MiB per body. For comparison: Pub/Sub, 1,000 messages and 10 MB; SQS, 10 and
1 MiB; Kafka and NATS, about 1 MiB per message; RabbitMQ 4.x, 16 MiB per message.

**Lease ids.** A `LeaseIds` trait, in PR 39's shape, supplies them, with a counting fake
beside it. The real source draws an epoch once at start from the operating system's RNG
and advances a serial with `checked_add`; the id is sent as an opaque string of 32
hexadecimal digits. The service refuses to start if the RNG fails. An ended lease leaves
the active registry, as in PR 38.

**Clock.** A `Clock` trait supplies `now`, with `FakeClock` beside it. The real clock is
`SystemTime` behind the queue's monotonic clamp, which both prototypes carry (PR 38
`crates/libpachinko/src/queue.rs` line 199): the queue's `now` is the clock's reading
held at the latest instant read so far. The trade-off is that a backward step of the
wall clock stretches every active lease by the size of the step, and nothing lapses
early. A suspend-aware clock is a listed follow-up that no ticket owns yet.

**Open questions 1 and 9; the rest.** Open questions 1 and 9 are built as labelled
placeholders, the rules as they stand, and pinned by tests so that an answer shows up as
a failing test. Open questions 2 and 4 to 7 are untouched.

**Configuration defaults.** The specification's five defaults are the MVP's, confirmed by
the maintainer: `lease_ttl` 5 minutes, `max_consecutive_expiries` 3, `max_pull_batch`
100, `superseded_ttl` 12 hours and `dead_letter_ttl` 12 hours. A deployment sets its own.

**Tickets.** The work is numbered from T22 onward, because T20 and T21 belong to the
prototype branches. A ticket stays `status: todo` until it is done, and is sized S, M or
L.

**This record.** It is one fresh record, 0019, that supersedes 0014 and cites PR 38 at
`cb77f07` and PR 39 at `cf1a6b2`.

**Tracer bullet.** It is split: a library tracer (T23), then a service tracer (T24).

**Parallel lanes.** T25, T27 and T29 run in parallel after T24 and merge one at a time:
files they share are listed `shared, own lines`, `origin/main` is merged before each
push, and the verification is rerun and requoted (`tickets/CONVENTIONS.md` §11, the
wave-2 rule).

**Evidence tickets.** T28, the walk and the scenario catalog, and T30, the benchmarks and
the load generator, stay separate tickets.

## Consequences

The service gains a dependency tree. T23 adds `thiserror` to the workspace table and the
library; T24 adds `thiserror` (a manifest line under T23's note), `tokio`, `axum`,
`serde`, `base64`, `getrandom`, `tracing` and `tracing-subscriber` to the service, and
`serde_json` and `ureq` as its dev-dependencies. Each takes its
[decision 0007](0007-dependency-policy.md) note in the workspace table; the tickets name
the versions and features.

The service now has a network surface. It listens on loopback by default, it has no
authentication and no TLS, and T27 adds the request-shape limits above.
[Security model](../explanation/security-model.md) describes the surface once T24 and
T27 have built it.

The effects boundary holds. The clock and the lease ids are traits libpachinko defines,
with fakes beside them, and `pachinko` supplies `SystemTime` and the RNG
([decision 0003](0003-effects-behind-traits.md),
[decision 0008](0008-std-library.md)). tokio stays out of the library, as `just effects`
requires.

Memory is unbounded, and that hurts. Ten seconds of batch publishing into PR 38 with
`MemoryRecords` reached 8,191 MiB of resident memory, and nothing bounds it:
`NeverRefusedForCapacity` forbids a capacity refusal, and open question 3, which would
say what gives instead, stays open.

## Rejected options

- **Arrow Flight over gRPC, PR 38's wire.** Reopened if columnar producers become a
  requirement, or if T30's JSON batch rate falls below a ratio, which the maintainer
  sets, of the 1,565,592 rows/s that Flight with `MemoryRecords` reached. This record
  names the figure and leaves the ratio to the maintainer; it is not a gate.
- **A global mutex, PR 39's executor.** Reopened only if the owner task's p50 at low
  concurrency becomes the limiting cost. Lock poisoning, and the argument that one owner
  serializes a lapse against an action, are why it lost.
- **Partitioning by pocket key, PR 36.** Reopened if one owner task's throughput is the
  bottleneck. PR 36 showed that positions and `PullClaimsFrontPocket` need a linearizable
  order first.
- **`ArrowRecords`.** Never. **`LogRecords`, T21**, is reopened if durability or an Arrow
  consumer becomes a requirement; it passes the same `check_contract`.

## Evidence

One Apple silicon Mac (macOS, Darwin 25.4.0, 18 CPUs), rustc 1.99.1, measured on
2026-10-31 and 2026-11-03; end-to-end runs over loopback with closed-loop clients;
medians of 5 runs; 64-byte payloads; P = 1,000 pockets. The in-process numbers are
single-thread over the prototype's own `Queue` with its fake clock. The heads measured
are PR 38 at `cb77f07`, PR 39 at `cf1a6b2` and T21 at `94fc9b8`.

| Measurement | Value |
| --- | --- |
| Single-record publish, C=64, PR 39 as shipped (INFO log) | 99,161 records/s |
| Single-record publish, C=64, PR 39 with the log capped at WARN | 157,715 records/s |
| Single-record publish, C=64, PR 38 as shipped (`ArrowRecords`) | 76,058 records/s |
| Single-record publish, C=64, PR 38 with `MemoryRecords` | 77,920 records/s |
| Batch publish, C=8, 256 rows per batch, PR 38 as shipped | 121,394 rows/s |
| Batch publish, C=8, 256 rows per batch, PR 38 with `MemoryRecords` | 1,565,592 rows/s |
| Batch publish, C=64, 1,024 rows per batch, PR 38 with `MemoryRecords` | 2,100,471 rows/s (139,374 as shipped) |
| In process, W1 publish at P=1,000: `ArrowRecords` vs `MemoryRecords` | 287,895 vs 4,246,025 ops/s (14.7x) |
| In process, W2 supersede | 781,874 vs 6,118,040 ops/s (7.8x) |
| In process, W3 pull 16, ack, release | 460,431 vs 2,550,028 ops/s (5.5x) |
| In process, W4 publish with every fifth op a consume step | 301,528 vs 2,831,593 ops/s (9.4x) |
| In process, W5 pull then lapse on tick | 257,835 vs 801,351 ops/s (3.1x) |
| Owner task vs mutex-with-yield vs bare mutex, W1 at C=64, in process, 18 tokio workers, PR 39's `Queue` | 1,375,780 vs 448,107 vs 818,772 ops/s; p50/p99 35.8/60.6 vs 4.96/337.7 vs 2.58/236.4 µs |
| Mutex at C=1 | 3,410,279 vs owner task 1,533,736 ops/s |
| Lapse lag p50 under batch load, PR 38 as shipped (Tick in the client FIFO) | 2,038 ms (max 3,257 ms), re-run 2,411 ms; PR 38 with `MemoryRecords` 439 ms; T21 560 ms; 315 to 689 ms in the other probe runs |
| RSS, PR 38 with `MemoryRecords`, 10 s of batch publishing | 8,191 MiB (nothing bounds it; `NeverRefusedForCapacity` forbids a capacity refusal) |
| RSS per record, consume at C=64 | about 0.93 KiB (`ArrowRecords`) vs 0.45 KiB (`MemoryRecords`) |
| T21 `LogRecords` in process | within ±2.5% of `MemoryRecords` on single-record calls; W1 at P=1,000 3,719,520 vs 4,885,113 ops/s (0.76x); p99 117 vs 44 ms at b=1,024, C=64 |

The caveats:

- One machine, and nothing measured on Linux.
- Closed-loop clients understate tail latency.
- The `MemoryRecords` and WARN-log variants are scratch variants, one change each, that
  neither prototype ships.
- HTTP batch publish was never measured, which is why T30 exists.

## What would reopen this

- Columnar producers become a requirement, or T30's JSON batch rate falls below the
  maintainer's ratio of 1,565,592 rows/s: reopen Arrow Flight.
- The owner task's p50 at low concurrency becomes the limiting cost: reopen the mutex.
- One owner task's throughput is the bottleneck: reopen partitioning by pocket key, after
  giving positions and `PullClaimsFrontPocket` a linearizable order.
- Durability or an Arrow consumer becomes a requirement: reopen `LogRecords` (T21, head
  `94fc9b8`, which a fresh clone cannot read until the branch is pushed).
- A machine that sleeps, or a wall clock that steps, makes the clamp's stretched leases a
  problem: add the suspend-aware clock.

## Related pages

- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Configuration](../reference/configuration.md)
- [Decision 0003: Effects behind traits](0003-effects-behind-traits.md)
- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
