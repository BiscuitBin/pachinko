#!/bin/sh
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$project_root"

git rev-parse --is-inside-work-tree >/dev/null

# The environment first: install-tools, install-allium and install-hooks need
# its cargo-binstall, `bg-*` scripts and prek; the toolchain lines need only
# rustup. `--locked` refuses a lockfile that disagrees with `pyproject.toml`, so
# a stale lock is fixed by `just lock` and committed, never rewritten silently by
# a first run.
pixi install --locked

# The toolchain rust-toolchain.toml pins, with the components the file names.
just install-toolchain

# The pin in `rust-toolchain.toml` is honoured only by rustup's cargo proxy, so
# this refuses any other cargo, the pixi environment holding none, before
# anything is built with it.
just check-toolchain

# What conda-forge lacks, from `tools.txt` and `tools-source.txt` into
# `.tools/bin` through the environment's cargo-binstall, the second list built
# from source: one of the two network downloads in the first-run path that no
# lockfile accounts for.
just install-tools

# The other one. The Allium checker for docs/specs/, pinned and checksummed in
# the biscuit-games-tooling package that pyproject.toml pins, landing in the
# gitignored .tools/bin. `just check-specs` and `just analyse-specs` run it, and
# both the hook gate and `just check` run those, so a worktree without it cannot
# reach a green gate.
just install-allium

# `Cargo.lock` is committed and every gate passes `--locked`; this line exists
# for a clone whose lockfile was deleted by hand.
test -f Cargo.lock || cargo generate-lockfile
just sync

# Hooks are installed only from the primary checkout. Every worktree of this
# repository shares one .git/hooks directory, and `prek install` writes a shim
# naming an absolute path into whichever worktree ran it — so a hook installed
# from a secondary worktree runs that worktree's .pixi/envs/default/bin/prek for
# commits made anywhere, and keeps doing so after the worktree is deleted, at
# which point every commit fails on a `prek` that is not on PATH. The comparison
# is the test git itself uses: in a secondary worktree the common directory and
# the git directory differ. The recipe also provisions every hook environment,
# so this is the last line of the first-run path that reaches the network.
if [ "$(git rev-parse --git-common-dir)" = "$(git rev-parse --git-dir)" ]; then
    just install-hooks
else
    printf '%s\n' 'Secondary worktree: skipping install-hooks.' >&2
    printf '%s\n' 'Run just install-hooks once from the primary checkout.' >&2
fi

printf '\n%s\n' 'Ready. Next: just check.'
printf '%s\n' 'Nothing has been formatted, staged, committed, tagged, or pushed.'
