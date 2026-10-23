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

There is one module, `pachinko.allium`, the root of a domain that has not yet been
defined. It imports nothing and declares no entities, rules, surfaces, invariants
or config. Its header makes that absence explicit.

The root exists so that the specification gate runs from the foundation onward.
Empty diagnostics and findings say the placeholder is valid, not that any domain
behaviour has been specified or implemented. Its empty test plan carries no
behavioural obligations.

The first specification change will define behaviour in the root or in a module
it imports. That change will establish scope and vocabulary, derive tests from
the new clauses and implement until those tests pass. New modules receive explicit
boundaries so that each behaviour has an owner.

## Open questions are a feature

An `open question` records a product decision nobody has made yet. An unwritten
gap is easily filled by whoever writes the code first; a written one stays visible
until someone entitled to decide answers it.

The placeholder has none today. As the domain is defined, a change that reaches
an unanswered decision should add a question rather than guess. A low count is
normal for a settled module, but it is not a reason to hide uncertainty.

## What a specification is not

A specification does not choose a programming language, framework, storage
mechanism or layout. It states what any implementation must satisfy.
`AGENTS.md` governs how this repository builds that implementation, including the
library's effects boundary and the quality gate.

## Related pages

- [Work with the specifications](../how-to/work-with-the-specs.md)
- [Decision 0002](../decisions/0002-specs-are-the-source-of-truth.md)
