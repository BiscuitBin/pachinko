"""The Publish section: the four Publish rules PD01 lands, the equal-rank
refusal, and the refusal of a publisher that never joined.

Each test drives `Queue.publish` and reads the outcome through the snapshot,
except PublishGuardsTest, which asks the registered Publish table's guards
directly whether the specification's disjoint-guards claim holds.
"""

import unittest

from pachinko_double.inspection import Snapshot, View
from pachinko_double.model import PocketStatus, Record, RecordStatus, ShedReason, SupersededRecord
from pachinko_double.publish import PUBLISH, Publish
from pachinko_double.queue import Queue
from pachinko_double.refusals import EqualRankRefused, UnknownPublisher

from .fixtures import PUBLISHER, SUPERSEDED_TTL, T0, scenario_queue


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)


def record(state: Snapshot, pocket_key: str, record_key: str) -> Record:
    found = state.record(pocket_key, record_key)
    assert found is not None, f"no record {pocket_key!r}/{record_key!r}"
    return found


def pocket_order(state: Snapshot) -> list[str]:
    """The queued pockets' keys, smallest position first: the pocket order."""
    queued = sorted((p for p in state.pockets if p.status is PocketStatus.QUEUED), key=lambda p: p.position)
    return [p.key for p in queued]


class UnknownPublisherTest(unittest.TestCase):
    # impl-choice (spec note N10): the Publish trigger takes a Publisher entity, so
    # the double refuses an id that never joined before any rule is consulted.
    def test_a_publish_by_a_publisher_that_never_joined_is_refused_and_changes_nothing(self) -> None:
        queue, clock = scenario_queue()
        before, reads = queue.snapshot(), clock.reads
        with self.assertRaises(UnknownPublisher) as raised:
            queue.publish("stranger", "pk", "a", 1, b"")
        self.assertEqual(raised.exception.publisher_id, "stranger")
        self.assertEqual(queue.snapshot(), before)
        self.assertEqual(clock.reads, reads)


class NewPocketTest(unittest.TestCase):
    # impl-choice (spec note N04): the absolute positions 0, 1 and 2 rest on the
    # double's first position, 0, which the specification leaves unstated; the
    # pocket-before-record order and the step of two are the specification's.
    # scenario: S011
    def test_a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record(self) -> None:
        queue, _ = scenario_queue()
        self.assertEqual(queue.snapshot().next_position, 0)
        publish(queue, "pk", "a", 3, b"x")
        state = queue.snapshot()
        (pocket,) = state.pockets_keyed("pk")
        self.assertEqual((pocket.status, pocket.position, pocket.consecutive_expiries), (PocketStatus.QUEUED, 0, 0))
        created = record(state, "pk", "a")
        self.assertEqual((created.position, created.status), (1, RecordStatus.QUEUED))
        self.assertEqual((created.rank, created.payload), (3, b"x"))
        self.assertEqual(state.next_position, 2)
        self.assertEqual(pocket_order(state), ["pk"])

    # scenario: S013
    def test_the_same_record_key_in_two_pockets_is_two_records(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "one", "k", 1, b"x")
        publish(queue, "two", "k", 1, b"y")
        state = queue.snapshot()
        one, two = record(state, "one", "k"), record(state, "two", "k")
        self.assertEqual((one.rank, one.payload), (1, b"x"))
        self.assertEqual((two.rank, two.payload), (1, b"y"))
        self.assertNotEqual(one.position, two.position)
        self.assertEqual(state.superseded, ())


class ExistingKeyTest(unittest.TestCase):
    # impl-choice (spec note N04): position 1 and next_position 3 rest on the
    # double's first position, 0; that the kept record keeps its position is
    # the specification's.
    # scenario: S014
    def test_a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser(self) -> None:
        queue, clock = scenario_queue()
        publish(queue, "pk", "a", 1, b"old")
        publish(queue, "pk", "b", 1, b"b")
        clock.advance(7)
        publish(queue, "pk", "a", 2, b"new")
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.position, kept.status), (1, RecordStatus.QUEUED))
        self.assertEqual((kept.rank, kept.payload), (2, b"new"))
        self.assertEqual(state.next_position, 3)
        now = T0 + 7
        self.assertEqual(
            state.superseded,
            (SupersededRecord("pk", "a", 1, b"old", ShedReason.SUPERSEDED, now, now + SUPERSEDED_TTL),),
        )

    # impl-choice (spec note N04): next_position 2 rests on the double's first
    # position, 0; that a stale publish allots nothing is the specification's.
    # scenario: S016
    def test_a_stale_publish_is_shed_not_applied_and_not_an_error(self) -> None:
        queue, clock = scenario_queue()
        publish(queue, "pk", "a", 5, b"kept")
        clock.advance(3)
        pocket_before = queue.snapshot().pockets
        publish(queue, "pk", "a", 4, b"late")
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.rank, kept.payload), (5, b"kept"))
        self.assertEqual(state.next_position, 2)
        self.assertEqual(state.pockets, pocket_before)
        now = T0 + 3
        self.assertEqual(
            state.superseded,
            (SupersededRecord("pk", "a", 4, b"late", ShedReason.STALE, now, now + SUPERSEDED_TTL),),
        )

    # scenario: S017
    def test_an_equal_rank_publish_is_rejected(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", 2, b"first")
        before = queue.snapshot()
        with self.assertRaises(EqualRankRefused) as raised:
            publish(queue, "pk", "a", 2, b"second")
        refused = raised.exception
        self.assertEqual((refused.pocket_key, refused.record_key, refused.rank), ("pk", "a", 2))
        self.assertEqual(queue.snapshot(), before)

    # scenario: S018
    def test_negative_and_zero_ranks_order_like_integers(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", -5, b"m5")
        publish(queue, "pk", "a", -1, b"m1")
        publish(queue, "pk", "a", 0, b"z")
        publish(queue, "pk", "a", -2, b"m2")
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.rank, kept.payload), (0, b"z"))
        self.assertEqual(
            [(shed.rank, shed.reason) for shed in state.superseded],
            [(-5, ShedReason.SUPERSEDED), (-1, ShedReason.SUPERSEDED), (-2, ShedReason.STALE)],
        )


class PublishGuardsTest(unittest.TestCase):
    """The specification's claim that the Publish guards are disjoint, over the
    states PD01 can reach: no pocket, a pocket without the key, and a queued
    record below, at and above the published rank."""

    def holding(self, queue: Queue, rank: int) -> list[str]:
        view = View(queue.snapshot(), T0)
        trigger = Publish(PUBLISHER, "pk", "a", rank, b"v")
        return [row.name for row in PUBLISH.rows if row.guard(view, trigger)]

    def test_at_most_one_guard_holds_and_it_is_the_expected_rule(self) -> None:
        empty, _ = scenario_queue()
        self.assertEqual(self.holding(empty, 1), ["PublishIntoNewPocket"])
        other_key, _ = scenario_queue()
        publish(other_key, "pk", "b", 1, b"v")
        self.assertEqual(self.holding(other_key, 1), ["PublishNewKey"])
        stored, _ = scenario_queue()
        publish(stored, "pk", "a", 5, b"v")
        self.assertEqual(self.holding(stored, 6), ["Supersede"])
        self.assertEqual(self.holding(stored, 4), ["DropStalePublish"])
        self.assertEqual(self.holding(stored, 5), [])
