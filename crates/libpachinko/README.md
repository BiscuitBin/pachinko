# libpachinko

`libpachinko` is the std library crate of the pachinko workspace. It implements the
domain the Allium module `docs/specs/pachinko.allium` specifies: a single-deployment
queue of pockets. Publishers publish records into pockets, one record per key, and a
higher rank supersedes in place; a consumer leases one pocket at a time, pulls its
oldest records in bounded batches, and acks, nacks, renews or releases under the lease
its batch names. Leases lapse, pockets park after consecutive expiries, and superseded
records and dead letters decay. `Queue` holds it all, synchronously, and every refusal
is a variant of `Error`.

The library performs no effect itself. It reaches the clock through the trait `Clock`
and mints lease ids through the trait `LeaseIds`, with the fakes `FakeClock` and
`CountingLeaseIds` beside them for tests. The `pachinko` service supplies their real
implementations and serves the queue over HTTP.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/libpachinko/),
or run `just doc` from the repository root and open `target/doc/libpachinko/index.html`.
The tested API example lives in rustdoc.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
