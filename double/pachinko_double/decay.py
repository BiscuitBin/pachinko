"""The specification's Decay section: the two decays.

Each rule waits on a clock trigger, `shed: SupersededRecord.expires_at <= now`
and `letter: DeadLetter.expires_at <= now`, so each table is keyed by its
trigger. As for the lapse, the double has no background time:
`Queue.tick()` asks `due_superseded` and `due_dead_letters` for every entry
whose `expires_at` has passed, read from the state the tick's lapses left,
and offers each one to its table, one outcome per entry. The trigger's
`<= now` is that filter.

The rules have no `requires`. A row's guard holds while the entry the
trigger binds is still in its queue, and when it is not, nothing happens.
Through `tick()` it always is, since each due entry is offered once. The row
removes the entry and answers it, so the tick reports what decayed.

An entry has no key of its own, so the trigger carries the entry itself.
Two equal entries, such as the same stale publish made twice at one instant,
are two decays, one entry each.
"""

from dataclasses import dataclass
from typing import TYPE_CHECKING

from .inspection import Snapshot, View
from .model import DeadLetter, SupersededRecord, Timestamp
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class SupersededRecordExpiresAt:
    """The clock trigger `shed: SupersededRecord.expires_at <= now`, for this entry."""

    shed: SupersededRecord


@dataclass(frozen=True)
class DeadLetterExpiresAt:
    """The clock trigger `letter: DeadLetter.expires_at <= now`, for this entry."""

    letter: DeadLetter


def due_superseded(state: Snapshot, now: Timestamp) -> tuple[SupersededRecord, ...]:
    """Every superseded record whose `expires_at <= now`, in the queue's order (N34)."""
    return tuple(shed for shed in state.superseded if shed.expires_at <= now)


def due_dead_letters(state: Snapshot, now: Timestamp) -> tuple[DeadLetter, ...]:
    """Every dead letter whose `expires_at <= now`, in the queue's order (N34)."""
    return tuple(letter for letter in state.dead_letters if letter.expires_at <= now)


# rule SupersededRecordDecays ----------------------------------------------


def _superseded_record_decays_guard(view: View, trigger: SupersededRecordExpiresAt) -> bool:
    # The trigger binds `shed`, which must still be in the queue.
    return trigger.shed in view.state.superseded


def _superseded_record_decays(view: View, outcome: "Outcome", trigger: SupersededRecordExpiresAt) -> None:
    # ensures: not exists shed
    outcome.remove(trigger.shed)
    outcome.answer(trigger.shed)


# rule DeadLetterDecays ----------------------------------------------------


def _dead_letter_decays_guard(view: View, trigger: DeadLetterExpiresAt) -> bool:
    # The trigger binds `letter`, which must still be in the queue.
    return trigger.letter in view.state.dead_letters


def _dead_letter_decays(view: View, outcome: "Outcome", trigger: DeadLetterExpiresAt) -> None:
    # ensures: not exists letter
    outcome.remove(trigger.letter)
    outcome.answer(trigger.letter)


SUPERSEDED_RECORD_EXPIRES_AT = SPECIFICATION.register(
    Table(
        "SupersededRecord.expires_at",
        (Rule("SupersededRecordDecays", _superseded_record_decays_guard, _superseded_record_decays),),
        NoGuardHolds.NOTHING,
    )
)

DEAD_LETTER_EXPIRES_AT = SPECIFICATION.register(
    Table(
        "DeadLetter.expires_at",
        (Rule("DeadLetterDecays", _dead_letter_decays_guard, _dead_letter_decays),),
        NoGuardHolds.NOTHING,
    )
)
