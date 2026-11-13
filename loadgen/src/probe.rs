//! The lapse probe: how long after a lease's `expires_at` another consumer claims
//! its pocket, under batch load, against a service started with a short lease and
//! sweep (`--lease-ttl-ms 2000 --sweep-ms 1000`).
//!
//! One run: the probe publishes one record into its own pocket while no other
//! pocket is queued, so that pocket holds the smallest position and keeps the
//! front on a lapse (`LeaseExpiryKeepsPocketAtFront`). Consumer A pulls it, and the
//! run fails unless A's pocket is the probe's: a queued pocket from elsewhere
//! would take the front. A reads `expires_at` from `GET /consumers/{id}/lease` and
//! never renews. Batch publishers then load other pockets. Consumer B pulls with
//! limit 0 every millisecond and releases at once any load pocket it claims, which
//! sends that pocket to the back, never ahead of the probe's. The lag is B's claim
//! instant, the system clock read when B's reply has been read whole, minus
//! `expires_at`; the service's clock is the system clock too, behind its monotonic
//! clamp, so on one machine the two compare to the millisecond.
//!
//! A run counts only if the load had counted a batch before `expires_at`, so the
//! lag is measured while the batch load runs; otherwise the run is an error.
//!
//! After the claim the load stops and the probe drains the queue, untimed: B
//! releases the probe pocket, and drain consumers ack every queued record until
//! none is left, which a fresh consumer's `no_pocket` confirms. That leaves no
//! queued pocket, so the next run's probe pocket is again the front.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Options;
use crate::client::{Client, Pull, zero_payload};
use crate::workloads::{Step, WhenEmpty, batch_publisher, consuming, prefix, timed};

/// How long after `expires_at` B waits for the claim before the run is an error.
const CLAIM_BOUND: Duration = Duration::from_secs(60);

/// How many consumers drain the queue after a run.
const DRAIN_CLIENTS: usize = 64;

/// The drain consumers' pull limit: the service's default `--max-pull-batch`.
const DRAIN_LIMIT: usize = 100;

/// One probe run: the lag in milliseconds, the claim and `expires_at` it is the
/// difference of, and the rows the load published; or the errors.
#[derive(Debug)]
pub(crate) struct Lapse {
    pub(crate) lag_ms: u64,
    pub(crate) expires_at: u64,
    pub(crate) claimed_at: u64,
    pub(crate) load_rows: u64,
}

/// Milliseconds since the Unix epoch on the system clock.
fn now_ms() -> Result<u64, String> {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("the system clock is before the epoch: {error}"))?;
    u64::try_from(since.as_millis()).map_err(|error| error.to_string())
}

/// Publishes the probe record and has A claim its pocket; returns `expires_at` as
/// `GET /consumers/{a}/lease` reads it.
fn claim_probe(client: &Client, run: usize, probe_pocket: &str) -> Result<u64, String> {
    let (publisher, a) = (
        format!("{}-probe", prefix(run)),
        format!("{}-a", prefix(run)),
    );
    client.join_publisher(&publisher)?;
    client.join_consumer(&a)?;
    client.publish(&publisher, probe_pocket, "probe", &zero_payload(1))?;
    match client.pull(&a, 1)? {
        Pull::Batch {
            pocket_key,
            expires_at,
            ..
        } if pocket_key == probe_pocket => {
            let read = client.lease_expires_at(&a)?;
            if read == expires_at {
                Ok(read)
            } else {
                Err(format!(
                    "A's lease read expires_at {read}, its pull {expires_at}"
                ))
            }
        }
        other @ (Pull::Batch { .. } | Pull::NoPocket) => Err(format!(
            "A's pull claimed {other:?}, not the probe pocket {probe_pocket}: another pocket was queued"
        )),
    }
}

/// B's loop: pull with limit 0 every millisecond, release any load pocket, and
/// return the instant B's reply named the probe pocket, with B's lease on it.
fn poll_for_probe(
    client: &Client,
    b: &str,
    probe_pocket: &str,
    expires_at: u64,
) -> Result<(u64, String), String> {
    let bound = expires_at + u64::try_from(CLAIM_BOUND.as_millis()).unwrap_or(u64::MAX);
    loop {
        match client.pull(b, 0)? {
            Pull::Batch {
                lease_id,
                pocket_key,
                ..
            } if pocket_key == probe_pocket => {
                let claimed_at = now_ms()?;
                if claimed_at > bound {
                    client.release(&lease_id, b)?;
                    return Err(format!(
                        "B claimed the probe pocket at {claimed_at}, more than {CLAIM_BOUND:?} after expires_at {expires_at}"
                    ));
                }
                return Ok((claimed_at, lease_id));
            }
            Pull::Batch { lease_id, .. } => client.release(&lease_id, b)?,
            Pull::NoPocket => {}
        }
        if now_ms()? > bound {
            return Err(format!(
                "B did not claim the probe pocket within {CLAIM_BOUND:?} of expires_at"
            ));
        }
        thread::sleep(Duration::from_millis(1));
    }
}

/// Acks every queued record with `DRAIN_CLIENTS` consumers, then checks that a
/// fresh consumer finds no pocket.
fn drain(options: &Options, run: usize) -> Result<(), Vec<String>> {
    let drainer = Options {
        rows: DRAIN_LIMIT,
        ..options.clone()
    };
    let (_, _, errors) = timed(DRAIN_CLIENTS, Duration::from_secs(3_600), |thread| {
        consuming(&drainer, run, thread, 0, WhenEmpty::Finish)
    });
    if !errors.is_empty() {
        return Err(errors
            .into_iter()
            .map(|error| format!("the drain: {error}"))
            .collect());
    }
    let client = Client::new(&options.base_url);
    let check = format!("{}-check", prefix(run));
    let checked = client
        .join_consumer(&check)
        .and_then(|()| match client.pull(&check, 0)? {
            Pull::NoPocket => Ok(()),
            other @ Pull::Batch { .. } => Err(format!(
                "after the drain a pocket was still queued: {other:?}"
            )),
        });
    checked.map_err(|error| vec![error])
}

/// The batch load: `--clients` publishers of `--rows` fresh rows each, until B
/// has claimed the probe pocket or given up. `loaded_at` records when the first
/// batch was counted, in milliseconds since the Unix epoch.
fn load_until(
    options: &Options,
    run: usize,
    claimed: &AtomicBool,
    loaded_at: &AtomicU64,
) -> (Vec<u64>, Duration, Vec<String>) {
    timed(options.clients, Duration::from_secs(3_600), |thread| {
        let mut next = batch_publisher(options, run, thread, "l")?;
        Ok(move || {
            if claimed.load(Ordering::SeqCst) {
                return Ok(Step::Done);
            }
            let step = next()?;
            if matches!(step, Step::Counted { .. }) {
                let _ =
                    loaded_at.compare_exchange(0, now_ms()?, Ordering::SeqCst, Ordering::SeqCst);
            }
            Ok(step)
        })
    })
}

/// One probe run.
pub(crate) fn lapse(options: &Options, run: usize) -> Result<Lapse, Vec<String>> {
    let client = Client::new(&options.base_url);
    let probe_pocket = format!("{}-probe", prefix(run));
    let expires_at = claim_probe(&client, run, &probe_pocket).map_err(|error| vec![error])?;
    let b = format!("{}-b", prefix(run));
    client.join_consumer(&b).map_err(|error| vec![error])?;
    let claimed = AtomicBool::new(false);
    let loaded_at = AtomicU64::new(0);
    let (polled, (counts, _, mut errors)) = thread::scope(|scope| {
        let poller = scope.spawn(|| {
            let result = poll_for_probe(&client, &b, &probe_pocket, expires_at);
            claimed.store(true, Ordering::SeqCst);
            result
        });
        let load = load_until(options, run, &claimed, &loaded_at);
        let joined = poller
            .join()
            .unwrap_or_else(|_| Err(String::from("B's thread panicked")));
        (joined, load)
    });
    let lapse = polled.and_then(|(claimed_at, lease_id)| {
        client.release(&lease_id, &b)?;
        let lag_ms = claimed_at
            .checked_sub(expires_at)
            .ok_or_else(|| format!("B claimed at {claimed_at}, before expires_at {expires_at}"))?;
        let loaded_at = loaded_at.load(Ordering::SeqCst);
        if counts[0] == 0 || loaded_at == 0 || loaded_at >= expires_at {
            return Err(format!(
                "the batch load had not started before expires_at {expires_at}: the lag was not measured under load"
            ));
        }
        Ok(Lapse {
            lag_ms,
            expires_at,
            claimed_at,
            load_rows: counts[0],
        })
    });
    if let Err(more) = drain(options, run) {
        errors.extend(more);
    }
    match lapse {
        Ok(lapse) if errors.is_empty() => Ok(lapse),
        Ok(_) => Err(errors),
        Err(error) => Err([vec![error], errors].concat()),
    }
}
