"""`Config`: the five fields, their defaults, and the refusal of a config
that breaks a config invariant.

A refused config never becomes a `Config`, so no queue is made from it and
there is no snapshot to compare: each refusal test shows that the
construction raised, which invariants it named, and that the boundary value
is admitted and makes a queue (spec note N09).

Of the refusals PD01's step 5 lists as comparing the complete snapshot
before and after, these three are the ones that cannot. A queue's config is fixed when the queue is made, and nothing changes
it, so comparing an unrelated queue's snapshot around `Config(...)` would
prove nothing.
"""

import dataclasses
import unittest

from pachinko_double.config import Config
from pachinko_double.model import HOUR, MINUTE, SECOND
from pachinko_double.refusals import ConfigRefused

from .fixtures import new_queue


class ConfigRefusalTest(unittest.TestCase):
    def assert_refused(self, invariant: str, **fields: int) -> None:
        with self.assertRaises(ConfigRefused) as raised:
            Config(**fields)
        self.assertEqual(raised.exception.invariants, (invariant,))

    # scenario: S003
    def test_a_zero_expiry_limit_is_refused(self) -> None:
        self.assert_refused("ExpiryLimitPositive", max_consecutive_expiries=0)
        queue, _, _ = new_queue(Config(max_consecutive_expiries=1))
        self.assertEqual(queue.snapshot().config.max_consecutive_expiries, 1)

    # scenario: S004
    def test_a_zero_pull_batch_is_refused(self) -> None:
        self.assert_refused("PullBatchPositive", max_pull_batch=0)
        queue, _, _ = new_queue(Config(max_pull_batch=1))
        self.assertEqual(queue.snapshot().config.max_pull_batch, 1)

    # impl-choice (spec note N09): the Rust refuses a negative TTL at compile time,
    # since its Duration cannot be negative; the double refuses it at run time.
    # scenario: S005
    def test_negative_ttls_are_refused(self) -> None:
        cases = {
            "lease_ttl": "LeaseTtlNonNegative",
            "superseded_ttl": "SupersededTtlNonNegative",
            "dead_letter_ttl": "DeadLetterTtlNonNegative",
        }
        for field, invariant in cases.items():
            with self.subTest(field=field):
                self.assert_refused(invariant, **{field: -1 * SECOND})
                self.assertEqual(getattr(Config(**{field: 0}), field), 0)

    def test_a_config_that_breaks_several_invariants_names_each(self) -> None:
        with self.assertRaises(ConfigRefused) as raised:
            Config(max_pull_batch=0, lease_ttl=-1, max_consecutive_expiries=0)
        self.assertEqual(
            raised.exception.invariants, ("ExpiryLimitPositive", "PullBatchPositive", "LeaseTtlNonNegative")
        )


class ConfigTest(unittest.TestCase):
    def test_the_defaults_are_the_specifications(self) -> None:
        config = Config()
        self.assertEqual(config.lease_ttl, 5 * MINUTE)
        self.assertEqual(config.max_consecutive_expiries, 3)
        self.assertEqual(config.max_pull_batch, 100)
        self.assertEqual(config.superseded_ttl, 12 * HOUR)
        self.assertEqual(config.dead_letter_ttl, 12 * HOUR)

    def test_it_holds_exactly_the_five_fields(self) -> None:
        self.assertEqual(
            [field.name for field in dataclasses.fields(Config)],
            ["lease_ttl", "max_consecutive_expiries", "max_pull_batch", "superseded_ttl", "dead_letter_ttl"],
        )

    def test_it_is_immutable(self) -> None:
        with self.assertRaises(dataclasses.FrozenInstanceError):
            Config().lease_ttl = 0
