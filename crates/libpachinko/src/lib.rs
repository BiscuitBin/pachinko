//! The pachinko library: the domain `docs/specs/pachinko.allium` defines, one queue
//! of pockets that publishers publish records into and consumers lease, pull and
//! settle.
//!
//! The library performs no effect itself (decision 0008): the clock is the
//! [`clock::Clock`] trait with [`clock::FakeClock`] beside it, and lease ids, which
//! need randomness, come from the [`lease_ids::LeaseIds`] trait with
//! [`lease_ids::CountingLeaseIds`] beside it. The `pachinko` service supplies the
//! real implementations.
//!
//! - [`clock`]: `Timestamp`, `Clock`, `FakeClock`.
//! - [`lease_ids`]: `LeaseId`, `LeaseIds`, `CountingLeaseIds`.
//! - [`ids`]: the names and values: publisher and consumer ids, pocket and record
//!   keys, rank, payload, position.
//! - [`config`]: the specification's `config` block with its defaults and load
//!   checks.
//! - [`error`]: one `thiserror` enum per operation.
//! - [`records`]: the store seam, the `PocketRecords` trait, its one implementation
//!   `MemoryRecords`, and, behind the `testing` feature, the contract every store
//!   passes.
//! - [`queue`]: `Queue`, with the publisher and consumer joins, publish, pull, the
//!   lease view, and the four lease actions, ack, nack, renew and release, each
//!   fenced to the lease it names; and, behind `testing`, the `Snapshot`
//!   inspection seam.
//!
//! See `docs/explanation/architecture.md`.

pub mod clock;
pub mod config;
pub mod error;
pub mod ids;
pub mod lease_ids;
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
