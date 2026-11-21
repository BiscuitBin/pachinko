"""The Publish section: the four Publish rules PD01 lands, the equal-rank
refusal, and the refusal of a publisher that never joined.

Each test drives `Queue.publish` and reads the outcome through the snapshot,
and where the Rust asserts a `PublishOutcome` it asserts that the publish
answers nothing (spec note N32), except PublishGuardsTest, which asks the registered Publish table's guards
directly whether the specification's disjoint-guards claim holds.
"""

import unittest

from pachinko_double.inspection import Snapshot, View
from pachinko_double.model import PocketStatus, Record, RecordStatus, ShedReason, SupersededRecord
from pachinko_double.publish import PUBLISH, Publish
from pachinko_double.queue import Queue
from pachinko_double.refusals import EqualRankRefused, UnknownPublisher

from .fixtures import (
    PUBLISHER,
    SUPERSEDED_TTL,
    T0,
    batch,
    keys,
    publish_answers_nothing,
    scenario_queue,
    two_pockets,
)


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
        publish_answers_nothing(self, queue, "pk", "a", 3, b"x")
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
        publish_answers_nothing(self, queue, "one", "k", 1, b"x")
        publish_answers_nothing(self, queue, "two", "k", 1, b"y")
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
        publish_answers_nothing(self, queue, "pk", "a", 2, b"new")
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
        publish_answers_nothing(self, queue, "pk", "a", 4, b"late")
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
        publish_answers_nothing(self, queue, "pk", "a", -1, b"m1")
        publish_answers_nothing(self, queue, "pk", "a", 0, b"z")
        publish_answers_nothing(self, queue, "pk", "a", -2, b"m2")
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.rank, kept.payload), (0, b"z"))
        self.assertEqual(
            [(shed.rank, shed.reason) for shed in state.superseded],
            [(-5, ShedReason.SUPERSEDED), (-1, ShedReason.SUPERSEDED), (-2, ShedReason.STALE)],
        )


class PublishThenPullTest(unittest.TestCase):
    """Publish rows whose Rust tests finish with a pull: what was published is
    what the pull hands out."""

    # impl-choice (spec note N16): positions 0 and 2 and next_position 3 rest on
    # the double's first position, 0 (N04); that the new key takes the next
    # position and the pocket keeps its own is the specification's.
    # scenario: S012
    def test_a_publish_of_a_new_key_appends_to_an_existing_pocket(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", 1, b"x")
        publish_answers_nothing(self, queue, "pk", "b", 7, b"y")
        state = queue.snapshot()
        self.assertEqual(len(state.pockets), 1)
        self.assertEqual(state.pockets_keyed("pk")[0].position, 0)
        appended = record(state, "pk", "b")
        self.assertEqual((appended.position, appended.status), (2, RecordStatus.QUEUED))
        self.assertEqual(state.next_position, 3)
        self.assertEqual(keys(batch(queue, "c1", 10)), ["a", "b"])

    # impl-choice (spec note N16): position 1 rests on the double's first
    # position, 0 (N04); that the key keeps its first slot is the specification's.
    # scenario: S015
    def test_repeated_supersede_keeps_the_first_slot_and_sheds_every_loser(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", 1, b"v1")
        publish(queue, "pk", "b", 1, b"b")
        for rank, payload in ((2, b"v2"), (3, b"v3"), (4, b"v4")):
            publish_answers_nothing(self, queue, "pk", "a", rank, payload)
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.position, kept.rank, kept.payload), (1, 4, b"v4"))
        self.assertEqual(
            [(shed.rank, shed.payload, shed.reason) for shed in state.superseded],
            [(1, b"v1", ShedReason.SUPERSEDED), (2, b"v2", ShedReason.SUPERSEDED), (3, b"v3", ShedReason.SUPERSEDED)],
        )
        answer = batch(queue, "c1", 10)
        self.assertEqual(keys(answer), ["a", "b"])
        self.assertEqual((answer.records[0].rank, answer.records[0].payload), (4, b"v4"))

    # The ranks are the Rust test's, a 64-bit integer's extremes; the double's
    # ranks are unbounded, so they are not its extremes (spec notes N03, N17).
    # scenario: S019
    def test_large_ranks_keep_full_precision_across_supersede_and_stale(self) -> None:
        low, high = -(2**63), 2**63 - 1
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", low, b"min")
        for rank, payload in ((low + 1, b"min+1"), (high - 1, b"max-1"), (high, b"max")):
            publish_answers_nothing(self, queue, "pk", "a", rank, payload)
        publish_answers_nothing(self, queue, "pk", "a", high - 1, b"late")
        state = queue.snapshot()
        kept = record(state, "pk", "a")
        self.assertEqual((kept.rank, kept.payload), (high, b"max"))
        self.assertEqual(
            [(shed.rank, shed.reason) for shed in state.superseded],
            [
                (low, ShedReason.SUPERSEDED),
                (low + 1, ShedReason.SUPERSEDED),
                (high - 1, ShedReason.SUPERSEDED),
                (high - 1, ShedReason.STALE),
            ],
        )
        answer = batch(queue, "c1", 1)
        self.assertEqual(answer.records[0].rank, high)
        self.assertEqual(record(queue.snapshot(), "pk", "a").held_rank, high)

    # Payloads are bytes (spec note N02) and keys are str, compared exactly.
    # scenario: S020
    def test_payload_bytes_and_keys_round_trip_unchanged(self) -> None:
        every_byte = bytes(range(256))
        pocket_key, record_key = "pocket-\u00e9\u4e2d\U0001f600", "key-\u03b1\u03b2\u0000"
        queue, _ = scenario_queue()
        publish(queue, pocket_key, record_key, 1, every_byte)
        answer = batch(queue, "c1", 1)
        self.assertEqual(answer.pocket, pocket_key)
        self.assertEqual(answer.records[0].key, record_key)
        self.assertEqual(answer.records[0].payload, every_byte)
        held = record(queue.snapshot(), pocket_key, record_key)
        self.assertEqual((held.key, held.held_payload, held.payload), (record_key, every_byte, every_byte))

    # impl-choice (spec note N16): position 0 rests on the double's first
    # position, 0 (N04); that the pocket's position does not move is the
    # specification's.
    # scenario: S022
    def test_publishing_into_a_queued_pocket_does_not_move_it_in_the_pocket_order(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "first", "a", 1, b"a")
        publish(queue, "second", "a", 1, b"a")
        publish(queue, "first", "b", 1, b"b")
        publish(queue, "first", "a", 2, b"a2")
        publish(queue, "first", "a", 0, b"a0")
        state = queue.snapshot()
        (first,) = state.pockets_keyed("first")
        self.assertEqual((first.position, first.status), (0, PocketStatus.QUEUED))
        self.assertEqual(pocket_order(state), ["first", "second"])
        self.assertEqual(batch(queue, "c1", 0).pocket, "first")


class RevivalTest(unittest.TestCase):
    # scenario: S023
    def test_a_publish_revives_an_idle_pocket_at_the_back(self) -> None:
        queue, _ = two_pockets()
        lease_id = batch(queue, "c1", 10).lease
        queue.ack("c1", lease_id, "a1")
        queue.ack("c1", lease_id, "a2")
        queue.release("c1", lease_id)
        (a,) = queue.snapshot().pockets_keyed("a")
        self.assertEqual(a.status, PocketStatus.IDLE)
        publish(queue, "c", "c1", 1, b"c")
        next_position = queue.snapshot().next_position
        publish_answers_nothing(self, queue, "a", "a3", 1, b"a3")
        state = queue.snapshot()
        (a,) = state.pockets_keyed("a")
        self.assertEqual(a.status, PocketStatus.QUEUED)
        self.assertEqual(record(state, "a", "a3").position, next_position, "PublishNewKey takes its position first")
        self.assertEqual(a.position, next_position + 1, "then the revive: the back")
        self.assertEqual(a.consecutive_expiries, 0)
        self.assertEqual(state.next_position, next_position + 2)
        self.assertEqual(pocket_order(state), ["b", "c", "a"])
        publish(queue, "a", "a4", 1, b"a4")
        (a,) = queue.snapshot().pockets_keyed("a")
        self.assertEqual(a.position, next_position + 1, "a queued pocket stays put")


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
