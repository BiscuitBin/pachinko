---
title: "Commands"
kind: "reference"
audience: [contributor, maintainer, operator, agent]
canonical_for: [command_reference]
requires: []
---

# Commands

`just --list` prints the live set, and a bare `just` runs the `default` recipe, which
prints the same list. This page says what each recipe is for, grouped as the `Justfile`
groups them. The `Justfile` is the only supported interface: if something is worth
running twice, it belongs there rather than in a shell history.

Nine recipes reach the network: `install-toolchain`, `install-tools`, `install-allium`,
`install-hooks`, `sync`, `lock`, `lock-upgrade`, `audit` and `check-links-online`.
`just initialize` runs the first five of them (`install-hooks` only from the primary
checkout), and is the one command a fresh clone needs the network for. Nothing inside
`just check` reaches it once `just sync` has fetched the registry index: `lock-check` and
`deny` then answer from that cache, and `lint` finds every hook environment already
prepared, because `install-hooks` prepared it. On a cold Cargo cache, `lock-check` fails
rather than fetching the index. `build`, `test`, `site`, `plan-spec`, `audit`, `mutants` and
`check-links-online` are outside `just check`, and so is every recipe that writes.

The gate numbers below are the order `just check` runs the gates in, which is the
`recipes` list in `pyproject.toml`. [Quality gates](quality-gates.md) says what each gate
proves and why.

## Setup

| Recipe | Purpose |
| --- | --- |
| `just initialize` | One explicit first run. Installs the pixi environment from `pixi.lock`, the pinned toolchain, the binaries `tools.txt` and `tools-source.txt` list and the pinned `allium` binary; fetches the Cargo dependencies; and installs the hook from the primary checkout; a secondary worktree skips the hook and says so. Over the network. Never formats, stages, commits, tags or pushes. |
| `just check-toolchain` | Refuse to go on unless `cargo` is rustup's proxy, then print the active toolchain; it does not check that toolchain against the pin in `rust-toolchain.toml`, which `RUSTUP_TOOLCHAIN`, `+toolchain` or a `rustup override` outranks. A cargo from pixi, Homebrew or a distribution ignores that file, so the gate refuses it before anything else runs. Gate 1. |
| `just install-toolchain` | Install the toolchain, components and targets `rust-toolchain.toml` names, through rustup. Over the network when any of them is missing. |
| `just install-tools` | Install the tools conda-forge lacks into `.tools/bin/` through the environment's cargo-binstall, in two passes. The `tools.txt` pass takes a release binary or fails, never compiling; the `tools-source.txt` pass, for a tool with no release binary, takes a signed cargo-quickinstall build if one exists and otherwise builds from crates.io source with `--locked`, without a GitHub token. A pass whose list is empty is skipped; a list file that is missing or unreadable fails the recipe. Over the network. |
| `just install-allium` | Download, verify and install the pinned `allium` binary into `.tools/bin/`, which Git ignores. Over the network; no lockfile can name a binary. |
| `just sync` | Install `Cargo.lock` and `pixi.lock` exactly as committed; never rewrites either. Run after pulling. Over the network. |
| `just lock` | Relock both: `Cargo.lock` at the versions the manifests allow, and `pixi.lock` against `pyproject.toml`. Over the network. |
| `just lock-upgrade` | `cargo update` and `pixi update`: move within the manifests' constraints. `pixi upgrade`, which rewrites the pins themselves, is not this recipe. Over the network. |
| `just lock-check` | Fail if a manifest and its lockfile disagree, for both lockfiles. Offline, and writes neither; fails on a cold Cargo cache until `just sync` has run. Gate 2. |
| `just install-hooks` | Install the read-only pre-commit gate and prepare every hook environment, so that `just lint` never fetches. Over the network. Run it from the primary checkout: every worktree shares one hooks directory, and the hook runs the environment of whichever worktree installed it. |

## Develop

| Recipe | Purpose |
| --- | --- |
| `just image` | Build the linux/amd64 container as `pachinko:dev` with Docker. Requires a running Docker daemon; outside `just check`. See [Build the container image](../how-to/build-the-container-image.md). |
| `just build` | Build every crate with every feature. Outside `just check`. |
| `just test` | Every unit and integration test through cargo-nextest with `INSTA_UPDATE=no`: a snapshot mismatch fails without writing pending files. Then every doctest through `cargo test --doc`, because nextest cannot run doctests. Outside `just check`, where `coverage` and `test-doc` run the same tests. |
| `just test-doc` | Every doctest. Gate 10. |
| `just semver [baseline]` | Report library API compatibility against an explicit Git revision, the last reachable `libpachinko-v*` tag, or, before the first release, the merge base with `origin/main` (`main` in a clone without that remote branch). Checks the locks, then runs cargo-semver-checks offline. Outside `just check`; advisory on pull requests until 1.0. |
| `just snapshots-check` | Check both locks, then run every unit and integration test with every feature through cargo-insta and nextest offline. cargo-insta cannot pass `--locked` through to nextest, so the locks are checked first. Fails mismatches without writing snapshots or pending files, and rejects unreferenced `.snap` files; doctests are left to gate 10. Gate 19. |
| `just snapshots-review` | Open cargo-insta's interactive review of pending snapshots for a person at a terminal. |
| `just snapshots-accept` | Check both locks, then rerun every unit and integration test with every feature through cargo-insta and nextest offline, and accept all pending snapshots without prompting, deleting any `.snap` file no test refers to. Writes and deletes `.snap` files; read the diff before committing and give the reason for the change. |

## Format

| Recipe | Purpose |
| --- | --- |
| `just format` | rustfmt and taplo, writing. |
| `just fix` | The mutating hook set, twice, because one fixer's first pass may fail on what another then repairs; then `cargo clippy --fix`; then `just lint`. The recipe that repairs what a check reports. Not the only one that writes: `just format`, `just initialize`, `just site`, `just snapshots-accept` and the lock recipes do too, and none of them is a check. |

## Check

| Recipe | Purpose |
| --- | --- |
| `just lint` | The whole read-only hook gate over every file. Gate 3. |
| `just fmt-check` | rustfmt, checking. Gate 4. |
| `just toml-check` | taplo's formatting check and its lint over every TOML file. Gate 5. |
| `just clippy` | Clippy over every crate target (the library, the binary, their tests, and any examples or benches) with every feature, for the host platform, with warnings as errors. The manifests say `warn`, so a local build never breaks mid-edit; this is where warnings become failures. Gate 6. |
| `just metrics` | rustqual against `rustqual.toml`, once for `crates/libpachinko` and once for `crates/pachinko`: complexity, cohesion and coupling thresholds, with warnings as failures. It first refuses any rustqual whose version is not the one `tools-source.txt` pins. Module-boundary rules wait for the first modules (decision 0012); the effect bans are gate 8, not this one. Gate 7. |
| `just effects` | libpachinko performs no effect itself (decision 0008; [Architecture](../explanation/architecture.md) explains the boundary). Runs `scripts/check-effects.sh`, which owns the mechanism and the crate bans; the std-API bans live in `crates/libpachinko/clippy.toml`. Offline, it checks libpachinko's normal dependency tree against the banned crates and its code against the std-API bans, then fails unless every ban fires on the probe under `tests/fixtures/effects-violation/` and none fires on `pachinko`. Gate 8. |
| `just features` | Every feature combination compiles, through cargo-hack's powerset. Not `--no-dev-deps`, which rewrites `Cargo.toml` while it runs. Gate 9. |
| `just coverage` | Every nextest test under instrumentation with `INSTA_UPDATE=no`, with the floor of 90 per cent of lines over `crates/*/src/**` enforced; a snapshot mismatch fails without writing pending files. Doctests are not measured; `test-doc` runs them. Writes `target/llvm-cov/lcov.info`. Gate 11. |
| `just doc` | rustdoc over the workspace with warnings as errors and `--cfg docsrs`. Gate 12. |
| `just site` | What GitHub Pages serves: empties `target/doc`, runs `just doc`, and writes a root `index.html` that links to the libpachinko and pachinko pages, because stable rustdoc writes none. Writes only under `target/`. Outside `just check`; the Pages workflow runs it on every push to `main`. [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) has the procedure. |
| `just deny` | cargo-deny's licence, ban and source checks over every dependency that ships, offline once `just sync` has run. Gate 13. |
| `just audit` | cargo-deny's advisory check against the RustSec database. Over the network, so outside `just check`; CI runs it on every pull request and weekly, in its own workflow. |
| `just mutants [args...]` | Mutation report over the workspace with every feature, using `cargo test` on a copy excluding Gitignored paths. Passes `--locked` to builds and tests. Writes `mutants.out/`, renaming the previous report to `mutants.out.old`. Outside `just check`; runs weekly and by hand. |
| `just deps-unused` | cargo-shear: a dependency a crate declares and never uses. Gate 14. |

## Documents

| Recipe | Purpose |
| --- | --- |
| `just check-docs` | markdownlint, `typos`, offline link check, then the documentation contract. Gate 15. |
| `just check-agents` | The agent contract: inventory, adapters, and skill bridges. Gate 16. |
| `just check-specs` | `allium check` over `docs/specs/`. Asserts that every module reports an empty `diagnostics` array; anything reported is a regression. Waiver terms: [Work with the specifications](../how-to/work-with-the-specs.md). Gate 17. |
| `just analyse-specs` | `allium analyse` over `docs/specs/`: the same structural diagnostics plus data flow, reachability, deadlocks and conflicts. Asserts that both arrays are empty; a finding cannot be waived, so any finding is a regression. Gate 18. |
| `just plan-spec <module>` | Print the raw JSON test obligations for `docs/specs/<module>.allium`; give the module name without the extension. Plans that module alone, reading no imports, so plan imported modules separately. A suggestion list: asserts nothing, writes nothing and is outside `just check`. Runs `.tools/bin/allium` by path, so it needs the pinned binary from `just install-allium` and never falls through to another `allium` on `PATH`. |
| `just check-links-online` | Follow external links. Manual; needs the network. |

The two spec gate recipes go through `bg-run-allium`, which reads the JSON rather than
trusting the exit code — `allium check` exits 0 on an `info` diagnostic and `allium
analyse` ignores diagnostics altogether. Both need the pinned binary, so a worktree that
has not run `just initialize` must run `just install-allium` first.

## Aggregate

| Recipe | Purpose |
| --- | --- |
| `just check` | The nineteen gates in order, proving the worktree is unchanged between each, then `check-clean`. |
| `just check-clean` | Assert the worktree is clean, or matches a supplied baseline. Runs last in `just check`. |

## Related pages

- [Quality gates](quality-gates.md)
- [Develop locally](../how-to/develop-locally.md)
- [Configuration](configuration.md)
