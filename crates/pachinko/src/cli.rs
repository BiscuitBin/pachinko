//! The command line, read with std alone: no arguments for the version line, or
//! `serve` and its options.
//!
//! Each option takes one value, as the next argument; a later option of the same
//! name wins. Durations are a whole number with a unit, `ms`, `s`, `m` or `h`.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use libpachinko::config::Config;
use libpachinko::error::ConfigError;

use crate::ServeError;

/// The units a duration may take, as suffixes, with their length in milliseconds.
/// `ms` is tried before `s` and `m`, which it ends with.
const UNITS: [(&str, u64); 4] = [("ms", 1), ("s", 1_000), ("m", 60_000), ("h", 3_600_000)];

/// What a duration's value must look like.
const DURATION: &str = "a duration such as 250ms, 30s, 5m or 2h";
/// What a count's value must look like.
const COUNT: &str = "a whole number";
/// What an address's value must look like.
const ADDRESS: &str = "an address such as 127.0.0.1:50051";

/// A command line [`run`](crate::run) cannot carry out.
///
/// ```
/// use pachinko::CliError;
///
/// let mut out = Vec::new();
/// let args = ["serve", "--lease-ttl", "soon"].map(str::to_owned);
/// let error = pachinko::run(args, &mut out).unwrap_err();
/// assert!(matches!(error, CliError::BadValue { .. }));
/// assert_eq!(
///     error.to_string(),
///     "--lease-ttl does not take soon: it needs a duration such as 250ms, 30s, 5m or 2h"
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CliError {
    /// The first argument is not a command; `serve` is the only one.
    #[error("unknown command {0}; the only command is serve")]
    UnknownCommand(String),
    /// An option `serve` does not take.
    #[error("unknown option {0}")]
    UnknownOption(String),
    /// An option given without its value.
    #[error("{0} needs a value")]
    MissingValue(String),
    /// An option whose value cannot be read.
    #[error("{option} does not take {value}: it needs {expected}")]
    BadValue {
        /// The option.
        option: String,
        /// The value given.
        value: String,
        /// What the value must look like.
        expected: &'static str,
    },
    /// The options make a configuration the module's config invariants reject.
    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigError),
    /// The runtime could not be built.
    #[error("cannot start the runtime: {0}")]
    Runtime(String),
    /// The listener could not be bound, or its address read.
    #[error("cannot listen on {address}: {reason}")]
    Listen {
        /// The address asked for.
        address: String,
        /// Why it failed.
        reason: String,
    },
    /// A line could not be written to the output.
    #[error("cannot write the output: {0}")]
    Output(String),
    /// The service failed while it served.
    #[error(transparent)]
    Serve(#[from] ServeError),
}

/// What the command line asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Invocation {
    /// No arguments: the version line.
    Version,
    /// `serve`, with its options read.
    Serve(ServeArgs),
}

/// `serve`'s options: where to listen and the queue's configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ServeArgs {
    /// The address to bind.
    pub(crate) listen: SocketAddr,
    /// The queue's configuration, validated.
    pub(crate) config: Config,
}

impl Default for ServeArgs {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            config: Config::default(),
        }
    }
}

/// Reads `args`, the arguments after the program's name.
pub(crate) fn read_args(args: impl IntoIterator<Item = String>) -> Result<Invocation, CliError> {
    let args: Vec<String> = args.into_iter().collect();
    match args.split_first() {
        None => Ok(Invocation::Version),
        Some((command, options)) => read_command(command, options),
    }
}

/// Reads one command and its options.
fn read_command(command: &str, options: &[String]) -> Result<Invocation, CliError> {
    match command {
        "serve" => read_serve(options).map(Invocation::Serve),
        other => Err(CliError::UnknownCommand(other.to_owned())),
    }
}

/// Reads `serve`'s options, in pairs, over the defaults, then validates the
/// configuration they make.
fn read_serve(options: &[String]) -> Result<ServeArgs, CliError> {
    let serve = options
        .chunks(2)
        .try_fold(ServeArgs::default(), read_pair)?;
    serve.config.validate()?;
    Ok(serve)
}

/// Applies one option and its value; a lone option at the end has no value.
fn read_pair(serve: ServeArgs, pair: &[String]) -> Result<ServeArgs, CliError> {
    match pair {
        [option, value] => read_option(serve, option, value),
        lone => Err(CliError::MissingValue(lone.concat())),
    }
}

/// Applies `option` with `value` to `serve`.
fn read_option(mut serve: ServeArgs, option: &str, value: &str) -> Result<ServeArgs, CliError> {
    match option {
        "--listen" => read_address(option, value).map(|listen| {
            serve.listen = listen;
            serve
        }),
        "--lease-ttl" => read_duration(option, value).map(|ttl| {
            serve.config.lease_ttl = ttl;
            serve
        }),
        "--superseded-ttl" => read_duration(option, value).map(|ttl| {
            serve.config.superseded_ttl = ttl;
            serve
        }),
        "--dead-letter-ttl" => read_duration(option, value).map(|ttl| {
            serve.config.dead_letter_ttl = ttl;
            serve
        }),
        "--max-consecutive-expiries" => read_count(option, value).map(|count| {
            serve.config.max_consecutive_expiries = count;
            serve
        }),
        "--max-pull-batch" => read_count(option, value).map(|count| {
            serve.config.max_pull_batch = count;
            serve
        }),
        other => Err(CliError::UnknownOption(other.to_owned())),
    }
}

/// A socket address, such as `127.0.0.1:0`.
fn read_address(option: &str, value: &str) -> Result<SocketAddr, CliError> {
    value.parse().map_err(|_| bad_value(option, value, ADDRESS))
}

/// A whole number of milliseconds, seconds, minutes or hours.
fn read_duration(option: &str, value: &str) -> Result<Duration, CliError> {
    UNITS
        .iter()
        .find_map(|(suffix, millis)| value.strip_suffix(suffix).map(|digits| (digits, *millis)))
        .and_then(|(digits, millis)| digits.parse::<u64>().ok()?.checked_mul(millis))
        .map(Duration::from_millis)
        .ok_or_else(|| bad_value(option, value, DURATION))
}

/// A whole number that fits in a `u32`.
fn read_count(option: &str, value: &str) -> Result<u32, CliError> {
    value.parse().map_err(|_| bad_value(option, value, COUNT))
}

/// The error for `option` given `value`, which is not `expected`.
fn bad_value(option: &str, value: &str, expected: &'static str) -> CliError {
    CliError::BadValue {
        option: option.to_owned(),
        value: value.to_owned(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use super::{CliError, Invocation, ServeArgs, read_args};
    use libpachinko::error::ConfigError;
    use std::time::Duration;

    fn serve(options: &[&str]) -> Result<ServeArgs, CliError> {
        let args = std::iter::once("serve")
            .chain(options.iter().copied())
            .map(str::to_owned);
        read_args(args).map(|invocation| match invocation {
            Invocation::Serve(serve) => serve,
            Invocation::Version => panic!("serve read as the version"),
        })
    }

    fn bad(option: &str, value: &str) -> CliError {
        serve(&[option, value]).unwrap_err()
    }

    #[test]
    fn no_arguments_is_the_version() {
        assert_eq!(read_args(Vec::new()), Ok(Invocation::Version));
    }

    #[test]
    fn serve_alone_listens_on_loopback_with_the_defaults() {
        let serve = serve(&[]).unwrap();
        assert_eq!(serve, ServeArgs::default());
        assert_eq!(serve.listen.to_string(), "127.0.0.1:0");
    }

    #[test]
    fn every_option_is_read() {
        let serve = serve(&[
            "--listen",
            "0.0.0.0:50051",
            "--lease-ttl",
            "250ms",
            "--superseded-ttl",
            "30s",
            "--dead-letter-ttl",
            "5m",
            "--max-consecutive-expiries",
            "7",
            "--max-pull-batch",
            "9",
        ])
        .unwrap();
        assert_eq!(serve.listen.to_string(), "0.0.0.0:50051");
        assert_eq!(serve.config.lease_ttl, Duration::from_millis(250));
        assert_eq!(serve.config.superseded_ttl, Duration::from_secs(30));
        assert_eq!(serve.config.dead_letter_ttl, Duration::from_mins(5));
        assert_eq!(serve.config.max_consecutive_expiries, 7);
        assert_eq!(serve.config.max_pull_batch, 9);
    }

    #[test]
    fn hours_and_zero_are_durations() {
        let serve = serve(&["--lease-ttl", "2h", "--dead-letter-ttl", "0s"]).unwrap();
        assert_eq!(serve.config.lease_ttl, Duration::from_hours(2));
        assert_eq!(serve.config.dead_letter_ttl, Duration::ZERO);
    }

    #[test]
    fn a_later_option_wins() {
        let serve = serve(&["--max-pull-batch", "3", "--max-pull-batch", "4"]).unwrap();
        assert_eq!(serve.config.max_pull_batch, 4);
    }

    #[test]
    fn an_unknown_command_is_refused() {
        let error = read_args(["start".to_owned()]).unwrap_err();
        assert_eq!(error, CliError::UnknownCommand("start".to_owned()));
        assert_eq!(
            error.to_string(),
            "unknown command start; the only command is serve"
        );
    }

    #[test]
    fn an_unknown_option_is_refused() {
        let error = bad("--port", "1");
        assert_eq!(error, CliError::UnknownOption("--port".to_owned()));
        assert_eq!(error.to_string(), "unknown option --port");
    }

    #[test]
    fn an_option_without_its_value_is_refused() {
        let error = serve(&["--lease-ttl", "1s", "--listen"]).unwrap_err();
        assert_eq!(error, CliError::MissingValue("--listen".to_owned()));
        assert_eq!(error.to_string(), "--listen needs a value");
    }

    #[test]
    fn a_bad_address_is_refused() {
        assert!(matches!(
            bad("--listen", "localhost"),
            CliError::BadValue {
                expected: super::ADDRESS,
                ..
            }
        ));
    }

    #[test]
    fn a_duration_without_a_unit_number_or_room_is_refused() {
        for value in ["5", "ms", "-1s", "1.5s", "5d", "18446744073709551615h"] {
            assert!(
                matches!(
                    bad("--superseded-ttl", value),
                    CliError::BadValue {
                        expected: super::DURATION,
                        ..
                    }
                ),
                "{value}"
            );
        }
    }

    #[test]
    fn a_count_that_is_not_a_whole_u32_is_refused() {
        for value in ["-1", "two", "4294967296"] {
            assert!(
                matches!(
                    bad("--max-consecutive-expiries", value),
                    CliError::BadValue {
                        expected: super::COUNT,
                        ..
                    }
                ),
                "{value}"
            );
        }
    }

    #[test]
    fn a_configuration_the_invariants_reject_is_refused() {
        let error = bad("--max-pull-batch", "0");
        assert_eq!(error, CliError::Config(ConfigError::PullBatchNotPositive));
        assert_eq!(
            error.to_string(),
            "invalid configuration: max_pull_batch must be at least 1"
        );
        let error = bad("--max-consecutive-expiries", "0");
        assert_eq!(error, CliError::Config(ConfigError::ExpiryLimitNotPositive));
    }

    #[test]
    fn the_runtime_listen_output_and_serve_errors_describe_themselves() {
        let listen = CliError::Listen {
            address: "1.2.3.4:5".to_owned(),
            reason: "in use".to_owned(),
        };
        assert_eq!(listen.to_string(), "cannot listen on 1.2.3.4:5: in use");
        let runtime = CliError::Runtime("no threads".to_owned());
        assert_eq!(runtime.to_string(), "cannot start the runtime: no threads");
        let output = CliError::Output("closed".to_owned());
        assert_eq!(output.to_string(), "cannot write the output: closed");
        let serve = CliError::from(crate::ServeError::Task("gone".to_owned()));
        assert_eq!(serve.to_string(), "a service task failed: gone");
    }
}
