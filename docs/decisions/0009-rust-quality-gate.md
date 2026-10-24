---
title: "Decision 0009: The Rust quality gate"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_rust_gate]
requires: []
---

# Decision 0009: The Rust quality gate

*Carried from libpawdoku's decision 0009 at `ccdd001`.*

## Context

`just check` runs nineteen gate recipes in order with the worktree snapshotted between
them, and then `check-clean`, the twentieth gate, proves the worktree unchanged;
[Quality gates](../reference/quality-gates.md) lists them. Gates 4 to 14 are Rust's:
`fmt-check`, `toml-check`, `clippy`, `metrics`, `effects`, `features`, `test-doc`,
`coverage`, `doc`, `deny` and `deps-unused`. The rest are the house's: the toolchain, the
lockfiles, the hooks, the documentation and agent contracts, the specifications and the
snapshots. This record says how nine of the eleven were chosen and where they depart from
the games' rules. The other two each came with a record of their own:
`metrics` at gate 7 with [decision 0012](0012-metrics-gate.md), and `effects` at gate 8
with [decision 0008](0008-std-library.md). `snapshots-check` at gate 19 is
[Testing](../reference/testing.md#snapshots)'s.

Two facts shape them. The toolchain is pinned to an exact stable release in
`rust-toolchain.toml`, so the clippy lint set is stable between deliberate bumps and
`-D warnings` cannot break with no diff. And nextest, which runs the tests, cannot run
doctests, while llvm-cov cannot measure them on stable, so doctests are a gate of their
own and sit outside the coverage figure.

## Decision

Clippy's `pedantic` and `cargo` groups, and a named set beyond them, at `warn` in the
`[workspace.lints]` table of the workspace `Cargo.toml`, and `-D warnings` on the `clippy`
recipe alone: a local build never breaks mid-edit, and the gate still fails. A lint is
silenced with `#[expect(lint, reason = "...")]` on the item it names, never a blanket
`allow`, so a suppression that stops being needed reports itself; `allow_attributes` and
`allow_attributes_without_reason` enforce that for an outer `#[allow]` and for any `allow`
without a reason, and review holds an inner `#![allow(..., reason = "...")]`, which passes
both. A line-coverage floor of 90 over
`crates/*/src/**`, measured by `cargo llvm-cov nextest` and enforced by
`--fail-under-lines`, lowered by lowering complexity and never by lowering the number.
`cargo hack --feature-powerset` on the host. rustdoc with `-D warnings --cfg docsrs`, so
a broken intra-doc link fails the gate. taplo for every TOML file, formatting and lint.
`cargo-shear` for a dependency nothing uses. nextest for unit and integration tests and
`cargo test --doc` for doctests, which run uncovered. Every tool pinned in
`pyproject.toml` and installed by pixi into the environment, cargo-hack through
`tools.txt`, one owner per pin ([decision 0011](0011-tool-manager.md)). Clippy runs at
gate 6 and in CI but never as a commit hook, because a cold run blocks every commit for
tens of seconds to minutes; the games make the same call by keeping `svelte-check` out of
their hook.

Tests are placed differently from the games, and this is a stated deviation. The games'
`AGENTS.md` says tests live in `tests/`, never colocated with `src/`. Here unit tests live
in `#[cfg(test)] mod tests` inside the module they test, integration tests in
`crates/libpachinko/tests/` and `crates/pachinko/tests/`, and doc examples are tests.
Rust's privacy model is the reason: a private item can only be tested from its own
module, and `tests/` sees only the public API, so a rule that kept every test out of
`src/` would leave every private function tested only through whatever public path happens
to reach it. Clippy's `tests_outside_test_module` lint holds the module shape, so a test
function outside a `mod tests` is a gate failure.

## Consequences

The ones that hurt. Pedantic lints produce false positives, and each needs an `expect`
with a reason rather than a shrug. The lint set moves with the toolchain pin, so every
bump is a clippy pass before anything else. The feature powerset doubles with each
feature added. The floor is a statement about unit and integration tests only; a crate
whose behaviour is best shown in doc examples earns no coverage for them. And a cold
`just check` is minutes, because clippy, the powerset, coverage and rustdoc each build.

## What would reopen this

llvm-cov measuring doctests on stable, which would fold them into the floor. Clippy's cold
time falling enough to run as a hook. Mutation testing or fuzzing as a gate: T15 and T16
add them as recipes outside `just check`, and if either joined it, it would change what
"tested" means, so the floor would be restated rather than raised.

## Related pages

- [Quality gates](../reference/quality-gates.md)
- [Testing](../reference/testing.md)
- [Quality philosophy](../explanation/quality-philosophy.md)
