"""The inspection seam: one complete, immutable snapshot of the queue's state.

Tests, the checker and the rules' guards all read state through a `Snapshot`.
It holds the config, `queue.next_position`, and every publisher, consumer,
pocket, record, lease, superseded record and dead letter, each a frozen
entity from model.py, in tuples. The specification's derived sets, such as a
pocket's `held_records` and a consumer's `active_leases`, are computed once,
when the snapshot is made, into read-only maps keyed by pocket key or
consumer id.

A snapshot refuses a reference to a pocket or consumer it does not hold. Two
snapshots of the same state are equal, so a test proves that a refusal changed
nothing by comparing the snapshot before and after.
"""

from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field, fields
from types import MappingProxyType

from .config import Config
from .model import (
    Consumer,
    DeadLetter,
    Duration,
    Lease,
    LeaseStatus,
    Mark,
    Pocket,
    Publisher,
    Record,
    RecordStatus,
    SupersededRecord,
    Timestamp,
)

type Index[T] = Mapping[str, tuple[T, ...]]


@dataclass(frozen=True)
class ConfigValues:
    """The config as a snapshot holds it.

    Unlike `Config` it takes any values, so that a hand-built snapshot can hold
    a config the five config invariants reject, and the checker can name them.
    """

    lease_ttl: Duration
    max_consecutive_expiries: int
    max_pull_batch: int
    superseded_ttl: Duration
    dead_letter_ttl: Duration

    @classmethod
    def of(cls, config: Config) -> "ConfigValues":
        return cls(**{item.name: getattr(config, item.name) for item in fields(Config)})


def _index[T](keys: Iterable[str], items: Iterable[T], key_of: str) -> Index[T]:
    grouped: dict[str, list[T]] = {key: [] for key in keys}
    for item in items:
        grouped[getattr(item, key_of)].append(item)
    return MappingProxyType({key: tuple(group) for key, group in grouped.items()})


def _is_ending(record: Record) -> bool:
    return record.status is RecordStatus.HELD and record.mark is not Mark.UNSETTLED and record.deferred_position is None


@dataclass(frozen=True)
class Snapshot:
    config: ConfigValues
    next_position: int
    publishers: tuple[Publisher, ...] = ()
    consumers: tuple[Consumer, ...] = ()
    pockets: tuple[Pocket, ...] = ()
    records: tuple[Record, ...] = ()
    leases: tuple[Lease, ...] = ()
    superseded: tuple[SupersededRecord, ...] = ()
    dead_letters: tuple[DeadLetter, ...] = ()

    # Derived once in __post_init__; left out of equality and repr.
    records_by_pocket: Index[Record] = field(init=False, compare=False, repr=False)
    queued_by_pocket: Index[Record] = field(init=False, compare=False, repr=False)
    held_by_pocket: Index[Record] = field(init=False, compare=False, repr=False)
    ending_by_pocket: Index[Record] = field(init=False, compare=False, repr=False)
    leases_by_pocket: Index[Lease] = field(init=False, compare=False, repr=False)
    active_leases_by_pocket: Index[Lease] = field(init=False, compare=False, repr=False)
    active_leases_by_consumer: Index[Lease] = field(init=False, compare=False, repr=False)

    def __post_init__(self) -> None:
        for name in ("publishers", "consumers", "pockets", "records", "leases", "superseded", "dead_letters"):
            object.__setattr__(self, name, tuple(getattr(self, name)))
        self._refuse_dangling_references()
        pocket_keys = [pocket.key for pocket in self.pockets]
        active = [lease for lease in self.leases if lease.status is LeaseStatus.ACTIVE]
        derived = {
            "records_by_pocket": _index(pocket_keys, self.records, "pocket"),
            "queued_by_pocket": _index(
                pocket_keys, (r for r in self.records if r.status is RecordStatus.QUEUED), "pocket"
            ),
            "held_by_pocket": _index(pocket_keys, (r for r in self.records if r.status is RecordStatus.HELD), "pocket"),
            "ending_by_pocket": _index(pocket_keys, filter(_is_ending, self.records), "pocket"),
            "leases_by_pocket": _index(pocket_keys, self.leases, "pocket"),
            "active_leases_by_pocket": _index(pocket_keys, active, "pocket"),
            "active_leases_by_consumer": _index((c.id for c in self.consumers), active, "consumer"),
        }
        for name, value in derived.items():
            object.__setattr__(self, name, value)

    def _refuse_dangling_references(self) -> None:
        pocket_keys = {pocket.key for pocket in self.pockets}
        consumer_ids = {consumer.id for consumer in self.consumers}
        dangling = [
            *(f"record {r.key!r} names pocket {r.pocket!r}" for r in self.records if r.pocket not in pocket_keys),
            *(f"lease {x.id!r} names pocket {x.pocket!r}" for x in self.leases if x.pocket not in pocket_keys),
            *(f"lease {x.id!r} names consumer {x.consumer!r}" for x in self.leases if x.consumer not in consumer_ids),
            *(f"superseded {s.key!r} names pocket {s.pocket!r}" for s in self.superseded if s.pocket not in pocket_keys),
            *(f"dead letter {d.key!r} names pocket {d.pocket!r}" for d in self.dead_letters if d.pocket not in pocket_keys),
            *(
                f"dead letter {d.key!r} names consumer {d.nacked_by!r}"
                for d in self.dead_letters
                if d.nacked_by not in consumer_ids
            ),
        ]
        if dangling:
            raise ValueError(f"a snapshot holds every entity it references: {'; '.join(dangling)}")

    def pockets_keyed(self, key: str) -> tuple[Pocket, ...]:
        """Every pocket with this key: one, by OnePocketPerKey, or none."""
        return tuple(pocket for pocket in self.pockets if pocket.key == key)

    def record(self, pocket_key: str, record_key: str) -> Record | None:
        """`Record{pocket: pocket, key: record_key}`: the first such record, or None."""
        for record in self.records_by_pocket.get(pocket_key, ()):
            if record.key == record_key:
                return record
        return None


@dataclass(frozen=True)
class View:
    """What a guard and an effect read: the state before the rule, and `now`.

    `now` is the outcome's one timestamp. The apply step gives every view of
    one outcome the same reader, which reads the clock the first time a rule
    reads `now` and never again in that outcome, so an outcome whose rules
    never read it, such as a refused mark, reads no clock (S085). A test that
    builds a view by hand gives it a plain timestamp.
    """

    state: Snapshot
    clock_read: Timestamp | Callable[[], Timestamp]

    @property
    def now(self) -> Timestamp:
        read = self.clock_read
        return read() if callable(read) else read
