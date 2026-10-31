//! A single-threaded router over the machines: steps them in place and delivers every
//! message in FIFO order per destination, with no thread and no clock. The replay and
//! invariant tests drive it, and it is the logic reference the pool's one-thread
//! configuration is compared with.
//!
//! One client input is submitted at a time and run to quiescence: every `Forward` and
//! `Notify` a step emits joins the back of its destination's queue, the queues are
//! drained front first (the order machine, then the partitions by index, round-robin),
//! and the one `Reply` the protocol promises is returned. Any other shape is a
//! [`Protocol`] error, never a panic.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::machine::Machine;
use crate::order::OrderMachine;
use crate::partition::route;
use crate::pocket::PocketMachine;
use crate::protocol::{Input, MachineId, Output, Partition, Response};
use crate::time::{Config, Timestamp};

/// Every input each machine was stepped with, in order, with the `now` it saw.
pub type Logs = BTreeMap<MachineId, Vec<(Input, Timestamp)>>;

/// How the router's protocol was broken: a shape the machines promise never to produce,
/// or an input the router cannot place.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    /// The input is one the machines send each other, or the tick; `submit` takes only
    /// a client's input.
    NotAClientInput,
    /// A machine replied while processing an input nobody was waiting on.
    ReplyWithoutRequest,
    /// The queues drained and no machine replied.
    NoReply,
    /// More than one machine replied to the one request.
    ManyReplies(Vec<Response>),
    /// An output named a machine the router does not have.
    UnknownMachine(MachineId),
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAClientInput => f.write_str("the input is not a client's"),
            Self::ReplyWithoutRequest => f.write_str("a machine replied without a request"),
            Self::NoReply => f.write_str("the queues drained without a reply"),
            Self::ManyReplies(replies) => {
                write!(f, "{} replies to one request: {replies:?}", replies.len())
            }
            Self::UnknownMachine(id) => write!(f, "no machine {id:?}"),
        }
    }
}

impl std::error::Error for Protocol {}

/// The reply handle of the one request in flight: the inputs chained from a client's
/// input carry it, the notifications and the tick carry none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Handle;

/// One message waiting on a destination's queue.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Envelope {
    input: Input,
    handle: Option<Handle>,
}

type Queues = BTreeMap<MachineId, VecDeque<Envelope>>;

/// The direct router: one order machine and one pocket machine per partition, stepped
/// in place on the caller's thread.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Direct {
    config: Config,
    order: OrderMachine,
    pockets: Vec<PocketMachine>,
    queues: Queues,
    logs: Logs,
}

impl Direct {
    /// A router over fresh machines: one order machine and `partitions` pocket machines
    /// sharing `config`.
    ///
    /// ```
    /// use worker_pool::{Config, Direct, MachineId, Partition};
    /// let direct = Direct::new(Config::default(), 3);
    /// assert_eq!(direct.partitions(), 3);
    /// assert_eq!(direct.machine_count(), 4);
    /// assert_eq!(direct.pockets()[2].partition(), Partition::new(2));
    /// assert!(direct.logs()[&MachineId::Order].is_empty());
    /// ```
    #[must_use]
    pub fn new(config: Config, partitions: u32) -> Self {
        let pockets: Vec<PocketMachine> = (0..partitions)
            .map(|i| PocketMachine::new(Partition::new(i), partitions, config))
            .collect();
        let ids = std::iter::once(MachineId::Order)
            .chain(pockets.iter().map(Machine::id))
            .collect::<Vec<_>>();
        Self {
            config,
            order: OrderMachine::new(config),
            pockets,
            queues: ids.iter().map(|id| (*id, VecDeque::new())).collect(),
            logs: ids.iter().map(|id| (*id, Vec::new())).collect(),
        }
    }

    /// The configuration every machine was built with.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The order machine.
    #[must_use]
    pub const fn order(&self) -> &OrderMachine {
        &self.order
    }

    /// The pocket machines, by partition index.
    #[must_use]
    pub fn pockets(&self) -> &[PocketMachine] {
        &self.pockets
    }

    /// How many partitions, which is how many pocket machines.
    #[must_use]
    pub fn partitions(&self) -> u32 {
        u32::try_from(self.pockets.len()).unwrap_or(u32::MAX)
    }

    /// How many machines: the order machine and the pocket machines.
    #[must_use]
    pub const fn machine_count(&self) -> usize {
        self.pockets.len() + 1
    }

    /// Every input each machine was stepped with, in order, with its `now`: the client
    /// inputs, the inputs chained from them and the ticks.
    #[must_use]
    pub const fn logs(&self) -> &Logs {
        &self.logs
    }

    /// Delivers one client input at time `now` and runs the machines to quiescence,
    /// returning the one reply.
    ///
    /// # Errors
    ///
    /// [`Protocol::NotAClientInput`] when the input is not one a client sends; the
    /// other variants when the machines break the one-reply promise.
    pub fn submit(&mut self, input: Input, now: Timestamp) -> Result<Response, Protocol> {
        let dest = route(&input, self.partitions()).ok_or(Protocol::NotAClientInput)?;
        self.enqueue(dest, input, Some(Handle))?;
        let mut replies = self.drain(now)?;
        match replies.len() {
            0 => Err(Protocol::NoReply),
            1 => replies.pop().ok_or(Protocol::NoReply),
            _ => Err(Protocol::ManyReplies(replies)),
        }
    }

    /// Steps the tick into every machine at time `now`, the order machine first, and
    /// runs to quiescence.
    ///
    /// # Errors
    ///
    /// [`Protocol::ReplyWithoutRequest`] when any machine replies to the tick or to a
    /// message chained from it.
    pub fn tick(&mut self, now: Timestamp) -> Result<(), Protocol> {
        let ids: Vec<MachineId> = self.queues.keys().copied().collect();
        for id in ids {
            self.enqueue(id, Input::Tick, None)?;
        }
        self.drain(now).map(drop)
    }

    fn enqueue(
        &mut self,
        dest: MachineId,
        input: Input,
        handle: Option<Handle>,
    ) -> Result<(), Protocol> {
        let queue = self
            .queues
            .get_mut(&dest)
            .ok_or(Protocol::UnknownMachine(dest))?;
        queue.push_back(Envelope { input, handle });
        Ok(())
    }

    fn drain(&mut self, now: Timestamp) -> Result<Vec<Response>, Protocol> {
        let Self {
            order,
            pockets,
            queues,
            logs,
            ..
        } = self;
        // The queues are keyed by the machines that exist, so a message never reaches
        // the dispatch for a partition that is missing; the empty fallback is for the
        // type, not for a case.
        run(queues, logs, now, &mut |id, input, now| match id {
            MachineId::Order => order.step(input, now),
            MachineId::Partition(partition) => pockets
                .get_mut(partition.index() as usize)
                .map_or_else(Vec::new, |machine| machine.step(input, now)),
        })
    }
}

/// Drains every queue to quiescence: each round pops the front of every non-empty
/// queue in key order (the order machine, then the partitions by index) and steps it,
/// so that everything one machine sends another arrives in the order sent. Returns the
/// replies to the request in flight.
fn run(
    queues: &mut Queues,
    logs: &mut Logs,
    now: Timestamp,
    step: &mut impl FnMut(MachineId, Input, Timestamp) -> Vec<Output>,
) -> Result<Vec<Response>, Protocol> {
    let ids: Vec<MachineId> = queues.keys().copied().collect();
    let mut replies = Vec::new();
    loop {
        let mut progressed = false;
        for id in &ids {
            let Some(envelope) = queues.get_mut(id).and_then(VecDeque::pop_front) else {
                continue;
            };
            progressed = true;
            logs.entry(*id)
                .or_default()
                .push((envelope.input.clone(), now));
            let outputs = step(*id, envelope.input, now);
            deliver(queues, &mut replies, envelope.handle, outputs)?;
        }
        if !progressed {
            return Ok(replies);
        }
    }
}

/// Places one step's outputs: a `Forward` keeps the handle of the input being
/// processed, a `Notify` carries none, and a `Reply` answers that handle.
fn deliver(
    queues: &mut Queues,
    replies: &mut Vec<Response>,
    handle: Option<Handle>,
    outputs: Vec<Output>,
) -> Result<(), Protocol> {
    for output in outputs {
        let (dest, input, carried) = match output {
            Output::Forward(dest, input) => (dest, input, handle),
            Output::Notify(dest, input) => (dest, input, None),
            Output::Reply(response) => {
                handle.ok_or(Protocol::ReplyWithoutRequest)?;
                replies.push(response);
                continue;
            }
        };
        let queue = queues
            .get_mut(&dest)
            .ok_or(Protocol::UnknownMachine(dest))?;
        queue.push_back(Envelope {
            input,
            handle: carried,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};

    use super::{Direct, Envelope, Handle, Logs, Protocol, Queues, run};
    use crate::protocol::{
        ConsumerId, Input, MachineId, Output, Partition, PocketKey, PublisherId, Response,
    };
    use crate::time::{Config, Timestamp};

    const ORDER: MachineId = MachineId::Order;
    const P0: MachineId = MachineId::Partition(Partition::new(0));
    const P1: MachineId = MachineId::Partition(Partition::new(1));

    fn join(name: &str) -> Input {
        Input::PublisherJoins {
            publisher: PublisherId::new(name),
        }
    }

    fn enqueue(key: &str) -> Input {
        Input::Enqueue {
            pocket_key: PocketKey::new(key),
            partition: Partition::new(0),
        }
    }

    fn queues_and_logs() -> (Queues, Logs) {
        let ids = [ORDER, P0, P1];
        (
            ids.iter().map(|id| (*id, VecDeque::new())).collect(),
            ids.iter().map(|id| (*id, Vec::new())).collect(),
        )
    }

    fn inputs(logs: &Logs, id: MachineId) -> Vec<Input> {
        logs[&id].iter().map(|(input, _)| input.clone()).collect()
    }

    #[test]
    fn a_notify_queued_behind_a_pending_forward_to_the_same_destination_arrives_after_it() {
        let (mut queues, mut logs) = queues_and_logs();
        // P0 already holds a forwarded message; the order machine's step then notifies
        // P0 and forwards to P1, and P1's step notifies P0 again.
        queues.get_mut(&P0).unwrap().push_back(Envelope {
            input: join("pending"),
            handle: Some(Handle),
        });
        queues.get_mut(&ORDER).unwrap().push_back(Envelope {
            input: join("client"),
            handle: Some(Handle),
        });
        let replies = run(
            &mut queues,
            &mut logs,
            Timestamp::ZERO,
            &mut |id, input, _| match (id, input) {
                (ORDER, _) => vec![
                    Output::Notify(P0, enqueue("from-order")),
                    Output::Forward(P1, join("hop")),
                ],
                (P1, _) => vec![
                    Output::Notify(P0, enqueue("from-p1")),
                    Output::Reply(Response::Joined),
                ],
                _ => Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(replies, vec![Response::Joined]);
        assert_eq!(
            inputs(&logs, P0),
            vec![join("pending"), enqueue("from-order"), enqueue("from-p1")]
        );
        assert_eq!(inputs(&logs, ORDER), vec![join("client")]);
        assert_eq!(inputs(&logs, P1), vec![join("hop")]);
        assert!(queues.values().all(VecDeque::is_empty));
    }

    #[test]
    fn a_reply_to_a_notification_is_a_protocol_error() {
        let (mut queues, mut logs) = queues_and_logs();
        queues.get_mut(&ORDER).unwrap().push_back(Envelope {
            input: join("client"),
            handle: Some(Handle),
        });
        let result = run(&mut queues, &mut logs, Timestamp::ZERO, &mut |id, _, _| {
            if id == ORDER {
                vec![
                    Output::Notify(P0, enqueue("k")),
                    Output::Reply(Response::Joined),
                ]
            } else {
                vec![Output::Reply(Response::Joined)]
            }
        });
        assert_eq!(result, Err(Protocol::ReplyWithoutRequest));
    }

    #[test]
    fn two_replies_to_one_request_are_a_protocol_error() {
        let mut direct = Direct::new(Config::default(), 1);
        direct.enqueue(ORDER, join("client"), Some(Handle)).unwrap();
        let Direct { queues, logs, .. } = &mut direct;
        let replies = run(queues, logs, Timestamp::ZERO, &mut |_, _, _| {
            vec![
                Output::Reply(Response::Joined),
                Output::Reply(Response::NoPocketAvailable),
            ]
        })
        .unwrap();
        let outcome = match replies.len() {
            1 => Ok(()),
            _ => Err(Protocol::ManyReplies(replies)),
        };
        assert_eq!(
            outcome,
            Err(Protocol::ManyReplies(vec![
                Response::Joined,
                Response::NoPocketAvailable
            ]))
        );
    }

    #[test]
    fn no_reply_is_a_protocol_error_and_a_chained_input_is_logged_with_its_now() {
        let mut direct = Direct::new(Config::default(), 2);
        let now = Timestamp::from_millis(7);
        // Enqueue never carries a reply handle in the protocol, so no machine, stub or
        // real, replies to it: the drain ends with no reply to the request.
        direct.enqueue(ORDER, enqueue("k"), Some(Handle)).unwrap();
        let replies = direct.drain(now).unwrap();
        assert_eq!(replies, Vec::new());
        let as_submit = match replies.len() {
            0 => Err(Protocol::NoReply),
            _ => Ok(()),
        };
        assert_eq!(as_submit, Err(Protocol::NoReply));
        assert_eq!(direct.logs()[&ORDER], vec![(enqueue("k"), now)]);
        assert_eq!(direct.logs()[&P0], Vec::new());
    }

    #[test]
    fn a_tick_is_logged_into_every_machine_order_first() {
        let mut direct = Direct::new(Config::default(), 2);
        direct.tick(Timestamp::from_millis(3)).unwrap();
        for id in [ORDER, P0, P1] {
            assert_eq!(
                direct.logs()[&id],
                vec![(Input::Tick, Timestamp::from_millis(3))]
            );
        }
        assert_eq!(direct.logs().len(), 3);
    }

    #[test]
    fn an_internal_input_is_not_a_client_input() {
        let mut direct = Direct::new(Config::default(), 1);
        let retry = Input::RetryPull {
            consumer: ConsumerId::new("c"),
            limit: 1,
        };
        assert_eq!(
            direct.submit(retry, Timestamp::ZERO),
            Err(Protocol::NotAClientInput)
        );
        assert_eq!(
            direct.submit(Input::Tick, Timestamp::ZERO),
            Err(Protocol::NotAClientInput)
        );
        assert!(direct.logs().values().all(Vec::is_empty));
    }

    #[test]
    fn an_output_to_a_machine_the_router_lacks_is_an_error() {
        let (mut queues, mut logs) = queues_and_logs();
        queues.get_mut(&ORDER).unwrap().push_back(Envelope {
            input: join("client"),
            handle: Some(Handle),
        });
        let missing = MachineId::Partition(Partition::new(9));
        let result = run(&mut queues, &mut logs, Timestamp::ZERO, &mut |_, _, _| {
            vec![Output::Forward(missing, join("hop"))]
        });
        assert_eq!(result, Err(Protocol::UnknownMachine(missing)));
    }

    #[test]
    fn every_error_displays_itself() {
        let shown: BTreeMap<&str, String> = [
            ("none", Protocol::NotAClientInput.to_string()),
            ("without", Protocol::ReplyWithoutRequest.to_string()),
            ("no", Protocol::NoReply.to_string()),
            (
                "many",
                Protocol::ManyReplies(vec![Response::Joined, Response::Acked]).to_string(),
            ),
            ("unknown", Protocol::UnknownMachine(P1).to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(shown["none"], "the input is not a client's");
        assert_eq!(shown["without"], "a machine replied without a request");
        assert_eq!(shown["no"], "the queues drained without a reply");
        assert!(shown["many"].starts_with("2 replies to one request"));
        assert!(shown["unknown"].contains("Partition"));
        let error: Box<dyn std::error::Error> = Box::new(Protocol::NoReply);
        assert_eq!(error.to_string(), Protocol::NoReply.to_string());
    }
}
