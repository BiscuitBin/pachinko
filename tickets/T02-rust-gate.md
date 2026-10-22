---
id: T02
title: "Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code"
status: open
depends_on: [T00]
parallel_with: [T01, T03, T04, T05, T06, T07, T08, T09]
branch: ticket/t02-rust-gate
estimated_size: M
---

# T02: Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code

## Context

T00 landed both crates and working copies of L's Rust tool configuration, with the deltas
`CONVENTIONS.md` §3 names, green on the day's stable. This lane makes that configuration
final and proves each Rust recipe does what its comment says, on this workspace rather
than on L's: a std library, a binary, no dependencies, one item each.

Read L `tickets/T02-rust-gate.md` for the method (each recipe proved red as well as
green) and L `docs/decisions/0009-rust-quality-gate.md` for why each setting is what it
is. L `tickets/T29-snapshot-harness.md` is the snapshot harness: its recipes are already
in the `Justfile`, and this lane proves them on a workspace that has no snapshot yet.

## Goal

`fmt-check`, `toml-check`, `clippy`, `features`, `test`, `coverage`, `doc`, `deny`,
`deps-unused` and `snapshots-check` are each green on real code, each has been seen to
fail on a deliberate violation, and every configuration file this lane owns is final
with a comment for each non-default setting.

## Non-goals

- No dependency, so `Cargo.lock` changes only if a manifest field does
  (`CONVENTIONS.md` §11).
- No effect bans in clippy or rustqual (T10). The crate bans in `deny.toml` stay the
  convenience layer `CONVENTIONS.md` §1 fact 3 describes.
- No edit to the `Justfile`. A recipe flag found wrong is handed back as a T00 follow-up.
- No new item in either crate beyond what a proof needs and then removes.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | final | the lint table and profiles, std-adjusted |
| `Cargo.lock` | generated | only if a manifest field moves |
| `rustfmt.toml` | final | L's, verified on the pin |
| `clippy.toml` | final | L's keys, verified on the pin |
| `taplo.toml` | final | L's excludes |
| `deny.toml` | final | licences, bans with `wrappers = ["pachinko"]`, sources |
| `.config/nextest.toml` | final | L's profiles |
| `crates/libpachinko/Cargo.toml` | final | package metadata the `cargo` lint group needs |
| `crates/libpachinko/src/lib.rs` | final | crate docs, `version()`, tests |
| `crates/pachinko/Cargo.toml` | final | package metadata, the path dependency |
| `crates/pachinko/src/lib.rs` | final | `run()` and its test |
| `crates/pachinko/tests/binary.rs` | final | the binary test |
| `tickets/T02-rust-gate.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **The lint table on the pin.** Run `just clippy` and read the clippy changelog between
   1.98 and the pin for lints added to the pedantic, cargo or restriction groups the
   table names. Each new lint is either kept (and satisfied) or set in the table with a
   comment. Confirm every `clippy.toml` key still exists under its name on the pin
   (`CONVENTIONS.md` §12 claim 3).
3. **The std adjustment.** Confirm `std_instead_of_core`, `std_instead_of_alloc` and
   `alloc_instead_of_core` are gone from the table, and that nothing else in it presumes
   `no_std`. `print_stdout` and `print_stderr` stay: `run()` takes a writer, and only
   `main.rs` touches `stdout`, which it does through `std::io::stdout()` rather than a
   print macro.
4. **Each recipe red, then green.** For each recipe in the Goal, make one deliberate
   violation in a scratch commit, run the recipe, quote the failure, and revert. For
   `coverage`, add an untested branch to `run()`; for `deny`, add a dependency with a
   disallowed licence in a scratch copy only; for `deps-unused`, a dependency nothing uses.
5. **Coverage on the binary** (`CONVENTIONS.md` §12 claim 4). `cargo llvm-cov report
   --show-missing-lines` reports nothing missing in either crate, `main.rs` included, or
   T00's fallback is in place and explained.
6. **The snapshot harness without insta** (`CONVENTIONS.md` §12 claim 10). `just
   snapshots-check` green on the tree; then add a stray `.snap` file in a scratch commit
   and confirm `--unreferenced reject` fails on it. Record that the first ticket to take
   a snapshot adds `insta` with the decision-0007 note.
7. **`deny.toml`.** L's `[licenses]`, `[sources]` and `[bans]`, with `wrappers` naming
   `pachinko` and the `[graph] targets` list without the wasm target. Run `just deny`
   offline after `just sync`.
8. `just check`, the final adversarial review, `just check` again if fixes landed, then
   commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff. Ask: which lint-table
   decision on the new pin is unjustified or has no comment; whether each recipe's quoted
   failure came from its deliberate violation and from nothing else; whether the evidence
   for `CONVENTIONS.md` §12 claims 3, 4 and 10 proves each claim.

## Acceptance criteria

- The ten recipes in the Goal are green, and the hand-back notes quote one failure of each.
- `cargo llvm-cov report --show-missing-lines` lists no missing line.
- Every non-default setting in the files this lane owns has a comment saying why.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just fmt-check && just toml-check && just clippy && just features
just test && just coverage && just doc && just deny && just deps-unused
just snapshots-check
cargo llvm-cov report --show-missing-lines
just check
```

## Hand-back notes

To be written by the executing agent: each lint the pin added and what was done about it;
each recipe's quoted failure; the outcome of `CONVENTIONS.md` §12 claims 3, 4 and 10; the
review record for each checkpoint (`CONVENTIONS.md` §14); any Justfile change handed
back.

## Open points

- `[profile.release]`: L's `lto = "fat"` and `codegen-units = 1` were for a library
  shipped to bindings. Keep them for the service binary, or drop them until T13 measures
  the image build. Recommend keeping; say so if the build time argues otherwise.
