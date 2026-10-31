//! The active leases, their expiry index and the fencing-token counter.

use std::collections::{BTreeMap, BTreeSet};

use super::{Lease, LeaseId, LeaseStatus};
use crate::clock::Timestamp;
use crate::error::ActionError;

/// One active lease.
#[derive(Clone, Debug)]
struct Active {
    pocket_key: String,
    consumer_id: String,
    granted_at: Timestamp,
    expires_at: Timestamp,
}

/// The active leases by id, indexed by expiry so a tick finds the lapsed ones
/// without a scan. An ended lease leaves this registry: it is terminal, so nothing
/// reads it again, and the id counter is never reset, so its id is never reused.
#[derive(Clone, Debug)]
pub(super) struct Leases {
    active: BTreeMap<LeaseId, Active>,
    by_expiry: BTreeSet<(Timestamp, LeaseId)>,
    next_id: u64,
}

impl Default for Leases {
    fn default() -> Self {
        Self {
            active: BTreeMap::new(),
            by_expiry: BTreeSet::new(),
            next_id: 1,
        }
    }
}

/// When a lease is granted and when it lapses.
#[derive(Clone, Copy, Debug)]
pub(super) struct Term {
    pub(super) granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
}

impl Leases {
    /// Grants a lease on `pocket_key` to `consumer_id` and returns its new id.
    ///
    /// The counter stops at exhaustion rather than wrapping, in release builds as in
    /// debug ones, so an id is never issued twice.
    #[expect(
        clippy::expect_used,
        reason = "2^64 grants cannot occur; wrapping would reissue a fencing token"
    )]
    pub(super) fn grant(&mut self, pocket_key: &str, consumer_id: &str, term: Term) -> LeaseId {
        let id = LeaseId::from_raw(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("lease id counter exhausted: ids are never reused");
        let lease = Active {
            pocket_key: pocket_key.to_owned(),
            consumer_id: consumer_id.to_owned(),
            granted_at: term.granted_at,
            expires_at: term.expires_at,
        };
        self.active.insert(id, lease);
        self.by_expiry.insert((term.expires_at, id));
        id
    }

    /// The lease must be active and the consumer's; returns its pocket key.
    pub(super) fn check(&self, id: LeaseId, consumer_id: &str) -> Result<String, ActionError> {
        let lease = self
            .active
            .get(&id)
            .ok_or(ActionError::LeaseNotActive(id))?;
        if lease.consumer_id != consumer_id {
            return Err(ActionError::LeaseNotYours(id));
        }
        Ok(lease.pocket_key.clone())
    }

    /// The active lease's pocket key and expiry.
    pub(super) fn pocket_of(&self, id: LeaseId) -> Option<(String, Timestamp)> {
        self.active
            .get(&id)
            .map(|lease| (lease.pocket_key.clone(), lease.expires_at))
    }

    /// Moves the lease's expiry to `expires_at`; returns its pocket key.
    pub(super) fn extend(&mut self, id: LeaseId, expires_at: Timestamp) -> Option<String> {
        let lease = self.active.get_mut(&id)?;
        self.by_expiry.remove(&(lease.expires_at, id));
        self.by_expiry.insert((expires_at, id));
        lease.expires_at = expires_at;
        Some(lease.pocket_key.clone())
    }

    /// Ends the lease with `status` at `now` and returns it as it ended.
    pub(super) fn end(
        &mut self,
        id: LeaseId,
        status: LeaseStatus,
        now: Timestamp,
    ) -> Option<Lease> {
        let lease = self.active.remove(&id)?;
        self.by_expiry.remove(&(lease.expires_at, id));
        Some(Lease {
            id,
            pocket_key: lease.pocket_key,
            consumer_id: lease.consumer_id,
            status,
            granted_at: lease.granted_at,
            expires_at: lease.expires_at,
            ended_at: Some(now),
        })
    }

    /// The active leases whose expiry is at or before `now`, earliest first.
    pub(super) fn due(&self, now: Timestamp) -> Vec<LeaseId> {
        self.by_expiry
            .iter()
            .take_while(|(expires_at, _)| *expires_at <= now)
            .map(|(_, id)| *id)
            .collect()
    }

    /// Every active lease, by id.
    #[cfg(feature = "testing")]
    pub(super) fn snapshot(&self) -> Vec<Lease> {
        self.active
            .iter()
            .map(|(id, lease)| Lease {
                id: *id,
                pocket_key: lease.pocket_key.clone(),
                consumer_id: lease.consumer_id.clone(),
                status: LeaseStatus::Active,
                granted_at: lease.granted_at,
                expires_at: lease.expires_at,
                ended_at: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{LeaseId, Leases, Term};
    use crate::clock::Timestamp;

    const TERM: Term = Term {
        granted_at: Timestamp::from_millis(0),
        expires_at: Timestamp::from_millis(1_000),
    };

    #[test]
    fn the_last_lease_id_is_issued_once() {
        let mut leases = Leases {
            next_id: u64::MAX - 1,
            ..Leases::default()
        };
        assert_eq!(
            leases.grant("pocket", "consumer", TERM),
            LeaseId::from_raw(u64::MAX - 1)
        );
    }

    #[test]
    #[should_panic(expected = "lease id counter exhausted")]
    fn an_exhausted_lease_id_counter_stops_instead_of_wrapping() {
        let mut leases = Leases {
            next_id: u64::MAX - 1,
            ..Leases::default()
        };
        leases.grant("pocket", "consumer", TERM);
        leases.grant("pocket", "consumer", TERM);
    }
}
