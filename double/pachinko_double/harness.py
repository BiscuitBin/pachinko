"""The test harness: what a test builds a queue with, and checks its state by.

`Clock` and `LeaseIds` are the two protocols `Queue`'s constructor takes, and
`FakeClock` and `SequentialLeaseIds` are the fakes that satisfy them.
`check(snapshot)` answers a `Violation` for every invariant a state breaks.
`ConfigValues` is the config a hand-built `Snapshot` carries.

This module only re-exports: each name is defined in `effects.py`,
`invariants.py` or `inspection.py`.
"""

from .effects import Clock, FakeClock, LeaseIds, SequentialLeaseIds
from .inspection import ConfigValues
from .invariants import Violation, check

__all__ = [
    "Clock",
    "LeaseIds",
    "FakeClock",
    "SequentialLeaseIds",
    "check",
    "Violation",
    "ConfigValues",
]
