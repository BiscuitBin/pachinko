"""The `Consuming` surface's exposure: what a consumer reads of its lease.

    let leases = Leases where consumer = consumer and status = active
    exposes:
        for lease in leases:
            lease.pocket.key
            lease.expires_at
            for record in lease.pocket.held_records:
                record.key
                record.held_rank
                record.held_payload
                record.mark

`OneActiveLeasePerConsumer` allows at most one such lease, so `lease_view`
answers one `LeaseView` or None. It reads a snapshot and nothing else: no
clock, no rule, no apply step.

The surface gives the held records no order. The double lists them smallest
position first, which is the order a pull handed them out in; S090 pins it,
so that row is `impl-choice` (spec note N23).
"""

from dataclasses import dataclass

from .inspection import Snapshot
from .model import Mark, Timestamp


@dataclass(frozen=True)
class HeldRecordView:
    """One held record as the surface exposes it."""

    key: str
    held_rank: int
    held_payload: bytes
    mark: Mark


@dataclass(frozen=True)
class LeaseView:
    """The consumer's active lease as the surface exposes it.

    `pocket` is the pocket's key. The surface exposes no lease identity, so
    neither does the view; a consumer names its lease by the id its batch
    carried (spec note N05).
    """

    pocket: str
    expires_at: Timestamp
    records: tuple[HeldRecordView, ...]


def lease_view(state: Snapshot, consumer_id: str) -> LeaseView | None:
    """The consumer's active lease and its held records, or None without one."""
    leases = state.active_leases_by_consumer.get(consumer_id, ())
    if not leases:
        return None
    (lease,) = leases
    held = sorted(state.held_by_pocket[lease.pocket], key=lambda record: record.position)
    records = []
    for record in held:
        assert record.held_rank is not None and record.held_payload is not None and record.mark is not None
        records.append(HeldRecordView(record.key, record.held_rank, record.held_payload, record.mark))
    return LeaseView(lease.pocket, lease.expires_at, tuple(records))
