//! The clock: the effect every `now` and every `expires_at` in the specification reads.
//!
//! The library never reads a clock itself (decision 0008). A
//! [`Queue`](crate::Queue) is built with a [`Clock`]; the `pachinko` service
//! supplies one over the system time, and tests supply a [`FakeClock`] they move by
//! hand.

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::timestamp::Timestamp;

/// The source of `now`.
///
/// A clock may step back, as the system time does when it is corrected. The queue's
/// time does not: it keeps the latest reading it has seen, so no stamp it writes is
/// earlier than one it wrote before.
///
/// ```
/// use libpachinko::clock::{Clock, FakeClock};
/// use libpachinko::Timestamp;
///
/// let clock = FakeClock::new(Timestamp::from_millis(42));
/// assert_eq!(clock.now(), Timestamp::from_millis(42));
/// ```
pub trait Clock: Clone + Debug + Send + Sync + 'static {
    /// The current time.
    ///
    /// ```
    /// use libpachinko::clock::{Clock, FakeClock};
    /// use libpachinko::Timestamp;
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
/// use libpachinko::clock::{Clock, FakeClock};
/// use libpachinko::Timestamp;
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
    /// use libpachinko::clock::{Clock, FakeClock};
    /// use libpachinko::Timestamp;
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
    /// use libpachinko::clock::{Clock, FakeClock};
    /// use libpachinko::Timestamp;
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
    /// use libpachinko::clock::{Clock, FakeClock};
    /// use libpachinko::Timestamp;
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

    use super::{Clock, FakeClock};
    use crate::timestamp::Timestamp;

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
