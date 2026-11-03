---
id: T21
title: "Pocket log records: a faster service record store at no extra memory, measured and verified"
status: done
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
| `crates/pachinko/src/lib.rs`, `actor.rs`, `arrow.rs` | changed | `pub mod log`; the service's store; the shared `kernel`; the baseline doc |
| `crates/pachinko/tests/log_records.rs` | new | The contract and the differential test |
| `docs/decisions/0021-pocket-log-records.md`, `docs/decisions/README.md`, `docs/manifest.yml` | new, own lines | Decision 0021 |
| `docs/decisions/0020-records-as-arrow-batches.md` | changed | Superseded in part by 0021 |
| `docs/reference/testing.md`, `docs/reference/api.md`, `docs/explanation/architecture.md`, `docs/project/repository-map.md`, `docs/project/terminology.md`, `crates/pachinko/README.md` | changed | The comparative bench and the service's store |
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

### LogRecords and its tests: `72d5dc8`

`crates/pachinko/src/log.rs` and `log/{chunk,rows,index,batch,compact}.rs`. The design
departs from the plan in five recorded ways, each with its reason in a code comment:

- `rows.rs` owns row addressing;
- renumbering reuses the sorted rebuild;
- a dead tail chunk is cleared in place rather than dropped;
- the pull batch uses the public `arrow::batch_schema()`, so `arrow/schema.rs` is
  untouched;
- the key arena is bytes, so a repack can move them in place without unsafe code.

A repack moves bytes down in place with no allocation, once dead bytes pass both
live/16 and 64 B. Before measurement, the first version repacked a front-draining chunk
every few settles into fresh arenas; that was changed as a design defect, not tuned.

- **`just lock`** (network, authorised) added no `[[package]]`. The whole `Cargo.lock`
  diff is `+ "hashbrown",` in `pachinko`'s dependency list.
- **Limit.** More than 2^32 − 64 live records panics under
  `#[expect(clippy::panic, reason = ...)]`. That is the same ceiling ArrowRecords' `u32`
  pull indexes already set.
- **Tests.**
  - The module tests run under small `cfg(test)` limits (4 rows a chunk, 16 B and 32 B
    arenas, 32 chunk ids), so the seal, spill, renumber and repack branches are reached.
    They include `check_contract` and a 16-seed comparison with MemoryRecords.
  - `tests/log_records.rs` runs the contract and the differential test: 64 seeds × 10,000
    steps, 16 of them with out-of-order positions. Each seed must reach at least 192
    records and at least 5 `free_all` calls on an empty pocket, and each group of 16
    seeds at least 1,000 supersedes of held records.

### LogRecords measured: `b616a0b`

The bench and the probe ran over all three stores, measured exactly as for the baseline.

- **Verdicts.** M1, M3, M4, M5, T1, T2, T3 and T4 passed, and M2 failed at
  n = 1000 × 1 KiB (+0.14%).
- **T1 repeat runs.** Repeated `just bench -- <step>` runs, 4 for defer and 3 for each
  other T1 step, showed MemoryRecords' medians moving 20–50% with no code change.
- **M2 breakdown.** A byte breakdown of the churned pocket, from a temporary layout
  report reverted afterwards, accounts for all 16,009 bytes beyond a fresh pocket. 10,240
  of them are the key index: tombstones grow hashbrown from 2,048 to 4,096 buckets at a
  constant count, within the 4× trim rule. The rest is front-chunk slack within the
  stated rules.
- **No change to pass.** The breakdown found no defect, so the store was not changed to
  pass M2. Only the index comment, which was wrong about churn, was corrected, at
  `a20ede1`.

### The service on LogRecords, decision 0021: `09f8ced`

`ServiceQueue` is `Queue<LogRecords, SystemClock>`. All 29 Flight integration tests pass
on it. Decision 0021 is registered, 0020 carries "superseded in part", and the handbook
pages name the new store. `just check` at `09f8ced`:

```text
All checks passed and the worktree is unchanged.
```

### Verification, step 7: `09f8ced`

The verification ran as a workflow of 27 agents on `claude-opus-5-5`. The re-measurer,
the three refuters and the code reviewer never saw the ledger's verdicts or these notes,
only the code, the claim definitions and the raw outputs. The re-measurer alone read the
ledger, in order to diff against it.

- **Independent re-measurement.** It built from fresh target directories and was alone on
  the machine.
  - All 300 ledger byte figures matched to the byte, so no memory flag was raised.
  - 13 timings moved more than 20%: 9 of them MemoryRecords', and 3 LogRecords' (two
    faster).
  - One verdict flipped: T1 defer at 10k × 1 KiB, 3.434× MemoryRecords.
- **Refuters.** A claim fails when 2 of the 3 refute it. The refuters (correctness,
  numbers, repository invariants) refuted **M2** and **T1**, 2–0 each, and upheld every
  other claim, 2–0 or 3–0.
  - The correctness refuter added adversarial differential cases in its own detached
    worktree and found no divergence.
  - The invariants refuter ran `just check` itself, which passed.
- **Defects.** The code review (high) and the refuters reported 22 defects. Each went to
  its own adversarial verifier, and 14 were confirmed:

| Confirmed defect | Disposition |
| --- | --- |
| Memory grows without bound behind a pinned front row: dead chunks stay as placeholders, and a survivor's chunk keeps 64 slots (medium; three reports) | Fixed in `f0bb61f`: a waste-bounded sorted rebuild |
| `Queue::release` never calls `free_all` on an empty pocket, so an idle LogRecords pocket kept its state on the service's path, and M4 had measured a path the queue never takes (medium) | Fixed in `f0bb61f`: a settle that empties the pocket frees it; M4 is now also measured through `Queue` |
| M3 measured only a FIFO drain (medium) | Covered by the waste bound; the scattered-survivor case is a module test |
| The index's `Debug` printed a hasher-dependent bucket count (low) | Fixed: `len` only |
| The pull batch resolved each row about 5 times (low) | Fixed: once |
| The `# Panics` bound was off by 63 rows (low) | Fixed: 2^32 − 64 |
| Stale doc lines: the Cargo.toml dev-dependency comment, testing.md's module row, this ticket's Files touched (low) | Fixed |
| M2 and T1 fail (two reports) | Reported as failures |

  Of the 8 not confirmed, two concerned `codec.rs`'s test still building its queue over
  ArrowRecords; it tests the codec's message bounds, not the store.

### A second review round: `f0bb61f` to `a9efedc`

A read-only reviewer of the fix diff found no correctness bugs, and one medium cost flaw.
The rebuild trigger counted slots while a rebuild copies every live byte, so one pinned
64 MiB payload under tiny churn was copied about every 66 settles.

`a9efedc` counts waste and live in bytes. A rebuild fires once waste exceeds the live
bytes plus 1,536 and copies at most 176 B per death; a module test shows it does not
fire every few settles. The same commit leaves the cursor on the first queued row after a
rebuild.

The reviewer's low finding is accepted: a pocket that empties on every pull reallocates
its state on the next insert. It is the cost of freeing idle pockets, and the code
comment records it.

### The ledger on the shipped code: `a9efedc`

The bench is one full `just bench` and the probe ran 3 times. Machine: Apple M5 Pro,
64 GiB, macOS, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`. MemoryRecords is from the same run.

| # | Verdict | Measured |
| --- | --- | --- |
| M1 | PASS, 32/32 | 1000 × 1 KiB: Log 1,071,458 B against Arrow 1,084,810 B requested; 10k × 16 B: 61.36 against 76.31 B/record |
| M2 | **FAIL**, 1 of 13 sub-cases | 1000 × 1 KiB after churn: Log 1,087,467 against Arrow 1,085,936 B requested (+0.141%), 1,088,320 against 1,086,272 glibc-model; the other 12 pass |
| M3 | PASS | 1.0778 requested and 1.0777 glibc-model, a drained 1k pocket over a fresh one |
| M4 | PASS | Store path: Log 8 B a pocket, against Arrow's 1,840 (requested) and 2,136 (glibc-model). Through `Queue`, the whole queue's heap: Log 137.06 / 192.53, Arrow 1,759.55 / 2,071.02, Memory 1,733.18 / 1,813.88 B a pocket |
| M5 | PASS, 32/32 | Byte-for-byte equal to M1 at k = 7 and k = 1000 |
| T1 | **FAIL**, 8 of 10 sub-cases | defer: Log/Memory 3.210 (16 B) and 3.386 (1 KiB). Arrow/Log is at least 54.9 on every step, and the other Log/Memory ratios are at most 2.345 |
| T2 | PASS, 6/6 | At 10k, 16.83× and 8.36× faster than Arrow; Log ≤ Arrow at 100 and 1k |
| T3 | PASS | At 10k, 37.6× and 27.9× faster than Arrow, and 169× and 139× faster than Memory |
| T4 | PASS, 6/6 | The narrowest is pull_and_ack 10000 × 100 at 4.89× faster than Arrow |
| C1 | PASS | `check_contract::<LogRecords>` and the 64-seed × 10,000-step differential test pass in `just test` (256 run, 256 passed, 1 skipped); the correctness refuter found no divergence |
| C2 | PASS | See the final gate below |

Across the four full measurements (`b616a0b`, the independent `09f8ced`, `f0bb61f`,
`a9efedc`), every claim but T1 kept its verdict. T1 is decided by defer:

| Run | defer 16 B, Log/Memory | defer 1 KiB, Log/Memory |
| --- | --- | --- |
| `b616a0b` | 2.435 | 2.234 |
| `09f8ced` | 2.719 | **3.434** |
| `f0bb61f` | 2.063 | 2.165 |
| `a9efedc` | **3.210** | **3.386** |

LogRecords' defer stays at 14.7–15.4 µs and 42.4–43.2 µs per 100 ops. The verdict
follows MemoryRecords' median, which swings 4.81–7.19 µs and 12.51–19.96 µs. The median
of all seven runs that timed defer, 2.44 and 2.41, is under 3, but the claim is read run
by run, and it fails on the shipped code and on the independent re-measurement.

### Final gate: `aa31ae9`

`just check` at `aa31ae9`, the last commit before this section:

```text
TOTAL                                     10232               243    97.63%         889                30    96.63%        5404                90    98.33%           0                 0         -
All checks passed and the worktree is unchanged.
```

Line coverage is 98.33%, against a floor of 90%, and `git status --short` was empty.

## Open points

- **M2 and T1 fail as written, and no threshold moved.**
  - M2: a churned 1k × 1 KiB pocket is 0.14% over ArrowRecords, mostly a key index that
    churn's tombstones double. A tighter trim rule, shrinking whenever the table exceeds
    a fresh one's size, would likely close it, at one O(n) rehash about every 800 settles
    at 1k. It is a maintainer decision and was not applied.
  - T1: LogRecords' defer copies the latest version to the tail and returns a copy of the
    held one, about 3× MemoryRecords' in-place defer. The queue defers only a record
    superseded while held.
- **Timings are one macOS machine.** The image runs on Linux, and these figures were not
  measured there.
- **The glibc figures are modelled, not measured.** The model rounds every live
  allocation as glibc's malloc does, Σ max(32, round_up_16(size + 8)); no Linux run
  confirmed it.
- **The bench and the probe disagree by more than 2× on some steps.** At 10k: LogRecords'
  supersede (113.6 ns in the bench, 54.7 ns in the probe), and MemoryRecords' supersede
  and `free_all`. The bench starts each sample on a cold fresh clone. The ledger reads the
  bench, which treats every store alike.
- **Stacked on T20's unmerged branch.** This ticket sits on pull request #38 at `cb77f07`.
  If another T20 prototype is chosen, it is ported or dropped.
- **No push and no pull request.** Each needs separate authorisation.
