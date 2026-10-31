//! The pachinko library: the queue `docs/specs/pachinko.allium` specifies.
//!
//! One deployment is one [`Queue`](queue::Queue) of pockets. Publishers publish
//! records into pockets; a pocket keeps one record per record key and a higher rank
//! supersedes in place; a consumer leases one pocket at a time, pulls its oldest
//! records in bounded batches, and acks, nacks, renews or releases. Every rule of
//! the module has a method or a step here, and the module wins wherever the two
//! disagree.
//!
//! The crate performs no effect itself. The queue reads the time through the
//! [`Clock`](clock::Clock) trait, with [`FakeClock`](clock::FakeClock) beside it for
//! tests, and keeps each pocket's records behind the
//! [`PocketRecords`](records::PocketRecords) trait, with
//! [`MemoryRecords`](records::MemoryRecords) as its reference. The `pachinko` service
//! supplies the system clock and an Arrow record store. See
//! `docs/explanation/architecture.md`.
//!
//! The `testing` feature adds the records contract and the queue's inspection
//! seam, which this crate's tests and the service's use.

pub mod clock;
pub mod config;
pub mod error;
mod fencing;
pub mod queue;
pub mod records;

/// The version of this crate, as its manifest states it.
///
/// ```
/// assert_eq!(libpachinko::version(), env!("CARGO_PKG_VERSION"));
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
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
