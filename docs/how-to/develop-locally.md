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
| Rosetta 2, on Apple silicon only | Only for `just fuzz`, whose pinned cargo-fuzz ships an x86 macOS binary alone ([Testing](../reference/testing.md#fuzzing)), and for a smolvm machine made with `--rosetta`, which, by smolvm's source, it makes without Rosetta all the same ([smolvm](#smolvm)). |

No other installer: not uv, not cargo-binstall. `rust-toolchain.toml` names the exact
toolchain and its components, and rustup installs them the first time cargo runs here.
`pyproject.toml` names every other tool and `pixi.lock` pins it, with `tools.txt` and
`tools-source.txt`, one `name@version` per line, the exceptions for what conda-forge
lacks, `tools/quint/`, an npm project whose lock pins the Quint CLI
([Quint](#quint)), and `tools/smolvm/smolvm.txt`, the smolvm release
([smolvm](#smolvm)).
[Configuration](../reference/configuration.md) has each figure; this page states none.
The page covers Ubuntu and macOS on Apple silicon, the two platforms `pyproject.toml`
solves for; smolvm, and Quint Studio with it, run on macOS on Apple silicon alone.
Windows is not supported: the `Justfile` and the Allium installer are POSIX-only.

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
checker, the Quint CLI and the Apalache it runs, and, on macOS on Apple silicon alone,
smolvm (any other host says it skips it), syncs the four Cargo and pixi lockfiles, and
installs the pre-commit hook with every hook environment. Run it once per clone. It needs the network for the pixi environment, the
toolchain, the `tools.txt` binaries, rustqual (a signed build if one exists, otherwise a
build from its crates.io source),
the Allium checker, the Quint CLI's npm packages, Apalache, smolvm, `cargo fetch` and the hook clones. It never formats, stages,
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

It installs a moved `pixi.lock` exactly as committed and fetches what `Cargo.lock`,
`fuzz/Cargo.lock` and `loadgen/Cargo.lock` name.
After `tools.txt` or `tools-source.txt` has moved, run `just install-tools` as well; it
skips a version that is already there. After `tools/quint/package-lock.json` has moved,
run `just install-quint`, then `just install-apalache`. After `tools/smolvm/smolvm.txt`
has moved, follow [Moving the smolvm pin](maintain-dependencies.md#moving-the-smolvm-pin)
before any other smolvm command: install, delete this worktree's machines, purge.

## Quint

The Quint CLI is pinned in `tools/quint/` and installed into the worktree by
`just initialize`, or on its own by `just install-quint`, which reaches the network
([decision 0021](../decisions/0021-project-managed-quint-cli.md)). Run it through its
recipe:

```console
just quint --version
just quint typecheck path/to/module.qnt
just quint run path/to/module.qnt
```

`typecheck` works offline. `run`, `test` and the REPL download Quint's Rust evaluator
the first time, unchecked, and so does a `verify` that reports a violation. `verify` and
`compile --target tlaplus` run Apalache, which
`just install-apalache` installs from the archive `tools/quint/apalache.txt` pins by
SHA-256, and which runs on the `openjdk` pixi pins; the `quint` recipe alone puts that
`java` on `PATH`. The recipe refuses `verify` and every `compile` until that install is in
place and matches its stamp, and refuses any option naming Apalache except
`--apalache-config`, so Quint never downloads Apalache itself through it, unless
`just install-apalache` is replacing the tree while a run starts.
`verify` and `compile --target tlaplus` use any Apalache server already listening on
`localhost:8822`, or the one `--server-endpoint` names, whatever Java it runs on, and
start their own only when none answers. Reporting a violation also needs the evaluator, which `verify` downloads then on first use.
Every recipe sets `QUINT_HOME` to `.quint`, the gitignored `.quint/` directory, and it
must stay exactly that value,
so both land in the worktree and `~/.quint` is never written. A bare `quint`
from your shell is not this one, or is nothing at all.

### Optional: Quint Studio

Quint Studio is Informal Systems' desktop application for Quint. Its own installer,
`https://studio.quint.sh/install.sh`, is a curl-pipe that downloads the latest release,
unpinned and with no checksum, installs it system-wide, into `/Applications` on macOS or
`~/.local/bin` on Linux, puts `quint-studio` on your `PATH`, and asks you to sign in with
an account. [Decision 0022](../decisions/0022-quint-studio-machine.md) brings Studio under
this repository instead, run headless inside the smolvm machine below, on macOS on Apple
silicon; no recipe installs or runs it yet. Until one does, that installer is a
per-machine choice made outside this repository: no recipe, gate or CI job depends on
Studio, and the CLI the recipes run is the pinned one above, whatever Studio installs.

## smolvm

smolvm runs the Linux machine that Quint Studio is to run in
([decision 0022](../decisions/0022-quint-studio-machine.md)). `tools/smolvm/smolvm.txt`
pins its release, and `just initialize` installs it into the gitignored `.tools/smolvm/`
on macOS on Apple silicon, or `just install-smolvm` on its own, which reaches the network
unless the stamped install is already in place. Any other host is untested: the
installer refuses it and `just initialize` skips it. Run it through its recipe:

```console
just smolvm --version
just smolvm machine ls
```

`just smolvm` runs the stamped install by path, never a `smolvm` on your `PATH`, in a
cleared environment that keeps its state where the table below says, and refuses a
caller's setting that would move that state; [Commands](../reference/commands.md#documents)
lists them. It prints only on standard error, so smolvm's standard output is smolvm's
alone. It holds its worktree's locks until smolvm ends, so meanwhile another smolvm
command for the same worktree and `just smolvm-purge` are refused, naming the process
that holds them, and so is `just install-smolvm`; a machine left running after its
command has returned holds nothing. Pulling an image, and every guest, reach the
network. By its source, smolvm does not refuse to make a machine with `--rosetta` on a host without
Rosetta, so test for it first:

```console
test -e /Library/Apple/usr/libexec/oah/RosettaLinux/rosetta
```

What a machine may read and write is the configuration you give it. The read-only
worktree with a few writable subdirectories that decision 0022 describes is the one the
Studio recipes will run; `just smolvm` passes its arguments through, and a directory you
mount with `-v` is open to the guest as you mount it.

### Where the machine's state lives

The state is outside the worktree, one for each worktree, named as
[decision 0022](../decisions/0022-quint-studio-machine.md) forms `<worktree>-<hash>`:

| Path | What it is |
| --- | --- |
| `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>/` | The state directory: smolvm's database, usage journal, image caches, cached registry tokens and every machine's disks, beside the marker `.pachinko-state`, which names this worktree, and the `tmp/` and empty `docker-config/` the recipe gives smolvm. |
| `/tmp/pachinko-smolvm-<hash>` | The state link, a symbolic link to the state directory that smolvm is given as its root, because smolvm limits the length of the root. macOS empties `/tmp` on a restart, and the next command makes the link again. |
| `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>.lock` | The state lock. |
| `.tools/smolvm.lock` | The install lock, inside the worktree and ignored. |
| `.tools/smolvm-state-id` | This worktree's id, inside the worktree and ignored, which the marker must match. |

The first `just smolvm` command makes the state, `--version` included, with the
directories above it where they are missing. `~/Library/Caches/pachinko` and its
`smolvm/` must have mode 0700 exactly, since smolvm opens the state directory to 0755
and they alone keep other accounts out; any other mode is refused, naming the
`chmod 700 <path>` that mends it.

smolvm expands two templates beside its binary on first use,
`.tools/smolvm/overlay-default.ext4` and `storage-template.ext4`, which the install's
stamp leaves out. If one is damaged, remove both: smolvm expands them again on its next
use, and `just smolvm` still accepts the install.

### Removing the state

```console
just smolvm machine ls
just smolvm machine delete -f --name <machine>
just smolvm-purge
```

`just smolvm-purge` removes the state link and the state directory, with every image,
cache and machine disk in it, and nothing else. It refuses, removing nothing, while
smolvm lists any machine there, naming each and the command that deletes it; when the
listing fails; without the stamped install, so run `just install-smolvm` first; and
while another smolvm command for this worktree holds the lock. With no state it says so
and exits 0, removing a state link left pointing at the absent directory. It leaves the
state lock, the directories above the state directory and
`.tools/smolvm-state-id`, so a new state takes the same id. Run it before you delete a
worktree: once the worktree is gone, only the orphan's form below removes its state.
Quint Studio's own data is kept apart from this state, and the purge does not touch it.

The purge renames the state directory to
`~/Library/Caches/pachinko/smolvm/.purging-<hash>-<pid>` before it removes it. If the
removal fails part way, that `.purging-*` directory is all that is left, and nothing
uses it. If the purge instead refused because what it renamed was not the directory it
checked, it says so and leaves both paths alone: look at each, and put back by hand
what belongs where. Before removing a `.purging-*` directory by hand, while no pachinko
smolvm command is running, check that its marker names the worktree you purged:

```console
grep '^path=' ~/Library/Caches/pachinko/smolvm/.purging-<hash>-<pid>/.pachinko-state
rm -rf ~/Library/Caches/pachinko/smolvm/.purging-<hash>-<pid>
```

### The state of a worktree that is gone

Each marker records its worktree's path on its `path=` line:

```console
grep -H '^path=' ~/Library/Caches/pachinko/smolvm/*/.pachinko-state
```

A state directory whose recorded path no longer exists, or no longer holds that state's
id, is an orphan. Remove it from any surviving checkout, naming the directory by its
`<worktree>-<hash>` name, not its path:

```console
just smolvm-purge --orphan <worktree>-<hash>
just smolvm-purge --orphan <worktree>-<hash> --delete <machine>
```

The first removes the orphan's state link and state directory under the purge's rules.
While the orphan's listing holds a machine, it refuses and names the second, which
deletes one machine that listing holds. It refuses a state whose worktree still exists
and holds that state's id, naming `just smolvm-purge` from there, and a state whose
recorded path lies behind a directory you cannot search, until that directory is
searchable again.

No tool removes a lock entry, this worktree's or an orphan's, since two commands that
found and missed a removed entry would hold two locks. A stale one, such as an orphan's
after its purge, may be deleted by hand while no pachinko smolvm command is running, at
your own risk.

## Related pages

- [Commands](../reference/commands.md)
- [Test and debug](test-and-debug.md)
- [Troubleshooting](../operations/troubleshooting.md)
- [Configuration](../reference/configuration.md)
- [Decision 0022: A project-managed machine for Quint Studio](../decisions/0022-quint-studio-machine.md)
