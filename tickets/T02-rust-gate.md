---
id: T02
title: "Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code"
status: done
depends_on: [T00]
parallel_with: [T01, T05, T06, T07]
branch: ticket/t02-rust-gate
estimated_size: M
---

# T02: Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code

## Context

T00 landed both crates and working copies of L's Rust tool configuration, with the deltas
`CONVENTIONS.md` §3 names, green on the day's stable. This lane makes that configuration
final and green on this workspace: a std library, a binary, no dependencies, one item
each.

L's T02 already proved these recipes fire on the configuration this repository copies
(L `tickets/T02-rust-gate.md`, its step 10 and hand-back notes), so this lane does not
repeat those proofs. It proves only what differs here: the lint table on the new pin
(step 2), coverage on the binary (step 4) and the snapshot harness with no `insta` and
a stray `.snap` (step 5). L `docs/decisions/0009-rust-quality-gate.md` says why each
setting is what it is. L `tickets/T29-snapshot-harness.md` is the snapshot harness: its
recipes are already in the `Justfile`, and this lane proves them on a workspace that has
no snapshot yet.

## Goal

`fmt-check`, `toml-check`, `clippy`, `features`, `test`, `coverage`, `doc`, `deny`,
`deps-unused` and `snapshots-check` are each green on real code, and every
configuration file this lane owns is final with a comment for each non-default setting.

## Non-goals

- No dependency, so `Cargo.lock` changes only if a manifest field does
  (`CONVENTIONS.md` §11).
- No effect bans in clippy or rustqual (T10), and no effect-crate bans in `deny.toml`:
  `wrappers` cannot scope a ban to libpachinko (`CONVENTIONS.md` §1 fact 3).
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
| `deny.toml` | final | licences, L's `openssl-sys` ban and no effect-crate ban, sources |
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
4. **Coverage on the binary** (`CONVENTIONS.md` §12 claim 4). `cargo llvm-cov report
   --show-missing-lines` reports nothing missing in either crate, `main.rs` included, or
   T00's fallback is in place and explained.
5. **The snapshot harness without insta** (`CONVENTIONS.md` §12 claim 10). `just
   snapshots-check` green on the tree; then add a stray `.snap` file in a scratch commit
   and confirm `--unreferenced reject` fails on it. Record that the first ticket to take
   a snapshot adds `insta` with the decision-0007 note.
6. **`deny.toml`.** L's `[licenses]`, `[sources]` and `[bans]`, without the `getrandom`
   and `rand` bans (`CONVENTIONS.md` §1 fact 3) and with L's `openssl-sys` ban kept; the
   `exclude-dev` comment, which cites the dropped ban, reworded; the `[graph] targets`
   list without the wasm target. After `just sync`, run `just deny` under
   `CONVENTIONS.md` §11's network denial, and quote it with both negative-control exit
   codes.
7. `just check`, the final adversarial review, `just check` again if fixes landed, then
   commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff. Ask: which lint-table
   decision on the new pin is unjustified or has no comment; whether the evidence for
   `CONVENTIONS.md` §12 claims 3, 4 and 10 proves each claim.

## Acceptance criteria

- The ten recipes in the Goal are green.
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

Evidence below ran at `052f9b9` (the last config commit) unless a quote names another
commit. The commit that adds these notes changes only this file. Run on 2026-10-22
on the maintainer's machine (Apple silicon), toolchain `1.99.1-aarch64-apple-darwin`.

### Baseline

At `8b01c0c`, before any edit, `just check` exited 0 and ended:

```text
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

### Lints the pin added (step 2)

Lint names and group members were diffed between `rustup run 1.98.1 clippy-driver -W
help` and `rustup run 1.99.1 clippy-driver -W help`. 1.99 adds eight lints and removes
or renames none. The `cargo` group's members did not change. Every lint the table
names still exists on 1.99.1. One existing lint changed level:
`nonstandard_macro_braces` moved from nursery (allow) to style (warn), so `-D warnings`
now enforces it. It is kept and satisfied (no site), with no table entry; it is the
only lint whose group or default level differs between the two dumps.

| Lint | Group on 1.99.1 | Disposition |
| --- | --- | --- |
| `assert_is_empty` | pedantic | Kept through the group, satisfied: T00 rewrote the one site in `crates/pachinko/tests/binary.rs`. |
| `manual_bit_width` | pedantic | Kept through the group, satisfied: no site. |
| `definition_in_module_root` | restriction | Not enabled: `mod_module_files` already forbids `mod.rs`, so it has nothing to police. Said in the comment above `mod_module_files`. |
| `rest_pattern_accessible_field` | restriction | Not enabled: no invariant asks for it, and the restriction group is never enabled whole. |
| `unnecessary_rest_pattern` | restriction | Not enabled, for the same reason. |
| `block_scrutinee` | suspicious | On by default, satisfied. |
| `mismatched_bit_width_type` | suspicious | On by default, satisfied. |
| `nonnull_unchecked_on_box_ptr` | complexity | On by default, satisfied. |

Table change: `module_name_repetitions = "allow"` and its comment are removed. Both
dumps place the lint in `clippy::restriction`, at level `allow`, on 1.98.1 and 1.99.1
alike. The table enables restriction lints one by one and never takes the group, so
the line set an already-allowed lint to `allow` and justified nothing. A green `just
clippy` cannot show this, since neither crate has a module, so the group membership
from the dumps is the evidence. The same dumps confirm the groups the new comments
name: `missing_const_for_fn` and `use_self` are nursery; `cognitive_complexity`,
`iter_over_hash_type`, `str_to_string`, `lossy_float_literal`, `float_cmp_const`,
`same_name_method` and the panicking, printing, `exhaustive_*`, `allow_attributes*`
and layout lints are restriction; `multiple_crate_versions` is cargo.

Std adjustment (step 3): `std_instead_of_core`, `std_instead_of_alloc` and
`alloc_instead_of_core` were already gone, and nothing else in the table presumes
`no_std`. `print_stdout` and `print_stderr` stay, and their comment says why.

### §12 claims this ticket checked

| # | Outcome | Evidence |
| --- | --- | --- |
| 3 | Holds. | `just clippy`, exit 0. Every `clippy.toml` key is in the valid-key list clippy 1.99.1 prints. Positive control: with `allow-unwrap-in-tests` mistyped as `allow-unwrap-in-testz` and `crates/libpachinko/src/lib.rs` touched, `just clippy` exited 101 with `error: unknown field name` at `clippy.toml:6:1`; reverted, `just clippy` exited 0. |
| 4 | Holds. No fallback needed. | `just coverage` exit 0; `cargo llvm-cov report --show-missing-lines` exit 0, `pachinko/src/main.rs` 3 lines 0 missed 100.00%, `TOTAL` 17 lines 0 missed 100.00%, no missing-lines section. |
| 10 | Holds, and the gate fires. | `just snapshots-check` on the tree: exit 0, `info: no unreferenced snapshots found`. Scratch commit `d1e6850` on `052f9b9` added `crates/libpachinko/src/snapshots/stray.snap` (a well-formed snapshot): exit 1, `warning: encountered unreferenced snapshots:` naming the file, then `error: aborting because of unreferenced snapshots`. Reset to `052f9b9`; `git status --short` empty. |

The first ticket to take a snapshot adds `insta` as a dev-dependency with the
decision-0007 note.

### The ten recipes (goal)

Each run alone, at `052f9b9`:

```text
just fmt-check exit=0
just toml-check exit=0
just clippy exit=0
just features exit=0
just test exit=0
just coverage exit=0
just doc exit=0
just deny exit=0
just deps-unused exit=0
just snapshots-check exit=0
```

`.config/nextest.toml` is in taplo's file set: with `fail-fast   =    true` written
into it, `taplo fmt --check` exited 1 naming the file; reverted, it exited 0.
`nextest-version` equals the `cargo-nextest = "==0.9.146"` pin in `pyproject.toml`.

### Offline `just deny` (step 6)

`just sync` (network, authorised) exited 0. Then, under
`sandbox-exec -p '(version 1)(allow default)(deny network*)'`:

```text
just deny                                                  exit=0
bans ok, licenses ok, sources ok
curl -fsS --max-time 10 https://static.rust-lang.org       exit=6  Could not resolve host
curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1      exit=7  Couldn't connect to server
```

`just deny` warns about an `unmatched license allowance` for licences no dependency
uses yet. That is expected on a tree with no dependencies, and the list is not trimmed.

### Findings recorded for later lanes

- `cargo_common_metadata` (cargo group): in a scratch copy of the workspace with
  `description`, `keywords` and `categories` removed from
  `crates/libpachinko/Cargo.toml`, a clean `cargo clippy -- -D warnings` exits 0 under
  `publish = false` and fails with three errors (`package libpachinko is missing
  package.categories metadata` and its siblings) under `publish = true`. So the lint
  skips unpublished packages, and the fields serve it only once publishing is turned on.
  The crate manifests' comments say this. An earlier probe in this session reported
  that the lint never fired; its BSD `sed` pattern used `\|`, which matched nothing, so
  no field had been removed. The code review caught it.
- cargo-deny's `wildcards = "deny"` rejects
  `libpachinko = { path = "../libpachinko" }` with no version (`bans FAILED`,
  `wildcard dependency`). The manifest comment gives that as the reason for the version.
- An inner `#![allow(lint, reason = "...")]` passes both `allow_attributes` and
  `allow_attributes_without_reason` (review finding BR-01, confirmed by probe). For that
  form, invariant 6 is enforced by review, not by the gate. Closing the gap would change
  a setting, which is outside this lane; see Open points.

### Review record, checkpoint 1 (step 7)

Harness and model: Codex, `codex exec -m gpt-6-astra -s read-only` (flags confirmed with
`codex exec --help`), conformance and breakage lenses over `git diff main...HEAD` at
`bfab7aa`. No model fallback. A fresh-context Claude subagent did the triage and wrote
`ai_tmp/review/T02-1/verdicts.md`, confirming each finding with a probe crate on clippy
1.99.1 that used this repository's `clippy.toml`. Both the executing agent and the
triage subagent were Claude Code on `claude-opus-5-5`; the triage subagent applied
the fixes, as §14 Phase 3 asks.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C01, BR-02 | low | valid | The `-in-tests` keys cover `unwrap_used`, `expect_used`, `panic`, `dbg_macro`, the print lints and `indexing_slicing`, and apply to `#[test]` functions too; `unreachable`, `todo` and `unimplemented` have no test exemption. | Comments corrected in `Cargo.toml` and `clippy.toml`. |
| BR-01 | medium | valid | An inner `#![allow(..., reason = "...")]` passes both attribute lints. | The comment now says review covers that form; gate change left as an Open point. |
| C02, BR-03 | low | valid | `yanked = "deny"` is stricter than cargo-deny 0.20.2's default `warn` and had no reason. | Reason added in `deny.toml`. |
| C03 | medium | valid | The hand-back notes were still unwritten. | These notes. |

No high-severity finding was valid, so there was no second round. Both reviewers
confirmed that the removal of `module_name_repetitions` is the only setting change and
that no frozen or other-lane file was touched. Fixes were committed as `67468a4`.

### Review record, the `code-review` skill (plan step 8)

Claude Code's `code-review` skill, at `xhigh` effort and on `claude-opus-5-5`, over
`main...HEAD` at `cee88bb`. It ran three scratch probes of its own and reported
fifteen findings, all about comments or this record. I checked each against the tools
and fixed the valid ones in `052f9b9`:

| Finding | Verdict | Fix |
| --- | --- | --- |
| `exclude-dev` also hides dev-dependencies from `[sources]` and `just audit` | valid | `deny.toml` comments say so, and say to review a dev-dependency's source. |
| `cargo_common_metadata` does fire under `publish = true` | valid | Reprobed (above); the notes are corrected. |
| `.config/nextest.toml` says CI selects `--profile ci`, but nothing does | valid | The comment says it is for T01's workflows and that no recipe selects it yet. |
| The `targets` comment called Windows a build host | valid | Reworded: Windows is kept from L so a Windows-only dependency is still checked. |
| "Restriction lints from here down" ran over the cargo-group `allow`; the header left out the `allow` | valid | Both reworded. |
| `just check` was not quoted at the final commit | valid | Quoted under Final gate. |
| `nonstandard_macro_braces` changed level and was not recorded | valid | Recorded above. |
| The rustdoc `deny` comment said a local build fails | valid | Rustdoc lints run only under `cargo doc`; the comment says so. |
| The root comment on the cargo group contradicted the crate manifests | valid | It now points to each manifest's comment. |
| `trivial_casts` was described as `trivial_numeric_casts` | valid | Reworded. |
| Removing `module_name_repetitions` departs from `CONVENTIONS.md` §3, which describes the line as kept and reworded | valid as a record gap | Kept removed, because it is a no-op; handed back below. |
| The triage model was not named | valid | Named above. |
| `confidence-threshold` and the rustfmt shorthands said what, not why | valid | Reasons added. |
| `[profile.test] opt-level = 1` was justified by tests that do not exist | partly | Kept, as L has it; the comment now says the build cost is negligible today. |
| The taplo comment overstated the sort rule | valid | It now names the uncovered `target` tables and says "rarely conflict". |

### Hand-backs to `CONVENTIONS.md` (for T11 to reconcile)

- §3, the root `Cargo.toml`: "the `module_name_repetitions` comment reworded" describes
  a line T02 removed, since the lint is restriction and the table never enables it.
- §3, `crates/libpachinko/Cargo.toml`: "the `cargo` lint group requires one, plus
  `keywords`, `categories` and `readme`" holds only once `publish` is turned on (see
  Findings).

### Final gate

On `052f9b9` with these notes in place, after both review rounds, `just check` exited 0
and ended `All checks passed and the worktree is unchanged.` It was rerun after the
commit that adds these notes, and `git status --short` printed nothing.

### Justfile hand-backs

None. Every recipe fired as written.

### `[profile.release]` (former Open point)

Kept: `lto = "fat"` and `codegen-units = 1`, with a comment saying they are for the
service binary and are revisited when T13 measures the container image build. The
binary is a few lines today, so build time gives no reason to drop them.

## Open points

- Invariant 6 and inner attributes: an inner `#![allow(lint, reason = "...")]` passes
  the gate. One possible closure, enabling a lint or check that rejects every `allow`
  form, belongs to whichever lane next owns the lint table or the agent contract (T05's
  `rust-change` rule could say it in the meantime).

  Closed on 2026-10-24 on branch `review-and-finish-bootstrap`. The `rust-change` skill
  now says an inner `#![allow(lint, reason = "...")]` passes both attribute lints, so
  review rejects it (commit `ae29219`). No gate rejects that form, by the maintainer's
  choice, which decision 0009 and `docs/reference/configuration.md` record (commit
  `e00bc3b`).
- `[profile.release]`: kept by the maintainer's choice on 2026-10-24, after T13's image
  build compiled the release binary in 6.24 s uncached for a 9.4 MB image. The
  `Cargo.toml` comment no longer waits for T13 (commit `b0c931f`).
- The nextest `ci` profile no recipe or workflow selects: still open, the maintainer's
  decision. It closes when CI selects the profile or the profile is deleted. Renovate now
  moves `nextest-version` with the pixi `cargo-nextest` pin in one branch (T17).
- The first snapshot and `insta`: still open, as T18 item 2. It closes with the first
  ticket that takes a snapshot.
