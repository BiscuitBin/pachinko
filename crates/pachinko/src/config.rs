//! The `serve` command's flags: where to listen and the specification's five config
//! values, each with its default. T26, T27 and T29 add their flags here.

use std::num::NonZeroUsize;
use std::time::Duration;

use libpachinko::config::Config;

use crate::ServeError;

/// Where `serve` listens unless `--listen` says otherwise: loopback only.
const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

/// The most rows one batch publish may carry unless `--max-batch-rows` says otherwise.
const DEFAULT_MAX_BATCH_ROWS: usize = 1000;
/// The largest decoded payload one row may carry unless `--max-payload-bytes` says
/// otherwise: 1 MiB.
const DEFAULT_MAX_PAYLOAD_BYTES: usize = 1 << 20;
/// The largest request body unless `--max-body-bytes` says otherwise: 16 MiB.
const DEFAULT_MAX_BODY_BYTES: usize = 16 << 20;
/// The shortest batch body around one row with empty keys and the longest rank
/// literal, less its payload: `{"records":[{"pocket_key":"","record_key":"",
/// "rank":-9223372036854775808,"payload":""}]}`. The single `PUT`'s envelope is
/// shorter, so this covers both.
const BATCH_ENVELOPE_BYTES: usize = 88;

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
    max_batch_rows: usize,
    max_payload_bytes: usize,
    max_body_bytes: usize,
}

impl ServeConfig {
    /// Reads `serve`'s flags, each a `--name value` pair: `--listen <address>`
    /// (default `127.0.0.1:8080`), `--lease-ttl-ms` (300000),
    /// `--max-consecutive-expiries` (3), `--max-pull-batch` (100),
    /// `--superseded-ttl-ms` (43200000) and `--dead-letter-ttl-ms` (43200000), and
    /// the request-shape limits `--max-batch-rows` (1000), `--max-payload-bytes`
    /// (1048576) and `--max-body-bytes` (16777216).
    ///
    /// # Errors
    ///
    /// [`ServeError::Usage`], naming the flag, for an unknown flag, a flag without a
    /// value, a value that is not an unsigned integer, a limit of 0, or a
    /// `--max-body-bytes` below the base64 size of one maximum payload plus the batch
    /// envelope; [`ServeError::Config`] when the library's config check refuses the
    /// values.
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
            max_batch_rows: DEFAULT_MAX_BATCH_ROWS,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        };
        let mut args = args.into_iter();
        while let Some(name) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| ServeError::Usage(format!("{} needs a value", name.as_ref())))?;
            config.set(name.as_ref(), value.as_ref())?;
        }
        config.queue.check()?;
        config.check_body_limit()?;
        Ok(config)
    }

    /// The load check: one maximum payload, base64-encoded in the shortest batch
    /// body, fits under `--max-body-bytes`.
    fn check_body_limit(&self) -> Result<(), ServeError> {
        let Some(minimum) = min_body_bytes(self.max_payload_bytes) else {
            return Err(ServeError::Usage(format!(
                "--max-payload-bytes {} is too large for any body cap: its batch body \
                exceeds the largest representable size",
                self.max_payload_bytes
            )));
        };
        if self.max_body_bytes < minimum {
            return Err(ServeError::Usage(format!(
                "--max-body-bytes must be at least {minimum} to fit one payload of \
                --max-payload-bytes {} in a batch, not {}",
                self.max_payload_bytes, self.max_body_bytes
            )));
        }
        Ok(())
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

    /// The most rows one batch publish may carry.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(Vec::<String>::new())?;
    /// assert_eq!(config.max_batch_rows(), 1000);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn max_batch_rows(&self) -> usize {
        self.max_batch_rows
    }

    /// The largest decoded payload one row, or one single publish, may carry.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(Vec::<String>::new())?;
    /// assert_eq!(config.max_payload_bytes(), 1_048_576);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }

    /// The largest request body any route reads.
    ///
    /// ```
    /// let config = pachinko::ServeConfig::parse(Vec::<String>::new())?;
    /// assert_eq!(config.max_body_bytes(), 16_777_216);
    /// # Ok::<(), pachinko::ServeError>(())
    /// ```
    #[must_use]
    pub const fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
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
            "--max-batch-rows" => self.max_batch_rows = at_least_one(name, value)?,
            "--max-payload-bytes" => self.max_payload_bytes = at_least_one(name, value)?,
            "--max-body-bytes" => self.max_body_bytes = at_least_one(name, value)?,
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

/// `value` as a limit of at least 1.
fn at_least_one(name: &str, value: &str) -> Result<usize, ServeError> {
    value.parse().map(NonZeroUsize::get).map_err(|_| {
        ServeError::Usage(format!("{name} takes a count of at least 1, not {value:?}"))
    })
}

/// The smallest `--max-body-bytes` that fits one payload of `max_payload_bytes`:
/// its base64 size, `ceil(max_payload_bytes / 3) * 4`, plus the batch envelope.
/// `None` when that size does not fit in a `usize`.
fn min_body_bytes(max_payload_bytes: usize) -> Option<usize> {
    max_payload_bytes
        .div_ceil(3)
        .checked_mul(4)
        .and_then(|encoded| encoded.checked_add(BATCH_ENVELOPE_BYTES))
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
        assert_eq!(config.max_batch_rows(), 1000);
        assert_eq!(config.max_payload_bytes(), 1_048_576);
        assert_eq!(config.max_body_bytes(), 16_777_216);
    }

    #[test]
    fn each_limit_flag_sets_its_value() {
        let config = ServeConfig::parse([
            "--max-batch-rows",
            "7",
            "--max-payload-bytes",
            "16",
            "--max-body-bytes",
            "200",
        ])
        .unwrap();
        assert_eq!(config.max_batch_rows(), 7);
        assert_eq!(config.max_payload_bytes(), 16);
        assert_eq!(config.max_body_bytes(), 200);
    }

    #[test]
    fn a_bad_limit_is_a_usage_error_naming_the_flag() {
        for flag in [
            "--max-batch-rows",
            "--max-payload-bytes",
            "--max-body-bytes",
        ] {
            for value in ["0", "-1", "x"] {
                assert_eq!(
                    ServeConfig::parse([flag, value]),
                    Err(ServeError::Usage(format!(
                        "{flag} takes a count of at least 1, not {value:?}"
                    )))
                );
            }
            assert_eq!(
                ServeConfig::parse([flag]),
                Err(ServeError::Usage(format!("{flag} needs a value")))
            );
        }
    }

    #[test]
    fn the_default_body_limit_fits_one_maximum_payload() {
        assert_eq!(super::min_body_bytes(1_048_576), Some(1_398_192));
        assert_eq!(super::min_body_bytes(16), Some(112));
        assert_eq!(super::min_body_bytes(usize::MAX), None);
    }

    #[test]
    fn a_payload_limit_no_body_can_carry_is_a_usage_error() {
        let max = usize::MAX.to_string();
        assert_eq!(
            ServeConfig::parse(["--max-payload-bytes", &max, "--max-body-bytes", &max]),
            Err(ServeError::Usage(format!(
                "--max-payload-bytes {max} is too large for any body cap: its batch body \
                exceeds the largest representable size"
            )))
        );
    }

    #[test]
    fn a_body_limit_below_one_maximum_payload_is_refused() {
        let with_body = |body: &str| {
            ServeConfig::parse(["--max-payload-bytes", "16", "--max-body-bytes", body])
        };
        assert_eq!(
            with_body("111"),
            Err(ServeError::Usage(String::from(
                "--max-body-bytes must be at least 112 to fit one payload of \
                --max-payload-bytes 16 in a batch, not 111"
            )))
        );
        assert_eq!(with_body("112").unwrap().max_body_bytes(), 112);
        assert!(matches!(
            ServeConfig::parse(["--max-body-bytes", "1398191"]),
            Err(ServeError::Usage(text)) if text.contains("at least 1398192")
        ));
        let at_minimum = ServeConfig::parse(["--max-body-bytes", "1398192"]).unwrap();
        assert_eq!(at_minimum.max_body_bytes(), 1_398_192);
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
