---
title: "Layering and dependency direction"
kind: "explanation"
audience: [contributor, maintainer, agent]
canonical_for: [dependency_boundaries]
requires: []
---

# Layering and dependency direction

The crate graph runs one way. `pachinko`, the service, depends on `libpachinko`, the
library; nothing in the workspace depends on `pachinko`, and `libpachinko` depends on
nothing in the workspace ([decision 0001](../decisions/0001-library-and-service.md)).

| Crate | Depends on | Depended on by |
| --- | --- | --- |
| `libpachinko` | nothing in the workspace | `pachinko` |
| `pachinko` | `libpachinko` | nothing |

`libpachinko` holds the domain once the specifications define one, and performs no
effect itself. `pachinko` is a minimal binary that supplies the real implementations of
the library's effect traits, and nothing imports it: it is the leaf where the library
meets the outside.

Inside `libpachinko` the module graph will be the specification import graph. Each
specification module under `docs/specs/` names the modules it imports, and the Rust
module that implements it may use what its specification imports and nothing above it.
Today there is one specification module, `pachinko.allium`, which specifies the domain
and imports nothing, and one Rust module, `crates/libpachinko/src/lib.rs`, so there is
nothing to order. When a specification change splits the domain into modules, the
code's modules follow it; [Work with the specifications](../how-to/work-with-the-specs.md#add-a-module)
shows how a module states its imports.

## Why the direction matters

The rule is not tidiness. It is what makes the claim below true, and that claim is
load-bearing:

**The library is testable without the service.** A library that imported the service
could only be tested with the service's real implementations in the way, and a module
that reached for an effect of its own could only be tested against whatever that effect
produced. Because nothing in `libpachinko` names `pachinko`, and because the only thing
that reaches outside the library is the effects boundary, a test of any part of it
supplies the fakes and the inputs it chose, and the real code path still runs. The same
argument will hold between the library's modules: a module that imports only what its
specification imports can be tested with only those in the way.

## Where a side effect goes

An effect is a trait `libpachinko` defines, a fake beside it, and the real
implementation in `pachinko`; [Architecture](architecture.md) explains the boundary,
decisions [0003](../decisions/0003-effects-behind-traits.md) and
[0008](../decisions/0008-std-library.md) record it, and gate 8, `just effects`, holds it
with the bans [Quality gates](../reference/quality-gates.md#gate-8-the-effects-bans)
lists.

The rule that follows: **tests inject fakes through the boundary, they never stub
globals.** A stubbed global leaks between tests and hides the fact that the code reached
outside its layer.

## Enforcement

Cargo enforces the crate direction. `crates/pachinko/Cargo.toml` lists `libpachinko` as a
dependency, so a normal dependency from `libpachinko` back on `pachinko` would close a
cycle, and Cargo refuses a cyclic package graph. Cargo does allow a development
dependency to close a cycle; invariant 5 in `AGENTS.md`, that `pachinko` depends on
`libpachinko` and never the reverse, and review hold that edge.

rustc will not enforce the module direction: it allows a cycle between modules of the
same crate. Gate 7, `just metrics`, will. It runs rustqual over each crate against
`rustqual.toml`, whose architecture section is on, and once there are modules that file
gains one `[[architecture.pattern]]` rule per module, each listing every `crate::` path
its module may not name, with the specification's imports as its reason
([decision 0012](../decisions/0012-metrics-gate.md)). Today it holds no pattern rule,
because there is no module to bound. A rule will cover the file named after its module
and everything under the directory of the same name, so a child is held to its parent's
row, and it will catch both a `use crate::…` line and an inline `crate::…` path in a
function body.

Three paths will be out of the rules' sight. A path written relative to the module, such
as `super::super::…`, names no `crate::` prefix. An alias of the crate, `use crate as
root;` and then `root::…`, replaces the prefix the rules match. And a re-export in
`lib.rs` lets any module name an item without its module's path. The third is guarded
from the side: `unreachable_pub` and `unnameable_types` in the workspace lint table keep
what is `pub` deliberate, so a module's surface is the one it meant to export, and
`lib.rs` is the one file to read for re-exports in review. `mod_module_files` bans
`mod.rs`, so every module is the file named after it and the graph can be read from the
file tree. What the rules cannot see is left to review, to the `rust-change` and
`review-change` skills, and to the shape of the tests: code in the wrong layer is usually
code that is hard to test.

## Related pages

- [Architecture](architecture.md)
- [Specifications](specifications.md)
- [Testing](../reference/testing.md)
- [Quality gates](../reference/quality-gates.md)
- [Decision 0001](../decisions/0001-library-and-service.md)
- [Decision 0003](../decisions/0003-effects-behind-traits.md)
- [Decision 0012](../decisions/0012-metrics-gate.md)
