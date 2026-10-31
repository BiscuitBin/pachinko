//! The publishers and consumers that have joined, and each consumer's active lease.

use std::collections::{BTreeMap, BTreeSet};

use super::LeaseId;
use crate::error::{ConsumerError, JoinError, PublishError};

/// The publishers that have joined.
#[derive(Clone, Debug, Default)]
pub(super) struct Publishers {
    ids: BTreeSet<String>,
}

impl Publishers {
    /// `PublisherJoins`: refuses an id that has joined.
    pub(super) fn join(&mut self, id: &str) -> Result<(), JoinError> {
        if self.ids.contains(id) {
            return Err(JoinError::AlreadyJoined(id.to_owned()));
        }
        self.ids.insert(id.to_owned());
        Ok(())
    }

    /// The publisher must have joined.
    pub(super) fn require(&self, id: &str) -> Result<(), PublishError> {
        if self.ids.contains(id) {
            return Ok(());
        }
        Err(PublishError::UnknownPublisher(id.to_owned()))
    }

    /// Every publisher id, in order.
    #[cfg(feature = "testing")]
    pub(super) fn snapshot(&self) -> Vec<String> {
        self.ids.iter().cloned().collect()
    }
}

/// The consumers that have joined, each mapped to its active lease, if any, which
/// is how `OneActiveLeasePerConsumer` is kept by construction.
#[derive(Clone, Debug, Default)]
pub(super) struct Consumers {
    leases: BTreeMap<String, Option<LeaseId>>,
}

impl Consumers {
    /// `ConsumerJoins`: refuses an id that has joined.
    pub(super) fn join(&mut self, id: &str) -> Result<(), JoinError> {
        if self.leases.contains_key(id) {
            return Err(JoinError::AlreadyJoined(id.to_owned()));
        }
        self.leases.insert(id.to_owned(), None);
        Ok(())
    }

    /// The consumer's active lease; the consumer must have joined.
    pub(super) fn active_lease(&self, id: &str) -> Result<Option<LeaseId>, ConsumerError> {
        self.leases
            .get(id)
            .copied()
            .ok_or_else(|| ConsumerError::UnknownConsumer(id.to_owned()))
    }

    /// Records the consumer's new active lease, or its end.
    pub(super) fn set_lease(&mut self, id: &str, lease: Option<LeaseId>) {
        self.leases
            .get_mut(id)
            .into_iter()
            .for_each(|slot| *slot = lease);
    }

    /// Every consumer id with its active lease, in order.
    #[cfg(feature = "testing")]
    pub(super) fn snapshot(&self) -> Vec<(String, Option<LeaseId>)> {
        self.leases
            .iter()
            .map(|(id, lease)| (id.clone(), *lease))
            .collect()
    }
}
