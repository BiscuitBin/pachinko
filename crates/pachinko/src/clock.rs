//! `SystemClock`, the real implementation of libpachinko's `Clock`.

use std::time::{SystemTime, UNIX_EPOCH};

use libpachinko::clock::{Clock, Timestamp};

/// The system's wall clock, in whole milliseconds since the Unix epoch. An instant
/// before the epoch reads as the epoch, and one past `u64::MAX` milliseconds as
/// that.
///
/// ```
/// use libpachinko::clock::{Clock, Timestamp};
/// use pachinko::clock::SystemClock;
///
/// // 2020-09-13T12:26:40Z, well before any machine this runs on.
/// assert!(SystemClock::default().now() > Timestamp::from_millis(1_600_000_000_000));
/// ```
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let millis = u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX);
        Timestamp::from_millis(millis)
    }
}

#[cfg(test)]
mod tests {
    use super::SystemClock;
    use libpachinko::clock::{Clock, Timestamp};

    #[test]
    fn the_system_clock_reads_milliseconds_since_the_epoch() {
        // 2020-09-13T12:26:40Z in milliseconds; a seconds reading would be far below.
        assert!(SystemClock.now() > Timestamp::from_millis(1_600_000_000_000));
    }
}
