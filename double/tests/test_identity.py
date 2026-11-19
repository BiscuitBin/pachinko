"""The Identity section: PublisherJoins and ConsumerJoins."""

import unittest

from pachinko_double.invariants import check
from pachinko_double.model import Consumer, Publisher
from pachinko_double.refusals import ConsumerAlreadyJoined, PublisherAlreadyJoined

from .fixtures import CONSUMERS, PUBLISHER, scenario_queue


class JoinTest(unittest.TestCase):
    # scenario: S009
    def test_a_publisher_joins_once(self) -> None:
        queue, _ = scenario_queue()
        queue.publisher_joins("q")
        before = queue.snapshot()
        with self.assertRaises(PublisherAlreadyJoined):
            queue.publisher_joins("q")
        after = queue.snapshot()
        self.assertEqual(after, before)
        self.assertEqual(after.publishers, (Publisher(PUBLISHER), Publisher("q")))
        self.assertEqual(check(after), ())

    # scenario: S010
    def test_a_consumer_joins_once(self) -> None:
        queue, _ = scenario_queue()
        queue.consumer_joins("c4")
        before = queue.snapshot()
        with self.assertRaises(ConsumerAlreadyJoined):
            queue.consumer_joins("c4")
        after = queue.snapshot()
        self.assertEqual(after, before)
        self.assertEqual(after.consumers, (*(Consumer(c) for c in CONSUMERS), Consumer("c4")))
        self.assertEqual(check(after), ())

    def test_a_publisher_and_a_consumer_may_share_an_id(self) -> None:
        # The two joins' guards each read their own entity only.
        queue, _ = scenario_queue()
        queue.consumer_joins(PUBLISHER)
        self.assertIn(Consumer(PUBLISHER), queue.snapshot().consumers)
