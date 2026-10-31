# pachinko

`pachinko` is the service crate of the pachinko workspace, a std binary built around
`libpachinko`. It supplies the real implementations of the library's traits:
`SystemClock`, the wall clock, and `ArrowRecords`, which holds each pocket's records as
one Arrow `RecordBatch` (decision 0020). `serve` runs the queue behind an Arrow Flight
API on tokio (decision 0019): one task owns the queue, a ticker processes lease lapses
and decay, and both live in a `JoinSet` that `serve` joins before it returns. The
binary's `main` calls `run()` and nothing else.

## Running it

From the repository root, `just build` builds both crates. With no arguments,
`./target/debug/pachinko` prints `pachinko <library version>` and exits. With `serve`
it binds a listener, prints `listening on <address>`, and serves until an interrupt
(Ctrl-C):

```console
./target/debug/pachinko serve --listen 127.0.0.1:50051 --lease-ttl 30s
```

The options are `--listen ADDR`, by default `127.0.0.1:0`, a free loopback port;
`--lease-ttl`, `--superseded-ttl` and `--dead-letter-ttl`, each a whole number with a
unit, `ms`, `s`, `m` or `h`; and `--max-consecutive-expiries` and `--max-pull-batch`,
whole numbers. An option left out keeps the module's default, and a configuration the
module's config invariants reject is refused before anything binds. An error is one
line on standard error and exit code 1.

`do_put` publishes, `do_get` pulls, and `do_action` carries the joins, the consumer's
four lease actions and a `status` read of the consumer's lease; every body and reply is
an Arrow IPC stream. The handbook's API reference, `docs/reference/api.md`, lists the
mapping.

The Flight service has no authentication and no encryption: anyone who reaches the port
can publish, pull, ack or nack. It listens on loopback unless `--listen` says otherwise,
and `docs/explanation/security-model.md` explains the exposure.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/pachinko/),
or run `just doc` and open `target/doc/pachinko/index.html`.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
