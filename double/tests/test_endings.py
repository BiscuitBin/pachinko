"""The lease's end by release or by lapse, and the consumer's view of a lease.

The six `Freed` rows read the mark and never how the lease ended, so the
specification's reconciliation is the same whichever way the lease ends.
Each scenario the Rust runs over its `ENDINGS` runs here twice, as two
subtests: once released at 50 ms, once lapsed by a tick at 2,000 ms, past
every expiry these tests reach and before any entry they shed decays. Each
half asserts its own end, the lease's status and `ended_at`, and the count
that end leaves. Those assertions follow the ending the loop chose, so each
test also records the status each ended lease shows in the queue and, after
its loop, asserts one `released` and one `expired`: a loop that skipped an
ending, ran one twice or ran none would fail.

Positions are asserted relative to the state before the step, never as the
absolute values that rest on the double's first position (spec note N04).
Publish answers nothing in the double, so a deferral is asserted as that
answer and the state it leaves (spec note N32).
"""

import unittest
from enum import Enum

from pachinko_double.effects import FakeClock
from pachinko_double.invariants import check
from pachinko_double.model import (
    DeadLetter,
    LeaseStatus,
    Mark,
    Pocket,
    PocketStatus,
    Record,
    RecordStatus,
    ShedReason,
    SupersededRecord,
    Timestamp,
)
from pachinko_double.queue import Queue

from .fixtures import (
    DEAD_LETTER_TTL,
    LEASE_TTL,
    PUBLISHER,
    SUPERSEDED_TTL,
    T0,
    batch,
    keys,
    publish_answers_nothing,
    scenario_queue,
    two_pockets,
)
from .test_publish import pocket_order


def at(millis: int) -> Timestamp:
    """`T0` plus `millis`."""
    return T0 + millis


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)


def record(queue: Queue, pocket_key: str, record_key: str) -> Record:
    found = queue.snapshot().record(pocket_key, record_key)
    assert found is not None, f"no record {pocket_key!r}/{record_key!r}"
    return found


def pocket(queue: Queue, key: str) -> Pocket:
    (found,) = queue.snapshot().pockets_keyed(key)
    return found


def held_as(found: Record) -> tuple[object, ...]:
    """A held record's status and held version, as the Rust's `held_as` reads it."""
    return (found.status, found.held_rank, found.held_payload, found.mark, found.nacked_at, found.deferred_position)


class Ending(Enum):
    """How a test ends c1's lease."""

    RELEASE = "release"
    LAPSE = "lapse"

    def end(self, test: unittest.TestCase, queue: Queue, clock: FakeClock, lease_id: str) -> Timestamp:
        """End the lease this way, check that it ended so, and return when."""
        if self is Ending.RELEASE:
            clock.set(at(50))
            queue.release("c1", lease_id)
            (ended,) = (lease for lease in queue.snapshot().leases if lease.id == lease_id)
            test.assertEqual((ended.status, ended.ended_at), (LeaseStatus.RELEASED, at(50)))
            return at(50)
        clock.set(at(2_000))
        report = queue.tick()
        test.assertEqual([lease.id for lease in report.lapsed], [lease_id], "the lease lapsed")
        test.assertEqual((report.lapsed[0].status, report.lapsed[0].ended_at), (LeaseStatus.EXPIRED, at(2_000)))
        return at(2_000)

    @property
    def count(self) -> int:
        """The pocket's count after this end of a lease that started at 0:
        a release resets it and a lapse counts one."""
        return 0 if self is Ending.RELEASE else 1


ENDINGS = (Ending.RELEASE, Ending.LAPSE)


def v3_deferred_behind_v1(test: unittest.TestCase) -> tuple[Queue, FakeClock, str, int, int]:
    """pk with k at rank 1, payload v1, then early; c1 pulls k alone at T0, and
    v3 arrives at 10 ms, deferred behind v1. Returns the queue, its clock, the
    lease, k's held position and v3's arrival position."""
    queue, clock = scenario_queue()
    publish(queue, "pk", "k", 1, b"v1")
    publish(queue, "pk", "early", 1, b"e")
    lease_id = batch(queue, "c1", 1).lease
    position, arrival = record(queue, "pk", "k").position, queue.snapshot().next_position
    clock.set(at(10))
    publish_answers_nothing(test, queue, "pk", "k", 3, b"v3")
    deferred = record(queue, "pk", "k")
    test.assertEqual(
        (held_as(deferred), deferred.rank),
        ((RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival), 3),
        "v3 is deferred behind the held v1",
    )
    return queue, clock, lease_id, position, arrival


class EndingsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.ended: list[LeaseStatus] = []

    def end(self, ending: Ending, queue: Queue, clock: FakeClock, lease_id: str) -> Timestamp:
        """End the lease `ending`'s way, record the status the queue then shows
        it in, and return when it ended."""
        when = ending.end(self, queue, clock, lease_id)
        (ended,) = (lease for lease in queue.snapshot().leases if lease.id == lease_id)
        self.ended.append(ended.status)
        return when

    def assert_both_endings_ran(self) -> None:
        """The loop ended one lease by release and one by lapse, as the queue
        shows them, whatever `ENDINGS` holds."""
        self.assertCountEqual(self.ended, [LeaseStatus.RELEASED, LeaseStatus.EXPIRED], "one release and one lapse")

    def after_the_end(self, queue: Queue, ending: Ending, rank: int, position: int, order: list[str]) -> None:
        """pk is queued with the count `ending` leaves, k is queued at `rank`
        and `position`, and c2 is handed `order`."""
        found = pocket(queue, "pk")
        self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.QUEUED, ending.count))
        k = record(queue, "pk", "k")
        self.assertEqual((k.status, k.rank, k.position), (RecordStatus.QUEUED, rank, position))
        self.assertEqual(keys(batch(queue, "c2", 10)), order)
        self.assertEqual(check(queue.snapshot()), ())

    # scenario: S078
    def test_scenario_a_an_acked_version_is_consumed_and_the_deferred_one_queued_at_its_arrival(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock, lease_id, _, arrival = v3_deferred_behind_v1(self)
                clock.set(at(20))
                queue.ack("c1", lease_id, "k")
                self.end(ending, queue, clock, lease_id)
                state = queue.snapshot()
                self.assertEqual(state.superseded, (), "v1 is recorded nowhere")
                self.assertEqual(state.dead_letters, ())
                self.after_the_end(queue, ending, 3, arrival, ["early", "k"])
        self.assert_both_endings_ran()

    # scenario: S079
    def test_scenario_b_a_nacked_version_is_dead_lettered_and_the_deferred_one_queued_at_its_arrival(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock, lease_id, _, arrival = v3_deferred_behind_v1(self)
                clock.set(at(20))
                queue.nack("c1", lease_id, "k")
                self.assertEqual(queue.snapshot().dead_letters, ())
                ended = self.end(ending, queue, clock, lease_id)
                state = queue.snapshot()
                letter = DeadLetter("pk", "k", 1, b"v1", "c1", at(20), ended + DEAD_LETTER_TTL)
                self.assertEqual(state.dead_letters, (letter,))
                self.assertEqual(state.superseded, ())
                self.after_the_end(queue, ending, 3, arrival, ["early", "k"])
        self.assert_both_endings_ran()

    # scenario: S080
    def test_scenario_c_v3_is_shed_when_v5_arrives_and_v5_takes_v3s_arrival_position(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock, lease_id, _, arrival = v3_deferred_behind_v1(self)
                clock.set(at(15))
                publish(queue, "pk", "between", 1, b"b")
                clock.set(at(18))
                publish(queue, "pk", "k", 5, b"v5")
                v3 = SupersededRecord("pk", "k", 3, b"v3", ShedReason.SUPERSEDED, at(18), at(18) + SUPERSEDED_TTL)
                self.assertEqual(queue.snapshot().superseded, (v3,), "shed at once")
                clock.set(at(20))
                queue.nack("c1", lease_id, "k")
                ended = self.end(ending, queue, clock, lease_id)
                state = queue.snapshot()
                self.assertEqual(state.superseded, (v3,))
                letter = DeadLetter("pk", "k", 1, b"v1", "c1", at(20), ended + DEAD_LETTER_TTL)
                self.assertEqual(state.dead_letters, (letter,))
                self.after_the_end(queue, ending, 5, arrival, ["early", "k", "between"])
        self.assert_both_endings_ran()

    # scenario: S081
    def test_scenario_d_an_unsettled_version_is_superseded_at_the_end_and_v3_takes_its_position(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock, lease_id, position, _ = v3_deferred_behind_v1(self)
                self.assertEqual(queue.snapshot().superseded, ())
                ended = self.end(ending, queue, clock, lease_id)
                state = queue.snapshot()
                v1 = SupersededRecord("pk", "k", 1, b"v1", ShedReason.SUPERSEDED, ended, ended + SUPERSEDED_TTL)
                self.assertEqual(state.superseded, (v1,), "shed at the end")
                self.assertEqual(state.dead_letters, ())
                self.after_the_end(queue, ending, 3, position, ["k", "early"])
        self.assert_both_endings_ran()

    # scenario: S087
    def test_a_pocket_whose_records_were_all_acked_or_nacked_idles_on_release_and_on_lapse(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock = two_pockets()
                lease_id = batch(queue, "c1", 10).lease
                clock.set(at(20))
                queue.ack("c1", lease_id, "a1")
                queue.nack("c1", lease_id, "a2")
                self.assertEqual(queue.snapshot().dead_letters, ())
                ended = self.end(ending, queue, clock, lease_id)
                found = pocket(queue, "a")
                records = queue.snapshot().records_by_pocket["a"]
                self.assertEqual(
                    (found.status, found.consecutive_expiries, len(records)), (PocketStatus.IDLE, ending.count, 0)
                )
                letter = DeadLetter("a", "a2", 1, b"a2", "c1", at(20), ended + DEAD_LETTER_TTL)
                self.assertEqual(queue.snapshot().dead_letters, (letter,))
                self.assertEqual(pocket_order(queue.snapshot()), ["b"])
        self.assert_both_endings_ran()

    # scenario: S088
    def test_a_pocket_whose_records_were_all_acked_with_one_deferred_version_is_queued(self) -> None:
        for ending in ENDINGS:
            with self.subTest(ending=ending.value):
                queue, clock = two_pockets()
                lease_id = batch(queue, "c1", 10).lease
                arrival = queue.snapshot().next_position
                clock.set(at(10))
                publish(queue, "a", "a2", 2, b"newer")
                self.assertEqual(
                    held_as(record(queue, "a", "a2")), (RecordStatus.HELD, 1, b"a2", Mark.UNSETTLED, None, arrival)
                )
                clock.set(at(20))
                for key in ("a1", "a2"):
                    queue.ack("c1", lease_id, key)
                self.end(ending, queue, clock, lease_id)
                found = pocket(queue, "a")
                self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.QUEUED, ending.count))
                (left,) = queue.snapshot().records_by_pocket["a"]
                self.assertEqual(
                    (left.key, left.status, left.rank, left.position), ("a2", RecordStatus.QUEUED, 2, arrival)
                )
                state = queue.snapshot()
                self.assertEqual((state.superseded, state.dead_letters), ((), ()))
        self.assert_both_endings_ran()

    # scenario: S089
    def test_at_an_expiry_limit_of_one_a_pocket_of_nacks_parks_with_no_records_and_its_dead_letters(self) -> None:
        queue, clock = scenario_queue(max_consecutive_expiries=1)
        publish(queue, "a", "a1", 1, b"a1")
        publish(queue, "a", "a2", 1, b"a2")
        lease_id = batch(queue, "c1", 10).lease
        clock.set(at(20))
        for key in ("a1", "a2"):
            queue.nack("c1", lease_id, key)
        self.assertEqual(queue.snapshot().dead_letters, (), "not before the lapse")
        ended = Ending.LAPSE.end(self, queue, clock, lease_id)
        found = pocket(queue, "a")
        records = queue.snapshot().records_by_pocket["a"]
        self.assertEqual((found.status, found.consecutive_expiries, len(records)), (PocketStatus.PARKED, 1, 0))
        expires_at = ended + DEAD_LETTER_TTL
        letters = (
            DeadLetter("a", "a1", 1, b"a1", "c1", at(20), expires_at),
            DeadLetter("a", "a2", 1, b"a2", "c1", at(20), expires_at),
        )
        self.assertEqual(queue.snapshot().dead_letters, letters)

    # The Rust lists the pocket's records in its snapshot's order; the double
    # compares them by key, as the maintainer ruled for S091 (spec note N31).
    # scenario: S093
    def test_at_an_expiry_limit_of_one_a_marked_version_with_a_deferred_one_parks_with_it_queued(self) -> None:
        for mark in (Mark.ACKED, Mark.NACKED):
            with self.subTest(mark=mark.value):
                queue, clock = scenario_queue(max_consecutive_expiries=1)
                publish(queue, "pk", "k", 1, b"v1")
                publish(queue, "pk", "early", 1, b"e")
                publish(queue, "other", "o", 1, b"o")
                lease_id = batch(queue, "c1", 1).lease
                early, arrival = record(queue, "pk", "early").position, queue.snapshot().next_position
                clock.set(at(10))
                publish(queue, "pk", "k", 3, b"v3")
                self.assertEqual(
                    held_as(record(queue, "pk", "k")), (RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival)
                )
                clock.set(at(20))
                if mark is Mark.ACKED:
                    queue.ack("c1", lease_id, "k")
                else:
                    queue.nack("c1", lease_id, "k")
                ended = Ending.LAPSE.end(self, queue, clock, lease_id)
                found = pocket(queue, "pk")
                self.assertEqual((found.status, found.consecutive_expiries), (PocketStatus.PARKED, 1))
                self.assertEqual(pocket_order(queue.snapshot()), ["other"])
                records = queue.snapshot().records_by_pocket["pk"]
                kept = {r.key: (r.status, r.rank, r.payload, r.position) for r in records}
                self.assertEqual(
                    kept,
                    {"early": (RecordStatus.QUEUED, 1, b"e", early), "k": (RecordStatus.QUEUED, 3, b"v3", arrival)},
                    "v3 is kept, queued at its arrival position",
                )
                state = queue.snapshot()
                self.assertEqual(state.superseded, ())
                letters = (
                    (DeadLetter("pk", "k", 1, b"v1", "c1", at(20), ended + DEAD_LETTER_TTL),)
                    if mark is Mark.NACKED
                    else ()
                )
                self.assertEqual(state.dead_letters, letters)


class ConsumerViewTest(unittest.TestCase):
    # scenario: S076
    def test_the_consumer_view_exposes_only_the_active_lease_and_its_held_versions(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "k", 1, b"v1")
        publish(queue, "pk", "queued", 1, b"q")
        publish(queue, "other", "x", 1, b"x")
        lease_id = batch(queue, "c1", 1).lease
        batch(queue, "c2", 10)
        publish(queue, "pk", "k", 3, b"v3")
        self.assertEqual(queue.snapshot().superseded, (), "v1 is held, not shed")
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual((view.pocket, view.expires_at), ("pk", T0 + LEASE_TTL))
        self.assertEqual([held.key for held in view.records], ["k"], "only the held records")
        self.assertEqual((view.records[0].held_rank, view.records[0].held_payload), (1, b"v1"), "the held version")
        self.assertEqual(view.records[0].mark, Mark.UNSETTLED)
        queue.release("c1", lease_id)
        self.assertIsNone(queue.lease_view("c1"))
        other = queue.lease_view("c2")
        assert other is not None
        self.assertEqual([held.key for held in other.records], ["x"])
