"""Every exception class the double defines, in two groups.

What the specification refuses: `Refusal` and its eleven subclasses. Each is
an outcome the specification defines, which a caller is meant to catch; a
refused action changes nothing.

What means the double itself is wrong: `Defect`, `GuardOverlap` and
`InvariantsBroken`. None is an outcome of the specification. A caller does
not catch them; each is a finding against the double, or against the
specification's claim that its guards are disjoint.

This module only re-exports: each name is defined in `refusals.py`,
`rules.py` or `invariants.py`.
"""

from .invariants import InvariantsBroken
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
    # What the specification refuses.
    "Refusal",
    "ConfigRefused",
    "PublisherAlreadyJoined",
    "ConsumerAlreadyJoined",
    "UnknownPublisher",
    "UnknownConsumer",
    "EqualRankRefused",
    "PullRefused",
    "LeaseFenced",
    "RecordNotHeld",
    "MarkConflict",
    "ReleaseRefused",
    # What means the double itself is wrong.
    "Defect",
    "GuardOverlap",
    "InvariantsBroken",
]
