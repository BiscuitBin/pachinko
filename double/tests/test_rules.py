"""Dispatch over rule tables, driven through scratch tables.

None of these tables is the specification's: they are built here, never
registered in `rules.SPECIFICATION`, so they cannot reach the coverage test's
registry or its fired-rule record.
"""

import unittest
from dataclasses import dataclass

from pachinko_double import rules
from pachinko_double.refusals import Refusal
from pachinko_double.rules import Defect, GuardOverlap, NoGuardHolds, Registry, Rule, Table


@dataclass(frozen=True)
class Poke:
    """A scratch trigger."""

    value: int


class ScratchRefused(Refusal):
    """The refusal the scratch tables below answer with."""


class Log:
    """Records which guards were asked and which effects ran."""

    def __init__(self) -> None:
        self.guards: list[str] = []
        self.effects: list[str] = []

    def row(self, name: str, holds: bool) -> Rule:
        def guard(view: object, trigger: Poke) -> bool:
            self.guards.append(name)
            return holds

        def effect(view: object, outcome: object, trigger: Poke) -> None:
            self.effects.append(name)

        return Rule(name, guard, effect)


def table(log: Log, holding: dict[str, bool], answer: NoGuardHolds) -> Table:
    refusal = (lambda trigger: ScratchRefused(f"scratch refused {trigger}")) if answer is NoGuardHolds.REFUSAL else None
    return Table(
        "Poke",
        tuple(log.row(name, holds) for name, holds in holding.items()),
        answer,
        refusal,
    )


class DispatchTest(unittest.TestCase):
    def test_two_guards_that_hold_raise_guard_overlap_naming_both(self) -> None:
        log = Log()
        scratch = table(log, {"First": True, "Second": False, "Third": True}, NoGuardHolds.NOTHING)
        with self.assertRaises(GuardOverlap) as raised:
            rules.dispatch(scratch, None, None, Poke(1))
        self.assertEqual(raised.exception.names, ("First", "Third"))
        self.assertIn("First", str(raised.exception))
        self.assertIn("Third", str(raised.exception))
        self.assertEqual(log.effects, [])

    def test_every_guard_is_evaluated_even_after_one_holds(self) -> None:
        log = Log()
        scratch = table(log, {"First": True, "Second": False, "Third": False}, NoGuardHolds.NOTHING)
        fired = rules.dispatch(scratch, None, None, Poke(1))
        self.assertEqual(fired, "First")
        self.assertEqual(log.guards, ["First", "Second", "Third"])
        self.assertEqual(log.effects, ["First"])

    def test_no_guard_holds_on_a_refusing_table_raises_its_refusal(self) -> None:
        log = Log()
        scratch = table(log, {"First": False, "Second": False}, NoGuardHolds.REFUSAL)
        with self.assertRaises(ScratchRefused):
            rules.dispatch(scratch, None, None, Poke(1))
        self.assertEqual(log.guards, ["First", "Second"])
        self.assertEqual(log.effects, [])

    def test_no_guard_holds_on_a_table_where_nothing_happens_returns_none(self) -> None:
        log = Log()
        scratch = table(log, {"First": False}, NoGuardHolds.NOTHING)
        self.assertIsNone(rules.dispatch(scratch, None, None, Poke(1)))
        self.assertEqual(log.effects, [])

    def test_no_guard_holds_on_a_defect_table_raises_a_defect(self) -> None:
        log = Log()
        scratch = table(log, {"First": False}, NoGuardHolds.DEFECT)
        with self.assertRaises(Defect) as raised:
            rules.dispatch(scratch, None, None, Poke(7))
        self.assertIn("Poke", str(raised.exception))
        self.assertEqual(log.effects, [])

    def test_a_defect_is_not_a_refusal(self) -> None:
        self.assertFalse(issubclass(Defect, Refusal))
        self.assertFalse(issubclass(GuardOverlap, Refusal))


class TableTest(unittest.TestCase):
    def test_a_refusing_table_needs_its_refusal(self) -> None:
        with self.assertRaises(ValueError):
            Table("Poke", (), NoGuardHolds.REFUSAL, None)

    def test_only_a_refusing_table_takes_a_refusal(self) -> None:
        for answer in (NoGuardHolds.NOTHING, NoGuardHolds.DEFECT):
            with self.subTest(answer=answer), self.assertRaises(ValueError):
                Table("Poke", (), answer, lambda trigger: ScratchRefused())

    def test_two_rows_of_one_table_cannot_share_a_name(self) -> None:
        log = Log()
        with self.assertRaises(ValueError):
            Table("Poke", (log.row("Same", True), log.row("Same", False)), NoGuardHolds.NOTHING)


class RegistryTest(unittest.TestCase):
    def test_a_scratch_table_leaves_the_specification_record_alone(self) -> None:
        before = rules.SPECIFICATION.fired()
        log = Log()
        rules.dispatch(table(log, {"ScratchRow": True}, NoGuardHolds.NOTHING), None, None, Poke(1))
        self.assertEqual(rules.SPECIFICATION.fired(), before)
        self.assertNotIn("ScratchRow", rules.SPECIFICATION.rule_names())

    def test_a_registered_table_records_the_row_that_fired(self) -> None:
        registry = Registry()
        log = Log()
        registered = registry.register(table(log, {"Fires": True, "Waits": False}, NoGuardHolds.NOTHING))
        self.assertEqual(registry.rule_names(), frozenset({"Fires", "Waits"}))
        self.assertEqual(registry.fired(), frozenset())
        rules.dispatch(registered, None, None, Poke(1), registry=registry)
        self.assertEqual(registry.fired(), frozenset({"Fires"}))

    def test_an_unregistered_table_records_nothing_in_a_registry(self) -> None:
        registry = Registry()
        log = Log()
        rules.dispatch(table(log, {"Fires": True}, NoGuardHolds.NOTHING), None, None, Poke(1), registry=registry)
        self.assertEqual(registry.fired(), frozenset())

    def test_a_trigger_has_one_table(self) -> None:
        registry = Registry()
        registry.register(table(Log(), {"A": True}, NoGuardHolds.NOTHING))
        with self.assertRaises(ValueError):
            registry.register(table(Log(), {"B": True}, NoGuardHolds.NOTHING))

    def test_a_rule_has_one_row_across_the_tables(self) -> None:
        registry = Registry()
        registry.register(Table("Poke", (Log().row("A", True),), NoGuardHolds.NOTHING))
        with self.assertRaises(ValueError):
            registry.register(Table("Prod", (Log().row("A", True),), NoGuardHolds.NOTHING))

