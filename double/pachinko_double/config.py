"""`Config`: the specification's five settings, with its defaults.

Durations are whole milliseconds (spec note N01). The specification says the
model is defined only where the five config invariants hold, so a `Config`
that breaks one is refused when it is made, with `ConfigRefused` naming each
invariant it breaks (spec note N09). The checker reads the same five
invariants again from every snapshot.
"""

from dataclasses import dataclass

from .model import HOUR, MINUTE, Duration
from .refusals import ConfigRefused


@dataclass(frozen=True)
class Config:
    lease_ttl: Duration = 5 * MINUTE
    max_consecutive_expiries: int = 3
    max_pull_batch: int = 100
    superseded_ttl: Duration = 12 * HOUR
    dead_letter_ttl: Duration = 12 * HOUR

    def __post_init__(self) -> None:
        holds = {
            "ExpiryLimitPositive": self.max_consecutive_expiries >= 1,
            "PullBatchPositive": self.max_pull_batch >= 1,
            "LeaseTtlNonNegative": self.lease_ttl >= 0,
            "SupersededTtlNonNegative": self.superseded_ttl >= 0,
            "DeadLetterTtlNonNegative": self.dead_letter_ttl >= 0,
        }
        broken = tuple(name for name, held in holds.items() if not held)
        if broken:
            raise ConfigRefused(broken)
