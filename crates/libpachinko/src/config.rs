//! The module's `config` block: the five figures a deployment sets.

use std::time::Duration;

use crate::error::ConfigError;

/// The queue's configuration, the `config` block of `docs/specs/pachinko.allium`.
///
/// The defaults are the module's: `lease_ttl` 5 minutes, `max_consecutive_expiries`
/// 3, `max_pull_batch` 100, `superseded_ttl` 12 hours and `dead_letter_ttl` 12
/// hours. The fields are public, so a caller starts from the default and changes
/// what it needs; [`Config::validate`] checks the result.
///
/// ```
/// use libpachinko::config::Config;
/// use std::time::Duration;
///
/// let mut config = Config::default();
/// assert_eq!(config.lease_ttl, Duration::from_secs(300));
/// config.max_pull_batch = 10;
/// assert!(config.validate().is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// How long a lease lasts after it is granted or last extended.
    pub lease_ttl: Duration,
    /// The consecutive lapses after which a pocket parks; at least 1.
    pub max_consecutive_expiries: u32,
    /// The most records one pull hands out; at least 1.
    pub max_pull_batch: u32,
    /// How long a superseded or stale payload stays before it decays.
    pub superseded_ttl: Duration,
    /// How long a dead letter stays before it decays.
    pub dead_letter_ttl: Duration,
}

/// The module's `lease_ttl` default: 5 minutes.
const DEFAULT_LEASE_TTL: Duration = Duration::from_mins(5);
/// The module's `max_consecutive_expiries` default.
const DEFAULT_MAX_CONSECUTIVE_EXPIRIES: u32 = 3;
/// The module's `max_pull_batch` default, a placeholder nobody has chosen.
const DEFAULT_MAX_PULL_BATCH: u32 = 100;
/// The module's `superseded_ttl` and `dead_letter_ttl` default: 12 hours.
const DEFAULT_DECAY_TTL: Duration = Duration::from_hours(12);

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

impl Config {
    /// Checks the module's five config invariants.
    ///
    /// `ExpiryLimitPositive` and `PullBatchPositive` are checked here. The three
    /// non-negative TTL invariants, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`
    /// and `DeadLetterTtlNonNegative`, hold by representation: a [`Duration`] cannot
    /// be negative, and zero is admitted, as the module says.
    ///
    /// # Errors
    ///
    /// [`ConfigError::ExpiryLimitNotPositive`] when `max_consecutive_expiries` is 0,
    /// and [`ConfigError::PullBatchNotPositive`] when `max_pull_batch` is 0, checked
    /// in that order.
    ///
    /// ```
    /// use libpachinko::config::Config;
    /// use libpachinko::error::ConfigError;
    ///
    /// let mut config = Config::default();
    /// config.max_consecutive_expiries = 0;
    /// assert_eq!(config.validate(), Err(ConfigError::ExpiryLimitNotPositive));
    /// ```
    pub const fn validate(&self) -> Result<(), ConfigError> {
        if self.max_consecutive_expiries < 1 {
            return Err(ConfigError::ExpiryLimitNotPositive);
        }
        if self.max_pull_batch < 1 {
            return Err(ConfigError::PullBatchNotPositive);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::error::ConfigError;
    use std::time::Duration;

    #[test]
    fn zero_ttls_are_admitted() {
        let config = Config {
            lease_ttl: Duration::ZERO,
            superseded_ttl: Duration::ZERO,
            dead_letter_ttl: Duration::ZERO,
            ..Config::default()
        };
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn a_zero_pull_batch_is_rejected() {
        let config = Config {
            max_pull_batch: 0,
            ..Config::default()
        };
        assert_eq!(config.validate(), Err(ConfigError::PullBatchNotPositive));
    }
}
