---
id: T16
title: "Benchmarks and fuzzing: a divan bench, a fuzz crate on stable, a weekly fuzz job"
status: done
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
7. `just check`, the final adversarial review, `just check` again if fixes landed, then
   **authorisation required: push and open a pull request**. After the merge, a
   `workflow_dispatch` of `fuzz.yml` (separately authorised).

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff, before the push. Ask:
   whether the fuzz crate stays outside the workspace; whether the evidence for
   `CONVENTIONS.md` §12 claim 8 shows the pinned stable; whether step 6's gates are green
   with divan present, and quoted.

## Acceptance criteria

- `just bench` and `just fuzz` outputs are quoted.
- `just fuzz` ran on the pinned stable; `rustup show active-toolchain` from `fuzz/` prints
  the pin.
- `fuzz.yml` is not in `check`'s `needs`.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just bench
just fuzz
(cd fuzz && rustup show active-toolchain)
just deps-unused && just deny && just coverage
just check
```

## Hand-back notes

Implemented on 2026-10-23 from `2c93ae3` (T11 and T14 merged). Kept the prepared
`ticket/T16-bench-and-fuzz` branch under the implement instruction to commit on the
current branch, rather than creating a branch differing only in case from the
frontmatter. The maintainer initialized the missing worktree environment. Commands use
rustup's cargo proxy first on PATH. No domain behaviour or public API changed.

### Baseline — `2c93ae3`, untouched

The first attempt lacked tooling; after initialization, the sandbox denied the user
hook-cache log. Re-running with access to the installed caches produced:

```text
$ just check (exit 0)
All checks passed and the worktree is unchanged.
```

### Pins — verified 2026-10-23

Public crates.io API metadata confirmed divan `0.1.21` (released 2025-04-10, MSRV
1.80.0), cargo-fuzz `0.13.2` (2026-06-09), and libfuzzer-sys `0.4.13` (2026-06-04).
GitHub metadata confirmed divan is not archived, with 51 open issues and its latest
commit `55ec68e31526c28c7825fa1bb884f326b619a879` on 2026-07-19, the documentation
change identified by L. Maintenance remains thin; the harness stays small and replaceable.
Sources: `https://crates.io/api/v1/crates/divan`, the corresponding `cargo-fuzz` and
`libfuzzer-sys` endpoints, `https://api.github.com/repos/nvzqz/divan`, its commits
endpoint, and `https://api.github.com/repos/rust-fuzz/cargo-fuzz/releases/latest`.
The cargo-fuzz release still has only an x86 macOS archive, so Apple silicon needs
Rosetta; the recipe explicitly selects rustc's native host triple.

### Verification — `2c93ae3` with all uncommitted T16 implementation and review fixes

The table measures the smoke call plus indirect-call overhead; it is not a meaningful
application performance baseline. Benchmarks remain manual and outside CI.

```text
$ just bench (exit 0)
Timer precision: 41 ns
version     fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ version  0.441 ns      │ 7.119 ns      │ 0.446 ns      │ 0.524 ns      │ 100     │ 819200
```

Claim 8 holds on the pin. The run below used `sandbox-exec` with `(deny network*)`;
both required negative controls failed inside that same denial before fuzzing ran.
The local smoke verification used 20 seconds; the recipe default and workflow use 300.
Compilation is additional time.

```text
$ (cd fuzz && rustup show active-toolchain) (exit 0)
1.99.1-aarch64-apple-darwin (overridden by '/Users/scutting/.supacode/repos/pachinko/ticket/T16-bench-and-fuzz/rust-toolchain.toml')
$ curl -fsS --max-time 10 https://static.rust-lang.org (exit 6)
$ curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1 (exit 7)
$ just fuzz smoke 20 (exit 0, pinned stable, --sanitizer none)
#48884773 DONE   cov: 9 ft: 9 corp: 2/2b lim: 4096 exec/s: 2327846 rss: 25Mb
Done 48884773 runs in 21 second(s)
```

All four step-6 gates were rerun after the final fixes. Dependency-policy warnings
about unmatched license allowances remain the existing harmless warnings; divan is
outside cargo-deny's graph, as `exclude-dev` prescribes.

```text
$ just deps-unused (exit 0)
  ✓ no issues found
$ just deny (exit 0)
bans ok, licenses ok, sources ok
$ just features (exit 0)
info: running `cargo check --locked` on libpachinko (1/2)
info: running `cargo check --locked` on pachinko (2/2)
$ just coverage (exit 0)
```

The coverage report records 100% of 17 lines, 5 functions and 25 regions, with none
missed. `just test` also exited 0: all 3 unit/integration tests and the library doctest
passed. The full gate includes formatting, both manifests' Clippy checks, the effects
probe, documentation, locks, specifications and snapshots:

```text
$ just check (exit 0)
All checks passed and the worktree is unchanged.
```

### Adversarial checkpoint — `2c93ae3` with uncommitted T16 changes

Two rounds ran, using Claude Code `claude-opus-5-5` with independent conformance and
breakage lenses. Automatic approval initially rejected the external payload; the user
then authorized sending diffs to Claude. Diff-only passes disabled file tools. The first
breakage pass also inspected local evidence. Preliminary independent Codex reviewers
and a separate Codex triage agent used `gpt-6-astra`. Triage checked every external
finding against source and applied valid in-scope fixes sequentially. The requested
`code-review` skill was unavailable; repository `review-change` supplied the final
invariant review instead. Raw reviews, verdicts and probe output remain ignored in
`ai_tmp/review/T16-1/`.

- Initial Codex B1 / external EC4: valid; copied the complete workspace Clippy policy
  into the excluded crate. A temporary `todo!()` then made `just clippy` exit 101 for
  `clippy::todo`. Pinned Clippy required `assert_ne!` instead of the original emptiness
  assertion; restoring the old assertion in a probe reproduced `assert_is_empty`, exit
  101. Both probes restored the target immediately.
- EC1 / EB1, high: valid release integration gap. Removed the unpublished path
  dependency's unnecessary version constraint and documented the exception in 0007.
  A library version bump still requires refreshing `fuzz/Cargo.lock`; the owning
  release-runbook follow-up is handed back below under the scope guard. This high
  finding triggered the second review round.
- EB7: valid; black-boxed the function pointer as well as its return to retain runtime
  invocation, and documented its indirect-call overhead. EC8–10 and second-round prose
  observations: valid small clarity issues, fixed. EB3: valid stale-evidence concern,
  resolved by rerunning every quoted command after all implementation fixes.
- C1 / EB4–5: valid owning-page omissions, handed back below. The Justfile's in-scope
  lock-count comment and commands reference were corrected.
- EC2–3: invalid after reading `set positional-arguments` and setup's `install-tools`
  and `sync`. EC5 / EB2: audit expansion is outside the existing intentional shipped
  dependency graph. EC6: no immutable-decision rule prohibits the requested 0007
  update. EC7 and EC11–13: no failing style gate, harmless defensive duplication,
  matching current edition/MSRV, and the expressly shared 0017 record respectively.
- EB6: optional cache optimization, not a correctness defect. EB8: prepared branch
  mismatch recorded above under the current-branch instruction. Second-round requests
  to verify copied lint levels and `publish = false` were checked and satisfied;
  changing the assertion back would reproduce the measured Clippy failure. Future
  lint-table drift is a documented maintenance risk, not a current mismatch.

Before the final rerun, temporary probes also proved a panic makes `just fuzz smoke 5`
exit 1 (target status 77) and writes
`fuzz/artifacts/smoke/crash-da39a3ee5e6b4b0d3255bfef95601890afd80709`, and an unsafe
block makes `just clippy` exit 101 for the unsafe-code forbid. The original target was
restored in both cases. The final successful fuzz run quoted above follows the probes
and all fixes. No generated crash file is committed.

### Departures and hand-back

The harnesses exercise the foundation, not a solver or parser. Fuzz byte handling lives
inside the fuzz crate. The recipe defaults to `smoke`, validates all three locks and
then runs offline instead of fetching before each run. The fuzz manifest repeats the
Clippy policy in addition to L's forbid levels. The bench retains an opaque runtime
call. All matching format/fix and lock-upgrade recipes also include fuzz. A cold native
fuzz Clippy build took 7.87 seconds locally; its warm check was about 0.06 seconds, so
it remains in the gate. The fuzz C++ build for instrumentation is separate.

Scope-guard follow-ups, with no edits to the excluded files:

- **Before a library version bump:** update `docs/operations/maintenance.md` release
  step 3 to run `just lock` and commit `fuzz/Cargo.lock` after cargo-release and before
  the final gate. Otherwise the excluded lock is stale and the locked gate fails.
- Add C++ and Rosetta prerequisites to `docs/how-to/develop-locally.md`; correct
  lock counts and harness references in `docs/how-to/maintain-dependencies.md`,
  `docs/reference/quality-gates.md`, `docs/reference/configuration.md`,
  `docs/how-to/test-and-debug.md`, `docs/tutorials/first-change.md`,
  `docs/operations/maintenance.md` and decision 0016. Marking `fuzz/Cargo.lock` generated
  in `.gitattributes` and caching `fuzz/target` are optional follow-ups. The in-scope
  testing page already documents the new prerequisites and lint maintenance obligation.

The branch was later pushed and merged as PR #16 (`5748060`); the post-merge dispatch
evidence is under Open points. `fuzz.yml` is independent of `check`'s needs; its upload
path matches the locally proven crash directory.

## Open points

- If the fuzz crate's recipe lines make `just check` slower than the maintainer accepts,
  `clippy` over `fuzz/` can move to `fuzz.yml`; say so with the timing.

  Closed on 2026-10-24: `clippy` over `fuzz/` stays in `just check` by the maintainer's
  choice, at 7.87 s cold and about 0.06 s warm. Decision 0017 and
  `docs/reference/quality-gates.md` record it (branch `review-and-finish-bootstrap`,
  commit `e00bc3b`).
- The post-merge dispatch: on 2026-10-24 (2026-10-25 UTC) a `workflow_dispatch` of
  `fuzz.yml` on `main` ran as
  `https://github.com/BiscuitBin/pachinko/actions/runs/40150109570`, head `f080d62`:
  conclusion `success`, job `fuzz (smoke)` `success`, no artifact. The crash upload runs
  only `if: failure()`, so CI has not exercised it; a failing run would.
- The scope-guard follow-ups: closed on 2026-10-24 on the same branch (commit
  `e00bc3b`). Maintenance's release step refreshes `fuzz/Cargo.lock` after a library
  bump, `docs/how-to/develop-locally.md` lists the C++ and Rosetta prerequisites, and
  the lock counts and harness references name the third lockfile.
- `fuzz/Cargo.lock` as generated: already true. `.gitattributes`'s `Cargo.lock` pattern
  has no slash, so it matches at any depth (`git check-attr` reports
  `linguist-generated: true` for `fuzz/Cargo.lock`).
- Caching `fuzz/target` in `fuzz.yml`: declined as optional, an optimization rather than
  a defect.
- Still open, gated on an event: the bench and the fuzz target exercise
  `libpachinko::version()` only. Real targets come with the domain implementation the
  specifications define.
