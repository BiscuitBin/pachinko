//! `Timestamp`: the point in time every `now` and every `expires_at` in the
//! specification is.

use std::ops::Add;
use std::time::Duration;

/// A point in time: milliseconds since the Unix epoch.
///
/// Adding a [`Duration`] counts its whole milliseconds and saturates at the largest
/// timestamp rather than wrapping.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::Timestamp;
///
/// let t = Timestamp::from_millis(1_000);
/// assert_eq!(t + Duration::from_secs(2), Timestamp::from_millis(3_000));
/// assert!(t < t + Duration::from_millis(1));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Timestamp(u64);

impl Timestamp {
    /// The timestamp `millis` milliseconds after the Unix epoch.
    ///
    /// ```
    /// use libpachinko::Timestamp;
    /// assert_eq!(Timestamp::from_millis(5).as_millis(), 5);
    /// ```
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// The milliseconds since the Unix epoch.
    ///
    /// ```
    /// use libpachinko::Timestamp;
    /// assert_eq!(Timestamp::from_millis(7).as_millis(), 7);
    /// ```
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// The timestamp `millis` milliseconds later, saturating at the largest one.
    ///
    /// ```
    /// use libpachinko::Timestamp;
    /// assert_eq!(Timestamp::from_millis(1).saturating_add_millis(2), Timestamp::from_millis(3));
    /// assert_eq!(
    ///     Timestamp::from_millis(u64::MAX).saturating_add_millis(1),
    ///     Timestamp::from_millis(u64::MAX)
    /// );
    /// ```
    #[must_use]
    pub const fn saturating_add_millis(self, millis: u64) -> Self {
        Self(self.0.saturating_add(millis))
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    /// Adds the duration's whole milliseconds, saturating.
    fn add(self, rhs: Duration) -> Self {
        self.saturating_add_millis(u64::try_from(rhs.as_millis()).unwrap_or(u64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Timestamp;

    #[test]
    fn adding_a_duration_counts_whole_milliseconds() {
        let t = Timestamp::from_millis(10);
        assert_eq!(t + Duration::from_micros(1_999), Timestamp::from_millis(11));
        assert_eq!(t + Duration::ZERO, t);
    }

    #[test]
    fn adding_saturates_at_the_largest_timestamp() {
        let last = Timestamp::from_millis(u64::MAX);
        assert_eq!(
            Timestamp::from_millis(u64::MAX - 1) + Duration::from_millis(5),
            last
        );
        assert_eq!(Timestamp::from_millis(0) + Duration::MAX, last);
    }
}
