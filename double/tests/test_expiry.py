"""Lease expiry: `tick()`, `LeaseLapses` and the three rules that answer
`LeaseLapsed`, parking and `ReviveParkedPocket`, fencing after a lapse, and
branch A (decision 0019) in both orders.

The double has no background time. A test moves the fake clock and calls
`Queue.tick()`, which reads the clock once and applies every lapse and decay
then due, each as its own outcome. Every other action is processed at the
clock's time without lapsing anything, so an action after `expires_at` but
before the tick succeeds (branch A).

Each test drives `Queue` and reads the outcome through the snapshot, the
lease view or the tick's report. Positions are asserted relative to the
state before the step, never as the absolute values that rest on the
double's first position (spec note N04). Every refusal compares the complete
snapshot before and after. Where the Rust forks a queue, the double runs the
same deterministic setup twice: two queues with the same history.
"""

import unittest
from collections.abc import Callable

from pachinko_double import decay
from pachinko_double.effects import FakeClock
from pachinko_double.expiry import LEASE_EXPIRES_AT, LEASE_LAPSED, LeaseExpiresAt, LeaseLapsed
from pachinko_double.inspection import Snapshot, View
from pachinko_double.invariants import check
from pachinko_double.lease_end import Freed
from pachinko_double.model import (
    Consumer,
    DeadLetter,
    Lease,
    LeaseStatus,
    Mark,
    Pocket,
    PocketStatus,
    Record,
    RecordStatus,
    ShedReason,
    SupersededRecord,
)
from pachinko_double.pull import NoPocketAvailable
from pachinko_double.queue import Queue, TickReport
from pachinko_double.refusals import EqualRankRefused, LeaseFenced, RecordNotHeld
from pachinko_double.rules import Registry, Table, dispatch

from . import fixtures
from .fixtures import (
    DEAD_LETTER_TTL,
    PUBLISHER,
    SUPERSEDED_TTL,
    T0,
    batch,
    keys,
    publish_answers_nothing,
    refused,
    scenario_queue,
)
from .test_publish import pocket_order
from .test_pull import RecordingOutcome

# The lease every test here uses, in milliseconds.
TTL = 10


def at(millis: int) -> int:
    """`T0` plus `millis`."""
    return T0 + millis


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)


def record(queue: Queue, pocket_key: str, record_key: str) -> Record:
    found = queue.snapshot().record(pocket_key, record_key)
    assert found is not None, f"no record {pocket_key!r}/{record_key!r}"
    return found


def short(**config_fields: int) -> tuple[Queue, FakeClock]:
    """The scenarios' setting with a 10 ms lease."""
    return scenario_queue(**({"lease_ttl": TTL} | config_fields))


def two_pockets(**config_fields: int) -> tuple[Queue, FakeClock]:
    """A 10 ms lease, pocket a holding a1 and a2, then pocket b holding b1."""
    queue, clock = short(**config_fields)
    publish(queue, "a", "a1", 1, b"a1")
    publish(queue, "a", "a2", 1, b"a2")
    publish(queue, "b", "b1", 1, b"b1")
    return queue, clock


def lapse(test: unittest.TestCase, queue: Queue, clock: FakeClock) -> TickReport:
    """Move the clock one lease on and tick: exactly one lease lapses."""
    clock.advance(TTL)
    report = queue.tick()
    test.assertEqual(len(report.lapsed), 1, "one lease lapsed")
    return report


class LapseTest(unittest.TestCase):
    # scenario: S006
    def test_a_zero_lease_ttl_lapses_at_the_instant_of_grant(self) -> None:
        queue, _ = short(lease_ttl=0)
        publish(queue, "a", "a1", 1, b"a1")
        lease_id = batch(queue, "c1", 10).lease
        (granted,) = queue.snapshot().leases
        self.assertEqual((granted.granted_at, granted.expires_at), (T0, T0))
        report = queue.tick()
        self.assertEqual([lease.id for lease in report.lapsed], [lease_id])
        self.assertEqual((report.lapsed[0].status, report.lapsed[0].ended_at), (LeaseStatus.EXPIRED, T0))
        self.assertIsNone(queue.lease_view("c1"))

    # scenario: S059
    def test_a_lease_lapses_exactly_at_expires_at(self) -> None:
        queue, clock = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        (granted,) = queue.snapshot().leases
        self.assertEqual(granted.expires_at, at(10))
        clock.set(at(9))
        before = queue.snapshot()
        self.assertEqual(queue.tick().lapsed, ())
        self.assertEqual(queue.snapshot(), before, "a tick before expiry changed the queue")
        clock.set(at(10))
        report = queue.tick()
        self.assertEqual(len(report.lapsed), 1)
        lapsed = report.lapsed[0]
        self.assertEqual(lapsed.id, lease_id)
        self.assertEqual((lapsed.status, lapsed.ended_at, lapsed.expires_at), (LeaseStatus.EXPIRED, at(10), at(10)))
        assert lapsed.ended_at is not None
        self.assertGreaterEqual(lapsed.ended_at, lapsed.granted_at, "EndedAfterGrant")
        self.assertEqual(queue.snapshot().leases, (lapsed,), "the report is the lease as the state keeps it")

    # scenario: S065
    def test_a_lapse_fires_once_per_lease(self) -> None:
        queue, clock = two_pockets()
        batch(queue, "c1", 10)
        lapse(self, queue, clock)
        after = queue.snapshot()
        for later in (10, 20, 1_000):
            clock.set(at(later))
            self.assertEqual(queue.tick().lapsed, (), "a lease lapsed twice")
        self.assertEqual(queue.snapshot(), after)
        (pocket,) = after.pockets_keyed("a")
        self.assertEqual(pocket.consecutive_expiries, 1)


def late() -> tuple[Queue, FakeClock, str]:
    """The branch-A setting: k in pocket a and b1 in b, k pulled by c1 at T0
    under a 10 ms lease, and the clock at 11 ms, past `expires_at` but before
    any tick. A test builds it twice, ticks one queue before the action and
    acts on the other first."""
    queue, clock = short()
    publish(queue, "a", "k", 1, b"k")
    publish(queue, "b", "b1", 1, b"b1")
    lease_id = batch(queue, "c1", 10).lease
    clock.set(at(11))
    return queue, clock, lease_id


class BranchATest(unittest.TestCase):
    """Decision 0019: the lapse and the actions are one serialized stream, so
    the guards decide in either order. An action never lapses its lease."""

    def tick_then_refused(self, first: Queue, lease_id: str, action: Callable[[], object]) -> None:
        """Tick `first`, which lapses the lease and keeps a at the front with k
        requeued; then the action naming the old lease is refused."""
        report = first.tick()
        self.assertEqual([lease.id for lease in report.lapsed], [lease_id])
        (pocket,) = first.snapshot().pockets_keyed("a")
        self.assertEqual(pocket.status, PocketStatus.QUEUED)
        self.assertEqual(pocket_order(first.snapshot()), ["a", "b"])
        self.assertEqual(record(first, "a", "k").status, RecordStatus.QUEUED)
        fenced = refused(self, first, LeaseFenced, action)
        self.assertEqual(fenced.lease_id, lease_id)

    def extended_and_survives(self, second: Queue, lease_id: str) -> None:
        """The lease is active, now expiring at 21 ms, and a tick at 11 ms
        lapses nothing."""
        view = second.lease_view("c1")
        assert view is not None
        self.assertEqual(view.expires_at, at(21))
        (active,) = second.snapshot().active_leases_by_consumer["c1"]
        self.assertEqual(active.id, lease_id)
        (pocket,) = second.snapshot().pockets_keyed("a")
        self.assertEqual(pocket.status, PocketStatus.LEASED)
        self.assertEqual(second.tick().lapsed, ())
        (pocket,) = second.snapshot().pockets_keyed("a")
        self.assertEqual(pocket.status, PocketStatus.LEASED)

    # scenario: S066
    def test_a_renew_after_expires_at_but_before_the_lapse_succeeds(self) -> None:
        first, _, lease_id = late()
        self.tick_then_refused(first, lease_id, lambda: first.renew("c1", lease_id))
        second, _, lease_id = late()
        second.renew("c1", lease_id)
        self.extended_and_survives(second, lease_id)

    # The release half of S066. The Rust also asserts that the released lease
    # leaves its registry; the double keeps it, ended (spec note N24).
    # scenario: S066
    def test_a_release_after_expires_at_but_before_the_lapse_succeeds(self) -> None:
        first, _, lease_id = late()
        self.tick_then_refused(first, lease_id, lambda: first.release("c1", lease_id))
        second, _, lease_id = late()
        fresh = second.snapshot().next_position
        second.release("c1", lease_id)
        (ended,) = second.snapshot().leases
        self.assertEqual((ended.id, ended.status, ended.ended_at), (lease_id, LeaseStatus.RELEASED, at(11)))
        self.assertEqual(second.snapshot().active_leases_by_pocket["a"], ())
        self.assertEqual(second.tick().lapsed, ())
        (pocket,) = second.snapshot().pockets_keyed("a")
        self.assertEqual((pocket.status, pocket.position), (PocketStatus.QUEUED, fresh), "a fresh position")
        self.assertEqual(pocket_order(second.snapshot()), ["b", "a"])

    # scenario: S067
    def test_an_ack_after_expires_at_but_before_the_lapse_succeeds(self) -> None:
        first, _, lease_id = late()
        self.tick_then_refused(first, lease_id, lambda: first.ack("c1", lease_id, "k"))
        second, _, lease_id = late()
        second.ack("c1", lease_id, "k")
        self.assertEqual(record(second, "a", "k").mark, Mark.ACKED)
        self.extended_and_survives(second, lease_id)

    # The pull half of S067. The Rust's batch carries `expires_at`; the double
    # reads it from the snapshot's lease (spec note N13).
    # scenario: S067
    def test_a_pull_after_expires_at_but_before_the_lapse_extends_the_lease(self) -> None:
        first, _, lease_id = late()
        first.tick()
        again = batch(first, "c1", 10)
        self.assertEqual(again.pocket, "a")
        self.assertNotEqual(again.lease, lease_id, "a new lease")
        self.assertEqual(keys(again), ["k"], "the record is handed out again")
        self.assertEqual(check(first.snapshot()), ())
        second, _, lease_id = late()
        within = batch(second, "c1", 10)
        self.assertEqual(within.lease, lease_id)
        self.assertEqual(within.records, (), "the record is still held")
        self.extended_and_survives(second, lease_id)

    def test_a_nack_after_expires_at_but_before_the_lapse_succeeds(self) -> None:
        # Branch A names the nack too; no catalog row covers it, so it is
        # untagged. The nack is processed at 11 ms, after expires_at.
        first, _, lease_id = late()
        self.tick_then_refused(first, lease_id, lambda: first.nack("c1", lease_id, "k"))
        second, _, lease_id = late()
        second.nack("c1", lease_id, "k")
        nacked = record(second, "a", "k")
        self.assertEqual((nacked.mark, nacked.nacked_at), (Mark.NACKED, at(11)))
        self.extended_and_survives(second, lease_id)


def pocket(queue: Queue, key: str) -> Pocket:
    (found,) = queue.snapshot().pockets_keyed(key)
    return found


def claim_and_lapse(test: unittest.TestCase, queue: Queue, clock: FakeClock, key: str) -> TickReport:
    """c1 claims the front pocket, which must be `key`, and lets the lease lapse."""
    test.assertEqual(batch(queue, "c1", 10).pocket, key)
    return lapse(test, queue, clock)


def parked(test: unittest.TestCase) -> tuple[Queue, FakeClock]:
    """Pocket a parked by three lapses in a row, with b queued behind it."""
    queue, clock = two_pockets()
    for _ in range(3):
        claim_and_lapse(test, queue, clock, "a")
    test.assertEqual(pocket(queue, "a").status, PocketStatus.PARKED)
    return queue, clock


def lapsed_once(test: unittest.TestCase) -> tuple[Queue, FakeClock]:
    """Pocket a with a1, a2 and a3, lapsed once, then pocket b."""
    queue, clock = two_pockets()
    publish(queue, "a", "a3", 1, b"a3")
    claim_and_lapse(test, queue, clock, "a")
    return queue, clock


class ExpiryRulesTest(unittest.TestCase):
    # scenario: S058
    def test_a_released_lease_never_lapses_afterwards(self) -> None:
        queue, clock = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        queue.release("c1", lease_id)
        after = queue.snapshot()
        for later in (10, 11, 1_000):
            clock.set(at(later))
            self.assertEqual(queue.tick().lapsed, (), "a released lease lapsed")
        self.assertEqual(queue.snapshot(), after)
        self.assertEqual(pocket(queue, "a").consecutive_expiries, 0)

    # scenario: S060
    def test_a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records(self) -> None:
        queue, clock = two_pockets()
        positions = [record(queue, "a", key).position for key in ("a1", "a2")]
        position = pocket(queue, "a").position
        first = batch(queue, "c1", 10)
        self.assertEqual(keys(first), ["a1", "a2"])
        lapse(self, queue, clock)
        found = pocket(queue, "a")
        self.assertEqual(
            (found.status, found.position, found.consecutive_expiries), (PocketStatus.QUEUED, position, 1)
        )
        self.assertEqual(pocket_order(queue.snapshot()), ["a", "b"])
        for key, kept in zip(("a1", "a2"), positions):
            requeued = record(queue, "a", key)
            self.assertEqual((requeued.status, requeued.position), (RecordStatus.QUEUED, kept))
        second = batch(queue, "c2", 10)
        self.assertEqual(second.pocket, "a", "the lapsed pocket kept the front")
        self.assertNotEqual(second.lease, first.lease)
        self.assertEqual(keys(second), ["a1", "a2"])

    # scenario: S061
    def test_a_lapse_with_no_records_idles_the_pocket(self) -> None:
        queue, clock = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        queue.ack("c1", lease_id, "a1")
        queue.ack("c1", lease_id, "a2")
        report = lapse(self, queue, clock)
        self.assertEqual((report.lapsed[0].status, report.lapsed[0].ended_at), (LeaseStatus.EXPIRED, at(10)))
        found = pocket(queue, "a")
        self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.IDLE, 1))
        self.assertEqual(pocket_order(queue.snapshot()), ["b"])

    # scenario: S062
    def test_three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it(self) -> None:
        queue, clock = two_pockets()
        for count in (1, 2):
            claim_and_lapse(self, queue, clock, "a")
            found = pocket(queue, "a")
            self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.QUEUED, count))
        report = claim_and_lapse(self, queue, clock, "a")
        self.assertEqual(report.lapsed[0].status, LeaseStatus.EXPIRED)
        found = pocket(queue, "a")
        self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.PARKED, 3))
        self.assertEqual(record(queue, "a", "a1").status, RecordStatus.QUEUED)
        self.assertEqual(batch(queue, "c2", 10).pocket, "b")
        fresh = queue.snapshot().next_position
        publish_answers_nothing(self, queue, "a", "a3", 1, b"a3")
        self.assertEqual(record(queue, "a", "a3").position, fresh)
        found = pocket(queue, "a")
        self.assertEqual(
            (found.status, found.position, found.consecutive_expiries),
            (PocketStatus.QUEUED, fresh + 1, 0),
            "revived at the back, after a3's own position, with the count reset",
        )
        self.assertEqual(check(queue.snapshot()), ())

    # scenario: S063
    def test_at_the_threshold_an_empty_pocket_parks_rather_than_idling(self) -> None:
        def fate(queue: Queue) -> tuple[PocketStatus, int, int]:
            found = pocket(queue, "a")
            return found.status, found.consecutive_expiries, len(queue.snapshot().records_by_pocket["a"])

        # At a limit of 1 every lapse is at the threshold, so even a pocket
        # whose one record was acked parks.
        once, clock = short(max_consecutive_expiries=1)
        publish(once, "a", "a1", 1, b"a1")
        lease_id = batch(once, "c1", 10).lease
        once.ack("c1", lease_id, "a1")
        lapse(self, once, clock)
        self.assertEqual(fate(once), (PocketStatus.PARKED, 1, 0), "limit 1, acked")

        # At the default 3: two lapses, then a third lease whose records are
        # all nacked or all acked. Either resets the count, so the lapse idles
        # the pocket.
        def third_lease() -> tuple[Queue, FakeClock, str]:
            queue, clock = two_pockets()
            for _ in range(2):
                claim_and_lapse(self, queue, clock, "a")
            return queue, clock, batch(queue, "c1", 10).lease

        nacked, clock, lease_id = third_lease()
        for key in ("a1", "a2"):
            nacked.nack("c1", lease_id, key)
        lapse(self, nacked, clock)
        self.assertEqual(fate(nacked), (PocketStatus.IDLE, 1, 0), "nacked")
        self.assertEqual(len(nacked.snapshot().dead_letters), 2, "nacked")
        acked, clock, lease_id = third_lease()
        for key in ("a1", "a2"):
            acked.ack("c1", lease_id, key)
        lapse(self, acked, clock)
        self.assertEqual(fate(acked), (PocketStatus.IDLE, 1, 0), "acked")

    # scenario: S048
    def test_a_version_deferred_behind_a_nacked_one_is_queued_at_its_arrival_position_by_the_lapse(self) -> None:
        queue, clock = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        position, arrival = record(queue, "a", "a1").position, queue.snapshot().next_position
        publish(queue, "a", "a1", 2, b"newer")
        queue.nack("c1", lease_id, "a1")
        held = record(queue, "a", "a1")
        self.assertEqual(
            (held.status, held.held_rank, held.held_payload, held.mark, held.nacked_at, held.deferred_position),
            (RecordStatus.HELD, 1, b"a1", Mark.NACKED, T0, arrival),
        )
        self.assertEqual(held.position, position, "still held at its own position, the arrival position beside it")
        self.assertEqual(queue.snapshot().dead_letters, ())
        lapse(self, queue, clock)
        requeued = record(queue, "a", "a1")
        self.assertEqual(
            (requeued.status, requeued.position), (RecordStatus.QUEUED, arrival), "queued at its arrival position"
        )
        letter = DeadLetter("a", "a1", 1, b"a1", "c1", T0, at(TTL) + DEAD_LETTER_TTL)
        self.assertEqual(queue.snapshot().dead_letters, (letter,), "dead-lettered at the lapse")
        again = batch(queue, "c2", 10)
        self.assertEqual(again.pocket, "a")
        self.assertEqual(keys(again), ["a2", "a1"])
        self.assertEqual((again.records[1].rank, again.records[1].payload), (2, b"newer"))

    # scenario: S069
    def test_after_a_lapse_the_old_lease_is_fenced_even_when_the_key_has_a_newer_rank(self) -> None:
        queue, clock = two_pockets()
        old = batch(queue, "c1", 10).lease
        lapse(self, queue, clock)
        publish(queue, "a", "a1", 2, b"newer")
        new = batch(queue, "c2", 10)
        self.assertEqual(new.pocket, "a")
        self.assertNotEqual(new.lease, old)
        for action in (
            lambda: queue.ack("c1", old, "a1"),
            lambda: queue.renew("c1", old),
            lambda: queue.release("c1", old),
        ):
            fenced = refused(self, queue, LeaseFenced, action)
            self.assertEqual(fenced.lease_id, old)


class ExpiryCountTest(unittest.TestCase):
    """The consecutive-expiry count: a lapse raises it, an ack, a nack, a
    release or a revival resets it, and a pull or a renew leaves it."""

    # The Rust's S064 also has a wire half; only its library half is ported.
    # scenario: S064
    def test_a_nack_resets_the_expiry_count_and_a_pull_or_a_renew_does_not(self) -> None:
        queue, clock = lapsed_once(self)

        def count() -> int:
            return pocket(queue, "a").consecutive_expiries

        self.assertEqual(count(), 1)
        lease_id = batch(queue, "c1", 10).lease
        self.assertEqual(count(), 1, "after the pull")
        queue.renew("c1", lease_id)
        self.assertEqual(count(), 1, "after the renew")
        self.assertEqual(batch(queue, "c1", 10).lease, lease_id, "a pull within the lease")
        self.assertEqual(count(), 1, "after that pull")
        queue.nack("c1", lease_id, "a1")
        self.assertEqual(count(), 0, "after the nack")
        lapse(self, queue, clock)
        self.assertEqual(count(), 1, "the lapse counts from the reset")

    # The Rust's S077 also has a wire half; only its library half is ported.
    # scenario: S077
    def test_an_ack_resets_the_consecutive_expiry_count(self) -> None:
        queue, clock = lapsed_once(self)
        lease_id = batch(queue, "c1", 10).lease

        def count() -> tuple[PocketStatus, int]:
            found = pocket(queue, "a")
            return found.status, found.consecutive_expiries

        self.assertEqual(count(), (PocketStatus.LEASED, 1))
        refused(self, queue, RecordNotHeld, lambda: queue.ack("c1", lease_id, "b1"))
        self.assertEqual(count(), (PocketStatus.LEASED, 1), "a refused ack")
        queue.ack("c1", lease_id, "a1")
        self.assertEqual(count(), (PocketStatus.LEASED, 0), "an ack")
        lapse(self, queue, clock)
        self.assertEqual(count(), (PocketStatus.QUEUED, 1))
        self.assertEqual(pocket_order(queue.snapshot()), ["a", "b"], "kept at the front")
        # An ack of a record with a newer version deferred behind it resets the
        # count too. The Rust's publish answers Superseded; the double's
        # answers nothing, which the test asserts, and defers (spec note N32).
        lease_id = batch(queue, "c1", 10).lease
        publish_answers_nothing(self, queue, "a", "a2", 2, b"newer")
        self.assertIsNotNone(record(queue, "a", "a2").deferred_position)
        queue.ack("c1", lease_id, "a2")
        self.assertEqual(count(), (PocketStatus.LEASED, 0), "an ack with a version deferred")
        # From there it parks on the third lapse, not the second.
        lapse(self, queue, clock)
        self.assertEqual(count(), (PocketStatus.QUEUED, 1))
        claim_and_lapse(self, queue, clock, "a")
        self.assertEqual(count(), (PocketStatus.QUEUED, 2))
        claim_and_lapse(self, queue, clock, "a")
        self.assertEqual(count(), (PocketStatus.PARKED, 3))
        self.assertEqual(check(queue.snapshot()), ())

    # scenario: S056
    def test_a_release_resets_the_consecutive_expiry_count(self) -> None:
        queue, clock = lapsed_once(self)
        claim_and_lapse(self, queue, clock, "a")
        self.assertEqual(pocket(queue, "a").consecutive_expiries, 2)
        lease_id = batch(queue, "c1", 10).lease
        queue.release("c1", lease_id)
        found = pocket(queue, "a")
        self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.QUEUED, 0))
        # a went to the back on release: b is claimed first, and released.
        claimed = batch(queue, "c1", 10)
        self.assertEqual(claimed.pocket, "b")
        queue.release("c1", claimed.lease)
        for count in (1, 2):
            claim_and_lapse(self, queue, clock, "a")
            found = pocket(queue, "a")
            self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.QUEUED, count))
        claim_and_lapse(self, queue, clock, "a")
        self.assertEqual(pocket(queue, "a").status, PocketStatus.PARKED)


class ParkedTest(unittest.TestCase):
    """A parked pocket: out of the pocket order until a publish that touches
    it revives it at the back with its count reset (`ReviveParkedPocket`)."""

    # scenario: S024
    def test_a_supersede_revives_a_parked_pocket(self) -> None:
        queue, _ = parked(self)
        fresh = queue.snapshot().next_position
        publish_answers_nothing(self, queue, "a", "a1", 2, b"x")
        self.assertEqual(record(queue, "a", "a1").rank, 2)
        found = pocket(queue, "a")
        self.assertEqual(
            (found.status, found.position, found.consecutive_expiries),
            (PocketStatus.QUEUED, fresh, 0),
            "a supersede revives at the back",
        )
        self.assertEqual(pocket_order(queue.snapshot()), ["b", "a"])
        self.assertEqual(check(queue.snapshot()), ())
        # A publish into a pocket that does not exist creates it; nothing is revived.
        other, _ = parked(self)
        publish_answers_nothing(self, other, "z", "z1", 1, b"z")
        self.assertEqual(pocket(other, "a").status, PocketStatus.PARKED)

    # scenario: S092
    def test_a_stale_publish_into_a_parked_pocket_leaves_it_parked_with_its_count(self) -> None:
        queue, clock = parked(self)
        before = queue.snapshot()
        position = pocket(queue, "a").position
        now = at(3 * TTL)
        self.assertEqual(clock.now(), now)
        publish_answers_nothing(self, queue, "a", "a1", 0, b"x")
        found = pocket(queue, "a")
        self.assertEqual(
            (found.status, found.position, found.consecutive_expiries),
            (PocketStatus.PARKED, position, 3),
            "a stale publish touches nothing",
        )
        state = queue.snapshot()
        self.assertEqual(pocket_order(state), ["b"])
        self.assertEqual(state.next_position, before.next_position)
        stale = SupersededRecord("a", "a1", 0, b"x", ShedReason.STALE, now, now + SUPERSEDED_TTL)
        self.assertEqual(state.superseded, (stale,), "the arrival is recorded as stale")
        self.assertEqual(check(state), ())

    # scenario: S025
    def test_an_equal_rank_publish_does_not_revive_a_parked_pocket(self) -> None:
        queue, _ = parked(self)
        refused(self, queue, EqualRankRefused, lambda: publish(queue, "a", "a1", 1, b"x"))
        self.assertEqual(pocket(queue, "a").status, PocketStatus.PARKED)

    # Only a publish that touches the pocket revives it. Neither time, nor a
    # tick, nor another pocket's lease, publish or release does. The
    # maintainer closed open question 7 on 2026-11-23: no operator action and
    # no cooldown revives a parked pocket (spec note N36).
    # scenario: S026
    def test_only_a_publish_revives_a_parked_pocket(self) -> None:
        queue, clock = parked(self)
        lease_id = batch(queue, "c2", 10).lease
        self.assertEqual(pocket(queue, "b").status, PocketStatus.LEASED)
        # A publish into a pocket that is not parked leaves it where it is.
        publish_answers_nothing(self, queue, "b", "b2", 1, b"b2")
        self.assertEqual(pocket(queue, "b").status, PocketStatus.LEASED)
        queue.release("c2", lease_id)
        for later in (1_000, 1_000_000):
            clock.set(at(later))
            queue.tick()
        lease_id = batch(queue, "c3", 10).lease
        queue.release("c3", lease_id)
        self.assertEqual(batch(queue, "c1", 10).pocket, "b")
        self.assertIsInstance(queue.pull("c2", 10), NoPocketAvailable)
        found = pocket(queue, "a")
        self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.PARKED, 3))

    # scenario: S034
    def test_idle_parked_and_leased_pockets_are_never_claimed(self) -> None:
        queue, _ = parked(self)
        claimed = batch(queue, "c1", 10)
        self.assertEqual(claimed.pocket, "b")
        queue.ack("c1", claimed.lease, "b1")
        queue.release("c1", claimed.lease)
        self.assertEqual(pocket(queue, "b").status, PocketStatus.IDLE)
        publish(queue, "c", "c1", 1, b"c1")
        publish(queue, "d", "d1", 1, b"d1")
        self.assertEqual(batch(queue, "c2", 10).pocket, "c")
        # Now a is parked, b idle and c leased: only d is claimable.
        self.assertEqual(batch(queue, "c3", 10).pocket, "d")
        queue.consumer_joins("c4")
        self.assertIsInstance(queue.pull("c4", 10), NoPocketAvailable)
        self.assertEqual(check(queue.snapshot()), ())


class OneStrictOrderTest(unittest.TestCase):
    # The Rust's step is its one_strict_order: the pocket positions and the
    # record positions are each distinct and below next_position (the four
    # position invariants), the records of each pocket have distinct
    # positions, and its stored pocket order is the queued pockets sorted by
    # position. The double stores no pocket order, since it is the queued
    # pockets by position, so that last check has no counterpart (spec note
    # N38); the test asserts the rest from the snapshot, then `check`.
    # scenario: S075
    def test_pockets_and_records_form_one_strict_deterministic_order(self) -> None:
        queue, clock = two_pockets()

        def step() -> None:
            state = queue.snapshot()
            pockets = [p.position for p in state.pockets]
            records = [r.position for r in state.records]
            self.assertEqual(len(pockets), len(set(pockets)), "UniquePocketPosition")
            self.assertEqual(len(records), len(set(records)), "UniqueRecordPosition")
            self.assertTrue(all(p < state.next_position for p in pockets), "PocketPositionsBelowNext")
            self.assertTrue(all(r < state.next_position for r in records), "RecordPositionsBelowNext")
            for key, held in state.records_by_pocket.items():
                self.assertEqual(len({r.position for r in held}), len(held), f"record positions in {key}")
            self.assertEqual(check(state), ())

        lease_id = batch(queue, "c1", 1).lease
        step()
        queue.ack("c1", lease_id, "a1")
        refused(self, queue, EqualRankRefused, lambda: publish(queue, "a", "a1", 1, b"again"))
        publish(queue, "a", "a1", 2, b"again")
        handed = batch(queue, "c1", 10)
        step()
        publish(queue, "a", "a2", 2, b"newer")
        self.assertEqual(keys(handed), ["a2"], "a1 waits for the lease's end")
        queue.nack("c1", lease_id, "a2")
        step()
        queue.release("c1", lease_id)
        step()
        # b is now the front, and it lapses; then the front lapses three times.
        claim_and_lapse(self, queue, clock, "b")
        step()
        for _ in range(3):
            claim_and_lapse(self, queue, clock, pocket_order(queue.snapshot())[0])
            step()
        publish(queue, "a", "a9", 1, b"revive")
        publish(queue, "b", "b1", 2, b"revive")
        step()


class LapseTablesTest(unittest.TestCase):
    """The clock-trigger and `LeaseLapsed` tables on hand-built states, against
    a private registry so the fired record is untouched.

    Through `tick()` a due lease on a leased pocket always meets one expiry
    rule, so these are the only tests that reach each table's answer when no
    guard holds: nothing happens (spec note N37).
    """

    NOW = T0 + 10

    def state(
        self,
        count: int = 0,
        *,
        mark: Mark = Mark.UNSETTLED,
        lease_status: LeaseStatus = LeaseStatus.ACTIVE,
        expires_at: int = T0 + 10,
        pocket_status: PocketStatus = PocketStatus.LEASED,
    ) -> Snapshot:
        ended_at = None if lease_status is LeaseStatus.ACTIVE else T0 + 5
        return fixtures.snapshot(
            config=fixtures.config(max_consecutive_expiries=3),
            consumers=(Consumer("c1"),),
            pockets=(fixtures.pocket("p", pocket_status, position=0, consecutive_expiries=count),),
            records=(fixtures.held("p", "k", position=1, mark=mark, nacked_at=T0 if mark is Mark.NACKED else None),),
            leases=(fixtures.lease("lease-1", "p", "c1", lease_status, T0, expires_at, ended_at),),
        )

    def run_table(self, table: Table, state: Snapshot, trigger: object) -> tuple[str | None, RecordingOutcome]:
        outcome = RecordingOutcome()
        name = dispatch(table, View(state, self.NOW), outcome, trigger, registry=Registry())
        return name, outcome

    def test_exactly_one_expiry_rule_answers_a_due_lapse(self) -> None:
        cases = {
            (0, Mark.UNSETTLED): "LeaseExpiryKeepsPocketAtFront",
            (1, Mark.UNSETTLED): "LeaseExpiryKeepsPocketAtFront",
            (2, Mark.UNSETTLED): "LeaseExpiryParksPocket",
            (0, Mark.ACKED): "LeaseExpiryIdlesEmptyPocket",
            (1, Mark.NACKED): "LeaseExpiryIdlesEmptyPocket",
            (2, Mark.ACKED): "LeaseExpiryParksPocket",
        }
        for (count, mark), expected in cases.items():
            with self.subTest(count=count, mark=mark.value):
                name, outcome = self.run_table(LEASE_LAPSED, self.state(count, mark=mark), LeaseLapsed("lease-1"))
                self.assertEqual(name, expected)
                (ended,) = (e for kind, e in outcome.writes if kind == "replace" and isinstance(e, Lease))
                self.assertEqual((ended.status, ended.ended_at), (LeaseStatus.EXPIRED, self.NOW))
                self.assertIn(("emit", Freed("lease-1", "k", self.NOW)), outcome.writes)
                self.assertEqual(outcome.allotted, 0, "no expiry rule allots a position")

    def test_the_lapse_answers_nothing_when_its_guards_fail(self) -> None:
        cases = {
            "renewed past now": self.state(expires_at=self.NOW + 1),
            "released": self.state(lease_status=LeaseStatus.RELEASED, pocket_status=PocketStatus.QUEUED),
            "expired": self.state(lease_status=LeaseStatus.EXPIRED, pocket_status=PocketStatus.QUEUED),
            "pocket not leased": self.state(pocket_status=PocketStatus.QUEUED),
        }
        for case, state in cases.items():
            with self.subTest(case=case):
                name, outcome = self.run_table(LEASE_LAPSED, state, LeaseLapsed("lease-1"))
                self.assertEqual((name, outcome.writes, outcome.answers), (None, [], []))

    def test_lease_lapses_emits_the_lapse_only_for_an_active_lease(self) -> None:
        name, outcome = self.run_table(LEASE_EXPIRES_AT, self.state(), LeaseExpiresAt("lease-1"))
        self.assertEqual((name, outcome.writes), ("LeaseLapses", [("emit", LeaseLapsed("lease-1"))]))
        for status in (LeaseStatus.RELEASED, LeaseStatus.EXPIRED):
            with self.subTest(status=status.value):
                state = self.state(lease_status=status, pocket_status=PocketStatus.QUEUED)
                name, outcome = self.run_table(LEASE_EXPIRES_AT, state, LeaseExpiresAt("lease-1"))
                self.assertEqual((name, outcome.writes), (None, []))

    def test_a_decay_answers_nothing_for_an_entry_no_longer_in_its_queue(self) -> None:
        kept, gone = fixtures.shed(rank=1), fixtures.shed(rank=2)
        letter, other = fixtures.letter(rank=1), fixtures.letter(rank=2)
        state = fixtures.snapshot(
            consumers=(Consumer("c"),), pockets=(fixtures.pocket("p"),), superseded=(kept,), dead_letters=(letter,)
        )
        for table, present, absent in (
            (
                decay.SUPERSEDED_RECORD_EXPIRES_AT,
                decay.SupersededRecordExpiresAt(kept),
                decay.SupersededRecordExpiresAt(gone),
            ),
            (decay.DEAD_LETTER_EXPIRES_AT, decay.DeadLetterExpiresAt(letter), decay.DeadLetterExpiresAt(other)),
        ):
            with self.subTest(table=table.trigger):
                name, outcome = self.run_table(table, state, present)
                entry = kept if table is decay.SUPERSEDED_RECORD_EXPIRES_AT else letter
                self.assertEqual((outcome.writes, outcome.answers), ([("remove", entry)], [entry]))
                self.assertIsNotNone(name)
                name, outcome = self.run_table(table, state, absent)
                self.assertEqual((name, outcome.writes, outcome.answers), (None, [], []))
