//! The module's `config` block: five values a deployment sets, with the module's
//! defaults, checked once at load against the five config invariants.

use std::time::Duration;

use crate::Error;

/// The deployment's configuration.
///
/// [`Config::default`] is the module's defaults. [`Config::new`] checks what the
/// config invariants exclude: `ExpiryLimitPositive` and `PullBatchPositive`. The
/// three TTL invariants (`LeaseTtlNonNegative`, `SupersededTtlNonNegative`,
/// `DeadLetterTtlNonNegative`) hold by type, since a [`Duration`] is never negative.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::Config;
///
/// let config = Config::default();
/// assert_eq!(config.lease_ttl(), Duration::from_secs(5 * 60));
/// assert_eq!(config.max_consecutive_expiries(), 3);
/// assert_eq!(config.max_pull_batch(), 100);
/// assert_eq!(config.superseded_ttl(), Duration::from_secs(12 * 60 * 60));
/// assert_eq!(config.dead_letter_ttl(), Duration::from_secs(12 * 60 * 60));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    lease_ttl: Duration,
    max_consecutive_expiries: u32,
    max_pull_batch: u32,
    superseded_ttl: Duration,
    dead_letter_ttl: Duration,
}

/// The module's default `lease_ttl`: five minutes.
const DEFAULT_LEASE_TTL: Duration = Duration::from_mins(5);
/// The module's default `max_consecutive_expiries`.
const DEFAULT_MAX_CONSECUTIVE_EXPIRIES: u32 = 3;
/// The module's default `max_pull_batch`, a placeholder nobody has chosen.
const DEFAULT_MAX_PULL_BATCH: u32 = 100;
/// The module's default `superseded_ttl` and `dead_letter_ttl`: twelve hours.
const DEFAULT_DECAY_TTL: Duration = Duration::from_hours(12);

impl Config {
    /// A configuration with the given values.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] when `max_consecutive_expiries` is 0
    /// (`ExpiryLimitPositive`) or `max_pull_batch` is 0 (`PullBatchPositive`).
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::{Config, Error};
    ///
    /// let ttl = Duration::from_secs(1);
    /// assert!(Config::new(ttl, 1, 1, ttl, ttl).is_ok());
    /// assert!(matches!(Config::new(ttl, 0, 1, ttl, ttl), Err(Error::InvalidConfig(_))));
    /// assert!(matches!(Config::new(ttl, 1, 0, ttl, ttl), Err(Error::InvalidConfig(_))));
    /// ```
    pub const fn new(
        lease_ttl: Duration,
        max_consecutive_expiries: u32,
        max_pull_batch: u32,
        superseded_ttl: Duration,
        dead_letter_ttl: Duration,
    ) -> Result<Self, Error> {
        if max_consecutive_expiries == 0 {
            return Err(Error::InvalidConfig(
                "max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)",
            ));
        }
        if max_pull_batch == 0 {
            return Err(Error::InvalidConfig(
                "max_pull_batch must be at least 1 (PullBatchPositive)",
            ));
        }
        Ok(Self {
            lease_ttl,
            max_consecutive_expiries,
            max_pull_batch,
            superseded_ttl,
            dead_letter_ttl,
        })
    }

    /// How long a lease lasts after its claim or its consumer's last pull, ack,
    /// nack or renewal.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::Config;
    /// let ttl = Duration::from_secs(30);
    /// let config = Config::new(ttl, 1, 1, Duration::ZERO, Duration::ZERO).unwrap();
    /// assert_eq!(config.lease_ttl(), ttl);
    /// ```
    #[must_use]
    pub const fn lease_ttl(&self) -> Duration {
        self.lease_ttl
    }

    /// How many consecutive lease expiries park a pocket.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::Config;
    /// let config = Config::new(Duration::ZERO, 4, 1, Duration::ZERO, Duration::ZERO).unwrap();
    /// assert_eq!(config.max_consecutive_expiries(), 4);
    /// ```
    #[must_use]
    pub const fn max_consecutive_expiries(&self) -> u32 {
        self.max_consecutive_expiries
    }

    /// The most records one pull hands out.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::Config;
    /// let config = Config::new(Duration::ZERO, 1, 25, Duration::ZERO, Duration::ZERO).unwrap();
    /// assert_eq!(config.max_pull_batch(), 25);
    /// ```
    #[must_use]
    pub const fn max_pull_batch(&self) -> u32 {
        self.max_pull_batch
    }

    /// How long a superseded record is kept before it decays.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::Config;
    /// let ttl = Duration::from_secs(60);
    /// let config = Config::new(Duration::ZERO, 1, 1, ttl, Duration::ZERO).unwrap();
    /// assert_eq!(config.superseded_ttl(), ttl);
    /// ```
    #[must_use]
    pub const fn superseded_ttl(&self) -> Duration {
        self.superseded_ttl
    }

    /// How long a dead letter is kept before it decays.
    ///
    /// ```
    /// use std::time::Duration;
    /// use libpachinko::Config;
    /// let ttl = Duration::from_secs(90);
    /// let config = Config::new(Duration::ZERO, 1, 1, Duration::ZERO, ttl).unwrap();
    /// assert_eq!(config.dead_letter_ttl(), ttl);
    /// ```
    #[must_use]
    pub const fn dead_letter_ttl(&self) -> Duration {
        self.dead_letter_ttl
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lease_ttl: DEFAULT_LEASE_TTL,
            max_consecutive_expiries: DEFAULT_MAX_CONSECUTIVE_EXPIRIES,
            max_pull_batch: DEFAULT_MAX_PULL_BATCH,
            superseded_ttl: DEFAULT_DECAY_TTL,
            dead_letter_ttl: DEFAULT_DECAY_TTL,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Config;
    use crate::Error;

    const SECOND: Duration = Duration::from_secs(1);

    // config-default.lease_ttl, config-default.max_consecutive_expiries,
    // config-default.max_pull_batch, config-default.superseded_ttl and
    // config-default.dead_letter_ttl: each value is the module's default.
    #[test]
    fn the_defaults_are_the_modules() {
        let config = Config::default();
        assert_eq!(config.lease_ttl(), Duration::from_mins(5));
        assert_eq!(config.max_consecutive_expiries(), 3);
        assert_eq!(config.max_pull_batch(), 100);
        assert_eq!(config.superseded_ttl(), Duration::from_hours(12));
        assert_eq!(config.dead_letter_ttl(), Duration::from_hours(12));
    }

    // invariant.ExpiryLimitPositive and invariant.PullBatchPositive: checked at load.
    #[test]
    fn a_zero_limit_is_rejected_at_load() {
        assert_eq!(
            Config::new(SECOND, 0, 1, SECOND, SECOND),
            Err(Error::InvalidConfig(
                "max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)"
            ))
        );
        assert_eq!(
            Config::new(SECOND, 1, 0, SECOND, SECOND),
            Err(Error::InvalidConfig(
                "max_pull_batch must be at least 1 (PullBatchPositive)"
            ))
        );
    }

    // invariant.LeaseTtlNonNegative, invariant.SupersededTtlNonNegative and
    // invariant.DeadLetterTtlNonNegative:
    // a Duration cannot be negative, and zero is admitted.
    #[test]
    fn zero_ttls_are_admitted() {
        let config = Config::new(Duration::ZERO, 1, 1, Duration::ZERO, Duration::ZERO).unwrap();
        assert_eq!(config.lease_ttl(), Duration::ZERO);
        assert_eq!(config.superseded_ttl(), Duration::ZERO);
        assert_eq!(config.dead_letter_ttl(), Duration::ZERO);
        assert_eq!(config.max_consecutive_expiries(), 1);
        assert_eq!(config.max_pull_batch(), 1);
    }
}
