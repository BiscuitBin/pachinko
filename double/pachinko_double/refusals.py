"""The refusals: one exception class per kind, under one base class.

A refusal is the answer to a trigger no rule accepts. It changes nothing: the
apply step raises it before it commits any part of the outcome. A refusal is
never a defect. Defects, which say the double or the specification is wrong,
are `rules.GuardOverlap`, `rules.Defect` and `invariants.InvariantsBroken`.
"""


class Refusal(Exception):
    """A trigger the specification's rules do not accept; nothing changed."""


class ConfigRefused(Refusal):
    """A config that breaks one or more config invariants; no queue is made.

    `invariants` names each one it breaks, in the specification's order.
    """

    def __init__(self, invariants: tuple[str, ...]) -> None:
        super().__init__(f"the config breaks {', '.join(invariants)}")
        self.invariants = invariants


class PublisherAlreadyJoined(Refusal):
    """PublisherJoins named an id a publisher already holds."""


class ConsumerAlreadyJoined(Refusal):
    """ConsumerJoins named an id a consumer already holds."""


class UnknownPublisher(Refusal):
    """A publish named a publisher id that never joined (spec note N10)."""

    def __init__(self, publisher_id: str) -> None:
        super().__init__(f"no publisher with id {publisher_id!r} has joined")
        self.publisher_id = publisher_id


class EqualRankRefused(Refusal):
    """A publish at the stored rank of its key: no Publish rule covers it (N11)."""

    def __init__(self, pocket_key: str, record_key: str, rank: int) -> None:
        super().__init__(f"{pocket_key!r}/{record_key!r} already stores rank {rank}")
        self.pocket_key = pocket_key
        self.record_key = record_key
        self.rank = rank
