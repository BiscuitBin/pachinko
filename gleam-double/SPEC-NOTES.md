# Specification notes from the Gleam double

The Gleam double under `gleam-double/` is derived from `docs/specs/pachinko.allium`, read
with the maintainer's four rulings on the Python double. This file records every place
where the specification was silent or ambiguous, or forced an interpretation, and every
catalog row the double ports although it asserts more than the specification states
(labelled `impl-choice`). The maintainer sets it beside
[`double/SPEC-NOTES.md`](../double/SPEC-NOTES.md) to judge the specification: where two
doubles written apart read a passage the same way, the reading is likely the
specification's; where they differ, the passage is ambiguous.

Each note gives its id, the ticket that wrote it, the rule, invariant, entity or comment
it concerns, what the specification says, what the double does, the `N`-note of the
Python double it agrees with or departs from, and the question it leaves. The ids run
`G01`, `G02` and so on, so that a note here cannot be mistaken for an `N`-note. A note
that only agrees with its `N`-note repeats that note's question, so that this file can be
read alone.

The double is one owner actor in front of a pure core. Where a note concerns what the
actor adds, which the Python double could not show, it says "no `N`-note".

## Notes

### G01: timestamps and durations are whole milliseconds

- **Ticket:** GD01.
- **Concerns:** the `Timestamp` and `Duration` types; `config.lease_ttl`,
  `config.superseded_ttl` and `config.dead_letter_ttl`.
- **The specification says:** fields are typed `Timestamp` and `Duration`, the defaults
  are written `5.minutes` and `12.hours`, and the TTL invariants compare with
  `0.seconds`. It names no resolution.
- **The double does:** both are `Int` counts of milliseconds, with the constants
  `millisecond`, `second`, `minute` and `hour` in `time.gleam`. An `Int` on the Erlang
  target has no bound, so `now + ttl` is exact. The fake clock starts at 0, the ported
  fixture sets it to the Rust fixture's start, 1,000,000, and two events may share a
  timestamp.
- **`N`-note:** agrees with N01.
- **Question:** does any rule depend on a resolution finer than a millisecond, or on two
  events never sharing a timestamp?

### G02: a config that breaks a config invariant is refused when it is made (S005 is `impl-choice`)

- **Ticket:** GD01.
- **Concerns:** `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
  `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`; scenarios S003, S004 and
  S005.
- **The specification says:** the comments above these invariants say "the model is
  defined for" `max_consecutive_expiries >= 1`, `max_pull_batch >= 1` and non-negative
  TTLs. It does not say what happens to a deployment configured outside that range.
- **The double does:**
  - `Config` is an opaque type. `config.new` is the only way to make one from values,
    and it answers the refusal `ConfigRefused`, naming every config invariant the values
    break, in the specification's order. The owner is started with a `Config`, so no
    owner and no queue is ever made from such values.
  - The boundary values, 1 for the two limits and 0 for each TTL, are admitted.
  - No owner exists before the refusal, so these three tests cannot compare a snapshot
    before and after. Each asserts the refusal itself.
  - S005 is `impl-choice`. Its owning Rust test is a `compile_fail` doctest: a Rust
    `Duration` cannot be negative, so the refusal happens at compile time. Here a
    duration is an `Int`, so it is refused at run time. The test refuses each of the
    three TTLs, where the Rust tags only `lease_ttl`'s doctest with S005.
- **`N`-note:** agrees with N09.
- **Question:** is "refused at configuration" the intended reading of "the model is
  defined for", or should the specification state the refusal itself?

### G03: a join with an id already taken is refused

- **Ticket:** GD01.
- **Concerns:** `PublisherJoins` and `ConsumerJoins`; scenarios S009 and S010.
- **The specification says:** each join `requires: not exists Publisher{id:
  publisher_id}` (or `Consumer`), and says nothing more about a join whose id is taken.
- **The double does:**
  - Each join's table answers a refusal when its one guard does not hold:
    `PublisherAlreadyJoined` and `ConsumerAlreadyJoined`, the names N12 uses. The refusal
    changes nothing: the owner replies with it and keeps the state it had.
  - A publisher and a consumer may share an id, since each guard reads its own entity
    only.
  - When several processes send the same join at once, the owner's mailbox orders them.
    Exactly one joins and every other is refused, whichever order they arrive in. A test
    with no scenario tag shows it; no catalog row names competing callers.
- **`N`-note:** agrees with N12. The competing callers have no `N`-note.
- **Question:** should a second join with the same id be refused, as here, or succeed
  and change nothing, which the specification's words also allow? The catalog rows S009
  and S010 say "joins once".

### G04: the queue's first position is 0

- **Ticket:** GD01.
- **Concerns:** `Queue.next_position` and the `Position` vocabulary entry.
- **The specification says:** every rule that allots a position takes `next_position`
  and advances it. It gives no initial value.
- **The double does:** a new queue's `next_position` is 0. Nothing in GD01 allots a
  position; the rows that assert absolute positions are ported in GD02, and their notes
  cite this one. GD01's ports of S003, S004 and S005 assert that an owner starts over an
  admitted config and holds no publisher and no consumer, and not the position it starts
  at. The two owner tests, which pin no catalog row, compare whole snapshots and so do
  pin the 0.
- **`N`-note:** agrees with N04.
- **Question:** is the starting value meant to be unspecified? Only order and uniqueness
  are observable through the rules, so any start satisfies them.

### G05: a rule asks for `now` and for a lease id; a guard and a refusal cannot

- **Ticket:** GD01.
- **Concerns:** every rule that reads `now`; `PullClaimsFrontPocket`, the one rule that
  creates a lease; the Lease expiry comment's "one serialized stream".
- **The specification says:** rules read `now` freely, in `ensures` clauses and, for
  the lapse and the two decays, in their triggers. It does not say how many times one
  outcome reads the clock, or whether a rejected action has a `now` at all. `Lease`
  declares no identity, though every action names its lease.
- **The double does:**
  - A guard is a function of the state and the trigger's input, and nothing hands it a
    clock. So a guard cannot read the time, and a refusal, which is a table none of
    whose guards held, reads no clock and draws no lease id.
  - A rule's effect asks for `now`, a lease id or a position by returning a
    continuation. The apply step answers the first request for `now` in an outcome by
    asking the owner, which calls the clock function it was started with, and answers
    every later request in that outcome with the same value. So one outcome has one
    `now`, shared by the rule the trigger reached and the rules its events reach.
  - A lease id is a `String`, drawn from the owner's injected lease-id function once for
    each request, and only a rule that creates a lease requests one.
  - The owner is the only process that calls either function, between taking a message
    from its mailbox and replying to it. The time an action is processed at is therefore
    the time the owner reached it, not the time its caller sent it.
- **`N`-note:** agrees with N28 (a refused action reads no clock) and N15 (a lease id is
  drawn only when a lease is created), and with N05 in giving a lease an id. Where the
  Python double's queue reads its clock inside a method call, this one's core never
  holds a clock at all. "One `now` per outcome, read when the owner processes the
  action" has no `N`-note of its own.
- **Question:** does a rejected action have a `now`? And should the specification say
  that an action's `now` is the time it is processed, which the Lease expiry comment
  implies when it says that a late action processed before the lapse succeeds?

### G06: the snapshot lists publishers and consumers in the order they joined (S009's order is `impl-choice`)

- **Ticket:** GD01.
- **Concerns:** the `Publisher` and `Consumer` entities; the inspection seam, which no
  surface of the specification names; scenarios S009 and S010.
- **The specification says:** a join creates a `Publisher` or a `Consumer`. It gives the
  publishers no order and the consumers none, and no rule reads one.
- **The double does:**
  - The state keeps each kind in a list, a join appends to it, and the snapshot shows the
    two lists as they are. A stated order is what lets two snapshots compare with `==`,
    as every test that shows a refusal changing nothing does.
  - S009 is `impl-choice` in one assertion. Its Rust test asserts the publishers as the
    ordered list `p`, `q`, and the port asserts the same. That holds because of this
    order, not because of anything the specification states.
  - S010's Rust test asserts only that `c4` is among the consumers, and the port asserts
    the same.
  - A test that pins no catalog row may assert the join order, since it is this double's
    stated order.
- **`N`-note:** no `N`-note.
- **Question:** are publishers and consumers meant to have no order? If so, should S009
  assert one?
