"""A Python double of libpachinko, derived from docs/specs/pachinko.allium alone.

`Queue` is the facade a reader drives; `Queue.snapshot()` is the one way to
read its state, and `check(snapshot)` names every invariant a state breaks.
"""

from . import consuming, identity, lease_end, publish, pull, rules, settle
from .config import Config
from .consuming import HeldRecordView, LeaseView
from .effects import Clock, FakeClock, LeaseIds, SequentialLeaseIds
from .inspection import ConfigValues, Snapshot
from .invariants import InvariantsBroken, Violation, check
from .model import (
    HOUR,
    MILLISECOND,
    MINUTE,
    SECOND,
    Consumer,
    DeadLetter,
    Lease,
    LeaseStatus,
    Mark,
    Pocket,
    PocketStatus,
    Publisher,
    Record,
    RecordStatus,
    ShedReason,
    SupersededRecord,
)
from .pull import Batch, NoPocketAvailable
from .queue import Queue
from .refusals import (
    ConfigRefused,
    ConsumerAlreadyJoined,
    EqualRankRefused,
    LeaseFenced,
    MarkConflict,
    PublisherAlreadyJoined,
    PullRefused,
    RecordNotHeld,
    Refusal,
    ReleaseRefused,
    UnknownConsumer,
    UnknownPublisher,
)
from .rules import Defect, GuardOverlap

__all__ = [
    "HOUR",
    "MILLISECOND",
    "MINUTE",
    "SECOND",
    "Batch",
    "Clock",
    "ConfigRefused",
    "Config",
    "ConfigValues",
    "Consumer",
    "ConsumerAlreadyJoined",
    "DeadLetter",
    "EqualRankRefused",
    "Defect",
    "FakeClock",
    "GuardOverlap",
    "HeldRecordView",
    "InvariantsBroken",
    "Lease",
    "LeaseFenced",
    "LeaseIds",
    "LeaseView",
    "LeaseStatus",
    "Mark",
    "MarkConflict",
    "NoPocketAvailable",
    "Pocket",
    "PocketStatus",
    "Publisher",
    "PublisherAlreadyJoined",
    "PullRefused",
    "RecordNotHeld",
    "Queue",
    "Record",
    "RecordStatus",
    "Refusal",
    "ReleaseRefused",
    "SequentialLeaseIds",
    "ShedReason",
    "Snapshot",
    "SupersededRecord",
    "UnknownConsumer",
    "UnknownPublisher",
    "Violation",
    "check",
    "consuming",
    "identity",
    "lease_end",
    "publish",
    "pull",
    "rules",
    "settle",
]
