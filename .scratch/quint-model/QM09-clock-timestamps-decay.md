---
id: QM09
title: "The clock, timestamps and decay"
status: todo
depends_on: [QM08, T41, T42]
parallel_with: [QM10, QM12, QM13]
branch: ticket/qm09-clock-timestamps-decay
estimated_size: L
---

# QM09: The clock, timestamps and decay

## Context

**`T42` in `depends_on` is a placeholder.** It stands for the ticket T42 creates that
changes `docs/specs/pachinko.allium`, which has no id yet. **Whoever creates that ticket
replaces `T42` with its id, in this frontmatter and in the index of
`.scratch/quint-model/README.md`.** Until then this ticket cannot start: step 1 stops if
the placeholder is still there.

After QM08 the model in `quint/pachinko.qnt` (created by QM02) holds 30 of the Allium's
32 rules and no time. A lease lapses at any moment (ruling 5 of
`.scratch/quint-model/README.md`). `Renew`, `AckRepeats` and `NackRepeats` change nothing
but `fired`. The two shed queues only grow. The header comment lists what is missing
under "Not yet modelled", each line naming QM09.

**The counts in this ticket are today's.** 32 rules, and traces that open with two
joins, are the Allium's at `cf1f3f3`. A T ticket that lands before this one changes the
model with it (README, "The T tickets that touch the model"), and no ticket edits this
text when it does. `tickets/T45-drop-joining.md` removes `PublisherJoins` and
`ConsumerJoins`: the Allium then has 30 rules, and no trace opens with a join. Step 2
counts the variants of `Rule` in the base's model, 30 today with this ticket's two
decay rules still to come, and names the T ticket behind any difference. Read every
"32" here as that count plus two; drop the joins from each trace in the tables below
if they are gone; and let the plan recompute each depth and the bound. A bound already
in `quint/verify.txt` is still never lowered.

This ticket adds the clock: `now`, every timestamp field, the three TTLs, both decay
rules, the stamps on every earlier rule that writes one, and the Allium's own condition
on a lapse in place of QM08's untimed one. It owns the last two rules, nine invariants
and the last guarantee.

**The ticket cannot say today how the clock moves.** At commit `cf1f3f3` the Allium
reads `now` in most rules and never says what `now` is, whether it can run back, or what
one tick does. Two tickets settle that first, and both are `todo`:

- `tickets/T41-tick-order-investigation.md` investigates the order of work inside one
  tick and rules nothing. It writes `docs/explanation/tick-order.md` (created by T41),
  which ends with the questions for the maintainer. It leaves open the order among
  several lapses due at one `now`, among several decays, and between the two shed
  queues.
- `tickets/T42-clock-model.md` explores the clock model, has it ruled, and creates the
  tickets that land it. Its "The rulings" (2026-11-24) fix four things about the clock:
  the specification models it, `now` never decreases and a refusal does not advance
  it; `now` may repeat; a tick drains, one `now`, every due lapse, then every due
  decay, until nothing is due at that `now`; and the specification names no source for
  the clock. Its "What the rulings leave open" lists seven points: where "`now` never
  decreases" lives; whether a tick is atomic as a whole; whether a tick with nothing
  due advances the latest time; what a tick reports; whether the expiry rules' two
  repeated `requires` are guards or restatements; the lapse intent; and the source.

QM10, QM12 and QM13 may run in parallel with this ticket. Each shares
`quint/pachinko.qnt` or what is generated from it, and whichever merges second rebases
and applies shared rule 11 to what the rebase changed.

- **QM10 and QM12** own the two replays. This ticket changes every earlier rule and
  adds state and steps, so if either has merged the fixtures are regenerated and that
  replay is kept green in this change. `advance` and `tick` are not in `replayStep`
  until QM11, so no fixture holds one.
- **QM13**, `.scratch/quint-model/QM13-inductive-exclusivity.md`, adds an inductive
  invariant, `LeaseExclusivityInductive`, with a `StateInRange` clause for every `var`,
  and says that whichever of QM13 and this ticket merges second extends it. If it is in
  the base, this ticket adds the clauses for the clock and the timestamps.

So step 2 reads the specification as it landed and the decision record T42 wrote, and
the plan at checkpoint 1, with a design lens, is where the transcription is fixed.
"Design proposed" below gives a recommended answer under each ruling T42 could land.
One of T42's points would change more than the clock. Under "The lapse intent" an
overdue lease would end only when another consumer takes its pocket. If that has landed,
the lapse, the fragments and the mutants below no longer fit: **stop** at step 2 and
ask whether this ticket is rewritten first.

What the Allium says today, at `cf1f3f3`. Read each at the ticket's base (shared rule
1):

- **The entities.** `Lease` has `granted_at`, `expires_at` and, once ended, `ended_at`.
  `Record` has `nacked_at` while held. `SupersededRecord` has `shed_at` and
  `expires_at`. `DeadLetter` has `nacked_at` and `expires_at`.
- **The rules that stamp.** `Supersede` and `DropStalePublish` write `shed_at: now` and
  `expires_at: now + config.superseded_ttl`. `PullClaimsFrontPocket` writes
  `granted_at: now` and `expires_at: now + config.lease_ttl`. `PullWithinLease`,
  `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats` and `Renew` write
  `lease.expires_at = now + config.lease_ttl`. `NackMarksRecord` writes
  `record.nacked_at = now`, and `HoldRecord` and the four lease-end rules that queue a
  record write it null. Both release rules and the three expiry rules write
  `lease.ended_at = now` and pass it on in `Freed`. `SupersedeUnsettledHeldVersion`
  stamps `shed_at: ended_at` and `expires_at: ended_at + config.superseded_ttl`. The two
  dead-letter rules keep the record's `nacked_at` and stamp
  `expires_at: ended_at + config.dead_letter_ttl`.
- **The Lease end comment**: the three rules that create an entry "stamp it from that,
  not from their own now", where "that" is the `ended_at` their `Freed` carries.
- **Decay.** `SupersededRecordDecays` is `when: shed: SupersededRecord.expires_at <= now`,
  `ensures: not exists shed`. `DeadLetterDecays` is the same for a `DeadLetter`.
- **The config.** `lease_ttl`, `superseded_ttl` and `dead_letter_ttl`, with the comment
  above `LeaseTtlNonNegative`: "A zero TTL is admitted".
- **The Lease expiry comment**, and the guarantee `ExpiryRecoversWithinTtl`.

Read these:

- `.scratch/quint-model/README.md`: ruling 4, both blocks of "Design fixed before QM01",
  "Coverage a later ticket adds" (its last four rows), "Design points a ticket settles"
  (the row for QM09), and claim 12.
- `docs/specs/pachinko.allium` at the base, in full for every item above, and whatever
  T42's specification ticket added about `now` and a tick. The decision record T42
  wrote, under `docs/decisions/`, and `docs/explanation/tick-order.md` (created by T41).
- `tickets/T41-tick-order-investigation.md` and `tickets/T42-clock-model.md`, Context
  and Goal, and the hand-back of each.
- `quint/pachinko.qnt`, `quint/verify.txt` and `scripts/verify-quint.sh` (created by
  QM03), every patch under `quint/mutants/`, and the record QM02 wrote as decision 0022.
- `.scratch/quint-model/QM03-verify-recipe-and-ci.md` ("Design proposed", points 1 to 3
  and 5 to 7), `.scratch/quint-model/QM06-ack-nack-renew-lease-end.md` (the parts of a
  guarantee, and "the entries a step added"),
  `.scratch/quint-model/QM08-untimed-lapse-and-parking.md` (the untimed lapse and what
  it left for this ticket), each with its hand-back. For what the replays need,
  `.scratch/quint-model/QM10-rust-replay.md`, with its hand-back if QM10 has merged at
  the base, and `.scratch/quint-model/QM11-replay-lapses-and-time.md`, whose text is all
  there is: QM11 depends on this ticket and has not run.
- `crates/libpachinko/tests/queue/walk.rs`, for values and names only: `START` (line 46
  at `cf1f3f3`), `FORWARD_MILLIS` and `BACKWARD_MILLIS` (lines 50 and 54), `config()`
  (lines 59 to 67), `WalkStep::Advance` and `WalkStep::Tick` (lines 113 to 116), and the
  comment on `Walk::now` (lines 168 to 173).
- `crates/libpachinko/src/queue/expiry.rs`, `tick_counting` (lines 127 to 142), and
  `crates/libpachinko/src/queue.rs`, `fn now` (lines 276 to 279), only to say what QM11
  will need from the model. They decide nothing here (shared rule 1).
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

The model has a clock that moves as the landed Allium says. Every rule that stamps a
time stamps it, a lease lapses only when its `expires_at` has passed, shed entries decay
and nothing else removes one, and `advance` and `tick` are steps of `step`. The nine
invariants and the guarantee below are checked properties on `pachinko_walk` with the
walk's TTLs and on a second instance with every TTL at zero. Each of the 32 rules has
its witness reached under the clock, the eleven mutants are caught, and no line under "Not
yet modelled" names QM09 any more.

`ExpiryRecoversWithinTtl` is checked as safety fragments only. That a lapse is ever
processed, and how long after `lease_ttl`, is liveness and stays excluded (decision
0022).

### Owns

| Kind | Names |
| --- | --- |
| Rules | `SupersededRecordDecays`, `DeadLetterDecays` |
| Invariants | `NackedHasItsTime`, `OnlyNackedHasNackTime`, `ShedBeforeExpiry`, `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant`, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative` |
| Guarantees | `ExpiryRecoversWithinTtl`, as safety fragments |
| Partial or extended, not counted | Every rule that writes a timestamp, with its effect: each one named under Context, which QM02 and QM04 to QM08 own. The lapse's time condition in `LeaseLapses` and the three expiry rules. `MarksStandHoweverLeaseEnds` (QM08's) and `EveryDropIsVisible` (QM07's), each completed by one clause. `FiredMatchesGuards` (QM02's), for a tick. `UnansweredStepChangesNothing` (QM02's), for a step that moves the clock. Every `effect_`, read stage by stage on a tick |

## Design proposed in this ticket

These are proposals. Step 2 reads what landed, the plan picks the row each ruling
selects, and the maintainer confirms or changes the whole at the **Stop** in step 3,
with checkpoint 1's findings in hand. After that the builder does not reopen them. They
are written against the recommended answers of QM02, QM03, QM05, QM06 and QM08.

The acceptance criteria, Verification, the README's row for QM09 under "What each ticket
delivers" and QM11's ticket are written against the recommended answers here. A ruling
at step 3 for an alternative changes some of them: with no second instance, for
example, nothing can be run on `pachinko_zero_ttl`. The README gives that work to the
ruling ticket ("Later tickets are written against recommended answers"). So before step
4 this ticket edits its own Goal, claims table, steps, acceptance lines and
Verification, the
README's sentences and the later tickets' sentences that the ruling changes, and lists
each edit under Departures. It never drops a step or a criterion the ruling leaves
standing. Ruled without the second instance, for example: the Goal names one instance;
step 6 becomes the line in decision 0022; claims C and D have no instance, so step
8's observation of C and step 6's witnesses for D are each recorded as not run; and
Verification loses `pachinko_zero_ttl` from
its three lists.

### How the clock moves

This is the README's design point for QM09. The model follows the Allium, so the answer
depends on T42's open point 1, where "`now` never decreases" lives.

- **Time is an integer count of milliseconds.** `pachinko_walk` starts the clock at
  1,000, the walk's `START`, so an instant in a fixture is the instant the Rust stamps.
- **The row is chosen by what the landed rules do with a reading behind the latest
  time**, not by which of T42's forms the text is called. Keeping the latest time as
  state does not settle it. T42's own sketch of form 1(a) has each rule require `now`
  to be at or after the latest time, which refuses such a reading; the Rust holds it
  instead. The two differ: with a lease active, step the clock back 50 ms and renew.
  Held, the renew is accepted and stamps the latest time. Refused, it changes nothing.
  Step 2 quotes the Allium's lines that decide between the three rows below. If they
  decide for none, **stop** and ask.
- **If the Allium keeps the latest time and holds a reading behind it** (form 1(a), with
  a rule that moves the latest time on to the greater of the two). Recommended: two
  values. `clock` is what `advance(millis)` moves, by any amount
  in the walk's range, back as well as forwards. `now` is the latest time: a step that
  reads the clock sets it to the greater of itself and `clock`, and every stamp reads
  that. Which steps read is the Allium's to say; T42's ruling 1 says a refusal does not.
- **If the Allium keeps the latest time and refuses a reading behind it** (form 1(a),
  with a `requires` that the reading is at or after the latest time). Recommended: the
  same two values. A rule that reads the clock answers a step only when `clock` is at
  or after `now`, and the step then sets `now` to `clock`. When `clock` is behind, no
  rule answers: the state is unchanged and `fired` is empty. Each guard gains that
  conjunct in the Allium's words. It is the Allium's `requires`, not a guard the model
  adds. **This row is not complete, in two respects, and nothing here settles either.**
  Earlier properties say a step is answered whatever the clock reads. QM04's
  `PublishAnsweredByOneRule` and QM07's `NeverRefusedForCapacity` count exactly one
  publish guard for a publish not at the stored rank, and clause 2 of
  `MarksStandHoweverLeaseEnds` and QM06's `MarksStandHoweverLeaseEnds_release` have a
  repeated ack or nack fire its rule. With `clock` behind `now`, each fails as written
  wherever the rule that would answer carries the conjunct. And `TickIsItsStages`
  below asks that nothing is due after a tick: a tick whose read is refused while a
  lease is due at `now` changes nothing and leaves it due, and the property has no
  case for it. If step 2 selects this row, the plan lists every such property with the
  trace on which it fails, and quotes what the landed Allium says of a tick taken at
  such a reading. No property is edited and no conjunct is left out to make the run
  pass: both go to the **Stop** in step 3 (Open points).
- **If the Allium only states that `now` never decreases** and keeps no latest time
  (form 1(b)). Recommended: one
  value, `now`, and `advance(millis)` adds a non-negative amount. An `advance` by a
  negative amount is a step no rule answers and changes nothing.
- **Never** one `now` on which a negative `advance` changes nothing while a later
  positive one counts in full, when the Allium has either two-value form. The Rust keeps
  both values (`fn now` in `crates/libpachinko/src/queue.rs`): from 1,000, an advance by
  -50 and one by 60 leave the Rust's next stamp at 1,010 and such a model's at 1,060,
  and QM11 could not replay it.
- **What `now` is before any step reads the clock**, under either two-value row. If the
  landed Allium gives the latest time a first value, in a start rule or beside the
  state, the model has that value. If it gives none, recommended: `init` reads `now`
  from a constant of its own that `pachinko_walk` sets to 0, below every reading, with
  `clock` at 1,000. The first step that reads the clock then takes `now` to the clock,
  and nothing is stamped before a read, so the constant shows in no timestamp. Decision
  0022 says the Allium gives no first value, as it does for the counter (QM04).
  Alternative: `now` starts at 1,000 with the clock. It reads more simply, and it holds
  a reading the Rust never took: from 1,000, an advance by -50 and then the first step
  that reads the clock leave the Rust's latest time at 950 and such a model's at 1,000.
  Read, not run: the walk starts its clock
  at `START` and leaves the queue's latest read at `Timestamp`'s default, which is 0
  (`crates/libpachinko/tests/queue/walk.rs`, lines 647 to 651 in `walk`, with the
  field at lines 168 to 173). Under the one-value row `now` is the clock and starts
  at 1,000.
- **A clock that would go below 0.** `advance` has a `WalkStep` and no Allium rule, so
  what it does to the clock is the walk's, as its range is. Recommended: an instant is
  never negative, and `advance(millis)` moves the clock to the greater of 0 and the
  clock plus `millis`. Read, not run: the walk's clock is an unsigned count that
  saturates (`walk.rs`, lines 542 to 546, the `advance` function, which calls
  `saturating_add_signed`). The case is inside the bound: nine advances by -120 take
  1,000 below 0. Alternative: an integer that goes negative. It needs no clamp, and a
  trace that takes it there stamps what the Rust cannot.
- **What the two alternatives cost.** A model that starts `now` at 1,000, or lets the
  clock go below 0, cannot be replayed over a negative advance, and QM11 then replays
  forward advances only, whichever clock landed. The plan says which was ruled, for
  QM11.
- **`advance` fires no rule.** It has a `WalkStep` and no Allium rule, so `fired` is
  empty after it and `last` records it with its amount.
- **So a step can change the state with `fired` empty.** The README's second design
  block says how QM02's `UnansweredStepChangesNothing` reads from this ticket on, and
  this ticket edits the property in the model to say it. When `fired` is empty the
  state equals `prev` and `answer` is nothing. The clock is the one exception, and the
  property states it by the step `last` names: after `advance`, and after a `tick`
  that finds nothing due, the clock may differ from `prev` as the action says and
  everything else is equal; after any other step with `fired` empty the clock is equal
  too. "As the action says" is exact, not a licence. After `advance`, the value it
  moves is the greater of 0 and `prev`'s plus the amount `last` records, or the plain
  sum if the clock is ruled to go below 0: `clock` under
  either two-value row, with `now` equal to `prev`'s; `now` under the one-value row
  when the amount is not negative, and `prev`'s `now` when it is. After a `tick` that
  finds nothing due, the clock is what the landed Allium says of T42's open point 3;
  if it says nothing, that is a ruling with no row, and step 2 stops. "Any other step"
  takes in every refused ack, nack, renew, release, pull and publish: that is T42's
  ruling 1, a refusal does not advance `now`. The mutant
  `qm09-refused-action-moves-clock` is the control. The README's sentence stands as
  it is; this ticket edits it only if the ruling at step 3 differs from it.
- **The walk's range is kept whole**: `millis` from -120 to 120. It is the walk's value,
  as the TTLs are. If the range costs more than the timeout allows, **stop** and ask
  (ruling 4); the alternative to offer is a set of amounts that still crosses each TTL.

### The constants and the instances

- `lease_ttl`, `superseded_ttl` and `dead_letter_ttl` are `const`s of `pachinko`.
  `pachinko_walk` sets them to 100, 150 and 250, the walk's `config()`.
- The three TTL invariants are properties that read only a constant, the form QM05
  recommends for `PullBatchPositive`. Each is a `val` of `pachinko`, not of an instance
  module, where QM03's helper refuses an upper-case `val`. Each has a committed mutant
  that makes the constant negative in `pachinko_walk`. If
  `tickets/T47-start-rule-and-config-bounds.md` has landed and made the bounds the
  start rule's requirements, such a patch is expected to leave the model no start
  state, so that nothing is violated and the property it names cannot catch it. That
  is expected and not shown: it is the README's claim 17, not read and not run, and
  T47 checks it on the patch it finds. Step 2 reads T47's hand-back,
  and the three mutants then take the control T47's ruling named, as QM05's
  `qm05-zero-pull-batch` does; the checker's answer is the run that hand-back quotes,
  not this bullet. If the hand-back quotes no such run, or names no control that covers
  a config-bound mutant written after it, step 2 runs the probe on its scratch module
  (an `init` that requires a constant's bound, an instance that breaks the bound and a
  twin that keeps it, each verified with a property that is false on purpose) and
  quotes it, and the question of which control the three take goes to the maintainer
  at the **Stop** in step 3. The lines here that name the three patches, or count them
  among the eleven, change with the ruled control, whichever section holds them: their
  rows in the mutants table and in Files touched, the count of eleven, and
  in Verification the count and the three header checks among them. Each is brought to
  the ruled control before step 4 and recorded under Departures; none is dropped
  without its replacement.
- **A second instance, `pachinko_zero_ttl`.** Recommended: equal to `pachinko_walk` but
  with all three TTLs at 0, which the Allium admits. At 0 a lease is due the moment it is
  granted and an entry the moment it is made, so this is where "a tick drains" and the
  equalities of `GrantedBeforeExpiry`, `ShedBeforeExpiry` and `NackedBeforeExpiry` are
  exercised. **The cost** is a second canary, pass and 32 witnesses, since QM03's recipe
  runs every witness on every instance; step 6 measures it. So every one of the 32 rules
  must be reachable on `pachinko_zero_ttl` too. Each should be (claim D). If one is not,
  **stop** and ask: the instance is not dropped, the rule's witness is not removed, and
  the recipe is not edited to excuse it. **Alternative:** no second instance, and a line
  in decision 0022 that a zero TTL is admitted by the Allium and checked on no instance.
  The README lists the second instance among what this ticket delivers, so that ruling
  also changes the README's row, as the paragraph at the head of this section says.
  Either way decision 0022 says that mixed settings, some TTLs zero and some not, are
  checked on no instance.

### The lapse, the decays and `tick`

- **`lapse(lease)` keeps its place** in `step` alone. `guard_LeaseLapses` and the three
  expiry guards gain `lease.expires_at <= now`, as the landed Allium words it, and
  QM08's "Not yet modelled" line for it goes. Every lease end writes `ended_at`.
- **Two more model-only actions, `decaySuperseded(entry)` and `decayDeadLetter(entry)`**,
  in `step` alone, each removing one entry of its queue, named by its place in the
  list. A decay of an entry that is not due, or of no entry, changes nothing.
  `guard_SupersededRecordDecays` and `guard_DeadLetterDecays` are the Allium's `when`:
  `expires_at <= now`. Each effect says the entry is gone and every other entry of both
  queues is there, in its order.
- **`tick` is one action, in `step`,** that does at one `now` what T42's ruling 3 says:
  every due lapse, then every due decay, until nothing is due at that `now`. It applies
  the same definitions `lapse` and the two decays apply, one stage after another, and
  transcribes no rule again.
- **How it is shown that a tick is a composition of steps `step` already allows.** One
  property, `TickIsItsStages`, over `prev`: when `last` is a tick, walking the stages
  in the tick's order, each stage's guard holds in the state the stage
  starts from, the state after the last stage is the state, and nothing is due in it at
  that `now`. The walk does not start from `prev` as it stands. A tick reads the clock
  before its first stage, where the selected row of "How the clock moves" has a step
  read it, so the walk starts from `prev` with `now` as that read leaves it and
  everything else equal, and the first due guard reads that `now`. A tick that finds
  nothing due has no stages, and the state is then that starting state: `prev` whole
  if the landed Allium has such a tick leave the clock alone, and `prev` with the
  clock read if it has it move (T42's open point 3), which is the one change
  `UnansweredStepChangesNothing` lets such a tick make. Every lapse stage comes before
  every decay stage, and the lapse stages
  are the leases of `prev` that are due at that `now`, each once. The order `last`
  records is written by `step`, as the README has it for every step, and by no
  action. By itself that order shows neither that it is the Allium's nor that the
  action took its stages in it, so the property takes neither on trust. Where the
  landed
  Allium fixes an order, among the lapses, among the decays or between the two queues,
  the property states that order over the stages from the Allium's text and from the
  states of the walk: each stage takes the one the Allium puts first among those still
  due in the state it starts from. It never reads the definition the action sorts with.
  That the action followed the order is shown only by the walk's last state being the
  state. Two orders that end in the same state, `fired` and `answer`, as two decays do
  when no report names them, are told apart by no property; checkpoint 2 reads `tick`
  for it (Open points).
  The plan shows the walk is written with the same definitions the three
  actions use, and checkpoint 2 asks it again of the diff.
- **A tick against every property that compares `prev` with the state.** After a tick
  `fired` is the union of what its stages fired. A tick's stages do not share one
  `prev` and one state: at a zero TTL a lapse inside a tick creates a dead letter that
  the same tick decays, so the entry is in neither. That touches four kinds of
  property, and the plan lists every property of the model under one of them or as
  safe:
  1. `FiredMatchesGuards`. `DeadLetterDecays` fires for an entry that was not in
     `prev`. The README already says how this is read: `fired` holds the rules whose
     guard held "in `prev`, or, for a rule later in a chain, in the state the chain had
     reached". A tick is the longest chain the model has.
  2. Every `effect_`. `effect_DeadLetterNackedRecord` looks for an entry that is not in
     the state, and a decay's effect says every other entry is still there.
  3. Every property that counts what one step added to a shed queue:
     `MarksStandHoweverLeaseEnds` clause 4, QM06's `EveryDropIsVisible_marked` and
     `HeldRecordsOldestFirst_marked`, and QM07's `EveryDropIsVisible`,
     `HeldRecordsOldestFirst` and `NothingLostWhileHeld` where they read the entries a
     lease's end made.
  4. Any property that reads the one lease `last` names.

  Recommended, in three parts, all in `quint/pachinko.qnt`:
  - Kind 1: QM02's `FiredMatchesGuards` gains a case for a tick that reads each
    stage's guard in the state the walk of the stages had reached, as the README's
    sentence says. No exemption.
  - Kind 3: QM06's two definitions of "the entries a step added", one per queue, give
    for a tick what its lapse stages made, in order, read from the walk of the stages
    and not from the two lists. The properties that use them stand unchanged.
  - Kinds 2 and 4: no exemption either. The README has `effect_<Rule>` hold whenever
    `fired` holds the rule, and names no step it may skip. So the body of each
    `effect_`, and of any property of kind 4 the plan finds, becomes one definition
    over a pair of states and the stage's arguments. On every step but a tick the
    property applies it to `prev` and the state, which is what it says today. On a tick
    it applies it to each stage that fired the rule, between the states the walk gives
    before and after that stage. No `ensures` and no frame changes its text; what it is
    read between does. The hand-back lists every effect edited this way.

  **Alternative:** each `effect_`, and any property of kind 4, holds for every step but
  a tick, through one shared definition, on the argument that each stage is a step of
  `step`, where the property is checked, and that `TickIsItsStages` shows the tick is
  exactly those stages. It is less to write, and it checks less, in three ways.
  `TickIsItsStages` recomputes the stages with the definitions the action uses, so a
  fault those definitions share passes both. A tick of k stages taken as the bound's
  last step ends where single steps arrive only k - 1 steps past the bound, and no
  effect is checked there. And it departs from the README's second design block, which
  this ticket would then edit. If it is ruled, decision 0022 says all three.
- **The order inside a tick** follows what T41's page led the maintainer to rule. If
  the Allium fixes an order, `tick` uses it. If it fixes none, `tick` takes the due
  lapses in any order, chosen at the step, so the model fixes nothing the Allium leaves
  open; with three consumers that is at most six orders. In every case, fixed by the
  Allium or not, `last` records the order of the lapses the tick took, so that
  `TickIsItsStages` can walk it and a fixture shows it to a replay. The decays need no
  such choice. Deleting
  the due entries of the two lists in any order, from either queue first, leaves the
  same two lists, so no order among decays shows in the state. It shows only if the
  landed Allium has a tick report what it decayed, in order (T42's open point 4). Then
  the decays' order is chosen at the step and recorded in `last`, as the lapses' is.
  And if the landed Allium fixes an order among the decays or between the two queues,
  with a report or without, `last` records the order of the decay stages as well. The
  state cannot show which order the action took, and `TickIsItsStages` can hold the
  Allium's order only against stages it is given.
- **If the Allium makes a tick atomic as a whole** (T42's open point 2), a single lapse
  or decay is no longer an outcome of the specification. Recommended: `lapse` and the
  two decays stay as model-only steps, with QM08's argument restated in decision 0022:
  the states between a tick's stages are states the specification never shows, and an
  invariant that holds in them holds after the tick. **Alternative:** remove the three
  and keep `tick` alone; then the effects can only be read stage by stage, as
  recommended above, and the exemption is no longer on offer, because no single stage
  is a step any more.
- **What `answer` holds after a tick** follows T42's open point 4. If the Allium says
  nothing a tick reports, `answer` is nothing. If it says a tick reports what it lapsed
  or decayed, `TickIsItsStages` states the report over the stages of the walk, order
  included, as it states the order of the stages.

### What QM11 needs, either way

The README gives `advance` and `tick` in `step` to this ticket and their place in
`replayStep` to QM11. This ticket adds neither to `replayStep`. It lands them so that
QM11 can:

- **A whole tick.** QM11 maps `WalkStep::Tick` only to one action that, at one `now`,
  lapses every due lease and then removes every due entry, with no step between. That
  is `tick` above. Under an unordered tick, QM11 cannot compare the order of two
  lapses' entries in a shed queue; the plan says so and names it for QM11.
- **The Rust's order**, for the record: earliest `expires_at` first, then lease id
  (`tick_counting` in `crates/libpachinko/src/queue/expiry.rs`, read, not run). Under
  QM02's recommended answer a lease's identity is its place in the order of grants, so
  "then lease id" is "then earlier grant" in the model.
- **A clock QM11 can drive.** QM11's ticket has a case for each of the three clocks.
  Under the two-value clock that holds, it replays a negative advance only if its step
  2 shows three things: `now` starts below every reading and the clock stops at 0,
  which "How the clock moves" recommends, and the model's steps that read the clock
  are the Rust's. This ticket follows the Allium on which steps read, so the third is
  not this ticket's to promise; the bullet below says where the two may differ. If any
  of the three fails, QM11 replays forward advances only. Under the one-value clock,
  and under the two-value clock that refuses, it replays forward advances only: there
  the model refuses a step the Rust accepts once the clock has stepped back.
- **Which steps move `now`.** The walk's comment on `Walk::now` says the Rust reads its
  clock in an accepted step and in a publish by a joined publisher, and the walk counts
  such a publish as a read whatever its outcome (`walk.rs` line 317 at `cf1f3f3`).
  T42's ruling 1 says a refusal does not advance `now`. The model follows the Allium; a
  difference from the Rust is QM11's to find.

### `ExpiryRecoversWithinTtl`, as safety fragments

One property under the Allium's name, the conjunction of these. It reads `prev`,
`last`, `fired` and the state.

1. **No lapse before `expires_at`.** For every lease active in `prev` and `expired` in
   the state: its `expires_at` in `prev` is at or before the step's `now`, and its
   `ended_at` is that `now`.
2. **An extension is exact.** After a step whose `fired` holds `PullClaimsFrontPocket`,
   `PullWithinLease`, `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats` or
   `Renew`, the lease concerned has `expires_at` equal to `now + lease_ttl`. Not "later
   than before": two steps at one `now` write the same value, and a zero TTL is allowed.
3. **Nothing else moves it.** An active lease's `expires_at` differs between `prev` and
   the state only in a step whose `fired` holds one of those seven for it, and its
   `granted_at` never changes.
4. **What a lapse leaves.** QM08's part, `ExpiryRecoversWithinTtl_untimed`, unchanged.

| Sentence of the guarantee | Fragment |
| --- | --- |
| "A consumer whose pulls, acks, nacks and renewals all stop loses its lease when the lapse is processed: lease_ttl after the last of them, plus the delay before the lapse is processed" | 1, 2 and 3 give the safety half: not before `lease_ttl` after the last of them. That the lapse is processed at all, and the delay, are not checked |
| "A late pull, ack, nack or renew processed before the lapse extends the lease instead, unless the extended expires_at has also passed when the lapse is processed" | 2, and 1 read against the extended value. That a late action is answered at all is `FiredMatchesGuards` with no guard reading `expires_at`, plus the observation below |
| "Once a lapse is processed, its unsettled records and the versions deferred behind its held ones are queued and, unless that lapse parks the pocket, are pullable again at once" | 4 |
| "A pocket left with no records has nothing to recover; it parks at the threshold like any other, and below it LeaseExpiryIdlesEmptyPocket says where it goes" | 4 |

**The late action, observed once by hand.** A definition `lateRenew`, true when `fired`
holds `Renew` and the lease's `expires_at` in `prev` was at or before `now`, is observed
on `pachinko_walk` at step 8 of this ticket. It is not a rule's witness, so the recipe
does not run it.

### The clauses this ticket adds to earlier properties

- **`MarksStandHoweverLeaseEnds`**, clause 2. QM08 wrote "the state equals `prev`" with
  a named exception. It becomes: the state equals `prev` but for the lease's
  `expires_at`, which is `now + lease_ttl`, and the clock. QM06's "Not yet modelled"
  line for the lease extension goes.
- **`EveryDropIsVisible`.** QM07 wrote "no step removes one" of a shed queue's entries
  and left that clause to this ticket (its Open points). This ticket replaces it. A
  clause added beside it would still fail the first legal decay. The new clause counts
  entries by their place in the list, never by value: two identical stale publishes at
  one instant make two equal entries, and `[E, E]` becoming `[E]` is one removal that a
  test of membership does not see. For each queue the clause tells two lists apart:
  the entries the step made, in the order made, and those of them that survive the
  step. The state's list is `prev`'s list with some places deleted and the rest in
  their order, followed by the made entries that survive, in their order. A place of
  `prev`'s list, or a made entry, is deleted only if `fired` holds that queue's decay
  rule, and every deleted entry had `expires_at` at or before `now`. On every step but
  a tick nothing made is deleted, and the two lists are one. The deleted places are
  read from `last` for `decaySuperseded` and `decayDeadLetter`, which name one, and
  from the walk of the stages for a tick. On a tick an entry can be made by a lapse
  stage and deleted by a decay stage, and so be in neither `prev` nor the state: with
  no dead letter in `prev`, a tick at a zero TTL that lapses a lease with a nacked
  record makes `E` and decays it, and the state's list is empty. The walk has `E`. It
  is among the entries made, which is what the properties of kind 3 count. It is not
  among the survivors, so nothing is appended for it. And the clause on deletion asks
  of it what it asks of a place of `prev`. QM06
  put "the entries a step added" behind one definition per queue so that this is one
  place.
- **Every lease extension** (README, last row of "Coverage a later ticket adds"). One
  definition moves an existing lease's `expires_at`, and the six extending rules all
  call it. `effect_PullWithinLease`, `effect_AckMarksRecord`, `effect_AckRepeats`,
  `effect_NackMarksRecord`, `effect_NackRepeats` and `effect_Renew` each gain
  `expires_at = now + lease_ttl`, written out in each. The mutant
  `qm09-extension-does-not-extend` cuts inside that definition and names `effect_Renew`;
  for the other five the plan names the definition, which that mutant breaks. The
  grant writes its own `expires_at`, `effect_PullClaimsFrontPocket` states it, and
  `qm09-expiry-before-grant` is its mutant.

### Mutants

Each is a patch with the header QM03's format asks. Steps are counted after `init`, on
`pachinko_walk`, with the clock at 1,000 and written for the two-value clock that holds.
Under the other two rows the seventh keeps its name, its property and its trace, and
cuts elsewhere, so that Verification stands as written. Under the two-value clock that
refuses, it drops the `requires` that refuses a reading behind the latest time. Under
the one-value clock, it drops the test that makes a negative `advance` change nothing.
Either way `now` runs back by 100 before the release. The eighth keeps its name, its
property and its trace too: under either two-value clock the refused action still sets
`now` from `clock`, and under the one-value clock, which has no second value to read,
it adds 1 to `now`.

| Patch | Where it cuts | Property | Shortest trace | Steps |
| --- | --- | --- | --- | --- |
| `quint/mutants/qm09-expiry-before-grant.patch` | the grant in `PullClaimsFrontPocket` writes `expires_at` as `now - lease_ttl` | `GrantedBeforeExpiry` | two joins, a publish, a pull | 4 |
| `quint/mutants/qm09-extension-does-not-extend.patch` | the definition every extension calls hands the lease back with `expires_at` as it was | `effect_Renew` | two joins, a publish, a pull, advance 50, renew | 6 |
| `quint/mutants/qm09-ack-repeat-does-not-extend.patch` | `ack`, the `AckRepeats` outcome: the extension is not called | `MarksStandHoweverLeaseEnds` | two joins, a publish, a pull of limit 1, ack, advance 50, ack | 7 |
| `quint/mutants/qm09-dead-letter-expiry-from-nack.patch` | `DeadLetterNackedRecord`: `expires_at` is `nacked_at + dead_letter_ttl` | `effect_DeadLetterNackedRecord` | two joins, a publish, a pull of limit 1, nack, advance 50, release | 7 |
| `quint/mutants/qm09-decay-before-expiry.patch` | `decaySuperseded` and the tick's decay stage: the due test is dropped | `EveryDropIsVisible` | `p0` joins, publish `k0` at rank 1 and at rank 2, decay the entry | 4 |
| `quint/mutants/qm09-lapse-before-expiry.patch` | `guard_LeaseLapses` and the three expiry guards: the time condition is dropped | `ExpiryRecoversWithinTtl` | two joins, a publish, a pull, lapse | 5 |
| `quint/mutants/qm09-now-runs-back.patch` | where a step takes `now`: it reads `clock` and not the greater of the two | `EndedAfterGrant` | two joins, a publish, advance 100, a pull, advance -100, release | 7 |
| `quint/mutants/qm09-refused-action-moves-clock.patch` | `renew`, the outcome no rule answers: the step still takes `now` from the clock | `UnansweredStepChangesNothing` | advance 50, a renew that names a lease never granted | 2 |
| `quint/mutants/qm09-negative-lease-ttl.patch` | `pachinko_walk` sets `lease_ttl` to -1 | `LeaseTtlNonNegative` | none: the first state | 0 |
| `quint/mutants/qm09-negative-superseded-ttl.patch` | `pachinko_walk` sets `superseded_ttl` to -1 | `SupersededTtlNonNegative` | none: the first state | 0 |
| `quint/mutants/qm09-negative-dead-letter-ttl.patch` | `pachinko_walk` sets `dead_letter_ttl` to -1 | `DeadLetterTtlNonNegative` | none: the first state | 0 |

Notes on the table.

- **The fourth row is the README's third mutant**: a dead letter's expiry measured
  from `nacked_at` instead of its lease's end. It needs the clock to move between the
  nack and the lease's end. A mutant that stamped the entry from `now` instead of
  `ended_at` would have no trace, because a lease's end is one action at one `now`.
- **The second and third need the clock to move** between the grant and the extension.
  Without the advance the mutant writes the value the model writes.
- **Four invariants have no mutant here**: `NackedHasItsTime`, `OnlyNackedHasNackTime`,
  `ShedBeforeExpiry` and `NackedBeforeExpiry`. The hand-back reports them as checked
  and not mutation-tested (shared rule 6). Each has a one-line cut (a nack that does
  not stamp; an ack that does; a shed entry or a dead letter whose expiry subtracts the
  TTL) if the maintainer asks for them at step 3; each costs one more run.
- **The eighth row is the control for the clock's exception.** The property lets the
  clock move after `advance` and after a tick that finds nothing due, and after no
  other step with `fired` empty. A refused lease action that moves the clock is the
  step it must still catch. With `now` starting at 0 the renew alone shows it, in one
  step; the advance keeps the trace good if `now` starts at 1,000.
- **One more cut is offered at step 3 on the same terms as the four**, for the reading
  of effects on a tick. It cuts in the walk of the stages that `tick` and
  `TickIsItsStages` both read, after a lapse stage: the pocket's count is put back to
  what it was. `TickIsItsStages` cannot see it and no single step can show it; the
  expiry rule's effect, read on that stage, must catch it.

### `--max-steps`, and what it costs

- **Proposed: 12 for `pachinko_walk`.** Under a clock a lapse needs an advance of at
  least `lease_ttl` first, so QM08's deepest witness, `witness_ReviveParkedPocket`,
  goes from 8 steps to 10, and `witness_DeadLetterDecays` needs 10 too (two joins, a
  publish, a pull, a nack, a release, three advances to cover 250 ms, a decay). QM03's
  rule for the bound is the deepest witness's depth plus two. QM08's mutant
  `qm08-parking-skipped` needs 12 on this instance. `pachinko_zero_ttl` needs no
  advance and takes the same bound.
- **The cost** is not known and is the main risk of this ticket: two more steps of
  depth, a clock in every state, an `advance` with 241 amounts, four new actions, a
  second instance and eleven mutants. Step 2 measures the recipe before the change, step 5
  after the model, step 6 with the second instance and step 7 with the mutants. The
  budget is the whole job's, and these runs are the recipe alone, on another machine:
  each figure is quoted with the durations of the setup and the installs that QM03's
  hand-back quotes from CI added, against the head-room QM03's ruling set (its ticket
  proposes 15 of the job's 20 minutes), and the hand-back says the sum is an estimate
  until the job runs. If it does not fit, **stop** and ask
  (ruling 4). The domains are not shrunk and no instance is dropped to make it fit.
- **What to offer at that stop**, none of it chosen here: a bound per run in QM03's
  recipe; the deep lapse chains reached on `pachinko_zero_ttl` only; a longer timeout.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `verify-quint` (every instance, witness and mutant, with setup) finishes inside the CI job's timeout. Each run being fast is not enough | steps 2, 5, 6 and 7: the wall time of `just verify-quint`, with the setup and install durations from QM03's hand-back added, quoted beside the `quint` job's `timeout-minutes` and the head-room QM03's ruling set |
| A | A `verify` accepts a step that chooses an integer from a range of 241 values and compares sums of it with other integers. Not read, not run | step 2, on a scratch module under `ai_tmp/qm09/`; step 5 for the cost |
| B | A `verify` accepts a walk over a list whose order is computed from the state (the due leases, by `expires_at` and then by grant). Not read, not run | step 2, on the same scratch module |
| C | On `pachinko_zero_ttl` one tick can create a dead letter and decay it, so the entry is in neither `prev` nor the state. Reasoned from T42's ruling 3, not run | step 2: the plan writes the trace; step 8: observed by hand on the instance, if step 3 kept it. If step 3 ruled against the instance, the outcome recorded is "not run" |
| D | Each of the 32 rules is reachable with all three TTLs at 0. Reasoned, not run | step 6: 32 witnesses observed on `pachinko_zero_ttl`, if step 3 kept the instance. If it did not, the outcome recorded is "not run" |
| E | The step counts in the mutants table and under `--max-steps` are the shortest. Counted by hand, not run | steps 5 and 7: each witness and counterexample has its length. A shorter one is recorded; one outside the bound is a finding |
| F | Every witness QM02 to QM08 reached is still reached on `pachinko_walk` once a lapse must wait for its `expires_at` | step 5: all 32 observed |

## Non-goals

- No liveness: nothing here shows that a lapse or a decay ever happens (decision 0022).
- No removal for capacity. `NeverRefusedForCapacity`'s eviction order stays excluded,
  so in the model only a decay removes a shed entry.
- No choice the Allium has not made: how the clock moves, the order inside a tick, what
  a tick reports and whether it is atomic are read at step 2, not decided here.
- No `advance` or `tick` in `replayStep`. QM11 extends it and maps the two steps.
  Under `crates/` and `double/` only the conditional rows of Files touched change, and
  only to keep each replay green on the regenerated fixtures.
- No change to `scripts/verify-quint.sh`, to the format of `quint/verify.txt` or of a
  patch's header, or to the `quint` job in `.github/workflows/ci.yml`. A need there is
  handed back.
- No smaller domain, and no answer to either `open question` in the Allium.

## Shared rules (QM01 to QM13)

1. **The Allium decides.** `docs/specs/pachinko.allium` is the source of truth for
   behaviour (decision 0002). The model is derived from it and from nothing else. Read
   each rule and invariant in the Allium at the ticket's base, never from a ticket's
   summary of it, a page or a chat. No ticket reads the Rust or the double to decide
   what the model does. A ticket may read them for names, for the walk's domains and
   configuration, and, in QM10 to QM12, to map steps and state.
2. **The model never leads.** A counterexample has one of three causes. The model
   transcribes the Allium wrongly: fix the model. The property transcribes the Allium
   wrongly: fix the property, against the Allium's text. The Allium is wrong or silent:
   stop, write the trace and the question under Open points, and ask the maintainer. Never
   weaken an invariant, narrow a domain, drop a rule or add a guard the Allium does not
   have to make a run pass. Never settle either `open question` in the Allium.
3. **Names are fixed.** The model is `quint/pachinko.qnt`, module `pachinko`. An
   instance is a module `pachinko_<bound>` at its foot; the first is `pachinko_walk`.
   The sum type `Rule` has one variant for each Allium rule the model has so far, spelt
   as the Allium spells it.
   A rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`. An Allium invariant
   or guarantee is a property under its Allium name; `FiredMatchesGuards` and
   `UnansweredStepChangesNothing` check the labels. The ghosts are `fired`, `last`,
   `answer` and `prev`. A mutant is `quint/mutants/<ticket>-<slug>.patch`. Top-level
   actions carry `WalkStep`'s names (`crates/libpachinko/tests/queue/walk.rs`) in lower
   camel case. The gate is `just check-quint`, the verify recipe is `just verify-quint`,
   and the trace fixtures live in `quint/traces/`.
4. **One action per cascade, and every step is total.** A pull with its `HoldRecord`
   steps is one action. A lease's end with its `Freed` rules is one action. A publish
   with the revival it causes is one action. A step that no rule answers changes
   nothing and leaves `fired` empty; from QM09, `advance` and a `tick` with nothing due
   move only the clock. No guard and no effect on the state reads a ghost.
5. **The domains are a ruling.** `pachinko_walk` has the walk's domains and
   configuration: 2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3,
   pull limits -1 to 3 (-1 and 0 until QM05), `max_pull_batch` 2 and
   `max_consecutive_expiries` 2. A ticket
   that cannot verify at these inside the CI job's timeout stops and asks. It does not
   shrink a domain or drop an instance. QM03 sets each instance's `--max-steps`; a
   later ticket may raise it and never lowers it.
6. **Verified means this.** From QM03 on, `just verify-quint` shows all of it. `Canary`
   is violated on every instance. Every property passes on every instance, each named
   on the command line. Every rule the ticket owns has its witness reached. Every
   mutant the ticket lists typechecks and is caught by the one property it names, which
   may be any property the recipe checks, and which the unchanged model's pass on that
   instance shows to hold at the same bound. Each check fails closed: zero instances, a
   witness not reached, a patch that does not apply, a mutant that survives, a mutant
   caught by another property, and an error or a deadlock read as a catch are all
   failures. QM02 shows the same by hand for one witness and one mutant. An owned
   invariant that no listed mutant aims at is reported in the hand-back as checked and
   not mutation-tested.
7. **No clock before QM09.** QM02 to QM08 add no `now`, no timestamp field and no TTL.
   While the model has no clock, a specification change that only says when something
   is due changes nothing in the model, as long as the untimed lapse still covers it.
8. **What the model lacks is written down.** A behaviour the Allium allows and the
   model does not yet have is one line in the model's header comment, under "Not yet
   modelled", naming the ticket that adds it. That ticket deletes the line. A property
   that would fail only because of such a gap carries the gap as a named exception, and
   the same ticket removes the exception. A property is transcribed whole, conditions
   included, even where a condition is always true so far.
9. **Quint runs through the recipe.** Every Quint command is `just quint <args>`, run
   from the repository's root, never a bare `quint` on `PATH`. A claim about the CLI's
   behaviour is checked by running the pinned CLI and quoting its output, never from
   memory or from a page.
10. **The network is named, and the checker is the pinned one.** `just initialize`,
    `just install-quint`, `just install-apalache` and `just install-quint-evaluator`
    reach the network, and each is a named, authorised step in the ticket that runs it
    (`tickets/CONVENTIONS.md` §11, "Network"). `just check` reaches none. No step lets
    Quint download anything itself. Before any `verify`, show that nothing is listening
    on `localhost:8822`; never pass `--server-endpoint`; never run a `verify`, or any
    other `just quint` command that needs the evaluator, while `just install-apalache`
    or `just install-quint-evaluator` runs or while another worktree verifies. A `verify` talks
    to its own server over the loopback interface, so a network denial used as evidence
    for a `verify` denies the external network and allows loopback, and its negative
    controls are §11's two `curl` commands.
11. **One change carries the model and what depends on it.** A change to
    `quint/pachinko.qnt` re-cuts every mutant patch it breaks, keeping the mutation and
    the property it names, and runs `just verify-quint` once QM03 has landed that recipe.
    Once `quint/traces/` exists it also regenerates the fixtures, before any gate runs,
    because `just check-quint` then compares the model's hash, and keeps both replays
    green: the Rust test in `just test` and, once QM12 has landed, `just python-double`. A
    field or a step the model gains is either mapped in each replay or added to that
    replay's list of what it does not compare, naming the ticket that will map it. A step
    on a replay's list ends that replay of every trace that holds it, so a ticket lists a
    step only with the maintainer's answer on which ticket maps it. The patches, the
    fixtures, their generator and the replays are in the Files touched of every ticket
    that changes the model, and the hand-back lists each patch re-cut. A change to
    `docs/specs/pachinko.allium` that touches a modelled rule, invariant or guarantee
    updates the model in the same change, runs `just check-quint`, and runs
    `just verify-quint` if the recipe exists.
12. **Files.** Edit only the files the ticket lists, plus its own `status:` line and
    hand-back notes. A shared file is listed as `shared, own lines`. A change needed
    elsewhere is handed back, not made (`tickets/CONVENTIONS.md` §11). Scratch goes in
    `ai_tmp/<ticket>/` and is never committed.
13. **Evidence, and the series base.** Each quoted output is headed by the commit it ran
    at and shows its exit status. A check that passes by printing nothing follows its
    positive control (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"
    and "Checks fail closed"). Where §11 says `main`, read the series base the
    maintainer named: for the worktree's start, for the closing fetch and merge, and
    for the evidence quoted afterwards. Pushing and opening a pull request are
    separately authorised.

## Adversarial review

This ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. These deltas
apply, and every ticket of the series carries them word for word.

- **Reviewer.** Codex, Sol, which is `codex exec -m gpt-6.2-sol`. The maintainer named
  Sol for this series on 2026-11-25, in place of the model §14 prints. Confirm
  `codex exec --help` on the day, as §14 asks. Fall back to a fresh-context
  `claude-opus-5-5` subagent only if `codex` is not on `PATH`, and say so in the
  hand-back notes.
- **A usage limit is not a clean review.** If a Codex log ends in a usage-limit error,
  the lens is incomplete. Stop and ask the maintainer whether to wait or to use the
  fallback. Never record it as "no findings".
- **Command.** Run this from the worktree, once per lens:

  ```sh
  dir="$PWD/ai_tmp/review/<id>-<n>"
  mkdir -p "$dir"
  codex exec -m gpt-6.2-sol -s read-only -C "$PWD" \
    -o "$dir/last-<lens>.md" "<brief>" \
    < /dev/null >| "$dir/log-<lens>.txt" 2>&1 &
  pid=$!
  wait "$pid"
  echo "exit $?"
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens. A
  second round uses `<lens>-2` in all three names. `< /dev/null` closes stdin; without
  it the run waits forever. `-o` holds only the last message. `>|` is for a shell with
  `noclobber`. Under `-s read-only` Codex cannot write files, so the brief tells it to
  put its complete findings in its final message, and the runner copies
  `last-<lens>.md` to `findings-<lens>.md` under a first line `Source: last-message`.
  If §14 on the ticket's base already prints a command that redirects stdin from
  `/dev/null`, use §14's, with Sol's model.
- **Completion.** A lens is complete when its process, followed by the process id the
  command captured, has exited 0 and `last-<lens>.md` is not empty. Never wait on a
  file name or a `pgrep` pattern, which can match a run in another worktree. A lens
  that exits non-zero or leaves an empty file runs again, once; a second failure stops
  the ticket.
- **Lenses.** `conformance` and `breakage`, as §14 defines them. A checkpoint marked
  "with a design lens" adds a third reviewer, `design`: does the model say what the
  Allium says, is anything the Allium allows missing without being listed under "Not
  yet modelled", and could a property hold for a reason other than the one claimed.
- **Brief.** It gives the ticket's path, `tickets/CONVENTIONS.md`,
  `docs/specs/pachinko.allium`, the checkpoint's questions, and the artefact. At a plan
  checkpoint the artefact is the plan file. At a checkpoint on a model or a recipe it
  is the files the ticket names there, with the output of the controls run so far. At
  a diff checkpoint it is `git diff "$base"...HEAD` plus the uncommitted changes.
  `base` is the commit step 1 records. The brief never gives the author's reasoning or
  the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one
  writer at a time. Before the lead runs `just check` or `just verify-quint`, it
  confirms every writer has stopped.
- **After the last checkpoint.** A merge or a rebase onto the series base, or a fix made
  after the last checkpoint, as after a red CI run, that changes a file under the
  ticket's Files touched was not reviewed. The agent shows that diff
  at the push stop and asks whether the last checkpoint runs over it first.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

`<id>` is `QM09`. Checkpoints:

1. The plan, `ai_tmp/qm09/plan.md`, with the conformance, breakage and design lenses,
   in step 3, before any earlier rule is changed. A **Stop** for the maintainer's
   ruling follows it.
2. The diff, with the conformance, breakage and design lenses, in step 10, before the
   final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | if the tree this ticket lands on holds QM13's `StateInRange`, at the base or after the closing merge or rebase, its clauses for the new state; QM02's `FiredMatchesGuards`, which gains the tick's case; QM06's two definitions of the entries a step added, which gain the tick's reading; QM02's `UnansweredStepChangesNothing`, completed for a step that moves the clock; the body of every `effect_` as a definition over a pair of states, applied stage by stage on a tick; the clock, the timestamp fields and the three constants; the stamps in every rule that writes one, and their effects; the lapse's time condition; two variants of `Rule` and four of `Step`; `decaySuperseded`, `decayDeadLetter`, `advance` and `tick`; `guard_`, `effect_` and `witness_` for the two decay rules; the nine invariants, `ExpiryRecoversWithinTtl` and `TickIsItsStages`; the clauses added to earlier properties; `pachinko_zero_ttl`; every "Not yet modelled" line that names QM09 removed |
| `quint/mutants/qm09-expiry-before-grant.patch` | new | the mutant of that name |
| `quint/mutants/qm09-extension-does-not-extend.patch` | new | the mutant of that name |
| `quint/mutants/qm09-ack-repeat-does-not-extend.patch` | new | the mutant of that name |
| `quint/mutants/qm09-dead-letter-expiry-from-nack.patch` | new | the mutant of that name |
| `quint/mutants/qm09-decay-before-expiry.patch` | new | the mutant of that name |
| `quint/mutants/qm09-lapse-before-expiry.patch` | new | the mutant of that name |
| `quint/mutants/qm09-now-runs-back.patch` | new | the mutant of that name |
| `quint/mutants/qm09-refused-action-moves-clock.patch` | new | the mutant of that name |
| `quint/mutants/qm09-negative-lease-ttl.patch` | new | the mutant of that name |
| `quint/mutants/qm09-negative-superseded-ttl.patch` | new | the mutant of that name |
| `quint/mutants/qm09-negative-dead-letter-ttl.patch` | new | the mutant of that name |
| `quint/mutants/` | shared, own lines | every earlier patch, re-cut with its mutation and its property kept, since every earlier rule changes (shared rule 11) |
| `quint/verify.txt` | shared, own lines | `pachinko_walk`'s bound, and a line for `pachinko_zero_ttl` |
| `quint/traces/` | shared, own lines | only if the tree this ticket lands on holds `quint/traces/`, at the base or after the closing merge or rebase: the fixtures regenerated by `just quint-traces`, never by hand, before any gate runs (shared rule 11) |
| `scripts/quint-traces.py` | shared, own lines | the fixtures' generator, created by QM10; on the same condition, and only if a rule this ticket adds is one `replayStep` reaches, which neither decay rule is: its witness list. No edit is expected; one that is made is recorded under Departures |
| `crates/libpachinko/tests/queue/walk/replay.rs` | shared, own lines | on the same condition: the model's `now`, and `clock` if it has one, added to the list of what is not read from the fixture, naming QM11; the timestamp rows stay, as QM10 wrote them. Created by QM10 |
| `crates/libpachinko/tests/queue/walk/model/trace.rs` | shared, own lines | on the same condition, and only if the decoder refuses a state with the new fields: the fields accepted and not read. Created by QM10 |
| `double/tests/test_replay.py` | shared, own lines | only if the tree this ticket lands on holds `double/tests/test_replay.py`, at the base or after the closing merge or rebase: the timestamps leave its exclusion list and are compared, and the double's clock starts where QM12's ticket says for the clock that landed: at the first state's `clock` under either two-value clock, and at its `now` under one value. Created by QM12 |
| `docs/decisions/` | shared, own lines | in the record QM02 wrote as decision 0022 only: how the model's clock follows the landed Allium; `ExpiryRecoversWithinTtl`'s fragments and the sentences not checked; the zero-TTL instance and the mixed settings checked nowhere; how each kind of property is read on a tick, and for any exemption a ruling grants, what it leaves unchecked; how `UnansweredStepChangesNothing` reads a step that moves the clock |
| `.scratch/quint-model/QM09-clock-timestamps-decay.md` | ticket | `status:` and the hand-back notes; and, only if a ruling at step 3 differs from a recommended answer or T47's ruling replaced the config-bound control, the sentences of the Goal, "Design proposed", "Claims this ticket checks", Files touched, the steps, acceptance and Verification it changes, each recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM09's Status cell; and, only if a ruling at step 3 differs from a recommended answer or from the README's own sentence on the clock's exception, the sentences it changes, the row for QM09 under "What each ticket delivers" among them |
| `.scratch/quint-model/QM*.md` | shared, own lines | Only if a ruling at step 3 differs from a recommended answer: the sentences it changes in the later tickets the README's index names. QM11 is written against all three clocks and the recommended `tick` |

## Steps

1. **Worktree and baseline.** If `depends_on` still holds `T42`, **stop**: the
   specification ticket has no id, so it cannot have merged. Otherwise create the
   worktree on `ticket/qm09-clock-timestamps-decay` from the series base the maintainer
   names, once QM08, T41 and that ticket have merged there, and record the base's full
   commit id as `base`. `git ls-files .scratch/quint-model/` must list the README and
   this ticket. Run `just initialize` (network, authorised). Run `just check` on the
   untouched worktree and quote its last line, headed by the commit.
2. **Read what landed, before writing.** Write `ai_tmp/qm09/landed.md`: for each of T42's
   seven open points and each question on T41's page, the ruling, the line of the Allium
   or of the decision record that states it, and the row of "Design proposed" it selects.
   A point the Allium still leaves open is written as open, never filled in. If the lapse
   intent landed, or a ruling selects no row, **stop** and ask. Then read the model and
   the hand-backs of QM03 and QM05 to QM08; record the bound, the lease-end definition,
   the "Not yet modelled" lines, and whether `replayStep`, `quint/traces/`,
   `double/tests/test_replay.py` and `StateInRange` exist. Count the variants of `Rule`
   and, if the count is not 30, name the T ticket that changed it (Context, "The counts
   in this ticket are today's"). Show nothing listens on
   `localhost:8822`, run `just verify-quint` and quote its wall time. Here and in every
   later step, each `verify`, by hand or through the recipe, follows a port probe of
   its own, the one Verification writes out (shared rule 10). Run claims A and B
   on a scratch module through `just quint`. Then write `ai_tmp/qm09/plan.md`:
   - the clock: its state, `advance`, which steps move `now`, and what a reading behind
     the latest time does, each beside the Allium's line; what `now` is before any
     step reads the clock, and what `advance` does at 0; and
     `UnansweredStepChangesNothing` as the README words it, by the step `last` names;
   - if the selected row has two values: the property that fails when an answered step
     that reads the clock leaves `now` as `prev` had it although `clock` was ahead of
     it, with its trace (a forward `advance`, then the step); if no planned property
     fails on it, the plan says so, for the **Stop** in step 3 (Open points). Under
     the one-value row nothing is owed here: `now` is the clock;
   - if the selected row refuses a reading behind the latest time: every property that
     says a step is answered, and `TickIsItsStages`, each with the trace on which it
     fails as written, and the Allium's line on a tick taken at such a reading;
   - each of the two owned rules and each rule that gains a stamp, the Allium's text
     beside the planned action, `guard_` and `effect_`;
   - `tick`, its stages, their order, and `TickIsItsStages`, with the clause that states
     the order from the Allium's text where the Allium fixes one;
   - each property and each added clause with the ghosts it reads;
   - each mutant as a diff against the base's model, with its property, its instance
     and its trace;
   - each earlier patch and whether it must be re-cut;
   - every property of the model, marked safe under a tick or put under one of the
     four kinds of "Design proposed", with the trace at zero TTLs that decides it;
   - each "Not yet modelled" line removed;
   - the bound, with each witness's depth under the clock;
   - if the tree holds QM13's `StateInRange`, the clauses it gains.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14: conformance,
   breakage and design, over `ai_tmp/qm09/plan.md` with `ai_tmp/qm09/landed.md`, before
   any earlier rule is changed). Questions:
   - Does `ai_tmp/qm09/landed.md` or the plan state anything about the clock that the
     Allium at the base does not say? Can `now` decrease on any planned step, or a
     refused step move it?
   - Under either two-value row, with `clock` ahead of `now` after a forward `advance`,
     would a pull, an ack, a nack or a renew that is answered and leaves `now` at
     `prev`'s value fail a planned property, and which one? If the selected row
     refuses a reading behind the latest time, which planned properties fail on a
     publish, a repeated ack or a tick that the clock alone refuses?
   - Take a tick on `pachinko_zero_ttl` that lapses a lease with a nacked record. Which
     properties of the model read `prev`, the state, `last` or the entries a step
     added in a way that fails on it, and does the plan list every one?
   - Is every stamp in the plan the Allium's expression, and is every entry a lease's
     end makes stamped from `ended_at`?
   - Can `tick` do anything a sequence of `lapse` and decay steps at one `now` cannot,
     or leave something due? Where the Allium fixes an order, would a tick that takes
     the due leases in another order pass `TickIsItsStages` as planned?
   - With `fired` empty, what does the planned `UnansweredStepChangesNothing` let
     change? Anything but the clock's own move on an `advance`, and on a tick with
     nothing due if the Allium has one move it? Is the exception keyed on the step
     `last` names, so that a refused lease action cannot use it?
   - Is any `effect_` planned to skip a tick, and if the ruling may allow it, does the
     plan say what is then left unchecked?
   - Could a fragment of `ExpiryRecoversWithinTtl` hold because no trace inside the
     bound moves the clock past a TTL?
   - Does each mutant have a trace on which it differs from the model?

   Then **stop** for the maintainer's ruling on "Design proposed", with the findings in
   hand, and record it. No tracked file changes before the ruling. If it differs from a
   recommended answer, make the edits the head of "Design proposed" lists, to this
   ticket, the README and the later tickets, before step 4, and list each in the
   hand-back.
4. **Red.** Add the clock, the fields, the constants in `pachinko_walk`, every stamp,
   `advance`, the nine invariants, `ExpiryRecoversWithinTtl`, and the two decay rules'
   variants, guards, effects and witnesses, leaving the lapse untimed and adding no
   decay action and no `tick`. Complete `UnansweredStepChangesNothing` for `advance`
   here, as ruled: left as it was, it fails on the first `advance` and the red run shows
   that instead of the lapse. Before either gate, bring what reads the model up to it
   (shared rule 11), here and after every later edit of the model. QM03's ticket
   proposes a recipe that applies every patch before it verifies anything, and every
   earlier rule changes: re-cut every earlier patch now, or the run stops at "does not
   apply". And if the tree holds `quint/traces/`, run `just quint-traces`, before any
   gate (shared rule 11): QM10 makes
   `just check-quint` refuse a model whose hash is not the manifest's, and ends
   `just verify-quint` on a diff of the regenerated fixtures. Bring each replay that
   exists up here too, before the next gate: `just test`, and `just python-double`
   where `double/tests/test_replay.py` exists, pass on the regenerated fixtures, with a
   field or a step the model gained mapped or listed as shared rule 11 says. Step 8
   then holds their controls. `just check-quint` passes. `just verify-quint` fails.
   By hand, with the commands the recipe prints, quote: `ExpiryRecoversWithinTtl`
   violated on a trace that lapses a lease before its `expires_at`, and both decay
   witnesses unreached. A recipe that passes here is a finding against it: stop.
5. **Green.** Give the lapse its time condition. Add the two decay actions and `tick`,
   `TickIsItsStages`, the tick's case in `FiredMatchesGuards`, in
   `UnansweredStepChangesNothing` and in every `effect_`, and
   the tick's reading of "the entries a step added". Complete the two earlier
   properties. If the tree holds QM13's `StateInRange`, extend it. Re-cut every earlier
   patch this step breaks, and regenerate the fixtures if they exist, as at step 4.
   Set the bound. Show the port free, run `just verify-quint` and quote it:
   `Canary` violated, every property passing, all 32 witnesses observed (claim F),
   every earlier mutant caught. Quote the wall time beside the `quint` job's
   `timeout-minutes`, counted as "`--max-steps`, and what it costs" says. If it does
   not fit, **stop** and ask.
6. **The second instance**, unless step 3 ruled against it, in which case this step is
   the line in decision 0022 and nothing else. Add `pachinko_zero_ttl` and its line in
   `quint/verify.txt`,
   and regenerate the fixtures if they exist: the model's file changed.
   Show the port free, run `just verify-quint` and quote, for that instance: `Canary`
   violated, every
   property passing, 32 witnesses observed (claim D). Quote the wall time. If a rule is
   unreachable or the recipe no longer fits, **stop** and ask.
7. **Mutants.** Add the eleven patches. Show the port free. For each, quote from
   `just verify-quint`: the
   patch applied, the mutant typechecked and the named property violated. The unchanged
   model's pass on the mutant's instance, quoted at steps 5 and 6, is what shows each
   named property holds at the same bound (shared rule 6). Record each
   counterexample's length (claim E). Quote the wall time.
8. **Controls.** By hand, through `just quint`, the port shown free first:
   - `lateRenew` is observed on `pachinko_walk`, and claim C's trace on
     `pachinko_zero_ttl` if step 3 kept that instance, each from a scratch copy that
     defines the observation.
   - **The replays** (shared rule 11). If the tree holds `quint/traces/`: `replayStep`
     is unchanged in the diff. Run `just quint-traces`, then `just quint-traces diff`,
     which QM10's ticket proposes as the read-only comparison of a fresh generation
     with the fixtures in the tree: it must exit 0. The control names no earlier run,
     so it holds when the fixtures first arrive in the closing merge and after the
     header's edit at step 9. The fixtures now carry `now` and
     the timestamps, every stamp at the start instant or a TTL after it, since no step
     of `replayStep` moves the clock; no fixture holds an `advance`, a `tick`, a lapse
     or a decay. Keep the Rust replay green: add the new state to its list of what is
     not read from the fixture, naming QM11, which maps the timestamps and the two
     steps. This is a change to Rust under `crates/`, so `AGENTS.md`'s block for it
     applies: use the `rust-change` skill, and run `just fmt-check`, `just clippy` and
     `just test`. If the tree holds `double/tests/test_replay.py`: bring it
     to the case its ticket describes for "QM09 is in the base and QM11 is not", and
     run `just python-double`. Any other change a replay needs is handed back.
9. **Words.** Add the lines to decision 0022. Remove every "Not yet modelled" line that
   names QM09. The header is part of the model's file, so this is an edit of the model:
   if the tree holds `quint/traces/`, run `just quint-traces` after it and before the
   next gate, and re-cut any patch it moved. Run `just check-docs`.
10. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14: conformance,
    breakage and design, over `git diff "$base"...HEAD` plus the uncommitted changes,
    before the final commit). Questions:
    - Is there a rule in the Allium that stamps a time and has no stamp, or no clause
      in its effect, in the diff?
    - Is any `effect_`, or any other property, not checked on a stage of a tick, so
      that a broken `lapse` or decay could hide inside one? Does `TickIsItsStages` take
      the order from `last` alone where the Allium fixes one? Are
      the tick's stages the definitions the single actions call? Does `tick` apply
      its stages in the order `last` records, and work out no other order of its own?
    - Does `step` offer `advance` for every amount from -120 to 120, `tick`, and
      `lapse` and the two decays for every lease and every place they may name,
      whatever the state holds?
    - Under either two-value row, with `clock` ahead of `now` in `prev`, does a
      property fail on an answered pull, ack, nack or renew that leaves `now` as `prev`
      had it?
    - With `fired` empty, does `UnansweredStepChangesNothing` let anything change but
      the clock, and the clock by anything but the amount an `advance` names?
    - Does each re-cut patch still make the mutation its name says, against the
      property it named before?
    - Could `EndedAfterGrant` or `GrantedBeforeExpiry` hold only because no trace
      inside the bound steps the clock back?
    - Is any decay, eviction or other removal of a shed entry possible outside the two
      decay rules?
    - Does decision 0022 say which sentences of the guarantee are not checked, and
      which TTL settings no instance has?
    - Is the body of each `witness_<Rule>` this ticket adds `fired.contains(<Rule>)` and
      nothing else? The recipe reaches a witness and never reads its body.
11. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed
    by the full commit id. Set `status: done` here and in
    `.scratch/quint-model/README.md`, write the hand-back notes, with each patch re-cut
    listed and each sentence step 3's ruling changed in this ticket, the README or a
    later ticket listed, and commit the record. Then run `just check` once more, on the
    record's commit, and show its last line and an empty `git status --short`. If it
    fails, fix the notes, commit again and run it again; never amend the work's commit.
    Before asking to push, fetch (network,
    authorised), merge or rebase onto the series base, and apply shared rule 11 to
    whatever that brought in (QM10, QM12 or QM13 may have merged meanwhile). Read again,
    against the merged tree, every condition here that turns on the tree holding
    `quint/traces/`, `double/tests/test_replay.py` or QM13's `StateInRange`. If the
    merge brought `StateInRange` in, add its clauses for the clock and every timestamp
    now, as step 5 does; that is an edit of the model, so re-cut and regenerate as at
    step 4. If it brought the fixtures or the double's replay in, do
    step 8's replay work now, regenerating the fixtures before any gate. Commit what
    these edits changed, as work and apart from the record, before the reruns:
    Verification runs after a commit and ends on an empty `git status --short`, which
    a tree holding those edits uncommitted fails. Then rerun
    `just check`
    and Verification, and requote both (`tickets/CONVENTIONS.md` §11, "Green"; shared
    rule 13). If the base had moved, the quotes already committed are stale
    (§11, "Quoted evidence names its commit"): put the fresh ones in the hand-back,
    headed by the commit they ran at, which is the merged commit or the work committed
    on top of it, commit that record, and run `just check` on it once
    more, with an empty `git status --short`, as after the first record.
    If the merge or rebase changed a file under Files touched, or work was committed
    after it, checkpoint 2
    did not see that change: show its diff at the stop and ask whether it is reviewed
    before the push. **Stop** before pushing or opening a pull request: both need
    authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `ai_tmp/qm09/landed.md` is quoted in the hand-back: every ruling with the line that
  states it, and every point still open marked open.
- `just check-quint` passes, quoted.
- `just verify-quint` passes, quoted: on `pachinko_walk` and on `pachinko_zero_ttl`,
  `Canary` violated, every property named and passing, and all 32 witnesses observed,
  `witness_SupersededRecordDecays` and `witness_DeadLetterDecays` among them.
- Each of the eleven mutants typechecks and is caught by the property the mutants table
  names, quoted, and every earlier mutant is still caught. The hand-back lists every
  patch re-cut.
- Step 4's red run is quoted: `ExpiryRecoversWithinTtl` was violated by the untimed
  lapse, and the decay witnesses were unreached.
- The wall time of `just verify-quint` before the change, after the model, with the
  second instance and with the mutants is quoted beside the `quint` job's
  `timeout-minutes`, each with the setup and install durations from QM03's hand-back
  added. No bound is lower than at `base`.
- No domain of `pachinko_walk` is smaller than at `base`, and `advance` ranges over
  -120 to 120, shown by the diff of the instance.
- The header's "Not yet modelled" list has no line naming QM09.
- Every lease extension's effect states `expires_at = now + lease_ttl`, and the
  hand-back names the one definition they call.
- `TickIsItsStages` passes on both instances, and the hand-back says which order a tick
  uses and where the Allium fixes it, or that it fixes none. Where it fixes one, the
  hand-back quotes the clause of the property that states it, which does not read the
  order from `last`.
- The hand-back lists every property of the model as safe under a tick, read stage by
  stage on one, or reading the tick's own entries, as the plan did. A property exempt
  from a tick is there only by the ruling at step 3, with what that leaves unchecked.
- `UnansweredStepChangesNothing` is quoted as the README words it: with `fired` empty,
  nothing but the clock differs from `prev`, and the clock only after `advance` or a
  tick that finds nothing due, read from `last`, and only as "How the clock moves"
  says. `qm09-refused-action-moves-clock` is caught by it.
- The hand-back says what `now` is before any step reads the clock and what `advance`
  does at 0, as ruled, and what that leaves QM11 able to replay.
- Under either two-value row, the hand-back names, as ruled at step 3, the property
  that fails when an answered step leaves `now` as `prev` had it although `clock` was
  ahead of it, or records the ruling that none is required. Under the one-value row
  nothing is owed here. If the selected row refuses a reading behind the latest time,
  the hand-back lists each property that says a step is answered, and
  `TickIsItsStages`, with the ruling on each.
- `EveryDropIsVisible` no longer holds QM07's "no step removes one", and its clause on
  removal counts places, shown by quoting it.
- If the tree this ticket lands on holds QM13's `StateInRange`, at the base or after
  the closing merge or rebase, its inductive run passes inside `just verify-quint`,
  with the clauses this ticket added named in the hand-back.
- `lateRenew` is observed, quoted.
- The four invariants with no mutant are reported as checked and not mutation-tested.
- Decision 0022 holds the lines Files touched lists.
- Claims 12 and A to F each have an outcome recorded.
- Both checkpoints, and the ruling between them, are recorded as §14's Record asks. If
  the ruling differed from a recommended answer, the hand-back lists each sentence
  edited here, in the README and in a later ticket.
- If the tree this ticket lands on holds `quint/traces/`, at the base or after the
  closing merge or rebase, the fixtures were regenerated by `just quint-traces`,
  none holds an `advance`, a `tick`, a lapse or a decay, the Rust replay's list names
  QM11 for everything this ticket added, and `just test` passes, quoted.
- If that tree holds `double/tests/test_replay.py`, the double's replay compares the
  timestamps and `just python-double` passes, quoted.
- After the record's commit, `just check` was run once more and its last line and an
  empty `git status --short` are quoted.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script, from the repository's root, after the commit.

```sh
set -eu
git rev-parse HEAD
model=quint/pachinko.qnt
mkdir -p ai_tmp/qm09

for rule in SupersededRecordDecays DeadLetterDecays; do
  grep -qw "guard_${rule}" "$model"
  grep -qw "effect_${rule}" "$model"
  grep -qw "witness_${rule}" "$model"
done
for name in NackedHasItsTime OnlyNackedHasNackTime ShedBeforeExpiry NackedBeforeExpiry \
  GrantedBeforeExpiry EndedAfterGrant LeaseTtlNonNegative SupersededTtlNonNegative \
  DeadLetterTtlNonNegative ExpiryRecoversWithinTtl ExpiryRecoversWithinTtl_untimed \
  TickIsItsStages pachinko_zero_ttl; do
  grep -qw "$name" "$model"
done

# The clock is in the code lines, and nothing is left for QM09 in the header.
grep -v '^ *//' "$model" >| ai_tmp/qm09/code.txt
for word in now granted_at expires_at ended_at nacked_at shed_at lease_ttl \
  superseded_ttl dead_letter_ttl; do
  grep -qw "$word" ai_tmp/qm09/code.txt
done
grep -F 'Not yet modelled' "$model"
if grep -F '(QM09)' "$model"; then exit 1; else test "$?" -eq 1; fi

set -- quint/mutants/qm09-*.patch
test "$#" -eq 11
for patch in "$@"; do
  grep -q '^# instance: pachinko_' "$patch"
  grep -q '^# property: ' "$patch"
done
grep -Fx '# property: GrantedBeforeExpiry' quint/mutants/qm09-expiry-before-grant.patch
grep -Fx '# property: effect_Renew' quint/mutants/qm09-extension-does-not-extend.patch
grep -Fx '# property: MarksStandHoweverLeaseEnds' quint/mutants/qm09-ack-repeat-does-not-extend.patch
grep -Fx '# property: effect_DeadLetterNackedRecord' quint/mutants/qm09-dead-letter-expiry-from-nack.patch
grep -Fx '# property: EveryDropIsVisible' quint/mutants/qm09-decay-before-expiry.patch
grep -Fx '# property: ExpiryRecoversWithinTtl' quint/mutants/qm09-lapse-before-expiry.patch
grep -Fx '# property: EndedAfterGrant' quint/mutants/qm09-now-runs-back.patch
grep -Fx '# property: UnansweredStepChangesNothing' quint/mutants/qm09-refused-action-moves-clock.patch
grep -Fx '# property: LeaseTtlNonNegative' quint/mutants/qm09-negative-lease-ttl.patch
grep -Fx '# property: SupersededTtlNonNegative' quint/mutants/qm09-negative-superseded-ttl.patch
grep -Fx '# property: DeadLetterTtlNonNegative' quint/mutants/qm09-negative-dead-letter-ttl.patch

for instance in pachinko_walk pachinko_zero_ttl; do
  bound=$(sed -n "s/^${instance} max-steps=//p" quint/verify.txt)
  test "$bound" -ge 12
done

just check-quint

# The port probe, as .scratch/apalache/AP01-java-and-verify.md writes it.
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

started=$(date +%s)
just verify-quint >| ai_tmp/qm09/verify.log 2>&1
echo "verify-quint: $(( $(date +%s) - started )) seconds"
grep -n 'timeout-minutes' .github/workflows/ci.yml
for name in witness_SupersededRecordDecays witness_DeadLetterDecays \
  witness_ReviveParkedPocket NackedHasItsTime OnlyNackedHasNackTime ShedBeforeExpiry \
  NackedBeforeExpiry GrantedBeforeExpiry EndedAfterGrant LeaseTtlNonNegative \
  SupersededTtlNonNegative DeadLetterTtlNonNegative ExpiryRecoversWithinTtl \
  TickIsItsStages pachinko_walk pachinko_zero_ttl Canary; do
  grep -qw "$name" ai_tmp/qm09/verify.log
done
for patch in "$@"; do grep -qF "$(basename "$patch" .patch)" ai_tmp/qm09/verify.log; done

if test -d quint/traces; then
  just quint-traces check
  grep -lw 'Pull' quint/traces/*.json
  if grep -lwiE 'advance|tick|lapse' quint/traces/*.json; then exit 1; else test "$?" -eq 1; fi
fi
if test -f double/tests/test_replay.py; then
  just python-double
fi

just check
status=$(git status --short)
test -z "$status"
```

The red tests are the eleven mutants, which `just verify-quint` passes only if each is caught
by the property its header names; the header checks pin each patch to the property this
ticket gives it, so a patch re-aimed at an easier property fails here. The green tests are
`just check-quint`, `just verify-quint` on both instances with the witnesses and the
canaries, `just quint-traces check` where the fixtures exist, `just python-double` where
the double's replay exists, and `just check`, which runs the Rust replay in `just test`.
Three probes pass by printing nothing, and each follows a positive control over the same
input: the search for a line naming QM09 follows a search of the same file for the header
that matches; the port probe follows the listener the script starts and then kills; and
the search for a clock step in the fixtures follows the search for a pull there.

The script assumes what earlier tickets fix, and a difference is recorded under Departures
with the corrected script kept in `ai_tmp/qm09/`: that the model's header comment uses
`//` lines; that `quint/verify.txt` and a patch's header have the form QM03 recommends;
that the bound ruled at step 3 is 12; that the log of `just verify-quint` names each
instance, property, witness and mutant it handles, a mutant by its patch's name without
`.patch`, as QM03's ticket proposes; and that a fixture names each step by
its `Step` variant. The script times the recipe alone; steps 5 to 7 hold the sum with
the job's setup. Step 4's red run and step 8's controls are quoted in the hand-back and
are not rerun here.

## Hand-back notes

### Base and branch

### Baseline

### What landed from T41 and T42

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Departures

### Verification

## Open points

- **The specification ticket's id.** Whoever creates the ticket T42 names replaces
  `T42` in this frontmatter and in the README's index. Until then the ticket is blocked.
- **Everything T41 and T42 leave open.** The order inside a tick, whether a tick is
  atomic, whether an empty tick moves `now`, what a tick reports, the two repeated
  `requires`, and the lapse intent are the maintainer's, ruled through T42. This ticket
  reads the rulings at step 2 and stops where one selects no row of "Design proposed".
- **The tick and the two-state properties.** A tick that creates and decays an entry in
  one step touches `FiredMatchesGuards`, the effects and every property that counts
  what a step added. The recommended answer edits a property QM02 owns, two
  definitions QM06 owns and the body of every `effect_`. The maintainer rules on it at
  step 3. The alternative, an exemption for a tick, is less to write and checks less;
  "Design proposed" says what it leaves out.
- **A step that moves the clock and fires no rule.** `advance` changes the state with
  `fired` empty, and so may a tick with nothing due. The README's second design block
  and shared rule 4 now state the exception, keyed on the step `last` names, and this
  ticket writes the property to that wording. What stays open is what a tick that
  finds nothing due does to the clock (T42's open point 3), and the README's sentence
  changes only if the ruling at step 3 differs from it.
- **The clock's first values and its floor.** What `now` is before any step reads the
  clock, and whether the clock stops at 0, are the Allium's to say and, where it is
  silent, the maintainer's at step 3. The recommended answers are the ones a replay
  over a negative advance needs; under the alternatives QM11 replays forwards only.
- **The clock that refuses, and the properties that say a step is answered.** Under
  the row that refuses a reading behind the latest time, a publish, a repeated ack or
  nack and a tick can each go unanswered because of the clock alone.
  `PublishAnsweredByOneRule`, `NeverRefusedForCapacity`, clause 2 of
  `MarksStandHoweverLeaseEnds` with QM06's part, and `TickIsItsStages` are written as
  if none could, so a faithful model fails them, and a builder who must reach green
  could leave the conjunct out or weaken a property. Whether each gains the clock's
  condition in the Allium's words, or the landed Allium answers such a step another
  way, is the maintainer's at the **Stop** in step 3. The first four are properties of
  QM04, QM07, QM08 and QM06, and "Design proposed" names no edit to them.
- **An answered step that does not read the clock.** Fragment 2 and each extension's
  effect compare `expires_at` with the state's `now`, and the exact clock checks cover
  `advance`, an empty tick and a refused step. Apart from a tick, whose walk starts
  from the read, nothing written here fails when, under either two-value row, an
  answered step skips its read and stamps from a stale `now`: after a forward
  `advance`, such a renew passes fragment 2 and `effect_Renew`, and none of the eleven
  mutants is that fault. What states the read, a clause in each
  effect in the form the selected row gives it or something else, is the maintainer's
  at the **Stop** in step 3, with the plan's answer to step 2's bullet in hand.
- **The order a tick records and the order it takes.** `step` writes the order into
  `last`, and `TickIsItsStages` checks that order against the Allium and replays it.
  Where two orders end in the same state, `fired` and `answer`, as two decays do when
  no report names them, a `tick` that took the other order passes every property, and
  the order `last` shows a reader or a fixture is then not the one taken. Checkpoint 2
  reads `tick` for it. Whether that reading is enough is the maintainer's at the
  **Stop** in step 3.
- **A T ticket that lands first.** The counts and traces here are the Allium's at
  `cf1f3f3`. T45 changes both. Step 2 reconciles them; nothing in T45 or in the README
  edits this ticket when T45 lands.
- **Mixed TTL settings.** One zero TTL beside two that are not is checked on no
  instance. A third instance is the maintainer's to ask for, at step 3.
- **Cost.** If the walk's domains with a clock do not fit the timeout, the maintainer
  chooses among the options under `--max-steps` at the stop in step 5, 6 or 7.
- **The Rust and a refused publish.** `tickets/T42-clock-model.md` reads the Rust as
  advancing its latest time on a publish refused for an equal rank. If that still
  holds when QM11 runs, QM11 finds it; the ticket that lands the clock in the Rust
  settles it.
