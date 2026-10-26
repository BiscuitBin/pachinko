---
title: "Develop locally"
kind: "how-to"
audience: [contributor, maintainer, agent]
canonical_for: [local_development]
requires: []
---

# Develop locally

## Prerequisites

| Tool | Why |
| --- | --- |
| `rustup` | Installs the exact toolchain `rust-toolchain.toml` pins, and provides the only cargo the gate accepts. `~/.cargo/bin` must come ahead of `~/.pixi/bin` on `PATH`, or run `pixi global uninstall rust`. |
| `pixi`, 0.81.0 or newer | Installs every other tool, and the Python environment the hook gate runs on, from `pixi.lock` into the gitignored `.pixi/envs/default`. |
| `just` | The task runner, and the only supported interface to the checks. `pixi global install just` provides one. Any `just` launches the recipes, because the pinned one in the environment runs every nested call and CI; `pixi run --frozen just <recipe>` is the escape hatch when no global `just` is installed. |
| `gh` | Only for the pin-bump commands in [Maintain dependencies](maintain-dependencies.md). |
| `npx`, from Node.js | Only for the Renovate configuration check in [Maintain dependencies](maintain-dependencies.md). |
| A C++ compiler | libfuzzer-sys compiles C++ when `just clippy`, and so `just check`, builds the fuzz crate: the Xcode command-line tools on macOS, `g++` (from `build-essential`) on Ubuntu. [Testing](../reference/testing.md#fuzzing) has the detail. |
| Rosetta 2, on Apple silicon only | Only for `just fuzz`: the pinned cargo-fuzz ships an x86 macOS binary alone. See [Testing](../reference/testing.md#fuzzing). |

No other installer: not uv, not cargo-binstall. `rust-toolchain.toml` names the exact
toolchain and its components, and rustup installs them the first time cargo runs here.
`pyproject.toml` names every other tool and `pixi.lock` pins it, with `tools.txt` and
`tools-source.txt`, one `name@version` per line, the exceptions for what conda-forge
lacks.
[Configuration](../reference/configuration.md) has each figure; this page states none.
The page covers Ubuntu and macOS on Apple silicon, the two platforms `pyproject.toml`
solves for. Windows is not supported: the `Justfile` and the Allium installer are
POSIX-only.

## First run

Two tools are installed once per machine, and everything after them comes from this
repository's pins; the C++ compiler, and Rosetta where it applies, come from the operating
system.

The first is rustup. No default toolchain is needed: `rust-toolchain.toml` chooses the
toolchain here, and prek builds its one Rust hook with the pinned toolchain rustup has
already installed. The one-time command is upstream's:

```console
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain stable --profile minimal
```

The `stable` default it sets is only for use outside this repository; here
`rust-toolchain.toml` overrides it, and `--default-toolchain none` works as well. Any
route that ends with `rustup` on `PATH` and `command -v cargo` printing
`~/.cargo/bin/cargo` is equivalent.

The second, and last, is pixi, with the installer its documentation gives:

```console
curl -fsSL https://pixi.sh/install.sh | sh
```

Copy the command from pixi's installation page on the day rather than from here if the
two differ. `requires-pixi` in `pyproject.toml` is the floor, and a pixi below it refuses
the manifest. These two are the only curl-pipes in this handbook. Then
`pixi global install just`, if nothing else on the machine provides a just.

One hazard catches almost everyone once. A cargo from `pixi global`, Homebrew's `rust`
formula or a Linux distribution ignores `rust-toolchain.toml` and builds with whatever
release it happens to be, so the lint set silently differs from CI's.
`just check-toolchain` refuses it before anything else runs, with one of three lines:

```text
rustup is not installed; see docs/how-to/develop-locally.md
cargo resolves to /home/you/.pixi/bin/cargo, not rustup's proxy
the active toolchain is 1.98.1-aarch64-apple-darwin (overridden by environment variable RUSTUP_TOOLCHAIN); it must be 1.99.1, selected by rust-toolchain.toml
```

The third is a `RUSTUP_TOOLCHAIN` or a `rustup override` outranking the pin; unset the
variable, or run `rustup override unset` in the checkout.

The remedy is `~/.cargo/bin` ahead of `~/.pixi/bin` on `PATH`, or
`pixi global uninstall rust`. The repository's own pixi environment holds no cargo and is
not the hazard; a `pixi global` rust is.

```console
just initialize
```

This installs the pixi environment from `pixi.lock`, installs the pinned toolchain,
refuses a cargo that is not rustup's, installs cargo-hack, cargo-release and cargo-fuzz
from `tools.txt` and rustqual from `tools-source.txt` into `.tools/bin`, then the Allium
checker, syncs all three lockfiles, and installs the pre-commit hook with every hook
environment. Run it once per clone. It needs the network for the pixi environment, the
toolchain, the `tools.txt` binaries, rustqual (a signed build if one exists, otherwise a
build from its crates.io source),
the Allium checker, `cargo fetch` and the hook clones. It never formats, stages,
commits, tags or pushes.

The hook is installed from the primary checkout only, because every worktree shares one
`.git/hooks`. In a secondary worktree `just initialize` says so and skips
`just install-hooks`, and the first `just lint` there clones and builds any hook
environment the primary checkout has not already built, which needs the network too.
`just check` runs `lint` as its third gate and adds no network need of its own, so once
the hook environments exist the whole gate is offline.

`.pixi/envs/default/bin` and `.tools/bin` are both gitignored, and the `Justfile`
prepends them to `PATH`, in that order, for every recipe. They are not on your shell's
`PATH`, so a bare `cargo nextest` from a shell needs
`PATH="$PWD/.pixi/envs/default/bin:$PWD/.tools/bin:$PATH"`, a recipe, or
`pixi run --frozen cargo nextest`.

## Every day

```console
just build
just test
just doc
```

The workspace holds two crates, the library `libpachinko` and the service `pachinko`,
and every one of these recipes covers both, so a change to either crate runs the same
loop. `just build` compiles the workspace with every feature. `just test` runs the unit
and integration tests through nextest, then the doctests; the service's integration
test runs the built binary. `just doc` builds the API documentation for both crates
into `target/doc/`, with warnings as errors, as CI does.

Each gate is also a recipe of its own, such as `just fmt-check`, `just clippy`,
`just coverage` or `just effects`, and running the ones a change touches is quicker
than the whole gate. [Commands](../reference/commands.md) lists every recipe.

## Before handing work back

```console
just fix       # formats and applies the safe automatic repairs
just check     # the whole gate, read-only
```

Of the two, only `just fix` modifies files; outside the pair, `just lock`,
`just lock-upgrade`, `just format` and `just snapshots-accept` write too. Every
check is read-only, and
`just check` proves it by comparing the worktree before and after each recipe.

## Keeping the workspace current

After pulling, re-sync so the installed dependencies match the lockfiles:

```console
just sync
```

It installs a moved `pixi.lock` exactly as committed and fetches what `Cargo.lock` and
`fuzz/Cargo.lock` name.
After `tools.txt` or `tools-source.txt` has moved, run `just install-tools` as well; it
skips a version that is already there.

## Related pages

- [Commands](../reference/commands.md)
- [Test and debug](test-and-debug.md)
- [Troubleshooting](../operations/troubleshooting.md)
- [Configuration](../reference/configuration.md)
