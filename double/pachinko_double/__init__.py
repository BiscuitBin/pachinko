"""A Python double of libpachinko, derived from docs/specs/pachinko.allium alone.

The package shows three interfaces, each in its own namespace:

- `pachinko_double`, this module, holds what a reader drives. `Queue` is the
  facade, built from a `Config`. A pull answers a `Batch` or
  `NoPocketAvailable`, and a tick a `TickReport`. `Queue.lease_view` answers
  a `LeaseView` of `HeldRecordView`s, or `None` when the consumer holds no
  active lease. `Queue.snapshot()` is the one way to read the queue's state,
  as a `Snapshot`.
- `pachinko_double.harness` holds what a test builds a queue with and checks
  it by: the `Clock` and `LeaseIds` protocols `Queue` takes, the fakes that
  satisfy them, and `check(snapshot)`, which names every invariant a state
  breaks.
- `pachinko_double.exceptions` holds every exception class the double
  defines, in two groups: what the specification refuses, and what means the
  double itself is wrong.

The entities, their enums and the durations are in `pachinko_double.model`.
Each rule module is imported by its own name, such as `pachinko_double.pull`.
"""

from .config import Config
from .consuming import HeldRecordView, LeaseView
from .inspection import Snapshot
from .pull import Batch, NoPocketAvailable
from .queue import Queue, TickReport

__all__ = [
    "Queue",
    "Config",
    "Batch",
    "NoPocketAvailable",
    "TickReport",
    "LeaseView",
    "HeldRecordView",
    "Snapshot",
]
