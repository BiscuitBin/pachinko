---
id: T16
title: "Benchmarks and fuzzing: a divan bench, a fuzz crate on stable, a weekly fuzz job"
status: open
depends_on: [T11]
parallel_with: [T12, T13, T14, T15]
branch: ticket/t16-bench-and-fuzz
estimated_size: M
---

# T16: Benchmarks and fuzzing: a divan bench, a fuzz crate on stable, a weekly fuzz job

## Context

L's spike S03 drafted two build tickets, one for divan benchmarks and one for a
cargo-fuzz crate on stable with `--sanitizer none`: L
`tickets/S03-bench-fuzz-mutants.md` lines 688-752 and 753-911, with the evidence above
them from line 154 (stable fuzzing is argued from line 255). Both drafts waited on a
trigger (a solver, a parser). The maintainer moved both into the foundation
(`CONVENTIONS.md` §1 decision 8), so this ticket builds the harnesses now, each with one
smoke target over the foundation's one item, and the real targets come with the
implementation.

What differs from the drafts: the targets are smoke targets over `libpachinko::version()`
(a bench that times it; a fuzz target that feeds arbitrary bytes to a trivial function
written for the purpose and asserts it does not panic), not over a solver or a parser;
divan is a development dependency of libpachinko, the first dependency in the workspace
other than the path one, so it carries the decision-0007 note; decision 0017 is shared
with T15 (`CONVENTIONS.md` §8); and wave-2 rules apply (`CONVENTIONS.md` §11).

## Goal

`just bench` runs one divan benchmark by hand and is never in CI; `just fuzz` runs one
fuzz target on the pinned stable toolchain with `--sanitizer none` for a bounded time; a
weekly `fuzz.yml` does the same and uploads any crash; the fuzz crate sits outside the
workspace with its own committed lockfile; every existing recipe that must see it does;
and `just check` stays green.

## Non-goals

- No nightly toolchain, ever (the draft's open point 1).
- No benchmark in CI: timings measure the machine.
- No fuzz target over a real parser; none exists.
- No change to libpachinko's public API for the bench or the fuzz target beyond what a
  smoke target needs; if one needs a function, it is `#[doc(hidden)]` and says why, or
  lives in the fuzz crate.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `exclude = ["fuzz"]`, `[workspace.dependencies] divan` |
| `Cargo.lock` | shared, regenerated | divan's packages |
| `crates/libpachinko/Cargo.toml` | shared, own lines | divan dev-dependency, the `[[bench]]` table |
| `crates/libpachinko/benches/version.rs` | new | one divan bench |
| `fuzz/Cargo.toml` | new | the draft's, for libpachinko |
| `fuzz/Cargo.lock` | new | committed |
| `fuzz/.gitignore` | new | the draft's |
| `fuzz/fuzz_targets/smoke.rs` | new | one smoke target |
| `tools.txt` | shared, own lines | cargo-fuzz, the draft's pin re-verified |
| `Justfile` | shared, own lines | `bench`, `fuzz`, and the fuzz crate's lines in `sync`, `lock`, `fmt-check`, `clippy` |
| `.pre-commit-config.yaml` | shared, own lines | the `fmt-check` hook's `files:` widened to `fuzz/` |
| `.github/workflows/fuzz.yml` | new | weekly and by hand, never in `check`'s `needs` |
| `.gitignore` | shared, own lines | fuzz artefacts the draft names |
| `docs/reference/commands.md` | shared, own lines | `bench` and `fuzz` rows |
| `docs/reference/testing.md` | shared, own lines | "Benchmarks" and "Fuzzing" |
| `docs/decisions/0007-dependency-policy.md` | shared, own lines | divan as a development dependency |
| `docs/decisions/0017-mutation-bench-and-fuzz.md` | new or shared | the bench and fuzz section (`CONVENTIONS.md` §8) |
| `docs/manifest.yml` | shared, own lines | 0017's entry, if this ticket creates it |
| `docs/README.md` | shared, own lines | 0017's link, if this ticket creates it |
| `docs/decisions/README.md` | shared, own lines | 0017's row, if this ticket creates it |
| `CHANGELOG.md` | shared, own lines | one `[Unreleased]` line |
| `tickets/T16-bench-and-fuzz.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged; rebase on any wave-2 ticket merged since.
   If T15 has merged, 0017 exists: add this ticket's section; otherwise create it.
2. Re-verify the drafts' pins on the day: divan (the draft notes no release since
   2025-04-10; re-check maintenance and record it), cargo-fuzz (`0.13.2` in the draft)
   and libfuzzer-sys.
3. **The bench.** Apply the benchmark draft with libpachinko for `pawdoku` and
   `version.rs` for `solver.rs`. `just bench` runs it; quote the table.
4. **The fuzz crate.** Apply the fuzzing draft: `fuzz/` outside the workspace, its own
   `Cargo.lock`, the recipe lines in `sync`, `lock`, `fmt-check` and `clippy` that name
   `--manifest-path fuzz/Cargo.toml`, and the hook pattern. `just fuzz` runs
   `cargo fuzz run smoke --sanitizer none -- -max_total_time=<seconds>` on the pinned
   stable (`CONVENTIONS.md` §12 claim 8); quote it.
5. **`fuzz.yml`.** The draft's shape: weekly and `workflow_dispatch`, 300 seconds per
   target, crashes uploaded, never in `check`'s `needs`.
6. Confirm `just deps-unused`, `just deny`, `just features` and `just coverage` are still
   green with divan present (divan is dev-only, outside cargo-deny's graph per `deny.toml`'s
   `exclude-dev`).
7. `just check`, then **authorisation required: push and open a pull request**. After the
   merge, a `workflow_dispatch` of `fuzz.yml` (separately authorised).

## Acceptance criteria

- `just bench` and `just fuzz` outputs are quoted.
- `just fuzz` ran on the pinned stable; `rustup show active-toolchain` from `fuzz/` prints
  the pin.
- `fuzz.yml` is not in `check`'s `needs`.
- `just check` is green.

## Verification

```sh
just bench
just fuzz
(cd fuzz && rustup show active-toolchain)
just deps-unused && just deny && just coverage
just check
```

## Hand-back notes

To be written by the executing agent: the pins and the day, divan's maintenance status,
both outputs, the outcome of `CONVENTIONS.md` §12 claim 8, the dispatch run URL, and where
this ticket departed from L's drafts.

## Open points

- If the fuzz crate's recipe lines make `just check` slower than the maintainer accepts,
  `clippy` over `fuzz/` can move to `fuzz.yml`; say so with the timing.
