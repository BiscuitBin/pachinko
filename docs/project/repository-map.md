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
├── Cargo.toml              The workspace: members, shared package fields, dependencies, lints, profiles
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
│   ├── libpachinko/        The library: the queue the specification defines
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── clippy.toml     The root clippy.toml restated, plus the effects gate's std-API bans
│   │   ├── benches/
│   │   │   └── version.rs  The smoke benchmark just bench runs
│   │   ├── src/
│   │   │   ├── lib.rs      The crate root: the crate docs, the modules and version()
│   │   │   ├── clock.rs    Timestamp, the Clock trait and FakeClock
│   │   │   ├── config.rs   Config: the module's five figures and their validation
│   │   │   ├── error.rs    The error enums and LeaseId
│   │   │   ├── records.rs  The PocketRecords trait and its value types
│   │   │   ├── records/    MemoryRecords, and the records contract behind `testing`
│   │   │   ├── queue.rs    Queue: one method per trigger
│   │   │   └── queue/      The rules, one file per family, and the inspection seam
│   │   └── tests/
│   │       ├── queue/      The tests from the specification, one module per rule family
│   │       └── records_contract.rs  The records contract over MemoryRecords
│   └── pachinko/           The service: a binary built around the library
│       ├── Cargo.toml
│       ├── README.md
│       ├── benches/
│       │   └── arrow.rs    The Arrow record store under load, which just bench runs
│       ├── src/
│       │   ├── lib.rs      The tested run(), and serve, which runs the queue
│       │   ├── main.rs     The one-line main: run() on standard output, then the exit code
│       │   ├── cli.rs      The command line: no arguments, or serve and its options
│       │   ├── clock.rs    SystemClock, the real Clock
│       │   ├── arrow.rs    ArrowRecords: one pocket's records as one RecordBatch
│       │   ├── arrow/      The batch schemas
│       │   ├── actor.rs    The owner task that holds the queue, and its ticker
│       │   ├── flight.rs   The Arrow Flight service
│       │   └── flight/     The action types and the wire's Arrow IPC
│       └── tests/
│           ├── binary.rs   Runs the built binary as a user would
│           ├── arrow_records.rs  The records contract over ArrowRecords
│           ├── flight.rs   The Flight API over loopback
│           └── flight/     Its fixture and one module per operation family
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
| `crates/libpachinko/src/` | The library: the queue `docs/specs/pachinko.allium` specifies. `queue` holds `Queue`, with one method per trigger, `records` the record-store trait and its reference, `clock` the clock trait and its fake, `config` the module's five figures and `error` the error enums; `lib.rs` holds the crate docs and `version()`. The `testing` feature adds the records contract and the queue's inspection seam. It performs no effect itself; [Architecture](../explanation/architecture.md) explains the effects boundary. Every public item is documented, and its example is a test. |
| `crates/pachinko/src/` | The service, the only place effects happen. `lib.rs` holds `run()`, which takes the writer it writes to, so its in-module unit test writes to a `Vec<u8>`, and `serve`, which runs the queue; `main.rs` calls `run()` on standard output and turns the result into an exit code, and holds nothing else. `cli` reads the arguments, `clock` and `arrow` supply `SystemClock` and `ArrowRecords`, `actor` holds the owner task and the ticker, and `flight` the Arrow Flight service ([decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md), [decision 0020](../decisions/0020-records-as-arrow-batches.md)). |
| `crates/libpachinko/tests/` | Integration tests. `queue/` is one target with a module per rule family, every test tagged with the obligations it covers, over `MemoryRecords` and `FakeClock`; `records_contract.rs` runs the records contract over `MemoryRecords`. Both need the `testing` feature, which the gate turns on. |
| `crates/pachinko/tests/` | Integration tests. `binary.rs` runs the built binary and asserts its line, its empty standard error and its exit status, and that `serve` writes the address it listens on; `arrow_records.rs` runs the records contract over `ArrowRecords`; `flight.rs` drives the Flight API over loopback. Unit tests sit in the module they test, a stated deviation recorded in [decision 0009](../decisions/0009-rust-quality-gate.md). |
| `crates/*/benches/` | Benchmarks, run by hand through `just bench` and never in CI or the gate, because their timings measure the machine. `crates/libpachinko/benches/version.rs` is a divan smoke benchmark of `version()`; `crates/pachinko/benches/arrow.rs` measures the Arrow record store under load. |
| `fuzz/` | The fuzz crate, `libpachinko-fuzz`, outside the workspace and pinned by its own `fuzz/Cargo.lock`. Its one target, `fuzz_targets/smoke.rs`, smoke-tests the harness until the specifications supply real targets; `just fuzz` runs it, and so does the weekly `fuzz` workflow. |
| `docs/specs/` | The Allium specifications. Behaviour is decided here, not in code. Today it holds one module, `pachinko.allium`, which specifies the queue that `libpachinko` implements. |
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
