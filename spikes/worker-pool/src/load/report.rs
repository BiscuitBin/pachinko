//! One run's report: the counts, the latency percentiles, the pool's counters, what
//! the settled machines show, and the one table the binary prints.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::time::Duration as StdDuration;

use crate::invariants;
use crate::machine::Machine;
use crate::pocket::PocketMachine;
use crate::pool::{Outcome, PoolConfig};
use crate::state::{LeaseStatus, PocketStatus};

use super::LoadConfig;
use super::clients::Tally;
use super::host::Host;

/// A worker thread at or above this share of the wall time in step is named the
/// bottleneck.
const WORKER_BUSY: u64 = 80;
/// No worker thread at or above this share of the wall time in step means the limit
/// is outside the machines' steps: the pool's loop (routing, replies, channel
/// operations, allocation) or the clients, which step time alone cannot tell apart.
const OUTSIDE_STEPS_BUSY: u64 = 60;

/// Every invariant [`invariants::check`] evaluates over the settled machines, as the
/// preamble names them: the module's twenty by their names in
/// `docs/specs/pachinko.allium` and the spike's three `spike:` consistency checks.
/// Maintained by hand beside `invariants.rs`; a unit test holds the two equal.
///
/// ```
/// use worker_pool::load::INVARIANTS_CHECKED;
/// assert_eq!(INVARIANTS_CHECKED.len(), 23);
/// assert!(INVARIANTS_CHECKED.contains(&"OneActiveLeasePerPocket"));
/// ```
pub const INVARIANTS_CHECKED: [&str; 23] = [
    "EndedAfterGrant",
    "ExpiriesWithinLimit",
    "ExpiryLimitPositive",
    "GrantedBeforeExpiry",
    "HeldVersionNotNewer",
    "IdleHasNoRecords",
    "LeasedHasOneActiveLease",
    "NackedBeforeExpiry",
    "OneActiveLeasePerConsumer",
    "OneActiveLeasePerPocket",
    "OnePocketPerKey",
    "OneRecordPerKeyPerPocket",
    "OutstandingOnlyWhileLeased",
    "PocketPositionsBelowNext",
    "PullBatchPositive",
    "RecordPositionsBelowNext",
    "ShedBeforeExpiry",
    "UniquePocketPosition",
    "UniqueRecordPosition",
    "UnleasedHasNoActiveLease",
    "spike:NoProtocolViolations",
    "spike:OrderMatchesViews",
    "spike:OrderViewConsistent",
];

/// One run as the table prints it: a median of repeats, or the run itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunReport {
    /// Worker threads.
    pub workers: usize,
    /// Partitions.
    pub partitions: u32,
    /// Publisher threads.
    pub publishers: usize,
    /// Consumer threads.
    pub consumers: usize,
    /// Pockets the publishers spread over.
    pub pockets: u64,
    /// The lease TTL in milliseconds.
    pub lease_ttl_ms: u64,
    /// The superseded-record and dead-letter TTL in milliseconds.
    pub decay_ttl_ms: u64,
    /// The tick interval in milliseconds.
    pub tick_ms: u64,
    /// Each lane's depth.
    pub queue_capacity: usize,
    /// The window the clients measured over: from when the counting opened after the
    /// warm-up to the deadline, so at most the configured duration. The rates divide
    /// by it.
    pub window: StdDuration,
    /// The warm-up excluded from the counts.
    pub warm_up: StdDuration,
    /// How many runs the median was taken over.
    pub repeats: usize,
    /// The pool's wall time, warm-up and stop included.
    pub wall: StdDuration,
    /// Publishes in the window.
    pub publishes: u64,
    /// Pulls in the window, `NoPocketAvailable` replies included.
    pub pulls: u64,
    /// Acks and nacks in the window.
    pub settles: u64,
    /// Releases in the window.
    pub releases: u64,
    /// Pulls answered `NoPocketAvailable` in the window.
    pub empty_pulls: u64,
    /// Publish service latency, send returned to reply: p50 and p99 in microseconds.
    pub pub_service: (u64, u64),
    /// Pull service latency: p50 and p99 in microseconds.
    pub pull_service: (u64, u64),
    /// Ack and nack service latency: p50 and p99 in microseconds.
    pub settle_service: (u64, u64),
    /// Release service latency: p50 and p99 in microseconds.
    pub release_service: (u64, u64),
    /// Publish send-wait, the time blocked in `Handle::send`: p50 and p99 in microseconds.
    pub pub_wait: (u64, u64),
    /// Pull send-wait: p50 and p99 in microseconds.
    pub pull_wait: (u64, u64),
    /// The order machine's `RetryPull` count over the pool's life.
    pub retry_pulls: u64,
    /// Messages delivered to a machine on the same worker thread, summed over them.
    pub msgs_local: u64,
    /// Messages handed to another worker thread's lane, summed over the threads.
    pub msgs_channel: u64,
    /// The highest outbox high-water mark of any worker thread.
    pub outbox_hwm: u64,
    /// Ticks stepped, summed over the worker threads.
    pub ticks: u64,
    /// Each worker thread's time inside `step` as a percentage of the pool's wall time.
    pub busy: Vec<u64>,
    /// Leases with status `Expired` in the settled pocket machines.
    pub lapses: u64,
    /// Pockets with status `Parked` in the settled pocket machines.
    pub parks: u64,
    /// The invariants that did not hold at the end, by name and detail.
    pub failed_invariants: Vec<String>,
    /// The sum of every machine's `violations()`.
    pub violations: u64,
    /// Client inputs sent, over the whole run.
    pub requests: u64,
    /// Replies received, over the whole run.
    pub replies: u64,
    /// Calls that got no reply within the client's timeout, over the whole run.
    pub timed_out: u64,
    /// Publish replies by kind in the window: new pocket, new key, superseded, stale.
    pub published: (u64, u64, u64, u64),
    /// Publishes rejected in the window (`EqualRank`).
    pub rejected_publishes: u64,
    /// Replies of `Rejected(LeaseNotActive)` in the window: the stalled consumer's.
    pub lease_not_active: u64,
    /// How many of the repeats ended [`Self::is_unhealthy`], this run included when it
    /// did; `measure` counts them over every repeat, so a dropped repeat's failure
    /// still shows here. One for a single unhealthy run, zero for a healthy one.
    pub unhealthy_repeats: usize,
}

impl RunReport {
    /// A report with every count zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The highest busy percentage and the worker thread that has it.
    #[must_use]
    pub fn busiest(&self) -> (usize, u64) {
        self.busy
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(_, busy)| *busy)
            .unwrap_or((0, 0))
    }

    /// The mean busy percentage over the worker threads.
    #[must_use]
    pub fn mean_busy(&self) -> u64 {
        if self.busy.is_empty() {
            return 0;
        }
        self.busy.iter().sum::<u64>() / to_u64(self.busy.len())
    }

    /// Whether the run ended with every invariant holding and no protocol violation.
    #[must_use]
    pub const fn invariants_hold(&self) -> bool {
        self.failed_invariants.is_empty() && self.violations == 0
    }

    /// Whether the run ended unhealthy: an invariant failed, a machine saw a protocol
    /// violation, a call timed out, or a request went without a reply.
    ///
    /// ```
    /// use worker_pool::load::RunReport;
    /// let mut report = RunReport::new();
    /// assert!(!report.is_unhealthy());
    /// report.timed_out = 1;
    /// assert!(report.is_unhealthy());
    /// ```
    #[must_use]
    pub const fn is_unhealthy(&self) -> bool {
        !self.invariants_hold() || self.timed_out > 0 || self.requests != self.replies
    }

    /// The one line naming what limited the run, by the thresholds in the header.
    ///
    /// ```
    /// use worker_pool::load::RunReport;
    /// let mut report = RunReport::new();
    /// report.workers = 2;
    /// report.partitions = 4;
    /// report.queue_capacity = 8;
    /// report.busy = vec![97, 40];
    /// assert_eq!(
    ///     report.bottleneck(),
    ///     "worker thread 0 (order machine + 2 partitions) at 97 % busy"
    /// );
    /// ```
    #[must_use]
    pub fn bottleneck(&self) -> String {
        let (worker, max) = self.busiest();
        if self.outbox_hwm > to_u64(self.queue_capacity) {
            return format!(
                "channels (outbox high-water {} > capacity {})",
                self.outbox_hwm, self.queue_capacity
            );
        }
        if max >= WORKER_BUSY {
            return format!(
                "worker thread {worker} ({}) at {max} % busy",
                self.machines_of(worker)
            );
        }
        if max < OUTSIDE_STEPS_BUSY {
            return format!(
                "outside the machines' steps (no worker thread above {OUTSIDE_STEPS_BUSY} % \
                in step; max {max} %): the pool's loop or the clients, not told apart"
            );
        }
        format!("mixed (worker thread {worker} at {max} % busy, no outbox above capacity)")
    }

    /// The machines the pool gives worker thread `index`: the order machine on thread 0
    /// and partition `p` on thread `p % N`.
    fn machines_of(&self, index: usize) -> String {
        let workers = self.workers.max(1);
        let partitions = (0..self.partitions)
            .filter(|p| usize::try_from(*p).unwrap_or(usize::MAX) % workers == index)
            .count();
        let noun = if partitions == 1 {
            "partition"
        } else {
            "partitions"
        };
        if index == 0 {
            format!("order machine + {partitions} {noun}")
        } else {
            format!("{partitions} {noun}")
        }
    }
}

/// The `p`-th percentile (0 to 100) of samples sorted ascending, by nearest rank; zero
/// when there are no samples.
///
/// ```
/// use worker_pool::load::percentile;
/// let sorted = [10, 20, 30, 40, 50];
/// assert_eq!(percentile(&sorted, 50), 30);
/// assert_eq!(percentile(&sorted, 99), 50);
/// assert_eq!(percentile(&[], 50), 0);
/// ```
#[must_use]
pub fn percentile(sorted: &[u64], p: u64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let p = usize::try_from(p.min(100)).unwrap_or(100);
    let rank = (sorted.len() * p).div_ceil(100).saturating_sub(1);
    sorted.get(rank).copied().unwrap_or(0)
}

/// p50 and p99 in microseconds of nanosecond samples, sorting them.
fn summarise(samples: &mut [u64]) -> (u64, u64) {
    samples.sort_unstable();
    (
        percentile(samples, 50) / 1_000,
        percentile(samples, 99) / 1_000,
    )
}

/// A count per second over the window.
#[expect(clippy::cast_precision_loss, reason = "a rate for printing")]
fn per_second(count: u64, window: StdDuration) -> f64 {
    let secs = window.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    count as f64 / secs
}

fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn nanos(duration: StdDuration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// The report of one run: the clients' tally, the pool's counters and the settled
/// machines. The window is the one the driver measured over, not the configured
/// duration: the rates divide by it.
pub(super) fn build(
    load: &LoadConfig,
    pool: PoolConfig,
    outcome: Outcome<Tally>,
    wall: StdDuration,
) -> RunReport {
    let mut report = from_machines(&outcome);
    let stats = outcome.stats();
    let wall_nanos = nanos(wall).max(1);
    report.busy = stats
        .workers
        .iter()
        .map(|w| w.busy_nanos.saturating_mul(100) / wall_nanos)
        .collect();
    report.msgs_local = stats.workers.iter().map(|w| w.local_deliveries).sum();
    report.msgs_channel = stats.workers.iter().map(|w| w.channel_deliveries).sum();
    report.outbox_hwm = stats
        .workers
        .iter()
        .map(|w| w.outbox_high_water)
        .max()
        .unwrap_or(0);
    report.ticks = stats.workers.iter().map(|w| w.ticks).sum();
    report.workers = pool.worker_threads;
    report.partitions = pool.partitions;
    report.queue_capacity = pool.queue_capacity;
    report.tick_ms = pool.tick_interval.nanos() / 1_000_000;
    report.publishers = load.publishers;
    report.consumers = load.consumers;
    report.pockets = load.pockets;
    report.lease_ttl_ms = load.lease_ttl.nanos() / 1_000_000;
    report.decay_ttl_ms = load.decay_ttl.nanos() / 1_000_000;
    report.warm_up = load.warm_up;
    report.repeats = 1;
    report.wall = wall;
    from_tally(&mut report, outcome.into_result());
    report
}

/// The clients' counts, the real window and the latency percentiles, into the report.
fn from_tally(report: &mut RunReport, mut tally: Tally) {
    report.window = tally.window;
    report.pub_service = summarise(&mut tally.pub_service);
    report.pull_service = summarise(&mut tally.pull_service);
    report.settle_service = summarise(&mut tally.settle_service);
    report.release_service = summarise(&mut tally.release_service);
    report.pub_wait = summarise(&mut tally.pub_wait);
    report.pull_wait = summarise(&mut tally.pull_wait);
    report.publishes = tally.publishes;
    report.pulls = tally.pulls;
    report.settles = tally.settles;
    report.releases = tally.releases;
    report.empty_pulls = tally.empty_pulls;
    report.requests = tally.requests;
    report.replies = tally.replies;
    report.timed_out = tally.timed_out;
    report.published = tally.published;
    report.rejected_publishes = tally.rejected_publishes;
    report.lease_not_active = tally.lease_not_active;
}

/// What the settled machines show: the invariants, the violations, lapses, parks and
/// the retry count.
fn from_machines(outcome: &Outcome<Tally>) -> RunReport {
    let order = outcome.order();
    let pockets = outcome.pockets();
    let (lapses, parks) = lapses_and_parks(pockets);
    RunReport {
        retry_pulls: order.retry_pulls(),
        lapses,
        parks,
        failed_invariants: invariants::check(order, pockets)
            .into_iter()
            .map(|v| format!("{}: {}", v.invariant, v.detail))
            .collect(),
        violations: order.violations() + pockets.iter().map(Machine::violations).sum::<u64>(),
        ..RunReport::default()
    }
}

/// Leases that expired and pockets that are parked, over every pocket machine.
fn lapses_and_parks(pockets: &[PocketMachine]) -> (u64, u64) {
    let mut lapses = 0;
    let mut parks = 0;
    for machine in pockets {
        for (_, pocket) in machine.pockets() {
            lapses += to_u64(
                pocket
                    .leases
                    .iter()
                    .filter(|lease| lease.status == LeaseStatus::Expired)
                    .count(),
            );
            if pocket.status == PocketStatus::Parked {
                parks += 1;
            }
        }
    }
    (lapses, parks)
}

/// The columns and their widths; the first two are left-aligned.
const COLUMNS: [(&str, usize); 20] = [
    ("N,P", 5),
    ("clients", 7),
    ("pub/s", 8),
    ("pull/s", 8),
    ("ack+nack/s", 10),
    ("release/s", 9),
    ("p50/p99 pub (us)", 16),
    ("p50/p99 pull (us)", 17),
    ("p50/p99 ack+nack (us)", 21),
    ("p50/p99 release (us)", 20),
    ("send-wait p99 pub (us)", 22),
    ("retry pulls", 11),
    ("msgs local", 10),
    ("msgs channel", 12),
    ("outbox hwm", 10),
    ("ticks", 7),
    ("busy % (max/mean)", 17),
    ("lapses", 6),
    ("parks", 5),
    ("invariants", 10),
];

/// The table: the host facts, the load, the thresholds, the column header and one row
/// per report with its bottleneck line under it.
///
/// # Errors
///
/// The writer's.
pub fn format_table(reports: &[RunReport], host: &Host, out: &mut impl Write) -> io::Result<()> {
    write_preamble(reports, host, out)?;
    let titles: Vec<String> = COLUMNS.iter().map(|(t, _)| (*t).to_owned()).collect();
    write_cells(out, &titles)?;
    for report in reports {
        write_cells(out, &cells(report))?;
        writeln!(out, "    bottleneck: {}", report.bottleneck())?;
        writeln!(
            out,
            "    requests {} replies {} timed out: {}; publishes new-pocket {} new-key {} \
            superseded {} stale {} rejected {}; lease-not-active {}; empty pulls {}; \
            unhealthy repeats {} of {}; wall {:.2} s",
            report.requests,
            report.replies,
            report.timed_out,
            report.published.0,
            report.published.1,
            report.published.2,
            report.published.3,
            report.rejected_publishes,
            report.lease_not_active,
            report.empty_pulls,
            report.unhealthy_repeats,
            report.repeats,
            report.wall.as_secs_f64(),
        )?;
        for failed in &report.failed_invariants {
            writeln!(out, "    failed invariant: {failed}")?;
        }
    }
    Ok(())
}

/// The host facts, the load, the note and the thresholds.
fn write_preamble(reports: &[RunReport], host: &Host, out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "worker-pool spike on {}; {}; hw.ncpu {}; perf cores {}; efficiency cores {}; \
        available_parallelism {}; commit {}",
        host.os,
        host.chip,
        optional(host.ncpu),
        optional(host.perf_cores),
        optional(host.efficiency_cores),
        host.parallelism,
        host.commit,
    )?;
    if let Some(first) = reports.first() {
        writeln!(
            out,
            "load: {} pockets, lease_ttl {} ms, superseded/dead-letter ttl {} ms, tick {} ms, \
            queue capacity {}, {:.1} s measured after {:.1} s warm-up, median of {} run(s) \
            by pub/s; clients = publishers+consumers, closed loop",
            first.pockets,
            first.lease_ttl_ms,
            first.decay_ttl_ms,
            first.tick_ms,
            first.queue_capacity,
            first.window.as_secs_f64(),
            first.warm_up.as_secs_f64(),
            first.repeats,
        )?;
    }
    writeln!(
        out,
        "note: N worker threads + C clients oversubscribe the host; numbers compare \
        configurations on this host on this day and claim nothing else; a difference within \
        run-to-run noise (a few %) orders nothing, so a ranking is read per column, from \
        repeated runs"
    )?;
    writeln!(
        out,
        "bottleneck thresholds: channels when outbox hwm > queue capacity; a worker thread \
        when its busy >= {WORKER_BUSY} %; outside the machines' steps (the pool's loop or \
        the clients, not told apart) when no worker thread is above {OUTSIDE_STEPS_BUSY} %; \
        else mixed. busy % = step time only: time inside step / pool wall time (warm-up and \
        stop included; routing, replies, channel operations and allocation are outside step; \
        a worker thread yielding on a pending outbox is not busy). lapses = leases Expired \
        and parks = pockets Parked in the settled machines; retry pulls = the order \
        machine's counter; both over the whole pool life"
    )?;
    writeln!(
        out,
        "invariants checked at the end, over the settled machines, and every machine's \
        violations() == 0 (ok = all held): {}",
        INVARIANTS_CHECKED.join(", ")
    )
}

fn optional(value: Option<u32>) -> String {
    value.map_or_else(|| "unknown".to_owned(), |v| v.to_string())
}

/// One row's cells, in column order.
fn cells(report: &RunReport) -> Vec<String> {
    let (_, max) = report.busiest();
    vec![
        format!("{},{}", report.workers, report.partitions),
        format!("{}+{}", report.publishers, report.consumers),
        format!("{:.0}", per_second(report.publishes, report.window)),
        format!("{:.0}", per_second(report.pulls, report.window)),
        format!("{:.0}", per_second(report.settles, report.window)),
        format!("{:.0}", per_second(report.releases, report.window)),
        format!("{}/{}", report.pub_service.0, report.pub_service.1),
        format!("{}/{}", report.pull_service.0, report.pull_service.1),
        format!("{}/{}", report.settle_service.0, report.settle_service.1),
        format!("{}/{}", report.release_service.0, report.release_service.1),
        report.pub_wait.1.to_string(),
        report.retry_pulls.to_string(),
        report.msgs_local.to_string(),
        report.msgs_channel.to_string(),
        report.outbox_hwm.to_string(),
        report.ticks.to_string(),
        format!("{}/{}", max, report.mean_busy()),
        report.lapses.to_string(),
        report.parks.to_string(),
        if report.invariants_hold() {
            "ok".to_owned()
        } else {
            (to_u64(report.failed_invariants.len()) + report.violations).to_string()
        },
    ]
}

/// One line of cells padded to the column widths and joined by pipes.
fn write_cells(out: &mut impl Write, cells: &[String]) -> io::Result<()> {
    let mut line = String::new();
    for (index, ((_, width), cell)) in COLUMNS.iter().zip(cells).enumerate() {
        if index > 0 {
            line.push_str(" | ");
        }
        // A String's `write!` cannot fail; the result is discarded on purpose.
        if index < 2 {
            let _ = write!(line, "{cell:<width$}");
        } else {
            let _ = write!(line, "{cell:>width$}");
        }
    }
    writeln!(out, "{line}")
}

#[cfg(test)]
mod tests {
    use super::{Host, INVARIANTS_CHECKED, RunReport, format_table, percentile};

    #[test]
    fn percentile_on_a_small_vec_by_nearest_rank() {
        let sorted = [5, 10, 20, 30];
        assert_eq!(percentile(&sorted, 0), 5);
        assert_eq!(percentile(&sorted, 50), 10);
        assert_eq!(percentile(&sorted, 75), 20);
        assert_eq!(percentile(&sorted, 99), 30);
        assert_eq!(percentile(&sorted, 100), 30);
        assert_eq!(percentile(&[7], 99), 7);
        assert_eq!(percentile(&[], 99), 0);
    }

    fn hand_built() -> RunReport {
        RunReport {
            workers: 2,
            partitions: 4,
            publishers: 8,
            consumers: 8,
            pockets: 256,
            lease_ttl_ms: 200,
            decay_ttl_ms: 1000,
            tick_ms: 10,
            queue_capacity: 1024,
            window: std::time::Duration::from_secs(2),
            warm_up: std::time::Duration::from_secs(1),
            repeats: 3,
            wall: std::time::Duration::from_secs(3),
            publishes: 20_000,
            pulls: 4_000,
            settles: 16_000,
            releases: 3_000,
            pub_service: (50, 900),
            pull_service: (70, 1_200),
            settle_service: (40, 800),
            release_service: (20, 300),
            pub_wait: (1, 30),
            retry_pulls: 3,
            msgs_local: 100,
            msgs_channel: 200,
            outbox_hwm: 5,
            ticks: 600,
            busy: vec![97, 40],
            lapses: 4,
            parks: 1,
            ..RunReport::new()
        }
    }

    #[test]
    fn format_table_on_a_hand_built_report() {
        let mut out = Vec::new();
        let host = Host::new(8).with_perf_cores(Some(6));
        format_table(&[hand_built()], &host, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.is_ascii());
        assert!(text.contains("available_parallelism 8"));
        assert!(text.contains("perf cores 6"));
        assert!(text.contains("claim nothing else"));
        assert!(text.contains("| busy % (max/mean) |"));
        let row = text
            .lines()
            .find(|line| line.starts_with("2,4 "))
            .expect("the row");
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_eq!(
            cells,
            vec![
                "2,4", "8+8", "10000", "2000", "8000", "1500", "50/900", "70/1200", "40/800",
                "20/300", "30", "3", "100", "200", "5", "600", "97/68", "4", "1", "ok"
            ]
        );
        assert!(
            text.contains("unhealthy repeats 0 of 3; wall 3.00 s"),
            "{text}"
        );
        assert!(
            text.contains(
                "bottleneck: worker thread 0 (order machine + 2 partitions) at 97 % busy"
            )
        );
    }

    #[test]
    fn the_bottleneck_follows_the_thresholds_in_order() {
        let mut report = hand_built();
        report.outbox_hwm = 2_000;
        assert!(
            report
                .bottleneck()
                .starts_with("channels (outbox high-water 2000")
        );
        report.outbox_hwm = 0;
        report.busy = vec![30, 55];
        assert!(
            report
                .bottleneck()
                .starts_with("outside the machines' steps (no worker thread above 60 %")
        );
        report.busy = vec![30, 70];
        assert!(
            report
                .bottleneck()
                .starts_with("mixed (worker thread 1 at 70 %")
        );
        report.busy = vec![30, 85];
        assert_eq!(
            report.bottleneck(),
            "worker thread 1 (2 partitions) at 85 % busy"
        );
    }

    /// The names `invariants.rs` passes to `Violation::new`: every quoted literal
    /// that is an invariant name (`spike:` prefix, else a capital), the detail
    /// strings being lower-case words or single letters.
    fn names_in_invariants_source() -> Vec<&'static str> {
        let source = include_str!("../invariants.rs");
        let mut names: Vec<&str> = source
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|literal| {
                let name = literal.strip_prefix("spike:").unwrap_or(literal);
                name.len() > 1
                    && name.starts_with(|c: char| c.is_ascii_uppercase())
                    && name.chars().all(|c| c.is_ascii_alphabetic())
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    #[test]
    fn the_invariants_listed_are_the_ones_the_source_checks() {
        let mut listed = INVARIANTS_CHECKED.to_vec();
        listed.sort_unstable();
        assert_eq!(listed, names_in_invariants_source());
        let mut out = Vec::new();
        format_table(&[hand_built()], &Host::new(1), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("invariants checked at the end"), "{text}");
        assert!(
            text.contains("OnePocketPerKey, OneRecordPerKeyPerPocket"),
            "{text}"
        );
    }

    #[test]
    fn a_violation_shows_as_a_count_and_a_line() {
        let mut report = hand_built();
        report.violations = 2;
        report.failed_invariants = vec!["OneActiveLeasePerPocket: x".to_owned()];
        report.unhealthy_repeats = 2;
        assert!(report.is_unhealthy());
        let mut out = Vec::new();
        format_table(&[report], &Host::new(1), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        let row = text.lines().find(|line| line.starts_with("2,4 ")).unwrap();
        assert!(
            row.trim_end().ends_with("| 3") || row.trim_end().ends_with("  3"),
            "{row}"
        );
        assert!(text.contains("failed invariant: OneActiveLeasePerPocket: x"));
        assert!(text.contains("unhealthy repeats 2 of 3;"), "{text}");
    }

    #[test]
    fn unhealthy_is_a_failed_invariant_a_violation_a_timeout_or_a_lost_reply() {
        let healthy = hand_built();
        assert!(!healthy.is_unhealthy());
        let mut timed_out = hand_built();
        timed_out.timed_out = 1;
        assert!(timed_out.is_unhealthy());
        let mut lost = hand_built();
        lost.requests = 10;
        lost.replies = 9;
        assert!(lost.is_unhealthy());
        let mut violated = hand_built();
        violated.violations = 1;
        assert!(violated.is_unhealthy());
    }
}
