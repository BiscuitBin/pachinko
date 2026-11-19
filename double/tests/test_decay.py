"""Decay, and the tick that applies it: `SupersededRecordDecays` and
`DeadLetterDecays`, each at its entry's own `expires_at`, not before and
once; and what one `tick()` reads, runs and reports.

A superseded record decays at `shed_at + superseded_ttl`; a dead letter at
its lease's end plus `dead_letter_ttl`, whatever its `nacked_at`. The double
decays only in `Queue.tick()`, which reads the clock once and applies each
due lapse, then each due decay, as its own outcome (spec notes N33 to N35).
"""

import unittest
from unittest import mock

from pachinko_double.config import Config
from pachinko_double.effects import FakeClock
from pachinko_double.invariants import check
from pachinko_double.model import HOUR, MINUTE, DeadLetter, LeaseStatus, PocketStatus, ShedReason, SupersededRecord
from pachinko_double.queue import Queue

from .fixtures import (
    DEAD_LETTER_TTL,
    LEASE_TTL,
    PUBLISHER,
    SUPERSEDED_TTL,
    T0,
    batch,
    new_queue,
    scenario_queue,
)


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, payload)


class DefaultsTest(unittest.TestCase):
    # The Rust reads the three defaults off its config. The port reads them
    # off the snapshot, then shows each one at work through `Queue`: a
    # pocket parks on its third consecutive lapse, and a superseded record
    # and a dead letter decay twelve hours on, not a millisecond sooner.
    # scenario: S002
    def test_the_default_parking_threshold_and_decay_ttls_apply(self) -> None:
        queue, clock, _ = new_queue(Config(), now=T0)
        config = queue.snapshot().config
        self.assertEqual(
            (config.max_consecutive_expiries, config.superseded_ttl, config.dead_letter_ttl), (3, 12 * HOUR, 12 * HOUR)
        )
        queue.publisher_joins(PUBLISHER)
        queue.consumer_joins("c1")
        publish(queue, "d", "d1", 1, b"d1")
        lease_id = batch(queue, "c1", 10).lease
        queue.nack("c1", lease_id, "d1")
        publish(queue, "d", "d1", 0, b"stale")
        queue.release("c1", lease_id)
        (letter,), (shed,) = queue.snapshot().dead_letters, queue.snapshot().superseded
        self.assertEqual((letter.expires_at, shed.expires_at), (T0 + 12 * HOUR, T0 + 12 * HOUR))
        publish(queue, "a", "a1", 1, b"a1")
        for count in (1, 2, 3):
            self.assertEqual(batch(queue, "c1", 10).pocket, "a")
            clock.advance(5 * MINUTE)
            self.assertEqual(len(queue.tick().lapsed), 1)
            (pocket,) = queue.snapshot().pockets_keyed("a")
            self.assertEqual(pocket.consecutive_expiries, count)
        self.assertEqual(pocket.status, PocketStatus.PARKED, "parked on the third consecutive lapse")
        clock.set(T0 + 12 * HOUR - 1)
        report = queue.tick()
        self.assertEqual((report.decayed_superseded, report.decayed_dead_letters), ((), ()))
        clock.set(T0 + 12 * HOUR)
        report = queue.tick()
        self.assertEqual((report.decayed_superseded, report.decayed_dead_letters), ((shed,), (letter,)))


class DecayTest(unittest.TestCase):
    # scenario: S007
    def test_zero_superseded_and_dead_letter_ttls_decay_at_once(self) -> None:
        queue, _ = scenario_queue(superseded_ttl=0, dead_letter_ttl=0)
        publish(queue, "a", "a1", 1, b"old")
        publish(queue, "a", "a2", 1, b"a2")
        publish(queue, "a", "a1", 2, b"new")
        lease_id = batch(queue, "c1", 10).lease
        queue.nack("c1", lease_id, "a2")
        self.assertEqual(queue.snapshot().dead_letters, (), "not before the end")
        queue.release("c1", lease_id)
        state = queue.snapshot()
        self.assertEqual((state.superseded[0].expires_at, state.dead_letters[0].expires_at), (T0, T0))
        report = queue.tick()
        self.assertEqual((len(report.decayed_superseded), len(report.decayed_dead_letters)), (1, 1))
        decayed = (report.decayed_superseded, report.decayed_dead_letters)
        self.assertEqual(decayed, (state.superseded, state.dead_letters))
        state = queue.snapshot()
        self.assertEqual((state.superseded, state.dead_letters), ((), ()))

    # scenario: S073
    def test_superseded_entries_decay_at_shed_at_plus_superseded_ttl(self) -> None:
        queue, clock = scenario_queue()
        publish(queue, "a", "a1", 2, b"kept")
        clock.advance(3)
        publish(queue, "a", "a1", 1, b"stale")
        clock.advance(1)
        publish(queue, "a", "a1", 3, b"newer")
        shed = queue.snapshot().superseded
        self.assertEqual((shed[0].reason, shed[0].shed_at), (ShedReason.STALE, T0 + 3))
        clock.set(T0 + 3 + SUPERSEDED_TTL - 1)
        self.assertEqual(queue.tick().decayed_superseded, ())
        self.assertEqual(len(queue.snapshot().superseded), 2)
        clock.set(T0 + 3 + SUPERSEDED_TTL)
        self.assertEqual(queue.tick().decayed_superseded, (shed[0],))
        (left,) = queue.snapshot().superseded
        self.assertEqual(left.reason, ShedReason.SUPERSEDED)
        clock.set(T0 + 4 + SUPERSEDED_TTL)
        self.assertEqual(queue.tick().decayed_superseded, (left,))
        self.assertEqual(queue.tick().decayed_superseded, (), "a decay fires once")
        self.assertEqual(queue.snapshot().superseded, ())

    # scenario: S074
    def test_dead_letters_decay_at_the_lease_end_plus_dead_letter_ttl(self) -> None:
        queue, clock = scenario_queue()
        publish(queue, "a", "a1", 1, b"a1")
        publish(queue, "a", "a2", 1, b"a2")
        lease_id = batch(queue, "c1", 10).lease
        clock.set(T0 + 2)
        queue.nack("c1", lease_id, "a1")
        clock.set(T0 + 3)
        queue.nack("c1", lease_id, "a2")
        self.assertEqual(queue.snapshot().dead_letters, (), "not before the end")
        clock.set(T0 + 10)
        queue.release("c1", lease_id)
        expires_at = T0 + 10 + DEAD_LETTER_TTL
        letters = (
            DeadLetter("a", "a1", 1, b"a1", "c1", T0 + 2, expires_at),
            DeadLetter("a", "a2", 1, b"a2", "c1", T0 + 3, expires_at),
        )
        self.assertEqual(
            queue.snapshot().dead_letters, letters, "each keeps its nack's time and expires from the lease's end"
        )
        clock.set(T0 + 3 + DEAD_LETTER_TTL)
        self.assertEqual(queue.tick().decayed_dead_letters, (), "not at nacked_at plus the TTL")
        clock.set(T0 + 10 + DEAD_LETTER_TTL - 1)
        self.assertEqual(queue.tick().decayed_dead_letters, ())
        clock.set(T0 + 10 + DEAD_LETTER_TTL)
        report = queue.tick()
        self.assertEqual((report.decayed_dead_letters, report.decayed_superseded), (letters, ()))
        self.assertEqual(queue.tick().decayed_dead_letters, (), "a decay fires once")
        self.assertEqual(queue.snapshot().dead_letters, ())

    def test_equal_entries_each_decay(self) -> None:
        # Two stale publishes of one version at one instant make two equal
        # entries. Each is its own decay.
        queue, clock = scenario_queue()
        publish(queue, "a", "a1", 2, b"kept")
        publish(queue, "a", "a1", 1, b"stale")
        publish(queue, "a", "a1", 1, b"stale")
        first, second = queue.snapshot().superseded
        self.assertEqual(first, second)
        clock.set(T0 + SUPERSEDED_TTL)
        self.assertEqual(queue.tick().decayed_superseded, (first, second))
        self.assertEqual(queue.snapshot().superseded, ())


def two_due_leases_and_a_due_entry() -> tuple[Queue, FakeClock]:
    """Pockets a and b leased by c1 and c2 at T0, a superseded entry shed at
    T0, and the clock at T0 + LEASE_TTL + SUPERSEDED_TTL: everything is due."""
    queue, clock = scenario_queue()
    publish(queue, "a", "a1", 1, b"a1")
    publish(queue, "b", "b1", 1, b"b1")
    publish(queue, "s", "s1", 2, b"kept")
    publish(queue, "s", "s1", 1, b"stale")
    batch(queue, "c1", 10)
    batch(queue, "c2", 10)
    clock.set(T0 + LEASE_TTL + SUPERSEDED_TTL)
    return queue, clock


class TickTest(unittest.TestCase):
    """What one tick reads, runs and reports (spec notes N33 to N35)."""

    def test_a_tick_reads_the_clock_once_and_every_outcome_shares_its_now(self) -> None:
        queue, clock = two_due_leases_and_a_due_entry()
        reads = clock.reads
        report = queue.tick()
        self.assertEqual(clock.reads, reads + 1)
        now = T0 + LEASE_TTL + SUPERSEDED_TTL
        self.assertEqual([(lease.status, lease.ended_at) for lease in report.lapsed], [(LeaseStatus.EXPIRED, now)] * 2)
        self.assertEqual(len(report.decayed_superseded), 1)
        reads = clock.reads
        self.assertEqual(queue.tick(), type(report)(), "a tick with nothing due reports nothing")
        self.assertEqual(clock.reads, reads + 1)

    def test_one_tick_lapses_in_expires_at_order_then_grant_order(self) -> None:
        queue, clock = scenario_queue()
        for pocket in ("a", "b", "c"):
            publish(queue, pocket, "k", 1, b"k")
        first = batch(queue, "c1", 10).lease
        clock.advance(1)
        second = batch(queue, "c2", 10).lease
        third = batch(queue, "c3", 10).lease
        clock.advance(1)
        queue.renew("c1", first)
        clock.set(T0 + 10 * LEASE_TTL)
        self.assertEqual([lease.id for lease in queue.tick().lapsed], [second, third, first])

    def test_the_checker_runs_after_each_lapse_and_each_decay(self) -> None:
        queue, _ = two_due_leases_and_a_due_entry()
        with mock.patch("pachinko_double.queue.check", wraps=check) as checker:
            report = queue.tick()
        self.assertEqual((len(report.lapsed), len(report.decayed_superseded)), (2, 1))
        self.assertEqual(checker.call_count, 3, "once per lapse's outcome and once per decay")

    def test_with_zero_ttls_a_lapses_entries_decay_in_the_tick_that_made_them(self) -> None:
        # The lapses come first, and the decays due are read from the state
        # they left, at the same `now`: after a tick nothing is due (N35).
        queue, clock = scenario_queue(superseded_ttl=0, dead_letter_ttl=0)
        publish(queue, "a", "a1", 1, b"a1")
        publish(queue, "a", "a2", 1, b"a2")
        lease_id = batch(queue, "c1", 10).lease
        publish(queue, "a", "a1", 2, b"newer")
        queue.nack("c1", lease_id, "a2")
        self.assertEqual((queue.snapshot().superseded, queue.snapshot().dead_letters), ((), ()))
        clock.advance(LEASE_TTL)
        report = queue.tick()
        self.assertEqual([lease.id for lease in report.lapsed], [lease_id], "the lease lapsed")
        now = T0 + LEASE_TTL
        self.assertEqual(
            (report.decayed_superseded, report.decayed_dead_letters),
            (
                (SupersededRecord("a", "a1", 1, b"a1", ShedReason.SUPERSEDED, now, now),),
                (DeadLetter("a", "a2", 1, b"a2", "c1", T0, now),),
            ),
            "the shed held version and the dead letter decay in the lapse's tick",
        )
        state = queue.snapshot()
        self.assertEqual((state.superseded, state.dead_letters), ((), ()))
        record = state.record("a", "a1")
        assert record is not None
        self.assertEqual(record.rank, 2)
        self.assertEqual(queue.tick().decayed_superseded, (), "a decay fires once")
