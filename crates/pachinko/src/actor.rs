//! The queue's owner: one tokio task holds the service's [`Queue`], and every action
//! reaches it as a [`Command`] on one bounded channel (decision 0019).
//!
//! The HTTP handlers send commands through a [`QueueHandle`] and wait on a `oneshot`
//! reply. The owner applies each command synchronously, in arrival order, with no
//! borrow of the queue held across an `.await` and no `Mutex`: a lapse and a
//! consumer's action are therefore totally ordered. A full channel makes the sender
//! wait, never refuses it.
//!
//! The sweep reaches the owner on a lane of its own (decision 0019): the
//! [`ticker`] sends one [`Tick`] every `--sweep-ms` on a channel that holds one,
//! and the owner polls that lane before the commands, so a tick waits behind at
//! most the command being applied, never behind a backlog. The ticker sends the
//! next tick only once the owner has answered the last and a full sweep has
//! passed, so the owner spends at most one tick per sweep and commands are never
//! starved. The tick applies [`Queue::tick`]: every lease whose `expires_at` has
//! passed lapses, and the superseded records and dead letters whose own has decay.
//!
//! A `Pull` whose requester has gone before the owner reaches it is skipped, so no
//! lease is granted to nobody. A requester that goes after that check, while the
//! owner applies the pull, keeps the lease until it lapses; the reply is then
//! dropped, as every unwanted reply is.
//!
//! A batch publish is one `PublishBatch` command, so no other command runs between
//! two of its rows. Its rows apply in order and independently: an equal rank refuses
//! its row alone, and any other refusal ends the batch where it stands, leaving the
//! rows before it applied. The only such refusal the library has, an unknown
//! publisher, comes at the first row, before any row lands.
//!
//! The owner reports [`Readiness::Serving`] as its first act, before it takes any
//! command or tick, and leaves its loop between them once the scope tells it to
//! stop.
//! Commands still in its inbox are then dropped unanswered, and their senders answer
//! 503 `not_ready`.

use std::time::Duration;

use libpachinko::clock::Timestamp;
use libpachinko::error::{JoinError, LeaseActionError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, PublisherId, RecordKey};
use libpachinko::lease_ids::LeaseId;
use libpachinko::queue::{
    Lease, LeaseView, PocketFate, Publication, PublishOutcome, PullOutcome, Queue, SettleOutcome,
};
use libpachinko::records::MemoryRecords;
use tokio::sync::{mpsc, oneshot, watch};

use crate::{EpochSerialLeaseIds, ServeError, SystemClock};

/// The service's queue: records in memory, the system clock and the epoch-serial
/// lease ids.
pub(crate) type ServiceQueue = Queue<MemoryRecords, SystemClock, EpochSerialLeaseIds>;

/// How many commands may wait for the owner before a sender waits too.
pub(crate) const COMMAND_CAPACITY: usize = 1024;

/// Where the owner sends one command's answer.
pub(crate) type Reply<T> = oneshot::Sender<T>;

/// One action for the owner, with the channel its answer goes back on. The sweep
/// travels on the tick lane instead, as a [`Tick`].
#[derive(Debug)]
pub(crate) enum Command {
    /// `PublisherJoins`.
    PublisherJoins {
        publisher_id: PublisherId,
        reply: Reply<Result<(), JoinError>>,
    },
    /// `ConsumerJoins`.
    ConsumerJoins {
        consumer_id: ConsumerId,
        reply: Reply<Result<(), JoinError>>,
    },
    /// `Publish` of one record.
    Publish {
        publisher_id: PublisherId,
        publication: Publication,
        reply: Reply<Result<PublishOutcome, PublishError>>,
    },
    /// `Publish` of each row in order, as one command (T27).
    PublishBatch {
        publisher_id: PublisherId,
        rows: Vec<Publication>,
        reply: Reply<Result<Vec<RowOutcome>, PublishError>>,
    },
    /// `Pull` of up to `limit` records; a `limit` of zero or less claims a pocket and
    /// hands out none.
    Pull {
        consumer_id: ConsumerId,
        limit: i64,
        reply: Reply<Result<PullOutcome, PullError>>,
    },
    /// The `Consuming` surface's `exposes` for one consumer: `None` when it holds no
    /// lease.
    LeaseView {
        consumer_id: ConsumerId,
        reply: Reply<Result<Option<LeaseView>, PullError>>,
    },
    /// `Ack` of `record_key` under `lease`.
    Ack {
        consumer_id: ConsumerId,
        lease: LeaseId,
        record_key: RecordKey,
        reply: Reply<Result<SettleOutcome, LeaseActionError>>,
    },
    /// `Nack` of `record_key` under `lease`.
    Nack {
        consumer_id: ConsumerId,
        lease: LeaseId,
        record_key: RecordKey,
        reply: Reply<Result<SettleOutcome, LeaseActionError>>,
    },
    /// `Renew` of `lease`: the answer is the new `expires_at`.
    Renew {
        consumer_id: ConsumerId,
        lease: LeaseId,
        reply: Reply<Result<Timestamp, LeaseActionError>>,
    },
    /// `Release` of `lease`: the answer is where the pocket went and the lease as
    /// it ended.
    Release {
        consumer_id: ConsumerId,
        lease: LeaseId,
        reply: Reply<Result<(PocketFate, Lease), LeaseActionError>>,
    },
}

/// One sweep for the owner: it applies [`Queue::tick`], then answers on `done`.
#[derive(Debug)]
pub(crate) struct Tick {
    done: Reply<()>,
}

/// The tick lane: a channel that holds one [`Tick`], so the ticker never has a
/// second one in flight.
pub(crate) fn tick_lane() -> (mpsc::Sender<Tick>, mpsc::Receiver<Tick>) {
    mpsc::channel(1)
}

/// The ticker: sleeps `sweep`, sends one [`Tick`] on `lane` and waits for the
/// owner's answer, then sleeps again, so the first tick comes one sweep after
/// start and two ticks are a full sweep of command service apart however long a
/// tick takes. It ends with `Ok` once the shutdown begins (`shutdown` turns true
/// or its sender is dropped), whether it is sleeping or waiting on the owner, and
/// when the owner has gone, so the lane is closed or the answer is dropped.
pub(crate) async fn ticker(
    lane: mpsc::Sender<Tick>,
    sweep: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), ServeError> {
    loop {
        tokio::select! {
            biased;
            _ = shutdown.wait_for(|shutdown| *shutdown) => return Ok(()),
            () = tokio::time::sleep(sweep) => {}
        }
        let (done, applied) = oneshot::channel();
        let ticked = async { lane.send(Tick { done }).await.is_ok() && applied.await.is_ok() };
        let owner_alive = tokio::select! {
            biased;
            _ = shutdown.wait_for(|shutdown| *shutdown) => return Ok(()),
            alive = ticked => alive,
        };
        if !owner_alive {
            return Ok(());
        }
    }
}

/// What one row of a batch publish came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowOutcome {
    /// The rule that took the row.
    Applied(PublishOutcome),
    /// The row carried the stored record's rank: the library's refusal, kept to its
    /// row (open question 1).
    EqualRank,
}

/// The owner has stopped, so a command could not be sent or its answer never came
/// back. The handlers answer 503 `not_ready`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Stopped {
    /// The owner task has ended, so its inbox or a reply channel is closed.
    #[error("the queue's owner has stopped")]
    Owner,
}

/// Whether the service is ready, as `GET /readyz` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readiness {
    /// The owner has not yet begun its loop.
    Starting,
    /// The owner is taking commands and shutdown has not started.
    Serving,
    /// Shutdown has started; in-flight requests may still finish.
    Stopping,
}

impl std::fmt::Display for Readiness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Starting => "starting",
            Self::Serving => "serving",
            Self::Stopping => "stopping",
        })
    }
}

/// A cloneable sender of commands to the queue's owner. Each method sends one
/// command and waits for its answer; the owner ends once every handle is dropped.
#[derive(Clone, Debug)]
pub(crate) struct QueueHandle {
    commands: mpsc::Sender<Command>,
}

impl QueueHandle {
    /// Sends the command `command` builds around a fresh reply channel, waiting for
    /// room when the channel is full, then waits for the answer.
    async fn ask<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> Result<T, Stopped> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_| Stopped::Owner)?;
        answer.await.map_err(|_| Stopped::Owner)
    }

    /// A handle whose commands wait in an inbox of `capacity` that no owner reads
    /// until the test runs one over it.
    #[cfg(test)]
    pub(crate) fn unowned(capacity: usize) -> (Self, mpsc::Receiver<Command>) {
        let (commands, inbox) = mpsc::channel(capacity);
        (Self { commands }, inbox)
    }

    /// `PublisherJoins` for `publisher_id`.
    pub(crate) async fn publisher_joins(
        &self,
        publisher_id: PublisherId,
    ) -> Result<Result<(), JoinError>, Stopped> {
        self.ask(|reply| Command::PublisherJoins {
            publisher_id,
            reply,
        })
        .await
    }

    /// `ConsumerJoins` for `consumer_id`.
    pub(crate) async fn consumer_joins(
        &self,
        consumer_id: ConsumerId,
    ) -> Result<Result<(), JoinError>, Stopped> {
        self.ask(|reply| Command::ConsumerJoins { consumer_id, reply })
            .await
    }

    /// `Publish` of `publication` by `publisher_id`.
    pub(crate) async fn publish(
        &self,
        publisher_id: PublisherId,
        publication: Publication,
    ) -> Result<Result<PublishOutcome, PublishError>, Stopped> {
        self.ask(|reply| Command::Publish {
            publisher_id,
            publication,
            reply,
        })
        .await
    }

    /// `Publish` of each of `rows` by `publisher_id`, in order, as one command.
    pub(crate) async fn publish_batch(
        &self,
        publisher_id: PublisherId,
        rows: Vec<Publication>,
    ) -> Result<Result<Vec<RowOutcome>, PublishError>, Stopped> {
        self.ask(|reply| Command::PublishBatch {
            publisher_id,
            rows,
            reply,
        })
        .await
    }

    /// `Pull` for `consumer_id`, up to `limit` records.
    pub(crate) async fn pull(
        &self,
        consumer_id: ConsumerId,
        limit: i64,
    ) -> Result<Result<PullOutcome, PullError>, Stopped> {
        self.ask(|reply| Command::Pull {
            consumer_id,
            limit,
            reply,
        })
        .await
    }

    /// The lease `consumer_id` holds, if any.
    pub(crate) async fn lease_view(
        &self,
        consumer_id: ConsumerId,
    ) -> Result<Result<Option<LeaseView>, PullError>, Stopped> {
        self.ask(|reply| Command::LeaseView { consumer_id, reply })
            .await
    }

    /// `Ack` of `record_key` under `lease` by `consumer_id`.
    pub(crate) async fn ack(
        &self,
        consumer_id: ConsumerId,
        lease: LeaseId,
        record_key: RecordKey,
    ) -> Result<Result<SettleOutcome, LeaseActionError>, Stopped> {
        self.ask(|reply| Command::Ack {
            consumer_id,
            lease,
            record_key,
            reply,
        })
        .await
    }

    /// `Nack` of `record_key` under `lease` by `consumer_id`.
    pub(crate) async fn nack(
        &self,
        consumer_id: ConsumerId,
        lease: LeaseId,
        record_key: RecordKey,
    ) -> Result<Result<SettleOutcome, LeaseActionError>, Stopped> {
        self.ask(|reply| Command::Nack {
            consumer_id,
            lease,
            record_key,
            reply,
        })
        .await
    }

    /// `Renew` of `lease` by `consumer_id`.
    pub(crate) async fn renew(
        &self,
        consumer_id: ConsumerId,
        lease: LeaseId,
    ) -> Result<Result<Timestamp, LeaseActionError>, Stopped> {
        self.ask(|reply| Command::Renew {
            consumer_id,
            lease,
            reply,
        })
        .await
    }

    /// `Release` of `lease` by `consumer_id`.
    pub(crate) async fn release(
        &self,
        consumer_id: ConsumerId,
        lease: LeaseId,
    ) -> Result<Result<(PocketFate, Lease), LeaseActionError>, Stopped> {
        self.ask(|reply| Command::Release {
            consumer_id,
            lease,
            reply,
        })
        .await
    }
}

/// The handle to `queue`'s owner and the owner itself, a future the caller spawns
/// into the task scope it owns. The owner holds `lease_ids`, a clone of the queue's
/// source, to see exhaustion before a pull, and takes the sweep's ticks from
/// `ticks`, the receiving end of a [`tick_lane`].
///
/// The owner sets `readiness` from [`Readiness::Starting`] to
/// [`Readiness::Serving`] as its first act, and ends with `Ok` once every handle is
/// dropped or `stop` turns true (or its sender is dropped), and with
/// [`ServeError::LeaseIdsExhausted`] at the first `Pull` after the lease-id source
/// is exhausted, rather than reissue an id.
pub(crate) fn start(
    queue: ServiceQueue,
    lease_ids: EpochSerialLeaseIds,
    readiness: watch::Sender<Readiness>,
    stop: watch::Receiver<bool>,
    ticks: mpsc::Receiver<Tick>,
) -> (
    QueueHandle,
    impl Future<Output = Result<(), ServeError>> + Send + 'static,
) {
    let (commands, inbox) = mpsc::channel(COMMAND_CAPACITY);
    let owner = async move {
        readiness.send_if_modified(|state| {
            let starting = *state == Readiness::Starting;
            if starting {
                *state = Readiness::Serving;
            }
            starting
        });
        own(queue, lease_ids, inbox, ticks, stop).await
    };
    (QueueHandle { commands }, owner)
}

/// The owner loop: applies each command in arrival order until every command
/// sender is gone or `stop` turns true, and each tick as it arrives. The stop is
/// checked first, so the owner takes nothing once told to stop; then the tick lane,
/// so a tick waits behind at most the command being applied; then the commands.
/// Each is applied synchronously, so the stop is only seen between them. Once the
/// ticker has gone its lane yields `None` once and is not polled again, so a closed
/// lane never spins the loop. The owner yields after each command or tick, so that
/// on a runtime with one worker the scope's grace timer runs and its stop lands
/// before the next, rather than after the owner's whole cooperative budget. Returning, or
/// being aborted, drops the queue, which releases its state without a yield, in
/// time proportional to its size: the shutdown's bound is two grace periods plus the
/// one command or tick the owner was applying, and releasing the queue's state when
/// the owner ends.
async fn own(
    mut queue: ServiceQueue,
    lease_ids: EpochSerialLeaseIds,
    mut inbox: mpsc::Receiver<Command>,
    mut ticks: mpsc::Receiver<Tick>,
    mut stop: watch::Receiver<bool>,
) -> Result<(), ServeError> {
    let mut ticks_open = true;
    loop {
        tokio::select! {
            biased;
            _ = stop.wait_for(|stop| *stop) => return Ok(()),
            tick = ticks.recv(), if ticks_open => match tick {
                Some(tick) => sweep(&mut queue, tick),
                None => ticks_open = false,
            },
            command = inbox.recv() => match command {
                Some(command) => apply(&mut queue, &lease_ids, command)?,
                None => return Ok(()),
            },
        }
        tokio::task::yield_now().await;
    }
}

/// Applies one tick and answers it. A tick that lapsed or decayed anything logs
/// one `info` line with the three counts, never one line per lease.
fn sweep(queue: &mut ServiceQueue, Tick { done }: Tick) {
    let report = queue.tick();
    if !report.is_empty() {
        tracing::info!(
            lapsed = report.lapsed.len(),
            decayed_superseded = report.decayed_superseded,
            decayed_dead_letters = report.decayed_dead_letters,
            "tick"
        );
    }
    answer(done, ());
}

/// Runs one command against the queue and sends its answer.
fn apply(
    queue: &mut ServiceQueue,
    lease_ids: &EpochSerialLeaseIds,
    command: Command,
) -> Result<(), ServeError> {
    match command {
        Command::PublisherJoins {
            publisher_id,
            reply,
        } => answer(reply, queue.publisher_joins(publisher_id)),
        Command::ConsumerJoins { consumer_id, reply } => {
            answer(reply, queue.consumer_joins(consumer_id));
        }
        Command::Publish {
            publisher_id,
            publication,
            reply,
        } => answer(reply, queue.publish(&publisher_id, publication)),
        Command::PublishBatch {
            publisher_id,
            rows,
            reply,
        } => answer(reply, publish_each(queue, &publisher_id, rows)),
        Command::Pull {
            consumer_id,
            limit,
            reply,
        } => return pull(queue, lease_ids, &consumer_id, limit, reply),
        Command::LeaseView { consumer_id, reply } => {
            answer(reply, queue.lease_view(&consumer_id));
        }
        Command::Ack {
            consumer_id,
            lease,
            record_key,
            reply,
        } => answer(reply, queue.ack(&consumer_id, lease, &record_key)),
        Command::Nack {
            consumer_id,
            lease,
            record_key,
            reply,
        } => answer(reply, queue.nack(&consumer_id, lease, &record_key)),
        Command::Renew {
            consumer_id,
            lease,
            reply,
        } => answer(reply, queue.renew(&consumer_id, lease)),
        Command::Release {
            consumer_id,
            lease,
            reply,
        } => answer(reply, queue.release(&consumer_id, lease)),
    }
    Ok(())
}

/// Applies a `Pull`, unless its requester has gone, in which case no lease is
/// granted to nobody.
///
/// # Errors
///
/// [`ServeError::LeaseIdsExhausted`] when the lease-id source is exhausted: the
/// owner stops rather than grant a lease under an id it already issued.
fn pull(
    queue: &mut ServiceQueue,
    lease_ids: &EpochSerialLeaseIds,
    consumer_id: &ConsumerId,
    limit: i64,
    reply: Reply<Result<PullOutcome, PullError>>,
) -> Result<(), ServeError> {
    if lease_ids.exhausted() {
        return Err(ServeError::LeaseIdsExhausted);
    }
    if reply.is_closed() {
        tracing::debug!(consumer = %consumer_id, "skipping a pull whose requester has gone");
        return Ok(());
    }
    answer(reply, queue.pull(consumer_id, limit));
    Ok(())
}

/// Publishes each of `rows` in order. An equal rank refuses its row alone; any other
/// refusal ends the batch, leaving the rows before it applied, as PR 38's
/// `publish_each` did. An unknown publisher is refused at the first row, before any
/// row has landed.
fn publish_each(
    queue: &mut ServiceQueue,
    publisher_id: &PublisherId,
    rows: Vec<Publication>,
) -> Result<Vec<RowOutcome>, PublishError> {
    rows.into_iter()
        .map(|row| match queue.publish(publisher_id, row) {
            Ok(outcome) => Ok(RowOutcome::Applied(outcome)),
            Err(PublishError::EqualRank { .. }) => Ok(RowOutcome::EqualRank),
            Err(error) => Err(error),
        })
        .collect()
}

/// Sends `value` back. A requester that has gone no longer wants it, so a failed
/// send is dropped.
fn answer<T>(reply: Reply<T>, value: T) {
    let _unwanted = reply.send(value);
}

#[cfg(test)]
mod tests {
    use std::future::{Future, poll_fn};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Poll;
    use std::time::Duration;

    use libpachinko::config::Config;
    use libpachinko::error::{JoinError, PublishError, PullError};
    use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use libpachinko::queue::{Publication, PublishOutcome, PullOutcome, Queue};
    use tokio::sync::mpsc::error::TrySendError;
    use tokio::sync::{mpsc, oneshot, watch};

    use super::{
        Command, QueueHandle, Readiness, RowOutcome, ServiceQueue, Stopped, Tick, own,
        publish_each, start, tick_lane, ticker,
    };
    use crate::{EpochSerialLeaseIds, ServeError, SystemClock};

    /// How long a test waits for the owner before it fails.
    const BOUND: Duration = Duration::from_secs(5);

    fn queue(lease_ids: &EpochSerialLeaseIds) -> ServiceQueue {
        Queue::new(
            Config::default(),
            SystemClock::new().unwrap(),
            lease_ids.clone(),
        )
        .unwrap()
    }

    /// A tick lane whose ticker has already gone.
    fn no_ticks() -> mpsc::Receiver<Tick> {
        tick_lane().1
    }

    fn publication(pocket: &str) -> Publication {
        Publication::new(
            PocketKey::new(pocket),
            RecordKey::new("k"),
            Rank::new(1),
            Payload::from(&b"x"[..]),
        )
    }

    /// Queues `commands` on a channel, closes it, and runs the owner over them to its
    /// end, so the order the owner sees them in is the order given.
    async fn run_owner(
        lease_ids: EpochSerialLeaseIds,
        commands: Vec<Command>,
    ) -> Result<(), ServeError> {
        let (sender, inbox) = mpsc::channel(commands.len().max(1));
        for command in commands {
            sender.try_send(command).unwrap();
        }
        drop(sender);
        let (_stop, never) = watch::channel(false);
        let owner = own(queue(&lease_ids), lease_ids, inbox, no_ticks(), never);
        tokio::time::timeout(BOUND, owner).await.unwrap()
    }

    /// The setup commands: publisher `p`, consumers `c1` and `c2`, and one record
    /// published into each of `pockets`.
    fn setup(pockets: &[&str]) -> Vec<Command> {
        let mut commands = vec![
            Command::PublisherJoins {
                publisher_id: PublisherId::new("p"),
                reply: oneshot::channel().0,
            },
            Command::ConsumerJoins {
                consumer_id: ConsumerId::new("c1"),
                reply: oneshot::channel().0,
            },
            Command::ConsumerJoins {
                consumer_id: ConsumerId::new("c2"),
                reply: oneshot::channel().0,
            },
        ];
        commands.extend(pockets.iter().map(|pocket| Command::Publish {
            publisher_id: PublisherId::new("p"),
            publication: publication(pocket),
            reply: oneshot::channel().0,
        }));
        commands
    }

    fn pull(consumer: &str) -> (Command, oneshot::Receiver<Result<PullOutcome, PullError>>) {
        let (reply, answer) = oneshot::channel();
        let command = Command::Pull {
            consumer_id: ConsumerId::new(consumer),
            limit: 10,
            reply,
        };
        (command, answer)
    }

    fn pocket_of(outcome: Result<PullOutcome, PullError>) -> String {
        match outcome {
            Ok(PullOutcome::Batch { pocket_key, .. }) => pocket_key.as_str().to_owned(),
            other => panic!("expected a batch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_pull_whose_requester_has_gone_is_skipped() {
        let mut commands = setup(&["pk"]);
        let (gone, answer) = pull("c1");
        drop(answer);
        let (second, second_answer) = pull("c2");
        commands.extend([gone, second]);
        run_owner(EpochSerialLeaseIds::starting_at(1), commands)
            .await
            .unwrap();
        assert_eq!(pocket_of(second_answer.await.unwrap()), "pk");
    }

    #[tokio::test]
    async fn the_owner_applies_commands_in_arrival_order() {
        let (early, early_answer) = pull("c");
        let (join, join_answer) = oneshot::channel();
        let (late, late_answer) = pull("c");
        let (again, again_answer) = oneshot::channel();
        let commands = vec![
            early,
            Command::ConsumerJoins {
                consumer_id: ConsumerId::new("c"),
                reply: join,
            },
            late,
            Command::ConsumerJoins {
                consumer_id: ConsumerId::new("c"),
                reply: again,
            },
        ];
        run_owner(EpochSerialLeaseIds::starting_at(1), commands)
            .await
            .unwrap();
        assert_eq!(
            early_answer.await.unwrap(),
            Err(PullError::UnknownConsumer(ConsumerId::new("c")))
        );
        assert_eq!(join_answer.await.unwrap(), Ok(()));
        assert_eq!(
            late_answer.await.unwrap(),
            Ok(PullOutcome::NoPocketAvailable)
        );
        assert_eq!(
            again_answer.await.unwrap(),
            Err(JoinError::ConsumerExists(ConsumerId::new("c")))
        );
    }

    #[tokio::test]
    async fn an_exhausted_source_grants_its_last_id_then_stops_the_owner() {
        let mut commands = setup(&["a", "b"]);
        let (first, first_answer) = pull("c1");
        let (second, second_answer) = pull("c2");
        commands.extend([first, second]);
        let ended = run_owner(EpochSerialLeaseIds::starting_at(u64::MAX), commands).await;
        assert_eq!(ended, Err(ServeError::LeaseIdsExhausted));
        assert_eq!(pocket_of(first_answer.await.unwrap()), "a");
        assert!(
            second_answer.await.is_err(),
            "the second pull is never answered"
        );
    }

    #[tokio::test]
    async fn a_command_to_a_stopped_owner_is_stopped() {
        let (commands, inbox) = mpsc::channel(1);
        drop(inbox);
        let handle = QueueHandle { commands };
        assert_eq!(
            handle.consumer_joins(ConsumerId::new("c")).await,
            Err(Stopped::Owner)
        );
    }

    #[tokio::test]
    async fn a_command_the_owner_drops_unanswered_is_stopped() {
        let (commands, mut inbox) = mpsc::channel(1);
        let handle = QueueHandle { commands };
        let (answered, ()) = tokio::join!(handle.publisher_joins(PublisherId::new("p")), async {
            drop(inbox.recv().await);
        });
        assert_eq!(answered, Err(Stopped::Owner));
    }

    #[tokio::test]
    async fn a_full_channel_makes_the_sender_wait_not_fail() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (commands, inbox) = mpsc::channel(1);
        let handle = QueueHandle { commands };
        let (first, second) = (handle.clone(), handle.clone());
        drop(handle);
        let mut both = std::pin::pin!(async move {
            tokio::join!(
                first.consumer_joins(ConsumerId::new("c1")),
                second.consumer_joins(ConsumerId::new("c2"))
            )
        });
        // With no owner, the first waits for its answer and the second for room in
        // the channel. Polled twice, both are still waiting: neither was refused.
        for _ in 0..2 {
            let waiting = poll_fn(|context| Poll::Ready(both.as_mut().poll(context).is_pending()));
            assert!(waiting.await, "a sender finished with no owner running");
        }
        let (_stop, never) = watch::channel(false);
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox, no_ticks(), never));
        let answers = tokio::time::timeout(BOUND, both).await.unwrap();
        assert_eq!(answers, (Ok(Ok(())), Ok(Ok(()))));
        let ended = tokio::time::timeout(BOUND, owner).await.unwrap();
        assert_eq!(ended.unwrap(), Ok(()));
    }

    fn row(pocket: &str, key: &str, rank: i64) -> Publication {
        Publication::new(
            PocketKey::new(pocket),
            RecordKey::new(key),
            Rank::new(rank),
            Payload::from(&b"x"[..]),
        )
    }

    #[test]
    fn each_row_applies_in_order_and_an_equal_rank_refuses_its_row_alone() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let mut queue = queue(&lease_ids);
        queue.publisher_joins(PublisherId::new("p")).unwrap();
        let rows = vec![
            row("a", "k1", 1),
            row("a", "k2", 1),
            row("a", "k1", 2),
            row("a", "k1", 1),
            row("a", "k1", 2),
            row("b", "k1", 1),
        ];
        let outcomes = publish_each(&mut queue, &PublisherId::new("p"), rows).unwrap();
        assert_eq!(
            outcomes,
            [
                RowOutcome::Applied(PublishOutcome::Created),
                RowOutcome::Applied(PublishOutcome::NewKey),
                RowOutcome::Applied(PublishOutcome::Superseded),
                RowOutcome::Applied(PublishOutcome::Stale),
                RowOutcome::EqualRank,
                RowOutcome::Applied(PublishOutcome::Created),
            ]
        );
    }

    #[test]
    fn an_unjoined_publisher_is_refused_at_the_first_row_with_nothing_landed() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let mut queue = queue(&lease_ids);
        let rows = vec![row("a", "k1", 1), row("b", "k1", 1)];
        assert_eq!(
            publish_each(&mut queue, &PublisherId::new("p"), rows),
            Err(PublishError::UnknownPublisher(PublisherId::new("p")))
        );
        queue.consumer_joins(ConsumerId::new("c")).unwrap();
        assert_eq!(
            queue.pull(&ConsumerId::new("c"), 10),
            Ok(PullOutcome::NoPocketAvailable)
        );
    }

    // guarantee: NeverRefusedForCapacity
    #[tokio::test]
    async fn a_full_channel_makes_a_batch_publish_wait_then_succeed() {
        let (handle, inbox) = QueueHandle::unowned(4);
        // Every filler's reply depends on the order the owner runs them in, and the
        // batch's outcomes depend on the fillers having run before it.
        let (reply, mut joined) = oneshot::channel();
        let join = Command::PublisherJoins {
            publisher_id: PublisherId::new("p0"),
            reply,
        };
        handle.commands.try_send(join).unwrap();
        let mut published = Vec::new();
        for rank in [1, 2, 1] {
            let (reply, answer) = oneshot::channel();
            let publish = Command::Publish {
                publisher_id: PublisherId::new("p0"),
                publication: row("a", "k1", rank),
                reply,
            };
            handle.commands.try_send(publish).unwrap();
            published.push(answer);
        }
        let (reply, _) = oneshot::channel();
        let extra = Command::PublisherJoins {
            publisher_id: PublisherId::new("p4"),
            reply,
        };
        assert!(
            matches!(handle.commands.try_send(extra), Err(TrySendError::Full(_))),
            "the channel filled at its capacity"
        );
        let rows = vec![row("a", "k1", 3), row("a", "k2", 1)];
        let mut batch =
            std::pin::pin!(async move { handle.publish_batch(PublisherId::new("p0"), rows).await });
        let waited = tokio::time::timeout(Duration::from_millis(50), batch.as_mut()).await;
        assert!(waited.is_err(), "the batch finished with no owner running");
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (_stop, never) = watch::channel(false);
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox, no_ticks(), never));
        let outcomes = tokio::time::timeout(BOUND, batch).await.unwrap();
        assert_eq!(
            outcomes,
            Ok(Ok(vec![
                RowOutcome::Applied(PublishOutcome::Superseded),
                RowOutcome::Applied(PublishOutcome::NewKey),
            ]))
        );
        assert_eq!(joined.try_recv(), Ok(Ok(())), "filler 0 ran first");
        let expected = [
            PublishOutcome::Created,
            PublishOutcome::Superseded,
            PublishOutcome::Stale,
        ];
        for (mut answer, outcome) in published.into_iter().zip(expected) {
            assert_eq!(
                answer.try_recv(),
                Ok(Ok(outcome)),
                "the fillers ran in order"
            );
        }
        let ended = tokio::time::timeout(BOUND, owner).await.unwrap();
        assert_eq!(ended.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn the_owner_ends_once_every_handle_is_dropped() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (readiness, _) = watch::channel(Readiness::Starting);
        let (_stop, never) = watch::channel(false);
        let (handle, owner) = start(queue(&lease_ids), lease_ids, readiness, never, no_ticks());
        let owner = tokio::spawn(owner);
        let copy = handle.clone();
        assert_eq!(
            copy.publisher_joins(PublisherId::new("p")).await,
            Ok(Ok(()))
        );
        let published = handle.publish(PublisherId::new("p"), publication("pk"));
        assert!(published.await.unwrap().is_ok());
        assert_eq!(
            handle.lease_view(ConsumerId::new("c")).await,
            Ok(Err(PullError::UnknownConsumer(ConsumerId::new("c"))))
        );
        drop((handle, copy));
        assert_eq!(
            tokio::time::timeout(BOUND, owner).await.unwrap().unwrap(),
            Ok(())
        );
    }

    #[tokio::test]
    async fn the_owner_reports_serving_then_stops_when_told_and_drops_its_inbox() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (readiness, mut watching) = watch::channel(Readiness::Starting);
        let (stop, stopping) = watch::channel(false);
        let (handle, owner) = start(
            queue(&lease_ids),
            lease_ids,
            readiness,
            stopping,
            no_ticks(),
        );
        assert_eq!(*watching.borrow_and_update(), Readiness::Starting);
        let owner = tokio::spawn(owner);
        tokio::time::timeout(
            BOUND,
            watching.wait_for(|state| *state == Readiness::Serving),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            handle.publisher_joins(PublisherId::new("p")).await,
            Ok(Ok(()))
        );
        stop.send_replace(true);
        let ended = tokio::time::timeout(BOUND, owner).await.unwrap().unwrap();
        assert_eq!(
            ended,
            Ok(()),
            "the owner ended while a handle is still held"
        );
        assert_eq!(
            handle.consumer_joins(ConsumerId::new("c")).await,
            Err(Stopped::Owner)
        );
    }

    #[tokio::test]
    async fn the_owner_never_turns_stopping_back_to_serving() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (readiness, watching) = watch::channel(Readiness::Stopping);
        let (stop, stopping) = watch::channel(true);
        let (_handle, owner) = start(
            queue(&lease_ids),
            lease_ids,
            readiness,
            stopping,
            no_ticks(),
        );
        assert_eq!(tokio::time::timeout(BOUND, owner).await.unwrap(), Ok(()));
        assert_eq!(*watching.borrow(), Readiness::Stopping);
        drop(stop);
    }

    #[test]
    fn readiness_reads_as_its_state() {
        let names = [Readiness::Starting, Readiness::Serving, Readiness::Stopping]
            .map(|state| state.to_string());
        assert_eq!(names, ["starting", "serving", "stopping"]);
    }

    /// The smallest sleep tokio's timer resolves: one millisecond.
    const SMALLEST_SLEEP: Duration = Duration::from_millis(1);

    /// The ticker's protocol at the smallest sleep, never stopping, counting each
    /// tick the owner answered.
    async fn counting_ticker(lane: mpsc::Sender<Tick>, answered: Arc<AtomicUsize>) {
        loop {
            tokio::time::sleep(SMALLEST_SLEEP).await;
            let (done, applied) = oneshot::channel();
            if lane.send(Tick { done }).await.is_err() || applied.await.is_err() {
                return;
            }
            answered.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Both lanes stay ready: more senders than the inbox holds each send their next
    // command as soon as the last is answered, so the inbox never drains, while the
    // ticker runs at the smallest sleep. Each command is answered within the bound,
    // and the ticks keep being answered meanwhile; nothing orders a tick against the
    // commands sent before it.
    #[tokio::test]
    async fn neither_lane_starves_the_other_when_both_stay_ready() {
        const CAPACITY: usize = 2;
        const SENDERS: usize = 8;
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (handle, inbox) = QueueHandle::unowned(CAPACITY);
        let (lane, ticks) = tick_lane();
        let (_stop, never) = watch::channel(false);
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox, ticks, never));
        let answered = Arc::new(AtomicUsize::new(0));
        let ticker = tokio::spawn(counting_ticker(lane, Arc::clone(&answered)));
        let (done, running) = watch::channel(false);
        let commands = Arc::new(AtomicUsize::new(0));
        let mut senders = tokio::task::JoinSet::new();
        for sender in 0..SENDERS {
            let sending = Sender {
                id: sender,
                handle: handle.clone(),
                answered: Arc::clone(&commands),
                done: running.clone(),
            };
            senders.spawn(sending.run());
        }
        drop(handle);
        let ticks_before = answered.load(Ordering::SeqCst);
        let rising = until_at_least(&answered, ticks_before + 5);
        let commands_before = commands.load(Ordering::SeqCst);
        tokio::time::timeout(BOUND, rising)
            .await
            .expect("the ticks stopped being answered under the command backlog");
        assert!(
            commands.load(Ordering::SeqCst) > commands_before,
            "no command was answered while the ticks were"
        );
        done.send_replace(true);
        tokio::time::timeout(BOUND, senders.join_all())
            .await
            .unwrap();
        let ended = tokio::time::timeout(BOUND, owner).await.unwrap().unwrap();
        assert_eq!(ended, Ok(()));
        tokio::time::timeout(BOUND, ticker).await.unwrap().unwrap();
    }

    /// Waits until `counter` reaches `target`.
    async fn until_at_least(counter: &AtomicUsize, target: usize) {
        while counter.load(Ordering::SeqCst) < target {
            tokio::time::sleep(SMALLEST_SLEEP).await;
        }
    }

    /// One sender of the backlog: joins consumer after consumer, each answered
    /// within the bound, until `done` turns true.
    struct Sender {
        id: usize,
        handle: QueueHandle,
        answered: Arc<AtomicUsize>,
        done: watch::Receiver<bool>,
    }

    impl Sender {
        async fn run(self) {
            let mut serial = 0_usize;
            while !*self.done.borrow() {
                let consumer = ConsumerId::new(format!("c{}-{serial}", self.id));
                let reply = tokio::time::timeout(BOUND, self.handle.consumer_joins(consumer))
                    .await
                    .expect("a command waited past the bound behind the ticks");
                assert_eq!(reply, Ok(Ok(())));
                self.answered.fetch_add(1, Ordering::SeqCst);
                serial += 1;
            }
        }
    }

    /// Asks for `consumer`'s lease until it holds none.
    async fn until_no_lease(handle: &QueueHandle, consumer: &str) {
        while handle.lease_view(ConsumerId::new(consumer)).await != Ok(Ok(None)) {
            tokio::time::sleep(SMALLEST_SLEEP).await;
        }
    }

    /// An owner that takes every tick and drops it unanswered; returns how many.
    async fn drop_every_tick(mut ticks: mpsc::Receiver<Tick>) -> usize {
        let mut taken = 0;
        while ticks.recv().await.is_some() {
            taken += 1;
        }
        taken
    }

    #[tokio::test]
    async fn the_ticker_ends_once_the_shutdown_begins_while_it_sleeps() {
        let (lane, _ticks) = tick_lane();
        let (shutdown, shutting_down) = watch::channel(false);
        let ticker = tokio::spawn(ticker(lane, Duration::from_secs(3600), shutting_down));
        shutdown.send_replace(true);
        assert_eq!(
            tokio::time::timeout(BOUND, ticker).await.unwrap().unwrap(),
            Ok(())
        );
    }

    #[tokio::test]
    async fn the_ticker_ends_once_the_shutdown_begins_while_it_waits_on_the_owner() {
        let (lane, mut ticks) = tick_lane();
        let (shutdown, shutting_down) = watch::channel(false);
        let ticker = tokio::spawn(ticker(lane, SMALLEST_SLEEP, shutting_down));
        // The owner takes the tick and never answers it.
        let held = tokio::time::timeout(BOUND, ticks.recv()).await.unwrap();
        shutdown.send_replace(true);
        assert_eq!(
            tokio::time::timeout(BOUND, ticker).await.unwrap().unwrap(),
            Ok(())
        );
        drop(held);
    }

    #[tokio::test]
    async fn the_ticker_ends_once_the_owner_has_gone_or_drops_its_answer() {
        let (_shutdown, never) = watch::channel(false);
        let (lane, ticks) = tick_lane();
        drop(ticks);
        let gone = ticker(lane, SMALLEST_SLEEP, never.clone());
        assert_eq!(tokio::time::timeout(BOUND, gone).await.unwrap(), Ok(()));
        let (lane, ticks) = tick_lane();
        let dropping = tokio::spawn(drop_every_tick(ticks));
        let dropped = ticker(lane, SMALLEST_SLEEP, never);
        assert_eq!(tokio::time::timeout(BOUND, dropped).await.unwrap(), Ok(()));
        assert_eq!(
            tokio::time::timeout(BOUND, dropping)
                .await
                .unwrap()
                .unwrap(),
            1
        );
    }

    // The real ticker against the real owner: a lease of zero lapses on the first
    // tick after the pull, and the record is pulled again.
    #[tokio::test]
    async fn the_owner_applies_the_tickers_ticks_and_a_lease_lapses() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let mut config = Config::default();
        config.lease_ttl = Duration::ZERO;
        let queue = Queue::new(config, SystemClock::new().unwrap(), lease_ids.clone()).unwrap();
        let (readiness, _) = watch::channel(Readiness::Starting);
        let (shutdown, stopping) = watch::channel(false);
        let (lane, ticks) = tick_lane();
        let (handle, owner) = start(queue, lease_ids, readiness, stopping.clone(), ticks);
        let owner = tokio::spawn(owner);
        let ticker = tokio::spawn(ticker(lane, SMALLEST_SLEEP, stopping));
        handle
            .publisher_joins(PublisherId::new("p"))
            .await
            .unwrap()
            .unwrap();
        for consumer in ["c1", "c2"] {
            handle
                .consumer_joins(ConsumerId::new(consumer))
                .await
                .unwrap()
                .unwrap();
        }
        handle
            .publish(PublisherId::new("p"), publication("pk"))
            .await
            .unwrap()
            .unwrap();
        let first = handle.pull(ConsumerId::new("c1"), 10).await.unwrap();
        assert_eq!(pocket_of(first), "pk");
        let lapsed = until_no_lease(&handle, "c1");
        tokio::time::timeout(BOUND, lapsed)
            .await
            .expect("the lease never lapsed");
        let second = handle.pull(ConsumerId::new("c2"), 10).await.unwrap();
        assert_eq!(pocket_of(second), "pk");
        shutdown.send_replace(true);
        assert_eq!(
            tokio::time::timeout(BOUND, ticker).await.unwrap().unwrap(),
            Ok(())
        );
        assert_eq!(
            tokio::time::timeout(BOUND, owner).await.unwrap().unwrap(),
            Ok(())
        );
    }
}
