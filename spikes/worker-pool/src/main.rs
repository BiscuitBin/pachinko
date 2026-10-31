//! The demo binary: the T20 measurement. With no arguments it runs the sweep the
//! host's core counts decide and prints one table; with arguments it runs one
//! configuration. `--help` lists the arguments, `--smoke` runs one 100 ms
//! configuration so that a test can see the table without waiting.

use std::fmt;
use std::io::Write;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use worker_pool::load::{self, Host, LoadConfig, LoadError, format_table, sweep_plan};
use worker_pool::pool::PoolConfig;
use worker_pool::{Config, Duration};

/// The usage, printed by `--help`.
const USAGE: &str = "\
usage: worker-pool [--workers N] [--partitions P] [--publishers A] [--consumers B]
    [--pockets K] [--lease-ttl-ms T] [--duration-s D] [--warm-up-ms W]
    [--repeats R] [--tick-ms I] [--queue-capacity Q] [--seed S]
or: worker-pool --smoke
or: worker-pool --help

With neither --workers nor --partitions: the sweep (1,1), (1,P), (Nperf,P), (N,P)
with N = available_parallelism, Nperf = the performance cores (N when unknown) and
P = max(16, N); publishers and consumers are each raised to N when together they
are below 2N. With either: one row; a missing count is 1.
Defaults: publishers 8, consumers 8, pockets 256, lease-ttl-ms 200, duration-s 3
(plus a 1 s warm-up excluded from the numbers), repeats 3 (the median by pub/s),
tick-ms 10, queue-capacity 1024, seed 1. --smoke: 1 worker, 1 partition, 2+2
clients, 100 ms after a 50 ms warm-up, lease 50 ms, 1 repeat.";

/// Why the binary could not run.
#[derive(Debug)]
enum Error {
    /// An argument was missing, unknown or not a number.
    Usage(String),
    /// A run failed.
    Load(LoadError),
    /// The output could not be written.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message} (see --help)"),
            Self::Load(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "could not write the output: {error}"),
        }
    }
}

impl From<LoadError> for Error {
    fn from(error: LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// The parsed arguments.
#[derive(Clone, Debug)]
struct Args {
    workers: Option<usize>,
    partitions: Option<u32>,
    load: LoadConfig,
    tick: Duration,
    queue_capacity: usize,
    repeats: usize,
    smoke: bool,
    help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            workers: None,
            partitions: None,
            load: LoadConfig::default(),
            tick: Duration::from_millis(10),
            queue_capacity: 1024,
            repeats: 3,
            smoke: false,
            help: false,
        }
    }
}

impl Args {
    /// The configurations to run and the load to run them under.
    fn plan(&self, host: &Host) -> (Vec<(usize, u32)>, LoadConfig, usize) {
        if self.smoke {
            let load = self
                .load
                .clone()
                .with_publishers(2)
                .with_consumers(2)
                .with_lease_ttl(Duration::from_millis(50))
                .with_decay_ttl(Duration::from_millis(100))
                .with_duration(StdDuration::from_millis(100))
                .with_warm_up(StdDuration::from_millis(50));
            return (vec![(1, 1)], load, 1);
        }
        match (self.workers, self.partitions) {
            (None, None) => (
                sweep_plan(host),
                self.load.clone().with_clients_for(host.parallelism),
                self.repeats,
            ),
            (workers, partitions) => (
                vec![(workers.unwrap_or(1), partitions.unwrap_or(1))],
                self.load.clone(),
                self.repeats,
            ),
        }
    }
}

/// Parses `--flag value` pairs and the two bare flags.
fn parse(args: &[String]) -> Result<Args, Error> {
    let mut parsed = Args::default();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--help" | "-h" => parsed.help = true,
            "--smoke" => parsed.smoke = true,
            _ => {
                let value = rest
                    .next()
                    .ok_or_else(|| Error::Usage(format!("{flag} needs a value")))?;
                apply(&mut parsed, flag, value)?;
            }
        }
    }
    Ok(parsed)
}

/// Applies one `--flag value` pair.
fn apply(args: &mut Args, flag: &str, value: &str) -> Result<(), Error> {
    let load = args.load.clone();
    match flag {
        "--workers" => args.workers = Some(number(flag, value)?),
        "--partitions" => args.partitions = Some(number(flag, value)?),
        "--publishers" => args.load = load.with_publishers(number(flag, value)?),
        "--consumers" => args.load = load.with_consumers(number(flag, value)?),
        "--pockets" => args.load = load.with_pockets(number(flag, value)?),
        "--lease-ttl-ms" => {
            args.load = load.with_lease_ttl(Duration::from_millis(number(flag, value)?));
        }
        "--duration-s" => args.load = load.with_duration(seconds(flag, value)?),
        "--warm-up-ms" => {
            args.load = load.with_warm_up(StdDuration::from_millis(number(flag, value)?));
        }
        "--repeats" => args.repeats = number(flag, value)?,
        "--tick-ms" => args.tick = Duration::from_millis(number(flag, value)?),
        "--queue-capacity" => args.queue_capacity = number(flag, value)?,
        "--seed" => args.load = load.with_seed(number(flag, value)?),
        _ => return Err(Error::Usage(format!("unknown argument {flag}"))),
    }
    Ok(())
}

/// A whole number argument.
fn number<T: FromStr>(flag: &str, value: &str) -> Result<T, Error> {
    value
        .parse()
        .map_err(|_| Error::Usage(format!("{flag} wants a whole number, not {value:?}")))
}

/// A duration argument in seconds, decimals allowed.
fn seconds(flag: &str, value: &str) -> Result<StdDuration, Error> {
    let secs: f64 = value
        .parse()
        .map_err(|_| Error::Usage(format!("{flag} wants a number of seconds, not {value:?}")))?;
    StdDuration::try_from_secs_f64(secs)
        .map_err(|_| Error::Usage(format!("{flag} wants a non-negative finite number")))
}

/// Parses the arguments, runs the plan and writes the table.
fn run(args: &[String], out: &mut impl Write) -> Result<(), Error> {
    let args = parse(args)?;
    if args.help {
        writeln!(out, "{USAGE}")?;
        return Ok(());
    }
    let host = Host::detect();
    let (plan, load, repeats) = args.plan(&host);
    let mut reports = Vec::with_capacity(plan.len());
    for (workers, partitions) in plan {
        let pool = PoolConfig::default()
            .with_worker_threads(workers)
            .with_partitions(partitions)
            .with_queue_capacity(args.queue_capacity)
            .with_tick_interval(args.tick);
        reports.push(load::measure(&load, pool, Config::default(), repeats)?);
    }
    format_table(&reports, &host, out)?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = std::io::stdout().lock();
    match run(&args, &mut out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // The lint bans eprintln!; a failure to report the failure has no remedy.
            let _ = writeln!(std::io::stderr(), "worker-pool: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, parse, run};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn smoke_prints_the_header_and_one_row() {
        let mut out = Vec::new();
        run(&args(&["--smoke"]), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.is_ascii());
        assert!(text.starts_with("worker-pool spike on "), "{text}");
        assert!(text.contains("| busy % (max/mean) |"), "{text}");
        let rows: Vec<&str> = text.lines().filter(|l| l.starts_with("1,1 ")).collect();
        assert_eq!(rows.len(), 1, "{text}");
        assert!(rows[0].contains("| 2+2 "), "{text}");
        assert!(rows[0].ends_with(" ok"), "{text}");
        assert!(text.contains("timed out: 0;"), "{text}");
    }

    #[test]
    fn help_prints_the_usage() {
        let mut out = Vec::new();
        run(&args(&["--help"]), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("usage: worker-pool"));
        assert!(text.contains("--queue-capacity"));
    }

    #[test]
    fn the_arguments_parse_and_refuse_nonsense() {
        let parsed = parse(&args(&[
            "--workers",
            "2",
            "--partitions",
            "4",
            "--publishers",
            "3",
            "--duration-s",
            "0.5",
            "--repeats",
            "1",
            "--seed",
            "9",
        ]))
        .unwrap();
        assert_eq!((parsed.workers, parsed.partitions), (Some(2), Some(4)));
        assert_eq!(parsed.load.publishers, 3);
        assert_eq!(parsed.load.duration, std::time::Duration::from_millis(500));
        assert_eq!((parsed.repeats, parsed.load.seed), (1, 9));
    }

    fn usage_error(list: &[&str]) -> String {
        match parse(&args(list)) {
            Err(Error::Usage(message)) => message,
            other => panic!("expected a usage error, got {other:?}"),
        }
    }

    #[test]
    fn nonsense_is_a_usage_error() {
        assert!(usage_error(&["--workers"]).contains("needs a value"));
        assert!(usage_error(&["--workers", "two"]).contains("whole number"));
        assert!(usage_error(&["--bogus", "1"]).contains("unknown argument"));
        assert!(usage_error(&["--duration-s", "-1"]).contains("non-negative"));
    }

    #[test]
    fn the_sweep_is_the_default_and_an_explicit_count_is_one_row() {
        let host = super::Host::new(4).with_perf_cores(Some(2));
        let (plan, load, repeats) = parse(&args(&[])).unwrap().plan(&host);
        assert_eq!(plan, vec![(1, 1), (1, 16), (2, 16), (4, 16)]);
        assert_eq!((load.publishers, load.consumers, repeats), (8, 8, 3));
        let (plan, _, _) = parse(&args(&["--workers", "2"])).unwrap().plan(&host);
        assert_eq!(plan, vec![(2, 1)]);
    }
}
