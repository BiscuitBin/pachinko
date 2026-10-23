---
title: "Decision 0003: Effects behind traits"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_ports_and_fakes]
requires: []
---

# Decision 0003: Effects behind traits

*Carried from libpawdoku's decision 0003 at `ccdd001`, itself carried from Pawdoku's decision 0002 at `78d03cdf`.*

## Context

The records this one is carried from isolate side effects behind ports in the owner's own
vocabulary, each with a real adapter and an in-memory fake, so that time and randomness
become ordinary values a test controls. The source record held to one effect,
randomness; this one names six: the clock, randomness, the filesystem, the network, the
environment and threads.

A computation that reaches for one of them directly can be tested only against whatever
that effect returns on the day, so its answer depends on the machine, the moment and the
run. The workspace also has a fixed direction: `pachinko` depends on `libpachinko`, never
the reverse (`AGENTS.md` invariant 5,
[Layering and dependency direction](../explanation/layering.md)), so whatever the
library needs from the outside world it must describe itself, in terms the service can
satisfy without the library naming the service.

The domain is not defined yet. No specification names an effect, so no trait exists, and
none can be designed ahead of the clause that needs it
([decision 0002](0002-specs-are-the-source-of-truth.md)).

## Decision

Every effect the library has sits behind a trait the library defines, and the library
performs none of them itself. The pattern has three parts, and they arrive together:

- **The trait,** in `libpachinko`, in the library's own terms: it says what the domain
  needs, as the specification words it, not how a particular clock, generator or store
  provides it.
- **The fake,** beside the trait in `libpachinko`, through which a test supplies exactly
  the times, draws or contents it wants and gets the same answer on every machine and on
  every run.
- **The real implementation,** in the `pachinko` service, the one crate allowed to
  perform the effect. The service builds it and hands it to the library through the
  library's API; the library never learns which implementation it was given.

A trait arrives with the specification change that names its effect: that change brings
the trait, its fake and the service's implementation in the same commit. A trait for one
of the six named effects is this decision applied, not a reopening of it. An effect of a
kind not named here needs a decision record before any code (`AGENTS.md`).

How the ban on performing an effect directly is held in a std library, and what gate 8
checks, is [decision 0008](0008-std-library.md)'s.

## Consequences

A test of any part of the library supplies the fakes and the inputs it chose, and the real
code path still runs. A result that depends on the clock, a draw or a stored value is
reproducible from the values the test handed in, because the library has nowhere else to
get them from.

The costs are two. Every caller threads an implementation through the API, including the
ones that would rather not think about the effect; today that caller is the service, and
it is the only place the real implementations are built. And the fake is part of the
public test story: an integration test under `crates/libpachinko/tests/` will exercise
each trait and its fake from outside the crate, so the fake's behaviour is a contract, not
a convenience, and a change to it is reviewed like any other public change.

## What would reopen this

An effect that cannot be passed as a trait argument: one the library would have to
perform itself, or one whose real implementation the service could not build and hand in
through the library's API. Cancellation from outside a long computation is not such a
case, because it is a trait the computation polls, which the service implements with a flag
shared across threads and which a test drives through a scripted fake. A second effect that
can be passed is a second trait beside the first, not a reopening of this decision.

## Related pages

- [Architecture](../explanation/architecture.md)
- [Layering and dependency direction](../explanation/layering.md)
- [Testing](../reference/testing.md)
- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
