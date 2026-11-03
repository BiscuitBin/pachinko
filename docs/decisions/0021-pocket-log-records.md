---
title: "Decision 0021: The service keeps each pocket's records as a log"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_pocket_log_records]
requires: []
---

# Decision 0021: The service keeps each pocket's records as a log

*Supersedes [decision 0020](0020-records-as-arrow-batches.md) in part.*

## Context

[Decision 0020](0020-records-as-arrow-batches.md) made `ArrowRecords` the service's
record store: each pocket's records are one `RecordBatch`, and every step rebuilds it with
the Arrow kernels. Nothing indexes it, so a step costs time in proportion to the pocket's
size. A scratch probe at `cb77f07` measured a lookup at 15.1 µs and an insert at 38.6 µs
in a pocket of 10,000 records, where `MemoryRecords`, the trait's reference, took 22 ns
and 109 ns. Memory ran the other way: about 75 bytes a record for the Arrow store at
16-byte payloads, and about 293 for the reference.

Ticket T21 set out to find a store faster than both at no more memory than the Arrow
store, and wrote its claims and their thresholds into a ledger before any code. Before
the ticket the maintainer chose a new store, `LogRecords`, chose its index,
`hashbrown::HashTable<u32>`, and chose to keep `ArrowRecords` as the baseline the
benchmarks measure against.

## Decision

**`LogRecords` is the service's store.** It lives in `crates/pachinko/src/log.rs`, and
`actor::ServiceQueue` is `Queue<LogRecords, SystemClock>`. Each pocket's records are a log
in position order, with a hash index by key:

- **The empty pocket.** `LogRecords` is one `Option<Box<Inner>>`: an empty pocket costs
  8 bytes and no heap. A pocket frees its buffers as soon as a settle or `free_all`
  leaves it with no records. The queue's release of an empty pocket never calls the
  store, so the store cannot wait for it.
- **Chunks.** `Inner` holds a `VecDeque` of chunks in position order. A chunk has up to 64
  rows, a key arena, a payload arena, and live and dead counters. Each row is a 24-byte
  slot: the latest rank, the full 64-bit position, where the key ends with the status and
  flag bits above it, and where the payload ends. A chunk seals before an arena outgrows
  its offset width, and a key or payload too large for an empty chunk is stored out of
  line, keyed by its row id, so no step can fail on size.
- **The index.** A `hashbrown::HashTable<u32>` maps a key to its row id, the chunk's id
  above six slot bits. It stores only the id and reads the key's bytes back from the log,
  about 5 bytes a key at full load, and it shrinks once its buckets reach four times its
  length.
- **Fast paths.** The queue's positions only increase, so an insert and a defer append at
  the tail and `hold_oldest` walks forward from a cursor no queued record precedes. A
  position that does not increase, which only the records contract and the doc examples
  hand out, rebuilds the store sorted by position in O(n log n).
- **Held versions.** A held record is a status flag. A second copy of its payload exists
  only once a supersede lands on it while it is held.
- **Settling.** A settle marks the row dead and removes its index entry. Fully dead front
  chunks are popped, and a chunk's arenas are repacked once its dead bytes pass a
  sixteenth of its live bytes. A repack never moves a slot to another chunk, so no row id
  changes. A chunk can die behind a live one, for example behind a record a consumer
  holds while it acks everything after it. So once the bytes held by dead slots and dead
  chunks' headers outweigh the live records' bytes (their slots, keys and payloads) by
  more than 1,536, the store rebuilds itself in position order. A rebuild copies less
  than twice that waste, and each death adds at most 88 bytes of it, so a step stays
  amortised O(1) in bytes copied, and the waste stays within the live bytes plus 1,536.

**The pull batch does not change.** `hold_oldest` builds a `RecordBatch` of the held
versions' `record_key`, `rank` and `payload`, oldest first, with exact-capacity builders:
the batch `ArrowRecords` hands out, read by the same `pachinko::arrow::held_records`. The
actor, the codec and the wire format are as decision 0020 left them.

**`ArrowRecords` stays, as the baseline.** It is not the service's store, but it keeps
its module, its contract test and its place in the benchmarks, so that every later store
is measured against the design it replaced.

**The index's hasher is a per-store `std::hash::RandomState`.** A publisher chooses the
keys, and a fixed hasher would let one choose keys that collide and turn every lookup in a
pocket into a scan. The standard library seeds `RandomState`'s keys from the operating
system's randomness once per thread and varies them for each new hasher. That is an
effect, and it is the service's, in `pachinko`, where effects belong: libpachinko gains no
dependency and no effect, so `just effects` is unaffected. The index is never iterated
and its `Debug` prints only its length, so neither the random order nor the bucket count
it shapes after churn can reach a pull, a status reply or a test's output.

**hashbrown comes in under decision 0007.** Its note sits beside its line in the
workspace `Cargo.toml`: default features off, so no default hasher and no
`allocator-api2`, and already in `Cargo.lock` through `arrow-array` and `indexmap`, so it
adds no crate. It is never a libpachinko dependency.

**The limits.** An insert or a defer panics when a pocket would hold more than 2^32 − 64
live records ((2^26 − 1) chunks of 64 rows, fewer if large keys or payloads seal chunks
early), whose slots alone take about 96 GiB, because a row id is a `u32`.
`ArrowRecords` has the same ceiling, because its pull indexes are `u32`. No other size
panics: a key or payload too large for a chunk's offsets goes out of line.

**What it supersedes.** Of decision 0020, this record replaces "`ArrowRecords` is the
service's store" and "The comparison is deferred". The rest stands: the library owns the
`PocketRecords` trait, `MemoryRecords` is its reference, one contract proves every store,
and no Arrow store can live in libpachinko while `arrow-array` brings in `getrandom`.

## Evidence

`crates/pachinko/benches/records.rs`, which `just bench` runs, times every store's steps
at 100, 1,000 and 10,000 records with 16-byte and 1 KiB payloads, and `publish` and
`pull_and_ack` through `Queue` on the fake clock, over `MemoryRecords`, `ArrowRecords` and
`LogRecords` in one run. The memory figures come from a scratch probe that was never
committed: a counting global allocator over the requested sizes, the same sizes rounded
as glibc's allocator rounds them, three runs per store, each in its own release process.

The figures below are from commit `a9efedc`, the code this decision ships, on one macOS
machine, an Apple M5 Pro with 64 GiB and `rustc 1.99.1`. The timings are divan medians of
one full `just bench` and hold only for that machine; the byte counts were identical in
all three probe runs. The ledger was measured four times in full: at `b616a0b`, by an
independent re-measurement from a clean build at `09f8ced`, and after each round of
review fixes, at `f0bb61f` and `a9efedc`.

| At 10,000 records, 16-byte payloads | MemoryRecords | ArrowRecords | LogRecords |
| --- | --- | --- | --- |
| lookup | 36.79 ns | 14.22 µs | 12.3 ns |
| insert, per op | 74.15 ns | 41.09 µs | 57.99 ns |
| supersede | 356.4 ns | 26.98 µs | 95.5 ns |
| settle, per op | 102.9 ns | 35.3 µs | 98.75 ns |
| defer, per op | 48.1 ns | 22.18 µs | 154.4 ns |
| `hold_oldest(100)` | 26.59 µs | 33.32 µs | 1.98 µs |
| `free_all`, 100 held and 100 deferred | 83.14 µs | 18.47 µs | 491.6 ns |
| `publish`, 10,000 records in 10 pockets | 2.35 ms | 36.32 ms | 1.853 ms |
| `pull_and_ack`, 10,000 records in 10 pockets | 2.735 ms | 43.02 ms | 2.322 ms |
| Heap bytes a record, requested / glibc-model | 283 / 345.13 | 76.31 / 76.35 | 61.36 / 62.12 |
| The same at 1 KiB payloads | 1,291 / 1,353.13 | 1,084.31 / 1,084.35 | 1,069.45 / 1,070.21 |
| Bytes an idle, released pocket, inline size included, requested / glibc-model | 1,576 / 1,600 | 1,840 / 2,136 | 8 / 8 |
| Bytes an idle pocket released through the queue (whole queue heap per pocket), requested / glibc-model | 1,733.18 / 1,813.88 | 1,759.55 / 2,071.02 | 137.06 / 192.53 |

The last row goes through the queue: each pocket gets one record, which is pulled, acked
and released, and the figure is the whole queue's heap per pocket.

Against its ledger, `LogRecords` passed every memory claim but one: insert-only pockets,
sparse positions, a pocket drained from 100,000 records to 1,000 (1.078 times a fresh
one) and idle pockets. It passed every timing claim but one: `hold_oldest` and `free_all`
past their margins, and `publish` and `pull_and_ack` faster than `ArrowRecords` on every
shape. The ledger's behaviour and gate rows, C1 and C2, are recorded with the ticket's
hand-back.

**M2, memory after churn: a failure.** After 4,000 publish, pull and ack cycles, a pocket
of 1,000 records with 1 KiB payloads holds 1,087,467 bytes against `ArrowRecords`'
1,085,936: 0.14% more requested, and 0.19% more in the glibc model. All four
measurements agree to within 16 bytes. The other twelve sub-cases of the claim pass,
including 10,000 records at 1 KiB, and the peak during churn stays far below the Arrow
store's in every case. The claim fails as written, and its threshold did not move. A
byte-by-byte breakdown of the churned pocket accounts for all 16,009 bytes it held
beyond a fresh one at `b616a0b`. Ten thousand of them are the key index: settles leave
tombstones in the hash table, and under steady churn hashbrown grows it from 2,048 to
4,096 buckets at a constant record count. That is within the index's shrink rule, which
waits until the table is four times what the records need. The rest is slack the store's
other rules allow: dead and spare payload bytes in the front chunk, its dead slots, and
the deque's capacity. None of it is a defect, so the store was not changed to pass the
claim. A tighter index rule, shrinking whenever the table exceeds a fresh one's size, is
left as an option.

**T1, the step timings: a failure.** `ArrowRecords` is never close: it is at least 52
times slower than `LogRecords` in every run of the claim's five steps. The bound against
`MemoryRecords`, within three times, holds for lookup, insert, supersede and settle in
every run, and fails for `defer`. Across the four full runs `defer` at 10,000 records
took 2.17 to 3.43 times as long as `MemoryRecords` at 1 KiB, and the claim failed in two
of them: the independent re-measurement and the run of the shipped code, at 3.21 (16
bytes) and 3.39 (1 KiB). `LogRecords`' own `defer` barely moves between runs, 42.4 to
43.2 µs for 100 at 1 KiB; `MemoryRecords`' moves by 20 to 60%, and its fast runs are the
failing ones. The median over all seven runs that timed it, 2.44 at 16 bytes and 2.41 at
1 KiB, is under three, but the claim is read run by run, so it fails as written. A
`LogRecords` defer copies the record's latest version to the tail of the log and returns
a copy of the held one, where `MemoryRecords` changes the record in place and copies only
the held version; the queue defers only a record superseded while held, which is the
rarer path.

## Consequences

The service's steps cost a hash lookup or an append, not a rebuild of the pocket, and an
idle pocket costs a pointer. A pull still hands the Flight service a `RecordBatch`, so the
transport sends it without copying rows.

The store is harder to read than either alternative: five submodules, bit-packed slots,
row ids that encode a chunk and a slot, and two layouts of the same data, in the arenas
and out of line. The records contract and a differential test against `MemoryRecords`
over random steps, `crates/pachinko/tests/log_records.rs`, are what make that tolerable;
the store's correctness rests on them more than on reading it.

The fast paths assume the queue's positions only increase, which `libpachinko`'s pockets
guarantee today. A queue that handed out positions out of order would land on the
rebuild on every out-of-order insert or defer, and those steps would cost what
`ArrowRecords`' steps cost.

Every pocket's index has a hasher of its own, keyed from the operating system's
randomness. That is a new use of an effect the service may already perform, not a new
kind of effect in the library, so no trait is added.

Three stores now carry the contract, and the benchmark compares them in one run, so a
change to the trait is a change to three implementations.

## What would reopen this

A workload whose positions do not only increase; a memory claim at a shape the ledger did
not measure that fails by more than M2 does; a widening of `PocketRecords` that the log
cannot serve without a rebuild; or a benchmark on the deployment's own machine, glibc on
Linux, that reverses the timing claims.

## Related pages

- [Decision 0007: Dependency policy: caret ranges and `Cargo.lock`](0007-dependency-policy.md)
- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
- [Decision 0020: Records as Arrow batches in the service](0020-records-as-arrow-batches.md)
- [Architecture](../explanation/architecture.md)
- [Testing](../reference/testing.md)
