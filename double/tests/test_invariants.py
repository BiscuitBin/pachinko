"""The invariant checker: a positive control for each of the 35 invariants.

Each control takes BASE, a hand-built snapshot that breaks no invariant, makes
one change, and asserts the exact set of invariants `check` names. Asserting
the exact set, not mere membership, means a control cannot pass because some
other invariant fired. Where the specification makes a second invariant fire
with the first, the control names both and says why.
"""

import unittest
from dataclasses import replace

from pachinko_double.inspection import Snapshot
from pachinko_double.invariants import REGISTRY, check
from pachinko_double.model import Consumer, LeaseStatus, Mark, PocketStatus, Publisher

from .fixtures import config, held, lease, letter, pocket, queued, shed, snapshot

UNSETTLED = held("p", "u", position=1)
ACKED = held("p", "a", position=2, mark=Mark.ACKED)
NACKED = held("p", "n", position=3, mark=Mark.NACKED, nacked_at=5)
DEFERRED = held(
    "p", "d", rank=2, payload=b"new", position=4, held_rank=1, held_payload=b"old", deferred_position=40
)
WAITING = queued("p", "w", position=5)
IN_QUEUED = queued("q", "x", position=11)
IN_PARKED = queued("k", "y", position=31)

LEASED = pocket("p", PocketStatus.LEASED, position=0)
QUEUED = pocket("q", PocketStatus.QUEUED, position=10)
IDLE = pocket("i", PocketStatus.IDLE, position=20)
PARKED = pocket("k", PocketStatus.PARKED, position=30, consecutive_expiries=3)

ACTIVE = lease("lease-2", "p", "c", granted_at=0, expires_at=300)
RELEASED = lease("lease-1", "q", "d", LeaseStatus.RELEASED, granted_at=0, expires_at=300, ended_at=10)

BASE = snapshot(
    next_position=100,
    publishers=(Publisher("pub"),),
    consumers=(Consumer("c"), Consumer("d")),
    pockets=(LEASED, QUEUED, IDLE, PARKED),
    records=(UNSETTLED, ACKED, NACKED, DEFERRED, WAITING, IN_QUEUED, IN_PARKED),
    leases=(RELEASED, ACTIVE),
    superseded=(shed("p", "u", shed_at=3, expires_at=8),),
    dead_letters=(letter("q", "z", nacked_by="d", nacked_at=4, expires_at=9),),
)


def swap[T](items: tuple[T, ...], old: T, new: T) -> tuple[T, ...]:
    assert old in items
    return tuple(new if item is old else item for item in items)


class CheckerTestCase(unittest.TestCase):
    def assert_names(self, state: Snapshot, expected: set[str]) -> None:
        named = {violation.invariant for violation in check(state)}
        self.assertEqual(named, expected)


class CleanStatesTest(CheckerTestCase):
    def test_the_base_snapshot_breaks_nothing(self) -> None:
        self.assertEqual(check(BASE), ())

    def test_an_empty_queue_breaks_nothing(self) -> None:
        self.assertEqual(check(snapshot(next_position=0)), ())

    def test_zero_ttls_and_a_limit_of_one_are_admitted(self) -> None:
        edge = config(lease_ttl=0, superseded_ttl=0, dead_letter_ttl=0, max_consecutive_expiries=1, max_pull_batch=1)
        self.assertEqual(check(snapshot(config=edge)), ())

    def test_the_registry_holds_35_invariants(self) -> None:
        self.assertEqual(len(REGISTRY), 35)

    def test_check_names_every_violation_with_a_witness_each(self) -> None:
        state = replace(BASE, records=swap(swap(BASE.records, UNSETTLED, replace(UNSETTLED, nacked_at=1)), ACKED, replace(ACKED, nacked_at=2)))
        violations = check(state)
        self.assertEqual([v.invariant for v in violations], ["OnlyNackedHasNackTime", "OnlyNackedHasNackTime"])
        self.assertIn("'u'", violations[0].witness)
        self.assertIn("'a'", violations[1].witness)


class PocketControlsTest(CheckerTestCase):
    # invariant-control: LeasedHasOneActiveLease
    def test_a_leased_pocket_with_no_active_lease(self) -> None:
        ended = replace(ACTIVE, status=LeaseStatus.RELEASED, ended_at=10)
        self.assert_names(replace(BASE, leases=swap(BASE.leases, ACTIVE, ended)), {"LeasedHasOneActiveLease"})

    # invariant-control: UnleasedHasNoActiveLease
    def test_a_queued_pocket_with_an_active_lease(self) -> None:
        extra = lease("lease-3", "q", "d")
        self.assert_names(replace(BASE, leases=(*BASE.leases, extra)), {"UnleasedHasNoActiveLease"})

    # invariant-control: IdleHasNoRecords
    def test_an_idle_pocket_with_a_record(self) -> None:
        self.assert_names(replace(BASE, records=(*BASE.records, queued("i", "v", position=21))), {"IdleHasNoRecords"})

    # invariant-control: ExpiriesWithinLimit
    def test_a_pocket_past_the_expiry_limit(self) -> None:
        over = replace(PARKED, consecutive_expiries=4)
        self.assert_names(replace(BASE, pockets=swap(BASE.pockets, PARKED, over)), {"ExpiriesWithinLimit"})


class RecordControlsTest(CheckerTestCase):
    def records(self, old: object, new: object) -> Snapshot:
        return replace(BASE, records=swap(BASE.records, old, new))

    # invariant-control: HeldVersionNotNewer
    def test_a_held_version_above_the_records_rank(self) -> None:
        self.assert_names(self.records(UNSETTLED, replace(UNSETTLED, held_rank=2)), {"HeldVersionNotNewer"})

    # invariant-control: NackedHasItsTime
    def test_a_nacked_record_with_no_nack_time(self) -> None:
        self.assert_names(self.records(NACKED, replace(NACKED, nacked_at=None)), {"NackedHasItsTime"})

    # invariant-control: OnlyNackedHasNackTime
    def test_an_unsettled_record_with_a_nack_time(self) -> None:
        self.assert_names(self.records(UNSETTLED, replace(UNSETTLED, nacked_at=5)), {"OnlyNackedHasNackTime"})

    # invariant-control: DeferredHasArrivalPosition
    def test_a_newer_version_with_no_arrival_position(self) -> None:
        newer = replace(UNSETTLED, rank=2, held_rank=1)
        self.assert_names(self.records(UNSETTLED, newer), {"DeferredHasArrivalPosition"})

    # invariant-control: OnlyDeferredHasArrivalPosition
    def test_an_arrival_position_with_nothing_deferred(self) -> None:
        self.assert_names(
            self.records(UNSETTLED, replace(UNSETTLED, deferred_position=41)), {"OnlyDeferredHasArrivalPosition"}
        )

    # invariant-control: DeferredArrivedAfterHeld
    def test_an_arrival_position_before_the_held_records(self) -> None:
        self.assert_names(self.records(DEFERRED, replace(DEFERRED, deferred_position=0)), {"DeferredArrivedAfterHeld"})

    # invariant-control: HeldCopiesAgree
    def test_held_copies_that_differ_with_nothing_deferred(self) -> None:
        self.assert_names(self.records(UNSETTLED, replace(UNSETTLED, held_payload=b"other")), {"HeldCopiesAgree"})


class TimestampControlsTest(CheckerTestCase):
    # invariant-control: ShedBeforeExpiry
    def test_a_superseded_record_shed_after_it_expires(self) -> None:
        self.assert_names(replace(BASE, superseded=(shed(shed_at=10, expires_at=5),)), {"ShedBeforeExpiry"})

    # invariant-control: NackedBeforeExpiry
    def test_a_dead_letter_nacked_after_it_expires(self) -> None:
        late = letter("q", "z", nacked_by="d", nacked_at=10, expires_at=5)
        self.assert_names(replace(BASE, dead_letters=(late,)), {"NackedBeforeExpiry"})

    # invariant-control: GrantedBeforeExpiry
    def test_a_lease_granted_after_it_expires(self) -> None:
        late = replace(ACTIVE, granted_at=10, expires_at=5)
        self.assert_names(replace(BASE, leases=swap(BASE.leases, ACTIVE, late)), {"GrantedBeforeExpiry"})

    # invariant-control: EndedAfterGrant
    def test_a_lease_that_ended_before_its_grant(self) -> None:
        early = replace(RELEASED, granted_at=10, ended_at=5)
        self.assert_names(replace(BASE, leases=swap(BASE.leases, RELEASED, early)), {"EndedAfterGrant"})


class ConfigControlsTest(CheckerTestCase):
    # invariant-control: ExpiryLimitPositive
    def test_an_expiry_limit_of_zero(self) -> None:
        self.assert_names(snapshot(config=config(max_consecutive_expiries=0)), {"ExpiryLimitPositive"})

    # invariant-control: PullBatchPositive
    def test_a_pull_batch_of_zero(self) -> None:
        self.assert_names(snapshot(config=config(max_pull_batch=0)), {"PullBatchPositive"})

    # invariant-control: LeaseTtlNonNegative
    def test_a_negative_lease_ttl(self) -> None:
        self.assert_names(snapshot(config=config(lease_ttl=-1)), {"LeaseTtlNonNegative"})

    # invariant-control: SupersededTtlNonNegative
    def test_a_negative_superseded_ttl(self) -> None:
        self.assert_names(snapshot(config=config(superseded_ttl=-1)), {"SupersededTtlNonNegative"})

    # invariant-control: DeadLetterTtlNonNegative
    def test_a_negative_dead_letter_ttl(self) -> None:
        self.assert_names(snapshot(config=config(dead_letter_ttl=-1)), {"DeadLetterTtlNonNegative"})


class UniquenessControlsTest(CheckerTestCase):
    # invariant-control: UniquePublisherId
    def test_two_publishers_with_one_id(self) -> None:
        # Value-equal twins: the check must count them as two publishers.
        self.assert_names(replace(BASE, publishers=(Publisher("pub"), Publisher("pub"))), {"UniquePublisherId"})

    # invariant-control: UniqueConsumerId
    def test_two_consumers_with_one_id(self) -> None:
        self.assert_names(replace(BASE, consumers=(*BASE.consumers, Consumer("c"))), {"UniqueConsumerId"})

    # invariant-control: OnePocketPerKey
    def test_two_pockets_with_one_key(self) -> None:
        twin = pocket("q", PocketStatus.QUEUED, position=12)
        self.assert_names(replace(BASE, pockets=(*BASE.pockets, twin)), {"OnePocketPerKey"})

    # invariant-control: OneRecordPerKeyPerPocket
    def test_two_records_with_one_key_in_one_pocket(self) -> None:
        twin = queued("q", "x", position=12)
        self.assert_names(replace(BASE, records=(*BASE.records, twin)), {"OneRecordPerKeyPerPocket"})

    # invariant-control: OneActiveLeasePerPocket
    def test_two_active_leases_on_one_pocket(self) -> None:
        # A leased pocket with two active leases also breaks
        # LeasedHasOneActiveLease, and an unleased one UnleasedHasNoActiveLease:
        # no state breaks OneActiveLeasePerPocket alone.
        second = lease("lease-3", "p", "d")
        self.assert_names(
            replace(BASE, leases=(*BASE.leases, second)), {"OneActiveLeasePerPocket", "LeasedHasOneActiveLease"}
        )

    # invariant-control: OneActiveLeasePerConsumer
    def test_a_consumer_with_two_active_leases(self) -> None:
        leased = replace(QUEUED, status=PocketStatus.LEASED)
        second = lease("lease-3", "q", "c")
        state = replace(BASE, pockets=swap(BASE.pockets, QUEUED, leased), leases=(*BASE.leases, second))
        self.assert_names(state, {"OneActiveLeasePerConsumer"})

    # invariant-control: OutstandingOnlyWhileLeased
    def test_a_held_record_in_a_queued_pocket(self) -> None:
        state = replace(BASE, records=swap(BASE.records, IN_QUEUED, held("q", "x", position=11)))
        self.assert_names(state, {"OutstandingOnlyWhileLeased"})

    # invariant-control: MarkedPocketHasNoExpiries
    def test_a_marked_record_in_a_pocket_with_expiries(self) -> None:
        counted = replace(LEASED, consecutive_expiries=1)
        self.assert_names(replace(BASE, pockets=swap(BASE.pockets, LEASED, counted)), {"MarkedPocketHasNoExpiries"})


class PositionControlsTest(CheckerTestCase):
    # invariant-control: UniquePocketPosition
    def test_two_pockets_at_one_position(self) -> None:
        moved = replace(QUEUED, position=LEASED.position)
        self.assert_names(replace(BASE, pockets=swap(BASE.pockets, QUEUED, moved)), {"UniquePocketPosition"})

    # invariant-control: UniqueRecordPosition
    def test_two_records_at_one_position(self) -> None:
        moved = replace(IN_QUEUED, position=UNSETTLED.position)
        self.assert_names(replace(BASE, records=swap(BASE.records, IN_QUEUED, moved)), {"UniqueRecordPosition"})

    # invariant-control: PocketPositionsBelowNext
    def test_a_pocket_at_next_position(self) -> None:
        moved = replace(PARKED, position=BASE.next_position)
        self.assert_names(replace(BASE, pockets=swap(BASE.pockets, PARKED, moved)), {"PocketPositionsBelowNext"})

    # invariant-control: RecordPositionsBelowNext
    def test_a_record_at_next_position(self) -> None:
        moved = replace(IN_PARKED, position=BASE.next_position)
        self.assert_names(replace(BASE, records=swap(BASE.records, IN_PARKED, moved)), {"RecordPositionsBelowNext"})

    # invariant-control: ArrivalPositionUnoccupied
    def test_a_record_standing_at_another_records_arrival_position(self) -> None:
        assert DEFERRED.deferred_position is not None
        moved = replace(IN_QUEUED, position=DEFERRED.deferred_position)
        self.assert_names(replace(BASE, records=swap(BASE.records, IN_QUEUED, moved)), {"ArrivalPositionUnoccupied"})

    def test_a_record_deferring_to_its_own_position_is_unoccupied_too(self) -> None:
        # The invariant has no `a != b`, so a record is paired with itself; its
        # own position also breaks DeferredArrivedAfterHeld.
        own = replace(DEFERRED, deferred_position=DEFERRED.position)
        state = replace(BASE, records=swap(BASE.records, DEFERRED, own))
        self.assert_names(state, {"ArrivalPositionUnoccupied", "DeferredArrivedAfterHeld"})

    # invariant-control: UniqueArrivalPosition
    def test_two_deferred_versions_with_one_arrival_position(self) -> None:
        also = replace(UNSETTLED, rank=2, held_rank=1, deferred_position=DEFERRED.deferred_position)
        self.assert_names(replace(BASE, records=swap(BASE.records, UNSETTLED, also)), {"UniqueArrivalPosition"})

    # invariant-control: ArrivalPositionsBelowNext
    def test_an_arrival_position_at_next_position(self) -> None:
        late = replace(DEFERRED, deferred_position=BASE.next_position)
        self.assert_names(replace(BASE, records=swap(BASE.records, DEFERRED, late)), {"ArrivalPositionsBelowNext"})
