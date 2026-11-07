//! Lease ids: the fencing token a batch names and every ack, nack, renew and
//! release carries back.
//!
//! Drawing an unguessable epoch is randomness, an effect the library never performs
//! itself (decision 0008). A [`Queue`](crate::queue::Queue) is built with a
//! [`LeaseIds`] source; the `pachinko` service supplies one whose epoch comes from
//! the operating system's random source once at start (decision 0019), and tests
//! supply [`CountingLeaseIds`].

use std::fmt::{self, Debug};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of one lease, opaque to its holder: a `u128` whose high 64 bits are
/// the source's epoch and whose low 64 bits are its serial.
///
/// It is written as exactly 32 lower-case hexadecimal digits, which is how the wire
/// carries it, and parsed back from exactly that form: a shorter or longer text, an
/// upper-case digit or a sign is refused.
///
/// ```
/// use libpachinko::lease_ids::LeaseId;
///
/// let id = LeaseId::from_u128(0xab);
/// assert_eq!(id.to_string(), "000000000000000000000000000000ab");
/// assert_eq!(id.to_string().parse::<LeaseId>(), Ok(id));
/// assert!("ab".parse::<LeaseId>().is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct LeaseId(u128);

impl LeaseId {
    /// The lease id with the given 128 bits.
    ///
    /// ```
    /// use libpachinko::lease_ids::LeaseId;
    /// assert_eq!(LeaseId::from_u128(7).as_u128(), 7);
    /// ```
    #[must_use]
    pub const fn from_u128(bits: u128) -> Self {
        Self(bits)
    }

    /// The id's 128 bits.
    ///
    /// ```
    /// use libpachinko::lease_ids::LeaseId;
    /// assert_eq!(LeaseId::from_u128(9).as_u128(), 9);
    /// ```
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

impl fmt::Display for LeaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Why a text is not a lease id.
///
/// ```
/// use libpachinko::lease_ids::{LeaseId, ParseLeaseIdError};
/// assert_eq!("xyz".parse::<LeaseId>(), Err(ParseLeaseIdError::NotThirtyTwoHexDigits));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseLeaseIdError {
    /// The text is not exactly 32 lower-case hexadecimal digits.
    #[error("a lease id is exactly 32 lower-case hexadecimal digits")]
    NotThirtyTwoHexDigits,
}

impl FromStr for LeaseId {
    type Err = ParseLeaseIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let well_formed =
            text.len() == 32 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        well_formed
            .then(|| u128::from_str_radix(text, 16).ok())
            .flatten()
            .map(Self)
            .ok_or(ParseLeaseIdError::NotThirtyTwoHexDigits)
    }
}

/// The source of lease ids. Every claim of a pocket takes one.
///
/// A source never repeats an id over its life: the id fences an ack, nack, renew or
/// release to the lease it was pulled under, so a repeat would let an action meant
/// for an ended lease land on a new one.
///
/// ```
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseId, LeaseIds};
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
/// use libpachinko::lease_ids::{CountingLeaseIds, LeaseId, LeaseIds};
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
        LeaseId(u128::from(previous) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{CountingLeaseIds, LeaseId, LeaseIds, ParseLeaseIdError};

    #[test]
    fn display_and_parse_round_trip_the_edges() {
        for bits in [
            0,
            1,
            u128::from(u64::MAX),
            u128::from(u64::MAX) + 1,
            u128::MAX,
        ] {
            let id = LeaseId::from_u128(bits);
            assert_eq!(id.to_string().len(), 32);
            assert_eq!(id.to_string().parse(), Ok(id));
        }
        assert_eq!(LeaseId::from_u128(u128::MAX).to_string(), "f".repeat(32));
    }

    #[test]
    fn parsing_refuses_every_other_form() {
        let upper = "A".repeat(32);
        let plus = format!("+{}", "0".repeat(31));
        let short = "0".repeat(31);
        let long = "0".repeat(33);
        let not_hex = "g".repeat(32);
        for bad in ["", "1", &short, &long, &not_hex, &upper, &plus] {
            assert_eq!(
                bad.parse::<LeaseId>(),
                Err(ParseLeaseIdError::NotThirtyTwoHexDigits),
                "{bad:?}"
            );
        }
        assert_eq!(
            ParseLeaseIdError::NotThirtyTwoHexDigits.to_string(),
            "a lease id is exactly 32 lower-case hexadecimal digits"
        );
    }

    #[test]
    fn the_counting_source_hands_out_one_two_three() {
        let ids = CountingLeaseIds::default();
        let got: Vec<u128> = (0..3).map(|_| ids.next().as_u128()).collect();
        assert_eq!(got, [1, 2, 3]);
    }
}
