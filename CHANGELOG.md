# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each crate has its own unreleased section and versions. Release headings name the crate
and version, dated on the UTC day its tag is pushed. The release process is described in
[Maintenance](docs/operations/maintenance.md#cut-a-release).

## libpachinko [Unreleased][libpachinko-unreleased]

### Changed

- The specification closes open question 8 by decision 0019: a pull, ack, nack, renew or
  release processed after its lease's `expires_at` but before the lapse succeeds. The
  root module keeps eight open questions, and decision 0019 confirms its five
  configuration defaults for the MVP.

### Added

- The library tracer: `Queue` over a record store, a clock and a lease-id source, with
  publisher and consumer joins, publish (new pocket, new key, supersede, stale shed),
  pull (claim the front pocket, pull within a lease, no pocket available) and the lease
  view; the `Clock` and `LeaseIds` effect traits with `FakeClock` and
  `CountingLeaseIds`; the `PocketRecords` store seam with `MemoryRecords`; `Config` with
  the specification's defaults and load checks; one `thiserror` error enum per
  operation; and a `testing` feature with the records contract and the queue snapshot.
- The first behavioural specification, `docs/specs/pachinko.allium`: pockets holding one
  record per key, higher ranks superseding in place, exclusive renewable leases, parking
  after consecutive lease expiries, and decaying superseded records and dead letters.
  Specified, not yet implemented.
- Manual divan benchmarks and stable byte fuzzing with a weekly crash-reporting job.
- Workspace mutation testing through `just mutants` and a weekly report workflow.
- An Apache-2.0 library, `libpachinko`, exposing its version. The domain awaits its
  implementation.
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

- An Apache-2.0 service, `pachinko`, that writes its name and the library version.
- `pachinko serve`: an HTTP/1.1 JSON API over one owner task holding the queue, with
  the publisher and consumer joins, single-record publish, pull and the lease read, and
  `GET /healthz`; six flags for the listen address and the specification's five config
  values; the real `SystemClock` and `EpochSerialLeaseIds`; and a clean shutdown on
  SIGTERM or SIGINT (decision 0019).
- A digest-pinned container image, `just image`, and a required CI build and run without publication.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow.
  The crate is not published to crates.io.

[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
