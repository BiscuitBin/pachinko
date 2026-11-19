"""The effects the double reaches through seams: the clock and lease ids.

The double ships no real clock and no random source. A test, or a reader,
drives time by setting a `FakeClock`, and lease ids come from a
`SequentialLeaseIds`, so every run is the same run.
"""

from typing import Protocol

from .model import Duration, Timestamp


class Clock(Protocol):
    def now(self) -> Timestamp: ...


class LeaseIds(Protocol):
    def next_id(self) -> str: ...


class FakeClock:
    """A clock whose time a test sets, and which counts its reads."""

    def __init__(self, now: Timestamp = 0) -> None:
        self._now = now
        self.reads = 0

    def now(self) -> Timestamp:
        self.reads += 1
        return self._now

    def set(self, now: Timestamp) -> None:
        self._now = now

    def advance(self, by: Duration) -> None:
        if by < 0:
            raise ValueError(f"a clock advances by a non-negative duration, not {by}")
        self._now += by


class SequentialLeaseIds:
    """Lease ids `lease-1`, `lease-2`, and so on, counting what it has drawn."""

    def __init__(self, prefix: str = "lease-") -> None:
        self._prefix = prefix
        self.drawn = 0

    def next_id(self) -> str:
        self.drawn += 1
        return f"{self._prefix}{self.drawn}"
