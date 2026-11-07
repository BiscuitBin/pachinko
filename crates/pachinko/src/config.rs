//! The `serve` command's flags: where to listen and the specification's five config
//! values, each with its default; then T29's log level, request timeout and shutdown
//! grace. T26 and T27 add their flags here.

use std::time::Duration;

use libpachinko::config::Config;
use tracing::Level;

use crate::ServeError;

/// Where `serve` listens unless `--listen` says otherwise: loopback only.
const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

/// The most detailed log level written unless `--log-level` says otherwise.
const DEFAULT_LOG_LEVEL: Level = Level::INFO;

/// How long a request or an idle connection may take unless `--request-timeout-ms`
/// says otherwise.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long shutdown waits before it cuts open connections, and again before it
/// aborts what is left, unless `--shutdown-grace-ms` says otherwise.
const DEFAULT_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// The configuration `serve` runs with: the address to listen on and the queue's
/// [`Config`], checked.
///
/// ```
/// use std::time::Duration;
///
/// let config = pachinko::ServeConfig::parse(["--listen", "127.0.0.1:0", "--lease-ttl-ms", "250"])?;
/// assert_eq!(config.listen(), "127.0.0.1:0");
/// assert_eq!(config.queue().lease_ttl, Duration::from_millis(250));
/// assert_eq!(config.queue().max_pull_batch, 100);
/// # Ok::<(), pachinko::ServeError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ServeConfig {
    listen: String,
    queue: Config,
    log_level: Level,
    request_timeout: Duration,
    shutdown_grace: Duration,
}

impl ServeConfig {
    /// Reads `serve`'s flags, each a `--name value` pair: `--listen <address>`
    /// (default `127.0.0.1:8080`), `--lease-ttl-ms` (300000),
    /// `--max-consecutive-expiries` (3), `--max-pull-batch` (100),
    /// `--superseded-ttl-ms` (43200000) and `--dead-letter-ttl-ms` (43200000); then
    /// `--log-level` (`info`), `--request-timeout-ms` (30000) and
    /// `--shutdown-grace-ms` (5000).
    ///
    /// # Errors
    ///
    /// [`ServeError::Usage`], naming the flag, for an unknown flag, a flag without a
    /// value, a value that is not an unsigned integer, a log level that is not one of
    /// `error`, `warn`, `info`, `debug` and `trace`, or a request timeout of zero,
    /// which would time every request out; [`ServeError::Config`] when
    /// the library's config check refuses the values.
    ///
    /// ```
    /// use pachinko::{ServeConfig, ServeError};
    ///
    /// assert!(matches!(ServeConfig::parse(["--bogus", "1"]), Err(ServeError::Usage(_))));
    /// assert!(matches!(ServeConfig::parse(["--max-pull-batch", "0"]), Err(ServeError::Config(_))));
    /// ```
    pub fn parse<I>(args: I) -> Result<Self, ServeError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut config = Self {
            listen: String::from(DEFAULT_LISTEN),
            queue: Config::default(),
            log_level: DEFAULT_LOG_LEVEL,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            shutdown_grace: DEFAULT_SHUTDOWN_GRACE,
        };
        let mut args = args.into_iter();
        while let Some(name) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| ServeError::Usage(format!("{} needs a value", name.as_ref())))?;
            config.set(name.as_ref(), value.as_ref())?;
        }
        config.queue.check()?;
        Ok(config)
    }

    /// The address to listen on.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(Vec::<String>::new())?;
    /// assert_eq!(config.listen(), "127.0.0.1:8080");
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub fn listen(&self) -> &str {
        &self.listen
    }

    /// The queue's configuration.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(["--max-pull-batch", "7"])?;
    /// assert_eq!(config.queue().max_pull_batch, 7);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn queue(&self) -> &Config {
        &self.queue
    }

    /// The most detailed level the log writer writes.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(["--log-level", "debug"])?;
    /// assert_eq!(config.log_level(), tracing::Level::DEBUG);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn log_level(&self) -> Level {
        self.log_level
    }

    /// How long a request's handler may run, and how long a connection may wait
    /// for its next bytes, before either is given up.
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let config = pachinko::ServeConfig::parse(["--request-timeout-ms", "250"])?;
    /// assert_eq!(config.request_timeout(), Duration::from_millis(250));
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// How long shutdown lets in-flight requests finish before it cuts the open
    /// connections, and how long it then waits before it aborts what is left.
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let config = pachinko::ServeConfig::parse(["--shutdown-grace-ms", "0"])?;
    /// assert_eq!(config.shutdown_grace(), Duration::ZERO);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn shutdown_grace(&self) -> Duration {
        self.shutdown_grace
    }

    /// Sets the flag `name` to `value`.
    fn set(&mut self, name: &str, value: &str) -> Result<(), ServeError> {
        let queue = &mut self.queue;
        match name {
            "--listen" => value.clone_into(&mut self.listen),
            "--lease-ttl-ms" => queue.lease_ttl = millis(name, value)?,
            "--max-consecutive-expiries" => queue.max_consecutive_expiries = count(name, value)?,
            "--max-pull-batch" => queue.max_pull_batch = count(name, value)?,
            "--superseded-ttl-ms" => queue.superseded_ttl = millis(name, value)?,
            "--dead-letter-ttl-ms" => queue.dead_letter_ttl = millis(name, value)?,
            "--log-level" => self.log_level = level(name, value)?,
            "--request-timeout-ms" => self.request_timeout = positive_millis(name, value)?,
            "--shutdown-grace-ms" => self.shutdown_grace = millis(name, value)?,
            _ => return Err(ServeError::Usage(format!("unknown flag {name}"))),
        }
        Ok(())
    }
}

/// `value` as a duration in milliseconds.
fn millis(name: &str, value: &str) -> Result<Duration, ServeError> {
    value
        .parse()
        .map(Duration::from_millis)
        .map_err(|_| ServeError::Usage(format!("{name} takes milliseconds, not {value:?}")))
}

/// `value` as a duration in milliseconds, refusing zero.
fn positive_millis(name: &str, value: &str) -> Result<Duration, ServeError> {
    let duration = millis(name, value)?;
    if duration.is_zero() {
        return Err(ServeError::Usage(format!("{name} must be at least 1")));
    }
    Ok(duration)
}

/// `value` as a log level: one of the five names, lower case. `Level`'s own parser
/// also reads digits and any case, which the flag does not promise.
fn level(name: &str, value: &str) -> Result<Level, ServeError> {
    match value {
        "error" => Ok(Level::ERROR),
        "warn" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(ServeError::Usage(format!(
            "{name} takes one of error, warn, info, debug and trace, not {value:?}"
        ))),
    }
}

/// `value` as a count.
fn count(name: &str, value: &str) -> Result<u32, ServeError> {
    value
        .parse()
        .map_err(|_| ServeError::Usage(format!("{name} takes a count, not {value:?}")))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use libpachinko::config::Config;
    use libpachinko::error::ConfigError;
    use tracing::Level;

    use super::ServeConfig;
    use crate::ServeError;

    #[test]
    fn no_flags_give_the_defaults() {
        let config = ServeConfig::parse(Vec::<&str>::new()).unwrap();
        assert_eq!(config.listen(), "127.0.0.1:8080");
        assert_eq!(config.queue(), &Config::default());
        assert_eq!(config.log_level(), Level::INFO);
        assert_eq!(config.request_timeout(), Duration::from_secs(30));
        assert_eq!(config.shutdown_grace(), Duration::from_millis(5_000));
    }

    #[test]
    fn each_operability_flag_sets_its_value() {
        let config = ServeConfig::parse([
            "--log-level",
            "trace",
            "--request-timeout-ms",
            "1",
            "--shutdown-grace-ms",
            "0",
        ])
        .unwrap();
        assert_eq!(config.log_level(), Level::TRACE);
        assert_eq!(config.request_timeout(), Duration::from_millis(1));
        assert_eq!(config.shutdown_grace(), Duration::ZERO);
        for (name, level) in [
            ("error", Level::ERROR),
            ("warn", Level::WARN),
            ("info", Level::INFO),
            ("debug", Level::DEBUG),
        ] {
            let config = ServeConfig::parse(["--log-level", name]).unwrap();
            assert_eq!(config.log_level(), level, "{name}");
        }
    }

    #[test]
    fn every_flag_sets_its_value() {
        let config = ServeConfig::parse([
            "--listen",
            "0.0.0.0:9",
            "--lease-ttl-ms",
            "1",
            "--max-consecutive-expiries",
            "2",
            "--max-pull-batch",
            "3",
            "--superseded-ttl-ms",
            "4",
            "--dead-letter-ttl-ms",
            "5",
        ])
        .unwrap();
        assert_eq!(config.listen(), "0.0.0.0:9");
        let queue = config.queue();
        assert_eq!(queue.lease_ttl, Duration::from_millis(1));
        assert_eq!(queue.max_consecutive_expiries, 2);
        assert_eq!(queue.max_pull_batch, 3);
        assert_eq!(queue.superseded_ttl, Duration::from_millis(4));
        assert_eq!(queue.dead_letter_ttl, Duration::from_millis(5));
    }

    #[test]
    fn a_bad_command_line_is_a_usage_error_naming_the_flag() {
        let usage = |args: &[&str]| match ServeConfig::parse(args) {
            Err(ServeError::Usage(text)) => text,
            other => panic!("{args:?} gave {other:?}"),
        };
        assert_eq!(usage(&["--bogus", "1"]), "unknown flag --bogus");
        assert_eq!(usage(&["--listen"]), "--listen needs a value");
        assert_eq!(
            usage(&["--lease-ttl-ms", "-1"]),
            "--lease-ttl-ms takes milliseconds, not \"-1\""
        );
        assert_eq!(
            usage(&["--dead-letter-ttl-ms", "x"]),
            "--dead-letter-ttl-ms takes milliseconds, not \"x\""
        );
        assert_eq!(
            usage(&["--max-pull-batch", "x"]),
            "--max-pull-batch takes a count, not \"x\""
        );
        assert_eq!(
            usage(&["--max-consecutive-expiries", "-1"]),
            "--max-consecutive-expiries takes a count, not \"-1\""
        );
        let levels = "takes one of error, warn, info, debug and trace";
        for bad in ["bogus", "INFO", "2", ""] {
            assert_eq!(
                usage(&["--log-level", bad]),
                format!("--log-level {levels}, not {bad:?}")
            );
        }
        assert_eq!(usage(&["--log-level"]), "--log-level needs a value");
        assert_eq!(
            usage(&["--request-timeout-ms", "0"]),
            "--request-timeout-ms must be at least 1"
        );
        assert_eq!(
            usage(&["--request-timeout-ms", "-5"]),
            "--request-timeout-ms takes milliseconds, not \"-5\""
        );
        assert_eq!(
            usage(&["--shutdown-grace-ms", "soon"]),
            "--shutdown-grace-ms takes milliseconds, not \"soon\""
        );
        assert_eq!(
            usage(&["--shutdown-grace-ms"]),
            "--shutdown-grace-ms needs a value"
        );
    }

    #[test]
    fn the_library_refuses_a_zero_limit() {
        assert_eq!(
            ServeConfig::parse(["--max-consecutive-expiries", "0"]),
            Err(ServeError::Config(ConfigError::MaxConsecutiveExpiriesZero))
        );
        assert_eq!(
            ServeConfig::parse(["--max-pull-batch", "0"]),
            Err(ServeError::Config(ConfigError::MaxPullBatchZero))
        );
    }
}
