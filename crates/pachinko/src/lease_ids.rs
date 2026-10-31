//! The real lease-id source: libpachinko's `LeaseIds` over the operating system's
//! random source, so that a lease id is unguessable (decision 0019).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use libpachinko::{LeaseId, LeaseIds};

/// 128 random bits per lease, from the operating system.
///
/// If the operating system's source fails, which it does not on the platforms the
/// service runs on, the id is drawn from a counter instead and the failure is
/// logged: the queue never reissues an id, so the lease stays safe, but it is then
/// guessable.
///
/// ```
/// use libpachinko::LeaseIds;
/// use pachinko::RandomLeaseIds;
///
/// let ids = RandomLeaseIds::default();
/// assert_ne!(ids.next(), ids.next());
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct RandomLeaseIds {
    fallback: Arc<AtomicU64>,
}

impl LeaseIds for RandomLeaseIds {
    fn next(&self) -> LeaseId {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_or_else(
            |error| {
                tracing::error!(%error, "the random source failed; counting lease ids instead");
                LeaseId::from_u128(u128::from(self.fallback.fetch_add(1, Ordering::SeqCst)))
            },
            |()| LeaseId::from_u128(u128::from_le_bytes(bytes)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use libpachinko::LeaseIds;

    use super::RandomLeaseIds;

    #[test]
    fn random_ids_do_not_repeat() {
        let ids = RandomLeaseIds::default();
        let drawn: BTreeSet<u128> = (0..1_000).map(|_| ids.next().as_u128()).collect();
        assert_eq!(drawn.len(), 1_000);
    }
}
