---
title: "Troubleshooting"
kind: "operations"
audience: [contributor, maintainer, operator, agent]
canonical_for: [troubleshooting]
requires: []
---

# Troubleshooting

Symptoms as headings, causes and fixes as bodies.

## `just check` reports that a recipe changed the worktree

`just <recipe> changed the worktree; checks must be read-only.` means the recipe is the
defect, not your change. Checks are read-only; anything that writes belongs in
`.pre-commit-fix.yaml` and runs from `just fix`. The `changed:` lines under the report
name which paths moved. Move the offending hook, or add the generated path to the ignore
rules. Here the usual paths are a build or tool directory that escaped the ignore rules
— `target/`, `.tools/` or `.pixi/` — or a rewritten `Cargo.lock` or `pixi.lock`, which
means a recipe resolved dependencies instead of reading the lockfile.

## `The Git worktree is not clean.`

`just check-clean` run on its own, with no baseline, asserts that `git status` is
empty, and it prints the status above this line. Commit the change or set it aside
first. Inside `just check` the same recipe compares against the snapshot taken when the
run began instead, so a worktree that was already dirty passes there; the hand-back
rule still asks for an empty `git status --short` afterwards.

## `docs validation: frontmatter <field> disagrees with the manifest`

Almost always list order. The comparison between `docs/manifest.yml` and a page's
frontmatter is order-sensitive, so `[maintainer, contributor]` fails against
`["contributor", "maintainer"]`. Copy the order from the manifest.

If the field is `title`, check for a stray difference in punctuation — the level-one
heading must match it byte for byte as well.

## `docs validation: not reachable from docs/README.md`

The page exists and is registered, but nothing links to it. Add it to
[the documentation map](../README.md), or to a page that is already reachable.

## `agent validation: unexpected managed file`

Something appeared under `.agents/`, `.claude/` or `.codex/` that is neither a declared
skill nor `.claude/settings.json`. If it is local tool state, add it to `.gitignore` —
the inventory reads Git, so an ignored file is invisible to it. If it is real content, it
belongs in `.agents/skills/` with bridges, or somewhere else entirely.

## `agent validation: must stay a thin pointer to the canonical skill`

A bridge under `.claude/` or `.codex/` has grown content, or its frontmatter has drifted
from the canonical skill. Regenerate it: the canonical frontmatter verbatim, one blank
line, then the fixed pointer sentence and nothing else. The body is compared against a
template rather than measured, so a clause of your own fails however short it is —
[Agent contract](../reference/agent-contract.md#what-a-bridge-must-be) carries the text.

## Coverage fails but everything is tested

Distinguish two cases. If a real path is untested, add the test. If the uncovered branch
cannot be reached by any input — a bounds check after a modulo, a fallback after an
exhaustive assignment — delete the branch. Do not lower the threshold.

On crates this small the denominator is small too, so one untested branch is enough to
breach the floor on its own. Test it or delete it; the number stays where it is.
[Test and debug](../how-to/test-and-debug.md#debug-a-coverage-failure) shows how to find
the lines.

## `effects: libpachinko depends on a banned crate`

`just effects` found a banned crate in libpachinko's normal dependency tree. Take the
effect through a trait libpachinko defines instead, as
[Architecture](../explanation/architecture.md) explains;
[Test and debug](../how-to/test-and-debug.md#debug-an-effects-or---locked-failure)
lists the bans the script checks.

## `effects: clippy <flags> over libpachinko exited <status>, want 0`

libpachinko calls a std API that `crates/libpachinko/clippy.toml` bans; clippy's output
above the line names the path and the file. Take the effect through a trait, as for a
banned crate. The other `effects:` failures say what drifted: the libpachinko
`clippy.toml` no longer restates the root file (mirror the root change there), or the
fixture under `tests/fixtures/effects-violation/` and a ban list disagree (add or remove
the break and its `// ban:` tag with the ban). If the probe reports that nothing fired,
a lint cap in `RUSTFLAGS` or a cargo config's `rustflags` is hiding every finding.

## `cargo resolves to <path>, not rustup's proxy`

`just check-toolchain`, the first gate, found a `cargo` on `PATH` that is not rustup's.
A cargo from pixi, Homebrew or a distribution ignores `rust-toolchain.toml` and builds
with whatever version it is, so the gate stops before anything compiles. Put
`~/.cargo/bin` ahead of the other directory on `PATH`, or remove the other cargo — for a
global pixi install, `pixi global uninstall rust` — until `command -v cargo` prints the
proxy in `~/.cargo/bin`.

## `rustup is not installed`

The same gate, one step earlier: there is no `rustup` on `PATH` at all. Install rustup
as [Develop locally](../how-to/develop-locally.md#prerequisites) describes, then run
`just install-toolchain`.

## A recipe cannot find `allium`, `prek` or a `bg-*` script

The pixi environment under `.pixi/` and the binaries under `.tools/bin/` are installed
per worktree, and a fresh worktree has neither. `pixi install --frozen` restores the
environment, `just install-tools` the tools `tools.txt` and `tools-source.txt` pin, and
`just install-allium` the `allium` checker; `just initialize` does all three. A worktree
that was renamed or moved needs `pixi install --frozen` again even though `.pixi/` is
there: the `bg-*` scripts' shebangs name the Python at the worktree's old absolute path.
`just install-hooks` says the same in its own words:
`the pixi environment is not installed; run just initialize first`.

## `allium is not installed; this project pins <version>. Run just install-allium`

`just check-specs` and `just analyse-specs` run the pinned checker from `.tools/bin`, and
a fresh worktree has none, so both fail, and with them `just check`, until it is there.
Run `just install-allium`, which reaches the network. The same message with
`allium <version> is installed` in place of `allium is not installed` means the binary
there is not the pin, as after a bump of the tooling package; the same recipe replaces
it. Never point the recipes at an `allium` already on `PATH`: the pin decides what the
checks report, and `just plan-spec` names `.tools/bin/allium` by path for that reason.

## `just lock-check` or `pixi install --locked` fails on `pixi.lock`

`pixi.lock` is stale against `pyproject.toml`: a pin moved in the manifest and the
lockfile was not solved again. Run `just lock`, read the diff, and commit the lockfile.

## `just lock-check` fails offline on a fresh machine

`just lock-check` runs offline, like every gate recipe, so a cold Cargo cache fails it
rather than fetching the registry index. Run `just sync`, which reaches the network once,
fetches what `Cargo.lock` names and installs `pixi.lock` exactly as committed; then the
gate runs offline again. `just snapshots-check` runs `just lock-check` first, so it fails
the same way and is cleared the same way.

## `cargo` reports that `Cargo.lock` needs to be updated but `--locked` was passed

Every recipe that builds, tests, lints or documents the workspace passes `--locked`, so a
manifest edit that the lockfile does not reflect fails each of them until the lockfile is
solved again. Run `just lock`, read the diff, and
commit `Cargo.lock` with the manifest. A new dependency is a decision before it is an
edit: [Decision 0007](../decisions/0007-dependency-policy.md) says what one has to
justify.

## `just lint` clones or downloads

Every hook environment should already be prepared, so `lint`, and `just check` around
it, never reaches the network. If it does, prek's cache was cleared, or this is a
secondary worktree whose primary checkout never ran `just initialize`. Run
`just install-hooks` from the primary checkout; it prepares every hook environment and
warms the link checker. `just initialize` is where the network belongs, and `just check`
must not need it.

## `Secondary worktree: skipping install-hooks.`

`just initialize` printed this because it ran in a secondary worktree, and that is
deliberate. Every worktree shares one `.git/hooks` directory, and `prek install` writes a
shim naming an absolute path into whichever worktree ran it, so a hook installed from a
secondary worktree runs that worktree's prek for commits made anywhere. Once that
worktree is deleted, every commit fails on a `prek` that is not on `PATH`. Run
`just install-hooks` once from the primary checkout, which also repairs a shim that
already points at a deleted worktree; never run it from a secondary one.

## The ripsecrets hook fails to install: the package believes it is in a workspace

`PREK_HOME` points inside a Cargo workspace, this checkout included. ripsecrets is a Rust
hook, built in prek's cache, and a build under a workspace root is taken for a member of
it. Point `PREK_HOME` at a directory outside any checkout, as CI does with the runner's
temporary directory, and run `just install-hooks` again.

## `just install-tools` will not compile a tool in `tools.txt`

The refusal comes from the `tools.txt` pass, and it is deliberate: that list is
binary-only (`--disable-strategies compile`), so a tool that cannot be installed from a
release binary fails loudly rather than compiling at bootstrap. The pin names a version
with no release binary for this host; choose one that has one. A tool with no release
binary for any version belongs in `tools-source.txt` instead, which may compile, and only
on a decision record that accepts the build, as
[decision 0012](../decisions/0012-metrics-gate.md) does for rustqual. To see which
cargo-binstall is running, ask it with `cargo binstall -V -v`: `cargo --list -v` names
the last one on `PATH`, not the one cargo runs.

## rustqual's source build fails with no toolchain

The `tools-source.txt` pass compiles rustqual with whichever toolchain rustup selects.
From inside the repository, that is the one `rust-toolchain.toml` pins; from anywhere
else it is rustup's default, and on a machine with no default rustup has no toolchain
to choose, so the build fails before it starts. `just` runs every recipe from the
repository root wherever it is called, so this happens only to a `cargo binstall` or
`cargo install` run by hand elsewhere: run `just install-toolchain`, then
`just install-tools`. A build that fails partway on a network error can simply be run
again; `--locked` means it resolves the same versions each time.

## `just metrics` reports the wrong rustqual

`metrics: rustqual missing on PATH, tools-source.txt pins 1.8.3; run just install-tools`
means `.tools/bin/rustqual` is absent, and a version in place of `missing` means the
binary found is not the pin, as after a bump to `tools-source.txt`. Run
`just install-tools`. The gate refuses any other rustqual because its version decides
what the gate reports.

## prek behaves as if a pin had not moved

A hook whose `rev` changed still runs its old environment. Clear prek's cache with
`pixi run --frozen prek cache clean`, then run `just install-hooks` again to prepare
every environment from the current configuration.

## Related pages

- [Test and debug](../how-to/test-and-debug.md)
- [Develop locally](../how-to/develop-locally.md)
- [Quality gates](../reference/quality-gates.md)
- [Maintenance](maintenance.md)
