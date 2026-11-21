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
    is still pending.
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
  - GD02 lands the field so that the model and the snapshot are complete. No rule creates
    a lease yet: `PullClaimsFrontPocket`, the rule that draws an id through the request
    G05 describes, is pending. The invariant controls write their ids by hand.
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
    `Supersede`'s guard does not hold for it. No publish through the owner can reach the
    input until a pull lands, since no record is held before then.
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
  - A rule's answer is an `Option(Answer)`. `Answer` has no constructor in GD02, so the
    answer of every rule landed so far can only be `None`.
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
    rule is pending.
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
  - No ported row asserts this order. S018's port reads the superseded queue, which
    holds three entries by its end, by rank and by count. No other GD02 port asserts a
    list with more than one member. S011's port asserts the pocket order, which it reads
    from the positions.
  - The double's own tests of the four rules, the ones named "… changes exactly what it
    ensures …", compare the complete snapshot and so do assert it: a new pocket, a new
    record and a new shed entry at the end of its list, and a superseded record in its
    place. Those assertions are `impl-choice`. They hold because of this order, not
    because of anything the specification states.
  - No GD02 rule creates a lease or a dead letter. Their order is the one stated here,
    for the tickets that land those rules to keep.
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
