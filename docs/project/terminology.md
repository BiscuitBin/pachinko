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

The domain's words come from the Vocabulary section of the root module,
`docs/specs/pachinko.allium`, which picks one word where several would do. Each row
below names that module. When a row and the module disagree, the module is right and the
row is corrected. A ticket that adds a domain word to a specification adds its row here,
naming the module it comes from.

| Term | Meaning (module `pachinko`) |
| --- | --- |
| Pocket | One entry of the queue, named by a pocket key; it owns its records. Queued pockets are ordered by when they were enqueued, oldest first, and that order is the queue. Never group, partition or primary-queue entry. |
| Record | The one slot a record key has in a pocket, carrying the latest rank and payload. Never message, item or event. |
| Rank | The publisher-supplied order between the versions of one key; higher wins, and pachinko only compares. A version is one publish of a key and its rank orders it; the rank itself is never called a version. |
| Payload | The opaque bytes a record carries; pachinko never reads them. Typed `String` in the module because Allium 3 has no byte type. Never body or value. |
| Supersede | A higher rank replaces a record's rank and payload in place; the record keeps its slot. Never duel, dedupe or upsert. |
| Superseded record | A payload that lost a supersede, or arrived with a lower rank than stored and is marked stale, kept visible until it decays. Never dropped: nothing is dropped silently. |
| Dead letter | A nacked payload, kept visible until it decays. Never poison message. |
| Lease | A consumer's exclusive, renewable claim on one pocket: one per consumer and one per pocket at a time. Pull, ack, nack and renew extend it; release ends it. Never lock. |
| Consumer | The party that pulls, acks, nacks, renews and releases. Never holder, subscriber or worker, and never the library's caller, which is the row below. |
| Publisher | The party that publishes records into pockets. Never producer. |
| Batch | What a pull hands back: the pocket key and zero or more records, oldest first. An empty batch is not an error. |
| Queued, leased, idle, parked | A pocket's states. Queued waits in the pocket order; leased is held by one consumer; idle has no records; parked was set aside after consecutive lease expiries until a publish revives it. |
| Queued, held, deferred | A record's states. Queued waits in its pocket; held has been handed to the pocket's consumer; deferred means a newer version arrived while the key was held and the held version was acked or nacked, so it restarts at the back and is pullable again once the lease ends. |
| Pocket order, the pocket's records | Say the pocket order for the queue of pockets and the pocket's records for what one pocket holds. Never primary queue or secondary queue. |

## The repository

| Term | Meaning |
| --- | --- |
| Repository | This Git repository, `BiscuitBin/pachinko`: both crates, the handbook, the specifications, the tickets and the tooling. Never template: nothing renders it. Never workspace for the whole: the workspace is the crates alone. |
| Workspace | The Cargo workspace at the root. Two members, `crates/libpachinko` and `crates/pachinko`. |
| Library | `libpachinko`, under `crates/libpachinko/`. It will hold the domain the specifications define, and it performs no effect itself. Always the library or `libpachinko`; never core, engine or SDK, and never the crate, since there are two. |
| Service | `pachinko`, under `crates/pachinko/`: a minimal binary built around the library, whose `main` calls a tested `run()` and nothing else. It supplies the real implementations of the library's effect traits. Always the service or `pachinko`; never server, daemon or app, because no runtime, transport or HTTP stack is chosen (decision 0014). A bare `pachinko` names the service, never the repository. |
| Caller | Whatever calls the library. The service is the only one today. Never consumer, which is a domain word: the party that pulls from a pocket. |
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
