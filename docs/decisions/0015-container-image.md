---
title: "Decision 0015: The container image"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_container_image]
requires: []
---

# Decision 0015: The container image

## Context

The foundation includes a minimal service binary. Building its deployment package
now proves that the workspace can produce an executable container before a runtime
or transport is chosen. The service currently writes its version and exits;
[decision 0014](0014-service-runtime-deferred.md) leaves its eventual surface to the
specifications.

## Decision

Use a two-stage Dockerfile: the official Rust slim Bookworm image builds with the
pinned toolchain and locked Cargo dependencies; distroless `cc-debian12:nonroot`
runs the copied binary as `nonroot:nonroot`. Both bases carry tag and digest, with
the resolution day recorded beside them. The build verifies the base's installed
default compiler before rustup can download the project's pin. The Rust image tag,
`rust-toolchain.toml` and `rust-version` move together in T17's Renovate group.

Distroless supplies glibc and the C/C++ runtime libraries a Rust std binary can
need without carrying a shell, package manager or compiler. Debian 12 in both
stages keeps the build and runtime on the same glibc generation. The image targets
`linux/amd64`; arm64 joins when something deploys it.

`just image` builds locally. CI builds, loads and runs that same image without
pushing it, and its `image` job is required through `check`. Docker stays outside
`just check`, so contributors can run the full source gate without Docker.

## Consequences

The runtime image stays small and runs without root privileges. Troubleshooting
cannot depend on an interactive shell inside it. There is no registry, publication,
signing or release tag for the image yet: a service that only prints its version
has no deployment consumer, and publication needs a release policy. No port,
health check or configuration is added ahead of a specification.

## What would reopen this

A deployment consumer that needs a registry reopens image publication and the
release policy. A runtime that requires a shell reopens the distroless choice.
An arm64 deployment reopens the platform scope.

## Related pages

- [Build the container image](../how-to/build-the-container-image.md)
- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
