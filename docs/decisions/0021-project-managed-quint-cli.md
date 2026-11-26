---
title: "Decision 0021: A project-managed Quint CLI"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_quint_cli]
requires: []
---

# Decision 0021: A project-managed Quint CLI

## Context

Quint is an executable specification language from Informal Systems, with a simulator,
a test runner and a model checker behind one command, `quint`. This repository wants it
for executable models of the queue beside the Allium specifications, and wants it the
way it holds every other tool: pinned by a lockfile, installed into the worktree, and
never on the machine's global `PATH`.

The CLI is published to npm as `@informalsystems/quint`, which needs Node 18 or newer.
conda-forge does not package it, so `pixi.lock` cannot name it, and it is not a Rust
crate, so neither `tools.txt` nor cargo-binstall can install it
([decision 0011](0011-tool-manager.md)). conda-forge does package Node.

Quint also installs things of its own on first use. `quint run`, `quint test` and the
REPL default to the Rust evaluator, which the 0.34.0 package downloads from Quint's GitHub
releases at a version it pins, `v0.8.0`. `quint verify`, and `quint compile --target tlaplus`,
left to the CLI, download Apalache, `0.62.4`, the same way, and run it on a Java runtime. Both land in `QUINT_HOME`, which defaults
to `~/.quint`, and the CLI checks neither download against a checksum; the Decision below
has the recipes install Apalache instead. `quint typecheck`
downloads nothing.

Informal Systems also ships Quint Studio, a desktop application, through a curl-pipe
installer that fetches the unpinned latest release with no checksum, installs it into
`/Applications` or `~/.local/bin` and expects an account login. Nothing about that
installer can be pinned or kept inside a worktree;
[decision 0022](0022-quint-studio-machine.md) records how the repository holds Studio
instead.

## Decision

pixi pins Node: `nodejs` in `pyproject.toml` at an exact LTS version, held in
`pixi.lock` like every other tool. npm, which comes with it, installs the CLI from a
project of its own under `tools/quint/`: `package.json` names `@informalsystems/quint`
at an exact version and `package-lock.json` holds the integrity hash of every package in
its closure. `tools/quint/.npmrc` saves exact versions and turns off funding notices,
audit calls, the update notifier and every install script. The registry package needs
none, and the one script in its closure, protobufjs's, does nothing the CLI uses.

The `Justfile` owns every use:

- `just install-quint` runs `npm ci` into the gitignored `tools/quint/node_modules/`.
  It reaches the network, and `just initialize` runs it.
- `just lock` and `just lock-upgrade` relock the npm project beside the Cargo and pixi
  locks, and `just lock-check` runs `npm ci --dry-run --offline`, which installs and
  fetches nothing, so a manifest the lock does not satisfy fails gate 2.
- `just quint <args>` runs the installed package's entry point by path, the package the
  guard checks, so a worktree without the install fails rather than falling through to
  another `quint` on `PATH`.
- The `Justfile` exports `QUINT_HOME` as `.quint`, the gitignored `.quint/` directory, so the evaluator and
  Apalache stay in the worktree rather than in the home directory. It must stay
  `.quint`: the installer and the `quint` recipe refuse any other value.
- `tools/quint/apalache.txt` pins Apalache, `0.62.4`, by the SHA-256 of the release's
  `apalache.tgz`, which matches the release's own `sha256sum.txt`. `just install-apalache`
  downloads that archive, checks it, and installs it into
  `.quint/apalache-dist-0.62.4/apalache`, where Quint looks before it downloads one
  itself, with a stamp recording what it installed. It reaches the network unless the
  stamped install is already in place, refuses a pin the installed CLI's
  `DEFAULT_APALACHE_VERSION_TAG` disagrees with, and `just initialize` runs it after
  `just install-quint`.
- `just lock-check` also fails unless the pin file's `quint=` is the version
  `tools/quint/package.json` and `package-lock.json` pin, so a move of the CLI fails gate 2
  until the Apalache pin has been reviewed against it. It checks that the review was
  recorded, not that it was right; the installer checks the tag.
- The `quint` recipe guards every run, offline and read-only. It refuses any option whose
  name contains `apalache` except `--apalache-config`. It checks nothing more when there
  are no arguments (the REPL), when the first is `parse`, `typecheck`, `run`, `test`,
  `repl` or `docs`, or when the only one is `--help` or `--version`; every other run,
  `verify`, `compile` to any target, `verify --help` and a leading option among them,
  needs the stamped install and pixi's Java, and is refused naming `just install-apalache`
  or `just sync`.

`just sync` does not install the CLI. The setup action runs `sync` in every CI job, and
no job needs Quint until one runs a Quint gate.

There is no Quint gate yet, because there is no Quint module, and a gate over zero files
asserts nothing. The change that adds the first module adds `check-quint`, running
`quint typecheck`, which is offline, to `just check` and to the recipe list in
`pyproject.toml`, and adds `just install-quint` to the CI job that runs it.

`quint verify` and TLA+ compilation run Apalache, which needs Java. pixi pins
conda-forge's `openjdk` exactly in `pyproject.toml`, and the `quint` recipe alone puts its
`java` first on `PATH`, since Apalache's launcher runs the first `java` there and ignores
`JAVA_HOME`. No gate runs either command yet.

Quint Studio is not part of this decision.
[Decision 0022](0022-quint-studio-machine.md) brings it under the repository, run in a
smolvm machine that the repository pins and manages, and replaces the sentence that
stood here, that Studio stays outside the repository. No recipe installs or runs
Studio yet, and no gate or CI job depends on it.

## Consequences

`tools/quint/package-lock.json` is a fifth lockfile, with a manager of its own beside
cargo and pixi. Renovate's npm manager reads it, held for dashboard approval like every
other tool pin; moving the pin by hand is an edit to `package.json` and `just lock`, as
[Maintain dependencies](../how-to/maintain-dependencies.md) describes. Renovate does not
track `tools/quint/apalache.txt`, so its `quint` pull request fails gate 2 until someone
reviews the Apalache pin against the new CLI.

Apalache is checked the way [decision 0005](0005-project-managed-allium-cli.md) checks
allium, by a hand-computed checksum, because the recipe downloads it rather than the CLI.
The guarantee has limits. The stamp detects a local change to the tree against the
installer's record, not a rewrite of tree and stamp together. It holds for
non-concurrent use: a guarded `verify` or `compile` that runs while
`just install-apalache` is replacing the tree finds no launcher, and Quint downloads
Apalache itself, unchecked; the next guarded run refuses that unstamped tree.

The evaluator is still pinned by version only, because the locked CLI names it, and
verified by nothing beyond TLS: Quint downloads it inside the CLI, which offers no hook
for a checksum. The exposure is limited to `run`, `test`, the REPL and a `verify` that reports a
violation, which evaluates the invariants with it; `typecheck`, the
only command a gate will run first, downloads nothing.

`tools/quint/node_modules/` and `.quint/` must stay ignored by Git. `just check`
snapshots the worktree between recipes, so files Git could see would abort the run.

Contributors need no global Node. The pixi environment's `node` and `npm` are first on
every recipe's `PATH`, so a Node installed elsewhere on the machine is never used here.

Java is in the default environment, so `just sync` installs it in every worktree and CI
job, though only `just quint` uses it. On macOS it roughly doubled the environment, from
about 337 MiB to 670 MiB; on linux-64 it brings 43 further packages, among them X11,
fonts, ALSA and CUPS libraries.

Quint talks to Apalache as a server on `localhost:8822`. It uses any server already
listening there, or the one `--server-endpoint` names, whoever started it and on
whatever Java, and starts its own, from the checked tree on pixi's Java, only when none
answers. The guard cannot see which Apalache such a server runs, and a remote
`--server-endpoint` is a network connection the recipe does not refuse.

## What would reopen this

conda-forge packaging Quint, which would move the pin into `pixi.lock` and retire
`tools/quint/`. Upstream publishing checksums for the evaluator, or an option to verify
it, which would close the evaluator's gap above. A gate that needs `quint verify`, which
would reopen the question of Apalache's provenance only for a server the recipe did not
start; the pinned Java it also needs is in place.

## Related pages

- [Decision 0005: A project-managed Allium binary](0005-project-managed-allium-cli.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
- [Decision 0022: A project-managed machine for Quint Studio](0022-quint-studio-machine.md)
- [Develop locally](../how-to/develop-locally.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
