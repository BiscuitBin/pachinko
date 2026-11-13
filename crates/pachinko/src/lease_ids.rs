//! The real lease-id source: the library's [`LeaseIds`] over an epoch drawn once
//! from the operating system's random source and a serial (decisions 0008 and 0019).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use libpachinko::LeaseId;
use libpachinko::lease_ids::LeaseIds;

use crate::ServeError;

/// The serial value that marks the source exhausted. Issued serials start at 1, so
/// it is never one of them.
const EXHAUSTED: u64 = 0;

/// Lease ids whose high 64 bits are an epoch drawn once at start and whose low 64
/// bits are a serial from 1. Two services started apart draw different epochs, so
/// their ids differ even though both serials start at 1; within one service the
/// serial never repeats.
///
/// Clones share the serial through an [`Arc`], so no clone reissues an id. Once the
/// serial cannot advance the source is exhausted: [`EpochSerialLeaseIds::exhausted`]
/// says so, and the queue's owner stops rather than grant another lease.
///
/// ```
/// use libpachinko::lease_ids::LeaseIds;
///
/// let ids = pachinko::EpochSerialLeaseIds::new()?;
/// let (first, second) = (ids.next(), ids.next());
/// assert_ne!(first, second);
/// assert_eq!(first.as_u128() >> 64, second.as_u128() >> 64);
/// assert_eq!(first.to_string().len(), 32);
/// # Ok::<(), pachinko::ServeError>(())
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EpochSerialLeaseIds {
    epoch: u64,
    /// The serial the next id carries, or [`EXHAUSTED`].
    serial: Arc<AtomicU64>,
}

impl EpochSerialLeaseIds {
    /// Draws the epoch from the operating system's random source.
    ///
    /// # Errors
    ///
    /// [`ServeError::RandomSource`] when the random source fails; the service then
    /// refuses to start rather than fall back to a guessable epoch.
    ///
    /// ```
    /// let ids = pachinko::EpochSerialLeaseIds::new()?;
    /// assert!(!ids.exhausted());
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    pub fn new() -> Result<Self, ServeError> {
        Self::drawn(getrandom::u64(), 1)
    }

    /// Whether the serial can no longer advance. Once it is, the next id would
    /// repeat one already issued.
    ///
    /// ```
    /// let ids = pachinko::EpochSerialLeaseIds::new()?;
    /// assert!(!ids.exhausted());
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.serial.load(Ordering::Acquire) == EXHAUSTED
    }

    /// The source over the epoch `draw` gave, with the serial starting at `first`.
    fn drawn(draw: Result<u64, getrandom::Error>, first: u64) -> Result<Self, ServeError> {
        let epoch = draw.map_err(|error| ServeError::RandomSource(error.to_string()))?;
        Ok(Self {
            epoch,
            serial: Arc::new(AtomicU64::new(first)),
        })
    }

    /// A source whose serial starts at `first`, so a test reaches exhaustion in a
    /// step or two.
    #[cfg(test)]
    pub(crate) fn starting_at(first: u64) -> Self {
        Self {
            epoch: 0x00c0_ffee,
            serial: Arc::new(AtomicU64::new(first)),
        }
    }
}

impl LeaseIds for EpochSerialLeaseIds {
    /// The id for the current serial, advancing the serial with `checked_add`; a
    /// serial that cannot advance leaves the source exhausted. Called once
    /// exhausted, it returns the serial-0 id, which no issued id carries, and the
    /// owner never calls it then.
    fn next(&self) -> LeaseId {
        let serial = self
            .serial
            .update(Ordering::AcqRel, Ordering::Acquire, advance);
        LeaseId::from_u128((u128::from(self.epoch) << 64) | u128::from(serial))
    }
}

/// The serial after `serial`: one more, or [`EXHAUSTED`] when it cannot advance or
/// already has.
const fn advance(serial: u64) -> u64 {
    match serial.checked_add(1) {
        Some(next) if serial != EXHAUSTED => next,
        _ => EXHAUSTED,
    }
}

#[cfg(test)]
mod tests {
    use libpachinko::LeaseId;
    use libpachinko::lease_ids::LeaseIds;

    use super::EpochSerialLeaseIds;
    use crate::ServeError;

    #[test]
    fn two_ids_differ_and_share_the_epoch() {
        let ids = EpochSerialLeaseIds::new().unwrap();
        let shared = ids.clone();
        let (first, second) = (ids.next(), shared.next());
        assert_ne!(first, second);
        assert_eq!(first.as_u128() >> 64, second.as_u128() >> 64);
        assert_eq!(first.as_u128() & u128::from(u64::MAX), 1);
        assert_eq!(second.as_u128() & u128::from(u64::MAX), 2);
    }

    #[test]
    fn the_wire_form_is_32_hex_digits() {
        let id = EpochSerialLeaseIds::starting_at(7).next();
        let text = id.to_string();
        assert_eq!(text, "0000000000c0ffee0000000000000007");
        assert_eq!(text.parse::<LeaseId>(), Ok(id));
    }

    #[test]
    fn the_last_serial_is_issued_once_and_leaves_the_source_exhausted() {
        let ids = EpochSerialLeaseIds::starting_at(u64::MAX - 1);
        assert_eq!(
            ids.next().as_u128() & u128::from(u64::MAX),
            u128::from(u64::MAX - 1)
        );
        assert!(!ids.exhausted());
        assert_eq!(
            ids.next().as_u128() & u128::from(u64::MAX),
            u128::from(u64::MAX)
        );
        assert!(ids.exhausted());
        let after = ids.next();
        assert_eq!(after.as_u128() & u128::from(u64::MAX), 0);
        assert!(ids.exhausted(), "an exhausted source stays exhausted");
    }

    #[test]
    fn a_failed_random_source_refuses_to_start() {
        let refused = EpochSerialLeaseIds::drawn(Err(getrandom::Error::UNSUPPORTED), 1);
        assert!(
            matches!(refused, Err(ServeError::RandomSource(_))),
            "{refused:?}"
        );
    }
}
