---
id: T18
title: "Verification gaps: the checks wave 0 and wave 1 never ran, run or recorded as open"
status: done
depends_on: [T11]
parallel_with: [T12, T13, T14, T15, T16, T17]
branch: ticket/t18-verification-gaps
estimated_size: M
---

# T18: Verification gaps: the checks wave 0 and wave 1 never ran, run or recorded as open

## Context

T11 listed every check a T00 to T10 hand-back said it did not run, skipped, or ran only
as a dry run, a documentation read or a scratch copy, and closed the ones its fresh
clone, its network-denied gate and its CI dispatch actually executed. The rows below are
the ones that stayed open and that no other ticket owns: T11's hand-back notes, table
"Checks the hand-backs did not run", gives each row's source. Each item names the
evidence that closes it. A green `just check` or CI run closes an item only if it
executes that item; a gate that does not run it is not evidence, and an item that cannot
be closed is recorded as open, not inferred from an unrelated green run.

This ticket is off the critical path. It adds no feature, recipe, page or dependency:
an item whose evidence needs one is handed back instead (`CONVENTIONS.md` §11).

## Goal

Every item below is either Closed, with its evidence quoted in the hand-back notes and
headed by the commit it ran at, or recorded as Open with the reason it cannot be closed
yet and the ticket or event that will close it.

## Non-goals

- No change to a gate's behaviour. An item that shows a gate cannot fire is a finding
  handed back to the maintainer, not a fix made here.
- No wave-2 work.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/T18-verification-gaps.md` | ticket | `status:` and hand-back notes |

Scratch copies, scratch branches and logs live under `ai_tmp/` and are never committed.

## Steps

1. Create the worktree from `main` after T11 has merged, run `just initialize` (network,
   authorised) and quote the baseline `just check` last line.
2. Work through the items in order. Each runs in a scratch copy or on a scratch branch
   that is deleted afterwards, and `git status --short` is quoted empty after each.
   Items 1 to 13:

   1. **The primary checkout's hook shim.** In the primary checkout on an up-to-date
      `main`, `just initialize`, then `ls -l .git/hooks/pre-commit`; on a scratch branch
      there, commit a malformed `docs/specs/pachinko.allium` and quote the
      `Specification diagnostics ... Failed` line and the commit's non-zero exit; delete
      the branch. T11 showed the shim refusing such a commit in a fresh clone only.
      **Authorisation required: the primary checkout is outside this worktree.**
   2. **The snapshot recipes.** `snapshots-accept` writing a `.snap` file,
      `snapshots-check` failing on a changed snapshot, and `snapshots-review` listing the
      pending one. Nothing can act on these before the first snapshot exists, so this item
      stays Open until the first ticket that takes a snapshot, which quotes all three.
   3. **Initialization with no cache reused.** `just initialize` then `just check` with
      empty rustup, Cargo, pixi and prek caches (a fresh user account, a VM or a container
      with `HOME` pointed at an empty directory), quoting both outputs and which caches
      were empty. T11's fresh clone had fresh `.pixi/` and `.tools/` but reused the host's
      toolchain and caches.
   4. **The network recipes no lane ran.** `just lock` on `main` (then
      `git status --short` empty), `just lock-upgrade` on a scratch branch (quote the diff
      it makes and reset), and `just check-links-online`, each with its exit code
      (**network, authorised**). `just audit` is closed by CI's Audit run on `main`; its
      weekly schedule is not, so quote the first scheduled Audit run on `main` (Mondays,
      06:00 UTC) by its URL and conclusion.
   5. **An optional banned dependency.** In a scratch copy, add `rand` to libpachinko as
      an optional path dependency behind a feature; quote `just effects` failing and
      naming `rand`. The crate line passes `--all-features`, but no lane exercised the
      optional case; the fixture probes a target-gated and a transitive dependency only.
   6. **The per-ban red runs at `main`'s tip.** Re-run T10's red harness, every ban
      both ways, in a scratch copy reverted after each run. For each of the eight crate
      bans, the crate added to libpachinko makes `just effects` fail naming it, and the
      crate removed from `tests/fixtures/effects-violation/` makes it fail with the crate
      probe not seeing it. For each `effect:` entry in `crates/libpachinko/clippy.toml`,
      the break moved into libpachinko fails naming the ban, the fixture's break silenced
      with its `// ban:` tag kept fails naming it as silent, and the break deleted with
      its tag fails the inventory check naming it as not tagged. Each run quotes its exit
      and that diagnostic; a run that fails for another reason does not count. T10 ran
      it at `a2f88a1`, before the last bans were added.
   7. **Network denial on Linux.** `unshare -rn just check` on Linux, in a namespace
      holding only `lo`, with both `CONVENTIONS.md` §11 curl probes failing; every lane
      showed the denial on macOS only.
   8. **The tutorial on `main`.** Replay `docs/tutorials/first-change.md` end to end on
      `main`, quoting each failure it predicts and the final green `just check`. T07
      replayed it before T02, T06 and T10 merged.
   9. **A negative control for each gate no lane made fire.** In a scratch copy, one
      breaking change per recipe that leaves the build and the recipe's earlier steps
      green, quoting the recipe's non-zero exit with the line in which the gate itself
      rejects the break, then the recipe passing again once the break is reverted:
      `fmt-check`, `features`, the `coverage` floor (its `--fail-under-lines` step, not
      a compile or test failure), `doc`, `deps-unused`, and `deny`'s `licenses`,
      `sources` and `openssl-sys` ban, each naming the licence, source or crate it
      refuses. A failure before the gate runs is not evidence.
   10. **Claims carried from L without a local run.** `cargo search allium-cli` shows
       3.6.1 (decision 0005); rustqual 1.8.3's line counting, its handling of an
       `#[expect]` against the suppression budget, and its having no release binary for
       any version, which puts it in `tools-source.txt` (decision 0012); Cargo accepts a
       dev-dependency cycle and refuses a normal one (`docs/explanation/layering.md`).
       Quote each, or record the sentence as unverified for the maintainer.
   11. **`exclude-dev` and dev-dependencies.** `deny.toml`'s `[graph]` comment says
       dev-dependencies skip `[sources]` and `just audit`. Show both halves in a scratch
       copy, each with a paired control: a git dependency fails `just deny` as a normal
       dependency and passes as a dev-dependency, and a crate release with a RustSec
       advisory fails `just audit` as a normal dependency and passes as a
       dev-dependency (**network, authorised**). Quote all four runs with their exit
       codes.
   12. **The std-API inventory against std.** A mechanical comparison of the `effect:`
       entries in `crates/libpachinko/clippy.toml` with the pinned toolchain's stable
       items under `std::fs`, `std::env`, `std::net` and `std::process` (rustdoc JSON or
       an equivalent listing), quoting the difference, which must be empty or explained
       (constants, which clippy cannot ban; windows-only paths).
   13. **The coverage debugging commands.** After `just coverage`, run
       `cargo llvm-cov report --html` and quote `target/llvm-cov/html/index.html` existing,
       then `cargo llvm-cov report --show-missing-lines` and its output, then from cold
       `INSTA_UPDATE=no cargo llvm-cov nextest --workspace --all-features --locked --html`,
       each with its exit code. `docs/how-to/test-and-debug.md` documents all three; T07
       checked the flags with `cargo-llvm-cov llvm-cov report --help` only.

   **Adversarial review** (`CONVENTIONS.md` §14): the hand-back notes before the final
   commit. Ask: which Closed item cites evidence that does not execute it; which quoted
   output lacks its commit or exit code; which item was dropped.
3. Set `status: done` when every item is Closed or Open with its reason, and commit.

## Acceptance criteria

- Every item in step 2 is Closed with quoted evidence headed by its commit, or Open with
  the reason and what will close it.
- No Closed item cites a gate or run that does not execute it.
- `just check` is green and `git status --short` is empty.

## Verification

```sh
just check
git status --short
```

## Hand-back notes

Executed on 2026-10-23. Only this ticket is a deliverable. The source under test was
`main` at `c1b4bcb`; the ticket branch is `ticket/t18-verification-gaps` (the
Supacode-created uppercase branch name was normalized before committing). No product
behaviour, gate, recipe, handbook page or dependency changed.

The table is the disposition of all thirteen items. An Open row is intentionally not a
claim of verification. The quoted excerpts below are the durable evidence; raw logs
remain uncommitted. Absolute paths in command output identify the original execution
locations, not files a later reader needs. Parenthesized empty-output annotations are
editorial, not command output.

| Item | Disposition | Evidence or reason; closing event for Open items |
| --- | --- | --- |
| 1. Primary hook shim | Open | This session has no authorization to initialize or make a trial commit in the primary checkout. A maintainer-authorized run of item 1 on primary `main`, including the rejected commit and branch cleanup, closes it. The secondary worktree's initialization skips hook installation. |
| 2. Snapshot recipes | Open | There is still no snapshot or `insta` dependency. The first ticket that takes a snapshot must quote accept writing it, check rejecting a change, and review listing it. No fixture or dependency was added here. |
| 3. Empty caches | Open | Initialization and the complete gate pass with the named cache directories isolated, but the existing home directory remains visible and Cargo reuses the host's whole `bin` directory, including `cargo-binstall`. This does not prove that no cache was reused. A maintainer's run in a fresh user account, VM or container with independently verified empty tool caches, quoting initialization, the full gate and its tested commit, closes it. |
| 4. Network recipes and scheduled Audit | Open | `lock`, `lock-upgrade` and online links exit 0; the lockfile changes were restored. The successful GitHub API read reports zero scheduled Audit runs on `main`. The first successful Monday 06:00 UTC scheduled Audit run on `main`, quoted by URL, tested commit and conclusion, closes the remaining part. |
| 5. Optional banned crate | Closed | The optional path dependency `rand`, behind `probe-rand`, is named by the effects rejection. |
| 6. Every ban, both ways | Closed | At the T11 merge: all 373 required negative cases match their own exact diagnostics, covering eight crate bans and 121 API entries through 119 fixture functions. The restored effects control passes. |
| 7. Linux network denial | Open | Host is macOS. The cached Linux container is ARM64, whereas the pinned pixi workspace supports `linux-64`; its unprivileged `unshare -rn` also fails to launch. This is not denial evidence. An authorized x86-64 Linux environment that can create the namespace must run the full gate with only `lo` and both failing curl controls under §11. |
| 8. Tutorial on main | Closed | Own main-based scratch clone initialized; baseline, docs, both predicted failures, corrected suite, full gate and restoration all executed. |
| 9. Gate negative controls | Closed | Format, feature powerset, coverage floor, rustdoc, unused dependency, and deny license/source/openssl controls each reach the intended rejection and pass again after restoration. |
| 10. Claims carried from L | Closed | crates.io search, rustqual counting and suppression controls, all upstream release asset records, and Cargo cycle pair executed. Third-party QuickInstall archives are distinguished from upstream release assets. |
| 11. Development dependency exclusion | Closed | The git-source and vulnerable-release pairs each fail as normal dependencies and pass as development dependencies. |
| 12. std inventory | Closed | Mechanical comparison against the exact 1.99.1 standard-library documentation: 82 stable top-level exports across the four modules; only the documented constants module is unmatched. |
| 13. Coverage debugging | Closed | HTML index exists, missing-lines report runs, and a new empty target directory builds and produces HTML. Every command exits 0. |

### Setup and untouched baseline — commit `c1b4bcb`

The first `just check` could not start (`bg-project-check` missing, exit 127). The first
initialization hit the sandbox's cache permission (exit 1); after elevation it reached
the toolchain check but found pixi's cargo first (exit 2). No gate was weakened. With
rustup's proxy first, the ticket-authorized initialization succeeded. Later gate runs
use this same `PATH`. These are prerequisite failures, not a red source baseline.

```text
$ PATH=/Users/scutting/.cargo/bin:$PATH just initialize
Secondary worktree: skipping install-hooks.
Run just install-hooks once from the primary checkout.

Ready. Next: just check.
Nothing has been formatted, staged, committed, tagged, or pushed.
Exit status: 0
```

```text
$ PATH=/Users/scutting/.cargo/bin:$PATH just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

```text
$ git status --short
(stdout empty)
Exit status: 0
```

### Item 1: primary hook shim — commit `c1b4bcb`

No primary-checkout command was run: the authorization stop in item 1 applies.
The ticket's explicit Open disposition is used; no primary-checkout operation is
awaiting or assumed to have approval. The maintainer's separately authorized primary
checkout trial, including its rejected commit and branch cleanup, is the closing event.

### Item 2: snapshots — commit `c1b4bcb`

No snapshot-writing claim is made. Baseline snapshot success reports no snapshots;
it cannot close this item. The first snapshot ticket owns the three required trials.

### Item 3: isolated cache directories, incomplete isolation — commit `c1b4bcb`

This item remains Open. The runs below establish only isolation of the named cache
directories. They do not establish that every tool ignored other home-rooted state.
Session instructions prohibit repurposing `HOME`; no fresh account, VM or container
was created. A fully isolated run with verified empty caches is the closing event.

A fresh local clone of `main` and cache directories were created in the session's
scratch directory, outside every Cargo workspace. Before initialization, rustup, pixi,
prek and XDG cache directories were empty. Cargo had no registry, package or compiler
cache in its redirected root, but its `bin` symlink exposed the host
`/Users/scutting/.cargo/bin` directory, including `cargo-binstall` and `rust-analyzer`
as well as rustup proxies. Cargo can resolve subcommands there before `PATH`, so this
run also reused host executables. No `HOME` override was used. The isolated cache
environment was retained for both commands:

```sh
CARGO_HOME="$scratch/caches/cargo"
RUSTUP_HOME="$scratch/caches/rustup"
PIXI_CACHE_DIR="$scratch/caches/pixi"
PREK_HOME="$scratch/caches/prek"
XDG_CACHE_HOME="$scratch/caches/xdg"
PATH="$CARGO_HOME/bin:$PATH"
```

These are the environment values passed to the child processes, not changes to the
user's shell configuration. The clone was a standalone scratch checkout, so its own
local hook was installed. The host's shared hook was untouched. The pixi cache acquired
`pkgs` and `uv-cache`; rustup acquired its toolchain; prek acquired `repos`, `tools`,
`cache` and `hooks`. The Cargo cache acquired its package-cache state.

An earlier isolated-cache attempt inside the workspace failed at prek's ripsecrets
installation (exit 2): Cargo treated that cached source as part of the enclosing
workspace. It is discarded evidence. The fresh attempt below started with new empty
caches outside that workspace and needed no project configuration change.

```text
$ git rev-parse --short HEAD # isolated-cache clone, checked again during review
c1b4bcb
Exit status: 0
```

```text
$ just initialize # fresh-cache clone, isolated environment above
info: downloading 6 components
Installed Git hook at `.git/hooks/pre-commit`
Ready. Next: just check.
Nothing has been formatted, staged, committed, tagged, or pushed.
initialize exit: 0
Exit status: 0
```

```text
$ just check # same fresh-cache clone and environment
All checks passed and the worktree is unchanged.
Exit status: 0
```

```text
$ git status --short # fresh-cache clone
(stdout empty)
Exit status: 0
```

### Item 4: network recipes and schedule — commit `c1b4bcb`

`just lock` ran on the scratch clone's `main`. `lock-upgrade` ran on its
`t18-lock-upgrade` branch. The captured pixi diff is quoted
with `--text` because the repository deliberately disables its normal text diff.

```text
$ just lock # isolated main-based clone
cargo update --workspace
     Locking 0 packages to highest Rust 1.99 compatible versions
pixi lock
✔ Lock-file was already up-to-date
Exit status: 0
```

```text
$ git status --short # scratch clone after this recipe/restoration
(stdout empty)
Exit status: 0
```

```text
$ just lock-upgrade # isolated main-based clone
~ (conda) python           3.14.8 h0ae1c2c_102_cp314  ->  3.14.9 h0ae1c2c_100_cp314
Platform: default:linux-64
  ~ (conda) libpython        3.14.8 hd51c5da_102_cp314  ->  3.14.9 hd51c5da_100_cp314
  ~ (conda) python           3.14.8 hcd007b5_102_cp314  ->  3.14.9 hcd007b5_100_cp314
Exit status: 0
```

The captured generated lockfile diff, before restoration (diff artifact, not a
separate command-exit claim):

```text
diff --git a/pixi.lock b/pixi.lock
index fdefd9e..fc8b87b 100644
--- a/pixi.lock
+++ b/pixi.lock
@@ -35,7 +35,7 @@ environments:
       - conda: https://conda.anaconda.org/conda-forge/linux-64/libgomp-16.2.0-he0feb66_7.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/liblzma-5.8.3-hb03c661_1.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/libmpdec-4.0.0-hb03c661_2.conda
-      - conda: https://conda.anaconda.org/conda-forge/linux-64/libpython-3.14.8-hd51c5da_102_cp314.conda
+      - conda: https://conda.anaconda.org/conda-forge/linux-64/libpython-3.14.9-hd51c5da_100_cp314.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/libsqlite-3.54.1-h0737f62_100.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/libstdcxx-16.2.0-h934c35e_7.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/libuuid-2.42.4-hcfc3c73_0.conda
@@ -43,19 +43,19 @@ environments:
       - conda: https://conda.anaconda.org/conda-forge/linux-64/ncurses-6.6-hdb14827_1.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/openssl-3.6.5-h781a0a9_0.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/prek-0.5.3-hf19af3b_0.conda
-      - conda: https://conda.anaconda.org/conda-forge/linux-64/python-3.14.8-hcd007b5_102_cp314.conda
+      - conda: https://conda.anaconda.org/conda-forge/linux-64/python-3.14.9-hcd007b5_100_cp314.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/readline-8.3-hd6e31c0_1.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/taplo-0.10.0-h2d90d6e_2.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/tk-8.6.13-noxft_h1df4ec4_4.conda
       - conda: https://conda.anaconda.org/conda-forge/linux-64/zstd-1.5.7-hb78ec9c_7.conda
-      - conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.19-hbd8a1cb_0.conda
+      - conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.23-hbd8a1cb_0.conda
       - conda: https://conda.anaconda.org/conda-forge/noarch/python_abi-3.14-9_cp314.conda
-      - conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026c-h151e31d_0.conda
+      - conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026e-h5d301ff_0.conda
       - pypi: git+https://github.com/steven-cutting/biscuit_games_tooling?tag=v0.3.0#6c5c07f6bec86e86b3930dfa41392e4b440e8c85
       osx-arm64:
-      - conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.19-hbd8a1cb_0.conda
+      - conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.23-hbd8a1cb_0.conda
       - conda: https://conda.anaconda.org/conda-forge/noarch/python_abi-3.14-9_cp314.conda
-      - conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026c-h151e31d_0.conda
+      - conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026e-h5d301ff_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/bzip2-1.0.8-h4e30115_10.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/cargo-binstall-1.23.0-h9d417a0_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/cargo-deny-0.20.2-h6fdd925_0.conda
@@ -64,18 +64,18 @@ environments:
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/cargo-nextest-0.9.146-hc2d82ec_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/cargo-shear-1.13.4-h6fdd925_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/just-1.58.0-hc2d82ec_2.conda
-      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libcxx-23.1.4-h55c6f16_0.conda
+      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libcxx-23.1.4-h55c6f16_2.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libexpat-2.8.5-h6111c0a_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libffi-3.7.0-h47dc5ef_1.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/liblzma-5.8.3-h8088a28_1.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libmpdec-4.0.0-h84a0fba_2.conda
-      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libpython-3.14.8-h281bc5c_102_cp314.conda
+      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libpython-3.14.9-h281bc5c_100_cp314.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libsqlite-3.54.1-h34a968b_100.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libzlib-1.3.2-h8088a28_3.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/ncurses-6.6-he64c551_1.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/openssl-3.6.5-h55eecbc_0.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/prek-0.5.3-hc2d82ec_0.conda
-      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/python-3.14.8-h0ae1c2c_102_cp314.conda
+      - conda: https://conda.anaconda.org/conda-forge/osx-arm64/python-3.14.9-h0ae1c2c_100_cp314.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/readline-8.3-h8b90a29_1.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/taplo-0.10.0-h29ab045_2.conda
       - conda: https://conda.anaconda.org/conda-forge/osx-arm64/tk-8.6.13-hbeba79b_4.conda
@@ -307,10 +307,10 @@ packages:
   run_exports: {}
   size: 92759
   timestamp: 1786650399772
-- conda: https://conda.anaconda.org/conda-forge/linux-64/libpython-3.14.8-hd51c5da_102_cp314.conda
-  build_number: 102
-  sha256: 8af871319cee737bf75913c7b6b2c65a0cc09376a8992dd80b3251d82066fc63
-  md5: 5bc9fb686950e9ef2543adc230476238
+- conda: https://conda.anaconda.org/conda-forge/linux-64/libpython-3.14.9-hd51c5da_100_cp314.conda
+  build_number: 100
+  sha256: a35534ec7cd9bb4b287954338ac9119c25400eae6c824fc82c2d4e0bd9d3b6ea
+  md5: e5a312eb1871eac00a7222b6c585a8dd
   depends:
   - __glibc >=2.17,<3.0.a0
   - libgcc >=15
@@ -319,8 +319,8 @@ packages:
   run_exports:
     weak:
     - libpython >=3.14,<4.0a0
-  size: 10537117
-  timestamp: 1791827266877
+  size: 10541470
+  timestamp: 1792665388525
 - conda: https://conda.anaconda.org/conda-forge/linux-64/libsqlite-3.54.1-h0737f62_100.conda
   sha256: 107ba2f9134b45a6fdaf8d2485f432e3a643fdfaf711a325d0c9139b580b3523
   md5: 5fdfd335c3d653e41c1eea1353d827d0
@@ -422,10 +422,10 @@ packages:
   run_exports: {}
   size: 4985975
   timestamp: 1789321834728
-- conda: https://conda.anaconda.org/conda-forge/linux-64/python-3.14.8-hcd007b5_102_cp314.conda
-  build_number: 102
-  sha256: 32a39d6b85fd55d9949ea48d63887bef0dc61dca46d9844d4b409eb993460617
-  md5: bcfadea0ff38555e26872aab0e974dbd
+- conda: https://conda.anaconda.org/conda-forge/linux-64/python-3.14.9-hcd007b5_100_cp314.conda
+  build_number: 100
+  sha256: 87874d825447bb21759c6378cbbc4ce380b2b639609bfd4cb67dc864641ac759
+  md5: 49fa41f5beb11c375c11af461301d74c
   depends:
   - __glibc >=2.17,<3.0.a0
   - bzip2 >=1.0.8,<2.0a0
@@ -435,8 +435,8 @@ packages:
   - libgcc >=15
   - liblzma >=5.8.3,<6.0a0
   - libmpdec >=4.0.0,<5.0a0
-  - libpython 3.14.8 hd51c5da_102_cp314
-  - libsqlite >=3.54.0,<4.0a0
+  - libpython 3.14.9 hd51c5da_100_cp314
+  - libsqlite >=3.54.1,<4.0a0
   - libuuid >=2.42.4,<3.0a0
   - libzlib >=1.3.2,<2.0a0
   - ncurses >=6.6,<7.0a0
@@ -453,8 +453,8 @@ packages:
     - python_abi 3.14.* *_cp314
     noarch:
     - python
-  size: 26647968
-  timestamp: 1791827304033
+  size: 26705680
+  timestamp: 1792665433190
   python_site_packages_path: lib/python3.14/site-packages
 - conda: https://conda.anaconda.org/conda-forge/linux-64/readline-8.3-hd6e31c0_1.conda
   sha256: 01b3fe073a66e321970d09e04b388708f8cbdca5cdfbfcb7c9eeb470ad10383d
@@ -517,16 +517,16 @@ packages:
     - zstd >=1.5.7,<1.6.0a0
   size: 601301
   timestamp: 1786599621503
-- conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.19-hbd8a1cb_0.conda
-  sha256: 9404ee878c6bd5b260655ab651366db43fac6be8d465fcc0acd0f7a50be91acb
-  md5: a1d1bf676059d2863f000d5e2777743f
+- conda: https://conda.anaconda.org/conda-forge/noarch/ca-certificates-2026.10.23-hbd8a1cb_0.conda
+  sha256: 95619b9c1184be7175392eeb0ee7eeb5ccf25d280951f4225f2d62e725790be3
+  md5: 3fd02989038a1e1cee627c14c2d73f29
   depends:
   - __unix
   license: ISC
   purls: []
   run_exports: {}
-  size: 131396
-  timestamp: 1792483552251
+  size: 130057
+  timestamp: 1792772446739
 - conda: https://conda.anaconda.org/conda-forge/noarch/python_abi-3.14-9_cp314.conda
   build_number: 9
   sha256: e7f8e965bbfb98e08feb2365cacdad8bb218fa5b5a0a2752f747287f425f213c
@@ -539,14 +539,14 @@ packages:
   run_exports: {}
   size: 6791
   timestamp: 1788302824440
-- conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026c-h151e31d_0.conda
-  sha256: b928c30ddcb0e3f544c6eade8352737e6e610e263276b90232db6a578ef899d8
-  md5: fcb489df604d100968b737f2cb6076c6
+- conda: https://conda.anaconda.org/conda-forge/noarch/tzdata-2026e-h5d301ff_0.conda
+  sha256: 2b7ca858879f183ce2dcc65b300f48a0133d5430550f35b23d80d405a0788dbf
+  md5: abccc3810d2b97e570dee487f2b46e38
   license: LicenseRef-Public-Domain
   purls: []
   run_exports: {}
-  size: 118849
-  timestamp: 1784250406640
+  size: 119208
+  timestamp: 1792758382821
 - conda: https://conda.anaconda.org/conda-forge/osx-arm64/bzip2-1.0.8-h4e30115_10.conda
   sha256: 8ec22f0ba25cbfc2e64d70cf29459eccd7ffdf6436f6a6ff15bbfef799f7d4f6
   md5: b50612e7d190b8061ab4e7dc119cf4d5
@@ -650,17 +650,17 @@ packages:
   run_exports: {}
   size: 1791527
   timestamp: 1787755611780
-- conda: https://conda.anaconda.org/conda-forge/osx-arm64/libcxx-23.1.4-h55c6f16_0.conda
-  sha256: a8ea9f3807c18e834968c805cddec95664c1ea2dd45ef3299b74a550efb97ab3
-  md5: 422cd818f43a2dc246ebecd3e59ecb90
+- conda: https://conda.anaconda.org/conda-forge/osx-arm64/libcxx-23.1.4-h55c6f16_2.conda
+  sha256: 9c390b4f403db32346754ccef6b84b7e5d9b70cb6d76258fd23b05f2db2ab216
+  md5: 08f01709cd5f8fd7af2c29e4e3ce1846
   depends:
   - __osx >=11.0
   license: Apache-2.0 WITH LLVM-exception
   license_family: Apache
   purls: []
   run_exports: {}
-  size: 578942
-  timestamp: 1792548237292
+  size: 577255
+  timestamp: 1792748855178
 - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libexpat-2.8.5-h6111c0a_0.conda
   sha256: 50aa761b83fc7a116ca88fe0cb4cefce5abababb10a926bf8afbafd5c298d102
   md5: 5383c550b785bdaafcc4eaf638071a7c
@@ -712,10 +712,10 @@ packages:
   run_exports: {}
   size: 73289
   timestamp: 1786651074391
-- conda: https://conda.anaconda.org/conda-forge/osx-arm64/libpython-3.14.8-h281bc5c_102_cp314.conda
-  build_number: 102
-  sha256: 6237bed72df34642662b7da36046fedfaea77831ee24e926846ce56e0568286d
-  md5: d8b70ed3d772724a19cb4a1a578d8fa5
+- conda: https://conda.anaconda.org/conda-forge/osx-arm64/libpython-3.14.9-h281bc5c_100_cp314.conda
+  build_number: 100
+  sha256: 91a6c3ede96aa84dfbe26ce973c4e335e405f1e72514156bac4f3115f01d5379
+  md5: a92b4a7b262cdd27bb53098145a17f7d
   depends:
   - __osx >=11.0
   license: Python-2.0
@@ -723,8 +723,8 @@ packages:
   run_exports:
     weak:
     - libpython >=3.14,<4.0a0
-  size: 1890587
-  timestamp: 1791826523560
+  size: 1883790
+  timestamp: 1792664342515
 - conda: https://conda.anaconda.org/conda-forge/osx-arm64/libsqlite-3.54.1-h34a968b_100.conda
   sha256: ed4bbcbd181a8491245c8120609ea288056c8f90db4f8e18b397066ce2a58ec7
   md5: eb7c3006eba21a8ebc3466dedd542919
@@ -794,10 +794,10 @@ packages:
   run_exports: {}
   size: 4468217
   timestamp: 1789321831632
-- conda: https://conda.anaconda.org/conda-forge/osx-arm64/python-3.14.8-h0ae1c2c_102_cp314.conda
-  build_number: 102
-  sha256: b8e5729761450f22fd591af7e565af02112a4dc0fa21717519c4f06c59ee6781
-  md5: a3bbba11fd9d39c475627842b34eda46
+- conda: https://conda.anaconda.org/conda-forge/osx-arm64/python-3.14.9-h0ae1c2c_100_cp314.conda
+  build_number: 100
+  sha256: 3704fc2b5557da0bed1b79e23c8dad1ffea284244c659da60a5b6673d71c453a
+  md5: acbe917ebaed3c466a25bdc788627fdb
   depends:
   - __osx >=11.0
   - bzip2 >=1.0.8,<2.0a0
@@ -805,8 +805,8 @@ packages:
   - libffi >=3.7.0,<3.8.0a0
   - liblzma >=5.8.3,<6.0a0
   - libmpdec >=4.0.0,<5.0a0
-  - libpython 3.14.8 h281bc5c_102_cp314
-  - libsqlite >=3.54.0,<4.0a0
+  - libpython 3.14.9 h281bc5c_100_cp314
+  - libsqlite >=3.54.1,<4.0a0
   - libzlib >=1.3.2,<2.0a0
   - ncurses >=6.6,<7.0a0
   - openssl >=3.5.9,<4.0a0
@@ -822,8 +822,8 @@ packages:
     - python_abi 3.14.* *_cp314
     noarch:
     - python
-  size: 12458293
-  timestamp: 1791826556052
+  size: 12487003
+  timestamp: 1792664375847
   python_site_packages_path: lib/python3.14/site-packages
 - conda: https://conda.anaconda.org/conda-forge/osx-arm64/readline-8.3-h8b90a29_1.conda
   sha256: d6782b430e6dce4fe8c7ac118cd988d5a193eb4a7f60741edfaede58016b6110
```

```text
$ git restore Cargo.lock pixi.lock # scratch copy only

Exit status: 0
```

```text
$ git status --short # scratch clone after this recipe/restoration
(stdout empty)
Exit status: 0
```

```text
$ just check-links-online # isolated main-based clone
Lychee online (manual)...................................................Passed
prek run --all-files --hook-stage manual lychee-online
Exit status: 0
```

```text
$ git status --short # scratch clone after this recipe/restoration
(stdout empty)
Exit status: 0
```

```text
$ gh api 'repos/BiscuitBin/pachinko/actions/workflows/audit.yml/runs?branch=main&event=schedule&per_page=100' --jq '{total_count, runs: [.workflow_runs[] | {html_url,head_sha,event,conclusion,created_at}]}'
{"runs":[],"total_count":0}
Exit status: 0
```

This is an HTTP-success read with a zero total, not an inaccessible-resource 404.
There is no scheduled-run URL or conclusion to quote yet; no dispatch was substituted.

### Item 5: optional banned dependency — commit `c1b4bcb` with uncommitted scratch changes

In libpachinko, the local fixture crate `rand` was declared `optional = true`,
with `probe-rand = ["dep:rand"]`. Offline lock generation succeeded (exit 0).

```text
$ just effects
sh scripts/check-effects.sh
libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/libpachinko)
rand v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/tests/fixtures/effects-violation/deps/rand)
effects: libpachinko depends on a banned crate:
rand
want none
error: recipe `effects` failed on line 187 with exit code 1
Exit status: 1
```

```text
$ git status --short # after restoring manifest and regenerating original lock
(stdout empty)
Exit status: 0
```

### Item 6: per-ban red runs — commit `c1b4bcb` with uncommitted scratch changes

T10's archived red harness was copied into this ticket's scratch directory and
run against `git archive HEAD` at `c1b4bcb`. Every case starts from restored source
with fresh modification times, preserving only build output. No negative fixture runs
its effect: clippy resolves it. The old harness's deleted-tag parser also captured a
new explanatory footer as a path; all 119 deleted-tag cases were rerun with that parser
stopping before the footer. The expected path sets were unchanged and compared exactly.

There are 373 required negative runs: 16 crate cases and 357 API cases. Two fixture
functions each cover two entries, so 119 functions cover all 121 entries in each of
three modes. All 373 exited 1 with the requested diagnostic and exact named set; the
harness's 12 additional controls also matched expectations. An unrelated nonzero exit
was never counted as a pass.

The compact API transcripts below join the actual diagnostic header and its following
path lines with ` | `. Each row is a separate `just effects` invocation and quotes
its exit status; it is not output from one combined invocation. No API row is omitted.

```text
$ just effects # lib-crate rand
effects: libpachinko depends on a banned crate:
rand
want none
Exit status: 1
```

```text
$ just effects # lib-crate rand_core
effects: libpachinko depends on a banned crate:
rand_core
want none
Exit status: 1
```

```text
$ just effects # lib-crate getrandom
effects: libpachinko depends on a banned crate:
getrandom
want none
Exit status: 1
```

```text
$ just effects # lib-crate tokio
effects: libpachinko depends on a banned crate:
tokio
want none
Exit status: 1
```

```text
$ just effects # lib-crate async-std
effects: libpachinko depends on a banned crate:
async-std
want none
Exit status: 1
```

```text
$ just effects # lib-crate smol
effects: libpachinko depends on a banned crate:
smol
want none
Exit status: 1
```

```text
$ just effects # lib-crate mio
effects: libpachinko depends on a banned crate:
mio
want none
Exit status: 1
```

```text
$ just effects # lib-crate socket2
effects: libpachinko depends on a banned crate:
socket2
want none
Exit status: 1
```

```text
$ just effects # fixture-crate-removed rand
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand_core
smol
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed rand_core
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
smol
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed getrandom
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
mio
rand
rand_core
smol
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed tokio
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
rand_core
smol
socket2
Exit status: 1
```

```text
$ just effects # fixture-crate-removed async-std
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
getrandom
mio
rand
rand_core
smol
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed smol
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
rand_core
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed mio
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
rand
rand_core
smol
socket2
tokio
Exit status: 1
```

```text
$ just effects # fixture-crate-removed socket2
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
rand_core
smol
tokio
Exit status: 1
```

Mode: `lib-api`.

```text
exit 1 | banned APIs used: | std::time::SystemTime::now
exit 1 | banned APIs used: | std::time::SystemTime::elapsed
exit 1 | banned APIs used: | std::time::Instant::now
exit 1 | banned APIs used: | std::time::Instant::elapsed
exit 1 | banned APIs used: | std::thread::spawn
exit 1 | banned APIs used: | std::thread::scope
exit 1 | banned APIs used: | std::thread::Scope::spawn
exit 1 | banned APIs used: | std::thread::sleep
exit 1 | banned APIs used: | std::thread::sleep_ms
exit 1 | banned APIs used: | std::thread::Builder
exit 1 | banned APIs used: | std::thread::park_timeout
exit 1 | banned APIs used: | std::thread::park_timeout_ms
exit 1 | banned APIs used: | std::sync::Condvar::wait_timeout
exit 1 | banned APIs used: | std::sync::Condvar::wait_timeout_ms
exit 1 | banned APIs used: | std::sync::Condvar::wait_timeout_while
exit 1 | banned APIs used: | std::sync::mpsc::Receiver::recv_timeout
exit 1 | banned APIs used: | std::thread::available_parallelism
exit 1 | banned APIs used: | std::hash::RandomState
exit 1 | banned APIs used: | std::fs::canonicalize
exit 1 | banned APIs used: | std::fs::copy
exit 1 | banned APIs used: | std::fs::create_dir
exit 1 | banned APIs used: | std::fs::create_dir_all
exit 1 | banned APIs used: | std::fs::exists
exit 1 | banned APIs used: | std::fs::hard_link
exit 1 | banned APIs used: | std::fs::metadata
exit 1 | banned APIs used: | std::fs::read
exit 1 | banned APIs used: | std::fs::read_dir
exit 1 | banned APIs used: | std::fs::read_link
exit 1 | banned APIs used: | std::fs::read_to_string
exit 1 | banned APIs used: | std::fs::remove_dir
exit 1 | banned APIs used: | std::fs::remove_dir_all
exit 1 | banned APIs used: | std::fs::remove_file
exit 1 | banned APIs used: | std::fs::rename
exit 1 | banned APIs used: | std::fs::Permissions | std::fs::set_permissions
exit 1 | banned APIs used: | std::fs::set_times
exit 1 | banned APIs used: | std::fs::set_times_nofollow
exit 1 | banned APIs used: | std::fs::soft_link
exit 1 | banned APIs used: | std::fs::symlink_metadata
exit 1 | banned APIs used: | std::fs::write
exit 1 | banned APIs used: | std::fs::DirBuilder
exit 1 | banned APIs used: | std::fs::DirEntry
exit 1 | banned APIs used: | std::fs::File
exit 1 | banned APIs used: | std::fs::FileTimes
exit 1 | banned APIs used: | std::fs::FileType
exit 1 | banned APIs used: | std::fs::Metadata
exit 1 | banned APIs used: | std::fs::OpenOptions
exit 1 | banned APIs used: | std::fs::ReadDir
exit 1 | banned APIs used: | std::fs::TryLockError
exit 1 | banned APIs used: | std::path::Path::canonicalize
exit 1 | banned APIs used: | std::path::Path::exists
exit 1 | banned APIs used: | std::path::Path::is_dir
exit 1 | banned APIs used: | std::path::Path::is_file
exit 1 | banned APIs used: | std::path::Path::is_symlink
exit 1 | banned APIs used: | std::path::Path::metadata
exit 1 | banned APIs used: | std::path::Path::read_dir
exit 1 | banned APIs used: | std::path::Path::read_link
exit 1 | banned APIs used: | std::path::Path::symlink_metadata
exit 1 | banned APIs used: | std::path::Path::try_exists
exit 1 | banned APIs used: | std::os::unix::fs::chown
exit 1 | banned APIs used: | std::os::unix::fs::chroot
exit 1 | banned APIs used: | std::os::unix::fs::fchown
exit 1 | banned APIs used: | std::os::unix::fs::lchown
exit 1 | banned APIs used: | std::os::unix::fs::symlink
exit 1 | banned APIs used: | std::env::args
exit 1 | banned APIs used: | std::env::args_os
exit 1 | banned APIs used: | std::env::current_dir
exit 1 | banned APIs used: | std::env::current_exe
exit 1 | banned APIs used: | std::env::home_dir
exit 1 | banned APIs used: | std::env::join_paths
exit 1 | banned APIs used: | std::env::remove_var
exit 1 | banned APIs used: | std::env::set_current_dir
exit 1 | banned APIs used: | std::env::set_var
exit 1 | banned APIs used: | std::env::split_paths
exit 1 | banned APIs used: | std::env::temp_dir
exit 1 | banned APIs used: | std::env::var
exit 1 | banned APIs used: | std::env::var_os
exit 1 | banned APIs used: | std::env::vars
exit 1 | banned APIs used: | std::env::vars_os
exit 1 | banned APIs used: | std::path::absolute
exit 1 | banned APIs used: | std::env::Args
exit 1 | banned APIs used: | std::env::ArgsOs
exit 1 | banned APIs used: | std::env::JoinPathsError
exit 1 | banned APIs used: | std::env::SplitPaths
exit 1 | banned APIs used: | std::env::VarError
exit 1 | banned APIs used: | std::env::Vars
exit 1 | banned APIs used: | std::env::VarsOs
exit 1 | banned APIs used: | std::net::ToSocketAddrs | std::net::ToSocketAddrs::to_socket_addrs
exit 1 | banned APIs used: | std::net::AddrParseError
exit 1 | banned APIs used: | std::net::Incoming
exit 1 | banned APIs used: | std::net::IpAddr
exit 1 | banned APIs used: | std::net::Ipv4Addr
exit 1 | banned APIs used: | std::net::Ipv6Addr
exit 1 | banned APIs used: | std::net::Shutdown
exit 1 | banned APIs used: | std::net::SocketAddr
exit 1 | banned APIs used: | std::net::SocketAddrV4
exit 1 | banned APIs used: | std::net::SocketAddrV6
exit 1 | banned APIs used: | std::net::TcpListener
exit 1 | banned APIs used: | std::net::TcpStream
exit 1 | banned APIs used: | std::net::UdpSocket
exit 1 | banned APIs used: | std::os::unix::net::Incoming
exit 1 | banned APIs used: | std::os::unix::net::UnixDatagram
exit 1 | banned APIs used: | std::os::unix::net::UnixListener
exit 1 | banned APIs used: | std::os::unix::net::UnixStream
exit 1 | banned APIs used: | std::process::abort
exit 1 | banned APIs used: | std::process::exit
exit 1 | banned APIs used: | std::process::id
exit 1 | banned APIs used: | std::os::unix::process::parent_id
exit 1 | banned APIs used: | std::process::Child
exit 1 | banned APIs used: | std::process::ChildStderr
exit 1 | banned APIs used: | std::process::ChildStdin
exit 1 | banned APIs used: | std::process::ChildStdout
exit 1 | banned APIs used: | std::process::Command
exit 1 | banned APIs used: | std::process::CommandArgs
exit 1 | banned APIs used: | std::process::CommandEnvs
exit 1 | banned APIs used: | std::process::ExitCode
exit 1 | banned APIs used: | std::process::ExitStatus
exit 1 | banned APIs used: | std::process::Output
exit 1 | banned APIs used: | std::process::Stdio
exit 1 | banned APIs used: | std::process::Termination
```

Mode: `fixture-api-silenced`.

```text
exit 1 | silent: | std::time::SystemTime::now
exit 1 | silent: | std::time::SystemTime::elapsed
exit 1 | silent: | std::time::Instant::now
exit 1 | silent: | std::time::Instant::elapsed
exit 1 | silent: | std::thread::spawn
exit 1 | silent: | std::thread::scope
exit 1 | silent: | std::thread::Scope::spawn
exit 1 | silent: | std::thread::sleep
exit 1 | silent: | std::thread::sleep_ms
exit 1 | silent: | std::thread::Builder
exit 1 | silent: | std::thread::park_timeout
exit 1 | silent: | std::thread::park_timeout_ms
exit 1 | silent: | std::sync::Condvar::wait_timeout
exit 1 | silent: | std::sync::Condvar::wait_timeout_ms
exit 1 | silent: | std::sync::Condvar::wait_timeout_while
exit 1 | silent: | std::sync::mpsc::Receiver::recv_timeout
exit 1 | silent: | std::thread::available_parallelism
exit 1 | silent: | std::hash::RandomState
exit 1 | silent: | std::fs::canonicalize
exit 1 | silent: | std::fs::copy
exit 1 | silent: | std::fs::create_dir
exit 1 | silent: | std::fs::create_dir_all
exit 1 | silent: | std::fs::exists
exit 1 | silent: | std::fs::hard_link
exit 1 | silent: | std::fs::metadata
exit 1 | silent: | std::fs::read
exit 1 | silent: | std::fs::read_dir
exit 1 | silent: | std::fs::read_link
exit 1 | silent: | std::fs::read_to_string
exit 1 | silent: | std::fs::remove_dir
exit 1 | silent: | std::fs::remove_dir_all
exit 1 | silent: | std::fs::remove_file
exit 1 | silent: | std::fs::rename
exit 1 | silent: | std::fs::Permissions | std::fs::set_permissions
exit 1 | silent: | std::fs::set_times
exit 1 | silent: | std::fs::set_times_nofollow
exit 1 | silent: | std::fs::soft_link
exit 1 | silent: | std::fs::symlink_metadata
exit 1 | silent: | std::fs::write
exit 1 | silent: | std::fs::DirBuilder
exit 1 | silent: | std::fs::DirEntry
exit 1 | silent: | std::fs::File
exit 1 | silent: | std::fs::FileTimes
exit 1 | silent: | std::fs::FileType
exit 1 | silent: | std::fs::Metadata
exit 1 | silent: | std::fs::OpenOptions
exit 1 | silent: | std::fs::ReadDir
exit 1 | silent: | std::fs::TryLockError
exit 1 | silent: | std::path::Path::canonicalize
exit 1 | silent: | std::path::Path::exists
exit 1 | silent: | std::path::Path::is_dir
exit 1 | silent: | std::path::Path::is_file
exit 1 | silent: | std::path::Path::is_symlink
exit 1 | silent: | std::path::Path::metadata
exit 1 | silent: | std::path::Path::read_dir
exit 1 | silent: | std::path::Path::read_link
exit 1 | silent: | std::path::Path::symlink_metadata
exit 1 | silent: | std::path::Path::try_exists
exit 1 | silent: | std::os::unix::fs::chown
exit 1 | silent: | std::os::unix::fs::chroot
exit 1 | silent: | std::os::unix::fs::fchown
exit 1 | silent: | std::os::unix::fs::lchown
exit 1 | silent: | std::os::unix::fs::symlink
exit 1 | silent: | std::env::args
exit 1 | silent: | std::env::args_os
exit 1 | silent: | std::env::current_dir
exit 1 | silent: | std::env::current_exe
exit 1 | silent: | std::env::home_dir
exit 1 | silent: | std::env::join_paths
exit 1 | silent: | std::env::remove_var
exit 1 | silent: | std::env::set_current_dir
exit 1 | silent: | std::env::set_var
exit 1 | silent: | std::env::split_paths
exit 1 | silent: | std::env::temp_dir
exit 1 | silent: | std::env::var
exit 1 | silent: | std::env::var_os
exit 1 | silent: | std::env::vars
exit 1 | silent: | std::env::vars_os
exit 1 | silent: | std::path::absolute
exit 1 | silent: | std::env::Args
exit 1 | silent: | std::env::ArgsOs
exit 1 | silent: | std::env::JoinPathsError
exit 1 | silent: | std::env::SplitPaths
exit 1 | silent: | std::env::VarError
exit 1 | silent: | std::env::Vars
exit 1 | silent: | std::env::VarsOs
exit 1 | silent: | std::net::ToSocketAddrs | std::net::ToSocketAddrs::to_socket_addrs
exit 1 | silent: | std::net::AddrParseError
exit 1 | silent: | std::net::Incoming
exit 1 | silent: | std::net::IpAddr
exit 1 | silent: | std::net::Ipv4Addr
exit 1 | silent: | std::net::Ipv6Addr
exit 1 | silent: | std::net::Shutdown
exit 1 | silent: | std::net::SocketAddr
exit 1 | silent: | std::net::SocketAddrV4
exit 1 | silent: | std::net::SocketAddrV6
exit 1 | silent: | std::net::TcpListener
exit 1 | silent: | std::net::TcpStream
exit 1 | silent: | std::net::UdpSocket
exit 1 | silent: | std::os::unix::net::Incoming
exit 1 | silent: | std::os::unix::net::UnixDatagram
exit 1 | silent: | std::os::unix::net::UnixListener
exit 1 | silent: | std::os::unix::net::UnixStream
exit 1 | silent: | std::process::abort
exit 1 | silent: | std::process::exit
exit 1 | silent: | std::process::id
exit 1 | silent: | std::os::unix::process::parent_id
exit 1 | silent: | std::process::Child
exit 1 | silent: | std::process::ChildStderr
exit 1 | silent: | std::process::ChildStdin
exit 1 | silent: | std::process::ChildStdout
exit 1 | silent: | std::process::Command
exit 1 | silent: | std::process::CommandArgs
exit 1 | silent: | std::process::CommandEnvs
exit 1 | silent: | std::process::ExitCode
exit 1 | silent: | std::process::ExitStatus
exit 1 | silent: | std::process::Output
exit 1 | silent: | std::process::Stdio
exit 1 | silent: | std::process::Termination
```

Mode: `fixture-api-deleted`.

```text
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::time::SystemTime::now
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::time::SystemTime::elapsed
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::time::Instant::now
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::time::Instant::elapsed
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::spawn
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::scope
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::Scope::spawn
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::sleep
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::sleep_ms
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::Builder
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::park_timeout
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::park_timeout_ms
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::sync::Condvar::wait_timeout
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::sync::Condvar::wait_timeout_ms
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::sync::Condvar::wait_timeout_while
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::sync::mpsc::Receiver::recv_timeout
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::thread::available_parallelism
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::hash::RandomState
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::canonicalize
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::copy
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::create_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::create_dir_all
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::exists
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::hard_link
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::metadata
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::read
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::read_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::read_link
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::read_to_string
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::remove_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::remove_dir_all
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::remove_file
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::rename
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::Permissions | std::fs::set_permissions
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::set_times
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::set_times_nofollow
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::soft_link
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::symlink_metadata
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::write
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::DirBuilder
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::DirEntry
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::File
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::FileTimes
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::FileType
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::Metadata
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::OpenOptions
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::ReadDir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::fs::TryLockError
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::canonicalize
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::exists
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::is_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::is_file
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::is_symlink
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::metadata
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::read_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::read_link
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::symlink_metadata
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::Path::try_exists
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::fs::chown
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::fs::chroot
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::fs::fchown
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::fs::lchown
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::fs::symlink
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::args
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::args_os
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::current_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::current_exe
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::home_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::join_paths
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::remove_var
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::set_current_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::set_var
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::split_paths
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::temp_dir
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::var
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::var_os
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::vars
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::vars_os
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::path::absolute
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::Args
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::ArgsOs
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::JoinPathsError
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::SplitPaths
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::VarError
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::Vars
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::env::VarsOs
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::ToSocketAddrs | std::net::ToSocketAddrs::to_socket_addrs
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::AddrParseError
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::Incoming
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::IpAddr
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::Ipv4Addr
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::Ipv6Addr
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::Shutdown
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::SocketAddr
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::SocketAddrV4
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::SocketAddrV6
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::TcpListener
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::TcpStream
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::net::UdpSocket
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::net::Incoming
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::net::UnixDatagram
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::net::UnixListener
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::net::UnixStream
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::abort
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::exit
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::id
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::os::unix::process::parent_id
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::Child
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::ChildStderr
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::ChildStdin
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::ChildStdout
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::Command
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::CommandArgs
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::CommandEnvs
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::ExitCode
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::ExitStatus
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::Output
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::Stdio
exit 1 | in crates/libpachinko/clippy.toml, not tagged: | std::process::Termination
```

```text
$ just effects # fully restored archive copy
effects: libpachinko names none of 8 banned crates and uses none of 121 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
sh scripts/check-effects.sh
Exit status: 0
```

The archive copies are not independent Git repositories, so their restoration is
checked directly against their base copies, excluding only build artifacts:

```text
$ diff -rq -x target ai_tmp/t18/red/base ai_tmp/t18/red/work
(stdout empty)
Exit status: 0
$ diff -rq -x target ai_tmp/t18/red-deleted/base ai_tmp/t18/red-deleted/work
(stdout empty)
Exit status: 0
```

The following status establishes only the ticket worktree's tracked cleanliness:

```text
$ git status --short # ticket worktree after the harness
(stdout empty)
Exit status: 0
```

### Item 7: Linux network denial — commit `c1b4bcb`

`uname -s` returned `Darwin` (exit 0). Docker inspection found the cached pixi
image to be `arm64 linux` (exit 0); its namespace prerequisite probe returned:

```text
$ docker run --rm --pull=never --entrypoint /bin/sh ghcr.io/prefix-dev/pixi:0.81.0 -c 'uname -m; command -v unshare; command -v curl; command -v cargo; command -v rustup; command -v git; unshare -rn true'
aarch64
/usr/bin/unshare
unshare: unshare failed: Operation not permitted
Exit status: 1
```

The command did not launch a denial namespace, run either curl control, or run
`just check`. Its failures are prerequisites only. The workspace has `linux-64` and
`osx-arm64` environments, not `linux-aarch64`; an x86-64 Linux runner with namespace
permission is the closing event. No macOS or Docker isolation result is substituted.
The `unshare` rejection describes this container's default execution restrictions;
it does not prove the host cannot run a suitably configured Linux container. Emulated
AMD64, a relaxed seccomp profile and a CI dispatch were not exercised. `--pull=never`
limited the probe to the cached image; it is not a claim that pulling an image is
forbidden. A maintainer's Linux verification run with a usable x86-64 environment,
namespace permission and both failing curl controls will close this item.

### Item 8: tutorial — commit `c1b4bcb` with uncommitted scratch changes

The standalone scratch clone started from `main` at `c1b4bcb`. It received its own
pixi environment, tools and hook via `just initialize` (exit 0). The baseline full gate
and `just doc` both exited 0. The root specification and both test expectations were
read. Then the documented format-string change inserted `version` and a space, before changing
either expectation. The regular test run failed; the documented no-fail-fast run showed
both predicted failures. Updating both byte strings made the suite and gate pass. The
two exercise files were restored afterward. The baseline and final gate are actual
runs in that clone after its own initialization.

```text
$ just initialize # tutorial clone
prek install --overwrite --hook-type=pre-commit --prepare-hooks
prek prepare-hooks --config .pre-commit-fix.yaml
RUSTUP_AUTO_INSTALL=0 prek run lychee --files README.md
Exit status: 0
```

```text
$ just check # tutorial baseline
All checks passed and the worktree is unchanged.
Exit status: 0
```

```text
$ just doc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
   Generated /Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/tutorial/target/doc/libpachinko/index.html and 1 other file
Exit status: 0
```

```text
$ just test # only the format string changed
FAIL [   0.008s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
    test tests::run_writes_the_name_and_the_library_version ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    assertion `left == right` failed
      left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
     right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
        FAIL [   0.192s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
    test the_binary_prints_its_line_and_exits_zero ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
    assertion `left == right` failed
      left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
     right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
     Summary [   0.192s] 3 tests run: 1 passed, 2 failed, 0 skipped
        FAIL [   0.008s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.192s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
error: test run failed
error: recipe `test` failed on line 106 with exit code 100
Exit status: 100
```

```text
$ pixi run --frozen cargo nextest run --workspace --all-features --locked --no-fail-fast
FAIL [   0.008s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
    test tests::run_writes_the_name_and_the_library_version ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    assertion `left == right` failed
      left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
     right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
        FAIL [   0.118s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
    test the_binary_prints_its_line_and_exits_zero ... FAILED
    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
    assertion `left == right` failed
      left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
     right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
     Summary [   0.119s] 3 tests run: 1 passed, 2 failed, 0 skipped
        FAIL [   0.008s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.118s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
Exit status: 100
```

```text
$ just test # both byte expectations updated
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Summary [   0.194s] 3 tests run: 3 passed, 0 skipped
Exit status: 0
```

```text
$ just check # tutorial change and tests
All checks passed and the worktree is unchanged.
Exit status: 0
```

```text
$ git status --short # after restoring the two exercise files
(stdout empty)
Exit status: 0
```

### Item 9: negative controls — commit `c1b4bcb` with uncommitted scratch changes

Each control changed one thing in a restored scratch copy. The format control
changed only spacing in `writeln!`. The feature control added `probe-a` and `probe-b`
and a `compile_error!` only for `probe-a` without `probe-b`: `just build` with every
feature passed before `features` found the failing subset. The coverage control added
an uncalled function with 44 `wrapping_add` statements; all tests passed, and the
failure was the floor step. The doc control introduced a broken intra-doc link. The
unused-dependency control added the local fixture `rand` to pachinko. The license and
openssl controls used valid, compilable local stubs with explicit `0.1.0` requirements;
they had no build scripts. The source control used a local git repository and an
explicit version so the wildcard ban stayed green.

Early attempts at the license and openssl fixtures had nested-workspace errors; the
first source attempt also tripped the wildcard ban. Those attempts are not evidence.
The corrected runs below built successfully before reaching the intended deny category.
No production gate configuration changed.

```text
$ just fmt-check # bad spacing
Diff in /Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/pachinko/src/lib.rs:12:
error: recipe `fmt-check` failed on line 152 with exit code 1
Exit status: 1
```

```text
$ just fmt-check # restored

Exit status: 0
```

```text
$ just build # all features, before subset check
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s
Exit status: 0
```

```text
$ just features # failing feature subset
info: running `cargo check --locked --no-default-features --features probe-a,probe-b` on pachinko (3/5)
info: running `cargo check --locked --no-default-features --features probe-a` on pachinko (4/5)
error: T18 feature combination rejected
31 | compile_error!("T18 feature combination rejected");
error: could not compile `pachinko` (lib) due to 1 previous error
error: process didn't exit successfully: `/Users/scutting/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin/cargo check --locked --manifest-path crates/pachinko/Cargo.toml --no-default-features --features probe-a` (exit status: 101)
error: recipe `features` failed on line 192 with exit code 1
Exit status: 1
```

```text
$ just features # restored
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
Exit status: 0
```

```text
$ just coverage # untested function
TOTAL                             160               135    15.62%           6                 1    83.33%          64                47    26.56%           0                 0         -
Summary [   0.205s] 3 tests run: 3 passed, 0 skipped
cargo llvm-cov report --fail-under-lines 90
error: recipe `coverage` failed on line 201 with exit code 1
Exit status: 1
```

The 26.56% total is printed by this failing gate run itself. `Justfile:201` is
the `--fail-under-lines 90` report step; the preceding tests succeeded and the later
LCOV-export step did not run. The tool prints the below-floor total and exits 1
without a separate prose threshold message. The successful report below uses the
same instrumentation and confirms the data are readable.

```text
$ cargo llvm-cov report --show-missing-lines # same failing-floor instrumentation
TOTAL                             160               135    15.62%           6                 1    83.33%          64                47    26.56%           0                 0         -
Uncovered Lines:
/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/pachinko/src/lib.rs: 32-78
Exit status: 0
```

```text
$ just coverage # restored
TOTAL                              25                 0   100.00%           5                 0   100.00%          17                 0   100.00%           0                 0         -
     Summary [   0.193s] 3 tests run: 3 passed, 0 skipped
Exit status: 0
```

```text
$ just doc # broken intra-doc link
error: unresolved link to `t18_missing_item`
9 | /// See [`t18_missing_item`].
  |           ^^^^^^^^^^^^^^^^ no item named `t18_missing_item` in scope
  = note: requested on the command line with `-D rustdoc::broken-intra-doc-links`
error: could not document `pachinko`
error: recipe `doc` failed on line 206 with exit code 101
Exit status: 101
```

```text
$ just doc # restored
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
   Generated /Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/target/doc/libpachinko/index.html and 1 other file
Exit status: 0
```

```text
$ just deps-unused # unused fixture dependency
shear/unused_dependency

  × unused dependency `rand`
    ╭─[crates/pachinko/Cargo.toml:23:1]
 22 │ [dependencies]
 23 │ rand = { path = "../../tests/fixtures/effects-violation/deps/rand" }
    · ──┬─
    ·   ╰── not used in code
 24 │ # Invariant 5: the service depends on the library, never the reverse. Without the
    ╰────
  help: remove this dependency

shear/summary

  ✗ 1 error

Advice:
  ☞ run with `--fix` to fix 1 issue
  ☞ to suppress a dependency issue
   ╭─[Cargo.toml:2:12]
 1 │ [package.metadata.cargo-shear] # or [workspace.metadata.cargo-shear]
 2 │ ignored = ["crate-name"]
   ·            ──────┬─────
   ·                  ╰── add a crate name here
   ╰────
cargo shear
error: recipe `deps-unused` failed on line 229 with exit code 1
Exit status: 1
```

```text
$ just deps-unused # restored
shear/summary

  ✓ no issues found
cargo shear
Exit status: 0
```

```text
$ just build # GPL-3.0-only fixture
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
Exit status: 0
```

```text
$ just deny # GPL-3.0-only
bans ok, licenses FAILED, sources ok
error[rejected]: failed to satisfy license requirements
5 │ license = "GPL-3.0-only"
  │            rejected: license is not explicitly allowed
  ├ GPL-3.0-only - GNU General Public License v3.0 only:
Exit status: 4
```

```text
$ just deny # restored after license
bans ok, licenses ok, sources ok
Exit status: 0
```

```text
$ just build # openssl-sys fixture
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
Exit status: 0
```

```text
$ just deny # openssl-sys
bans FAILED, licenses ok, sources ok
error[banned]: crate 'openssl-sys = 0.1.0' is explicitly banned
60 │ deny = [{ crate = "openssl-sys", reason = "no native TLS anywhere in this workspace" }]
   ├ openssl-sys v0.1.0
Exit status: 2
```

```text
$ just deny # restored after openssl
bans ok, licenses ok, sources ok
Exit status: 0
```

The source rejection is quoted in item 11. After that pair, the dependency was
removed and the original lockfile restored; the restored-tree control was:

```text
$ just deny # restored after source dependency removal
bans ok, licenses ok, sources ok
Exit status: 0
```

```text
$ git status --short # restored control copy
(stdout empty)
Exit status: 0
```

### Item 10: carried claims — commit `c1b4bcb` with uncommitted scratch changes

```text
$ cargo search allium-cli --limit 5
allium-cli = "3.6.1"    # CLI for checking Allium specification files
note: to learn more about a package, run `cargo info <name>`
Exit status: 0
```

For function counting, an added function had 62 comment/blank lines and one
`if` expression: 65 lines from opening to closing brace. Its length finding below
shows rustqual counts those comments and blanks when the function contains logic.
Replacing that expression with `false`, retaining the same 65-line extent, passes:
logic-free functions are exempt from the function-length rule. For the budget pair the same short
function was annotated first with `#[allow(dead_code)]`, then `#[expect(dead_code,
reason = "T18 budget control")]`, with the rest of the source and configuration
unchanged. Only the allow case charged the budget.

```text
$ just metrics # rustqual-lines
::notice file=src/lib.rs,line=20::function length 65 in t18_probe
::error::Quality analysis: 1 finding(s) (0 IOSP violation(s)), 58.0% quality score
error: recipe `metrics` failed with exit code 1
Exit status: 1
```

```text
$ just metrics # same 65 lines, no logic or calls
::notice::Quality score: 100.0% (3 functions analyzed)
::notice::Quality score: 100.0% (4 functions analyzed)
Exit status: 0
```

```text
$ just metrics # rustqual-allow
::warning::Suppression ratio exceeds configured maximum (1 suppressions across 3 functions)
::notice::Quality score: 100.0% (3 functions analyzed)
Warning: 1 suppression(s) found (33.3% of functions, max: 5.0%)
Error: warnings present and --fail-on-warnings is set
error: recipe `metrics` failed with exit code 1
Exit status: 1
```

```text
$ just metrics # rustqual-expect
::notice::Quality score: 100.0% (3 functions analyzed)
::notice::Quality score: 100.0% (4 functions analyzed)
Exit status: 0
```

```text
$ just metrics # rustqual-green
::notice::Quality score: 100.0% (2 functions analyzed)
::notice::Quality score: 100.0% (4 functions analyzed)
Exit status: 0
```

Clippy's separate control used a test function inside the existing test module.
With 61 `let _value = version();` statements, only `too_many_lines` rejected it.
Replacing its body with 31 comment lines, 31 blank lines and one such statement
made a 65-line brace-to-brace function pass. This verifies that clippy excludes
comments and blank lines; the restored source also passes.

```text
$ just clippy # 61 statements
error: this function has too many lines (61/60)
error: could not compile `libpachinko` (lib test) due to 1 previous error
error: recipe `clippy` failed on line 161 with exit code 101
Exit status: 101
```

```text
$ just clippy # 31 comments, 31 blank lines, one statement
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
Exit status: 0
$ just clippy # restored source
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
Exit status: 0
```

The paginated upstream release read succeeded. All 35 returned release records,
from `v0.3.8` through `v1.8.3`, have an empty asset list:

```text
$ gh api --paginate repos/SaschaOnTour/rustqual/releases --jq '.[] | {tag_name,assets:[.assets[].name]}'
{"assets":[],"tag_name":"v1.8.3"}
{"assets":[],"tag_name":"v1.8.2"}
{"assets":[],"tag_name":"v1.8.1"}
{"assets":[],"tag_name":"v1.8.0"}
{"assets":[],"tag_name":"v1.7.0"}
{"assets":[],"tag_name":"v1.6.1"}
{"assets":[],"tag_name":"v1.6.0"}
{"assets":[],"tag_name":"v1.5.0"}
{"assets":[],"tag_name":"v1.4.2"}
{"assets":[],"tag_name":"v1.4.1"}
{"assets":[],"tag_name":"v1.3.0"}
{"assets":[],"tag_name":"v1.2.6"}
{"assets":[],"tag_name":"v1.2.5"}
{"assets":[],"tag_name":"v1.2.4"}
{"assets":[],"tag_name":"v1.2.3"}
{"assets":[],"tag_name":"v1.2.2"}
{"assets":[],"tag_name":"v1.2.0"}
{"assets":[],"tag_name":"v1.1.0"}
{"assets":[],"tag_name":"v1.0.1"}
{"assets":[],"tag_name":"v1.0.0"}
{"assets":[],"tag_name":"v0.5.6"}
{"assets":[],"tag_name":"v0.5.5"}
{"assets":[],"tag_name":"v0.5.4"}
{"assets":[],"tag_name":"v0.5.3"}
{"assets":[],"tag_name":"v0.5.2"}
{"assets":[],"tag_name":"v0.5.1"}
{"assets":[],"tag_name":"v0.5.0"}
{"assets":[],"tag_name":"v0.4.6"}
{"assets":[],"tag_name":"v0.4.4"}
{"assets":[],"tag_name":"v0.4.3"}
{"assets":[],"tag_name":"v0.4.2"}
{"assets":[],"tag_name":"v0.4.1"}
{"assets":[],"tag_name":"v0.4.0"}
{"assets":[],"tag_name":"v0.3.9"}
{"assets":[],"tag_name":"v0.3.8"}
Exit status: 0
```

Thus the upstream releases publish no binaries at this read. Initialization did
find a signed third-party QuickInstall archive for 1.8.3, which decision 0012 explicitly
permits; the release-assets claim does not mean that archive is unavailable.

The existing normal edge is `pachinko -> libpachinko`. Adding the reverse edge
as a dev-dependency allowed offline lock generation and a build (both exit 0).
Changing only the section to normal dependencies failed lock generation and build
(both exit 101) with the cycle diagnostic. A further test run compiled the dev edge
and executed an added libpachinko test that calls `pachinko::run`, asserting its output
is `b"pachinko 0.1.0\n"`. Thus the development cycle is exercised by test compilation
and execution as well as resolution. The temporary test, manifest edge and lockfile
change were restored, and the original three-test suite passed again.

```text
$ just lock # dev-cycle probe; CARGO_NET_OFFLINE=true PIXI_OFFLINE=true
cargo update --workspace
     Locking 0 packages to highest Rust 1.99 compatible versions
pixi lock
✔ Lock-file was already up-to-date
Exit status: 0
$ just test # libpachinko dev-depends on pachinko and calls its run function
INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
        PASS [   0.009s] (2/4) libpachinko t18_cycle_executes_service
     Summary [   0.193s] 4 tests run: 4 passed, 0 skipped
cargo test --doc --workspace --all-features --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Exit status: 0
$ just test # restored source, manifest and lockfile
     Summary [   0.200s] 3 tests run: 3 passed, 0 skipped
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Exit status: 0
```

```text
$ just build # libpachinko dev-depends on pachinko
cargo build --workspace --all-features --locked
   Compiling libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/libpachinko)
   Compiling pachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/pachinko)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
Exit status: 0
```

```text
$ just build # the same reverse edge is a normal dependency
cargo build --workspace --all-features --locked
error: cyclic package dependency: package `libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/libpachinko)` depends on itself. Cycle:
package `libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/libpachinko)`
    ... which satisfies path dependency `libpachinko` (locked to 0.1.0) of package `pachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/pachinko)`
    ... which satisfies path dependency `pachinko` (locked to 0.1.0) of package `libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/crates/libpachinko)`
error: recipe `build` failed on line 100 with exit code 101
Exit status: 101
```

```text
$ git status --short # after restoring the manifest and lock
(stdout empty)
Exit status: 0
```

### Item 11: exclude-dev pairs — commit `c1b4bcb` with uncommitted scratch changes

The source probe was a local git repository containing a compilable MIT stub,
with `version = "0.1.0"` in the dependency declaration. For each section, lock
generation and `just build` exited 0. Only the source category failed for the normal
edge; the dev edge passed every deny category.

```text
$ just deny # source-normal
bans ok, licenses ok, sources FAILED
error[source-not-allowed]: detected 'git' source not explicitly allowed
3 │ t18-source-probe 0.1.0 git+file:///Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/probes/ai_tmp/dependencies/t18-source-probe
Exit status: 8
```

```text
$ just deny # source-dev
bans ok, licenses ok, sources ok
Exit status: 0
```

For the advisory pair, `time = "=0.1.44"` was added to pachinko, first as a
normal dependency, then as a dev-dependency. Lock generation and `cargo fetch --locked`
exited 0 in each case, so neither audit failed on resolution or a missing crate.

```text
$ just audit # normal time 0.1.44 dependency
advisories FAILED
error[vulnerability]: Potential segfault in the time crate
  ├ ID: RUSTSEC-2020-0071
  ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2020-0071
  ├ time v0.1.44
Exit status: 1
```

```text
$ just audit # development time 0.1.44 dependency
advisories ok
cargo deny --locked check advisories
Exit status: 0
```

```text
$ git status --short # restored source and advisory copies
(stdout empty in both copies)
Exit status: 0
```

### Item 12: std API inventory — commit `c1b4bcb`

The exact pinned documentation archive was downloaded from
`https://static.rust-lang.org/dist/rust-docs-1.99.1-aarch64-apple-darwin.tar.xz`
and the four module indexes extracted. A mechanical parser enumerated every rustdoc
`dt` item with a public struct, enum, trait, function, type, constant or module title;
items marked `stab unstable` were excluded. It read `effect:` paths from the committed
clippy configuration, printed the nonempty inventory count, and took the set difference.
Banned types cover their associated APIs; the explicit ToSocketAddrs method ban is
already present. The parser source below makes this comparison reproducible from the
four extracted `index.html` files and committed configuration. It rejects a wrong
toolchain version, an empty module or ban inventory, conflicting stability markers,
and any unexplained difference. Deprecated stable items remain included.

```python
from collections import Counter
from pathlib import Path
import re

stable, unstable = set(), set()
for module in ('fs', 'env', 'net', 'process'):
    html = Path(f'ai_tmp/t18/std-docs/{module}/index.html').read_text()
    assert 'data-rustdoc-version="1.99.1 ' in html, module
    found = set()
    for entry in re.findall(r'<dt(?: [^>]*)?>(.*?)</dt>', html, re.S):
        match = re.search(r'title="(?:struct|enum|trait|fn|type|constant|mod) (std::[^"]+)"', entry)
        if match:
            path = match[1]
            assert path.startswith(f'std::{module}::'), path
            found.add(path)
            (unstable if 'class="stab unstable"' in entry else stable).add(path)
    assert found, f'empty inventory for {module}'
config = Path('crates/libpachinko/clippy.toml').read_text()
bans = set(re.findall(r'path = "([^"]+)", reason = "effect:', config))
assert bans, 'empty ban inventory'
assert not stable.intersection(unstable), 'contradictory stability markers'
print(f'stable top-level exports: {len(stable)}')
print(dict(Counter(path.split('::')[1] for path in sorted(stable))))
print(f'unstable exports excluded: {len(unstable)}')
print('stable inventory minus effect paths:')
for path in sorted(stable - bans):
    print(path)
assert stable - bans == {'std::env::consts'}, 'unexplained inventory difference'
```

The source above was saved and executed as follows:

```text
$ python3 ai_tmp/t18/review-inventory.py
stable top-level exports: 82
{'env': 23, 'fs': 31, 'net': 13, 'process': 15}
unstable exports excluded: 8
stable inventory minus effect paths:
std::env::consts
Exit status: 0
```

`std::env::consts` contains compile-time platform constants, the documented
exception that clippy cannot ban. No other stable top-level export is missing on the
pinned host target. Eight experimental exports were excluded by their rustdoc marker;
this is not a claim that they are banned. The four requested std modules have no
remaining unexplained difference. This comparison does not claim coverage of APIs
compiled only for a Windows target, or APIs stabilized after this toolchain pin.

After the original inventory run and before the hand-back edits, the ticket worktree
was clean:

```text
$ git status
On branch ticket/t18-verification-gaps
Your branch is up to date with 'origin/main'.

nothing to commit, working tree clean
Exit status: 0
$ git status --short
(stdout empty)
Exit status: 0
```

### Item 13: coverage debugging — commit `c1b4bcb`

```text
$ just coverage
TOTAL                              25                 0   100.00%           5                 0   100.00%          17                 0   100.00%           0                 0         -
     Summary [   0.194s] 3 tests run: 3 passed, 0 skipped
Exit status: 0
```

```text
$ cargo llvm-cov report --html
Finished report saved to /Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/network/target/llvm-cov/html
Exit status: 0
```

```text
$ ls target/llvm-cov/html/index.html
target/llvm-cov/html/index.html
Exit status: 0
$ test -f target/llvm-cov/html/index.html
(stdout empty)
Exit status: 0
```

```text
$ cargo llvm-cov report --show-missing-lines
Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
libpachinko/src/lib.rs              7                 0   100.00%           2                 0   100.00%           6                 0   100.00%           0                 0         -
pachinko/src/lib.rs                13                 0   100.00%           2                 0   100.00%           8                 0   100.00%           0                 0         -
pachinko/src/main.rs                5                 0   100.00%           1                 0   100.00%           3                 0   100.00%           0                 0         -
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
TOTAL                              25                 0   100.00%           5                 0   100.00%          17                 0   100.00%           0                 0         -
Exit status: 0
```

The final command used `CARGO_TARGET_DIR=ai_tmp/t18/coverage-cold`, a newly
created, initially empty output directory. This makes the run cold with respect to
compilation and coverage data; no previous `.profraw` or object files were reused.

```text
$ INSTA_UPDATE=no cargo llvm-cov nextest --workspace --all-features --locked --html # empty CARGO_TARGET_DIR
Compiling libpachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/network/crates/libpachinko)
   Compiling pachinko v0.1.0 (/Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/network/crates/pachinko)
     Summary [   0.214s] 3 tests run: 3 passed, 0 skipped
    Finished report saved to /Users/scutting/.supacode/repos/pachinko/ticket/T18-verification-gaps/ai_tmp/t18/coverage-cold/llvm-cov/html
Exit status: 0
```

```text
$ ls "$CARGO_TARGET_DIR/llvm-cov/html/index.html"
ai_tmp/t18/coverage-cold/llvm-cov/html/index.html
Exit status: 0
$ test -f "$CARGO_TARGET_DIR/llvm-cov/html/index.html"
(stdout empty)
Exit status: 0
```

The coverage scratch clone remained clean; the scratch branch still pointed at
the tested source commit:

```text
$ git rev-parse --short HEAD
c1b4bcb
Exit status: 0
$ git status
On branch t18-lock-upgrade
nothing to commit, working tree clean
Exit status: 0
$ git status --short
(stdout empty)
Exit status: 0
```

### Restoration and review record

Before editing this ticket, `git status --short` exited 0 with empty stdout in the
network, control, tutorial and advisory scratch clones, and in the ticket worktree.
The fresh-cache clone independently reported the same after its full gate. Archive
copies were restored from their base; the restored effects run passed. After review,
all 12 scratch trees were removed, including the standalone clones and their local
branches, archive copies, isolated caches and cold coverage output. The deletion
script and all 12 absence assertions exited 0. Raw logs remain ignored.

The requested external `code-review` skill was not installed. The repository's
`review-change` skill and the §14 adversarial review cover the ticket instead.
No product test or behaviour was introduced. Temporary tests were used only for
the ticket's gate and development-cycle probes, then restored.

The final checkpoint used two independent, parallel Claude Code reviewers with
`--model claude-opus-5-5 --permission-mode plan`, one for conformance and one for
breakage. Both exited 0. Two separate fresh-context Codex subagents on
`gpt-6-astra` checked the findings and applied ticket-only fixes sequentially.
Neither reviewer wrote or triaged its own findings.

| Finding | Verdict and action |
| --- | --- |
| C-1 | Invalid as a checkpoint defect: the final gate and review record were still pending, and the Markdown typo was already corrected. Finalization is recorded below. |
| C-2 | Invalid: the ticket explicitly permits an Open disposition; no primary-checkout operation was attempted or assumed approved. |
| C-3 | Invalid: the named maintainer trials are closing events. The Linux prerequisite result was clarified without claiming all container configurations fail. |
| C-4 | Valid: named cache directories did not establish complete isolation. Item 3 is Open. |
| C-5 | Valid excerpt omission: added the failing gate's own 26.56% total and exact floor-step explanation. |
| C-6 | Valid: added the restored source-dependency control, which exits 0. |
| C-7 | Valid: added the inventory and coverage status evidence. |
| C-8 | Invalid as a checkpoint defect: cleanup was scheduled after review, and restoring generated files resets their content. Removed an unsupported lockfile assertion and labeled the diff artifact accurately. |
| C-9 | Partly valid: removed the scratch-log citation and distinguished silent-output annotations from actual output; added positive HTML file controls. Genuine absolute paths in output were retained. |
| B-1 | Valid: the reused Cargo bin symlink includes host tools as well as proxies. Corrected the description; item 3 stays Open. |
| B-2 | Partly valid: embedded and executed a fail-closed inventory parser. The actual command results support the other exit annotations; a missing JSON wrapper does not invalidate them. |
| B-3 | Valid: added direct comparisons of both restored archive copies against their bases. Git status alone cannot inspect ignored scratch copies. |
| B-4 | Valid excerpt omission, resolved with C-5. |
| B-5 | Valid: added the no-logic rustqual result and paired clippy line-count controls, including restoration. |
| B-6 | Valid: compiled and executed the development cycle in a test, then restored and reran the original suite. |
| B-7 | Invalid as a checkpoint defect: cleanup and final gate were pending at review time. Both are completed in this finalization record. |

There were no valid high-severity findings, so §14 requires no second round.
The reviewers independently confirmed the required per-ban cases, gate rejection
stages, dependency-exclusion pairs and inventory counts. All valid findings were
resolved within this ticket's scope. The five Open verification items retain their
reasons and closing events; no gate, source, dependency or owning page was changed.

### Final gate — commit `c1b4bcb` with uncommitted ticket changes

After all review fixes and scratch cleanup, the full gate passed. The only tracked
change was this ticket; the production sources, lockfiles and gate configuration
still match the tested source commit.

```text
$ just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

## Open points

Items 1, 2, 3, 4 and 7 remain Open for the reasons and closing events in the disposition
table. No other verification item was dropped.
