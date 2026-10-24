# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- An Apache-2.0 Rust workspace with `libpachinko`, exposing its version, and `pachinko`,
  a minimal service that writes its name and the library version. Both crates remain
  unpublished; the domain awaits its Allium specifications.
- A pinned toolchain and tools, the `just initialize` setup, and the read-only
  `just check` gate: linting, tests, coverage, API documentation, dependency checks,
  metrics and a probed effects boundary for the library.
- The handbook, architecture decisions, a placeholder Allium module and its pinned
  checker, and an agent contract with skills and provider bridges.
- GitHub Actions for continuous integration and dependency advisory audits, a shared
  pre-commit hook, and both crates' API reference hosted on GitHub Pages.

[Unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
