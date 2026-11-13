//! Why an operation was refused: one `thiserror` enum per operation, one variant per
//! refusal (invariant 10).

use crate::ids::{ConsumerId, LeaseId, PublisherId, Rank, RecordKey};

/// A refused join.
///
/// ```
/// use libpachinko::{JoinError, PublisherId};
///
/// let error = JoinError::PublisherExists(PublisherId::new("p1"));
/// assert_eq!(error.to_string(), "publisher p1 has already joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum JoinError {
    /// `PublisherJoins` requires that no publisher has the id.
    #[error("publisher {0} has already joined")]
    PublisherExists(PublisherId),
    /// `ConsumerJoins` requires that no consumer has the id.
    #[error("consumer {0} has already joined")]
    ConsumerExists(ConsumerId),
}

/// A refused publish. A refused publish changes nothing.
///
/// ```
/// use libpachinko::{PublishError, PublisherId};
///
/// let error = PublishError::UnknownPublisher(PublisherId::new("p1"));
/// assert_eq!(error.to_string(), "publisher p1 has not joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PublishError {
    /// The publish names a publisher that has not joined.
    #[error("publisher {0} has not joined")]
    UnknownPublisher(PublisherId),
    /// The rank equals the stored record's. No rule covers it, so the publish is
    /// refused: a placeholder for the specification's first open question, not a
    /// decision.
    ///
    /// ```
    /// use libpachinko::{PublishError, Rank, RecordKey};
    ///
    /// let error = PublishError::EqualRank { key: RecordKey::new("k"), rank: Rank::new(2) };
    /// assert!(error.to_string().contains("placeholder"));
    /// ```
    #[error(
        "rank {rank} equals the stored rank of {key}; no rule covers an equal rank \
        (open question 1), so this refusal is a placeholder"
    )]
    EqualRank {
        /// The record key published.
        key: RecordKey,
        /// The rank both the stored record and the publish carry.
        rank: Rank,
    },
}

/// A refused pull or lease read. Finding no pocket is not a refusal: that is
/// [`PullOutcome::NoPocketAvailable`](crate::PullOutcome::NoPocketAvailable).
///
/// ```
/// use libpachinko::{ConsumerId, PullError};
///
/// let error = PullError::UnknownConsumer(ConsumerId::new("c1"));
/// assert_eq!(error.to_string(), "consumer c1 has not joined");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PullError {
    /// The pull or lease read names a consumer that has not joined.
    #[error("consumer {0} has not joined")]
    UnknownConsumer(ConsumerId),
    /// A pull that claims a pocket found no lease id for it: the source is
    /// exhausted, or it returned an id that is still active. A pull within a lease
    /// and a pull that finds no pocket draw no id, so they never meet this.
    ///
    /// ```
    /// use libpachinko::{LeaseIdError, PullError};
    ///
    /// let error = PullError::from(LeaseIdError::Exhausted);
    /// assert_eq!(error, PullError::LeaseId(LeaseIdError::Exhausted));
    /// assert_eq!(
    ///     error.to_string(),
    ///     "no lease id for the claim: the lease-id source is exhausted: it has no id it has not given before"
    /// );
    /// ```
    #[error("no lease id for the claim: {0}")]
    LeaseId(#[from] LeaseIdError),
}

/// Why no lease id could be drawn for a claim: the refusal of a
/// [`LeaseIds`](crate::lease_ids::LeaseIds) source, or the queue's refusal of the id
/// it gave.
///
/// ```
/// use libpachinko::{LeaseId, LeaseIdError};
///
/// assert_eq!(
///     LeaseIdError::Exhausted.to_string(),
///     "the lease-id source is exhausted: it has no id it has not given before"
/// );
/// let repeated = LeaseIdError::StillActive(LeaseId::from_u128(1));
/// assert_eq!(
///     repeated.to_string(),
///     "the lease-id source returned 00000000000000000000000000000001, which is still active"
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LeaseIdError {
    /// The source's refusal: it cannot give an id it has not given before.
    #[error("the lease-id source is exhausted: it has no id it has not given before")]
    Exhausted,
    /// The queue's refusal: the source broke its contract and returned an id that is
    /// still active. The queue never draws again for the same claim.
    #[error("the lease-id source returned {0}, which is still active")]
    StillActive(LeaseId),
}

/// A refused ack, nack, renew or release. The guards are checked in this order,
/// the order of the rules' `requires`, and a refused action changes nothing, the
/// lease's `expires_at` included.
///
/// ```
/// use libpachinko::{LeaseActionError, LeaseId};
///
/// let error = LeaseActionError::LeaseNotActive(LeaseId::from_u128(1));
/// assert_eq!(error.to_string(), "lease 00000000000000000000000000000001 is not active");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LeaseActionError {
    /// The action names a consumer that has not joined; checked first.
    #[error("consumer {0} has not joined")]
    UnknownConsumer(ConsumerId),
    /// The lease is not in the active registry: it has ended, or never existed.
    /// An ended lease leaves the registry, so the two are one refusal.
    #[error("lease {0} is not active")]
    LeaseNotActive(LeaseId),
    /// The lease is active and another consumer's.
    #[error("lease {0} is not this consumer's")]
    LeaseNotYours(LeaseId),
    /// An ack or nack named a record key the lease's pocket does not hold.
    #[error("record {0} is not held under this lease")]
    RecordNotHeld(RecordKey),
}

/// A configuration the config invariants exclude.
///
/// ```
/// use libpachinko::ConfigError;
///
/// assert_eq!(
///     ConfigError::MaxPullBatchZero.to_string(),
///     "max_pull_batch must be at least 1 (PullBatchPositive)"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// `ExpiryLimitPositive`: `max_consecutive_expiries` is 0.
    #[error("max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)")]
    MaxConsecutiveExpiriesZero,
    /// `PullBatchPositive`: `max_pull_batch` is 0.
    #[error("max_pull_batch must be at least 1 (PullBatchPositive)")]
    MaxPullBatchZero,
}

#[cfg(test)]
mod tests {
    use super::{ConfigError, JoinError, LeaseActionError, LeaseIdError, PublishError, PullError};
    use crate::ids::{ConsumerId, LeaseId, PublisherId, Rank, RecordKey};

    #[test]
    fn every_variant_says_what_was_refused() {
        let cases = [
            (
                JoinError::ConsumerExists(ConsumerId::new("c")).to_string(),
                "consumer c has already joined",
            ),
            (
                PullError::UnknownConsumer(ConsumerId::new("c")).to_string(),
                "consumer c has not joined",
            ),
            (
                PullError::LeaseId(LeaseIdError::Exhausted).to_string(),
                "no lease id for the claim: the lease-id source is exhausted: \
                it has no id it has not given before",
            ),
            (
                PullError::LeaseId(LeaseIdError::StillActive(LeaseId::from_u128(0xab))).to_string(),
                "no lease id for the claim: the lease-id source returned \
                000000000000000000000000000000ab, which is still active",
            ),
            (
                JoinError::PublisherExists(PublisherId::new("p")).to_string(),
                "publisher p has already joined",
            ),
            (
                LeaseActionError::UnknownConsumer(ConsumerId::new("c")).to_string(),
                "consumer c has not joined",
            ),
            (
                LeaseActionError::LeaseNotYours(LeaseId::from_u128(0xab)).to_string(),
                "lease 000000000000000000000000000000ab is not this consumer's",
            ),
            (
                LeaseActionError::RecordNotHeld(RecordKey::new("k")).to_string(),
                "record k is not held under this lease",
            ),
            (
                ConfigError::MaxConsecutiveExpiriesZero.to_string(),
                "max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)",
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(text, expected);
        }
        let equal = PublishError::EqualRank {
            key: RecordKey::new("k"),
            rank: Rank::new(2),
        };
        assert!(
            equal
                .to_string()
                .starts_with("rank 2 equals the stored rank of k;")
        );
    }
}
