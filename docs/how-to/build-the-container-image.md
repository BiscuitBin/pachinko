---
title: "Build the container image"
kind: "how-to"
audience: [contributor, maintainer, operator, agent]
canonical_for: [container_image_workflow]
requires: []
---

# Build the container image

Install Docker with a running daemon, and `just`. The image targets `linux/amd64`;
another host architecture needs Docker's amd64 emulation. Docker is a prerequisite
for this recipe only; `just check` does not call it. The first build needs network
access to pull the digest-pinned base images and any missing Rust components.

From the repository root:

```sh
just image
docker run --rm pachinko:dev
```

The container prints `pachinko <version>` (the library's version, currently `0.1.0`)
and exits 0. Inspect its configured user and entrypoint with:

```sh
docker image inspect pachinko:dev
```

Expect `Config.User` to be `nonroot:nonroot` and `Config.Entrypoint` to be
`["/pachinko"]`. The final image contains the release binary and
the glibc runtime from distroless Debian 12, with no Rust toolchain or shell.
The build stage uses the official slim Bookworm Rust image at the exact toolchain
pin, and `cargo build --release --locked -p pachinko`. Before building, it checks
the image's default compiler outside the project directory, so rustup cannot hide
a stale base image by installing the project's new pin. Update the Rust base tag,
its digest, `rust-toolchain.toml` and `rust-version` together.

CI builds this Dockerfile, loads the image locally and runs it on every pull
request. Its `image` job joins the required `check` aggregate. Nothing pushes an
image to a registry. [Decision 0015](../decisions/0015-container-image.md) explains
why this foundation includes an image and why it uses distroless.

## Run the service in the container

The entrypoint is the bare binary, so `serve` and its flags are run arguments. Bind
all interfaces inside the container, because the default loopback address is
unreachable through a published port:

```sh
docker run -d -p 8080:8080 --name pachinko pachinko:dev serve --listen 0.0.0.0:8080
curl -i http://127.0.0.1:8080/readyz
docker stop -t 15 pachinko
```

Use `GET /readyz` as the health check: it answers 204 while serving and 503 while
starting or stopping ([HTTP API](../reference/http-api.md#get-readyz)). The flags are in
[Configuration](../reference/configuration.md#service-flags); the service reads no
environment variable.

`docker stop` sends SIGTERM and kills the process after its timeout, `-t`, ten seconds
by default. The service needs up to twice `--shutdown-grace-ms` (the grace, then the
bounded join after it), plus the longest command or tick the owner applies and the
release of the queue's state, which you estimate from your own load. With the default
grace of 5000 ms the two grace periods alone equal `docker stop`'s default, so pass a
`-t` above it, as above, or a smaller grace; otherwise the container is killed
mid-join.

## Related pages

- [Commands](../reference/commands.md)
- [Architecture](../explanation/architecture.md)
