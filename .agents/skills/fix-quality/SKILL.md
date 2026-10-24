---
name: fix-quality
description: Diagnose and repair a failing quality gate at its root instead of suppressing the finding.
---

# Fix a failing gate

1. Read `AGENTS.md`, then reproduce with the narrowest recipe rather than the whole gate. `just check` names the recipe that failed; run that one alone.
2. Dispatch on which gate failed:
   - `check-toolchain` — cargo is not rustup's proxy, or rustup is missing. Put `~/.cargo/bin` first on `PATH`; the pin in `rust-toolchain.toml` is honoured only through that proxy.
   - `lock-check` — run `just lock`, which reaches the network and so needs explicit authorization, and read the lockfile diff before accepting it.
   - `lint` — a pre-commit hook failed. A hook named after a gate (`fmt-check`, `toml-check`, `deps-unused`, `validate-docs` for `check-docs`, `validate-agents` for `check-agents`, `check-specs`, `analyse-specs`) goes to that gate's line below. For the rest (Markdown, typos, links, shell, workflows, secrets, whitespace), run `just fix` for what a fixer can repair, then fix the remainder by hand; a secret found by `ripsecrets` is removed, never allowlisted.
   - `fmt-check` or `toml-check` — run `just fix`; never hand-format to match.
   - `clippy` — fix the code. A lint that is wrong for one item gets `#[expect(lint, reason = "...")]` on that item; a lint that is wrong for the crate is changed in the workspace lint table in `Cargo.toml` with a comment saying why, never disabled at the call site with `allow`.
   - `metrics` — lower the complexity, cohesion or coupling rustqual reports. Never raise a threshold in `rustqual.toml`. A version mismatch means `just install-tools`, a network recipe that needs explicit authorization, has not run in this worktree.
   - `effects` — libpachinko reaches an effect directly. Put it behind a trait libpachinko defines, with a fake beside it, and its real implementation in pachinko. A failure on the fixture under `tests/fixtures/effects-violation` names its cause; read it before touching a ban list. When the fixture's `// ban:` tags and a ban list (`scripts/check-effects.sh` or `crates/libpachinko/clippy.toml`) disagree, or a ban stays silent or fires twice, add or remove the break and its `// ban:` tag with the ban, and never weaken a break to quiet the probe. When nothing fired, a lint cap or `-A warnings` in `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` or a cargo config's `rustflags` is hiding every finding: remove it, not a ban. When the fixture did not compile, either a ban names an API unstable on the pinned toolchain or the fixture has an error. When a matcher missed a crate or a ban the probe saw, the defect is in `scripts/check-effects.sh`. A drift failure means the root `clippy.toml` changed without its mirror in `crates/libpachinko/clippy.toml`.
   - `features` — a feature that does not compile alone or in combination. Fix the `cfg`; a feature combination is never dropped from the powerset.
   - `test-doc` — a doctest fails. Fix the example or the item it documents; doc examples are tests.
   - `coverage` below the floor — add the missing test. Never lower `coverage_floor` in the `Justfile`.
   - `doc` — a missing doc comment or a broken intra-doc link; `missing_docs` is on and warnings are errors there.
   - `deny` or `deps-unused` — a licence, a ban or an unused dependency; a new dependency is a decision recorded per decision 0007, and `just fix` runs `cargo shear --fix`.
   - `check-docs` — a frontmatter list that disagrees with `docs/manifest.yml` is usually list order; the comparison is order-sensitive.
   - `check-agents` — a bridge under `.claude/` or `.codex/` has grown content, or a managed file is missing from the inventory.
   - `check-specs` or `analyse-specs` — hand to the `spec-change` skill; a diagnostic is a regression and a finding cannot be waived.
   - `snapshots-check` — a snapshot differs or a `.snap` file is unreferenced. Run `just snapshots-review`, accept only a change you can explain, and commit the diff.
3. Distinguish an unreachable branch from an untested one. Defensive code no input can reach should be removed, not covered by a contrived test.
4. Never disable a gate to make a run green. A suppression is a last resort: one rule, one line, with a stated reason.
5. If a recipe changed the worktree, that is a defect in the recipe — checks are read-only. Fix the recipe.
6. Run the narrow recipe, then `just check` to confirm nothing else moved.
