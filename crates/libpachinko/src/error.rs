//! Why a surface action was refused: one variant per `requires` an action can fail.

use crate::{ConsumerId, LeaseId, PocketKey, PublisherId, Rank, RecordKey};

/// A refused action. A pull is never refused for want of a pocket: that is
/// [`PullOutcome::NoPocketAvailable`](crate::PullOutcome::NoPocketAvailable).
///
/// ```
/// use libpachinko::{Error, PublisherId};
///
/// let error = Error::PublisherExists(PublisherId::new("p1"));
/// assert_eq!(error.to_string(), "publisher p1 already exists");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// `PublisherJoins` requires that no publisher has the id.
    #[error("publisher {0} already exists")]
    PublisherExists(PublisherId),
    /// `ConsumerJoins` requires that no consumer has the id.
    #[error("consumer {0} already exists")]
    ConsumerExists(ConsumerId),
    /// A publish names a publisher that has not joined.
    #[error("publisher {0} has not joined")]
    UnknownPublisher(PublisherId),
    /// A pull or a lease read names a consumer that has not joined.
    #[error("consumer {0} has not joined")]
    UnknownConsumer(ConsumerId),
    /// A publish whose rank equals the stored record's. No rule covers it, so the
    /// publish is rejected; that is the checker's placeholder for the module's
    /// first open question, not a decision.
    #[error(
        "rank {rank} equals the stored rank of {record_key} in {pocket_key}; no rule covers an \
            equal rank, which the module's first open question leaves undecided, so this \
            rejection is a placeholder"
    )]
    EqualRank {
        /// The pocket published into.
        pocket_key: PocketKey,
        /// The key published.
        record_key: RecordKey,
        /// The rank both the stored record and the publish carry.
        rank: Rank,
    },
    /// An ack, nack, renew or release names a lease that is not active: it has
    /// ended, or it never existed (`ActionsNameTheirLease`).
    #[error("lease {0} is not active")]
    LeaseNotActive(LeaseId),
    /// An ack, nack, renew or release names another consumer's lease.
    #[error("lease {0} is not this consumer's")]
    LeaseNotYours(LeaseId),
    /// An ack or nack names a key the lease does not hold.
    #[error("record {0} is not held under this lease")]
    RecordNotHeld(RecordKey),
    /// A configuration the config invariants exclude; the text names the invariant.
    #[error("invalid configuration: {0}")]
    InvalidConfig(&'static str),
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::{ConsumerId, LeaseId, PocketKey, PublisherId, Rank, RecordKey};

    #[test]
    fn every_variant_says_what_was_refused() {
        let lease = LeaseId::from_u128(1);
        let cases = [
            (
                Error::PublisherExists(PublisherId::new("p")),
                "publisher p already exists",
            ),
            (
                Error::ConsumerExists(ConsumerId::new("c")),
                "consumer c already exists",
            ),
            (
                Error::UnknownPublisher(PublisherId::new("p")),
                "publisher p has not joined",
            ),
            (
                Error::UnknownConsumer(ConsumerId::new("c")),
                "consumer c has not joined",
            ),
            (
                Error::LeaseNotActive(lease),
                "lease 00000000000000000000000000000001 is not active",
            ),
            (
                Error::LeaseNotYours(lease),
                "lease 00000000000000000000000000000001 is not this consumer's",
            ),
            (
                Error::RecordNotHeld(RecordKey::new("k")),
                "record k is not held under this lease",
            ),
            (Error::InvalidConfig("x"), "invalid configuration: x"),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
        }
    }

    #[test]
    fn the_equal_rank_rejection_names_the_open_question() {
        let error = Error::EqualRank {
            pocket_key: PocketKey::new("a"),
            record_key: RecordKey::new("k"),
            rank: Rank::new(2),
        };
        let text = error.to_string();
        assert!(
            text.starts_with("rank 2 equals the stored rank of k in a;"),
            "{text}"
        );
        assert!(text.contains("first open question"), "{text}");
        assert!(text.contains("placeholder"), "{text}");
    }
}
