---
id: T13
title: "Container image: a multi-stage Dockerfile, `just image`, a CI build"
status: open
depends_on: [T11]
parallel_with: [T12, T14, T15, T16]
branch: ticket/t13-container-image
estimated_size: M
---

# T13: Container image: a multi-stage Dockerfile, `just image`, a CI build

## Context

The service is a minimal std binary (`CONVENTIONS.md` §1 decision 4), and the maintainer
wants it packaged as a container image from the foundation on (decision 8), so that the
deployment shape is proved before the runtime is chosen. L has no image and no draft for
one; this ticket is new design, kept small.

This is a wave-2 ticket: it edits only its own lines in the shared files and merges
rebased on whichever wave-2 ticket merged last (`CONVENTIONS.md` §11). docker is a
prerequisite here and nowhere else (`CONVENTIONS.md` §12 claim 9).

## Goal

`just image` builds an image whose entrypoint is the `pachinko` binary, built with the
toolchain `rust-toolchain.toml` pins, from base images pinned by digest; running it prints
`run()`'s line and exits 0. A CI job, `image`, builds the same image on every pull
request without pushing it and joins `check`'s `needs` (`CONVENTIONS.md` §10). The
how-to and decision 0015 say what the image is and is not.

## Non-goals

- No registry, no push, no tag, no signing: a published image waits for the release
  process to cover it and for the service to do something.
- No runtime, port, health check or configuration: decision 0014.
- No `docker` call inside `just check`. docker is not a prerequisite for contributors.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Dockerfile` | new | two stages: build, run |
| `.dockerignore` | new | everything Git ignores, plus `.git` and `tickets/` |
| `Justfile` | shared, own lines | the `image` recipe, outside `check` |
| `.github/workflows/ci.yml` | shared, own lines | the `image` job, in `check`'s `needs` |
| `docs/how-to/build-the-container-image.md` | new | how to build and run it |
| `docs/decisions/0015-container-image.md` | new | `CONVENTIONS.md` §8 |
| `docs/manifest.yml` | shared, own lines | two entries |
| `docs/README.md` | shared, own lines | two links |
| `docs/decisions/README.md` | shared, own lines | one row |
| `docs/reference/commands.md` | shared, own lines | the `image` row |
| `CHANGELOG.md` | shared, own lines | one `[Unreleased]` line |
| `tickets/T13-container-image.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged; rebase on any wave-2 ticket merged since.
2. **The bases.** Build stage: the official `rust` image at the tag equal to the
   toolchain pin, `-slim-bookworm` variant. Run stage: `gcr.io/distroless/cc-debian12`
   (glibc, no shell), `nonroot` tag. Resolve each to a digest on the day
   (`docker buildx imagetools inspect <image>:<tag>`) and write `FROM <image>:<tag>@sha256:<digest>`
   with a comment naming the tag and date. Renovate (T17) keeps them current.
3. **The Dockerfile.** Build stage copies `rust-toolchain.toml`, both manifests,
   `Cargo.lock` and `crates/`, and runs `cargo build --release --locked -p pachinko`.
   The rust image carries rustup, which reads `rust-toolchain.toml`; a build-time check
   (`rustc --version` against the pin) makes a mismatch fail loudly rather than download
   silently. Run stage copies the binary only, runs as `nonroot`, entrypoint the binary.
4. **The recipe.** In the `Justfile`'s develop section:
   `image: docker build --tag pachinko:dev .`, with a comment saying it needs docker and is
   outside `check`. Then `docker run --rm pachinko:dev` and quote the line it prints.
5. **The CI job.** `image`, on `ubuntu-latest`, timeout 20: checkout
   (`persist-credentials: false`), `docker/setup-buildx-action`, `docker/build-push-action`
   with `push: false`, `load: true`, `tags: pachinko:dev` and the GitHub Actions cache,
   each pinned to a SHA read on the day; then `docker run --rm pachinko:dev`. Without
   `load: true` the `docker-container` driver leaves the image in the BuildKit cache,
   where `docker run` cannot find it. Add `image` to `check`'s `needs`.
6. **The docs.** The how-to: prerequisites, `just image`, `docker run`, what the image
   contains and why distroless. Decision 0015: why an image now, why distroless, why no
   push, what would reopen it (a registry, a runtime that needs a shell). Register both in
   the manifest and the map, and add the index row.
7. `just check` (without docker), the final adversarial review, `just check` again if
   fixes landed, then **authorisation required: push and open a pull request**; read the
   `image` job's run.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff, before the push. Ask:
   whether each digest is its named tag's on the day; whether the run stage really runs
   as `nonroot`; whether the toolchain check fails when the pin and the image's rust
   differ; whether `image` is in `check`'s `needs`.

## Acceptance criteria

- `just image` output is quoted, and `docker run --rm pachinko:dev` prints
  `pachinko <version>` and exits 0.
- The `image` job is green on the pull request and `check` lists it in `needs`.
- Every `FROM` line carries a digest; every new remote `uses:` a SHA with a version comment.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just image
docker run --rm pachinko:dev; echo "exit $?"
docker image inspect pachinko:dev --format '{{.Config.User}} {{.Config.Entrypoint}}'
rg -n '^FROM ' Dockerfile
just check
```

## Hand-back notes

To be written by the executing agent: the digests and the day, the image size, the build
output, the CI run URL, the outcome of `CONVENTIONS.md` §12 claim 9, and the review
record for each checkpoint (`CONVENTIONS.md` §14).

## Open points

- The image is `linux/amd64` only. `linux/arm64` joins when something deploys it.
