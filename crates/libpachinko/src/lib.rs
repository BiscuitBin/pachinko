//! The pachinko library: the domain `docs/specs/pachinko.allium` specifies, a
//! single-deployment queue of pockets.
//!
//! Publishers publish records into pockets; a pocket keeps one record per record key,
//! and a higher rank supersedes in place. A consumer leases one pocket at a time,
//! pulls its oldest records in bounded batches, and acks, nacks, renews or releases
//! under the lease its batch names. Leases lapse, pockets park after consecutive
//! expiries, and superseded records and dead letters decay. [`Queue`] holds it all.
//!
//! The crate performs no effect itself. The clock and the lease-id source, the two
//! effects the module needs, are the traits [`Clock`] and [`LeaseIds`], with the
//! fakes [`FakeClock`] and [`CountingLeaseIds`] beside them; the `pachinko` service
//! supplies the real implementations. Every operation is synchronous. See
//! `docs/explanation/architecture.md`.
//!
//! ```
//! use libpachinko::{Config, ConsumerId, CountingLeaseIds, FakeClock, Payload, PocketKey,
//!     Publication, PublisherId, PullOutcome, Queue, Rank, RecordKey, SettleOutcome};
//!
//! let mut queue = Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default());
//! queue.publisher_joins(PublisherId::new("p1")).unwrap();
//! queue.consumer_joins(ConsumerId::new("c1")).unwrap();
//! queue.publish(Publication::new(PublisherId::new("p1"), PocketKey::new("tenant-a"),
//!     RecordKey::new("k1"), Rank::new(1), Payload::from(b"hello".as_slice()))).unwrap();
//! let PullOutcome::Batch(batch) = queue.pull(&ConsumerId::new("c1"), 10).unwrap() else {
//!     panic!("a pocket is queued");
//! };
//! let acked = queue.ack(&ConsumerId::new("c1"), batch.lease_id, &RecordKey::new("k1"));
//! assert_eq!(acked, Ok(SettleOutcome::Settled));
//! ```

mod clock;
mod config;
mod error;
mod ids;
mod lease_ids;
mod queue;

pub use clock::{Clock, FakeClock, Timestamp};
pub use config::Config;
pub use error::Error;
pub use ids::{ConsumerId, Payload, PocketKey, Position, PublisherId, Rank, RecordKey};
pub use lease_ids::{CountingLeaseIds, LeaseId, LeaseIds, ParseLeaseIdError};
pub use queue::{
    Batch, DeadLetter, Decayed, HeldLease, HeldRecord, Lapsed, LeaseStatus, LeaseView, PocketFate,
    PocketStatus, PocketView, Publication, PublishOutcome, PullOutcome, Queue, RecordStatus,
    RecordView, SettleOutcome, ShedReason, Snapshot, SupersededRecord,
};

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
