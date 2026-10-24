---
name: rust-change
description: Implement or review a change to either crate with the effects boundary, tests and documentation evidence the invariants require.
---

# Change a crate

1. Read `AGENTS.md` and `docs/explanation/architecture.md`. Identify which crate the change lands in: `crates/libpachinko` holds the domain and performs no effect; `crates/pachinko` is the service that depends on it and supplies the real implementations. A dependency never points from libpachinko to pachinko. Note whether the change touches libpachinko's public API, because a public item is a contract the service builds on and its shape is part of the change.
2. Find the clause. Every behaviour has an owning module under `docs/specs/`; read the rule, its guards and its outcomes before writing code. If no clause says what you need, this is a `spec-change` first, not a judgement call in code. The one exception is scaffolding with no domain behaviour that a ticket scopes, such as the service's wiring or a gate's harness: cite the ticket in place of the clause.
3. Put any effect behind a trait. The clock, randomness, the filesystem, the network, the environment and threads are each reached through a trait libpachinko defines, with a fake beside it that tests supply values through; the real implementation lives in pachinko. A new kind of effect needs a decision record before code (decisions 0003 and 0008). `just effects` checks the boundary, but read the diff for a direct `std` call too: the std-API bans list items, so a std item newer than the list, or one behind another target's `cfg`, a mix of features, `cfg(not(clippy))` or a custom `cfg`, passes the gate.
4. Derive the test from the clause before implementing, and confirm it fails first. A unit test lives in the module beside the code it tests, under `#[cfg(test)]`; an integration test under `crates/libpachinko/tests/` drives the public API through the fakes, and one under `crates/pachinko/tests/` drives the service; a public item carries a doctest that asserts, because doc examples are tests. A test already green proves nothing about the new behaviour.
5. Every public type is `Send + Sync + 'static`, `Clone` and `Debug`; public enums and structs are `#[non_exhaustive]`.
6. Silence a lint only with `#[expect(lint, reason = "...")]` on the item it names, never a blanket `allow`, and never write `unsafe`; the workspace forbids it.
7. Run `just fmt-check`, `just clippy`, `just test`, `just effects` and `just coverage`, then `just check` before handoff.
8. Report by `file:line`: what changed, which clause it implements, and which test proves it.
