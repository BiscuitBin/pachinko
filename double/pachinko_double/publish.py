"""The specification's Publish section: the Publish and PocketTouched tables.

The rules that share the `Publish` trigger have disjoint guards. No rule
covers a rank equal to the stored rank, so that publish reaches the table's
answer when no guard holds, and is refused with `EqualRankRefused` (spec note
N11). The stored rank is `record.rank` whatever the record's state.

`ReviveIdlePocket` answers `PocketTouched` on an idle pocket, queueing it at
a fresh position at the back; PD05 adds `ReviveParkedPocket` beside it. A
touched pocket that is queued or leased is left as it is, which is the
table's answer when no guard holds. PD04 adds `DeferBehindHeldVersion` to the
Publish table.
"""

from dataclasses import dataclass, replace
from typing import TYPE_CHECKING

from .inspection import View
from .model import Pocket, PocketStatus, Record, RecordStatus, ShedReason, SupersededRecord
from .refusals import EqualRankRefused
from .rules import SPECIFICATION, NoGuardHolds, Rule, Table

if TYPE_CHECKING:
    from .queue import Outcome


@dataclass(frozen=True)
class Publish:
    """The trigger `Publish(publisher, pocket_key, record_key, rank, payload)`.

    `publisher` is the id of a publisher that has joined (spec note N10).
    """

    publisher: str
    pocket_key: str
    record_key: str
    rank: int
    payload: bytes


@dataclass(frozen=True)
class PocketTouched:
    """The event `PocketTouched(pocket_key)`."""

    pocket_key: str


def _pocket(view: View, pocket_key: str) -> Pocket | None:
    """`Pocket{key: pocket_key}`, or None when no such pocket exists."""
    found = view.state.pockets_keyed(pocket_key)
    return found[0] if found else None


# rule PublishIntoNewPocket ------------------------------------------------


def _publish_into_new_pocket_guard(view: View, trigger: Publish) -> bool:
    # requires: not exists Pocket{key: pocket_key}
    return _pocket(view, trigger.pocket_key) is None


def _publish_into_new_pocket(view: View, outcome: "Outcome", trigger: Publish) -> None:
    # let pocket_position = queue.next_position
    # let record_position = queue.next_position + 1
    # ensures: queue.next_position = record_position + 1
    pocket_position = outcome.allot_position()
    record_position = outcome.allot_position()
    # ensures: Pocket.created(key, status: queued, position: pocket_position, consecutive_expiries: 0)
    outcome.create(Pocket(trigger.pocket_key, PocketStatus.QUEUED, pocket_position, 0))
    # ensures: Record.created(pocket, key, rank, payload, position: record_position, status: queued)
    outcome.create(
        Record(
            pocket=trigger.pocket_key,
            key=trigger.record_key,
            rank=trigger.rank,
            payload=trigger.payload,
            position=record_position,
            status=RecordStatus.QUEUED,
        )
    )


# rule PublishNewKey -------------------------------------------------------


def _publish_new_key_guard(view: View, trigger: Publish) -> bool:
    # let pocket = Pocket{key: pocket_key}
    # requires: exists pocket
    # requires: not exists Record{pocket: pocket, key: record_key}
    return (
        _pocket(view, trigger.pocket_key) is not None
        and view.state.record(trigger.pocket_key, trigger.record_key) is None
    )


def _publish_new_key(view: View, outcome: "Outcome", trigger: Publish) -> None:
    # let position = queue.next_position
    # ensures: queue.next_position = position + 1
    position = outcome.allot_position()
    # ensures: Record.created(pocket, key, rank, payload, position, status: queued)
    outcome.create(
        Record(
            pocket=trigger.pocket_key,
            key=trigger.record_key,
            rank=trigger.rank,
            payload=trigger.payload,
            position=position,
            status=RecordStatus.QUEUED,
        )
    )
    # ensures: PocketTouched(pocket_key: pocket_key)
    outcome.emit(PocketTouched(trigger.pocket_key))


# rule Supersede -----------------------------------------------------------


def _supersede_guard(view: View, trigger: Publish) -> bool:
    # let pocket = Pocket{key: pocket_key}
    # requires: exists pocket
    if _pocket(view, trigger.pocket_key) is None:
        return False
    # let record = Record{pocket: pocket, key: record_key}
    # requires: exists record
    record = view.state.record(trigger.pocket_key, trigger.record_key)
    if record is None:
        return False
    # requires: rank > record.rank
    # requires: record.status = queued or (record.status = held and record.deferred_position != null)
    return trigger.rank > record.rank and (
        record.status is RecordStatus.QUEUED
        or (record.status is RecordStatus.HELD and record.deferred_position is not None)
    )


def _supersede(view: View, outcome: "Outcome", trigger: Publish) -> None:
    record = view.state.record(trigger.pocket_key, trigger.record_key)
    assert record is not None
    # let old_rank = record.rank
    # let old_payload = record.payload
    # ensures: SupersededRecord.created(pocket, key, rank: old_rank, payload: old_payload,
    #     reason: superseded, shed_at: now, expires_at: now + config.superseded_ttl)
    outcome.create(
        SupersededRecord(
            pocket=trigger.pocket_key,
            key=trigger.record_key,
            rank=record.rank,
            payload=record.payload,
            reason=ShedReason.SUPERSEDED,
            shed_at=view.now,
            expires_at=view.now + outcome.config.superseded_ttl,
        )
    )
    # ensures: record.rank = rank
    # ensures: record.payload = payload
    outcome.replace(replace(record, rank=trigger.rank, payload=trigger.payload))
    # ensures: PocketTouched(pocket_key: pocket_key)
    outcome.emit(PocketTouched(trigger.pocket_key))


# rule DropStalePublish ----------------------------------------------------


def _drop_stale_publish_guard(view: View, trigger: Publish) -> bool:
    # let pocket = Pocket{key: pocket_key}
    # requires: exists pocket
    if _pocket(view, trigger.pocket_key) is None:
        return False
    # let record = Record{pocket: pocket, key: record_key}
    # requires: exists record
    record = view.state.record(trigger.pocket_key, trigger.record_key)
    # requires: rank < record.rank
    return record is not None and trigger.rank < record.rank


def _drop_stale_publish(view: View, outcome: "Outcome", trigger: Publish) -> None:
    # ensures: SupersededRecord.created(pocket, key, rank, payload, reason: stale,
    #     shed_at: now, expires_at: now + config.superseded_ttl)
    # The record is left as it is, and no PocketTouched is emitted.
    outcome.create(
        SupersededRecord(
            pocket=trigger.pocket_key,
            key=trigger.record_key,
            rank=trigger.rank,
            payload=trigger.payload,
            reason=ShedReason.STALE,
            shed_at=view.now,
            expires_at=view.now + outcome.config.superseded_ttl,
        )
    )


# rule ReviveIdlePocket ----------------------------------------------------


def _revive_idle_pocket_guard(view: View, event: PocketTouched) -> bool:
    # let pocket = Pocket{key: pocket_key}
    # requires: exists pocket
    # requires: pocket.status = idle
    pocket = _pocket(view, event.pocket_key)
    return pocket is not None and pocket.status is PocketStatus.IDLE


def _revive_idle_pocket(view: View, outcome: "Outcome", event: PocketTouched) -> None:
    pocket = _pocket(view, event.pocket_key)
    assert pocket is not None
    # let position = queue.next_position
    # ensures: queue.next_position = position + 1
    position = outcome.allot_position()
    # ensures: pocket.status = queued
    # ensures: pocket.position = position
    # ensures: pocket.consecutive_expiries = 0
    outcome.replace(replace(pocket, status=PocketStatus.QUEUED, position=position, consecutive_expiries=0))


PUBLISH = SPECIFICATION.register(
    Table(
        "Publish",
        (
            Rule("PublishIntoNewPocket", _publish_into_new_pocket_guard, _publish_into_new_pocket),
            Rule("PublishNewKey", _publish_new_key_guard, _publish_new_key),
            Rule("Supersede", _supersede_guard, _supersede),
            Rule("DropStalePublish", _drop_stale_publish_guard, _drop_stale_publish),
        ),
        NoGuardHolds.REFUSAL,
        lambda view, trigger: EqualRankRefused(trigger.pocket_key, trigger.record_key, trigger.rank),
    )
)

POCKET_TOUCHED = SPECIFICATION.register(
    Table(
        "PocketTouched",
        (Rule("ReviveIdlePocket", _revive_idle_pocket_guard, _revive_idle_pocket),),
        NoGuardHolds.NOTHING,
    )
)
