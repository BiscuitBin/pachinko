---
title: "Decision 0020: Records as Arrow batches in the service"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_records_as_arrow_batches]
requires: []
---

# Decision 0020: Records as Arrow batches in the service

## Context

The maintainer's brief for the T20 prototype, on 2026-10-31, asked for each pocket's
records to be an Arrow table, so that the prototype measures the queue with Arrow doing
the internal processing as well as the transport. The natural home for that is the
library, which holds the domain.

It cannot live there. `arrow-array` depends on `ahash` with its `runtime-rng` feature on
every target but wasm, and `runtime-rng` turns on `getrandom`, which reads the operating
system's randomness. `getrandom` is one of the eight crates `just effects` bans from
libpachinko's normal dependency tree ([decision 0008](0008-std-library.md)), and the ban
is right: a library that seeds its hashing from the operating system performs an effect
itself, whoever wrote the crate that does it.

## Decision

**The library owns a record-store trait.** `libpachinko::records::PocketRecords` is the
record-level steps the module's rules take and nothing more: look a key up, insert a
queued record at a position, supersede in place, hold the oldest queued records as a
batch, settle a held key, defer a key at a fresh position, free every held and deferred
record, count the records and list the held ones. Each pocket owns one instance. The
queue, `libpachinko::queue::Queue`, is generic over it and decides every rule; the store only
carries out a step the queue has already decided.

**`MemoryRecords` is the trait's reference.** It sits beside the trait in libpachinko, a
`BTreeMap` from record key to record with an index of queued positions, and it is the
store every library test runs on. It is not a benchmark subject.

**`ArrowRecords` is the service's store.** In `crates/pachinko`, each pocket's records are
one `arrow_array::RecordBatch` with the columns `record_key`, `rank`, `payload`,
`position`, `status`, `held_rank` and `held_payload`, and every step rebuilds the batch
with the Arrow kernels: `arrow_ord` compares and sorts, `arrow_select` filters, takes,
zips and concatenates. The rebuild cost is what the prototype measures. A batch handed to
a consumer is itself a `RecordBatch`, so the Flight service sends it without copying rows.

**One contract proves both.** The behaviour every store must show is written once, as
`libpachinko::records::contract::check_contract` behind libpachinko's `testing` feature,
and run by libpachinko's tests over `MemoryRecords` and by the service's tests over
`ArrowRecords`.

**The comparison is deferred.** A benchmark of `ArrowRecords` against `MemoryRecords`, or
against any other store behind the trait, is a later ticket. Today `just bench` measures
the Arrow store alone.

## Consequences

The effects ban stands with no gate, script, fixture or ban list changed, and `just
effects` still fails if an Arrow crate reaches libpachinko's normal tree.

The trait is a second public contract in the library, and the queue cannot use anything
Arrow offers that the trait does not name. A store that wanted to do more of a rule's
work, such as pulling and holding in one pass over columns it already sorted, would need
the trait widened.

Every step rebuilds a whole pocket's batch, so one step costs time in proportion to the
pocket's size, where the `BTreeMap` store costs a logarithm. For pockets of thousands of
records that is the cost the prototype is meant to show, not a defect to hide.

`ArrowRecords`'s trait methods cannot fail, but the Arrow kernels return `Result`. The
schema is fixed by the module that builds every batch, so a kernel error is a programming
error, and the store turns it into a panic in one place with one stated reason.

## What would reopen this

A benchmark that shows the Arrow store loses to the reference store at the sizes the
maintainer cares about; an `arrow-array` release that drops `ahash`'s `runtime-rng`, which
would let the store move into the library; or a specification that needs the store to do
work the trait does not name.

## Related pages

- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
- [Decision 0019: The service runs on tokio and speaks Arrow Flight](0019-service-runtime-tokio-and-flight.md)
- [Architecture](../explanation/architecture.md)
- [Testing](../reference/testing.md)
