# Specification notes from the Python double

The Python double under `double/` is derived from `docs/specs/pachinko.allium` alone.
This file records every place where the specification was silent or ambiguous, or
forced an interpretation, and every catalog row the double ports although it asserts
more than the specification states (labelled `impl-choice`). It is the deliverable the
maintainer uses to judge the specification: each note ends with the question it leaves.

Each note gives its id, the ticket that wrote it, the rule, invariant, entity or comment
it concerns, what the specification says, what the double does, and the question.

## N01: timestamps and durations are whole milliseconds

- **Ticket:** PD01.
- **Concerns:** the `Timestamp` and `Duration` types; `config.lease_ttl`,
  `config.superseded_ttl` and `config.dead_letter_ttl`; every `now + config.…` sum.
- **The specification says:** fields are typed `Timestamp` and `Duration`, defaults
  are written `5.minutes` and `12.hours`, and the TTL invariants compare with
  `0.seconds`. It names no resolution.
- **The double does:** both are Python `int` counts of milliseconds, with the named
  constants `MILLISECOND`, `SECOND`, `MINUTE` and `HOUR` in `model.py`. Arithmetic is
  exact, so `expires_at = now + ttl` never rounds. The fake clock starts at 0.
- **Question:** does any rule depend on a resolution finer than a millisecond, or on two
  events sharing a timestamp being impossible? The double lets them share one.

## N02: a payload is `bytes`

- **Ticket:** PD01.
- **Concerns:** `Record.payload`, `Record.held_payload`, `SupersededRecord.payload`,
  `DeadLetter.payload` and the vocabulary entry for Payload.
- **The specification says:** the fields are typed `String`, "because Allium 3 has no
  byte type; the implementation stores bytes".
- **The double does:** every payload field is `bytes`, as the vocabulary says the
  implementation stores it. Nothing in the double reads a payload; it only compares two
  for equality, in `HeldCopiesAgree`.
- **Question:** none, unless the maintainer wants the double to follow the `String` typing
  instead.

## N03: a rank is an unbounded integer

- **Ticket:** PD01.
- **Concerns:** `Record.rank`, `Record.held_rank`, the ranks of `SupersededRecord` and
  `DeadLetter`, and the Publish rules' comparisons `rank > record.rank` and
  `rank < record.rank`.
- **The specification says:** a rank is an `Integer`, with no bound stated.
- **The double does:** a rank is a Python `int`, which has no bound, so negative, zero
  and very large ranks all order as integers. Positions are unbounded `int` too.
- **Question:** should the specification bound a rank? An implementation with a
  fixed-width integer refuses or wraps a rank the double accepts.

## N04: the queue's first position is 0 (S011, S014 and S016 are `impl-choice`)

- **Ticket:** PD01.
- **Concerns:** `Queue.next_position` and the `Position` vocabulary entry.
- **The specification says:** every rule that allots a position takes `next_position`
  and advances it, and the invariants keep every position below it. It gives no initial
  value.
- **The double does:** a new queue's `next_position` is 0, so the first pocket takes
  position 0 and its first record position 1.
- **`impl-choice`:** S011, S014 and S016 assert absolute positions, as their Rust tests
  do: S011 that a new queue's `next_position` is 0, then pocket 0, record 1 and
  `next_position` 2; S014 the kept record at position 1 and `next_position` 3; S016
  `next_position` 2. Those values hold only because the double starts at 0. A double
  that started at 10 would satisfy every Publish rule and fail all three. The relative
  facts they also assert, pocket before record, a step of two for a new pocket, the
  kept record's unchanged position and no allotment for a stale publish, are the
  specification's.
- **Question:** is the starting value meant to be unspecified? Only order and uniqueness
  are observable through the rules, so any start satisfies them.

## N05: a lease has an id, which the specification does not declare

- **Ticket:** PD01.
- **Concerns:** the `Lease` entity, and the `lease` that `Ack`, `Nack`, `Renew` and
  `Release` carry.
- **The specification says:** `Lease` has `pocket`, `consumer`, `status`, `granted_at`,
  `expires_at` and `ended_at`. Actions name their lease as an entity, and the
  `ActionsNameTheirLease` guarantee says an action naming an ended lease is rejected
  rather than landing on a successor lease over the same pocket.
- **The double does:** `Lease` has one more field, `id`, drawn from an injected lease-id
  source (`SequentialLeaseIds` in tests: `lease-1`, `lease-2`, and so on). That id is
  how a caller names a lease, so two leases over the same pocket and consumer stay
  distinct.
- **Question:** should the specification declare a lease's identity, since a caller has
  to be able to name one lease apart from a successor with the same pocket and consumer?

## N06: a reference to an entity is that entity's key

- **Ticket:** PD01.
- **Concerns:** `Record.pocket`, `Lease.pocket`, `Lease.consumer`,
  `SupersededRecord.pocket`, `DeadLetter.pocket` and `DeadLetter.nacked_by`.
- **The specification says:** each is a reference to a `Pocket` or a `Consumer`, and
  every lookup is by key, such as `Pocket{key: pocket_key}`.
- **The double does:** each holds the pocket's key or the consumer's id.
  `OnePocketPerKey` and `UniqueConsumerId` make the key an identity in every reachable
  state. In a hand-built snapshot that breaks one of them, the checker reads a reference
  as naming every entity with that key.
- **Question:** none; recorded so the reader knows why `Record.pocket` is a string.

## N07: `a != b` in an invariant compares entities, not their fields

- **Ticket:** PD01.
- **Concerns:** the invariants of the form `for a in X: for b in X: a != b implies …`,
  such as `UniquePublisherId`, `OnePocketPerKey` and `UniqueRecordPosition`; and
  `ArrivalPositionUnoccupied`, which has no `a != b`.
- **The specification says:** `a != b` between two entities of one collection.
- **The double does:** it reads `a != b` as two distinct members of the collection, so
  two publishers with equal fields still count as two and break `UniquePublisherId`.
  `ArrivalPositionUnoccupied` ranges over every pair, a record paired with itself
  included, so a held record whose `deferred_position` equals its own `position` breaks
  it as well as `DeferredArrivedAfterHeld`.
- **Question:** is `ArrivalPositionUnoccupied` meant to include `b = a`? It is
  harmless, since `DeferredArrivedAfterHeld` already rules that state out, but it means
  the two overlap.

## N08: a field declared `when status = …` is the entity's shape, not an invariant

- **Ticket:** PD01.
- **Concerns:** `Record.held_rank`, `held_payload`, `mark`, `nacked_at` and
  `deferred_position`, declared `when status = held`; `Lease.ended_at`, declared
  `when status = released | expired`.
- **The specification says:** `held_rank`, `held_payload` and `mark` are present when
  the record is held, `nacked_at` and `deferred_position` are optional (`?`) then, and
  the rules that queue a record set all five to null. `ended_at` is present once a lease
  has ended.
- **The double does:** constructing a record or a lease that breaks those declarations
  raises `ValueError`, so the checker never meets a held record with no `held_rank` or
  `mark`. The checker compares `nacked_at` and `deferred_position` with null exactly as
  each invariant does.
- **Question:** none; recorded because an invariant such as `HeldVersionNotNewer` would
  otherwise have to say what `null <= rank` means.

## N09: a config that breaks a config invariant is refused when it is made (S005 is `impl-choice`)

- **Ticket:** PD01.
- **Concerns:** `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
  `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`; scenarios S003, S004 and
  S005.
- **The specification says:** the comments above these invariants say "the model is
  defined for" `max_consecutive_expiries >= 1`, `max_pull_batch >= 1` and non-negative
  TTLs. It does not say what happens to a deployment configured outside that range.
- **The double does:**
  - `Config(...)` raises `ConfigRefused`, naming every config invariant the values break,
    so no queue is ever made from such a config. The checker still reads the same five
    invariants from every snapshot.
  - The boundary values, 1 for the two limits and 0 for each TTL, are admitted.
  - Because no queue exists before the refusal, these three refusal tests cannot compare
    a snapshot before and after. Each one shows that the construction raised and named
    the invariant. They are the one exception to PD01's rule that each refusal test
    compares the complete snapshot, which its step 5 applies to them by name.
  - S005 is `impl-choice`. Its owning Rust test is a `compile_fail` doctest: a Rust
    `Duration` cannot be negative, so the refusal happens at compile time. In the
    double, a TTL is an `int`, so it is refused at run time. The double's test refuses
    all three TTLs, where the Rust tags only `lease_ttl`'s doctest with S005.
- **Question:** is "refused at configuration" the intended reading of "the model is
  defined for", or should the specification state the refusal itself?

## N10: a publish by a publisher that never joined is refused (`impl-choice`)

- **Ticket:** PD01.
- **Concerns:** the `Publish(publisher, …)` trigger, the `Publishing` surface's
  `facing publisher: Publisher`, and `PublisherJoins`.
- **The specification says:** `Publish` takes a `Publisher` entity, and only
  `PublisherJoins` creates one. No rule's guard reads the publisher, and nothing says
  what a publish naming an id that never joined does.
- **The double does:**
  - `Queue.publish` takes a publisher id and raises `UnknownPublisher` when no publisher
    with that id has joined.
  - It does so before the outcome starts: the clock is not read, no table is
    consulted, and the snapshot is unchanged.
  - The reading is that such a trigger cannot be formed, since there is no `Publisher`
    to pass it.
  - This is `impl-choice`, because the specification states no such refusal. The Rust
    has the same refusal, `PublishError::UnknownPublisher`, in a test the catalog does
    not list.
- **Question:** should the specification say that a publish requires a joined
  publisher, for example as a `requires` on each Publish rule or as a surface
  guarantee? As written, the guard of every Publish rule ignores the publisher.

## N11: the equal-rank publish is the Publish table's refusal, `EqualRankRefused`

- **Ticket:** PD01.
- **Concerns:** the Publish comment ("No rule covers rank = stored rank, so that publish
  is rejected and changes nothing, whatever its payload"), and S017.
- **The specification says:**
  - five rules share the trigger, with disjoint guards;
  - an equal rank matches none of them, and is "rejected";
  - the rejection has no name.
- **The double does:**
  - The Publish table's answer when no guard holds is a refusal, `EqualRankRefused`,
    carrying the pocket key, the record key and the rank. It changes nothing, and the
    payload is never read.
  - Once all five rows exist, a publish at the stored rank is the only input no guard
    holds for, because the other four guards cover the remaining cases:
    - no pocket: `PublishIntoNewPocket`;
    - no record of that key: `PublishNewKey`;
    - a higher rank: `Supersede` or `DeferBehindHeldVersion`;
    - a lower rank: `DropStalePublish`.
  - Until PD04 lands `DeferBehindHeldVersion`, a higher rank published onto a held record
    with nothing deferred would also reach this refusal. No record can be held before
    PD02, and that input is pending until PD04.
  - A test checks the claimed disjointness over every state PD01 reaches: at most one
    guard holds, and it is the expected rule.
- **Question:** none on the behaviour, which open question 1's closure decided. The
  refusal's name is the double's.

## N12: a join with an id already taken is refused

- **Ticket:** PD01.
- **Concerns:** `PublisherJoins` and `ConsumerJoins`, scenarios S009 and S010.
- **The specification says:** each join `requires: not exists Publisher{id:
  publisher_id}` (or `Consumer`), and says nothing more about a join whose id is taken.
- **The double does:**
  - Each join's table answers a refusal when its one guard does not hold:
    `PublisherAlreadyJoined` and `ConsumerAlreadyJoined`. The refusal changes nothing.
  - A publisher and a consumer may share an id, since each guard reads its own entity
    only.
- **Question:** should a second join with the same id be refused, as here, or succeed
  and change nothing, which the specification's words also allow? The Rust refuses it
  (`JoinError::PublisherExists`), and the catalog rows S009 and S010 say "joins once".
