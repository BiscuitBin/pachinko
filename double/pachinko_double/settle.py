"""The specification's Renew and release section, and later Ack and nack:
the actions a consumer takes on its lease.

PD02 lands the Renew table. Renew names its lease by id (spec note N05), and
its guard is the specification's in full: the lease is active and the
consumer's. When the guard does not hold, as for an ended lease, another
consumer's lease or an id that names no lease, the renew is refused with
`LeaseFenced` and changes nothing (spec note N20). PD03 lands the tests of
that refusal with fencing for an ended lease and another consumer's (S052,
S070), and the Ack, Nack and Release tables beside this one.
"""

from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from .inspection import View
from .model import Lease, LeaseStatus
from .refusals import LeaseFenced
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class Renew:
    """The trigger `Renew(consumer, lease)`: a joined consumer's id and a lease id."""

    consumer: str
    lease: str


def _lease(view: View, lease_id: str) -> Lease | None:
    """The lease with this id, or None when no lease has it."""
    for lease in view.state.leases:
        if lease.id == lease_id:
            return lease
    return None


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


RENEW = SPECIFICATION.register(
    Table(
        "Renew",
        (Rule("Renew", _renew_guard, _renew),),
        NoGuardHolds.REFUSAL,
        lambda trigger: LeaseFenced("Renew", trigger.consumer, trigger.lease),
    )
)
