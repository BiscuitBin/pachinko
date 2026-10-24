---
id: T18
title: "Verification gaps: the checks wave 0 and wave 1 never ran, run or recorded as open"
status: open
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

To be written by the executing agent: one row per item (Item, Disposition, Evidence or
reason), the quoted evidence, and the review record (`CONVENTIONS.md` §14).

## Open points

None.
