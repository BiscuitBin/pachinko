---
id: T13
title: "Container image: a multi-stage Dockerfile, `just image`, a CI build"
status: done
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
| `docs/explanation/architecture.md` | shared, own lines | T13's wave-2 sentence (T07 step 6) made present tense, citing decision 0015 |
| `tickets/T13-container-image.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged; rebase on any wave-2 ticket merged since.
2. **The bases.** Build stage: the official `rust` image at the tag equal to the
   toolchain pin, `-slim-bookworm` variant. Run stage: `gcr.io/distroless/cc-debian12`
   (glibc, no shell), `nonroot` tag. Resolve each to a digest on the day
   (`docker buildx imagetools inspect <image>:<tag>`) and write `FROM <image>:<tag>@sha256:<digest>`
   with a comment naming the tag and date. Renovate (T17) keeps them current. The build
   stage's comment also says its tag moves with `rust-toolchain.toml` and `rust-version`
   in one Renovate group (T17), because step 3's toolchain check fails the required
   `image` job whenever they differ.
3. **The Dockerfile.** Build stage copies `rust-toolchain.toml`, both manifests,
   `Cargo.lock` and `crates/`, and runs `cargo build --release --locked -p pachinko`.
   The rust image carries rustup, which reads `rust-toolchain.toml` and installs a
   missing pinned toolchain on first use, so `rustc --version` run beside that file
   always reports the pin. The build-time check therefore runs `rustc --version` from a
   directory with no `rust-toolchain.toml` and compares it with the pin's `channel`, so a
   mismatch fails loudly rather than downloading silently. Run stage copies the binary only, runs as `nonroot`, entrypoint the binary.
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
   the manifest and the map, and add the index row. Make T13's sentence in
   `docs/explanation/architecture.md` (T07 step 6) present tense, citing decision 0015.
7. `just check` (without docker), the final adversarial review, `just check` again if
   fixes landed, then **authorisation required: push and open a pull request**; read the
   `image` job's run.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff, before the push. Ask:
   whether each digest is its named tag's on the day; whether the run stage really runs
   as `nonroot`; whether `architecture.md` still calls the image future work; whether the toolchain check fails when the pin and the image's rust
   differ (try it by building with a bumped `channel`); whether `image` is in `check`'s `needs`.

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

Implementation, local verification and PR CI verification are complete. The user
subsequently authorised pushing this branch and opening a visual PR: PR 14 is
`https://github.com/BiscuitBin/pachinko/pull/14`. The successful first run and
its tested merge commit are recorded below. No registry publication or repository
setting change was performed.

### Baseline — commit `2c93ae3`, untouched

The first attempt could not find `bg-project-check` (exit 127). Ran the fresh-worktree
`just initialize` authorised by §11, with rustup's cargo first on `PATH`; the
secondary worktree skipped installing hooks. The full untouched baseline then passed:

```text
$ just check
All checks passed and the worktree is unchanged.
exit 0
```

The supplied branch used uppercase `T13`; renamed it to the ticket's lowercase
`ticket/t13-container-image` through a temporary name because the filesystem is
case-insensitive. Fetched `origin` on 2026-10-24 UTC; `origin/main` remains `2c93ae3`,
the base of this lane. No rebase changes were needed.

### Pins — commit `2c93ae3` with uncommitted changes

Resolved on 2026-10-24 UTC (2026-10-23 locally), using each full, unfiltered
`docker buildx imagetools inspect` output, exit 0:

| Tag | Index digest |
| --- | --- |
| `rust:1.99.1-slim-bookworm` | `sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be` |
| `gcr.io/distroless/cc-debian12:nonroot` | `sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f` |

On the same day, read `gh api repos/<owner>/<repo>/git/ref/tags/<tag>` for each
row below (exit 0; all returned commit objects, so no annotated-tag dereference).
The conformance reviewer independently confirmed them through `git ls-remote`.

| Action | Tag | Commit |
| --- | --- | --- |
| `actions/checkout` | `v7.0.1` | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `docker/setup-buildx-action` | `v4.4.1` | `f87e5991a6d7451dcb8d9637bfbc97413f497069` |
| `docker/build-push-action` | `v7.4.0` | `c3c9e263c25d99ce0380d002d59b67737d91b0dc` |

### Local verification — commit `2c93ae3` with uncommitted changes

`docker version` exited 0: client and server Docker 29.4.0, OrbStack on
`darwin/arm64` with a `linux/arm64` daemon. Thus §12 claim 9 holds locally;
`ubuntu-latest` is verified by the first PR run recorded below. The recipe adds `--platform linux/amd64`
to step 4's command to enforce this ticket's platform scope on this host.

The first uncached build finished the release compilation in 6.24 seconds;
subsequent verification reused its layers. The runtime image reports 9,359,369 bytes
(about 9.4 MB); the build toolchain stays in the build stage.

```text
$ just image
 docker build --platform linux/amd64 --tag pachinko:dev .
#15 naming to docker.io/library/pachinko:dev done
#15 DONE 0.1s
exit 0

$ docker run --rm pachinko:dev
pachinko 0.1.0
exit 0

$ docker image inspect pachinko:dev --format '{{.Config.User}} {{.Config.Entrypoint}} {{.Architecture}} {{.Size}}'
nonroot:nonroot [/pachinko] amd64 9359369
exit 0
```

The bare `docker run` also emits Docker's expected amd64-on-arm64 emulation warning.
Both `FROM` lines carry the full digests above (`rg -n '^FROM ' Dockerfile`, exit 0).
The workflow adds `image` to `check.needs`, loads before running, and never pushes.

Copied the build inputs to `ai_tmp/mismatch/`, changed only the copied `channel`
from `1.99.1` to `1.99.2`, and built that context with `--platform linux/amd64`.
The build rejected the mismatch before copying crates or running cargo:

```text
$ docker build --platform linux/amd64 --tag pachinko:t13-mismatch ai_tmp/mismatch
Rust base image mismatch: pin=1.99.2; image=rustc 1.99.1 (58cf8f9c3 2026-10-13)
exit 1 (expected)
```

The original workspace pin remained unchanged. The negative-control wrapper also
required a nonzero Docker status and the specific mismatch diagnostic, and exited 0.

```text
$ just check-docs
Validated 42 pages and 43 canonical topics.
exit 0

$ just check
All checks passed and the worktree is unchanged.
exit 0
```

The staged-file hook gate also passed, including the new files, actionlint, secret
scanning, Markdown, spelling, links and the documentation contract. The initial docs
failure was Docker's Go template braces being treated as unresolved handbook template
syntax; the how-to now uses plain inspection and explains the expected JSON fields.
No Rust source, dependency, lockfile, threshold or source-quality gate changed.

### Checkpoint 1 review — commit `2c93ae3` with uncommitted changes

Two fresh-context reviewers inspected the complete lane diff: Claude Code with
`claude-opus-5-5` for conformance, and a Codex subagent with `gpt-6-astra` for breakage.
A separate Codex `gpt-6-astra` triager checked every finding against the files and raw
verification logs, and applied the one valid fix as the only writer. The breakage
reviewer found no actionable defect. The conformance findings were:

- C1, medium: valid. Decision 0015 omitted §8's required Related pages section.
  The triager added it and links to the how-to and decision 0014.
- C2, low: rejected as a defect. The copied toolchain file makes rustup download
  missing pinned components; step 3 requires that file beside the build, and the
  how-to documents the network requirement. The compiler mismatch guard still fails
  before the build can install a different compiler.
- C3, informational: no defect. The explicit amd64 flag enforces the stated scope.
- C4, informational: current ignore patterns match. Future `.gitignore` edits must
  update `.dockerignore`; no new drift gate is required by this ticket.

No high-severity finding, so no second review round was required. The root agent
also applied `review-change`: all nine invariants remain intact, documentation has
one owner per topic, and the change stays inside this ticket's files. The unavailable
`code-review` skill was covered by that repository review and the independent reviews.
The final full gate was rerun after the documentation fix. Raw review and verification
files remain gitignored under `ai_tmp/`.

One attempted Claude breakage call was rejected by automatic approval review because
it would export repository content to an external model API. That pass used the fresh
Codex reviewer instead. The separately approved Claude conformance call completed.

The subsequently authorised push and PR supplied the pending CI evidence below;
the ticket is now `done`.

### PR evidence — source commit `d47e74c`, tested merge commit `47df0d7`

On 2026-10-24 UTC, the user requested `$show-me` followed by `$visual-pr` and
explicitly asked to open the PR with diagrams. Pushed `ticket/t13-container-image`
and opened PR 14. The visual description includes build-stage boundaries, the
runtime call sequence, required CI dependencies, and observed local outcomes.
Its requested saved artifact is `.humanlayer/tasks/pr-14/description.md`.

Re-ran `just image`, `docker run`, image inspection and `just check` on source
commit `d47e74c` before the push: all exited 0, the same output and image
configuration as above, and the full gate ended with
`All checks passed and the worktree is unchanged.`

CI run: `https://github.com/BiscuitBin/pachinko/actions/runs/40065797710`.
Image job: `https://github.com/BiscuitBin/pachinko/actions/runs/40065797710/job/120788668006`.
The image checkout log confirms tested merge commit
`47df0d7d3202f78c176ec7a66605be1aa8374bff`, merging source commit
`d47e74c5805f3271758d2130502508d41bc7915e` into main `2c93ae3`.
The run API reports completed/success, with `rust`, `coverage`, `deny`,
`documents`, `image`, `semver` and required `check` all successful.

```text
$ gh run view 40065797710 --json status,conclusion
{"conclusion":"success","status":"completed"}
exit 0

$ docker run --rm pachinko:dev  # from the image job log
pachinko 0.1.0
step conclusion: success
```

The image log confirms both Docker client and server are available on
`ubuntu-latest`, and records `push: false`, `load: true` and successful runtime
output. Thus both halves of §12 claim 9 hold; T17 can carry this result into the
conventions table. The follow-up commit adds only this completion evidence and
the user-requested saved visual description, with no build or runtime changes.

## Open points

- The image is `linux/amd64` only. `linux/arm64` joins when something deploys it.
