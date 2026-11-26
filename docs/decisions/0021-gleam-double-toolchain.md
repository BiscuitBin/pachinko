---
title: "Decision 0021: The Gleam double's toolchain and packages"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_gleam_double_toolchain]
requires: []
---

# Decision 0021: The Gleam double's toolchain and packages

## Context

The Gleam double under `gleam-double/` is a second re-implementation of the domain
`docs/specs/pachinko.allium` defines, beside the Python double under `double/`. The
Python double needed nothing new: it runs on the Python the pixi environment already
holds and imports only the standard library. A Gleam double needs a compiler, a runtime
and packages from hex.pm, none of which this repository had.

[Decision 0011](0011-tool-manager.md) makes pixi the owner of every tool binary, pinned
exactly in `pyproject.toml` with `pixi.lock` as the pin.
[Decision 0007](0007-dependency-policy.md) asks for a note before a dependency is added,
and words that note for a crate: caret ranges in the manifest, `Cargo.lock` as the pin and
`--locked` on every gate. None of that carries to Gleam as written, so this record is the
note for the double's packages and says what stands in for each part.

Three facts shaped it:

- conda-forge packages `gleam` and `erlang` for both supported platforms, and its `gleam`
  declares no run dependency on `erlang`, so an environment that names only `gleam` has a
  compiler that cannot run anything.
- Gleam has no frozen, locked or offline flag. A `gleam.toml` that disagrees with
  `manifest.toml` makes `gleam build`, `gleam test` and `gleam deps download` rewrite
  `manifest.toml` and exit 0.
- The maintainer's Gleam project, `BiscuitBin/knarr`, runs `gleam` 1.19.0 on `erlang`
  29.1.2 through pixi, and guards every `gleam` recipe against that rewrite.

## Decision

**The toolchain.** `pyproject.toml` pins `gleam = "==1.19.1"` and `erlang = "==29.1.2"`
exactly, as decision 0011 pins a tool, and `pixi.lock` holds their hashes. Both are
pinned because conda-forge's `gleam` does not bring `erlang`.

**A feature of their own.** The two pins sit in a pixi feature, `gleam`, whose
environment, also `gleam`, sets `no-default-feature = true`. It holds the compiler and the
runtime and nothing else. The default environment is unchanged, and it is the only one on
the `Justfile`'s exported `PATH`. The maintainer chose this over adding the pins to the
default environment, as knarr keeps its cluster tools out of its own default one.
`.github/actions/setup/action.yml` passes `setup-pixi` no `environments` input, and
`just sync` runs `pixi install --frozen` with no `-e`; with neither naming an environment,
pixi installs the default one alone, so no CI job and no `just initialize` installs
Erlang.

**The packages.** `gleam-double/gleam.toml` names four, each as a range, and nothing else
is added without amending this record:

| Package | Why the double cannot do without it |
| --- | --- |
| `gleam_stdlib` | Lists, dictionaries, strings, `Result` and `Option`: the language's own library, which every Gleam package takes. |
| `gleam_erlang` | Processes, subjects and monitors. The owner's callers reach it through a subject, and the test runner runs each test in a monitored process. |
| `gleam_otp` | The actor behind the owner: one process, one mailbox, one message at a time. |
| `simplifile` | File reads, for the tests alone: the coverage check reads the scenario catalog and the specification. A development dependency. It brings `filepath`. |

**The pin.** `gleam-double/manifest.toml` is the pin, committed and written only by
`gleam`: it is to `gleam.toml` what `Cargo.lock` is to `Cargo.toml` under decision 0007.
It records each package's version and its checksum from hex.pm.

**The guard.** With no `--locked` to pass, `just gleam-double` checks before it runs
`gleam` and compares afterwards. Two lines of Python, from the default environment, run
before any `gleam` command; two more, under "The recipes" below, compare the toolchain
with its pins. The first refuses a `gleam.toml` whose requirements differ
from the ones `manifest.toml` records, as knarr's pre-check does: `gleam` would resolve
them on hex.pm and write the answer back. The second refuses a
`gleam-double/build/packages/` that does not hold the versions `manifest.toml` pins, as
after a checkout that moved the pins: `gleam` would fetch what is missing. Both are there
because the recipe is to reach no network, and a drift of either kind sends `gleam` to
hex.pm before it rewrites anything. Then each line that runs `gleam build`, `gleam test`
or `gleam deps download` compares `manifest.toml` before and after with `cksum` and fails
if it changed. That is the backstop for a rewrite neither check sees, and it fails after
the rewrite, which `git diff` then shows.

**The recipes.** `just gleam-double` checks the format, builds with warnings as errors
and runs the tests, and reaches no network: with `gleam-double/build/packages/` holding
what `manifest.toml` pins and `manifest.toml` unchanged, `gleam` downloads nothing.
`just gleam-double-deps` is the one recipe that does: it installs the `gleam` environment
exactly as `pixi.lock` pins it, then fetches the packages, and refuses to start without
`manifest.toml`, since `gleam` would otherwise resolve `gleam.toml`'s ranges afresh and
write a new pin. `just gleam-double-format`
rewrites the sources in `gleam`'s own layout, which `just gleam-double` only checks; it
reads no package and reaches no network. Each recipe line puts `.pixi/envs/gleam/bin`
first on its own `PATH`, and `just gleam-double` refuses to start unless `gleam` and
`erl` are there, so a `gleam` installed elsewhere on the machine cannot stand in for the
pinned one. It also refuses unless that environment holds the `gleam` and the `erlang`
that `pyproject.toml` pins, which it reads from the environment's `conda-meta` records:
`just lock` moves `pixi.lock` and installs nothing, so without the comparison an
environment installed before a pin moved would run, and pass, as the old pair. It then
compares the same records, by the URL each package was installed from, with the packages
`pixi.lock` lists for the `gleam` environment on the machine's platform, so a relock that
changes a build or a dependency of either, but no version, is refused too. None of the
three is in `just check`.

**No gleeunit.** Gleam's usual test package runs every test through EUnit, which fixes no
order and never runs a test in the caller's process, and then halts the virtual machine.
The double's coverage check has to run after the last test and read what each test fired,
so the double has its own runner in `gleam-double/test/` and takes no test package, no
property-test or snapshot package and no linter.

## Consequences

`pixi.lock` gains a second environment. The lockfile's hash keys CI's pixi cache and its
prek cache, so the commit that adds the feature misses both once, as any lockfile change
does. After that, CI pays nothing for the double, and runs none of it.

Running the double takes one step more than running the Python double:
`just gleam-double-deps`, once per worktree, with the network. The `gleam` environment
adds an Erlang runtime to `.pixi/` in a worktree that runs it.

The `PATH` for `gleam` is set per recipe line and not by the `Justfile`'s export, so a
new recipe that calls `gleam` must set it too, and a bare `gleam` typed in a shell is
whatever the machine has, or nothing. A bare `gleam` also skips the `manifest.toml` guard.

How the pins move is not settled here, and until it is they move by hand.
`renovate.json` approves pixi pins through the `pixi` manager
([decision 0018](0018-renovate.md)), and whether that manager reads a feature's table is
proved only by Renovate's dashboard once this reaches `main`, as decision 0018 says of
each manager it could not run. Nothing in `renovate.json` names
`gleam-double/gleam.toml`; whether Renovate reads it unprompted is unproved for the same
reason. `just audit` asks RustSec about the workspace's `Cargo.lock` and covers no hex
package, and no advisory source for hex is wired in. Until the dashboard shows otherwise,
a pin moves like this: edit the pin or the range, run `just lock` for a pixi pin or
`gleam deps update` for a package from inside the `gleam` environment, read the diff, run
`just gleam-double-deps`, which installs a moved pixi pin, and run `just gleam-double`.
Until that install has run, `just gleam-double` refuses a moved pixi pin: `just lock`
installs nothing, so the environment still holds the old version.

`gleam` 1.19.1 is conda-forge's newest and is one patch ahead of what knarr proves on the
same Erlang. If the pair had not run, the fallback was knarr's 1.19.0.

A populated `gleam-double/build/` holds the packages' own TOML, Markdown and sources.
`.gitignore` keeps it from the hook gate, which reads the files Git can see.

## What would reopen this

The double joining `just check` or CI, which would put the `gleam` environment on a
runner and ask for the linter knarr's gate has. A fifth package,
which amends this record first. A second Gleam project in the repository, which would
make the per-line `PATH` a `Justfile` concern. Gleam gaining a locked or frozen mode,
which replaces the `cksum` guard. Renovate's dashboard showing that it reads, or
misreads, the feature's pins or `gleam.toml`, which settles how the pins move.
conda-forge's `gleam` gaining a run dependency on `erlang`, which makes the second pin
a choice rather than a need.

## Related pages

- [Decision 0007: Dependency policy](0007-dependency-policy.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
- [Decision 0018: Dependency updates through Renovate](0018-renovate.md)
- [Decision 0019: The service runtime](0019-service-runtime-tokio-and-http.md)
- [Commands](../reference/commands.md)
- [Repository map](../project/repository-map.md)
