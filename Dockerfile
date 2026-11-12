# rust:1.99.1-slim-bookworm, resolved 2026-10-24 (UTC).
# Move this tag with rust-toolchain.toml and rust-version in one Renovate group
# (T17); the check below fails the required image job if the compiler differs.
FROM rust:1.100.0-slim-bookworm@sha256:7c7d4eb4d05b7781187a327809466cfddf29753f6defdc3d19a0e291951b7a37 AS build

WORKDIR /build
COPY rust-toolchain.toml ./
# / has no rust-toolchain.toml: inspect the image's installed default compiler,
# before rustup can auto-install the project's pin. A mismatch must fail loudly.
RUN pin=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml) \
    && have=$(cd / && RUSTUP_AUTO_INSTALL=0 rustc --version) \
    && version=$(printf '%s\n' "$have" | cut -d ' ' -f 2) \
    && { [ -n "$pin" ] && [ "$version" = "$pin" ]; } \
    || { printf 'Rust base image mismatch: pin=%s; image=%s\n' "$pin" "$have" >&2; exit 1; }
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked -p pachinko

# gcr.io/distroless/cc-debian12:nonroot, resolved 2026-10-24 (UTC).
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f AS run
COPY --from=build /build/target/release/pachinko /pachinko
USER nonroot:nonroot
ENTRYPOINT ["/pachinko"]
