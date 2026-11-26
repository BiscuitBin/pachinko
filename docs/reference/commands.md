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

Fourteen recipes reach the network: `install-toolchain`, `install-tools`,
`install-allium`, `install-quint`, `install-apalache`, `install-smolvm`, `install-hooks`,
`sync`, `lock`, `lock-upgrade`, `audit`, `check-links-online`, `image` and `smolvm`;
`install-apalache` and `install-smolvm` only when the stamped install is not already in
place, and `smolvm` when smolvm pulls an image and whenever a guest reaches out. `quint` reaches it too the first time `run`, `test`, the
REPL or a `verify` that reports a violation needs the evaluator, whenever `verify` or
`compile` is given a remote `--server-endpoint`, and for Apalache only if a guarded `verify` or `compile`
runs while `just install-apalache` is replacing the tree, when Quint can download
Apalache unchecked.
`just initialize` runs the first eight of them (`install-smolvm` only on macOS on arm64,
`install-hooks` only from the primary checkout), and is the one command a fresh clone needs the network for. Nothing inside
`just check` reaches it once `just sync` has fetched the registry index: `lock-check` and
`deny` then answer from that cache, and `lint` finds every hook environment already
prepared, because `install-hooks` prepared it. On a cold Cargo cache, `lock-check` fails
rather than fetching the index. `build`, `test`, `site`, `plan-spec`, `audit`, `mutants`,
`bench`, `loadgen`, `test-loadgen`, `fuzz`, `semver`, `image`, `quint`, `smolvm`, `smolvm-purge` and `check-links-online` are outside `just check`, and
so is every recipe that writes. CI's `loadgen` job runs `test-loadgen`.

The gate numbers below are the order `just check` runs the gates in, which is the
`recipes` list in `pyproject.toml`. [Quality gates](quality-gates.md) says what each gate
proves and why.

## Setup

| Recipe | Purpose |
| --- | --- |
| `just initialize` | One explicit first run. Installs the pixi environment from `pixi.lock`, the pinned toolchain, the binaries `tools.txt` and `tools-source.txt` list, the pinned `allium` binary, the Quint CLI, the Apalache `tools/quint/apalache.txt` pins and, on macOS on arm64 alone, the smolvm `tools/smolvm/smolvm.txt` pins, printing a skip on any other host; fetches the Cargo dependencies; and installs the hook from the primary checkout; a secondary worktree skips the hook and says so. Over the network. Never formats, stages, commits, tags or pushes. |
| `just check-toolchain` | Refuse to go on unless `cargo` is rustup's proxy and the active toolchain is the pin, selected by `rust-toolchain.toml`, then print it. A cargo from pixi, Homebrew or a distribution ignores that file, and `RUSTUP_TOOLCHAIN` or a `rustup override` outranks it, so the gate refuses each before anything else runs. A `+toolchain` on a direct cargo call is out of its sight; no recipe passes one. Gate 1. |
| `just install-toolchain` | Install the toolchain and components `rust-toolchain.toml` names, through rustup. Over the network when any of them is missing. |
| `just install-tools` | Install the tools conda-forge lacks into `.tools/bin/` through the environment's cargo-binstall, in two passes. The `tools.txt` pass takes a release binary or fails, never compiling; the `tools-source.txt` pass, for a tool with no release binary, takes a signed cargo-quickinstall build if one exists and otherwise builds from crates.io source with `--locked`, without a GitHub token. A pass whose list is empty is skipped; a list file that is missing or unreadable fails the recipe. Over the network. |
| `just install-allium` | Download, verify and install the pinned `allium` binary into `.tools/bin/`, which Git ignores. Over the network; no lockfile can name a binary. |
| `just install-quint` | Install the Quint CLI from `tools/quint/package-lock.json` into `tools/quint/node_modules/`, which Git ignores, with `npm ci` from the pixi environment. Over the network. Not part of `sync`; see [decision 0021](../decisions/0021-project-managed-quint-cli.md). |
| `just install-apalache` | Download Apalache's `apalache.tgz` at the version `tools/quint/apalache.txt` pins, refuse it unless its SHA-256 matches the pin, and install it into `.quint/apalache-dist-<version>/apalache`, which Git ignores, with a stamp recording what it installed, where Quint looks before downloading one itself. Over the network, unless the stamped install is already in place, when it skips. Refuses a pin the installed CLI's `DEFAULT_APALACHE_VERSION_TAG` disagrees with, so it needs `just install-quint` first. It also refuses a `QUINT_HOME` other than `.quint`, and a symlink or non-directory at `.quint/`, its `apalache-dist-<version>/` or the tree; remove that path (a symlink itself, not its target) and run it again. Otherwise it replaces any tree its stamp does not match. See [decision 0021](../decisions/0021-project-managed-quint-cli.md). |
| `just install-smolvm` | Download the smolvm release tarball `tools/smolvm/smolvm.txt` pins, refuse it unless its SHA-256 matches the pin, check its members, unpack it with the archive's modes, check it against the release's own `checksums.txt` and check the signature and Hypervisor entitlement of `smolvm-bin`, read-only, then stamp it and move it into `.tools/smolvm/`, which Git ignores. Over the network, unless the stamped install is already in place, when it skips. macOS on arm64 only; it refuses any other host. It refuses a symlink or non-directory at `.tools` or `.tools/smolvm` (remove the symlink itself, not its target), and refuses at once while another process holds the install lock, `.tools/smolvm.lock`. Otherwise it replaces any tree its stamp does not match. It never runs, signs or unsigns the binary. See [decision 0022](../decisions/0022-quint-studio-machine.md). |
| `just sync` | Install `Cargo.lock`, `fuzz/Cargo.lock`, `loadgen/Cargo.lock` and `pixi.lock` exactly as committed; never rewrites them. Run after pulling. Over the network. |
| `just lock` | Relock `Cargo.lock`, `fuzz/Cargo.lock` and `loadgen/Cargo.lock` at the versions the manifests allow, `pixi.lock` against `pyproject.toml`, and `tools/quint/package-lock.json` against its `package.json`. Over the network. |
| `just lock-upgrade` | `cargo update` for the three Rust locks, `pixi update` and `npm update --package-lock-only` for the Quint CLI's: move within the manifests' constraints. `pixi upgrade`, which rewrites the pins themselves, is not this recipe. Over the network. |
| `just lock-check` | Fail if a manifest and its lockfile disagree, for all five lockfiles, the Quint CLI's included, or if the `quint=` line of `tools/quint/apalache.txt`, as it is in the checkout, is not the Quint CLI version both npm files pin, so a move of the CLI fails until the Apalache pin is reviewed; it checks that the review was recorded, not that it was right. It also fails unless `tools/smolvm/smolvm.txt`, the smolvm pin, holds exactly `version=` (`x.y.z`), `asset=` (that version's darwin-arm64 tarball) and `sha256=` (64 lowercase hex digits), reading that file alone, with a `tools/smolvm/smolvm.txt: <key>: <reason>` line. Offline; installs nothing and rewrites no lockfile, though npm writes its log under its own cache; fails on a cold Cargo cache until `just sync` has run. Gate 2. |
| `just install-hooks` | Install the read-only pre-commit gate and prepare every hook environment, so that `just lint` never fetches. Over the network. Run it from the primary checkout: every worktree shares one hooks directory, and the hook runs the environment of whichever worktree installed it. |

## Develop

| Recipe | Purpose |
| --- | --- |
| `just bench [args…]` | Run the divan benchmarks manually, the `version` smoke bench and W1 to W5 over the queue, with optional Cargo arguments. Never in CI or `just check`. See [Benchmarks](testing.md#benchmarks). |
| `just loadgen <workload> [--name value…]` | Run the load generator or the lapse probe in the excluded `loadgen/` crate against a running `pachinko serve`; `just loadgen --help` lists the workloads and flags. A run with any error prints no rate and exits non-zero. Never in CI or `just check`. See [Load generation](testing.md#load-generation). |
| `just test-loadgen` | Run the excluded `loadgen/` crate's unit tests, which `just test` cannot reach. Outside `just check`; CI's `loadgen` job runs it. |
| `just python-double` | Run the Python double's `unittest` suite under `double/tests/` on pixi's Python, refusing a Python older than 3.14 and printing the version it found. Runs `python -B`, so it writes no `__pycache__/`, and `-t double`, so the suite's `load_tests` runs the coverage test last. Prints the number of tests run. Outside `just check` and CI. |
| `just fuzz [target] [seconds]` | Fuzz on pinned stable with no sanitizer; defaults to `smoke` and 300 seconds. Checks all five locks, then runs offline. Compilation adds time. See [Fuzzing](testing.md#fuzzing) for prerequisites and crash output. |
| `just image` | Build the linux/amd64 container as `pachinko:dev` with Docker. Requires a running Docker daemon; outside `just check`. Over the network: the build pulls the digest-pinned base images and fetches the crates inside the container. See [Build the container image](../how-to/build-the-container-image.md). |
| `just build` | Build every crate with every feature. Outside `just check`. |
| `just test` | Every unit and integration test through cargo-nextest with `INSTA_UPDATE=no`: a snapshot mismatch fails without writing pending files. Then every doctest through `cargo test --doc`, because nextest cannot run doctests. Outside `just check`, where `coverage` and `test-doc` run the same tests. |
| `just test-doc` | Every doctest. Gate 10. |
| `just test-doc-default` | Every doctest with default features, libpachinko and pachinko each in its own `cargo test --doc --package` run, after `cargo tree` shows that neither package's dependencies enable `testing`, so a doc example that needs the `testing` feature fails unless it guards its lines with `# #[cfg(feature = "testing")]`. Gate 20. |
| `just semver [baseline]` | Report library API compatibility against an explicit Git revision, the last reachable `libpachinko-v*` tag, or, before the first release, the merge base with `origin/main` (`main` in a clone without that remote branch). Checks all five locks, then runs cargo-semver-checks offline. Outside `just check`; advisory on pull requests until 1.0. |
| `just snapshots-check` | Check all five locks, then run every unit and integration test with every feature through cargo-insta and nextest offline. cargo-insta cannot pass `--locked` through to nextest, so the locks are checked first. Fails mismatches without writing snapshots or pending files, and rejects unreferenced `.snap` files; doctests are left to gate 10. Gate 19. |
| `just snapshots-review` | Open cargo-insta's interactive review of pending snapshots for a person at a terminal. |
| `just snapshots-accept` | Check all five locks, then rerun every unit and integration test with every feature through cargo-insta and nextest offline, and accept all pending snapshots without prompting, deleting any `.snap` file no test refers to. Writes and deletes `.snap` files; read the diff before committing and give the reason for the change. |

## Format

| Recipe | Purpose |
| --- | --- |
| `just format` | rustfmt and taplo, writing. |
| `just fix` | The mutating hook set, twice, because one fixer's first pass may fail on what another then repairs, over every tracked file and then over every untracked file Git does not ignore; then `cargo clippy --fix`; then `just lint`. The trailing-whitespace fixer leaves the Markdown under `tickets/` alone, because a hand-back quotes command output verbatim; markdownlint still repairs a trailing space in ticket prose. The recipe that repairs what a check reports. Not the only one that writes: `just format`, `just initialize`, `just site`, `just snapshots-accept`, `just diagrams` and the lock recipes do too, and none of them is a check. |

## Check

| Recipe | Purpose |
| --- | --- |
| `just lint` | The whole read-only hook gate over every tracked file, then over every untracked file Git does not ignore, which prek's `--all-files` leaves out; the large-file check reads staged files only, so it passes an untracked file. Stages nothing. An untracked path that begins with `-` fails the gate by name, because a hook would read it as an option. An untracked scratch file belongs under `ai_tmp/`, or its faults fail the gate. Gate 3. |
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
| `just quint <args>` | Run the pinned Quint CLI by its package's entry point, `tools/quint/node_modules/@informalsystems/quint/dist/src/cli.js`, the package the guard checks, never the `node_modules/.bin` shim, so it needs `just install-quint` and never falls through to another `quint` on `PATH`; for example `just quint typecheck <file>.qnt`. `typecheck` is offline; `run`, `test` and the REPL download the evaluator into `.quint/`, which Git ignores, on first use, unchecked. A guard runs first, offline and read-only: it refuses any option whose name contains `apalache` except `--apalache-config`, and, unless there are no arguments (the REPL), the first is `parse`, `typecheck`, `run`, `test`, `repl` or `docs`, or the only one is `--help` or `--version`, it refuses to go on without the stamped install `just install-apalache` made and pixi's Java, naming `just install-apalache` or `just sync`. So `verify`, every `compile` whatever its target, `verify --help` and a leading option need both. It refuses a `QUINT_HOME` other than `.quint`, and holds for non-concurrent use only: a run that starts while `just install-apalache` is replacing the tree can make Quint download Apalache unchecked. Apalache runs on pixi's `openjdk`, which this recipe alone puts on `PATH`, and on the jar in `.quint/`, since the recipe unsets `APALACHE_JAR`, `BASH_ENV`, `CDPATH` and the variables that pass options to the JVM (`JVM_ARGS`, `JVM_GC_ARGS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS`), and the guard refuses an exported bash function, which the launcher would import; both commands use any Apalache server already on `localhost:8822`, or the one `--server-endpoint` (or `--serverEndpoint`) names, before starting their own, and the guard cannot see which Apalache that runs; `verify` reports a violation through the evaluator, downloading it on first use, and a remote `--server-endpoint` is a network connection. Not a gate and outside `just check`; see [decision 0021](../decisions/0021-project-managed-quint-cli.md). |
| `just smolvm <args>` | Run the stamped smolvm, `.tools/smolvm/smolvm`, by path, never a `smolvm` on `PATH`, with its machine state outside the worktree, in the state directory `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>/` reached through the state link `/tmp/pachinko-smolvm-<hash>`, made again after a restart; for example `just smolvm machine ls`. One process, `scripts/smolvm.sh`: it takes the install lock and runs `scripts/install-smolvm.sh --check`, naming `just install-smolvm` when it fails; refuses a caller's `SMOLVM_*`, `DYLD_*`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME` or `DOCKER_CONFIG`, a `HOME` other than the user database's, an exported shell function and a `--state-dir`; takes the state lock beside the state directory; makes or checks the state; then prints `smolvm launcher: running the stamped smolvm` on standard error and runs smolvm in a cleared environment. Every refusal exits 1 before smolvm starts. It prints only on standard error, and holds both locks until smolvm ends, so another smolvm command for the worktree or its purge is refused meanwhile, naming the holder, and so is a reinstall; a machine that goes on running holds neither, and only `kill -9` of the launcher alone frees them early. Under a state made with another pin it runs only `machine ls`, `machine stop` and `machine delete`. Its arguments pass through, mounts included. Over the network when smolvm pulls an image and whenever a guest reaches out. macOS on arm64 only; not a gate and outside `just check`. [Develop locally](../how-to/develop-locally.md#smolvm) and [decision 0022](../decisions/0022-quint-studio-machine.md) have the rest. |
| `just smolvm-purge` | Remove this worktree's smolvm state, the state link and the state directory, and nothing else. It needs the stamped install, takes both locks and checks the state as `just smolvm` does, then takes smolvm's own listing of the machines there; it refuses, removing nothing, when the listing fails or lists any machine, naming each and how to delete it. With no state it says so and exits 0. It keeps the state lock, the directories above the state directory and `.tools/smolvm-state-id`, and accepts a state made with another pin. `just smolvm-purge --orphan <worktree>-<hash>` does the same for the state of a worktree that is gone, named by its directory's name under `~/Library/Caches/pachinko/smolvm/`, and refuses one whose worktree still exists and holds that state's id; `--orphan <worktree>-<hash> --delete <machine>` deletes one machine that orphan's listing holds. Outside `just check`. See [Develop locally](../how-to/develop-locally.md#removing-the-state). |
| `just diagrams` | Redraw the SVG diagrams under `docs/decisions/0020/` from `scripts/diagrams.py`, standard-library Python run by the pixi environment's interpreter. The script owns each diagram's layout and words, so edit it and rerun, never the SVG. Writes, so it is outside `just check`, and nothing checks that the files match the script. |
| `just check-links-online` | Follow external links. Manual; needs the network. |

The two spec gate recipes go through `bg-run-allium`, which reads the JSON rather than
trusting the exit code — `allium check` exits 0 on an `info` diagnostic and `allium
analyse` ignores diagnostics altogether. Both need the pinned binary, so a worktree that
has not run `just initialize` must run `just install-allium` first.

## Aggregate

| Recipe | Purpose |
| --- | --- |
| `just check` | The twenty gates in order, proving the worktree is unchanged between each, then `check-clean`. |
| `just check-clean` | Assert the worktree is clean, or matches a supplied baseline. Runs last in `just check`. |

## Related pages

- [Quality gates](quality-gates.md)
- [Develop locally](../how-to/develop-locally.md)
- [Configuration](configuration.md)
