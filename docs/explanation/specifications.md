---
title: "Specifications"
kind: "explanation"
audience: [contributor, maintainer, agent]
canonical_for: [specification_model]
requires: []
---

# Specifications

The specifications under `docs/specs/` decide what libpachinko and pachinko do.
Behaviour is written down in Allium before it is built. The handbook, code and tests
answer to those specifications; when they disagree, the specification wins.
[Decision 0002](../decisions/0002-specs-are-the-source-of-truth.md) records that choice.

The procedure is in [Work with the specifications](../how-to/work-with-the-specs.md).
This page explains why.

## What a specification is for

A rule left to prose or implicit in code can drift as implementations change.
Stating its trigger, guards and outcomes makes the required behaviour explicit,
and gives tests something independent of the implementation to assert. Named
invariants and surface guarantees likewise describe obligations an implementation
must satisfy.

Passing a syntax check alone does not prove that the code fulfils those obligations.
Tests derived from the clauses and reviews of the implementation establish that
connection. A change in intended behaviour starts with the specification and carries
its tests and implementation with it.

## What exists today

There is one module, `pachinko.allium`, the root. It specifies the queue domain
elicited from the maintainer on 2026-10-30: publishers publish records into pockets,
one record per key, and a higher rank supersedes in place; a consumer leases one
pocket at a time, pulls its oldest records in bounded batches, and acks, nacks,
renews or releases; a lease that lapses frees the pocket's records, and consecutive
lapses park the pocket until a publish revives it; superseded records and dead
letters stay visible until they decay. It imports nothing, and
[Terminology](../project/terminology.md) holds its words.

None of it is implemented. Empty diagnostics and findings say the module is valid,
not that the library does any of it. Its test plan carries obligations no test yet
covers; the first implementation change derives those tests, confirms they fail,
and builds until they pass. Behaviour that outgrows the root's Scope goes in a
module it imports, with explicit boundaries so that each behaviour has an owner.

## Open questions are a feature

An `open question` records a product decision nobody has made yet. An unwritten
gap is easily filled by whoever writes the code first; a written one stays visible
until someone entitled to decide answers it.

The root carries nine: equal-rank publishes, who reads the superseded and
dead-letter queues, capacity beyond shedding the decaying queues, routing by pool
or priority, an adaptive lease TTL, a pocket jumping the queue, reviving a parked
pocket other than by a publish, a consumer's action that arrives after its lease's
time has passed but before the lapse is processed, and a key published again under
the lease that removed its record. Seven mark decisions the maintainer declined to
make in the elicitation and the last two gaps that writing and reviewing the rules
exposed; where the rules' reading matters meanwhile, the question says what it is,
and no code answers one. A change that reaches an unanswered decision adds a
question rather than guessing. A low count is normal for a settled module, but it
is not a reason to hide uncertainty.

## What a specification is not

A specification does not choose a programming language, framework, storage
mechanism or layout. It states what any implementation must satisfy.
`AGENTS.md` governs how this repository builds that implementation, including the
library's effects boundary and the quality gate.

## Related pages

- [Work with the specifications](../how-to/work-with-the-specs.md)
- [Decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)
