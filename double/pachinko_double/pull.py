"""The specification's Pull section: the Pull and Handed tables.

A pull is one outcome. Its rule computes the batch from the state before the
pull, emits `Handed` for each record of the batch, and answers `Batch` (or
`NoPocketAvailable`) through `Outcome.answer`. `HoldRecord` answers each
`Handed` and moves that record to held. The checker runs once, after every
`Handed` has been answered (queue.py).

The batch is `oldest(pocket.queued_records, r => r.position,
min(limit, config.max_pull_batch))`: the queued records with the smallest
position, in that order, all of them when the size exceeds their count and
none when it is not positive.

The three Pull rules have disjoint guards. In a state that holds the
invariants one of them always holds, so the Pull table's refusal,
`PullRefused`, is unreachable through `Queue` (spec note N14). `Handed` with
no guard holding is a defect.

A lease id is drawn only by `PullClaimsFrontPocket`'s effect, which runs only
once its guard holds, so a refused pull, a pull within a lease and a pull
that finds no pocket draw none (spec note N15).
"""

from collections.abc import Iterable
from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from .inspection import View
from .model import Lease, LeaseStatus, Mark, Pocket, PocketStatus, Record, RecordStatus
from .refusals import PullRefused
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class Pull:
    """The trigger `Pull(consumer, limit)`; `consumer` is a joined consumer's id."""

    consumer: str
    limit: int


@dataclass(frozen=True)
class Handed:
    """The event `Handed(pocket, record_key)`; `pocket` is the pocket's key."""

    pocket: str
    record_key: str


@dataclass(frozen=True)
class Batch:
    """What a pull hands back: `Batch(consumer, lease, pocket, records)`.

    `consumer` is the consumer's id, `lease` the lease's id (spec note N05),
    `pocket` the pocket's key, and `records` the batch's records, oldest
    first, as they stood before the pull: the records `HoldRecord` then
    moves to held. Their key, rank and payload are what the consumer was
    handed; their status is the one they had before the pull (spec note N13).
    """

    consumer: str
    lease: str
    pocket: str
    records: tuple[Record, ...]


@dataclass(frozen=True)
class NoPocketAvailable:
    """The answer `NoPocketAvailable(consumer)`: an answer, not a refusal."""

    consumer: str


def oldest(records: Iterable[Record], n: int) -> tuple[Record, ...]:
    """`oldest(records, r => r.position, n)`: the n with the smallest position, in order."""
    if n <= 0:
        return ()
    return tuple(sorted(records, key=lambda record: record.position)[:n])


def _queued_pockets(view: View) -> tuple[Pocket, ...]:
    """`Pockets where status = queued`."""
    return tuple(pocket for pocket in view.state.pockets if pocket.status is PocketStatus.QUEUED)


def _has_active_lease(view: View, consumer: str) -> bool:
    """`exists Lease{consumer: consumer, status: active}`."""
    return bool(view.state.active_leases_by_consumer.get(consumer, ()))


def _batch(view: View, outcome: "Outcome", pocket: Pocket, limit: int) -> tuple[Record, ...]:
    """`oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))`."""
    return oldest(view.state.queued_by_pocket[pocket.key], min(limit, outcome.config.max_pull_batch))


def _hand(outcome: "Outcome", pocket: Pocket, records: tuple[Record, ...]) -> None:
    # ensures: for record in batch: Handed(pocket: pocket, record_key: record.key)
    for record in records:
        outcome.emit(Handed(pocket.key, record.key))


# rule PullWithinLease -----------------------------------------------------


def _lease_and_pocket(view: View, consumer: str) -> tuple[Lease, Pocket] | None:
    # let lease = Lease{consumer: consumer, status: active}
    # requires: exists lease
    # let pocket = lease.pocket
    leases = view.state.active_leases_by_consumer.get(consumer, ())
    if not leases:
        return None
    lease = leases[0]
    (pocket,) = view.state.pockets_keyed(lease.pocket)
    return lease, pocket


def _pull_within_lease_guard(view: View, trigger: Pull) -> bool:
    found = _lease_and_pocket(view, trigger.consumer)
    # requires: pocket.status = leased
    return found is not None and found[1].status is PocketStatus.LEASED


def _pull_within_lease(view: View, outcome: "Outcome", trigger: Pull) -> None:
    found = _lease_and_pocket(view, trigger.consumer)
    assert found is not None
    lease, pocket = found
    # let batch = oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))
    records = _batch(view, outcome, pocket, trigger.limit)
    # ensures: lease.expires_at = now + config.lease_ttl
    outcome.replace(replace(lease, expires_at=view.now + outcome.config.lease_ttl))
    _hand(outcome, pocket, records)
    # ensures: Batch(consumer: consumer, lease: lease, pocket: pocket, records: batch)
    outcome.answer(Batch(trigger.consumer, lease.id, pocket.key, records))


# rule PullClaimsFrontPocket -----------------------------------------------


def _front_pocket(view: View) -> Pocket | None:
    # let free = Pockets where status = queued
    # let pocket = min_by(free, p => p.position)
    free = _queued_pockets(view)
    return min(free, key=lambda pocket: pocket.position) if free else None


def _pull_claims_front_pocket_guard(view: View, trigger: Pull) -> bool:
    # requires: not exists Lease{consumer: consumer, status: active}
    if _has_active_lease(view, trigger.consumer):
        return False
    # requires: free.count > 0
    pocket = _front_pocket(view)
    # requires: pocket.status = queued
    return pocket is not None and pocket.status is PocketStatus.QUEUED


def _pull_claims_front_pocket(view: View, outcome: "Outcome", trigger: Pull) -> None:
    pocket = _front_pocket(view)
    assert pocket is not None
    # let batch = oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))
    records = _batch(view, outcome, pocket, trigger.limit)
    # ensures: Lease.created(pocket, consumer, status: active, granted_at: now,
    #     expires_at: now + config.lease_ttl)
    # The lease's id is the double's (N05), drawn here and only here (N15).
    lease = Lease(
        id=outcome.draw_lease_id(),
        pocket=pocket.key,
        consumer=trigger.consumer,
        status=LeaseStatus.ACTIVE,
        granted_at=view.now,
        expires_at=view.now + outcome.config.lease_ttl,
    )
    outcome.create(lease)
    # ensures: pocket.status = leased
    outcome.replace(replace(pocket, status=PocketStatus.LEASED))
    _hand(outcome, pocket, records)
    # ensures: Batch(consumer, lease: Lease{pocket: pocket, consumer: consumer, status: active},
    #     pocket, records: batch)
    # The lease that lookup finds is the one just created.
    outcome.answer(Batch(trigger.consumer, lease.id, pocket.key, records))


# rule HoldRecord ----------------------------------------------------------


def _hold_record_guard(view: View, event: Handed) -> bool:
    # let record = Record{pocket: pocket, key: record_key}
    # requires: exists record
    # requires: record.status = queued
    record = view.state.record(event.pocket, event.record_key)
    return record is not None and record.status is RecordStatus.QUEUED


def _hold_record(view: View, outcome: "Outcome", event: Handed) -> None:
    record = view.state.record(event.pocket, event.record_key)
    assert record is not None
    # ensures: record.status = held
    # ensures: record.held_rank = record.rank
    # ensures: record.held_payload = record.payload
    # ensures: record.mark = unsettled
    # ensures: record.nacked_at = null
    # ensures: record.deferred_position = null
    outcome.replace(
        replace(
            record,
            status=RecordStatus.HELD,
            held_rank=record.rank,
            held_payload=record.payload,
            mark=Mark.UNSETTLED,
            nacked_at=None,
            deferred_position=None,
        )
    )


# rule PullFindsNoPocket ---------------------------------------------------


def _pull_finds_no_pocket_guard(view: View, trigger: Pull) -> bool:
    # requires: not exists Lease{consumer: consumer, status: active}
    # let free = Pockets where status = queued
    # requires: free.count = 0
    return not _has_active_lease(view, trigger.consumer) and not _queued_pockets(view)


def _pull_finds_no_pocket(view: View, outcome: "Outcome", trigger: Pull) -> None:
    # ensures: NoPocketAvailable(consumer: consumer)
    outcome.answer(NoPocketAvailable(trigger.consumer))


PULL = SPECIFICATION.register(
    Table(
        "Pull",
        (
            Rule("PullWithinLease", _pull_within_lease_guard, _pull_within_lease),
            Rule("PullClaimsFrontPocket", _pull_claims_front_pocket_guard, _pull_claims_front_pocket),
            Rule("PullFindsNoPocket", _pull_finds_no_pocket_guard, _pull_finds_no_pocket),
        ),
        NoGuardHolds.REFUSAL,
        lambda view, trigger: PullRefused(trigger.consumer),
    )
)

HANDED = SPECIFICATION.register(
    Table("Handed", (Rule("HoldRecord", _hold_record_guard, _hold_record),), NoGuardHolds.DEFECT)
)
