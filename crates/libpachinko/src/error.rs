//! The error enums: why a configuration, a join, a publish, a consumer's read or a
//! consumer's action was rejected.
//!
//! A rejection leaves the queue exactly as it was. A pull that finds no pocket is
//! an outcome, never an error, as the module's `PullFindsNoPocket` rule says.

use crate::fencing::LeaseId;

/// A [`Config`](crate::config::Config) outside the module's config invariants.
///
/// ```
/// use libpachinko::error::ConfigError;
///
/// assert_eq!(
///     ConfigError::PullBatchNotPositive.to_string(),
///     "max_pull_batch must be at least 1"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// `max_consecutive_expiries` is 0 (`ExpiryLimitPositive`).
    #[error("max_consecutive_expiries must be at least 1")]
    ExpiryLimitNotPositive,
    /// `max_pull_batch` is 0 (`PullBatchPositive`).
    #[error("max_pull_batch must be at least 1")]
    PullBatchNotPositive,
}

/// A join the `PublisherJoins` or `ConsumerJoins` rule refuses.
///
/// ```
/// use libpachinko::error::JoinError;
///
/// let error = JoinError::AlreadyJoined("p1".to_owned());
/// assert_eq!(error.to_string(), "p1 has already joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum JoinError {
    /// A party with this id has already joined.
    #[error("{0} has already joined")]
    AlreadyJoined(String),
}

/// A publish no publish rule accepts.
///
/// ```
/// use libpachinko::error::PublishError;
///
/// let error = PublishError::UnknownPublisher("p9".to_owned());
/// assert_eq!(error.to_string(), "no publisher p9 has joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PublishError {
    /// No publisher with this id has joined.
    #[error("no publisher {0} has joined")]
    UnknownPublisher(String),
    /// The rank equals the stored rank. No rule covers that publish, so it is
    /// rejected: the module's placeholder, not a decision (its first open question).
    #[error("rank {rank} equals the stored rank of {record_key} in {pocket_key}")]
    EqualRank {
        /// The pocket published into.
        pocket_key: String,
        /// The record key published.
        record_key: String,
        /// The rank both versions carry.
        rank: i64,
    },
}

/// A consumer the queue does not know.
///
/// ```
/// use libpachinko::error::ConsumerError;
///
/// let error = ConsumerError::UnknownConsumer("c9".to_owned());
/// assert_eq!(error.to_string(), "no consumer c9 has joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConsumerError {
    /// No consumer with this id has joined.
    #[error("no consumer {0} has joined")]
    UnknownConsumer(String),
}

/// An ack, nack, renew or release the rules reject.
///
/// ```
/// use libpachinko::error::ActionError;
/// use libpachinko::queue::LeaseId;
///
/// let error = ActionError::LeaseNotActive(LeaseId::from_raw(4));
/// assert_eq!(error.to_string(), "lease 4 is not active");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ActionError {
    /// No consumer with this id has joined.
    #[error("no consumer {0} has joined")]
    UnknownConsumer(String),
    /// The lease has ended, or was never granted: `lease.status = active` fails.
    #[error("lease {0} is not active")]
    LeaseNotActive(LeaseId),
    /// The lease is active but another consumer's: `lease.consumer = consumer` fails.
    #[error("lease {0} belongs to another consumer")]
    LeaseNotYours(LeaseId),
    /// The pocket has no record with this key, or the record is not held.
    #[error("record {0} is not held under this lease")]
    RecordNotHeld(String),
}

impl From<ConsumerError> for ActionError {
    fn from(error: ConsumerError) -> Self {
        match error {
            ConsumerError::UnknownConsumer(id) => Self::UnknownConsumer(id),
        }
    }
}
