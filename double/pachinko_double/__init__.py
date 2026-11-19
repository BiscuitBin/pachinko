"""A Python double of libpachinko, derived from docs/specs/pachinko.allium alone.

`Queue` is the facade a reader drives; `Queue.snapshot()` is the one way to
read its state, and `check(snapshot)` names every invariant a state breaks.
"""

from . import identity, publish, pull, rules, settle
from .config import Config
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
    PublisherAlreadyJoined,
    PullRefused,
    Refusal,
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
    "InvariantsBroken",
    "Lease",
    "LeaseFenced",
    "LeaseIds",
    "LeaseStatus",
    "Mark",
    "NoPocketAvailable",
    "Pocket",
    "PocketStatus",
    "Publisher",
    "PublisherAlreadyJoined",
    "PullRefused",
    "Queue",
    "Record",
    "RecordStatus",
    "Refusal",
    "SequentialLeaseIds",
    "ShedReason",
    "Snapshot",
    "SupersededRecord",
    "UnknownConsumer",
    "UnknownPublisher",
    "Violation",
    "check",
    "identity",
    "publish",
    "pull",
    "rules",
    "settle",
]
