---
title: "Decision 0019: The service runs on tokio and speaks Arrow Flight"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_service_runtime_tokio_flight]
requires: []
---

# Decision 0019: The service runs on tokio and speaks Arrow Flight

*Supersedes [decision 0014](0014-service-runtime-deferred.md).*

## Context

[Decision 0014](0014-service-runtime-deferred.md) chose no runtime, transport or HTTP
stack, and named its trigger: the first specification that names a surface the service
exposes. `docs/specs/pachinko.allium` now names two, `Publishing` and `Consuming`, so the
spike that record asked for has run. Its evidence is the T20 prototype: the queue in
`libpachinko`, and in `pachinko` an owner task, a ticker and an Arrow Flight service in
front of it.

The maintainer's brief for the prototype, on 2026-10-31, asked for tokio, and for Apache
Arrow both on the wire and inside the pockets, because the prototype exists to measure
the queue under load and compare it later. Two sources shaped the design. Martin
Kleppmann's "How to do distributed locking" (2016) argues that a lease without a fencing
token is unsafe: a paused or delayed client cannot know its lease has ended, so every
action must carry a monotonic token the lock service issued once, and the service must
reject a stale one. Nathaniel J. Smith's "Notes on structured concurrency, or: Go
statement considered harmful" (2018) argues that a task must not outlive the scope that
spawned it, so that errors reach a parent and a block's exit means its work is done.

## Decision

**tokio is the runtime.** The service builds one multi-thread tokio runtime in
`pachinko::run`, for the `serve` subcommand only; the binary with no arguments still
prints `pachinko <version>` and builds no runtime. The features are `rt-multi-thread`,
`macros`, `net`, `sync`, `time` and `signal`.

**Arrow Flight over gRPC is the transport.** `arrow-flight` implements the service with
`tonic` underneath, and its protobuf code ships pre-generated, so no `protoc` is needed.
`do_put` is a publish, `do_get` a pull, and `do_action` carries the joins, the consumer's
four actions and a `status` read of the `Consuming` surface's `exposes`; every body and
reply is an Arrow IPC stream. A rejected action answers `FAILED_PRECONDITION`, an unknown
party `NOT_FOUND` and a malformed body `INVALID_ARGUMENT`; a pull that finds no pocket is
an empty stream with `outcome` = `no_pocket` in its schema metadata, never an error. No
TLS feature of `tonic` or `arrow-flight` is enabled, so `deny.toml`'s `openssl-sys` ban
holds unchanged, and the service listens on loopback unless `--listen` says otherwise.
[API reference](../reference/api.md) lists the mapping.

**One owner task holds the queue.** `libpachinko::Queue` is plain data with `&mut self`
methods. One tokio task owns the service's `Queue`, and every command reaches it through
one bounded `mpsc` channel with a `oneshot` reply: the Flight handlers send the parties'
commands, and a ticker task on a fixed interval sends `Tick`, which processes lapses and
decay. A lease's lapse and a consumer's action on that lease are therefore processed in
one stream, in arrival order, which is how the module's expiry comment says the two meet.

**Every task lives in one `JoinSet`.** `serve` spawns the owner task and the ticker into
a `JoinSet` it owns, and joins the set before it returns. On shutdown the ticker stops,
the last sender is dropped, the owner task drains the commands already sent and ends, and
`serve` joins both. tonic's per-connection tasks are tonic's own, outside the set, and
end with its graceful shutdown, which `serve` awaits before it closes the channel.
That shutdown waits for every open request, and tonic reads a unary request's message
before it calls the handler, so when it starts every request is told: a request whose
handler has not begun, or an upload still open, is answered `UNAVAILABLE` and sends the
owner nothing rather than waiting for a client that may never finish it, and a handler
that has begun runs to its end.

**The lease id is a fencing token.** `libpachinko::queue::LeaseId` is a `u64` taken from the
queue's counter when a lease is granted, monotonic and never reused. Every ack, nack,
renew and release carries it, and the queue rejects one that names a lease no longer
active, even when the same consumer holds the same pocket again under a newer id. This is
the wire form of the module's `ActionsNameTheirLease` guarantee.

## Consequences

The service's dependency tree grows from one crate to a few hundred: tokio, tonic, hyper,
h2, prost and the Arrow crates, each under [decision 0007](0007-dependency-policy.md)'s
review, each licence on `deny.toml`'s list. The licence check, the bans and the duplicate
warnings now have real input, and Renovate proposes far more updates than before.

The owner task serialises the queue. Throughput is bounded by one core's speed at the
queue's operations plus the channel's hop, which is the point of the prototype's
measurement and a limit any later design must beat with evidence.

The service is unauthenticated: anyone who reaches the port can publish, pull, ack or
nack as any party. Loopback is the default for that reason, and
[Security model](../explanation/security-model.md) says so.

A fencing token protects the queue only from actions that reach it. A consumer that does
work outside the queue, such as writing to another store, needs that store to check the
token too, which nothing here provides.

The ticker reads the wall clock through `SystemClock`, and a lapse is processed up to one
tick interval after `expires_at` passes. A clock that jumps forward ends leases early; one
that jumps back extends them. The module's eighth open question decides whether an action
that arrives after `expires_at` but before the lapse is processed should succeed.

## What would reopen this

A specification that needs more than one queue per process, or a queue that outgrows one
core; a client that cannot speak Flight; or a deployment beyond one host, which also
needs authentication and TLS and so a decision on `deny.toml`'s TLS ban.

## Related pages

- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
- [Decision 0020: Records as Arrow batches in the service](0020-records-as-arrow-batches.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
- [API reference](../reference/api.md)
