//! The clock: the effect every `now` and every `expires_at` in the specification reads.
//!
//! The library never reads a clock itself (decision 0008). A
//! [`Queue`](crate::queue::Queue) is built with a [`Clock`]; the `pachinko` service
//! supplies one over the system time, and tests supply a [`FakeClock`] they move by
//! hand.

use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// A point in time: milliseconds since the Unix epoch.
///
/// Adding a [`Duration`] counts its whole milliseconds and saturates at the largest
/// timestamp rather than wrapping.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::clock::Timestamp;
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
    /// use libpachinko::clock::Timestamp;
    /// assert_eq!(Timestamp::from_millis(5).as_millis(), 5);
    /// ```
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// The milliseconds since the Unix epoch.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
    /// assert_eq!(Timestamp::from_millis(7).as_millis(), 7);
    /// ```
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// The timestamp `millis` milliseconds later, saturating at the largest one.
    ///
    /// ```
    /// use libpachinko::clock::Timestamp;
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

/// The source of `now`.
///
/// A clock may step back, as the system time does when it is corrected. The queue's
/// time does not: it keeps the latest reading it has seen, so no stamp it writes is
/// earlier than one it wrote before.
///
/// ```
/// use libpachinko::clock::{Clock, FakeClock, Timestamp};
///
/// let clock = FakeClock::new(Timestamp::from_millis(42));
/// assert_eq!(clock.now(), Timestamp::from_millis(42));
/// ```
pub trait Clock: Clone + Debug + Send + Sync + 'static {
    /// The current time.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    /// assert_eq!(FakeClock::new(Timestamp::from_millis(7)).now(), Timestamp::from_millis(7));
    /// ```
    fn now(&self) -> Timestamp;
}

/// A clock that stands still until a test moves it.
///
/// Clones share one instant through an [`Arc`], so a test keeps a clone after
/// handing the clock to a queue and moves the queue's time from outside. It reads
/// nothing from `std::time`.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::clock::{Clock, FakeClock, Timestamp};
///
/// let clock = FakeClock::new(Timestamp::from_millis(0));
/// let shared = clock.clone();
/// shared.advance(Duration::from_millis(250));
/// assert_eq!(clock.now(), Timestamp::from_millis(250));
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
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
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
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    /// let clock = FakeClock::default();
    /// clock.set(Timestamp::from_millis(9));
    /// assert_eq!(clock.now(), Timestamp::from_millis(9));
    /// ```
    pub fn set(&self, to: Timestamp) {
        self.millis.store(to.as_millis(), Ordering::SeqCst);
    }

    /// Moves every clone of this clock forward by `by`, saturating, in one atomic
    /// read-modify-write.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::clock::{Clock, FakeClock, Timestamp};
    /// let clock = FakeClock::default();
    /// clock.advance(Duration::from_secs(1));
    /// assert_eq!(clock.now(), Timestamp::from_millis(1_000));
    /// ```
    pub fn advance(&self, by: Duration) {
        let step = Timestamp::from_millis(0) + by;
        self.millis
            .update(Ordering::SeqCst, Ordering::SeqCst, |millis| {
                millis.saturating_add(step.as_millis())
            });
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
        let last = Timestamp::from_millis(u64::MAX);
        assert_eq!(
            Timestamp::from_millis(u64::MAX - 1) + Duration::from_millis(5),
            last
        );
        assert_eq!(Timestamp::from_millis(0) + Duration::MAX, last);
    }

    #[test]
    fn clones_of_a_fake_clock_share_one_instant() {
        let clock = FakeClock::new(Timestamp::from_millis(100));
        let other = clock.clone();
        other.advance(Duration::from_millis(5));
        assert_eq!(clock.now(), Timestamp::from_millis(105));
        clock.set(Timestamp::from_millis(1));
        assert_eq!(other.now(), Timestamp::from_millis(1));
    }

    #[test]
    fn advancing_saturates_at_the_largest_timestamp() {
        let clock = FakeClock::new(Timestamp::from_millis(u64::MAX - 1));
        clock.advance(Duration::MAX);
        assert_eq!(clock.now(), Timestamp::from_millis(u64::MAX));
    }
}
