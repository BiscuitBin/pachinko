"""The rule-table machinery.

The rules that share a trigger are the rows of one table for that trigger. A
row is `Rule(name, guard, effect)`, named exactly as the specification's
`rule`. A guard reads an immutable view and the trigger and answers whether the
rule's `requires` clauses hold; it allots nothing and changes nothing. An
effect reads the same view, for the rule's `let`s, and writes the rule's
`ensures` through the outcome the apply step gives it.

`dispatch` evaluates every guard of the table. Two that hold are a
specification finding, `GuardOverlap`. When none holds, the table's own answer
applies: a refusal, nothing, or a `Defect`.

The specification's tables are registered in `SPECIFICATION`. Only a
registered table records the row that fired, so a scratch table a test builds
never reaches the coverage test's record.
"""

from collections.abc import Callable
from dataclasses import dataclass
from enum import Enum
from typing import Any

from .refusals import Refusal

type Guard = Callable[[Any, Any], bool]
type Effect = Callable[[Any, Any, Any], None]
type RefusalFor = Callable[[Any], Refusal]


class NoGuardHolds(Enum):
    """What a table answers when no guard of it holds."""

    REFUSAL = "a refusal"
    NOTHING = "nothing happens"
    DEFECT = "a defect"


@dataclass(frozen=True)
class Rule:
    """One row of a trigger's table: the specification's rule of that name."""

    name: str
    guard: Guard
    effect: Effect


@dataclass(frozen=True)
class Table:
    """The rules that share one trigger, and the answer when no guard holds.

    `refusal` builds the refusal from the trigger, and a table takes one only
    when its answer is `NoGuardHolds.REFUSAL`.
    """

    trigger: str
    rows: tuple[Rule, ...]
    when_no_guard_holds: NoGuardHolds
    refusal: RefusalFor | None = None

    def __post_init__(self) -> None:
        refuses = self.when_no_guard_holds is NoGuardHolds.REFUSAL
        if refuses != (self.refusal is not None):
            raise ValueError(f"the {self.trigger} table takes a refusal exactly when it answers with one")
        names = [row.name for row in self.rows]
        if len(names) != len(set(names)):
            raise ValueError(f"the {self.trigger} table repeats a row name: {names}")


class GuardOverlap(Exception):
    """Two guards of one table held at once: a specification finding."""

    def __init__(self, trigger: str, names: tuple[str, ...]) -> None:
        super().__init__(f"{trigger}: the guards of {', '.join(names)} all hold")
        self.trigger = trigger
        self.names = names


class Defect(Exception):
    """No guard held on a table where some guard always must."""


class Registry:
    """The specification's tables, at most one per trigger and one row per rule.

    It also keeps the record of which of its rows fired.
    """

    def __init__(self) -> None:
        self._tables: dict[str, Table] = {}
        self._fired: set[str] = set()

    def register(self, table: Table) -> Table:
        if table.trigger in self._tables:
            raise ValueError(f"the {table.trigger} table is already registered")
        taken = self.rule_names() & {row.name for row in table.rows}
        if taken:
            raise ValueError(f"rules already registered under another trigger: {sorted(taken)}")
        self._tables[table.trigger] = table
        return table

    def holds(self, table: Table) -> bool:
        return self._tables.get(table.trigger) is table

    def tables(self) -> tuple[Table, ...]:
        return tuple(self._tables.values())

    def rule_names(self) -> frozenset[str]:
        return frozenset(row.name for table in self._tables.values() for row in table.rows)

    def fired(self) -> frozenset[str]:
        return frozenset(self._fired)

    def record_fired(self, table: Table, name: str) -> None:
        if self.holds(table):
            self._fired.add(name)


SPECIFICATION = Registry()


def dispatch(table: Table, view: Any, outcome: Any, trigger: Any, *, registry: Registry = SPECIFICATION) -> str | None:
    """Run the one row of `table` whose guard holds; return its name.

    Every guard is evaluated, whether or not an earlier one held. With none
    holding, a refusing table raises its refusal, a defect table raises
    `Defect`, and any other returns None, having changed nothing.
    """
    holding = tuple(row for row in table.rows if row.guard(view, trigger))
    if len(holding) > 1:
        raise GuardOverlap(table.trigger, tuple(row.name for row in holding))
    if not holding:
        match table.when_no_guard_holds:
            case NoGuardHolds.REFUSAL:
                assert table.refusal is not None
                raise table.refusal(trigger)
            case NoGuardHolds.DEFECT:
                raise Defect(f"no rule of the {table.trigger} table answers {trigger!r}")
            case NoGuardHolds.NOTHING:
                return None
    (row,) = holding
    row.effect(view, outcome, trigger)
    registry.record_fired(table, row.name)
    return row.name
