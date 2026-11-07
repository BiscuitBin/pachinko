---
title: "Testing"
kind: "reference"
audience: [contributor, maintainer, agent]
canonical_for: [testing_reference]
requires: []
---

# Testing

## Framework

cargo-nextest runs the unit and integration tests. It cannot run doctests, so `just test`
runs nextest and then `cargo test --doc`, and `just test-doc` runs the doctests alone.
`.config/nextest.toml` names the minimum nextest version it needs (the pin itself is in
`pyproject.toml` and `pixi.lock`) and holds the profiles: the default one
stops at the first failure, and a `ci` profile, which runs everything and writes a JUnit
report, is there for a job that wants one; no recipe selects it today. Divan is the
library’s development dependency for benchmarks. A property-test crate and insta, the
snapshot crate, each arrive with the first test that needs it, as a development
dependency only, with the note that
[decision 0007](../decisions/0007-dependency-policy.md) asks for beside its range.

## Layout

Tests live in three places, and each is deliberate.

| Where | What |
| --- | --- |
| `#[cfg(test)] mod tests` at the foot of a module | Unit tests of that module, with access to its private items. |
| `crates/libpachinko/tests/*.rs` and `crates/pachinko/tests/*.rs` | Integration tests: the crate as a consumer sees it, through its public API only. |
| `///` examples on public items | Doctests: each example compiles and runs as a test. |

Unit tests live in the module they test. This departs from the games' rule that tests are
never colocated with the code: in Rust a unit test can reach a private item only from
inside its module, and [Decision 0009](../decisions/0009-rust-quality-gate.md) records
the deviation.

libpachinko's integration tests are the queue suite under `tests/queue/` and
`tests/records_contract.rs`, which "Tests from the specifications" describes.
`crates/pachinko/tests/binary.rs` is the binary test: it runs the built service as a user
would, through the path Cargo hands it as `CARGO_BIN_EXE_pachinko`, and asserts that the
process exits zero, prints `pachinko`, a space, `libpachinko::version()` and a newline,
and writes nothing to standard error. The file opens with an `#![expect]` of `clippy::tests_outside_test_module` and its
reason, because an integration test file is its own test module.
[Run the binary test](../how-to/test-and-debug.md#run-the-binary-test) says how to run it
and how it divides from the unit test beside `run`.

A helper shared between two files under `tests/` would be compiled into each, and each
would report the functions it does not call as dead code. So each file carries the few
helpers and fixtures it uses, and the snapshot tests, once there are any, share one file.

Once a property test exists, a failing case is written to a `proptest-regressions/` file
beside the test that produced it. That file is committed, so the case is replayed first
on every later run.

## Conventions

**Derive a test from the clause it proves, and name it after the clause.** A test named
`the_binary_prints_its_line_and_exits_zero` says which guarantee broke when it fails; a
test named after the function it calls does not.

**A doc example is a test.** It runs under `just test-doc`, so an example that stops
being true stops the gate. Every public item is to carry one.

**Reach an effect through the fake beside its trait; never through the real thing.** The
effect traits arrive with the specifications that need them, each with a fake beside it,
and a test drives the library through that fake and gets exactly the behaviour it chose.
[Architecture](../explanation/architecture.md) explains the boundary, and decisions
[0003](../decisions/0003-effects-behind-traits.md) and
[0008](../decisions/0008-std-library.md) record it.

**Restriction lints relax inside tests only.** `clippy.toml` allows `unwrap`, `expect`,
`panic`, indexing, `dbg!` and printing inside `#[cfg(test)]` code, and nowhere else, so a
test can fail loudly while the library cannot.

## Tests from the specifications

A specification's clauses are what the tests prove.
`just plan-spec <module>` prints, as JSON, the test obligations the Allium checker
derives from one module under `docs/specs/`: what each rule, invariant, transition and
surface clause would have a test for. It plans that one file and reads no import. The
list is a suggestion for a ticket's test list, not a gate: nothing asserts on it and it
sits outside `just check`. [Work with the specifications](../how-to/work-with-the-specs.md)
covers the workflow.

The tests of `docs/specs/pachinko.allium` follow one layout.

- **The `testing` feature.** `libpachinko`'s `testing` feature adds
  `records::contract::check_contract` and `queue::Queue::snapshot`, the inspection seam
  every invariant is checked through. The gate's `just test`, `just coverage` and
  `just clippy` turn it on with `--all-features`; the integration test files open with
  `#![cfg(feature = "testing")]`, because `just features` builds without it.
- **The queue suite.** `crates/libpachinko/tests/queue/` is one test target: `main.rs`
  declares the modules, each `#[cfg(test)]`, and states the tag conventions;
  `fixture.rs` builds a queue over `MemoryRecords`, a `FakeClock` at a fixed instant and
  `CountingLeaseIds`, with helpers by literal keys and each invariant's check by its
  specification name; one file per rule family (`identity.rs`, `publish.rs`, `pull.rs`)
  holds one test per scenario and the invariant tests.
- **The records contract.** `tests/records_contract.rs` runs `check_contract` over
  `MemoryRecords`; a second store would get a line of its own.
- **Tags.** A test names what it pins in comments above it: `// scenario:` and the id,
  once per scenario across the crate; `// obligation:` and an id `just plan-spec`
  prints; and `// obligation: guarantee.<Name>` for a surface guarantee, which has no
  checker id. A `compile_fail` doctest may carry the tags when the type system is the
  test, as the negative-TTL scenario does in `config.rs`.

These rules hold for every module the specifications add.

- **The oracle comes from the specification.** Where a test needs to know whether an
  operation should be accepted, the answer is a model written in test code from the
  rule's `requires` clauses, never the implementation.
- **Invariants and surfaces are held by property.** A script of arbitrary operations,
  refused ones included, runs against the module, and after every step the property
  holds each invariant under its own name, as far as the surface shows it, and that an
  operation is accepted exactly when its rule allows it.
- **What the specification leaves open is pinned as the implementation's choice.** Where
  a specification leaves an order or a tie-break to the implementation, the module
  documents its rule, and a test that pins what that rule gives is a test of the rule. A
  change to the pinned value is a change to the rule and is read as one.
- **A test that passed when first run is shown to bite.** The code is broken on purpose
  and the named test is seen to fail; the ticket that built the module lists the breaks.
- **A path no input reaches is called directly.** A conversion that only a refusal
  further down could reach is tested by calling it with every variant.
- **What no test can see is said.** A call to a function, rather than through a
  boundary, leaves no trace a test can observe; a clause about such a call is read from
  the code, and the ticket says so.
- **A test file is split before the metrics gate's limit.** `rustqual.toml` holds test
  code to a thousand lines per file and a hundred per function; a module whose tests
  would pass that divides them between files by what each proves.

## Coverage

cargo-llvm-cov runs every nextest test under instrumentation, and `just coverage` fails
when line coverage over `crates/*/src/**` falls below 90 per cent. The floor is the
`coverage_floor` variable in the `Justfile`. Doctests are not measured, because
llvm-cov's doctest support needs a nightly toolchain, so `test-doc` runs them uncovered
and the floor is a statement about unit and integration tests. A binary an integration
test runs is instrumented too, so the binary test counts towards the service's lines.
While the crates are small the denominator is too: one untested line moves the
percentage by several points. The recipe also writes `target/llvm-cov/lcov.info`, which
CI keeps as an artifact once the floor has passed.

Coverage counts a snapshot test like any other test. That count proves no specification
clause: every clause needs its own asserting test, even when a snapshot runs the same code.

Distinguish an untested branch from an unreachable one. Defensive code no input can reach
should be deleted rather than covered; see
[Quality philosophy](../explanation/quality-philosophy.md).

## Mutation testing

`just mutants` runs cargo-mutants over both crates with every feature, mutating a copy
of the tree and leaving its report in the ignored `mutants.out/`. The previous report
moves to `mutants.out.old`. Gitignored paths are excluded from the copy. Additional
arguments can be passed through the recipe, for example `just mutants --list`.
It uses `cargo test` so doctests participate; nextest would miss those assertions.

- **Caught:** the mutation compiled and a test failed.
- **Missed:** the changed program still passed its tests; inspect the missing assertion.
- **Unviable:** the mutation did not compile.
- **Timed out:** the test exceeded its time limit; inspect the log before drawing a conclusion.

Exit 0 means every viable mutant was caught, 2 means a survivor, 3 a timeout and 4 a
failing baseline. A red weekly report asks for investigation; it blocks no merge.
The tool never mutates functions named `new`, so constructor logic still needs tests
written by a person. Test code is excluded too. Equivalent mutants may be excluded
through `exclude_re` in `.cargo/mutants.toml`, with a comment explaining why no
observable behaviour changes. Create that file only when needed; do not add the
`mutants` crate or a skip attribute. The policy is explained in
[Quality philosophy](../explanation/quality-philosophy.md).

## Snapshots

No test takes a snapshot yet. The recipes are in place and cargo-insta is pinned with the
other tools, but the `insta` crate becomes a development dependency with the first value
worth snapshotting; until then `just snapshots-check` runs the tests and finds nothing to
compare. The rule below holds from that first snapshot.

A snapshot has two purposes and no other:

1. **Show a reviewer what a value looks like.** It is print-based debugging kept in the
   repository: a reviewer sees the value without running anything.
2. **Cheap change detection.** A change to what an interface hands out appears as a diff
   in a committed file, as a component snapshot shows a changed front end.

**A snapshot never verifies correctness.** The first snapshot records what the code
printed that day; it was not checked against the specification. No clause is proved by
a snapshot. Each clause keeps its own test that asserts, and a ticket's table from
clause to test never names a snapshot test.

Write snapshot tests only after the behaviour's own tests are green, never as the failing
test of a red-green loop. Their names begin `snapshot_`, so readers and filters can
distinguish them. Use a fixed input and a fixed script, never a property-test input.

Render a text picture in test code from what the public surface hands out. Add no
`Display`, method or field to the library to feed a snapshot. A small value, such as an
error, may use its `Debug` or serialised form. Every picture has no trailing whitespace
and ends in a newline.

Snapshots are `.snap` files, never inline in Rust source. They appear as their own diffs,
and acceptance rewrites no source. A test in `src/foo.rs` stores files in
`src/snapshots/`; a test in `src/foo/bar.rs` stores them in `src/foo/snapshots/`; a test
in `tests/foo.rs` stores them in `tests/snapshots/`, beside the source file's directory.
These files are committed. Pending `*.snap.new` and `*.pending-snap` files from runs
outside the recipes are ignored.

`just test`, `just coverage` and `just snapshots-check` fail on a mismatch without
writing a snapshot or pending file. The snapshot gate also rejects a `.snap` file no
test refers to. It runs the entire suite through cargo-insta with nextest and every
feature, so every reference is seen. cargo-insta cannot pass `--locked` to nextest;
the snapshot recipes validate all four locks first and run cargo-insta offline.

A changed snapshot asks whether the change was intended. The person who changed it reads
the diff, reviews pending snapshots through `just snapshots-review` at a terminal, and
accepts intended changes through `just snapshots-accept`, which reruns the tests,
accepts without prompting and deletes any `.snap` file no test refers to. Give the reason
in the pull request or hand-back notes.
The [snapshot workflow](../how-to/test-and-debug.md#read-and-accept-a-changed-snapshot)
explains the commands in order.

## Benchmarks

`just bench` runs two divan benches over libpachinko, with every feature. `version` is
the harness's smoke bench over `libpachinko::version()`. `workloads` runs the queue's
five workloads, W1 to W5, at 10, 1,000 and 100,000 pockets:

| Workload | One sample |
| --- | --- |
| W1 publish | 10,000 publishes of fresh keys, round-robin over the pockets |
| W2 supersede | 10,000 publishes of each pocket's one key at its rank plus one |
| W3 pull 16, ack, release | up to 1,000 cycles of a pull with limit 16, 16 acks and a release |
| W4 publish with every fifth op a consume step | up to 1,000 rounds of four publishes and one W3 cycle |
| W5 pull then lapse on tick | up to 1,000 cycles of a pull, the fake clock past `lease_ttl`, and a tick |

Each sample runs over one `Queue<MemoryRecords, FakeClock, CountingLeaseIds>` with
`Config::default()`, payloads of 64 bytes and the pockets pre-created. divan's
`with_inputs` builds the queue and the sample's publications outside the timed region.
Every call's result passes through `divan::black_box`, and each sample asserts the
outcomes it expects. The unit of one op is one library call on the queue, and divan's
`ItemsCount` counter turns the table's throughput column into ops per second. The bench's
doc comment gives each workload's call count and cycle bound. It also notes that W5's
clock step is timed but not counted. Extra Cargo options may follow the recipe.
[Decision 0019](../decisions/0019-service-runtime-tokio-and-http.md#evidence-from-the-mvp)
records the figures.

Timings depend on the machine and its load, so benchmarks run manually and never in CI
or `just check`. Clippy still compiles the bench targets in the gate.

## Load generation

`just loadgen <workload>` runs the load generator in the excluded `loadgen/` crate,
`pachinko-loadgen`, against a `pachinko serve` that is already running. It speaks HTTP
only and names no workspace crate. Each client is a std thread with its own `ureq`
agent, so it has its own keep-alive connection. The clients run a closed loop: send, read
the whole reply, parse it, count it, send the next. The workloads are `publish`
(single-record `PUT`, C=64), `batch` (batch publish, b=256 at C=8 by default), `consume`
(an untimed fill, then pull, ack each, release, C=64), `mix` (half the clients publish,
half consume) and `lapse`, the lapse probe. Run `just loadgen --help` for the flags and
their defaults. The joins and the fill run before the timed window, and each run's ids
carry the process id and a nonce, so two runs against one service never collide.

A run fails closed. A reply counts only when its status is 2xx and its body parses and
carries the outcome the workload expects. Anything else is an error, and the run prints
it. A run with any error prints no rate, and the process exits non-zero, so a refused
connection or a 404 route can never produce a number. In `consume`, a `no_pocket`
inside the window ends that consumer's loop. The `lapse` run fails in four
cases: the probe pocket is not the front, no batch landed before `expires_at`, the claim
takes more than 60 s, or the queue still holds a pocket after the drain.

The recipe runs `cargo run --locked` against `loadgen/Cargo.lock`, which `just lock-check`
checks beside the workspace, fuzz and pixi lockfiles. The gates `fmt-check` and `clippy` cover the crate
by `--manifest-path`, as they cover `fuzz/`. `loadgen/Cargo.toml` copies the root lint
table, so keep it aligned. `just test-loadgen` runs the crate's own unit tests, which
`just test` cannot reach. No gate or CI job runs the load generator or its tests, because
timings measure the machine.

## Fuzzing

`just fuzz` runs the `smoke` target for 300 seconds; `just fuzz smoke 20` selects a
shorter run. Compilation is additional time. The target feeds arbitrary bytes to a
local UTF-8 round-trip check and calls the library’s version function. It is a harness
smoke test, with no claim about a domain parser. A panic fails the run and writes a
reproducer under `fuzz/artifacts/smoke/`.

The recipe uses the pinned stable toolchain with `--sanitizer none`; it does not need
nightly and provides no sanitizer instrumentation. It checks the locks before running
cargo-fuzz offline, since cargo-fuzz cannot take `--locked`. Run `just sync` after a
lock update. `fuzz/` is excluded from the workspace, has its own committed lockfile and
repeats the workspace’s unsafe-code and non-ASCII-identifier forbid levels and Clippy
policy. Keep the copied lint levels aligned with the root table. The existing
`fmt-check`, `clippy` and `lock-check` recipes reach it by `--manifest-path`, as do the
matching format, fix, sync and lock operations. libfuzzer-sys compiles C++, so the gate
needs Xcode command-line tools on macOS or a C++ compiler such as `g++` on Linux.
The cargo-fuzz 0.13.2 macOS binary is x86 only and needs Rosetta 2 on Apple silicon;
the recipe selects the native rustc host triple explicitly.

The separate `fuzz.yml` workflow runs weekly and by hand, for 300 seconds per target,
with crash artifacts uploaded on failure. It is outside the required CI aggregate.
Corpora, crash artifacts, coverage output and builds are ignored; reproducer files must
be promoted to regression tests when domain targets arrive. See
[decision 0017](../decisions/0017-mutation-bench-and-fuzz.md) for the scope and tradeoffs.

## What the current suite proves

| Suite | Covers |
| --- | --- |
| In-module tests, `crates/libpachinko/src/lib.rs` | `the_version_is_the_manifests`: `version` returns the version the manifest states. |
| Doctests | The example on `libpachinko::version`, which asserts the same value from outside the crate. |
| In-module tests, `crates/pachinko/src/lib.rs` | `run_writes_the_name_and_the_library_version`: `run` writes `pachinko`, the library's version and a newline into a buffer. |
| `crates/pachinko/tests/binary.rs` | `the_binary_prints_its_line_and_exits_zero`: the built binary exits zero, prints that line on standard output and writes nothing to standard error. |

## Related pages

- [Test and debug](../how-to/test-and-debug.md)
- [Quality gates](quality-gates.md)
- [Architecture](../explanation/architecture.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
- [Decision 0009: The Rust quality gate](../decisions/0009-rust-quality-gate.md)
