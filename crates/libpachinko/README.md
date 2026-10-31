# libpachinko

`libpachinko` is the std library crate of the pachinko workspace. It implements the
queue the Allium module `docs/specs/pachinko.allium` specifies, and the module wins
wherever the two disagree. One deployment is one `Queue` of pockets: publishers publish
records into pockets, a pocket keeps one record per record key and a higher rank
supersedes in place, and a consumer leases one pocket at a time, pulls its oldest records
in bounded batches, and acks, nacks, renews or releases. `Queue` has one method per
trigger of the module, and `tick` processes lease lapses and the decay of superseded
records and dead letters.

Its modules are `clock` (`Timestamp`, the `Clock` trait and `FakeClock`), `config`
(`Config`, the module's five figures, and `validate`), `error` (the `thiserror` enums),
`records` (`PocketRecords`, the record-store trait each pocket's records sit behind,
with `MemoryRecords` as its reference) and `queue` (`Queue`, `LeaseId` and the values
its methods return). The `testing` feature adds the records contract, which the tests
of both crates run, and the queue's inspection seam, which this crate's tests read.

The library performs no effect itself. The clock is its first effect trait, with
`FakeClock` beside it; randomness, the filesystem, the network, the environment and
threads get theirs when a specification names them. The `pachinko` service supplies the
system clock and an Arrow record store.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/libpachinko/),
or run `just doc` from the repository root and open `target/doc/libpachinko/index.html`.
The tested API examples live in rustdoc.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
