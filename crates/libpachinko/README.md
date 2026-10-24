# libpachinko

`libpachinko` is the std library crate of the pachinko workspace. Its domain is not
defined yet: the Allium specifications under `docs/specs/` will define its behaviour,
and the implementation follows them. Today its public API is one item, `version()`,
which returns the version in the crate's manifest.

The library performs no effect itself. As the specifications introduce effects, it
will define traits with fakes beside them for the clock, randomness, the filesystem,
the network, the environment and threads. The `pachinko` service will supply their real
implementations.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/libpachinko/),
or run `just doc` from the repository root and open `target/doc/libpachinko/index.html`.
The tested API example lives in rustdoc.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
