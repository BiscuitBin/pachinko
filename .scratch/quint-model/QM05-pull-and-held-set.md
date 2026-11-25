---
id: QM05
title: "Pull and the held set"
status: todo
depends_on: [QM04]
parallel_with: []
branch: ticket/qm05-pull-and-held-set
estimated_size: L
---

# QM05: Pull and the held set

## Context

After QM04 the Quint model, `quint/pachinko.qnt`, starts from an empty deployment.
Publishers and consumers join, four publish rules build pockets and records, a consumer
claims the front pocket and releases it. Nothing is ever handed out: a pull's limit
ranges over -1 and 0, so `PullClaimsFrontPocket` creates a lease and an empty batch,
which is what `docs/specs/pachinko.allium` says of a limit that is not positive. A
consumer that pulls again while it holds a lease is answered by no rule, because
`PullWithinLease` is not in the model. Every record is queued.

This ticket lands the full pull. The limit ranges over -1 to 3, the walk's domain. A
pull hands out the oldest queued records of the pocket, at most `max_pull_batch` of
them, and each becomes held at the rank and payload it was handed out with. A release
ends the lease and requeues every held record. The held set, "the records handed out
under a lease" in the Allium's Vocabulary (lines 76 to 78), exists in the model from
here on, and so do the four guarantees a consumer relies on while it holds a lease.

Marks arrive in QM06, so every held record is unsettled and a release requeues all of
them. Deferral arrives in QM07, and that leaves one gap this ticket must name: once a
record can be held, a publish of a higher rank onto a held key is a reachable step, and
the only rule that answers it, `DeferBehindHeldVersion`, is not in the model.

Two tickets outside this series touch what this ticket models. Each carries its own step
for the model (`.scratch/quint-model/README.md`, "The T tickets that touch the model").

- `tickets/T46-no-claim-without-records.md` adds `requires: limit >= 1` to
  `PullClaimsFrontPocket`. The README places it after this ticket. This ticket is
  written so that T46 changes one definition, `guard_PullClaimsFrontPocket`; design
  point 8 says how.
- `tickets/T47-start-rule-and-config-bounds.md` turns the five config bounds, of which
  `PullBatchPositive` is one, into the requirements of a start rule.

Read these:

- `.scratch/quint-model/README.md`, whole. "Design fixed before QM01" is fixed for this
  ticket, both blocks. "Coverage a later ticket adds" names the paths QM06, QM07 and
  QM08 add to the properties this ticket owns.
- `docs/specs/pachinko.allium` at the base: the Vocabulary entries for Batch, Position,
  the record's states, Held set and Deferred (lines 59 to 83); the `Pocket` entity with
  `queued_records`, `held_records` and `ending_records` (lines 124 to 164); the `Record`
  entity with its comment and all seven invariants (lines 166 to 223); the config (lines
  292 to 298); the Publish comment on the stored rank (lines 324 to 335),
  `DeferBehindHeldVersion` (lines 415 to 436) and `DropStalePublish` (lines 438 to 454);
  the Pull section with both comments and all four rules (lines 486 to 575);
  `ReleaseRequeuesPocket` (lines 668 to 687); the Lease end comment with its table
  (lines 794 to 829) and `RequeueUnsettledRecord` (lines 831 to 844);
  `PullBatchPositive` (lines 976 to 981) and `OutstandingOnlyWhileLeased` (lines 1042 to
  1045); and the `Consuming` surface, with `ExclusivePocket`, `OldestFirst`,
  `OneVisitPerKeyPerLease` and `HeldSetStableUntilLeaseEnds` (lines 1150 to 1213).
- `crates/libpachinko/tests/queue/walk.rs`, for names and domains only: the walk's
  configuration at lines 59 to 67, `WalkStep::Pull` at lines 91 to 94, and the limit's
  domain at line 134.
- `quint/pachinko.qnt`, `quint/verify.txt`, `scripts/verify-quint.sh` and every patch
  under `quint/mutants/`, as QM02 to QM04 left them at the base.
- `.scratch/quint-model/QM02-tracer-claim-and-release.md`,
  `.scratch/quint-model/QM03-verify-recipe-and-ci.md` and
  `.scratch/quint-model/QM04-publish.md`: "Design proposed in this ticket", and in their
  hand-back notes "Design rulings". They settle how a lease is identified, how the
  pocket order is written, how a witness is run, what counts as a mutant caught, a
  patch's header, how the recipe reads the model's properties, the time budget, what a
  step by a party that has not joined does, and how a record is keyed. Where a ruling
  there differs from what this ticket assumes, the ruling wins and the plan says what
  changes.
- In the installed CLI, under
  `tools/quint/node_modules/@informalsystems/quint/dist/src/`: `parsing/ToIrListener.js`
  lines 230 to 242, `cliHelpers.js` lines 91 to 96 and 117 to 125, and `builtin.qnt`
  lines 192, 240 to 257, 496 and 522.
- Decision 0022 under `docs/decisions/`, created by QM02.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

`pull(consumer, limit)` is the Allium's whole pull, in one action: the limit ranges over
-1 to 3, the batch is the Allium's `oldest` of the pocket's queued records, at most
`min(limit, config.max_pull_batch)` of them, each record of it is held by `HoldRecord`
in the same step, and `answer` carries the batch in order. A consumer that holds a
lease is answered by `PullWithinLease`. `release` requeues every held record by
`RequeueUnsettledRecord` in the same step. `max_pull_batch` is a constant, 2 in
`pachinko_walk`, and `PullBatchPositive` bounds it. Three rules have their `guard_`,
`effect_` and `witness_`, four invariants and four guarantees are checked properties
under their Allium names, and `just verify-quint` is green at the walk's domains with
the mutants below each caught by the property it names.

Owning a guarantee here does not mean it is checked in full. Each of the four is stated
in the form it keeps, and checked over every path the model has after this ticket: pull,
publish and release, with every held record unsettled. QM06 adds marks, QM07 deferral
and QM08 the lapse, and each of those tickets shows the property catches a mutant on its
new path. The properties are written so that none of those tickets edits their text.

### Owns

| Kind | Names |
| --- | --- |
| Rules | `PullWithinLease`, `HoldRecord`, `RequeueUnsettledRecord` |
| Invariants | `OutstandingOnlyWhileLeased`, `HeldVersionNotNewer`, `HeldCopiesAgree`, `PullBatchPositive` |
| Guarantees | `ExclusivePocket`, `OldestFirst`, `OneVisitPerKeyPerLease`, `HeldSetStableUntilLeaseEnds` |
| Partial or extended, not counted | `PullClaimsFrontPocket`, QM02's, extended with its `Handed` chain and the positive limits, in its action and in `effect_PullClaimsFrontPocket`. `ReleaseRequeuesPocket`, QM02's, extended with its `Freed` chain, in its action and in `effect_ReleaseRequeuesPocket`. QM04's `PublishAnsweredByOneRule` gains one named exception, which QM07 removes. `ExclusivePocket` was partial in QM02, at the lease's level. |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them after checkpoint 1 and
before step 4, and the hand-back records the ruling. After that the builder does not
reopen them. Point 1 is the README's design point for this ticket; later tickets assume
its recommended answer.

1. **A config bound is an invariant over the constants.** The Allium writes
   `PullBatchPositive` as an invariant, `config.max_pull_batch >= 1` (lines 976 to 981).
   - Recommended: a property named `PullBatchPositive` that reads only the constant,
     named on the `verify` command line like every other property. An instance that
     sets `max_pull_batch` to 0 then fails in its first state. The control is a
     committed mutant that sets it to 0 in `pachinko_walk` and names `PullBatchPositive`.
     The property is a `val` of `pachinko`, beside the constant, and not of the
     instance module: QM03's helper takes its properties from `pachinko` and refuses a
     definition with an upper-case name inside an instance.
   - Alternative: `assume PullBatchPositive = max_pull_batch >= 1`. Read in the
     installed CLI, not run: an `assume` is a declaration of its own kind
     (`parsing/ToIrListener.js`, lines 230 to 242), `declaredProperties` collects only
     definitions (`cliHelpers.js`, lines 91 to 96), and `getInvariants` checks what the
     command line names (lines 117 to 125). So a `verify` does not select an `assume`,
     and the README's claim 13 adds that a false one can leave no start state, which
     passes every property vacuously. Step 2 runs it.
   - QM08 and QM09 own the other four config bounds and write them the same way. T47
     later makes all five the start rule's requirements.
2. **The three black boxes.** The Allium calls `oldest`, `min_by` and `min` black boxes
   (lines 487 to 490).
   - `min` is a conditional on two integers.
   - `min_by(free, p => p.position)` is QM02's, inside the section it landed for the
     pocket order, unchanged. It is
     deterministic because `UniquePocketPosition` holds, so one queued pocket has the
     smallest position.
   - `oldest(records, r => r.position, n)` is one definition that the two pull rules'
     action reads. Recommended form: a record's index is the number of queued records of
     the pocket with a smaller position; the batch is the list, for each index from 0
     up to the batch's length less one, of the one record with that index; the length
     is 0 when `n` is not positive and otherwise the smaller of `n` and the count. It is
     deterministic because `UniqueRecordPosition` holds: no two records share an index,
     so "the one record" is one record. `builtin.qnt` has `getOnlyElement`, whose
     comment says "If the size of `s` is not 1, this operator has undefined behavior"
     (line 240), and `chooseSome`, "deterministically, one element" (line 249); read,
     not run. The plan says which the model uses and why a mutant that breaks
     `UniqueRecordPosition` is still reported as a violation and not as an error.
   - Alternative: a fold over the set that keeps the record with the smaller position.
     A fold's order over a set is not stated, so it is deterministic for the same
     reason and no other.
3. **`OldestFirst` never reads `oldest`.** A property that called the action's own
   definition would be mutated with it and would pass. `OldestFirst`, and the batch
   clause of `effect_PullWithinLease` and `effect_PullClaimsFrontPocket`, read one
   declarative definition over `prev` and `answer` instead: with Q the queued records of
   the batch's pocket in `prev`, the batch is empty when the limit is not positive, and
   otherwise has as many records as the least of the limit, `max_pull_batch` and the
   count of Q; each is a record of Q, at the rank and payload it had in `prev`;
   positions rise strictly along the batch; and every record of Q outside the batch has
   a position greater than every record in it.

   That definition says what a batch holds, not that there is one. The two pull rules'
   effects say that. When `fired` holds `PullWithinLease` or `PullClaimsFrontPocket`,
   `answer` is a batch: it names the consumer `last` names, the lease the rule extended
   or created, and that lease's pocket. The Allium ensures `Batch(...)` in both rules
   whatever the batch holds, an empty one included (line 524, and lines 548 to 553). So
   a pull within a lease that finds nothing queued, or a claim at a limit that is not
   positive, hands back an empty batch and never nothing. The mutant
   `qm05-empty-batch-not-answered` is the control. No property says that a pull is
   answered by a rule at all; design point 8 says why.
4. **A held record.** The Allium gives a record `held_rank`, `held_payload`, `mark`,
   `nacked_at` and `deferred_position` "when status = held" (lines 183 to 187).
   - Recommended: a record's status, which so far is only queued, gains a held value
     that carries `held_rank`, `held_payload`, `mark` and `deferred_position`. A queued
     record has none of the four, and `RequeueUnsettledRecord`'s five `= null` lines are
     the move back to queued. The type of `mark` has all three of the Allium's values,
     although only `unsettled` is written before QM06. `deferred_position` is optional
     and always absent before QM07. Both are read now, by
     `guard_RequeueUnsettledRecord` and `HeldCopiesAgree`, each transcribed whole as the
     README asks. `ending_records` stays QM02's one definition, empty until QM06, which
     first writes a mark it counts. `nacked_at` is a timestamp and waits for QM09
     (shared rule 7).
   - Alternative: flat optional fields on every record. It lets a queued record carry a
     held rank, which the Allium's `when` clause does not allow.
   - An element of the batch in `answer` holds the record's key and the rank and payload
     it was handed out with, which is what the `Consuming` surface exposes of a held
     record (lines 1160 to 1164).
5. **The four guarantees as properties.** Each reads ghosts, not history.

   | Property | Says | Reads |
   | --- | --- | --- |
   | `ExclusivePocket` | If `answer` is a batch with at least one record, for the consumer `last` names: no lease that is active in `prev` on the batch's pocket has another consumer, and the lease the batch names is active in the state, on that pocket, and that consumer's | `answer`, `last`, `prev` |
   | `OldestFirst` | If `answer` is a batch, design point 3's definition holds of it, with the limit `last` names | `answer`, `last`, `prev` |
   | `OneVisitPerKeyPerLease` | If `answer` is a batch, no record in it was held in `prev` | `answer`, `prev` |
   | `HeldSetStableUntilLeaseEnds` | For every lease active in `prev` and in the state, each record held in `prev` in its pocket is held in the state with the same held rank and held payload; its mark is the same unless `ownMark` holds of the step; and `heldVersionShed` does not hold of it. For every lease active in the state, each record of its pocket held in the state and not in `prev` is in the batch of `answer`, which names that lease, and `last` is a pull by its consumer | `last`, `answer`, `prev` |

   - `OneVisitPerKeyPerLease` is a property of one step. The guarantee is about a whole
     lease, and it follows by this argument, which decision 0022 records: a key handed
     out under a lease is held; by `HeldSetStableUntilLeaseEnds` it stays held while the
     lease is active; and a batch never holds a record that was held before the step.
     The alternative is a fifth ghost, the set of lease and key pairs handed out so far,
     which the README's four ghosts do not include.
   - `ownMark(step, lease, key)` says the step is an ack or a nack by the lease's
     consumer, naming that lease and that key. `Step` has no such variant yet, so it is
     false here. QM06 adds the variants to `Step` and completes this one definition.
   - `heldVersionShed` says the step appended, to a shed queue, an entry for that pocket
     and key at the held rank and held payload as a loser: here, an entry of the
     superseded queue with reason `superseded`. A stale publish at the held rank makes
     an entry with reason `stale`, which is a different version. QM06 adds the
     dead-letter queue to this one definition.
   - Later paths, none of which edits a property's text: an acked or nacked key stays
     held, so `OneVisitPerKeyPerLease` covers it (QM06). A deferred version leaves the
     record held, so it is never in a batch under the same lease, and the version queued
     at its arrival position is read by `OldestFirst` like any queued record (QM07). A
     publish onto a held key must leave the held rank and payload alone (QM07). A lapse
     ends the lease, so `HeldSetStableUntilLeaseEnds` asks nothing of that step and
     `ExclusivePocket` reads the lease as ended in `prev` at the next pull (QM08).
6. **The gap: a higher rank onto a held key.** `DropStalePublish` reads `record.rank`
   whatever the record's state (lines 324 to 330), so a stale publish onto a held key is
   modelled now, and an equal rank is rejected as before. A higher rank onto a held key
   with nothing deferred is answered only by `DeferBehindHeldVersion`, QM07's.
   The README's "Every step is total" and shared rule 8 fix what the model does until
   then: the step is taken, no rule answers it, the state is unchanged and `fired` is
   empty, so `UnansweredStepChangesNothing` holds of it. The model's header lists it
   under "Not yet modelled", naming QM07 and saying the model drops that version where
   the Allium defers it. `PublishAnsweredByOneRule` carries it as one named exception, a
   definition of its own with a lower-case name, which QM07 deletes with the header's
   line.
7. **Chained rules.** `HoldRecord` answers `Handed` and `RequeueUnsettledRecord` answers
   `Freed`; neither trigger is in `last`. `guard_HoldRecord` takes a state, a pocket and
   a record key, and `guard_RequeueUnsettledRecord` a state, a lease and a record key,
   as the Allium's triggers do. The README has `FiredMatchesGuards` read the guard of
   a rule later in a chain "in the state the chain had reached". Here that is: a chained
   rule is in `fired` exactly when the outer rule that emits its trigger is there and
   the inner guard holds for at least one argument the outer rule emits, a record of
   the batch that design point 3's definition gives, or a held record of the released
   lease's pocket. Each such record is as it was in `prev` when its turn comes, because
   the chain's earlier links wrote only other records, so the guard is read in `prev`.
   `effect_HoldRecord` says every record of the batch is held in the state at the rank
   and payload it had in `prev`, unsettled, with nothing deferred.
   `effect_RequeueUnsettledRecord` says every record that was held, unsettled and with
   nothing deferred in `prev`, in the released lease's pocket, is queued in the state
   with its key, rank, payload and position unchanged, the first row of the Allium's
   table (line 804). A pull whose batch is empty leaves `HoldRecord` out of `fired`,
   and its pull rule's effect still asks for the empty batch in `answer` (design
   point 3).
8. **T46 changes one guard.** The limit's domain keeps -1 and 0. A claim at such a limit
   still creates a lease and an empty batch. No property says a pull is always
   answered, and none says a leased pocket has a held record. That a pull rule in
   `fired` hands back a batch is a different statement, conditional on the rule, and
   T46 leaves it true. T46 then adds one
   conjunct to `guard_PullClaimsFrontPocket` and re-cuts what that breaks.
9. **The lease's extension waits.** `PullWithinLease` writes only
   `lease.expires_at`, the batch and its `Handed`. With no clock (shared rule 7), a pull
   within a lease whose batch is empty changes `fired`, `last` and `answer` and nothing
   else. No mutant can catch a broken extension before QM09, and the model's header says
   so on its line for the clock.
10. **The bound follows the deepest trace.** The plan writes the shortest trace for
    every witness and every mutant in the model and counts its steps after `init`.
    QM03's ticket proposes a bound of 10 for `pachinko_walk` and a floor of the deepest
    witness's depth plus two. If the bound in `quint/verify.txt` is below the longest
    trace here plus two, this ticket raises it. It never lowers it.

The mutants. A mutant names one property, which may be any property the recipe checks
(shared rule 6): an Allium name, an `effect_<Rule>`, a part of a guarantee, or a label
check. Seven of the eight here name an Allium invariant or guarantee, and one names an
effect. Each names it in the
header QM03 landed (its ticket proposes three lines, `# mutant:`, `# instance:` and
`# property:`), and is cut as QM03's ticket says a patch is cut. The unchanged model's
pass on `pachinko_walk` is the evidence that each named property holds at the same
bound; no mutant has a run of its own for that.

| Patch | Mutation | Property that must catch it | Shortest trace, steps after `init` |
| --- | --- | --- | --- |
| `quint/mutants/qm05-batch-exceeds-cap.patch` | A pull hands out `limit` records, ignoring `max_pull_batch` | `OldestFirst` | two joins, three publishes into one pocket, a pull at 3: 6 |
| `quint/mutants/qm05-held-key-handed-again.patch` | `PullWithinLease` takes its batch from all the pocket's records, not the queued ones | `OneVisitPerKeyPerLease` | two joins, a publish, two pulls at 1: 5 |
| `quint/mutants/qm05-batch-skips-oldest.patch` | `oldest` starts at the second smallest position | `OldestFirst` | two joins, two publishes into one pocket, a pull at 1: 5 |
| `quint/mutants/qm05-second-consumer-handed-leased-pocket.patch` | `PullClaimsFrontPocket` counts a leased pocket as free | `ExclusivePocket` | three joins, a publish, a pull at 0, another consumer's pull at 1: 6 |
| `quint/mutants/qm05-release-leaves-record-held.patch` | A release ends the lease and sends no `Freed` | `OutstandingOnlyWhileLeased` | two joins, a publish, a pull at 1, a release: 5 |
| `quint/mutants/qm05-stale-publish-requeues-held-record.patch` | `DropStalePublish` onto a held record makes it queued | `HeldSetStableUntilLeaseEnds` | two joins, a publish, a pull at 1, a publish at a lower rank: 5 |
| `quint/mutants/qm05-zero-pull-batch.patch` | `pachinko_walk` sets `max_pull_batch` to 0 | `PullBatchPositive` | none: the first state |
| `quint/mutants/qm05-empty-batch-not-answered.patch` | `PullWithinLease` hands back nothing when its batch is empty | `effect_PullWithinLease` | two joins, a publish, two pulls at 1, the second with nothing left queued: 5 |

The last cuts in `PullWithinLease` and not in the claim, because T46 removes the claim
at a limit that is not positive and would leave a mutant there with no trace.

`qm05-zero-pull-batch` stands only while the bound is a property over the constants. If
`tickets/T47-start-rule-and-config-bounds.md` has landed and made the bounds the start
rule's requirements, the patch leaves the model no start state: nothing is violated, a
check over no states passes, and `PullBatchPositive` cannot catch it. T47's own step
stops for the maintainer on exactly this, and its hand-back names the control that was
ruled: the bound kept as a property beside the start rule, or a control that shows the
start rule refuses the value. This ticket then uses that control in the patch's place.
The lines here that name the patch change with it: the row of Files touched, the count
of eight, the acceptance bullet on the zero-batch control, and in Verification the `ls`,
the name loop and the first of the two hand-run pairs. Each is brought to the ruled
control before step 4 and recorded under Departures; none is dropped without its
replacement. QM08's and QM09's config-bound mutants say the same.

`HeldVersionNotNewer` and `HeldCopiesAgree` have no mutant here. Before deferral
`HoldRecord` writes the only held states, with the held copy equal to the record's, and
`effect_HoldRecord` checks that write. The hand-back reports both as checked and not
mutation-tested; the README has QM07 add the path that makes them more than that.

## Claims this ticket checks

Rows 12 and 13 are the README's. The others are claims this ticket rests on that nobody
has run.

| # | Claim | Check |
| --- | --- | --- |
| 13 | An `assume` over a `const` is not among the properties a `verify` selects, and one that is false can leave no start state, which passes vacuously | step 2, on a scratch module under `ai_tmp/qm05/`: two instances of one module, one that makes an `assume` false and a twin that makes it true, each verified with the same property that is false on purpose. The twin must report it. Read in `parsing/ToIrListener.js` and `cliHelpers.js`, not run |
| 12 | The whole of `verify-quint` finishes inside the CI job's timeout after this ticket's three witnesses, eight mutants, wider limit and deeper bound are added | steps 1 and 6: the wall time before and after, against `timeout-minutes` of the `quint` job in `.github/workflows/ci.yml`. Not read, not run |
| D | `quint verify` accepts the definition that picks the one element of a set, `getOnlyElement` or `chooseSome`, and reports a violation, not an error, when a mutant makes that set hold two | step 2, on a scratch module, then step 6 with QM04's `qm04-new-key-reuses-position` mutant re-run. Read in `builtin.qnt`, not run |
| E | `quint verify` accepts a pure definition that builds a list with `foldl` over `range` | step 2, on a scratch module. Read in `builtin.qnt`, not run |

An answer to claim 13 that differs from the README's is a design change: stop and bring
it to the maintainer. A failure of D or E changes design point 2: stop and bring the
alternative.

## Non-goals

- No ack, no nack, no renew and no mark other than unsettled; QM06.
- No `DeferBehindHeldVersion`, no second case of `Supersede`, and no other row of the
  lease's end than the first; QM07.
- No lapse; QM08. No `expires_at`, no `nacked_at` and no clock; QM09.
- No property that says a pull is always answered, and none that says a leased pocket
  has a held record. T46 depends on their absence.
- No guard on a claim's limit. That is T46's change to the Allium, not this ticket's.
- No fifth ghost.
- No new instance module. The zero-batch control is a mutant, because `verify-quint`
  verifies every instance at the foot of the model and that one must fail.
- No change to `scripts/verify-quint.sh`, to the CI job or to any page of the handbook
  other than decision 0022.
- No smaller domain.

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

`<id>` is `QM05`. Checkpoints:

- Checkpoint 1, the plan, `ai_tmp/qm05/plan.md`: step 3, with the `conformance` and
  `breakage` lenses, before any tracked file changes.
- Checkpoint 2, the diff, with a design lens: step 8, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | Created by QM02. `max_pull_batch` as a constant, 2 in `pachinko_walk`; the limit over -1 to 3; the held value of a record's status; `oldest` and the declarative batch definition; the pull and release actions extended; `PullWithinLease`, `HoldRecord` and `RequeueUnsettledRecord` in `Rule`, with `guard_`, `effect_` and `witness_`; `effect_PullClaimsFrontPocket` extended; `FiredMatchesGuards` over the chained rules; four invariants; four guarantees with `ownMark` and `heldVersionShed`; the exception in `PublishAnsweredByOneRule`; the header's "Not yet modelled" |
| `quint/mutants/qm05-batch-exceeds-cap.patch` | new | names `OldestFirst` |
| `quint/mutants/qm05-held-key-handed-again.patch` | new | names `OneVisitPerKeyPerLease` |
| `quint/mutants/qm05-batch-skips-oldest.patch` | new | names `OldestFirst` |
| `quint/mutants/qm05-second-consumer-handed-leased-pocket.patch` | new | names `ExclusivePocket` |
| `quint/mutants/qm05-release-leaves-record-held.patch` | new | names `OutstandingOnlyWhileLeased` |
| `quint/mutants/qm05-stale-publish-requeues-held-record.patch` | new | names `HeldSetStableUntilLeaseEnds` |
| `quint/mutants/qm05-zero-pull-batch.patch` | new | names `PullBatchPositive` |
| `quint/mutants/qm05-empty-batch-not-answered.patch` | new | names `effect_PullWithinLease` |
| `quint/mutants/*.patch` | shared, own lines | Created by QM03 and QM04. Each patch the model's change breaks, re-cut with its mutation and its named property kept (shared rule 11) |
| `quint/verify.txt` | shared, own lines | Created by QM03. `pachinko_walk`'s `--max-steps`, only if design point 10 raises it; whatever else the recipe needs to list a new witness or mutant, in QM03's format |
| `quint/traces/`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines | Created by QM10, and the last by QM12, so absent when this ticket runs in series order. If the tree holds them: the fixtures regenerated by `just quint-traces`, never by hand, before any gate runs, since `just check-quint` and `just verify-quint` both read them; the generator's witness list brought up to the rules this ticket adds; and each replay kept green, each new field mapped or added to that replay's list of what it does not compare, and a new step listed there only with the maintainer's answer on which ticket maps it (shared rule 11). A change under `crates/` follows `AGENTS.md`'s block for Rust |
| `docs/decisions/0022-*.md` | shared, own lines | Decision 0022, created by QM02; the file name is the one QM02 landed. The ruling that a config bound is an invariant over the constants; the argument that carries `OneVisitPerKeyPerLease` from one step to a lease; the gap of design point 6 |
| `.scratch/quint-model/QM05-pull-and-held-set.md` | ticket | `status:` and the hand-back notes; and, only if T47's ruling replaced the config-bound control, the lines of Files touched, acceptance and Verification that name `qm05-zero-pull-batch`, each recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM05's Status cell; the sentences a ruling on design point 1 changes, if it differs from the recommended answer |
| `.scratch/quint-model/QM*.md` | shared, own lines | Only if the ruling on design point 1 differs from the recommended answer: the sentences it changes in the later tickets the README's index names |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm05-pull-and-held-set`
   from the series base the maintainer names, once QM04 has merged there, and record the
   base's full commit id as `base`. `git ls-files .scratch/quint-model/` must list the
   README and this ticket. Run `just initialize` (network, authorised). Run `just check`
   on the untouched worktree and quote its last line, headed by the commit. Then show
   that nothing listens on `localhost:8822`, with the port probe and its positive
   control from Verification below, and run `just verify-quint` on the untouched
   worktree. Quote its closing lines, its exit status and its wall time. That time is
   the baseline for claim 12.
2. **Read before writing, the probes, and the plan.** Read everything under "Read
   these". Compare each Allium line this ticket quotes with the Allium at `base`. If a
   rule, an invariant or a guarantee this ticket owns or extends reads differently,
   **stop** and ask: T46 or T47 may have merged. Then run the probes, each through
   `just quint` on a scratch module under `ai_tmp/qm05/`, with port 8822 shown free
   before each `verify`, quoting each output and exit status:
   - claim 13: a module with a constant, an `assume` over it, and two instances, one
     that makes the `assume` false and a twin that makes it true and differs in nothing
     else. `typecheck` it. On the twin first, `verify` naming a property that is true,
     and again naming one that is false on purpose: the false one must be reported, or
     the probe shows nothing and the step stops. That run is the control: it shows the
     property is selected and can fail. Then the same two runs on the instance whose
     `assume` is false. Record whether the false property is reported there, and what
     the CLI says when the `assume` itself is named as an invariant;
   - claims D and E: a module with a set of records with distinct positions and the
     recommended `oldest`. `typecheck` it. `verify` a
     property that says the batch is in position order, and one that is false on
     purpose, which must be reported as a violation. Then give two records one position
     and record what `verify` reports.

   Write `ai_tmp/qm05/plan.md` with:
   - for each of the three owned rules and the two extended ones, the Allium's text
     beside the planned action, its `guard_` and its `effect_`;
   - each of the four invariants and four guarantees, with the ghost it reads, and for
     each guarantee the paths QM06, QM07 and QM08 add and why its text does not change;
   - `oldest` as it will be written beside the declarative definition, and the reason
     neither reads the other;
   - each mutant, the one property that must catch it, and its shortest trace;
   - the shortest trace from the empty start for every witness and every mutant in the
     model, its length, and the `--max-steps` that follows against `quint/verify.txt`;
   - each "Not yet modelled" line added or removed: the line for a pull with a positive
     limit goes; the line for `DeferBehindHeldVersion` now says the step is reachable
     and changes nothing; lines for marks (QM06) and the clock (QM09) are there
     afterwards;
   - each patch at the base that the change will break.
3. **Adversarial review** (checkpoint 1: `conformance` and `breakage`, over
   `ai_tmp/qm05/plan.md` with the probes' output, before any tracked file changes;
   `tickets/CONVENTIONS.md` §14). Questions:
   - Does `OldestFirst` read anything the pull action also reads to choose its batch?
     Would the mutant that skips the oldest record, or the one that ignores the cap,
     also change the property?
   - Does each of the four guarantees, as written, already constrain a step QM06, QM07
     or QM08 will add, or will one of those tickets have to edit it? Are `ownMark` and
     `heldVersionShed` the only definitions that grow?
   - Is the step from `OneVisitPerKeyPerLease` over one step to the guarantee over a
     lease sound? Name a trace, in the full Allium, that satisfies both step properties
     and hands a key out twice under one lease.
   - Could `HeldSetStableUntilLeaseEnds` or `ExclusivePocket` hold because nothing
     satisfies its premise: no lease active in both states, or no batch with a record?
   - Does the planned `FiredMatchesGuards` line for a chained rule hold `HoldRecord` in
     `fired` on a pull with an empty batch, or leave it out on one with a record?
   - Is the gap of design point 6 the only publish the model answers differently from
     the Allium once a record is held?
4. **Red: the properties before the behaviour.** **Stop** first for the maintainer's
   ruling on "Design proposed", with the checkpoint's findings and the probes in hand,
   and record it. Then, in `quint/pachinko.qnt`: add `max_pull_batch`, widen the limit
   to -1 to 3, add the held value of a record's status with its four fields, and add
   the three rules' guards, effects and witnesses, the extended effects, the four
   invariants and the four guarantees. Leave the pull and release actions as they are,
   so that a claim still hands out nothing and a pull within a lease is still
   unanswered. QM03's ticket proposes a recipe that reads the rules, witnesses and
   properties from the model itself, so that nothing is listed by hand; if its ruling
   differs, list them where the recipe reads them. A helper such as `ownMark` starts
   with a lower-case letter, as the README's naming rule has every definition that is
   not a property.
   `just check-quint` passes. QM03's ticket also proposes that the recipe applies every
   patch before it verifies anything. If that landed, a patch these additions break
   stops the run at "does not apply": re-cut
   each such patch first (shared rule 11) and run again. Then, with port 8822 shown
   free, `just verify-quint` fails. Quote what it reports, with its exit status. It must
   fail on `OldestFirst`, on `FiredMatchesGuards` or on a witness that is not reached,
   and the hand-back says which it met first.
5. **Green: the pull and the release.** Write `oldest`, the `Handed` chain in both pull
   rules, `PullWithinLease`, and the `Freed` chain in the release. Add the exception to
   `PublishAnsweredByOneRule`. Re-cut each patch under `quint/mutants/` that no longer
   applies, keeping its mutation and its named property, and list each in the hand-back
   with those of step 4. Raise `--max-steps` in `quint/verify.txt` if the plan says so.
   With port 8822 shown free, `just verify-quint` passes: quote `Canary` violated on
   `pachinko_walk`, every property named, and the three owned witnesses and every earlier
   one reached.
6. **The mutants.** Add the eight patches. With port 8822 shown free, `just verify-quint`
   passes and reports each of the eight caught by the property it names, and every
   earlier mutant still caught by its own, QM04's `qm04-new-key-reuses-position` among
   them (claim D). Quote those lines. Quote the run's wall time beside step 1's and
   beside the CI job's `timeout-minutes` (claim 12). The head-room QM03's ruling set is
   the whole job's (its ticket proposes that a job over 15 of its 20 minutes stops and
   asks), and this run is the recipe alone, on another machine. So add the durations of
   the setup and the installs that QM03's hand-back quotes from CI, and say in the
   hand-back that the sum is an estimate until the job runs. If the sum does not leave
   that head-room, **stop** and ask; do not shrink a domain, lower the bound or drop a
   mutant.
7. **Words.** Bring the model's header to the "Not yet modelled" list of the plan. Add
   this ticket's lines to decision 0022. If the ruling on design point 1 differs from
   the recommended answer, edit the sentences it changes in
   `.scratch/quint-model/README.md` and in the later tickets its index names, and list
   them in the hand-back. Run
   `just check-docs`.
8. **Adversarial review** (checkpoint 2: `conformance`, `breakage` and `design`, over
   `git diff "$base"...HEAD` plus the uncommitted changes, with the output of steps 2
   and 4 to 6, before the final commit; `tickets/CONVENTIONS.md` §14). Questions:
   - Does the model say what the Allium says of a pull at each limit from -1 to 3, with
     and without a lease, and of a release with records held? Is anything the Allium
     allows missing and not listed under "Not yet modelled"?
   - Could any of the four guarantees hold for a reason other than the one claimed: a
     premise nothing satisfies, a definition shared with the action, or a ghost that a
     guard or an effect on the state reads?
   - Does any property say that a pull is answered, or that a leased pocket has a held
     record? Would T46 need to change more than `guard_PullClaimsFrontPocket`?
   - Does each pull rule's effect fail when the rule is in `fired` and `answer` is not
     a batch, for an empty batch as for a full one?
   - Is a publish of a higher rank onto a held key the only step the exception in
     `PublishAnsweredByOneRule` excuses, and does the header say what the model does
     with it?
   - Could Verification pass with `max_pull_batch` at 0, with a mutant surviving, or
     with a server on port 8822 that the run did not start?
   - Is every domain of `pachinko_walk` the walk's, is `max_pull_batch` 2, and is the
     bound at least the deepest trace?
9. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in `.scratch/quint-model/README.md`,
   write the hand-back notes, and commit the record. Then run `just check` once more,
   on the record's commit, and show its last line and an empty `git status --short`. If
   it fails, fix the notes, commit again and run it again; never amend the work's
   commit. Before asking to push, fetch
   (network, authorised), merge or rebase onto the series base, rerun `just check` and
   Verification, and requote both (`tickets/CONVENTIONS.md` §11, "Green"; shared rule
   13). If the base had moved, the quotes already committed are stale
   (§11, "Quoted evidence names its commit"): put the fresh ones in the hand-back,
   headed by the merged commit, commit that record, and run `just check` on it once
   more, with an empty `git status --short`, as after the first record.
   If the merge or rebase changed a file under Files touched, checkpoint 2 did not
   see that change: show its diff at the stop and ask whether it is reviewed before the
   push. **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just check-quint` is green, quoted.
- Step 2's probes are quoted, the twin instance's run among them, and claim 13's
  outcome is recorded beside the README's.
- `effect_PullWithinLease` and `effect_PullClaimsFrontPocket` each ask for a batch in
  `answer` whenever their rule is in `fired`, shown by quoting the clause.
- Step 4's red run is quoted: `just verify-quint` failed before the actions changed.
- `just verify-quint` is green, quoted: `Canary` violated on `pachinko_walk`; every
  property named; `witness_PullWithinLease`, `witness_HoldRecord` and
  `witness_RequeueUnsettledRecord` reached, and every earlier witness still reached;
  each of the eight mutants caught by the property it names; every earlier mutant still
  caught.
- The zero-batch control is shown twice: by the recipe, and by hand in Verification,
  where `PullBatchPositive` is violated on the patched copy and holds on the model.
- The run's wall time is quoted against step 1's baseline and against the CI job's
  `timeout-minutes`, with the setup and install durations from QM03's hand-back added.
- The model's "Not yet modelled" list has no line for a pull with a positive limit, and
  has a line for a higher rank onto a held key that names QM07 and says the step changes
  nothing. Quoted.
- No domain is shrunk: `pachinko_walk` has publishers `p0` and `p1`, consumers `c0` to
  `c2`, pockets `a`, `b` and `c`, record keys `k0` to `k2`, ranks 0 to 3, pull limits -1
  to 3 and `max_pull_batch` 2, and `--max-steps` is not below the base's. Quoted.
- No property says a pull is always answered or that a leased pocket has a held record,
  shown by the design reviewer's answer at checkpoint 2.
- Decision 0022 has this ticket's three lines.
- The hand-back lists each patch re-cut, reports `HeldVersionNotNewer` and
  `HeldCopiesAgree` as checked and not mutation-tested, and says for each of the four
  guarantees which later ticket adds a path.
- Each of claims 12, 13, D and E has its outcome recorded, and each design point its
  ruling.
- Both checkpoints are recorded as §14's Record asks, with the series' deltas.
- After the record's commit, `just check` was run once more and its last line and an
  empty `git status --short` are quoted.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script, from the repository's root, with `steps` set to the
`--max-steps` that `quint/verify.txt` gives `pachinko_walk`. `set -eu` stops it at the
first command that fails. No other worktree may verify while it runs.

```sh
set -eu
git rev-parse HEAD
: "${steps:?set steps to the --max-steps quint/verify.txt gives pachinko_walk}"
mkdir -p ai_tmp/qm05

# Every owned rule and property is in the model. grep -c exits 1 on a count of 0.
for rule in PullWithinLease HoldRecord RequeueUnsettledRecord; do
  for prefix in guard_ effect_ witness_; do
    grep -c "${prefix}${rule}" quint/pachinko.qnt
  done
done
for name in OutstandingOnlyWhileLeased HeldVersionNotNewer HeldCopiesAgree PullBatchPositive \
  ExclusivePocket OldestFirst OneVisitPerKeyPerLease HeldSetStableUntilLeaseEnds; do
  grep -c "$name" quint/pachinko.qnt
done
ls quint/mutants/qm05-batch-exceeds-cap.patch \
  quint/mutants/qm05-held-key-handed-again.patch \
  quint/mutants/qm05-batch-skips-oldest.patch \
  quint/mutants/qm05-second-consumer-handed-leased-pocket.patch \
  quint/mutants/qm05-release-leaves-record-held.patch \
  quint/mutants/qm05-stale-publish-requeues-held-record.patch \
  quint/mutants/qm05-zero-pull-batch.patch \
  quint/mutants/qm05-empty-batch-not-answered.patch

just check-quint

# The port probe, as AP01 wrote it. lsof exits 1 when nothing matches.
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

# The whole recipe, timed, and what it named.
start=$(date +%s)
just verify-quint >| ai_tmp/qm05/verify.log 2>&1
end=$(date +%s)
echo "verify-quint: $((end - start)) seconds"
grep -n 'timeout-minutes' .github/workflows/ci.yml
for name in witness_PullWithinLease witness_HoldRecord witness_RequeueUnsettledRecord \
  witness_PullClaimsFrontPocket witness_ReleaseRequeuesPocket \
  qm05-batch-exceeds-cap qm05-held-key-handed-again qm05-batch-skips-oldest \
  qm05-second-consumer-handed-leased-pocket qm05-release-leaves-record-held \
  qm05-stale-publish-requeues-held-record qm05-zero-pull-batch \
  qm05-empty-batch-not-answered \
  qm04-new-key-reuses-position Canary ExclusivePocket OldestFirst \
  OneVisitPerKeyPerLease HeldSetStableUntilLeaseEnds; do
  grep -c -F "$name" ai_tmp/qm05/verify.log
done

# Two mutants by hand: red on the patched copy, green on the model, same command.
for pair in "qm05-zero-pull-batch PullBatchPositive" "qm05-batch-skips-oldest OldestFirst"; do
  slug=${pair% *}
  property=${pair#* }
  copy="ai_tmp/qm05/$slug.qnt"
  patch -o "$copy" -i "quint/mutants/$slug.patch" quint/pachinko.qnt
  just quint typecheck "$copy"
  if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
  if FORCE_COLOR=0 just quint verify --main=pachinko_walk --invariant="$property" --max-steps="$steps" "$copy" >| "ai_tmp/qm05/$slug.red.log" 2>&1; then exit 1; else test "$?" -eq 1; fi
  grep -F '[violation] Found an issue' "ai_tmp/qm05/$slug.red.log"
  grep -F 'found a counterexample' "ai_tmp/qm05/$slug.red.log"
  if grep -F 'reached a deadlock' "ai_tmp/qm05/$slug.red.log"; then exit 1; else test "$?" -eq 1; fi
  mark=$(printf '\342\235\214')
  grep -Fx "  ${mark} ${property}" "ai_tmp/qm05/$slug.red.log"
  test "$(grep -c -F "$mark" "ai_tmp/qm05/$slug.red.log")" -eq 1
  if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
  just quint verify --main=pachinko_walk --invariant="$property" --max-steps="$steps" quint/pachinko.qnt >| "ai_tmp/qm05/$slug.green.log" 2>&1
  grep -F '[ok] No violation found' "ai_tmp/qm05/$slug.green.log"
done
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

just check
status=$(git status --short)
test -z "$status"
```

The red tests are the two `verify` runs on patched copies. Each must exit non-zero and
print the violation line, after `typecheck` has shown the copy is well typed, so the
failure is not a type error. The violation line alone is not enough: the CLI prints it
for a deadlock too (`cliReporting.js`, line 246, with the two explanations at
`apalache.js`, lines 144 and 146). So each log must also hold `found a counterexample`
and must not hold `reached a deadlock`. That a run names one invariant is not taken as
proof of which one failed: each log must hold the reporter's own line for it, two
spaces, the cross mark U+274C, a space and the property's name, and no other cross-mark
line, and each run must exit 1. That is the evidence QM03's ticket proposes for a mutant
(its point 2); if the reporter cannot evaluate the trace it prints a warning and no such
line, and the check fails. The search for the deadlock line passes by
printing nothing, and the search before it, over the same log, is its positive control.
The first is the control design point 1 asks for: an
instance with `max_pull_batch` at 0 fails. The green tests are the same commands on the
unchanged model. Each pair stands in for the canary on its property: the same `--main`,
the same bound, one result each way. `just verify-quint` is the green test for
everything else, and step 4 holds its red run. The recipe itself runs no
unchanged-model run for a mutant: its pass on the instance is that evidence (shared
rule 6). The two result lines are the ones AP01's
Verification greps for with the pinned CLI. Each `grep -c` prints a count and fails on
zero, so none passes by printing nothing. The listener's `lsof` line is the positive
control for the silent `lsof` probes, one before each `verify` and one after the last:
the same command, matching something.
`git status --short` is silent on a clean tree; `git rev-parse HEAD` and the commit of
step 9 show there was a tree to read. Three things here were written without being run.
The loop over names assumes the recipe's log prints each witness's and each mutant's
name, as QM03's ticket proposes. `patch -o` is not the recipe's way to apply a patch:
QM03's ticket proposes `git apply`, which takes no fuzz. The two explanation lines and
the cross-mark line were read in the installed source (`apalache.js`, lines 144 and 146;
`cliReporting.js`, line 171); QM03's step 2 runs them. If any does not hold at the base,
correct the line to what QM03 landed and record it under Departures.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Patches re-cut

### Invariants checked and not mutation-tested

### Coverage later tickets add

### Departures

### Verification

## Open points

- **`OneVisitPerKeyPerLease` over a lease.** The model checks one step and decision 0022
  carries the argument for the rest (design point 5). As a step property it also follows
  from `OldestFirst`, which already says every record of a batch was queued in `prev`,
  so it cannot fail while `OldestFirst` holds; its mutant is caught because the run
  names it alone. A fifth ghost would check the guarantee directly, would make the
  property independent, and would change the README's second design block. The
  maintainer rules at step 4.
- **Two definitions grow.** `ownMark` and `heldVersionShed` are completed by QM06, when
  `Step` gains an ack and a nack and the state gains the dead-letter queue. The text of
  `HeldSetStableUntilLeaseEnds` does not change, but what it means does. If the
  maintainer reads that as a later ticket changing an earlier property, QM06 says so in
  its own hand-back.
- **T46 before this ticket.** The README has T46 land afterwards. If the Allium at the
  base already holds `limit >= 1` on `PullClaimsFrontPocket`, step 2 stops and asks.
- **How deep a pass reaches.** A green run says no property fails within `--max-steps`
  steps of the empty start, and a pull with a record needs four of them. The bound this
  ticket sets is the deepest witness or mutant, no more. Whether to spend more of the CI
  budget on depth is the maintainer's, with step 6's wall time in hand.
