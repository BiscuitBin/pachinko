//! The closed-loop workloads: `publish`, `batch`, `consume` and `mix`.
//!
//! Each client is a std thread with its own [`Client`], so its own keep-alive
//! connection. A thread joins its publisher or consumer, then waits at the ready
//! barrier; the clock starts when every thread has reached it, before any thread
//! passes the go barrier, so the joins and any fill are outside the timed window
//! and no unit is counted before the clock starts. Each thread then sends, reads the whole
//! reply, checks it and counts it, and sends the next, until the window closes.
//! The first error stops the whole run: a run with any error has no rate.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Barrier, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::Options;
use crate::client::{Client, Pull, zero_payload};
use crate::report::Outcome;

/// What one step of a client's loop did.
#[derive(Debug)]
pub(crate) enum Step {
    /// Completed `count` units of the role numbered `role`.
    Counted { role: usize, count: u64 },
    /// The client has finished, with no error, and ends its loop.
    Done,
}

/// One client thread's tally over the window.
#[derive(Debug, Default)]
struct Tally {
    counts: [u64; 2],
    error: Option<String>,
}

/// This invocation's nonce: the process id and the system clock's nanoseconds
/// since the Unix epoch when it was first read, so that a reused process id
/// never repeats an earlier invocation's names.
fn nonce() -> &'static str {
    static NONCE: OnceLock<String> = OnceLock::new();
    NONCE.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        format!("{}-{nanos}", std::process::id())
    })
}

/// The ids every run's names carry: the invocation's nonce and the run number, so
/// that two runs, or two processes, against one service never collide.
pub(crate) fn prefix(run: usize) -> String {
    format!("lg{}-r{run}", nonce())
}

/// The next unit's place in the round-robin over the pockets, shared by every
/// thread of the process.
static NEXT_POCKET: AtomicU64 = AtomicU64::new(0);

/// The pocket the `index`th unit of a workload goes to, round-robin over the
/// `pockets`, named with the invocation's nonce.
pub(crate) fn pocket(index: u64, pockets: usize) -> String {
    let pockets = u64::try_from(pockets.max(1)).unwrap_or(1);
    format!("lg{}-p{}", nonce(), index % pockets)
}

/// The next pocket in the process's round-robin over the `pockets`.
pub(crate) fn next_pocket(pockets: usize) -> String {
    pocket(NEXT_POCKET.fetch_add(1, Ordering::Relaxed), pockets)
}

/// Runs `clients` threads, each set up by `setup` with its thread number, then
/// stepping in a closed loop until `window` closes, the first error, or every
/// client is done, and adds up what they counted.
pub(crate) fn timed<S, W>(
    clients: usize,
    window: Duration,
    setup: S,
) -> (Vec<u64>, Duration, Vec<String>)
where
    S: Fn(usize) -> Result<W, String> + Sync,
    W: FnMut() -> Result<Step, String>,
{
    let stop = AtomicBool::new(false);
    let running = AtomicUsize::new(clients);
    let gates = (Barrier::new(clients + 1), Barrier::new(clients + 1));
    thread::scope(|scope| {
        let threads: Vec<_> = (0..clients)
            .map(|thread| {
                let (setup, stop, gates, running) = (&setup, &stop, &gates, &running);
                scope.spawn(move || {
                    let tally = client_loop(setup(thread), gates, stop);
                    running.fetch_sub(1, Ordering::SeqCst);
                    tally
                })
            })
            .collect();
        gates.0.wait();
        let start = Instant::now();
        gates.1.wait();
        while !stop.load(Ordering::SeqCst)
            && running.load(Ordering::SeqCst) > 0
            && start.elapsed() < window
        {
            thread::sleep(Duration::from_millis(5).min(window.saturating_sub(start.elapsed())));
        }
        stop.store(true, Ordering::SeqCst);
        let tallies: Vec<Tally> = threads
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Tally::failed("a client thread panicked"))
            })
            .collect();
        let elapsed = start.elapsed();
        let mut counts = vec![0; 2];
        let mut errors = Vec::new();
        for tally in tallies {
            counts[0] += tally.counts[0];
            counts[1] += tally.counts[1];
            errors.extend(tally.error);
        }
        (counts, elapsed, errors)
    })
}

impl Tally {
    fn failed(error: &str) -> Self {
        Self {
            error: Some(error.to_owned()),
            ..Self::default()
        }
    }
}

/// One client thread: waits at the ready barrier and then the go barrier whatever
/// its setup did, so a failed setup never strands the others, then steps until
/// the stop.
fn client_loop<W>(
    worker: Result<W, String>,
    (ready, go): &(Barrier, Barrier),
    stop: &AtomicBool,
) -> Tally
where
    W: FnMut() -> Result<Step, String>,
{
    ready.wait();
    go.wait();
    let mut next = match worker {
        Ok(next) => next,
        Err(error) => {
            stop.store(true, Ordering::SeqCst);
            return Tally::failed(&error);
        }
    };
    let mut tally = Tally::default();
    while !stop.load(Ordering::SeqCst) {
        match next() {
            Ok(Step::Counted { role, count }) => tally.counts[role.min(1)] += count,
            Ok(Step::Done) => break,
            Err(error) => {
                stop.store(true, Ordering::SeqCst);
                tally.error = Some(error);
                break;
            }
        }
    }
    tally
}

/// The outcome of a timed run counted in `units`.
fn outcome(
    units: &[&'static str],
    (counts, elapsed, errors): (Vec<u64>, Duration, Vec<String>),
) -> Outcome {
    Outcome {
        counts: units.iter().copied().zip(counts).collect(),
        elapsed,
        errors,
    }
}

/// A publisher's client: joined under `{prefix}-t{thread}`.
fn publisher(options: &Options, run: usize, thread: usize) -> Result<(Client, String), String> {
    let client = Client::new(&options.base_url);
    let id = format!("{}-t{thread}", prefix(run));
    client.join_publisher(&id)?;
    Ok((client, id))
}

/// A consumer's client: joined under `{prefix}-c{thread}`.
fn consumer(options: &Options, run: usize, thread: usize) -> Result<(Client, String), String> {
    let client = Client::new(&options.base_url);
    let id = format!("{}-c{thread}", prefix(run));
    client.join_consumer(&id)?;
    Ok((client, id))
}

/// A step publishing one fresh record by `PUT`, counted under `role`.
fn single_publisher(
    options: &Options,
    run: usize,
    thread: usize,
    role: usize,
) -> Result<impl FnMut() -> Result<Step, String>, String> {
    let (client, id) = publisher(options, run, thread)?;
    let payload = zero_payload(options.payload_bytes);
    let pockets = options.pockets;
    let mut sequence = 0_u64;
    Ok(move || {
        sequence += 1;
        let key = format!("r{run}-t{thread}-{sequence}");
        client.publish(&id, &next_pocket(pockets), &key, &payload)?;
        Ok(Step::Counted { role, count: 1 })
    })
}

/// A step publishing one batch of `rows` fresh rows, counted in rows the reply
/// carried an applied outcome for. `tag` keeps the fill's keys apart from a timed
/// run's.
pub(crate) fn batch_publisher(
    options: &Options,
    run: usize,
    thread: usize,
    tag: &'static str,
) -> Result<impl FnMut() -> Result<Step, String>, String> {
    let (client, id) = publisher(options, run, thread)?;
    let payload = zero_payload(options.payload_bytes);
    let (pockets, rows) = (options.pockets, options.rows);
    let mut sequence = 0_u64;
    Ok(move || {
        let batch: Vec<(String, String)> = (0..rows)
            .map(|_| {
                sequence += 1;
                (
                    next_pocket(pockets),
                    format!("{tag}{run}-t{thread}-{sequence}"),
                )
            })
            .collect();
        let applied = client.publish_batch(&id, &batch, &payload)?;
        Ok(Step::Counted {
            role: 0,
            count: u64::try_from(applied).unwrap_or(0),
        })
    })
}

/// What a consumer does when a pull finds no pocket queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WhenEmpty {
    /// Sleep a millisecond and pull again (`mix`).
    Wait,
    /// End this consumer's loop (`consume` and the probe's drain).
    Finish,
}

/// A step that pulls up to `--rows`, acks each record back to back and releases,
/// counted in records acked under `role`; `when_empty` says what `no_pocket` does.
pub(crate) fn consuming(
    options: &Options,
    run: usize,
    thread: usize,
    role: usize,
    when_empty: WhenEmpty,
) -> Result<impl FnMut() -> Result<Step, String>, String> {
    let (client, id) = consumer(options, run, thread)?;
    let limit = options.rows;
    Ok(move || match client.pull(&id, limit)? {
        Pull::NoPocket => Ok(empty(when_empty, role)),
        Pull::Batch {
            lease_id,
            record_keys,
            ..
        } => {
            for key in &record_keys {
                client.ack(&lease_id, &id, key)?;
            }
            client.release(&lease_id, &id)?;
            Ok(Step::Counted {
                role,
                count: u64::try_from(record_keys.len()).unwrap_or(0),
            })
        }
    })
}

/// The step a `no_pocket` reply makes under `when_empty`.
fn empty(when_empty: WhenEmpty, role: usize) -> Step {
    match when_empty {
        WhenEmpty::Wait => {
            thread::sleep(Duration::from_millis(1));
            Step::Counted { role, count: 0 }
        }
        WhenEmpty::Finish => Step::Done,
    }
}

/// `publish`: single `PUT`s of fresh keys; records per second.
pub(crate) fn publish(options: &Options, run: usize) -> Outcome {
    let window = Duration::from_secs(options.seconds);
    outcome(
        &["records"],
        timed(options.clients, window, |thread| {
            single_publisher(options, run, thread, 0)
        }),
    )
}

/// `batch`: batches of `rows` fresh rows; rows per second.
pub(crate) fn batch(options: &Options, run: usize) -> Outcome {
    let window = Duration::from_secs(options.seconds);
    outcome(
        &["rows"],
        timed(options.clients, window, |thread| {
            batch_publisher(options, run, thread, "r")
        }),
    )
}

/// The untimed fill before a `consume` run: `--fill` fresh records in batches of
/// `fill_rows` from `FILL_CLIENTS` threads.
fn fill(options: &Options, run: usize) -> Result<(), Vec<String>> {
    const FILL_CLIENTS: usize = 8;
    const FILL_ROWS: usize = 1_000;
    let filler = Options {
        clients: FILL_CLIENTS,
        rows: FILL_ROWS,
        ..options.clone()
    };
    let batches = options.fill.div_ceil(FILL_ROWS * FILL_CLIENTS);
    let errors: Vec<String> = thread::scope(|scope| {
        let threads: Vec<_> = (0..FILL_CLIENTS)
            .map(|thread| {
                let filler = &filler;
                scope.spawn(move || fill_thread(filler, run, thread, batches))
            })
            .collect();
        threads
            .into_iter()
            .filter_map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Err(String::from("a fill thread panicked")))
                    .err()
            })
            .collect()
    });
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// One fill thread: `batches` batches from its own publisher.
fn fill_thread(filler: &Options, run: usize, thread: usize, batches: usize) -> Result<(), String> {
    let mut next = batch_publisher(filler, run, thread, "f")?;
    for _ in 0..batches {
        next()?;
    }
    Ok(())
}

/// `consume`: the fill, untimed, then consumers pulling, acking and releasing;
/// records acked per second.
pub(crate) fn consume(options: &Options, run: usize) -> Outcome {
    if let Err(errors) = fill(options, run) {
        return Outcome {
            errors: errors
                .into_iter()
                .map(|error| format!("the fill: {error}"))
                .collect(),
            ..Outcome::default()
        };
    }
    let window = Duration::from_secs(options.seconds);
    outcome(
        &["acked"],
        timed(options.clients, window, |thread| {
            consuming(options, run, thread, 0, WhenEmpty::Finish)
        }),
    )
}

/// `mix`: even threads publish single records, odd threads consume; records
/// published and records acked per second.
pub(crate) fn mix(options: &Options, run: usize) -> Outcome {
    let window = Duration::from_secs(options.seconds);
    let measured = timed(options.clients, window, |thread| {
        let mut publish = if thread % 2 == 0 {
            Some(single_publisher(options, run, thread, 0)?)
        } else {
            None
        };
        let mut consume = if thread % 2 == 1 {
            Some(consuming(options, run, thread, 1, WhenEmpty::Wait)?)
        } else {
            None
        };
        Ok(move || match (&mut publish, &mut consume) {
            (Some(step), _) => step(),
            (None, Some(step)) => step(),
            (None, None) => Err(String::from("a mix thread with no role")),
        })
    });
    outcome(&["published", "acked"], measured)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Step, pocket, timed};

    #[test]
    fn pockets_go_round_robin_under_the_invocation_nonce() {
        let id = super::nonce();
        assert_eq!(pocket(0, 3), format!("lg{id}-p0"));
        assert_eq!(pocket(4, 3), format!("lg{id}-p1"));
    }

    #[test]
    fn a_failed_setup_stops_the_run_with_its_error() {
        let (counts, _, errors) = timed(3, Duration::from_secs(5), |thread| {
            if thread == 1 {
                return Err(String::from("refused"));
            }
            Ok(|| {
                std::thread::sleep(Duration::from_millis(1));
                Ok(Step::Counted { role: 0, count: 1 })
            })
        });
        assert_eq!(errors, vec![String::from("refused")]);
        assert!(counts[0] < 1_000, "the run stopped early: {counts:?}");
    }

    /// Counts one unit a step until `left` reaches zero, then fails.
    fn countdown(left: &mut u32) -> Result<Step, String> {
        *left -= 1;
        if *left == 0 {
            return Err(String::from("404"));
        }
        Ok(Step::Counted { role: 0, count: 1 })
    }

    #[test]
    fn an_error_mid_run_is_counted_and_stops_the_run() {
        let (_, elapsed, errors) = timed(2, Duration::from_secs(5), |_| {
            let mut left = 3;
            Ok(move || countdown(&mut left))
        });
        assert!(
            !errors.is_empty() && errors.iter().all(|error| error == "404"),
            "{errors:?}"
        );
        assert!(elapsed < Duration::from_secs(5));
    }

    #[test]
    fn the_run_ends_when_every_client_is_done() {
        let (counts, elapsed, errors) = timed(2, Duration::from_secs(5), |_| Ok(|| Ok(Step::Done)));
        assert_eq!((counts, errors), (vec![0, 0], Vec::<String>::new()));
        assert!(elapsed < Duration::from_secs(5));
    }
}
