//! The lease registry: the active leases by id and an index of them by expiry.
//!
//! An ended lease leaves both, so the registry holds exactly the active leases and
//! a tick reads the `by_expiry` prefix that is due, never the whole.
//! Two leases may share an `expires_at`; the index key carries the id beside it, so
//! each keeps its own entry.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Lease, LeaseStatus};
use crate::error::LeaseActionError;
use crate::ids::{ConsumerId, LeaseId, PocketKey};
use crate::timestamp::Timestamp;

/// One active lease.
#[derive(Clone, Debug)]
pub(super) struct Active {
    pub(super) pocket_key: PocketKey,
    pub(super) consumer_id: ConsumerId,
    pub(super) granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
}

/// What [`Leases::due`] found.
#[derive(Debug, Default)]
pub(super) struct Due {
    /// The due leases, earliest first.
    pub(super) ids: Vec<LeaseId>,
    /// The index entries the walk read, the one that stopped it included.
    pub(super) examined: usize,
}

/// The active leases by id, indexed by expiry.
#[derive(Clone, Debug, Default)]
pub(super) struct Leases {
    active: BTreeMap<LeaseId, Active>,
    by_expiry: BTreeSet<(Timestamp, LeaseId)>,
}

impl Leases {
    /// Whether `id` names an active lease.
    pub(super) fn is_active(&self, id: LeaseId) -> bool {
        self.active.contains_key(&id)
    }

    /// Registers a new active lease under `id`.
    pub(super) fn grant(&mut self, id: LeaseId, lease: Active) {
        self.by_expiry.insert((lease.expires_at, id));
        self.active.insert(id, lease);
    }

    /// The active lease `id`.
    pub(super) fn get(&self, id: LeaseId) -> Option<&Active> {
        self.active.get(&id)
    }

    /// Moves the active lease's expiry to `expires_at`, in both indexes; returns its
    /// pocket key.
    pub(super) fn extend(&mut self, id: LeaseId, expires_at: Timestamp) -> Option<PocketKey> {
        let lease = self.active.get_mut(&id)?;
        self.by_expiry.remove(&(lease.expires_at, id));
        self.by_expiry.insert((expires_at, id));
        lease.expires_at = expires_at;
        Some(lease.pocket_key.clone())
    }

    /// The fencing guards, in rule order: the lease is active
    /// ([`LeaseActionError::LeaseNotActive`], which an ended lease and one that
    /// never existed share), then it is `consumer_id`'s
    /// ([`LeaseActionError::LeaseNotYours`]). Returns its pocket key.
    pub(super) fn check(
        &self,
        id: LeaseId,
        consumer_id: &ConsumerId,
    ) -> Result<PocketKey, LeaseActionError> {
        let lease = self
            .active
            .get(&id)
            .ok_or(LeaseActionError::LeaseNotActive(id))?;
        if &lease.consumer_id != consumer_id {
            return Err(LeaseActionError::LeaseNotYours(id));
        }
        Ok(lease.pocket_key.clone())
    }

    /// Ends the active lease `id` with `status`, released by a release or expired
    /// by a lapse: it leaves both indexes and is returned as it ended, its
    /// `expires_at` untouched.
    pub(super) fn end(&mut self, id: LeaseId, status: LeaseStatus) -> Option<Lease> {
        let lease = self.active.remove(&id)?;
        self.by_expiry.remove(&(lease.expires_at, id));
        Some(Lease {
            id,
            pocket_key: lease.pocket_key,
            consumer_id: lease.consumer_id,
            status,
            granted_at: lease.granted_at,
            expires_at: lease.expires_at,
        })
    }

    /// The leases due at `now`: the `by_expiry` prefix with `expires_at <= now`,
    /// earliest first. The walk stops at the first later entry, so it reads the
    /// due entries and at most one more, never the whole index; `examined` counts
    /// what it read.
    pub(super) fn due(&self, now: Timestamp) -> Due {
        let mut due = Due::default();
        for &(expires_at, id) in &self.by_expiry {
            due.examined += 1;
            if expires_at > now {
                break;
            }
            due.ids.push(id);
        }
        due
    }

    /// The expiry index, earliest first.
    #[cfg(test)]
    pub(super) fn by_expiry(&self) -> Vec<(Timestamp, LeaseId)> {
        self.by_expiry.iter().copied().collect()
    }

    /// Every active lease, by id.
    #[cfg(feature = "testing")]
    pub(super) fn all(&self) -> impl Iterator<Item = (&LeaseId, &Active)> {
        self.active.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Active, Leases};
    use crate::error::LeaseActionError;
    use crate::ids::{ConsumerId, LeaseId, PocketKey};
    use crate::queue::types::LeaseStatus;
    use crate::timestamp::Timestamp;

    fn lease(pocket: &str, expires_at: u64) -> Active {
        Active {
            pocket_key: PocketKey::new(pocket),
            consumer_id: ConsumerId::new("c"),
            granted_at: Timestamp::from_millis(0),
            expires_at: Timestamp::from_millis(expires_at),
        }
    }

    #[test]
    fn two_leases_sharing_an_expiry_keep_their_own_entries() {
        let mut leases = Leases::default();
        let (one, two) = (LeaseId::from_u128(1), LeaseId::from_u128(2));
        leases.grant(one, lease("a", 10));
        leases.grant(two, lease("b", 10));
        assert_eq!(leases.by_expiry.len(), 2);
        assert_eq!(
            leases.extend(one, Timestamp::from_millis(20)),
            Some(PocketKey::new("a"))
        );
        let index: Vec<_> = leases.by_expiry.iter().copied().collect();
        assert_eq!(
            index,
            [
                (Timestamp::from_millis(10), two),
                (Timestamp::from_millis(20), one)
            ]
        );
        assert_eq!(leases.get(two).map(|l| l.pocket_key.as_str()), Some("b"));
        assert_eq!(
            leases.extend(LeaseId::from_u128(3), Timestamp::from_millis(1)),
            None
        );
    }

    #[test]
    fn check_reports_not_active_before_not_yours() {
        let mut leases = Leases::default();
        let (one, gone) = (LeaseId::from_u128(1), LeaseId::from_u128(2));
        leases.grant(one, lease("a", 10));
        let (mine, theirs) = (ConsumerId::new("c"), ConsumerId::new("d"));
        assert_eq!(leases.check(one, &mine), Ok(PocketKey::new("a")));
        assert_eq!(
            leases.check(one, &theirs),
            Err(LeaseActionError::LeaseNotYours(one))
        );
        assert_eq!(
            leases.check(gone, &theirs),
            Err(LeaseActionError::LeaseNotActive(gone))
        );
    }

    #[test]
    fn an_ended_lease_leaves_both_indexes_and_keeps_its_expiry() {
        let mut leases = Leases::default();
        let (one, two) = (LeaseId::from_u128(1), LeaseId::from_u128(2));
        leases.grant(one, lease("a", 10));
        leases.grant(two, lease("b", 10));
        let released = LeaseStatus::Released {
            ended_at: Timestamp::from_millis(4),
        };
        let ended = leases.end(one, released).unwrap();
        assert_eq!(ended.status, released);
        assert_eq!(ended.expires_at, Timestamp::from_millis(10));
        assert!(!leases.is_active(one));
        assert_eq!(leases.by_expiry(), [(Timestamp::from_millis(10), two)]);
        assert_eq!(leases.end(one, released), None);
        assert_eq!(
            leases.check(one, &ConsumerId::new("c")),
            Err(LeaseActionError::LeaseNotActive(one))
        );
    }

    #[test]
    fn due_reads_the_due_prefix_and_the_entry_that_stops_it() {
        let mut leases = Leases::default();
        let ids: Vec<_> = (1..=4).map(LeaseId::from_u128).collect();
        for (id, expires_at) in ids.iter().zip([10, 10, 20, 30]) {
            leases.grant(*id, lease("a", expires_at));
        }
        let due = leases.due(Timestamp::from_millis(9));
        assert_eq!((due.ids.len(), due.examined), (0, 1));
        let due = leases.due(Timestamp::from_millis(20));
        assert_eq!(due.ids, ids[..3]);
        assert_eq!(due.examined, 4);
        let due = leases.due(Timestamp::from_millis(30));
        assert_eq!((due.ids.len(), due.examined), (4, 4));
    }
}
