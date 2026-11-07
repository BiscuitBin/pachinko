# pachinko

`pachinko` is the service crate of the pachinko workspace: an HTTP/1.1 JSON server over
`libpachinko`'s queue, on tokio and axum, as
[decision 0019](../../docs/decisions/0019-service-runtime-tokio-and-http.md) decided. One
owner task holds the queue in memory and applies every command in arrival order; the
service supplies the library's real effects, `SystemClock` and `EpochSerialLeaseIds`.

From the repository root, `just build` builds both crates. Then:

- `./target/debug/pachinko` prints `pachinko <library version>` and exits.
- `./target/debug/pachinko serve --listen 127.0.0.1:0` prints `listening on <address>`
  and serves until SIGTERM or ctrl-c, then exits 0. Without `--listen` it binds
  `127.0.0.1:8080`.

The routes, bodies and error codes are in the
[HTTP API](../../docs/reference/http-api.md) reference, the flags in
[Configuration](../../docs/reference/configuration.md#service-flags), and a curl
walkthrough in [Test and debug](../../docs/how-to/test-and-debug.md#run-the-service).
There is no authentication or TLS; the surface is loopback by default.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/pachinko/),
or run `just doc` and open `target/doc/pachinko/index.html`.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
