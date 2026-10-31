//! Lease ids: the fencing token a batch names and every ack, nack, renew and
//! release carries back.
//!
//! Minting an unguessable id is randomness, an effect the library never performs
//! itself (decision 0008). A [`Queue`](crate::Queue) is built with a [`LeaseIds`]
//! source; the `pachinko` service supplies one over the operating system's random
//! source, and tests supply [`CountingLeaseIds`]. The queue never reissues an id it
//! has handed out: it draws again if the source repeats one.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of one lease, opaque to its holder.
///
/// It is written as 32 lowercase hexadecimal digits, which is how the wire carries
/// it, and parsed back from exactly that form.
///
/// ```
/// use libpachinko::LeaseId;
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
    /// assert_eq!(libpachinko::LeaseId::from_u128(7).as_u128(), 7);
    /// ```
    #[must_use]
    pub const fn from_u128(bits: u128) -> Self {
        Self(bits)
    }

    /// The id's 128 bits.
    ///
    /// ```
    /// assert_eq!(libpachinko::LeaseId::from_u128(9).as_u128(), 9);
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
/// use libpachinko::{LeaseId, ParseLeaseIdError};
/// assert_eq!(
///     "xyz".parse::<LeaseId>(),
///     Err(ParseLeaseIdError::NotThirtyTwoHexDigits)
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseLeaseIdError {
    /// The text is not exactly 32 hexadecimal digits.
    #[error("a lease id is exactly 32 hexadecimal digits")]
    NotThirtyTwoHexDigits,
}

impl FromStr for LeaseId {
    type Err = ParseLeaseIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let well_formed = text.len() == 32 && text.bytes().all(|b| b.is_ascii_hexdigit());
        well_formed
            .then(|| u128::from_str_radix(text, 16).ok())
            .flatten()
            .map(Self)
            .ok_or(ParseLeaseIdError::NotThirtyTwoHexDigits)
    }
}

/// The source of lease ids. Every claim of a pocket takes one.
///
/// ```
/// use libpachinko::{CountingLeaseIds, LeaseId, LeaseIds};
///
/// let ids = CountingLeaseIds::default();
/// assert_eq!(ids.next(), LeaseId::from_u128(1));
/// ```
pub trait LeaseIds: Send + Sync {
    /// A lease id. A source should not repeat one; the queue draws again if it does.
    ///
    /// ```
    /// use libpachinko::{CountingLeaseIds, LeaseIds};
    /// let ids = CountingLeaseIds::default();
    /// assert_ne!(ids.next(), ids.next());
    /// ```
    fn next(&self) -> LeaseId;
}

/// The fake lease-id source: 1, 2, 3 and so on. Clones share one counter.
///
/// ```
/// use libpachinko::{CountingLeaseIds, LeaseId, LeaseIds};
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
    fn a_lease_id_is_written_as_32_hex_digits() {
        assert_eq!(LeaseId::from_u128(0).to_string(), "0".repeat(32));
        assert_eq!(LeaseId::from_u128(u128::MAX).to_string(), "f".repeat(32));
    }

    #[test]
    fn parsing_accepts_exactly_the_written_form_and_either_case() {
        let id = LeaseId::from_u128(0x0123_4567_89ab_cdef);
        assert_eq!(id.to_string().parse(), Ok(id));
        assert_eq!(id.to_string().to_uppercase().parse(), Ok(id));
        for bad in [
            "",
            "1",
            &"0".repeat(31),
            &"0".repeat(33),
            &"g".repeat(32),
            &"+".repeat(32),
        ] {
            assert_eq!(
                bad.parse::<LeaseId>(),
                Err(ParseLeaseIdError::NotThirtyTwoHexDigits),
                "{bad:?}"
            );
        }
        assert_eq!(
            ParseLeaseIdError::NotThirtyTwoHexDigits.to_string(),
            "a lease id is exactly 32 hexadecimal digits"
        );
    }

    #[test]
    fn the_counting_source_hands_out_one_two_three() {
        let ids = CountingLeaseIds::default();
        let got: Vec<u128> = (0..3).map(|_| ids.next().as_u128()).collect();
        assert_eq!(got, [1, 2, 3]);
    }
}
