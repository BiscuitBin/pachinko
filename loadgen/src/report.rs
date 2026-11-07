//! What a run prints: one line per run with its rate and error count, or its errors
//! and no rate, and one median line once every run is clean.

use std::io::{self, Write};
use std::time::Duration;

/// One timed run: how many of each counted unit it completed, over how long, and
/// the errors its clients met.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    /// Each counted unit with its count, such as `("records", 1_000)`.
    pub(crate) counts: Vec<(&'static str, u64)>,
    /// From the ready barrier to the last client's join.
    pub(crate) elapsed: Duration,
    /// Every error, each printed; a run with any has no rate.
    pub(crate) errors: Vec<String>,
}

/// `count` per second over `elapsed`, in whole units, without a float.
pub(crate) fn per_second(count: u64, elapsed: Duration) -> u64 {
    let micros = elapsed.as_micros().max(1);
    u64::try_from(u128::from(count) * 1_000_000 / micros).unwrap_or(u64::MAX)
}

/// The lower median: the middle value of an odd count, the lower middle of an even
/// one; `None` for no values.
pub(crate) fn median(values: &[u64]) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len().checked_sub(1)? / 2).copied()
}

/// Writes run `run`'s line to `out`: its rate for each unit, or, when it has
/// errors, the count and each error, and no rate. Returns the rates, or `None` for
/// a run with errors.
pub(crate) fn run_line(
    out: &mut impl Write,
    run: usize,
    outcome: &Outcome,
) -> io::Result<Option<Vec<u64>>> {
    if !outcome.errors.is_empty() {
        writeln!(out, "run {run}: {} errors, no rate", outcome.errors.len())?;
        for error in &outcome.errors {
            writeln!(out, "  error: {error}")?;
        }
        return Ok(None);
    }
    let rates: Vec<u64> = outcome
        .counts
        .iter()
        .map(|&(_, count)| per_second(count, outcome.elapsed))
        .collect();
    let parts: Vec<String> = outcome
        .counts
        .iter()
        .zip(&rates)
        .map(|(&(unit, count), rate)| format!("{rate} {unit}/s ({count} {unit})"))
        .collect();
    writeln!(
        out,
        "run {run}: {} over {:.3} s, 0 errors",
        parts.join(", "),
        outcome.elapsed.as_secs_f64()
    )?;
    Ok(Some(rates))
}

/// Writes the median line over every run's rates, unit by unit.
pub(crate) fn median_line(
    out: &mut impl Write,
    units: &[&str],
    runs: &[Vec<u64>],
) -> io::Result<()> {
    let parts: Vec<String> = units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            let column: Vec<u64> = runs
                .iter()
                .filter_map(|rates| rates.get(index).copied())
                .collect();
            let value =
                median(&column).map_or_else(|| String::from("none"), |rate| rate.to_string());
            format!("{value} {unit}/s")
        })
        .collect();
    writeln!(out, "median of {} runs: {}", runs.len(), parts.join(", "))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Outcome, median, median_line, per_second, run_line};

    #[test]
    fn the_rate_is_per_second_of_the_elapsed_time() {
        assert_eq!(per_second(500, Duration::from_millis(250)), 2_000);
        assert_eq!(per_second(7, Duration::ZERO), 7_000_000);
    }

    #[test]
    fn the_median_is_the_lower_middle() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3, 1, 2]), Some(2));
        assert_eq!(median(&[4, 1, 3, 2]), Some(2));
    }

    #[test]
    fn a_run_with_an_error_prints_no_rate() {
        let outcome = Outcome {
            counts: vec![("records", 10)],
            elapsed: Duration::from_secs(1),
            errors: vec![String::from("refused")],
        };
        let mut out = Vec::new();
        assert_eq!(run_line(&mut out, 1, &outcome).unwrap(), None);
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text, "run 1: 1 errors, no rate\n  error: refused\n");
    }

    #[test]
    fn a_clean_run_prints_its_rate_and_the_median_line_follows() {
        let outcome = Outcome {
            counts: vec![("records", 10)],
            elapsed: Duration::from_secs(2),
            errors: Vec::new(),
        };
        let mut out = Vec::new();
        assert_eq!(run_line(&mut out, 2, &outcome).unwrap(), Some(vec![5]));
        median_line(&mut out, &["records"], &[vec![5], vec![9], vec![1]]).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            text,
            "run 2: 5 records/s (10 records) over 2.000 s, 0 errors\nmedian of 3 runs: 5 records/s\n"
        );
    }
}
