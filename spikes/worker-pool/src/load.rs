//! The synthetic load and the latency histograms the demo binary measures with.
//!
//! One run is a [`Pool::start`] around closed-loop clients: publishers publishing into
//! a bounded set of pockets with repeated keys and rising ranks, so that supersedes and
//! stale drops happen; consumers pulling, acking most, nacking some and releasing; and
//! one consumer stalling past the lease TTL so that lapses happen. The clients count
//! only after a warm-up, every latency is a client-side `Instant`, and when the body
//! returns the [`RunReport`] reads the pool's counters and the settled machines,
//! running the invariant check over them. [`measure`] repeats a run and keeps the
//! median by publishes a second; [`format_table`] prints one row per configuration
//! under the host facts.
//!
//! The service side of the spike, with the pool: it reaches threads, the clock and, in
//! [`Host::detect`], the operating system. Nothing in it is the implementation.

use std::fmt;
use std::time::{Duration as StdDuration, Instant};

use crate::pool::{Pool, PoolConfig, PoolError};
use crate::time::{Config, Duration};

mod clients;
mod host;
mod report;

pub use host::Host;
pub use report::{INVARIANTS_CHECKED, RunReport, format_table, percentile};

/// The load's shape: how many clients, how many pockets, the lease, and how long.
///
/// ```
/// use worker_pool::load::LoadConfig;
/// let load = LoadConfig::default().with_publishers(2).with_consumers(3);
/// assert_eq!((load.publishers, load.consumers), (2, 3));
/// assert_eq!(load.pockets, 256);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoadConfig {
    /// Publisher threads.
    pub publishers: usize,
    /// Consumer threads; consumer 0 stalls past the lease.
    pub consumers: usize,
    /// Pockets the publishers spread over.
    pub pockets: u64,
    /// The lease TTL the run's `Config` gets.
    pub lease_ttl: Duration,
    /// The superseded-record and dead-letter TTL the run's `Config` gets, so that the
    /// decays happen within the run instead of growing the machines for twelve hours.
    pub decay_ttl: Duration,
    /// The measured window.
    pub duration: StdDuration,
    /// The warm-up before it, excluded from the numbers.
    pub warm_up: StdDuration,
    /// The seed of every client's generator.
    pub seed: u64,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            publishers: 8,
            consumers: 8,
            pockets: 256,
            lease_ttl: Duration::from_millis(200),
            decay_ttl: Duration::from_millis(1_000),
            duration: StdDuration::from_secs(3),
            warm_up: StdDuration::from_secs(1),
            seed: 1,
        }
    }
}

impl LoadConfig {
    /// The same load with this many publishers.
    #[must_use]
    pub const fn with_publishers(mut self, publishers: usize) -> Self {
        self.publishers = publishers;
        self
    }

    /// The same load with this many consumers.
    #[must_use]
    pub const fn with_consumers(mut self, consumers: usize) -> Self {
        self.consumers = consumers;
        self
    }

    /// The same load over this many pockets, at least 1.
    #[must_use]
    pub const fn with_pockets(mut self, pockets: u64) -> Self {
        self.pockets = if pockets == 0 { 1 } else { pockets };
        self
    }

    /// The same load with this lease TTL.
    #[must_use]
    pub const fn with_lease_ttl(mut self, lease_ttl: Duration) -> Self {
        self.lease_ttl = lease_ttl;
        self
    }

    /// The same load with this decay TTL.
    #[must_use]
    pub const fn with_decay_ttl(mut self, decay_ttl: Duration) -> Self {
        self.decay_ttl = decay_ttl;
        self
    }

    /// The same load measured over this window.
    #[must_use]
    pub const fn with_duration(mut self, duration: StdDuration) -> Self {
        self.duration = duration;
        self
    }

    /// The same load with this warm-up.
    #[must_use]
    pub const fn with_warm_up(mut self, warm_up: StdDuration) -> Self {
        self.warm_up = warm_up;
        self
    }

    /// The same load with this seed.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// The same load with at least `parallelism` publishers and as many consumers, so
    /// that the closed-loop clients, each waiting on one reply at a time, number at
    /// least twice the host's parallelism and the count is not what the comparison
    /// turns on. A closed loop's throughput is its clients over their round trip, so
    /// the count alone caps nothing; the hand-back's client-scaling run shows whether
    /// more clients move the numbers.
    ///
    /// ```
    /// use worker_pool::load::LoadConfig;
    /// let load = LoadConfig::default().with_publishers(2).with_consumers(20);
    /// let raised = load.clone().with_clients_for(12);
    /// assert_eq!((raised.publishers, raised.consumers), (12, 20));
    /// let kept = load.with_clients_for(8);
    /// assert_eq!((kept.publishers, kept.consumers), (2, 20));
    /// ```
    #[must_use]
    pub fn with_clients_for(self, parallelism: usize) -> Self {
        if self.publishers + self.consumers >= 2 * parallelism {
            return self;
        }
        let publishers = self.publishers.max(parallelism);
        let consumers = self.consumers.max(parallelism);
        self.with_publishers(publishers).with_consumers(consumers)
    }
}

/// Why a run could not complete.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadError {
    /// The pool failed to start or lost a machine.
    Pool(PoolError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "the pool failed: {error}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<PoolError> for LoadError {
    fn from(error: PoolError) -> Self {
        Self::Pool(error)
    }
}

/// One run: the pool around the clients for the warm-up and the window, then the
/// report over the clients' tallies, the pool's counters and the settled machines.
/// The run's `Config` takes the load's lease and decay TTLs.
///
/// # Errors
///
/// The pool's.
pub fn run_once(
    load: &LoadConfig,
    pool: PoolConfig,
    config: Config,
) -> Result<RunReport, LoadError> {
    let config = config
        .with_lease_ttl(load.lease_ttl)
        .with_superseded_ttl(load.decay_ttl)
        .with_dead_letter_ttl(load.decay_ttl);
    let started = Instant::now();
    let outcome = Pool::start(config, pool, |handle| clients::drive(handle, load))?;
    let wall = started.elapsed();
    Ok(report::build(load, pool, outcome, wall))
}

/// `repeats` runs (at least one), keeping the whole report of the run whose publishes
/// a second are the median (the upper one of an even count), so that every column of
/// a row comes from one run. A run that failed an invariant, saw a protocol violation,
/// timed out or lost a reply is kept ahead of the median, and the kept report counts
/// every repeat that ended so in `unhealthy_repeats`, so no repeat's failure is
/// dropped.
///
/// # Errors
///
/// The first failing run's.
pub fn measure(
    load: &LoadConfig,
    pool: PoolConfig,
    config: Config,
    repeats: usize,
) -> Result<RunReport, LoadError> {
    let repeats = repeats.max(1);
    let mut reports = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        reports.push(run_once(load, pool, config)?);
    }
    Ok(select(reports, repeats))
}

/// The one report a row prints: the first [`RunReport::is_unhealthy`] one by publishes
/// a second when there is one, else the median by publishes a second (the upper one of
/// an even count).
/// `repeats` and the count of unhealthy repeats are written into it. Empty input yields
/// an empty report.
fn select(mut reports: Vec<RunReport>, repeats: usize) -> RunReport {
    if reports.is_empty() {
        return RunReport::new();
    }
    reports.sort_by(|a, b| {
        a.publishes_per_second()
            .total_cmp(&b.publishes_per_second())
    });
    let unhealthy_repeats = reports.iter().filter(|r| r.is_unhealthy()).count();
    let failing = reports.iter().position(RunReport::is_unhealthy);
    let mut chosen = reports.swap_remove(failing.unwrap_or(reports.len() / 2));
    chosen.repeats = repeats;
    chosen.unhealthy_repeats = unhealthy_repeats;
    chosen
}

/// The sweep for a host: `(1, 1)`, `(1, P)`, `(Nperf, P)` and `(N, P)` with `N` the
/// host's parallelism, `Nperf` its performance cores (`N` when unknown) and
/// `P = max(16, N)`, without repeats.
///
/// ```
/// use worker_pool::load::{Host, sweep_plan};
/// let host = Host::new(10).with_perf_cores(Some(4));
/// assert_eq!(sweep_plan(&host), vec![(1, 1), (1, 16), (4, 16), (10, 16)]);
/// ```
#[must_use]
pub fn sweep_plan(host: &Host) -> Vec<(usize, u32)> {
    let n = host.parallelism.max(1);
    let perf = host
        .perf_cores
        .and_then(|cores| usize::try_from(cores).ok())
        .filter(|cores| *cores >= 1)
        .unwrap_or(n);
    let p = u32::try_from(n.max(16)).unwrap_or(u32::MAX);
    let mut plan = vec![(1, 1), (1, p), (perf, p), (n, p)];
    plan.dedup();
    plan
}

#[cfg(test)]
mod tests {
    use super::{Host, LoadConfig, RunReport, measure, run_once, select, sweep_plan};
    use crate::pool::PoolConfig;
    use crate::time::{Config, Duration};
    use std::time::Duration as StdDuration;

    #[test]
    fn sweep_plan_with_a_fake_host() {
        let host = Host::new(18).with_perf_cores(Some(6));
        assert_eq!(sweep_plan(&host), vec![(1, 1), (1, 18), (6, 18), (18, 18)]);
        let unknown_cores = Host::new(8);
        assert_eq!(sweep_plan(&unknown_cores), vec![(1, 1), (1, 16), (8, 16)]);
        let one_core = Host::new(1).with_perf_cores(Some(1));
        assert_eq!(sweep_plan(&one_core), vec![(1, 1), (1, 16)]);
    }

    #[test]
    fn clients_are_raised_to_twice_the_parallelism() {
        let load = LoadConfig::default().with_clients_for(18);
        assert_eq!((load.publishers, load.consumers), (18, 18));
        let kept = LoadConfig::default().with_clients_for(8);
        assert_eq!((kept.publishers, kept.consumers), (8, 8));
        assert_eq!(LoadConfig::default().with_pockets(0).pockets, 1);
    }

    fn short_load() -> LoadConfig {
        LoadConfig::default()
            .with_publishers(2)
            .with_consumers(2)
            .with_pockets(16)
            .with_lease_ttl(Duration::from_millis(50))
            .with_decay_ttl(Duration::from_millis(100))
            .with_duration(StdDuration::from_millis(100))
            .with_warm_up(StdDuration::from_millis(50))
            .with_seed(7)
    }

    #[test]
    fn a_short_run_keeps_the_invariants_and_answers_every_request() {
        let pool = PoolConfig::default().with_tick_interval(Duration::from_millis(5));
        let load = short_load();
        let report = run_once(&load, pool, Config::default()).unwrap();
        assert!(
            report.window > StdDuration::ZERO && report.window <= load.duration,
            "the measured window {:?} is at most the configured {:?}",
            report.window,
            load.duration
        );
        assert!(report.invariants_hold(), "{:?}", report.failed_invariants);
        assert_eq!(report.violations, 0);
        assert!(report.publishes > 0);
        assert!(report.pulls > 0);
        assert_eq!(report.timed_out, 0);
        assert_eq!(
            report.requests, report.replies,
            "exactly one reply per request"
        );
        assert_eq!(report.lease_ttl_ms, 50);
        assert_eq!((report.workers, report.partitions), (1, 1));
        assert_eq!(report.busy.len(), 1);
    }

    #[test]
    fn measure_keeps_one_whole_run_and_counts_the_repeats() {
        let pool = PoolConfig::default().with_tick_interval(Duration::from_millis(5));
        let load = short_load().with_duration(StdDuration::from_millis(20));
        let report = measure(&load, pool, Config::default(), 2).unwrap();
        assert_eq!(report.repeats, 2);
        assert!(report.invariants_hold());
        assert!(!report.is_unhealthy());
        assert_eq!(report.unhealthy_repeats, 0);
    }

    /// A healthy report over a one-second window, so that its publishes a second equal
    /// its publishes.
    fn healthy(publishes: u64) -> RunReport {
        RunReport {
            publishes,
            requests: 10,
            replies: 10,
            window: StdDuration::from_secs(1),
            ..RunReport::new()
        }
    }

    #[test]
    fn select_takes_the_median_when_every_repeat_is_healthy() {
        let reports = vec![healthy(30), healthy(10), healthy(20)];
        let chosen = select(reports, 3);
        assert_eq!((chosen.publishes, chosen.repeats), (20, 3));
        assert_eq!(chosen.unhealthy_repeats, 0);
        let even = vec![healthy(10), healthy(40), healthy(20), healthy(30)];
        assert_eq!(select(even, 4).publishes, 30);
        assert_eq!(select(Vec::new(), 1), RunReport::new());
    }

    #[test]
    fn select_takes_the_median_by_publishes_a_second_not_by_count() {
        let over = |publishes: u64, millis: u64| RunReport {
            window: StdDuration::from_millis(millis),
            ..healthy(publishes)
        };
        // Rates 200/s, 166.7/s and 300/s: the median by rate is the 100-publish run;
        // by count it would have been the 150-publish one.
        let reports = vec![over(100, 500), over(150, 900), over(300, 1000)];
        assert_eq!(select(reports, 3).publishes, 100);
    }

    #[test]
    fn select_keeps_a_failed_repeat_ahead_of_the_median() {
        let mut timed_out = healthy(0);
        timed_out.timed_out = 1;
        timed_out.replies = 9;
        let chosen = select(vec![healthy(20), timed_out.clone(), healthy(30)], 3);
        assert_eq!((chosen.timed_out, chosen.repeats), (1, 3));
        assert_eq!(chosen.unhealthy_repeats, 1);
        let mut failed = healthy(40);
        failed.failed_invariants = vec!["OnePocketPerKey: x".to_owned()];
        let chosen = select(vec![healthy(20), healthy(30), failed], 3);
        assert_eq!(chosen.failed_invariants.len(), 1);
        let mut violated = healthy(25);
        violated.violations = 2;
        let chosen = select(vec![healthy(20), violated, healthy(30)], 3);
        assert_eq!(chosen.violations, 2);
        let chosen = select(vec![timed_out, failed_first_by_publishes()], 2);
        assert_eq!(chosen.publishes, 0, "the lowest failing run by publishes");
        assert_eq!(
            chosen.unhealthy_repeats, 2,
            "the dropped failure is counted"
        );
    }

    #[test]
    fn select_counts_every_unhealthy_repeat_not_only_the_shown_one() {
        let mut timed_out = healthy(10);
        timed_out.timed_out = 1;
        let mut failed = healthy(40);
        failed.failed_invariants = vec!["OnePocketPerKey: x".to_owned()];
        let chosen = select(vec![healthy(20), failed, timed_out, healthy(30)], 4);
        assert_eq!(
            (chosen.publishes, chosen.timed_out),
            (10, 1),
            "the first by publishes"
        );
        assert!(
            chosen.failed_invariants.is_empty(),
            "the other failure is not shown"
        );
        assert_eq!((chosen.unhealthy_repeats, chosen.repeats), (2, 4));
    }

    fn failed_first_by_publishes() -> RunReport {
        let mut failed = healthy(5);
        failed.violations = 1;
        failed
    }
}
