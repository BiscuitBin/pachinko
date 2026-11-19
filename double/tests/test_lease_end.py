"""The lease-end module on hand-built states: the shared end of a lease, the
Freed table's rows, and the comparison that decides the pocket's fate.

Through `Queue` the lease's `ended_at` and the Freed rows' `now` are always
the same timestamp, since one outcome reads one clock value, so a row that
stamped its entry from `now` would pass every scenario. These tests dispatch
the Freed table on a view whose `now` differs from the event's `ended_at`.
Each runs against a private registry, so the fired record is untouched.
"""

import unittest

from pachinko_double.config import Config
from pachinko_double.inspection import Snapshot, View
from pachinko_double.lease_end import (
    FREED,
    Freed,
    a_record_outlasts_the_lease,
    end_lease,
    ending_records,
    no_record_outlasts_the_lease,
)
from pachinko_double.model import Consumer, DeadLetter, Lease, LeaseStatus, Mark, PocketStatus, Record, RecordStatus
from pachinko_double.refusals import ReleaseRefused
from pachinko_double.rules import Defect, Registry, dispatch
from pachinko_double.settle import RELEASE, Release

from .fixtures import T0, held, lease, pocket, queued, snapshot
from .test_pull import RecordingOutcome

DEAD_LETTER_TTL = 7_000
CONFIG = Config(dead_letter_ttl=DEAD_LETTER_TTL)
ENDED_AT = T0 + 5
LATER = T0 + 999
NOTHING_DEFERRED = ("RequeueUnsettledRecord", "ConsumeAckedRecord", "DeadLetterNackedRecord")

# Three held records whose tuple order, key order and position order all differ.
UNSETTLED = held("p", "k1", position=5, payload=b"one")
ACKED = held("p", "k2", position=2, payload=b"two", mark=Mark.ACKED)
NACKED = held("p", "k3", position=3, payload=b"three", mark=Mark.NACKED, nacked_at=T0 + 1)
ACTIVE = lease("lease-1", "p", "c", granted_at=T0, expires_at=T0 + 1_000)
ENDED = lease("lease-1", "p", "c", LeaseStatus.RELEASED, granted_at=T0, expires_at=T0 + 1_000, ended_at=ENDED_AT)


def state(*records: Record, the_lease: Lease = ACTIVE, status: PocketStatus = PocketStatus.LEASED) -> Snapshot:
    """Pocket p, in `status`, with `records` and one lease of consumer c."""
    return snapshot(consumers=(Consumer("c"),), pockets=(pocket("p", status),), records=records, leases=(the_lease,))


def freed(record_key: str, *records: Record) -> RecordingOutcome:
    """Dispatch one Freed, after the lease ended at ENDED_AT, on a view whose now is LATER."""
    outcome = RecordingOutcome(CONFIG)
    view = View(state(*records, the_lease=ENDED), LATER)
    dispatch(FREED, view, outcome, Freed("lease-1", record_key, ENDED_AT), registry=Registry())
    return outcome


class EndLeaseTest(unittest.TestCase):
    def test_it_ends_the_lease_then_frees_each_held_record_smallest_position_first(self) -> None:
        outcome = RecordingOutcome()
        view = View(state(UNSETTLED, ACKED, NACKED, queued("p", "q", position=7)), LATER)
        end_lease(view, outcome, ACTIVE, LeaseStatus.RELEASED, ENDED_AT)
        self.assertEqual(
            outcome.writes,
            [
                ("replace", ENDED),
                ("emit", Freed("lease-1", "k2", ENDED_AT)),
                ("emit", Freed("lease-1", "k3", ENDED_AT)),
                ("emit", Freed("lease-1", "k1", ENDED_AT)),
            ],
        )

    def test_a_lapse_ends_it_expired_through_the_same_steps(self) -> None:
        outcome = RecordingOutcome()
        end_lease(View(state(ACKED), LATER), outcome, ACTIVE, LeaseStatus.EXPIRED, ENDED_AT)
        (write, ended), emitted = outcome.writes[0], outcome.writes[1:]
        self.assertEqual((write, ended.status, ended.ended_at), ("replace", LeaseStatus.EXPIRED, ENDED_AT))
        self.assertEqual(emitted, [("emit", Freed("lease-1", "k2", ENDED_AT))])

    def test_a_lease_does_not_end_active(self) -> None:
        with self.assertRaises(ValueError):
            end_lease(View(state(ACKED), LATER), RecordingOutcome(), ACTIVE, LeaseStatus.ACTIVE, ENDED_AT)


class FreedTableTest(unittest.TestCase):
    def test_a_nacked_record_becomes_a_dead_letter_stamped_from_ended_at_not_now(self) -> None:
        outcome = freed("k3", UNSETTLED, ACKED, NACKED)
        letter = DeadLetter("p", "k3", 1, b"three", "c", T0 + 1, ENDED_AT + DEAD_LETTER_TTL)
        self.assertEqual(outcome.writes, [("create", letter), ("remove", NACKED)])

    def test_the_dead_letter_holds_the_held_version_not_the_records_newest(self) -> None:
        # held_rank and held_payload are what the consumer nacked; here they
        # differ from rank and payload only to show which the row reads.
        nacked = Record("p", "k3", 1, b"v", 3, RecordStatus.HELD, 1, b"held", Mark.NACKED, T0 + 1)
        (create, _) = freed("k3", nacked).writes
        self.assertEqual(create, ("create", DeadLetter("p", "k3", 1, b"held", "c", T0 + 1, ENDED_AT + DEAD_LETTER_TTL)))

    def test_an_acked_record_is_consumed(self) -> None:
        self.assertEqual(freed("k2", UNSETTLED, ACKED, NACKED).writes, [("remove", ACKED)])

    def test_an_unsettled_record_is_queued_at_its_own_position(self) -> None:
        (write,) = freed("k1", UNSETTLED, ACKED, NACKED).writes
        self.assertEqual(write, ("replace", queued("p", "k1", payload=b"one", position=5)))

    def test_a_freed_no_row_answers_is_a_defect(self) -> None:
        for key, records in (("q", (queued("p", "q"),)), ("absent", (ACKED,))):
            with self.subTest(key=key), self.assertRaises(Defect):
                freed(key, *records)

    def test_these_rows_leave_a_version_deferred_behind_the_held_one_to_the_rows_for_it(self) -> None:
        # Only the three rows with nothing deferred: the rows for a deferred
        # version join the same table in PD04 and answer this record then.
        rows = [row for row in FREED.rows if row.name in NOTHING_DEFERRED]
        self.assertEqual(sorted(row.name for row in rows), sorted(NOTHING_DEFERRED))
        for mark in Mark:
            nacked_at = T0 if mark is Mark.NACKED else None
            deferred = held(
                "p", "k", rank=2, held_rank=1, held_payload=b"old", mark=mark, nacked_at=nacked_at, deferred_position=9
            )
            view, event = View(state(deferred, the_lease=ENDED), LATER), Freed("lease-1", "k", ENDED_AT)
            with self.subTest(mark=mark):
                self.assertEqual([row.name for row in rows if row.guard(view, event)], [])


class PocketFateTest(unittest.TestCase):
    def test_ending_records_are_held_marked_and_with_nothing_deferred(self) -> None:
        deferred = held(
            "p", "d", rank=2, held_rank=1, held_payload=b"old", position=4, mark=Mark.ACKED, deferred_position=9
        )
        current = state(UNSETTLED, ACKED, NACKED, deferred)
        self.assertEqual(ending_records(current, "p"), (ACKED, NACKED))

    def test_no_record_outlasts_a_lease_whose_records_are_all_ending(self) -> None:
        current = state(ACKED, NACKED)
        self.assertEqual(
            (no_record_outlasts_the_lease(current, "p"), a_record_outlasts_the_lease(current, "p")), (True, False)
        )

    def test_an_unsettled_or_queued_record_outlasts_the_lease(self) -> None:
        for extra in (UNSETTLED, queued("p", "q", position=7)):
            current = state(ACKED, extra)
            with self.subTest(extra=extra.key):
                self.assertEqual(
                    (no_record_outlasts_the_lease(current, "p"), a_record_outlasts_the_lease(current, "p")),
                    (False, True),
                )


class ReleaseRefusedTest(unittest.TestCase):
    # impl-choice (spec note N26): this state breaks UnleasedHasNoActiveLease, so
    # Queue never reaches it; it shows the Release table's refusal past fencing.
    def test_no_release_rule_answers_an_active_lease_on_a_pocket_that_is_not_leased(self) -> None:
        view, trigger = View(state(status=PocketStatus.QUEUED), LATER), Release("c", "lease-1")
        self.assertEqual([row.name for row in RELEASE.rows if row.guard(view, trigger)], [])
        outcome = RecordingOutcome()
        with self.assertRaises(ReleaseRefused) as raised:
            dispatch(RELEASE, view, outcome, trigger, registry=Registry())
        self.assertEqual((raised.exception.consumer_id, raised.exception.lease_id), ("c", "lease-1"))
        self.assertEqual(outcome.writes, [])
