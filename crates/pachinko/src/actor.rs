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
//! A batch publish is one `PublishBatch` command, so no other command runs between
//! two of its rows. Its rows apply in order and independently: an equal rank refuses
//! its row alone, and any other refusal ends the batch where it stands, leaving the
//! rows before it applied. The only such refusal the library has, an unknown
//! publisher, comes at the first row, before any row lands.

use libpachinko::error::{JoinError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, PublisherId};
use libpachinko::queue::{LeaseView, Publication, PublishOutcome, PullOutcome, Queue};
use libpachinko::records::MemoryRecords;
use tokio::sync::{mpsc, oneshot};

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
}

/// The handle to `queue`'s owner and the owner itself, a future the caller spawns
/// into the task scope it owns. The owner holds `lease_ids`, a clone of the queue's
/// source, to see exhaustion before a pull.
///
/// The owner ends with `Ok` once every handle is dropped, and with
/// [`ServeError::LeaseIdsExhausted`] at the first `Pull` after the lease-id source
/// is exhausted, rather than reissue an id.
pub(crate) fn start(
    queue: ServiceQueue,
    lease_ids: EpochSerialLeaseIds,
) -> (
    QueueHandle,
    impl Future<Output = Result<(), ServeError>> + Send + 'static,
) {
    let (commands, inbox) = mpsc::channel(COMMAND_CAPACITY);
    (QueueHandle { commands }, own(queue, lease_ids, inbox))
}

/// The owner loop: applies each command in arrival order until every sender is
/// gone.
async fn own(
    mut queue: ServiceQueue,
    lease_ids: EpochSerialLeaseIds,
    mut inbox: mpsc::Receiver<Command>,
) -> Result<(), ServeError> {
    while let Some(command) = inbox.recv().await {
        apply(&mut queue, &lease_ids, command)?;
    }
    Ok(())
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
    use std::task::Poll;
    use std::time::Duration;

    use libpachinko::config::Config;
    use libpachinko::error::{JoinError, PublishError, PullError};
    use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
    use libpachinko::queue::{Publication, PublishOutcome, PullOutcome, Queue};
    use tokio::sync::mpsc::error::TrySendError;
    use tokio::sync::{mpsc, oneshot};

    use super::{
        Command, QueueHandle, RowOutcome, ServiceQueue, Stopped, own, publish_each, start,
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
        let owner = own(queue(&lease_ids), lease_ids, inbox);
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
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox));
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
        let owner = tokio::spawn(own(queue(&lease_ids), lease_ids, inbox));
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
        let (handle, owner) = start(queue(&lease_ids), lease_ids);
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
}
