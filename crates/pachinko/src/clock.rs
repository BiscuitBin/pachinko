//! The real clock: libpachinko's `Clock` over the system time.

use std::time::{SystemTime, UNIX_EPOCH};

use libpachinko::{Clock, Timestamp};

/// The system time, as milliseconds since the UNIX epoch. A system time before the
/// epoch reads as the epoch.
///
/// ```
/// use libpachinko::{Clock, Timestamp};
/// use pachinko::SystemClock;
///
/// assert!(SystemClock::default().now() > Timestamp::from_millis(0));
/// ```
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Timestamp::from_millis(u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use libpachinko::{Clock, Timestamp};

    use super::SystemClock;

    #[test]
    fn the_system_clock_reads_after_2026() {
        assert!(SystemClock.now() > Timestamp::from_millis(1_767_225_600_000));
    }
}
