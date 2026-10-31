//! The `serve` command's flags: where to listen, the module's five config values,
//! and how often the sweep runs. Each flag has the module's default.

use std::time::Duration;

use crate::Error;

/// Where `serve` listens unless `--listen` says otherwise.
const DEFAULT_LISTEN: &str = "127.0.0.1:8080";
/// How often the sweep fires `LeaseLapses` and the decay rules unless `--sweep-ms`
/// says otherwise.
const DEFAULT_SWEEP: Duration = Duration::from_secs(1);

/// The configuration `serve` runs with.
///
/// ```
/// use std::time::Duration;
/// use pachinko::ServeConfig;
///
/// let config = ServeConfig::parse(["--listen", "127.0.0.1:0", "--lease-ttl-ms", "250"]).unwrap();
/// assert_eq!(config.listen(), "127.0.0.1:0");
/// assert_eq!(config.queue().lease_ttl(), Duration::from_millis(250));
/// assert_eq!(config.queue().max_pull_batch(), 100);
/// assert_eq!(config.sweep(), Duration::from_secs(1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ServeConfig {
    listen: String,
    queue: libpachinko::Config,
    sweep: Duration,
}

/// The flags as read, before the queue's configuration is checked.
#[derive(Debug)]
struct Flags {
    listen: String,
    lease_ttl: Duration,
    max_consecutive_expiries: u32,
    max_pull_batch: u32,
    superseded_ttl: Duration,
    dead_letter_ttl: Duration,
    sweep: Duration,
}

impl ServeConfig {
    /// Reads `serve`'s flags, each `--name value`: `--listen <address>`,
    /// `--lease-ttl-ms`, `--max-consecutive-expiries`, `--max-pull-batch`,
    /// `--superseded-ttl-ms`, `--dead-letter-ttl-ms` and `--sweep-ms`.
    ///
    /// # Errors
    ///
    /// [`Error::Usage`] for an unknown flag, a flag without a value, a value that is
    /// not a non-negative integer, or a sweep of 0 ms; [`Error::Config`] when the
    /// queue's configuration breaks a config invariant.
    ///
    /// ```
    /// use pachinko::{Error, ServeConfig};
    /// assert!(matches!(ServeConfig::parse(["--bogus"]), Err(Error::Usage(_))));
    /// assert!(matches!(ServeConfig::parse(["--max-pull-batch", "0"]), Err(Error::Config(_))));
    /// ```
    pub fn parse<I>(args: I) -> Result<Self, Error>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let flags = read_flags(args)?;
        let queue = libpachinko::Config::new(
            flags.lease_ttl,
            flags.max_consecutive_expiries,
            flags.max_pull_batch,
            flags.superseded_ttl,
            flags.dead_letter_ttl,
        )?;
        Ok(Self {
            listen: flags.listen,
            queue,
            sweep: flags.sweep,
        })
    }

    /// The address to listen on.
    #[must_use]
    pub fn listen(&self) -> &str {
        &self.listen
    }

    /// The queue's configuration.
    #[must_use]
    pub const fn queue(&self) -> &libpachinko::Config {
        &self.queue
    }

    /// How often the sweep runs.
    #[must_use]
    pub const fn sweep(&self) -> Duration {
        self.sweep
    }
}

/// Reads every `--name value` pair over the defaults.
fn read_flags<I>(args: I) -> Result<Flags, Error>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let defaults = libpachinko::Config::default();
    let mut flags = Flags {
        listen: String::from(DEFAULT_LISTEN),
        lease_ttl: defaults.lease_ttl(),
        max_consecutive_expiries: defaults.max_consecutive_expiries(),
        max_pull_batch: defaults.max_pull_batch(),
        superseded_ttl: defaults.superseded_ttl(),
        dead_letter_ttl: defaults.dead_letter_ttl(),
        sweep: DEFAULT_SWEEP,
    };
    let mut args = args.into_iter();
    while let Some(name) = args.next() {
        let name = name.as_ref();
        let value = args
            .next()
            .ok_or_else(|| Error::Usage(format!("{name} needs a value")))?;
        let value = value.as_ref();
        match name {
            "--listen" => value.clone_into(&mut flags.listen),
            "--lease-ttl-ms" => flags.lease_ttl = millis(name, value)?,
            "--max-consecutive-expiries" => flags.max_consecutive_expiries = count(name, value)?,
            "--max-pull-batch" => flags.max_pull_batch = count(name, value)?,
            "--superseded-ttl-ms" => flags.superseded_ttl = millis(name, value)?,
            "--dead-letter-ttl-ms" => flags.dead_letter_ttl = millis(name, value)?,
            "--sweep-ms" => flags.sweep = millis(name, value)?,
            _ => return Err(Error::Usage(format!("unknown flag {name}"))),
        }
    }
    if flags.sweep.is_zero() {
        return Err(Error::Usage(String::from("--sweep-ms must be at least 1")));
    }
    Ok(flags)
}

fn millis(name: &str, value: &str) -> Result<Duration, Error> {
    value
        .parse()
        .map(Duration::from_millis)
        .map_err(|_| Error::Usage(format!("{name} takes milliseconds, not {value:?}")))
}

fn count(name: &str, value: &str) -> Result<u32, Error> {
    value
        .parse()
        .map_err(|_| Error::Usage(format!("{name} takes a count, not {value:?}")))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::ServeConfig;
    use crate::Error;

    #[test]
    fn no_flags_give_the_modules_defaults() {
        let config = ServeConfig::parse(Vec::<&str>::new()).unwrap();
        assert_eq!(config.listen(), "127.0.0.1:8080");
        assert_eq!(config.queue(), &libpachinko::Config::default());
        assert_eq!(config.sweep(), Duration::from_secs(1));
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
            "--sweep-ms",
            "6",
        ])
        .unwrap();
        assert_eq!(config.listen(), "0.0.0.0:9");
        let queue = config.queue();
        assert_eq!(queue.lease_ttl(), Duration::from_millis(1));
        assert_eq!(queue.max_consecutive_expiries(), 2);
        assert_eq!(queue.max_pull_batch(), 3);
        assert_eq!(queue.superseded_ttl(), Duration::from_millis(4));
        assert_eq!(queue.dead_letter_ttl(), Duration::from_millis(5));
        assert_eq!(config.sweep(), Duration::from_millis(6));
    }

    #[test]
    fn a_bad_command_line_is_a_usage_error() {
        let usage = |args: &[&str]| match ServeConfig::parse(args) {
            Err(Error::Usage(text)) => text,
            other => panic!("{args:?} gave {other:?}"),
        };
        assert_eq!(usage(&["--bogus", "1"]), "unknown flag --bogus");
        assert_eq!(usage(&["--listen"]), "--listen needs a value");
        assert_eq!(
            usage(&["--lease-ttl-ms", "-1"]),
            "--lease-ttl-ms takes milliseconds, not \"-1\""
        );
        assert_eq!(
            usage(&["--max-pull-batch", "x"]),
            "--max-pull-batch takes a count, not \"x\""
        );
        assert_eq!(usage(&["--sweep-ms", "0"]), "--sweep-ms must be at least 1");
    }

    #[test]
    fn a_configuration_the_invariants_exclude_is_refused() {
        assert_eq!(
            ServeConfig::parse(["--max-consecutive-expiries", "0"]),
            Err(Error::Config(libpachinko::Error::InvalidConfig(
                "max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)"
            )))
        );
    }
}
