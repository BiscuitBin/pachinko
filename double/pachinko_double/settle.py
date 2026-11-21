"""The specification's Ack and nack, and Renew and release, sections: the
actions a consumer takes on its lease.

Each action names its lease by id (spec note N05), and each table's rows
require the lease to be active and the consumer's. With that false, as for
an ended lease, another consumer's lease or an id that names no lease, no row
holds and the action is refused with `LeaseFenced` (spec note N20). An action
naming an ended lease is refused even when the same consumer holds the same
pocket again under a new lease (S068): leases are compared by id.
"""

from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from . import lease_end
from .inspection import View
from .model import Lease, LeaseStatus, Mark, Pocket, PocketStatus, Record, RecordStatus
from .refusals import LeaseFenced, MarkConflict, RecordNotHeld, Refusal, ReleaseRefused
from .rules import SPECIFICATION, NoGuardHolds, RefusalFor, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class Ack:
    """The trigger `Ack(consumer, lease, record_key)`: a joined consumer's id and a lease id."""

    consumer: str
    lease: str
    record_key: str


@dataclass(frozen=True)
class Nack:
    """The trigger `Nack(consumer, lease, record_key)`: a joined consumer's id and a lease id."""

    consumer: str
    lease: str
    record_key: str


type Marking = Ack | Nack


@dataclass(frozen=True)
class Renew:
    """The trigger `Renew(consumer, lease)`: a joined consumer's id and a lease id."""

    consumer: str
    lease: str


@dataclass(frozen=True)
class Release:
    """The trigger `Release(consumer, lease)`: a joined consumer's id and a lease id."""

    consumer: str
    lease: str


def _lease(view: View, lease_id: str) -> Lease | None:
    """The lease with this id, or None when no lease has it."""
    for lease in view.state.leases:
        if lease.id == lease_id:
            return lease
    return None


def _acting_lease(view: View, consumer: str, lease_id: str) -> Lease | None:
    """The lease an action names, when it is active and the consumer's."""
    lease = _lease(view, lease_id)
    # requires: lease.status = active
    # requires: lease.consumer = consumer
    if lease is None or lease.status is not LeaseStatus.ACTIVE or lease.consumer != consumer:
        return None
    return lease


def _held_record(view: View, trigger: Marking) -> tuple[Lease, Pocket, Record] | None:
    """The lease, its pocket and the held record a mark names, or None.

    The shared head of the four mark rules: the lease is active and the
    consumer's, and `Record{pocket: lease.pocket, key: record_key}` exists
    and is held.
    """
    lease = _acting_lease(view, trigger.consumer, trigger.lease)
    if lease is None:
        return None
    # let pocket = lease.pocket
    (pocket,) = view.state.pockets_keyed(lease.pocket)
    # let record = Record{pocket: pocket, key: record_key}
    # requires: exists record
    # requires: record.status = held
    record = view.state.record(pocket.key, trigger.record_key)
    if record is None or record.status is not RecordStatus.HELD:
        return None
    return lease, pocket, record


def _marked(view: View, trigger: Marking, mark: Mark) -> tuple[Lease, Pocket, Record] | None:
    """`_held_record`, when the record's mark is `mark`."""
    found = _held_record(view, trigger)
    # requires: record.mark = <mark>
    return found if found is not None and found[2].mark is mark else None


def _mark_refusal(action: str) -> RefusalFor:
    """The refusal of an Ack or a Nack no row answers, naming the first `requires` that fails.

    The lease is checked before the record, so an action on a lease that is
    not active and the consumer's is `LeaseFenced` whatever key it names
    (S070). With the lease good, a key its pocket does not hold is
    `RecordNotHeld`, and a held record with the other mark is `MarkConflict`.
    The names are the double's (spec note N21).
    """

    def refusal(view: View, trigger: Marking) -> Refusal:
        if _acting_lease(view, trigger.consumer, trigger.lease) is None:
            return LeaseFenced(action, trigger.consumer, trigger.lease)
        found = _held_record(view, trigger)
        if found is None:
            return RecordNotHeld(action, trigger.consumer, trigger.lease, trigger.record_key)
        # The record is held under the lease, so its mark is the other one:
        # the first mark stands, and no rule covers changing it.
        mark = found[2].mark
        assert mark is not None
        return MarkConflict(action, trigger.consumer, trigger.lease, trigger.record_key, mark)

    return refusal


def _extend(view: View, outcome: "Outcome", lease: Lease) -> None:
    # ensures: lease.expires_at = now + config.lease_ttl
    outcome.replace(replace(lease, expires_at=view.now + outcome.config.lease_ttl))


# rule AckMarksRecord ------------------------------------------------------


def _ack_marks_record_guard(view: View, trigger: Ack) -> bool:
    return _marked(view, trigger, Mark.UNSETTLED) is not None


def _ack_marks_record(view: View, outcome: "Outcome", trigger: Ack) -> None:
    found = _marked(view, trigger, Mark.UNSETTLED)
    assert found is not None
    lease, pocket, record = found
    _extend(view, outcome, lease)
    # ensures: pocket.consecutive_expiries = 0
    outcome.replace(replace(pocket, consecutive_expiries=0))
    # ensures: record.mark = acked
    outcome.replace(replace(record, mark=Mark.ACKED))


# rule AckRepeats ----------------------------------------------------------


def _ack_repeats_guard(view: View, trigger: Ack) -> bool:
    return _marked(view, trigger, Mark.ACKED) is not None


def _ack_repeats(view: View, outcome: "Outcome", trigger: Ack) -> None:
    found = _marked(view, trigger, Mark.ACKED)
    assert found is not None
    # The first mark stands: the repeat changes nothing but the lease's expiry.
    _extend(view, outcome, found[0])


# rule NackMarksRecord -----------------------------------------------------


def _nack_marks_record_guard(view: View, trigger: Nack) -> bool:
    return _marked(view, trigger, Mark.UNSETTLED) is not None


def _nack_marks_record(view: View, outcome: "Outcome", trigger: Nack) -> None:
    found = _marked(view, trigger, Mark.UNSETTLED)
    assert found is not None
    lease, pocket, record = found
    _extend(view, outcome, lease)
    # ensures: pocket.consecutive_expiries = 0
    outcome.replace(replace(pocket, consecutive_expiries=0))
    # ensures: record.mark = nacked
    # ensures: record.nacked_at = now
    outcome.replace(replace(record, mark=Mark.NACKED, nacked_at=view.now))


# rule NackRepeats ---------------------------------------------------------


def _nack_repeats_guard(view: View, trigger: Nack) -> bool:
    return _marked(view, trigger, Mark.NACKED) is not None


def _nack_repeats(view: View, outcome: "Outcome", trigger: Nack) -> None:
    found = _marked(view, trigger, Mark.NACKED)
    assert found is not None
    # The first mark stands, its nacked_at with it (S086): only the expiry moves.
    _extend(view, outcome, found[0])


# The release rules ---------------------------------------------------------


def _releasing(view: View, trigger: Release) -> tuple[Lease, Pocket] | None:
    """The shared head of the two release rules: the lease and its leased pocket."""
    lease = _acting_lease(view, trigger.consumer, trigger.lease)
    if lease is None:
        return None
    # let pocket = lease.pocket
    (pocket,) = view.state.pockets_keyed(lease.pocket)
    # requires: pocket.status = leased
    return (lease, pocket) if pocket.status is PocketStatus.LEASED else None


def _release_refusal(view: View, trigger: Release) -> Refusal:
    """`LeaseFenced` when the lease is not active and the consumer's.

    Otherwise the pocket is not leased, which no state that holds the
    invariants allows (`UnleasedHasNoActiveLease`), so `Queue` never raises
    `ReleaseRefused` (spec note N26).
    """
    if _acting_lease(view, trigger.consumer, trigger.lease) is None:
        return LeaseFenced("Release", trigger.consumer, trigger.lease)
    return ReleaseRefused(trigger.consumer, trigger.lease)


# rule ReleaseRequeuesPocket -----------------------------------------------


def _release_requeues_pocket_guard(view: View, trigger: Release) -> bool:
    found = _releasing(view, trigger)
    # requires: pocket.records.count > pocket.ending_records.count
    return found is not None and lease_end.a_record_outlasts_the_lease(view.state, found[1].key)


def _release_requeues_pocket(view: View, outcome: "Outcome", trigger: Release) -> None:
    found = _releasing(view, trigger)
    assert found is not None
    lease, pocket = found
    # let position = queue.next_position
    # ensures: queue.next_position = position + 1
    position = outcome.allot_position()
    lease_end.end_lease(view, outcome, lease, LeaseStatus.RELEASED, view.now)
    # ensures: pocket.status = queued
    # ensures: pocket.position = position
    # ensures: pocket.consecutive_expiries = 0
    outcome.replace(replace(pocket, status=PocketStatus.QUEUED, position=position, consecutive_expiries=0))


# rule ReleaseIdlesEmptyPocket ---------------------------------------------


def _release_idles_empty_pocket_guard(view: View, trigger: Release) -> bool:
    found = _releasing(view, trigger)
    # requires: pocket.records.count = pocket.ending_records.count
    return found is not None and lease_end.no_record_outlasts_the_lease(view.state, found[1].key)


def _release_idles_empty_pocket(view: View, outcome: "Outcome", trigger: Release) -> None:
    found = _releasing(view, trigger)
    assert found is not None
    lease, pocket = found
    lease_end.end_lease(view, outcome, lease, LeaseStatus.RELEASED, view.now)
    # ensures: pocket.status = idle
    # ensures: pocket.consecutive_expiries = 0
    # The pocket keeps its position, which no rule reads while it is idle (N27).
    outcome.replace(replace(pocket, status=PocketStatus.IDLE, consecutive_expiries=0))


# rule Renew ---------------------------------------------------------------


def _renew_guard(view: View, trigger: Renew) -> bool:
    lease = _lease(view, trigger.lease)
    # requires: lease.status = active
    # requires: lease.consumer = consumer
    return lease is not None and lease.status is LeaseStatus.ACTIVE and lease.consumer == trigger.consumer


def _renew(view: View, outcome: "Outcome", trigger: Renew) -> None:
    lease = _lease(view, trigger.lease)
    assert lease is not None
    # ensures: lease.expires_at = now + config.lease_ttl
    outcome.replace(replace(lease, expires_at=view.now + outcome.config.lease_ttl))


ACK = SPECIFICATION.register(
    Table(
        "Ack",
        (
            Rule("AckMarksRecord", _ack_marks_record_guard, _ack_marks_record),
            Rule("AckRepeats", _ack_repeats_guard, _ack_repeats),
        ),
        NoGuardHolds.REFUSAL,
        _mark_refusal("Ack"),
    )
)

RENEW = SPECIFICATION.register(
    Table(
        "Renew",
        (Rule("Renew", _renew_guard, _renew),),
        NoGuardHolds.REFUSAL,
        lambda view, trigger: LeaseFenced("Renew", trigger.consumer, trigger.lease),
    )
)

NACK = SPECIFICATION.register(
    Table(
        "Nack",
        (
            Rule("NackMarksRecord", _nack_marks_record_guard, _nack_marks_record),
            Rule("NackRepeats", _nack_repeats_guard, _nack_repeats),
        ),
        NoGuardHolds.REFUSAL,
        _mark_refusal("Nack"),
    )
)

RELEASE = SPECIFICATION.register(
    Table(
        "Release",
        (
            Rule("ReleaseRequeuesPocket", _release_requeues_pocket_guard, _release_requeues_pocket),
            Rule("ReleaseIdlesEmptyPocket", _release_idles_empty_pocket_guard, _release_idles_empty_pocket),
        ),
        NoGuardHolds.REFUSAL,
        _release_refusal,
    )
)
