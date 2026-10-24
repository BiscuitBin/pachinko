# pachinko

`pachinko` is the service crate of the pachinko workspace, a minimal std binary built
around `libpachinko`. Its public API is one item, `run()`, which writes
`pachinko <library version>` to a supplied writer. The binary calls it with standard
output and exits.

The domain is not defined yet. The Allium specifications under `docs/specs/` will
define it; as they introduce effects, this crate will supply the real implementations
of the library's effect traits. No runtime, transport or HTTP stack has been chosen.

From the repository root, `just build` builds both crates. Run
`./target/debug/pachinko` to print the current name and library version.

Read the [API reference for main](https://biscuitbin.github.io/pachinko/pachinko/),
or run `just doc` and open `target/doc/pachinko/index.html`.

Requires Rust 1.99 or newer. Licensed under Apache-2.0; not published to crates.io.
