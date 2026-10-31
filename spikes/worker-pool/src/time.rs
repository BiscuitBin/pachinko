//! Time as values: a timestamp, a duration and the module's configuration.
//!
//! A machine never reads a clock. The pool stamps every input with a [`Timestamp`]
//! as it dequeues it, and the machine receives that value as the `now` argument of
//! its step. A test chooses `now` outright.

/// Nanoseconds since the pool started, as a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Timestamp(u64);

impl Timestamp {
    /// The pool's start.
    pub const ZERO: Self = Self(0);

    /// A timestamp this many nanoseconds after the pool's start.
    ///
    /// ```
    /// use worker_pool::Timestamp;
    /// assert_eq!(Timestamp::from_nanos(5).nanos(), 5);
    /// ```
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// A timestamp this many milliseconds after the pool's start.
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * 1_000_000)
    }

    /// Nanoseconds since the pool's start.
    #[must_use]
    pub const fn nanos(self) -> u64 {
        self.0
    }

    /// This timestamp moved forward by `duration`, saturating.
    ///
    /// ```
    /// use worker_pool::{Duration, Timestamp};
    /// let later = Timestamp::from_nanos(10).plus(Duration::from_nanos(5));
    /// assert_eq!(later, Timestamp::from_nanos(15));
    /// ```
    #[must_use]
    pub const fn plus(self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration.0))
    }

    /// The duration from `earlier` to this timestamp, zero when `earlier` is later.
    #[must_use]
    pub const fn since(self, earlier: Self) -> Duration {
        Duration(self.0.saturating_sub(earlier.0))
    }
}

/// A span of time in nanoseconds, as a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Duration(u64);

impl Duration {
    /// No time at all.
    pub const ZERO: Self = Self(0);

    /// A duration of this many nanoseconds.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// A duration of this many milliseconds.
    ///
    /// ```
    /// use worker_pool::Duration;
    /// assert_eq!(Duration::from_millis(2).nanos(), 2_000_000);
    /// ```
    #[must_use]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * 1_000_000)
    }

    /// A duration of this many seconds.
    #[must_use]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * 1_000_000_000)
    }

    /// The duration in nanoseconds.
    #[must_use]
    pub const fn nanos(self) -> u64 {
        self.0
    }

    /// The same span as a [`std::time::Duration`], for the pool's clock only.
    #[must_use]
    pub const fn as_std(self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.0)
    }
}

/// The module's `config` block: the values every rule reads.
///
/// The defaults are the module's. A test or a measurement sets its own through the
/// `with_*` methods, which is how lease TTLs become milliseconds in tests.
///
/// ```
/// use worker_pool::{Config, Duration};
/// let config = Config::default().with_lease_ttl(Duration::from_millis(20));
/// assert_eq!(config.lease_ttl, Duration::from_millis(20));
/// assert_eq!(config.max_consecutive_expiries, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// How long a lease lasts from its grant or its last extension.
    pub lease_ttl: Duration,
    /// The consecutive lapse that parks a pocket; the module needs it to be at least 1.
    pub max_consecutive_expiries: u32,
    /// The most records one pull hands back; the module needs it to be at least 1.
    pub max_pull_batch: u64,
    /// How long a superseded record stays visible.
    pub superseded_ttl: Duration,
    /// How long a dead letter stays visible.
    pub dead_letter_ttl: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lease_ttl: Duration::from_secs(5 * 60),
            max_consecutive_expiries: 3,
            max_pull_batch: 100,
            superseded_ttl: Duration::from_secs(12 * 60 * 60),
            dead_letter_ttl: Duration::from_secs(12 * 60 * 60),
        }
    }
}

impl Config {
    /// The same configuration with another lease TTL.
    #[must_use]
    pub const fn with_lease_ttl(mut self, lease_ttl: Duration) -> Self {
        self.lease_ttl = lease_ttl;
        self
    }

    /// The same configuration with another parking threshold.
    #[must_use]
    pub const fn with_max_consecutive_expiries(mut self, max: u32) -> Self {
        self.max_consecutive_expiries = max;
        self
    }

    /// The same configuration with another batch bound.
    #[must_use]
    pub const fn with_max_pull_batch(mut self, max: u64) -> Self {
        self.max_pull_batch = max;
        self
    }

    /// The same configuration with another superseded-record TTL.
    #[must_use]
    pub const fn with_superseded_ttl(mut self, ttl: Duration) -> Self {
        self.superseded_ttl = ttl;
        self
    }

    /// The same configuration with another dead-letter TTL.
    #[must_use]
    pub const fn with_dead_letter_ttl(mut self, ttl: Duration) -> Self {
        self.dead_letter_ttl = ttl;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Duration, Timestamp};

    #[test]
    fn plus_saturates_instead_of_wrapping() {
        let t = Timestamp::from_nanos(u64::MAX).plus(Duration::from_nanos(1));
        assert_eq!(t.nanos(), u64::MAX);
    }

    #[test]
    fn since_is_zero_when_earlier_is_later() {
        assert_eq!(
            Timestamp::from_nanos(1).since(Timestamp::from_nanos(5)),
            Duration::ZERO
        );
        assert_eq!(
            Timestamp::from_nanos(5).since(Timestamp::from_nanos(1)),
            Duration::from_nanos(4)
        );
    }

    #[test]
    fn the_defaults_are_the_modules() {
        let c = Config::default();
        assert_eq!(c.lease_ttl, Duration::from_secs(300));
        assert_eq!(c.max_consecutive_expiries, 3);
        assert_eq!(c.max_pull_batch, 100);
        assert_eq!(c.superseded_ttl, Duration::from_secs(43_200));
        assert_eq!(c.dead_letter_ttl, Duration::from_secs(43_200));
    }

    #[test]
    fn the_builders_replace_one_field_each() {
        let c = Config::default()
            .with_max_consecutive_expiries(1)
            .with_max_pull_batch(2)
            .with_superseded_ttl(Duration::from_millis(3))
            .with_dead_letter_ttl(Duration::from_millis(4));
        assert_eq!(c.max_consecutive_expiries, 1);
        assert_eq!(c.max_pull_batch, 2);
        assert_eq!(c.superseded_ttl, Duration::from_millis(3));
        assert_eq!(c.dead_letter_ttl, Duration::from_millis(4));
        assert_eq!(c.lease_ttl, Config::default().lease_ttl);
        assert_eq!(
            Duration::from_secs(1).as_std(),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(Timestamp::from_millis(1), Timestamp::from_nanos(1_000_000));
    }
}
