//! The pachinko library: the domain `docs/specs/pachinko.allium` defines, one queue
//! of pockets that publishers publish records into and consumers lease, pull and
//! settle.
//!
//! The crate has three readers, and each has its own surface:
//!
//! | Reader | Surface | What it holds |
//! | --- | --- | --- |
//! | The caller, who drives a queue | the crate root | [`Queue`], [`Config`], and every value an operation takes, returns or refuses with |
//! | The seam author, who supplies an effect or a store | [`clock`], [`lease_ids`], [`records`] | one trait per seam, with its fake or its one implementation beside it |
//! | The test author, who checks the queue from outside | `testing`, behind the `testing` feature | the inspection seam, the tick's cost counts and the records contract |
//!
//! # The caller
//!
//! One `use libpachinko::{…}` names everything a caller touches. A [`Queue`] is the
//! specification's `Queue` entity with its publishers, consumers, pockets, records,
//! leases and shed queue, and one method per trigger:
//!
//! | Surface | Call | Takes | `Ok` | Refusals, in guard order |
//! | --- | --- | --- | --- | --- |
//! | build | [`Queue::new`] | a [`Config`], a clock, a lease-id source | the queue | [`ConfigError`]: `MaxConsecutiveExpiriesZero`, `MaxPullBatchZero` |
//! | join | [`Queue::publisher_joins`] | a [`PublisherId`] | `()` | [`JoinError`]: `PublisherExists` |
//! | join | [`Queue::consumer_joins`] | a [`ConsumerId`] | `()` | [`JoinError`]: `ConsumerExists` |
//! | `Publishing` | [`Queue::publish`] | the publisher and a [`Publication`] | [`PublishOutcome`]: `Created`, `NewKey`, `Superseded`, `Stale` | [`PublishError`]: `UnknownPublisher`, `EqualRank` |
//! | `Consuming` | [`Queue::pull`] | the consumer and a limit | [`PullOutcome`]: `Batch { lease, pocket_key, expires_at, records }`, `NoPocketAvailable` | [`PullError`]: `UnknownConsumer`, `LeaseId` ([`LeaseIdError`]: `Exhausted`, `StillActive`) |
//! | `Consuming` | [`Queue::lease_view`] | the consumer | `Some` [`LeaseView`] or `None` | [`PullError`]: `UnknownConsumer` |
//! | `Consuming` | [`Queue::ack`] | the consumer, its [`LeaseId`] and a [`RecordKey`] | [`SettleOutcome`]: `Settled`, `Deferred` | [`LeaseActionError`]: `UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours`, `RecordNotHeld` |
//! | `Consuming` | [`Queue::nack`] | the consumer, its [`LeaseId`] and a [`RecordKey`] | [`SettleOutcome`]: `Settled`, `Deferred` | [`LeaseActionError`]: `UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours`, `RecordNotHeld` |
//! | `Consuming` | [`Queue::renew`] | the consumer and its [`LeaseId`] | the new `expires_at`, a [`Timestamp`] | [`LeaseActionError`]: `UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours` |
//! | `Consuming` | [`Queue::release`] | the consumer and its [`LeaseId`] | a [`PocketFate`], `Queued` or `Idle`, and the ended [`Lease`] | [`LeaseActionError`]: `UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours` |
//! | timeout | [`Queue::tick`] | nothing | a [`TickReport`]: `lapsed`, `decayed_superseded`, `decayed_dead_letters` | none |
//!
//! A refusal changes nothing, and each is one variant of a `thiserror` enum, one
//! enum per operation (invariant 10). `NoPocketAvailable` is an outcome, not an
//! error. `EqualRank` is a placeholder for the specification's open question 1, not
//! a decision.
//!
//! The names and values are the specification's: who publishes and consumes
//! ([`PublisherId`], [`ConsumerId`]), which pocket and key a record has
//! ([`PocketKey`], [`RecordKey`]), its [`Rank`] and [`Payload`], and a held record
//! as a batch carries it ([`HeldRecord`]). [`Config`] is the specification's
//! `config` block: five values a deployment sets, with the specification's
//! defaults, checked once at load against the config invariants.
//!
//! `Queue` is plain data with `&mut self` methods, and it has one owner. Whoever
//! owns it applies one operation at a time, so a lapse, which [`Queue::tick`]
//! processes, and a consumer's action meet in one stream, as decision 0019
//! requires: a tick runs between two actions, never inside one. A release processed
//! first ends the lease, and a renewal, ack, nack or pull processed first moves the
//! lease's `expires_at` to `now + lease_ttl`, so the lapse stops at its guards
//! unless that new `expires_at` has passed too, as it always has when `lease_ttl`
//! is zero. A lapse processed first refuses the action.
//!
//! ```
//! use libpachinko::clock::FakeClock;
//! use libpachinko::lease_ids::CountingLeaseIds;
//! use libpachinko::records::MemoryRecords;
//! use libpachinko::{
//!     Config, ConsumerId, Payload, PocketKey, Publication, PublishOutcome, PublisherId,
//!     PullOutcome, Queue, Rank, RecordKey, SettleOutcome,
//! };
//!
//! let mut queue: Queue<MemoryRecords, _, _> =
//!     Queue::new(Config::default(), FakeClock::default(), CountingLeaseIds::default()).unwrap();
//! let (publisher, consumer) = (PublisherId::new("p"), ConsumerId::new("c"));
//! queue.publisher_joins(publisher.clone()).unwrap();
//! queue.consumer_joins(consumer.clone()).unwrap();
//! let publication =
//!     Publication::new(PocketKey::new("orders"), RecordKey::new("o-1"), Rank::new(1), Payload::default());
//! assert_eq!(queue.publish(&publisher, publication), Ok(PublishOutcome::Created));
//! let Ok(PullOutcome::Batch { lease, records, .. }) = queue.pull(&consumer, 10) else { panic!() };
//! assert_eq!(queue.ack(&consumer, lease, &records[0].key), Ok(SettleOutcome::Settled));
//! assert!(queue.tick().is_empty());
//! ```
//!
//! # The seam author
//!
//! The library performs no effect itself (decision 0008). A queue reads the time
//! only through its [`clock::Clock`] and draws lease ids, which need randomness,
//! only from its [`lease_ids::LeaseIds`]; [`clock::FakeClock`] and
//! [`lease_ids::CountingLeaseIds`] are the fakes beside them, and the `pachinko`
//! service supplies the real implementations. A lease-id source is fallible: once
//! it has no id it has not given before it answers [`LeaseIdError::Exhausted`],
//! and a pull that claims a pocket draws once and is refused with
//! [`PullError::LeaseId`] rather than draw again; a pull within a lease and a pull
//! that finds no pocket never draw. [`records::PocketRecords`] is the
//! store seam, the record-level steps the specification's rules take, with
//! [`records::MemoryRecords`] its one implementation (decision 0019).
//!
//! # The test author
//!
//! The `testing` feature adds one module, `testing`, and nothing it adds changes
//! what the other two surfaces hold: the inspection seam (`Queue::snapshot` and
//! the `Snapshot` it returns), the tick's cost counts (`Queue::tick_with_cost` and
//! `TickCost`), and the records contract (`check_contract`, over a store that also
//! implements `InspectRecords`). The pocket's status, the superseded records and
//! the dead letters are values only that module exports: no surface exposes them,
//! and who reads the shed queue is the specification's second open question.
//!
//! See `docs/explanation/architecture.md`.

mod config;
mod error;
mod ids;
mod queue;
mod timestamp;

pub mod clock;
pub mod lease_ids;
pub mod records;
#[cfg(feature = "testing")]
pub mod testing;

// Each re-export is written from `crate::`, never relative: rustqual counts a module's
// importers by `crate::` paths alone, so `pub use queue::…` would hide this file's edge
// to `queue` and `just metrics` would report `queue` over the instability cap.
pub use crate::config::Config;
pub use crate::error::{
    ConfigError, JoinError, LeaseActionError, LeaseIdError, PublishError, PullError,
};
pub use crate::ids::{
    ConsumerId, HeldRecord, LeaseId, ParseLeaseIdError, Payload, PocketKey, PublisherId, Rank,
    RecordKey,
};
pub use crate::queue::{
    Lease, LeaseStatus, LeaseView, PocketFate, Publication, PublishOutcome, PullOutcome, Queue,
    SettleOutcome, TickReport,
};
pub use crate::timestamp::Timestamp;

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
