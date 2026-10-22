//! The pachinko library: the domain, once the Allium specifications under
//! `docs/specs/` define one.
//!
//! The domain is not defined yet, so this crate holds one item. When it grows, it
//! performs no effect itself: the clock, randomness, the filesystem, the network,
//! the environment and threads are reached through traits this crate defines, with
//! fakes beside them, and the `pachinko` service supplies the real implementations.
//! See `docs/explanation/architecture.md`.

/// The version of this crate, as its manifest states it.
///
/// ```
/// assert_eq!(libpachinko::version(), "0.1.0");
/// ```
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn the_version_is_the_manifests() {
        assert_eq!(version(), "0.1.0");
    }
}
