set positional-arguments := true
set shell := ["sh", "-eu", "-c"]

# pixi owns every tool binary and the Python environment (.pixi/envs/default;
# pyproject.toml is the manifest, pixi.lock the pin). Tools conda-forge lacks
# (tools.txt, and tools-source.txt for those with no release binary) live in
# .tools/bin, and so does the Allium checker. Every recipe, and every hook that
# runs through a recipe, sees both first; cargo finds cargo-nextest and friends
# on PATH by name. Neither directory holds a cargo, so rustup's proxy stays
# first for the compiler (check-toolchain proves it).
export PATH := justfile_directory() / ".pixi" / "envs" / "default" / "bin" + ":" + justfile_directory() / ".tools" / "bin" + ":" + env("PATH")

# Line-coverage floor over crates/*/src/**. Lower the complexity, not the
# number.
coverage_floor := "90"

default:
    @just --list

# ------------------------------------------------------------------ setup ---

# One explicit first-run command. Never stages, commits, tags, or pushes.
initialize:
    sh scripts/initialize.sh

# The pin in rust-toolchain.toml is honoured only by rustup's cargo proxy. A
# cargo from pixi, Homebrew or a distribution ignores the file and builds with
# whatever it is, so the gate refuses it before anything else runs.
check-toolchain:
    command -v rustup >/dev/null || { printf '%s\n' 'rustup is not installed; see docs/how-to/develop-locally.md' >&2; exit 2; }
    case "$(command -v cargo)" in "${CARGO_HOME:-$HOME/.cargo}"/bin/cargo) ;; *) printf '%s\n' "cargo resolves to $(command -v cargo), not rustup's proxy" >&2; exit 2;; esac
    rustup show active-toolchain

# Installs the toolchain, components and targets rust-toolchain.toml names.
install-toolchain:
    rustup toolchain install

# Tools conda-forge lacks into .tools/bin (network). cargo-binstall comes from
# the pixi environment. tools.txt is binary-only: a release archive or nothing.
# tools-source.txt holds tools with no release binary, so its loop may compile,
# and cargo-binstall passes --locked through to `cargo install`. It tries a
# signed cargo-quickinstall build first, and would take one that appeared for
# the pin; --only-signed refuses an unsigned one, and an upstream release
# asset, which nothing would sign, is refused too (decision 0012). That pass runs without a GitHub token, so no build script it
# compiles can read one; its one quickinstall lookup needs no rate-limit lift.
# A pass whose list is empty is skipped, since cargo binstall with no crate
# fails; the guard is plain sh, so it holds for GNU and BSD alike. sed, not
# grep, strips the comments: it exits 0 on a list with no pins but still fails,
# and so fails the recipe, when the file is missing or unreadable.
install-tools:
    pins=$(sed -e '/^#/d' -e '/^$/d' tools.txt); [ -z "$pins" ] || cargo binstall --root .tools --no-confirm --locked --disable-strategies compile $pins
    pins=$(sed -e '/^#/d' -e '/^$/d' tools-source.txt); [ -z "$pins" ] || env -u GITHUB_TOKEN -u GH_TOKEN cargo binstall --root .tools --no-confirm --locked --disable-strategies crate-meta-data --only-signed --no-discover-github-token $pins

# The Allium checker for docs/specs/, pinned and checksummed in the tooling
# package. Downloads over the network into .tools/bin, which Git ignores.
install-allium:
    bg-install-allium

# All three lockfiles, exactly as committed. Never rewrites any.
sync:
    cargo fetch --locked
    cargo fetch --locked --manifest-path fuzz/Cargo.toml
    pixi install --frozen

lock:
    cargo update --workspace
    cargo update --workspace --manifest-path fuzz/Cargo.toml
    pixi lock

# Within the manifest's constraints only; every direct pin is exact, so this
# moves transitives (openssl, libcxx, the Python patch level). `pixi upgrade`
# rewrites the pins themselves and is not this recipe.
lock-upgrade:
    cargo update
    cargo update --manifest-path fuzz/Cargo.toml
    pixi update

# Offline, like every gate recipe: a cold Cargo cache fails rather than
# fetching the registry index (`just sync` fills it), and an offline solve on a
# stale lock still fails, which is the right answer. --dry-run because pixi
# would otherwise rewrite a stale lock from that restricted solve.
lock-check:
    cargo update --workspace --locked --offline
    cargo update --workspace --locked --offline --manifest-path fuzz/Cargo.toml
    pixi lock --check --offline --dry-run

# The shim, then every hook environment (seven clones; Go under prek's cache;
# the ripsecrets build with the machine's rustup and the pinned toolchain), so
# that `just check` never fetches. lychee is `language: script` and downloads
# its binary at first run, not at prepare, so it is run once here on one
# Markdown file; its exit status is lint's business, not this recipe's. Its
# checkout pins `stable`, and its script's cargo-binstall calls rustc, so
# without RUSTUP_AUTO_INSTALL=0 rustup would install that toolchain.
install-hooks:
    git rev-parse --is-inside-work-tree >/dev/null
    test -x .pixi/envs/default/bin/prek || { printf '%s\n' 'the pixi environment is not installed; run just initialize first' >&2; exit 2; }
    prek install --overwrite --hook-type=pre-commit --prepare-hooks
    prek prepare-hooks --config .pre-commit-fix.yaml
    -RUSTUP_AUTO_INSTALL=0 prek run lychee --files README.md

# ---------------------------------------------------------------- develop ---

# Timings measure the machine: run manually, never in CI or the gate.
bench *args:
    cargo bench --workspace --all-features --locked --bench '*' "$@"

# cargo-fuzz has no --locked: validate locks first, then run offline. The host
# target avoids the x86 default of cargo-fuzz's macOS binary on Apple silicon.
# Stable with no sanitizer; seconds bounds fuzzing time, excluding compilation.
fuzz target="smoke" seconds="300": lock-check
    CARGO_NET_OFFLINE=true cargo fuzz run "$1" --sanitizer none --target "$(rustc -vV | sed -n 's/^host: //p')" -- -max_total_time="$2"

build:
    cargo build --workspace --all-features --locked

# nextest runs the unit and integration tests; it cannot run doctests, so those
# follow through cargo test. INSTA_UPDATE=no makes a mismatch fail without a
# pending file, which would trip the gate's worktree check.
test:
    INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
    cargo test --doc --workspace --all-features --locked

test-doc:
    cargo test --doc --workspace --all-features --locked

# Optional baseline overrides discovery. Before any library release, compare this
# branch with main's merge base; afterwards use the latest reachable library tag.
# Outside the full gate: API compatibility is advisory until the 1.0 release.
# semver-checks does not accept --locked, so prove all locks first and run offline.
semver baseline="": lock-check
    #!/bin/sh
    set -eu
    baseline="$1"
    if [ -z "$baseline" ]; then
        tags=$(git tag --merged HEAD --list 'libpachinko-v[0-9]*')
        if [ -n "$tags" ]; then
            baseline=$(git describe --tags --match 'libpachinko-v[0-9]*' --abbrev=0 HEAD)
        else
            main_ref=main
            if git show-ref --verify --quiet refs/remotes/origin/main; then
                main_ref=origin/main
            fi
            baseline=$(git merge-base HEAD "$main_ref")
        fi
    fi
    git rev-parse --verify "$baseline^{commit}" >/dev/null
    printf 'semver: libpachinko baseline %s\n' "$baseline"
    CARGO_NET_OFFLINE=true cargo semver-checks -p libpachinko --baseline-rev "$baseline"

# Gate 19: snapshots match, and every .snap file is referenced by a test.
# Run the whole suite so the orphan check sees every reference. --check and
# INSTA_UPDATE=no fail mismatches without writing pending files.
# cargo-insta 1.48.0 forwards trailing options as test arguments, so nextest
# rejects --locked. Check the locks first, offline, before cargo-insta runs.
# Doctests already have their own gate; disabling its extra run avoids a warning.
snapshots-check: lock-check
    INSTA_UPDATE=no CARGO_NET_OFFLINE=true cargo insta test --check --unreferenced reject --test-runner nextest --disable-nextest-doctest --workspace --all-features

# Interactive review for a person at a terminal.
snapshots-review:
    cargo insta review --workspace

# The snapshot recipe that writes: rerun and accept without a terminal.
# Read the diff before committing, and explain each intended change. A .snap
# file no test refers to, left by a renamed or removed test, is deleted, so the
# deletion shows in the diff instead of blocking the accept.
snapshots-accept: lock-check
    INSTA_UPDATE=new CARGO_NET_OFFLINE=true cargo insta test --accept --unreferenced delete --test-runner nextest --disable-nextest-doctest --workspace --all-features

# ----------------------------------------------------------------- format ---

format:
    cargo fmt --all
    cargo fmt --manifest-path fuzz/Cargo.toml
    taplo fmt

# Automatic repairs. The fix config is run twice because a
# fixer's first pass may itself fail on what another fixer then repairs.
fix:
    -prek run --all-files --config .pre-commit-fix.yaml
    prek run --all-files --config .pre-commit-fix.yaml
    cargo fmt --manifest-path fuzz/Cargo.toml
    cargo clippy --workspace --all-targets --all-features --locked --fix --allow-dirty --allow-staged
    cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked --fix --allow-dirty --allow-staged
    just lint

# ------------------------------------------------------------------ check ---

lint:
    prek run --all-files

fmt-check:
    cargo fmt --all --check
    cargo fmt --manifest-path fuzz/Cargo.toml --check

toml-check:
    taplo fmt --check
    taplo lint

# Every lint the manifests declare, as errors. The manifests say `warn` so a
# local build never breaks mid-edit; this is where warnings become failures.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings

# Gate 7: complexity, cohesion and coupling, through rustqual against
# rustqual.toml, once per crate. The version check comes first: rustqual's
# version decides what the gate reports, so a missing .tools/bin/rustqual must
# not fall through to another one on PATH. The effect bans and their probe are
# gate 8 (`effects`).
metrics:
    #!/bin/sh
    set -eu
    pin=$(sed -n 's/^rustqual@//p' tools-source.txt)
    have=$(rustqual --version 2>/dev/null | sed -n 's/^rustqual //p')
    if [ "$have" != "$pin" ]; then
        printf 'metrics: rustqual %s on PATH, tools-source.txt pins %s; run just install-tools\n' "${have:-missing}" "$pin" >&2
        exit 1
    fi
    rustqual crates/libpachinko --config rustqual.toml --fail-on-warnings --format github
    rustqual crates/pachinko --config rustqual.toml --fail-on-warnings --format github

# Gate 8: libpachinko performs no effect itself (decision 0008). The script
# checks libpachinko's dependency tree and source against the bans, then runs
# the same checks over tests/fixtures/effects-violation, which breaks each
# ban once, and fails unless every ban fires there and none fires on the
# pachinko crate. The bans and the mechanism are the script's; this recipe
# only runs it.
effects:
    sh scripts/check-effects.sh

# Every feature combination compiles. Not --no-dev-deps: that rewrites Cargo.toml
# while it runs and would trip the worktree snapshot on a crash.
features:
    cargo hack check --workspace --feature-powerset --locked

# Runs every nextest test under instrumentation and enforces the floor. Doctests
# are not measured (nightly-only in llvm-cov); test-doc runs them uncovered.
# llvm-cov does not create the lcov file's directory, so the recipe does.
# INSTA_UPDATE=no fails a mismatch without a pending file that would trip the
# gate's worktree check.
coverage:
    INSTA_UPDATE=no cargo llvm-cov nextest --workspace --all-features --locked --no-report
    cargo llvm-cov report --fail-under-lines {{coverage_floor}}
    mkdir -p target/llvm-cov
    cargo llvm-cov report --lcov --output-path target/llvm-cov/lcov.info

doc:
    RUSTDOCFLAGS="-D warnings --cfg docsrs" cargo doc --workspace --no-deps --all-features --locked

# What GitHub Pages serves: the API reference `doc` builds, from an empty
# target/doc so that nothing stale is published, and a root index.html that
# links to both crates' pages, because stable rustdoc writes none. Not a
# gate and outside `just check`: `doc` is the gate, and this adds one file
# to what it built. The flags are `doc`'s own, through the recipe, so the
# hosted reference is the one the gate proved.
site:
    rm -rf target/doc
    just doc
    printf '%s\n' '<!doctype html>' '<html lang="en">' '<meta charset="utf-8">' '<title>pachinko: API reference</title>' '<p><a href="libpachinko/index.html">The libpachinko API reference</a></p>' '<p><a href="pachinko/index.html">The pachinko API reference</a></p>' '</html>' > target/doc/index.html

# Licences, bans and sources are answerable offline once `just sync` has run.
deny:
    cargo deny --locked check licenses bans sources

# Advisories need the RustSec database over the network, so this sits outside
# `just check` for the same reason check-links-online does.
audit:
    cargo deny --locked check advisories

deps-unused:
    cargo shear

# --------------------------------------------------------------- documents ---

check-docs:
    prek run --all-files markdownlint-cli2 typos lychee
    bg-validate-docs

check-agents:
    bg-validate-agents

# The specifications, checked mechanically: every module must report an empty
# `diagnostics` array. The wrapper asserts that, because neither subcommand's
# exit code does. Waiver terms: docs/how-to/work-with-the-specs.md.
check-specs:
    bg-run-allium check

analyse-specs:
    bg-run-allium analyse

# The test obligations allium derives from one module, as JSON: what each
# rule, invariant, transition and surface clause would have a test for. A
# suggestion list for a test list, not a gate: nothing asserts on it and it
# is outside `just check`. `allium plan` takes one file and plans that module
# alone, reading no import. The binary is named by path so that a missing
# .tools/bin/allium fails here and does not fall through to another allium
# on PATH.
plan-spec module:
    .tools/bin/allium plan "docs/specs/$1.allium"

check-links-online:
    prek run --all-files --hook-stage manual lychee-online

# --------------------------------------------------------------- aggregate ---

check-clean baseline="":
    bg-project-check clean "$1"

# The complete gate: the recipes pyproject.toml lists, in order, with the
# worktree snapshotted between each, then check-clean.
check:
    bg-project-check run
