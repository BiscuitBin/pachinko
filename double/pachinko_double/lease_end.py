"""The specification's Lease end section: how a lease ends, and the Freed table.

A lease ends by release or by lapse, and the specification writes the end the
same way in all five rules that end one: the lease takes its end status and
`ended_at = now`, and every record of the held set is sent `Freed`. Six rules
answer `Freed`, one per row of the reconciliation table, and none of them
reads how the lease ended. So this module owns the end, and the rules that end
a lease call it:

- `end_lease(view, outcome, lease, ended, ended_at)` is the shared end of a
  lease: the status, `ended_at`, and one `Freed` per held record. Its three
  arguments are the lease, how it ended (`LeaseStatus.RELEASED` or
  `LeaseStatus.EXPIRED`) and when. The caller writes the pocket's own fields,
  which differ from rule to rule.
- `ending_records(state, pocket_key)` is `pocket.ending_records`: held,
  marked acked or nacked, and with nothing deferred behind it. The snapshot
  computes the set once (inspection.py), where the specification's where
  clause is written in full.
- `a_record_outlasts_the_lease` and `no_record_outlasts_the_lease` are the
  comparison that decides the pocket's fate, `pocket.records.count` against
  `pocket.ending_records.count`, which the release guards and the lapse
  guards read.
- `FREED` is the Freed table. A `Freed` no row answers is a defect.

`Freed` carries `ended_at`, the timestamp the ending rule wrote to
`lease.ended_at`, and a row that creates an entry stamps it from that, never
from its own `now`: both dead letters' `expires_at`, and the `shed_at` and
`expires_at` of the held version SupersedeUnsettledHeldVersion sheds.

The table has six rows, one per row of the specification's reconciliation
table: three with nothing deferred, which guard
`record.deferred_position = null`, and three for a newer version deferred
behind the held one, which guard `record.deferred_position != null`. For
each mark the two guards are disjoint. Behind an unsettled version the newer
version is queued at the held version's position; behind an acked or nacked
one, at its arrival position, `deferred_position`. No row allots a position:
the arrival position was allotted when the newer version arrived
(DeferBehindHeldVersion), and the counter does not advance again here.

The `Freed` events of one end go out smallest held position first, as the
Lease end comment and the `HeldRecordsOldestFirst` guarantee state. That order
decides the order of the entries one end creates (spec notes N25 and N29).
"""

from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from .model import DeadLetter, Lease, LeaseStatus, Mark, Record, RecordStatus, ShedReason, SupersededRecord, Timestamp
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .inspection import Snapshot, View
    from .queue import Outcome


@dataclass(frozen=True)
class Freed:
    """The event `Freed(lease, record_key, ended_at)`; `lease` is the lease's id (N05)."""

    lease: str
    record_key: str
    ended_at: Timestamp


# The shared end of a lease ------------------------------------------------


def end_lease(view: "View", outcome: "Outcome", lease: Lease, ended: LeaseStatus, ended_at: Timestamp) -> None:
    """End `lease` as `ended`, at `ended_at`, and free every record it holds."""
    if ended is LeaseStatus.ACTIVE:
        raise ValueError("a lease ends released or expired, not active")
    # ensures: lease.status = released (or expired)
    # ensures: lease.ended_at = now
    outcome.replace(replace(lease, status=ended, ended_at=ended_at))
    # ensures: for record in pocket.held_records:
    #     Freed(lease: lease, record_key: record.key, ended_at: now)
    for record in sorted(view.state.held_by_pocket[lease.pocket], key=lambda held: held.position):
        outcome.emit(Freed(lease.id, record.key, ended_at))


def ending_records(state: "Snapshot", pocket_key: str) -> tuple[Record, ...]:
    """`pocket.ending_records`: `records where status = held and mark != unsettled
    and deferred_position = null`."""
    return state.ending_by_pocket[pocket_key]


def a_record_outlasts_the_lease(state: "Snapshot", pocket_key: str) -> bool:
    """`pocket.records.count > pocket.ending_records.count`."""
    return len(state.records_by_pocket[pocket_key]) > len(ending_records(state, pocket_key))


def no_record_outlasts_the_lease(state: "Snapshot", pocket_key: str) -> bool:
    """`pocket.records.count = pocket.ending_records.count`."""
    return len(state.records_by_pocket[pocket_key]) == len(ending_records(state, pocket_key))


# The Freed table ----------------------------------------------------------


def _freed_record(view: "View", event: Freed) -> tuple[Lease, Record] | None:
    """The ended lease and `Record{pocket: lease.pocket, key: record_key}`, held.

    A `Freed` naming a lease the state lacks finds nothing, so no row answers
    it and the table's own answer, a `Defect`, applies.
    """
    lease = next((lease for lease in view.state.leases if lease.id == event.lease), None)
    if lease is None:
        return None
    # let record = Record{pocket: lease.pocket, key: record_key}
    # requires: exists record
    # requires: record.status = held
    record = view.state.record(lease.pocket, event.record_key)
    if record is None or record.status is not RecordStatus.HELD:
        return None
    return lease, record


def _freed_with(view: "View", event: Freed, mark: Mark) -> tuple[Lease, Record] | None:
    """`_freed_record`, when the record's mark is `mark` and nothing is deferred."""
    found = _freed_record(view, event)
    if found is None:
        return None
    record = found[1]
    # requires: record.mark = <mark>
    # requires: record.deferred_position = null
    return found if record.mark is mark and record.deferred_position is None else None


def _freed_deferred(view: "View", event: Freed, mark: Mark) -> tuple[Lease, Record] | None:
    """`_freed_record`, when the record's mark is `mark` and a newer version is deferred."""
    found = _freed_record(view, event)
    if found is None:
        return None
    record = found[1]
    # requires: record.mark = <mark>
    # requires: record.deferred_position != null
    return found if record.mark is mark and record.deferred_position is not None else None


def _queued_at_arrival(record: Record) -> Record:
    """The newer version queued at its arrival position, which was allotted when
    it arrived: nothing is allotted here."""
    # let arrival_position = record.deferred_position
    # ensures: record.position = arrival_position
    assert record.deferred_position is not None
    return replace(_queued_again(record), position=record.deferred_position)


def _queued_again(record: Record) -> Record:
    # ensures: record.status = queued
    # ensures: record.held_rank = null, record.held_payload = null, record.mark = null,
    #     record.nacked_at = null, record.deferred_position = null
    return replace(
        record,
        status=RecordStatus.QUEUED,
        held_rank=None,
        held_payload=None,
        mark=None,
        nacked_at=None,
        deferred_position=None,
    )


# rule RequeueUnsettledRecord ----------------------------------------------


def _requeue_unsettled_record_guard(view: "View", event: Freed) -> bool:
    return _freed_with(view, event, Mark.UNSETTLED) is not None


def _requeue_unsettled_record(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_with(view, event, Mark.UNSETTLED)
    assert found is not None
    # The record keeps its position: a key nobody acked or nacked keeps its place.
    outcome.replace(_queued_again(found[1]))


# rule SupersedeUnsettledHeldVersion ---------------------------------------


def _supersede_unsettled_held_version_guard(view: "View", event: Freed) -> bool:
    return _freed_deferred(view, event, Mark.UNSETTLED) is not None


def _supersede_unsettled_held_version(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_deferred(view, event, Mark.UNSETTLED)
    assert found is not None
    lease, record = found
    # let held_rank = record.held_rank
    # let held_payload = record.held_payload
    assert record.held_rank is not None and record.held_payload is not None
    # ensures: SupersededRecord.created(pocket, key: record_key, rank: held_rank, payload: held_payload,
    #     reason: superseded, shed_at: ended_at, expires_at: ended_at + config.superseded_ttl)
    outcome.create(
        SupersededRecord(
            pocket=lease.pocket,
            key=event.record_key,
            rank=record.held_rank,
            payload=record.held_payload,
            reason=ShedReason.SUPERSEDED,
            shed_at=event.ended_at,
            expires_at=event.ended_at + view.config.superseded_ttl,
        )
    )
    # The newer version is queued at the held version's position: a key
    # nobody acked or nacked keeps its place, so position is left as it is.
    outcome.replace(_queued_again(record))


# rule ConsumeAckedRecord --------------------------------------------------


def _consume_acked_record_guard(view: "View", event: Freed) -> bool:
    return _freed_with(view, event, Mark.ACKED) is not None


def _consume_acked_record(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_with(view, event, Mark.ACKED)
    assert found is not None
    # ensures: not exists record
    outcome.remove(found[1])


# rule ConsumeAckedVersionQueueDeferred ------------------------------------


def _consume_acked_version_queue_deferred_guard(view: "View", event: Freed) -> bool:
    return _freed_deferred(view, event, Mark.ACKED) is not None


def _consume_acked_version_queue_deferred(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_deferred(view, event, Mark.ACKED)
    assert found is not None
    # The acked version is consumed: recorded nowhere. Behind it the newer
    # version is a new arrival and stands where it arrived.
    outcome.replace(_queued_at_arrival(found[1]))


# rule DeadLetterNackedRecord ----------------------------------------------


def _dead_letter_nacked_record_guard(view: "View", event: Freed) -> bool:
    return _freed_with(view, event, Mark.NACKED) is not None


def _dead_letter_nacked_record(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_with(view, event, Mark.NACKED)
    assert found is not None
    lease, record = found
    # let held_rank = record.held_rank
    # let held_payload = record.held_payload
    # let nacked_at = record.nacked_at
    assert record.held_rank is not None and record.held_payload is not None and record.nacked_at is not None
    # ensures: DeadLetter.created(pocket, key: record_key, rank: held_rank, payload: held_payload,
    #     nacked_by: lease.consumer, nacked_at: nacked_at, expires_at: ended_at + config.dead_letter_ttl)
    outcome.create(
        DeadLetter(
            pocket=lease.pocket,
            key=event.record_key,
            rank=record.held_rank,
            payload=record.held_payload,
            nacked_by=lease.consumer,
            nacked_at=record.nacked_at,
            expires_at=event.ended_at + view.config.dead_letter_ttl,
        )
    )
    # ensures: not exists record
    outcome.remove(record)


# rule DeadLetterNackedVersionQueueDeferred --------------------------------


def _dead_letter_nacked_version_queue_deferred_guard(view: "View", event: Freed) -> bool:
    return _freed_deferred(view, event, Mark.NACKED) is not None


def _dead_letter_nacked_version_queue_deferred(view: "View", outcome: "Outcome", event: Freed) -> None:
    found = _freed_deferred(view, event, Mark.NACKED)
    assert found is not None
    lease, record = found
    # let held_rank = record.held_rank
    # let held_payload = record.held_payload
    # let nacked_at = record.nacked_at
    assert record.held_rank is not None and record.held_payload is not None and record.nacked_at is not None
    # ensures: DeadLetter.created(pocket, key: record_key, rank: held_rank, payload: held_payload,
    #     nacked_by: lease.consumer, nacked_at: nacked_at, expires_at: ended_at + config.dead_letter_ttl)
    outcome.create(
        DeadLetter(
            pocket=lease.pocket,
            key=event.record_key,
            rank=record.held_rank,
            payload=record.held_payload,
            nacked_by=lease.consumer,
            nacked_at=record.nacked_at,
            expires_at=event.ended_at + view.config.dead_letter_ttl,
        )
    )
    # The newer version is a new arrival and stands where it arrived.
    outcome.replace(_queued_at_arrival(record))


FREED = SPECIFICATION.register(
    Table(
        "Freed",
        (
            Rule("RequeueUnsettledRecord", _requeue_unsettled_record_guard, _requeue_unsettled_record),
            Rule(
                "SupersedeUnsettledHeldVersion",
                _supersede_unsettled_held_version_guard,
                _supersede_unsettled_held_version,
            ),
            Rule("ConsumeAckedRecord", _consume_acked_record_guard, _consume_acked_record),
            Rule(
                "ConsumeAckedVersionQueueDeferred",
                _consume_acked_version_queue_deferred_guard,
                _consume_acked_version_queue_deferred,
            ),
            Rule("DeadLetterNackedRecord", _dead_letter_nacked_record_guard, _dead_letter_nacked_record),
            Rule(
                "DeadLetterNackedVersionQueueDeferred",
                _dead_letter_nacked_version_queue_deferred_guard,
                _dead_letter_nacked_version_queue_deferred,
            ),
        ),
        NoGuardHolds.DEFECT,
    )
)
