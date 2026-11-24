"""The Renew and release section: renew, release and fencing.

Renew and release answer nothing (spec note N13), so a test reads the lease's
new `expires_at`, its end and the pocket's fate through the snapshot. A
release's `Freed` rows are the lease-end module's (test_lease_end.py for its
rows on hand-built states). Every refusal compares the complete snapshot
before and after: a renew naming a lease id never issued, a second release,
and fencing, an action on an ended lease or another consumer's (S052, S057,
S068, S070).
"""

import unittest
from collections.abc import Callable

from pachinko_double.inspection import Snapshot, View
from pachinko_double.model import Consumer, Lease, LeaseStatus, Mark, Pocket, PocketStatus, Record, RecordStatus
from pachinko_double.pull import NoPocketAvailable
from pachinko_double.queue import Queue
from pachinko_double.refusals import LeaseFenced, UnknownConsumer
from pachinko_double.rules import Registry, Table, dispatch
from pachinko_double.settle import ACK, NACK, RELEASE, RENEW, Ack, Nack, Release, Renew

from .fixtures import (
    PUBLISHER,
    T0,
    batch,
    config,
    held,
    lease,
    pocket,
    queued,
    refused,
    scenario_queue,
    scenario_setting,
    snapshot,
    two_pockets,
)
from .test_publish import pocket_order
from .test_pull import RecordingOutcome


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


def records_of(state: Snapshot, pocket_key: str) -> dict[str, Record]:
    """Pocket `pocket_key`'s records, by key."""
    return {record.key: record for record in state.records_by_pocket[pocket_key]}


def ended_lease(state: Snapshot, lease_id: str) -> Lease:
    """The lease with this id; an ended lease stays in the state (spec note N24)."""
    (found,) = (lease for lease in state.leases if lease.id == lease_id)
    return found


class ReleaseTest(unittest.TestCase):
    # scenario: S053
    def test_a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place(self) -> None:
        queue, clock = two_pockets()
        a2 = records_of(queue.snapshot(), "a")["a2"]
        lease_id = batch(queue, "c1", 10).lease
        queue.ack("c1", lease_id, "a1")
        a1 = records_of(queue.snapshot(), "a")["a1"]
        self.assertEqual(
            (a1.status, a1.mark), (RecordStatus.HELD, Mark.ACKED), "an acked record stays until the lease ends"
        )
        clock.advance(7)
        next_position = queue.snapshot().next_position
        self.assertIsNone(queue.release("c1", lease_id))
        state = queue.snapshot()
        ended = ended_lease(state, lease_id)
        self.assertEqual(
            (ended.consumer, ended.status, ended.granted_at, ended.ended_at),
            ("c1", LeaseStatus.RELEASED, T0, T0 + 7),
        )
        (a,) = state.pockets_keyed("a")
        self.assertEqual((a.status, a.position, a.consecutive_expiries), (PocketStatus.QUEUED, next_position, 0))
        self.assertEqual(state.next_position, next_position + 1)
        self.assertEqual(records_of(state, "a"), {"a2": a2}, "a2 is queued in place, the acked a1 is consumed")
        self.assertEqual(pocket_order(state), ["b", "a"])
        self.assertEqual((state.active_leases_by_consumer["c1"], queue.lease_view("c1")), ((), None))
        self.assertEqual((state.superseded, state.dead_letters), ((), ()))
        self.assertEqual(batch(queue, "c1", 10).pocket, "b")

    # scenario: S055
    def test_a_release_idles_an_empty_pocket(self) -> None:
        queue, _ = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        for key in ("a1", "a2"):
            queue.ack("c1", lease_id, key)
        before = queue.snapshot()
        (a,) = before.pockets_keyed("a")
        queue.release("c1", lease_id)
        state = queue.snapshot()
        ended = ended_lease(state, lease_id)
        self.assertEqual((ended.status, ended.ended_at), (LeaseStatus.RELEASED, T0))
        (idle,) = state.pockets_keyed("a")
        self.assertEqual(
            (idle.status, idle.consecutive_expiries, idle.position),
            (PocketStatus.IDLE, 0, a.position),
            "an idle pocket keeps its position (spec note N27)",
        )
        self.assertEqual(records_of(state, "a"), {}, "IdleHasNoRecords")
        self.assertEqual(state.next_position, before.next_position, "an idle pocket takes no position")
        self.assertEqual(pocket_order(state), ["b"])
        self.assertEqual(batch(queue, "c1", 10).pocket, "b")
        self.assertEqual(queue.pull("c2", 10), NoPocketAvailable("c2"))

    # scenario: S057
    def test_a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal(self) -> None:
        queue, _, lease_id = short_lease()
        queue.release("c1", lease_id)
        ended = ended_lease(queue.snapshot(), lease_id)
        self.assertEqual((ended.granted_at, ended.status, ended.ended_at), (T0, LeaseStatus.RELEASED, T0))
        self.assertGreaterEqual(ended.ended_at, ended.granted_at, "EndedAfterGrant")
        again = refused(self, queue, LeaseFenced, lambda: queue.release("c1", lease_id))
        self.assertEqual((again.action, again.consumer_id, again.lease_id), ("Release", "c1", lease_id))

    # scenario: S035
    def test_a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket(self) -> None:
        queue, _ = two_pockets()
        lease_id = batch(queue, "c1", 1).lease
        again = batch(queue, "c1", 1)
        self.assertEqual((again.lease, again.pocket), (lease_id, "a"))
        theirs = batch(queue, "c2", 10).lease
        self.assertEqual(pocket_order(queue.snapshot()), [], "no pocket is queued")
        empty = batch(queue, "c1", 10)
        self.assertEqual((empty.lease, empty.pocket, empty.records), (lease_id, "a", ()))
        queue.release("c2", theirs)
        still = batch(queue, "c1", 10)
        self.assertEqual((still.lease, still.pocket), (lease_id, "a"))
        queue.release("c1", lease_id)
        other = batch(queue, "c1", 10)
        self.assertNotEqual(other.lease, lease_id)
        self.assertEqual(other.pocket, "b")

    # impl-choice (spec note N22): the Release trigger takes a Consumer entity, so
    # the double refuses an id that never joined before any rule is consulted.
    def test_a_release_by_a_consumer_that_never_joined_is_refused_and_reads_no_clock(self) -> None:
        queue, clock, lease_id = short_lease()
        reads = clock.reads
        unknown = refused(self, queue, UnknownConsumer, lambda: queue.release("nobody", lease_id))
        self.assertEqual((unknown.consumer_id, clock.reads), ("nobody", reads))


class FencingTest(unittest.TestCase):
    """Ack, nack, renew and release each require their lease to be active and
    the consumer's; the refusal is `LeaseFenced`, and it changes nothing."""

    # scenario: S051
    def test_renewals_do_not_stack(self) -> None:
        queue, clock, lease_id = short_lease()
        clock.advance(4)
        queue.ack("c1", lease_id, "k")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(view.expires_at, T0 + 14)
        clock.advance(2)
        queue.renew("c1", lease_id)
        first = only_lease(queue).expires_at
        queue.renew("c1", lease_id)
        second = only_lease(queue).expires_at
        self.assertEqual((first, second), (T0 + 16, T0 + 16))
        self.assertEqual(len(queue.snapshot().leases), 1)

    # scenario: S052
    def test_a_renew_of_an_ended_lease_is_rejected_and_moves_nothing(self) -> None:
        queue, clock, lease_id = short_lease()
        queue.release("c1", lease_id)
        clock.advance(3)
        ended = refused(self, queue, LeaseFenced, lambda: queue.renew("c1", lease_id))
        self.assertEqual((ended.action, ended.lease_id), ("Renew", lease_id))
        batch(queue, "c1", 10)
        refused(self, queue, LeaseFenced, lambda: queue.renew("c1", lease_id))

    # scenario: S068
    def test_a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease(self) -> None:
        queue, clock = scenario_queue()
        queue.publish(PUBLISHER, "pk", "k", 1, b"v1")
        old = batch(queue, "c1", 10).lease
        queue.release("c1", old)
        queue.publish(PUBLISHER, "pk", "k", 2, b"v2")
        new = batch(queue, "c1", 10)
        self.assertNotEqual(new.lease, old)
        self.assertEqual((new.pocket, [r.rank for r in new.records]), ("pk", [2]))
        clock.advance(5)
        actions = {
            "Renew": lambda: queue.renew("c1", old),
            "Ack": lambda: queue.ack("c1", old, "k"),
            "Nack": lambda: queue.nack("c1", old, "k"),
            "Release": lambda: queue.release("c1", old),
        }
        for name, action in actions.items():
            with self.subTest(action=name):
                fenced = refused(self, queue, LeaseFenced, action)
                self.assertEqual((fenced.action, fenced.consumer_id, fenced.lease_id), (name, "c1", old))
        self.assertIsNone(queue.ack("c1", new.lease, "k"))

    # scenario: S070
    def test_a_consumer_cannot_act_on_another_consumers_lease(self) -> None:
        queue, _ = two_pockets()
        mine = batch(queue, "c1", 10).lease
        batch(queue, "c2", 10)

        def others(lease_id: str) -> dict[str, Callable[[], None]]:
            return {
                "Ack": lambda: queue.ack("c2", lease_id, "a1"),
                "Nack": lambda: queue.nack("c2", lease_id, "a1"),
                "Renew": lambda: queue.renew("c2", lease_id),
                "Release": lambda: queue.release("c2", lease_id),
            }

        for name, action in [*others(mine).items(), ("Ack", lambda: queue.ack("c3", mine, "absent"))]:
            with self.subTest(active=name):
                fenced = refused(self, queue, LeaseFenced, action)
                self.assertEqual((fenced.action, fenced.lease_id), (name, mine))
        queue.release("c1", mine)
        for name, action in others(mine).items():
            with self.subTest(ended=name):
                refused(self, queue, LeaseFenced, action)


def only_lease(queue: Queue, consumer_id: str = "c1") -> Lease:
    """The consumer's one active lease, read from the snapshot."""
    (found,) = queue.snapshot().active_leases_by_consumer[consumer_id]
    return found


class ResetsOnHandBuiltStatesTest(unittest.TestCase):
    """A mark and a release reset the pocket's consecutive_expiries, and a
    repeat leaves it. These rows run on a hand-built pocket whose count is
    2, a count a lapse leaves before the pocket is claimed again; through
    `Queue`, S056, S064 and S077 in test_expiry.py show the same."""

    def view(self, mark: Mark, *more: Record) -> View:
        state = snapshot(
            config=config(max_consecutive_expiries=5),
            consumers=(Consumer("c1"),),
            pockets=(pocket("p", PocketStatus.LEASED, position=0, consecutive_expiries=2),),
            records=(held("p", "k", position=1, mark=mark, nacked_at=T0 if mark is Mark.NACKED else None), *more),
            leases=(lease("lease-1", "p", "c1", granted_at=T0, expires_at=T0 + 1_000),),
        )
        return View(state, T0 + 7)

    def pockets_written(self, table: Table, view: View, trigger: object) -> list[tuple[int, PocketStatus]]:
        outcome = RecordingOutcome()
        dispatch(table, view, outcome, trigger, registry=Registry())
        return [(e.consecutive_expiries, e.status) for kind, e in outcome.writes if isinstance(e, Pocket)]

    def test_a_first_mark_resets_the_count(self) -> None:
        for table, trigger in ((ACK, Ack("c1", "lease-1", "k")), (NACK, Nack("c1", "lease-1", "k"))):
            with self.subTest(table=table.trigger):
                written = self.pockets_written(table, self.view(Mark.UNSETTLED), trigger)
                self.assertEqual(written, [(0, PocketStatus.LEASED)])

    def test_a_repeat_leaves_the_pocket(self) -> None:
        cases = ((ACK, Mark.ACKED, Ack("c1", "lease-1", "k")), (NACK, Mark.NACKED, Nack("c1", "lease-1", "k")))
        for table, mark, trigger in cases:
            with self.subTest(table=table.trigger):
                self.assertEqual(self.pockets_written(table, self.view(mark), trigger), [])

    def test_a_release_resets_the_count_whichever_way_the_pocket_goes(self) -> None:
        requeue = self.view(Mark.ACKED, queued("p", "q", position=3))
        idle = self.view(Mark.ACKED)
        release = Release("c1", "lease-1")
        self.assertEqual(self.pockets_written(RELEASE, requeue, release), [(0, PocketStatus.QUEUED)])
        self.assertEqual(self.pockets_written(RELEASE, idle, release), [(0, PocketStatus.IDLE)])
