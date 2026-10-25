---
title: "Quality gates"
kind: "reference"
audience: [contributor, maintainer, agent]
canonical_for: [quality_gate_reference]
requires: []
---

# Quality gates

`just check` runs the gates in the order below and snapshots the worktree between each
one. A recipe that modifies a file Git does not ignore fails the run, because checks are
read-only; an ignored path is outside the snapshot, which is why every build output below
is ignored. The order is the `recipes` list under `[tool.biscuit-games-tooling]` in
`pyproject.toml`, and `check-clean` follows it.

| Order | Gate | Runs | Proves |
| --- | --- | --- | --- |
| 1 | `check-toolchain` | `rustup show active-toolchain` | `cargo` is rustup's proxy, and the active toolchain is `rust-toolchain.toml`'s channel, selected by that file rather than by `RUSTUP_TOOLCHAIN` or a `rustup override`; the run log names it. A `+toolchain` on a direct cargo call is out of the recipe's sight, and no recipe passes one. |
| 2 | `lock-check` | `cargo update --workspace --locked --offline`, the same over `fuzz/Cargo.toml`, `pixi lock --check --offline --dry-run` | `Cargo.toml` agrees with `Cargo.lock`, `fuzz/Cargo.toml` with `fuzz/Cargo.lock`, and `pyproject.toml` with `pixi.lock`. |
| 3 | `lint` | `prek run --all-files` | The whole hook gate passes over every file. |
| 4 | `fmt-check` | `cargo fmt --all --check`, and the same over `fuzz/Cargo.toml` | Every Rust file, the fuzz crate's included, is as rustfmt would write it. |
| 5 | `toml-check` | `taplo fmt --check`, `taplo lint` | Every TOML file is as taplo would write it, and passes taplo's lint. |
| 6 | `clippy` | `cargo clippy -- -D warnings`, then the same over `fuzz/Cargo.toml` | Every lint in the workspace table is clean over every crate target and feature, and every lint in the fuzz crate's copy of it over the fuzz targets, for the host platform, with warnings as errors. |
| 7 | `metrics` | rustqual, once per crate | Every function is within the complexity, length, nesting and parameter thresholds and none that holds logic calls itself, every struct within the cohesion and size thresholds, every file within the length threshold, and every module within the coupling thresholds, in `crates/libpachinko` and in `crates/pachinko`. |
| 8 | `effects` | `scripts/check-effects.sh` | libpachinko's normal dependency tree, on every target, names no banned crate; libpachinko calls no banned std API in what clippy compiles for the host with every feature, with none, and with every feature in the release profile; and the probe fixture trips every ban once while pachinko trips none. |
| 9 | `features` | `cargo hack check --feature-powerset` | Every feature combination compiles. |
| 10 | `test-doc` | `cargo test --doc` | Every doc example compiles and passes. |
| 11 | `coverage` | `cargo llvm-cov nextest` | Every unit and integration test passes, and line coverage over `crates/*/src/**` is at or above the floor of 90%. Doctests are gate 10's. |
| 12 | `doc` | `cargo doc` with `-D warnings --cfg docsrs` | rustdoc is warning-free under `--cfg docsrs`, intra-doc links and missing docs included. |
| 13 | `deny` | `cargo deny check licenses bans sources` | Every dependency that ships has an allowed licence, no banned crate is in that graph, and every source is crates.io. Development dependencies are outside the graph. |
| 14 | `deps-unused` | `cargo shear` | No crate declares a dependency it does not use. |
| 15 | `check-docs` | markdownlint, typos, lychee, `bg-validate-docs` | The documentation contract holds. |
| 16 | `check-agents` | `bg-validate-agents` | The agent contract holds. |
| 17 | `check-specs` | `bg-run-allium check` | Every specification reports an empty `diagnostics` array. |
| 18 | `analyse-specs` | `bg-run-allium analyse` | Every specification reports an empty `findings` array too. |
| 19 | `snapshots-check` | `cargo insta test --check` | Every snapshot matches, and no `.snap` file is unreferenced. Runs `lock-check` first, then all tests through cargo-insta and nextest, offline, with every feature, without writing snapshots or pending files. |
| 20 | `check-clean` | `bg-project-check clean` | The run changed nothing. |

The table leaves out `--locked`, which the recipes
[decision 0007](../decisions/0007-dependency-policy.md) names pass; `deps-unused` does
not, and `snapshots-check` cannot pass it through cargo-insta, which is why it runs
`lock-check` first.

`fuzz/` is a workspace of its own, so gates 2, 4 and 6 name its manifest beside the
root one. Its clippy pass stays inside `just check` by the maintainer's choice on
2026-10-24: it costs 7.87 s on a cold cache and about 0.06 s warm.

Gates 4, 5 and 14 to 18 also run inside `lint`, as its hooks.
The duplication is deliberate: a regression names itself in the list of gates rather than
being one line inside `lint`.

Gates 17 and 18 cost the gate something real: the pinned `allium` binary lives in the
gitignored `.tools/bin/`, which is a per-worktree install, so a worktree that has never
run `just initialize` fails `just lint` and `just check` until `just install-allium`
puts one there. The alternative was a gate that skipped itself whenever its tool was
absent, which asserts nothing.
[Decision 0005](../decisions/0005-project-managed-allium-cli.md) is the record.

Neither gate trusts the tool's exit code, because neither exit code means what this
project means by clean. `allium check` exits 0 on an `info` diagnostic —
`allium.field.unused` is one — and `allium analyse` keys its status on findings alone and
ignores diagnostics entirely, so a module that does not parse passes it with the `error`
sitting in the JSON it has just printed. `bg-run-allium` runs the subcommand,
prints its output whole, and asserts what the contract actually says: every module reports
an empty `diagnostics` array and an empty `findings` array. A diagnostic may be waived
only where the checker itself is wrong, on the terms in
[Work with the specifications](../how-to/work-with-the-specs.md); a finding cannot be
waived at all.

Gate 7 does not trust a green run either. rustqual skips its architecture section without
a word when the section is switched off or misconfigured, and exits 0 over a directory that
does not exist; either way it reports every boundary as met. And its version decides what
it reports, so
`just metrics` first fails unless `rustqual --version` matches the pin in
`tools-source.txt`, rather than let a missing `.tools/bin/rustqual` fall through to
another on `PATH`. Then it runs rustqual twice against `rustqual.toml`, over
`crates/libpachinko` and over `crates/pachinko`, each with warnings as failures. The
module-boundary rules wait for there to be modules to order, as
[Layering](../explanation/layering.md) records, so there is no rule to probe yet; the
effect bans, which are boundary rules of a kind, are gate 8's, with a probe of their
own.
Nothing proves the complexity, cohesion and coupling sections are running with the numbers
written, and [Decision 0012](../decisions/0012-metrics-gate.md) records that as a risk it
accepted. Like gates 17 and 18, gate 7 needs a per-worktree binary: `just install-tools`
builds `.tools/bin/rustqual` from source, and a worktree without it fails the gate.

## Gate 8: the effects bans

libpachinko performs no effect itself;
[Architecture](../explanation/architecture.md#gate-8-how-the-boundary-is-enforced) lists
the bans and explains the boundary, and
[Decision 0008](../decisions/0008-std-library.md) why a gate holds it. `just effects`
runs `scripts/check-effects.sh`, which applies to libpachinko alone.

The crate bans are a list at the top of the script, read against
`cargo tree -p libpachinko -e normal --all-features --target all --prefix none --locked
--offline`, which covers transitive and optional dependencies on every target. The
std-API bans are clippy's `disallowed-methods` and `disallowed-types` entries in
`crates/libpachinko/clippy.toml` whose reason begins `effect:`. The script runs offline,
in four checks:

0. `crates/libpachinko/clippy.toml`, less comments and its effect entries, still reads
   line for line as the root `clippy.toml`, which it shadows, and the root file holds no
   effect entry.
1. libpachinko names no banned crate, after a positive control that fails unless the
   tree names libpachinko itself, and uses no banned std API in three clippy passes:
   every feature, no default features, and every feature in the release profile.
2. The fixture under `tests/fixtures/effects-violation/` trips every ban exactly once,
   read by the same matchers checks 1 and 3 decide with, and its path dependencies under
   `deps/` and the `// ban:` tags on its breaks name exactly the two ban lists, so a ban
   deleted from one list but not the other fails the gate.
3. pachinko, linted with the root file, trips no std-API ban.

The script pins cargo's colour, progress bar, warnings level and debug assertions, so
the caller's settings cannot hide a finding. A lint cap in the caller's `RUSTFLAGS` or a
cargo config's `rustflags` is not overridden: the probe then sees nothing fire, fails,
and names that cause. Clippy lints only what its three passes compile, so the following
are left to review: code behind another target's `cfg`, such as `cfg(windows)`; code
under a mix of features none of the passes sets; code under `cfg(not(clippy))` or a
custom `cfg`; and a std item added after the inventory. `std::env::consts` stays
reachable, because its items are compile-time constants rather than effects, and the
crate bans are a denylist of eight names, not an allowlist. These limits stand by the
maintainer's choice on 2026-10-24.

A failure names the banned crate or std path and where it was found. Move the effect
behind a trait libpachinko defines, with the real implementation in `pachinko` and a fake
beside the trait; a banned crate a test needs belongs in the development dependencies.
The bans apply to libpachinko's own unit and integration tests too, by the maintainer's
choice on 2026-10-24, so a test that needs a real effect belongs under
`crates/pachinko/tests/`.

## Outside the gate

`test`, `build`, `audit`, `mutants`, `bench`, `fuzz`, `semver`, `image`, `site`, `plan-spec`
and `check-links-online` are recipes that `just check` does not run. `test` runs nextest
and the doctests in one go, which `coverage` and `test-doc` already cover between them;
`build` compiles what `clippy` and `features` already compile; `site` builds the API
reference that `doc` has proved, for
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md); and `plan-spec` prints a
suggestion list for tests, which nothing asserts on. `check-links-online` needs the
network, and a check that can fail because a third party is down is not a gate. `audit`
is missing for the same reason: it reads the RustSec advisory database, and runs in a
workflow of its own. Mutation testing is a report because runtime grows with the
workspace and survivors require judgement; there is no score threshold. Benchmarks run
by hand, because machine load makes a timing no gate, and fuzzing runs weekly in a
workflow of its own ([decision 0017](../decisions/0017-mutation-bench-and-fuzz.md));
`semver` is advisory before 1.0 ([decision 0016](../decisions/0016-release-process.md));
and `image` needs Docker, which the source gate does not, so CI's `image` job builds the
image instead ([decision 0015](../decisions/0015-container-image.md)).
[Commands](commands.md) lists every recipe.

## When a gate fails

`just check` stops at the first gate that fails, so the last recipe it ran is the one
that failed. Rerun that one recipe on its own and read its first failure; fix the cause,
not the finding. Never disable a gate or lower a threshold to make a run green: a
suppression is one lint on one line, as
`#[expect(lint, reason = "...")]`. The `fix-quality` skill is the agent's procedure, and
[Troubleshooting](../operations/troubleshooting.md) has the messages that come up most.

| Gate | First move |
| --- | --- |
| `check-toolchain` | Put rustup's cargo first on `PATH`; `just install-toolchain` installs the pinned toolchain; unset `RUSTUP_TOOLCHAIN` or run `rustup override unset`. |
| `lock-check` | A manifest changed without its lockfile: `just lock`, then commit both. A cold Cargo cache fails too: `just sync` fills it. |
| `lint` | Read which hook failed. `just fix` repairs what the mutating configuration can. |
| `fmt-check`, `toml-check` | `just fix`, or `just format` for these two alone. |
| `clippy` | Fix the code. `just fix` applies clippy's own suggestions, then the rest is by hand. |
| `metrics` | Split the function, type or module that crossed a threshold. A version mismatch: `just install-tools`. |
| `effects` | Move the effect behind a trait libpachinko defines, with the real implementation in `pachinko` and a fake beside the trait. |
| `features` | Fix the combination the output names. |
| `test-doc`, `coverage` | Fix the failing test or example; below the floor, test the uncovered lines or remove them. |
| `doc` | Document the item, or repair the link rustdoc names. |
| `deny` | Replace the dependency whose licence, source or name is refused. |
| `deps-unused` | Remove the dependency; `just fix` runs `cargo shear --fix`. |
| `check-docs`, `check-agents` | The [Documentation contract](documentation-contract.md) or the [Agent contract](agent-contract.md) names the rule. |
| `check-specs`, `analyse-specs` | `just install-allium` if the binary is missing; otherwise fix the module. |
| `snapshots-check` | `just snapshots-review`, or `just snapshots-accept` and read the diff before committing. |
| `check-clean` | Find the recipe that wrote the file. A recipe that changes the worktree is a defect in that recipe. |

`just lock`, `just sync`, `just install-toolchain`, `just install-tools` and
`just install-allium` reach the network, and an agent runs them only when its task
authorises it.

## What the hook gate contains

`just lint` runs `.pre-commit-config.yaml` over every file. This is the read-only
configuration, and it is the one installed as the pre-commit hook.

| Hook | Checks |
| --- | --- |
| `fmt-check`, `toml-check`, `deps-unused` | Gates 4, 5 and 14, when a Rust file, a TOML file or a manifest is among the files. Each runs through its `just` recipe: a git hook does not inherit the `Justfile`'s `PATH`, so routing through the recipe is what lets it find the environment's tools, and it keeps each tool's arguments in one place. |
| `validate-docs`, `validate-agents` | The two contracts, so a hook catches them before the aggregate does. |
| `check-specs`, `analyse-specs` | The specifications, through `allium`. Needs the pinned binary; see above. |
| `editorconfig-checker` | Whitespace, line endings, final newlines. |
| `markdownlint-cli2` | Markdown structure. |
| `typos` | Spelling, excluding the lockfiles. |
| `lychee` | Link targets, offline. |
| `shellcheck` | The shell scripts under `scripts/`. |
| `actionlint` | Every GitHub Actions workflow, its structure only — see below. |
| `ripsecrets` | Credential material, with its output suppressed so a match is never logged. |
| Builtin `check-*` | Large files, case conflicts, merge markers, JSON, TOML, YAML, private keys, shebangs. |

The four contract and specification hooks run their `bg-*` scripts through
`pixi run --frozen`, because a git hook does not inherit the `Justfile`'s `PATH`, and a
bare `pixi run` could rewrite `pixi.lock` from inside a commit. Clippy is deliberately not
a hook: a cold build over every target blocks a commit for minutes, so it is gate 6 and a
CI step instead.

Third-party hooks are pinned to commit SHAs with a version comment beside each.

One gap is worth knowing about rather than being surprised by. `actionlint` analyses a
`run:` block by handing it to `shellcheck`, and it reports nothing at all when it cannot
find `shellcheck` on its own `PATH`. Under `prek` each hook gets its own environment, so
the `shellcheck` hook one row up is not the one `actionlint` can see, and the shell
embedded in a workflow goes unread. Every `run:` in this repository's workflows and its
setup action is a single line, nearly all of them one `just` recipe, so there is no
embedded shell to miss today. A block of more than a line
added to a workflow deserves `shellcheck` by hand until the gap is closed.

## The mutating counterpart

`.pre-commit-fix.yaml` holds the hooks that write: `cargo fmt`, `taplo fmt`,
`cargo shear --fix`, end-of-file and trailing-whitespace repair, and
`markdownlint --fix`. It is never installed as a hook and runs only from `just fix`.
`cargo clippy --fix` is not among them: it needs a full build and permission to touch a
dirty tree, so it is a line in the `fix` recipe itself.

## In continuous integration

`.github/workflows/ci.yml` runs the same recipes in four gate jobs, each on Ubuntu and
each starting with the repository's composite setup action, which installs the pixi
environment, the toolchain, the tools `tools.txt` and `tools-source.txt` pin, and the hook
environments.

| Job | Runs |
| --- | --- |
| `rust` | `check-toolchain`, `lock-check`, `fmt-check`, `toml-check`, `clippy`, `metrics`, `effects`, `features`, `test`, `snapshots-check`, `doc`, `deps-unused` |
| `coverage` | `coverage`, then uploads `lcov.info` as an artifact kept for seven days |
| `deny` | `deny` |
| `documents` | `install-allium`, `lint`, `check-docs`, `check-agents`, `check-specs`, `analyse-specs` |

A fifth gate job, `image`, runs no recipe and no setup action: it builds the container
image with Docker, loads it without pushing it, and runs it once
([decision 0015](../decisions/0015-container-image.md)). A sixth job, `check`, needs all
five and runs no recipe: it fails when any of them failed, was cancelled or was skipped.
It exists so that branch protection can name one check that never changes. A gate job
added later joins its `needs`, and protection stays as it is. A seventh, `semver`, runs
`just semver` on pull requests only; it is advisory before 1.0 and is not among `check`'s
`needs` ([decision 0016](../decisions/0016-release-process.md)).

A newer run of `ci.yml` on the same ref cancels the one still in progress, on `main` as on
a branch, by the maintainer's choice on 2026-10-24.

The mapping is not one to one. `rust` runs `test`, where the gate runs `test-doc` and
`coverage`; `lint` runs in `documents`; and each gate job that runs recipes ends with
`check-clean`, so a recipe that rewrites a tracked file or leaves an untracked one fails
in CI as it does in `just check`. Past the composite setup action and `image`'s Docker
steps, nothing in CI runs a command that does not exist in the `Justfile`.

`.github/workflows/audit.yml` runs `just audit` on every pull request, weekly, and by
hand. It is never a required check: a fetch of the advisory database can fail for reasons
that have nothing to do with the diff.

`.github/workflows/mutants.yml` runs `just mutants` every Monday at 07:00 UTC and by
hand, with a thirty-minute timeout. It uploads `mutants.out/` even after a failure unless
cancelled, retaining reports for 28 days. It runs on no pull request, is never required,
and is never among `check`'s `needs`. Raise the timeout when the mutation step takes
more than half of it.

`.github/workflows/pages.yml` runs on every push to `main`, and by hand. Its `build` job
runs `check-toolchain` and `site` after the same setup action and uploads `target/doc`;
its `deploy` job publishes that to GitHub Pages, and only from `main`. It is not a gate:
neither job is among `check`'s `needs`, so it is never a required check, and a
deployment that fails blocks no merge.
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) has the procedure.

## On `main`

`main` is protected, and one check, `check`, must pass before a branch merges into it.
That is the aggregate job above, which fails unless `rust`, `coverage`, `deny`,
`documents` and `image` all succeeded, so a gate job is added by joining its `needs` and
the protection itself never changes. `audit`, `mutants` and `semver` are never required.

The branch is not required to be up to date with `main` first, and no review is required —
neither earns its cost on a repository with one author. Force pushes and deletion are
refused. Administrators are not bound by the rule, so the direct push remains available
when it is genuinely wanted; the protection is there to stop an unproved merge, not to stop
the author.

A push to `main` publishes the API reference through `pages.yml`. That deployment is not
a gate and not a required check.

## Related pages

- [Commands](commands.md)
- [Architecture](../explanation/architecture.md)
- [Quality philosophy](../explanation/quality-philosophy.md)
- [Documentation contract](documentation-contract.md)
- [Agent contract](agent-contract.md)
- [Troubleshooting](../operations/troubleshooting.md)
- [Decision 0008](../decisions/0008-std-library.md)
- [Decision 0009](../decisions/0009-rust-quality-gate.md)
- [Decision 0012](../decisions/0012-metrics-gate.md)
