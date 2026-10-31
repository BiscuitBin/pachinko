//! The closed-loop clients: publishers that publish into a bounded set of pockets with
//! repeated keys and rising ranks, consumers that pull, ack most, nack some and
//! release, and one consumer that stalls past the lease so that lapses happen.
//!
//! Every client keeps its own tally and returns it when the deadline passes; the
//! driver sums them. Latency is a client-side `Instant` split in two: the time blocked
//! in `Handle::send` (back-pressure) and the time from the send's return to the reply
//! (service).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::pool::Handle;
use crate::protocol::{
    ConsumerId, HandedRecord, Input, LeaseId, Payload, PocketKey, Published, PublisherId, Rank,
    Reason, RecordKey, Response,
};

use super::LoadConfig;

/// How long a client waits for one reply before counting the call as timed out.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
/// Record keys per pocket.
const RECORD_KEYS: u64 = 8;
/// A pull's limit.
const PULL_LIMIT: u64 = 16;
/// The payload every publish carries.
const PAYLOAD_LEN: usize = 64;
/// One publish in this many repeats the previous rank minus one.
const STALE_EVERY: u64 = 16;
/// One handed record in this many is nacked.
const NACK_EVERY: u64 = 8;
/// One pull in this many skips the release so the next pull is within the lease.
const SKIP_RELEASE_EVERY: u64 = 4;
/// What a consumer sleeps when no pocket was available.
const EMPTY_PULL_PAUSE: Duration = Duration::from_millis(1);

/// A seeded linear congruential generator: deterministic across runs and hosts.
#[derive(Clone, Debug)]
pub(super) struct Lcg(u64);

impl Lcg {
    pub(super) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next value, from the high bits of the state.
    pub(super) const fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// The next value below `n` (at least 1).
    pub(super) const fn below(&mut self, n: u64) -> u64 {
        let n = if n == 0 { 1 } else { n };
        self.next() % n
    }
}

/// What every client shares: the deadline and whether the warm-up is over.
#[derive(Debug)]
pub(super) struct Clock {
    deadline: Instant,
    measuring: AtomicBool,
}

impl Clock {
    pub(super) const fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            measuring: AtomicBool::new(false),
        }
    }

    /// Whether an operation starting now counts: after the warm-up, before the deadline.
    fn counting(&self) -> bool {
        self.measuring.load(Ordering::Relaxed) && Instant::now() < self.deadline
    }

    fn over(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

/// What one client, or all of them summed, counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Tally {
    /// Publishes started in the window.
    pub(super) publishes: u64,
    /// Pulls started in the window.
    pub(super) pulls: u64,
    /// Acks and nacks started in the window.
    pub(super) settles: u64,
    /// Pulls in the window answered `NoPocketAvailable`.
    pub(super) empty_pulls: u64,
    /// Publish replies in the window by kind: new pocket, new key, superseded, stale.
    pub(super) published: (u64, u64, u64, u64),
    /// Publishes in the window rejected (`EqualRank`).
    pub(super) rejected_publishes: u64,
    /// `Rejected(LeaseNotActive)` replies in the window.
    pub(super) lease_not_active: u64,
    /// Every input sent, whole run.
    pub(super) requests: u64,
    /// Every reply received, whole run.
    pub(super) replies: u64,
    /// Every call with no reply within the timeout, whole run.
    pub(super) timed_out: u64,
    /// Publish send-wait samples in nanoseconds, window only.
    pub(super) pub_wait: Vec<u64>,
    /// Publish service samples.
    pub(super) pub_service: Vec<u64>,
    /// Pull send-wait samples.
    pub(super) pull_wait: Vec<u64>,
    /// Pull service samples.
    pub(super) pull_service: Vec<u64>,
}

impl Tally {
    /// Adds another client's tally to this one.
    pub(super) fn merge(&mut self, other: Self) {
        self.publishes += other.publishes;
        self.pulls += other.pulls;
        self.settles += other.settles;
        self.empty_pulls += other.empty_pulls;
        self.published.0 += other.published.0;
        self.published.1 += other.published.1;
        self.published.2 += other.published.2;
        self.published.3 += other.published.3;
        self.rejected_publishes += other.rejected_publishes;
        self.lease_not_active += other.lease_not_active;
        self.requests += other.requests;
        self.replies += other.replies;
        self.timed_out += other.timed_out;
        self.pub_wait.extend(other.pub_wait);
        self.pub_service.extend(other.pub_service);
        self.pull_wait.extend(other.pull_wait);
        self.pull_service.extend(other.pull_service);
    }

    /// Counts a reply, or its absence, whole run.
    const fn account(&mut self, reply: Option<&Response>) {
        self.requests += 1;
        if reply.is_some() {
            self.replies += 1;
        } else {
            self.timed_out += 1;
        }
        if let Some(Response::Rejected(Reason::LeaseNotActive)) = reply {
            self.lease_not_active += 1;
        }
    }
}

/// One call's result: the reply, if any, and the two latencies in nanoseconds.
#[derive(Debug)]
struct Call {
    reply: Option<Response>,
    wait: u64,
    service: u64,
}

/// Sends one input and waits for its reply, timing the send and the wait apart.
fn call(handle: &Handle, input: Input) -> Call {
    let started = Instant::now();
    let Ok(receiver) = handle.send(input) else {
        return Call {
            reply: None,
            wait: nanos(started.elapsed()),
            service: 0,
        };
    };
    let sent = Instant::now();
    let reply = receiver.recv_timeout(REPLY_TIMEOUT).ok();
    Call {
        reply,
        wait: nanos(sent.duration_since(started)),
        service: nanos(sent.elapsed()),
    }
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A publisher until the deadline: pockets and keys from the LCG, ranks rising per
/// (pocket, key) with the publisher's index folded in so that two publishers never
/// mint the same rank, and one publish in sixteen repeating the previous rank minus one.
pub(super) fn publisher(handle: &Handle, clock: &Clock, load: &LoadConfig, index: usize) -> Tally {
    let mut tally = Tally::default();
    let mut lcg = Lcg::new(load.seed ^ (to_u64(index) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut ranks: BTreeMap<(u64, u64), u64> = BTreeMap::new();
    let publishers = to_u64(load.publishers).max(1);
    let publisher = PublisherId::new(format!("p{index}"));
    let payload = Payload::new(vec![b'x'; PAYLOAD_LEN]);
    let mut n = 0u64;
    while !clock.over() {
        n += 1;
        let pocket = lcg.below(load.pockets);
        let key = lcg.below(RECORD_KEYS);
        let counter = ranks.entry((pocket, key)).or_insert(0);
        let rank = if n.is_multiple_of(STALE_EVERY) && *counter > 0 {
            (*counter * publishers + to_u64(index)).saturating_sub(1)
        } else {
            *counter += 1;
            *counter * publishers + to_u64(index)
        };
        let input = Input::Publish {
            publisher: publisher.clone(),
            pocket_key: PocketKey::new(format!("pocket-{pocket}")),
            record_key: RecordKey::new(format!("key-{key}")),
            rank: Rank::new(rank),
            payload: payload.clone(),
        };
        let counting = clock.counting();
        let outcome = call(handle, input);
        tally.account(outcome.reply.as_ref());
        if counting {
            tally.publishes += 1;
            tally.pub_wait.push(outcome.wait);
            tally.pub_service.push(outcome.service);
            count_published(&mut tally, outcome.reply.as_ref());
        }
    }
    tally
}

/// Counts a publish reply by kind.
const fn count_published(tally: &mut Tally, reply: Option<&Response>) {
    match reply {
        Some(Response::Published(Published::NewPocket)) => tally.published.0 += 1,
        Some(Response::Published(Published::NewKey)) => tally.published.1 += 1,
        Some(Response::Published(Published::Superseded)) => tally.published.2 += 1,
        Some(Response::Published(Published::Stale)) => tally.published.3 += 1,
        Some(Response::Rejected(_)) => tally.rejected_publishes += 1,
        _ => {}
    }
}

/// A consumer until the deadline: pull, settle each handed record, release three
/// pulls in four. Consumer 0 stalls three lease TTLs after every batch, so its lease
/// lapses and its actions are `Rejected(LeaseNotActive)`.
pub(super) fn consumer(handle: &Handle, clock: &Clock, load: &LoadConfig, index: usize) -> Tally {
    Consumer {
        handle,
        clock,
        id: ConsumerId::new(format!("c{index}")),
        lcg: Lcg::new(load.seed ^ (to_u64(index) + 1).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)),
        tally: Tally::default(),
        stall: (index == 0).then(|| load.lease_ttl.as_std() * 3),
    }
    .run()
}

/// One consumer's loop state.
struct Consumer<'a> {
    handle: &'a Handle,
    clock: &'a Clock,
    id: ConsumerId,
    lcg: Lcg,
    tally: Tally,
    stall: Option<Duration>,
}

impl Consumer<'_> {
    fn run(mut self) -> Tally {
        let mut pulls = 0u64;
        while !self.clock.over() {
            pulls += 1;
            match self.pull() {
                Some(Response::Batch { lease, records, .. }) => {
                    self.on_batch(lease, &records, pulls);
                }
                Some(Response::NoPocketAvailable) => std::thread::sleep(EMPTY_PULL_PAUSE),
                _ => {}
            }
        }
        self.tally
    }

    /// Stalls when this is the stalling consumer, settles the batch, and releases on
    /// every pull but the fourth.
    fn on_batch(&mut self, lease: LeaseId, records: &[HandedRecord], pulls: u64) {
        if let Some(stall) = self.stall {
            std::thread::sleep(stall);
        }
        self.settle(lease, records);
        if !pulls.is_multiple_of(SKIP_RELEASE_EVERY) {
            self.release(lease);
        }
    }

    /// One pull, its latencies recorded when it counts.
    fn pull(&mut self) -> Option<Response> {
        let counting = self.clock.counting();
        let input = Input::Pull {
            consumer: self.id.clone(),
            limit: PULL_LIMIT,
        };
        let outcome = call(self.handle, input);
        self.tally.account(outcome.reply.as_ref());
        if counting {
            self.tally.pulls += 1;
            self.tally.pull_wait.push(outcome.wait);
            self.tally.pull_service.push(outcome.service);
            if outcome.reply == Some(Response::NoPocketAvailable) {
                self.tally.empty_pulls += 1;
            }
        }
        outcome.reply
    }

    /// Acks seven handed records in eight and nacks the eighth.
    fn settle(&mut self, lease: LeaseId, records: &[HandedRecord]) {
        for record in records {
            let input = if self.lcg.below(NACK_EVERY) == 0 {
                Input::Nack {
                    consumer: self.id.clone(),
                    lease,
                    record_key: record.key.clone(),
                }
            } else {
                Input::Ack {
                    consumer: self.id.clone(),
                    lease,
                    record_key: record.key.clone(),
                }
            };
            let counting = self.clock.counting();
            let outcome = call(self.handle, input);
            self.tally.account(outcome.reply.as_ref());
            if counting {
                self.tally.settles += 1;
            }
        }
    }

    fn release(&mut self, lease: LeaseId) {
        let input = Input::Release {
            consumer: self.id.clone(),
            lease,
        };
        let outcome = call(self.handle, input);
        self.tally.account(outcome.reply.as_ref());
    }
}

/// Runs the publishers and consumers against the handle under a nested scope, opens
/// the counting window after the warm-up, and sums their tallies when they return.
pub(super) fn drive(handle: &Handle, load: &LoadConfig) -> Tally {
    let clock = &Clock::new(Instant::now() + load.warm_up + load.duration);
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for index in 0..load.publishers {
            threads.push(scope.spawn(move || publisher(handle, clock, load, index)));
        }
        for index in 0..load.consumers {
            threads.push(scope.spawn(move || consumer(handle, clock, load, index)));
        }
        std::thread::sleep(load.warm_up);
        clock.measuring.store(true, Ordering::SeqCst);
        let mut total = Tally::default();
        for thread in threads {
            total.merge(thread.join().unwrap_or_default());
        }
        total
    })
}

#[cfg(test)]
mod tests {
    use super::{Lcg, Tally};
    use crate::protocol::{Reason, Response};

    #[test]
    fn the_lcg_is_deterministic_and_bounded() {
        let mut a = Lcg::new(1);
        let mut b = Lcg::new(1);
        let first: Vec<u64> = (0..8).map(|_| a.next()).collect();
        let second: Vec<u64> = (0..8).map(|_| b.next()).collect();
        assert_eq!(first, second);
        assert!(first.windows(2).any(|w| w[0] != w[1]));
        let mut c = Lcg::new(2);
        assert_ne!(c.next(), first[0]);
        let mut d = Lcg::new(3);
        assert!((0..100).all(|_| d.below(5) < 5));
        assert_eq!(d.below(0), 0);
    }

    #[test]
    fn a_tally_accounts_replies_and_merges() {
        let mut a = Tally::default();
        a.account(Some(&Response::Acked));
        a.account(None);
        a.account(Some(&Response::Rejected(Reason::LeaseNotActive)));
        assert_eq!((a.requests, a.replies, a.timed_out), (3, 2, 1));
        assert_eq!(a.lease_not_active, 1);
        let mut b = Tally::default();
        b.account(Some(&Response::Acked));
        b.pub_wait.push(9);
        b.merge(a);
        assert_eq!((b.requests, b.replies, b.timed_out), (4, 3, 1));
        assert_eq!(b.pub_wait, vec![9]);
    }
}
