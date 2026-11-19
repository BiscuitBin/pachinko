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

use crate::error::LeaseIdError;
use crate::ids::LeaseId;

/// The source of lease ids. Every claim of a pocket takes one.
///
/// A source never repeats an id over its life: the id fences an ack, nack, renew or
/// release to the lease it was pulled under, so a repeat would let an action meant
/// for an ended lease land on a new one. Once it cannot honour that, it returns
/// [`LeaseIdError::Exhausted`] on every later call.
///
/// ```
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
/// use libpachinko::LeaseId;
///
/// let ids = CountingLeaseIds::default();
/// assert_eq!(ids.next(), Ok(LeaseId::from_u128(1)));
/// ```
pub trait LeaseIds: Clone + Debug + Send + Sync + 'static {
    /// The next lease id, never one this source has returned before.
    ///
    /// # Errors
    ///
    /// [`LeaseIdError::Exhausted`] once the source has no id it has not given
    /// before, and on every call after that.
    ///
    /// ```
    /// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
    /// let ids = CountingLeaseIds::default();
    /// assert_ne!(ids.next()?, ids.next()?);
    /// # Ok::<(), libpachinko::LeaseIdError>(())
    /// ```
    fn next(&self) -> Result<LeaseId, LeaseIdError>;
}

/// The fake lease-id source: epoch 0 and a serial from 1, so its ids are 1, 2, 3
/// and so on, up to `u64::MAX` once, then [`LeaseIdError::Exhausted`]. Clones share
/// one counter through an [`Arc`].
///
/// ```
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseIds};
/// use libpachinko::LeaseId;
///
/// let ids = CountingLeaseIds::default();
/// let shared = ids.clone();
/// assert_eq!(ids.next(), Ok(LeaseId::from_u128(1)));
/// assert_eq!(shared.next(), Ok(LeaseId::from_u128(2)));
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CountingLeaseIds {
    /// How many ids the source has issued, which is also the last id issued.
    issued: Arc<AtomicU64>,
}

impl LeaseIds for CountingLeaseIds {
    /// One more than the last id issued, advancing the count with `checked_add`:
    /// a count that cannot advance has issued `u64::MAX`, and stays exhausted.
    fn next(&self) -> Result<LeaseId, LeaseIdError> {
        let previous = self
            .issued
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |issued| {
                issued.checked_add(1)
            })
            .map_err(|_| LeaseIdError::Exhausted)?;
        Ok(LeaseId::from_u128(u128::from(previous) + 1))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    use super::{CountingLeaseIds, LeaseIds};
    use crate::error::LeaseIdError;
    use crate::ids::LeaseId;

    #[test]
    fn the_counting_source_hands_out_one_two_three() {
        let ids = CountingLeaseIds::default();
        let got: Vec<u128> = (0..3).map(|_| ids.next().unwrap().as_u128()).collect();
        assert_eq!(got, [1, 2, 3]);
    }

    #[test]
    fn the_counting_source_issues_its_last_id_once_then_stays_exhausted() {
        let ids = CountingLeaseIds {
            issued: Arc::new(AtomicU64::new(u64::MAX - 1)),
        };
        assert_eq!(ids.next(), Ok(LeaseId::from_u128(u128::from(u64::MAX))));
        assert_eq!(ids.next(), Err(LeaseIdError::Exhausted));
        assert_eq!(ids.next(), Err(LeaseIdError::Exhausted));
    }
}
