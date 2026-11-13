//! `pachinko-loadgen`: a closed-loop load generator and lapse probe that speaks
//! HTTP to a running `pachinko serve`, run by `just loadgen` and never by a gate
//! (decision 0017). It names no workspace crate.
//!
//! A run fails closed: a reply counts only when its status is 2xx and its body
//! parses and carries the outcome the workload expects; anything else is an error,
//! printed, and a run with any error prints no rate and makes the process exit
//! non-zero.

mod client;
mod probe;
mod report;
mod workloads;

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use report::{Outcome, median, median_line, run_line};

const USAGE: &str = "\
usage: pachinko-loadgen <workload> [--name value]...

workloads, each with its own default --clients (C) and --rows (b):
    publish     single PUT of fresh keys, C=64; records per second
    batch       POST /publishers/{id}/publish of b fresh rows, C=8, b=256; rows per second
    consume     untimed fill of --fill records, then pull b (100), ack each, release,
                C=64; records acked per second
    mix         half the clients publish single records, half pull b (100), ack each,
                release, C=64; both rates
    lapse       the lapse probe against a service started with --lease-ttl-ms 2000
                --sweep-ms 1000: batch load at b=256, C=8; lag p50 and max in ms

flags, each --name value:
    --base-url       http://127.0.0.1:8080
    --seconds        10, each run's window
    --runs           5
    --clients        the workload's C
    --rows           the workload's b
    --pockets        1000, the pockets keys spread over round-robin
    --payload-bytes  64
    --fill           1000000, the records a consume run's fill publishes
";

/// The flags, with each workload's defaults applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub(crate) base_url: String,
    pub(crate) seconds: u64,
    pub(crate) runs: usize,
    pub(crate) clients: usize,
    pub(crate) rows: usize,
    pub(crate) pockets: usize,
    pub(crate) payload_bytes: usize,
    pub(crate) fill: usize,
}

/// The workloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workload {
    Publish,
    Batch,
    Consume,
    Mix,
    Lapse,
}

impl Workload {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "publish" => Self::Publish,
            "batch" => Self::Batch,
            "consume" => Self::Consume,
            "mix" => Self::Mix,
            "lapse" => Self::Lapse,
            _ => return None,
        })
    }

    /// The workload's C and b.
    const fn defaults(self) -> (usize, usize) {
        match self {
            Self::Publish => (64, 1),
            Self::Batch | Self::Lapse => (8, 256),
            Self::Consume | Self::Mix => (64, 100),
        }
    }
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(Workload, Options),
}

fn number<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{name} takes a whole number, not {value:?}"))
}

/// Reads the command line after the program name.
fn parse(args: &[String]) -> Result<Command, String> {
    let Some(first) = args.first() else {
        return Err(String::from("no workload given"));
    };
    if matches!(first.as_str(), "--help" | "-h" | "help") {
        return Ok(Command::Help);
    }
    let workload = Workload::parse(first).ok_or_else(|| format!("no workload named {first:?}"))?;
    let (clients, rows) = workload.defaults();
    let mut options = Options {
        base_url: String::from("http://127.0.0.1:8080"),
        seconds: 10,
        runs: 5,
        clients,
        rows,
        pockets: 1_000,
        payload_bytes: 64,
        fill: 1_000_000,
    };
    for pair in args[1..].chunks(2) {
        let [name, value] = pair else {
            return Err(format!("{} needs a value", pair[0]));
        };
        match name.as_str() {
            "--base-url" => value.clone_into(&mut options.base_url),
            "--seconds" => options.seconds = number(name, value)?,
            "--runs" => options.runs = number(name, value)?,
            "--clients" => options.clients = number(name, value)?,
            "--rows" => options.rows = number(name, value)?,
            "--pockets" => options.pockets = number(name, value)?,
            "--payload-bytes" => options.payload_bytes = number(name, value)?,
            "--fill" => options.fill = number(name, value)?,
            _ => return Err(format!("no flag named {name:?}")),
        }
    }
    if options.runs == 0 || options.clients == 0 || options.rows == 0 || options.pockets == 0 {
        return Err(String::from(
            "--runs, --clients, --rows and --pockets must be at least 1",
        ));
    }
    Ok(Command::Run(workload, options))
}

/// Runs a rate workload `--runs` times, printing each run's line, and the median
/// once every run is clean. Stops at the first run with an error. Returns whether
/// every run was clean.
fn rates(
    out: &mut impl Write,
    options: &Options,
    units: &[&str],
    run: impl Fn(usize) -> Outcome,
) -> io::Result<bool> {
    let mut clean = Vec::new();
    for number in 1..=options.runs {
        match run_line(out, number, &run(number))? {
            Some(rates) => clean.push(rates),
            None => return Ok(false),
        }
    }
    median_line(out, units, &clean)?;
    Ok(true)
}

/// Runs the lapse probe `--runs` times, printing each run's lag, then p50 and max.
fn lapses(out: &mut impl Write, options: &Options) -> io::Result<bool> {
    let mut lags = Vec::new();
    for number in 1..=options.runs {
        match probe::lapse(options, number) {
            Ok(lapse) => {
                writeln!(
                    out,
                    "run {number}: lag {} ms (expires_at {}, claimed at {}), {} load rows, 0 errors",
                    lapse.lag_ms, lapse.expires_at, lapse.claimed_at, lapse.load_rows
                )?;
                lags.push(lapse.lag_ms);
            }
            Err(errors) => {
                writeln!(out, "run {number}: {} errors, no lag", errors.len())?;
                for error in errors {
                    writeln!(out, "  error: {error}")?;
                }
                return Ok(false);
            }
        }
    }
    let p50 = median(&lags).unwrap_or_default();
    let max = lags.iter().copied().max().unwrap_or_default();
    writeln!(
        out,
        "lapse lag over {} runs: p50 {p50} ms, max {max} ms",
        lags.len()
    )?;
    Ok(true)
}

/// Runs the command, writing its report to `out`. Returns whether it succeeded.
fn run(out: &mut impl Write, workload: Workload, options: &Options) -> io::Result<bool> {
    let window = match workload {
        Workload::Lapse => String::from("each until B's claim"),
        Workload::Consume => format!(
            "{:?} each after a fill of {}",
            Duration::from_secs(options.seconds),
            options.fill
        ),
        Workload::Publish | Workload::Batch | Workload::Mix => {
            format!("{:?} each", Duration::from_secs(options.seconds))
        }
    };
    writeln!(
        out,
        "{} against {}: C={} b={} P={} payload {} bytes, {} runs, {window}",
        format!("{workload:?}").to_lowercase(),
        options.base_url,
        options.clients,
        options.rows,
        options.pockets,
        options.payload_bytes,
        options.runs,
    )?;
    match workload {
        Workload::Publish => rates(out, options, &["records"], |run| {
            workloads::publish(options, run)
        }),
        Workload::Batch => rates(out, options, &["rows"], |run| {
            workloads::batch(options, run)
        }),
        Workload::Consume => rates(out, options, &["acked"], |run| {
            workloads::consume(options, run)
        }),
        Workload::Mix => rates(out, options, &["published", "acked"], |run| {
            workloads::mix(options, run)
        }),
        Workload::Lapse => lapses(out, options),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = io::stdout().lock();
    let succeeded = match parse(&args) {
        Ok(Command::Help) => out.write_all(USAGE.as_bytes()).is_ok(),
        Ok(Command::Run(workload, options)) => run(&mut out, workload, &options).unwrap_or(false),
        Err(error) => {
            let _ = writeln!(io::stderr(), "pachinko-loadgen: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Workload, parse};

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn each_workload_takes_its_own_defaults_and_flags_override_them() {
        let Ok(Command::Run(Workload::Batch, options)) = parse(&args("batch")) else {
            panic!("batch parses");
        };
        assert_eq!(
            (options.clients, options.rows, options.runs, options.seconds),
            (8, 256, 5, 10)
        );
        let Ok(Command::Run(Workload::Batch, options)) =
            parse(&args("batch --rows 1024 --clients 64"))
        else {
            panic!("batch parses");
        };
        assert_eq!((options.clients, options.rows), (64, 1_024));
        let Ok(Command::Run(Workload::Mix, options)) = parse(&args("mix")) else {
            panic!("mix parses");
        };
        assert_eq!((options.clients, options.rows), (64, 100));
        assert_eq!(parse(&args("--help")), Ok(Command::Help));
    }

    #[test]
    fn a_bad_command_line_is_refused() {
        for line in [
            "",
            "bogus",
            "publish --runs",
            "publish --runs x",
            "publish --nope 1",
            "publish --clients 0",
        ] {
            assert!(parse(&args(line)).is_err(), "{line}");
        }
    }
}
