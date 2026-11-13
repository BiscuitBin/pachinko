//! The specification's `config` block: five values a deployment sets, with the
//! specification's defaults, checked once at load against the config invariants.

use std::time::Duration;

use crate::error::ConfigError;

/// The deployment's configuration.
///
/// [`Config::default`] is the specification's five defaults, which decision 0019
/// confirms for the MVP. The fields are public so a deployment changes the ones it
/// sets; [`Config::check`], which [`Queue::new`](crate::Queue::new) calls,
/// refuses what `ExpiryLimitPositive` and `PullBatchPositive` exclude.
/// `LeaseTtlNonNegative`, `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`
/// hold by type, since a [`Duration`] is never negative: a negative TTL cannot be
/// written.
///
/// ```
/// use std::time::Duration;
/// use libpachinko::Config;
///
/// let config = Config::default();
/// assert_eq!(config.lease_ttl, Duration::from_secs(5 * 60));
/// assert_eq!(config.max_consecutive_expiries, 3);
/// assert_eq!(config.max_pull_batch, 100);
/// assert_eq!(config.superseded_ttl, Duration::from_secs(12 * 60 * 60));
/// assert_eq!(config.dead_letter_ttl, Duration::from_secs(12 * 60 * 60));
/// assert_eq!(config.check(), Ok(()));
/// ```
///
/// A TTL is a [`Duration`], which is never negative:
///
/// ```
/// // obligation: invariant.LeaseTtlNonNegative
/// // obligation: invariant.SupersededTtlNonNegative
/// // obligation: invariant.DeadLetterTtlNonNegative
/// use std::time::Duration;
/// let config = libpachinko::Config::default();
/// let ttls: [Duration; 3] = [config.lease_ttl, config.superseded_ttl, config.dead_letter_ttl];
/// assert!(ttls.iter().all(|ttl| *ttl >= Duration::ZERO));
/// ```
///
/// A negative TTL does not compile, for each of the three:
///
/// ```compile_fail,E0600
/// // scenario: S005
/// // obligation: invariant.LeaseTtlNonNegative
/// let mut config = libpachinko::Config::default();
/// config.lease_ttl = -config.lease_ttl;
/// ```
///
/// ```compile_fail,E0600
/// // obligation: invariant.SupersededTtlNonNegative
/// let mut config = libpachinko::Config::default();
/// config.superseded_ttl = -config.superseded_ttl;
/// ```
///
/// ```compile_fail,E0600
/// // obligation: invariant.DeadLetterTtlNonNegative
/// let mut config = libpachinko::Config::default();
/// config.dead_letter_ttl = -config.dead_letter_ttl;
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// How long a lease lasts after its claim or its consumer's last pull, ack, nack
    /// or renewal. Zero is admitted.
    pub lease_ttl: Duration,
    /// How many consecutive lease expiries park a pocket; at least 1.
    pub max_consecutive_expiries: u32,
    /// The most records one pull hands out; at least 1.
    pub max_pull_batch: u32,
    /// How long a superseded or stale payload is kept before it decays.
    pub superseded_ttl: Duration,
    /// How long a dead letter is kept before it decays.
    pub dead_letter_ttl: Duration,
}

impl Config {
    /// Checks the configuration against the config invariants a value can break.
    ///
    /// # Errors
    ///
    /// [`ConfigError::MaxConsecutiveExpiriesZero`] (`ExpiryLimitPositive`) and
    /// [`ConfigError::MaxPullBatchZero`] (`PullBatchPositive`), in that order.
    ///
    /// ```
    /// use libpachinko::{Config, ConfigError};
    ///
    /// let mut config = Config::default();
    /// config.max_pull_batch = 0;
    /// assert_eq!(config.check(), Err(ConfigError::MaxPullBatchZero));
    /// ```
    pub const fn check(&self) -> Result<(), ConfigError> {
        if self.max_consecutive_expiries < 1 {
            return Err(ConfigError::MaxConsecutiveExpiriesZero);
        }
        if self.max_pull_batch < 1 {
            return Err(ConfigError::MaxPullBatchZero);
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lease_ttl: Duration::from_mins(5),
            max_consecutive_expiries: 3,
            max_pull_batch: 100,
            superseded_ttl: Duration::from_hours(12),
            dead_letter_ttl: Duration::from_hours(12),
        }
    }
}
