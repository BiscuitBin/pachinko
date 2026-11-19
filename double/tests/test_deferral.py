"""Deferral behind the held version: a newer version published onto a held
key, the supersede of a deferred version, and the lease-end rows that queue
it when the lease is released.

`DeferBehindHeldVersion` answers a higher rank onto a held record with
nothing deferred: `rank` and `payload` move to the new version, and
`deferred_position` takes the counter's value, its arrival position. The held
version, `held_rank` and `held_payload`, does not change, whatever its mark.
A still higher rank is `Supersede`'s held branch: the deferred version is
shed at once and the newer one inherits the arrival position.

Each test drives `Queue` and reads the outcome through the snapshot or the
lease view. Publish answers nothing in the double, so the Rust's
`PublishOutcome` is asserted as the state it leaves (spec note N32).
Positions are asserted relative to the state before the step, never as the
absolute values that rest on the double's first position (spec note N04).
Every refusal compares the complete snapshot before and after.
"""

import unittest
from dataclasses import replace

from pachinko_double.consuming import HeldRecordView
from pachinko_double.effects import FakeClock
from pachinko_double.inspection import View
from pachinko_double.invariants import check
from pachinko_double.model import (
    DeadLetter,
    Mark,
    PocketStatus,
    Record,
    RecordStatus,
    ShedReason,
    SupersededRecord,
    Timestamp,
)
from pachinko_double.publish import PUBLISH, PocketTouched, Publish
from pachinko_double.pull import NoPocketAvailable
from pachinko_double.queue import Queue
from pachinko_double.refusals import EqualRankRefused
from pachinko_double.rules import Registry, dispatch

from .fixtures import DEAD_LETTER_TTL, PUBLISHER, SUPERSEDED_TTL, T0, batch, keys, refused, scenario_queue
from .test_pull import RecordingOutcome


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)


def record(queue: Queue, pocket_key: str, record_key: str) -> Record:
    found = queue.snapshot().record(pocket_key, record_key)
    assert found is not None, f"no record {pocket_key!r}/{record_key!r}"
    return found


def held_as(found: Record) -> tuple[object, ...]:
    """A held record's status and held version, as the Rust's `held_as` reads it:
    status, held_rank, held_payload, mark, nacked_at and deferred_position."""
    return (found.status, found.held_rank, found.held_payload, found.mark, found.nacked_at, found.deferred_position)


def shed_at(at: Timestamp, rank: int, payload: bytes, reason: ShedReason, key: str = "k") -> SupersededRecord:
    """An entry of pk's `key` in the superseded queue, shed at `at`."""
    return SupersededRecord("pk", key, rank, payload, reason, at, at + SUPERSEDED_TTL)


def held_k() -> tuple[Queue, FakeClock, str]:
    """Pocket pk holds k at rank 1, payload v1, pulled by c1 at T0."""
    queue, clock = scenario_queue()
    publish(queue, "pk", "k", 1, b"v1")
    return queue, clock, batch(queue, "c1", 10).lease


def held_and_marked(mark: Mark) -> tuple[Queue, FakeClock, str, Timestamp | None]:
    """`held_k`, then marked at T0 + 10 as `mark` says. Returns the nack's
    time when `mark` is a nack."""
    queue, clock, lease_id = held_k()
    clock.set(T0 + 10)
    if mark is Mark.ACKED:
        queue.ack("c1", lease_id, "k")
    elif mark is Mark.NACKED:
        queue.nack("c1", lease_id, "k")
    return queue, clock, lease_id, (T0 + 10 if mark is Mark.NACKED else None)


class DeferralTest(unittest.TestCase):
    # scenario: S043
    def test_a_supersede_while_the_key_is_held_is_deferred_behind_the_held_version(self) -> None:
        queue, _, lease_id = held_k()
        position, arrival = record(queue, "pk", "k").position, queue.snapshot().next_position
        publish(queue, "pk", "k", 3, b"v3")
        deferred = record(queue, "pk", "k")
        self.assertEqual(
            held_as(deferred),
            (RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival),
            "the newer version is deferred at the counter's value",
        )
        self.assertEqual((deferred.rank, deferred.payload, deferred.position), (3, b"v3", position))
        self.assertEqual(queue.snapshot().next_position, arrival + 1)
        self.assertEqual(queue.snapshot().superseded, (), "nothing is shed")
        queue.ack("c1", lease_id, "k")
        self.assertEqual(held_as(record(queue, "pk", "k")), (RecordStatus.HELD, 1, b"v1", Mark.ACKED, None, arrival))
        view = queue.lease_view("c1")
        assert view is not None
        lease_read = (HeldRecordView("k", 1, b"v1", Mark.ACKED),)
        self.assertEqual(view.records, lease_read, "the lease read shows the held version")
        self.assertEqual(check(queue.snapshot()), ())

    # scenario: S021
    def test_publishing_into_a_leased_pocket_neither_requeues_it_nor_escapes_the_lease(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "a", 1, b"a")
        lease_id = batch(queue, "c1", 10).lease
        (pocket_before,) = queue.snapshot().pockets_keyed("pk")
        leases_before = queue.snapshot().leases
        publish(queue, "pk", "b", 1, b"b")
        publish(queue, "pk", "a", 2, b"a2")
        publish(queue, "pk", "a", 0, b"a0")
        state = queue.snapshot()
        self.assertEqual(
            state.superseded,
            (shed_at(T0, 0, b"a0", ShedReason.STALE, key="a"),),
            "the held a1 is not shed; a2 waits behind it",
        )
        a = record(queue, "pk", "a")
        held_then_deferred = (a.status, a.held_rank, a.held_payload, a.rank, a.payload)
        self.assertEqual(held_then_deferred, (RecordStatus.HELD, 1, b"a", 2, b"a2"))
        (pocket,) = state.pockets_keyed("pk")
        self.assertEqual((pocket.status, pocket.position), (PocketStatus.LEASED, pocket_before.position))
        self.assertEqual([p.key for p in state.pockets if p.status is PocketStatus.QUEUED], [])
        self.assertEqual(state.leases, leases_before)
        self.assertIsInstance(queue.pull("c2", 10), NoPocketAvailable)
        again = batch(queue, "c1", 10)
        self.assertEqual(again.lease, lease_id)
        self.assertEqual(keys(again), ["b"])

    # scenario: S037
    def test_a_held_record_is_not_handed_out_again_under_the_same_lease(self) -> None:
        queue, _ = scenario_queue()
        for key in ("r0", "r1"):
            publish(queue, "pk", key, 1, key.encode())
        first = batch(queue, "c1", 1)
        publish(queue, "pk", "r0", 2, b"r0-v2")
        second = batch(queue, "c1", 10)
        self.assertEqual(second.lease, first.lease)
        self.assertEqual(keys(second), ["r1"])
        self.assertEqual(batch(queue, "c1", 10).records, ())
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual(view.records[0].held_rank, 1)


class JudgedAgainstTheStoredRankTest(unittest.TestCase):
    # scenario: S046
    def test_a_stale_check_while_held_compares_with_the_latest_rank_not_the_held_one(self) -> None:
        queue, _, _ = held_k()
        arrival = queue.snapshot().next_position
        publish(queue, "pk", "k", 3, b"v3")
        publish(queue, "pk", "k", 2, b"v2")
        self.assertEqual(queue.snapshot().superseded, (shed_at(T0, 2, b"v2", ShedReason.STALE),))
        refusal = refused(self, queue, EqualRankRefused, lambda: publish(queue, "pk", "k", 3, b"again"))
        named = (refusal.pocket_key, refusal.record_key, refusal.rank)
        self.assertEqual(named, ("pk", "k", 3), "against the latest rank")
        kept = record(queue, "pk", "k")
        self.assertEqual(kept.rank, 3)
        self.assertEqual(held_as(kept), (RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival))

    # scenario: S083
    def test_publishing_again_at_the_held_rank_is_rejected_until_a_newer_version_is_deferred_then_stale(self) -> None:
        queue, _, _, _ = held_and_marked(Mark.UNSETTLED)
        refusal = refused(self, queue, EqualRankRefused, lambda: publish(queue, "pk", "k", 1, b"again"))
        self.assertEqual(refusal.rank, 1, "nothing deferred: the held rank is the stored rank")
        arrival = queue.snapshot().next_position
        publish(queue, "pk", "k", 3, b"v3")
        deferred = (RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival)
        self.assertEqual(held_as(record(queue, "pk", "k")), deferred)
        publish(queue, "pk", "k", 1, b"again")
        self.assertEqual(
            queue.snapshot().superseded,
            (shed_at(T0 + 10, 1, b"again", ShedReason.STALE),),
            "v3 deferred: the held rank is below the stored rank",
        )
        self.assertEqual(held_as(record(queue, "pk", "k")), deferred)

    # scenario: S082
    def test_a_publish_onto_a_held_record_is_judged_alike_whether_it_is_unsettled_acked_or_nacked(self) -> None:
        for mark in Mark:
            with self.subTest(mark=mark):
                self.the_publish_table_against_a_held_record(mark)

    def the_publish_table_against_a_held_record(self, mark: Mark) -> None:
        queue, clock, _, nacked_at = held_and_marked(mark)
        position, arrival = record(queue, "pk", "k").position, queue.snapshot().next_position
        deferred = (RecordStatus.HELD, 1, b"v1", mark, nacked_at, arrival)
        clock.set(T0 + 20)
        publish(queue, "pk", "k", 3, b"v3")
        self.assertEqual((held_as(record(queue, "pk", "k")), record(queue, "pk", "k").rank), (deferred, 3), "higher")
        self.assertEqual(queue.snapshot().superseded, (), "nothing is shed")
        self.assertEqual(queue.snapshot().next_position, arrival + 1)
        clock.set(T0 + 30)
        publish(queue, "pk", "k", 5, b"v5")
        v3 = shed_at(T0 + 30, 3, b"v3", ShedReason.SUPERSEDED)
        self.assertEqual(queue.snapshot().superseded, (v3,), "a second higher sheds v3 at once")
        refusal = refused(self, queue, EqualRankRefused, lambda: publish(queue, "pk", "k", 5, b"again"))
        self.assertEqual(refusal.rank, 5, "equal")
        publish(queue, "pk", "k", 4, b"v4")
        state = queue.snapshot()
        self.assertEqual(state.superseded, (v3, shed_at(T0 + 30, 4, b"v4", ShedReason.STALE)), "lower")
        self.assertEqual(state.dead_letters, (), "no dead letter while held")
        kept = record(queue, "pk", "k")
        self.assertEqual(
            (held_as(kept), kept.rank, kept.payload, kept.position),
            (deferred, 5, b"v5", position),
            "v5 keeps v3's arrival position",
        )
        self.assertEqual(state.next_position, arrival + 1, "the second supersede allots nothing")
        self.assertEqual(check(state), ())


class ReleaseWithADeferredVersionTest(unittest.TestCase):
    """The deferred rows of the lease-end table, by release. The arrival
    position was allotted when the newer version arrived, so the release
    allots only the pocket's own fresh position."""

    # scenario: S045
    def test_two_supersedes_while_held_shed_only_the_deferred_version_and_never_the_held_one(self) -> None:
        queue, clock, lease_id = held_k()
        arrival = queue.snapshot().next_position
        clock.set(T0 + 10)
        publish(queue, "pk", "k", 3, b"v3")
        self.assertEqual(queue.snapshot().superseded, (), "v1 is held")
        clock.set(T0 + 20)
        publish(queue, "pk", "k", 5, b"v5")
        v3 = shed_at(T0 + 20, 3, b"v3", ShedReason.SUPERSEDED)
        self.assertEqual(queue.snapshot().superseded, (v3,), "v3 is shed at once")
        kept = record(queue, "pk", "k")
        self.assertEqual(
            (held_as(kept), kept.rank),
            ((RecordStatus.HELD, 1, b"v1", Mark.UNSETTLED, None, arrival), 5),
            "v5 keeps v3's arrival position",
        )
        queue.ack("c1", lease_id, "k")
        clock.set(T0 + 30)
        before_release = queue.snapshot().next_position
        queue.release("c1", lease_id)
        state = queue.snapshot()
        self.assertEqual(state.superseded, (v3,), "the acked v1 is consumed")
        self.assertEqual(state.dead_letters, ())
        queued = record(queue, "pk", "k")
        queued_at = (queued.status, queued.position, queued.rank, queued.payload)
        self.assertEqual(queued_at, (RecordStatus.QUEUED, arrival, 5, b"v5"))
        self.assertEqual(state.next_position, before_release + 1, "only the pocket's fresh position is allotted")

    # scenario: S044
    def test_a_version_deferred_behind_an_acked_one_is_queued_at_its_arrival_position(self) -> None:
        queue, _ = scenario_queue()
        publish(queue, "pk", "k", 1, b"v1")
        publish(queue, "pk", "early", 1, b"e")
        lease_id = batch(queue, "c1", 1).lease
        position, arrival = record(queue, "pk", "k").position, queue.snapshot().next_position
        publish(queue, "pk", "k", 3, b"v3")
        publish(queue, "pk", "between", 1, b"b")
        queue.ack("c1", lease_id, "k")
        kept = record(queue, "pk", "k").position
        self.assertEqual(kept, position, "a held record keeps its position until the lease ends")
        again = batch(queue, "c1", 10)
        self.assertEqual(again.lease, lease_id)
        self.assertEqual(keys(again), ["early", "between"], "the deferred key is not handed out again under this lease")
        queue.release("c1", lease_id)
        queued = record(queue, "pk", "k")
        self.assertEqual((queued.status, queued.position, queued.rank), (RecordStatus.QUEUED, arrival, 3))
        self.assertEqual((queue.snapshot().superseded, queue.snapshot().dead_letters), ((), ()))
        after = batch(queue, "c1", 10)
        self.assertNotEqual(after.lease, lease_id)
        self.assertEqual(keys(after), ["early", "k", "between"])

    # scenario: S054
    def test_a_release_with_only_an_acked_record_and_its_deferred_version_requeues_the_pocket(self) -> None:
        queue, _ = scenario_queue(lease_ttl=10)
        publish(queue, "pk", "k", 1, b"v")
        lease_id = batch(queue, "c1", 10).lease
        arrival = queue.snapshot().next_position
        publish(queue, "pk", "k", 2, b"v2")
        queue.ack("c1", lease_id, "k")
        self.assertEqual(
            held_as(record(queue, "pk", "k")),
            (RecordStatus.HELD, 1, b"v", Mark.ACKED, None, arrival),
            "acked, with v2 deferred: not one of the ending records",
        )
        self.assertEqual(queue.snapshot().ending_by_pocket["pk"], ())
        queue.release("c1", lease_id)
        (pocket,) = queue.snapshot().pockets_keyed("pk")
        self.assertEqual(pocket.status, PocketStatus.QUEUED)
        queued = record(queue, "pk", "k")
        self.assertEqual((queued.status, queued.rank, queued.position), (RecordStatus.QUEUED, 2, arrival))
        self.assertEqual(check(queue.snapshot()), ())

    # scenario: S071
    def test_a_key_acked_then_published_again_within_the_lease_waits_until_the_lease_ends(self) -> None:
        queue, _, lease_id = held_k()
        queue.ack("c1", lease_id, "k")
        arrival = queue.snapshot().next_position
        publish(queue, "pk", "k", 2, b"v2")
        kept = record(queue, "pk", "k")
        self.assertEqual(
            (held_as(kept), kept.rank),
            ((RecordStatus.HELD, 1, b"v1", Mark.ACKED, None, arrival), 2),
            "the record stays held and acked, with v2 deferred",
        )
        again = batch(queue, "c1", 10)
        self.assertEqual(again.lease, lease_id)
        self.assertEqual(again.records, (), "not pullable again in this lease")
        queue.release("c1", lease_id)
        queued = record(queue, "pk", "k")
        self.assertEqual((queued.status, queued.position), (RecordStatus.QUEUED, arrival))
        after = batch(queue, "c1", 10)
        self.assertNotEqual(after.lease, lease_id)
        self.assertEqual(keys(after), ["k"])
        self.assertEqual(after.records[0].rank, 2)

    # scenario: S072
    def test_a_key_nacked_then_published_again_within_the_lease_waits_until_the_lease_ends(self) -> None:
        queue, clock, lease_id = held_k()
        queue.nack("c1", lease_id, "k")
        arrival = queue.snapshot().next_position
        publish(queue, "pk", "k", 2, b"v2")
        kept = record(queue, "pk", "k")
        self.assertEqual((held_as(kept), kept.rank), ((RecordStatus.HELD, 1, b"v1", Mark.NACKED, T0, arrival), 2))
        self.assertEqual(batch(queue, "c1", 10).records, (), "not pullable again in this lease")
        self.assertEqual(queue.snapshot().dead_letters, ())
        clock.set(T0 + 5)
        queue.release("c1", lease_id)
        state = queue.snapshot()
        self.assertEqual(
            state.dead_letters,
            (DeadLetter("pk", "k", 1, b"v1", "c1", T0, T0 + 5 + DEAD_LETTER_TTL),),
            "the dead letter appears at the release",
        )
        self.assertEqual(state.superseded, (), "one publish ends in at most one shed queue")
        queued = record(queue, "pk", "k")
        self.assertEqual(
            (queued.status, queued.position, queued.rank),
            (RecordStatus.QUEUED, arrival, 2),
            "v2 is queued at its arrival position",
        )
        self.assertEqual(keys(batch(queue, "c1", 10)), ["k"])

    # impl-choice (spec note N29): the order of the entries one lease end
    # creates, smallest held position first, is the double's (N25); the
    # specification sends Freed for a set. The entries themselves, their
    # stamps and the kept positions are the specification's.
    # scenario: S091
    def test_the_entries_one_lease_end_creates_come_out_smallest_held_position_first(self) -> None:
        queue, clock = scenario_queue()
        # The key order is the reverse of the position order.
        for key in ("z", "y", "b", "a"):
            publish(queue, "pk", key, 1, key.encode())
        lease_id = batch(queue, "c1", 10).lease
        held_positions = {key: record(queue, "pk", key).position for key in ("b", "z")}
        clock.set(T0 + 10)
        for key in ("b", "z"):
            publish(queue, "pk", key, 2, b"newer")
        self.assertEqual(queue.snapshot().superseded, (), "nothing is shed while held")
        clock.set(T0 + 20)
        for key in ("a", "y"):
            queue.nack("c1", lease_id, key)
        clock.set(T0 + 50)
        queue.release("c1", lease_id)
        state = queue.snapshot()
        ended = T0 + 50
        self.assertEqual(
            state.superseded,
            (
                shed_at(ended, 1, b"z", ShedReason.SUPERSEDED, key="z"),
                shed_at(ended, 1, b"b", ShedReason.SUPERSEDED, key="b"),
            ),
        )
        self.assertEqual(
            state.dead_letters,
            (
                DeadLetter("pk", "y", 1, b"y", "c1", T0 + 20, ended + DEAD_LETTER_TTL),
                DeadLetter("pk", "a", 1, b"a", "c1", T0 + 20, ended + DEAD_LETTER_TTL),
            ),
        )
        # The Rust lists the pocket's records in its own order; the double's
        # snapshot order is not behaviour (N31), so the positions are compared by key.
        positions = {r.key: r.position for r in state.records_by_pocket["pk"]}
        self.assertEqual(positions, held_positions, "held positions kept")
        self.assertEqual(
            {(r.key, r.status, r.rank, r.payload) for r in state.records_by_pocket["pk"]},
            {("b", RecordStatus.QUEUED, 2, b"newer"), ("z", RecordStatus.QUEUED, 2, b"newer")},
        )


class PublishGuardsOnAHeldRecordTest(unittest.TestCase):
    """The specification's claim that the Publish guards are disjoint, over the
    held states PD04 reaches: held at rank 1 with nothing deferred, and held at
    rank 1 with rank 3 deferred, for each mark."""

    def holding(self, queue: Queue, rank: int) -> list[str]:
        view = View(queue.snapshot(), T0)
        trigger = Publish(PUBLISHER, "pk", "k", rank, b"v")
        return [row.name for row in PUBLISH.rows if row.guard(view, trigger)]

    def test_at_most_one_guard_holds_and_it_is_the_expected_rule(self) -> None:
        for mark in Mark:
            with self.subTest(mark=mark, deferred=False):
                queue, _, _, _ = held_and_marked(mark)
                self.assertEqual(self.holding(queue, 2), ["DeferBehindHeldVersion"])
                self.assertEqual(self.holding(queue, 1), [])
                self.assertEqual(self.holding(queue, 0), ["DropStalePublish"])
            with self.subTest(mark=mark, deferred=True):
                publish(queue, "pk", "k", 3, b"v3")
                self.assertEqual(self.holding(queue, 4), ["Supersede"])
                self.assertEqual(self.holding(queue, 3), [])
                self.assertEqual(self.holding(queue, 2), ["DropStalePublish"])
                self.assertEqual(self.holding(queue, 1), ["DropStalePublish"], "the held rank is below the stored rank")


class ArrivalPositionInvariantsAreReachedTest(unittest.TestCase):
    """The seven invariants on the arrival position became reachable in PD04.
    A false premise holds vacuously, so this shows each premise true in a
    state `Queue` committed, which is the state its checker passed: S091's
    state before the release, where b and z have a newer version deferred and
    y and a do not."""

    def test_each_premise_is_true_in_a_committed_state_the_checker_passed(self) -> None:
        queue, _ = scenario_queue()
        for key in ("z", "y", "b", "a"):
            publish(queue, "pk", key, 1, key.encode())
        batch(queue, "c1", 10)
        for key in ("b", "z"):
            publish(queue, "pk", key, 2, b"newer")
        state = queue.snapshot()
        held_records = state.held_by_pocket["pk"]
        deferred = [r for r in held_records if r.deferred_position is not None]
        premises = {
            "DeferredHasArrivalPosition": any(r.held_rank is not None and r.held_rank < r.rank for r in held_records),
            "OnlyDeferredHasArrivalPosition": any(r.held_rank == r.rank for r in held_records),
            "DeferredArrivedAfterHeld": bool(deferred),
            "HeldCopiesAgree": any(r.deferred_position is None for r in held_records),
            "ArrivalPositionUnoccupied": bool(deferred),
            "UniqueArrivalPosition": len(deferred) >= 2,
            "ArrivalPositionsBelowNext": bool(deferred),
        }
        self.assertEqual([name for name, true in premises.items() if not true], [])
        self.assertEqual(sorted(r.key for r in deferred), ["b", "z"])
        self.assertEqual(check(state), ())


class DeferralTouchesItsPocketTest(unittest.TestCase):
    """DeferBehindHeldVersion emits PocketTouched, as the specification ensures.
    A held record's pocket is leased, so through `Queue` the event changes
    nothing and no scenario can see it; this dispatches the registered Publish
    table on a committed state and reads what the row writes."""

    def test_the_deferral_allots_one_arrival_position_and_touches_the_pocket(self) -> None:
        queue, _, _ = held_k()
        state = queue.snapshot()
        before = record(queue, "pk", "k")
        outcome = RecordingOutcome()
        trigger = Publish(PUBLISHER, "pk", "k", 3, b"v3")
        fired = dispatch(PUBLISH, View(state, T0), outcome, trigger, registry=Registry())
        self.assertEqual(fired, "DeferBehindHeldVersion")
        deferred = replace(before, rank=3, payload=b"v3", deferred_position=1_001)
        self.assertEqual(outcome.writes, [("replace", deferred), ("emit", PocketTouched("pk"))])
        self.assertEqual(outcome.allotted, 1)
