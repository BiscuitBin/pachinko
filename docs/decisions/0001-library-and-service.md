---
title: "Decision 0001: A library and a service in one workspace"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_library_and_service]
requires: []
---

# Decision 0001: A library and a service in one workspace

## Context

The repository was founded before its domain was defined. The Allium specifications under
`docs/specs/` will define it, and until they do the code holds no behaviour of its own.
What could be decided up front was where that behaviour will live and what will run it.

Two needs pull in different directions. The domain must be testable as plain computation:
a test supplies its inputs, reads its outputs and repeats the same run as often as it
likes, which it can do only where nothing reaches outside the computation. Something must
also run that computation as a process, with a real clock, real storage and real
connections once the specifications ask for them. Code that does both in one place tests
badly, and a seam drawn by convention alone erodes one convenient call at a time.

The service is today the only consumer of the domain, and the library and the service
are written and reviewed by the same people, with most changes crossing both.

## Decision

The repository is one Cargo workspace of two crates. `crates/libpachinko` is a library
that holds the domain once the specifications define one and performs no effect itself.
`crates/pachinko` is a minimal service binary that depends on `libpachinko`, never the
reverse, and supplies the real implementations of the library's effect traits. How those
traits work is [decision 0003](0003-effects-behind-traits.md); why the library is a std
crate, and what keeps its effects out, is [decision 0008](0008-std-library.md).

The service stays thin. Its binary's `main` is one line that calls `pachinko::run` on
standard output and turns the result into the exit status; `run` takes any writer, so a
unit test exercises it against a buffer, and `tests/binary.rs` runs the built binary as a
user would. What the service will grow into, a runtime and a transport, waits for the
specifications ([decision 0014](0014-service-runtime-deferred.md)).

Both crates share one `Cargo.lock` at the root, one lint table in the root `Cargo.toml`,
and one gate: the `Justfile`'s build, test, lint, coverage and documentation recipes run
with `--workspace`, and `just check` covers both crates in one run.

Two alternatives were considered. One crate, a library with a binary target beside it,
was the smallest shape. It lost because the library and the binary would share one
`[dependencies]` table: whatever the service needs to perform its effects would become
the library's dependency too, and the check that the library's dependency tree names no
effect crate would have nothing to separate. Two repositories, one per crate, would have
given each its own release. They lost because every change that crosses the seam, which
is most changes while the specifications are young, would become two pull requests and a
version bump, with two lockfiles, two gates and two copies of the handbook to keep in
step, and the specifications could sit beside only one of the two.

## Consequences

The seam between the library and the service is a crate boundary, which the compiler
enforces: the library cannot call into the service, and the service reaches the library
only through its public API. Invariant 5 in `AGENTS.md` states the direction, and
[Layering and dependency direction](../explanation/layering.md) explains it.

The crates move together. Both are version `0.1.0` and `publish = false`, and a change to
the library reaches the service in the same commit, under the same gate. Nothing outside
this workspace can depend on the library through a registry today.

The service's dependencies share the library's lockfile. When the service gains a runtime,
its transitive crates enter the same `Cargo.lock` and the same cargo-deny policy, and the
gate that keeps effects out of the library has to look at the library's own dependency
tree rather than the workspace's ([decision 0008](0008-std-library.md)).

One gate means one cost. A slow test in either crate slows every run of `just check`, and
the 90% line-coverage floor is measured over `crates/*/src/**` for both crates together.

## What would reopen this

A second consumer of the library that needs its own release cadence: a crate outside
this workspace that wants to depend on a published `libpachinko` and upgrade it on its
own schedule, which this workspace cannot serve while the library is `publish = false` and
every change to it is shaped by the one service it ships beside. The
question then is whether the library moves to a repository of its own or is published
from this one; the answer is a new record, not this one amended.

## Related pages

- [Architecture](../explanation/architecture.md)
- [Layering and dependency direction](../explanation/layering.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Repository map](../project/repository-map.md)
- [Decision 0003: Effects behind traits](0003-effects-behind-traits.md)
- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
