//! The lease registry: the active leases by id and an index of them by expiry.
//!
//! An ended lease leaves both, so the registry holds exactly the active leases and
//! a lapse sweep (T26) reads the `by_expiry` prefix that is due, never the whole.
//! Two leases may share an `expires_at`; the index key carries the id beside it, so
//! each keeps its own entry.

use std::collections::{BTreeMap, BTreeSet};

use crate::clock::Timestamp;
use crate::ids::{ConsumerId, PocketKey};
use crate::lease_ids::LeaseId;

/// One active lease.
#[derive(Clone, Debug)]
pub(super) struct Active {
    pub(super) pocket_key: PocketKey,
    #[cfg_attr(
        not(feature = "testing"),
        expect(
            dead_code,
            reason = "read by the snapshot today; T25's fencing and T26's lapse report read it"
        )
    )]
    pub(super) consumer_id: ConsumerId,
    #[cfg_attr(
        not(feature = "testing"),
        expect(
            dead_code,
            reason = "read by the snapshot today; T25's fencing and T26's lapse report read it"
        )
    )]
    pub(super) granted_at: Timestamp,
    pub(super) expires_at: Timestamp,
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

    /// Every active lease, by id.
    #[cfg(feature = "testing")]
    pub(super) fn all(&self) -> impl Iterator<Item = (&LeaseId, &Active)> {
        self.active.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Active, Leases};
    use crate::clock::Timestamp;
    use crate::ids::{ConsumerId, PocketKey};
    use crate::lease_ids::LeaseId;

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
}
