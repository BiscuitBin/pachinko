//! The window: between a pocket machine's decision and the order machine's hearing of
//! it, or between the order machine's routing and the pocket machine's hearing of
//! that. `Direct` runs every submit to quiescence, so no test through it can put a
//! client input inside the window; these tests step the order machine and one pocket
//! machine by hand, hold a chained message back, deliver the client input first, then
//! deliver the held message in the order the pool's FIFO lanes promise. Each ends at
//! quiescence with zero violations, every record handed, nothing claimed twice and the
//! order machine's view consistent with the pocket.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use worker_pool::{
    Config, ConsumerId, Duration, Input, LeaseId, LeaseStatus, Machine, MachineId, OrderMachine,
    OrderStatus, Output, Partition, Payload, PocketKey, PocketMachine, PublisherId, Rank,
    RecordKey, Response, Timestamp, invariants,
};

const fn ms(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

/// A short lease and one record per batch, so a lapse is cheap to reach and a pull
/// hands one record at a time.
fn config() -> Config {
    Config::default()
        .with_lease_ttl(Duration::from_millis(10))
        .with_max_pull_batch(1)
}

fn key() -> PocketKey {
    PocketKey::new("A")
}

fn publish(record: &str, rank: u64) -> Input {
    Input::Publish {
        publisher: PublisherId::new("p1"),
        pocket_key: key(),
        record_key: RecordKey::new(record),
        rank: Rank::new(rank),
        payload: Payload::new(format!("{record}@{rank}")),
    }
}

fn pull(consumer: &str, limit: u64) -> Input {
    Input::Pull {
        consumer: ConsumerId::new(consumer),
        limit,
    }
}

fn release(consumer: &str, lease: LeaseId) -> Input {
    Input::Release {
        consumer: ConsumerId::new(consumer),
        lease,
    }
}

/// One step's outputs, sorted by destination: the messages for the order machine,
/// the messages for the one pocket machine, and the replies.
#[derive(Debug, Default)]
struct Split {
    to_order: Vec<Input>,
    to_pocket: Vec<Input>,
    replies: Vec<Response>,
}

fn split(outputs: Vec<Output>) -> Result<Split, String> {
    let mut split = Split::default();
    for output in outputs {
        match output {
            Output::Forward(MachineId::Order, input) | Output::Notify(MachineId::Order, input) => {
                split.to_order.push(input);
            }
            Output::Forward(MachineId::Partition(_), input)
            | Output::Notify(MachineId::Partition(_), input) => split.to_pocket.push(input),
            Output::Reply(response) => split.replies.push(response),
            other => return Err(format!("an output the protocol never produces: {other:?}")),
        }
    }
    Ok(split)
}

/// Exactly one item, or what came instead.
fn only<T: std::fmt::Debug>(items: Vec<T>) -> Result<T, String> {
    <[T; 1]>::try_from(items)
        .map(|[item]| item)
        .map_err(|items| format!("expected exactly one, got {items:?}"))
}

/// Steps a client input or a `RetryPull` into the order machine, which must forward
/// exactly one message to the pocket machine and reply nothing.
fn order_forwards(order: &mut OrderMachine, input: Input, now: Timestamp) -> Result<Input, String> {
    let split = split(order.step(input, now))?;
    if !split.replies.is_empty() || !split.to_order.is_empty() {
        return Err(format!(
            "the order machine did more than forward: {split:?}"
        ));
    }
    only(split.to_pocket)
}

/// Steps a message into the pocket machine, which must reply exactly once and chain
/// nothing.
fn pocket_replies(
    pocket: &mut PocketMachine,
    input: Input,
    now: Timestamp,
) -> Result<Response, String> {
    let split = split(pocket.step(input, now))?;
    if !split.to_order.is_empty() || !split.to_pocket.is_empty() {
        return Err(format!("the pocket machine chained a message: {split:?}"));
    }
    only(split.replies)
}

/// Steps a message into the pocket machine, which must send exactly one message to
/// the order machine and reply nothing: the message the test then holds back.
fn pocket_notifies(
    pocket: &mut PocketMachine,
    input: Input,
    now: Timestamp,
) -> Result<Input, String> {
    let split = split(pocket.step(input, now))?;
    if !split.replies.is_empty() || !split.to_pocket.is_empty() {
        return Err(format!(
            "the pocket machine did more than notify: {split:?}"
        ));
    }
    only(split.to_order)
}

/// Steps `input` (a `Pull` or a `RetryPull`) into the order machine, which must
/// forward a `Claim`, delivers it, and returns the batch.
fn claims(
    order: &mut OrderMachine,
    pocket: &mut PocketMachine,
    input: Input,
    now: Timestamp,
) -> Result<Response, String> {
    let claim = order_forwards(order, input, now)?;
    if !matches!(claim, Input::Claim { .. }) {
        return Err(format!("not a claim: {claim:?}"));
    }
    pocket_replies(pocket, claim, now)
}

const fn lease_of(response: &Response) -> Option<LeaseId> {
    match response {
        Response::Batch { lease, .. } => Some(*lease),
        _ => None,
    }
}

fn handed(response: &Response) -> Option<Vec<&str>> {
    match response {
        Response::Batch { records, .. } => Some(records.iter().map(|r| r.key.as_str()).collect()),
        _ => None,
    }
}

fn view_status(order: &OrderMachine) -> Option<&OrderStatus> {
    order.view(&key()).map(|view| &view.status)
}

/// Quiescence: every invariant and every `spike:` check holds, and no machine counted
/// a protocol violation.
fn holds(order: &OrderMachine, pocket: &PocketMachine) {
    assert_eq!(
        invariants::check(order, std::slice::from_ref(pocket)),
        Vec::new()
    );
    assert_eq!(order.violations(), 0);
    assert_eq!(pocket.violations(), 0);
}

/// Nothing double-claimed: across the pocket machine exactly one lease is active, it
/// is `lease`, and the order machine routes `consumer` under it.
fn claimed_once(order: &OrderMachine, pocket: &PocketMachine, consumer: &str, lease: LeaseId) {
    let active: Vec<LeaseId> = pocket
        .pockets()
        .flat_map(|(_, pocket)| pocket.leases.iter())
        .filter(|held| held.status == LeaseStatus::Active)
        .map(|held| held.id)
        .collect();
    assert_eq!(active, vec![lease]);
    assert_eq!(order.active_lease(&ConsumerId::new(consumer)), Some(lease));
}

/// A pocket `A` with `r1` and `r2`, enqueued in the order machine; both quiescent.
fn setup() -> Result<(OrderMachine, PocketMachine), String> {
    let mut order = OrderMachine::new(config());
    let mut pocket = PocketMachine::new(Partition::new(0), 1, config());
    let first = split(pocket.step(publish("r1", 1), ms(0)))?;
    let enqueue = only(first.to_order)?;
    only(first.replies)?;
    if !order.step(enqueue, ms(0)).is_empty() {
        return Err("an enqueue produced outputs".to_owned());
    }
    pocket_replies(&mut pocket, publish("r2", 1), ms(0))?;
    holds(&order, &pocket);
    Ok((order, pocket))
}

#[test]
fn a_second_pull_before_the_first_claim_is_processed_is_forwarded_behind_it() {
    let (mut order, mut pocket) = setup().unwrap();
    // The first pull: a `Claim` forwarded, not yet delivered.
    let claim = order_forwards(&mut order, pull("c1", 10), ms(1)).unwrap();
    assert!(matches!(claim, Input::Claim { .. }));
    // The second pull while the `Claim` is in flight: routed under the recorded lease.
    let under_lease = order_forwards(&mut order, pull("c1", 10), ms(1)).unwrap();
    assert!(matches!(under_lease, Input::PullUnderLease { .. }));
    // FIFO at the pocket: the `Claim` first, then the `PullUnderLease`.
    let batch_1 = pocket_replies(&mut pocket, claim, ms(1)).unwrap();
    let batch_2 = pocket_replies(&mut pocket, under_lease, ms(1)).unwrap();
    assert_eq!(handed(&batch_1), Some(vec!["r1"]));
    assert_eq!(handed(&batch_2), Some(vec!["r2"]));
    assert_eq!(lease_of(&batch_1), lease_of(&batch_2));
    assert_eq!(order.retry_pulls(), 0);
    claimed_once(&order, &pocket, "c1", lease_of(&batch_1).unwrap());
    holds(&order, &pocket);
}

#[test]
fn a_pull_racing_a_lapse_is_retried_after_the_pocket_became_is_heard() {
    let (mut order, mut pocket) = setup().unwrap();
    let first = lease_of(&claims(&mut order, &mut pocket, pull("c1", 10), ms(1)).unwrap()).unwrap();
    holds(&order, &pocket);
    // The lease lapses at the pocket; the `PocketBecame` is in flight to the order.
    let became = pocket_notifies(&mut pocket, Input::Tick, ms(11)).unwrap();
    assert!(matches!(became, Input::PocketBecame { .. }));
    // The consumer pulls again before the order machine hears it: a stale route, which
    // the pocket machine bounces back as a `RetryPull`.
    let under_lease = order_forwards(&mut order, pull("c1", 10), ms(11)).unwrap();
    assert!(matches!(under_lease, Input::PullUnderLease { .. }));
    let retry = pocket_notifies(&mut pocket, under_lease, ms(11)).unwrap();
    assert!(matches!(retry, Input::RetryPull { .. }));
    // FIFO at the order machine: the `PocketBecame` was sent first, the `RetryPull`
    // second, so the retry finds the pocket queued and claims it under a new lease.
    assert_eq!(order.step(became, ms(11)), Vec::new());
    let batch = claims(&mut order, &mut pocket, retry, ms(11)).unwrap();
    let second = lease_of(&batch).unwrap();
    assert_ne!(first, second);
    // The record the lapse freed is handed again, at the pocket's kept position.
    assert_eq!(handed(&batch), Some(vec!["r1"]));
    assert!(matches!(
        view_status(&order),
        Some(OrderStatus::Leased { position: 0, .. })
    ));
    assert_eq!(
        pocket.lease(first).map(|(_, lease)| lease.status),
        Some(LeaseStatus::Expired)
    );
    assert_eq!(order.retry_pulls(), 1);
    claimed_once(&order, &pocket, "c1", second);
    holds(&order, &pocket);
}

#[test]
fn a_release_followed_at_once_by_a_pull_is_retried_and_claims_the_pocket_at_the_back() {
    let (mut order, mut pocket) = setup().unwrap();
    let first = lease_of(&claims(&mut order, &mut pocket, pull("c1", 10), ms(1)).unwrap()).unwrap();
    // The release at the pocket: `PocketBecame` in flight, the reply already returned.
    let released = split(pocket.step(release("c1", first), ms(2))).unwrap();
    assert_eq!(released.replies, vec![Response::Released]);
    let became = only(released.to_order).unwrap();
    assert!(matches!(became, Input::PocketBecame { .. }));
    // The pull before the order machine hears it: the same retry path.
    let under_lease = order_forwards(&mut order, pull("c1", 10), ms(2)).unwrap();
    let retry = pocket_notifies(&mut pocket, under_lease, ms(2)).unwrap();
    assert!(matches!(retry, Input::RetryPull { .. }));
    assert_eq!(order.step(became, ms(2)), Vec::new());
    let batch = claims(&mut order, &mut pocket, retry, ms(2)).unwrap();
    let second = lease_of(&batch).unwrap();
    assert_ne!(second, first);
    // The released record is handed again, and the pocket was requeued at the back.
    assert_eq!(handed(&batch), Some(vec!["r1"]));
    assert!(matches!(
        view_status(&order),
        Some(OrderStatus::Leased { position: 1, .. })
    ));
    assert_eq!(order.next_pocket_position(), 2);
    assert_eq!(order.retry_pulls(), 1);
    claimed_once(&order, &pocket, "c1", second);
    holds(&order, &pocket);
}

#[test]
fn a_publish_overtaking_a_claim_leaves_the_pocket_claimable() {
    let (mut order, mut pocket) = setup().unwrap();
    let claim = order_forwards(&mut order, pull("c1", 10), ms(1)).unwrap();
    assert!(matches!(claim, Input::Claim { .. }));
    // A publish into `A` reaches the pocket before the `Claim` does: a queued pocket is
    // not re-enqueued, so nothing is chained.
    let published = pocket_replies(&mut pocket, publish("r3", 1), ms(1)).unwrap();
    assert_eq!(
        published,
        Response::Published(worker_pool::Published::NewKey)
    );
    let batch = pocket_replies(&mut pocket, claim, ms(1)).unwrap();
    assert_eq!(handed(&batch), Some(vec!["r1"]));
    assert_eq!(pocket.pocket(&key()).map(|p| p.records.len()), Some(3));
    claimed_once(&order, &pocket, "c1", lease_of(&batch).unwrap());
    holds(&order, &pocket);
}
