//! The clock, the first effect trait (decision 0003), with its fake beside it.
//!
//! The queue never reads the time itself: every rule that stamps a timestamp, and
//! every lapse and decay, asks the [`Clock`] it was built with. The service supplies
//! a clock over the system time; tests supply a [`FakeClock`] and move it by hand.

use std::fmt::Debug;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// A point in time, in whole milliseconds since the Unix epoch.
///
/// Milliseconds are the module's finest grain: a TTL is added at that resolution,
/// and a sub-millisecond part of a [`Duration`] is dropped. Arithmetic saturates at
/// the largest representable instant rather than wrapping.
///
/// ```
/// use libpachinko::clock::Timestamp;
/// use std::time::Duration;
///
/// let start = Timestamp::from_millis(1_000);
/// assert_eq!(start.saturating_add(Duration::from_secs(2)).as_millis(), 3_000);
/// assert!(start < Timestamp::from_millis(1_001));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Timestamp(u64);

impl Timestamp {
    /// The instant `millis` milliseconds after the Unix epoch.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
    ///
    /// assert_eq!(Timestamp::from_millis(42).as_millis(), 42);
    /// ```
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Milliseconds since the Unix epoch.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
    ///
    /// assert_eq!(Timestamp::default().as_millis(), 0);
    /// ```
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// This instant moved `duration` later, in whole milliseconds, saturating.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
    /// use std::time::Duration;
    ///
    /// let late = Timestamp::from_millis(u64::MAX - 1);
    /// assert_eq!(late.saturating_add(Duration::from_secs(1)).as_millis(), u64::MAX);
    /// assert_eq!(
    ///     Timestamp::from_millis(5).saturating_add(Duration::from_micros(1_999)),
    ///     Timestamp::from_millis(6)
    /// );
    /// ```
    #[must_use]
    pub fn saturating_add(self, duration: Duration) -> Self {
        let millis = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        Self(self.0.saturating_add(millis))
    }
}

/// Where the queue reads the time.
///
/// ```
/// use libpachinko::clock::{Clock, FakeClock, Timestamp};
///
/// fn stamp(clock: &impl Clock) -> Timestamp {
///     clock.now()
/// }
/// assert_eq!(stamp(&FakeClock::new(Timestamp::from_millis(7))).as_millis(), 7);
/// ```
pub trait Clock: Clone + Debug + Send + Sync + 'static {
    /// The current instant.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    ///
    /// assert_eq!(FakeClock::new(Timestamp::from_millis(5)).now(), Timestamp::from_millis(5));
    /// ```
    fn now(&self) -> Timestamp;
}

/// A clock that stands still until a test moves it.
///
/// Clones share one instant, so a test keeps a clone and moves the clock the
/// queue owns.
///
/// ```
/// use libpachinko::clock::{Clock, FakeClock, Timestamp};
/// use std::time::Duration;
///
/// let clock = FakeClock::new(Timestamp::from_millis(100));
/// let handle = clock.clone();
/// handle.advance(Duration::from_millis(50));
/// assert_eq!(clock.now(), Timestamp::from_millis(150));
/// handle.set(Timestamp::from_millis(10));
/// assert_eq!(clock.now(), Timestamp::from_millis(10));
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FakeClock {
    now: Arc<Mutex<Timestamp>>,
}

impl FakeClock {
    /// A clock that reads `start` until it is moved.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    ///
    /// assert_eq!(FakeClock::new(Timestamp::from_millis(3)).now().as_millis(), 3);
    /// ```
    #[must_use]
    pub fn new(start: Timestamp) -> Self {
        Self {
            now: Arc::new(Mutex::new(start)),
        }
    }

    /// Moves the clock to `instant`, earlier or later.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    ///
    /// let clock = FakeClock::default();
    /// clock.set(Timestamp::from_millis(9));
    /// assert_eq!(clock.now().as_millis(), 9);
    /// ```
    pub fn set(&self, instant: Timestamp) {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner) = instant;
    }

    /// Moves the clock `duration` later, saturating.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    /// use std::time::Duration;
    ///
    /// let clock = FakeClock::default();
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(clock.now().as_millis(), 1_000);
    /// ```
    pub fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.saturating_add(duration);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, FakeClock, Timestamp};
    use std::time::Duration;

    #[test]
    fn a_duration_beyond_u64_millis_saturates() {
        let huge = Duration::from_secs(u64::MAX);
        assert_eq!(
            Timestamp::from_millis(1).saturating_add(huge),
            Timestamp::from_millis(u64::MAX)
        );
    }

    #[test]
    fn the_fake_moves_back_as_well_as_forward() {
        let clock = FakeClock::new(Timestamp::from_millis(50));
        clock.set(Timestamp::from_millis(20));
        assert_eq!(clock.now().as_millis(), 20);
    }

    #[test]
    fn clones_of_the_fake_share_one_instant() {
        let clock = FakeClock::new(Timestamp::from_millis(1));
        let other = clock.clone();
        other.advance(Duration::from_millis(1));
        assert_eq!(clock.now(), Timestamp::from_millis(2));
    }
}
