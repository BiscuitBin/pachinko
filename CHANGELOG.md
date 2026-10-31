# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each crate has its own unreleased section and versions. Release headings name the crate
and version, dated on the UTC day its tag is pushed. The release process is described in
[Maintenance](docs/operations/maintenance.md#cut-a-release).

## libpachinko [Unreleased][libpachinko-unreleased]

### Added

- The queue `docs/specs/pachinko.allium` specifies, every rule implemented: `Queue`, with
  one method per trigger and a `tick` for lease lapses and decay; the lease id as a
  fencing token; `Config` with the module's five figures; and `thiserror` error enums.
  No open question of the specification is answered.
- The first effect trait, `Clock`, with `FakeClock` beside it, and the `PocketRecords`
  record-store trait, with `MemoryRecords` as its reference. A `testing` feature exposes
  the records contract and an inspection seam to the tests.
- Tests covering every obligation `just plan-spec pachinko` lists, each named by its id,
  and a proptest state machine over the module's invariants and transition graphs.
- The first behavioural specification, `docs/specs/pachinko.allium`: pockets holding one
  record per key, higher ranks superseding in place, exclusive renewable leases, parking
  after consecutive lease expiries, and decaying superseded records and dead letters.
- Manual divan benchmarks and stable byte fuzzing with a weekly crash-reporting job.
- Workspace mutation testing through `just mutants` and a weekly report workflow.
- An Apache-2.0 library, `libpachinko`, exposing its version.
- A pinned toolchain and tools, the `just initialize` setup, and the read-only
  `just check` gate: linting, tests, coverage, API documentation, dependency checks,
  metrics and a probed effects boundary for the library.
- The handbook, architecture decisions, a placeholder Allium module and its pinned
  checker, and an agent contract with skills and provider bridges.
- GitHub Actions for continuous integration and dependency advisory audits, a shared
  pre-commit hook, and both crates' API reference hosted on GitHub Pages.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow,
  with a semver report for the library. Neither crate is published to crates.io.
- Renovate configuration over the repository's pins, each update held for approval on its dashboard
  except vulnerability fixes, with the pins that stay manual named in the handbook.

## pachinko [Unreleased][pachinko-unreleased]

### Added

- `pachinko serve`, which serves the queue over Arrow Flight on tokio: publish, pull,
  the joins, ack, nack, renew, release and a lease status, every body and reply an Arrow
  IPC stream. It listens on loopback unless `--listen` says otherwise, takes the
  module's five figures as options, and is unauthenticated and unencrypted.
- One task owning the queue and a ticker processing lapses and decay, both in a
  `JoinSet` joined at shutdown, where a request whose handler has not begun is answered
  `UNAVAILABLE`.
- `SystemClock`, and `ArrowRecords`, which holds each pocket's records as one Arrow
  `RecordBatch`, with a divan benchmark of publish, pull and ack, and the store's steps.
- An Apache-2.0 service, `pachinko`, that writes its name and the library version.
- A digest-pinned container image, `just image`, and a required CI build and run without publication.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow.
  The crate is not published to crates.io.

[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
