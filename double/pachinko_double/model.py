"""The specification's entities and their enums, every field it declares.

Each entity is a frozen dataclass with the specification's field names. A
reference to another entity is that entity's key: `Record.pocket`,
`Lease.pocket`, `SupersededRecord.pocket` and `DeadLetter.pocket` hold a pocket
key, and `Lease.consumer` and `DeadLetter.nacked_by` a consumer id. `Queue`'s
one field, `next_position`, is held by the snapshot (see inspection.py).

A field the specification declares `when status = ...` is present exactly in
those states, so constructing an entity that breaks that raises `ValueError`.
That is the shape of the entity, not an invariant: the checker in
invariants.py reads only entities of the declared shape.

Timestamps and durations are whole milliseconds, an `int` (spec note N01);
payloads are `bytes` (N02); ranks and positions are unbounded `int` (N03).
"""

from dataclasses import dataclass
from enum import StrEnum

type Timestamp = int
type Duration = int

MILLISECOND: Duration = 1
SECOND: Duration = 1_000 * MILLISECOND
MINUTE: Duration = 60 * SECOND
HOUR: Duration = 60 * MINUTE


class PocketStatus(StrEnum):
    QUEUED = "queued"
    LEASED = "leased"
    IDLE = "idle"
    PARKED = "parked"


class RecordStatus(StrEnum):
    QUEUED = "queued"
    HELD = "held"


class Mark(StrEnum):
    UNSETTLED = "unsettled"
    ACKED = "acked"
    NACKED = "nacked"


class ShedReason(StrEnum):
    SUPERSEDED = "superseded"
    STALE = "stale"


class LeaseStatus(StrEnum):
    ACTIVE = "active"
    RELEASED = "released"
    EXPIRED = "expired"


@dataclass(frozen=True)
class Publisher:
    id: str


@dataclass(frozen=True)
class Consumer:
    id: str


@dataclass(frozen=True)
class Pocket:
    key: str
    status: PocketStatus
    position: int
    consecutive_expiries: int


@dataclass(frozen=True)
class Record:
    """One record key's slot in a pocket.

    `held_rank`, `held_payload` and `mark` are present exactly while the record
    is held; `nacked_at` and `deferred_position` may be present only then.
    """

    pocket: str
    key: str
    rank: int
    payload: bytes
    position: int
    status: RecordStatus
    held_rank: int | None = None
    held_payload: bytes | None = None
    mark: Mark | None = None
    nacked_at: Timestamp | None = None
    deferred_position: int | None = None

    def __post_init__(self) -> None:
        held_only = {
            "held_rank": self.held_rank,
            "held_payload": self.held_payload,
            "mark": self.mark,
            "nacked_at": self.nacked_at,
            "deferred_position": self.deferred_position,
        }
        if self.status is RecordStatus.HELD:
            missing = [name for name in ("held_rank", "held_payload", "mark") if held_only[name] is None]
            if missing:
                raise ValueError(f"a held record has {', '.join(missing)}: {self!r}")
        else:
            present = [name for name, value in held_only.items() if value is not None]
            if present:
                raise ValueError(f"a queued record has no {', '.join(present)}: {self!r}")


@dataclass(frozen=True)
class SupersededRecord:
    pocket: str
    key: str
    rank: int
    payload: bytes
    reason: ShedReason
    shed_at: Timestamp
    expires_at: Timestamp


@dataclass(frozen=True)
class DeadLetter:
    pocket: str
    key: str
    rank: int
    payload: bytes
    nacked_by: str
    nacked_at: Timestamp
    expires_at: Timestamp


@dataclass(frozen=True)
class Lease:
    """A consumer's claim on one pocket.

    `id` is the double's, not the specification's: an action names its lease by
    it (spec note N05). `ended_at` is present exactly once the lease has ended.
    """

    id: str
    pocket: str
    consumer: str
    status: LeaseStatus
    granted_at: Timestamp
    expires_at: Timestamp
    ended_at: Timestamp | None = None

    def __post_init__(self) -> None:
        ended = self.status is not LeaseStatus.ACTIVE
        if ended != (self.ended_at is not None):
            raise ValueError(f"a lease has ended_at exactly when it has ended: {self!r}")
