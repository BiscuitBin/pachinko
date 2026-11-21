"""The invariant checker: `check(snapshot) -> violations`.

Each of the specification's 35 invariants is one function here, registered
under its name in `REGISTRY`, in the specification's order. Each is a direct
reading of the invariant, and yields one witness for each entity, or pair of
entities, that breaks it:

- `for a in X: for b in X: a != b implies ...` ranges over ordered pairs of
  distinct members of the snapshot's tuple (`_distinct_pairs`), so two members
  with equal fields still count as two (spec note N07).
  `ArrivalPositionUnoccupied` has no `a != b`, so it ranges over every pair,
  a record paired with itself included (`_all_pairs`).
- `p implies q` is `not p or q`: a false premise holds.
- A reference such as `r.pocket` is the pocket with that key. With two such
  pockets, which only a snapshot that breaks OnePocketPerKey holds, the
  reading must hold for each of them.
- A field the specification declares `when status = held` is present exactly
  while the record is held (model.py enforces it), so a premise that a record
  is held makes its `held_rank` and `mark` safe to read. `nacked_at` and
  `deferred_position` are optional even then, and each reading compares them
  with null only as the invariant does.

`Queue`'s apply step calls `check` once after each whole outcome and raises
`InvariantsBroken` naming every invariant that fails.
"""

from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass
from itertools import permutations, product

from .inspection import Snapshot
from .model import LeaseStatus, Mark, PocketStatus, RecordStatus

type Check = Callable[[Snapshot], Iterator[str]]

REGISTRY: dict[str, Check] = {}


@dataclass(frozen=True)
class Violation:
    invariant: str
    witness: str


class InvariantsBroken(Exception):
    """An outcome left the state breaking one or more invariants: a defect."""

    def __init__(self, violations: tuple[Violation, ...]) -> None:
        names = sorted({violation.invariant for violation in violations})
        super().__init__(f"invariants broken: {', '.join(names)}; " + "; ".join(v.witness for v in violations))
        self.violations = violations

    @property
    def names(self) -> frozenset[str]:
        return frozenset(violation.invariant for violation in self.violations)


def check(snapshot: Snapshot) -> tuple[Violation, ...]:
    """Every violation of every invariant in the snapshot, in registry order."""
    return tuple(Violation(name, witness) for name, rule in REGISTRY.items() for witness in rule(snapshot))


def invariant(name: str) -> Callable[[Check], Check]:
    def register(rule: Check) -> Check:
        if name in REGISTRY:
            raise ValueError(f"invariant {name} is registered twice")
        REGISTRY[name] = rule
        return rule

    return register


def _distinct_pairs[T](items: tuple[T, ...]) -> Iterable[tuple[T, T]]:
    return permutations(items, 2)


def _all_pairs[T](items: tuple[T, ...]) -> Iterable[tuple[T, T]]:
    return product(items, items)


# Pocket ------------------------------------------------------------------


@invariant("LeasedHasOneActiveLease")
def _leased_has_one_active_lease(s: Snapshot) -> Iterator[str]:
    for p in s.pockets:
        if p.status is PocketStatus.LEASED and len(s.active_leases_by_pocket[p.key]) != 1:
            yield f"leased pocket {p.key!r} has {len(s.active_leases_by_pocket[p.key])} active leases"


@invariant("UnleasedHasNoActiveLease")
def _unleased_has_no_active_lease(s: Snapshot) -> Iterator[str]:
    for p in s.pockets:
        if p.status is not PocketStatus.LEASED and len(s.active_leases_by_pocket[p.key]) != 0:
            yield f"{p.status} pocket {p.key!r} has {len(s.active_leases_by_pocket[p.key])} active leases"


@invariant("IdleHasNoRecords")
def _idle_has_no_records(s: Snapshot) -> Iterator[str]:
    for p in s.pockets:
        if p.status is PocketStatus.IDLE and len(s.records_by_pocket[p.key]) != 0:
            yield f"idle pocket {p.key!r} has {len(s.records_by_pocket[p.key])} records"


@invariant("ExpiriesWithinLimit")
def _expiries_within_limit(s: Snapshot) -> Iterator[str]:
    for p in s.pockets:
        if not p.consecutive_expiries <= s.config.max_consecutive_expiries:
            yield f"pocket {p.key!r} has {p.consecutive_expiries} consecutive expiries"


# Record ------------------------------------------------------------------


@invariant("HeldVersionNotNewer")
def _held_version_not_newer(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD:
            assert r.held_rank is not None
            if not r.held_rank <= r.rank:
                yield f"record {r.pocket!r}/{r.key!r} holds rank {r.held_rank} above its rank {r.rank}"


@invariant("NackedHasItsTime")
def _nacked_has_its_time(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.mark is Mark.NACKED and r.nacked_at is None:
            yield f"nacked record {r.pocket!r}/{r.key!r} has no nacked_at"


@invariant("OnlyNackedHasNackTime")
def _only_nacked_has_nack_time(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.mark is not Mark.NACKED and r.nacked_at is not None:
            yield f"{r.mark} record {r.pocket!r}/{r.key!r} has nacked_at {r.nacked_at}"


@invariant("DeferredHasArrivalPosition")
def _deferred_has_arrival_position(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD:
            assert r.held_rank is not None
            if r.held_rank < r.rank and r.deferred_position is None:
                yield f"record {r.pocket!r}/{r.key!r} defers rank {r.rank} with no deferred_position"


@invariant("OnlyDeferredHasArrivalPosition")
def _only_deferred_has_arrival_position(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.held_rank == r.rank and r.deferred_position is not None:
            yield f"record {r.pocket!r}/{r.key!r} defers nothing but has deferred_position {r.deferred_position}"


@invariant("DeferredArrivedAfterHeld")
def _deferred_arrived_after_held(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.deferred_position is not None and not r.deferred_position > r.position:
            yield f"record {r.pocket!r}/{r.key!r} deferred at {r.deferred_position}, not after {r.position}"


@invariant("HeldCopiesAgree")
def _held_copies_agree(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.deferred_position is None and r.held_payload != r.payload:
            yield f"record {r.pocket!r}/{r.key!r} defers nothing but its two payloads differ"


# SupersededRecord, DeadLetter and Lease ----------------------------------


@invariant("ShedBeforeExpiry")
def _shed_before_expiry(s: Snapshot) -> Iterator[str]:
    for x in s.superseded:
        if not x.shed_at <= x.expires_at:
            yield f"superseded {x.pocket!r}/{x.key!r} shed at {x.shed_at}, after it expires at {x.expires_at}"


@invariant("NackedBeforeExpiry")
def _nacked_before_expiry(s: Snapshot) -> Iterator[str]:
    for x in s.dead_letters:
        if not x.nacked_at <= x.expires_at:
            yield f"dead letter {x.pocket!r}/{x.key!r} nacked at {x.nacked_at}, after it expires at {x.expires_at}"


@invariant("GrantedBeforeExpiry")
def _granted_before_expiry(s: Snapshot) -> Iterator[str]:
    for x in s.leases:
        if not x.granted_at <= x.expires_at:
            yield f"lease {x.id!r} granted at {x.granted_at}, after it expires at {x.expires_at}"


@invariant("EndedAfterGrant")
def _ended_after_grant(s: Snapshot) -> Iterator[str]:
    for x in s.leases:
        if x.status in {LeaseStatus.RELEASED, LeaseStatus.EXPIRED}:
            assert x.ended_at is not None
            if not x.ended_at >= x.granted_at:
                yield f"lease {x.id!r} ended at {x.ended_at}, before its grant at {x.granted_at}"


# Config ------------------------------------------------------------------


@invariant("ExpiryLimitPositive")
def _expiry_limit_positive(s: Snapshot) -> Iterator[str]:
    if not s.config.max_consecutive_expiries >= 1:
        yield f"max_consecutive_expiries is {s.config.max_consecutive_expiries}"


@invariant("PullBatchPositive")
def _pull_batch_positive(s: Snapshot) -> Iterator[str]:
    if not s.config.max_pull_batch >= 1:
        yield f"max_pull_batch is {s.config.max_pull_batch}"


@invariant("LeaseTtlNonNegative")
def _lease_ttl_non_negative(s: Snapshot) -> Iterator[str]:
    if not s.config.lease_ttl >= 0:
        yield f"lease_ttl is {s.config.lease_ttl}"


@invariant("SupersededTtlNonNegative")
def _superseded_ttl_non_negative(s: Snapshot) -> Iterator[str]:
    if not s.config.superseded_ttl >= 0:
        yield f"superseded_ttl is {s.config.superseded_ttl}"


@invariant("DeadLetterTtlNonNegative")
def _dead_letter_ttl_non_negative(s: Snapshot) -> Iterator[str]:
    if not s.config.dead_letter_ttl >= 0:
        yield f"dead_letter_ttl is {s.config.dead_letter_ttl}"


# Identity and uniqueness -------------------------------------------------


@invariant("UniquePublisherId")
def _unique_publisher_id(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.publishers):
        if not a.id != b.id:
            yield f"two publishers share id {a.id!r}"


@invariant("UniqueConsumerId")
def _unique_consumer_id(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.consumers):
        if not a.id != b.id:
            yield f"two consumers share id {a.id!r}"


@invariant("OnePocketPerKey")
def _one_pocket_per_key(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.pockets):
        if not a.key != b.key:
            yield f"two pockets share key {a.key!r}"


@invariant("OneRecordPerKeyPerPocket")
def _one_record_per_key_per_pocket(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.records):
        if a.pocket == b.pocket and not a.key != b.key:
            yield f"two records of pocket {a.pocket!r} share key {a.key!r}"


@invariant("OneActiveLeasePerPocket")
def _one_active_lease_per_pocket(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.leases):
        if a.status is LeaseStatus.ACTIVE and b.status is LeaseStatus.ACTIVE and not a.pocket != b.pocket:
            yield f"active leases {a.id!r} and {b.id!r} share pocket {a.pocket!r}"


@invariant("OneActiveLeasePerConsumer")
def _one_active_lease_per_consumer(s: Snapshot) -> Iterator[str]:
    for c in s.consumers:
        if not len(s.active_leases_by_consumer[c.id]) <= 1:
            yield f"consumer {c.id!r} has {len(s.active_leases_by_consumer[c.id])} active leases"


@invariant("OutstandingOnlyWhileLeased")
def _outstanding_only_while_leased(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD:
            for p in s.pockets_keyed(r.pocket):
                if p.status is not PocketStatus.LEASED:
                    yield f"held record {r.pocket!r}/{r.key!r} is in a {p.status} pocket"


@invariant("MarkedPocketHasNoExpiries")
def _marked_pocket_has_no_expiries(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.mark is not Mark.UNSETTLED:
            for p in s.pockets_keyed(r.pocket):
                if p.consecutive_expiries != 0:
                    yield f"{r.mark} record {r.pocket!r}/{r.key!r} is in a pocket with {p.consecutive_expiries} expiries"


# Positions ---------------------------------------------------------------


@invariant("UniquePocketPosition")
def _unique_pocket_position(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.pockets):
        if not a.position != b.position:
            yield f"pockets {a.key!r} and {b.key!r} share position {a.position}"


@invariant("UniqueRecordPosition")
def _unique_record_position(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.records):
        if not a.position != b.position:
            yield f"records {a.pocket!r}/{a.key!r} and {b.pocket!r}/{b.key!r} share position {a.position}"


@invariant("PocketPositionsBelowNext")
def _pocket_positions_below_next(s: Snapshot) -> Iterator[str]:
    for p in s.pockets:
        if not p.position < s.next_position:
            yield f"pocket {p.key!r} at {p.position} is not below next_position {s.next_position}"


@invariant("RecordPositionsBelowNext")
def _record_positions_below_next(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if not r.position < s.next_position:
            yield f"record {r.pocket!r}/{r.key!r} at {r.position} is not below next_position {s.next_position}"


@invariant("ArrivalPositionUnoccupied")
def _arrival_position_unoccupied(s: Snapshot) -> Iterator[str]:
    for a, b in _all_pairs(s.records):
        if a.status is RecordStatus.HELD and a.deferred_position is not None and not a.deferred_position != b.position:
            yield f"record {a.pocket!r}/{a.key!r} defers to {a.deferred_position}, where {b.pocket!r}/{b.key!r} stands"


@invariant("UniqueArrivalPosition")
def _unique_arrival_position(s: Snapshot) -> Iterator[str]:
    for a, b in _distinct_pairs(s.records):
        if (
            a.status is RecordStatus.HELD
            and b.status is RecordStatus.HELD
            and a.deferred_position is not None
            and b.deferred_position is not None
            and not a.deferred_position != b.deferred_position
        ):
            yield f"records {a.pocket!r}/{a.key!r} and {b.pocket!r}/{b.key!r} share arrival position {a.deferred_position}"


@invariant("ArrivalPositionsBelowNext")
def _arrival_positions_below_next(s: Snapshot) -> Iterator[str]:
    for r in s.records:
        if r.status is RecordStatus.HELD and r.deferred_position is not None and not r.deferred_position < s.next_position:
            yield f"record {r.pocket!r}/{r.key!r} defers to {r.deferred_position}, not below {s.next_position}"
