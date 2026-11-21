# Specification notes from the Gleam double

The Gleam double under `gleam-double/` is derived from `docs/specs/pachinko.allium`, read
with the maintainer's four rulings on the Python double. This file records every place
where the specification was silent or ambiguous, or forced an interpretation, and every
catalog row the double ports although it asserts more than the specification states
(labelled `impl-choice`). The maintainer sets it beside
[`double/SPEC-NOTES.md`](../double/SPEC-NOTES.md) to judge the specification: where two
doubles written apart read a passage the same way, the reading is likely the
specification's; where they differ, the passage is ambiguous. Agreement is weaker
evidence than that sounds: this double took an `N`-note's reading wherever the
specification did not contradict it, so where the two agree the text did not contradict
the Python double's reading, which is not two readers reaching it apart.

Each note gives its id, the ticket that wrote it, the rule, invariant, entity or comment
it concerns, what the specification says, what the double does, the `N`-note of the
Python double it agrees with or departs from, and the question it leaves. The ids run
`G01`, `G02` and so on, so that a note here cannot be mistaken for an `N`-note. A note
that only agrees with its `N`-note repeats that note's question, so that this file can be
read alone.

The double is one owner actor in front of a pure core. Where a note concerns what the
actor adds, which the Python double could not show, it says "no `N`-note".

The findings come first. The 67 notes follow, each under one of four headings and in id
order within it, keeping the id it was written with:

- [where the second reading agrees with an `N`-note](#where-the-second-reading-agrees-with-an-n-note);
- [where it departs from an `N`-note](#where-it-departs-from-an-n-note);
- [where it raises what no `N`-note holds](#where-it-raises-what-no-n-note-holds);
- [the `impl-choice` rows](#the-impl-choice-rows).

[`README.md`](README.md) maps the specification's entities, rules and invariants to the
double's types, tables and checker entries, and walks Scenarios A to D through the owner.

## Findings

Each finding is a question for the maintainer, with the notes or the tests that hold its
evidence. None of them has been applied to the specification, and none answers one of
its open questions. The Python double's findings are numbered as
[`double/SPEC-NOTES.md`](../double/SPEC-NOTES.md) numbers them.

### The maintainer's rulings, followed

The double ports each row a ruling covers as the ruling says, with no new stop. None of
the rulings settles a question about the specification, and
`.scratch/python-double/PD07-inspection-contract.md`, which takes them up, is still
todo.

| Ruling | Rows | `N`-note | Applied in | Note here |
| --- | --- | --- | --- | --- |
| A publish answers nothing, as the specification states. A ported row asserts that answer and does not port the Rust's outcome. | Every publish row the Rust checks for an outcome | N32 | GD02 | G15, and G54 for GD05's deferrals |
| An ended lease stays in the state. The Rust's empty lease registry is its inspection shape. | S053 | N24 | GD04 | G37 |
| S091's kept positions are compared by key. The Rust's record order is its inspection shape. | S091 | N31 | GD05 | G49 |
| The three ports stand as the Rust's inspection shape. | S066's release half, S093 and S075's stored pocket order | N38 | GD06 | G67 |

### How the notes are grouped

A note sits under the heading its main claim fits. A title that labels a row
`impl-choice` decides first; otherwise the note's `N`-note line decides:

- **Agrees.** Its `N`-note line says it agrees with an `N`-note, and what it adds is a
  detail or a test.
- **Departs.** It holds a fact by other means than its `N`-note does.
- **No `N`-note.** Its `N`-note line says "no `N`-note".
- **`impl-choice`.** Its title labels a ported row, or named rows, `impl-choice`.

A note that also fits another heading is listed there:

- Under **agrees**: the `impl-choice` notes that agree with an `N`-note, G02 (N09), G12
  (N10), G16 (N04), G26 (N18), G29 (N16), G31 (N23) and G36 (N22).
- Under **departs**: G09, G17 and G26, which report no dangling reference where N18 says
  the Python double's snapshot refuses one; and G22, G48 and G56, each of which differs
  from its `N`-note in one detail. "Where the second reading departs", below, gives the
  specification text for each.
- Under **no `N`-note**: G06, G33 and GD05's nine rows in G48, which are `impl-choice`
  notes with no `N`-note; and the part of an agreeing note that the Python double could
  not show or did not state: G03's and G63's competing callers, G05's one `now` per
  outcome, read when the owner processes the action, G09's reference that names nothing,
  G15's and G56's `fired`, G59's `PartialTick` and G60's `now` carried by `LeaseLapsed`.
- Under **`impl-choice`**: G19, G38, G49, G57 and G64, whose titles label only part of
  what they record; and G23, G27, G30, G32, G34 and G39, each of which labels one or more
  assertions or tests `impl-choice` in its body.

| Heading | Notes |
| --- | --- |
| Agrees with an `N`-note | 34 |
| Departs from an `N`-note | 2 |
| Raises what no `N`-note holds | 21 |
| The `impl-choice` rows | 10 |

### Where the two readings agree

The Python double's findings stand as it wrote them. The notes that agree repeat their
`N`-note's question and add no answer to it. That they agree shows that the
specification's text did not contradict the Python double's reading, since shared rule 1
of the GD tickets told this double to take that reading unless it did; it does not show
that the reading is the only one.

| Finding of the Python double | Its question, in short | The notes here that read it the same |
| --- | --- | --- |
| 1 | Is the inspection shape part of the contract? | G37, G49 and G67 |
| 2 | Should the surfaces name what each trigger answers? | G15, G22, G37, G54 and G56 |
| 3 | Should a lease have an identity, and should the `Consuming` surface expose it? | G10, G27 and G31 |
| 4 | Should each action require a joined publisher or consumer? | G12, G26 and G36 |
| 5 | Should the specification name its refusals and the order of an action's `requires`, and refuse a second join? | G03, G13, G28 and G34; G35 departs in one field |
| 6 | Is a `requires` that restates an invariant a guard in its own right? | G23, G39 and G61 |
| 7 | Should the entries one lease end, or one tick, creates be ordered? | G31, G38, G49 and G57 |
| 8 | Should a reader be told that `shed_at` does not order a key's versions? | G50 |
| 9 | How are the temporal triggers processed, and is a tick atomic? | G56, G58, G59 and G60 |
| 10 | Does a rejected action have a `now`? | G05 |
| 11 | Is a claim that hands nothing intended? | G25 |
| 12 | Does an idle pocket's position mean anything? | G40 |
| 13 | Should the specification fix its numeric domains? | G01, G04, G08, G16, G29, G30, G33 and G48 |
| 14 | Is "refused at configuration" the intended reading of "the model is defined for"? | G02 |
| 15 | Is `ArrivalPositionUnoccupied` meant to range over a record paired with itself? | G11 |

G07 and G09 agree with N02 and N06, which leave no question of their own; G14's reading
is N08's, and it departs in the means (below). G65 agrees with N36, which is open
question 7 (below).

### Where the second reading departs

The two notes filed here differ from their `N`-note in means, where the specification's
text left the means open. The third item is a difference of reading, spread over notes
filed elsewhere, and the fourth lists smaller differences:

- **G14, against N08.** The text is `held_rank: Integer when status = held`, with
  `held_payload`, `mark`, `nacked_at` and `deferred_position` declared the same way, and
  `ended_at: Timestamp when status = released | expired`. Both doubles read the
  declaration as the entity's shape and not an invariant. N08 refuses at construction a
  record or a lease that breaks it; here the held fields are fields of the held variant,
  so no such value can be written, and the snapshot carries the variant rather than the
  flat field names shared rule 4 words. **The maintainer decides:** is a snapshot whose
  record carries the held fields on its held variant acceptable, or should it show the
  flat record, with the five fields optional?
- **G35, against N21.** The text is the Ack and nack comment: "no rule covers an ack of a
  nacked record or a nack of an acked one, so that action is rejected". It names neither
  the rejection nor what the rejection carries. N21's `MarkConflict` carries the mark
  that stands; this one carries none, for a reason in the double's module graph, and a
  caller learns the same from either. **The maintainer decides** finding 5: is a
  refusal's kind, and what it carries, the specification's to name at all?
- **A reference that names nothing: G09, G17 and G26, against N18.** The text, as G09
  quotes it, "declares no invariant that a reference names an entity". N18 says that
  without its refusal of an unknown consumer such a lease would be refused by the
  snapshot "as a dangling reference". This checker reports no dangling reference, so a
  hand-built state with one breaks only the invariants it otherwise breaks, and G26
  keeps the refusal for N18's other reason, that the specification gives the trigger a
  `Consumer` only a join creates. **The maintainer decides** G09's question: what does
  an invariant that reads through a reference ask when the reference names nothing?
- **Smaller differences.** G22: the text is `Batch(consumer: consumer, lease: lease,
  pocket: pocket, records: batch)`, with `batch` from `oldest(pocket.queued_records, …)`,
  which does not say which fields of a record a batch carries; this `BatchRecord` holds
  the key, the rank and the payload, where N13's batch holds the `Record` entities. G48:
  the text gives `next_position` no initial value, so an absolute position is the
  double's; GD06's ports keep the Rust's absolute positions beside the comparison with
  the state before, where N38's keep the comparison alone. G56: the text names no
  process that notices a due trigger and no surface that reports one, so a tick's report
  is the double's; it carries `fired` as well as N33's lapsed leases and decayed entries.

### What this double raises that no `N`-note holds

Each item is a question for the maintainer, and the notes hold its evidence. The
serialized stream has a section of its own, below.

1. **References that name nothing.** In a checker that holds keys, what does an invariant
   that reads through a reference ask when the reference names no entity? Should
   `OneActiveLeasePerConsumer` range over the leases, as `OneActiveLeasePerPocket` does,
   so that it does not depend on the consumer being there? As the specification types
   its references, neither state arises (G09, G17).
2. **Positions.** Is "positions are unique" meant across pockets, records and arrival
   positions together, which would need an invariant the specification lacks (G18)? Is a
   revived pocket's position meant to be above its new record's, the reverse of a new
   pocket's (G45)? Should `SupersedeUnsettledHeldVersion` ensure the position it keeps, as
   the other two rules that queue a deferred version ensure theirs (G52)?
3. **What a rule leaves alone.** Is a field that no `ensures` names unchanged, for every
   rule? The specification says so field by field for `Supersede` only (G21).
4. **Orders the specification does not state.** Are publishers, consumers, pockets,
   records, leases and the two shed queues meant to have no order beyond the positions'?
   If PD07 makes an inspection order part of the contract, which order is it for each
   (G06, G19)? Should a shed entry have an identity, if PD07 or open question 2 makes an
   entry something a reader names (G64)? The lookups and `min_by` choose between
   entities only in a state built by hand (G24).
5. **Identity and contents.** If the specification declares a lease's identity, should it
   also say that two leases never share one (G27)? Which fields of a record does a batch
   hand over (G22)? Is an ack of a queued record of the leased pocket refused as a key not
   held, or as a refusal of its own (G34)?
6. **Time within an outcome.** Should the specification say that an action's `now` is
   the time it is processed (G05)? Should it say that a rule answering an event reads the
   `now` of the outcome that emitted it, so that an event need carry no time? As written,
   the expiry rules' `requires: lease.expires_at <= now` assumes it (G60).
7. **Comments that could say more.** Should the comment above `PublishNewKey` say outright
   that a publish into a new pocket emits nothing (G20)? Should the comment above
   `ReleaseIdlesEmptyPocket` say that a pocket with no record idles too (G41)? Are the
   release rules' `requires: pocket.status = leased` and the mark rules' silence on it
   meant to differ (G42)? Should the two dead-letter rules state
   `requires: record.nacked_at != null`, or is `NackedHasItsTime` meant to carry it
   (G43)? Is the comparison of `pocket.records.count` with `pocket.ending_records.count`
   meant to be the same in each of the four rules that write it out (G46)? Should the
   Lease end comment's "one outcome, so the invariants hold after the whole of it" be
   said of a publish and the revival it causes too (G47)?
8. **A Rust comment.** Should S075's comment in the Rust say which pocket parks (G67)?

G44, G51, G53, G55 and G66 record a reading and leave no question.

### What the actor showed about the serialized stream

The Lease expiry comment says: "The implementation applies the lapse and every action
from one serialized stream (decision 0019), so the guards above decide in either order."
In this double the stream is the owner's mailbox. What the actor showed, finding by
finding:

1. **Competing pulls, in the orders that occurred.** The three tests in
   `competing_test.gleam`, "eight consumers pull at once while one pocket is queued…",
   "six consumers pull at once while six pockets are queued…" and "nine consumers pull at
   once while four pockets are queued…", let several consumer processes pull through one
   owner at once. From the answers they exercise `ExclusivePocket` and `OldestFirst`;
   from the snapshot afterwards, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`,
   `LeasedHasOneActiveLease` and `UnleasedHasNoActiveLease`; and from the fakes, one
   clock read and one lease id for each lease granted. GD03's scratch owner that applied
   a `Pull` outside the owner's process, and took the new state back as a second message,
   failed these three and passed every other test (G32). What they cannot show: a run
   samples the mailbox orders the scheduler produces, and the tests do not enumerate
   interleavings. Each test runs once for each caller, with that caller started first,
   and GD03's repeat of 598 runs saw 8, 24 and 27 different grant orders across the three
   bodies. A pass says nothing about an order that never occurred. **The maintainer
   decides** G32's question: should the specification state, as a guarantee of its own,
   that actions are applied one at a time, each to the state the one before it left?
   And should the catalog hold a row in which two consumers pull at the same moment?
2. **Competing joins, in the one order that occurred.** `owner_test`'s "the same join
   from several processes succeeds exactly once" starts eight processes that send the
   same join, with no gate and one run. It shows one join succeeding and every other
   refused, in whatever order that run produced, with the limits of finding 1 and fewer
   orders sampled (G03). **The
   maintainer decides** finding 5's join question, which this does not change.
3. **An action against a lapse.** `branch_a_test`'s "a renew and a tick sent at once by
   two callers agree whichever the owner takes first" runs twenty times and accepts
   either of two consistent results, each with an empty `check`. It never asserts which
   message came first, and it cannot show that both orders occurred: a scratch print that
   showed both was not saved (G63). The two orders the guards must decide are shown
   instead by fixing them. S066's and S067's four halves, and the untagged late nack,
   send the `Tick` and the action from one process, in a chosen order, to two owners
   built alike. A scratch `Renew` that lapsed its own lease failed S066's renew half and
   the competing test (`ai_tmp/gd/GD06/ev-a-break.txt`). **The maintainer decides** G63's
   question: should the catalog hold a row in which an action and a lapse compete?
4. **No rule needed more than one message, and no process but the owner holds queue
   state.** This
   is a reading of `owner.gleam`, not a test result. Each trigger a surface provides is
   one `Act` message, which the owner applies through the core and answers before it
   takes the next message. The events a rule emits run inside that one apply step:
   `core_test`'s three witness tests, "a publish of a new key into an idle pocket
   stands…" and the two release tests beside it, pass through states part-way that break
   an invariant and stand (G47). The clock triggers' rules run inside one `Tick`: one
   message, with one outcome for each due lapse and each due decay. While it applies a
   message the owner calls the clock and lease-id functions it was started with. In the
   tests those are calls to the fakes' processes, which hold the time, the reads and the
   draws, and the harness's log holds the names of the rules that fired. None of them
   holds queue state, though the owner relies on them: that two leases never share an
   id rests on the injected source, and `owner_test`'s `PartialTick` test breaks it on
   purpose (G27). `owner` is the only module under `src/` that imports `gleam/erlang` or
   `gleam/otp`, which GD07's Verification checks. No test can show that no other process
   holds state, and none is offered as showing it. The scratch owner of finding 1 kept
   the state but applied each `Pull` in a process of its own and needed a second message
   to bring the new state back; only the three competing tests caught it, in the orders
   they sampled.
   **The maintainer decides:** does "one serialized stream" require that one owner hold
   the state, or only that applications be ordered? One owner sufficed for every rule
   here, which does not show that it is required.
5. **A lapse ahead of an already-queued action: a single mailbox never does it.** The
   comment says "arrival order does not decide, since the owner may apply a lapse ahead
   of an already-queued action", and decision 0019 explains it by a tick lane polled
   first. The double has one mailbox and no second lane, and a lapse happens only inside
   a `Tick`, which the owner takes in its turn. So a lapse never overtakes a message that
   reached the mailbox before its `Tick`, and no lapse happens without one (G62). Every
   order the double produces is one the sentence allows. That too is a reading of
   `owner.gleam`: no test can show that an order never occurs. **The maintainer decides**
   G62's question: is a lapse overtaking a queued action a freedom of the implementation
   only, or something every implementation must be able to show?
6. **A failure part-way through a tick.** GD06 chose that a tick keeps each outcome that
   succeeded before a failure, stops at the failure, and does not roll back. The owner
   answers `Error(PartialTick(report, failure))` and keeps the state the last outcome
   that succeeded left (G59). That is what finding 9 records of the Python double, which
   commits each lapse and each decay as it goes. The double adds three facts: if a lapse
   fails, no decay runs; the outcomes after the failure never run; and a failure that
   stays in the state stops every later tick at the same place. The tests are
   `core_test`'s "a run of due outcomes keeps each outcome that succeeded before a
   failure, stops at the failure, and checks each outcome at its end", `expiry_test`'s "a
   tick whose second lapse fails keeps the first, stops there, and decays nothing", and
   `owner_test`'s "a tick whose outcome fails is answered PartialTick, and the owner
   keeps what the outcomes before it did". The owner test reaches the failure through a
   lease-id source that repeats an id. GD07's review found a second way, with unique
   ids: a clock set back between a nack and a renew, so that the lease ends before the
   record's `nacked_at` and its dead letter breaks `NackedBeforeExpiry`
   (`owner_test`'s "a clock set back after a nack leaves the lease's end before
   nacked_at, so the lapse and a release fail NackedBeforeExpiry"; G59). Nothing shows
   there is no other. One owner did not force the choice: it holds the state from before
   the tick and could have kept it. **The maintainer decides** finding 9's question,
   whether a tick is atomic as a whole, and G59's two: should one outcome that cannot be
   applied hold back the outcomes due after it, at this tick and at every later one? And
   does the specification mean `now` never to go back?
7. **Whether one owner made a question of the Python double easier or harder.**
   - Finding 10 is no easier to decide, but sharper. The owner reads the clock only
     between taking a message and replying, so an action's `now` is the time the owner
     reached it, and a refusal reads no clock (G05). Whether a rejected action has a
     `now` is still the specification's to say.
   - Finding 9 is sharper too. A tick is one message, so making it atomic would cost one
     owner nothing, and the question is what the specification wants, not what an
     implementation can do (item 6).
   - The overtaking freedom is harder to judge from this double. A single mailbox cannot
     produce the order the Lease expiry comment permits, so this double is no evidence
     about an implementation that uses it (item 5).
   - Finding 6 is unchanged. Through the owner the expiry rules' restated guards still
     cannot fail, because a tick offers only what is due and applies each lapse in the
     outcome that found it due (G61).
   - The notes record no way in which one owner bears on the other findings.

### Open questions left untouched

The specification's six open questions, numbered 2 to 7 as the specification's own
comment numbers them, are untouched. No note answers one, and the double adds no rule for
any of them:

- Question 2: "Who reads the superseded and dead-letter queues, and what can they do: an
  operator listing, a replay action back into the pocket, or a consumable stream under
  the lease model? The answer also fixes what an entry must carry; reason and nacked_by
  are kept on that assumption."
- Question 3: "Capacity beyond shedding the decaying queues first: which superseded
  records and dead letters shed first, a bound on records, disk spill, a back-pressure
  signal to publishers, or none? Request-shape limits (rows, bytes, timeouts) are not
  capacity refusals."
- Question 4: "Routing by worker pool or priority (standard against high-memory workers,
  newly onboarded tenants first, spot against reserved): separate deployments, or a
  property of pockets?"
- Question 5: "Adaptive lease TTL: should lease_ttl be derived from observed lease
  durations to shorten recovery after a consumer dies?"
- Question 6: "Should a newly created pocket be able to jump to the front of the queue,
  opt-in, for fast first service?"
- Question 7: "Can anything other than a publish revive a parked pocket, such as an
  operator action or a cooldown?"

The catalog has one `oq:N` row, S026 (`oq:7`). Its port, "only a publish revives a parked
pocket (oq:7)", ports the rules as they stand: only a publish revives a parked pocket.
G65 records it and leaves question 7 open.

## Where the second reading agrees with an `N`-note

The note's `N`-note line says it agrees. What it adds is a detail of this double or a
test the Python double does not have. A part with no `N`-note is listed in the findings,
under "How the notes are grouped".

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
  position; the rows that assert absolute positions are ported in GD02 to GD06, and their
  notes, G16, G29, G33 and G48, cite this one; G48 holds GD05's and GD06's. GD01's ports
  of S003, S004 and S005 assert that an owner starts over an admitted config and holds
  no publisher and no consumer, and not the position it starts at. The two owner tests,
  which pin no catalog row, compare whole snapshots and so do pin the 0.
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
  - A `Tick`, from GD06, is the one message that reads the clock before any rule asks:
    once, even when nothing is due. Each outcome it runs has that `now` in hand, so a
    row that asks for the time reads no clock, and a clock trigger's guard reads `now`
    from the trigger's input (G56, G60).
  - The owner is the only process that calls either function, between taking a message
    from its mailbox and replying to it. The time an action is processed at is therefore
    the time the owner reached it, not the time its caller sent it.
  - S085, ported in GD04, is the one catalog row that says a refusal "reads no clock".
    Its port, "a changed mark is refused as a mark conflict and changes nothing, not
    even the lease's expiry, and reads no clock", shows it twice. The fake clock's read
    count is the same after both refusals and the lease read as before them. And, as
    the Rust shows it, a renew at an earlier time than the refusals extends the lease
    from that earlier time, which it would not if a refusal's time had been kept. Every
    other refusal from GD04 on goes through `harness.refused`, which compares the read
    count before and after as well.
- **`N`-note:** agrees with N28 (a refused action reads no clock) and N15 (a lease id is
  drawn only when a lease is created), and with N05 in giving a lease an id. Where the
  Python double's queue reads its clock inside a method call, this one's core never
  holds a clock at all. "One `now` per outcome, read when the owner processes the
  action" has no `N`-note of its own.
- **Question:** does a rejected action have a `now`? And should the specification say
  that an action's `now` is the time it is processed, which the Lease expiry comment
  implies when it says that a late action processed before the lapse succeeds?

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

### G13: the equal-rank publish is the Publish table's refusal, `EqualRankRefused`

- **Ticket:** GD02.
- **Concerns:** the Publish comment, and scenarios S017, S046, S082 and S083.
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
    a held record only with a version deferred behind it, and DeferBehindHeldVersion only
    with none" asserts that a rank below the stored rank of a held record reaches
    `DropStalePublish`, with a version deferred and with none, and that a rank equal to
    it finds no rule, whether the record is unsettled, acked or nacked.
  - GD02 landed four of the five rows: `PublishIntoNewPocket`, `PublishNewKey`,
    `Supersede` and `DropStalePublish`. With those four, no guard held for two inputs: a
    rank equal to the stored rank, and a higher rank published onto a held record with
    nothing deferred, which is `DeferBehindHeldVersion`'s. Until GD05 the second was
    pending, and no test pinned it as a refusal, as shared rule 4 of the GD tickets
    asks.
  - GD05 lands `DeferBehindHeldVersion` as the table's fourth row, so the Publish table
    holds all five rules and the equal rank is the one input it refuses. Three ports
    publish the stored rank of a held record through the owner and are refused, each
    through `harness.refused`, which compares the complete snapshot, the clock's read
    count and the lease-id draws before and after:
    - S046's "a stale check while held compares with the latest rank, not the held one":
      the deferred version's rank;
    - S082's "a publish onto a held record is judged alike whether it is unsettled,
      acked or nacked: higher defers, a second higher sheds the deferred version, equal
      is rejected, lower is stale": the rank of the version that superseded the deferred
      one, for each mark;
    - S083's "publishing again at the held rank is rejected while nothing is deferred
      and stale once a newer version is": the held rank while nothing is deferred, which
      is then the stored rank. Once a newer version is deferred, the same publish is
      `DropStalePublish`'s, as the comment says.
- **`N`-note:** agrees with N11.
- **Question:** none on the behaviour, which the closure of open question 1 decided. The
  refusal's name is the double's.

### G15: a publish answers nothing (the maintainer's ruling on N32)

- **Ticket:** GD02.
- **Concerns:** `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`,
  `DeferBehindHeldVersion` and `DropStalePublish`; the `Publishing` surface; scenarios
  S011, S013, S014, S016, S017 and S018, and, through G54, GD05's ports.
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
  - `fired` shows an emitted event only through the rule that answers it. In GD02 the
    `PocketTouched` table had no row, so the event changed neither the rules that fired
    nor the snapshot. The test "PublishNewKey, Supersede and DeferBehindHeldVersion touch
    the pocket, and the other two emit nothing" pins the events by walking each row's
    plan (G20). GD04 gives the table `ReviveIdlePocket`, and a publish of a new key into
    an idle pocket replies `Ok(Outcome(["PublishNewKey", "ReviveIdlePocket"], None))`
    (G45).
  - GD05's deferral answers nothing too, and its reply names `DeferBehindHeldVersion`
    alone (G54).
- **`N`-note:** agrees with N32 and follows its ruling. The owner's `fired` has no
  `N`-note.
- **Question:** N32's, which the ruling leaves open. N32 records that decision 0020 fixes
  the Rust's answer, and the specification does not state it. Should the `Publishing`
  surface name what a publish answers, so that a double or a second implementation can
  port it?

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
    after the last. No pull shows that the checker waits: no state between a pull's own
    row and its `HoldRecord`s breaks an invariant, so a check part-way would pass too.
    A publish into an idle pocket and a release do pass through such states, and the
    witness is theirs (G47).
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
    `model.replace_lease` replaces every lease with that id. The fake hands out
    `lease-1`, `lease-2` and so on, and never repeats.
  - One test, from GD06, reaches that state on purpose. `owner_test`'s "a tick whose
    outcome fails is answered PartialTick, and the owner keeps what the outcomes before
    it did" wraps the fake so that its third draw repeats its second, and two active
    leases are named `lease-2`. The tick's lapse of the first ends both, because
    `model.replace_lease` replaces each lease with the id, and that leaves a leased
    pocket with no active lease, which the checker names (G59). The test depends on that
    replacement. Were only the first lease replaced, the second would stay active and,
    by this double's reading, the lapse would not fail there; no test runs that.
  - S027's port asserts that the batch names `lease-1`. That is `impl-choice`: the id is
    the fake source's first, and the port's comment says the Rust's `1` is its counting
    source's. Five other tests that drive the owner depend on literal ids:
    - the `owner_test` test above, whose wrapper repeats `lease-2` in place of
      `lease-3` and which reads `lease-2` and `lease-1` from the snapshot;
    - `guide_test`'s four walkthrough tests, Scenarios A to D as the guide walks them.
      Their shared start, `v3_deferred_behind_v1`, asserts that the batch and the
      snapshot name `lease-1`, so that the page prints the id the owner hands out, and
      A, B and C send `lease-1` in an ack, a nack or a release. A fake whose first id
      were spelled otherwise would fail all four.

    Every other test that drives the owner compares a lease id only with another the
    owner handed it.
- **`N`-note:** agrees with N15. It differs in six tests: N15 says none of its tests
  asserts a literal id, and S027's port, that `owner_test` test and the four
  walkthrough tests here do.
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
  - GD04 ports S052 and S070, and uses the refusal for an ack, a nack and a release as
    well (G34). A lease id never issued is fenced for each of the four: "a renew that
    names another consumer's lease or no lease is fenced", "a release that names
    another consumer's lease or no lease is fenced", and the ports of S042 and S049.
  - The rule sets `expires_at` and nothing else (G21). "Renew changes exactly what it
    ensures, reads the clock once and draws no lease id" compares the complete snapshot
    after each of three renews, and S050's port asserts that `granted_at` stays. "Renew
    leaves a retained version and a dead letter as they were" runs the rule over a state
    built by hand that holds one of each, and compares the complete state.
- **`N`-note:** agrees with N20. It differs in what is tested: N20 records a test that
  pins `LeaseFenced` for a lease id never issued, and no GD03 test pins the refusal for
  any input. From GD04 the tests pin it, a lease id never issued included.
- **Question:** N20 leaves none on the behaviour. The refusal's name is the double's. A
  lease that was never issued is not a refusal of its own: GD04 keeps it `LeaseFenced`
  for every action under a lease (G34).

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

### G34: the refusals of a mark are `LeaseFenced`, `RecordNotHeld` and `MarkConflict`, the lease checked first

- **Ticket:** GD04.
- **Concerns:** `AckMarksRecord`, `AckRepeats`, `NackMarksRecord` and `NackRepeats`; the
  Ack and nack comment; the `ActionsNameTheirLease` guarantee; scenarios S042, S049,
  S068, S070 and S085.
- **The specification says:**
  - Each of the four mark rules requires `lease.status = active` and
    `lease.consumer = consumer`, then, with
    `let record = Record{pocket: pocket, key: record_key}`, `exists record`,
    `record.status = held` and the mark the rule names.
  - "no rule covers an ack of a nacked record or a nack of an acked one, so that action
    is rejected". And: "An action meant for a lease that has ended is rejected, even
    when the same consumer holds the same pocket again under a new lease and the same
    key at a newer rank."
  - It names no rejection, and does not say which applies when more than one `requires`
    fails.
- **The double does:**
  - The Ack and Nack tables have two rows each. Their answer when no guard holds is a
    refusal that names the first `requires` that fails, in the order the rules state
    them. To find it the refusal reads the state it is handed, as a guard does, and
    changes nothing. These two and the Release table's (G39) are the first refusals that
    read the state. Every earlier table's refusal reads the trigger alone: the joins',
    `EqualRankRefused` (G13), `PullRefused` (G23) and Renew's `LeaseFenced` (G28).
  - The lease first. A lease that is not active, is not the consumer's, or whose id
    names no lease is `LeaseFenced`, carrying the action's name, the consumer's id and
    the lease's id, whatever key the action names. That is G28's refusal, now used for
    all four actions under a lease. The Rust's `LeaseNotActive` and `LeaseNotYours` are
    both `LeaseFenced`: the specification draws no line between the two `requires`.
  - Then the record. With the lease good, a key the lease's pocket does not hold as a
    held record is `RecordNotHeld`, carrying the action's name, the lease's id and the
    key. That covers a key the pocket does not have, a key held only in another pocket,
    since the lookup is `Record{pocket: lease.pocket, ...}`, and a queued record of the
    leased pocket. The queued record passes `exists record` and fails
    `record.status = held`; the double gives it the same refusal as an absent key.
  - Then the mark: the record is held with the other mark, and the action is
    `MarkConflict` (G35).
  - S070's ack by `c3`, under `c1`'s lease, names a key that lease's pocket does not
    have, and is fenced, not "not held".
  - Every refusal through the owner goes through `harness.refused`, which compares the
    complete snapshot, the clock's read count and the lease-id draws before and after.
    The tests:
    - S042's "rejected acks change nothing, not even the lease's expiry; a repeat ack is
      not a refusal" and S049's "rejected nacks change nothing and create no dead
      letter": `LeaseFenced` for another consumer's lease and for an id no lease has,
      and `RecordNotHeld` for an absent key, a queued one and a key held in another
      pocket;
    - S068's "a consumer that re-claims the same pocket cannot use its old lease", and
      "an ack, a nack and a release that name a lease that has ended are fenced": an
      ended lease;
    - S070's "a consumer cannot act on another consumer's lease": the order;
    - "a refused mark names the first requirement that fails, the lease first, and asks
      for no time", over a state built by hand through the apply step: every case above,
      a lease ended by expiry included.
  - Each name and shape, and the order, is `impl-choice`, and each port says so.
- **`N`-note:** agrees with N21, but for one field of `MarkConflict` (G35).
- **Question:** N21's. Is a refusal's kind the specification's to name at all? If so,
  should it name these three, and the order in which they are checked? This double adds
  one: is an ack of a queued record of the leased pocket meant to be refused as a key not
  held, or as a refusal of its own?

### G37: an ended lease stays in the state, and a mark and a release answer nothing (the maintainer's ruling on N24)

- **Ticket:** GD04.
- **Concerns:** the `Lease` entity's `terminal: released, expired`,
  `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, the four mark rules, and scenarios
  S053, S055 and S057.
- **The specification says:** a lease ends `released` or `expired`, both terminal, with
  `ended_at: Timestamp when status = released | expired`. No rule removes a lease. The
  release and mark rules ensure no answer to the caller.
- **The double does:**
  - It follows a ruling, not a reading of its own. On 2026-11-18 the maintainer ruled on
    N24 that S053's empty lease registry is the Rust's inspection shape, and that the
    double keeps the ended lease.
  - A release ends the lease in its place in the state's list of leases, as
    `LeaseReleased(ended_at)`, and nothing removes it. Fencing reads it: an action that
    names it is `LeaseFenced` (G34).
  - S053's port, "a release requeues the pocket at the back and its unsettled records in
    place, and consumes the acked ones", asserts the specification's facts in place of
    the Rust's empty registry: the lease is in the snapshot, released, with its
    `ended_at`, no lease is active, and no consumer has a lease view. S055's and S057's
    ports read the ended lease from the snapshot where the Rust reads the lease its
    release returned.
  - An ack, a nack and a release that succeed reply `Ok(Outcome(fired, None))`, with no
    answer, where the Rust's marks answer `SettleOutcome` and its release the pocket's
    fate and the ended lease. Every port of a mark or a release asserts that reply.
- **`N`-note:** agrees with N24 and follows its ruling.
- **Question:** none on the behaviour. N24's stays open: if the specification means an
  ended lease to be forgotten it does not say so, and fencing by lease needs the ended
  lease's identity kept somewhere. PD07 takes it up.

### G38: one lease end sends `Freed` smallest held position first (`impl-choice` where a test asserts it)

- **Ticket:** GD04.
- **Concerns:** the `Freed(lease, record_key, ended_at)` event; the two release rules'
  `for record in pocket.held_records: Freed(...)`; the Lease end comment.
- **The specification says:** the ending rule emits one `Freed` for each record of
  `pocket.held_records`, which is a set. The Lease end comment says the rule that ends a
  lease and the rules that answer its `Freed` "are one outcome". Nothing orders the
  events.
- **The double does:**
  - `lease_end.end` emits `Freed` for the held records of the lease's pocket in order of
    their positions, smallest first, and the apply step answers them in that order. The
    order decides the order in which the superseded records and the dead letters one end
    creates are added (G19), and the order of the `Freed` rules in `fired`.
  - With nothing deferred, no catalog row can tell the order. S053's port asserts its
    two `Freed` rules in the order `a1`, `a2`, where position order, key order and the
    order of creation agree. S047's and S086's ends create one dead letter each.
  - Two catalog rows ported in GD05 tell position order from key order, though neither
    from the order of creation, which agrees with the positions in both:
    - S091's port (G49). Its `z`, `y`, `b` and `a` are published in that order, so their
      positions run the reverse of the key order. It asserts `z` then `b` in the
      superseded queue, `y` then `a` in the dead letters, and the five rules its release
      fires in that order. It is labelled `impl-choice` (N29).
    - S044's port, "a version deferred behind an acked version is queued at its arrival
      position". Its release fires `ConsumeAckedVersionQueueDeferred` for `k` and then
      `RequeueUnsettledRecord` for `early` and for `between`, the order of their
      positions, 1, 2 and 4, against a key order that is the reverse. It is labelled
      `impl-choice` too.
  - Hand-built states pin it, each listing the records out of position order:
    "ending a lease sets its status and ended_at and sends Freed for each held record,
    smallest position first, and changes nothing else"; "a release frees the held
    records smallest position first, each by its mark, and stamps every entry from the
    lease's end", whose two dead letters are added smaller position first though the
    other is listed first. Both are labelled `impl-choice`.
  - A lapse ends its lease through the same `lease_end.end`, so its `Freed` go in the
    same order (G46). Nine GD06 ports assert, as `impl-choice`, the order of `Freed`
    rules of different kinds in a fired list, or of the entries one end creates, and
    the comment of each test or of its module says so: S048, S064, S075 and S077 in
    `parking_test`; S007 and S074 in `decay_test`; and S087, S088 and S089 in
    `lease_endings_test`, S087 and S088 in both halves (G66). Where two lapses in one
    tick create entries, their order between the lapses is G57's.
  - `Freed.lease` is the lease's id (G10).
- **`N`-note:** agrees with N25, and with N29 for S091.
- **Question:** N25's and N29's. Should the specification order the entries one lease end
  creates, or say that their order is not observable?

### G39: the Release table's refusal past fencing is unreachable through the owner

- **Ticket:** GD04.
- **Concerns:** `ReleaseRequeuesPocket`'s and `ReleaseIdlesEmptyPocket`'s
  `requires: pocket.status = leased`, and the invariant `UnleasedHasNoActiveLease`.
- **The specification says:** both release rules require `lease.status = active`,
  `lease.consumer = consumer` and `pocket.status = leased`. Between them,
  `pocket.records.count > pocket.ending_records.count` and
  `pocket.records.count = pocket.ending_records.count` cover every pocket, since the
  ending records are some of its records.
- **The double does:**
  - A release whose lease is not active, is not the consumer's or was never issued is
    `LeaseFenced` (G34): "a second release is rejected; a release at the grant instant
    is legal", "a release that names another consumer's lease or no lease is fenced"
    and "an ack, a nack and a release that name a lease that has ended are fenced".
  - Once the lease is good, `UnleasedHasNoActiveLease` makes its pocket leased in every
    state the rules reach, so one of the two rows holds. What the table answers past
    fencing, `ReleaseRefused`, carrying the consumer's id and the lease's, is never met
    through the owner, as `PullRefused` is not (G23).
  - "no Release rule holds for an active lease on a pocket that is not leased" is
    `impl-choice` and carries no scenario tag. It builds by hand three states that break
    the invariant, with the lease's pocket queued, idle or parked. In each, dispatch
    finds no row, and the apply step answers `ReleaseRefused` at once: it asks for no
    clock and no lease id and hands back no state. The test runs the apply step, not
    the owner.
- **`N`-note:** agrees with N26.
- **Question:** N26's, as G23's for a pull. Is `requires: pocket.status = leased` a guard,
  or a restatement of `UnleasedHasNoActiveLease`?

### G40: an idle pocket keeps its position

- **Ticket:** GD04.
- **Concerns:** `ReleaseIdlesEmptyPocket`, `ReviveIdlePocket`, the Pocket comment,
  `UniquePocketPosition`, `PocketPositionsBelowNext` and scenario S055.
- **The specification says:** `ReleaseIdlesEmptyPocket` ensures `pocket.status = idle`
  and `pocket.consecutive_expiries = 0`, and names no position. `ReleaseRequeuesPocket`
  and `ReviveIdlePocket` each ensure `pocket.position = position`, a fresh one. "Queued
  pockets are ordered by position, smallest first. That order is the pocket order;
  nothing else positions a pocket." The two position invariants range over every
  pocket, an idle one included.
- **The double does:**
  - An idle pocket keeps the position it had when its lease was claimed, and the release
    allots none (G21). S055's port, "a release idles an empty pocket", asserts the
    position kept, and that the pocket order is then `b` alone; it does not read the
    counter. "ReleaseIdlesEmptyPocket changes exactly what it ensures, reads the clock
    once and draws no lease id" compares the complete state, so it asserts both the
    position and the counter unchanged.
  - Whether the kept position matters: no rule reads an idle pocket's position, the
    pocket order holds only queued pockets, and a revival replaces the position before
    the pocket is queued again (G45). So the stale value is never used to order
    anything. It does still count for the two invariants, which hold: no other pocket
    was given it, and it is below `next_position`.
  - From GD06, `LeaseExpiryIdlesEmptyPocket` and `LeaseExpiryParksPocket` name no
    position either, and an idle or a parked pocket a lapse leaves keeps the position it
    had. "LeaseExpiryParksPocket changes exactly what it ensures and reads no clock" and
    "LeaseExpiryIdlesEmptyPocket changes exactly what it ensures and reads no clock"
    compare the complete state, so they assert it. `ReviveParkedPocket` replaces the
    position, as `ReviveIdlePocket` does (G45).
- **`N`-note:** agrees with N27.
- **Question:** N27's. Does an idle pocket's position mean anything? If not, the
  specification might say so, so that no reader orders idle pockets by it.

### G49: S091's kept positions are compared by key and against the state before (the maintainer's ruling on N31), and its order of entries is `impl-choice`

- **Ticket:** GD05.
- **Concerns:** the Record comment ("One record per key per pocket. Queued records are
  ordered by position, smallest first."); `OldestSlotNewestPayload`;
  `SupersedeUnsettledHeldVersion` and `DeadLetterNackedRecord`; the two release rules'
  `for record in pocket.held_records: Freed(...)`; the inspection seam, which no surface
  of the specification names; and scenario S091.
- **The specification says:**
  - A pocket's records are a set, and queued records are ordered by position. No rule
    asks for them to be listed.
  - `OldestSlotNewestPayload`: "A key no consumer has acked or nacked keeps the position
    of its first publish however many times it is superseded, held or not; only its
    rank and payload change." For an unsettled held version with a newer one deferred,
    the Lease end table's last column reads "newer version queued at", wrapped onto "the
    held version's position".
  - The ending rule emits one `Freed` for each record of a set, and each entry is added
    to a queue the specification lists in no order (G38).
- **The double does:**
  - It follows a ruling for the positions. On 2026-11-18 the maintainer accepted N31's
    port, which compares S091's kept positions by key, "for now", and asked for further
    investigation; PD07 takes it up.
  - S091's port, "the entries one lease end creates come out smallest held position
    first, the implementation's pinned order", publishes `z`, `y`, `b` and `a` into one
    pocket, so that their positions run the reverse of the key order, and pulls all
    four. At 10 ms `b` and `z` are published again and deferred; at 20 ms `a` and `y`
    are nacked. At 50 ms the lease is released.
  - The kept positions: the port lists the pocket's records as pairs of key and
    position, sorted by key, and asserts that after the release they are the pairs `b`
    and `z` had before it. That is the ruling's "by key", and N31's note text's
    comparison with the state before, "never as the absolute values that rest on N04".
    It also shows `y` and `a` gone. The Rust's literal list, `[("b", 3), ("z", 1)]`, is
    not asserted. It is in the Rust's snapshot order; this double's snapshot lists
    records in the order they were created (G19), `z` before `b`, so the list would fail
    here as it did for the Python double.
  - The entries: the superseded queue holds `z` then `b`, each at its held rank and
    payload with `shed_at` the release's time (G50), and the dead letters `y` then `a`,
    each with the nacks' time and `expires_at` from the release's time. The release
    replies with `ReleaseRequeuesPocket` and then `SupersedeUnsettledHeldVersion`,
    `DeadLetterNackedRecord`, `SupersedeUnsettledHeldVersion` and
    `DeadLetterNackedRecord`, smallest held position first (G38). That order is more
    than the specification states, and the port labels it `impl-choice` (N29).
- **`N`-note:** agrees with N31 and follows its ruling, and agrees with N29 on the order.
- **Question:** N31's and N29's, which the ruling leaves open. Is the order in which a
  snapshot lists a pocket's records part of the inspection contract, and if so which
  order? And should the specification order the entries one lease end creates, or say
  that their order is not observable?

### G50: a deferred version shed by a later publish takes the publish's `now`, and a held version shed at the lease's end takes `ended_at`

- **Ticket:** GD05.
- **Concerns:** `Supersede`'s held branch, `SupersedeUnsettledHeldVersion`, the Lease end
  comment, and scenarios S045, S082 and S091.
- **The specification says:**
  - `Supersede` sheds the loser with `shed_at: now` and
    `expires_at: now + config.superseded_ttl`, whether the record is queued or held with
    a version deferred.
  - `SupersedeUnsettledHeldVersion` sheds the held version with `shed_at: ended_at` and
    `expires_at: ended_at + config.superseded_ttl`. The Lease end comment: "the three
    rules that create an entry stamp it from that, not from their own now", and "a
    superseded record made here has shed_at at the lease's end".
- **The double does:**
  - A deferred version that a still higher rank supersedes is shed with the publish's
    `now`, the one time that outcome reads (G05). The lease has not ended, and the rule
    reads no lease. S045's port, "two supersedes while held: only the deferred version
    is shed, and the held version never is", sheds `v3` at 20 ms, the second publish's
    time, though the lease ends at 30 ms. S082's port sheds `v3` at 30 ms, the second
    publish's time, for each mark, with the lease still active.
  - A held version shed at the lease's end takes `shed_at` and `expires_at` from the
    `ended_at` that `Freed` carries, and `SupersedeUnsettledHeldVersion` asks for no
    time at all (G46). "SupersedeUnsettledHeldVersion changes exactly what it ensures,
    stamps the entry from ended_at and asks for no time" runs it over a state built by
    hand with `ended_at` 777 and a `superseded_ttl` of 5000, unlike the
    `dead_letter_ttl` of 7000, so that a rule that read the other TTL would show. The
    step is done at once: the rule never asks for the time. S091's port sheds `z` and
    `b` at the release's 50 ms, not at the 10 ms of the publishes that deferred their
    newer versions.
  - "the Freed rules read the mark and never how the lease ended" shows the same entry
    whether the lease was released or expired.
- **`N`-note:** agrees with N30.
- **Question:** none on the stamps, which the rules state. N30's consequence stands: for
  a held key, `shed_at` need not order its superseded versions by rank. A deferred
  version is shed at its publish, and the held version, the lowest rank, only when the
  lease ends, later. Should a reader of the superseded queue be told that `shed_at` does
  not order a key's versions?

### G54: a deferral answers nothing (the maintainer's ruling on N32)

- **Ticket:** GD05.
- **Concerns:** `DeferBehindHeldVersion`, `Supersede`'s held branch, the release rules
  and the six `Freed` rules, the `Publishing` surface, decision 0020's publish table, and
  scenarios S021, S037, S043, S044, S045, S046, S054, S071, S072, S082, S083 and S091.
- **The specification says:** `DeferBehindHeldVersion` ensures state and
  `PocketTouched`, and answers nothing; the surface names no outcome. Decision 0020's
  publish table gives a deferral the answer `superseded`: "Nothing is shed; the
  maintainer chose `superseded` for it at step 1."
- **The double does:**
  - It follows the ruling of 2026-11-19 on N32, as G15 does for the other four Publish
    rules. A deferral replies `Ok(Outcome(["DeferBehindHeldVersion"], None))`. The
    `PocketTouched` it emits finds the pocket leased, and no row of that table holds for
    a leased pocket, so the reply names the one rule. A second higher rank replies
    `Ok(Outcome(["Supersede"], None))`.
  - Every deferral in the twelve ports is asserted to reply exactly that. Where the Rust
    checks it for `Superseded`, the port asserts the state the deferral leaves through
    the snapshot instead: the record's rank, payload and `deferred_position`, the held
    version unchanged and, in S021, S043 and S082, the counter one higher. S021's port
    compares the whole record with the one read just before the deferral, so its
    position, mark and nack time are asserted unchanged too.
  - A release with a deferred version replies with the rules it fired and no answer, as
    in G37. S054's Rust release answers the pocket's fate `Queued`; the port asserts the
    reply `ReleaseRequeuesPocket` then `ConsumeAckedVersionQueueDeferred`, and that the
    pocket is queued.
- **`N`-note:** agrees with N32 and follows its ruling.
- **Question:** N32's, which the ruling leaves open. Decision 0020 fixes the Rust's
  answer and the specification does not state it. Should the `Publishing` surface name
  what a publish answers, so that a double or a second implementation can port it?

### G56: a tick is the double's only clock, and it reports what it did

- **Ticket:** GD06.
- **Concerns:** the three clock triggers, `lease: Lease.expires_at <= now`,
  `shed: SupersededRecord.expires_at <= now` and `letter: DeadLetter.expires_at <= now`,
  and their rules `LeaseLapses`, `SupersededRecordDecays` and `DeadLetterDecays`; the
  Lease expiry comment ("The clock emits LeaseLapsed once per active lease whose
  expires_at has passed"); G05; scenarios S006, S059 and S065.
- **The specification says:** each of the three rules fires when its entity's
  `expires_at <= now`. It names no process that notices this, and no surface reports a
  lapse or a decay.
- **The double does:**
  - It has no background time and no timer. The owner's `Tick` message is the clock:
    `owner.tick` sends it, and the owner takes it in mailbox order like any action.
    `core.Trigger`, the type of every other message that changes state, holds only the
    triggers a surface provides, so no `Act` carries a clock trigger, and only
    `core.tick`, which the `Tick` message calls, reaches the four temporal tables. The
    ticket's Verification checks that nothing under `src/` arms a timer, sleeps or reads
    a wall clock.
  - The owner reads the injected clock once per tick, before any outcome and even when
    nothing is due, and `core.tick` hands that one `now` to every outcome. Each due
    lapse is its own outcome, run through `core.run_at`: `LeaseLapses`, the expiry rule
    that answers its `LeaseLapsed`, and each `Freed`, checked once at its end (G47).
    Each due decay is its own outcome, checked once. "a tick reads the clock exactly
    once, with nothing due and with several things due" counts the reads, and that no
    lease id is drawn.
  - A lease lapses once. A lease a tick lapsed is expired, and the next tick offers it
    nothing: "a lease lapses once: a second tick at the same instant lapses nothing and
    changes nothing". S065's port, "a lapse fires once per lease", ticks again at the
    lapse's instant and at two later times.
  - A tick answers `TickReport(fired, lapsed, superseded, dead_letters)`: the names of
    the rules every outcome fired, in order; the leases it lapsed, in the order they
    lapsed, as the state keeps them once ended; and the superseded records and dead
    letters that decayed, as they stood. A due input whose outcome fired no rule is not
    reported. Where an outcome fails, the owner answers `PartialTick(report, failure)`
    instead (G59).
  - The four tables are named `LeaseLapses`, `LeaseLapsed`, `SupersededRecordDecays`
    and `DeadLetterDecays`. Three of those are the double's own: the specification
    names no temporal trigger, so each clock trigger's table takes the name of its one
    rule. `LeaseLapsed` is the event's name.
  - Every other trigger reads the clock lazily, at most once and only when a rule asks
    (G05).
- **`N`-note:** agrees with N33. It departs in the report's shape: N33's `TickReport`
  has the lapsed leases and the decayed entries, and this one also has `fired`, which
  the coverage check reads, as it reads the owner's reply to an action (G15). Neither
  is the Rust's, which has the lapsed leases and two counts.
- **Question:** N33's. Should the specification, or a surface, name how the temporal
  triggers are processed and what that processing reports?

### G57: one tick lapses in `expires_at` order, then decays in queue order (`impl-choice` where a test asserts it)

- **Ticket:** GD06.
- **Concerns:** `LeaseLapses`, `SupersededRecordDecays` and `DeadLetterDecays`; the
  `Lease` entity's `granted_at`; G19, G24 and G38.
- **The specification says:** every trigger due at `now` fires. It gives no order among
  several due at once.
- **The double does:**
  - First the lapses: the active leases whose `expires_at <= now`, sorted by
    `expires_at` with the standard library's sort, which is stable. Equal `expires_at`
    keep the order of the state's list of leases, which is the order they were created
    in (G19). Then, over the state the lapses left, the superseded records and then the
    dead letters whose `expires_at <= now`, each queue oldest entry first.
  - A tie goes by creation order, not by `granted_at`. Only `PullClaimsFrontPocket`
    creates a lease, at its own `now`, and adds it at the end of the list, so the two
    agree while the clock does not go back. Two leases granted at one instant have
    equal `granted_at` and are ordered by creation. The injected clock may go back,
    and the double does not clamp it, and then the two can disagree. "a tie in
    expires_at lapses in creation order, even against granted_at after the clock
    stepped back" builds that state through the owner: `c1`'s lease granted at
    t0 + 10, `c2`'s at t0 after the clock is set back, `c2`'s renewed at t0 + 10, both
    expiring at t0 + 20. The tick lapses `c1`'s first, by creation, where `granted_at`
    would put `c2`'s first. A scratch sort that broke ties by `granted_at` failed that
    test alone (`ai_tmp/review/GD06-1/ev-C3-tie-by-granted-killed.txt`).
  - "a tick lapses in expires_at order, with equal expires_at in grant order" grants
    `lease-1` to `lease-3` at t0 and `lease-4` at t0 + 3, renews `lease-1` at t0 + 5,
    and ticks at t0 + 20. The report lists `lease-2`, `lease-3`, `lease-4`, `lease-1`.
    The tied pair were granted at the same instant, so the test pins creation order
    against a tie in `granted_at` too. Their consumers, `c3` then `c2`, are not in id
    order, so a tie broken by consumer would show.
  - "two distinct superseded entries due in one tick decay in queue order" leaves S073's
    two entries until both are overdue; one tick decays and reports the one shed first,
    then the other. S074's port, "dead letters decay at the lease's end plus
    dead_letter_ttl", already fails if the due dead letters are reversed
    (`ai_tmp/review/GD06-1/ev-B2-letters-reverse-killed.txt`).
  - "a tick lapses in expires_at order, with equal expires_at in grant order", "a tie
    in expires_at lapses in creation order, even against granted_at after the clock
    stepped back" and "two distinct superseded entries due in one tick decay in queue
    order" are `impl-choice`: the specification orders none of what they assert, and
    each test's comment says so.
  - No expiry rule allots a position, and one active lease per pocket keeps two lapses
    apart. So the order changes no pocket and no record. It shows in the order of the
    report and of `fired`, and in the order in which two lapses' `Freed` rows add
    entries to the shed queues, as G38 says of one end's entries.
  - The decays run only if every lapse succeeded (G59).
- **`N`-note:** agrees with N34, whose ties go "in the order the leases were granted".
  This double states the tie within one instant, which N34 does not.
- **Question:** N34's, as N29's: should the specification order what several lapses in
  one tick create, or say that the order is not observable?

### G58: an entry a lapse creates with a zero TTL decays in the same tick (S007)

- **Ticket:** GD06.
- **Concerns:** `SupersedeUnsettledHeldVersion`, `DeadLetterNackedRecord`,
  `DeadLetterNackedVersionQueueDeferred`, the two decays, `SupersededTtlNonNegative`,
  `DeadLetterTtlNonNegative`, and scenario S007.
- **The specification says:** a zero TTL is admitted, and then `expires_at` equals the
  timestamp it is measured from. An entry a lapse creates at `now` therefore has
  `expires_at = now`, and `expires_at <= now` holds at once.
- **The double does:**
  - `core.tick` finds the due decays in the state its lapses left, at the same `now`.
    So an entry a lapse creates with a zero TTL decays in the tick whose lapse made it,
    and once a tick returns `Ok` nothing is due at its `now`. A tick answered
    `PartialTick` stopped at a failed outcome, and what was due after it, a decay
    included, is still due (G59).
  - "an entry a lapse creates with a zero TTL decays in the same tick" lapses a lease
    over a nacked record and an unsettled one with a newer version deferred. The one
    tick fires `LeaseLapses`, `LeaseExpiryKeepsPocketAtFront`,
    `DeadLetterNackedRecord`, `SupersedeUnsettledHeldVersion`, `SupersededRecordDecays`
    and `DeadLetterDecays`, reports both entries with `expires_at` the tick's `now`,
    leaves both queues empty, and a second tick at the same instant reports nothing.
  - S007's port, "zero superseded and dead-letter TTLs decay at once, the dead letter at
    its lease's end", makes its entries by a supersede and a release, and one tick at
    the same instant decays both.
- **`N`-note:** agrees with N35.
- **Question:** N35's. Is "nothing is due once a tick succeeds" intended? The other
  reading, that an entry made in a tick waits for the next one, also satisfies every
  rule.

### G59: a tick keeps each outcome that succeeded before a failure, and does not roll back

- **Ticket:** GD06.
- **Concerns:** the three clock triggers; the Lease expiry and Lease end comments; the
  checker; shared rule 4 of the GD tickets ("On an `Error` the owner replies with it and
  keeps the state it had"); finding 9 of `double/SPEC-NOTES.md`.
- **The specification says:** each due trigger fires, and the invariants hold after each
  outcome. It does not group the outcomes of one instant, and so says nothing of one
  that fails among several.
- **The double does:**
  - A tick is one message and several outcomes. `core.run_due` runs them in order, each
    over the state the one before left, each to its end and through the checker. At the
    first that fails it stops. The outcomes before it stand, the failed outcome leaves
    no state, and the outcomes after it never run. If a lapse fails, no decay runs.
  - The owner keeps the state the last outcome that succeeded left and answers
    `Error(PartialTick(report, failure))`, where `report` is what the outcomes before
    the failure did. Nothing is rolled back to the state before the `Tick`.
  - **This departs from shared rule 4 for a tick as a whole.** Each outcome holds to
    it: an outcome that fails changes nothing. The `Tick` message does not: the owner
    answers an `Error` and keeps a state the message changed. The ticket's step 5 asked
    for that choice to be made, and this is the one made.
  - An outcome a tick runs must ask for nothing. `run_at` hands it the tick's `now`, so
    a row that asks for the time is answered without a read; one that asks for a lease
    id, or one run without `now` in hand that asks for the time, stops the run as a
    defect. No temporal row asks for a lease id.
  - The tests:
    - `core_test`'s "a run of due outcomes keeps each outcome that succeeded before a
      failure, stops at the failure, and checks each outcome at its end", on scratch
      tables: three outcomes add the publishers `q`, `p` and `r`, and `p` is taken. The
      run keeps `q`, fails with `BrokenInvariant(["UniquePublisherId"])`, and never
      adds `r`. A checker run once at the end of the run would not have stopped it.
    - `core_test`'s "a due outcome that asks for a lease id or for the time is a defect
      of the tick".
    - `expiry_test`'s "a tick whose second lapse fails keeps the first, stops there, and
      decays nothing", through the real tables over a state built by hand: the second
      lapse parks a pocket that has already seen the limit, the checker names
      `ExpiriesWithinLimit`, and the due superseded record stays in its queue.
      `core.tick` is pure and is called directly.
    - `owner_test`'s "a tick whose outcome fails is answered PartialTick, and the owner
      keeps what the outcomes before it did", through the owner. A lease-id source that
      repeats an id leaves two active leases named `lease-2` (G27). The tick lapses
      `lease-1`; its second lapse ends both `lease-2` leases, which leaves pocket `c`
      leased with no active lease, and the checker names `LeasedHasOneActiveLease`. The
      owner answers `PartialTick` with `lease-1`'s lapse, keeps that state, and reads the
      clock once. Its comment calls this one way a tick through the owner can fail
      with the specification's tables.
    - `owner_test`'s "a clock set back after a nack leaves the lease's end before
      nacked_at, so the lapse and a release fail NackedBeforeExpiry" is another, with
      unique lease ids, found by GD07's review. With `lease_ttl` 10 and
      `dead_letter_ttl` 0, `c1` nacks `k` at 100, the clock is set back, and `c1`
      renews at 1, so the lease expires at 11. A tick at 11 lapses it: the lease ends
      at 11, and `DeadLetterNackedRecord` makes the dead letter with `nacked_at` 100
      and `expires_at` 11, as its `ensures` states (`expires_at: ended_at +
      config.dead_letter_ttl`), which breaks `NackedBeforeExpiry`
      (`nacked_at <= expires_at`). The owner answers `PartialTick` with an empty report
      and keeps the state it had; a release at 11 fails the same way. The rows follow
      the rules' text. The specification's comment on the TTL invariants says "A dead
      letter's expires_at is measured from its lease's end, which is at or after its
      nacked_at", which holds only while the clock does not go back, and the
      specification states no such requirement. The double does not clamp the
      injected clock (G57).
  - A failure that stays in the state stops every later tick at the same place. The
    `owner_test` test ticks a second time, meets the same failure, and changes nothing,
    so nothing after the failing outcome, such as a decay, is ever applied.
- **`N`-note:** agrees with N33 and finding 9, which record that the Python double
  commits each lapse and each decay as it goes. `PartialTick` has no `N`-note.
- **Question:** finding 9's: should a tick be atomic as a whole? This double adds two:
  should one outcome that cannot be applied hold back the outcomes due after it, at
  this tick and at every later one? And does the specification mean `now` never to go
  back? If it does, it could say so; if it does not, a lease that ends before a
  `nacked_at` stamped under it cannot end at all while the clock stays there.

### G60: a clock trigger's `expires_at <= now` is the tick's filter, and the trigger carries the tick's `now`

- **Ticket:** GD06.
- **Concerns:** `LeaseLapses`'s `when: lease: Lease.expires_at <= now`, the two decays'
  `when` clauses, the `LeaseLapsed(lease)` event, the three expiry rules'
  `requires: lease.expires_at <= now`, and G05.
- **The specification says:** a clock trigger binds an entity whose `expires_at <= now`.
  `LeaseLapses` ensures `LeaseLapsed(lease: lease)`, which carries no time, and each
  expiry rule that answers it requires `lease.expires_at <= now` again.
- **The double does:**
  - The comparison in each `when` is `core.tick`'s filter: it offers `LeaseLapses` only
    the active leases whose `expires_at <= now`, and each decay only the entries whose
    `expires_at <= now`. `LeaseLapses`'s guard is its `requires: lease.status = active`
    alone, and the decays' guards hold while the entry is in its queue (G64).
  - A guard cannot read the clock (G05), so a `requires` that reads `now` reads it from
    the trigger's input. `LeaseLapses`'s input is `Lapse(lease, now)`, the lease's id
    and the tick's `now`. Its effect asks for `now`, which `run_at` answers with the
    same value, and emits `LeaseLapsed(lease, now)`. The expiry rules' guards compare
    `expires_at` with the `now` the event carries.
  - So the double's `LeaseLapsed` carries a time the specification's does not. It is the
    `now` of the outcome that emitted it. The double reads one outcome as having one
    `now`, shared by the rule a trigger reached and the rules its events reach (G05);
    the specification does not say so, and the event's field makes that reading hold for
    the expiry guards, which cannot ask for the time.
  - "LeaseLapses changes nothing, emits LeaseLapsed with the tick's now, and holds only
    for an active lease" pins the event. "the LeaseLapsed rows hold only for an active
    lease past expires_at at the event's now on a leased pocket, and otherwise nothing
    happens" sends the event a millisecond before `expires_at`, and no row holds.
- **`N`-note:** agrees with N37 that the `<= now` is the tick's filter. The time the
  event carries has no `N`-note.
- **Question:** should the specification say that a rule answering an event reads the
  `now` of the outcome that emitted it, so that an event need carry no time? As
  written, `requires: lease.expires_at <= now` in an expiry rule assumes it.

### G61: the expiry rules' answer when no guard holds is unreachable through the owner

- **Ticket:** GD06.
- **Concerns:** `LeaseLapses`; the three expiry rules' `requires: lease.status = active`,
  `requires: lease.expires_at <= now` and `requires: pocket.status = leased`; the two
  decays; the Lease expiry comment; G23 and G39.
- **The specification says:** "the lapse stops at its guards unless that new expires_at
  has also passed by the time the lapse is processed". It does not say whether a lapse
  that reaches the expiry rules can find none of their guards holding.
- **The double does:**
  - Each of the four temporal tables answers that nothing happens when no guard holds,
    as shared rule 4 of the GD tickets states for the lapse and the two decays. Two of
    those answers are told apart here.
  - The `LeaseLapses` table's. A tick offers only active leases, and while no two leases
    share an id one lapse ends only its own lease, so the guard holds for every lease
    offered. Two leases of one id are G27's case, and G59's `owner_test` test meets
    it. The test "a lease lapses once: a second tick at the same instant lapses nothing
    and changes nothing" shows that an expired lease is not offered again, not that the
    table refuses it. One mutant from the mutation pass offered each due lease twice
    and survived as equivalent: the second offer stops at this guard, and nothing
    happens.
  - The `LeaseLapsed` table's. `LeaseLapses` and its expiry rule are one outcome, so
    the expiry rules read the state the tick filtered, and their restated guards hold.
    Under branch A a late action's effect shows at the filter, never at those guards:
    a renewed lease is due only once its new `expires_at` has passed, at once when
    `lease_ttl` is 0 (S006, S066, S067), and then the guards hold; a released one is
    not active (G63). So through
    the owner the answer is not reached. "the LeaseLapsed rows hold only for an active
    lease past expires_at at the event's now on a leased pocket, and otherwise nothing
    happens" reaches it through `core.run_at`, over a state built by hand: the event a
    millisecond before `expires_at`, the lease released or expired, the pocket queued,
    idle or parked, and an id that names no lease.
  - Dispatch evaluates all three expiry guards on every lapse and returns
    `GuardOverlap` if two hold, so each lapse through the owner shows that exactly one
    answers.
- **`N`-note:** agrees with N37. N37's hand-built cases are a renewed, a released and
  an expired lease and a pocket that is not leased; this double's add an id that names
  no lease.
- **Question:** N37's, as N14's and N26's: are the expiry rules' `lease.status = active`
  and `lease.expires_at <= now` guards in their own right, or restatements of what
  `LeaseLapses` already established in the same chain?

### G63: branch A in both orders, competing callers, and the break that lapses a lease from an action

- **Ticket:** GD06.
- **Concerns:** the Lease expiry comment; decision 0019's branch A; `Renew`,
  `ReleaseRequeuesPocket`, `AckMarksRecord` and `PullWithinLease`; `LeaseLapses`;
  scenarios S066 and S067; G32.
- **The specification says:** a renewal, ack, nack or pull processed first "moves
  expires_at to now + lease_ttl", and a release processed first ends the lease; "the
  guards above decide in either order".
- **The double does:**
  - S066 and S067 have two halves each, in four Rust tests, and each half is its own
    registered test carrying the scenario's tag: S066's "a renew after expires_at but
    before the lapse succeeds (branch A)" and "a release after expires_at but before the
    lapse succeeds (branch A)", and S067's "an ack after expires_at but before the
    lapse succeeds (branch A)" and "a pull after expires_at but before the lapse extends
    the lease (branch A)".
  - The Rust has no late nack. "a nack after expires_at but before the lapse succeeds
    and extends the lease from its own time (branch A)", with no scenario tag, runs the
    same two orders for a nack, and asserts the lease extended to t0 + 21, from the
    nack's time. A scratch nack that extended from the passed `expires_at` survived
    every test before it and fails it
    (`ai_tmp/review/GD06-1/ev-B1-nack-min-survives.txt`, `ev-B1-nack-min-killed.txt`).
  - The Rust forks one queue into two. The double cannot fork an owner, so each half
    builds the same setup on two owners, asserts their snapshots equal, and drives both
    from the one test process, whose messages each owner takes in the order they were
    sent. On the first the `Tick` goes first: it lapses the lease, and the action behind
    it is fenced, or for the pull claims the pocket under a new lease. On the second the
    action goes first: it succeeds and moves `expires_at` on, or for the release ends
    the lease, and the `Tick` behind it lapses nothing.
  - An action never lapses a lease: no action's row emits `LeaseLapsed`. That is a
    property of the rows, not of a type: `LeaseLapsed` is an `Event`, and the apply
    step answers it from whichever row emits it.
  - The break the ticket's step 4 asks for is recorded in
    `ai_tmp/gd/GD06/ev-a-break.txt`. A scratch change to `settle.gleam` made `Renew`
    emit `LeaseLapsed` for a lease past its `expires_at`, so the renew lapsed its own
    lease. `just gleam-double` exited with status 1, `253 tests run, 251 passed`: S066's
    renew half failed, its renew answering `Renew`, `LeaseExpiryKeepsPocketAtFront` and
    `RequeueUnsettledRecord`, and so did the competing-callers test. The file was
    restored and its SHA-256 matched.
  - "a renew and a tick sent at once by two callers agree whichever the owner takes
    first", with no scenario tag, starts a process that renews and one that ticks and
    lets both go at once from gates. It runs twenty times, ten with each started first.
    Each run asserts an empty `check` and one of two consistent results: the renew
    succeeded, the tick lapsed nothing and the lease is active, expiring at t0 + 21; or
    the tick lapsed the lease, the renew was fenced, and the pocket and its record are
    queued. It never asserts which came first, and it cannot show that both orders
    occurred. A scratch print showed both, but its output was not saved.
- **`N`-note:** agrees with N33 that an action never lapses a lease, and with N38's
  ruling for S066's release half (G67). The competing callers have no `N`-note.
- **Question:** none on the behaviour, which decision 0019 closed. As G32: should the
  catalog hold a row in which an action and a lapse compete?

### G65: only a publish revives a parked pocket, as the rules stand (`oq:7`)

- **Ticket:** GD06.
- **Concerns:** `ReviveParkedPocket`, open question 7 ("Can anything other than a
  publish revive a parked pocket, such as an operator action or a cooldown?"), the
  comment above `PublishNewKey`, and scenarios S024, S025, S026, S034, S062 and S092.
- **The specification says:** `ReviveParkedPocket` answers `PocketTouched`, which only
  `PublishNewKey`, `Supersede` and `DeferBehindHeldVersion` emit. A stale publish and a
  publish at the stored rank emit nothing.
- **The double does:**
  - `ReviveParkedPocket` is the second row of the `PocketTouched` table, beside
    `ReviveIdlePocket`; the two share one helper and differ in the status they require.
    Nothing but those three Publish rules emits `PocketTouched`.
  - S062's and S024's ports revive a parked pocket by a new key and by a supersede.
    S025's and S092's show that a refused equal-rank publish and a stale publish leave
    it parked with its count. S034's shows a parked pocket never claimed.
  - S026's port, "only a publish revives a parked pocket (oq:7)", is labelled `oq:7` in
    its comment and its name. Ticks at 1,000 ms and at 1,000,000 ms, and another
    pocket's lease, publish and release, leave `a` parked with its count of 3.
  - "ReviveParkedPocket changes exactly what it ensures and reads no clock" compares the
    complete state.
  - The double adds no cooldown and no operator action. The question stays open.
- **`N`-note:** agrees with N36.
- **Question:** open question 7 itself. If anything else may revive a parked pocket, it
  needs a rule, and S026 changes with it.

### G67: S066's release half, S093 and S075 are ported as the Rust's inspection shape (the maintainer's ruling on N38)

- **Ticket:** GD06.
- **Concerns:** for S066's release half, `ReleaseRequeuesPocket` and the `Lease`
  entity's `terminal: released, expired`; for S093, the Record comment and
  `OldestSlotNewestPayload`; for S075, the Pocket comment ("Queued pockets are ordered
  by position, smallest first. That order is the pocket order; nothing else positions a
  pocket."), `UniquePocketPosition`, `UniqueRecordPosition`, `PocketPositionsBelowNext`
  and `RecordPositionsBelowNext`; the inspection seam.
- **The specification says:** nothing of the Rust's snapshot shape.
- **The double does:**
  - It follows a ruling, not a reading of its own. On 2026-11-19 the maintainer accepted
    the Python double's ports of these three rows as the Rust's inspection shape and
    added them to PD07's scope.
  - S066's release half asserts that the released lease stays in the state, released
    with its `ended_at`, and that no lease is active, where the Rust asserts an empty
    registry (G37).
  - S093's port compares the pocket's records by key, where the Rust lists them in its
    snapshot order (G49).
  - S075's port asserts, at each step, the pocket positions distinct and below
    `next_position`, the record positions distinct within each pocket and over all of
    them and below `next_position`, and an empty `check`. The Rust also checks its
    stored pocket order, which the double does not store.
  - S075's Rust comment says that `a` lapses three times and parks. Traced through the
    rules, `b` keeps the front after its first lapse, so the loop lapses `b` twice more,
    which parks it, and then `a` once. The port asserts the rules each step fired, as
    they trace, where the Rust asserts no fate.
- **`N`-note:** agrees with N38 and follows its ruling.
- **Question:** none on the behaviour. N38's stay open in PD07, which may change these
  ports. Should S075's comment in the Rust say which pocket parks?

## Where it departs from an `N`-note

The note's `N`-note line says it departs: it holds a fact by other means than its `N`-note
does. The findings, under "Where the second reading departs", give the specification text
that left each means open.

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
  - So `ensures: record.held_rank = null` and the four lines like it, in each rule that
    queues a held record, are one change of variant: the record becomes `RecordQueued`,
    and the five fields go with `RecordHeld`. `DeferBehindHeldVersion` sets
    `deferred_position` inside the held variant and leaves its other four fields as they
    were (G51).
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

### G35: `MarkConflict` carries no mark

- **Ticket:** GD04.
- **Concerns:** the Ack and nack comment; scenario S085.
- **The specification says:** "The first mark stands: a repeat of it succeeds and
  changes nothing but the lease's expires_at, and no rule covers an ack of a nacked
  record or a nack of an acked one, so that action is rejected."
- **The double does:**
  - An ack of a nacked record, or a nack of an acked one, is refused
    `MarkConflict(action, lease_id, record_key)`. It carries no mark: the action says
    which mark stands. `Ack` is refused because the record is nacked, and `Nack` because
    it is acked.
  - The reason is the double's module graph, not the specification. `Mark` is defined in
    `model`, `model` imports `config`, and `config` imports `refusals`, so `refusals`
    cannot import `model` without a cycle, and a refusal cannot carry a `Mark`.
  - S085's port, "a changed mark is refused as a mark conflict and changes nothing, not
    even the lease's expiry, and reads no clock", refuses both, and the lease view then
    shows both first marks. "a refused mark names the first requirement that fails, the
    lease first, and asks for no time" refuses both over a state built by hand.
- **`N`-note:** departs from N21, whose `MarkConflict` carries the mark that stands. A
  caller learns the same from either, since the action leaves one mark possible.
- **Question:** as G34. The field is the double's, and the specification names no
  refusal.

## Where it raises what no `N`-note holds

The note's `N`-note line says "no `N`-note": the Python double did not record the point,
or could not show it.

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
    competing-callers tests (G32) assert that order, which is `impl-choice`. GD04 keeps
    it for the dead letters and the records: `DeadLetterNackedRecord` adds its entry at
    the end of the dead-letter queue, a lease that ends changes in its place, and a
    record a `Freed` rule removes leaves the others in the order they had. "a release
    frees the held records smallest position first, each by its mark, and stamps every
    entry from the lease's end" asserts both, which is `impl-choice` too (G38).
  - GD05 keeps it. `DeferBehindHeldVersion` changes the record in its place.
    `SupersedeUnsettledHeldVersion` adds its entry at the end of the superseded queue and
    `DeadLetterNackedVersionQueueDeferred` its entry at the end of the dead-letter queue,
    and each of the three rows that queue a deferred version changes the record in its
    place, whatever position it is given. Two GD05 ports assert a shed queue with more
    than one entry, in order, and each marks it `impl-choice`: S082's superseded queue,
    `v3` then `v4`, for each mark, and S091's two shed queues, whose order is G38's and
    G49's.
  - GD06 keeps it. A lapse ends its lease in its place, as a release does, and its
    `Freed` rows add entries as G38 says. A decay removes one entry and keeps every
    other in its order (G64). The order in which one tick takes its due leases and
    entries is G57's, and rests on this one.
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
  - It follows the rules. The rows of `PublishNewKey`, `Supersede` and
    `DeferBehindHeldVersion` finish with the event, and the rows of
    `PublishIntoNewPocket` and `DropStalePublish` with none. The test "PublishNewKey,
    Supersede and DeferBehindHeldVersion touch the pocket, and the other two emit
    nothing" asserts the events of each row.
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
    repeated an id would leave two leases of one id in a state the rules reach. GD06's
    `owner_test` builds one (G27).
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
      stream as a consumer's action. In these three tests only a `Pull` competes with a
      `Pull`, and the clock stands still throughout. GD06 adds the case: a renew
      competes with a `Tick` in `branch_a_test` (G63), and what the double's one mailbox
      does not do of it is G62's.
    - That an action's `now` is the time the owner reached it (G05).
  - G03 records the one earlier test with competing callers, of a join.
- **`N`-note:** no `N`-note: the Python double has no callers that compete.
- **Question:** should the specification state, as a guarantee of its own, that actions
  are applied one at a time, each to the state the one before it left? Every rule
  assumes it, and only the Lease expiry comment says it. Should the catalog hold a row
  in which two consumers pull at the same moment?

### G41: `ReleaseIdlesEmptyPocket` holds for a leased pocket with no record

- **Ticket:** GD04.
- **Concerns:** `ReleaseIdlesEmptyPocket`'s
  `requires: pocket.records.count = pocket.ending_records.count`, and the comments above
  it and above `LeaseExpiryIdlesEmptyPocket`.
- **The specification says:** the comment above the rule: "Empty once the lease has
  ended: every record the pocket has is held, acked or nacked, with no newer version
  deferred behind it, and the Freed each one is sent removes it." The comment above
  `LeaseExpiryIdlesEmptyPocket`: "No lease is granted on an empty pocket, and a pocket
  loses records only when a lease ends (see PullWithinLease), so such a pocket still has
  a record".
- **The double does:**
  - The guard is taken as written, so a leased pocket with no record at all satisfies
    it, `0 = 0`. Its release ends the lease, sends no `Freed` and idles the pocket with
    its count reset.
  - No state the rules reach has such a pocket, by the comment's reasoning. Only a state
    built by hand does. "ReleaseIdlesEmptyPocket changes exactly what it ensures, reads
    the clock once and draws no lease id" uses one, a pocket that has seen two expiries,
    to show the reset on its own; and "each release rule's guard holds only for the
    consumer's active lease on a leased pocket, by whether a record outlasts it" sends
    that release to `ReleaseIdlesEmptyPocket`.
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour: the comment's "every record the pocket has" holds
  of no records. Should the comment say that a pocket with none idles too, so that the
  guard and the words agree outright?

### G42: the mark rules require nothing of `lease.pocket`

- **Ticket:** GD04.
- **Concerns:** `AckMarksRecord`, `AckRepeats`, `NackMarksRecord` and `NackRepeats`; the
  release rules' `requires: pocket.status = leased`; `UnleasedHasNoActiveLease`.
- **The specification says:** each mark rule has `let pocket = lease.pocket`, and reads
  the pocket only through `Record{pocket: pocket, key: record_key}` and, in the two
  marking rules, `ensures: pocket.consecutive_expiries = 0`. The two release rules have
  the same `let` and then `requires: pocket.status = leased`. No mark rule requires
  anything of the pocket.
- **The double does:**
  - The mark guards read no pocket. Each looks up the record by the lease's pocket key
    alone (G09), so a mark's guard holds whatever the pocket's status, and whether or not
    a pocket has the key.
  - The reset sets `consecutive_expiries` to 0 on every pocket with that key (G09), and
    on no other. "AckMarksRecord changes exactly what it ensures, reads the clock once
    and draws no lease id" and "NackMarksRecord changes exactly what it ensures, reads
    the clock once and draws no lease id" show that only the lease's pocket's count
    changes, over a state built by hand whose idle pocket has a count of its own.
  - In every state the rules reach the difference cannot show:
    `UnleasedHasNoActiveLease` makes an active lease's pocket leased. No test marks
    under an active lease whose pocket is not leased.
- **`N`-note:** no `N`-note.
- **Question:** are the release rules' `requires: pocket.status = leased` and the mark
  rules' silence meant to differ? Either the mark rules could state it, or the release
  rules could drop it as a restatement of the invariant (G39).

### G43: a nacked record without `nacked_at` cannot become a dead letter

- **Ticket:** GD04.
- **Concerns:** `DeadLetterNackedRecord` and `DeadLetterNackedVersionQueueDeferred`, the
  `DeadLetter` entity, the `Record` entity's `nacked_at`, and `NackedHasItsTime`.
- **The specification says:** `DeadLetterNackedRecord` has `let nacked_at =
  record.nacked_at` and creates a `DeadLetter` with `nacked_at: nacked_at`, and
  `DeadLetterNackedVersionQueueDeferred` has the same two lines. On a record it is
  `nacked_at: Timestamp? when status = held`; on a `DeadLetter` it is
  `nacked_at: Timestamp`, with no `?`. Neither rule has a `requires` on it.
  `NackedHasItsTime` is `status = held and mark = nacked implies nacked_at != null`.
- **The double does:**
  - Each rule's guard is the specification's four `requires` and nothing more: the
    record exists, it is held, its mark is `nacked`, and nothing is deferred behind it,
    for `DeadLetterNackedRecord`, or a newer version is, for
    `DeadLetterNackedVersionQueueDeferred`. GD05 lands the second. Each effect reads the
    record's `nacked_at`, through one helper the two rows share. Where it is `None`, the
    effect answers a defect, `Impossible`, naming the record, and no dead letter is
    made: an optional time cannot fill a field that is not optional. Where it is `Some`,
    the dead letter carries it. `DeadLetterNackedVersionQueueDeferred` reads the arrival
    position first (G53), so a record without either is a defect for the arrival
    position.
  - `NackedHasItsTime` is checked after every outcome, and `NackMarksRecord` is the only
    rule that nacks, with `nacked_at = now`. So no state the rules reach holds a nacked
    record without the time, and the defect is never met through the owner.
  - "a Freed rule's effect without what its guard saw is a defect" shows the defect for
    each of the two rules over a state built by hand. "DeadLetterNackedRecord changes
    exactly what it ensures, stamps the entry from ended_at and asks for no time" and
    S086's "a repeat nack keeps the first nack's time" show the time carried over, and so
    do "DeadLetterNackedVersionQueueDeferred changes exactly what it ensures, stamps the
    entry from ended_at and asks for no time" and S072's "a key nacked, then published
    again within the same lease, waits until the lease ends; the dead letter appears
    then".
- **`N`-note:** no `N`-note.
- **Question:** should each of the two rules state `requires: record.nacked_at != null`,
  or is `NackedHasItsTime` meant to carry it? As written each rule's `let` types a
  `Timestamp?` into a `Timestamp`, and only the invariant makes that sound.

### G44: a dead letter's `held_rank` and the record's `rank` cannot be told apart in a state that holds the invariants

- **Ticket:** GD04.
- **Concerns:** `DeadLetterNackedRecord`'s `rank: held_rank` and `payload: held_payload`;
  `HeldVersionNotNewer`, `DeferredHasArrivalPosition` and `HeldCopiesAgree`.
- **The specification says:** the rule creates the `DeadLetter` with
  `rank: held_rank` and `payload: held_payload`, from `let held_rank = record.held_rank`
  and `let held_payload = record.held_payload`, and requires
  `record.deferred_position = null`. The invariants: `held_rank <= rank`;
  `held_rank < rank implies deferred_position != null`; and, of `HeldCopiesAgree`, "With
  nothing deferred, the held version is the record's version, and its two copies agree."
- **The double does:**
  - The dead letter takes the held rank and the held payload, as written.
  - With nothing deferred the three invariants make `held_rank` equal `rank` and
    `held_payload` equal `payload`. So no state the rules reach shows which pair the rule
    reads, and a `DeadLetterNackedRecord` that took `rank` and `payload` would pass every
    other test.
  - From GD05 the two dead-letter rows build the entry through one helper, and
    `DeadLetterNackedVersionQueueDeferred` meets the two pairs apart in states the rules
    reach. S072's port, "a key nacked, then published again within the same lease, waits
    until the lease ends; the dead letter appears then", asserts a dead letter at the
    held rank 1 and payload `v1` while the record is queued at rank 2. A helper that
    took `rank` and `payload` fails it; a change to `DeadLetterNackedRecord`'s row alone
    still does not.
  - "a dead letter carries the held version, not a newer one the record has" runs the
    rule alone over a before-state built to tell them apart: the record's rank is 50 and
    its payload another, while its held version is at 43, with no arrival position. That
    state breaks `DeferredHasArrivalPosition` and `HeldCopiesAgree` before the rule runs.
    The rule removes the record, so the state it leaves holds every invariant, and the
    dead letter carries the held version.
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour. The rule's `held_rank` and `held_payload` are the
  version the consumer was handed, which the Record comment names; only a deferred
  version, in `DeadLetterNackedVersionQueueDeferred`, makes the two pairs differ.

### G45: a revived pocket's position is above its new record's

- **Ticket:** GD04.
- **Concerns:** `ReviveIdlePocket`, `PublishNewKey`, `PublishIntoNewPocket`, the
  `PocketTouched` event, and scenario S023.
- **The specification says:**
  - `PublishNewKey` takes `let position = queue.next_position`, creates the record
    there, ensures `queue.next_position = position + 1` and
    `PocketTouched(pocket_key: pocket_key)`.
  - `ReviveIdlePocket` answers `PocketTouched`, requires `pocket.status = idle`, takes
    `let position = queue.next_position`, and ensures `pocket.position = position` and
    `queue.next_position = position + 1`.
  - `PublishIntoNewPocket` gives the pocket `queue.next_position` and its record
    `queue.next_position + 1`.
- **The double does:**
  - The revival is a row of the `PocketTouched` table and runs after `PublishNewKey`'s
    row in the same outcome, over the state that row left. So it reads the counter the
    publish advanced: the record takes the counter's value and the pocket the next. A
    revived pocket's position is therefore above its new record's, the reverse of a new
    pocket's, whose position is below its first record's.
  - S023's port, "a publish revives an idle pocket at the back", asserts both
    positions against the counter, as its Rust test does: the record at `next`, the
    pocket at `next + 1`, and the pocket at the back of the pocket order.
  - No invariant compares a pocket's position with a record's (G18), so either order
    holds them all.
  - GD06's `ReviveParkedPocket` is the same row with `parked` in place of `idle`, and
    a revived parked pocket's position is above its new record's in the same way. S062's
    port, "three consecutive lapses park the pocket, and a publish revives it", asserts
    the pocket at the back after the new key's position.
- **`N`-note:** no `N`-note. N27 says only that a revival gives the pocket a fresh
  position.
- **Question:** is the difference intended? It follows from the event being answered
  after the rule that emits it, which the rules imply and the specification does not
  state as a rule of its own.

### G46: one module owns the lease's end, and `ending_records` stays in `model`

- **Ticket:** GD04.
- **Concerns:** the Renew and release comment, the Lease end comment and its table,
  `Pocket.ending_records`, the two release rules, and the six `Freed` rules; scenario
  S054.
- **The specification says:**
  - Of a release: "The pocket is queued again when a record outlasts the lease and idle
    when none does: the records the lease's end removes are the pocket's
    ending_records."
  - `ending_records` is `records where status = held and mark != unsettled and
    deferred_position = null`. The four rules that end a lease and decide the pocket's
    fate compare `pocket.records.count` with `pocket.ending_records.count`.
  - "The rules read the mark and never how the lease ended, so a lapse keeps every ack
    and nack." And: "Freed carries ended_at, the timestamp the rule that ended the lease
    wrote to lease.ended_at, and the three rules that create an entry stamp it from that,
    not from their own now".
- **The double does:**
  - `lease_end.gleam` owns the lease's end. Its interface, which GD05 added to without
    changing and GD06 uses as it stands:
    - `Ending`, `Release` or `Lapse`, how the lease ended;
    - `end(state, lease, ending, at)`, the steps the five ending rules share: the lease
      takes `LeaseReleased(at)` or `LeaseExpired(at)`, and the caller is handed one
      `Freed` to emit for each held record of its pocket (G38). It leaves the pocket to
      the calling rule;
    - `outlasts(state, pocket_key)`, the comparison
      `pocket.records.count > pocket.ending_records.count`, which both release rules
      and the two expiry rules below the parking threshold read;
    - `Freeing(lease, record_key, ended_at)`, the input of `Freed`, and `freed()`, its
      table. GD04 landed the three rows with nothing deferred, with a comment where each
      deferred row would go. GD05 puts the three deferred rows in those places, so the
      table holds the six in the specification's order. A `Freed` no row answers is a
      defect.
  - GD05 changed neither the interface nor the release callers. A held record with a
    version deferred is not one of `ending_records`, so `outlasts` counts it as a record
    that outlasts the lease. S054's port, "a release with only an acked record and its
    deferred version requeues the pocket rather than idling it", shows a release then
    fire `ReleaseRequeuesPocket`, not `ReleaseIdlesEmptyPocket`.
  - GD06 left `lease_end.gleam` unchanged. Each of its three expiry rows calls
    `end(state, lease, Lapse, now)` with the tick's `now` and sets the pocket's status
    and count itself; `LeaseExpiryKeepsPocketAtFront` and `LeaseExpiryIdlesEmptyPocket`
    split on `outlasts`, and `LeaseExpiryParksPocket` reads neither. No expiry row adds
    a `Freed` row or repeats a step of the end. The six ports that end a lease either
    way (G66) show the same `Freed` rules by lapse as by release, and each entry
    stamped from its own end's `ended_at`.
  - `Ending` decides the lease's status alone, and `Freeing` has no field for it. The
    interface does not stop a row from reading it: each row finds the whole lease by
    the event's id, `status` included, and a row could branch on it. None does. Once
    the lease is found, the six rows read its `pocket` and `consumer` and nothing else
    of it. "the Freed rules read the mark and never how the lease ended" runs each row
    over a lease ended either way and finds the same result.
  - Each row finds the lease by the event's id whatever its status, and stamps any entry
    from the event's `ended_at`, never from a `now` of its own: no `Freed` row asks for
    the time. "DeadLetterNackedRecord changes exactly what it ensures, stamps the entry
    from ended_at and asks for no time" sends an `ended_at` unlike every time in the
    state, and so do the tests of the two deferred rows that create an entry (G50).
  - `ending_records` is computed in one place, `model.ending`, landed in GD02. The
    snapshot's derived sets read it there, for the checker's views, and so does
    `lease_end.outlasts`; the module does not repeat it.
- **`N`-note:** no `N`-note. N25 names the Python double's `lease_end.end_lease`, and
  says nothing of the module's interface.
- **Question:** none on the behaviour. The specification writes the comparison out in
  each of the four rules that split on it; is it meant to be the same comparison in
  each, as it reads?

### G47: the witness that the checker runs once, after the whole chained outcome

- **Ticket:** GD04.
- **Concerns:** the Lease end comment's "one outcome, as a pull and its HoldRecord are,
  so the invariants hold after the whole of it"; `ReviveIdlePocket`; the two release
  rules and the `Freed` rules; `IdleHasNoRecords` and `OutstandingOnlyWhileLeased`.
- **The specification says:** "The rule that ends a lease and the rules that answer its
  Freed are one outcome, as a pull and its HoldRecord are, so the invariants hold after
  the whole of it." It says nothing of the state part-way through.
- **The double does:**
  - The apply step runs the checker once, over the state the whole chain left. GD02's
    review and GD03's hand-back left a witness owing: an outcome whose state part-way
    breaks an invariant that the end mends. A pull has none (G22). GD04 has three, each
    a test in `core_test.gleam` with no scenario tag, each driving the owner:
    - "a publish of a new key into an idle pocket stands, though the pocket is idle
      with a record until the revival answers": after `PublishNewKey`, before
      `ReviveIdlePocket`, the pocket is idle with a queued record, which breaks
      `IdleHasNoRecords`;
    - "a release that requeues its pocket stands, though the pocket is queued with a
      held record until Freed is answered": after `ReleaseRequeuesPocket`, before
      `RequeueUnsettledRecord`, the record is held in a queued pocket, which breaks
      `OutstandingOnlyWhileLeased`;
    - "a release that idles its pocket stands, though the pocket is idle with a held
      record until Freed is answered": after `ReleaseIdlesEmptyPocket`, before
      `ConsumeAckedRecord`, the pocket is idle with a held record, which breaks both.
  - Two scratch changes to `core.gleam` show that the tests tell the checker's place.
    The file was restored afterwards with the SHA-256 it had before.
    - one that ran the checker before every chained event's table failed all three.
      The two release tests failed with the invariants above. The publish test failed in
      its setup, the idling release, naming both of that release's invariants;
    - one that ran it before `PocketTouched`'s table alone failed the publish test at
      the publish, naming `IdleHasNoRecords` alone, and passed the two release tests.
  - GD06's lapse is the third caller of the lease's end, and its outcome passes through
    the same part-way states: after `LeaseExpiryKeepsPocketAtFront`, before
    `RequeueUnsettledRecord`, a held record stands in a queued pocket. A tick runs each
    lapse through `core.run_at`, which ends in the same `conclude` as every outcome, so
    the checker runs once at the end of each lapse and never between `LeaseLapses`, its
    expiry rule and the `Freed` rows. No GD06 test is a witness of its own for the
    lapse, and the two scratch changes were not run again. `core_test`'s "a run of due
    outcomes keeps each outcome that succeeded before a failure, stops at the failure,
    and checks each outcome at its end" shows the other side: the checker runs at the
    end of each of a tick's outcomes, not once at the end of the tick (G59).
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour, which the comment states for the lease's end. The
  comment names a pull and a lease's end; should it say the same of a publish and the
  revival it causes, whose part-way state also breaks an invariant?

### G51: the arrival position is allotted at the deferral, never at the lease's end, and the deferral changes only what it ensures

- **Ticket:** GD05.
- **Concerns:** `DeferBehindHeldVersion`; the Queue comment; the Position vocabulary
  entry; `Supersede`'s comment; `ConsumeAckedVersionQueueDeferred` and
  `DeadLetterNackedVersionQueueDeferred`; scenarios S043, S044, S045 and S082.
- **The specification says:**
  - `DeferBehindHeldVersion` takes `let position = queue.next_position` and ensures
    `record.rank = rank`, `record.payload = payload`,
    `record.deferred_position = position`, `queue.next_position = position + 1` and
    `PocketTouched(pocket_key: pocket_key)`. It reads no `now`.
  - Its comment: "The loser is the held version. It is not shed and nothing about it
    changes: it stays in the held set, at held_rank and held_payload, whatever its mark,
    until the lease ends".
  - The Queue comment: "A deferred version's arrival position is allotted the same way
    when it arrives, and the two rules that queue it there when the lease ends assign
    that position without advancing the counter again." And above `Supersede`: "A
    deferred version's arrival position stays with the record, as a queued record's
    position does."
- **The double does:**
  - `DeferBehindHeldVersion`'s effect asks the apply step for one position, the only
    place positions are allotted (G18), and sets it as `deferred_position` inside the
    held variant (G14). It changes the record's rank and payload, and nothing else of
    the record: its position, `held_rank`, `held_payload`, mark and `nacked_at` stay
    (G21). It asks for no time.
  - No `Freed` row asks for a position. `ConsumeAckedVersionQueueDeferred` and
    `DeadLetterNackedVersionQueueDeferred` queue the record at the `deferred_position`
    it holds, and `SupersedeUnsettledHeldVersion` at its own position (G52). Each
    `Freed` row leaves the counter as it receives it. That need not be where the
    deferral left it: a later publish may allot positions (G18), and
    `ReleaseRequeuesPocket` allots the pocket's before the lease ends.
  - The held branch of `Supersede` neither allots nor clears the arrival position: the
    newer version takes the record's rank and payload, and `deferred_position` stays.
  - The tests:
    - "DeferBehindHeldVersion changes exactly what it ensures, reads no clock and draws
      no lease id" runs the rule through the apply step over a state built by hand, once
      for each mark. It compares the complete state: the counter from 6 to 7, the
      arrival position 6, and every other field and entity as it was. The step reads no
      clock and draws no lease id.
    - S043's port asserts the arrival position against the counter before the publish,
      and the counter one higher after it.
    - S044's port shows the arrival is allotted when the newer version arrives: `k`
      is deferred after `early` is published and before `between` is, and once the lease
      ends the next pull hands out `early`, `k` and `between`, in that order.
    - S045's and S082's ports show the second supersede keep the arrival position.
    - The three "… changes exactly what it ensures …" tests of the deferred `Freed`
      rows compare the complete state, the counter included.
- **`N`-note:** no `N`-note. N32 records the same state a deferral leaves, the counter
  advanced by one and the held version unchanged, and says nothing of when the arrival
  position is allotted.
- **Question:** none on the behaviour, which the Queue comment states.

### G52: `SupersedeUnsettledHeldVersion` queues the record at its own position, which no `ensures` names

- **Ticket:** GD05.
- **Concerns:** `SupersedeUnsettledHeldVersion`, the Lease end table,
  `OldestSlotNewestPayload`, and the three rules that queue a deferred version;
  scenario S091.
- **The specification says:** `SupersedeUnsettledHeldVersion` ensures the
  `SupersededRecord`, `record.status = queued` and the five held fields null. It has no
  `ensures` on `record.position`, where the two other rules that queue a deferred
  version ensure `record.position = arrival_position`. The Lease end table's row puts the
  newer version at the held version's position (G49), and `OldestSlotNewestPayload` says
  a key no consumer has acked or nacked "keeps the position of its first publish". None
  of the three rules names `record.rank` or `record.payload`.
- **The double does:**
  - It reads the rule as G21 reads every rule: a field no `ensures` names is unchanged.
    The record is queued at the position it had while held, which is the held version's,
    and keeps the rank and payload, which are the newer version's. The other two rules
    queue the record at its arrival position, and keep its rank and payload too.
  - "SupersedeUnsettledHeldVersion changes exactly what it ensures, stamps the entry
    from ended_at and asks for no time" compares the complete state: the record queued
    at 6, where it was held, with the newer version's rank and payload. S091's port
    shows `b` and `z` keep their positions (G49).
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour, which the table and the guarantee state. Should
  the rule ensure the position, or say that it keeps it, as the other two rules ensure
  theirs?

### G53: the arrival position the queue-deferred rows read is optional, and only their guard promises it

- **Ticket:** GD05.
- **Concerns:** `ConsumeAckedVersionQueueDeferred` and
  `DeadLetterNackedVersionQueueDeferred`, the `Record` entity's `deferred_position`, and
  `DeferBehindHeldVersion`'s effect.
- **The specification says:** each of the two rules has
  `requires: record.deferred_position != null`, then
  `let arrival_position = record.deferred_position` and
  `ensures: record.position = arrival_position`. The field is
  `deferred_position: Integer? when status = held`, and a record's `position` is an
  `Integer`.
- **The double does:**
  - Unlike `nacked_at` (G43), the guard promises the value: a row's effect runs only
    once its guard has held. Each effect reads `deferred_position` again, through one
    helper the two rows share. Where it is `None`, which only a wrong guard could bring
    about, the effect answers a defect, `Impossible`, naming the record, and the record
    is not queued. Where it is `Some`, the record is queued there.
  - "a Freed rule's effect without what its guard saw is a defect" runs each of the two
    rows under a guard that always holds, for a held record with nothing deferred, and
    shows the defect.
  - `DeferBehindHeldVersion`'s effect reads the record its guard saw in the same way.
    Where there is no record, or the record is queued, it answers `Impossible`.
    "DeferBehindHeldVersion's effect without its held record is a defect, and reads no
    clock" shows both, over states built by hand. Through the owner none of these
    defects is met.
- **`N`-note:** no `N`-note.
- **Question:** none. The guard makes the `let` sound, as the rules are written.

### G55: each arrival-position invariant's premise is met in a state the rules reach

- **Ticket:** GD05.
- **Concerns:** `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`,
  `DeferredArrivedAfterHeld`, `HeldCopiesAgree`, `ArrivalPositionUnoccupied`,
  `UniqueArrivalPosition` and `ArrivalPositionsBelowNext`.
- **The specification says:** each is an implication whose premise names a held record:
  `held_rank < rank`, `held_rank = rank`, `deferred_position != null` or
  `deferred_position = null`, and for `UniqueArrivalPosition` two distinct held records
  each with a `deferred_position`.
- **The double does:**
  - GD02's checker holds all seven, and each has a positive control over a snapshot
    built by hand. Before GD05 no state the rules reach met five of the premises,
    since only `DeferBehindHeldVersion` sets `deferred_position`. The premises of
    `OnlyDeferredHasArrivalPosition` and `HeldCopiesAgree` have been met since GD03 by
    every held record with nothing deferred.
  - "the premise of each arrival-position invariant is true in a state the checker sees
    after a deferral" drives S091's setup through the owner, up to the last nack (G49),
    and reads that snapshot's derived sets once, through `inspection.views`. `pk`'s held
    records are `z` and `b`, each held at rank 1 with a newer version at rank 2
    deferred, and `y` and `a`, nacked with nothing deferred. Over them it asserts each
    premise: a held rank below the rank and one equal to it, a `deferred_position` and
    its absence, and two records with one each. It then asserts that `invariants.check`
    names nothing for the snapshot; the apply step had already run the checker over the
    state after the last nack.
  - The test shows each premise met and each conclusion holding there. That a broken
    conclusion is named is the positive controls' work.
- **`N`-note:** no `N`-note.
- **Question:** none.

### G62: what "the owner may apply a lapse ahead of an already-queued action" permits that the double does not do

- **Ticket:** GD06.
- **Concerns:** the Lease expiry comment; the `ExpiryRecoversWithinTtl` guarantee;
  decision 0019's closure of open question 8; G32.
- **The specification says:** "A pull, ack, nack, renew or release that the
  implementation processes after expires_at but before the lapse therefore succeeds, as
  long as its other guards hold; arrival order does not decide, since the owner may
  apply a lapse ahead of an already-queued action." `ExpiryRecoversWithinTtl` says a
  silent consumer loses its lease "lease_ttl after the last of them, plus the delay
  before the lapse is processed". Decision 0019 gives the reason: "The tick lane is
  polled first, so a ready tick can apply a lapse ahead of an action queued earlier,
  which the owner then applies as if it came after the lapse".
- **The double does:**
  - One mailbox, and no second lane. A lapse happens only inside a `Tick`, and the owner
    takes a `Tick` in its turn among the messages. So a lapse is never applied ahead of
    a message that reached the mailbox before its `Tick`, and an action sent before a
    `Tick` from the same process is always applied first.
  - No lapse happens without a `Tick`. A lease whose `expires_at` has passed stays
    active until a `Tick` reaches it, and every action that names it meanwhile is a late
    action and succeeds. The "delay before the lapse is processed" is however long the
    caller waits to send a `Tick`; nothing bounds it.
  - What the sentence permits and the double never does, then: a lapse that overtakes an
    action already queued, and a lapse that comes about with no tick sent. Each order the
    double can produce is one the sentence allows, and both orders the guards must
    decide are produced by choosing which message to send first (G63).
  - Decision 0019 also says "A tick samples now once, so a lease that expires during a
    long tick waits for the next one". The double's tick reads the clock once too (G56).
    Its fake clock does not move during a tick.
- **`N`-note:** no `N`-note. N33 records branch A, and not the mailbox's order.
- **Question:** is a lapse overtaking a queued action meant to be a freedom of the
  implementation only, or something every implementation must be able to show? A double
  that never shows it still satisfies every rule, since the guards decide in either
  order.

### G64: a decay names its entry by value and removes one occurrence (the first is `impl-choice`)

- **Ticket:** GD06.
- **Concerns:** `SupersededRecordDecays` (`when: shed: SupersededRecord.expires_at <=
  now`, `ensures: not exists shed`), `DeadLetterDecays`, and G11.
- **The specification says:** each decay's trigger binds one entry, and the rule removes
  that entry. It declares no identity for an entry and does not say what tells two equal
  entries apart (G11).
- **The double does:**
  - An entry is a value, with no identity apart from its fields (G11). The trigger's
    input is the entry itself. Its guard holds while an equal entry is in the queue, and
    its effect removes the first equal one and keeps every other entry in its order.
  - Two entries can be equal: two stale publishes of one key at one rank at one instant
    shed two identical `SupersededRecord`s. A tick offers both, each its own outcome,
    and each removes one. "two identical stale versions both decay in one tick, and the
    report names both" shows it through the owner. "a decay of one of two equal entries
    removes the first and keeps the other" shows over a state built by hand that one
    decay removes one occurrence, the first. That it is the first is `impl-choice`, and
    the test's comment says so: the specification cannot tell equal entries apart, and
    removing the last would leave the other entries in another order.
  - Which of two equal entries goes cannot be told from the values, only from their
    places in the list.
  - "SupersededRecordDecays removes exactly the entry it is given and reads no clock"
    and "DeadLetterDecays removes exactly the entry it is given and reads no clock"
    compare the complete state.
- **`N`-note:** no `N`-note.
- **Question:** none on the behaviour. If PD07 or open question 2 makes an entry
  something a reader names, should an entry have an identity, as G10 asks of a lease?

### G66: a port of a test that loops over `ENDINGS` is two registered tests, one per ending

- **Ticket:** GD06.
- **Concerns:** the Lease end comment ("A lease ends by release or by lapse"), its table
  and the six `Freed` rules; scenarios S078, S079, S080, S081, S087 and S088.
- **The specification says:** "The rules read the mark and never how the lease ended, so
  a lapse keeps every ack and nack."
- **The double does:**
  - Six Rust tests loop over `ENDINGS`, ending the lease by a release at 50 ms or by a
    tick at 2,000 ms. Each port is two registered tests carrying the same tag, named
    ", by release" and ", by lapse", so the runner's count shows that both ran and the
    coverage check sees the tag on each. That is twelve tests for six scenarios.
  - The lapse half moves the clock past `expires_at` and ticks; nothing else ends the
    lease. It asserts that the tick lapsed that lease alone, the rules it fired, and an
    empty `check`, and then what the release half asserts, with the count of
    consecutive expiries the lapse leaves and the lapse's `ended_at` in each entry.
  - S089's Rust test has a lapse half only, and so has its port. S093's loops over the
    two marks, not the endings: one test runs each mark on an owner of its own.
  - S066 and S067 are two halves each in separate Rust tests (G63).
- **`N`-note:** no `N`-note.
- **Question:** none.

## The `impl-choice` rows

A catalog row the double ports although it asserts more than the specification states.
The note's title labels the row, or the named rows, `impl-choice`, and each such test
carries the label and names its note.

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

### G31: the lease view lists the held records smallest position first (S036's, S037's, S085's and S090's order is `impl-choice`)

- **Ticket:** GD03, rewritten in GD04 when the owner's `LeaseView` message replaced the
  test helper.
- **Concerns:** the `Consuming` surface's `exposes` block; scenarios S033, S036, S037,
  S085 and S090.
- **The specification says:** for each active lease of the consumer the surface exposes
  `lease.pocket.key`, `lease.expires_at` and, for each record of
  `lease.pocket.held_records`, `record.key`, `record.held_rank`, `record.held_payload`
  and `record.mark`. Of the leases it says: "Zero or one lease by the invariant;
  iterating avoids a null read." `held_records` is `records where status = held`, with
  no order. The block exposes no lease identity.
- **The double does:**
  - The owner's `LeaseView` message, from GD04, answers the view. The owner hands it to
    `core.lease_view`, which refuses a consumer that never joined (G36) and otherwise
    calls `consuming.lease_view`. That reads the consumer's `active_leases` and that
    lease's pocket's `held_records` from the snapshot's derived sets, and answers the
    pocket's key, the lease's `expires_at`, and each held record's key, held rank, held
    payload and mark, or `None` when the consumer has no active lease. The read goes
    through no table, reads no clock and changes nothing: "a lease read through the
    owner changes nothing and reads no clock".
  - In GD03 a test helper, `harness.lease_view`, built the same view from one snapshot.
    Its body is now a call to the owner, so S033's and S036's ports read the view
    through the real seam. What they assert did not change.
  - The view lists the held records smallest position first, the order a pull handed
    them out in. It carries no lease id, as the surface exposes none.
  - S033's port, "a pull with no queued pocket returns NoPocketAvailable, not an error",
    asserts that a consumer told `NoPocketAvailable` has no lease view.
  - Four ports assert the order, which is more than the surface states, and each marks
    it `impl-choice`:
    - S036's, "a pull under a lease extends the lease": its records' keys as the ordered
      lists `r0`, then `r0`, `r1`. Its records were published in key order, so position
      order, key order and the order of creation agree, and it cannot tell which the view
      keeps.
    - S037's, "a held record is not handed out again under the same lease", from GD05:
      it takes the view's first record to be `r0`, the smaller position, and asserts
      its held rank 1, not the rank 2 deferred behind it. It rests on the order, though
      `r1` is held at rank 1 too, so the assertion would also pass with the two in the
      other order.
    - S090's, "the lease read shows every held record with its mark, smallest position
      first": `z`, `a` and `m`, published in that order, are shown in that order with
      their marks, and not in key order. This is the row N23 records.
    - S085's, "a changed mark is refused as a mark conflict and changes nothing, not
      even the lease's expiry, and reads no clock": `k1` then `k2`.
  - "the lease view shows the pocket of the consumer's active lease, its expiry, and
    each held record at its held version, smallest position first" builds a state by
    hand whose records are listed out of position order and whose smaller position has
    the later key, and shows that the position decides.
- **`N`-note:** agrees with N23.
- **Question:** N23's. Should the surface state an order for the held records? And should
  it expose the lease's identity, which every action it provides takes?

### G33: S040 asserts an absolute position (`impl-choice`)

- **Ticket:** GD04.
- **Concerns:** `Queue.next_position`, G04, and scenario S040.
- **The specification says:** no initial value for `next_position` (G04).
  `AckMarksRecord` ensures `record.mark = acked` and names no position.
- **The double does:**
  - S040's port, "an ack marks a held record and removes nothing", asserts that the
    acked record is at position 1, as its Rust test does. That holds only because the
    double starts at 0 (G04): the pocket takes 0 and its first record 1. The port marks
    the assertion `impl-choice`.
  - The facts the row also asserts are the specification's: the record is acked, the
    pocket still holds both its keys, and neither shed queue has an entry.
- **`N`-note:** no `N`-note. N04 and N16 list the rows that assert an absolute position,
  and S040 is not among them. This note extends G16 and G29 to it.
- **Question:** as G04, G16 and G29. Is the starting value meant to be unspecified? If
  so, should S040 assert an absolute position?

### G36: an ack, nack, release or lease read by a consumer that never joined is refused (`impl-choice`)

- **Ticket:** GD04.
- **Concerns:** the `Ack`, `Nack` and `Release` triggers, the `Consuming` surface's
  `facing consumer: Consumer` and its `exposes` block, G26, and scenarios S042 and S049.
- **The specification says:** each trigger takes a `Consumer` entity, which only
  `ConsumerJoins` creates. The surface faces a `Consumer` too. Nothing says what an
  action or a read naming an id that never joined does.
- **The double does:**
  - As G26 does for a pull and a renew. The apply step looks for the consumer before it
    runs the Ack, Nack or Release table, and answers `UnknownConsumer`, carrying the id,
    when there is none. No table is consulted, no clock is read, no lease id is drawn,
    and the owner keeps the state it had. The consumer is looked for before the lease,
    so a release by `cx` naming a lease never issued is `UnknownConsumer` too.
  - The lease read is not a trigger and goes through no table. The owner's `LeaseView`
    message calls `core.lease_view`, which holds the same check beside the apply step's
    and answers `Error(Refused(UnknownConsumer(id)))`, or the view (G31).
  - The tests: S042's and S049's ports, whose Rust tests act as `"nobody"`; "a release
    by a consumer that never joined is refused"; "a lease read by a consumer that never
    joined is refused"; and "a refused mark names the first requirement that fails, the
    lease first, and asks for no time". Each acts as `cx`, which never joined, or as
    `p`, which joined as a publisher only, and each shows the snapshot, the clock's read
    count and the lease-id draws unchanged. Each is labelled `impl-choice`.
- **`N`-note:** agrees with N22.
- **Question:** N22's, which is N18's. Should the specification say that each action and
  the lease read require a joined consumer, as G12 and G26 ask for a publish, a pull and
  a renew?

### G48: nine of GD05's rows and seven of GD06's assert absolute positions (`impl-choice`)

- **Ticket:** GD05, extended in GD06 when seven more ports rested on G04.
- **Concerns:** `Queue.next_position`, G04, `DeferBehindHeldVersion`'s arrival position,
  and scenarios S043, S044, S045, S046, S054, S071, S072, S082 and S083, and from GD06
  S048, S078, S079, S080, S081, S088 and S093.
- **The specification says:** no initial value for `next_position` (G04).
  `DeferBehindHeldVersion` takes `let position = queue.next_position` and ensures
  `record.deferred_position = position` and `queue.next_position = position + 1`. The
  Position vocabulary entry: "The first version deferred behind a held one is given the
  counter's value when it arrives, its arrival position; a version that supersedes it
  while it waits inherits that position."
- **The double does:** it ports the Rust tests' absolute positions, which hold only
  because the double starts at 0 (G04): the pocket takes 0 and its first record 1, so
  in a pocket of one record the first arrival position is 2. Each port marks them
  `impl-choice`:
  - S043, "a supersede while the key is held is deferred behind the held version": the
    record at position 1. Its arrival position is asserted against the counter before
    the publish, not as a number.
  - S044, "a version deferred behind an acked version is queued at its arrival
    position": the record at 1 while held, and the arrival position 3, at which it is
    queued.
  - S045, "two supersedes while held: only the deferred version is shed, and the held
    version never is": the arrival position 2, kept by the second supersede and taken
    when the lease ends.
  - S046, "a stale check while held compares with the latest rank, not the held one":
    the arrival position 2.
  - S054, "a release with only an acked record and its deferred version requeues the
    pocket rather than idling it": the arrival position 2, at which the record is
    queued.
  - S071, "a key acked, then published again within the same lease, waits behind the
    held version until the lease ends", and S072, "a key nacked, then published again
    within the same lease, waits until the lease ends; the dead letter appears then":
    the arrival position 2, at which the record is queued.
  - S082, "a publish onto a held record is judged alike whether it is unsettled, acked
    or nacked: higher defers, a second higher sheds the deferred version, equal is
    rejected, lower is stale": the arrival position 2, the counter at 3 and the record
    at 1, for each mark.
  - S083, "publishing again at the held rank is rejected while nothing is deferred and
    stale once a newer version is": the arrival position 2.
  - The relative facts these rows also assert are the specification's: the arrival
    position is the counter's value when the first newer version arrives, the counter
    advances by one, a supersede of the deferred version keeps it, a held record keeps
    its position, and a row that queues the deferred version puts it at its arrival
    position or, for an unsettled one, at the held version's.
  - GD05's other three rows assert no absolute position. S021's port compares the
    pocket's position with the one before the publish, S037's asserts none, and S091's
    compares the kept positions with the ones before the release (G49).
  - The double's own tests of the four GD05 rules, the ones named "… changes exactly
    what it ensures …", assert each position against a counter or a position they set
    by hand, and so do not rest on G04.
  - GD06's seven, each marked `impl-choice` in its port. Each port also reads the
    position from the state before the step and compares it with that, which is the
    specification's fact:
    - S048, "a version deferred behind a nacked one is queued at its arrival position by
      the lapse": the held record at 1 and the arrival position 5;
    - S078, S079, S080 and S081, Scenarios A to D, in both halves (G66): the arrival
      position 3, asserted in the setup the four share;
    - S081 also asserts `k`'s own position 1;
    - S088, "a pocket whose records were all acked, one with a deferred version, is
      queued", in both halves: the arrival position 5;
    - S093, "at an expiry limit of one a marked version with a deferred one parks with it
      queued": `early` at 2 and the arrival position 5, for each mark.
- **`N`-note:** no `N`-note for GD05's nine: N04 and N16 list the rows that assert an
  absolute position, and none of them is among them. This note extends G16, G29 and G33
  to them. For GD06's seven it departs from N38, whose ports of S048, S078 to S081 and
  S088 compare positions with the state before the step, "not as the Rust's absolute
  values". These ports compare them so too, and keep the Rust's absolute values beside
  that comparison.
- **Question:** as G04, G16, G29 and G33. Is the starting value meant to be
  unspecified? If so, should these sixteen rows assert absolute positions?
