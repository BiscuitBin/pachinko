---
title: "Decision 0017: Mutation testing, benchmarks and fuzzing"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_mutation_bench_fuzz]
requires: []
---

# Decision 0017: Mutation testing, benchmarks and fuzzing

## Context

The foundation needs runnable measurement and fuzzing harnesses before a solver or
parser exists. T16 adapts libpawdoku’s S03 spike at `ccdd001` to the foundation’s
version function. T15 owns the mutation-testing section when that lane lands.

## Decision

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

Clippy now builds libFuzzer’s C++ code and requires a C++ compiler. Apple silicon needs
Rosetta 2 for the current cargo-fuzz binary. Stable without sanitizers gives coverage
guidance and panic detection, without sanitizer checks. A clean smoke run validates
wiring, not domain correctness. Real targets and deterministic crash regression tests
arrive with the specifications that define real behaviour. Fuzz output stays ignored.

## What would reopen this

Revisit divan if maintenance prevents a compiler upgrade; criterion with default
features disabled is the spike’s fallback. Recheck stable support when cargo-fuzz moves.
An arm64 macOS binary would remove the Rosetta prerequisite. Real workloads may justify
corpus caching and additional targets, but do not put benchmark timings in CI.

## Related pages

- [Testing](../reference/testing.md)
- [Commands](../reference/commands.md)
- [Dependency policy](0007-dependency-policy.md)
