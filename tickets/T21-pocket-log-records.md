---
id: T21
title: "Pocket log records: a faster service record store at no extra memory, measured and verified"
status: in-progress
depends_on: [T20]
parallel_with: []
branch: ticket/t21-pocket-log-records
estimated_size: L
---

# T21: Pocket log records: a faster service record store at no extra memory, measured and verified

## Context

The service's `ArrowRecords` (decision 0020) rebuilds the whole pocket on every step, and
nothing indexes it. A scratch probe at `cb77f07` measured these medians at 10k records,
with 16 B payloads:

| Step | ArrowRecords | MemoryRecords |
| --- | --- | --- |
| lookup | 15.1 µs | 22 ns |
| insert | 38.6 µs | 109 ns |
| settle | 51 µs | 89 ns |

Memory runs the other way. ArrowRecords holds about 75 B per record at 16 B payloads, and
MemoryRecords about 293 B. A read-only design study (four independent designs, each with
two adversarial reviews) estimated that a new store could be 10–1000× faster per step at
10k records while using less memory than ArrowRecords. Those figures are estimates. This
ticket measures them, proves or disproves them, and has them verified independently.

**Stacking.** `PocketRecords` and `ArrowRecords` exist only on T20's unmerged branch
(pull request #38, head `cb77f07`), so this ticket branches from `cb77f07` rather than
`main`, and is stacked on #38. If another T20 prototype is chosen instead (pull
requests #36 and #39), this ticket is ported or dropped.

**Decisions the maintainer made before this ticket:**

- The service gets a new store, `LogRecords`. ArrowRecords stays, as the benchmark
  baseline. Decision 0021 partly supersedes 0020.
- The key index is `hashbrown::HashTable<u32>`, with its decision-0007 note and a
  `just lock` run.
- The work is a new ticket in a new worktree.

## Claims ledger

Written before any code. Each claim gets a measured value and a pass or a fail. A failure
is reported as a failure, and no threshold moves after the baseline run.

| # | Claim | Threshold to pass |
| --- | --- | --- |
| M1 | Live bytes per record, insert-only, 1k and 10k (also n = 897 and 7169, just after index growth), 16 B and 1 KiB payloads | LogRecords ≤ ArrowRecords, both in requested bytes and in glibc-model bytes |
| M2 | Same, after 4n publish → pull → ack cycles (churn) | ≤ ArrowRecords at the same live n |
| M3 | Drain: grow to 100k, settle down to 1k | Within 1.25× of a fresh 1k LogRecords pocket |
| M4 | 10k idle (empty, released) pockets | Fewer bytes than 10k empty ArrowRecords |
| M5 | Sparse positions (each pocket gets every k-th global position) | M1 still holds |
| T1 | lookup, insert, supersede, settle, defer at 10k | ≥ 10× faster than ArrowRecords, and ≤ 3× MemoryRecords |
| T2 | hold_oldest(100) at 100, 1k and 10k | ≤ ArrowRecords at every size, and ≥ 5× faster at 10k |
| T3 | free_all at 10k, with 100 held and 100 deferred | ≥ 5× faster than both stores |
| T4 | `publish` and `pull_and_ack` through `Queue` (the existing bench shapes) | Faster than ArrowRecords on every shape |
| C1 | Behaviour | `check_contract` passes, and the differential test agrees with MemoryRecords over ≥ 10k random steps × 64 seeds, including out-of-order positions |
| C2 | Gate | `just check` ends with `All checks passed and the worktree is unchanged.`, coverage ≥ 90% |

How the ledger is read:

- **Requested bytes** come from a counting global allocator over the sizes requested.
- **glibc-model bytes** apply the shipped allocator's rounding to every live allocation:
  Σ max(32, round_up_16(request + 8)). The image is distroless on glibc (`Dockerfile`),
  so this catches the per-allocation overhead that requested bytes never see.
- **MemoryRecords in T1 and T3** means MemoryRecords as measured in step 5's run, after
  step 3's quick wins. Step 2's pre-change figures are recorded beside them.
- **Timings** are divan medians and the probe's medians of 3 runs, on one macOS machine.
  A step is "faster" when its median is lower.

## Design

`LogRecords` lives in `crates/pachinko/src/log.rs` and `log/{chunk,index,batch,compact}.rs`.

- **Empty store.** `LogRecords { inner: Option<Box<Inner>> }`. An empty pocket costs 8 B
  and no heap, so the hasher state lives in `Inner`. `Inner` is dropped only when
  `free_all` finds 0 records, not on every settle that reaches 0.
- **Chunks.** `Inner` holds a `VecDeque<Chunk>` in position order. Each chunk has up to 64
  rows, a key arena, a payload arena, and live and dead counters.
- **Slots.** Each row is a 24 B slot: `rank: i64`, `position: u64` (full width, so a sparse
  position never splits a chunk), `key_end: u32` with the status bits and flag bits in its
  top bits, and `payload_end: u32`. A chunk seals before an arena outgrows its offset
  width. A key or payload too large for any chunk's offsets is stored out of line, so no
  step can fail.
- **Index.** `hashbrown::HashTable<u32>` maps a key to its row id (chunk id and slot). It is
  never iterated, so its order cannot leak. It shrinks when it is 4× larger than the live
  count needs. The hasher is a per-store `std::hash::RandomState`, which resists hash
  flooding. That is randomness on the service side, and decision 0021 records it.
- **Fast paths and the fallback.** The queue's positions only increase, so insert and defer
  append to the tail, and `hold_oldest` walks forward from a cursor. The store tracks the
  last position, a `held_end` and the deferred row ids. Out-of-order positions occur only
  in the contract and the doc examples; for those the store rebuilds itself sorted in
  O(n log n).
- **Held version.** A held record is a status flag. A second copy of its payload exists only
  when a supersede lands on a held record.
- **Settle.** It marks the row dead and removes its index entry. Fully dead front chunks are
  popped. A chunk's arena is repacked once its dead bytes pass live/16, and a repack never
  moves a slot to another chunk, so no row id changes.
- **Pull.** `hold_oldest` builds the pull `RecordBatch` with exact-capacity builders. The
  actor, the codec and the wire format do not change.

MemoryRecords also gets two quick wins, each measured before and after: `hold_oldest`
stops cloning the latest version into `held`, and `settle` takes the held version instead
of cloning it.

## Goal

The service runs on `LogRecords`, which passes the records contract and a differential
test against `MemoryRecords`; every ledger row carries a measured value and a pass or a
fail, confirmed by an independent re-measurement; decision 0021 records the store with its
evidence; `just check` is green.

## Non-goals

- No change to `PocketRecords`, `docs/specs/pachinko.allium`, `codec.rs` or the wire format.
- No change to libpachinko's dependencies, so `just effects` is unaffected.
- No new kind of effect in libpachinko. The hasher's randomness is the service's own.
- No unsafe code and no `as` casts.
- No threshold moves after step 2's baseline run.
- No push and no pull request; each needs separate authorisation.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/T21-pocket-log-records.md`, `tickets/README.md` | new, own row | This ticket and its index row |
| `crates/pachinko/benches/records.rs` | new | The comparative divan bench over the three stores; replaces `benches/arrow.rs` |
| `crates/pachinko/benches/arrow.rs` | removed | Replaced by `records.rs` |
| `crates/pachinko/Cargo.toml` | shared | `hashbrown`; the bench target |
| `Cargo.toml`, `Cargo.lock` | shared, own lines | `hashbrown` with its decision-0007 note |
| `crates/libpachinko/src/records/memory.rs` | changed | The two quick wins |
| `crates/pachinko/src/log.rs` and `log/*.rs` | new | `LogRecords` |
| `crates/pachinko/src/lib.rs`, `actor.rs`, `arrow.rs`, `arrow/schema.rs` | changed | `pub mod log`; the service's store; the shared `kernel` and `BATCH`; the baseline doc |
| `crates/pachinko/tests/log_records.rs` | new | The contract and the differential test |
| `docs/decisions/0021-pocket-log-records.md`, `docs/decisions/README.md`, `docs/manifest.yml` | new, own lines | Decision 0021 |
| `docs/decisions/0020-records-as-arrow-batches.md` | changed | Superseded in part by 0021 |
| `docs/reference/testing.md`, `docs/explanation/architecture.md`, `docs/project/repository-map.md` | changed | The comparative bench and the service's store |
| `CHANGELOG.md` | changed | The service's new store |

## Steps

0. **Set up.** Create the worktree from `cb77f07`, run `just initialize` (§11; hooks are
   skipped in a secondary worktree) and `just check` on the untouched tree.
1. **Write this ticket**, with the ledger, before any code.
2. **Baseline.** Add `benches/records.rs`, generic over `R: PocketRecords`, timing every
   step at 100, 1k and 10k records with 16 B and 1 KiB payloads, plus `publish` and
   `pull_and_ack` through `Queue`, with `divan::AllocProfiler`. Extend the scratch probe to
   M1–M5 (requested and glibc-model bytes; release, offline; 3 runs, median with min and
   max). Run both over ArrowRecords and MemoryRecords and record the results here.
3. **MemoryRecords quick wins.** Rerun the contract and the proptest state machine, and
   re-measure.
4. **LogRecords**, with `hashbrown` and its note, `just lock`, and the tests.
5. **Measure LogRecords** exactly as in step 2 and fill the ledger. Fix only a real defect,
   and record what changed and why.
6. **Switch the service and write decision 0021**, with the handbook pages above.
7. **Verify.** An independent re-measurement from a clean build; three adversarial
   refuters (correctness, numbers, repository invariants), where a claim fails if a
   majority refutes it; and a code review. Fix confirmed findings, then rerun the affected
   measurements and `just check`.

   **Adversarial review.** Step 7 is this ticket's checkpoint under §14: reviewers that
   never see the implementer's conclusions, only the code and the raw outputs.
8. **Hand back.**

## Acceptance criteria

- Every ledger row has a measured value and a pass or a fail, from step 5, confirmed by
  step 7's re-measurement.
- `check_contract::<LogRecords>` and the differential test pass, and the Flight
  integration tests pass with the service on `LogRecords`.
- `just lock` adds no new crate to `Cargo.lock`, and `just deny` and `just deps-unused`
  are green.
- Decision 0021 exists, is registered and listed, and 0020 carries its note.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing after the final commit.

## Verification

```sh
just test
just coverage
just deny
just deps-unused
just check-docs
just check-agents
just bench
just check
git status --short
```

## Hand-back notes

The raw logs, bench and probe outputs, briefs, findings and verdicts are under `ai_tmp/`,
not committed.

### Worktree and baseline: `cb77f07`

The worktree `T21-pocket-log-records` was created with
`supacode repo worktree-new --branch ticket/t21-pocket-log-records --base cb77f07`. The
plan authorised `just install-allium`; `just initialize` ran instead, as §11 requires of
every new worktree, and it skipped `install-hooks` in this secondary worktree. Then
`just check` on the untouched tree ended:

```text
All checks passed and the worktree is unchanged.
```

### Baseline, before any store change: `a548785`

Apple M5 Pro, 64 GiB, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`. No other `cargo` or `rustc`
process ran during either measurement.

- **Timings.** `just bench` runs divan, with medians of 100 samples. `insert`, `settle` and
  `defer` time 100 ops on one fresh clone, and the per-op figure below is the median ÷
  100. Every other step times one op on a fresh clone, except `lookup`, which runs on one
  shared pocket.
- **Memory.** The scratch probe runs 3 times in release, offline, with each store in its
  own process; every memory figure was identical in all 3 runs.
- **Bench against probe.** At 10k, MemoryRecords' bench and probe timings disagree by up
  to 11.6× (supersede: 485.4 ns in the bench, 41.7 ns in the probe). The bench's sample
  starts on a cold fresh clone, while the probe repeats one key on one clone. ArrowRecords
  agrees within 1.5×. The ledger reads the bench, which treats every store the same way.

| Step at 10k (16 B / 1 KiB) | MemoryRecords | ArrowRecords |
| --- | --- | --- |
| lookup | 34.9 / 33.92 ns | 13.37 / 14.14 µs |
| insert, per op | 77.63 / 78.68 ns | 45.64 / 171.7 µs |
| supersede | 485.4 / 622.1 ns | 27.21 / 252 µs |
| settle, per op | 125.4 / 362.3 ns | 37.93 / 188.1 µs |
| defer, per op | 47.7 / 66.45 ns | 27.43 / 25.05 µs |
| hold_oldest(100) | 36.12 / 52.93 µs | 35.26 / 37.17 µs |
| free_all, 100 held and 100 deferred | 74.77 / 67.31 µs | 19.5 / 19.21 µs |

| hold_oldest(100) (16 B / 1 KiB) | MemoryRecords | ArrowRecords |
| --- | --- | --- |
| 100 records | 4.433 / 10.61 µs | 2.869 / 4.871 µs |
| 1,000 records | 6.272 / 24.13 µs | 5.729 / 7.942 µs |

| Queue bench | MemoryRecords | ArrowRecords |
| --- | --- | --- |
| publish, 1000 records x 10 pockets | 164.6 µs | 1.79 ms |
| publish, 10000 records x 10 pockets | 2.354 ms | 38.24 ms |
| publish, 10000 records x 100 pockets | 2.044 ms | 20.21 ms |
| pull_and_ack, 1000 records x 10 pockets | 210.7 µs | 1.278 ms |
| pull_and_ack, 10000 records x 10 pockets | 2.745 ms | 44.14 ms |
| pull_and_ack, 10000 records x 100 pockets | 2.26 ms | 12.9 ms |

| Memory, heap bytes per record (requested / glibc-model) | MemoryRecords | ArrowRecords |
| --- | --- | --- |
| M1, 897 x 16 B | 281.03 / 345.17 | 77.02 / 77.41 |
| M1, 1000 x 16 B | 281.45 / 345.58 | 76.81 / 77.15 |
| M1, 1000 x 1 KiB | 1,289.45 / 1,353.58 | 1,084.81 / 1,085.15 |
| M1, 7169 x 16 B | 283 / 345.22 | 76.35 / 76.39 |
| M1, 10000 x 16 B | 283 / 345.13 | 76.31 / 76.35 |
| M1, 10000 x 1 KiB | 1,291 / 1,353.13 | 1,084.31 / 1,084.35 |
| M2 end, 1000 x 16 B | 295.70 / 357.66 | 77.94 / 78.27 |
| M2 end, 10000 x 1 KiB | 1,306.74 / 1,366.74 | 1,085.42 / 1,085.46 |
| M4, 10k released pockets, bytes per pocket with inline size | 1,576 / 1,600 | 1,840 / 2,136 |
| M4, 10k default pockets, bytes per pocket with inline size | 48 / 48 | 1,592 / 1,848 |

M3 (drained 100k → 1k against a fresh 1k pocket) is 1.051 requested and 1.029
glibc-model for MemoryRecords. ArrowRecords is skipped there, because building 100k
records takes a full-batch rebuild per insert. M5 (positions `i × k`, k = 7 and 1000) is
byte-for-byte equal to M1 for both stores.

### MemoryRecords' quick wins: `35a3037`

Same machine. The bench ran `just bench -- MemoryRecords`, and the probe ran as above.
At 10k, before → after (16 B / 1 KiB):

- settle: 125.4 → 98.53 ns and 362.3 → 128.9 ns per op;
- supersede: 485.4 → 339.6 ns and 622.1 → 523.2 ns;
- hold_oldest(100): 36.12 → 24.14 µs and 52.93 → 44.59 µs;
- free_all: 74.77 → 60.61 µs and 67.31 → 54.51 µs;
- pull_and_ack: 7.8% to 14.4% faster on every shape.

`defer` is slower: at 10k x 1 KiB it went from 66.45 to 126.1 ns per op, and at 100 and
1k it is 23% to 135% slower. The bench defers held records that were never superseded,
so the held version is now cloned from the latest one instead of moved. The queue defers
only when the held rank differs from the latest (`pockets.rs:219`), that is after a
supersede has made the copy, so the queue's own defers still move it.

Memory figures M1–M4 are unchanged, except that M2's peak fell (1000 x 1 KiB: 1,645.98
→ 1,543.58 B/record). T1 and T3 compare against these post-change figures.
