# libpachinko

`libpachinko` is the std library crate of the pachinko workspace. It implements the
domain `docs/specs/pachinko.allium` defines: one queue of pockets. Publishers publish
records into pockets, a pocket keeps one record per record key and a higher rank
supersedes in place, and a consumer leases one pocket at a time and pulls its oldest
records in bounded batches. `Queue` is the entry point, with publisher and consumer
joins, `publish`, `pull`, `lease_view`, the four lease actions `ack`, `nack`, `renew`
and `release`, and `tick`, which lapses expired leases and decays superseded records and
dead letters.

The library performs no effect itself. The clock is the `Clock` trait and lease ids,
which need randomness, come from the `LeaseIds` trait; `FakeClock` and
`CountingLeaseIds` sit beside them for tests, and the `pachinko` service supplies the
real implementations.

A pocket's records live behind the `PocketRecords` trait, the store seam. `MemoryRecords`
is its one implementation, and every implementation passes one contract,
`records::contract::check_contract`.

The `testing` feature adds that contract and `Queue::snapshot`, the inspection seam the
tests check every invariant through. No normal build turns it on.

`version()` returns the version in the crate's manifest:

```rust
assert_eq!(libpachinko::version(), env!("CARGO_PKG_VERSION"));
```

Read the [API reference for main](https://biscuitbin.github.io/pachinko/libpachinko/),
or run `just doc` from the repository root and open `target/doc/libpachinko/index.html`.
The tested API examples live in rustdoc.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
