"""The injected effects: the fake clock and the sequential lease ids."""

import unittest

from pachinko_double.effects import FakeClock, SequentialLeaseIds
from pachinko_double.model import HOUR, MILLISECOND, MINUTE, SECOND


class TimeUnitsTest(unittest.TestCase):
    def test_durations_are_whole_milliseconds(self) -> None:
        self.assertEqual(MILLISECOND, 1)
        self.assertEqual(SECOND, 1_000)
        self.assertEqual(MINUTE, 60_000)
        self.assertEqual(HOUR, 3_600_000)


class FakeClockTest(unittest.TestCase):
    def test_it_reads_the_time_a_test_set_and_counts_each_read(self) -> None:
        clock = FakeClock(5 * SECOND)
        self.assertEqual(clock.reads, 0)
        self.assertEqual(clock.now(), 5_000)
        clock.set(7 * MINUTE)
        self.assertEqual(clock.now(), 420_000)
        self.assertEqual(clock.reads, 2)

    def test_advance_moves_it_on_by_a_duration(self) -> None:
        clock = FakeClock()
        clock.advance(1 * HOUR)
        clock.advance(1 * MILLISECOND)
        self.assertEqual(clock.now(), 3_600_001)

    def test_it_starts_at_zero(self) -> None:
        self.assertEqual(FakeClock().now(), 0)

    def test_it_does_not_advance_backwards(self) -> None:
        with self.assertRaises(ValueError):
            FakeClock().advance(-1)

    def test_setting_it_does_not_count_as_a_read(self) -> None:
        clock = FakeClock()
        clock.set(3)
        clock.advance(4)
        self.assertEqual(clock.reads, 0)


class SequentialLeaseIdsTest(unittest.TestCase):
    def test_it_draws_the_same_ids_in_the_same_order_every_time(self) -> None:
        first, second = SequentialLeaseIds(), SequentialLeaseIds()
        drawn = [first.next_id() for _ in range(3)]
        self.assertEqual(drawn, ["lease-1", "lease-2", "lease-3"])
        self.assertEqual([second.next_id() for _ in range(3)], drawn)
        self.assertEqual(first.drawn, 3)
