//! Lease ids: the fencing token a batch names and every ack, nack, renew and
//! release carries back.
//!
//! Drawing an unguessable epoch is randomness, an effect the library never performs
//! itself (decision 0008). A [`Queue`](crate::Queue) is built with a
//! [`LeaseIds`] source; the `pachinko` service supplies one whose epoch comes from
//! the operating system's random source once at start (decision 0019), and tests
//! supply [`CountingLeaseIds`].

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ids::LeaseId;

/// The source of lease ids. Every claim of a pocket takes one.
///
/// A source never repeats an id over its life: the id fences an ack, nack, renew or
/// release to the lease it was pulled under, so a repeat would let an action meant
/// for an ended lease land on a new one.
///
/// ```
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
/// use libpachinko::LeaseId;
///
/// let ids = CountingLeaseIds::default();
/// assert_eq!(ids.next(), LeaseId::from_u128(1));
/// ```
pub trait LeaseIds: Clone + Debug + Send + Sync + 'static {
    /// The next lease id, never one this source has returned before.
    ///
    /// ```
    /// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
    /// let ids = CountingLeaseIds::default();
    /// assert_ne!(ids.next(), ids.next());
    /// ```
    fn next(&self) -> LeaseId;
}

/// The fake lease-id source: epoch 0 and a serial from 1, so its ids are 1, 2, 3
/// and so on. Clones share one counter through an [`Arc`].
///
/// ```
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
/// use libpachinko::LeaseId;
///
/// let ids = CountingLeaseIds::default();
/// let shared = ids.clone();
/// assert_eq!(ids.next(), LeaseId::from_u128(1));
/// assert_eq!(shared.next(), LeaseId::from_u128(2));
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CountingLeaseIds {
    issued: Arc<AtomicU64>,
}

impl LeaseIds for CountingLeaseIds {
    fn next(&self) -> LeaseId {
        let previous = self.issued.fetch_add(1, Ordering::SeqCst);
        LeaseId::from_u128(u128::from(previous) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{CountingLeaseIds, LeaseIds};

    #[test]
    fn the_counting_source_hands_out_one_two_three() {
        let ids = CountingLeaseIds::default();
        let got: Vec<u128> = (0..3).map(|_| ids.next().as_u128()).collect();
        assert_eq!(got, [1, 2, 3]);
    }
}
