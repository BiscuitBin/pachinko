# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each crate has its own unreleased section and versions. Release headings name the crate
and version, dated on the UTC day its tag is pushed. The release process is described in
[Maintenance](docs/operations/maintenance.md#cut-a-release).

## libpachinko [Unreleased][libpachinko-unreleased]

### Added

- Manual divan benchmarks and stable byte fuzzing with a weekly crash-reporting job.
- Workspace mutation testing through `just mutants` and a weekly report workflow.
- An Apache-2.0 library, `libpachinko`, exposing its version. The domain awaits its
  Allium specifications.
- A pinned toolchain and tools, the `just initialize` setup, and the read-only
  `just check` gate: linting, tests, coverage, API documentation, dependency checks,
  metrics and a probed effects boundary for the library.
- The handbook, architecture decisions, a placeholder Allium module and its pinned
  checker, and an agent contract with skills and provider bridges.
- GitHub Actions for continuous integration and dependency advisory audits, a shared
  pre-commit hook, and both crates' API reference hosted on GitHub Pages.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow,
  with a semver report for the library. Neither crate is published to crates.io.

## pachinko [Unreleased][pachinko-unreleased]

### Added

- An Apache-2.0 service, `pachinko`, that writes its name and the library version.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow.
  The crate is not published to crates.io.

[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
