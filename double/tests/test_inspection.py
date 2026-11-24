"""The inspection seam: one complete, immutable snapshot, derived sets once."""

import dataclasses
import unittest

from pachinko_double.config import Config
from pachinko_double.inspection import ConfigValues, Snapshot
from pachinko_double.model import (
    Consumer,
    Lease,
    LeaseStatus,
    Mark,
    PocketStatus,
    Publisher,
    Record,
    RecordStatus,
)

from .fixtures import DEFAULT_CONFIG, held, lease, letter, pocket, queued, shed, snapshot


class SnapshotShapeTest(unittest.TestCase):
    def test_it_holds_the_whole_state(self) -> None:
        self.assertEqual(
            [field.name for field in dataclasses.fields(Snapshot) if field.init],
            [
                "config",
                "next_position",
                "publishers",
                "consumers",
                "pockets",
                "records",
                "leases",
                "superseded",
                "dead_letters",
            ],
        )

    def test_its_config_carries_the_five_values(self) -> None:
        self.assertEqual(
            dataclasses.asdict(ConfigValues.of(Config())),
            dataclasses.asdict(Config()),
        )

    def test_it_cannot_be_changed(self) -> None:
        view = snapshot(pockets=(pocket(),))
        with self.assertRaises(dataclasses.FrozenInstanceError):
            view.next_position = 7
        with self.assertRaises(dataclasses.FrozenInstanceError):
            view.pockets[0].status = PocketStatus.IDLE
        with self.assertRaises(TypeError):
            view.held_by_pocket["p"] = ()

    def test_its_collections_are_tuples_whatever_it_was_given(self) -> None:
        view = Snapshot(config=DEFAULT_CONFIG, next_position=1, publishers=[Publisher("a")])
        self.assertIsInstance(view.publishers, tuple)

    def test_two_snapshots_of_the_same_state_are_equal(self) -> None:
        first = snapshot(pockets=(pocket(),), records=(queued(),))
        second = snapshot(pockets=(pocket(),), records=(queued(),))
        self.assertEqual(first, second)
        self.assertNotEqual(first, snapshot(pockets=(pocket(),), records=(queued(rank=2),)))


class DerivedSetsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.unsettled = held(key="u", position=1)
        self.acked = held(key="a", position=2, mark=Mark.ACKED)
        self.deferred = held(key="d", position=3, mark=Mark.NACKED, nacked_at=4, rank=2, held_rank=1, deferred_position=9)
        self.waiting = queued(key="q", position=4)
        self.other = queued(pocket="o", key="u", position=5)
        self.active = lease(id="lease-2")
        self.ended = lease(id="lease-1", status=LeaseStatus.RELEASED, ended_at=10)
        self.view = snapshot(
            consumers=(Consumer("c"),),
            pockets=(pocket(status=PocketStatus.LEASED), pocket("o", position=6)),
            records=(self.unsettled, self.acked, self.deferred, self.waiting, self.other),
            leases=(self.ended, self.active),
        )

    def test_a_pockets_records_split_into_queued_and_held(self) -> None:
        self.assertEqual(self.view.records_by_pocket["p"], (self.unsettled, self.acked, self.deferred, self.waiting))
        self.assertEqual(self.view.queued_by_pocket["p"], (self.waiting,))
        self.assertEqual(self.view.held_by_pocket["p"], (self.unsettled, self.acked, self.deferred))
        self.assertEqual(self.view.records_by_pocket["o"], (self.other,))
        self.assertEqual(self.view.held_by_pocket["o"], ())

    def test_ending_records_are_held_marked_and_with_nothing_deferred(self) -> None:
        self.assertEqual(self.view.ending_by_pocket["p"], (self.acked,))

    def test_active_leases_are_found_by_pocket_and_by_consumer(self) -> None:
        self.assertEqual(self.view.leases_by_pocket["p"], (self.ended, self.active))
        self.assertEqual(self.view.active_leases_by_pocket["p"], (self.active,))
        self.assertEqual(self.view.active_leases_by_pocket["o"], ())
        self.assertEqual(self.view.active_leases_by_consumer["c"], (self.active,))

    def test_a_record_and_a_pocket_are_found_by_key(self) -> None:
        self.assertEqual(self.view.pockets_keyed("o"), (pocket("o", position=6),))
        self.assertEqual(self.view.record("p", "a"), self.acked)
        self.assertIsNone(self.view.record("o", "a"))
        self.assertEqual(self.view.pockets_keyed("missing"), ())


class ReferencesTest(unittest.TestCase):
    def test_every_reference_names_an_entity_the_snapshot_holds(self) -> None:
        cases = {
            "record": dict(records=(queued(pocket="nowhere"),)),
            "lease pocket": dict(consumers=(Consumer("c"),), leases=(lease(pocket="nowhere"),)),
            "lease consumer": dict(pockets=(pocket(),), leases=(lease(consumer="nobody"),)),
            "superseded": dict(superseded=(shed(pocket="nowhere"),)),
            "dead letter pocket": dict(consumers=(Consumer("c"),), dead_letters=(letter(pocket="nowhere"),)),
            "dead letter consumer": dict(pockets=(pocket(),), dead_letters=(letter(nacked_by="nobody"),)),
        }
        for case, fields in cases.items():
            with self.subTest(case=case), self.assertRaises(ValueError):
                snapshot(**fields)


class EntityShapeTest(unittest.TestCase):
    def test_a_held_record_has_its_held_version_and_mark(self) -> None:
        for missing in ("held_rank", "held_payload", "mark"):
            fields = dict(held_rank=1, held_payload=b"v", mark=Mark.UNSETTLED) | {missing: None}
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                Record("p", "k", 1, b"v", 1, RecordStatus.HELD, **fields)

    def test_a_queued_record_has_no_held_field(self) -> None:
        present = dict(held_rank=1, held_payload=b"v", mark=Mark.UNSETTLED, nacked_at=1, deferred_position=2)
        for name, value in present.items():
            with self.subTest(field=name), self.assertRaises(ValueError):
                Record("p", "k", 1, b"v", 1, RecordStatus.QUEUED, **{name: value})

    def test_a_lease_has_ended_at_exactly_once_it_has_ended(self) -> None:
        with self.assertRaises(ValueError):
            Lease("lease-1", "p", "c", LeaseStatus.ACTIVE, 0, 1, ended_at=1)
        for status in (LeaseStatus.RELEASED, LeaseStatus.EXPIRED):
            with self.subTest(status=status), self.assertRaises(ValueError):
                Lease("lease-1", "p", "c", status, 0, 1)
