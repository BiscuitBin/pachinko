"""The Ack and nack section: marks, repeats, mark conflicts and their
refusals, the dead letter a nack becomes when its lease ends, and the lease
view of the `Consuming` surface, which shows the marks.

Ack and nack answer nothing (spec note N13), so a test reads the mark, the
lease's new `expires_at` and everything else through the snapshot or the
lease view. Every refusal compares the complete snapshot before and after.
"""

import unittest
from dataclasses import replace

from pachinko_double.consuming import HeldRecordView
from pachinko_double.effects import FakeClock
from pachinko_double.model import DeadLetter, Lease, LeaseStatus, Mark, RecordStatus
from pachinko_double.queue import Queue
from pachinko_double.refusals import LeaseFenced, MarkConflict, RecordNotHeld, UnknownConsumer

from .fixtures import DEAD_LETTER_TTL, LEASE_TTL, PUBLISHER, T0, batch, refused, scenario_queue


def held_k() -> tuple[Queue, FakeClock, str]:
    """Pocket pk holds k at rank 1, payload v1, pulled by c1 at T0.

    pk stands at 0 and k at 1, so the next position is 2.
    """
    queue, clock = scenario_queue()
    queue.publish(PUBLISHER, "pk", "k", 1, b"v1")
    return queue, clock, batch(queue, "c1", 10).lease


def record_keys(queue: Queue, pocket_key: str) -> list[str]:
    """The keys of every record pocket `pocket_key` has, by key."""
    return sorted(record.key for record in queue.snapshot().records_by_pocket[pocket_key])


def only_lease(queue: Queue, consumer_id: str = "c1") -> Lease:
    """The consumer's one active lease, read from the snapshot."""
    (lease,) = queue.snapshot().active_leases_by_consumer[consumer_id]
    assert lease.status is LeaseStatus.ACTIVE
    return lease


class AckTest(unittest.TestCase):
    # scenario: S040
    def test_an_ack_marks_a_held_record_and_removes_nothing(self) -> None:
        queue, _, lease_id = held_k()
        queue.publish(PUBLISHER, "pk", "other", 1, b"o")
        before = queue.snapshot().record("pk", "k")
        assert before is not None
        self.assertIsNone(queue.ack("c1", lease_id, "k"))
        state = queue.snapshot()
        record = state.record("pk", "k")
        assert record is not None
        self.assertEqual(
            (record.status, record.held_rank, record.held_payload, record.mark, record.nacked_at),
            (RecordStatus.HELD, 1, b"v1", Mark.ACKED, None),
        )
        self.assertIsNone(record.deferred_position)
        # The Rust asserts position 1, which rests on the first position being 0
        # (N04); the double asserts what the specification states: unchanged.
        self.assertEqual(record.position, before.position)
        self.assertEqual(record_keys(queue, "pk"), ["k", "other"], "nothing is removed")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(view.records, (HeldRecordView("k", 1, b"v1", Mark.ACKED),))
        self.assertEqual((state.superseded, state.dead_letters), ((), ()))

    # scenario: S041
    def test_an_ack_extends_the_lease(self) -> None:
        queue, clock = scenario_queue(lease_ttl=10)
        queue.publish(PUBLISHER, "pk", "k", 1, b"v")
        lease_id = batch(queue, "c1", 10).lease
        self.assertEqual(only_lease(queue).expires_at, T0 + 10)
        clock.advance(4)
        queue.ack("c1", lease_id, "k")
        self.assertEqual(only_lease(queue).expires_at, T0 + 14)
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(view.expires_at, T0 + 14)

    # scenario: S042
    def test_rejected_acks_change_nothing_not_even_the_leases_expiry(self) -> None:
        queue, clock = scenario_queue()
        queue.publish(PUBLISHER, "pk", "k", 1, b"v")
        queue.publish(PUBLISHER, "pk", "queued", 1, b"q")
        queue.publish(PUBLISHER, "other", "x", 1, b"x")
        mine = batch(queue, "c1", 1).lease
        theirs = batch(queue, "c2", 1).lease
        clock.advance(500)
        # impl-choice (spec note N22): a consumer that never joined.
        unknown = refused(self, queue, UnknownConsumer, lambda: queue.ack("nobody", mine, "k"))
        self.assertEqual(unknown.consumer_id, "nobody")
        for lease_id, key in (("never-issued", "k"), (theirs, "x")):
            with self.subTest(lease=lease_id):
                fenced = refused(self, queue, LeaseFenced, lambda: queue.ack("c1", lease_id, key))
                self.assertEqual((fenced.action, fenced.consumer_id, fenced.lease_id), ("Ack", "c1", lease_id))
        # Absent, queued in the lease's pocket, and held in another pocket.
        for key in ("absent", "queued", "x"):
            with self.subTest(key=key):
                not_held = refused(self, queue, RecordNotHeld, lambda: queue.ack("c1", mine, key))
                self.assertEqual((not_held.action, not_held.lease_id, not_held.record_key), ("Ack", mine, key))
        self.assertEqual(only_lease(queue, "c1").expires_at, T0 + LEASE_TTL, "no refusal moved the expiry")
        queue.ack("c1", mine, "k")
        self.assertIsNone(queue.ack("c1", mine, "k"), "a repeat is not a refusal")


class NackTest(unittest.TestCase):
    # scenario: S049
    def test_rejected_nacks_change_nothing_and_create_no_dead_letter(self) -> None:
        queue, clock = scenario_queue()
        queue.publish(PUBLISHER, "pk", "k", 1, b"v")
        queue.publish(PUBLISHER, "pk", "queued", 1, b"q")
        queue.publish(PUBLISHER, "other", "x", 1, b"x")
        mine = batch(queue, "c1", 1).lease
        theirs = batch(queue, "c2", 1).lease
        clock.advance(500)
        # impl-choice (spec note N22): a consumer that never joined.
        unknown = refused(self, queue, UnknownConsumer, lambda: queue.nack("nobody", mine, "k"))
        self.assertEqual(unknown.consumer_id, "nobody")
        for lease_id, key in (("never-issued", "k"), (theirs, "x")):
            with self.subTest(lease=lease_id):
                fenced = refused(self, queue, LeaseFenced, lambda: queue.nack("c1", lease_id, key))
                self.assertEqual((fenced.action, fenced.consumer_id, fenced.lease_id), ("Nack", "c1", lease_id))
        for key in ("absent", "queued", "x"):
            with self.subTest(key=key):
                not_held = refused(self, queue, RecordNotHeld, lambda: queue.nack("c1", mine, key))
                self.assertEqual((not_held.action, not_held.lease_id, not_held.record_key), ("Nack", mine, key))
        self.assertEqual(queue.snapshot().dead_letters, ())
        self.assertEqual(only_lease(queue, "c1").expires_at, T0 + LEASE_TTL)

    def test_a_nack_marks_the_held_record_with_its_time_and_extends_the_lease(self) -> None:
        # S047's first half, before the lease ends (its whole test is below).
        queue, clock, lease_id = held_k()
        clock.set(T0 + 20)
        self.assertIsNone(queue.nack("c1", lease_id, "k"))
        record = queue.snapshot().record("pk", "k")
        assert record is not None
        self.assertEqual(
            (record.status, record.held_rank, record.held_payload, record.mark, record.nacked_at),
            (RecordStatus.HELD, 1, b"v1", Mark.NACKED, T0 + 20),
        )
        self.assertEqual(only_lease(queue).expires_at, T0 + 20 + LEASE_TTL)
        self.assertEqual(queue.snapshot().dead_letters, (), "not before the lease ends")

    # scenario: S047
    def test_a_nack_marks_the_held_version_without_redelivery_and_dead_letters_it_at_the_lease_end(self) -> None:
        queue, clock, lease_id = held_k()
        clock.set(T0 + 20)
        queue.nack("c1", lease_id, "k")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(view.expires_at, T0 + 20 + LEASE_TTL, "the nack extended it")
        self.assertEqual(view.records, (HeldRecordView("k", 1, b"v1", Mark.NACKED),))
        record = queue.snapshot().record("pk", "k")
        assert record is not None
        self.assertEqual(
            (record.status, record.nacked_at, record.deferred_position), (RecordStatus.HELD, T0 + 20, None)
        )
        self.assertEqual(queue.snapshot().dead_letters, (), "not before the end")
        again = batch(queue, "c1", 10)
        self.assertEqual((again.lease, again.records), (lease_id, ()), "no redelivery")
        self.assertEqual(only_lease(queue).expires_at, T0 + 20 + LEASE_TTL, "the pull within the lease extends it")
        clock.set(T0 + 30)
        queue.release("c1", lease_id)
        state = queue.snapshot()
        letter = DeadLetter("pk", "k", 1, b"v1", "c1", T0 + 20, T0 + 30 + DEAD_LETTER_TTL)
        self.assertEqual(state.dead_letters, (letter,))
        self.assertEqual(state.records_by_pocket["pk"], ())
        self.assertLessEqual(state.dead_letters[0].nacked_at, state.dead_letters[0].expires_at, "NackedBeforeExpiry")

    # scenario: S086
    def test_a_repeat_nack_keeps_the_first_nacks_time(self) -> None:
        queue, clock, lease_id = held_k()
        clock.set(T0 + 10)
        queue.nack("c1", lease_id, "k")
        clock.set(T0 + 30)
        self.assertIsNone(queue.nack("c1", lease_id, "k"))
        clock.set(T0 + 40)
        queue.release("c1", lease_id)
        self.assertEqual(
            queue.snapshot().dead_letters, (DeadLetter("pk", "k", 1, b"v1", "c1", T0 + 10, T0 + 40 + DEAD_LETTER_TTL),)
        )


def marked_both_ways() -> tuple[Queue, FakeClock, str]:
    """pk with k1 and k2, both held by c1 from T0, then k1 nacked and k2 acked at T0 + 10."""
    queue, clock = scenario_queue()
    queue.publish(PUBLISHER, "pk", "k1", 1, b"k1")
    queue.publish(PUBLISHER, "pk", "k2", 1, b"k2")
    lease_id = batch(queue, "c1", 10).lease
    clock.set(T0 + 10)
    queue.nack("c1", lease_id, "k1")
    queue.ack("c1", lease_id, "k2")
    return queue, clock, lease_id


class RepeatAndConflictTest(unittest.TestCase):
    # scenario: S084
    def test_a_repeat_ack_or_nack_answers_like_the_first_extends_the_lease_and_changes_nothing_else(self) -> None:
        queue, clock, lease_id = marked_both_ways()
        clock.set(T0 + 30)
        before = queue.snapshot()
        (lease,) = before.leases
        expected = replace(before, leases=(replace(lease, expires_at=T0 + 30 + LEASE_TTL),))
        self.assertIsNone(queue.ack("c1", lease_id, "k2"))
        self.assertIsNone(queue.nack("c1", lease_id, "k1"))
        self.assertEqual(queue.snapshot(), expected, "a repeat moves the lease's expiry and nothing else")
        k1 = queue.snapshot().record("pk", "k1")
        assert k1 is not None
        self.assertEqual((k1.mark, k1.nacked_at), (Mark.NACKED, T0 + 10), "the first nack's time stands")

    # S084 runs both repeats at one time and compares once, so either could
    # miss its extension and the other would supply it. Here each repeat runs
    # alone, at its own time, and its snapshot is compared at once.
    def test_each_repeat_on_its_own_extends_the_lease_from_its_own_time(self) -> None:
        queue, clock, lease_id = marked_both_ways()
        for action, key, at in ((queue.ack, "k2", T0 + 30), (queue.nack, "k1", T0 + 50)):
            with self.subTest(key=key):
                clock.set(at)
                before = queue.snapshot()
                (lease,) = before.leases
                expected = replace(before, leases=(replace(lease, expires_at=at + LEASE_TTL),))
                self.assertIsNone(action("c1", lease_id, key))
                self.assertEqual(queue.snapshot(), expected, "this repeat alone moves the expiry, and nothing else")

    # scenario: S085
    def test_a_changed_mark_is_refused_as_a_mark_conflict_and_changes_nothing(self) -> None:
        queue, clock, lease_id = marked_both_ways()
        clock.set(T0 + 500)
        reads = clock.reads
        for action, key, mark in ((queue.ack, "k1", Mark.NACKED), (queue.nack, "k2", Mark.ACKED)):
            with self.subTest(key=key):
                conflict = refused(self, queue, MarkConflict, lambda: action("c1", lease_id, key))
                self.assertEqual((conflict.lease_id, conflict.record_key, conflict.mark), (lease_id, key, mark))
        self.assertEqual(clock.reads, reads, "a refused mark reads no clock (spec note N28)")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(
            view.records,
            (HeldRecordView("k1", 1, b"k1", Mark.NACKED), HeldRecordView("k2", 1, b"k2", Mark.ACKED)),
            "the first marks stand",
        )
        self.assertEqual(view.expires_at, T0 + 10 + LEASE_TTL, "not even the lease's expiry moved")
        # The Rust's own proof, kept: a renew at an earlier time extends from it.
        clock.set(T0 + 100)
        queue.renew("c1", lease_id)
        self.assertEqual(only_lease(queue).expires_at, T0 + 100 + LEASE_TTL)


class LeaseViewTest(unittest.TestCase):
    # impl-choice (spec note N23): the surface exposes the held records in no
    # order; the double lists them smallest position first.
    # scenario: S090
    def test_the_lease_read_shows_every_held_record_with_its_mark_smallest_position_first(self) -> None:
        queue, _ = scenario_queue()
        for key in ("z", "a", "m"):
            queue.publish(PUBLISHER, "pk", key, 1, key.encode())
        lease_id = batch(queue, "c1", 10).lease
        queue.ack("c1", lease_id, "a")
        queue.nack("c1", lease_id, "m")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(
            view.records,
            (
                HeldRecordView("z", 1, b"z", Mark.UNSETTLED),
                HeldRecordView("a", 1, b"a", Mark.ACKED),
                HeldRecordView("m", 1, b"m", Mark.NACKED),
            ),
        )
        self.assertEqual((view.pocket, view.expires_at), ("pk", T0 + LEASE_TTL))

    def test_a_consumer_with_no_active_lease_sees_none(self) -> None:
        queue, _ = scenario_queue()
        self.assertIsNone(queue.lease_view("c1"))

    def test_it_shows_only_the_consumers_own_lease(self) -> None:
        queue, _ = scenario_queue()
        queue.publish(PUBLISHER, "a", "a1", 1, b"a1")
        queue.publish(PUBLISHER, "b", "b1", 1, b"b1")
        batch(queue, "c1", 10)
        batch(queue, "c2", 10)
        views = (queue.lease_view("c1"), queue.lease_view("c2"), queue.lease_view("c3"))
        shown = [None if v is None else (v.pocket, [r.key for r in v.records]) for v in views]
        self.assertEqual(shown, [("a", ["a1"]), ("b", ["b1"]), None])

    # impl-choice (spec note N22): the surface faces a Consumer entity.
    def test_a_consumer_that_never_joined_is_refused_and_reads_no_clock(self) -> None:
        queue, clock = scenario_queue()
        before, reads = queue.snapshot(), clock.reads
        with self.assertRaises(UnknownConsumer):
            queue.lease_view("nobody")
        self.assertEqual((queue.snapshot(), clock.reads), (before, reads))

    def test_reading_the_lease_view_changes_nothing_and_reads_no_clock(self) -> None:
        queue, clock, _ = held_k()
        before, reads = queue.snapshot(), clock.reads
        queue.lease_view("c1")
        self.assertEqual((queue.snapshot(), clock.reads), (before, reads))
