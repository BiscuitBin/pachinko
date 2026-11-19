"""The Renew and release section: renew, the refusal of a renew by a
consumer that never joined, and the Renew table's refusal of a lease id never
issued.

Renew answers nothing (spec note N13), so a test reads the lease's new
`expires_at` through the snapshot. The Renew table's own refusal, for an
ended lease or another consumer's, is fencing, which PD03 tests (S052, S070).
"""

import unittest

from pachinko_double.inspection import View
from pachinko_double.model import Consumer, LeaseStatus, PocketStatus
from pachinko_double.queue import Queue
from pachinko_double.refusals import LeaseFenced, UnknownConsumer
from pachinko_double.settle import RENEW, Renew

from .fixtures import PUBLISHER, T0, batch, held, lease, pocket, scenario_setting, snapshot


def short_lease() -> tuple[Queue, object, str]:
    """A 10 ms lease_ttl and one record held under c1's lease, pulled at T0."""
    queue, clock, _ = scenario_setting(lease_ttl=10)
    queue.publish(PUBLISHER, "pk", "k", 1, b"v")
    return queue, clock, batch(queue, "c1", 10).lease


class RenewTest(unittest.TestCase):
    # scenario: S050
    def test_a_renew_extends_the_lease_to_now_plus_lease_ttl(self) -> None:
        queue, clock, lease_id = short_lease()
        clock.advance(6)
        self.assertIsNone(queue.renew("c1", lease_id))
        (lease,) = queue.snapshot().leases
        self.assertEqual((lease.id, lease.status), (lease_id, LeaseStatus.ACTIVE))
        self.assertEqual(lease.expires_at, T0 + 16)
        self.assertEqual(lease.granted_at, T0, "a renew is not a new grant")

    # impl-choice (spec note N18): the Renew trigger takes a Consumer entity, so
    # the double refuses an id that never joined before any rule is consulted.
    def test_a_renew_by_a_consumer_that_never_joined_is_refused_and_changes_nothing(self) -> None:
        queue, clock, lease_id = short_lease()
        before, reads = queue.snapshot(), clock.reads
        with self.assertRaises(UnknownConsumer) as raised:
            queue.renew("stranger", lease_id)
        self.assertEqual(raised.exception.consumer_id, "stranger")
        self.assertEqual(queue.snapshot(), before)
        self.assertEqual(clock.reads, reads)


class RenewGuardTest(unittest.TestCase):
    """Renew's guard is the specification's in full: the lease is active and
    the consumer's. It is asked directly, on a hand-built state with an
    ended lease, which no PD02 trigger reaches; the refusal itself is PD03's
    to test."""

    def test_it_holds_only_for_the_consumers_own_active_lease(self) -> None:
        state = snapshot(
            consumers=(Consumer("c1"), Consumer("c2")),
            pockets=(pocket("p", PocketStatus.LEASED, position=0), pocket("q", PocketStatus.IDLE, position=2)),
            records=(held("p", "k", position=1),),
            leases=(
                lease("lease-1", "p", "c1"),
                lease("lease-0", "q", "c1", LeaseStatus.RELEASED, granted_at=0, expires_at=10, ended_at=5),
            ),
        )
        view = View(state, T0)
        (row,) = RENEW.rows
        cases = {
            ("c1", "lease-1"): True,
            ("c2", "lease-1"): False,
            ("c1", "lease-0"): False,
            ("c1", "no-such-lease"): False,
        }
        self.assertEqual({case: row.guard(view, Renew(*case)) for case in cases}, cases)


class RenewRefusalTest(unittest.TestCase):
    # The Renew table's refusal (spec note N20) on the one input that needs no
    # ended lease and no second consumer: a lease id the source never issued.
    # The ended-lease and other-consumer cases are PD03's S052 and S070.
    def test_a_renew_naming_a_lease_id_never_issued_is_refused_and_changes_nothing(self) -> None:
        queue, clock, lease_id = short_lease()
        clock.advance(3)
        before = queue.snapshot()
        with self.assertRaises(LeaseFenced) as raised:
            queue.renew("c1", "never-issued")
        refused = raised.exception
        self.assertEqual((refused.action, refused.consumer_id, refused.lease_id), ("Renew", "c1", "never-issued"))
        self.assertEqual(queue.snapshot(), before)
        self.assertEqual([lease.id for lease in before.leases], [lease_id])
