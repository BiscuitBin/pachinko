---
id: T00
title: "Foundation: a shape-complete skeleton that passes `just check` on its first commit"
status: open
depends_on: []
parallel_with: []
branch: ticket/t00-foundation
estimated_size: XL
---

# T00: Foundation: a shape-complete skeleton that passes `just check` on its first commit

## Context

This repository (`BiscuitBin/pachinko`, one `first commit` holding a stub `README.md`,
plus `tickets/`) becomes the two-crate workspace `CONVENTIONS.md` §0 describes. Read
`CONVENTIONS.md` in full first, then `README.md` in this directory.

This is the tracer bullet: one commit that passes through every layer the foundation
has, in a valid shape, so that ten lanes can then replace stubs in parallel. It has to
be one ticket because B's `bg-validate-docs` and `bg-validate-agents` walk every
registered page and every skill, and `bg-project-check` snapshots the worktree between
recipes: `just check` is green only if every path exists from the first commit.

Almost everything is a copy of L at `ccdd001` with a stated delta. L's own foundation
ticket, L `tickets/T00-foundation.md`, is the procedure this one follows; read its Steps
and Hand-back notes for the order of work and the traps it met. The difference is the
baseline: copy L's files as they are at `ccdd001`, not as L's T00 first wrote them
(`CONVENTIONS.md` §1 fact 5), because L's gate grew afterwards.

Preconditions, stated and not performed (`CONVENTIONS.md` §2): rustup with
`~/.cargo/bin/cargo` first on `PATH`, pixi 0.81.0 or newer, any just, gh. If one is
missing, stop and report it.

## Goal

A first commit on `ticket/t00-foundation` after which, in a fresh clone, `just initialize`
then `just check` are green, with every T00 path in `CONVENTIONS.md` §3 present: final
content for the frozen files, the licence, the manifest, the map, the adapters and the
bridges; a placeholder specification module; and a stub for every page, record, skill,
root document and working configuration a lane will replace. `cargo nextest run` runs at
least three tests (libpachinko's unit test, pachinko's unit test, the binary test),
`cargo test --doc` runs at least one doctest, and `just coverage` is at or above 90%.

## Non-goals

- No domain code, domain prose or behavioural specification (`CONVENTIONS.md` §1,
  decision 1).
- No dependency beyond `pachinko`'s path dependency on `libpachinko`; no
  `[workspace.dependencies]` (`CONVENTIONS.md` §1 fact 6).
- No effect trait; they arrive with the specifications.
- No final prose. Pages, records, skills, `AGENTS.md`, `README.md`, `CHANGELOG.md`,
  `SECURITY.md` and both crate READMEs are stubs of 40 or more real words that say what
  the file will contain and name the lane that writes it.
- No effect bans beyond `scripts/check-effects.sh`'s crate line (T10), no final tool
  configuration beyond what the gate needs (T02, T03), no CI beyond the one-job stub
  (T04), no repository settings (T01).
- No wave-2 recipe, file or pin (`CONVENTIONS.md` §11, "Wave 2 shares files by rebasing").
- No `skills-lock.json` and no `allium-skill-reference/` (T05).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | skeleton -> T02 | `CONVENTIONS.md` §3 "The two crates" |
| `Cargo.lock` | generated | `cargo generate-lockfile`; two members |
| `rust-toolchain.toml` | final | `CONVENTIONS.md` §2, with the day's stable |
| `rustfmt.toml` | working -> T02 | L `rustfmt.toml` |
| `clippy.toml` | working -> T02 | L `clippy.toml` |
| `taplo.toml` | working -> T02 | L `taplo.toml` |
| `deny.toml` | working -> T02 | L `deny.toml`; `wrappers` name `pachinko`; no wasm target in `[graph]` |
| `.config/nextest.toml` | working -> T02 | L `.config/nextest.toml` |
| `rustqual.toml` | working -> T10 | L `rustqual.toml` minus every `[[architecture.pattern]]` table and the comments about them |
| `tools.txt` | final in wave 1 | L `tools.txt`, verbatim |
| `tools-source.txt` | final in wave 1 | L `tools-source.txt`, verbatim |
| `crates/libpachinko/Cargo.toml` | skeleton -> T02 | `CONVENTIONS.md` §3 |
| `crates/libpachinko/src/lib.rs` | skeleton -> T02 | `CONVENTIONS.md` §3: crate docs, `version()`, doctest, unit test |
| `crates/libpachinko/README.md` | stub -> T12 | 40 words |
| `crates/pachinko/Cargo.toml` | skeleton -> T02 | `CONVENTIONS.md` §3 |
| `crates/pachinko/src/lib.rs` | skeleton -> T02 | `run()` and its unit test |
| `crates/pachinko/src/main.rs` | final | the one-line main |
| `crates/pachinko/tests/binary.rs` | skeleton -> T02 | runs the built binary |
| `crates/pachinko/README.md` | stub -> T12 | 40 words |
| `scripts/check-effects.sh` | working -> T10 | step 6 |
| `scripts/initialize.sh` | working -> T03 | L `scripts/initialize.sh`, renamed references only |
| `LICENSE` | final | the Apache-2.0 text from `https://www.apache.org/licenses/LICENSE-2.0.txt`, as L `LICENSE` |
| `README.md` | stub -> T12 | replaces the one-line stub |
| `CHANGELOG.md` | stub -> T12 | L `CHANGELOG.md`'s shape, one `[Unreleased]` section |
| `SECURITY.md` | stub -> T12 | 40 words |
| `AGENTS.md` | stub -> T05 | the six phrases, 300 words |
| `CLAUDE.md` | final | byte-identical to L `CLAUDE.md` |
| `.github/copilot-instructions.md` | final | byte-identical to L `.github/copilot-instructions.md` |
| `.claude/settings.json` | final | byte-identical to L `.claude/settings.json` |
| `.agents/skills/<14>/SKILL.md` | stub -> T05 | L's frontmatter, stub body |
| `.claude/skills/<14>/SKILL.md` | final | B's bridge body, as L's |
| `.codex/skills/<14>/SKILL.md` | final | B's bridge body, as L's |
| `Justfile` | final in wave 1 | `CONVENTIONS.md` §4 |
| `.pre-commit-config.yaml` | final | L `.pre-commit-config.yaml` |
| `.pre-commit-fix.yaml` | final | L `.pre-commit-fix.yaml` |
| `.editorconfig` | working -> T03 | L `.editorconfig` |
| `.gitattributes` | working -> T03 | L `.gitattributes` |
| `.gitignore` | working -> T03 | L `.gitignore` |
| `.markdownlint-cli2.jsonc` | working -> T03 | L `.markdownlint-cli2.jsonc` |
| `lychee.toml` | working -> T03 | L `lychee.toml` |
| `_typos.toml` | working -> T03 | L `_typos.toml`, `CONVENTIONS.md` §5 |
| `pyproject.toml` | final pins -> T03 comments | `CONVENTIONS.md` §2 |
| `pixi.lock` | generated | `pixi lock` |
| `.github/actions/setup/action.yml` | stub -> T04 | L `.github/actions/setup/action.yml`, as it stands |
| `.github/workflows/ci.yml` | stub -> T04 | one job named `check` running `just check` |
| `docs/manifest.yml` | final in wave 1 | `CONVENTIONS.md` §6, §8 |
| `docs/README.md` | final in wave 1 | `CONVENTIONS.md` §6 |
| `docs/project/*.md`, `docs/tutorials/*.md`, `docs/how-to/*.md`, `docs/explanation/*.md`, `docs/reference/*.md`, `docs/operations/*.md` | stub -> T06, T07, T08 | one per manifest entry |
| `docs/decisions/README.md`, `docs/decisions/0001-*.md` to `0014-*.md` | stub -> T09 | `CONVENTIONS.md` §8 file names and titles |
| `docs/specs/pachinko.allium` | placeholder -> T06 | `CONVENTIONS.md` §9 |
| `tickets/T00-foundation.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree (`README.md`, "How to pick up a ticket") and verify the
   preconditions as L `tickets/T00-foundation.md` step 1 does.

2. **The toolchain.** Read the newest stable from
   `https://static.rust-lang.org/dist/channel-rust-stable.toml` and write
   `rust-toolchain.toml` as `CONVENTIONS.md` §2 prints it. Run `rustup toolchain install`
   from the worktree (network; authorised as the one install this ticket performs) and
   confirm `rustup show active-toolchain` prints the pin.

3. **The workspace.** Write both crates as `CONVENTIONS.md` §3 describes, then
   `cargo generate-lockfile`. Run `cargo clippy --workspace --all-targets --all-features
   -- -D warnings` now, before anything else exists: a lint the new stable added or
   renamed shows here, and the fix is a lint-table change recorded in the hand-back notes
   for T02 (`CONVENTIONS.md` §12 claim 3). `#[expect]` with a reason is the only allowed
   suppression.

4. **Tool configuration.** Copy the root configuration files the table names from L,
   applying only the stated deltas. Write `tools.txt`, `tools-source.txt` and
   `pyproject.toml` (`CONVENTIONS.md` §2), then `pixi lock` and `pixi install --frozen`
   (network, authorised). Then `just install-tools` (network; rustqual may compile from
   source, which L's decision 0013 accepts) and `just install-allium` (network).

5. **The Justfile and hooks.** Write the `Justfile` as `CONVENTIONS.md` §4 states it: L's
   text with the listed changes and no others. Copy both prek configurations verbatim.
   `just install-hooks` (network, authorised).

6. **`scripts/check-effects.sh`.** POSIX `sh`, `set -eu`. It runs
   `cargo tree -p libpachinko -e normal --prefix none --locked --offline`, fails if any
   line names a crate `CONVENTIONS.md` §9 bans, and prints one line saying the std-API
   bans and the fixture probe arrive with T10. This is the whole of gate 8 until T10.

7. **The specification.** Write `docs/specs/pachinko.allium` (`CONVENTIONS.md` §9). Run
   `just check-specs` and `just analyse-specs`; this is `CONVENTIONS.md` §12 claim 1. If
   allium rejects a module with no declarations, apply §9's fallback and record it.

8. **The handbook.** Write `docs/manifest.yml` from L `docs/manifest.yml`: drop the
   entries `CONVENTIONS.md` §3 lists as not carried; renumber and retitle the decision
   entries as §8's table, with its slugs; keep every other entry's slug. Write
   `docs/README.md` from L `docs/README.md` (`CONVENTIONS.md` §6). Then one stub per
   manifest entry, as L `tickets/T00-foundation.md` step 10 describes stubs.

9. **The agent contract.** `AGENTS.md` stub: L `AGENTS.md`'s section headings, the six
   phrases of `CONVENTIONS.md` §1 fact 2 each in a real sentence, the runes sentence in
   Provenance, 300 or more words. Fourteen skill stubs with L's frontmatter (the
   description states a trigger) and a body that cites `AGENTS.md` and one `just` recipe.
   Twenty-eight bridges copied from L. The three adapter files copied byte-for-byte.

10. **The root documents and CI stub.** `README.md`, `CHANGELOG.md`, `SECURITY.md`, both
    crate READMEs as stubs; `LICENSE`. `.github/workflows/ci.yml`: one job, `name: check`,
    using L's setup action and running `just check`, so that the check name branch
    protection requires exists from the first push (`CONVENTIONS.md` §10).

11. **The gate.** `just check`. Then the claims T00 owns first: `CONVENTIONS.md` §12
    claims 4, 10, 11 and 12. For claim 4, run `cargo llvm-cov report --show-missing-lines`
    and confirm `crates/pachinko/src/main.rs` reports no missing line; if it does, apply
    `CONVENTIONS.md` §4's `--ignore-filename-regex` fallback now, before the Justfile
    freezes. For claim 12, run `just effects` with
    `HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9`.

12. **A fresh clone.** Commit, clone the worktree into `ai_tmp/fresh`, and run
    `just initialize` then `just check` there, so that nothing green depends on a file
    outside Git. Delete `ai_tmp/fresh` afterwards.

## Acceptance criteria

- `just check` prints all nineteen gates of `CONVENTIONS.md` §4 and `check-clean`, green,
  in the worktree and in the fresh clone.
- `rustup show active-toolchain` prints the pin; `rust-version` matches its minor.
- `cargo nextest run --workspace --locked` runs three or more tests; `cargo test --doc
  --workspace --locked` runs one or more doctests; `just coverage` reports 90% or more.
- `just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`
- `just check-specs` and `just analyse-specs` each print one block with empty
  `diagnostics` and `findings`.
- `grep -rn -i 'pawdoku\|sudoku\|wasm' --exclude-dir=.git --exclude-dir=tickets --exclude-dir=allium-skill-reference --exclude-dir=.pixi --exclude-dir=target --exclude-dir=.tools .`
  prints nothing, or only lines the hand-back notes justify (a carried record's
  "Carried from libpawdoku" opening is one).
- Every path in the Files-touched table exists; no path outside it was created.

## Verification

```sh
rustup show active-toolchain
just check
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked
cargo llvm-cov report --show-missing-lines
just check-specs
just analyse-specs
just check-agents
HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 just effects
git status --short
```

Quote each command's output in the hand-back notes, and the fresh-clone `just check`.

## Hand-back notes

To be written by the executing agent: the toolchain pin and the date it was read; any
lint the new stable forced, and the change made; the outcome of each `CONVENTIONS.md` §12
claim this ticket checked (1, 3, 4, 10, 11, 12); every deviation from L's files beyond
the stated deltas, and why; and anything handed back to a lane.

## Open points

- If allium rejects an empty module, the fallback declaration is recorded here and in
  `CONVENTIONS.md` §9 by a `main` pull request.
- If the new stable's lint set forces more than a handful of table changes, say so: the
  maintainer may prefer to pin the stable L uses and bump it as a follow-up.
