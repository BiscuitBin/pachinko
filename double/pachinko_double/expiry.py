"""The specification's Lease expiry section: the lapse and the three expiry rules.

`LeaseLapses` waits on a clock trigger, `lease: Lease.expires_at <= now`, so
its table is keyed by that trigger, `Lease.expires_at`. The double has no
background time: `Queue.tick()` reads `now` once, asks `due_lapses` for every
active lease whose `expires_at` has passed, and offers each one to this table
as `LeaseExpiresAt`, one outcome per lease. The trigger's `<= now` is that
filter; the row's guard is the rule's `requires: lease.status = active`, and
when it fails nothing happens.

The row emits `LeaseLapsed`, which the three expiry rules answer in the same
outcome, with disjoint guards:

- `LeaseExpiryParksPocket`: `consecutive_expiries + 1 >= max_consecutive_expiries`,
  whether or not records remain;
- `LeaseExpiryKeepsPocketAtFront`: below the threshold, with a record that
  outlasts the lease; the pocket is queued at its old position, the front;
- `LeaseExpiryIdlesEmptyPocket`: below the threshold, with none.

Each one ends the lease through the lease-end module, `lease_end.end_lease`,
which writes `expired` and `ended_at = now` and sends every held record
`Freed`; the `Freed` table answers each one. Then the row writes the pocket's
own fields: the count raised by one, and its status. No expiry row allots a
position.

Each expiry rule restates `lease.status = active` and `lease.expires_at <= now`.
Under branch A (decision 0019) an action processed after `expires_at` but
before the lapse may have released the lease or moved `expires_at` on, so the
`LeaseLapsed` table answers nothing when no guard holds. Through `tick()` the
lapse and its expiry rule are one outcome, so that answer is reached only by
a hand-built dispatch (spec note N37).
"""

from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from . import lease_end
from .inspection import Snapshot, View
from .model import Lease, LeaseStatus, Pocket, PocketStatus, Timestamp
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class LeaseExpiresAt:
    """The clock trigger `lease: Lease.expires_at <= now`, for the lease with this id (N05)."""

    lease: str


@dataclass(frozen=True)
class LeaseLapsed:
    """The event `LeaseLapsed(lease)`; `lease` is the lease's id (N05)."""

    lease: str


def due_lapses(state: Snapshot, now: Timestamp) -> tuple[Lease, ...]:
    """Every active lease whose `expires_at <= now`, the order one tick lapses them in.

    Smallest `expires_at` first, and among equals in the order the leases
    were granted, the snapshot's order (spec note N34).
    """
    due = (lease for lease in state.leases if lease.status is LeaseStatus.ACTIVE and lease.expires_at <= now)
    return tuple(sorted(due, key=lambda lease: lease.expires_at))


def _lease(view: View, lease_id: str) -> Lease | None:
    for lease in view.state.leases:
        if lease.id == lease_id:
            return lease
    return None


# rule LeaseLapses ---------------------------------------------------------


def _lease_lapses_guard(view: View, trigger: LeaseExpiresAt) -> bool:
    lease = _lease(view, trigger.lease)
    # requires: lease.status = active
    return lease is not None and lease.status is LeaseStatus.ACTIVE


def _lease_lapses(view: View, outcome: "Outcome", trigger: LeaseExpiresAt) -> None:
    # ensures: LeaseLapsed(lease: lease)
    outcome.emit(LeaseLapsed(trigger.lease))


# The three expiry rules ---------------------------------------------------


def _lapsing(view: View, event: LeaseLapsed) -> tuple[Lease, Pocket] | None:
    """The shared head of the three expiry rules: the lease, due, and its leased pocket."""
    lease = _lease(view, event.lease)
    # requires: lease.status = active
    # requires: lease.expires_at <= now
    if lease is None or lease.status is not LeaseStatus.ACTIVE or not lease.expires_at <= view.now:
        return None
    # let pocket = lease.pocket
    (pocket,) = view.state.pockets_keyed(lease.pocket)
    # requires: pocket.status = leased
    return (lease, pocket) if pocket.status is PocketStatus.LEASED else None


def _at_threshold(view: View, pocket: Pocket) -> bool:
    """`pocket.consecutive_expiries + 1 >= config.max_consecutive_expiries`."""
    return pocket.consecutive_expiries + 1 >= view.config.max_consecutive_expiries


def _expire(view: View, outcome: "Outcome", lease: Lease, pocket: Pocket, status: PocketStatus) -> None:
    """The writes every expiry rule shares, then the pocket's status."""
    # ensures: lease.status = expired
    # ensures: lease.ended_at = now
    # ensures: for record in pocket.held_records: Freed(lease: lease, record_key: record.key, ended_at: now)
    lease_end.end_lease(view, outcome, lease, LeaseStatus.EXPIRED, view.now)
    # ensures: pocket.consecutive_expiries = pocket.consecutive_expiries + 1
    # ensures: pocket.status = <status>
    # The pocket's position is untouched: queued, it keeps its place at the front.
    outcome.replace(replace(pocket, status=status, consecutive_expiries=pocket.consecutive_expiries + 1))


# rule LeaseExpiryParksPocket ----------------------------------------------


def _lease_expiry_parks_pocket_guard(view: View, event: LeaseLapsed) -> bool:
    found = _lapsing(view, event)
    # requires: pocket.consecutive_expiries + 1 >= config.max_consecutive_expiries
    return found is not None and _at_threshold(view, found[1])


def _lease_expiry_parks_pocket(view: View, outcome: "Outcome", event: LeaseLapsed) -> None:
    found = _lapsing(view, event)
    assert found is not None
    _expire(view, outcome, *found, PocketStatus.PARKED)


# rule LeaseExpiryKeepsPocketAtFront ---------------------------------------


def _lease_expiry_keeps_pocket_at_front_guard(view: View, event: LeaseLapsed) -> bool:
    found = _lapsing(view, event)
    # requires: pocket.consecutive_expiries + 1 < config.max_consecutive_expiries
    # requires: pocket.records.count > pocket.ending_records.count
    return (
        found is not None
        and not _at_threshold(view, found[1])
        and lease_end.a_record_outlasts_the_lease(view.state, found[1].key)
    )


def _lease_expiry_keeps_pocket_at_front(view: View, outcome: "Outcome", event: LeaseLapsed) -> None:
    found = _lapsing(view, event)
    assert found is not None
    _expire(view, outcome, *found, PocketStatus.QUEUED)


# rule LeaseExpiryIdlesEmptyPocket -----------------------------------------


def _lease_expiry_idles_empty_pocket_guard(view: View, event: LeaseLapsed) -> bool:
    found = _lapsing(view, event)
    # requires: pocket.consecutive_expiries + 1 < config.max_consecutive_expiries
    # requires: pocket.records.count = pocket.ending_records.count
    return (
        found is not None
        and not _at_threshold(view, found[1])
        and lease_end.no_record_outlasts_the_lease(view.state, found[1].key)
    )


def _lease_expiry_idles_empty_pocket(view: View, outcome: "Outcome", event: LeaseLapsed) -> None:
    found = _lapsing(view, event)
    assert found is not None
    # The pocket keeps its position, which no rule reads while it is idle (N27).
    _expire(view, outcome, *found, PocketStatus.IDLE)


LEASE_EXPIRES_AT = SPECIFICATION.register(
    Table(
        "Lease.expires_at",
        (Rule("LeaseLapses", _lease_lapses_guard, _lease_lapses),),
        NoGuardHolds.NOTHING,
    )
)

LEASE_LAPSED = SPECIFICATION.register(
    Table(
        "LeaseLapsed",
        (
            Rule("LeaseExpiryParksPocket", _lease_expiry_parks_pocket_guard, _lease_expiry_parks_pocket),
            Rule(
                "LeaseExpiryKeepsPocketAtFront",
                _lease_expiry_keeps_pocket_at_front_guard,
                _lease_expiry_keeps_pocket_at_front,
            ),
            Rule(
                "LeaseExpiryIdlesEmptyPocket",
                _lease_expiry_idles_empty_pocket_guard,
                _lease_expiry_idles_empty_pocket,
            ),
        ),
        NoGuardHolds.NOTHING,
    )
)
