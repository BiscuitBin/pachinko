---
title: "Decision 0017: Mutation testing, benchmarks and fuzzing"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_mutation_bench_fuzz]
requires: []
---

# Decision 0017: Mutation testing, benchmarks and fuzzing

## Context

Coverage records executed lines but cannot show whether assertions would notice a
behaviour change. Libpawdoku's S03 spike at `ccdd001` measured cargo-mutants 27.1.0
and drafted its adoption as a weekly report. This workspace has two crates and only
a handful of items today, but the report should grow with the specifications.
This record is shared with T16, which owns the benchmark and fuzzing decisions.

## Decision

### Mutation testing

Pin cargo-mutants 27.1.0 in the pixi manifest and commit its generated lockfile.
On 2026-10-23 conda-forge provided builds for linux-64 and osx-arm64 with no Rust
dependency, preserving rustup's compiler ownership (decision 0011). This is a tool,
not a crate dependency; neither crate's dependency graph changes (decision 0007).

`just mutants` runs the whole workspace with every feature on a source copy,
excluding Gitignored files and passing `--locked` to builds and tests. Use `cargo test`
so doctests participate. The independent `mutants.yml` workflow runs weekly and by hand,
keeping the report for 28 days even when mutants survive. It is never a required check,
never among `check`'s dependencies, and never a pull-request workflow. There is no score
threshold and no diff-only mutation run.

A missed mutant requires an assertion or an explanation of observable equivalence.
Only an equivalent mutant is excluded, with a comment in `.cargo/mutants.toml`, created
when needed. Do not add the `mutants` crate for skip attributes.

## Consequences

The coverage floor stays at 90 per cent. Mutation reports add evidence about assertions,
but require a person to read and act on them. Runtime grows with the workspace and may
reach hours, so it cannot determine whether a change may merge. Constructors named
`new` are not mutated by this tool and still need deliberately written tests.
The workflow starts with a thirty-minute timeout; maintenance raises it when the
mutation step takes more than fifteen minutes.

## What would reopen this

Revisit the tool or schedule if the report becomes too slow to read regularly, tool
behaviour changes, or conda-forge packaging starts bringing its own Rust compiler.
A proposal to require a mutation score needs evidence beyond line coverage and would
reopen this decision explicitly.

## Related pages

- [Testing](../reference/testing.md#mutation-testing)
- [Quality philosophy](../explanation/quality-philosophy.md)
- [Maintenance](../operations/maintenance.md)
- [Decision 0007: Dependency policy](0007-dependency-policy.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
