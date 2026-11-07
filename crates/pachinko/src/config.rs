//! The `serve` command's flags: where to listen and the specification's five config
//! values, each with its default. T26, T27 and T29 add their flags here.

use std::time::Duration;

use libpachinko::config::Config;

use crate::ServeError;

/// Where `serve` listens unless `--listen` says otherwise: loopback only.
const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

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
}

impl ServeConfig {
    /// Reads `serve`'s flags, each a `--name value` pair: `--listen <address>`
    /// (default `127.0.0.1:8080`), `--lease-ttl-ms` (300000),
    /// `--max-consecutive-expiries` (3), `--max-pull-batch` (100),
    /// `--superseded-ttl-ms` (43200000) and `--dead-letter-ttl-ms` (43200000).
    ///
    /// # Errors
    ///
    /// [`ServeError::Usage`], naming the flag, for an unknown flag, a flag without a
    /// value, or a value that is not an unsigned integer; [`ServeError::Config`] when
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

    use super::ServeConfig;
    use crate::ServeError;

    #[test]
    fn no_flags_give_the_defaults() {
        let config = ServeConfig::parse(Vec::<&str>::new()).unwrap();
        assert_eq!(config.listen(), "127.0.0.1:8080");
        assert_eq!(config.queue(), &Config::default());
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
