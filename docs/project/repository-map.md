---
title: "Repository map"
kind: "project"
audience: [contributor, maintainer, agent]
canonical_for: [repository_layout]
requires: []
---

# Repository map

The repository root is a Cargo workspace with two members under `crates/`: the library
at `crates/libpachinko` and the service at `crates/pachinko`. The service depends on the
library and the library never on the service.

```text
.
├── Cargo.toml              The workspace: members, shared package fields, lints, profiles
├── Cargo.lock              The pin for every workspace crate; the build gates pass --locked
├── rust-toolchain.toml     The exact toolchain and its components
├── tools.txt               Tools conda-forge lacks, from release binaries into .tools/bin
├── tools-source.txt        Tools conda-forge lacks with no release binary, built into .tools/bin
├── rustfmt.toml            Rust formatting
├── clippy.toml             Clippy's thresholds and disallowed types; the lint table is in Cargo.toml
├── rustqual.toml           The metrics gate's thresholds, run once per crate
├── taplo.toml              TOML formatting and lint
├── deny.toml               Licences, bans, sources and advisories
├── renovate.json           Which pins Renovate proposes moving, and how they are grouped
├── Dockerfile              The service's container image, built in CI and pushed nowhere
├── .dockerignore           What the image build leaves out of its context
├── .config/nextest.toml    The test runner's profiles
├── crates/
│   ├── libpachinko/        The library: the domain docs/specs/pachinko.allium specifies
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── clippy.toml     The root clippy.toml restated, plus the effects gate's std-API bans
│   │   ├── benches/
│   │   │   └── version.rs  The smoke benchmark just bench runs
│   │   ├── src/
│   │   │   ├── lib.rs      The crate root: the crate docs, the re-exports and version()
│   │   │   ├── ids.rs  config.rs  error.rs    Names and values, the config block, refusals
│   │   │   ├── clock.rs  lease_ids.rs         The two effect traits, each with its fake
│   │   │   ├── queue.rs    The Queue, the one place the module's rules run
│   │   │   └── queue/      One file per group of rules, plus state, view and outcomes
│   │   └── tests/          One file per group of rules, and invariants.rs, the property walk
│   └── pachinko/           The service: the library's queue over HTTP
│       ├── Cargo.toml
│       ├── README.md
│       ├── src/
│       │   ├── lib.rs      The tested run(): the version line, or serve
│       │   ├── main.rs     The one-line main: run() on the arguments and standard output
│       │   ├── serve.rs    The task scope: listener, sweep and shutdown signal
│       │   ├── http.rs     The routes, the bodies and each refusal's status
│       │   ├── config.rs  error.rs            serve's flags, and the service's errors
│       │   └── clock.rs  lease_ids.rs         SystemClock and RandomLeaseIds
│       └── tests/
│           ├── binary.rs   Runs the built binary as a user would, and stops serve
│           └── http.rs     Drives an in-process server over HTTP
├── fuzz/                   The fuzz crate, outside the workspace, with its own lockfile
│   ├── Cargo.toml
│   ├── Cargo.lock          The pin for the fuzz crate's separate graph
│   └── fuzz_targets/
│       └── smoke.rs        The smoke target just fuzz runs
├── tests/fixtures/
│   └── effects-violation/  The effects probe's fixture: breaks every ban once
├── docs/                   This handbook, plus specs/ and decisions/
│   ├── manifest.yml        Every page and record: its title, kind, audience and the topics it owns
│   ├── specs/              The Allium modules: behaviour is decided here
│   └── decisions/          The decision records and their index
├── scripts/                The first-run script and the effects gate
├── tickets/                The tickets and the conventions every ticket obeys
├── .agents/skills/         Canonical agent procedures
├── .claude/  .codex/       Thin bridges to the skills, and Claude Code's shared settings
├── allium-skill-reference/ Reference material the Allium skills link to
├── .github/                The workflows under workflows/ and the composite setup action
├── AGENTS.md               Engineering conventions and the agent working agreement
├── CLAUDE.md               A pointer to AGENTS.md, as is .github/copilot-instructions.md
├── Justfile                Every supported command
├── pyproject.toml          The pixi manifest: every other tool, Python and prek
├── pixi.lock               The pin for everything pyproject.toml names
├── .pre-commit-config.yaml  .pre-commit-fix.yaml   The hooks prek runs, and the set just fix runs
├── .markdownlint-cli2.jsonc  lychee.toml  _typos.toml   The prose and link checkers
├── .editorconfig  .gitattributes  .gitignore   Editor, line-ending and ignore rules
├── README.md  CHANGELOG.md  SECURITY.md  LICENSE   The front page, the history, reporting, Apache-2.0
└── target/ .pixi/ .tools/ ai_tmp/ mutants.out/   Gitignored; never committed
```

The effects gate's std-API bans live in `crates/libpachinko/clippy.toml`, which clippy
reads for libpachinko alone in place of the root file, so it restates every root line.

## What each part is responsible for

| Directory | Responsibility |
| --- | --- |
| `crates/libpachinko/src/` | The library. It holds the domain `docs/specs/pachinko.allium` specifies, one Rust module per concern: `lib.rs` holds the crate docs, the re-exports and `version()`; `queue.rs` and the files under `queue/` hold the `Queue` and its rules; `clock.rs` and `lease_ids.rs` hold the two effect traits and their fakes. It performs no effect itself; [Architecture](../explanation/architecture.md) explains the effects boundary. Every public item is documented, and its example is a test. |
| `crates/pachinko/src/` | The service, the only place effects happen. `lib.rs` holds `run()`, which takes the arguments and the writer it writes to, so its in-module unit test writes to a `Vec<u8>`; `main.rs` calls `run()` on the command line and standard output, turns the result into an exit code, and holds nothing else. `serve.rs`, `http.rs` and `config.rs` hold the `serve` command ([decision 0019](../decisions/0019-tokio-and-axum-service.md)); `clock.rs` and `lease_ids.rs` hold the real implementations of the library's effect traits. |
| `crates/libpachinko/tests/` | Integration tests of the queue through its public API: one file per group of rules, and `invariants.rs`, a seeded property walk over the module's invariants. |
| `crates/pachinko/tests/` | Integration tests. `binary.rs` runs the built binary and asserts its line, its empty standard error and its exit status, a usage error, and that `serve` exits zero on SIGTERM; `http.rs` drives an in-process server through the [HTTP API](../reference/http-api.md). Unit tests sit in the module they test, a stated deviation recorded in [decision 0009](../decisions/0009-rust-quality-gate.md). |
| `crates/libpachinko/benches/` | Benchmarks. `version.rs` is a divan smoke benchmark of `version()`, run by hand through `just bench` and never in CI or the gate, because its timings measure the machine. |
| `fuzz/` | The fuzz crate, `libpachinko-fuzz`, outside the workspace and pinned by its own `fuzz/Cargo.lock`. Its one target, `fuzz_targets/smoke.rs`, smoke-tests the harness until the specifications supply real targets; `just fuzz` runs it, and so does the weekly `fuzz` workflow. |
| `docs/specs/` | The Allium specifications. Behaviour is decided here, not in code. Today it holds one module, `pachinko.allium`, the root, which specifies the queue the library implements. |
| `docs/decisions/` | The decision records, one file each, and `README.md`, their index. |
| `scripts/` | `initialize.sh`, the first-run script, and `check-effects.sh`, the effects gate `just effects` runs. The two validators, the allium installer and runner, the gate runner and the ripsecrets wrapper are console scripts of `biscuit-games-tooling`, pinned in `pyproject.toml`. |
| `tickets/` | The tickets, their `README.md` and `CONVENTIONS.md`, the conventions every ticket obeys. |
| `.agents/skills/` | Canonical agent procedures. The two bridge trees, `.claude/skills/` and `.codex/skills/`, point at them and add nothing. |
| `allium-skill-reference/` | Vendored reference material for the Allium skills, kept outside `.agents/` and ignored by every linter ([decision 0010](../decisions/0010-skill-reference-material.md)). |
| `.pixi/envs/default/bin` | Every tool `pyproject.toml` pins and the tooling package's console scripts, installed from `pixi.lock`. First on `PATH` inside every recipe, and also on `PATH` under `pixi run`, which the hooks use, and in every CI step after the setup action activates the environment; it is not on a plain shell's `PATH`. |
| `.github/workflows/` | `ci.yml`, the gate on every pull request and push to `main`, with the `check` job that aggregates the required ones; `audit.yml`, the weekly and per-pull-request advisory check; `mutants.yml` and `fuzz.yml`, the weekly reports; `pages.yml`, which deploys the API reference; and `release.yml`, which runs on a release tag. |
| `.tools/bin` | cargo-hack, cargo-release and cargo-fuzz from `tools.txt`, rustqual from `tools-source.txt`, and the Allium checker. On `PATH` inside every recipe, after the pixi environment. |
| `tests/fixtures/effects-violation/` | The effects gate's probe. A crate that is its own workspace, with a committed `Cargo.lock` and an empty offline path dependency named for each banned crate, one of them reached only transitively. It breaks every ban on libpachinko exactly once, and `just effects` fails unless every ban fires there. |
| `target/` | Cargo's build output. `just coverage` writes its report under `target/llvm-cov/`. |

Which of these may import which is not a matter of taste; see
[Layering and dependency direction](../explanation/layering.md).

## Related pages

- [Architecture](../explanation/architecture.md)
- [Commands](../reference/commands.md)
