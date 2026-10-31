//! The lease id, the fencing token every lease action carries.
//!
//! A leaf module, so that the error enums can name it without depending on the
//! queue; [`crate::queue`] re-exports it.

use std::fmt;

/// A lease's id: the fencing token.
///
/// The queue takes each id from a counter when it grants a lease, so ids are
/// monotonic and never reused. Every ack, nack, renew and release names one, and
/// the queue rejects an id that is not active and the consumer's, so an action
/// meant for an ended lease never lands on its successor.
///
/// ```
/// use libpachinko::queue::LeaseId;
///
/// let id = LeaseId::from_raw(7);
/// assert_eq!(id.as_raw(), 7);
/// assert_eq!(id.to_string(), "7");
/// assert!(LeaseId::from_raw(1) < id);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct LeaseId(u64);

impl LeaseId {
    /// The id with this number, as the wire carries it.
    ///
    /// ```
    /// use libpachinko::queue::LeaseId;
    ///
    /// assert_eq!(LeaseId::from_raw(3), LeaseId::from_raw(3));
    /// ```
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The id's number, as the wire carries it.
    ///
    /// ```
    /// use libpachinko::queue::LeaseId;
    ///
    /// assert_eq!(LeaseId::from_raw(9).as_raw(), 9);
    /// ```
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for LeaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::LeaseId;

    #[test]
    fn a_lease_id_round_trips_its_number_and_prints_it() {
        let id = LeaseId::from_raw(41);
        assert_eq!(id.as_raw(), 41);
        assert_eq!(id.to_string(), "41");
    }
}
