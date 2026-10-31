//! The error enums: why a configuration, a join, a publish, a consumer's read or a
//! consumer's action was rejected.
//!
//! A rejection leaves the queue exactly as it was. A pull that finds no pocket is
//! an outcome, never an error, as the module's `PullFindsNoPocket` rule says.

use fencing::LeaseId;

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

/// The lease id, the fencing token every lease action carries.
///
/// A leaf module, so that the error enums can name it without depending on the
/// queue; [`crate::queue`] re-exports it.
pub(crate) mod fencing {
    use std::fmt;

    /// A lease's id: the fencing token.
    ///
    /// The queue takes each id from a counter when it grants a lease, so ids are
    /// monotonic and never reused. Every ack, nack, renew and release names one, and
    /// the queue rejects an id that is not active and the consumer's, so an action
    /// meant for an ended lease never lands on its successor.
    ///
    /// ```
    /// use libpachinko::queue::LeaseId;
    ///
    /// let id = LeaseId::from_raw(7);
    /// assert_eq!(id.as_raw(), 7);
    /// assert_eq!(id.to_string(), "7");
    /// assert!(LeaseId::from_raw(1) < id);
    /// ```
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[non_exhaustive]
    pub struct LeaseId(u64);

    impl LeaseId {
        /// The id with this number, as the wire carries it.
        ///
        /// ```
        /// use libpachinko::queue::LeaseId;
        ///
        /// assert_eq!(LeaseId::from_raw(3), LeaseId::from_raw(3));
        /// ```
        #[must_use]
        pub const fn from_raw(raw: u64) -> Self {
            Self(raw)
        }

        /// The id's number, as the wire carries it.
        ///
        /// ```
        /// use libpachinko::queue::LeaseId;
        ///
        /// assert_eq!(LeaseId::from_raw(9).as_raw(), 9);
        /// ```
        #[must_use]
        pub const fn as_raw(self) -> u64 {
            self.0
        }
    }

    impl fmt::Display for LeaseId {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}", self.0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::LeaseId;

        #[test]
        fn a_lease_id_round_trips_its_number_and_prints_it() {
            let id = LeaseId::from_raw(41);
            assert_eq!(id.as_raw(), 41);
            assert_eq!(id.to_string(), "41");
        }
    }
}
