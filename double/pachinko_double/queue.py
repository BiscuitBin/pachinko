"""`Queue`, the facade: one method per trigger a surface provides.

Every method hands its trigger to the one apply step, `Queue._apply`, which
makes the whole outcome of that trigger, in this order:

- It reads the clock once; that is `now` for the whole outcome.
- It dispatches the trigger through its table (rules.py), on a working copy
  of the state.
- It dispatches each event an effect emitted, first in first out, through
  that event's table, each guard reading the state the rules before it left.
- It runs the invariant checker once, on the state after the whole chain,
  and raises `InvariantsBroken` naming each invariant that fails.
- Only then does it commit the working copy. A refusal, a defect or a broken
  invariant anywhere in the chain leaves the queue as it was.
- It returns what an effect handed back through `Outcome.answer`, the
  trigger's answer to its caller, such as Pull's `Batch`; a trigger that
  answers nothing returns None. An answer never leaves an outcome that is
  refused, defective or breaks an invariant.

The apply step is the only place that allots positions: an effect asks its
`Outcome` for one, and the outcome takes `next_position` and advances it.
"""

from collections import deque
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from typing import Any

from . import identity, publish
from .config import Config
from .effects import Clock, LeaseIds
from .inspection import ConfigValues, Snapshot, View
from .invariants import InvariantsBroken, check
from .model import Consumer, DeadLetter, Lease, Pocket, Publisher, Record, SupersededRecord, Timestamp
from .refusals import UnknownPublisher
from .rules import Defect, Table, dispatch

# The tables of the events a rule emits, by event type. Handed's table follows
# in PD02, Freed's in PD03 and LeaseLapsed's in PD05.
EVENT_TABLES: Mapping[type, Table] = {publish.PocketTouched: publish.POCKET_TOUCHED}

type Entity = Publisher | Consumer | Pocket | Record | Lease | SupersededRecord | DeadLetter

# Each keyed entity's collection in the state, and its key there.
_KEYED: dict[type, tuple[str, Callable[[Any], Any]]] = {
    Publisher: ("publishers", lambda e: e.id),
    Consumer: ("consumers", lambda e: e.id),
    Pocket: ("pockets", lambda e: e.key),
    Record: ("records", lambda e: (e.pocket, e.key)),
    Lease: ("leases", lambda e: e.id),
}
# The shed queues are lists: their entries have no key of their own.
_LISTED: dict[type, str] = {SupersededRecord: "superseded", DeadLetter: "dead_letters"}


@dataclass
class _State:
    """The queue's mutable state, which only the apply step changes."""

    next_position: int = 0
    publishers: dict[str, Publisher] = field(default_factory=dict)
    consumers: dict[str, Consumer] = field(default_factory=dict)
    pockets: dict[str, Pocket] = field(default_factory=dict)
    records: dict[tuple[str, str], Record] = field(default_factory=dict)
    leases: dict[str, Lease] = field(default_factory=dict)
    superseded: list[SupersededRecord] = field(default_factory=list)
    dead_letters: list[DeadLetter] = field(default_factory=list)

    def copy(self) -> "_State":
        return _State(
            next_position=self.next_position,
            publishers=dict(self.publishers),
            consumers=dict(self.consumers),
            pockets=dict(self.pockets),
            records=dict(self.records),
            leases=dict(self.leases),
            superseded=list(self.superseded),
            dead_letters=list(self.dead_letters),
        )

    def snapshot(self, config: ConfigValues) -> Snapshot:
        return Snapshot(
            config=config,
            next_position=self.next_position,
            publishers=tuple(self.publishers.values()),
            consumers=tuple(self.consumers.values()),
            pockets=tuple(self.pockets.values()),
            records=tuple(self.records.values()),
            leases=tuple(self.leases.values()),
            superseded=tuple(self.superseded),
            dead_letters=tuple(self.dead_letters),
        )


class Outcome:
    """What an effect writes through: the working state of one outcome.

    Entities are frozen, so an effect changes one by `replace`-ing it with a
    new value under the same key.
    """

    def __init__(self, state: _State, now: Timestamp, config: Config, lease_ids: LeaseIds) -> None:
        self._state = state
        self._lease_ids = lease_ids
        self.now = now
        self.config = config
        self.events: deque[Any] = deque()
        self._answer: Any = None
        self._answered = False

    def answer(self, value: Any) -> None:
        """What the trigger hands back to its caller, such as Pull's `Batch`.

        One outcome hands back at most one answer, and the apply step returns
        it only once the outcome is checked and committed. The value goes back
        exactly as given: an effect that answers with entities it read from
        the view hands back those entities as they were before the chain ran.
        """
        if self._answered:
            raise Defect(f"the outcome already answers {self._answer!r}")
        self._answer = value
        self._answered = True

    def allot_position(self) -> int:
        """`let position = queue.next_position` with `queue.next_position = position + 1`."""
        position = self._state.next_position
        self._state.next_position = position + 1
        return position

    def draw_lease_id(self) -> str:
        return self._lease_ids.next_id()

    def emit(self, event: Any) -> None:
        self.events.append(event)

    def create(self, entity: Entity) -> None:
        if type(entity) in _LISTED:
            getattr(self._state, _LISTED[type(entity)]).append(entity)
            return
        collection, key = self._keyed(entity)
        if key in collection:
            raise Defect(f"{type(entity).__name__} {key!r} already exists")
        collection[key] = entity

    def replace(self, entity: Entity) -> None:
        collection, key = self._keyed(entity)
        if key not in collection:
            raise Defect(f"{type(entity).__name__} {key!r} does not exist")
        collection[key] = entity

    def remove(self, entity: Entity) -> None:
        if type(entity) in _LISTED:
            getattr(self._state, _LISTED[type(entity)]).remove(entity)
            return
        collection, key = self._keyed(entity)
        if collection.get(key) != entity:
            raise Defect(f"{type(entity).__name__} {key!r} is not as the rule read it")
        del collection[key]

    def _keyed(self, entity: Entity) -> tuple[dict[Any, Any], Any]:
        name, key_of = _KEYED[type(entity)]
        return getattr(self._state, name), key_of(entity)


class Queue:
    """The deployment's one queue, driven one trigger at a time."""

    def __init__(self, config: Config = Config(), *, clock: Clock, lease_ids: LeaseIds) -> None:
        self._config = config
        self._config_values = ConfigValues.of(config)
        self._clock = clock
        self._lease_ids = lease_ids
        self._state = _State()

    def snapshot(self) -> Snapshot:
        """The inspection seam: the whole committed state, immutable."""
        return self._state.snapshot(self._config_values)

    # Identity ------------------------------------------------------------

    def publisher_joins(self, publisher_id: str) -> None:
        self._apply(identity.PUBLISHER_JOINS, identity.PublisherJoins(publisher_id))

    def consumer_joins(self, consumer_id: str) -> None:
        self._apply(identity.CONSUMER_JOINS, identity.ConsumerJoins(consumer_id))

    # Publish -------------------------------------------------------------

    def publish(self, publisher_id: str, pocket_key: str, record_key: str, rank: int, payload: bytes) -> None:
        # The trigger takes a Publisher, so an id that never joined cannot make
        # one: refused before the outcome starts (spec note N10, impl-choice).
        if not any(publisher.id == publisher_id for publisher in self._state.publishers.values()):
            raise UnknownPublisher(publisher_id)
        trigger = publish.Publish(publisher_id, pocket_key, record_key, rank, payload)
        self._apply(publish.PUBLISH, trigger)

    # The apply step ------------------------------------------------------

    def _apply(self, table: Table, trigger: Any, *, event_tables: Mapping[type, Table] | None = None) -> Any:
        """Make the whole outcome of one trigger, check it, then commit it.

        It returns the outcome's answer, what an effect gave `Outcome.answer`,
        or None when no effect answered.

        `event_tables` replaces `EVENT_TABLES` for a test that drives the apply
        step with scratch tables.
        """
        routes = EVENT_TABLES if event_tables is None else event_tables
        outcome = Outcome(self._state.copy(), self._clock.now(), self._config, self._lease_ids)
        self._dispatch(table, trigger, outcome)
        while outcome.events:
            event = outcome.events.popleft()
            if type(event) not in routes:
                raise Defect(f"no table answers the event {event!r}")
            self._dispatch(routes[type(event)], event, outcome)
        after = outcome._state.snapshot(self._config_values)
        violations = check(after)
        if violations:
            raise InvariantsBroken(violations)
        self._state = outcome._state
        return outcome._answer

    def _dispatch(self, table: Table, trigger: Any, outcome: Outcome) -> None:
        view = View(outcome._state.snapshot(self._config_values), outcome.now)
        dispatch(table, view, outcome, trigger)
