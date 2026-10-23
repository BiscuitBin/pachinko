---
title: "Decision 0011: Tool manager"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_tool_manager]
requires: []
---

# Decision 0011: Tool manager

*Carried from libpawdoku's decision 0011 at `ccdd001`.*

## Context

[Decision 0004](0004-hook-runner-and-checkers.md) keeps the Python hook runner and the
shared checkers, and the installers its argument started from were uv for the Python
side and cargo-binstall for the cargo tools, each with its own lockfile or pin list,
beside rustup for the compiler. A fresh machine would install four tools before the
first-run script could start, and the tool pin list named versions rather than hashes,
trusting each tool's GitHub release naming.

The record this one carries stopped on that trust. taplo 0.10.0 publishes no binstall
metadata and its release archives are in a shape binstall cannot unpack, so the pinned,
current version could not be installed without compiling it, which a binary-only install
refuses by design. The other tools installed only because binstall's default naming guess
matched their releases.

pixi resolves the same tools from conda-forge, where every one of them except cargo-hack
is packaged at a pinnable version, and records the whole environment, Python and the
checkers included, in one hashed lockfile. conda-forge also packages the Rust compiler,
but without the llvm tools the coverage floor needs; and only rustup's cargo proxy
honours `rust-toolchain.toml`.

## Decision

pixi owns every tool binary and the Python environment. `pyproject.toml` is the manifest:
its `[tool.pixi.dependencies]` table pins just, prek, cargo-binstall, cargo-nextest,
cargo-llvm-cov, cargo-deny, cargo-insta, cargo-shear and taplo exactly and Python to its
`3.14.*` series, and `[tool.pixi.pypi-dependencies]` pins `biscuit-games-tooling` as a
git dependency at a release tag. `pixi.lock` is the pin; `pixi install --locked` is the
first thing the first-run script does; `pixi lock --check --offline --dry-run` is the
gate's lockfile check, inside `just lock-check`; the environment lives in the gitignored
`.pixi/`. The `Justfile` puts the environment's `bin` first on every recipe's `PATH`, then
`.tools/bin`, and calls every tool by name; nothing runs through `pixi run` inside a
recipe.

rustup keeps the compiler. `rust-toolchain.toml` remains the single owner of the Rust
version and its components; the pixi environment holds no `rust` package, and
`just check-toolchain` refuses any cargo that is not rustup's proxy.

`tools.txt` is the escape hatch for a tool conda-forge does not carry. It lists
`cargo-hack` alone; cargo-binstall, itself a pixi dependency, installs it into
`.tools/bin` through `just install-tools`, beside the Allium checker the tooling package
puts there. Its pass is binary-only and never compiles. The hooks that prek pins (typos,
lychee, shellcheck, actionlint, markdownlint-cli2, editorconfig-checker, ripsecrets) keep
their pins in the hook configuration. `tools-source.txt` is the second escape hatch, for
a tool that also has no release binary: it lists rustqual, which cargo-binstall builds
from source, as [decision 0012](0012-metrics-gate.md) accepts.

## Consequences

Of the consequences decision 0004's argument named for uv, none holds here: uv is not a
prerequisite and not a resolver; `uv.lock`, `.venv/` and `.python-version` do not exist;
CI carries no `setup-uv` step; no recipe begins with `uv run --frozen`. These stand: the
hook runner is prek, the checkers are the six console scripts of `biscuit-games-tooling`
at a pinned tag, `pyproject.toml` is where the package reads its configuration, and the
interim `runes` sentence in `AGENTS.md` waits for the same follow-up.

Contributors need rustup, pixi and just (any just launches the recipes; the pinned one in
the environment runs every nested call and CI). CI installs pixi with one action, restores
the environment from a cache keyed on the lockfile's hash, and keeps `.tools` in a cache
of its own, keyed on both pin lists, for what pixi cannot own. Every tool pin conda-forge
carries is a hash in `pixi.lock`; the compiler's stays in `rust-toolchain.toml`, the two
escape-hatch lists name versions, and the hook pins stay in the hook configuration. A
moved tool pin is one edited line, one `pixi update <name>` and a committed lockfile, as
[Maintain dependencies](../how-to/maintain-dependencies.md) describes.

The cost is a package manager that Rust contributors do not usually carry, needed for
the gate and not for `cargo build`; one environment per worktree, not relocatable; a
version that conda-forge lags on occasionally (the cargo-shear pin follows conda-forge's
newest, not crates.io's); and cargo-hack and rustqual still arriving by a second
mechanism, rustqual as a source build, until conda-forge packages them.

## What would reopen this

cargo-hack appearing on conda-forge, which empties `tools.txt`, and with rustqual there
too ([decision 0012](0012-metrics-gate.md)) removes cargo-binstall; that is a pin move,
not a new decision. conda-forge shipping llvm-tools for its Rust package and a cargo that
honours `rust-toolchain.toml`, which would make a pixi-owned compiler possible and would
reopen the question of who owns the Rust pin. pixi's PyPI git resolution failing for the
tooling package, which would return the Python side to uv without moving the tools. A
house-wide move of every Biscuit Games repository to pixi, which would make the manifest
shape a template concern.

## Related pages

- [Decision 0004: Hook runner and checkers](0004-hook-runner-and-checkers.md)
- [Decision 0012: The metrics gate](0012-metrics-gate.md)
- [Develop locally](../how-to/develop-locally.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Configuration](../reference/configuration.md)
