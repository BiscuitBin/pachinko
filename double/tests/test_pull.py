"""The Pull section: the claim of the front pocket, pulls within a lease, the
batch's size and order, `NoPocketAvailable`, the held set, and the refusal of
a consumer that never joined.

Each scenario test drives `Queue`'s trigger methods. The batch is the pull's
answer, so it is read from `Queue.pull`'s return value; the lease, its
`granted_at` and `expires_at`, and each handed record's held fields are read
through the snapshot. Neither the specification's `Batch` nor its `Renew`
carries an expiry (spec note N13), so a test reads every expiry from the
snapshot's lease.

The guard tests at the end ask the registered tables' guards and the batch's
`oldest` directly, on hand-built snapshots whose tuple order, key order and
position order disagree, which no state the scenarios reach can show.
"""

import inspect
import unittest

from pachinko_double.config import Config
from pachinko_double.inspection import ConfigValues, Snapshot, View
from pachinko_double.model import MINUTE, Consumer, Lease, LeaseStatus, Mark, PocketStatus, Record, RecordStatus
from pachinko_double.pull import HANDED, PULL, Batch, Handed, NoPocketAvailable, Pull, oldest
from pachinko_double.queue import Queue
from pachinko_double.refusals import PullRefused, UnknownConsumer
from pachinko_double.rules import Defect, Registry, dispatch

from .fixtures import (
    DEFAULT_CONFIG,
    LEASE_TTL,
    PUBLISHER,
    T0,
    batch,
    config,
    held,
    keys,
    lease,
    new_queue,
    pocket,
    queued,
    scenario_setting,
    snapshot,
)


def publish(queue: Queue, pocket_key: str, record_key: str, rank: int = 1, payload: bytes | None = None) -> None:
    queue.publish(PUBLISHER, pocket_key, record_key, rank, record_key.encode() if payload is None else payload)


def pocket_of(count: int, **config_fields: int):
    """The scenarios' setting with records r0, r1, ... published into pocket pk."""
    queue, clock, lease_ids = scenario_setting(**config_fields)
    for index in range(count):
        publish(queue, "pk", f"r{index}")
    return queue, clock, lease_ids


def record(state: Snapshot, pocket_key: str, record_key: str) -> Record:
    found = state.record(pocket_key, record_key)
    assert found is not None, f"no record {pocket_key!r}/{record_key!r}"
    return found


def active_lease(state: Snapshot, consumer: str) -> Lease | None:
    """The consumer's active lease, by OneActiveLeasePerConsumer at most one."""
    found = state.active_leases_by_consumer.get(consumer, ())
    assert len(found) <= 1, found
    return found[0] if found else None


def held_keys(state: Snapshot, pocket_key: str) -> list[str]:
    """The pocket's held set, smallest position first."""
    return [r.key for r in sorted(state.held_by_pocket[pocket_key], key=lambda r: r.position)]


class UnknownConsumerTest(unittest.TestCase):
    # impl-choice (spec note N18): the Pull trigger takes a Consumer entity, so the
    # double refuses an id that never joined before any rule is consulted.
    def test_a_pull_by_a_consumer_that_never_joined_is_refused_and_changes_nothing(self) -> None:
        queue, clock, lease_ids = pocket_of(1)
        before, reads = queue.snapshot(), clock.reads
        with self.assertRaises(UnknownConsumer) as raised:
            queue.pull("stranger", 10)
        self.assertEqual(raised.exception.consumer_id, "stranger")
        self.assertEqual(queue.snapshot(), before)
        self.assertEqual((clock.reads, lease_ids.drawn), (reads, 0))


class ClaimTest(unittest.TestCase):
    # scenario: S027
    def test_a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch(self) -> None:
        queue, clock, lease_ids = pocket_of(2)
        clock.advance(5)
        answer = batch(queue, "c1", 10)
        now = T0 + 5
        state = queue.snapshot()
        (lease,) = state.leases
        self.assertEqual(lease_ids.drawn, 1)
        self.assertEqual((answer.consumer, answer.lease, answer.pocket), ("c1", lease.id, "pk"))
        self.assertEqual(keys(answer), ["r0", "r1"])
        self.assertEqual([(r.rank, r.payload) for r in answer.records], [(1, b"r0"), (1, b"r1")])
        self.assertEqual((lease.consumer, lease.pocket), ("c1", "pk"))
        self.assertEqual((lease.status, lease.granted_at, lease.expires_at), (LeaseStatus.ACTIVE, now, now + LEASE_TTL))
        self.assertIsNone(lease.ended_at)
        (pocket,) = state.pockets_keyed("pk")
        self.assertEqual(pocket.status, PocketStatus.LEASED)
        self.assertEqual([p.key for p in state.pockets if p.status is PocketStatus.QUEUED], [])
        for key in ("r0", "r1"):
            held = record(state, "pk", key)
            self.assertEqual(held.status, RecordStatus.HELD)
            self.assertEqual((held.held_rank, held.held_payload), (1, key.encode()))
            self.assertEqual((held.mark, held.nacked_at, held.deferred_position), (Mark.UNSETTLED, None, None))
        self.assertEqual(held_keys(state, "pk"), ["r0", "r1"])
        self.assertEqual(active_lease(state, "c1"), lease)


class NoPocketTest(unittest.TestCase):
    # scenario: S033
    def test_a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error(self) -> None:
        queue, _, lease_ids = scenario_setting()
        before = queue.snapshot()
        self.assertEqual(queue.pull("c1", 10), NoPocketAvailable("c1"))
        self.assertEqual(queue.snapshot(), before)
        self.assertEqual(queue.snapshot().leases, ())
        publish(queue, "pk", "a")
        batch(queue, "c1", 10)
        before, drawn = queue.snapshot(), lease_ids.drawn
        self.assertEqual(queue.pull("c2", 10), NoPocketAvailable("c2"))
        self.assertEqual(queue.snapshot(), before)
        self.assertIsNone(active_lease(queue.snapshot(), "c2"))
        self.assertEqual(lease_ids.drawn, drawn)


class WithinLeaseTest(unittest.TestCase):
    # scenario: S036
    def test_a_pull_under_a_lease_extends_the_lease(self) -> None:
        queue, clock, lease_ids = pocket_of(3)
        self.assertIsNone(active_lease(queue.snapshot(), "c1"))
        self.assertIsNone(queue.lease_view("c1"))
        first = batch(queue, "c1", 1)
        state = queue.snapshot()
        before = active_lease(state, "c1")
        assert before is not None
        self.assertEqual((before.pocket, before.expires_at), ("pk", T0 + LEASE_TTL))
        self.assertEqual(held_keys(state, "pk"), ["r0"])
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual((view.pocket, view.expires_at), ("pk", T0 + LEASE_TTL))
        self.assertEqual([held.key for held in view.records], ["r0"])
        clock.advance(400)
        second = batch(queue, "c1", 1)
        extended = T0 + 400 + LEASE_TTL
        state = queue.snapshot()
        after = active_lease(state, "c1")
        assert after is not None
        view = queue.lease_view("c1")
        assert view is not None
        self.assertEqual((view.pocket, view.expires_at), ("pk", extended), "the consumer reads the extended expiry")
        self.assertEqual([held.key for held in view.records], ["r0", "r1"])
        self.assertEqual(second.lease, first.lease)
        self.assertEqual(after.id, first.lease)
        self.assertEqual((after.granted_at, after.expires_at), (T0, extended))
        self.assertEqual(keys(second), ["r1"])
        self.assertEqual(held_keys(state, "pk"), ["r0", "r1"])
        self.assertEqual(len(state.leases), 1)
        self.assertEqual(lease_ids.drawn, 1)


class EmptyPullWithinLeaseTest(unittest.TestCase):
    """PullWithinLease sets `lease.expires_at = now + config.lease_ttl` whatever
    the batch holds, so a pull under a lease that hands nothing still extends
    it. No catalog row covers the empty batch; S036's pull hands a record."""

    def assert_extended(self, queue: Queue, lease_id: str, now: int) -> None:
        state = queue.snapshot()
        (lease,) = state.leases
        self.assertEqual((lease.id, lease.consumer, lease.status), (lease_id, "c1", LeaseStatus.ACTIVE))
        self.assertEqual((lease.granted_at, lease.expires_at), (T0, now + LEASE_TTL))

    def test_a_pull_on_a_drained_pocket_extends_the_lease(self) -> None:
        queue, clock, lease_ids = pocket_of(2)
        claim = batch(queue, "c1", 10)
        self.assertEqual(keys(claim), ["r0", "r1"])
        records = queue.snapshot().records
        clock.advance(400)
        answer = batch(queue, "c1", 10)
        self.assertEqual((answer.lease, answer.pocket, answer.records), (claim.lease, "pk", ()))
        self.assert_extended(queue, claim.lease, T0 + 400)
        self.assertEqual(queue.snapshot().records, records)
        self.assertEqual(lease_ids.drawn, 1)

    def test_a_zero_or_negative_limit_pull_extends_the_lease_and_hands_nothing(self) -> None:
        queue, clock, lease_ids = pocket_of(3)
        claim = batch(queue, "c1", 1)
        self.assertEqual(keys(claim), ["r0"])
        records, now = queue.snapshot().records, T0
        for advance, limit in ((300, 0), (200, -1), (100, -(2**63))):
            with self.subTest(limit=limit):
                clock.advance(advance)
                now += advance
                answer = batch(queue, "c1", limit)
                self.assertEqual((answer.lease, answer.pocket, answer.records), (claim.lease, "pk", ()))
                self.assert_extended(queue, claim.lease, now)
                self.assertEqual(queue.snapshot().records, records)
        self.assertEqual(lease_ids.drawn, 1)
        self.assertEqual(keys(batch(queue, "c1", 10)), ["r1", "r2"])


class BatchSizeTest(unittest.TestCase):
    """OldestFirst: the batch is the oldest queued records, at most
    min(limit, max_pull_batch) of them, and none for a limit that is not
    positive."""

    # scenario: S001
    def test_the_default_lease_ttl_and_pull_batch_apply(self) -> None:
        queue, _, _ = new_queue(Config(), now=T0)
        self.assertEqual((queue.snapshot().config.lease_ttl, queue.snapshot().config.max_pull_batch), (5 * MINUTE, 100))
        queue.publisher_joins(PUBLISHER)
        queue.consumer_joins("c1")
        for index in range(101):
            publish(queue, "pk", f"r{index:03}")
        answer = batch(queue, "c1", 1_000)
        self.assertEqual(keys(answer), [f"r{index:03}" for index in range(100)])
        lease = active_lease(queue.snapshot(), "c1")
        assert lease is not None
        self.assertEqual(lease.expires_at, T0 + 5 * MINUTE)

    # scenario: S008
    def test_a_pull_batch_of_one_caps_every_pull_at_one_record(self) -> None:
        queue, _, _ = scenario_setting(max_pull_batch=1)
        for key in ("a", "b", "c"):
            publish(queue, "pk", key)
        self.assertEqual(keys(batch(queue, "c1", 100)), ["a"])
        self.assertEqual(keys(batch(queue, "c1", 2)), ["b"])
        self.assertEqual(keys(batch(queue, "c1", 1)), ["c"])

    # scenario: S028
    def test_a_positive_limit_returns_at_most_that_many_oldest_first_and_later_pulls_continue(self) -> None:
        queue, _, _ = pocket_of(5)
        self.assertEqual(keys(batch(queue, "c1", 2)), ["r0", "r1"])
        self.assertEqual(keys(batch(queue, "c1", 2)), ["r2", "r3"])
        self.assertEqual(keys(batch(queue, "c1", 2)), ["r4"])

    # scenario: S029
    def test_a_limit_above_max_pull_batch_is_capped(self) -> None:
        queue, _, _ = pocket_of(5, max_pull_batch=3)
        self.assertEqual(keys(batch(queue, "c1", 1_000)), ["r0", "r1", "r2"])

    # scenario: S030
    def test_a_limit_above_the_queued_count_returns_every_queued_record(self) -> None:
        queue, _, _ = pocket_of(3)
        self.assertEqual(keys(batch(queue, "c1", 50)), ["r0", "r1", "r2"])
        self.assertEqual(keys(batch(queue, "c1", 50)), [])

    # scenario: S031
    def test_a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch(self) -> None:
        queue, _, _ = pocket_of(2)
        answer = batch(queue, "c1", 0)
        state = queue.snapshot()
        (lease,) = state.leases
        self.assertEqual((answer.lease, answer.pocket, answer.records), (lease.id, "pk", ()))
        self.assertEqual(state.pockets_keyed("pk")[0].status, PocketStatus.LEASED)
        self.assertEqual(record(state, "pk", "r0").status, RecordStatus.QUEUED)
        self.assertEqual(held_keys(state, "pk"), [])
        self.assertEqual((lease.consumer, lease.status), ("c1", LeaseStatus.ACTIVE))

    # scenario: S032
    def test_a_negative_limit_behaves_like_zero(self) -> None:
        queue, _, _ = pocket_of(2)
        self.assertEqual(batch(queue, "c1", -1).records, ())
        self.assertEqual(queue.snapshot().pockets_keyed("pk")[0].status, PocketStatus.LEASED)
        self.assertEqual(batch(queue, "c1", -(2**63)).records, ())
        self.assertEqual(keys(batch(queue, "c1", 10)), ["r0", "r1"])


class PocketOrderTest(unittest.TestCase):
    # scenario: S038
    def test_pockets_are_claimed_in_the_order_they_entered(self) -> None:
        queue, _, _ = scenario_setting()
        for pocket in ("b", "c", "a"):
            publish(queue, pocket, "k")
        self.assertEqual(batch(queue, "c1", 1).pocket, "b")
        self.assertEqual(batch(queue, "c2", 1).pocket, "c")
        self.assertEqual(batch(queue, "c3", 1).pocket, "a")

    # scenario: S039
    def test_a_second_consumer_never_receives_records_from_a_leased_pocket(self) -> None:
        queue, _, _ = pocket_of(3)
        publish(queue, "other", "o")
        self.assertEqual(batch(queue, "c1", 1).pocket, "pk")
        second = batch(queue, "c2", 10)
        self.assertEqual((second.pocket, keys(second)), ("other", ["o"]))
        self.assertEqual(queue.pull("c3", 10), NoPocketAvailable("c3"))
        self.assertEqual(keys(batch(queue, "c2", 10)), [])
        self.assertEqual(keys(batch(queue, "c1", 10)), ["r1", "r2"])
        state = queue.snapshot()
        self.assertEqual(held_keys(state, "pk"), ["r0", "r1", "r2"])
        self.assertEqual(held_keys(state, "other"), ["o"])


class RecordingOutcome:
    """Stands in for the apply step's `Outcome` when a test runs one row
    on a hand-built snapshot: it records what the row writes and answers."""

    def __init__(self) -> None:
        self.writes: list[tuple[str, object]] = []
        self.answers: list[object] = []
        self.drawn = 0
        self.allotted = 0

    def draw_lease_id(self) -> str:
        self.drawn += 1
        return f"scratch-{self.drawn}"

    def allot_position(self) -> int:
        self.allotted += 1
        return 1_000 + self.allotted

    def create(self, entity: object) -> None:
        self.writes.append(("create", entity))

    def replace(self, entity: object) -> None:
        self.writes.append(("replace", entity))

    def remove(self, entity: object) -> None:
        self.writes.append(("remove", entity))

    def emit(self, event: object) -> None:
        self.writes.append(("emit", event))

    def answer(self, value: object) -> None:
        self.answers.append(value)


def run_pull(state: Snapshot, consumer: str, limit: int) -> RecordingOutcome:
    """Dispatch one Pull through the registered Pull table on a hand-built
    state, against a private registry so the fired record is untouched."""
    outcome = RecordingOutcome()
    dispatch(PULL, View(state, T0), outcome, Pull(consumer, limit), registry=Registry())
    return outcome


class OldestTest(unittest.TestCase):
    """`oldest` orders by position alone: here tuple order, key order and
    position order all disagree."""

    RECORDS = (queued(key="a", position=5), queued(key="b", position=2), queued(key="c", position=9), queued(key="d", position=1))

    def test_it_takes_the_n_smallest_positions_in_position_order(self) -> None:
        self.assertEqual([r.key for r in oldest(self.RECORDS, 2)], ["d", "b"])

    def test_it_takes_every_record_when_n_exceeds_their_count(self) -> None:
        self.assertEqual([r.key for r in oldest(self.RECORDS, 10)], ["d", "b", "a", "c"])

    def test_it_takes_none_when_n_is_not_positive(self) -> None:
        self.assertEqual((oldest(self.RECORDS, 0), oldest(self.RECORDS, -1)), ((), ()))


class PullRowsOnHandBuiltStatesTest(unittest.TestCase):
    """The Pull rows on states the scenarios cannot reach yet: pockets and
    records whose tuple order is not their position order."""

    def scrambled(self, config_values: ConfigValues = DEFAULT_CONFIG) -> Snapshot:
        # x and z are queued, x first in the tuple but z at the smaller
        # position; y is leased by c2 at the smallest position of all.
        return snapshot(
            config=config_values,
            consumers=(Consumer("c1"), Consumer("c2")),
            pockets=(
                pocket("x", position=7),
                pocket("y", PocketStatus.LEASED, position=1),
                pocket("z", position=3),
            ),
            records=(
                queued("x", "x1", position=8),
                held("y", "y1", position=2),
                queued("z", "z3", position=40),
                queued("z", "z1", position=10),
                queued("z", "z2", position=20),
            ),
            leases=(lease("lease-9", "y", "c2"),),
        )

    def test_the_claim_takes_the_queued_pocket_with_the_smallest_position(self) -> None:
        outcome = run_pull(self.scrambled(), "c1", 10)
        (answer,) = outcome.answers
        assert isinstance(answer, Batch)
        self.assertEqual((answer.pocket, answer.lease), ("z", "scratch-1"))
        self.assertEqual(keys(answer), ["z1", "z2", "z3"])
        handed = [event for kind, event in outcome.writes if kind == "emit"]
        self.assertEqual(handed, [Handed("z", "z1"), Handed("z", "z2"), Handed("z", "z3")])

    def test_the_claim_caps_the_batch_at_max_pull_batch(self) -> None:
        outcome = run_pull(self.scrambled(config(max_pull_batch=2)), "c1", 10)
        (answer,) = outcome.answers
        assert isinstance(answer, Batch)
        self.assertEqual(keys(answer), ["z1", "z2"])

    def test_the_claim_caps_the_batch_at_a_positive_limit(self) -> None:
        outcome = run_pull(self.scrambled(), "c1", 1)
        (answer,) = outcome.answers
        assert isinstance(answer, Batch)
        self.assertEqual(keys(answer), ["z1"])

    def test_a_pull_within_a_lease_hands_only_its_own_pockets_queued_records(self) -> None:
        state = snapshot(
            consumers=(Consumer("c2"),),
            pockets=(pocket("y", PocketStatus.LEASED, position=1),),
            records=(held("y", "y1", position=2), queued("y", "y3", position=6), queued("y", "y2", position=4)),
            leases=(lease("lease-9", "y", "c2"),),
        )
        outcome = run_pull(state, "c2", 10)
        (answer,) = outcome.answers
        assert isinstance(answer, Batch)
        self.assertEqual((answer.pocket, answer.lease, keys(answer)), ("y", "lease-9", ["y2", "y3"]))
        self.assertEqual(outcome.drawn, 0)

    # impl-choice (spec note N14): this state breaks UnleasedHasNoActiveLease, so
    # Queue never reaches it; it shows the Pull table's refusal is the only gap.
    def test_no_pull_rule_answers_an_active_lease_on_a_pocket_that_is_not_leased(self) -> None:
        state = snapshot(
            consumers=(Consumer("c1"),),
            pockets=(pocket("y", PocketStatus.QUEUED, position=1),),
            records=(queued("y", "y1", position=2),),
            leases=(lease("lease-9", "y", "c1"),),
        )
        view, trigger = View(state, T0), Pull("c1", 10)
        self.assertEqual([row.name for row in PULL.rows if row.guard(view, trigger)], [])
        outcome = RecordingOutcome()
        with self.assertRaises(PullRefused) as raised:
            dispatch(PULL, view, outcome, trigger, registry=Registry())
        self.assertEqual(raised.exception.consumer_id, "c1")
        self.assertEqual((outcome.writes, outcome.answers, outcome.drawn), ([], [], 0))


class PullGuardsTest(unittest.TestCase):
    """The Pull guards are disjoint over the states PD02 reaches, and one of
    them always holds there."""

    def holding(self, queue: Queue, consumer: str) -> list[str]:
        view, trigger = View(queue.snapshot(), T0), Pull(consumer, 10)
        return [row.name for row in PULL.rows if row.guard(view, trigger)]

    def test_exactly_one_guard_holds_and_it_is_the_expected_rule(self) -> None:
        queue, _, _ = scenario_setting()
        self.assertEqual(self.holding(queue, "c1"), ["PullFindsNoPocket"])
        publish(queue, "pk", "a")
        self.assertEqual(self.holding(queue, "c1"), ["PullClaimsFrontPocket"])
        batch(queue, "c1", 0)
        self.assertEqual(self.holding(queue, "c1"), ["PullWithinLease"])
        self.assertEqual(self.holding(queue, "c2"), ["PullFindsNoPocket"])
        publish(queue, "other", "o")
        self.assertEqual(self.holding(queue, "c2"), ["PullClaimsFrontPocket"])
        self.assertEqual(self.holding(queue, "c1"), ["PullWithinLease"])


class HoldRecordGuardTest(unittest.TestCase):
    """HoldRecord's guard needs a record of that key that is queued; with none,
    the Handed table answers a defect."""

    def test_it_holds_for_a_queued_record_only(self) -> None:
        state = snapshot(
            pockets=(pocket("p", PocketStatus.LEASED, position=0),),
            consumers=(Consumer("c"),),
            records=(queued("p", "q", position=1), held("p", "h", position=2)),
            leases=(lease("lease-1", "p", "c"),),
        )
        view = View(state, T0)
        guard = HANDED.rows[0].guard
        self.assertEqual(
            [guard(view, Handed("p", key)) for key in ("q", "h", "missing")],
            [True, False, False],
        )
        with self.assertRaises(Defect):
            dispatch(HANDED, view, RecordingOutcome(), Handed("p", "h"), registry=Registry())


class BatchShapeTest(unittest.TestCase):
    # The batch is computed before HoldRecord moves its records (spec note N13):
    # its records are the entities the pull read, as they stood before it.
    def test_the_batch_holds_the_records_as_they_stood_before_the_pull(self) -> None:
        queue, _, _ = pocket_of(3)
        before = queue.snapshot()
        answer = batch(queue, "c1", 2)
        self.assertEqual(answer.records, (record(before, "pk", "r0"), record(before, "pk", "r1")))
        after = queue.snapshot()
        self.assertEqual([record(after, "pk", key).held_payload for key in ("r0", "r1")], [b"r0", b"r1"])


class PullSignatureTest(unittest.TestCase):
    # Queue.pull's return annotation once named the pull module from inside the
    # class, where `pull` is the method, so evaluating it raised.
    def test_queue_pulls_annotations_name_its_two_answers(self) -> None:
        self.assertEqual(inspect.get_annotations(Queue.pull)["return"], Batch | NoPocketAvailable)
