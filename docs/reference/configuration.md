---
title: "Configuration"
kind: "reference"
audience: [contributor, maintainer, operator, agent]
canonical_for: [configuration_reference]
requires: []
---

# Configuration

The service's runtime configuration is the flags `pachinko serve` takes, listed under
[Service flags](#service-flags); it reads no file and no variable
([decision 0019](../decisions/0019-service-runtime-tokio-and-http.md)). libpachinko's one
Cargo feature, `testing`, adds the `testing` module and two `Queue` methods for tests,
which [Testing](testing.md#tests-from-the-specifications) describes, and is off in every
normal build. Everything else on this page configures the tools that build and check the
workspace.

## Service flags

`pachinko serve` reads `--name value` pairs. An unknown flag, a flag without a value, a
value that is not an unsigned integer, a log level that is not one of the five names, or
a request timeout of 0 is a usage error naming the flag, and the binary exits 1 with
`pachinko: usage: <reason>` on standard error. The five specification values
then go through the library's config check, which refuses a zero
`--max-consecutive-expiries` or `--max-pull-batch` the same way, as
`pachinko: invalid configuration: <reason>`. The three request-shape limits must each be
at least 1, and `--max-body-bytes` must be at least the base64 size of one maximum payload
plus the shortest batch envelope around it, `ceil(max_payload_bytes / 3) * 4 + 88`
(1,398,192 bytes with the defaults); a smaller body cap is a usage error naming that
minimum. The 88 bytes are
`{"records":[{"pocket_key":"","record_key":"","rank":-9223372036854775808,"payload":""}]}`
less its payload, which also covers the single `PUT`'s shorter envelope.

| Flag | Default | What it sets |
| --- | --- | --- |
| `--listen` | `127.0.0.1:8080` | The address the HTTP listener binds; loopback unless set. Port 0 picks a free port, which the `listening on <address>` line names. |
| `--lease-ttl-ms` | `300000` (5 minutes) | `lease_ttl`: how long a lease lasts after its claim or its consumer's last pull. |
| `--max-consecutive-expiries` | `3` | `max_consecutive_expiries`: how many consecutive lease expiries park a pocket; at least 1. |
| `--max-pull-batch` | `100` | `max_pull_batch`: the most records one pull hands out; at least 1. |
| `--superseded-ttl-ms` | `43200000` (12 hours) | `superseded_ttl`: how long a superseded or stale payload is kept. |
| `--dead-letter-ttl-ms` | `43200000` (12 hours) | `dead_letter_ttl`: how long a dead letter is kept. |
| `--max-batch-rows` | `1000` | The most rows one batch publish carries; more is 413 `too_many_rows`. |
| `--max-payload-bytes` | `1048576` (1 MiB) | The largest decoded payload of a single publish or a batch row; more is 413 `payload_too_large`. |
| `--max-body-bytes` | `16777216` (16 MiB) | The largest request body any route reads; more is 413 `body_too_large`. At least the minimum above. |
| `--log-level` | `info` | The most detailed log line written to standard error: `error`, `warn`, `info`, `debug` or `trace`, lower case. Each request writes one line at `debug`. |
| `--request-timeout-ms` | `30000` (30 seconds) | How long a handler may run before it is answered 408 `request_timeout`, and how long a connection may send nothing; it also bounds a request head: its connection is closed two timeouts after the head's first byte if no reply has been written ([HTTP API](http-api.md#request-timeout)); at least 1. |
| `--shutdown-grace-ms` | `5000` (5 seconds) | How long shutdown lets requests in flight finish before it cuts the open connections, and how long it then waits before aborting what is left; 0 cuts at once, then waits up to 1 second for the server's tasks, which end on the cut, before aborting what is left. |
| `--sweep-ms` | `1000` (1 second) | How long the ticker waits after each sweep before the next; each sweep lapses the leases and decays the shed entries whose `expires_at` has passed; at least 1. |

The five specification values' defaults are the specification's, which decision 0019
confirms; the request-shape limits' defaults are the comparables decision 0019's
evidence lists. The log level comes from `--log-level`
alone: `RUST_LOG` is ignored. The routes, the timeout and the shutdown window are in
[HTTP API](http-api.md).

## Environment

libpachinko reads no environment variable, and never will: the environment is one of the
effects it reaches only through a trait it defines, as
[Architecture](../explanation/architecture.md) explains. The `env!` in its `version()` is
read by the compiler, not at run time. The service reads none either, `RUST_LOG`
included: its log level is `--log-level`. The tooling reads a
few, each in one place:

| Variable | Read by | Effect |
| --- | --- | --- |
| `PATH` | Every recipe, which the `Justfile` exports it to | `.pixi/envs/default/bin`, then `.tools/bin`, then the caller's `PATH`. Neither directory holds a cargo, so rustup's proxy stays first for the compiler. |
| `RUSTDOCFLAGS` | The `doc` recipe, which sets it | `-D warnings --cfg docsrs`: warnings fail, and the build is the one docs.rs would make. Set on that recipe and nowhere else; `site` reaches it through `doc`. |
| `CARGO_HOME` | The `check-toolchain` recipe | Where rustup's proxy is expected; `~/.cargo` when unset. |
| `CARGO_TERM_COLOR`, `CARGO_INCREMENTAL`, `CARGO_NET_RETRY`, `RUST_BACKTRACE` | The CI workflows | Coloured logs, no incremental artefacts on a fresh runner, retries for flaky fetches, and a backtrace on a panic. Exported by `ci.yml` and `pages.yml`; `audit.yml` and `mutants.yml` export the first three, `fuzz.yml` the first two, and `release.yml` none. |
| `GITHUB_TOKEN` | CI's `install-tools` step only | Lifts GitHub's anonymous rate limit for cargo-binstall's release lookup in the `tools.txt` pass. The run's own token. The recipe unsets it, and `GH_TOKEN`, for the `tools-source.txt` pass, so no build script compiled in that pass can read it. |
| `PREK_HOME` | CI's setup action | Moves prek's cache outside the checkout, so the worktree snapshot never sees it and the cache can be restored between runs. |
| `RUSTUP_AUTO_INSTALL` | The `install-hooks` recipe, and the `Dockerfile`'s build stage | Set to `0` for the one lychee run, so that rustup does not install the `stable` toolchain lychee's hook checkout pins; and for the build stage's compiler check, so that the image's own `rustc` is compared with `rust-toolchain.toml` before rustup can install the pin. |
| `INSTA_UPDATE` | insta in tests | `test`, `coverage` and `snapshots-check` set `no`: a mismatch fails without writing files. `snapshots-accept` sets `new`, then cargo-insta accepts the pending files. Outside the recipes, `auto` defaults to `no` in CI and `new` elsewhere; `always` overwrites `.snap` files. The recipes set it ahead of the `insta` crate, which no crate depends on until something is snapshotted. |
| `CARGO_NET_OFFLINE` | Cargo in the snapshot recipes, `fuzz` and `semver` | Set to `true` after `lock-check`, because cargo-insta cannot forward `--locked` to nextest, and cargo-fuzz and cargo-semver-checks take no `--locked`. Dependency resolution uses the local cache. |

There is no `.cargo/config.toml`. `RUSTFLAGS` set there would change every build's
fingerprint and make rust-analyzer and the recipes rebuild `target/` against each other;
a value that matters to one recipe is set on that recipe. The same argument applies to
the code: a value baked into a build is not configuration, it is a constant, and
constants belong in source where they can be reviewed.

## Configuration files

| File | What it configures |
| --- | --- |
| `rust-toolchain.toml` | The compiler: an exact stable release, the minimal profile, and three components: rustfmt, clippy and `llvm-tools-preview` for coverage. Exact rather than `stable`, because clippy's lint set moves every six weeks and would break `-D warnings` with no diff here. No extra target, because libpachinko is a std library ([decision 0008](../decisions/0008-std-library.md)), and no `rust-src`, which served rust-analyzer only; a contributor who wants it runs `rustup component add rust-src`. Honoured only by rustup's proxy. |
| `Cargo.toml` (root) | The workspace: resolver 3, every directory under `crates/` a member and `fuzz/` and `loadgen/` excluded, the package fields each crate inherits (edition, `rust-version`, licence, repository, homepage, authors, `publish = false`), the lint table, and the release and test profiles. The release profile builds with fat LTO and one codegen unit, kept by the maintainer's choice on 2026-10-24 after the container image's release build compiled in 6.24 s uncached for a 9.4 MB image; the test profile at `opt-level = 1`. The `[workspace.dependencies]` table holds every third-party dependency, each with its note ([decision 0007](../decisions/0007-dependency-policy.md)): divan and proptest, which libpachinko takes as development dependencies only; thiserror, which both crates take; the service's tokio, axum, serde, base64, getrandom, tracing and tracing-subscriber; and serde_json and ureq, the service's development dependencies for its wire tests. |
| `crates/libpachinko/Cargo.toml` | The library: its name, version, description, keywords, categories and README, the metadata the `cargo` lint group's `cargo_common_metadata` names, kept for the docs and for the day publishing is decided, since the lint skips a `publish = false` package; every other package field drawn from the workspace; `[lints] workspace = true`; the docs.rs metadata; and `[package.metadata.release]`, cargo-release's tag prefix and the `CHANGELOG.md` replacements a release makes ([decision 0016](../decisions/0016-release-process.md)). One feature, `testing`, off by default, which adds the `testing` module and two `Queue` methods for tests: the queue's inspection seam, the tick's cost counts and the records contract; one normal dependency, thiserror; and two development dependencies from the workspace table: divan, for the `version` and `workloads` benches under `benches/`, each a `[[bench]]` table with `harness = false`, and proptest, for the queue suite's invariant walk. |
| `crates/pachinko/Cargo.toml` | The service, in the same shape: `libpachinko` by path and version, and the workspace entries above as `{ workspace = true }` lines ([decision 0019](../decisions/0019-service-runtime-tokio-and-http.md)). Cargo then enforces the direction, since libpachinko cannot name `pachinko` without a cycle; see [Layering](../explanation/layering.md). |
| `fuzz/Cargo.toml` | The fuzz crate, `libpachinko-fuzz`: excluded from the workspace, so it is its own, with its own `fuzz/Cargo.lock`, a dependency on libfuzzer-sys and on libpachinko by path, and the `smoke` target. An excluded crate inherits nothing, so it restates `rust-version`, the workspace's two `forbid` lints and its clippy levels, which are kept in step with the root table by hand. |
| `loadgen/Cargo.toml` | The load generator, `pachinko-loadgen`: excluded from the workspace and pinned by its own `loadgen/Cargo.lock`, with `publish = false`, its own `rust-version`, and two dependencies, ureq without default features or TLS, and serde_json, each with its decision-0007 note. It names no workspace crate and speaks HTTP only. Like the fuzz crate, it restates the two `forbid` lints and the clippy levels, which are kept in step with the root table by hand. |
| `clippy.toml` | The restriction lints relaxed inside tests only (`allow-unwrap-in-tests` and its siblings for `expect`, `panic`, indexing, `dbg!` and printing), `HashMap` and `HashSet` disallowed, because their iteration order is not deterministic (the reason names `BTreeMap` and `BTreeSet`); and four thresholds, for cognitive complexity, nesting depth, function length and parameter count, whose numbers `rustqual.toml` repeats. [Decision 0012](../decisions/0012-metrics-gate.md) owns the numbers and says why each. |
| `rustfmt.toml` | The 2024 style edition, a width of 100, Unix newlines, and the field-init and `?` shorthands. Stable options only. |
| `taplo.toml` | TOML formatting at a width of 100 with two-space indentation, keys sorted in the named dependency tables of every manifest (a target-specific table is not covered until its key is added), and `target/`, `.tools/`, `.pixi/` and `ai_tmp/` excluded. The exclusion is load-bearing: the pixi environment holds TOML files of its own. |
| `deny.toml` | cargo-deny, by section, below. |
| `.config/nextest.toml` | cargo-nextest's minimum version and its profiles: `default` stops at the first failure; `ci` runs everything, retries nothing and writes a JUnit report. No recipe or workflow selects `ci` today. |
| `pyproject.toml` | The pixi manifest: Python, just, prek, cargo-binstall and every cargo tool conda-forge carries, cargo-insta among them, under `[tool.pixi.*]`, for Linux and macOS on Apple silicon, with the tooling package as a git dependency; and the tooling package's own table, `[tool.biscuit-games-tooling]`, whose `recipes` list is what `just check` runs, in order. No `rust`: rustup owns the compiler. `pixi.lock` is the pin, generated by pixi and never edited by hand. |
| `tools.txt` | The exception list: binaries conda-forge lacks, one `name@version` per line, installed by `just install-tools` from a release archive and never compiled. Today three lines: cargo-hack, cargo-release and cargo-fuzz. |
| `tools-source.txt` | The second exception list: tools conda-forge lacks that also have no release binary, one `name@version` per line, which `just install-tools` builds from crates.io source with `--locked` unless cargo-quickinstall has a signed build of the pin. Today one line, rustqual. |
| `rustqual.toml` | rustqual, for the `metrics` gate: the dimensions it runs (complexity, SRP, coupling, architecture, and IOSP, which cannot be switched off) and their thresholds, with duplicates, boilerplate, test quality and structural checks switched off. Architecture is enabled but holds no `[[architecture.pattern]]` rule yet: the rules that hold module direction wait for there to be several specification modules, as [Layering](../explanation/layering.md) says. Its globs resolve against the crate directory, so the recipe runs it from the root with `--config`, once per crate. |
| `scripts/check-effects.sh` | The `effects` gate, below. |
| `crates/libpachinko/clippy.toml` | The root `clippy.toml` restated for libpachinko, plus the `effects` gate's std-API bans, below. |
| `tests/fixtures/effects-violation/` | The `effects` gate's probe, below. |
| `Justfile` | Every recipe, and the `coverage_floor` variable, the line-coverage floor `just coverage` enforces: 90, over `crates/*/src/**`. Lower the complexity, not the number. |
| `Dockerfile` | The container image `just image` and CI's `image` job build: a build stage on the Rust image at the toolchain pin, which fails unless its compiler matches `rust-toolchain.toml`, and a distroless run stage with the release binary and no toolchain or shell. Both bases are a tag with its digest. [Build the container image](../how-to/build-the-container-image.md) has the rest. |
| `.dockerignore` | What the image's build context leaves out: `.gitignore`'s entries, kept in step with it by hand, plus `.git` and `tickets/`. |
| `renovate.json` | Renovate's configuration: which pins it reads, how it groups them, and the dependency-dashboard approval its updates wait for. [Decision 0018](../decisions/0018-renovate.md) is the record, and [Maintain dependencies](../how-to/maintain-dependencies.md) the procedure. |
| `.pre-commit-config.yaml` | The read-only hook gate `just lint` runs and `just install-hooks` installs. |
| `.pre-commit-fix.yaml` | The mutating hooks (rustfmt, taplo, `cargo shear --fix`, the end-of-file and trailing-whitespace fixers and markdownlint's `--fix`), run only by `just fix`. |
| `.markdownlint-cli2.jsonc` | markdownlint's rules and the directories it ignores. |
| `lychee.toml` | The link checker's settings and excluded paths. |
| `_typos.toml` | The only typos configuration: the excluded lockfiles and directories, and the allowlisted `mis`, `als` and `ede`. A `[tool.typos]` table in `pyproject.toml` would be ignored. |
| `.editorconfig` | Whitespace for every file type: UTF-8, LF, a final newline and two-space indentation by default; four spaces for Rust, the `Justfile` and the specifications; Markdown and `LICENSE` left to their own indentation. |
| `.gitattributes` | LF line endings for every text file; every `Cargo.lock`, `fuzz/Cargo.lock` and `loadgen/Cargo.lock` included, and `pixi.lock` marked generated, `pixi.lock` merged as binary and left out of diffs; `allium-skill-reference/` marked vendored. |
| `.gitignore` | Everything the gate must never see in the worktree: `target/`, `.pixi/`, `.tools/`, `ai_tmp/`, the lychee cache, coverage and snapshot leftovers, Claude Code's runtime state under `.claude/` apart from its tracked settings and skills, and the Allium loop's `.allium-loop/` records. `just check` snapshots the worktree between recipes, so a file Git can see aborts the run. |
| `docs/manifest.yml` | The handbook's registry: every page's path, title, kind, audience and the topics it owns. The [Documentation contract](documentation-contract.md) owns its rules. |

### The lint table

`[workspace.lints]` in the root `Cargo.toml`, which every crate opts into. Every level is
`warn`, so a local build never breaks mid-edit; `just clippy` passes `-D warnings`, which
is where a warning becomes a failure. The exceptions are the `forbid` lints, which fail
every build, and the rustdoc `deny` pair, which fails any `cargo doc`.

| Lint | Level | Why |
| --- | --- | --- |
| `unsafe_code` | forbid | No crate may contain `unsafe`, and `forbid` means no item can opt back in. |
| `non_ascii_idents` | forbid | Identifiers read the same to every reader and tool. |
| `missing_docs` | warn | Every public item is documented; see [API reference](api.md). |
| `unreachable_pub`, `unnameable_types` | warn | What is `pub` is exported on purpose. |
| `rustdoc::broken_intra_doc_links`, `rustdoc::private_intra_doc_links` | deny | A doc link that does not resolve, or resolves to something a reader cannot see, fails. |
| `clippy::pedantic`, `clippy::cargo` | warn, priority -1 | The whole group, so a single lint can be set above it. |
| `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented` | warn | The library does not panic; an error is returned. `clippy.toml` relaxes `unwrap_used`, `expect_used` and `panic` in tests; the other three hold there too. |
| `exhaustive_enums`, `exhaustive_structs` | warn | A public type is `#[non_exhaustive]`, so growing it breaks no caller. |
| `allow_attributes`, `allow_attributes_without_reason` | warn | A suppression is `#[expect(lint, reason = "...")]`, which reports itself when it stops being needed. They reject an outer `#[allow]` and any `allow` without a reason; an inner `#![allow(..., reason = "...")]` passes both, so review holds that form, by the maintainer's choice on 2026-10-24. |
| `mod_module_files` | warn | No `mod.rs`: a module is the file named after it. |
| `cognitive_complexity` | warn | A restriction lint, so the pedantic group leaves it off; `clippy.toml` sets its threshold. |
| `multiple_crate_versions` | allow | cargo-deny's `bans.multiple-versions` owns that check. |

The table holds more, each one line: `missing_debug_implementations`, `rust_2018_idioms`,
`unused_qualifications`, the trivial-cast pair, `missing_const_for_fn`, `use_self`,
`dbg_macro`, the print pair, `tests_outside_test_module`, `iter_over_hash_type`,
`str_to_string`, `lossy_float_literal`, `float_cmp_const` and `same_name_method`, and the
rustdoc pair `missing_crate_level_docs` and `unescaped_backticks`. There is no
`std_instead_of_core` family: libpachinko is a std library, and what keeps effects out of
it is the `effects` gate, not the path a name is imported by.

### `deny.toml`

| Section | Holds |
| --- | --- |
| `graph` | Every feature, on three targets (Linux, macOS on Apple silicon and Windows), with development dependencies excluded: the bans are on what ships, not on the test harness. |
| `advisories` | Yanked crates denied, unmaintained crates reported for the workspace's own dependencies. Read only by `just audit`, which needs the network. |
| `licenses` | The allowed licences, all permissive and compatible with Apache-2.0, at a confidence threshold of 0.9; the workspace's own unpublished crates are not checked. |
| `bans` | Wildcard versions denied; duplicate versions warned. One crate denied: `openssl-sys`, because no native TLS belongs anywhere in the workspace. |
| `sources` | crates.io only: an unknown registry or git source is denied. |

`deny.toml` bans no effect crate. cargo-deny's `wrappers` names direct parents only, so a
workspace-wide ban on a runtime crate would also reject the copy the service reaches
through one of its own dependencies, where every effect is allowed. The crate bans are
the `effects` gate's, over libpachinko's tree alone.

### `scripts/check-effects.sh`

The `effects` gate's crate bans: the list of banned crates is a variable at the top of
the script, and the script is the only place it lives. libpachinko performs no effect
itself, as [Architecture](../explanation/architecture.md) explains;
[Quality gates](quality-gates.md#gate-8-the-effects-bans) lists the script's checks.

### `crates/libpachinko/clippy.toml`

The `effects` gate's std-API bans, as `disallowed-methods` and `disallowed-types` entries
whose reason begins `effect:`. Clippy reads the first `clippy.toml` it finds walking up
from a crate's manifest and merges nothing, so this file shadows the root `clippy.toml`
for libpachinko: it restates every root setting in the same order, and gate 8 fails when
the two differ in anything but comments, blank lines and the effect entries. A change to the root file is mirrored here. A
ban is added with a reason beginning `effect:` and broken once in the fixture with a
`// ban:` tag; a path that does not resolve is only a warning, so the fixture is what
proves an entry is live.

### `tests/fixtures/effects-violation/`

The `effects` gate's probe, a crate that is its own workspace with a committed
`Cargo.lock`. Its `src/lib.rs` breaks each std-API ban once, on a line that starts with
`fn` and ends with a `// ban:` tag naming the path or paths it breaks; `deps/` holds an empty offline
path dependency named for each banned crate, one of them reached only through `deps/wrapper` and one only
under `cfg(windows)`. The fixture's `rustfmt.toml` turns formatting off, so each break
stays on its one line.

## Values the specifications decide

A figure a specification states is mirrored in code once, by name, in a constant a
doctest and a unit test hold to the specification's `config` entry. A Rust constant that
stands for a figure but has no `config` entry to name is drift the other way: the figure
belongs in the specification first. Today the one module, `docs/specs/pachinko.allium`,
declares five figures in its `config` block, and `libpachinko::Config::default`
mirrors them, held by its doctest and by the tests of scenarios S001 and S002; see
[Specifications](../explanation/specifications.md).

## Version pins

Every pin lives in exactly one file, and this page names the file rather than the number,
so a bump never touches the handbook.

| Pin | Where |
| --- | --- |
| The Rust toolchain | `rust-toolchain.toml`, with `rust-version` in the root `Cargo.toml`, in `fuzz/Cargo.toml` and in `loadgen/Cargo.toml`, and the `Dockerfile`'s build-stage tag, following it |
| Rust dependencies | `Cargo.lock`, `fuzz/Cargo.lock` and `loadgen/Cargo.lock`, within the caret ranges the manifests write ([decision 0007](../decisions/0007-dependency-policy.md)) |
| Container base images | Each `FROM` in the `Dockerfile`, a tag with its digest |
| pixi's tools and Python | `pyproject.toml`, locked in `pixi.lock` |
| Tools conda-forge lacks | `tools.txt`, or `tools-source.txt` for a tool with no release binary |
| The Allium checker | The tooling package, which pins its version and checksums; `pyproject.toml` pins the package |
| Hooks | The `rev` of each remote hook in the two prek configurations, a commit SHA with a version comment |
| GitHub Actions | Each `uses:` in the workflows and the setup action, a commit SHA with a version comment |

How to move each is in [Maintain dependencies](../how-to/maintain-dependencies.md).

## Related pages

- [Commands](commands.md)
- [Quality gates](quality-gates.md)
- [Architecture](../explanation/architecture.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
