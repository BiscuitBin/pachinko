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
  position; the rows that assert absolute positions are ported in GD02 and GD03, and
  their notes, G16 and G29, cite this one. GD01's ports of S003, S004 and S005 assert
  that an owner starts over an admitted config and holds no publisher and no consumer,
  and not the position it starts at. The two owner tests, which pin no catalog row,
  compare whole snapshots and so do pin the 0.
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

### G07: a payload is a `BitArray`

- **Ticket:** GD02.
- **Concerns:** `Record.payload`, `Record.held_payload`, `SupersededRecord.payload`,
  `DeadLetter.payload` and the vocabulary entry for Payload.
- **The specification says:** the vocabulary calls a payload "the opaque bytes a record
  carries; pachinko never reads them". The fields are typed `String`, "because Allium 3
  has no byte type", and the "implementation stores bytes".
- **The double does:**
  - Every payload field is a `BitArray`, and so is the payload a `Publication` carries.
  - No guard of the Publish table reads a payload. An effect stores the payload it was
    handed: the publication's in a new record, in a raised record and in a stale entry,
    and the record's old one in the entry `Supersede` sheds. The tests "Supersede changes
    exactly what it ensures and reads the clock once" and "DropStalePublish changes
    exactly what it ensures and reads the clock once" compare the complete snapshot,
    payloads included.
  - The checker reads a payload in one place: `HeldCopiesAgree` compares `held_payload`
    with `payload` for equality. Comparing two snapshots with `==` compares them too.
  - `EqualRankRefused` carries no payload. S017's port publishes the stored rank again
    with another payload, and it is refused.
- **`N`-note:** agrees with N02.
- **Question:** none, unless the maintainer wants the double to follow the `String`
  typing instead.

### G08: a rank is an `Int`, which has no bound

- **Ticket:** GD02.
- **Concerns:** `Record.rank`, `Record.held_rank`, the ranks of `SupersededRecord` and
  `DeadLetter`, and the Publish rules' comparisons `rank > record.rank` and
  `rank < record.rank`; scenario S018.
- **The specification says:** a rank is an `Integer`, and of two ranks the "higher wins".
  It states no bound.
- **The double does:**
  - Every rank is an `Int`: the four fields above, the rank a `Publication` carries and
    the rank `EqualRankRefused` names. An `Int` on the Erlang target has no bound.
  - Positions, `next_position` and `consecutive_expiries` are `Int` too.
  - S018's port, "negative and zero ranks order like integers", publishes one key at
    -5, -1, 0 and then -2. It asserts that the publishes at -1 and 0 fire `Supersede`,
    that the one at -2 fires `DropStalePublish`, and that the record ends at rank 0.
  - No GD02 test publishes a very large rank. S019, the row N17 records for large ranks,
    is ported in GD03 (G30).
- **`N`-note:** agrees with N03.
- **Question:** should the specification bound a rank? An implementation with a
  fixed-width integer refuses or wraps a rank the double accepts.

### G09: a reference to an entity is that entity's key, and may name none or several in a hand-built snapshot

- **Ticket:** GD02.
- **Concerns:** `Record.pocket`, `Lease.pocket`, `Lease.consumer`,
  `SupersededRecord.pocket`, `DeadLetter.pocket` and `DeadLetter.nacked_by`; the derived
  collections `records`, `leases` and `active_leases`; `OutstandingOnlyWhileLeased` and
  `MarkedPocketHasNoExpiries`, which read a field through a reference;
  `OneRecordPerKeyPerPocket` and `OneActiveLeasePerPocket`, which compare two references;
  the lookup `Record{pocket: pocket, key: record_key}` in the Publish rules.
- **The specification says:**
  - Each field is typed as an entity, `pocket: Pocket` or `consumer: Consumer`, and every
    lookup is by key, such as `Pocket{key: pocket_key}`.
  - A collection is defined through the reference, as in
    `records: Record with pocket = this`.
  - Two invariants read a field through one: `r.pocket.status = leased` and
    `r.pocket.consecutive_expiries = 0`. Two compare references: `a.pocket = b.pocket`
    and `a.pocket != b.pocket`.
  - It declares no invariant that a reference names an entity. No rule removes a `Pocket`
    or a `Consumer`: the only `ensures: not exists` are of a record, a superseded record
    and a dead letter.
  - Three Publish rules write `let record = Record{pocket: pocket, key: record_key}`,
    and `PublishNewKey` requires that no such record exists, each after
    `requires: exists pocket`. It does not say what the lookup is where there is no
    pocket.
- **The double does:**
  - Each field holds the pocket's key or the consumer's id, a `String`.
  - Two references are equal when their keys are, whether or not any entity has the key.
  - An entity's collection is every member whose reference holds that entity's key.
  - A field read through a reference is read from every entity that has the key, and
    the check must hold for each.
  - In every state the rules reach, at most one entity has a given key, because
    `OnePocketPerKey` and `UniqueConsumerId` are checked after every outcome. A snapshot
    built by hand can give a key to several entities, or to none.
    - **Several.** A record whose pocket key two pockets have is read against both, and
      each of the two pockets counts the same records and leases.
    - **None.** A field read through the reference asks nothing, so the invariant holds
      of that record. A held, acked record whose pocket key no pocket has breaks neither
      `OutstandingOnlyWhileLeased` nor `MarkedPocketHasNoExpiries`. A lease whose pocket
      or consumer does not exist is counted for no pocket and no consumer. The checker
      reports no dangling reference, since the specification declares no such invariant.
  - Two active leases over one pocket key break `OneActiveLeasePerPocket` even where no
    pocket has the key. That is the only snapshot in which it breaks alone: a pocket with
    two active leases also breaks `LeasedHasOneActiveLease` if it is leased and
    `UnleasedHasNoActiveLease` if it is not.
  - The tests, none of which carries a scenario tag:
    - "a record whose pocket key no pocket has breaks nothing" and "a record whose pocket
      key two pockets have is read against both" pin the two cases in the checker;
    - "a reference that names nothing is in no view" and "a key or an id several entities
      hold gets a view for each" pin them in the derived sets;
    - the control "OneActiveLeasePerPocket: two active leases over a key no pocket has"
      pins the last point.
  - `model.pocket` and `model.record`, the lookups written for the rules, answer the
    first member with the key, which is the only one in every state the rules reach.
  - Dispatch evaluates every guard of the Publish table on every publish, so the guards
    that read the record are evaluated where no pocket has the publication's pocket key.
    The lookup then answers no record, and none of those guards holds. No test publishes
    into a state built by hand in which a record names a pocket that does not exist.
- **`N`-note:** agrees with N06, which holds the key and reads a reference in a
  hand-built snapshot "as naming every entity with that key". N06 does not say that the
  invariant must hold of each such entity, and does not state the case of a reference
  that names none.
- **Question:** N06 leaves none. What does an invariant that reads through a reference
  ask when the reference names nothing? The double reads it as asking nothing. The other
  reading is that a dangling reference breaks every invariant that reads through it. As
  typed in the specification the state cannot arise, so the question is for a checker
  that holds keys.

### G10: a lease has an id, which the specification does not declare

- **Ticket:** GD02.
- **Concerns:** the `Lease` entity, and the `lease` that `Ack`, `Nack`, `Renew` and
  `Release` carry.
- **The specification says:** `Lease` has `pocket`, `consumer`, `status`, `granted_at`,
  `expires_at` and `ended_at`. Actions name their lease as an entity, and the
  `ActionsNameTheirLease` guarantee says one naming an ended lease is rejected "rather
  than landing on a successor lease over the same pocket".
- **The double does:**
  - `Lease` has one more field, `id`, a `String`. It is how a caller names a lease, so
    two leases over the same pocket and consumer stay distinct.
  - GD02 lands the field so that the model and the snapshot are complete.
    `PullClaimsFrontPocket`, the one rule that creates a lease, lands in GD03 and draws
    the id through the request G05 describes (G27). The invariant controls write their
    ids by hand.
  - No invariant reads `id`. The checker tells two leases apart by their place in the
    snapshot's list (G11), not by their ids, and it has no check that two ids differ,
    since the specification declares none.
- **`N`-note:** agrees with N05.
- **Question:** should the specification declare a lease's identity, since a caller has
  to be able to name one lease apart from a successor with the same pocket and consumer?

### G11: `a != b` in an invariant compares members, not their fields

- **Ticket:** GD02.
- **Concerns:** the invariants of the form `for a in X: for b in X: a != b implies …`:
  `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`,
  `OneActiveLeasePerPocket`, `UniquePocketPosition`, `UniqueRecordPosition` and
  `UniqueArrivalPosition`; and `ArrivalPositionUnoccupied`, which has no `a != b`.
- **The specification says:** `a != b` between two entities of one collection. It does
  not say what makes two entities different.
- **The double does:**
  - It reads `a != b` as two distinct members of the collection. A Gleam value has no
    identity apart from its fields, and `==` compares fields, so the checker tells two
    members apart by their place in the snapshot's list. Two publishers with equal fields
    still count as two and break `UniquePublisherId`, as the control "UniquePublisherId:
    two publishers with one id" shows. Compared by value they would be one publisher,
    and nothing would break. The test "a pocket or a record repeated in every field is
    still two" shows the same of a pocket and of a record.
  - `ArrivalPositionUnoccupied` ranges over every pair, a record paired with itself
    included. A held record whose `deferred_position` equals its own `position` breaks it
    as well as `DeferredArrivedAfterHeld`. The control for that snapshot,
    "ArrivalPositionUnoccupied: an arrival position where its own record stands", is
    tagged for both invariants and asserts both names.
- **`N`-note:** agrees with N07.
- **Question:** is `ArrivalPositionUnoccupied` meant to include `b = a`? It is harmless,
  since `DeferredArrivedAfterHeld` already rules that state out, but it means the two
  overlap.

### G12: a publish by a publisher that never joined is refused (`impl-choice`)

- **Ticket:** GD02.
- **Concerns:** the `Publish(publisher, …)` trigger, the `Publishing` surface's
  `facing publisher: Publisher`, and `PublisherJoins`.
- **The specification says:** `Publish` takes a `Publisher` entity, and only
  `PublisherJoins` creates one. No Publish rule's guard reads the publisher, and nothing
  says what a publish naming an id that never joined does.
- **The double does:**
  - A `Publication` carries the publisher's id. The apply step looks for a publisher with
    that id before it runs the Publish table, and answers the refusal `UnknownPublisher`,
    carrying the id, when there is none.
  - The refusal comes before the outcome starts. No table is consulted, no guard is
    evaluated, the clock is not read, the checker is not run, and the owner keeps the
    state it had.
  - The reading is N10's: such a trigger cannot be formed, since there is no `Publisher`
    to pass it.
  - This is `impl-choice`, because the specification states no such refusal. No catalog
    row names it, so its test, "a publish by a publisher that never joined is refused",
    carries no scenario tag. It sends four publishes as `q`, which never joined, each of
    which would fire a different Publish rule had the joined publisher sent it, and one
    as `c1`, which joined as a consumer only. Each is refused. The test then compares the
    complete snapshot with the one taken before and asserts that the fake clock was not
    read.
- **`N`-note:** agrees with N10.
- **Question:** should the specification say that a publish requires a joined publisher,
  for example as a `requires` on each Publish rule or as a surface guarantee? As written,
  the guard of every Publish rule ignores the publisher.

### G13: the equal-rank publish is the Publish table's refusal, `EqualRankRefused`

- **Ticket:** GD02.
- **Concerns:** the Publish comment, and scenario S017.
- **The specification says:** "Five rules share the trigger with disjoint guards." No
  rule covers a rank equal to the stored rank, "so that publish is rejected and changes
  nothing", whatever its payload. The rejection has no name. "The stored rank is
  record.rank whatever the record's state".
- **The double does:**
  - The Publish table's answer when no guard holds is a refusal, `EqualRankRefused`,
    carrying the pocket key, the record key and the rank. The name is N11's. The refusal
    is built from the publication's three fields: it reads neither the payload nor the
    state. The port of S017 notes that the Rust's refusal carries the record key and the
    rank, and that the double's names the pocket as well.
  - A refusal is a table none of whose guards held, and a guard cannot read the clock
    (G05). So the refusal reads no clock, and the owner keeps the state it had.
  - S017's port, "an equal-rank publish is rejected", publishes the stored rank again
    with another payload. It asserts the refusal, compares the complete snapshot with the
    one taken before, and asserts that the fake clock was not read. The tests "Supersede
    changes exactly what it ensures and reads the clock once" and "DropStalePublish
    changes exactly what it ensures and reads the clock once" assert one read each.
  - Dispatch evaluates every guard of the table on every publish and returns
    `GuardOverlap` if two hold, so the claimed disjointness is checked each time and not
    only by a test.
  - The stored rank is `record.rank` whatever the record's status, as the comment says:
    `DropStalePublish`'s guard has no condition on the status. The test "Supersede takes
    a held record only with a version deferred behind it" asserts that a rank below the
    stored rank of a held record reaches `DropStalePublish`, with a version deferred and
    with none, whether the record is unsettled, acked or nacked.
  - GD02 lands four of the five rows: `PublishIntoNewPocket`, `PublishNewKey`,
    `Supersede` and `DropStalePublish`. With those four, no guard holds for two inputs: a
    rank equal to the stored rank, and a higher rank published onto a held record with
    nothing deferred, which is `DeferBehindHeldVersion`'s.
  - That second input is pending, and it meets `EqualRankRefused` until GD05 lands its
    row. Shared rule 4 of the GD tickets says: "An input whose only matching rule is
    still pending is itself pending. No test pins it as a refusal." None does. "Supersede
    takes a held record only with a version deferred behind it" asserts only that
    `Supersede`'s guard does not hold for it. GD03 lands the pull, so a publish through
    the owner can now reach the input: a higher rank of a key that is held. No test
    publishes through the owner onto a held record.
- **`N`-note:** agrees with N11.
- **Question:** none on the behaviour, which the closure of open question 1 decided. The
  refusal's name is the double's.

### G14: a field declared `when status = …` is a field of that variant alone, in the snapshot too

- **Ticket:** GD02.
- **Concerns:** `Record.held_rank`, `held_payload`, `mark`, `nacked_at` and
  `deferred_position`, declared `when status = held`; `Lease.ended_at`, declared
  `when status = released | expired`; the inspection seam, which no surface of the
  specification names.
- **The specification says:** it declares `held_rank: Integer when status = held`, and
  `held_payload` and `mark` in the same way. Two are optional as well:
  `nacked_at: Timestamp? when status = held` and
  `deferred_position: Integer? when status = held`. The rules that queue a record write
  null to all five, as in `ensures: record.held_rank = null`. A lease has
  `ended_at: Timestamp when status = released | expired`.
- **The double does:**
  - Gleam's types made a reading unnecessary. A record's status is `RecordQueued` or
    `RecordHeld(held_rank, held_payload, mark, nacked_at, deferred_position)`, so the
    five fields are fields of the held variant alone. A lease's status is `LeaseActive`,
    `LeaseReleased(ended_at)` or `LeaseExpired(ended_at)`, so `ended_at` is a field of
    the two ended variants alone.
  - A queued record with a `held_rank`, a held record with no `mark`, an active lease
    with an `ended_at` and an ended lease with none cannot be written. Nothing refuses
    them at run time, because no such value exists.
  - In the checker `status = held` is the match on `RecordHeld`, and a queued record has
    no held field to compare. `EndedAfterGrant` matches the two ended variants.
  - Two fields are still an `Option` inside the held variant: `nacked_at` and
    `deferred_position`. The specification types them `Timestamp?` and `Integer?`, so
    each may be null while the record is held, and four invariants say when:
    `NackedHasItsTime`, `OnlyNackedHasNackTime`, `DeferredHasArrivalPosition` and
    `OnlyDeferredHasArrivalPosition`. `= null` is `None` and `!= null` is `Some`.
  - `mark` is not an `Option`: a held record is `Unsettled`, `Acked` or `Nacked`.
    `nacked_at` stands beside `mark` and not inside `Nacked`, as the specification
    declares it. The controls "NackedHasItsTime: a nacked record with no nack time" and
    "OnlyNackedHasNackTime: a nack time on a record not nacked" build those two records.
  - **The snapshot departs from shared rule 4 of the GD tickets.** That rule says the
    snapshot holds "the records in the specification's flat field names". The snapshot
    holds the model's `Record` as it is. The five held fields keep the specification's
    names, and a reader reaches them through `record.status`, on the held variant, not
    on the record itself. GD02's first review checkpoint raised the difference and the
    variant was kept, for two reasons: a flat record needs five `Option`s, and it admits
    the states the declaration excludes, such as a queued record with a `held_rank`.
- **`N`-note:** agrees with N08's reading, that the declaration is the entity's shape and
  not an invariant. It departs in the means. N08 refuses at construction a record or a
  lease that breaks the declaration; here the type cannot hold one.
- **Question:** N08 leaves none on the reading; it is recorded because an invariant such
  as `HeldVersionNotNewer` would otherwise have to say what `null <= rank` means. The
  snapshot's shape leaves one. Does the maintainer accept a snapshot whose record carries
  the held fields on its held variant, or should the snapshot show the flat record that
  shared rule 4 words, with the five fields optional?

### G15: a publish answers nothing (the maintainer's ruling on N32)

- **Ticket:** GD02.
- **Concerns:** `PublishIntoNewPocket`, `PublishNewKey`, `Supersede` and
  `DropStalePublish`; the `Publishing` surface; scenarios S011, S013, S014, S016, S017
  and S018.
- **The specification says:** the Publish rules ensure state, and three of the five emit
  `PocketTouched`. No rule answers the publisher, and the surface names no outcome.
- **The double does:**
  - It follows a ruling, not a reading of its own. On 2026-11-19 the maintainer ruled on
    N32 that a publish answers nothing, as the specification states, and that a ported
    row asserts that answer and does not port the Rust's outcome.
  - A rule's answer is an `Option(Answer)`. `Answer` had no constructor in GD02. GD03
    gives it two, `Batch` and `NoPocketAvailable`, and only a Pull rule answers either
    (G22), so the answer of every Publish rule is still `None`.
  - The owner's reply to a publish that succeeded is `Ok(Outcome(fired, None))`. `fired`
    is the names of the rules that fired, in order. A test returns it to the runner, and
    the coverage check reads it there. It is the double's report and not an answer of
    the specification's. Through it a caller of the owner can tell which Publish rule its
    publish reached.
  - Every publish that succeeds in the six ports, a setup publish included, is asserted
    to reply exactly that: the one rule it fired, and no answer.
  - Where the Rust checks a publish for an outcome, the port also asserts, through the
    snapshot, the state that publish leaves. S018's port does so after each of its three
    such publishes: the version the record stores, and the one version that publish
    shed, with its reason. S013's port asserts both of its publishes after the second, as
    the Rust does. A setup publish the Rust only unwraps is asserted by its reply alone.
  - `fired` does not show an emitted `PocketTouched`. Its table has no row until GD04,
    so the event changes neither the rules that fired nor the snapshot. The test
    "PublishNewKey and Supersede touch the pocket, and the other two emit nothing" pins
    the events by walking each row's plan (G20).
- **`N`-note:** agrees with N32 and follows its ruling. The owner's `fired` has no
  `N`-note.
- **Question:** N32's, which the ruling leaves open. N32 records that decision 0020 fixes
  the Rust's answer, and the specification does not state it. Should the `Publishing`
  surface name what a publish answers, so that a double or a second implementation can
  port it?

### G16: S011, S014 and S016 assert absolute positions (`impl-choice`)

- **Ticket:** GD02.
- **Concerns:** `Queue.next_position`, G04, and scenarios S011, S014 and S016.
- **The specification says:** no initial value for `next_position` (G04).
  `PublishIntoNewPocket` takes two positions,
  `let pocket_position = queue.next_position` and
  `let record_position = queue.next_position + 1`, and ensures
  `queue.next_position = record_position + 1`. The vocabulary says "A supersede keeps a
  record's position". `DropStalePublish` ensures a `SupersededRecord` and nothing else.
- **The double does:** it ports the Rust tests' absolute positions, which hold only
  because the double starts at 0 (G04).
  - The absolute assertions, as N04 lists them, each marked `impl-choice` in its port:
    - S011: a new queue's `next_position` is 0, then the pocket at 0, the record at 1 and
      `next_position` 2;
    - S014: the kept record at position 1, and `next_position` 3;
    - S016: `next_position` 2.
  - The relative facts these rows also assert are the specification's: the pocket before
    its record, a step of two for a new pocket, the kept record's unchanged position, and
    no allotment for a stale publish.
  - A double that started at 10 would satisfy every Publish rule and fail all three.
  - The double's own tests of the four rules, the ones named "… changes exactly what it
    ensures …", assert every position against the counter's value before the publish and
    never as an absolute number.
  - Two more facts about these ports, neither about a position. S014's port stands in
    for the Rust's `invariants::shed_before_expiry` by asserting that `invariants.check`
    does not name `ShedBeforeExpiry` for the snapshot. S016's port asserts the shed
    entry's rank, reason, payload and two timestamps, and not its pocket or its key.
- **`N`-note:** agrees with N04.
- **Question:** as G04. Is the starting value meant to be unspecified? If so, should
  S011, S014 and S016 assert absolute positions?

### G17: `OneActiveLeasePerConsumer` ranges over consumers, so a lease that names no consumer breaks nothing

- **Ticket:** GD02.
- **Concerns:** `OneActiveLeasePerConsumer`, `Consumer.active_leases`, and the
  `Consuming` surface's comment on its `leases`.
- **The specification says:**
  - The invariant is `for c in Consumers:` `c.active_leases.count <= 1`, where
    `leases: Lease with consumer = this` and
    `active_leases: leases where status = active`.
  - `OneActiveLeasePerPocket` is written the other way, over pairs of `Leases`.
  - The `Consuming` surface relies on it: "Zero or one lease by the invariant; iterating
    avoids a null read."
- **The double does:**
  - The check is taken as written: for each consumer in the snapshot, it counts the
    active leases whose `consumer` holds that consumer's id. The control is
    "OneActiveLeasePerConsumer: one consumer with two active leases".
  - A reference is a key (G09), so a hand-built snapshot can hold two active leases that
    name an id no consumer has. No consumer counts them, and the invariant holds. No
    test builds that snapshot for the checker; "a reference that names nothing is in no
    view" shows one such lease in no consumer's view. Two active leases over a pocket key
    no pocket has do break `OneActiveLeasePerPocket`, because that invariant ranges over
    the leases.
  - A snapshot with two consumers of one id breaks `UniqueConsumerId`, and each of the
    two counts every lease that names the id. The control "UniqueConsumerId: two
    consumers with one id" has one active lease naming the id, and breaks nothing else.
  - Only `PullClaimsFrontPocket` creates a lease, and no rule removes a `Consumer`. That
    rule lands in GD03, and a pull by a consumer that never joined is refused before any
    rule runs (G26). So no pull through the owner creates a lease that names no consumer.
- **`N`-note:** no `N`-note.
- **Question:** is the invariant meant to bound the leases or the consumers? Written over
  pairs of `Leases`, as `OneActiveLeasePerPocket` is, it would not depend on the consumer
  being there. As typed in the specification a lease's `consumer` is a `Consumer`, so the
  two forms differ only for a checker that holds keys.

### G18: positions are unique within a kind, not across kinds

- **Ticket:** GD02.
- **Concerns:** the Queue comment, the `Position` vocabulary entry,
  `UniquePocketPosition`, `UniqueRecordPosition`, `ArrivalPositionUnoccupied` and
  `UniqueArrivalPosition`.
- **The specification says:**
  - The Queue comment says every rule that allots a position takes `next_position` and
    advances it, "so positions are unique".
  - The invariants compare within a kind. `UniquePocketPosition` compares a pocket with a
    pocket and `UniqueRecordPosition` a record with a record. `ArrivalPositionUnoccupied`
    compares an arrival position with a record's position, and `UniqueArrivalPosition`
    one arrival position with another.
  - No invariant compares a pocket's position with a record's position or with an
    arrival position.
- **The double does:**
  - The checker holds the four invariants as written and adds none. A snapshot in which a
    record's position, or an arrival position, equals a pocket's position breaks nothing:
    the tests "RecordPositionsBelowNext: a record one below the next position breaks
    nothing" and "ArrivalPositionsBelowNext: an arrival one below the next position
    breaks nothing" each put one at the idle pocket's position.
  - `UniqueRecordPosition` ranges over every record of the queue, not over one pocket's:
    two records at one position in two pockets break it, as the control
    "UniqueRecordPosition: two records at one position, in two pockets" shows.
  - Every position a rule is handed comes from one counter. The interpreter in
    `rules.gleam`, which the apply step runs, hands the counter's value to the row that
    asks and advances it by one, so no two requests are handed the same value.
- **`N`-note:** no `N`-note.
- **Question:** is "positions are unique" meant across pockets, records and arrival
  positions together? If so, an invariant is missing. If it is meant within each order
  only, should the Queue comment say so?

### G19: the snapshot lists pockets, records, leases and the two shed queues in the order they were created (`impl-choice` where a test asserts it)

- **Ticket:** GD02.
- **Concerns:** the `Pocket`, `Record`, `Lease`, `SupersededRecord` and `DeadLetter`
  entities; the Pocket and Record comments; the derived collections of `Pocket` and
  `Consumer`; the inspection seam, which no surface of the specification names.
- **The specification says:**
  - It orders two things, both by position: "Queued pockets are ordered by position,
    smallest first.", and that order is "the pocket order; nothing else positions a
    pocket." A pocket's queued records are ordered by position too.
  - It gives no order to the pockets as a collection, to all the records, to the leases
    or to either shed queue. Who reads those two queues is open: "Reading the superseded
    and dead-letter queues: open question."
- **The double does:**
  - This extends G06 from the publishers and the consumers to every other collection. The
    state keeps each kind in a list, in the order its members were created, and a member
    that is changed keeps its place. The snapshot shows the lists as they are.
  - `PublishIntoNewPocket` adds its pocket at the end of the pockets and its record at
    the end of the records, and `PublishNewKey` its record at the end of the records.
    `Supersede` changes the record in its place. `Supersede` and `DropStalePublish` add
    their entry at the end of the superseded queue, so that queue's newest entry is last.
  - No rule reads that order. The pocket order and a pocket's record order are the
    positions'. The checker's answer does not depend on it either: every invariant ranges
    over all members or all pairs. The checker does use a member's place in its list, to
    tell two members apart (G11).
  - The derived sets are computed once for each snapshot, by `inspection.views`: one
    view for each pocket and each consumer, each set in the snapshot's order. The test "a
    pocket's and a consumer's derived sets, in the snapshot's order" asserts the whole
    value.
  - A stated order is what lets two snapshots compare with `==`, as every test that shows
    a refusal changing nothing does. No collection is a `Dict` in iteration order.
  - No row ported in GD02 asserts this order. S018's port reads the superseded queue,
    which holds three entries by its end, by rank and by count. No other GD02 port
    asserts a list with more than one member. S011's port asserts the pocket order, which
    it reads from the positions. Two rows ported in GD03 do assert it: the ports of S015
    and S019 each assert the superseded queue's entries in order, marked `impl-choice`.
  - The double's own tests of the four rules, the ones named "… changes exactly what it
    ensures …", compare the complete snapshot and so do assert it: a new pocket, a new
    record and a new shed entry at the end of its list, and a superseded record in its
    place. Those assertions are `impl-choice`. They hold because of this order, not
    because of anything the specification states.
  - No GD02 rule creates a lease or a dead letter. Their order is the one stated here,
    for the tickets that land those rules to keep. GD03 keeps it for the leases:
    `PullClaimsFrontPocket` adds its lease at the end of the leases, and `PullWithinLease`
    and `Renew` change a lease in its place. The test "PullClaimsFrontPocket changes
    exactly what it ensures, reads the clock once and draws one lease id" and the three
    competing-callers tests (G32) assert that order, which is `impl-choice`.
- **`N`-note:** no `N`-note. The rulings on N31 and N38 treat the Rust's record order and
  its stored pocket order as its inspection shape, and their rows are ported in GD05 and
  GD06. PD07, which asks whether the inspection shape is part of the contract, is still
  todo.
- **Question:** are these collections meant to have no order beyond the positions'? If
  PD07 makes an inspection order part of the contract, which order is it for each?

### G20: `PublishIntoNewPocket` emits no `PocketTouched`

- **Ticket:** GD02.
- **Concerns:** `PublishIntoNewPocket`, and the comment above `PublishNewKey`.
- **The specification says:** "A publish into an existing pocket that adds or raises a
  record emits PocketTouched: PublishNewKey, Supersede and DeferBehindHeldVersion." Each
  of those three rules has an `ensures: PocketTouched(pocket_key: pocket_key)`.
  `PublishIntoNewPocket` has none, and neither has `DropStalePublish`: "A stale publish
  changes no record and does not touch the pocket". The two revive rules answer the
  event "only when the pocket is idle or parked".
- **The double does:**
  - It follows the rules. The rows of `PublishNewKey` and `Supersede` finish with the
    event, and the rows of `PublishIntoNewPocket` and `DropStalePublish` with none. The
    test "PublishNewKey and Supersede touch the pocket, and the other two emit nothing"
    asserts the events of each row.
  - The comment and the rules agree: the comment says "into an existing pocket". GD02's
    ticket summarised it as "A publish that changes a record emits it", which is wider:
    a publish into a new pocket creates a record. The double takes the rule, not the
    summary.
  - The two readings cannot be told apart through the rules. A new pocket is queued, and
    the revive rules answer the event only for an idle or a parked pocket.
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour. Should the comment say outright that a publish
  into a new pocket emits nothing, since a reader who stops at "adds or raises a record"
  reads it wider?

### G21: a rule changes only what its `ensures` names

- **Ticket:** GD02.
- **Concerns:** the `ensures` of every rule, and `Supersede`'s first; the `Position`
  vocabulary entry, the Record comment and the `OldestSlotNewestPayload` guarantee.
- **The specification says:**
  - `Supersede` ensures `record.rank = rank` and `record.payload = payload`, and names
    no other field of the record.
  - Its comments say in words what stays. Of the position: "A supersede keeps a record's
    position", and "A supersede changes rank and payload and leaves position alone." Of
    the arrival position: "A deferred version's arrival position stays with the record,
    as a queued record's position does." Of the held fields: "While a record is held,
    held_rank and held_payload are the version the consumer was handed and mark is what
    the consumer has said of it."
  - `OldestSlotNewestPayload` says of a key no consumer has acked or nacked that "only
    its rank and payload change".
  - It does not say, as a rule for every `ensures`, that a field no `ensures` names is
    unchanged.
- **The double does:**
  - It reads every rule so: an effect builds the new state from the old, replacing only
    what an `ensures` names. `Supersede` on a held record keeps its position, its held
    rank and payload, its mark, its nack time and its arrival position, which is also
    what the comments above say of each.
  - The test "Supersede sheds the version deferred behind a held one" asserts the whole
    state such a supersede leaves, once for each of the three marks. The four tests
    named "… changes exactly what it ensures …" compare the complete snapshot after one
    publish with the one before, changed as the rule's `ensures` lines say and in no
    other way.
- **`N`-note:** no `N`-note.
- **Question:** is a field that no `ensures` names unchanged, for every rule? For
  `Supersede` the comments say so field by field. The specification states no general
  rule, and the double reads every rule this way.

### G22: a batch names its lease by id and its pocket by key, and carries no expiry

- **Ticket:** GD03.
- **Concerns:** the `Batch` answer of `PullWithinLease` and `PullClaimsFrontPocket`, the
  Batch vocabulary entry, the Pull comment and the `Renew` rule; scenarios S027, S036 and
  S050.
- **The specification says:**
  - Both rules end with `Batch`, as in
    `ensures: Batch(consumer: consumer, lease: lease, pocket: pocket, records: batch)`,
    where `batch` is `oldest(pocket.queued_records, …)`. The vocabulary calls a batch
    "what a pull hands back, the lease, the pocket key and zero or more records".
  - `PullClaimsFrontPocket` names the lease as
    `Lease{pocket: pocket, consumer: consumer, status: active}`. The Pull comment says:
    "Batch names the lease the records were handed under; PullClaimsFrontPocket creates
    that lease in the same rule, so its emission looks the lease up, which reads the
    resulting state."
  - "A pull emits Handed for each record of the batch and HoldRecord moves that record to
    held; the whole pull is one outcome, the chain is how a per-record transition is
    written."
  - `Renew` ensures `lease.expires_at = now + config.lease_ttl` and nothing else. It
    answers nothing.
- **The double does:**
  - `Batch` has four fields: the consumer's id, the lease's id (G10), the pocket's key
    and `records`, a list of `BatchRecord(key, rank, payload)`, smallest position first.
  - A `BatchRecord` is built from the record as the row read it, before any `Handed` is
    answered, so it carries the rank and payload the record had then. `HoldRecord` copies
    the same two into `held_rank` and `held_payload` within the same outcome. A
    `BatchRecord` carries neither the record's position nor its status.
  - `PullWithinLease` names the consumer's active lease. `PullClaimsFrontPocket` names
    the lease it has just created, with no lookup. In the resulting state that is the one
    lease the specification's lookup can find, by `OneActiveLeasePerPocket`.
  - Neither `Batch` nor the reply to a renew carries an expiry. A renew that succeeds
    replies `Ok(Outcome(["Renew"], None))`. N13 records that the Rust's batch and renew
    both answer the lease's `expires_at`; the ports read it from the snapshot's lease
    instead: S027's "a pull claims the front pocket, leases it and holds the batch",
    S036's "a pull under a lease extends the lease" and S050's "a renew extends the lease
    to now plus lease_ttl".
  - A batch is an answer, not state, so a test asserts it through the owner's reply. The
    lease's `granted_at` and `expires_at` and the held fields of each handed record are
    asserted through the snapshot. "PullClaimsFrontPocket changes exactly what it
    ensures, reads the clock once and draws one lease id" asserts both for one pull.
  - The apply step answers each `Handed` in the batch's order and runs the checker once,
    after the last. No test shows that the checker waits: no state between a pull's own
    row and its `HoldRecord`s breaks an invariant, so a check part-way would pass too.
- **`N`-note:** agrees with N13. It differs in one detail: N13's `records` are the
  `Record` entities the rule read, which still carry their queued status, and a
  `BatchRecord` holds the key, the rank and the payload alone.
- **Question:** N13's. Should `Batch` carry the lease's `expires_at`? A consumer needs it
  to know when to renew, and the `Consuming` surface exposes it only through the lease.
  This double adds one: which fields of a record does a batch hand over? The rules say
  `records: batch`, and the surface exposes a held record's key, `held_rank`,
  `held_payload` and `mark`.

### G23: the Pull table's refusal is unreachable through the owner

- **Ticket:** GD03.
- **Concerns:** `PullWithinLease`'s `requires: pocket.status = leased`,
  `PullClaimsFrontPocket`'s `requires: pocket.status = queued`, `PullFindsNoPocket`, and
  the invariant `UnleasedHasNoActiveLease`.
- **The specification says:**
  - `PullWithinLease` has `let lease = Lease{consumer: consumer, status: active}`,
    `requires: exists lease`, `let pocket = lease.pocket` and
    `requires: pocket.status = leased`.
  - The other two rules have
    `requires: not exists Lease{consumer: consumer, status: active}`. With
    `let free = Pockets where status = queued`, one requires `free.count > 0` and the
    other `free.count = 0`.
  - `PullClaimsFrontPocket` also has `let pocket = min_by(free, p => p.position)` and
    `requires: pocket.status = queued`.
  - `UnleasedHasNoActiveLease` is `status != leased implies active_leases.count = 0`.
  - It does not say what a pull that no rule covers does. For a publish and for a mark it
    says that an input no rule covers "is rejected".
- **The double does:**
  - The Pull table has the three rows, and its answer when no guard holds is a refusal,
    `PullRefused`, carrying the consumer's id. The name is N14's.
  - In a state that holds `UnleasedHasNoActiveLease`, an active lease's pocket is leased,
    so `PullWithinLease`'s second `requires` cannot fail once its first holds.
    `PullClaimsFrontPocket`'s `requires: pocket.status = queued` cannot fail alone
    either, since `pocket` is chosen from the queued pockets. So one of the three guards
    holds in every state the rules reach, and no pull through the owner meets
    `PullRefused`.
  - Dispatch evaluates every guard of the table on every pull and returns `GuardOverlap`
    if two hold (G13), so the three guards are shown disjoint on each pull and not by
    one test.
  - The test "no Pull rule holds for an active lease on a pocket that is not leased" is
    `impl-choice` and carries no scenario tag. It builds by hand three states that break
    the invariant: an active lease whose pocket is queued, idle or parked. In each,
    dispatch finds no row, and the apply step answers `PullRefused` at once: it asks for
    no clock and no lease id and hands back no state. The test runs the apply step, not
    the owner.
  - `PullWithinLease`'s guard also fails where the lease's pocket key names no pocket
    (G09), and the pull is then refused in the same way. No test pulls over such a
    state.
  - A lease that has ended is not the consumer's active lease. "a consumer whose lease
    has ended pulls as one with no lease" builds by hand a consumer with a released or an
    expired lease: it claims the front pocket when one is queued, and is told
    `NoPocketAvailable` when none is.
- **`N`-note:** agrees with N14. N14's hand-built state is an active lease on a queued
  pocket; this double's test covers an idle and a parked pocket as well.
- **Question:** N14's. Are the two `requires` meant as guards, or as restatements of what
  the invariants already give? If restatements, the specification could say so, or drop
  them.

### G24: a lookup that names one lease, and `min_by` over a tie, take the first in the snapshot's order

- **Ticket:** GD03.
- **Concerns:** `let lease = Lease{consumer: consumer, status: active}` in
  `PullWithinLease`; `min_by(free, p => p.position)` in `PullClaimsFrontPocket`;
  `oldest`; `OneActiveLeasePerConsumer`, `UniquePocketPosition` and
  `UniqueRecordPosition`.
- **The specification says:** the lookup names one lease. "min_by and min are black
  boxes: the element with the smallest key, and the smaller of two integers." `oldest`
  is "the n records with the smallest position, in that order". It does not say which
  lease the lookup names when a consumer has two active leases, or which element
  `min_by` and `oldest` put first when two have the same position.
- **The double does:**
  - The lookup answers the first active lease of the consumer in the state's list of
    leases, which is the order they were created in (G19).
  - `min_by` and `oldest` sort by position with the standard library's sort, which is
    stable, and take from the front. Of two pockets or two records at one position, the
    one listed first comes first.
  - This extends G09, where `model.pocket` and `model.record` answer the first member
    with the key. `model.lease`, which finds the lease an action names, answers the first
    lease with the id (G27).
  - For the consumer's active lease, `min_by` and `oldest` there is nothing to choose
    between in any state the rules reach: `OneActiveLeasePerConsumer`,
    `UniquePocketPosition` and `UniqueRecordPosition` are checked after every outcome.
    Only a state built by hand holds two.
  - That does not cover `model.lease`. No invariant compares two lease ids, and the
    double does not check that the id it is handed is new, so a lease-id source that
    repeated an id would leave two leases of one id in a state the rules reach (G27).
  - No test runs a Pull rule over such a state. "a claim takes the queued pocket with
    the smallest position, and a batch the queued records with the smallest" runs the
    apply step over a hand-built state whose lists are not in position order, with no
    two positions equal, and shows that the positions decide and the list order does
    not.
- **`N`-note:** no `N`-note.
- **Question:** none for the consumer's active lease, `min_by` or `oldest` in a state
  the rules reach. The three invariants make each of them name one entity, so the choice
  matters only to a checker or a test that builds a state by hand. For a lease named by
  its id, G27 asks whether two leases may share one.

### G25: a pull whose limit is not positive still claims a pocket

- **Ticket:** GD03.
- **Concerns:** `PullClaimsFrontPocket`, `PullWithinLease`, the `OldestFirst` guarantee
  and the Pull comment; scenarios S031 and S032.
- **The specification says:**
  - `PullClaimsFrontPocket` has no `requires` on `limit`. `oldest` returns "all of them
    when n exceeds their count and none when n is not positive".
  - `OldestFirst` says "a limit that is not positive yields an empty batch, not an
    error".
  - The Pull comment says "PullWithinLease extends the lease whatever the batch holds, an
    empty batch included", and "No lease is granted on an empty pocket".
  - The catalog row S031 is "A zero-limit pull claims a pocket and returns an empty
    batch".
- **The double does:**
  - A pull with a limit of 0 or below, by a consumer with no lease, fires
    `PullClaimsFrontPocket` alone. It creates a lease with
    `expires_at = now + lease_ttl`, leases the front pocket and answers a batch with no
    record. No `Handed` is emitted, so the pocket's records stay queued.
  - S031's port, "a zero-limit pull claims a pocket and returns an empty batch", asserts
    the empty batch, the leased pocket, one lease and a record still queued. S032's port,
    "a negative limit behaves like zero", claims with a limit of -1, pulls within the
    lease with the lowest 64-bit integer and is handed nothing, then pulls with 10 and is
    handed both records.
  - The pocket leaves the pocket order for the lease's life, though it holds queued
    records. In "PullClaimsFrontPocket changes exactly what it ensures, reads the clock
    once and draws one lease id", `c1` claims `y` with a limit of 0. Only the pocket's
    status and the new lease change, and the next consumer to pull claims `z`, the
    pocket behind `y`.
  - Under a lease, a limit that is not positive extends the lease and hands nothing, as
    the comment states. "PullWithinLease changes exactly what it ensures, on an empty
    batch too, reads the clock once and draws no lease id" pins it with limits of 0 and
    -1 and with a drained pocket: only the lease's `expires_at` changes.
- **`N`-note:** agrees with N19.
- **Question:** N19's. Is a claim that hands nothing intended? The comment above
  `PullWithinLease` explains why an empty batch extends a held lease, but not why a limit
  that is not positive claims a new one. A consumer probing with `limit = 0` takes a
  pocket away from every other consumer until it releases it or the lease lapses.

### G26: a pull or a renew by a consumer that never joined is refused (`impl-choice`)

- **Ticket:** GD03.
- **Concerns:** the `Pull(consumer, limit)` and `Renew(consumer, lease)` triggers, the
  `Consuming` surface's `facing consumer: Consumer`, and `ConsumerJoins`.
- **The specification says:** both triggers take a `Consumer` entity, and only
  `ConsumerJoins` creates one. The Pull rules read the consumer only to look for its
  active lease, and `Renew` only in `requires: lease.consumer = consumer`. Nothing says
  what a pull or a renew naming an id that never joined does.
- **The double does:**
  - A pull and a renew carry the consumer's id. The apply step looks for a consumer with
    that id before it runs the Pull or the Renew table, and answers the refusal
    `UnknownConsumer`, carrying the id, when there is none. The name is N18's.
  - The refusal comes before the outcome starts, as `UnknownPublisher` does (G12). No
    table is consulted, no guard is evaluated, the clock is not read, no lease id is
    drawn, the checker is not run, and the owner keeps the state it had.
  - Without the check, such a pull would fire `PullClaimsFrontPocket` or
    `PullFindsNoPocket`, since neither guard needs the consumer to exist. A claim would
    create a lease that names a consumer the state does not hold, and the checker would
    not report it (G09, G17). Such a renew would meet the Renew table's refusal instead
    (G28).
  - This is `impl-choice`, because the specification states no such refusal. No catalog
    row names it, so the two tests carry no scenario tag.
    - "a pull by a consumer that never joined is refused" pulls as `cx`, which never
      joined, over an empty queue and over a queued pocket, with limits of 10 and 0, and
      as `p`, which joined as a publisher only.
    - "a renew by a consumer that never joined is refused" renews as `cx` and as `p`,
      naming another consumer's active lease, and as `cx` naming a lease id never
      issued. That last renew is answered `UnknownConsumer`: the consumer is looked for
      before the lease.
    - Each test compares the complete snapshot with the one taken before, and asserts
      that the fake clock was not read and that no lease id was drawn.
- **`N`-note:** agrees with N18, but not with its reason that "the snapshot refuses" such
  a lease "as a dangling reference": this double's checker reports no dangling reference
  (G09).
- **Question:** N18's. Should the specification say that a pull and a renew require a
  joined consumer, as G12 asks for a publish?

### G27: a pull draws a lease id only when it creates a lease

- **Ticket:** GD03.
- **Concerns:** `PullClaimsFrontPocket`'s `Lease.created`, and G10's lease id; scenario
  S027.
- **The specification says:** `Lease` declares no id (G10). `PullClaimsFrontPocket` is
  the one rule with `ensures: Lease.created(…)`.
- **The double does:**
  - `PullClaimsFrontPocket`'s effect asks for a lease id through the request G05
    describes, after it has asked for `now`. An effect runs only once dispatch has chosen
    its row, so only a pull that creates a lease draws an id. No other row asks for one.
  - The tests count the fake source's draws.
    - "PullClaimsFrontPocket changes exactly what it ensures, reads the clock once and
      draws one lease id" counts one for each of two claims.
    - "PullWithinLease changes exactly what it ensures, on an empty batch too, reads the
      clock once and draws no lease id", "PullFindsNoPocket changes nothing, reads no
      clock and draws no lease id" and "Renew changes exactly what it ensures, reads the
      clock once and draws no lease id" count none.
    - The two tests of a consumer that never joined (G26) count none.
    - Each competing-callers test (G32) counts one draw for each lease granted.
  - A pull the Pull table refuses draws none: "no Pull rule holds for an active lease on
    a pocket that is not leased" shows the apply step done before it asks for anything
    (G23).
  - After the claim only `HoldRecord` and the checker run. A pull that has drawn an id
    can therefore fail only on a defect or a broken invariant, never on a refusal. The
    owner has called the source by then, and the id is not given back.
  - The double does not check that an id it is handed is new, and no invariant compares
    two ids (G10). A source that repeated an id would give two leases one id. Of two
    such leases `model.lease` answers the first in the snapshot's order (G24), and
    `model.replace_lease` replaces both. The fake hands out `lease-1`, `lease-2` and so
    on, and never repeats, so no test reaches that state.
  - S027's port asserts that the batch names `lease-1`. That is `impl-choice`: the id is
    the fake source's first, and the port's comment says the Rust's `1` is its counting
    source's. No other test that drives the owner asserts a literal id: each compares a
    lease id only with another the owner handed it.
- **`N`-note:** agrees with N15. It differs in one assertion: N15 says none of its tests
  asserts a literal id, and S027's port here does.
- **Question:** N15 leaves none, "unless the maintainer wants lease identity in the
  specification". As G10: should the specification declare a lease's identity? If it
  does, should it also say that two leases never share one?

### G28: Renew's refusal is `LeaseFenced`, and an unknown lease id meets it

- **Ticket:** GD03.
- **Concerns:** the `Renew` rule, the comment above `AckMarksRecord` and the
  `ActionsNameTheirLease` guarantee; scenario S050.
- **The specification says:**
  - `Renew` requires `lease.status = active` and `lease.consumer = consumer`. One rule
    has the trigger.
  - "Ack, nack, renew and release name the lease they act under and require it to be
    active and the consumer's. An action meant for a lease that has ended is rejected".
    `ActionsNameTheirLease` says "one naming an ended lease is rejected rather than
    landing on a successor lease over the same pocket".
  - Neither the rule, the comment nor the guarantee names the rejection. The trigger
    takes a `Lease` entity, so nothing says what a renew naming a lease that was never
    issued does.
- **The double does:**
  - The Renew table has one row. Its guard holds when a lease has the id the renew
    carries, is active and names the renew's consumer.
  - The table's answer when the guard does not hold is a refusal, `LeaseFenced`,
    carrying the action's name, `"Renew"`, the consumer's id and the lease's id. The name
    is N20's, and GD04's fencing can use it for ack, nack and release.
  - A lease id that names no lease fails the same guard and meets the same refusal. It
    is not refused before the outcome, as a consumer that never joined is (G26). N20's
    reason is taken: a lease id, unlike a consumer id, is the double's own (G10).
  - "Renew's guard holds only for an active lease of the consumer's" runs dispatch over
    states built by hand. It finds the row for the consumer's active lease, and none for
    that lease once released or expired, for another consumer's active lease, or for an
    id that names no lease. It asserts only what dispatch finds, not what the table then
    answers.
  - No GD03 test pins `LeaseFenced`. Its catalog rows, S052 and S070, are GD04's. The one
    GD03 test that renews with a lease id never issued sends it as a consumer that never
    joined, and is answered `UnknownConsumer` (G26).
  - The rule sets `expires_at` and nothing else (G21). "Renew changes exactly what it
    ensures, reads the clock once and draws no lease id" compares the complete snapshot
    after each of three renews, and S050's port asserts that `granted_at` stays. "Renew
    leaves a retained version and a dead letter as they were" runs the rule over a state
    built by hand that holds one of each, and compares the complete state.
- **`N`-note:** agrees with N20. It differs in what is tested: N20 records a test that
  pins `LeaseFenced` for a lease id never issued, and no GD03 test pins the refusal for
  any input.
- **Question:** N20 leaves none on the behaviour. The refusal's name is the double's.
  Whether a lease that was never issued is a refusal of its own is left to GD04.

### G29: S012, S015 and S022 assert absolute positions (`impl-choice`)

- **Ticket:** GD03.
- **Concerns:** `Queue.next_position`, G04 and G16, and scenarios S012, S015 and S022.
- **The specification says:** no initial value for `next_position` (G04).
- **The double does:** it ports the Rust tests' absolute positions, which hold only
  because the double starts at 0 (G04). GD03 owns the three rows because each ends with
  a pull.
  - The absolute assertions, as N16 lists them, each marked `impl-choice` in its port:
    - S012, "a publish of a new key appends to an existing pocket": the pocket at 0, the
      new key's record at 2, and `next_position` 3;
    - S015, "repeated supersede keeps the first slot and sheds every loser": the
      superseded key's record at position 1;
    - S022, "publishing into a queued pocket does not move it in the pocket order": the
      pocket at 0.
  - The relative facts these rows also assert are the specification's: a new key takes
    the next position, a supersede keeps the record's position, and a publish into a
    queued pocket leaves the pocket's position alone.
  - The pulls assert no position. S012's and S015's are handed `a` then `b`, and S015's
    `a` at rank 4, the last supersede's. S022's, with a limit of 0, claims the pocket
    that was published into first.
  - S015's port also asserts the superseded queue's order, which is `impl-choice` for
    another reason (G19).
- **`N`-note:** agrees with N16.
- **Question:** as G04 and G16. Is the starting value meant to be unspecified? If so,
  should S012, S015 and S022 assert absolute positions?

### G30: S019's large ranks are a 64-bit integer's extremes, not the double's

- **Ticket:** GD03.
- **Concerns:** `Record.rank`, G08, and scenario S019.
- **The specification says:** a rank is an `Integer`. It states no bound.
- **The double does:**
  - S019's port, "large ranks keep full precision across supersede and stale", publishes
    one key at the Rust test's ranks: -(2^63), -(2^63) + 1, 2^63 - 2 and 2^63 - 1, and
    then 2^63 - 2 again. It asserts that the second, third and fourth publishes fire
    `Supersede` and the fifth `DropStalePublish`, that the record ends at 2^63 - 1, and
    that a pull hands that rank out.
  - A rank is an `Int`, which has no bound on the Erlang target (G08). These values are
    not a boundary for the double: it would accept 2^63 as a rank and order it above
    them. The port publishes no rank beyond them.
  - S019 is a `wire` row, and only its library half is ported.
  - Everything the port asserts of the ranks follows from the specification's `Integer`.
    One assertion is `impl-choice` for another reason: the port lists the superseded
    queue's ranks in order (G19).
- **`N`-note:** agrees with N17, which says its row is not `impl-choice`. Here that holds
  of the ranks, and not of the superseded queue's order.
- **Question:** as G08 and N17. Should the specification bound a rank, so that S019's
  values become the boundary for every implementation?

### G31: the lease view is read from the snapshot for now (S036's record order is `impl-choice`)

- **Ticket:** GD03.
- **Concerns:** the `Consuming` surface's `exposes` block; scenarios S033 and S036.
- **The specification says:** for each active lease of the consumer the surface exposes
  `lease.pocket.key`, `lease.expires_at` and, for each record of
  `lease.pocket.held_records`, `record.key`, `record.held_rank`, `record.held_payload`
  and `record.mark`. Of the leases it says: "Zero or one lease by the invariant;
  iterating avoids a null read." `held_records` is `records where status = held`, with
  no order. The block exposes no lease identity.
- **The double does:**
  - The owner has no message for the lease view yet. Its `LeaseView` message lands in
    GD04.
  - Until then the two ports that read a lease view use a test helper,
    `harness.lease_view`. It takes one snapshot and reads the consumer's `active_leases`
    and that lease's pocket's `held_records` from the snapshot's derived sets. It answers
    the pocket's key, the lease's `expires_at`, and each held record's key, held rank,
    held payload and mark, or nothing when the consumer has no active lease.
  - The helper lists the held records smallest position first. The view carries no lease
    id, as the surface exposes none.
  - S033's port, "a pull with no queued pocket returns NoPocketAvailable, not an error",
    asserts that a consumer told `NoPocketAvailable` has no lease view.
  - S036's port, "a pull under a lease extends the lease", asserts the view's pocket key
    and `expires_at`, and its records' keys as the ordered lists `r0`, then `r0`, `r1`.
    The order is `impl-choice`: it is more than the surface states.
  - S036 cannot tell which order the helper keeps. Its records were published in key
    order, so position order, key order and the order of creation agree. S090, the row
    N23 records, is the one that separates them, and it is GD04's.
- **`N`-note:** takes N23's reading, smallest position first. N23 is written for S090 and
  for a lease view the queue answers; here the view is a test helper over the snapshot
  until GD04.
- **Question:** N23's. Should the surface state an order for the held records? And should
  it expose the lease's identity, which every action it provides takes?

### G32: what the competing callers show of "one serialized stream"

- **Ticket:** GD03.
- **Concerns:** the Lease expiry comment; `PullClaimsFrontPocket` and
  `PullFindsNoPocket`; the `ExclusivePocket` and `OldestFirst` guarantees;
  `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `LeasedHasOneActiveLease` and
  `UnleasedHasNoActiveLease`.
- **The specification says:**
  - "The lapse is handled in the same stream as the consumer's actions", and "The
    implementation applies the lapse and every action from one serialized stream
    (decision 0019), so the guards above decide in either order."
  - `ExclusivePocket`: "While a lease is active no other consumer is handed a record from
    its pocket."
  - Every rule reads the state as the last outcome left it. The comment is the one place
    that says actions are applied one at a time, and it says so of the lapse. No catalog
    row has two callers that act at the same moment.
- **The double does:**
  - The owner's mailbox is the stream. Each `Pull` is one message, applied to the state
    the message before it left.
  - Three tests in `competing_test.gleam`, none with a scenario tag, start one owner and
    a process for each consumer, and let every process pull once at the same moment:
    - "eight consumers pull at once while one pocket is queued: one is handed its records
      under a lease and seven are told there is none";
    - "six consumers pull at once while six pockets are queued: each is handed a
      different pocket, and the pockets are claimed front first";
    - "nine consumers pull at once while four pockets are queued: four are handed a
      pocket each and five are told there is none".
  - Each asserts only what holds in every order the mailbox could take, and never which
    consumer won or which pocket a consumer got.
    - From the callers' answers: as many batches as the fewer of the consumers and the
      queued pockets, and `NoPocketAvailable` for every other caller. No two batches name
      one pocket, one lease or one consumer. The pockets named are the front ones. Each
      batch is its own pocket's records, oldest first, up to the limit.
    - From the snapshot afterwards: `invariants.check` names nothing. The leases are the
      batches' leases, one for one, each active, and no two share a pocket or a
      consumer. The pockets claimed are no longer queued. The held records are the ones
      the batches handed out.
    - From the fakes: one lease id drawn and one clock read for each lease granted.
  - So the tests exercise `ExclusivePocket` and `OldestFirst` on what each caller was
    handed, and the four invariants above on the state. `ExclusivePocket` is a statement
    about what callers were handed, which no snapshot holds, so it is asserted from the
    answers: every payload names its pocket, and a batch must be its own pocket's
    records.
  - Two assertions rest on an `impl-choice`, the order in which the snapshot lists a
    collection (G19). One is that the snapshot lists the leases in the pocket order of
    their pockets, which holds because it lists leases in the order they were created.
    The other compares a pocket's held records, as the snapshot lists them, with the
    keys of its batch, in the batch's order. That holds because the snapshot lists
    records in the order they were created, which within one pocket here is the order
    of their positions. A snapshot that listed the same records by key would fail it,
    though every batch were right.
  - Only these tests tell one scratch owner from the real one. It applied each `Pull` in
    a process of its own, from the state the owner held when it read the message, and
    took the new state back as a second message. All three competing-callers tests failed
    against it and every other test passed. A second scratch owner, which answered a
    `Pull` from the state held before the previous message, failed these three and other
    tests as well.
  - What the tests cannot show:
    - That the properties hold in every interleaving. A run samples the mailbox orders
      the scheduler produces, and the scheduler mostly lets the callers reach the mailbox
      in the order they were started. Each test therefore runs once for each caller, with
      that caller started first.
    - Anything about the case the comment is written for, a lapse applied in the same
      stream as a consumer's action. Only a `Pull` competes with a `Pull`. No publish,
      renew, ack, nack, release or lapse competes with anything, and the clock stands
      still throughout.
    - That an action's `now` is the time the owner reached it (G05).
  - G03 records the one earlier test with competing callers, of a join.
- **`N`-note:** no `N`-note: the Python double has no callers that compete.
- **Question:** should the specification state, as a guarantee of its own, that actions
  are applied one at a time, each to the state the one before it left? Every rule
  assumes it, and only the Lease expiry comment says it. Should the catalog hold a row
  in which two consumers pull at the same moment?
