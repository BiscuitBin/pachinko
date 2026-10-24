# Security policy

## Reporting a vulnerability

Report suspected vulnerabilities through
[GitHub's private vulnerability reporting](https://github.com/BiscuitBin/pachinko/security/advisories/new)
on `BiscuitBin/pachinko`. Please do not open a public issue for anything you believe is
exploitable.

Include the affected commit, what you did, what happened, and what you expected. A small
program or a sequence of commands that reproduces the problem helps us investigate.

## Supported versions

Only `main` is supported. No version has been released, and neither crate is published
to crates.io. Fixes land on `main`; there are no release branches to backport to.

## What is in scope

- Vulnerabilities in `libpachinko` or the `pachinko` service.
- Credential material committed to this repository.
- A tampered dependency, tool binary or GitHub Action, or a lockfile that disagrees with
  its manifest.

## Out of scope

Applications and infrastructure maintained outside this repository are outside this
policy's scope. A vulnerability originating in these crates or their build tooling is
still in scope when discovered through a consumer.

## What this project already does

The workspace forbids unsafe Rust and checks the library's effects boundary with
probes. Dependency lockfiles are committed, and `just lock-check` rejects drift between
manifests and lockfiles. Every `just check` checks licences, dependency sources and bans.
Advisory audits run weekly and on pull requests. Remote hooks and GitHub Actions are
pinned to commit SHAs, and the pre-commit gate scans for secrets with redacted output.

The [security model](docs/explanation/security-model.md) explains these controls, tool
provenance, workflow permissions and the limits of the effects checks. Today the service
only writes its name and the library version; it has no network listener.
