"""Builders for hand-made entities and snapshots.

Each builder has defaults that make a valid entity, so a test names only what
it changes. Most of the states the invariant controls need are not reachable
through `Queue` yet, which is why they are built by hand.
"""

from collections.abc import Callable
from unittest import TestCase

from pachinko_double.config import Config
from pachinko_double.effects import FakeClock, SequentialLeaseIds
from pachinko_double.inspection import ConfigValues, Snapshot
from pachinko_double.model import (
    MINUTE,
    Consumer,
    DeadLetter,
    Lease,
    LeaseStatus,
    Mark,
    Pocket,
    PocketStatus,
    Publisher,
    Record,
    RecordStatus,
    ShedReason,
    SupersededRecord,
    Timestamp,
)
from pachinko_double.pull import Batch
from pachinko_double.queue import Queue

DEFAULT_CONFIG = ConfigValues.of(Config())


def config(**fields: int) -> ConfigValues:
    values = {name: getattr(DEFAULT_CONFIG, name) for name in DEFAULT_CONFIG.__dataclass_fields__}
    return ConfigValues(**(values | fields))


def snapshot(
    *,
    config: ConfigValues = DEFAULT_CONFIG,
    next_position: int = 100,
    publishers: tuple[Publisher, ...] = (),
    consumers: tuple[Consumer, ...] = (),
    pockets: tuple[Pocket, ...] = (),
    records: tuple[Record, ...] = (),
    leases: tuple[Lease, ...] = (),
    superseded: tuple[SupersededRecord, ...] = (),
    dead_letters: tuple[DeadLetter, ...] = (),
) -> Snapshot:
    return Snapshot(
        config=config,
        next_position=next_position,
        publishers=publishers,
        consumers=consumers,
        pockets=pockets,
        records=records,
        leases=leases,
        superseded=superseded,
        dead_letters=dead_letters,
    )


def pocket(
    key: str = "p",
    status: PocketStatus = PocketStatus.QUEUED,
    position: int = 0,
    consecutive_expiries: int = 0,
) -> Pocket:
    return Pocket(key=key, status=status, position=position, consecutive_expiries=consecutive_expiries)


def queued(pocket: str = "p", key: str = "k", rank: int = 1, payload: bytes = b"v", position: int = 1) -> Record:
    return Record(pocket=pocket, key=key, rank=rank, payload=payload, position=position, status=RecordStatus.QUEUED)


def held(
    pocket: str = "p",
    key: str = "k",
    rank: int = 1,
    payload: bytes = b"v",
    position: int = 1,
    *,
    mark: Mark = Mark.UNSETTLED,
    held_rank: int | None = None,
    held_payload: bytes | None = None,
    nacked_at: Timestamp | None = None,
    deferred_position: int | None = None,
) -> Record:
    """A held record; with no held version named, it holds its own version."""
    return Record(
        pocket=pocket,
        key=key,
        rank=rank,
        payload=payload,
        position=position,
        status=RecordStatus.HELD,
        held_rank=rank if held_rank is None else held_rank,
        held_payload=payload if held_payload is None else held_payload,
        mark=mark,
        nacked_at=nacked_at,
        deferred_position=deferred_position,
    )


def lease(
    id: str = "lease-1",
    pocket: str = "p",
    consumer: str = "c",
    status: LeaseStatus = LeaseStatus.ACTIVE,
    granted_at: Timestamp = 0,
    expires_at: Timestamp = 5 * MINUTE,
    ended_at: Timestamp | None = None,
) -> Lease:
    return Lease(
        id=id,
        pocket=pocket,
        consumer=consumer,
        status=status,
        granted_at=granted_at,
        expires_at=expires_at,
        ended_at=ended_at,
    )


def shed(
    pocket: str = "p",
    key: str = "k",
    rank: int = 1,
    payload: bytes = b"old",
    reason: ShedReason = ShedReason.SUPERSEDED,
    shed_at: Timestamp = 0,
    expires_at: Timestamp = 12 * 60 * MINUTE,
) -> SupersededRecord:
    return SupersededRecord(
        pocket=pocket, key=key, rank=rank, payload=payload, reason=reason, shed_at=shed_at, expires_at=expires_at
    )


def letter(
    pocket: str = "p",
    key: str = "k",
    rank: int = 1,
    payload: bytes = b"bad",
    nacked_by: str = "c",
    nacked_at: Timestamp = 0,
    expires_at: Timestamp = 12 * 60 * MINUTE,
) -> DeadLetter:
    return DeadLetter(
        pocket=pocket,
        key=key,
        rank=rank,
        payload=payload,
        nacked_by=nacked_by,
        nacked_at=nacked_at,
        expires_at=expires_at,
    )


def new_queue(config: Config | None = None, now: Timestamp = 0) -> tuple[Queue, FakeClock, SequentialLeaseIds]:
    clock, lease_ids = FakeClock(now), SequentialLeaseIds()
    return Queue(Config() if config is None else config, clock=clock, lease_ids=lease_ids), clock, lease_ids


# The scenarios' setting, ported from the Rust tests' fixture: the clock starts
# at T0, the TTLs are short and differ from one another so a mix-up shows, and
# publisher "p" and consumers "c1" to "c3" have joined.
T0: Timestamp = 1_000_000
LEASE_TTL = 1_000
SUPERSEDED_TTL = 5_000
DEAD_LETTER_TTL = 7_000
PUBLISHER = "p"
CONSUMERS = ("c1", "c2", "c3")


def scenario_queue(**config_fields: int) -> tuple[Queue, FakeClock]:
    queue, clock, _ = scenario_setting(**config_fields)
    return queue, clock


def scenario_setting(**config_fields: int) -> tuple[Queue, FakeClock, SequentialLeaseIds]:
    """The scenarios' setting, with the lease-id source a test counts draws from."""
    settings = dict(lease_ttl=LEASE_TTL, superseded_ttl=SUPERSEDED_TTL, dead_letter_ttl=DEAD_LETTER_TTL)
    queue, clock, lease_ids = new_queue(Config(**(settings | config_fields)), now=T0)
    queue.publisher_joins(PUBLISHER)
    for consumer in CONSUMERS:
        queue.consumer_joins(consumer)
    return queue, clock, lease_ids


def batch(queue: Queue, consumer: str, limit: int) -> Batch:
    """A pull that must answer a batch."""
    answer = queue.pull(consumer, limit)
    assert isinstance(answer, Batch), f"{consumer} was answered {answer!r}"
    return answer


def keys(answer: Batch) -> list[str]:
    """The batch's record keys, in the batch's order."""
    return [record.key for record in answer.records]


def publish_answers_nothing(
    test: TestCase, queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes
) -> None:
    """A publish where the Rust test asserts a `PublishOutcome`.

    No Publish rule answers the publisher, so the double's publish answers
    None, and the test asserts it beside the state the publish leaves. The
    Rust's outcomes are not ported (spec note N32).
    """
    answer = queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)
    test.assertIsNone(answer, "a publish answers nothing (N32)")


def refused[R: Exception](test: TestCase, queue: Queue, refusal: type[R], action: Callable[[], object]) -> R:
    """Run `action`, which must raise `refusal`; the complete snapshot is unchanged.

    It returns the refusal, so a test can check what it names.
    """
    before = queue.snapshot()
    with test.assertRaises(refusal) as raised:
        action()
    test.assertEqual(queue.snapshot(), before, "a refused action changed the queue")
    return raised.exception


def two_pockets() -> tuple[Queue, FakeClock]:
    """Pocket a with a1 and a2, then pocket b with b1, in the scenarios' setting.

    a stands at 0, a1 at 1, a2 at 2, b at 3 and b1 at 4; the next position is 5.
    """
    queue, clock = scenario_queue()
    queue.publish(PUBLISHER, "a", "a1", 1, b"a1")
    queue.publish(PUBLISHER, "a", "a2", 1, b"a2")
    queue.publish(PUBLISHER, "b", "b1", 1, b"b1")
    return queue, clock
