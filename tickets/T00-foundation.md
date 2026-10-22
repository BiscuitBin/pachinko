---
id: T00
title: "Foundation: a shape-complete skeleton that passes `just check` on its first commit"
status: done
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
has, in a valid shape, so that six lanes can then replace stubs in parallel. It has to
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
then `just check` are green, and `just lint` passes there under `CONVENTIONS.md` §11's
network denial, with
every T00 path in `CONVENTIONS.md` §3 present: final content for the frozen files, the
dotfiles, the first-run script, the licence, the manifest, the map, the adapters and the
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
- No effect bans beyond `scripts/check-effects.sh`'s crate line (T10), no final Rust tool
  configuration beyond what the gate needs (T02), no CI beyond the one-job stub (T01),
  no repository settings (T01).
- No wave-2 recipe, file or pin (`CONVENTIONS.md` §11, "Wave 2 shares files by rebasing").
- No `allium-skill-reference/` (T05), and no `skills-lock.json` (`CONVENTIONS.md` §7).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | skeleton -> T02 | `CONVENTIONS.md` §3 "The two crates" |
| `Cargo.lock` | generated | `cargo generate-lockfile`; two members |
| `rust-toolchain.toml` | final | `CONVENTIONS.md` §2, with the day's stable |
| `rustfmt.toml` | working -> T02 | L `rustfmt.toml` |
| `clippy.toml` | working -> T02 | L `clippy.toml` |
| `taplo.toml` | working -> T02 | L `taplo.toml` |
| `deny.toml` | working -> T02 | L `deny.toml` minus the `getrandom` and `rand` bans; no wasm target in `[graph]` |
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
| `scripts/initialize.sh` | final | L `scripts/initialize.sh`, with this repository's names, minus `just format` and its comment (`CONVENTIONS.md` §5) |
| `LICENSE` | final | the Apache-2.0 text from `https://www.apache.org/licenses/LICENSE-2.0.txt`, as L `LICENSE` |
| `README.md` | stub -> T12 | replaces the one-line stub |
| `CHANGELOG.md` | stub -> T12 | L `CHANGELOG.md`'s shape, one `[Unreleased]` section |
| `SECURITY.md` | stub -> T12 | 40 words |
| `AGENTS.md` | stub -> T05 | the six phrases, 300 words |
| `CLAUDE.md` | final | byte-identical to L `CLAUDE.md` |
| `.github/copilot-instructions.md` | final | byte-identical to L `.github/copilot-instructions.md` |
| `.claude/settings.json` | final | byte-identical to L `.claude/settings.json` |
| `.agents/skills/<14>/SKILL.md` | stub -> T05 | final frontmatter (`CONVENTIONS.md` §7), stub body |
| `.claude/skills/<14>/SKILL.md` | final | B's bridge body, as L's; frontmatter per `CONVENTIONS.md` §7 |
| `.codex/skills/<14>/SKILL.md` | final | B's bridge body, as L's; frontmatter per `CONVENTIONS.md` §7 |
| `Justfile` | final in wave 1 | `CONVENTIONS.md` §4 |
| `.pre-commit-config.yaml` | final | L `.pre-commit-config.yaml`, `CONVENTIONS.md` §5 |
| `.pre-commit-fix.yaml` | final | L `.pre-commit-fix.yaml` |
| `.editorconfig` | final | L `.editorconfig`, with only this repository's file types |
| `.gitattributes` | final | L `.gitattributes`'s lines for `Cargo.lock`, `pixi.lock`, `allium-skill-reference/**` |
| `.gitignore` | final | L `.gitignore`, with the `mutants.out*/` comment reworded for T15 and Claude Code's runtime state ignored |
| `.markdownlint-cli2.jsonc` | final | L `.markdownlint-cli2.jsonc`, with `important` added to `MD033`'s `allowed_elements` (`CONVENTIONS.md` §5) |
| `lychee.toml` | final | L `lychee.toml` |
| `_typos.toml` | final | L `_typos.toml`, `CONVENTIONS.md` §5 |
| `pyproject.toml` | final | `CONVENTIONS.md` §2, with comments that describe this repository |
| `pixi.lock` | generated | `pixi lock` |
| `.github/actions/setup/action.yml` | stub -> T01 | L `.github/actions/setup/action.yml`, as it stands |
| `.github/workflows/ci.yml` | stub -> T01 | one job named `check` running `just check` |
| `docs/manifest.yml` | final in wave 1 | `CONVENTIONS.md` §6, §8 |
| `docs/README.md` | final in wave 1 | `CONVENTIONS.md` §6 |
| `docs/project/*.md`, `docs/tutorials/*.md`, `docs/how-to/*.md`, `docs/explanation/*.md`, `docs/reference/*.md`, `docs/operations/*.md` | stub -> T06, T07 | one per manifest entry |
| `docs/decisions/README.md`, `docs/decisions/0001-*.md` to `0014-*.md` | stub -> T07 | `CONVENTIONS.md` §8 file names and titles |
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
   (network, authorised). The dotfiles, `scripts/initialize.sh` and the `pyproject.toml`
   comments are final here, because they freeze when this ticket merges:
   - Remove every line that serves a file type or tool this repository does not have;
     keep every comment that explains a setting this repository still uses, reworded
     where it names L. `rg -n -i 'pawdoku|sudoku|svelte|node|wasm|\.py\b'` over these
     files lists the candidates.
   - `_typos.toml`: keep `mis`; drop `als` unless `just check-docs` then fails on a file
     that needs it.
   - `.gitignore`: L's `mutants.out*/` comment says "if S03 adopts it"; reword it to name
     T15. Replace L's `.claude/settings.local.json` line with `.claude/*`, then
     `!.claude/settings.json` and `!.claude/skills/`, so that state Claude Code writes
     into a worktree mid-session (scheduled-task locks, checkpoints, nested worktrees) is
     ignored and the tracked paths are not. B's `bg-validate-agents` lists files with
     `git ls-files --cached --others --exclude-from=.gitignore` and reports any other
     file under `.claude/` as an unexpected managed file; the comment above the lines
     says so. Prove it: `touch .claude/scheduled_tasks.lock`, run `just check-agents`
     and quote it green, then remove the file. Add nothing a wave-2 ticket will add
     itself (the fuzz artefacts are T16's).
   - `.markdownlint-cli2.jsonc`: `MD033`'s `allowed_elements` becomes
     `["br", "important"]`, with a comment saying `AGENTS.md` uses `<important if>`
     blocks (`CONVENTIONS.md` §7).
   - `scripts/initialize.sh`: drop L's `just format` line and the comment above it, and
     `format` from the first comment's list of recipes (`CONVENTIONS.md` §5). Keep the
     closing message's statement that nothing was staged,
     committed, tagged or pushed, and add that nothing was formatted.

5. **The Justfile and hooks.** Write the `Justfile` as `CONVENTIONS.md` §4 states it: L's
   text with the listed changes and no others. Then `just install-tools` (network;
   rustqual may compile from source, which L's decision 0013 accepts) and
   `just install-allium` (network). Copy both prek configurations verbatim. Do not run
   `just install-hooks`: its `prek install` writes the hook shim into the Git directory
   every worktree shares, which L `scripts/initialize.sh` refuses to do from a secondary
   worktree. Run the recipe's other lines instead (network, authorised):

   ```sh
   .pixi/envs/default/bin/prek prepare-hooks
   .pixi/envs/default/bin/prek prepare-hooks --config .pre-commit-fix.yaml
   RUSTUP_AUTO_INSTALL=0 .pixi/envs/default/bin/prek run lychee --files README.md || true
   ```

   The last line ends in `|| true` as the recipe's `-` prefix does. The primary checkout
   installs the shim after this ticket merges (`README.md`, "How to pick up a ticket").

6. **`scripts/check-effects.sh`.** POSIX `sh`, `set -eu`. It runs
   `cargo tree -p libpachinko -e normal --prefix none --locked --offline`, fails if any
   line names a crate `CONVENTIONS.md` §9 bans, and prints one line saying the std-API
   bans and the fixture probe arrive with T10. This is the whole of gate 8 until T10.

   **Adversarial review** (`CONVENTIONS.md` §14): the files that freeze when this ticket
   merges: `rust-toolchain.toml`, the `Justfile`, both prek configurations, `tools.txt`,
   `tools-source.txt`, `crates/pachinko/src/main.rs`, `pyproject.toml`, the six dotfiles
   of step 4 and `scripts/initialize.sh`, with `scripts/check-effects.sh`, which T10
   builds on. Ask: which delta from L goes beyond `CONVENTIONS.md` §2, §4 and §5; which
   recipe a lane will need changed after the freeze; whether the effects script can pass
   while libpachinko depends on a banned crate.

7. **The specification.** Write `docs/specs/pachinko.allium` (`CONVENTIONS.md` §9). Run
   `just check-specs` and `just analyse-specs`; this is `CONVENTIONS.md` §12 claim 1. If
   allium rejects a module with no declarations, apply §9's fallback and record it.

8. **The handbook.** Write `docs/manifest.yml` from L `docs/manifest.yml`: drop the
   entries `CONVENTIONS.md` §3 lists as not carried; renumber and retitle the decision
   entries as §8's table, with its slugs; keep every other entry's slug. Write
   `docs/README.md` from L `docs/README.md` (`CONVENTIONS.md` §6). Then one stub per
   manifest entry, as L `tickets/T00-foundation.md` step 10 describes stubs.

9. **The agent contract.** `AGENTS.md` stub: the headings `CONVENTIONS.md` §7 names
   (`Project map`, `Invariants`, `Provenance`), the six phrases of `CONVENTIONS.md` §1
   fact 2 each in a real sentence outside any `<important if>` block, the runes sentence
   in Provenance, 300 or more words. Fourteen skill stubs with the final frontmatter of
   `CONVENTIONS.md` §7 (L's, except `rust-change`'s description; each description states
   a trigger) and a body that cites `AGENTS.md` and one `just` recipe. Twenty-eight
   bridges copied from L, with `rust-change`'s two carrying the new description, because
   the bridges freeze here and repeat the canonical frontmatter. The three adapter files
   copied byte-for-byte.

10. **The root documents and CI stub.** `README.md`, `CHANGELOG.md`, `SECURITY.md`, both
    crate READMEs as stubs; `LICENSE`. `.github/workflows/ci.yml`: one job, `name: check`,
    using L's setup action (`cache-key: check`), then `just install-allium`, then
    `just check`, so that the check name branch protection requires exists from the first
    push (`CONVENTIONS.md` §10). The setup action does not install allium (L
    `.github/actions/setup/action.yml`, its description), and `check-specs`,
    `analyse-specs` and the hook gate in `lint` need it.

11. **The gate.** `just check`. Then the claims T00 owns first: `CONVENTIONS.md` §12
    claims 4, 10, 11 and 12. For claim 4, run `cargo llvm-cov report --show-missing-lines`
    and confirm `crates/pachinko/src/main.rs` reports no missing line; if it does, apply
    `CONVENTIONS.md` §4's `--ignore-filename-regex` fallback now, before the Justfile
    freezes. For claim 12, run `just effects` under `CONVENTIONS.md` §11's network
    denial, with its negative control; run `just lint` the same way, which step 12
    repeats in the fresh clone.

    **Adversarial review** (`CONVENTIONS.md` §14): the whole tree, before step 12 commits
    it. Ask: which acceptance criterion is asserted but not shown; whether the evidence
    for `CONVENTIONS.md` §12 claims 1, 3, 4, 10, 11 and 12 proves each claim; whether the
    adapters, the bridges and `.claude/settings.json` are byte-identical to L's, except
    the `rust-change` bridges' description line (`CONVENTIONS.md` §7); which
    stub has fewer than 40 real words; what the acceptance criteria's `pawdoku`, `sudoku`
    and `wasm` grep prints; which line step 4 removed from the dotfiles that something in
    this repository, a later lane or a wave-2 ticket still needs; whether the offline
    `just lint` ran under a denial its negative control proved, or passed only because a
    cache was warm.

12. **A fresh clone, offline.** Commit, clone the worktree into `ai_tmp/fresh`, and run
    `just initialize`, then `just lint` under `CONVENTIONS.md` §11's network denial, with
    its negative control, then `just check` there, so that nothing green
    depends on a file outside Git. Then the secondary worktree, with the fresh clone as
    its primary: `git -C ai_tmp/fresh worktree add ../fresh-2`, then `just initialize` in
    `ai_tmp/fresh-2`, which skips `install-hooks` there, and `just lint` works without a
    second `install-hooks`, as L `scripts/initialize.sh`'s comment claims. Delete
    `ai_tmp/fresh` and `ai_tmp/fresh-2` afterwards.

## Acceptance criteria

- `just check` prints all nineteen gates of `CONVENTIONS.md` §4 and `check-clean`, green,
  in the worktree and in the fresh clone.
- `just lint` passes in the fresh clone after `just initialize`, under
  `CONVENTIONS.md` §11's network denial with both negative-control probes failing, and
  in the secondary worktree without a second `install-hooks`.
- `scripts/initialize.sh` does not contain the word `format`, and `just check-agents` stays green
  with an untracked `.claude/scheduled_tasks.lock` present.
- `rustup show active-toolchain` prints the pin; `rust-version` matches its minor.
- `cargo nextest run --workspace --locked` runs three or more tests; `cargo test --doc
  --workspace --locked` runs one or more doctests; `just coverage` reports 90% or more.
- `just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`
- `just check-specs` and `just analyse-specs` each print one block with empty
  `diagnostics` and `findings`.
- `grep -rn -i 'pawdoku\|sudoku\|wasm' --exclude-dir=.git --exclude-dir=tickets --exclude-dir=allium-skill-reference --exclude-dir=.pixi --exclude-dir=target --exclude-dir=.tools .`
  prints nothing, or only lines the hand-back notes justify (a carried record's
  "Carried from libpawdoku" opening is one).
- Step 4's `rg` prints nothing over the dotfiles, `scripts/initialize.sh` and
  `pyproject.toml`, or only lines the hand-back notes justify.
- Every path in the Files-touched table exists; no path outside it was created.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

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
touch .claude/scheduled_tasks.lock && just check-agents; echo "exit $?"; rm .claude/scheduled_tasks.lock
# macOS; on Linux, unshare -rn in place of sandbox-exec (CONVENTIONS.md §11)
deny='(version 1)(allow default)(deny network*)'
sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org; echo "exit $?"
sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1; echo "exit $?"
sandbox-exec -p "$deny" just effects; echo "exit $?"
sandbox-exec -p "$deny" just lint; echo "exit $?"
rg -n -w 'format' scripts/initialize.sh; echo "exit $?"   # must be 1
rg -n -i 'pawdoku|sudoku|svelte|node|wasm|\.py\b' .editorconfig .gitattributes .gitignore .markdownlint-cli2.jsonc lychee.toml _typos.toml scripts/initialize.sh pyproject.toml
git status --short
```

Quote each command's output in the hand-back notes, and the fresh clone's offline
`just lint` and `just check`, and the secondary worktree's `just lint`.

## Hand-back notes

Evidence below ran at `09401de` (the skeleton commit). The commit that adds these notes
changes only this file.

### Toolchain

- Pin: `channel = "1.99.1"`, the stable rustup reported on 2026-10-21 local time
  (2026-10-22 UTC): `info: latest update on 2026-10-15 for version 1.99.1 (58cf8f9c3
  2026-10-13)`. `rust-version = "1.99"` matches its minor. `rustup show
  active-toolchain` prints `1.99.1-aarch64-apple-darwin (overridden by
  '…/rust-toolchain.toml')`, exit 0.
- Installing the pin also self-updated rustup from 1.29.1 to 1.29.2. That happened
  outside the worktree and changes nothing in it.
- Lints the new stable forced: one. `clippy::assert_is_empty` is new in 1.99 and fired
  on `assert!(output.stderr.is_empty())` in `crates/pachinko/tests/binary.rs`. I changed
  the assertion to `assert_eq!(output.stderr, b"")`; the lint table is unchanged. L's
  tree has `assert!(… .is_empty())` sites the lint may flag (for example
  `crates/pawdoku/tests/solver.rs:202` at `ccdd001`); I did not run clippy over L, so
  that is for T02 to check (claim 3).

### §12 claims this ticket checked

| # | Outcome | Evidence |
| --- | --- | --- |
| 1 | Holds. No fallback declaration needed. | `just check-specs` and `just analyse-specs` each print one block with `"diagnostics": []` and `"findings": []`, exit 0. |
| 3 | Holds on T00's tree after the one assertion change above. L's full tree on 1.99.1 is unchecked (T02). | `just clippy` inside `just check`, exit 0. |
| 4 | Holds. `main.rs` counts as covered; no `--ignore-filename-regex` needed. | `cargo llvm-cov report --show-missing-lines`: `TOTAL … 17 0 100.00%` lines, no missing lines, exit 0. |
| 10 | Holds. | `just snapshots-check`: `info: no unreferenced snapshots found`, `info: no snapshots to review`, exit 0. |
| 11 | Holds. | `just metrics`: `Quality score: 100.0% (2 functions analyzed)` and `(4 functions analyzed)`, exit 0. |
| 12 | Holds. `just effects` and `just lint` run with the network denied. The review found a gap in the prescribed command (below). | See Verification and Review records. |

### Deviations from L beyond the stated deltas

- `tools-source.txt:6` and `Justfile:44` (the `install-tools` comment) say "decision
  0012" where L says "decision 0013". L's 0013 is the metrics gate, which §8 renumbers to
  0012, and pachinko's 0013 is the API reference on Pages. A verbatim copy would cite the
  wrong record. §2 says "verbatim" and §4 "exactly these changes"; a `main` edit to
  CONVENTIONS could name renumbered cross-references as an implied delta.
- `deny.toml`: `taplo fmt` folded `targets` and `deny` onto one line each once the
  removals shortened them; `just toml-check` fails otherwise.
- `scripts/initialize.sh`: beyond dropping `just format` and its comment, the first
  comment loses "format" and "taplo" from its list of what needs the environment and is
  rewrapped, and the closing message reads "Nothing has been formatted, staged,
  committed, tagged, or pushed." so a first run says it rewrote nothing.
- `.gitignore`: the new `.claude/*` comment ends with a sentence that explains the
  `.codex/settings.local.json` line it keeps.
- `_typos.toml` keeps `als = "als"` under a new comment. Step 4 drops it unless a check
  fails without it, and one does: `typos` rewrites `als` to `also` in
  `tickets/CONVENTIONS.md:341` and `tickets/T00-foundation.md:146`. The first `just lint`
  run did exactly that; I restored both files with `git checkout --` and kept the entry.
  The rewrite came from the `typos` hook inheriting upstream's `--write-changes`; the
  hook now passes `args: [--force-exclude]` (`CONVENTIONS.md` §5), so typos only
  reports. The entry stays, because typos still reports `als` without it.
- `AGENTS.md` Provenance points to `tickets/CONVENTIONS.md` instead of naming the
  upstream library, so the acceptance grep stays clean.
- New decision records 0001, 0008 and 0014 carry audiences chosen here: 0001
  `[maintainer, agent]`; 0008 and 0014 `[contributor, maintainer, agent]`.
- Each carried record's "Carried from libpawdoku's decision NNNN at `ccdd001`." line is
  italic. These are the 11 lines the acceptance grep prints.

### Lines removed from L's dotfiles

- `.editorconfig`: the `[*.py]` and `[Makefile]` sections. This repository has no Python
  source and no Makefile.
- `.gitignore`: `__pycache__/`, `*.py[cod]` and `.ruff_cache/`, for the same reason.
  L's `.claude/settings.local.json` line and its comment are replaced by `.claude/*`,
  `!.claude/settings.json` and `!.claude/skills/`. Claude Code writes other runtime
  files there mid-session, such as `scheduled_tasks.lock`, and `bg-validate-agents`
  reports any untracked, unignored file under `.claude/` as unexpected. The cargo-mutants
  comment names T15 where L named S03.
- `deny.toml`: the `getrandom` and `rand` bans (§1 fact 3: the effects gate is the only
  crate guard) and the `wasm32-unknown-unknown` target. The comment that cited proptest
  is reworded.
- `rustqual.toml`: L's `[[architecture.pattern]]` tables, which named L's modules, and
  "(step 6)", a step in L's ticket. The header names both crates.

Added, not removed: `.markdownlint-cli2.jsonc` allows `important` as well as `br`, with a
comment, for the `<important if>` blocks T05's `AGENTS.md` uses.

Step 4's `rg` over the dotfiles, `scripts/initialize.sh` and `pyproject.toml` prints
nothing, exit 1; the same command with `|als` added matches `_typos.toml:21-23`, exit 0,
as a positive control.

### Review records (§14)

Reviewers: Codex CLI 0.163.0, `codex exec -m gpt-6-astra -s read-only`, one run per
lens. Triage: five fresh-context Claude Opus 5.5 subagents, one per merged finding.
Neither the reviewers nor the triagers wrote the change. The first launch hung because
`codex exec` read stdin from the background shell. It ran again with `</dev/null`,
with the same briefs.

Checkpoint 1 (step 6, the frozen files and the effects script), at `09401de`:

- F1, medium (BR-01, C2, C3). Valid. `scripts/check-effects.sh` runs the exact §9 line,
  `cargo tree -p libpachinko -e normal --prefix none --locked --offline`. That resolves
  default features on the host target only, so an optional banned dependency, or one
  under `cfg(windows)`, passes gate 8. A triager reproduced this offline with path stubs:
  the prescribed line lists only libpachinko, exit 0, and adding `--all-features --target
  all` lists `rand` and `socket2`, exit 0. Nothing is exposed today because libpachinko
  has no dependencies. **Handed back**, not applied. §9 prescribes the line and gives the
  mechanism to T10. CONVENTIONS §9 (the crate line and the fixture-probe line) and §12
  claim 12 should read `cargo tree -p libpachinko -e normal --all-features --target all
  --prefix none --locked --offline`, through a `main` pull request. T10 then applies it
  to the script and adds an optional banned stub and a `cfg(windows)` banned stub to
  `tests/fixtures/effects-violation/`.
- F2, low (C1). Valid as a record. The 0013→0012 change stays, recorded under
  Deviations.
- Later-lane recipes: no findings from either lens.

Checkpoint 2 (step 11, the whole tree), at `09401de`:

- F3, medium (B01, C2-01). Valid: these notes were still the placeholder. Fixed by this
  section.
- F4, low (C2-02). Valid. The saved grep evidence added `--exclude-dir=ai_tmp`, so it was
  not the acceptance command. Fixed: the acceptance grep below runs exactly as written in
  the fresh clone, which has no `ai_tmp/`.
- F5, low (B02). Valid. Two scratch evidence lines piped into `tail` or `grep -c` and
  recorded the filter's exit status. Fixed: every command quoted here ran unpiped, with
  its own exit code.
- Adapters, bridges, `.claude/settings.json`, stub word counts, removed dotfile lines,
  the Files-touched inventory and §3 conformance: no findings from either lens.

No finding was high, so no second round ran. Raw files:
`ai_tmp/review/T00-{1,2}/findings-*.md` and `verdicts.md`, uncommitted.

### Hooks

The hook shim is not installed: this is a secondary worktree, and step 5 forbids
`just install-hooks` here. The primary checkout installs it once after merge.

### Handed back

- To T01: `.github/actions/setup/action.yml:80` still says "PREK_HOME is the variable
  T03 confirms", where T03 is L's ticket id.
- The `install-toolchain` comment in the Justfile and the comment in
  `scripts/initialize.sh` still say "components and targets", though
  `rust-toolchain.toml` lists no targets. Both files are frozen. A later owner may drop
  "and targets".
- To T05: `AGENTS.md` says the network recipes "include" its list; T05 makes it exact.
- To T05: invariant 4 reads "`#![forbid(unsafe_code)]` in every crate", as L's does.
  Like L, the crates get it from the workspace lint table (`unsafe_code = "forbid"`) and
  carry no crate attribute. T05 either words the invariant that way or the crate roots
  gain the attribute.
- To CONVENTIONS and T10: F1 above.
- To T10: the `effects` recipe comment (§4's text, verbatim) describes T10's script:
  source checks and the `tests/fixtures/effects-violation` probe. T00's script checks
  crate names only, and says so when it runs.
- To T01: `.github/actions/setup/action.yml` (L's comment) says no step reaches the
  network after `just install-hooks`, but the stub `ci.yml` runs `just install-allium`
  next. The stub's `concurrency` also cancels in-progress runs on `main`. T01 replaces
  `ci.yml`.

### Code review

The implement skill's `/code-review` at medium effort over `main...HEAD` and these notes
returned nine findings. Two are the hand-backs above, the `effects` comment and the
forbid wording, and a third is F1 again. The CI findings are handed to T01 above. The
rest were not applied, for these reasons:

- The tests assert the literal `0.1.0` on purpose. An expected value taken from
  `env!("CARGO_PKG_VERSION")` would be tautological, and a version bump that breaks four
  tests is the intended signal.
- `main.rs` turning a write error into a bare `ExitCode::FAILURE` is the shape §3
  prescribes.
- The `check-toolchain` path comparison and the ripsecrets hook are byte-for-byte L's,
  and both files freeze at T00.

Copilot's review of PR #2 found that the `typos` hook inherited upstream's
`--write-changes`, so the read-only gate could rewrite files. Fixed: the hook now passes
`args: [--force-exclude]`, named as a delta in `CONVENTIONS.md` §5.

### Verification

All at `09401de`. Scratch logs: `ai_tmp/evidence/verify-block-09401de.txt` (this worktree)
and `ai_tmp/evidence/step12-09401de.txt` (fresh clone). Each command ran unpiped and its
exit code was recorded right after it.

This worktree, the Verification block in order:

| Command | Exit | Output |
| --- | --- | --- |
| `rustup show active-toolchain` | 0 | `1.99.1-aarch64-apple-darwin (overridden by '…/rust-toolchain.toml')` |
| `just check` | 0 | 19 gates and check-clean; last line `All checks passed and the worktree is unchanged.` |
| `cargo nextest run --workspace --locked` | 0 | `3 tests run: 3 passed, 0 skipped` |
| `cargo test --doc --workspace --locked` | 0 | `test result: ok. 1 passed; 0 failed` (libpachinko) |
| `cargo llvm-cov report --show-missing-lines` | 0 | `TOTAL … 17 0 100.00%` lines; no missing lines |
| `just check-specs`, `just analyse-specs` | 0, 0 | one block each, `"diagnostics": []`, `"findings": []` |
| `just check-agents` | 0 | `Validated AGENTS.md, 2 adapters, and 14 skills.` |
| the same with `.claude/scheduled_tasks.lock` present | 0 | the same line |
| `sandbox-exec -p "$deny" curl … https://static.rust-lang.org` | 6 | `curl: (6) Could not resolve host: static.rust-lang.org` |
| `sandbox-exec -p "$deny" curl --noproxy '*' … https://1.1.1.1` | 7 | `curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server` |
| `sandbox-exec -p "$deny" just effects` | 0 | `effects: libpachinko's normal dependency tree names no banned crate.` |
| `sandbox-exec -p "$deny" just lint` | 0 | 23 hooks `Passed`, none `Failed` |
| `rg -n -w 'format' scripts/initialize.sh` | 1 | nothing |
| step 4's `rg` | 1 | nothing (positive control above) |
| `git status --short` | 0 | nothing |

Fresh clone of `ticket/t00-foundation` at `09401de` into a scratch directory:

- `just initialize`: exit 0, ends `Ready. Next: just check.` It installed the hook in
  the clone, which is that clone's primary checkout. Two earlier attempts exited 2
  because the recording shell put a global pixi `cargo` ahead of rustup's proxy;
  `check-toolchain` refused it, as it should. A clean shell fixed it.
- Network denied: the two curl probes exited 6 and 7 with the messages above; the same
  `1.1.1.1` probe without the sandbox exited 0. `sandbox-exec -p "$deny" just lint` exited
  0 with all 23 hooks `Passed`, lychee included. `sandbox-exec -p "$deny" just effects`
  exited 0.
- `just check`: exit 0, last line `All checks passed and the worktree is unchanged.`
  `git status --short` printed nothing; a touched probe file showed as `??` and
  disappeared on removal.
- `cargo llvm-cov report --show-missing-lines`: exit 0, `TOTAL … 17 0 100.00%`.
- The acceptance grep, copied verbatim from the criteria and run from the clone's root,
  exited 0 and printed exactly 11 lines, each line 11 of a record under `docs/decisions/`
  reading "Carried from libpawdoku's decision NNNN at ccdd001", in italics, for records
  0002 to 0007 and 0009 to 0013.
  These are the carried-record openings the criterion allows. Nothing else matched.
- Secondary worktree (`git worktree add ../fresh-2` from the clone): `just initialize`
  exited 0 and printed `Secondary worktree: skipping install-hooks.`; `just lint` exited
  0 with every hook `Passed`. The clone's hook file had the same checksum and mtime
  before and after, so no second install happened.
- Both scratch checkouts were then removed; `git worktree list` no longer shows them.

## Open points

- If allium rejects an empty module, the fallback declaration is recorded here and in
  `CONVENTIONS.md` §9 by a `main` pull request.
- If the new stable's lint set forces more than a handful of table changes, say so: the
  maintainer may prefer to pin the stable L uses and bump it as a follow-up.
