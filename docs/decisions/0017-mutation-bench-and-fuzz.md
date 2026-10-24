---
title: "Decision 0017: Mutation testing, benchmarks and fuzzing"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_mutation_bench_fuzz]
requires: []
---

# Decision 0017: Mutation testing, benchmarks and fuzzing

## Context

Coverage records executed lines but cannot show whether assertions would notice a
behaviour change. Libpawdoku's S03 spike at `ccdd001` measured cargo-mutants 27.1.0
and drafted its adoption as a weekly report. This workspace has two crates and only
a handful of items today, but the report should grow with the specifications.
This record covers T15 mutation testing and T16 benchmarks and fuzzing.

The foundation needs runnable measurement and fuzzing harnesses before a solver or
parser exists. T16 adapts libpawdoku’s S03 spike at `ccdd001` to the foundation’s
version function.

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

### Benchmarks and fuzzing

Use divan 0.1.21 as a development dependency for one version benchmark. Its result
passes through a black box. Benchmarks are manual only: machine load makes timings
unsuitable as a CI gate. The tool was rechecked on 2026-10-23: its latest release is
still 2025-04-10, its MSRV is 1.80.0, and its unarchived repository’s latest commit is
a documentation change on 2026-07-19. Maintenance is thin; the smoke harness is small
enough to replace if that becomes a practical problem.

Use cargo-fuzz 0.13.2 and libfuzzer-sys 0.4.13, rechecked on the same day. Run with
`--sanitizer none` on the exact stable compiler in `rust-toolchain.toml`; no nightly is
needed. The host triple is explicit because the macOS cargo-fuzz binary is x86 only.
The excluded, unpublished fuzz crate has its own lockfile and repeats the workspace’s
forbid levels and Clippy policy; keep the copied lint levels aligned with the root.
Existing lock, format and clippy recipes include it explicitly. Before cargo-fuzz runs,
validate the locks and then run offline because cargo-fuzz has no locked option.

The smoke target accepts arbitrary bytes, checks a local UTF-8 round trip when valid,
and calls `libpachinko::version()`. It introduces no library API and promises no parser
coverage. The independent weekly and manual workflow runs each target for 300 seconds
and uploads crash files on failure. Matrix targets do not cancel each other.

## Consequences

The coverage floor stays at 90 per cent. Mutation reports add evidence about assertions,
but require a person to read and act on them. Runtime grows with the workspace and may
reach hours, so it cannot determine whether a change may merge. Constructors named
`new` are not mutated by this tool and still need deliberately written tests.
The workflow starts with a thirty-minute timeout; maintenance raises it when the
mutation step takes more than fifteen minutes.

Clippy now builds libFuzzer’s C++ code and requires a C++ compiler. Apple silicon needs
Rosetta 2 for the current cargo-fuzz binary. Stable without sanitizers gives coverage
guidance and panic detection, without sanitizer checks. A clean smoke run validates
wiring, not domain correctness. Real targets and deterministic crash regression tests
arrive with the specifications that define real behaviour. Fuzz output stays ignored.

## What would reopen this

Revisit the tool or schedule if the report becomes too slow to read regularly, tool
behaviour changes, or conda-forge packaging starts bringing its own Rust compiler.
A proposal to require a mutation score needs evidence beyond line coverage and would
reopen this decision explicitly.

Revisit divan if maintenance prevents a compiler upgrade; criterion with default
features disabled is the spike’s fallback. Recheck stable support when cargo-fuzz moves.
An arm64 macOS binary would remove the Rosetta prerequisite. Real workloads may justify
corpus caching and additional targets, but do not put benchmark timings in CI.

## Related pages

- [Testing](../reference/testing.md)
- [Quality philosophy](../explanation/quality-philosophy.md)
- [Maintenance](../operations/maintenance.md)
- [Decision 0007: Dependency policy](0007-dependency-policy.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
- [Commands](../reference/commands.md)
