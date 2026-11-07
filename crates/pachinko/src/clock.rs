//! The real clock: the library's [`Clock`] over the system time (decisions 0008 and
//! 0019).

use std::time::{SystemTime, UNIX_EPOCH};

use libpachinko::clock::{Clock, Timestamp};

use crate::ServeError;

/// The system time, in milliseconds since the Unix epoch.
///
/// The system time can step back when it is corrected; the queue keeps the latest
/// reading it has seen, so its own time never does. [`SystemClock::new`] refuses a
/// clock that reads before the epoch; a later reading before the epoch is
/// [`Timestamp`] zero, which the queue's clamp then ignores.
///
/// ```
/// use libpachinko::clock::Clock;
///
/// let clock = pachinko::SystemClock::new()?;
/// assert!(clock.now().as_millis() > 0);
/// # Ok::<(), pachinko::ServeError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SystemClock;

impl SystemClock {
    /// Reads the system time once and returns the clock.
    ///
    /// # Errors
    ///
    /// [`ServeError::ClockBeforeEpoch`] when the system time is before the Unix
    /// epoch.
    ///
    /// ```
    /// assert!(pachinko::SystemClock::new().is_ok());
    /// ```
    pub fn new() -> Result<Self, ServeError> {
        Self::reading(SystemTime::now())
    }

    /// The clock, given that it reads `time` now.
    fn reading(time: SystemTime) -> Result<Self, ServeError> {
        since_epoch(time)
            .map(|_| Self)
            .ok_or(ServeError::ClockBeforeEpoch)
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        since_epoch(SystemTime::now()).unwrap_or_default()
    }
}

/// `time` in milliseconds since the Unix epoch, or `None` before it. A time past
/// `u64::MAX` milliseconds saturates.
fn since_epoch(time: SystemTime) -> Option<Timestamp> {
    let elapsed = time.duration_since(UNIX_EPOCH).ok()?;
    Some(Timestamp::from_millis(
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
    ))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use libpachinko::clock::{Clock, Timestamp};

    use super::{SystemClock, since_epoch};

    #[test]
    fn now_is_after_the_epoch() {
        let clock = SystemClock::new().unwrap();
        assert!(clock.now() > Timestamp::from_millis(0));
        assert!(clock.now() > Timestamp::from_millis(0));
    }

    #[test]
    fn a_time_before_the_epoch_has_no_timestamp() {
        let before = UNIX_EPOCH.checked_sub(Duration::from_secs(1)).unwrap();
        assert_eq!(since_epoch(before), None);
        assert_eq!(
            SystemClock::reading(before),
            Err(crate::ServeError::ClockBeforeEpoch)
        );
        assert_eq!(since_epoch(UNIX_EPOCH), Some(Timestamp::from_millis(0)));
        assert_eq!(
            since_epoch(UNIX_EPOCH + Duration::from_millis(1_762_500_000_123)),
            Some(Timestamp::from_millis(1_762_500_000_123))
        );
    }
}
