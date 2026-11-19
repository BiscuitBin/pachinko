"""`Queue`, the facade: one method per trigger a surface provides.

Every method hands its trigger to the one apply step, `Queue._apply`, which
makes the whole outcome of that trigger, in this order:

- It reads the clock at most once, the first time a guard, a refusal or an
  effect reads `now`; that one value is `now` for the whole outcome. An
  outcome whose rules never read `now`, such as a refused mark, reads no
  clock at all (S085).
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

from . import consuming, decay, expiry, identity, lease_end, publish, pull, settle
from .config import Config
from .consuming import LeaseView
from .effects import Clock, LeaseIds
from .inspection import ConfigValues, Snapshot, View
from .invariants import InvariantsBroken, check
from .model import Consumer, DeadLetter, Lease, LeaseStatus, Pocket, Publisher, Record, SupersededRecord, Timestamp
from .pull import Batch, NoPocketAvailable
from .refusals import UnknownConsumer, UnknownPublisher
from .rules import Defect, Table, dispatch

# The tables of the events a rule emits, by event type.
EVENT_TABLES: Mapping[type, Table] = {
    publish.PocketTouched: publish.POCKET_TOUCHED,
    pull.Handed: pull.HANDED,
    lease_end.Freed: lease_end.FREED,
    expiry.LeaseLapsed: expiry.LEASE_LAPSED,
}

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


@dataclass(frozen=True)
class TickReport:
    """What one `Queue.tick()` did, in the order it did it (spec note N33).

    `lapsed` holds each lease the tick lapsed, as the state keeps it once
    ended: status expired, `ended_at` the tick's `now`. `decayed_superseded`
    and `decayed_dead_letters` hold each entry that decayed, as it stood.
    """

    lapsed: tuple[Lease, ...] = ()
    decayed_superseded: tuple[SupersededRecord, ...] = ()
    decayed_dead_letters: tuple[DeadLetter, ...] = ()


class _Instant:
    """A clock that answers one timestamp: the `now` one tick read."""

    def __init__(self, now: Timestamp) -> None:
        self._now = now

    def now(self) -> Timestamp:
        return self._now


class _Now:
    """`now` for one outcome: the clock, read at most once, when first asked."""

    def __init__(self, clock: Clock) -> None:
        self._clock = clock
        self._value: Timestamp | None = None

    def __call__(self) -> Timestamp:
        if self._value is None:
            self._value = self._clock.now()
        return self._value


class Outcome:
    """What an effect writes through: the working state of one outcome.

    Entities are frozen, so an effect changes one by `replace`-ing it with a
    new value under the same key.
    """

    def __init__(self, state: _State, now: Callable[[], Timestamp], config: Config, lease_ids: LeaseIds) -> None:
        self._state = state
        self._lease_ids = lease_ids
        self._now = now
        self.config = config
        self.events: deque[Any] = deque()
        self._answer: Any = None
        self._answered = False

    @property
    def now(self) -> Timestamp:
        """The outcome's one timestamp; reading it may read the clock, once."""
        return self._now()

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

    # Pull ----------------------------------------------------------------

    def pull(self, consumer_id: str, limit: int) -> Batch | NoPocketAvailable:
        """`Pull(consumer, limit)`: the pull's answer, a `Batch` or `NoPocketAvailable`."""
        self._require_consumer(consumer_id)
        return self._apply(pull.PULL, pull.Pull(consumer_id, limit))

    # Ack and nack --------------------------------------------------------

    def ack(self, consumer_id: str, lease_id: str, record_key: str) -> None:
        """`Ack(consumer, lease, record_key)`, naming the lease by its id; it answers nothing."""
        self._require_consumer(consumer_id)
        self._apply(settle.ACK, settle.Ack(consumer_id, lease_id, record_key))

    def nack(self, consumer_id: str, lease_id: str, record_key: str) -> None:
        """`Nack(consumer, lease, record_key)`, naming the lease by its id; it answers nothing."""
        self._require_consumer(consumer_id)
        self._apply(settle.NACK, settle.Nack(consumer_id, lease_id, record_key))

    # Renew and release ---------------------------------------------------

    def renew(self, consumer_id: str, lease_id: str) -> None:
        """`Renew(consumer, lease)`, naming the lease by its id (N05); it answers nothing."""
        self._require_consumer(consumer_id)
        self._apply(settle.RENEW, settle.Renew(consumer_id, lease_id))

    def release(self, consumer_id: str, lease_id: str) -> None:
        """`Release(consumer, lease)`, naming the lease by its id; it answers nothing.

        The release and every `Freed` it sends are one outcome, checked once.
        """
        self._require_consumer(consumer_id)
        self._apply(settle.RELEASE, settle.Release(consumer_id, lease_id))

    # The clock -----------------------------------------------------------

    def tick(self) -> TickReport:
        """Apply every temporal trigger due at the clock's `now`, and report it.

        The double has no background time: a lease lapses and an entry decays
        only here. The clock is read once, and every outcome of the tick sees
        that one `now`. Each due lapse is its own outcome, `LeaseLapses` and
        the expiry rule and every `Freed` it sends, checked once; then each
        due decay is its own outcome, checked once (spec notes N33 to N35).

        The lapses come first, in `expiry.due_lapses` order. The decays due
        are read from the state the lapses left, so an entry a lapse creates
        with a zero TTL decays in the same tick: once a tick returns, nothing
        is due at its `now`.
        """
        now = self._clock.now()
        instant = _Instant(now)
        lapsed: list[Lease] = []
        for lease in expiry.due_lapses(self.snapshot(), now):
            self._apply(expiry.LEASE_EXPIRES_AT, expiry.LeaseExpiresAt(lease.id), clock=instant)
            ended = self._state.leases[lease.id]
            if lease.status is LeaseStatus.ACTIVE and ended.status is LeaseStatus.EXPIRED:
                lapsed.append(ended)
        state = self.snapshot()
        decayed_superseded = tuple(
            self._apply(decay.SUPERSEDED_RECORD_EXPIRES_AT, decay.SupersededRecordExpiresAt(shed), clock=instant)
            for shed in decay.due_superseded(state, now)
        )
        decayed_dead_letters = tuple(
            self._apply(decay.DEAD_LETTER_EXPIRES_AT, decay.DeadLetterExpiresAt(letter), clock=instant)
            for letter in decay.due_dead_letters(state, now)
        )
        return TickReport(
            lapsed=tuple(lapsed),
            decayed_superseded=tuple(entry for entry in decayed_superseded if entry is not None),
            decayed_dead_letters=tuple(entry for entry in decayed_dead_letters if entry is not None),
        )

    # The Consuming surface's exposure ------------------------------------

    def lease_view(self, consumer_id: str) -> LeaseView | None:
        """The consumer's active lease and its held records, or None (consuming.py).

        A read, not a trigger: it reads the committed snapshot, no clock and
        no rule. An id that never joined is refused, as for the actions (N22).
        """
        self._require_consumer(consumer_id)
        return consuming.lease_view(self.snapshot(), consumer_id)

    def _require_consumer(self, consumer_id: str) -> None:
        # The trigger takes a Consumer, so an id that never joined cannot make
        # one: refused before the outcome starts (spec note N18, impl-choice).
        if consumer_id not in self._state.consumers:
            raise UnknownConsumer(consumer_id)

    # The apply step ------------------------------------------------------

    def _apply(
        self,
        table: Table,
        trigger: Any,
        *,
        event_tables: Mapping[type, Table] | None = None,
        clock: Clock | None = None,
    ) -> Any:
        """Make the whole outcome of one trigger, check it, then commit it.

        It returns the outcome's answer, what an effect gave `Outcome.answer`,
        or None when no effect answered.

        `event_tables` replaces `EVENT_TABLES` for a test that drives the apply
        step with scratch tables. `clock` replaces the queue's clock for this
        outcome: `tick()` passes the one instant it read.
        """
        routes = EVENT_TABLES if event_tables is None else event_tables
        now = _Now(self._clock if clock is None else clock)
        outcome = Outcome(self._state.copy(), now, self._config, self._lease_ids)
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
        view = View(outcome._state.snapshot(self._config_values), outcome._now)
        dispatch(table, view, outcome, trigger)
