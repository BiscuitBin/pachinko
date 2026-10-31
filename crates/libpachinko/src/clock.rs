//! The clock: the effect every `now` and every `expires_at` in the module reads.
//!
//! The library never reads a clock itself (decision 0008). A [`Queue`](crate::Queue)
//! is built with a [`Clock`]; the `pachinko` service supplies one over the system
//! time, and tests supply a [`FakeClock`] they move by hand.

use std::fmt;
use std::ops::Add;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// A point in time: a non-negative count of milliseconds since an epoch the clock
/// defines. The service's clock counts from the UNIX epoch.
///
/// Adding a [`Duration`] saturates at the largest timestamp rather than wrapping.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::Timestamp;
///
/// let t = Timestamp::from_millis(1_000);
/// assert_eq!(t + Duration::from_secs(2), Timestamp::from_millis(3_000));
/// assert!(t < t + Duration::from_millis(1));
/// assert_eq!(t.as_millis(), 1_000);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Timestamp(u64);

impl Timestamp {
    /// The timestamp `millis` milliseconds after the epoch.
    ///
    /// ```
    /// assert_eq!(libpachinko::Timestamp::from_millis(5).as_millis(), 5);
    /// ```
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// The milliseconds since the epoch.
    ///
    /// ```
    /// assert_eq!(libpachinko::Timestamp::from_millis(7).as_millis(), 7);
    /// ```
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        let millis = u64::try_from(rhs.as_millis()).unwrap_or(u64::MAX);
        Self(self.0.saturating_add(millis))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

/// The source of `now`. Every queue operation reads it once, at its start.
///
/// A clock may step back, as the system time does when it is corrected. The queue's
/// time does not: a reading earlier than one an operation has already seen counts as
/// that one, so no lease, superseded record or dead letter is stamped before an
/// earlier stamp.
///
/// ```
/// use libpachinko::{Clock, FakeClock, Timestamp};
///
/// let clock = FakeClock::new(Timestamp::from_millis(42));
/// assert_eq!(clock.now(), Timestamp::from_millis(42));
/// ```
pub trait Clock: Send + Sync {
    /// The current time.
    ///
    /// ```
    /// use libpachinko::{Clock, FakeClock, Timestamp};
    /// let clock = FakeClock::new(Timestamp::from_millis(7));
    /// assert_eq!(clock.now(), Timestamp::from_millis(7));
    /// ```
    fn now(&self) -> Timestamp;
}

/// A clock that stands still until a test moves it.
///
/// Clones share one time, so a test can keep a clone after handing the clock to a
/// [`Queue`](crate::Queue) and move the queue's time from outside.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::{Clock, FakeClock, Timestamp};
///
/// let clock = FakeClock::new(Timestamp::from_millis(0));
/// let shared = clock.clone();
/// shared.advance(Duration::from_millis(250));
/// assert_eq!(clock.now(), Timestamp::from_millis(250));
/// shared.set(Timestamp::from_millis(10));
/// assert_eq!(clock.now(), Timestamp::from_millis(10));
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FakeClock {
    millis: Arc<AtomicU64>,
}

impl FakeClock {
    /// A fake clock reading `start`.
    ///
    /// ```
    /// use libpachinko::{Clock, FakeClock, Timestamp};
    /// assert_eq!(FakeClock::new(Timestamp::from_millis(3)).now().as_millis(), 3);
    /// ```
    #[must_use]
    pub fn new(start: Timestamp) -> Self {
        Self {
            millis: Arc::new(AtomicU64::new(start.as_millis())),
        }
    }

    /// Moves every clone of this clock to `to`, forwards or backwards.
    ///
    /// ```
    /// use libpachinko::{Clock, FakeClock, Timestamp};
    /// let clock = FakeClock::default();
    /// clock.set(Timestamp::from_millis(9));
    /// assert_eq!(clock.now(), Timestamp::from_millis(9));
    /// ```
    pub fn set(&self, to: Timestamp) {
        self.millis.store(to.as_millis(), Ordering::SeqCst);
    }

    /// Moves every clone of this clock forward by `by`, saturating.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::{Clock, FakeClock, Timestamp};
    /// let clock = FakeClock::default();
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(clock.now(), Timestamp::from_millis(1_000));
    /// ```
    pub fn advance(&self, by: Duration) {
        self.set(self.now() + by);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.millis.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Clock, FakeClock, Timestamp};

    #[test]
    fn adding_a_duration_counts_whole_milliseconds() {
        let t = Timestamp::from_millis(10);
        assert_eq!(t + Duration::from_micros(1_999), Timestamp::from_millis(11));
        assert_eq!(t + Duration::ZERO, t);
    }

    #[test]
    fn adding_saturates_at_the_largest_timestamp() {
        let t = Timestamp::from_millis(u64::MAX - 1);
        assert_eq!(
            t + Duration::from_millis(5),
            Timestamp::from_millis(u64::MAX)
        );
        assert_eq!(
            Timestamp::from_millis(0) + Duration::MAX,
            Timestamp::from_millis(u64::MAX)
        );
    }

    #[test]
    fn timestamps_order_by_their_milliseconds() {
        assert!(Timestamp::from_millis(1) < Timestamp::from_millis(2));
        assert_eq!(Timestamp::default(), Timestamp::from_millis(0));
        assert_eq!(Timestamp::from_millis(12).to_string(), "12ms");
    }

    #[test]
    fn clones_of_a_fake_clock_share_one_time() {
        let clock = FakeClock::new(Timestamp::from_millis(100));
        let other = clock.clone();
        other.advance(Duration::from_millis(5));
        assert_eq!(clock.now(), Timestamp::from_millis(105));
        clock.set(Timestamp::from_millis(1));
        assert_eq!(other.now(), Timestamp::from_millis(1));
    }
}
