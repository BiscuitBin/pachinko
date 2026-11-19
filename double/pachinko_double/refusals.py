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


class PullRefused(Refusal):
    """A pull no Pull rule answers (spec note N14).

    The three Pull rules leave no gap in any state that holds the invariants,
    so `Queue` never raises this; it is the Pull table's answer when no guard
    holds, as shared rule 4 asks of every action's table.
    """

    def __init__(self, consumer_id: str) -> None:
        super().__init__(f"no Pull rule answers a pull by {consumer_id!r}")
        self.consumer_id = consumer_id


class UnknownConsumer(Refusal):
    """A consumer action or read named a consumer id that never joined (N18, N22)."""

    def __init__(self, consumer_id: str) -> None:
        super().__init__(f"no consumer with id {consumer_id!r} has joined")
        self.consumer_id = consumer_id


class LeaseFenced(Refusal):
    """An action on a lease that is not active or not the consumer's.

    `action` names the trigger, such as "Renew". The specification's lease
    actions each require `lease.status = active` and
    `lease.consumer = consumer`; with either false, or with an id that names
    no lease, no rule answers, and the action is refused.
    """

    def __init__(self, action: str, consumer_id: str, lease_id: str) -> None:
        super().__init__(f"{action} by {consumer_id!r}: lease {lease_id!r} is not active and theirs")
        self.action = action
        self.consumer_id = consumer_id
        self.lease_id = lease_id


class RecordNotHeld(Refusal):
    """A mark naming a key its lease's pocket does not hold (spec note N21).

    The lease is active and the consumer's, but `Record{pocket: lease.pocket,
    key: record_key}` does not exist or is queued, so no Ack or Nack rule
    answers. A key held in another pocket meets the same refusal.
    """

    def __init__(self, action: str, consumer_id: str, lease_id: str, record_key: str) -> None:
        super().__init__(f"{action} by {consumer_id!r} under lease {lease_id!r}: {record_key!r} is not held")
        self.action = action
        self.consumer_id = consumer_id
        self.lease_id = lease_id
        self.record_key = record_key


class MarkConflict(Refusal):
    """An ack of a nacked record or a nack of an acked one (spec note N21).

    The first mark stands, and no rule covers the other of the two, so the
    action is refused. `mark` is the mark that stands.
    """

    def __init__(self, action: str, consumer_id: str, lease_id: str, record_key: str, mark: str) -> None:
        super().__init__(f"{action} by {consumer_id!r} under lease {lease_id!r}: {record_key!r} is already {mark}")
        self.action = action
        self.consumer_id = consumer_id
        self.lease_id = lease_id
        self.record_key = record_key
        self.mark = mark


class ReleaseRefused(Refusal):
    """A release of an active lease of the consumer's that no Release rule answers (N26).

    Both release rules also require the lease's pocket to be leased, which
    `UnleasedHasNoActiveLease` makes true of every active lease, so `Queue`
    never raises this; it is the Release table's answer once the lease is
    not fenced, as `PullRefused` is the Pull table's.
    """

    def __init__(self, consumer_id: str, lease_id: str) -> None:
        super().__init__(f"no Release rule answers a release of {lease_id!r} by {consumer_id!r}")
        self.consumer_id = consumer_id
        self.lease_id = lease_id
