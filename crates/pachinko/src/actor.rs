//! The queue's owner: one tokio task holds the service's [`Queue`], and every action
//! reaches it as a [`Command`] on one bounded channel (decision 0019).
//!
//! The HTTP handlers send commands through a [`QueueHandle`] and wait on a `oneshot`
//! reply. The owner applies each command synchronously, in arrival order, with no
//! borrow of the queue held across an `.await` and no `Mutex`: a lapse (T26) and a
//! consumer's action are therefore totally ordered. A full channel makes the sender
//! wait, never refuses it.
//!
//! A `Pull` whose requester has gone before the owner reaches it is skipped, so no
//! lease is granted to nobody. A requester that goes after that check, while the
//! owner applies the pull, keeps the lease until it lapses (T26); the reply is then
//! dropped, as every unwanted reply is.
//!
//! The owner reports [`Readiness::Serving`] as its first act, before it takes any
//! command, and leaves its loop between commands once the scope tells it to stop.
//! Commands still in its inbox are then dropped unanswered, and their senders answer
//! 503 `not_ready`.

use libpachinko::error::{JoinError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, PublisherId};
use libpachinko::queue::{LeaseView, Publication, PublishOutcome, PullOutcome, Queue};
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

/// One action for the owner, with the channel its answer goes back on. T25 adds the
/// lease actions, T26 the `Tick` lane and T27 `PublishBatch`.
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
}

/// The handle to `queue`'s owner and the owner itself, a future the caller spawns
/// into the task scope it owns. The owner holds `lease_ids`, a clone of the queue's
/// source, to see exhaustion before a pull.
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
        own(queue, lease_ids, inbox, stop).await
    };
    (QueueHandle { commands }, owner)
}

/// The owner loop: applies each command in arrival order until every sender is
/// gone or `stop` turns true. The stop is checked first, so the owner takes no
/// command once told to stop; it is only seen between commands, each applied
/// synchronously. The owner yields after each command, so that on a runtime with
/// one worker the scope's grace timer runs and its stop lands before the next
/// command, rather than after the owner's whole cooperative budget. Returning, or
/// being aborted, drops the queue, which releases its state without a yield, in
/// time proportional to its size: the shutdown's bound is two grace periods plus the
/// one command or tick the owner was applying, and releasing the queue's state when
/// the owner ends.
async fn own(
    mut queue: ServiceQueue,
    lease_ids: EpochSerialLeaseIds,
    mut inbox: mpsc::Receiver<Command>,
    mut stop: watch::Receiver<bool>,
) -> Result<(), ServeError> {
    loop {
        let command = tokio::select! {
            biased;
            _ = stop.wait_for(|stop| *stop) => return Ok(()),
            command = inbox.recv() => command,
        };
        let Some(command) = command else {
            return Ok(());
        };
        apply(&mut queue, &lease_ids, command)?;
        tokio::task::yield_now().await;
    }
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
        Command::Pull {
            consumer_id,
            limit,
            reply,
        } => return pull(queue, lease_ids, &consumer_id, limit, reply),
        Command::LeaseView { consumer_id, reply } => {
            answer(reply, queue.lease_view(&consumer_id));
        }
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
    let _unwanted = reply.send(queue.pull(consumer_id, limit));
    Ok(())
}

/// Sends `value` back. A requester that has gone no longer wants it, so a failed
/// send is dropped.
fn answer<T>(reply: Reply<T>, value: T) {
    let _unwanted = reply.send(value);
}

#[cfg(test)]
mod tests {
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use std::time::Duration;

    use libpachinko::config::Config;
    use libpachinko::error::{JoinError, PullError};
    use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use libpachinko::queue::{Publication, PullOutcome, Queue};
    use tokio::sync::{mpsc, oneshot, watch};

    use super::{Command, QueueHandle, Readiness, ServiceQueue, Stopped, own, start};
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
        let owner = own(queue(&lease_ids), lease_ids, inbox, never);
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
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox, never));
        let answers = tokio::time::timeout(BOUND, both).await.unwrap();
        assert_eq!(answers, (Ok(Ok(())), Ok(Ok(()))));
        let ended = tokio::time::timeout(BOUND, owner).await.unwrap();
        assert_eq!(ended.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn the_owner_ends_once_every_handle_is_dropped() {
        let lease_ids = EpochSerialLeaseIds::starting_at(1);
        let (readiness, _) = watch::channel(Readiness::Starting);
        let (_stop, never) = watch::channel(false);
        let (handle, owner) = start(queue(&lease_ids), lease_ids, readiness, never);
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
        let (handle, owner) = start(queue(&lease_ids), lease_ids, readiness, stopping);
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
        let (_handle, owner) = start(queue(&lease_ids), lease_ids, readiness, stopping);
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
}
