---
title: "Terminology"
kind: "project"
audience: [contributor, maintainer, operator, agent]
canonical_for: [project_terminology]
requires: []
---

# Terminology

These words mean one thing here. Using them loosely is how a review ends up arguing
about vocabulary instead of behaviour, so each row names the word to use and the words
to avoid.

## The domain

The domain is not defined yet, and this page holds none of its words. They arrive with
the specifications: each module's Vocabulary section picks one word where several would
do, and the root module, `docs/specs/pachinko.allium`, says today that it has none. The
ticket that adds a domain word to a specification adds its row here, in a table of its
own above the repository's, naming the module it comes from. When a row and a module
disagree, the module is right and the row is corrected.

## The repository

| Term | Meaning |
| --- | --- |
| Repository | This Git repository, `BiscuitBin/pachinko`: both crates, the handbook, the specifications, the tickets and the tooling. Never template: nothing renders it. Never workspace for the whole: the workspace is the crates alone. |
| Workspace | The Cargo workspace at the root. Two members, `crates/libpachinko` and `crates/pachinko`. |
| Library | `libpachinko`, under `crates/libpachinko/`. It holds the domain once the specifications define one, and it performs no effect itself. Always the library or `libpachinko`; never core, engine or SDK, and never the crate, since there are two. |
| Service | `pachinko`, under `crates/pachinko/`: a minimal binary built around the library, whose `main` calls a tested `run()` and nothing else. It supplies the real implementations of the library's effect traits. Always the service or `pachinko`; never server, daemon or app, because no runtime, transport or HTTP stack is chosen (decision 0014). A bare `pachinko` names the service, never the repository. |
| Consumer | Whatever calls the library. The service is the only one today. |
| Effect | A reach outside the computation itself: the clock, randomness, the filesystem, the network, the environment or threads. The library performs none; see [Architecture](../explanation/architecture.md). Never side effect, and never I/O: standard input and output are not on the list, and the list is wider than I/O. |
| Effect trait | The trait the library defines for one effect, through which its code reaches that effect. The service supplies the real implementation and a test supplies the fake. None exists yet: each arrives with the specification that names its effect (decisions 0003 and 0008). Never port, adapter or interface. The effects boundary is the rule as a whole, never one trait. |
| Fake | The in-memory implementation of an effect trait, kept beside the trait in the library and used by tests. Not a mock: it behaves, rather than recording calls. Never mock or stub. |
| Specification | An `.allium` file under `docs/specs/`. Decides behaviour, and wins wherever the code disagrees. Never requirements or design document: a ticket says what to do next, a specification says what the code does. |
| Module | One specification, named by its file without the extension: `pachinko` is `docs/specs/pachinko.allium`, the root module and today the only one. `just plan-spec <module>` takes this name. Where a Rust module could be meant, say specification module. |
| Surface | A boundary in a specification: what is exposed, what operations are provided, and what is guaranteed. |
| Guarantee | A named prose assertion on a surface. Acceptance criteria, not aspiration. |
| Gate | A check that can fail the build. Listed in [Quality gates](../reference/quality-gates.md); `just check` runs them all. Never lint for a gate in general: `lint` is one recipe. Never test for a gate in general: `test` is a recipe outside `just check`, and the gate runs the tests through `coverage` and `test-doc`. |
| Recipe | A `Justfile` target. The only supported interface to the checks. Never script or task for one: a script under `scripts/` is what a recipe may call, and a missing recipe is added to the `Justfile` rather than replaced by a pipeline. |

## Related pages

- [Specifications](../explanation/specifications.md)
- [Architecture](../explanation/architecture.md)
- [Repository map](repository-map.md)
- [Commands](../reference/commands.md)
