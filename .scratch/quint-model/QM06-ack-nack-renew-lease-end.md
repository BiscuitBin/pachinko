---
id: QM06
title: "Ack, nack, renew and the lease's end"
status: todo
depends_on: [QM05]
parallel_with: []
branch: ticket/qm06-ack-nack-renew-lease-end
estimated_size: L
---

# QM06: Ack, nack, renew and the lease's end

## Context

After QM05 the model in `quint/pachinko.qnt` (created by QM02) has the two joins, the
publish rules for a key nobody holds, the full pull with its `HoldRecord` steps, and a
release that requeues every held record through `RequeueUnsettledRecord`. A consumer can
take records and give them back. It cannot say anything about them: every held record is
unsettled, no record ever leaves a pocket, no pocket is ever idle, and the dead-letter
queue does not exist.

The Allium, `docs/specs/pachinko.allium`, says more. An ack or a nack marks a held record
and removes nothing (the comment at lines 578 to 599). The lease's end reads the mark:
an acked version is consumed, a nacked version becomes a `DeadLetter`, an unsettled one
is queued again (the table at lines 803 to 812). A pocket whose records are all removed
by the lease's end goes idle instead of back into the pocket order (lines 689 to 707),
and the next publish that adds a record revives it (lines 462 to 472). Ack, nack, renew
and release name the lease they act under, and one that names a lease that has ended is
rejected "rather than landing on a successor lease over the same pocket" (the guarantee
`ActionsNameTheirLease`, lines 1179 to 1183).

This ticket adds that slice. It comes before deferral (QM07) because QM07 needs marks to
tell the six rows of the lease's end apart, and before the lapse (QM08) because an idle
pocket needs an ack and a release to be reached.

Three things about the model so far shape the work, and "Design proposed in this ticket"
answers each:

- **The model has no clock** (shared rule 7). `Renew`, `AckRepeats` and `NackRepeats`
  write nothing in the Allium but `lease.expires_at`. In the model they change nothing
  but `fired`. No property and no mutant can tell a lease extension from a broken one
  until QM09.
- **One action holds a whole chain** (shared rule 4). A release and the `Freed` rules
  that answer it are one action, and so are a publish and the revival it causes. Two
  rules of one chain can write the same field: `PublishNewKey` and `ReviveIdlePocket`
  both advance `queue.next_position`. The README has their effects read such a field
  through one shared definition, added by the ticket that first meets the case. This is
  that ticket.
- **Three guarantees this ticket touches belong to later tickets.**
  `MarksStandHoweverLeaseEnds` is QM08's, `EveryDropIsVisible` and
  `HeldRecordsOldestFirst` are QM07's. The README lists a part of each as "Partial in an
  earlier ticket" here, and makes such a part a property of its own that the owner
  keeps.

Tickets outside the series that touch this slice (the README's "The T tickets that touch
the model"): `tickets/T43-caller-contract.md` changes every consumer action and `answer`
(the lease's id and the named refusals); `tickets/T40-pocket-order-model.md` changes the
pocket's position in release and revive; `tickets/T46-no-claim-without-records.md`
changes `guard_PullClaimsFrontPocket` and lands after QM05. None blocks this ticket. This
ticket therefore asserts nothing of the form "a leased pocket has a held record", and
keeps every action's use of the pocket's position inside the section QM02 landed for
the pocket order (README, "Design points a ticket settles"). The position invariants
and the effect of each rule that writes the order state the position in their own
words and never call that section's writer.

Read these:

- `.scratch/quint-model/README.md`, in full. It wins over this ticket.
- `docs/specs/pachinko.allium`: the `Pocket` entity with `ending_records` and
  `IdleHasNoRecords` (lines 122 to 164); the `Record` entity and its comment (lines 166
  to 223); the `DeadLetter` and `Lease` entities (lines 243 to 278); the comment above
  `PublishNewKey` and the two revive rules (lines 359 to 366 and 456 to 484); "Ack and
  nack" (lines 577 to 652); "Renew and release" (lines 654 to 707); "Lease end" with its
  table and its comment (lines 794 to 951); the guarantees `ActionsNameTheirLease`,
  `HeldRecordsOldestFirst`, `OneVisitPerKeyPerLease`, `HeldSetStableUntilLeaseEnds` and
  `MarksStandHoweverLeaseEnds` (lines 1179 to 1221), and `EveryDropIsVisible` (lines 1126
  to 1134).
- `docs/decisions/0020-held-set-until-lease-ends.md`, "Decision", for why a mark removes
  nothing.
- `crates/libpachinko/tests/queue/walk.rs`: `LeaseChoice` (lines 69 to 78) and `WalkStep`
  (lines 80 to 117), for names and for the leases an action may name. Not for behaviour
  (shared rule 1).
- `quint/pachinko.qnt` at this ticket's base, in full, and the hand-back notes of QM05's
  ticket, `QM05-pull-and-held-set.md` in this directory: the record's type, how `oldest`
  is written, how the release folds over the held records, and the text of
  `OneVisitPerKeyPerLease` and `HeldSetStableUntilLeaseEnds` as landed.
- `scripts/verify-quint.sh` and `quint/verify.txt` (both created by QM03), and one patch
  under `quint/mutants/`: how a witness and a mutant are declared, and the bound.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

The model gains marks, the dead-letter queue, the idle pocket and its revival. `ack`,
`nack` and `renew` are new top-level actions, named after `WalkStep`, with the arguments
`consumer`, `lease` and `key`. The release stays one action: it now answers each held
record by its mark (`RequeueUnsettledRecord`, `ConsumeAckedRecord` or
`DeadLetterNackedRecord`), in position order, and chooses between `ReleaseRequeuesPocket`
and `ReleaseIdlesEmptyPocket` by the Allium's `ending_records`. A publish into an idle
pocket fires `ReviveIdlePocket` in the same action as the publish rule. Nine rules get
their guard, effect and witness, `IdleHasNoRecords` and `ActionsNameTheirLease` become
checked properties, and eight mutants are caught, each by the one property it names.

Owning `ActionsNameTheirLease` is not "checked in full": QM08 adds a lease ended by a
lapse (the README's "Coverage a later ticket adds"). What stays unmodelled after this
ticket is written down in the model's header: the lease extension, the reset of
`consecutive_expiries`, `nacked_at`, and deferral. QM02's ticket adds no property under
the name `ActionsNameTheirLease`; this ticket is the first to use it.

### Owns

| Kind | Names |
| --- | --- |
| Rules | `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats`, `Renew`, `ReleaseIdlesEmptyPocket`, `ConsumeAckedRecord`, `DeadLetterNackedRecord`, `ReviveIdlePocket` |
| Invariants | `IdleHasNoRecords` |
| Guarantees | `ActionsNameTheirLease` |
| Partial or extended, not counted | The release half of `MarksStandHoweverLeaseEnds` (QM08 owns it). `EveryDropIsVisible` and `HeldRecordsOldestFirst` for acked and nacked records (QM07 owns them). `ReleaseRequeuesPocket` (QM02's) now reads `ending_records` with real marks. `OneVisitPerKeyPerLease` and `HeldSetStableUntilLeaseEnds` (QM05's) gain a path each |

## Design proposed in this ticket

These are proposals. The maintainer confirms them, or changes them, after checkpoint 1
and before step 4, and the hand-back records the ruling. After that the builder does not
reopen them. Names the README fixes are fixed; every other name below (`freed`,
`held_oldest_first`, `allotted`, the fragment names) is a proposal the plan may change,
and QM07 and QM08 read the landed names from the model.

- **What the state gains.** A held record's `mark` takes the values `acked` and
  `nacked`. QM05's ticket gives the field all three values and writes only `unsettled`;
  if the field landed with one value, this ticket adds the other two. The dead-letter
  queue: a list in creation order, as the superseded queue is, whose entries carry
  `pocket`, `key`, `rank`, `payload` and `nacked_by`. `rank` and `payload` are the held
  version's, `held_rank` and `held_payload` (Allium lines 908 to 919). `nacked_at` and
  `expires_at` wait for QM09. The pocket status `idle` becomes reachable. The `Step` type
  gains `Ack`, `Nack` and `Renew`. Nothing is added to `pachinko_walk`: this ticket reads
  no new constant.
- **Which leases an action may name.** The `lease` argument of `ack`, `nack` and `renew`
  ranges over what QM02's ruling gave `release`: every lease the model has issued, active
  or ended, whoever's it is, and one that was never issued. QM02's ticket proposes the
  indices of the list of leases and the one index past its end. Those are `LeaseChoice`'s
  `Own`, `Issued` and `Never`. `ActionsNameTheirLease` means nothing over a smaller
  domain. If `release`'s domain as landed leaves one of the three out, **stop** and ask.
- **How the lease's end walks the held records, in position order, in one action.**
  Recommended:
  1. `held_oldest_first(s, pocket)` is the pocket's held records as a list, smallest
     position first, built with QM05's `oldest` over the held records and a count equal
     to their number. No second ordering is written. If `oldest` as landed takes a
     pocket and not a set of records, this ticket gives it the set as its argument,
     changes nothing else in it, re-cuts `quint/mutants/qm05-batch-skips-oldest.patch`
     (created by QM05) and lists both in the hand-back.
  2. `freed(s, lease, key)` is a pure definition that returns the state after the one
     `Freed` rule whose guard holds for that record, with the rule it fired. It has three
     branches here and QM07 adds three.
  3. The release applies the release rule's own writes, then `foldl`s `freed` over
     `held_oldest_first`. The dead letters it creates are appended in that order, which
     is the order the Allium states at lines 825 to 829.
  4. QM08's lapse calls the same definitions.

  Alternative: no fold. The new records are one map over the old, and the new dead
  letters are `oldest` over the nacked held records, mapped and appended. It may cost the
  checker less. It spreads each row of the lease's end over two expressions and writes an
  ordering once per queue. Take it only if the fold does not verify inside the budget,
  and only by a ruling.
- **`ending_records` gets its Allium body** (line 136): held, mark not `unsettled`, and
  `deferred_position = null`. The definition exists from QM02 and is empty until this
  ticket, which makes its conditions on the mark live. The last condition stays in the
  text, always true, until QM07 makes it live.
- **Two of QM05's definitions are completed.** QM05's ticket writes
  `HeldSetStableUntilLeaseEnds` over two definitions it leaves for this ticket to finish:
  `ownMark`, which says a step is an ack or a nack by the lease's consumer that names
  that lease and that key, and `heldVersionShed`, which says a step put the held version
  into a shed queue. This ticket adds the `Ack` and `Nack` steps to the first and the
  dead-letter queue to the second. The property's text does not change; what it checks
  does, and the hand-back says so. Each completed half has a mutant below.
- **`ReleaseIdlesEmptyPocket` is reachable.** It needs every record of the pocket acked
  or nacked. The shortest trace has six steps: `publisherJoins(p0)`, `consumerJoins(c0)`,
  `publish(p0, a, k0, 0)`, `pull(c0, 1)`, `ack(c0, L1, k0)`, `release(c0, L1)`, where
  `L1` is the lease the claim created. Its witness is the check. A leased pocket with no
  record at all would also satisfy the rule's `records.count = ending_records.count`
  (0 = 0). The Allium argues no such state exists (lines 506 to 510). The model asserts
  nothing about it: T46 changes that argument.
- **A publish and its revival are one action.** `PocketTouched` is a chain. The publish
  action applies its publish rule and then, if the pocket was idle before the step,
  `ReviveIdlePocket`. The rules that emit `PocketTouched` (`PublishNewKey` and
  `Supersede` today; QM07 adds `DeferBehindHeldVersion`) are one set behind one
  definition. No state shows an idle pocket that holds a record. `Supersede` on an idle
  pocket is unreachable, because an idle pocket has no record to supersede; the
  transcription keeps it.
- **Two rules of one chain that advance the counter.** `PublishNewKey` ensures
  `queue.next_position = position + 1` (lines 373 to 382). `ReviveIdlePocket`, chained
  behind it, reads the counter and advances it again (lines 467 to 471). The Queue
  entity's comment (lines 100 to 106) says every rule that allots a position takes
  `next_position` and advances it, so the chain is sequential: the record takes
  `prev.next_position`, the pocket takes the next value, and the counter ends two
  higher. `effect_PublishNewKey` as QM04 wrote it for a publish alone is then false on
  this path. The README's second design block fixes the answer: where two rules of one
  chain write the same field, their effects read it through one shared definition, and
  the ticket that first meets the case adds it and says which earlier effect it edits.
  This ticket adds it as `allotted(fired)`, a pure definition that gives the number of
  positions the rules in `fired` allot. Every effect that has a clause on the counter
  states it as `next_position = prev.next_position + allotted(fired)`. The earlier
  effect it edits is QM04's `effect_PublishNewKey`: its counter clause changes, and it
  keeps "the record stands at `prev.next_position`". `effect_ReviveIdlePocket` says "the
  pocket stands at `next_position - 1`, which is not the new record's position". The
  hand-back lists the edit. Only the definition's name and body are proposals. QM08
  reuses it for `ReviveParkedPocket`.
- **Where a chained rule's guard is read.** `FiredMatchesGuards` covers the nine rules.
  The README has a rule later in a chain read its guard in the state the chain had
  reached. `fired` holds `ConsumeAckedRecord` exactly when a release rule's guard held
  and `guard_ConsumeAckedRecord` held for some held record of the lease's pocket;
  `DeadLetterNackedRecord` likewise. It holds `ReviveIdlePocket` exactly when the guard
  of a rule that emits `PocketTouched` held and the pocket was idle. For these three the
  state the chain had reached and `prev` agree on every field the guard reads, because
  the rule before it in the chain writes none of them. The plan shows that for each, and
  says which of the two states the property reads.
- **What the three clockless rules do.** `Renew`, `AckRepeats` and `NackRepeats` change
  nothing but `fired`. The README has each effect state its frame: what neither the rule
  nor another rule fired in the same step writes is equal in `prev` and the state. For
  these three the frame is everything the model holds: no pocket, record, queue or
  counter changes, and every lease keeps its `pocket`, `consumer` and `status`. A
  lease's part of a frame is written field by field, never as equality of a whole lease,
  so that QM09 adds `expires_at` without removing a clause. `effect_AckMarksRecord` and
  `effect_NackMarksRecord` state the mark and leave out the two `ensures` the model
  cannot hold yet. No mutant can catch a broken lease extension until QM09.
- **How `ActionsNameTheirLease` is written.** One property with four clauses. It reads
  `last`, `fired`, `answer` and `prev`, and it reads the leases in `prev` directly, never
  through a `guard_` definition, because a mutant may edit a guard.
  1. If `answer` is a batch, its lease is active after the step, is the consumer's that
     `last` names, and is over the batch's pocket.
  2. If `last` is an ack, nack, renew or release and `fired` is not empty, the lease it
     names was issued, active and that consumer's in `prev`.
  3. If `last` is one of the four and the lease it names was never issued, or was not
     active in `prev`, or was another consumer's, then `fired` is empty. The README's
     `UnansweredStepChangesNothing` (QM02's) then says the state equals `prev`; this
     property does not repeat it.
  4. If `last` is one of the four and `fired` is not empty, the only lease whose status
     changes is the one named, and the only records that change belong to its pocket.

  For `renew` the model can state success only through `fired`. Clause 3 is the
  guarantee's last sentence. It is exercised only on a trace that ends a lease and
  claims the same pocket again, seven steps at the least: `publisherJoins(p0)`,
  `consumerJoins(c0)`, `publish(p0, a, k0, 0)`, `pull(c0, 1)`, `release(c0, L1)`,
  `pull(c0, 1)`, which creates `L2` over the same pocket and holds `k0` again, and then
  `ack(c0, L1, k0)`, which names the ended lease. The mutant
  `qm06-ack-lands-on-successor` is the evidence that the path is inside the bound.

  QM02's ticket recommends a lower-case definition, `staleReleaseRefused`, for the
  release half of this guarantee, and leaves to this ticket whether it stays. It holds
  when `last` is a release that names a lease ended in `prev`, the same consumer has an
  active lease on the same pocket in `prev`, and `fired` is empty. Recommended: keep
  it, unchanged, as the witness of clause 3's release case. The mutant above reaches
  clause 3 through an ack; nothing else shows that a release naming an ended lease,
  with its successor active, is a step the model takes. Step 5 observes it once by
  hand, the way the recipe runs a witness. From the empty start it is seven steps: the
  trace above with a second `release(c0, L1)` in place of the ack. It is not a property,
  its name
  is lower-case, and the recipe does not run it. If QM02's ruling did not land it, the
  observation runs from a scratch copy that defines it, and the model gains nothing.
  Alternative: delete it, on the ground that clause 3 now states the refusal as a
  checked property; the release case then has no run that shows it reached. The
  hand-back says which was done.
- **Three parts of guarantees that later tickets own.** The README fixes the rule: a
  part is a property of its own, named `<Guarantee>_<part>` or for what it says, and the
  owner keeps it. Shared rule 6 lets a mutant name any property the recipe checks, a
  part included. This ticket takes the first form of name, and states each part so that
  it stays true as the model grows:
  - `MarksStandHoweverLeaseEnds_release`. A record that is acked or nacked in `prev` and
    whose lease is still active after the step has the same mark. For an ack or a nack
    by the lease's consumer that names the record's active lease: an ack of an acked
    record fires `AckRepeats` alone, a nack of a nacked one `NackRepeats` alone, and an
    ack of a nacked record or a nack of an acked one fires nothing, so that
    `UnansweredStepChangesNothing` says it changes nothing.
    When a lease's status goes from active to `released`, no record of its pocket that
    was acked in `prev` has a new dead letter, and each that was nacked has exactly one.
  - `EveryDropIsVisible_marked`. A step adds a dead letter only if a lease ended in it,
    and then exactly one for each record of that lease's pocket that was nacked in
    `prev`, carrying the record's `held_rank` and `held_payload` and the lease's
    consumer. A record disappears only in a step that ends its pocket's lease, and only
    if it was acked or nacked in `prev` with nothing deferred behind it.
  - `HeldRecordsOldestFirst_marked`. The dead letters one step adds for one pocket are in
    increasing order of the `position` their records had in `prev`.

  "A lease ended in the step" is read from the state: active in `prev`, not active
  after. It is not read from `last`, so QM08's lapse needs no change to these. "The
  entries a step added" sits behind one definition per queue (today the list beyond its
  length in `prev`, filtered by pocket), so that QM09's decay changes one place. A part
  is needed, and not only effects: `effect_ConsumeAckedRecord` can carry "an acked
  record is gone", but no effect can catch a rejected action being accepted.
  `effect_NackMarksRecord` holds when a nack lands on an acked record.
- **Mutants, and the bound they need.** Each is `quint/mutants/qm06-<slug>.patch` and
  names one property. "Depth" is the shortest trace that shows the violation, counted
  from QM04's empty `init` with both joins; the plan recomputes each. The deepest is 8
  steps, and the deepest witness 7, so this ticket needs a bound of at least 8 for
  `pachinko_walk` in `quint/verify.txt`. QM03's ticket proposes 10. If the base has less
  than 8, this ticket raises it (shared rule 5).

  | Slug | The mutation | Caught by | Depth |
  | --- | --- | --- | --- |
  | `nack-after-ack` | `guard_NackMarksRecord` accepts a record whose mark is `acked` | `MarksStandHoweverLeaseEnds_release` | 6 |
  | `acked-requeued` | the lease's end requeues an acked record | `effect_ConsumeAckedRecord` | 6 |
  | `ack-lands-on-successor` | an ack acts on the active lease over the named lease's pocket, not on the lease named | `ActionsNameTheirLease` | 7 |
  | `release-idles-with-record` | the release idles the pocket when every record is held, whatever its mark | `IdleHasNoRecords` | 5 |
  | `publish-skips-revival` | a publish into an idle pocket fires no `ReviveIdlePocket` | `IdleHasNoRecords` | 7 |
  | `dead-letters-newest-first` | the lease's end takes the held records largest position first | `HeldRecordsOldestFirst_marked` | 8 |
  | `other-consumer-marks` | `guard_AckMarksRecord` drops `lease.consumer = consumer` | `HeldSetStableUntilLeaseEnds` | 6 |
  | `nack-dead-letters-at-once` | `NackMarksRecord` also appends the dead letter, while the lease is active | `HeldSetStableUntilLeaseEnds` | 5 |

  The last two are the mutants the README's "Coverage a later ticket adds" asks of this
  ticket for `HeldSetStableUntilLeaseEnds`: one through `ownMark`, one through
  `heldVersionShed`. For `OneVisitPerKeyPerLease` the new path, an acked or nacked key
  is not handed out again, runs a definition a QM05 mutant already breaks: the pull's
  candidates are the records whose `status` is queued, an acked record is still held,
  and `quint/mutants/qm05-held-key-handed-again.patch` (created by QM05) takes the batch
  from all the pocket's records. The plan confirms that from the landed patch. If the
  patch breaks something else, this ticket adds `qm06-acked-key-handed-again`, aimed at
  `OneVisitPerKeyPerLease`.
- **"Not yet modelled".** This ticket deletes every line of the header's list that names
  QM06 and adds three. Earlier tickets wrote the lines that go: for ack, nack and renew,
  for marks, for the release that idles an empty pocket, and for `ReviveIdlePocket`. A
  line that names QM06 beside another ticket is edited so that it no longer names what
  this ticket adds. The line for deferral that names QM07, and the exception QM05 gave
  `PublishAnsweredByOneRule`, stay until QM07. Each new line is one line of the comment
  and contains this text:

  ```text
  lease extension by AckMarksRecord, AckRepeats, NackMarksRecord, NackRepeats and Renew (QM09)
  the reset of consecutive_expiries by AckMarksRecord, NackMarksRecord, ReleaseIdlesEmptyPocket and ReviveIdlePocket (QM08)
  nacked_at on a held record and on a dead letter (QM09)
  ```

  If an earlier ticket already listed one of these for its own rules, extend that line
  so that it contains the text above. `consecutive_expiries` is not in the state at all
  before QM08, and QM02's seed does not carry it, so the second line names the four
  resets this ticket's rules would make and nothing else about the count.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `just verify-quint` still finishes inside the CI job's timeout with nine more witnesses and eight more mutants | step 6: the wall time, quoted against `timeout-minutes` of the `quint` job in `.github/workflows/ci.yml`. Not run |
| A | `foldl`, `append`, `slice`, `length` and `select` exist for lists | read in `tools/quint/node_modules/@informalsystems/quint/dist/src/builtin.qnt` (lines 522, 395, 483, 435 and 507), not run. Step 5's typecheck |
| B | Apalache checks a `foldl` whose accumulator is the model's state inside the budget | not read. Step 5's first run of the release, timed |
| C | The bound in `quint/verify.txt` reaches every witness and mutant here; the deepest needs 8 steps | QM03 sets the bound and its ticket proposes 10; not known until it lands. Step 2 compares; step 6 raises it if it is below 8 |
| D | `HeldSetStableUntilLeaseEnds` as QM05 landed it reads `ownMark` and `heldVersionShed`, and with those two completed it catches `other-consumer-marks` and `nack-dead-letters-at-once` with its own text unchanged | read in QM05's ticket, which had not landed when this ticket was written. Step 2 reads the model; step 5 runs the two mutants |
| E | `quint/mutants/qm05-held-key-handed-again.patch` breaks the pull's candidate set, which reads `status` and never `mark` | read in QM05's ticket, not in a landed patch. Step 2 reads the patch |
| F | `ReleaseIdlesEmptyPocket` is reachable in six steps | argued from the Allium under "Design proposed". Step 5's witness |
| G | `scripts/verify-quint.sh` prints, for each witness and each mutant it runs, a line that names it | not read: QM03 creates the script, and its ticket fixes what a failure prints, not what a pass prints. Step 2 reads the script, and Verification's last greps depend on it |

## Non-goals

- No deferral. A publish of a higher rank onto a held key keeps no rule until QM07, with
  QM05's line and exception.
- No lapse, no parking and no `consecutive_expiries` (QM08).
- No `now`, no `nacked_at`, no `expires_at`, no TTL and no decay (QM09). A dead letter
  never leaves the model's list.
- No change to the text of a property an earlier ticket owns, apart from the counter
  clause of `effect_PublishNewKey`, which the README's shared-definition rule asks for.
  The two definitions QM05 left to complete, `ownMark` and `heldVersionShed`, are
  completed; both are under "Design proposed".
- No action that reads the `Consuming` surface's `exposes` or either shed queue. The
  Allium's open question about who reads the queues is not touched.
- No named refusal in `answer`. An ack, nack, renew or release hands back nothing; T43
  rules on the caller contract.
- No change to `scripts/verify-quint.sh`, to the CI job or to `just check`'s recipe
  list.
- No trace fixtures (QM10).

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

`<id>` is `QM06`. Checkpoints:

- 1, the plan, `ai_tmp/qm06/plan.md`, with the `conformance` and `breakage` lenses, in
  step 3, before any tracked file changes.
- 2, the diff, with the `conformance`, `breakage` and `design` lenses, in step 8, before
  the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | created by QM02. `mark`, the dead-letter queue, `ack`, `nack` and `renew`, the release by mark, the revival inside the publish; `guard_`, `effect_` and `witness_` for the nine rules; `FiredMatchesGuards` extended; `IdleHasNoRecords`, `ActionsNameTheirLease` and the three parts; `ownMark` and `heldVersionShed` completed; the counter clause of the effects that have one; the "Not yet modelled" lines that name QM06 deleted and three added |
| `quint/mutants/qm06-*.patch` | new | the eight mutants under "Design proposed", each declared the way QM03's recipe reads a mutant |
| `quint/mutants/*.patch` | shared, own lines | created by QM03 and later tickets. Each earlier patch this ticket's edit breaks is re-cut, keeping its mutation and the property it names (shared rule 11) |
| `quint/verify.txt` | shared, own lines | created by QM03. Only if the bound for `pachinko_walk` must rise; it is never lowered |
| `quint/traces/`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines | only if the tree holds them, which needs the series run out of order: QM10 creates the first four, QM12 the last, and QM10 depends on QM07. If it does, regenerate the fixtures with `just quint-traces` before any gate runs, since `just check-quint` and `just verify-quint` both read them; bring the generator's witness list up to the rules this ticket adds; and keep both replays green, mapping each new field or adding it to the replay's not-compared list, and listing a new step there only with the maintainer's answer on which ticket maps it (shared rule 11). A change under `crates/` follows `AGENTS.md`'s block for Rust |
| `docs/decisions/0022-*.md` | shared, own lines | created by QM02 under whatever name it landed. One line for each invariant the representation makes true, one for the surface's `exposes` having no action in the model, and one for the three parts and their owners |
| `.scratch/quint-model/QM06-ack-nack-renew-lease-end.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that the recipe's landed output needs, recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM06's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on
   `ticket/qm06-ack-nack-renew-lease-end` from the series base the maintainer names,
   which must hold QM05 (`tickets/CONVENTIONS.md` §11, "Worktrees and branches", reading
   "the series base" for `main`). Record the base's full commit id as `base`, in the
   hand-back and in `ai_tmp/qm06/base.txt`. `git ls-files .scratch/quint-model/` must
   list the README and this ticket. Run `just initialize` (network, authorised). Run
   `just check` on the untouched worktree and quote its last line, headed by the commit.
   Then show that nothing listens on `localhost:8822`, with the probe and its positive
   control from Verification, and run `just verify-quint` on the untouched worktree:
   quote its exit status and its wall time. That time is the figure step 6 compares
   with.
2. **Read before writing.** Read every path under "Read these". From the model and QM05's
   hand-back, record in `ai_tmp/qm06/plan.md`: the record's type and whether `mark`
   exists; the name and text of `oldest`; how the release folds today; the domain of
   `release`'s `lease` argument; the text of `OneVisitPerKeyPerLease`,
   `HeldSetStableUntilLeaseEnds`, `ownMark` and `heldVersionShed`; which definition
   `quint/mutants/qm05-held-key-handed-again.patch` breaks (claims D and E); the bound in
   `quint/verify.txt` (claim C); and what the recipe prints for a witness and for a mutant
   (claim G). Copy every line of the header's "Not yet modelled" list that names QM06,
   exactly, into `ai_tmp/qm06/gap-lines.txt`; Verification reads it. **Stop** and ask the
   maintainer if `HeldSetStableUntilLeaseEnds` as landed has no clause on marks: this
   ticket does not rewrite a property QM05 owns. Then write the plan:
   - for each of the nine rules, the Allium's text beside the planned action, its
     `guard_<Rule>` and its `effect_<Rule>`, with each `ensures` the model cannot hold
     yet named beside the "Not yet modelled" line that covers it;
   - each property (the two owned, the three parts, `FiredMatchesGuards`' new cases)
     with the ghosts it reads;
   - each mutant with its diff in outline, the one property that must catch it, and its
     shortest trace;
   - each witness with its shortest trace;
   - each "Not yet modelled" line removed, each added, and that no line naming another
     ticket goes.
3. **Adversarial review** (checkpoint 1: `conformance` and `breakage`, over
   `ai_tmp/qm06/plan.md`, before any tracked file changes; `tickets/CONVENTIONS.md`
   §14). Questions:
   - Does any planned guard or effect say more, or less, than the Allium's rule at the
     lines the plan cites? Is `ending_records` whole?
   - Can `ActionsNameTheirLease` hold in a model where an ack that names an ended lease
     marks a record under its successor? Does any clause read a `guard_` definition?
   - Can each part hold for a reason other than the one claimed, and does each stay true
     once QM07 adds deferral and QM08 adds the lapse?
   - Is each mutant caught by the property it names when that property alone is
     checked, and is its shortest trace inside the bound?
   - Does `allotted(fired)` weaken what QM04's effects checked on a publish with no
     revival?
   - Is anything the Allium allows after this ticket missing from the model and from
     "Not yet modelled"?

   **Stop** for the maintainer's ruling on "Design proposed", with the checkpoint's
   findings in hand, and record it.
4. **Red: names, properties and witnesses.** First add what the new definitions read,
   with no behaviour: the `Ack`, `Nack` and `Renew` variants of `Step`; the `acked` and
   `nacked` values of `mark`, if the field landed with one value; and the dead-letter
   queue in the state, empty in `init` and carried unchanged by every action. Without
   them `ActionsNameTheirLease`, the mark effects and `effect_DeadLetterNackedRecord` do
   not typecheck. Then add the `Rule` variants that are missing,
   the nine `guard_`, `effect_` and `witness_` definitions, the two owned properties and
   the three parts. Add no action and change none beyond that frame: no step makes
   `last` an ack, a nack or a renew yet. `just check-quint` passes. Then show the
   red: run the witness check for `witness_AckMarksRecord` and for
   `witness_ReleaseIdlesEmptyPocket` the way QM03's recipe runs a witness, and quote each
   failing as not reached. If the recipe cannot run one witness alone, run
   `just verify-quint` and quote its first failure; re-cut first each earlier patch these
   additions break (shared rule 11), so that the failure is a witness and not a patch
   that does not apply. Here and in every later step, each
   `verify`, by hand or through the recipe, follows the port probe (shared rule 10).
5. **Green, one slice at a time.** After each slice run `just check-quint`, and run the
   slice's witnesses and mutants; quote each.
   - Marks: the writes to `mark`, and the actions `ack`, `nack` and `renew`, which step
     4 left out. Witnesses for the four
     mark rules and `Renew`. `ownMark` completed. Mutants `nack-after-ack` and
     `other-consumer-marks`.
   - The lease's end: the writes to the dead-letter queue, `held_oldest_first`, `freed`,
     the release's choice between its two rules. Time the first verifying run of the
     release (claim
     B). `heldVersionShed` completed. Witnesses for `ConsumeAckedRecord`,
     `DeadLetterNackedRecord` and `ReleaseIdlesEmptyPocket`. Mutants `acked-requeued`,
     `release-idles-with-record`, `dead-letters-newest-first`, `ack-lands-on-successor`
     and `nack-dead-letters-at-once`. The observation of `staleReleaseRefused`, as
     "Design proposed" has it.
   - The revival: `ReviveIdlePocket` inside the publish, `allotted(fired)` and the
     counter clauses. Witness for `ReviveIdlePocket`. Mutant `publish-skips-revival`.
   - Every patch under `quint/mutants/` from an earlier ticket: show that it still
     applies, or re-cut it with the same mutation and the same named property, and list
     it.

   A counterexample against the unchanged model is handled as shared rule 2 says. If it
   shows the Allium wrong or silent, **stop**, write the trace under Open points, and
   ask.
6. **The whole recipe, and the budget.** Show the port free. Run `just verify-quint` and
   quote its exit status and wall time beside step 1's and beside the `quint` job's
   `timeout-minutes`. If the bound in `quint/verify.txt` is below a depth in the plan,
   8 as this ticket counts them, raise it for `pachinko_walk`, never lower it (shared
   rule 5), and run again. The head-room QM03's ruling set is the whole job's (its
   ticket proposes 15 of the job's 20 minutes), and this run is the recipe alone, on
   another machine: add the durations of the setup and the installs that QM03's
   hand-back quotes from CI, and say in the hand-back that the sum is an estimate until
   the job runs. If the sum does not fit that head-room,
   **stop** and ask (ruling 4): do not shrink a domain, drop an instance, drop a mutant,
   lower a bound or raise the timeout.
7. **Words.** Delete the "Not yet modelled" lines in `ai_tmp/qm06/gap-lines.txt` and add
   the three new ones. Add this ticket's lines to decision 0022. Run `just check-docs`.
8. **Adversarial review** (checkpoint 2: `conformance`, `breakage` and `design`, over
   `git diff "$base"...HEAD` plus the uncommitted changes, with step 6's output, before
   the final commit; `tickets/CONVENTIONS.md` §14). Questions:
   - Does the release as written answer each held record by exactly one `Freed` rule,
     in position order, and choose its own rule by `ending_records`?
   - Is there a reachable state after one action in which an idle pocket holds a
     record, or a step that no rule answers and that changes something?
   - Does `FiredMatchesGuards` cover the three chained rules, and does each chained
     guard read only fields the rule before it leaves alone?
   - Could `just verify-quint` pass with a witness of this ticket not reached, or a
     mutant of this ticket uncaught, or caught by another property?
   - Does any effect compare a whole lease, so that QM09 must remove a clause?
   - Did any earlier property or patch change, other than what the hand-back lists?
9. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in the index of
   `.scratch/quint-model/README.md`, write the hand-back notes, and commit the record.
   Then run `just check` once more, on the record's commit, and show its last line and
   an empty `git status --short`. If it fails, fix the notes, commit again and run it
   again; never amend the work's commit.
   Before asking to push, fetch (network, authorised), merge or rebase onto the series
   base, rerun `just check` and Verification, and requote both. If the base had
   moved, the quotes already committed are stale
   (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"): put the fresh
   ones in the hand-back,
   headed by the merged commit, commit that record, and run `just check` on it once
   more, with an empty `git status --short`, as after the first record.
   If the merge or rebase
   changed a file under Files touched, checkpoint 2 did not see that change: show its
   diff at the stop and ask whether it is reviewed before the push. **Stop** before
   pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just check-quint` passes, quoted.
- `just verify-quint` passes, quoted with its commit and exit status, and its output
  shows `Canary` violated on `pachinko_walk`, the nine witnesses reached, and each of
  the eight mutants caught by the property the table names. The instance's pass run, in
  the same output, shows each of those properties holding on the unchanged model at the
  same bound (shared rule 6); no mutant has a run of the unchanged model of its own.
- `quint/pachinko.qnt` holds `guard_`, `effect_` and `witness_` for each of the nine
  rules, and the base's model holds none of them (Verification's two greps).
- `IdleHasNoRecords` and `ActionsNameTheirLease` are properties of the model under those
  names, each named on the recipe's command line.
- `ActionsNameTheirLease` reads the leases in `prev` and no `guard_` definition, shown
  by quoting it.
- The hand-back says whether `staleReleaseRefused` was kept or deleted, and if kept
  quotes it observed.
- The release's wall time (claim B) and the whole recipe's wall time are quoted beside
  the `quint` job's `timeout-minutes`.
- The header's "Not yet modelled" list holds the three lines of "Design proposed", no
  longer holds the lines in `ai_tmp/qm06/gap-lines.txt`, and still holds a line that
  names QM07.
- No domain shrank: the `pachinko_walk` module is byte for byte the base's, and
  `git diff "$base" -- quint/verify.txt` shows no bound lowered.
- Each earlier patch re-cut is listed with its mutation and its property unchanged.
- Decision 0022 holds this ticket's lines, and `just check-docs` passes.
- Each claim, 12 and A to G, has its outcome recorded.
- An owned invariant or guarantee that no mutant aims at is listed as checked and not
  mutation-tested. Today that list is empty.
- Both checkpoints are recorded as `tickets/CONVENTIONS.md` §14's Record asks, with the
  review deltas.
- After the record's commit, `just check` was run once more and its last line and an
  empty `git status --short` are quoted.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script, from the worktree's root, after the commit, with no
other worktree verifying; `set -eu` stops it at the first command that fails.

```sh
set -eu
git rev-parse HEAD
base=$(cat ai_tmp/qm06/base.txt)
git cat-file -e "${base}^{commit}"
git show "${base}:quint/pachinko.qnt" >| ai_tmp/qm06/base-model.qnt
test -s ai_tmp/qm06/base-model.qnt

just check-quint

# Each owned rule has its three definitions here, and none of them at the base.
for rule in AckMarksRecord AckRepeats NackMarksRecord NackRepeats Renew \
  ReleaseIdlesEmptyPocket ConsumeAckedRecord DeadLetterNackedRecord ReviveIdlePocket; do
  for prefix in guard_ effect_ witness_; do
    grep -c -F "${prefix}${rule}" quint/pachinko.qnt
    if grep -F "${prefix}${rule}" ai_tmp/qm06/base-model.qnt; then exit 1; else test "$?" -eq 1; fi
  done
done
for property in IdleHasNoRecords ActionsNameTheirLease; do
  grep -c -E "(val|def) +${property} *[=(:]" quint/pachinko.qnt
  if grep -E "(val|def) +${property} *[=(:]" ai_tmp/qm06/base-model.qnt; then exit 1; else test "$?" -eq 1; fi
done

# The lines that named QM06 are gone: the base holds them and HEAD does not.
test -s ai_tmp/qm06/gap-lines.txt
grep -c -F -f ai_tmp/qm06/gap-lines.txt ai_tmp/qm06/base-model.qnt
if grep -F -f ai_tmp/qm06/gap-lines.txt quint/pachinko.qnt; then exit 1; else test "$?" -eq 1; fi

# What the model still lacks is written down, and the line for QM07 is kept.
grep -F 'Not yet modelled' quint/pachinko.qnt
grep -F 'lease extension by AckMarksRecord, AckRepeats, NackMarksRecord, NackRepeats and Renew (QM09)' quint/pachinko.qnt
grep -F 'the reset of consecutive_expiries by AckMarksRecord, NackMarksRecord, ReleaseIdlesEmptyPocket and ReviveIdlePocket (QM08)' quint/pachinko.qnt
grep -F 'nacked_at on a held record and on a dead letter (QM09)' quint/pachinko.qnt
grep -F 'QM07' quint/pachinko.qnt

# No domain shrank.
sed -n '/^module pachinko_walk/,$p' ai_tmp/qm06/base-model.qnt >| ai_tmp/qm06/walk-base.txt
sed -n '/^module pachinko_walk/,$p' quint/pachinko.qnt >| ai_tmp/qm06/walk-head.txt
test -s ai_tmp/qm06/walk-base.txt
cmp ai_tmp/qm06/walk-base.txt ai_tmp/qm06/walk-head.txt
git diff "$base" -- quint/verify.txt

# The port probe. lsof exits 1 when nothing matches, so the status is read directly.
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

started=$(date +%s)
just verify-quint >| ai_tmp/qm06/verify.log 2>&1
finished=$(date +%s)
echo "verify-quint: $((finished - started)) seconds"

# The run named this ticket's witnesses and mutants (claim G).
for rule in AckMarksRecord AckRepeats NackMarksRecord NackRepeats Renew \
  ReleaseIdlesEmptyPocket ConsumeAckedRecord DeadLetterNackedRecord ReviveIdlePocket; do
  grep -c -F "witness_${rule}" ai_tmp/qm06/verify.log
done
for slug in nack-after-ack acked-requeued ack-lands-on-successor \
  release-idles-with-record publish-skips-revival dead-letters-newest-first \
  other-consumer-marks nack-dead-letters-at-once; do
  test -s "quint/mutants/qm06-${slug}.patch"
  grep -c -F "qm06-${slug}" ai_tmp/qm06/verify.log
done

just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The red tests are the greps against `ai_tmp/qm06/base-model.qnt`: the base's model has
none of the definitions, and the same pattern has just matched the model at `HEAD`, which
is the positive control for each of those probes that passes by printing nothing. For the
header's lines it is the other way round: `grep -c -F -f` matches the base's model first,
with the same pattern file, and only then must the model at `HEAD` print nothing;
`test -s` keeps an empty pattern file from passing. The property pattern matches a
definition (`val` or `def`, the name, then `=`, `(` or `:`), not a mention in a comment;
if the model defines a property another way, correct the pattern and record it under
Departures. `test -s` on the base's model keeps an empty file from reading as "absent".
Each probe that must print nothing reads `grep`'s status: 1 is no match, and 2, a file
or a pattern file that could not be read, stops the script.
The mutants are red tests inside `just verify-quint`: each patched copy must violate the
property it names, and the instance's pass run shows that property holding on the
unchanged model at the same bound, or the recipe fails.
The green tests are `just check-quint`, the recipe's exit status, and the greps over its
log. The listener that `python` starts is the positive control for the port probe after
it. The `wait ... || true` is on a process the script has just killed, and no claim rests
on it. `git diff "$base" -- quint/verify.txt` prints for a reader; the hand-back says
whether it is empty or raises a bound. The last two loops rest on claim G: if the recipe
prints other names, correct the two patterns and record the correction under Departures.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1

### Checkpoint 2

### Patches re-cut

### Properties not mutation-tested

### Departures

### Verification

## Open points

- **The shared definition for the counter.** The README fixes that there is one and
  that this ticket adds it. Its name, `allotted`, and its body are this ticket's
  proposal, and the edit to QM04's `effect_PublishNewKey` follows from it. The
  maintainer rules at step 3. QM08 reuses the definition for `ReviveParkedPocket`.
- **Two of QM05's definitions change what a QM05 property checks.** This ticket
  completes `ownMark` and `heldVersionShed`, as QM05's ticket asks, and leaves the text
  of `HeldSetStableUntilLeaseEnds` alone. If the maintainer reads that as a later ticket
  changing an earlier property, the ruling at step 3 says what to do instead.
- **The surface's `exposes` has no action.** The first sentence of
  `HeldRecordsOldestFirst`, "the held records exposed for a lease are listed in order of
  position", is carried only by `held_oldest_first`, which the lease's end folds over.
  Nothing reads the listing. QM07 owns the guarantee and QM10's replay may compare the
  Rust's listing with the definition. The maintainer says at QM07's ruling whether that
  is enough.
- **The three clockless rules have no mutant that can fail.** `Renew`, `AckRepeats` and
  `NackRepeats` change nothing but `fired` until QM09. Their witnesses rest on
  `FiredMatchesGuards` alone. QM09 adds the mutant ("a lease extension that does not
  extend").
- **What the recipe prints** (claim G). If `scripts/verify-quint.sh` names no witness and
  no mutant in its output, Verification cannot show that this ticket's were run. That is
  a finding for QM03's owner, handed back at step 2, not fixed here.
- **The bound.** This ticket needs 8 steps and QM03's ticket proposes 10. If the base
  has less than 8, step 6 raises it and the run gets slower for every later ticket. The
  maintainer rules at step 6 if the timeout no longer fits.
