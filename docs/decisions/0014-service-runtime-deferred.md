---
title: "Decision 0014: The service runtime waits for the specifications"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_service_runtime_deferred]
requires: []
---

# Decision 0014: The service runtime waits for the specifications

## Context

The workspace holds a library and a service
([decision 0001](0001-library-and-service.md)). The service is the binary that supplies
the real implementations of the library's effect traits
([decision 0003](0003-effects-behind-traits.md)), and a service usually exposes something:
a port, a socket, a queue, a command line. What `pachinko` exposes, to whom and under
what load is behaviour, and behaviour is the specifications' to decide
([decision 0002](0002-specs-are-the-source-of-truth.md)). Today the specifications define
no domain: `docs/specs/` holds one placeholder module that declares no behaviour.

An async runtime, a transport and an HTTP stack are each a large choice that is costly to
reverse. Each brings a tree of dependencies, a licence and advisory surface for
cargo-deny, an execution model the service's tests must live inside, and a shape that
every effect implementation in the service would take on. Choosing one now would be a
guess about a surface no specification has named, and the guess would then argue for
itself when the specifications arrive.

## Decision

No async runtime, transport or HTTP stack is chosen. The service is a minimal std binary
that depends on `libpachinko` and nothing else. `pachinko::run` takes any writer and
writes one line, `pachinko <version>`, with the library's version, and `main` is one line
that calls it on locked standard output and turns the result into the process's exit
status. A unit test checks `run` against a buffer and an integration test runs the built
binary; [Architecture](../explanation/architecture.md) describes both.

The open question is which runtime, transport and HTTP stack, if any, the service needs.
This record does not answer it and states no preference. The trigger is the first
specification that names a surface the service exposes. When that specification lands, a
spike compares the options against it, for example a runtime such as `tokio`, `async-std`
or `smol` against plain threads and blocking I/O, and a new decision record supersedes
this one with the choice and the evidence.

## Consequences

There is nothing to audit for a network surface yet. The service listens on nothing, and
[Security model](../explanation/security-model.md) describes that surface only once the
superseding record names one. The service's only dependency is `libpachinko`, so its
dependency tree adds nothing for cargo-deny to judge.

When the service gains a runtime, the effects boundary keeps it out of the library.
`tokio`, `async-std`, `smol`, `mio` and `socket2` are among the crates `just effects`
bans from libpachinko's normal dependency tree, and the mechanism T10 chooses will ban
`std::net`, `std::thread::spawn` and the rest of the std APIs that reach an effect, so
the runtime can live only in `pachinko` ([decision 0008](0008-std-library.md)).
cargo-deny cannot draw that line: its `wrappers` name direct parents only, so `deny.toml`
bans no effect crate ([Configuration](../reference/configuration.md)). One cargo-deny ban
does reach the service: `openssl-sys` is denied across the workspace, so whatever stack
the spike picks carries no native TLS unless a record changes that ban. Each crate the
choice adds follows [decision 0007](0007-dependency-policy.md).

The wave-2 extras, such as T13's container image of this binary, are planned foundation
work, not decisions deferred to the specifications.

## What would reopen this

The first specification that names a surface the service exposes: a protocol, an
endpoint, a listener, or a client the service must call. That opens the spike, and the
spike's record supersedes this one. Whether a specification that needs concurrency inside
the service without naming a surface is also a trigger is the maintainer's question; this
record names only the surface.

## Related pages

- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Configuration](../reference/configuration.md)
- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
