# pachinko

`pachinko` is the service crate of the pachinko workspace, a std binary built around
`libpachinko`. The binary calls `run()` with its command line and standard output. With
no argument it writes `pachinko <library version>` and exits; `pachinko serve [flags]`
serves the library's queue over HTTP on a tokio runtime until SIGTERM or ctrl-c, logging
to standard error. The routes are in [HTTP API](../../docs/reference/http-api.md), the
flags in [Configuration](../../docs/reference/configuration.md), and the choice of
runtime in
[decision 0019](../../docs/decisions/0019-tokio-and-axum-service.md).

The crate supplies the real implementations of the library's effect traits:
`SystemClock` reads the system time, and `RandomLeaseIds` draws each lease id from the
operating system. The queue lives in memory, so a restart empties it, and the listener
speaks plain HTTP with no TLS or authentication.

From the repository root, `just build` builds both crates. Run
`./target/debug/pachinko` to print the current name and library version, or
`./target/debug/pachinko serve` to listen on `127.0.0.1:8080`.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/pachinko/),
or run `just doc` and open `target/doc/pachinko/index.html`.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
