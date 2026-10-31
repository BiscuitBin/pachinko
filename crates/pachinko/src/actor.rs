//! The queue's owner: one tokio task holds the service's `Queue`, and every trigger
//! reaches it as a command on one bounded channel (decision 0019).
//!
//! The Flight handlers send the parties' commands through a [`QueueHandle`] and wait
//! on a `oneshot` reply; a ticker task sends `Tick` on the same channel at a fixed
//! interval. A lapse and a consumer's action on the same lease are therefore
//! processed in one stream, in arrival order, as the module's expiry comment
//! requires. Both tasks live in the `JoinSet` that `serve` owns: there is no
//! detached task.
//!
//! A command the owner has started is finished even when its requester has gone:
//! the reply is then dropped, and the queue keeps the change, as a pull whose
//! client disconnected keeps its lease until it lapses.

use std::time::Duration;

use arrow_array::RecordBatch;
use libpachinko::clock::Timestamp;
use libpachinko::error::{ActionError, ConsumerError, JoinError, PublishError};
use libpachinko::queue::{
    Lease, LeaseId, LeaseView, Publication, PublishOutcome, PullOutcome, Queue, SettleOutcome,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::MissedTickBehavior;

use crate::ServeError;
use crate::arrow::ArrowRecords;
use crate::clock::SystemClock;

/// The service's queue: Arrow records and the system clock.
pub(crate) type ServiceQueue = Queue<ArrowRecords, SystemClock>;

/// One publish row's outcome: the rule that took it, or `None` when the queue
/// rejected it for an equal rank, the module's placeholder (its first open
/// question).
pub(crate) type RowOutcome = Option<PublishOutcome>;

/// How many commands may wait for the owner before a sender waits too.
const COMMAND_CAPACITY: usize = 1024;

/// The shortest tick period: `tokio::time::interval` rejects a zero period.
const SHORTEST_TICK: Duration = Duration::from_millis(1);

/// Where the owner sends one command's answer.
type Reply<T> = oneshot::Sender<T>;

/// One trigger for the owner, with the channel its answer goes back on.
#[derive(Debug)]
enum Command {
    /// `PublisherJoins`.
    PublisherJoins {
        publisher_id: String,
        reply: Reply<Result<(), JoinError>>,
    },
    /// `ConsumerJoins`.
    ConsumerJoins {
        consumer_id: String,
        reply: Reply<Result<(), JoinError>>,
    },
    /// `Publish`, once per row of one input batch, in row order.
    Publish {
        publisher_id: String,
        publications: Vec<Publication>,
        reply: Reply<Result<Vec<RowOutcome>, PublishError>>,
    },
    /// `Pull`.
    Pull {
        consumer_id: String,
        limit: i64,
        reply: Reply<Result<PullOutcome<RecordBatch>, ConsumerError>>,
    },
    /// `Ack`.
    Ack {
        consumer_id: String,
        lease: LeaseId,
        record_key: String,
        reply: Reply<Result<SettleOutcome, ActionError>>,
    },
    /// `Nack`.
    Nack {
        consumer_id: String,
        lease: LeaseId,
        record_key: String,
        reply: Reply<Result<SettleOutcome, ActionError>>,
    },
    /// `RenewLease`.
    Renew {
        consumer_id: String,
        lease: LeaseId,
        reply: Reply<Result<Timestamp, ActionError>>,
    },
    /// `ReleaseLease`.
    Release {
        consumer_id: String,
        lease: LeaseId,
        reply: Reply<Result<Lease, ActionError>>,
    },
    /// The `Consuming` surface's `exposes` for one consumer.
    Status {
        consumer_id: String,
        reply: Reply<Result<Option<LeaseView>, ConsumerError>>,
    },
    /// Process the lapses and the decay that are due.
    Tick,
}

/// The owner task has stopped, so a command could not be sent or its answer never
/// came back. The Flight handlers answer `UNAVAILABLE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the queue's owner task has stopped")]
pub(crate) struct Stopped;

/// A cloneable sender of commands to the queue's owner. Each method sends one
/// command and waits for its answer.
#[derive(Clone, Debug)]
pub(crate) struct QueueHandle {
    commands: mpsc::Sender<Command>,
}

/// What [`start`] hands back: the handle the Flight service sends through, and the
/// signal that stops the ticker.
#[derive(Debug)]
pub(crate) struct Started {
    /// The handle the Flight service sends commands through.
    pub(crate) queue: QueueHandle,
    /// Sending on it, or dropping it, stops the ticker.
    pub(crate) stop_ticker: oneshot::Sender<()>,
}

impl QueueHandle {
    /// Sends the command `command` builds around a fresh reply channel, then waits
    /// for the answer.
    async fn ask<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> Result<T, Stopped> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_| Stopped)?;
        answer.await.map_err(|_| Stopped)
    }

    /// `PublisherJoins` for `publisher_id`.
    pub(crate) async fn join_publisher(
        &self,
        publisher_id: String,
    ) -> Result<Result<(), JoinError>, Stopped> {
        self.ask(|reply| Command::PublisherJoins {
            publisher_id,
            reply,
        })
        .await
    }

    /// `ConsumerJoins` for `consumer_id`.
    pub(crate) async fn join_consumer(
        &self,
        consumer_id: String,
    ) -> Result<Result<(), JoinError>, Stopped> {
        self.ask(|reply| Command::ConsumerJoins { consumer_id, reply })
            .await
    }

    /// `Publish` for each of `publications`, in order, as one command: no other
    /// command runs between two of its rows.
    pub(crate) async fn publish_batch(
        &self,
        publisher_id: String,
        publications: Vec<Publication>,
    ) -> Result<Result<Vec<RowOutcome>, PublishError>, Stopped> {
        self.ask(|reply| Command::Publish {
            publisher_id,
            publications,
            reply,
        })
        .await
    }

    /// `Pull` for `consumer_id`, up to `limit` records.
    pub(crate) async fn pull_batch(
        &self,
        consumer_id: String,
        limit: i64,
    ) -> Result<Result<PullOutcome<RecordBatch>, ConsumerError>, Stopped> {
        self.ask(|reply| Command::Pull {
            consumer_id,
            limit,
            reply,
        })
        .await
    }

    /// `Ack` of `record_key` under `lease`.
    pub(crate) async fn ack_record(
        &self,
        consumer_id: String,
        lease: LeaseId,
        record_key: String,
    ) -> Result<Result<SettleOutcome, ActionError>, Stopped> {
        self.ask(|reply| Command::Ack {
            consumer_id,
            lease,
            record_key,
            reply,
        })
        .await
    }

    /// `Nack` of `record_key` under `lease`.
    pub(crate) async fn nack_record(
        &self,
        consumer_id: String,
        lease: LeaseId,
        record_key: String,
    ) -> Result<Result<SettleOutcome, ActionError>, Stopped> {
        self.ask(|reply| Command::Nack {
            consumer_id,
            lease,
            record_key,
            reply,
        })
        .await
    }

    /// `RenewLease` of `lease`.
    pub(crate) async fn renew_lease(
        &self,
        consumer_id: String,
        lease: LeaseId,
    ) -> Result<Result<Timestamp, ActionError>, Stopped> {
        self.ask(|reply| Command::Renew {
            consumer_id,
            lease,
            reply,
        })
        .await
    }

    /// `ReleaseLease` of `lease`.
    pub(crate) async fn release_lease(
        &self,
        consumer_id: String,
        lease: LeaseId,
    ) -> Result<Result<Lease, ActionError>, Stopped> {
        self.ask(|reply| Command::Release {
            consumer_id,
            lease,
            reply,
        })
        .await
    }

    /// The `Consuming` surface's `exposes` for `consumer_id`.
    pub(crate) async fn lease_status(
        &self,
        consumer_id: String,
    ) -> Result<Result<Option<LeaseView>, ConsumerError>, Stopped> {
        self.ask(|reply| Command::Status { consumer_id, reply })
            .await
    }
}

#[cfg(test)]
impl QueueHandle {
    /// A handle whose owner has already stopped, for tests of what never reaches it.
    pub(crate) fn stopped() -> Self {
        let (commands, _) = mpsc::channel(1);
        Self { commands }
    }
}

/// Spawns the owner of `queue` and a ticker that sends `Tick` every `period` (at
/// least a millisecond) into `tasks`.
///
/// The owner ends once every sender is gone: the handle and its clones, and the
/// ticker's, which it drops when `stop_ticker` fires or is dropped, or when the
/// channel closes. The ticker skips the interval's immediate first tick.
pub(crate) fn start(tasks: &mut JoinSet<()>, mut queue: ServiceQueue, period: Duration) -> Started {
    let (commands, mut inbox) = mpsc::channel(COMMAND_CAPACITY);
    let (stop_ticker, mut stopped) = oneshot::channel::<()>();
    let ticks = commands.clone();
    let period = period.max(SHORTEST_TICK);
    tasks.spawn(async move {
        while let Some(command) = inbox.recv().await {
            apply(&mut queue, command);
        }
    });
    tasks.spawn(async move {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                _ = interval.tick() => {
                    if ticks.send(Command::Tick).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    Started {
        queue: QueueHandle { commands },
        stop_ticker,
    }
}

/// Waits for every task in `tasks` to end, and reports the first that panicked or
/// was cancelled.
///
/// # Errors
///
/// [`ServeError::Task`] with that task's failure.
pub(crate) async fn join_tasks(mut tasks: JoinSet<()>) -> Result<(), ServeError> {
    let mut first = Ok(());
    while let Some(joined) = tasks.join_next().await {
        if let Err(error) = joined {
            first = first.and(Err(ServeError::Task(error.to_string())));
        }
    }
    first
}

/// Runs one command against the queue and sends its answer.
fn apply(queue: &mut ServiceQueue, command: Command) {
    match command {
        Command::PublisherJoins {
            publisher_id,
            reply,
        } => answer(reply, queue.publisher_joins(&publisher_id)),
        Command::ConsumerJoins { consumer_id, reply } => {
            answer(reply, queue.consumer_joins(&consumer_id));
        }
        Command::Publish {
            publisher_id,
            publications,
            reply,
        } => answer(reply, publish_each(queue, &publisher_id, publications)),
        Command::Pull {
            consumer_id,
            limit,
            reply,
        } => answer(reply, queue.pull(&consumer_id, limit)),
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
        Command::Status { consumer_id, reply } => answer(reply, queue.lease_view(&consumer_id)),
        Command::Tick => {
            queue.tick();
        }
    }
}

/// Sends `value` back. A requester that has gone no longer wants it, so a failed
/// send is dropped.
fn answer<T>(reply: Reply<T>, value: T) {
    let _unwanted = reply.send(value);
}

/// Publishes each row in order. An equal rank rejects its row alone; an unknown
/// publisher rejects the batch at its first row, before any row has landed.
fn publish_each(
    queue: &mut ServiceQueue,
    publisher_id: &str,
    publications: Vec<Publication>,
) -> Result<Vec<RowOutcome>, PublishError> {
    publications
        .into_iter()
        .map(
            |publication| match queue.publish(publisher_id, publication) {
                Ok(outcome) => Ok(Some(outcome)),
                Err(PublishError::EqualRank { .. }) => Ok(None),
                Err(error) => Err(error),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{QueueHandle, ServiceQueue, Stopped, publish_each, start};
    use crate::ServeError;
    use crate::clock::SystemClock;
    use libpachinko::config::Config;
    use libpachinko::error::PublishError;
    use libpachinko::queue::{Publication, PublishOutcome, Queue};
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::task::JoinSet;

    fn queue() -> ServiceQueue {
        Queue::new(Config::default(), SystemClock).unwrap()
    }

    #[test]
    fn an_equal_rank_rejects_its_row_alone() {
        let mut queue = queue();
        queue.publisher_joins("p").unwrap();
        let rows = vec![
            Publication::new("pk", "rk", 1, Vec::new()),
            Publication::new("pk", "rk", 1, Vec::new()),
            Publication::new("pk", "rk", 2, Vec::new()),
        ];
        let outcomes = publish_each(&mut queue, "p", rows).unwrap();
        assert_eq!(
            outcomes,
            [
                Some(PublishOutcome::New),
                None,
                Some(PublishOutcome::Superseded)
            ]
        );
    }

    #[test]
    fn an_unknown_publisher_rejects_the_batch() {
        let rows = vec![Publication::new("pk", "rk", 1, Vec::new())];
        let error = publish_each(&mut queue(), "nobody", rows).unwrap_err();
        assert_eq!(error, PublishError::UnknownPublisher("nobody".to_owned()));
    }

    #[tokio::test]
    async fn a_command_to_a_stopped_owner_is_refused() {
        let handle = QueueHandle::stopped();
        assert_eq!(handle.join_consumer("c".to_owned()).await, Err(Stopped));
    }

    #[tokio::test]
    async fn a_command_the_owner_drops_unanswered_is_refused() {
        let (commands, mut inbox) = mpsc::channel(1);
        let handle = QueueHandle { commands };
        let (answered, ()) = tokio::join!(handle.join_publisher("p".to_owned()), async {
            drop(inbox.recv().await);
        });
        assert_eq!(answered, Err(Stopped));
    }

    #[tokio::test]
    async fn the_tasks_end_once_the_handle_and_the_ticker_are_gone() {
        let mut tasks = JoinSet::new();
        let started = start(&mut tasks, queue(), Duration::ZERO);
        let joined = started.queue.join_consumer("c".to_owned()).await;
        assert_eq!(joined, Ok(Ok(())));
        drop(started);
        assert_eq!(super::join_tasks(tasks).await, Ok(()));
    }

    #[tokio::test]
    async fn a_panicked_task_is_reported() {
        let mut tasks = JoinSet::new();
        tasks.spawn(async { panic!("the owner failed") });
        let joined = super::join_tasks(tasks).await;
        assert!(matches!(joined, Err(ServeError::Task(message)) if message.contains("panic")));
    }
}
