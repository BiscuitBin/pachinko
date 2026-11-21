"""`Queue` and its one apply step.

The apply-step tests drive `Queue._apply` directly with scratch tables and
scratch event routes, so each property is shown on the smallest chain that
has it; the specification's own chains, a pull and its `Handed`, a release
and its `Freed`, are tested through `Queue` in their sections' modules. None
of these tables is registered, so none reaches the coverage test's record.
"""

import unittest
from dataclasses import dataclass

from pachinko_double.inspection import Snapshot, View
from pachinko_double.invariants import InvariantsBroken
from pachinko_double.model import DeadLetter, Pocket, PocketStatus, Record, RecordStatus, ShedReason, SupersededRecord
from pachinko_double.queue import Outcome
from pachinko_double.refusals import Refusal
from pachinko_double.rules import Defect, NoGuardHolds, Rule, Table

from .fixtures import DEFAULT_CONFIG, new_queue


@dataclass(frozen=True)
class Open:
    """A scratch trigger: create pocket `key` with one record."""

    key: str
    status: PocketStatus


@dataclass(frozen=True)
class Settle:
    """A scratch event: make pocket `key` queued."""

    key: str


@dataclass(frozen=True)
class Stray:
    """A scratch event no route answers."""


class ScratchRefused(Refusal):
    pass


def open_pocket(view: View, outcome: Outcome, trigger: Open) -> None:
    pocket_position = outcome.allot_position()
    record_position = outcome.allot_position()
    outcome.create(Pocket(trigger.key, trigger.status, pocket_position, 0))
    outcome.create(Record(trigger.key, "k", 1, b"v", record_position, RecordStatus.QUEUED))
    outcome.emit(Settle(trigger.key))


def settle(view: View, outcome: Outcome, event: Settle) -> None:
    (pocket,) = view.state.pockets_keyed(event.key)
    outcome.replace(Pocket(pocket.key, PocketStatus.QUEUED, pocket.position, pocket.consecutive_expiries))


def open_and_answer(view: View, outcome: Outcome, trigger: Open) -> None:
    open_pocket(view, outcome, trigger)
    outcome.answer(("opened", trigger.key))


OPEN = Table("Open", (Rule("Opens", lambda view, t: True, open_pocket),), NoGuardHolds.NOTHING)
ANSWER = Table("Open", (Rule("OpensAndAnswers", lambda view, t: True, open_and_answer),), NoGuardHolds.NOTHING)
SETTLE = Table(
    "Settle",
    (Rule("Settles", lambda view, e: bool(view.state.pockets_keyed(e.key)), settle),),
    NoGuardHolds.DEFECT,
)
LEAVE = Table("Settle", (), NoGuardHolds.NOTHING)
REFUSE = Table("Open", (), NoGuardHolds.REFUSAL, lambda view, trigger: ScratchRefused())


class NewQueueTest(unittest.TestCase):
    def test_a_new_queue_is_empty_and_holds_its_config(self) -> None:
        queue, clock, lease_ids = new_queue()
        self.assertEqual(queue.snapshot(), Snapshot(config=DEFAULT_CONFIG, next_position=0))
        self.assertEqual(clock.reads, 0)
        self.assertEqual(lease_ids.drawn, 0)


class ApplyStepTest(unittest.TestCase):
    def test_it_runs_the_trigger_then_its_events_then_commits(self) -> None:
        queue, _, _ = new_queue()
        queue._apply(OPEN, Open("p", PocketStatus.IDLE), event_tables={Settle: SETTLE})
        state = queue.snapshot()
        self.assertEqual(state.pockets, (Pocket("p", PocketStatus.QUEUED, 0, 0),))
        self.assertEqual(state.records, (Record("p", "k", 1, b"v", 1, RecordStatus.QUEUED),))

    def test_it_checks_after_the_whole_chain_not_part_way(self) -> None:
        # Open leaves an idle pocket with a record, which breaks
        # IdleHasNoRecords until Settle queues the pocket: a check between the
        # two would raise.
        queue, _, _ = new_queue()
        queue._apply(OPEN, Open("p", PocketStatus.IDLE), event_tables={Settle: SETTLE})
        self.assertEqual(queue.snapshot().pockets[0].status, PocketStatus.QUEUED)

    def test_a_broken_invariant_after_the_chain_raises_and_commits_nothing(self) -> None:
        queue, _, _ = new_queue()
        before = queue.snapshot()
        with self.assertRaises(InvariantsBroken) as raised:
            queue._apply(OPEN, Open("p", PocketStatus.IDLE), event_tables={Settle: LEAVE})
        self.assertEqual(raised.exception.names, frozenset({"IdleHasNoRecords"}))
        self.assertEqual(queue.snapshot(), before)

    def test_a_refusal_commits_nothing(self) -> None:
        queue, _, _ = new_queue()
        queue._apply(OPEN, Open("p", PocketStatus.QUEUED), event_tables={Settle: SETTLE})
        before = queue.snapshot()
        with self.assertRaises(ScratchRefused):
            queue._apply(REFUSE, Open("q", PocketStatus.QUEUED))
        self.assertEqual(queue.snapshot(), before)

    def test_a_defect_part_way_through_the_chain_commits_nothing(self) -> None:
        queue, _, _ = new_queue()
        before = queue.snapshot()
        bad = Table("Settle", (Rule("Never", lambda view, e: False, settle),), NoGuardHolds.DEFECT)
        with self.assertRaises(Defect):
            queue._apply(OPEN, Open("p", PocketStatus.QUEUED), event_tables={Settle: bad})
        self.assertEqual(queue.snapshot(), before)

    def test_an_event_no_table_answers_is_a_defect(self) -> None:
        def stray(view: View, outcome: Outcome, trigger: Open) -> None:
            outcome.emit(Stray())

        strays = Table("Open", (Rule("Strays", lambda view, t: True, stray),), NoGuardHolds.NOTHING)
        queue, _, _ = new_queue()
        with self.assertRaises(Defect):
            queue._apply(strays, Open("p", PocketStatus.QUEUED), event_tables={})

    def test_it_reads_the_clock_once_per_outcome(self) -> None:
        # The trigger's guard and effect, and the event's guard and effect, all
        # read now; the outcome reads the clock once, and all four see that value.
        seen: list[int] = []

        def sees(view: View) -> bool:
            seen.append(view.now)
            return True

        def open_seeing(view: View, outcome: Outcome, trigger: Open) -> None:
            seen.append(outcome.now)
            open_pocket(view, outcome, trigger)

        def settle_seeing(view: View, outcome: Outcome, event: Settle) -> None:
            seen.append(view.now)
            settle(view, outcome, event)

        opens = Table("Open", (Rule("OpensSeeing", lambda view, t: sees(view), open_seeing),), NoGuardHolds.NOTHING)
        settles = Table(
            "Settle", (Rule("SettlesSeeing", lambda view, e: sees(view), settle_seeing),), NoGuardHolds.DEFECT
        )
        queue, clock, _ = new_queue(now=40)
        queue._apply(opens, Open("p", PocketStatus.IDLE), event_tables={Settle: settles})
        self.assertEqual((clock.reads, seen), (1, [40, 40, 40, 40]))

    def test_an_outcome_whose_rules_never_read_now_reads_no_clock(self) -> None:
        # So does a refusal whose table never reads now, as a mark conflict's
        # does (S085).
        queue, clock, _ = new_queue()
        queue._apply(OPEN, Open("p", PocketStatus.IDLE), event_tables={Settle: SETTLE})
        with self.assertRaises(ScratchRefused):
            queue._apply(REFUSE, Open("q", PocketStatus.QUEUED))
        self.assertEqual(clock.reads, 0)

    def test_positions_come_from_next_position_in_allotment_order(self) -> None:
        queue, _, _ = new_queue()
        queue._apply(OPEN, Open("p", PocketStatus.QUEUED), event_tables={Settle: SETTLE})
        queue._apply(OPEN, Open("q", PocketStatus.QUEUED), event_tables={Settle: SETTLE})
        state = queue.snapshot()
        self.assertEqual([p.position for p in state.pockets], [0, 2])
        self.assertEqual([r.position for r in state.records], [1, 3])
        self.assertEqual(state.next_position, 4)

    def test_creating_an_entity_twice_is_a_defect(self) -> None:
        def twice(view: View, outcome: Outcome, trigger: Open) -> None:
            outcome.create(Pocket(trigger.key, PocketStatus.IDLE, outcome.allot_position(), 0))
            outcome.create(Pocket(trigger.key, PocketStatus.IDLE, outcome.allot_position(), 0))

        doubles = Table("Open", (Rule("Twice", lambda view, t: True, twice),), NoGuardHolds.NOTHING)
        queue, _, _ = new_queue()
        with self.assertRaises(Defect):
            queue._apply(doubles, Open("p", PocketStatus.IDLE))
        self.assertEqual(queue.snapshot().pockets, ())

    def test_removing_a_shed_entry_the_state_lacks_is_a_defect(self) -> None:
        absent = (
            SupersededRecord("p", "k", 1, b"v", ShedReason.SUPERSEDED, 0, 10),
            DeadLetter("p", "k", 1, b"v", "c", 0, 10),
        )
        for entry in absent:

            def remove(
                view: View, outcome: Outcome, trigger: Open, entry: SupersededRecord | DeadLetter = entry
            ) -> None:
                outcome.remove(entry)

            removes = Table("Open", (Rule("Removes", lambda view, t: True, remove),), NoGuardHolds.NOTHING)
            queue, _, _ = new_queue()
            before = queue.snapshot()
            with self.subTest(entry=type(entry).__name__), self.assertRaises(Defect):
                queue._apply(removes, Open("p", PocketStatus.IDLE))
            self.assertEqual(queue.snapshot(), before)

    def test_it_returns_the_answer_an_effect_gave_once_committed(self) -> None:
        queue, _, _ = new_queue()
        answer = queue._apply(ANSWER, Open("p", PocketStatus.IDLE), event_tables={Settle: SETTLE})
        self.assertEqual(answer, ("opened", "p"))
        self.assertEqual(queue.snapshot().pockets, (Pocket("p", PocketStatus.QUEUED, 0, 0),))

    def test_a_trigger_that_answers_nothing_returns_none(self) -> None:
        queue, _, _ = new_queue()
        self.assertIsNone(queue._apply(OPEN, Open("p", PocketStatus.IDLE), event_tables={Settle: SETTLE}))

    def test_an_answer_does_not_leave_an_outcome_that_breaks_an_invariant(self) -> None:
        queue, _, _ = new_queue()
        before = queue.snapshot()
        with self.assertRaises(InvariantsBroken):
            queue._apply(ANSWER, Open("p", PocketStatus.IDLE), event_tables={Settle: LEAVE})
        self.assertEqual(queue.snapshot(), before)

    def test_an_answer_does_not_leave_an_outcome_with_a_defect(self) -> None:
        queue, _, _ = new_queue()
        before = queue.snapshot()
        bad = Table("Settle", (Rule("Never", lambda view, e: False, settle),), NoGuardHolds.DEFECT)
        with self.assertRaises(Defect):
            queue._apply(ANSWER, Open("p", PocketStatus.QUEUED), event_tables={Settle: bad})
        self.assertEqual(queue.snapshot(), before)

    def test_an_outcome_answers_at_most_once(self) -> None:
        def twice(view: View, outcome: Outcome, trigger: Open) -> None:
            outcome.answer(1)
            outcome.answer(2)

        twice_table = Table("Open", (Rule("AnswersTwice", lambda view, t: True, twice),), NoGuardHolds.NOTHING)
        queue, _, _ = new_queue()
        before = queue.snapshot()
        with self.assertRaises(Defect):
            queue._apply(twice_table, Open("p", PocketStatus.QUEUED))
        self.assertEqual(queue.snapshot(), before)
