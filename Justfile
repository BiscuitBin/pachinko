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

# Quint downloads its Rust evaluator (`run`, `test`, the REPL, and `verify`
# when it reports a violation) on first use, unchecked, into QUINT_HOME,
# which defaults to ~/.quint. Apalache (`verify`, `compile --target tlaplus`)
# it would download the same way, but
# `install-apalache` puts the copy tools/quint/apalache.txt pins by SHA-256
# where Quint looks first, and the `quint` recipe refuses those commands
# without it; only a guarded run racing `install-apalache` can still make Quint
# download Apalache itself. Every recipe points QUINT_HOME into the gitignored
# .quint/ instead, so the downloads stay in the worktree like every other tool
# (decision 0021). Relative, because every recipe runs from this directory and
# Quint 0.34.0 pastes the path unquoted into shell `tar` and `chmod` commands,
# which a checkout path with a space would break. It must stay `.quint`: the
# installer and the guard refuse any other value, so `just QUINT_HOME=x ...` is
# not supported. Apalache runs on the openjdk pyproject.toml pins, which the
# `quint` recipe alone puts on PATH.
export QUINT_HOME := ".quint"

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
# whatever it is, so the gate refuses it before anything else runs. The active
# toolchain must then be the pin, selected by that file: RUSTUP_TOOLCHAIN or a
# rustup override outranks it, so either is refused too.
check-toolchain:
    command -v rustup >/dev/null || { printf '%s\n' 'rustup is not installed; see docs/how-to/develop-locally.md' >&2; exit 2; }
    case "$(command -v cargo)" in "${CARGO_HOME:-$HOME/.cargo}"/bin/cargo) ;; *) printf '%s\n' "cargo resolves to $(command -v cargo), not rustup's proxy" >&2; exit 2;; esac
    pin=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml); active=$(rustup show active-toolchain); case "$active" in "$pin"-*" (overridden by '"*"/rust-toolchain.toml')") printf '%s\n' "$active" ;; *) printf '%s\n' "the active toolchain is $active; it must be $pin, selected by rust-toolchain.toml" >&2; exit 2;; esac

# Installs the toolchain and components rust-toolchain.toml names.
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

# The Quint CLI from tools/quint/package-lock.json into the gitignored
# tools/quint/node_modules (network), through the environment's npm. Not in
# `sync`: the setup action runs that in every CI job, and no job needs Quint
# until a gate does (decision 0021).
install-quint:
    npm ci --prefix tools/quint

# Apalache from the archive tools/quint/apalache.txt pins by SHA-256, into
# .quint/apalache-dist-<v>/, where Quint looks before it downloads one itself
# (network, unless the stamped install is already in place, when it skips).
# Refuses a pin the installed CLI's Apalache version disagrees with, so it runs
# after `install-quint`, and replaces a tree directory its stamp does not
# match, Quint's own download included; a symlink or non-directory at .quint,
# the dist directory or the tree is refused, not replaced (decision 0021).
install-apalache:
    sh scripts/install-apalache.sh

# The four smolvm script lines (here, lock-check's, smolvm, smolvm-purge) clear
# SHELLOPTS, BASH_ENV, ENV and PS4 so none can make the script's /bin/sh skip or
# trace it; the recipe's own sh has read them already, which this cannot undo.
# smolvm, the microVM that runs Quint Studio, from the release tarball
# tools/smolvm/smolvm.txt pins by SHA-256, into the gitignored .tools/smolvm/
# (network, unless the stamped install is already in place, when it skips).
# macOS on arm64 only. Replaces a tree its stamp does not match, after checking
# the download's hash, its members, its own checksums.txt and the binary's
# signature and Hypervisor entitlement; a symlink or non-directory at .tools or
# .tools/smolvm is refused, not replaced, and so is a run while another holds
# the install lock, .tools/smolvm.lock (decision 0022). It never runs smolvm.
install-smolvm:
    /usr/bin/env -u SHELLOPTS -u BASH_ENV -u ENV -u PS4 /bin/sh scripts/install-smolvm.sh

# The four Cargo and pixi lockfiles, exactly as committed. Never rewrites any.
# The fifth, tools/quint/package-lock.json, is `install-quint`'s.
sync:
    cargo fetch --locked
    cargo fetch --locked --manifest-path fuzz/Cargo.toml
    cargo fetch --locked --manifest-path loadgen/Cargo.toml
    pixi install --frozen

lock:
    cargo update --workspace
    cargo update --workspace --manifest-path fuzz/Cargo.toml
    cargo update --workspace --manifest-path loadgen/Cargo.toml
    pixi lock
    npm install --package-lock-only --prefix tools/quint

# Within the manifest's constraints only; every direct pin is exact, so this
# moves transitives (openssl, libcxx, the Python patch level). `pixi upgrade`
# rewrites the pins themselves and is not this recipe.
lock-upgrade:
    cargo update
    cargo update --manifest-path fuzz/Cargo.toml
    cargo update --manifest-path loadgen/Cargo.toml
    pixi update
    npm update --package-lock-only --prefix tools/quint

# Offline, like every gate recipe: a cold Cargo cache fails rather than
# fetching the registry index (`just sync` fills it), and an offline solve on a
# stale lock still fails, which is the right answer. --dry-run because pixi
# would otherwise rewrite a stale lock from that restricted solve. npm's
# --dry-run installs nothing and --offline fetches nothing: a manifest the lock
# does not satisfy fails, on a warm cache as out of sync and on a cold one as
# not cached. The last line checks tools/quint/apalache.txt, the Apalache pin,
# reading it and the two npm files as they are in the checkout, never
# node_modules/: its quint= must be the Quint CLI version both
# npm files pin, so a move of that pin fails here until the Apalache pin is
# reviewed against the new CLI. It checks that quint= was edited, not that
# apalache= is the new CLI's; `install-apalache` checks that. The line after it
# checks the form of tools/smolvm/smolvm.txt, the smolvm pin, from the file alone
# (decision 0022); `install-smolvm` checks the download against it.
lock-check:
    cargo update --workspace --locked --offline
    cargo update --workspace --locked --offline --manifest-path fuzz/Cargo.toml
    cargo update --workspace --locked --offline --manifest-path loadgen/Cargo.toml
    pixi lock --check --offline --dry-run
    npm ci --dry-run --offline --prefix tools/quint
    sh scripts/check-apalache-pin.sh
    /usr/bin/env -u SHELLOPTS -u BASH_ENV -u ENV -u PS4 /bin/sh scripts/check-smolvm-pin.sh

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

# The load generator and lapse probe against a running `pachinko serve`, from
# the excluded loadgen/ crate. Timings measure the machine: never a gate.
loadgen *args:
    cargo run --manifest-path loadgen/Cargo.toml --release --locked -- "$@"

# The excluded loadgen/ crate's own unit tests, which `test` cannot reach: the
# crate is outside the workspace. Outside `check`, like the crate; CI's
# `loadgen` job runs it.
test-loadgen:
    cargo test --manifest-path loadgen/Cargo.toml --locked

# cargo-fuzz has no --locked: validate locks first, then run offline. The host
# target avoids the x86 default of cargo-fuzz's macOS binary on Apple silicon.
# Stable with no sanitizer; seconds bounds fuzzing time, excluding compilation.
fuzz target="smoke" seconds="300": lock-check
    CARGO_NET_OFFLINE=true cargo fuzz run "$1" --sanitizer none --target "$(rustc -vV | sed -n 's/^host: //p')" -- -max_total_time="$2"

build:
    cargo build --workspace --all-features --locked

# Needs docker; builds the linux/amd64 image and sits outside just check.
image:
    docker build --platform linux/amd64 --tag pachinko:dev .

# nextest runs the unit and integration tests; it cannot run doctests, so those
# follow through cargo test. INSTA_UPDATE=no makes a mismatch fail without a
# pending file, which would trip the gate's worktree check.
test:
    INSTA_UPDATE=no cargo nextest run --workspace --all-features --locked
    cargo test --doc --workspace --all-features --locked

test-doc:
    cargo test --doc --workspace --all-features --locked

# Gate 20: every doctest with default features. libpachinko has no default
# features, so this is the build a user of the crate gets, and a doc example that
# needs `testing` must guard its lines with `# #[cfg(feature = "testing")]`.
# Each package runs on its own, so a sibling's dev-dependency features cannot
# unify into libpachinko's doctest build. A package's own dependencies,
# dev-dependencies included, could still enable `testing`, so the recipe first
# reads each package's feature graph and fails if it names libpachinko's
# `testing`, or does not name libpachinko at all.
test-doc-default:
    #!/bin/sh
    set -eu
    for package in libpachinko pachinko; do
        tree=$(cargo tree --edges features --package "$package" --invert libpachinko --locked --offline)
        case $tree in
            *'libpachinko v'*) ;;
            *) printf '%s\n' "$package: cargo tree does not list libpachinko" >&2; exit 1 ;;
        esac
        case $tree in
            *'libpachinko feature "testing"'*) printf '%s\n' "$package: its doctest build enables libpachinko's testing feature" >&2; exit 1 ;;
        esac
    done
    cargo test --doc --package libpachinko --locked
    cargo test --doc --package pachinko --locked

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
    cargo fmt --manifest-path loadgen/Cargo.toml
    taplo fmt

# Automatic repairs. The fix config is run twice because a
# fixer's first pass may itself fail on what another fixer then repairs.
fix:
    -prek run --all-files --config .pre-commit-fix.yaml
    prek run --all-files --config .pre-commit-fix.yaml
    just _fix-untracked
    cargo fmt --manifest-path fuzz/Cargo.toml
    cargo fmt --manifest-path loadgen/Cargo.toml
    cargo clippy --workspace --all-targets --all-features --locked --fix --allow-dirty --allow-staged
    cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked --fix --allow-dirty --allow-staged
    cargo clippy --manifest-path loadgen/Cargo.toml --all-targets --locked --fix --allow-dirty --allow-staged
    just lint

# The same two fixer passes over untracked files Git does not ignore, which
# `--all-files` leaves out, as `lint` reads them, over one list; the first pass
# may fail on what a fixer repaired, the second must pass. The leading
# underscore keeps it out of `just --list`.
_fix-untracked:
    just _prek-untracked .pre-commit-fix.yaml 2

# prek over every untracked file Git does not ignore, with the given hook
# configuration; skipped when there is none. The list goes through a file
# outside the worktree, not a pipe, so a failing `git ls-files` fails the
# recipe; -z and xargs -0 keep a path with a space or a newline whole. prek
# hands paths to hooks without `--`, so a path that begins with `-`, such as
# `--help`, would be read as an option: it is named and fails the recipe
# instead, and the other paths are still checked. Nothing is staged:
# `git add -N` would change the index `just check` snapshots. With `passes` 2
# the same list is run twice, and only the first run's failure is ignored.
_prek-untracked config passes="1":
    #!/bin/sh
    set -eu
    config=$1
    passes=$2
    case $passes in
        1 | 2) ;;
        *) printf "%s\n" "passes must be 1 or 2, not $passes" >&2; exit 2 ;;
    esac
    list=$(mktemp)
    trap 'rm -f "$list"' EXIT
    git ls-files -z --others --exclude-standard >"$list"
    if [ ! -s "$list" ]; then
        exit 0
    fi
    # The inner shell takes the configuration as $0 and the paths as $@.
    run() {
        xargs -0 sh -c '
            bad=0
            n=$#
            for f do
                case $f in
                    -*) printf "%s\n" "untracked path begins with -: $f" >&2; bad=1 ;;
                    *) set -- "$@" "$f" ;;
                esac
            done
            shift "$n"
            if [ "$#" -gt 0 ]; then
                prek run --config "$0" --files "$@" || bad=1
            fi
            exit "$bad"
        ' "$config" <"$list"
    }
    if [ "$passes" = 2 ]; then
        run || true
    fi
    run

# ------------------------------------------------------------------ check ---

# prek's `--all-files` reads tracked files only, so `_prek-untracked` runs the
# hook gate a second time over every untracked file Git does not ignore.
lint:
    prek run --all-files
    just _prek-untracked .pre-commit-config.yaml

fmt-check:
    cargo fmt --all --check
    cargo fmt --manifest-path fuzz/Cargo.toml --check
    cargo fmt --manifest-path loadgen/Cargo.toml --check

toml-check:
    taplo fmt --check
    taplo lint

# Every lint the manifests declare, as errors. The manifests say `warn` so a
# local build never breaks mid-edit; this is where warnings become failures.
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings
    cargo clippy --manifest-path loadgen/Cargo.toml --all-targets --locked -- -D warnings

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
# pachinko crate. The crate bans and the mechanism are the script's, the
# std-API bans the `effect:` entries in crates/libpachinko/clippy.toml; this
# recipe only runs it.
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

# Mutation testing, a report and never a gate: it grows with the workspace towards
# hours, and a mutant that survives asks for a judgement. It mutates a copy of the
# tree, so the worktree gains only mutants.out/, which Git ignores; the copy leaves
# out what Git ignores, or it would carry .pixi/ and .tools/ into every build.
# `cargo test`, not nextest, because nextest skips doctests and a mutant only a
# doc example catches would be reported missed. Exit 2 means a mutant survived.
mutants *args:
    cargo mutants --workspace --all-features --gitignore=true --cargo-arg=--locked "$@"

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

# The pinned Quint CLI, by path, so that a worktree without `just
# install-quint` fails here rather than falling through to a quint on PATH.
# The path is the package's own entry point, in the package the guard reads,
# not the node_modules/.bin shim, which could name another CLI.
# `typecheck` is offline; `run`, `test`, the REPL and a `verify` that reports a
# violation download the evaluator into .quint/ on first use. Not a gate
# (decision 0021). The first line is the guard, offline and read-only
# (scripts/install-apalache.sh --check). It refuses any option whose name
# contains `apalache`, except --apalache-config, since --apalache-version would
# make Quint download that Apalache unchecked. Unless there are no arguments
# (the REPL), the first is parse, typecheck, run, test, repl or docs, or the
# only one is --help or --version, it refuses to go on unless the stamped
# install `install-apalache` made is in place and matches its stamp, and pixi's
# java is executable. So `verify` and every `compile`, whatever its target, need
# both, and Quint never downloads Apalache through this recipe unless
# `install-apalache` is replacing the tree at the same time.
# Apalache, which `verify` and `compile --target tlaplus` run, runs the first
# `java` on PATH and never reads JAVA_HOME. conda-forge's openjdk, which
# pyproject.toml pins, keeps its `java` in lib/jvm/bin rather than bin/, so this
# recipe alone puts that directory first; no other recipe sees a Java. Quint
# uses any Apalache server already listening on localhost:8822, or the one
# --server-endpoint names, before it starts its own, and the guard cannot see
# which Apalache that is. APALACHE_JAR is unset, since the launcher would run
# that jar in place of the one in .quint/; BASH_ENV, since the launcher's
# bash would source that file and could change PATH; and CDPATH, since the
# launcher finds its jar with `cd` on the relative path Quint passes it, which
# CDPATH could send to another checkout's .quint/. JVM_ARGS and JVM_GC_ARGS,
# which the launcher passes to java, and JAVA_TOOL_OPTIONS, JDK_JAVA_OPTIONS and
# _JAVA_OPTIONS, which the JVM reads, are unset too, since any of them can load
# an agent; the guard refuses an exported bash function. TMPDIR defaults to /tmp,
# since an unset one makes the launcher create tmp/ in this directory.
quint *args:
    sh scripts/install-apalache.sh --check -- "$@"
    env -u APALACHE_JAR -u BASH_ENV -u CDPATH -u JVM_ARGS -u JVM_GC_ARGS -u JAVA_TOOL_OPTIONS -u JDK_JAVA_OPTIONS -u _JAVA_OPTIONS TMPDIR="${TMPDIR:-/tmp}" PATH={{ quote(justfile_directory() / ".pixi" / "envs" / "default" / "lib" / "jvm" / "bin") }}:"$PATH" tools/quint/node_modules/@informalsystems/quint/dist/src/cli.js "$@"

# The stamped smolvm, run by path in a cleared environment with its machine
# state outside the worktree (decision 0022): the state directory
# ~/Library/Caches/pachinko/smolvm/<worktree>-<hash>/, reached through the state
# link /tmp/pachinko-smolvm-<hash>, which is made again after a restart. One
# process checks the install under .tools/smolvm.lock, refuses a caller's
# SMOLVM_*, DYLD_*, XDG_*_HOME or DOCKER_CONFIG and --state-dir, takes the state
# lock beside the state directory, checks the state, then runs smolvm, holding
# both locks until smolvm ends; a second smolvm command for this worktree is
# refused meanwhile. Network: smolvm pulls images and its guests reach out.
smolvm *args:
    /usr/bin/env -u SHELLOPTS -u BASH_ENV -u ENV -u PS4 /bin/sh scripts/smolvm.sh run "$@"

# Removes this worktree's smolvm state, the state link and the state directory,
# and nothing else; it refuses while smolvm lists any machine there, or when the
# listing fails. The lock entry, the parents and .tools/smolvm-state-id stay.
# `--orphan <name>` does the same for the state of a worktree that is gone, and
# `--orphan <name> --delete <machine>` deletes one machine left in it.
smolvm-purge *args:
    /usr/bin/env -u SHELLOPTS -u BASH_ENV -u ENV -u PS4 /bin/sh scripts/smolvm.sh purge "$@"

# Redraw the SVG diagrams a decision record embeds, from scripts/diagrams.py,
# which owns their layout and words: edit the script, never an SVG. It writes
# under docs/decisions/, so it is not a check and is outside `just check`.
diagrams:
    python scripts/diagrams.py

# Run the Python double's tests under double/tests on pixi's Python, which must
# be 3.14 or newer. It is not a gate of libpachinko and is outside `just check`.
# `-t double` makes the start directory the package `tests`, so discovery
# calls its load_tests, which runs test_coverage last; PACHINKO_DOUBLE_SUITE
# tells test_coverage that this run must have gone through load_tests. `-B`
# writes no __pycache__.
python-double:
    python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
    PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'

check-links-online:
    prek run --all-files --hook-stage manual lychee-online

# --------------------------------------------------------------- aggregate ---

check-clean baseline="":
    bg-project-check clean "$1"

# The complete gate: the recipes pyproject.toml lists, in order, with the
# worktree snapshotted between each, then check-clean.
check:
    bg-project-check run
