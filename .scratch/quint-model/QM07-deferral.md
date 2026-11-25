---
id: QM07
title: "Deferral behind the held version"
status: todo
depends_on: [QM06]
parallel_with: []
branch: ticket/qm07-deferral
estimated_size: L
---

# QM07: Deferral behind the held version

## Context

After QM06 the model in `quint/pachinko.qnt` (created by QM02) has joins, publishes onto
a key nobody holds, the full pull, marks, the dead-letter queue, and a release that
answers each held record by its mark and idles an emptied pocket. One publish still has
no rule: a higher rank onto a held key. QM05 left it without a rule, listed it under
"Not yet modelled" in the model's header, and gave the publish-totality property, QM04's
`PublishAnsweredByOneRule`, a named exception for it (shared rule 8).

The Allium, `docs/specs/pachinko.allium`, answers that publish with
`DeferBehindHeldVersion` (lines 415 to 436): the held version stays exactly as it was
handed out, the newer version waits behind it in the same record, and it is given the
counter's value as its arrival position. A still higher rank supersedes the waiting
version at once, by `Supersede`'s second case (line 398). When the lease ends, six rules
answer `Freed`, one per row of the table at lines 803 to 812; QM05 and QM06 landed the
three rows with nothing deferred, and this ticket lands the other three. Six invariants
tie `deferred_position` to the rest of the record, and five surface guarantees are stated
in full only once deferral exists.

Four things in the Allium are easy to transcribe wrongly, and "Design proposed in this
ticket" fixes each:

- **The position the newer version takes differs by row.** Behind an unsettled held
  version it keeps the record's position: `SupersedeUnsettledHeldVersion` (lines 846
  to 871) has no `ensures` on `position`. Behind an acked or a nacked one it takes its
  arrival position: `ConsumeAckedVersionQueueDeferred` (lines 883 to 898) and
  `DeadLetterNackedVersionQueueDeferred` (lines 923 to 951) set
  `record.position = arrival_position`. Neither advances the counter again (the Queue
  entity's comment, lines 103 to 106).
- **`ending_records` excludes a record with a version deferred** (line 136). A pocket
  whose records are all acked, one of them with a newer version waiting, is requeued by
  `ReleaseRequeuesPocket`, not idled.
- **The stored rank is the newest version's, not the held one's** (lines 332 to 335). A
  publish at the held rank is rejected when nothing is deferred and stale when something
  is.
- **Two of QM05's invariants stop being trivially true.** `HeldVersionNotNewer` and
  `HeldCopiesAgree` say something only once `rank` and `payload` can move away from
  `held_rank` and `held_payload`.

Tickets outside the series that touch this slice (the README's "The T tickets that touch
the model"): `tickets/T43-caller-contract.md` rules on what a publish answers, a deferral
told apart from a supersede among it. In the model a publish hands back nothing, so
nothing here pre-empts that ruling. `tickets/T40-pocket-order-model.md` puts
`deferred_position` and the order of a pocket's records out of its scope.

Read these:

- `.scratch/quint-model/README.md`, in full. It wins over this ticket.
- `docs/specs/pachinko.allium`: the Vocabulary entries "Position", "Held set" and
  "Deferred" (lines 63 to 83); the Queue entity and its comment (lines 100 to 109);
  `ending_records` (lines 133 to 136); the `Record` entity, its comment and its seven
  invariants (lines 166 to 223); the Publish comment (lines 319 to 335); `Supersede`,
  `DeferBehindHeldVersion` and `DropStalePublish` (lines 386 to 454); "Lease end" with
  its table, its comment and its six rules (lines 794 to 951); the three invariants on
  arrival positions and their comment (lines 1078 to 1103); the guarantees
  `NeverRefusedForCapacity`, `EveryDropIsVisible` and `OldestSlotNewestPayload` (lines
  1116 to 1143), `OldestFirst` (lines 1185 to 1189), `HeldRecordsOldestFirst`,
  `OneVisitPerKeyPerLease` and `HeldSetStableUntilLeaseEnds` (lines 1191 to 1213) and
  `NothingLostWhileHeld` (lines 1223 to 1229).
- `docs/decisions/0020-held-set-until-lease-ends.md`, "Decision" and "Consequences".
- `crates/libpachinko/tests/queue/walk.rs`: `WalkStep` (lines 80 to 117) and `config()`
  (lines 59 to 67), for names and for how a payload is built. Not for behaviour (shared
  rule 1).
- `quint/pachinko.qnt` at this ticket's base, in full, and the hand-back notes of QM04's,
  QM05's and QM06's tickets in this directory (`QM04-publish.md`,
  `QM05-pull-and-held-set.md`, `QM06-ack-nack-renew-lease-end.md`): the publish-totality
  property and its exception, the text of the properties this ticket adds paths to, the
  definitions the lease's end is built from, and the rulings on how a part of a
  guarantee is named and how two effects of one chain compose on the counter.
- `scripts/verify-quint.sh` and `quint/verify.txt` (both created by QM03), and one patch
  under `quint/mutants/`: how a witness and a mutant are declared, and the bound.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

The gap QM05 named is closed. A higher rank onto a held key is deferred. A higher rank
onto a deferred version supersedes it at once. The lease's end answers all six rows of
the Allium's `Freed` table, each with the position that row gives the newer version. The
"Not yet modelled" lines for deferral and the named exception in
`PublishAnsweredByOneRule` are deleted, so the checked fragment of
`NeverRefusedForCapacity` holds with no exception. Four rules get their guard, effect
and witness, six invariants and five guarantees become checked properties under their
Allium names, and thirteen mutants are caught, each by the one property it names.

Owning five guarantees is not "checked in full" (the README's "Coverage a later ticket
adds"). QM08 adds a lease ended by a lapse to four of them: `EveryDropIsVisible`,
`OldestSlotNewestPayload`, `HeldRecordsOldestFirst` and `NothingLostWhileHeld`. QM09 adds
the decay to `EveryDropIsVisible`. `NeverRefusedForCapacity` shows that every publish in
the model's finite domains is accepted or is at the stored rank. It does not show
unbounded capacity, and the eviction order is excluded (decision 0022).

### Owns

| Kind | Names |
| --- | --- |
| Rules | `DeferBehindHeldVersion`, `SupersedeUnsettledHeldVersion`, `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedVersionQueueDeferred` |
| Invariants | `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` |
| Guarantees | `NeverRefusedForCapacity`, `EveryDropIsVisible`, `OldestSlotNewestPayload`, `HeldRecordsOldestFirst`, `NothingLostWhileHeld` |
| Partial or extended, not counted | `Supersede` (QM04's) gains its second case, a held record with a version already deferred. `DropStalePublish` (QM04's) gains a publish below a deferred rank. `ReleaseRequeuesPocket` (QM02's) gains the pocket whose marked records have a version deferred. Eight earlier properties gain a path, the ones the README's "Coverage a later ticket adds" lists for this ticket: `OldestFirst`, `OneVisitPerKeyPerLease`, `HeldSetStableUntilLeaseEnds`, `HeldVersionNotNewer`, `HeldCopiesAgree`, `UniqueRecordPosition`, `RecordPositionsBelowNext` and `IdleHasNoRecords` |

## Design proposed in this ticket

These are proposals. The maintainer confirms them, or changes them, after checkpoint 1
and before step 4, and the hand-back records the ruling. After that the builder does not
reopen them. Names the README fixes are fixed. This ticket calls QM06's definitions
`freed` (one held record answered by its `Freed` rule), `held_oldest_first` (a pocket's
held records in position order) and `allotted` (the shared definition the README asks
for where two rules of one chain advance the counter); read the landed names from the
model. A publish that no rule answers changes nothing, which the README's
`UnansweredStepChangesNothing` (QM02's) checks; no property here repeats it.

- **What the state gains.** Nothing new is declared if QM05 landed `deferred_position` as
  an optional field that is always empty; it now takes a value. If QM05 did not land it,
  this ticket adds it to a held record. `rank` and `payload` now move while a record is
  held; `held_rank` and `held_payload` never do. No constant and no domain changes, so
  `pachinko_walk` is untouched.
- **What answers a publish onto a held record.** Five guards share the `Publish` trigger
  and stay disjoint. For a record that is held:

  | Deferred behind it | The publish's rank against `record.rank` | Rule |
  | --- | --- | --- |
  | nothing | higher | `DeferBehindHeldVersion` |
  | nothing | equal | none: rejected, nothing changes |
  | nothing | lower | `DropStalePublish` |
  | a version | higher | `Supersede`, its second case |
  | a version | equal | none: rejected, nothing changes |
  | a version | lower, the held rank included | `DropStalePublish` |

  `guard_Supersede` is the Allium's line 398 whole. If QM04 or QM05 already wrote the
  second disjunct, the guard does not change and only the action and the effect are
  checked against the new case. `Supersede` on a deferred version leaves
  `deferred_position` alone (lines 388 to 390).
- **`DeferBehindHeldVersion` is one more rule that emits `PocketTouched` and allots a
  position.** It joins the set of emitting rules QM06 put behind one definition. Its
  pocket is leased, so no revival follows. `allotted` counts one position for it, and
  `effect_DeferBehindHeldVersion` reads the counter through it, as the README has every
  effect of a chain do. The two rules that queue a deferred version at the lease's end
  allot none.
- **The lease's end gains three rows.** `freed` gets three more branches, chosen by the
  mark and by `deferred_position != null`:

  | Rule | The held version | The record afterwards |
  | --- | --- | --- |
  | `SupersedeUnsettledHeldVersion` | a superseded-queue entry: `held_rank`, `held_payload`, reason `superseded` | queued, at its own `position`, with `rank` and `payload` as they are |
  | `ConsumeAckedVersionQueueDeferred` | consumed: no entry | queued, at `deferred_position`, with `rank` and `payload` as they are |
  | `DeadLetterNackedVersionQueueDeferred` | a dead letter: `held_rank`, `held_payload`, `nacked_by` the lease's consumer | queued, at `deferred_position`, with `rank` and `payload` as they are |

  The arrival position an unsettled record never takes is not reused; positions need
  not be dense, and no invariant says they are. `ending_records` exists from QM02 as a
  definition, empty until QM06 gave it its Allium body (line 136) and made its
  conditions on the mark live. This ticket makes its last condition,
  `deferred_position = null`, live, and with it the choice between the two release
  rules. If the landed body lacks that condition, this ticket adds it.
- **The exception goes, and the count gains the fifth guard.** QM04's ticket writes
  `PublishAnsweredByOneRule` as the number of publish rules whose guard holds, over the
  four rules it has. This ticket adds `guard_DeferBehindHeldVersion` to that count and
  deletes the named exception QM05 gave the property, a definition of its own. Nothing
  else in the property changes. Every line of the header's "Not yet modelled" list that
  names QM07 is deleted in the same change (shared rule 8): earlier tickets wrote them
  for `DeferBehindHeldVersion` and for the held case of `Supersede`.
- **The six invariants are transcribed whole** from lines 206 to 216 and 1084 to 1103,
  each under its Allium name. None is expected to be true by representation. If the
  record's type makes one so (for example held fields that exist only in a held
  variant), decision 0022 gets a line with the reason.
- **How each guarantee is written.** Each is one property under its Allium name. "A
  lease ended in the step" is read from the state, active in `prev` and not active
  after, never from `last`, so that QM08's lapse needs no change to them. "The entries a
  step added" to a queue is the definition QM06 landed for it.
  - `NeverRefusedForCapacity` reads no ghost. For every publisher the publish rules
    accept, and every pocket key, record key and rank of the instance: if a record of
    that key stands in that pocket at exactly that rank, no publish guard holds;
    otherwise exactly one of the five holds. That no guard reads a count or a size is
    checked by reading the five guards at checkpoint 2, not by the checker. The
    property reads guards, not `step`: a `step` that stopped offering a publish in some
    state would break no property, because a step that is not offered is in no trace.
    So checkpoint 2 also reads `step`, as QM04's did, and step 5 repeats QM04's
    observation of a publish answered in a deployment whose pocket keys all have a
    pocket, this time for a deferral.
  - `EveryDropIsVisible` reads `prev` and `last`. A publish by a publisher the publish
    rules accept adds one superseded entry when its rank is above a record that is
    queued or has a version deferred (the
    loser's rank and payload, reason `superseded`), one when its rank is below the
    stored rank (its own rank and payload, reason `stale`), and none otherwise; it adds
    no dead letter. A step that ends a lease adds, for that lease's pocket, exactly one
    superseded entry for each record that was held unsettled with a version deferred,
    exactly one dead letter for each record that was nacked, and nothing for any other
    record. No other step adds an entry, and no step removes one. A record disappears
    only when its lease ends and it was acked or nacked with nothing deferred. An entry
    added to one queue in a step shares its pocket, key and rank with no entry added to
    the other. It is stated per step because one key can be published at one rank twice,
    once its record has been removed.
  - `OldestSlotNewestPayload` reads `prev`. A record present before and after a step
    has the same `position`, unless its lease ended in the step and it was acked or
    nacked with a version deferred; then its `position` is the `deferred_position` it
    had in `prev`. A record held before and after with a `deferred_position` keeps it.
  - `HeldRecordsOldestFirst` reads `prev`. In a step that ends a lease, the superseded
    entries added for its pocket are in increasing order of the `position` their records
    had in `prev`, and so are the dead letters. The order is by `position`, never by
    `deferred_position`. The guarantee's first sentence, on what the surface exposes, is
    carried by `held_oldest_first`, which the lease's end folds over.
  - `NothingLostWhileHeld` reads `prev` and `last`. After a publish above the stored
    rank onto a held record, by a publisher the publish rules accept, the record is
    still held, carries the publish's rank and
    payload, and has the `held_rank`, `held_payload` and `mark` it had. After a step
    that ends a lease, each record of its pocket that had a version deferred is present
    and queued with the `rank` and `payload` it had. An ack or a nack changes no
    record's `rank`, `payload` or `deferred_position`.
- **The two shed queues are two lists.** `HeldRecordsOldestFirst` is checked inside
  each. Whether a superseded entry or a dead letter of one lease's end was created first
  is not in the state, and the Allium's entities carry nothing that would say.
- **The parts earlier tickets added stay.** The README fixes it: a part of a guarantee
  that a ticket checks before the owner lands is a property of its own, and the owner
  keeps it. QM04's three (`PublishAnsweredByOneRule`, `QueuedRecordKeepsPosition` and
  `ShedVersionIsQueued`) and QM06's three for acked and nacked records stay, with their
  names and their mutants (shared rule 11). The Allium-named property is written from
  the guarantee's text, not by editing a part. `PublishAnsweredByOneRule`, with the
  exception gone and the fifth guard counted, may say what `NeverRefusedForCapacity`
  says; both stay.
- **Mutants, and the bound they need.** Each is `quint/mutants/qm07-<slug>.patch` and
  names one property, which shared rule 6 lets be any property the recipe checks.
  "Depth" is the shortest trace that shows the violation, counted from QM04's empty
  `init` with both joins; the plan recomputes each. The deepest mutant is 8 steps and
  the `OldestFirst` path below is 9, so this ticket needs a bound of at least 9 for
  `pachinko_walk` in `quint/verify.txt`. QM03's ticket proposes 10. If the base has less
  than 9, this ticket raises it (shared rule 5).

  | Slug | The mutation | Caught by | Depth |
  | --- | --- | --- | --- |
  | `deferred-version-lost` | the lease's end removes an acked record that has a version deferred | `NothingLostWhileHeld` | 7 |
  | `arrival-position-reused` | `DeferBehindHeldVersion` does not advance the counter | `ArrivalPositionUnoccupied` | 6 |
  | `arrival-position-collides` | the same mutation as the row above | `UniqueRecordPosition` | 8 |
  | `defer-changes-held-version` | `DeferBehindHeldVersion` also writes `held_payload` | `HeldSetStableUntilLeaseEnds` | 5 |
  | `deferred-handed-under-same-lease` | the pull's candidates include a held record with a version deferred | `OneVisitPerKeyPerLease` | 6 |
  | `acked-row-keeps-held-position` | `ConsumeAckedVersionQueueDeferred` leaves `position` alone | `OldestSlotNewestPayload` | 7 |
  | `held-rank-above-rank` | `DeferBehindHeldVersion` writes the rank into `held_rank` and leaves `rank` | `HeldVersionNotNewer` | 5 |
  | `defer-without-arrival-position` | `DeferBehindHeldVersion` leaves `deferred_position` empty | `HeldCopiesAgree` | 5 |
  | `nacked-row-past-counter` | `DeadLetterNackedVersionQueueDeferred` queues the record one past the counter's value | `RecordPositionsBelowNext` | 7 |
  | `ending-ignores-deferred` | `ending_records` drops `deferred_position = null` | `IdleHasNoRecords` | 7 |
  | `deferred-loser-dropped` | `Supersede`'s second case adds no superseded entry | `EveryDropIsVisible` | 6 |
  | `lease-end-newest-first` | the lease's end takes the held records largest position first | `HeldRecordsOldestFirst` | 8 |
  | `stale-against-held-rank` | `guard_DropStalePublish` compares with `held_rank` when the record is held | `NeverRefusedForCapacity` | 5 |

  `deferred-loser-dropped` is also the evidence that `Supersede`'s second case is
  reachable: `witness_Supersede` is reached through the queued case and says nothing
  about the second.
- **Coverage of earlier properties** (the README's "Coverage a later ticket adds").
  `OneVisitPerKeyPerLease`, `HeldSetStableUntilLeaseEnds`, `HeldVersionNotNewer` and
  `HeldCopiesAgree` each have a mutant in the table. `OldestFirst` gains the path "a
  version queued at its arrival position is pulled in position order"; it runs `oldest`,
  which `quint/mutants/qm05-batch-skips-oldest.patch` (created by QM05) already breaks,
  and the plan confirms that from the landed patch. Its shortest trace has nine steps:
  two joins, `publish(p0, a, k0, 0)`, `pull(c0, 1)`, `publish(p0, a, k1, 0)`,
  `publish(p0, a, k0, 1)`, `ack(c0, L1, k0)`, `release(c0, L1)` and `pull(c0, 2)`, whose
  batch is `k1` then `k0`. A bound below nine does not check the path at all.
  `UniqueRecordPosition` and `RecordPositionsBelowNext` (QM04's) gain the path "a record
  takes its arrival position when its lease ends", and `IdleHasNoRecords` (QM06's) the
  path "a pocket whose acked record has a version deferred requeues and does not idle";
  each of the three has a mutant in the table.
- **Five owned invariants have no mutant.** `DeferredHasArrivalPosition`,
  `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `UniqueArrivalPosition`
  and `ArrivalPositionsBelowNext` are checked on every instance and reported as not
  mutation-tested (shared rule 6). Alternative: five more patches, at five more runs of
  the checker. The maintainer chooses at the ruling.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `just verify-quint` still finishes inside the CI job's timeout with four more witnesses and thirteen more mutants | step 6: the wall time, quoted against `timeout-minutes` of the `quint` job in `.github/workflows/ci.yml`. Not run |
| A | The bound in `quint/verify.txt` reaches every witness and mutant here, 8 steps at the deepest, and the nine-step `OldestFirst` path | QM03 sets the bound and its ticket proposes 10; not known until it lands. Step 2 compares; step 6 raises it if it is below 9 |
| B | QM06 landed one definition for a held record's `Freed` rule, one for the held records in position order, one for the positions a step allots, and one per queue for the entries a step added | not read: QM06 had not landed when this ticket was written. Step 2 reads the model |
| C | `HeldSetStableUntilLeaseEnds` and `OneVisitPerKeyPerLease` as QM05 landed them catch their two mutants here unchanged | not read. Step 5 runs the mutants |
| D | `HeldVersionNotNewer` and `HeldCopiesAgree` were transcribed whole by QM05, `deferred_position = null` included | not read. Step 2 reads them beside Allium lines 194 to 196 and 218 to 222 |
| E | `quint/mutants/qm05-batch-skips-oldest.patch` breaks `oldest`, and the pull reads a record's `position` through `oldest` alone | read in QM05's ticket, not in a landed patch. Step 2 reads the patch and the pull |
| F | With the exception deleted and four guards counted, `PublishAnsweredByOneRule` is violated; with the fifth counted and no action, `FiredMatchesGuards` is | step 4, the red tests. Not run |
| G | `scripts/verify-quint.sh` prints, for each witness and each mutant it runs, a line that names it | not read: QM03 creates the script, and its ticket fixes what a failure prints, not what a pass prints. Step 2 reads the script, and Verification's last greps depend on it |

## Non-goals

- No lapse and no parking (QM08). The six `Freed` rules are called from a release only.
- No `now`, no timestamp, no TTL and no decay (QM09). No entry ever leaves a shed queue
  in the model.
- No eviction for capacity and no claim about unbounded memory. Decision 0022 excludes
  the eviction order.
- No answer for a publish. T43 rules on whether a deferral is told apart from a
  supersede.
- No change to the meaning of a property an earlier ticket owns. The one property
  edited is `PublishAnsweredByOneRule`: its exception goes and its count gains the fifth
  guard.
- No new constant, no new domain value and no change to `pachinko_walk`.
- No change to `scripts/verify-quint.sh`, to the CI job or to `just check`'s recipe
  list.
- No trace fixtures. QM10 follows this ticket.
- Neither of the Allium's open questions is touched.

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

`<id>` is `QM07`. Checkpoints:

- 1, the plan, `ai_tmp/qm07/plan.md`, with the `conformance` and `breakage` lenses, in
  step 3, before any tracked file changes.
- 2, the diff, with the `conformance`, `breakage` and `design` lenses, in step 8, before
  the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | created by QM02. Deferral in the publish, `Supersede`'s second case, three more rows of the lease's end; `guard_`, `effect_` and `witness_` for the four rules; `FiredMatchesGuards` extended; the six invariants and the five guarantees; in `PublishAnsweredByOneRule`, QM05's exception deleted and the fifth guard counted; the "Not yet modelled" lines that name QM07 deleted |
| `quint/mutants/qm07-*.patch` | new | the thirteen mutants under "Design proposed", each declared the way QM03's recipe reads a mutant |
| `quint/mutants/*.patch` | shared, own lines | created by QM03 and later tickets. Each earlier patch this ticket's edit breaks is re-cut, keeping its mutation and the property it names (shared rule 11) |
| `quint/verify.txt` | shared, own lines | created by QM03. Only if the bound for `pachinko_walk` must rise; it is never lowered |
| `quint/traces/`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines | only if the tree holds them, which needs the series run out of order: QM10 creates the first four, QM12 the last, and QM10 depends on this ticket. If it does, regenerate the fixtures with `just quint-traces` before any gate runs, since `just check-quint` and `just verify-quint` both read them; bring the generator's witness list up to the rules this ticket adds; and keep both replays green, mapping each new field or adding it to the replay's not-compared list, and listing a new step there only with the maintainer's answer on which ticket maps it (shared rule 11). A change under `crates/` follows `AGENTS.md`'s block for Rust |
| `docs/decisions/0022-*.md` | shared, own lines | created by QM02 under whatever name it landed. One line for each invariant the representation makes true; one saying that the order of one lease's end is checked inside each shed queue and not across the two; one saying what `NeverRefusedForCapacity` shows and does not |
| `.scratch/quint-model/QM07-deferral.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that the recipe's landed output needs, recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM07's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm07-deferral` from the
   series base the maintainer names, which must hold QM06 (`tickets/CONVENTIONS.md` §11,
   "Worktrees and branches", reading "the series base" for `main`). Record the base's
   full commit id as `base`, in the hand-back and in `ai_tmp/qm07/base.txt`.
   `git ls-files .scratch/quint-model/` must list the README and this ticket. Run
   `just initialize` (network, authorised). Run `just check` on the untouched worktree
   and quote its last line, headed by the commit. Then show that nothing listens on
   `localhost:8822`, with the probe and its positive control from Verification, and run
   `just verify-quint` on the untouched worktree: quote its exit status and its wall
   time. That time is the figure step 6 compares with.
2. **Read before writing.** Read every path under "Read these". Copy every line of the
   header's "Not yet modelled" list that names QM07, exactly, into
   `ai_tmp/qm07/gap-lines.txt`, and the name of the exception's definition in
   `PublishAnsweredByOneRule` into `ai_tmp/qm07/exception-name.txt`, one name on one
   line; Verification
   reads both files. Record in `ai_tmp/qm07/plan.md` what claims A, B, D, E and G turn
   on. **Stop** and ask the maintainer if an earlier ticket already uses one of the
   five guarantees' Allium names for a property, or if `HeldVersionNotNewer` or
   `HeldCopiesAgree` as landed is not the Allium's text whole: this ticket does not
   rewrite a property another ticket owns. Then write the plan:
   - for each of the four rules, and for `Supersede`'s second case, the Allium's text
     beside the planned action, its `guard_<Rule>` and its `effect_<Rule>`;
   - the table of what answers a publish onto a held record, with the guard that makes
     each row exclusive;
   - each of the six invariants beside the Allium's text, and each of the five
     guarantees with the ghosts it reads;
   - each mutant with its diff in outline, the one property that must catch it, and its
     shortest trace; each witness with its shortest trace;
   - for each earlier property that gains a path, the mutant on that path or the
     definition an existing patch breaks;
   - the "Not yet modelled" line removed, and that none is added.
3. **Adversarial review** (checkpoint 1: `conformance` and `breakage`, over
   `ai_tmp/qm07/plan.md`, before any tracked file changes; `tickets/CONVENTIONS.md`
   §14). Questions:
   - For each of the six rows of the Allium's table at lines 803 to 812, does the plan
     give the record the position that row says, and does any row advance the counter?
   - Are the five publish guards disjoint for a held record, and is a publish at the
     held rank stale exactly when a version is deferred?
   - Can each of the five guarantees hold for a reason other than the one claimed? Does
     any read `last` to find a lease's end, so that QM08 must change it?
   - Is each mutant caught by the property it names when that property alone is
     checked, and is its shortest trace inside the bound?
   - Does any earlier property gain a path that the plan covers with neither a mutant
     nor a named definition?
   - With the exception gone, is anything the Allium allows on a publish still missing
     from the model and from "Not yet modelled"?

   **Stop** for the maintainer's ruling on "Design proposed", with the checkpoint's
   findings in hand, and record it.
4. **Red: the exception, the names and the witnesses.** Add the `Rule` variants that are
   missing, the four `guard_`, `effect_` and `witness_` definitions, the six invariants
   and the five guarantees. Delete the exception from `PublishAnsweredByOneRule` and do
   not add the fifth guard to its count yet. Do not change an action yet.
   `just check-quint` passes. Then show the red, in three runs. Check
   `PublishAnsweredByOneRule` alone, the way QM03's recipe checks one property, and quote
   it violated at a state where a higher rank onto a held key has no rule counted (claim
   F). Add `guard_DeferBehindHeldVersion` to the count: the property now passes, and
   `FiredMatchesGuards`, checked alone, is violated by the same publish, whose guard
   holds while `fired` stays empty. Run the witness check for
   `witness_DeferBehindHeldVersion` and quote it not reached. If the recipe cannot run
   one property or one witness alone, run `just verify-quint` and quote its first
   failure; re-cut first each earlier patch these additions break (shared rule 11), so
   that the failure is a property or a witness and not a patch that does not apply.
   Here and in every later step, each `verify`, by hand or through the recipe,
   follows the port probe (shared rule 10).
5. **Green, one slice at a time.** After each slice run `just check-quint`, and run the
   slice's witnesses and mutants; quote each.
   - The publish: `DeferBehindHeldVersion`, `Supersede`'s second case, the stale publish
     below a deferred rank, and the "Not yet modelled" line deleted.
     `PublishAnsweredByOneRule` and `NeverRefusedForCapacity` pass. Witness for
     `DeferBehindHeldVersion`. One observation by hand, run the way the recipe runs a
     witness, on a scratch copy under `ai_tmp/qm07/` that adds one definition,
     `deferIntoFullDeployment`: `fired` holds `DeferBehindHeldVersion` and every pocket
     key of the instance has a pocket in `prev`. Seven steps reach it: two joins, a
     publish into each of `a`, `b` and `c`, a pull of limit 1, and a higher rank onto
     the held key. Quote it observed. Mutants `arrival-position-reused`,
     `defer-changes-held-version`, `deferred-handed-under-same-lease`,
     `held-rank-above-rank`, `defer-without-arrival-position`, `deferred-loser-dropped`
     and `stale-against-held-rank`.
   - The lease's end: the three rows with a version deferred. Witnesses for
     `SupersedeUnsettledHeldVersion`, `ConsumeAckedVersionQueueDeferred` and
     `DeadLetterNackedVersionQueueDeferred`. Mutants `deferred-version-lost`,
     `arrival-position-collides`, `acked-row-keeps-held-position`,
     `nacked-row-past-counter`, `ending-ignores-deferred` and `lease-end-newest-first`.
   - Every patch under `quint/mutants/` from an earlier ticket: show that it still
     applies, or re-cut it with the same mutation and the same named property, and list
     it.

   A counterexample against the unchanged model is handled as shared rule 2 says. If it
   shows the Allium wrong or silent, **stop**, write the trace under Open points, and
   ask.
6. **The whole recipe, and the budget.** Show the port free. Run `just verify-quint` and
   quote its exit status and wall time beside step 1's and beside the `quint` job's
   `timeout-minutes`. If the bound in `quint/verify.txt` is below a depth in the plan, the
   nine steps of the `OldestFirst` path included, raise it for `pachinko_walk`, never
   lower it (shared rule 5), and run again. The head-room QM03's ruling set is the whole
   job's (its ticket proposes 15 of the job's 20 minutes), and this run is the recipe
   alone, on another machine: add the durations of the setup and the installs that
   QM03's hand-back quotes from CI, and say in the hand-back that the sum is an
   estimate until the job runs. If the sum does not fit that head-room, **stop** and ask
   (ruling 4): do not shrink a domain, drop an instance, drop a mutant, lower a bound or
   raise the timeout.
7. **Words.** Add this ticket's lines to decision 0022. Confirm that the model's header
   no longer lists deferral and lists nothing new. Run `just check-docs`.
8. **Adversarial review** (checkpoint 2: `conformance`, `breakage` and `design`, over
   `git diff "$base"...HEAD` plus the uncommitted changes, with step 6's output, before
   the final commit; `tickets/CONVENTIONS.md` §14). Questions:
   - Does the lease's end as written answer each held record by exactly one of the six
     rules, and does `FiredMatchesGuards` cover the three new ones?
   - Does any of the five publish guards read a count, a size or a length? Does `step`
     offer `publish` for every publisher, pocket key, record key and rank of the instance
     in every state, or is a choice made only when the state allows it?
   - Is there a reachable state in which a version published above the stored rank is
     in the record, in no shed queue and not consumed, and yet gone after the lease's
     end?
   - Could `just verify-quint` pass with a witness of this ticket not reached, or a
     mutant of this ticket uncaught, or caught by another property?
   - Did any property of an earlier ticket change, other than the two edits to
     `PublishAnsweredByOneRule`? Did any patch change its mutation or the property it
     names?
   - Does the header's "Not yet modelled" list still say everything the model lacks?
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
  shows `Canary` violated on `pachinko_walk`, the four witnesses reached, and each of
  the thirteen mutants caught by the property the table names. The instance's pass run,
  in the same output, shows each of those properties holding on the unchanged model at
  the same bound (shared rule 6); no mutant has a run of the unchanged model of its own.
- `quint/pachinko.qnt` holds `guard_`, `effect_` and `witness_` for each of the four
  rules, and a property under each of the eleven Allium names this ticket owns; the
  base's model holds none of them (Verification's greps).
- `deferIntoFullDeployment` is observed, quoted, and checkpoint 2's design reviewer
  answered the question on what `step` offers.
- `PublishAnsweredByOneRule` carries no exception and counts five guards, and step 4's
  red runs are quoted: the property violated before the fifth guard was counted, and
  `FiredMatchesGuards` violated before the action fired the rule.
- The header's "Not yet modelled" list no longer holds the lines in
  `ai_tmp/qm07/gap-lines.txt`, and this ticket added none.
- For each row of the Allium's table at lines 803 to 812, the hand-back names the rule,
  its witness and the position the record ends at.
- Each earlier property that gains a path has its mutant caught, or the hand-back names
  the definition and the existing patch that breaks it. The nine-step `OldestFirst` path
  is inside the bound.
- The whole recipe's wall time is quoted beside the `quint` job's `timeout-minutes`.
- No domain shrank: the `pachinko_walk` module is byte for byte the base's, and
  `git diff "$base" -- quint/verify.txt` shows no bound lowered.
- Each earlier patch re-cut is listed with its mutation and its property unchanged.
- Decision 0022 holds this ticket's lines, and `just check-docs` passes.
- Each claim, 12 and A to G, has its outcome recorded.
- The five owned invariants no mutant aims at are listed as checked and not
  mutation-tested.
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
base=$(cat ai_tmp/qm07/base.txt)
git cat-file -e "${base}^{commit}"
git show "${base}:quint/pachinko.qnt" >| ai_tmp/qm07/base-model.qnt
test -s ai_tmp/qm07/base-model.qnt

just check-quint

# Each owned rule has its three definitions here, and none of them at the base.
for rule in DeferBehindHeldVersion SupersedeUnsettledHeldVersion \
  ConsumeAckedVersionQueueDeferred DeadLetterNackedVersionQueueDeferred; do
  for prefix in guard_ effect_ witness_; do
    grep -c -F "${prefix}${rule}" quint/pachinko.qnt
    if grep -F "${prefix}${rule}" ai_tmp/qm07/base-model.qnt; then exit 1; else test "$?" -eq 1; fi
  done
done
for property in DeferredHasArrivalPosition OnlyDeferredHasArrivalPosition \
  DeferredArrivedAfterHeld ArrivalPositionUnoccupied UniqueArrivalPosition \
  ArrivalPositionsBelowNext NeverRefusedForCapacity EveryDropIsVisible \
  OldestSlotNewestPayload HeldRecordsOldestFirst NothingLostWhileHeld; do
  grep -c -E "(val|def) +${property} *[=(:]" quint/pachinko.qnt
  if grep -E "(val|def) +${property} *[=(:]" ai_tmp/qm07/base-model.qnt; then exit 1; else test "$?" -eq 1; fi
done

# The gap is closed: the base lists it and names its exception, and HEAD does neither.
test -s ai_tmp/qm07/gap-lines.txt
test -s ai_tmp/qm07/exception-name.txt
grep -F 'Not yet modelled' quint/pachinko.qnt
grep -c -F -f ai_tmp/qm07/gap-lines.txt ai_tmp/qm07/base-model.qnt
if grep -F -f ai_tmp/qm07/gap-lines.txt quint/pachinko.qnt; then exit 1; else test "$?" -eq 1; fi
grep -c -F -f ai_tmp/qm07/exception-name.txt ai_tmp/qm07/base-model.qnt
if grep -F -f ai_tmp/qm07/exception-name.txt quint/pachinko.qnt; then exit 1; else test "$?" -eq 1; fi

# No domain shrank.
sed -n '/^module pachinko_walk/,$p' ai_tmp/qm07/base-model.qnt >| ai_tmp/qm07/walk-base.txt
sed -n '/^module pachinko_walk/,$p' quint/pachinko.qnt >| ai_tmp/qm07/walk-head.txt
test -s ai_tmp/qm07/walk-base.txt
cmp ai_tmp/qm07/walk-base.txt ai_tmp/qm07/walk-head.txt
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
just verify-quint >| ai_tmp/qm07/verify.log 2>&1
finished=$(date +%s)
echo "verify-quint: $((finished - started)) seconds"

# The run named this ticket's witnesses and mutants (claim G).
for rule in DeferBehindHeldVersion SupersedeUnsettledHeldVersion \
  ConsumeAckedVersionQueueDeferred DeadLetterNackedVersionQueueDeferred; do
  grep -c -F "witness_${rule}" ai_tmp/qm07/verify.log
done
for slug in deferred-version-lost arrival-position-reused arrival-position-collides \
  defer-changes-held-version deferred-handed-under-same-lease \
  acked-row-keeps-held-position held-rank-above-rank defer-without-arrival-position \
  ending-ignores-deferred deferred-loser-dropped lease-end-newest-first \
  stale-against-held-rank nacked-row-past-counter; do
  test -s "quint/mutants/qm07-${slug}.patch"
  grep -c -F "qm07-${slug}" ai_tmp/qm07/verify.log
done

just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The red tests are the greps against `ai_tmp/qm07/base-model.qnt` and the two probes for
the gap. For the definitions, the base's model has none and the same pattern has just
matched the model at `HEAD`, which is the positive control for each probe that passes by
printing nothing. For the gap it is the other way round: `grep -c -F -f` matches the
base's model first, with the same pattern file, and only then must the model at `HEAD`
print nothing. `test -s` on the base's model and on both pattern files keeps an empty file
from reading as "absent": an empty pattern file matches nothing, and the positive control
would fail. Each probe that must print nothing reads `grep`'s status: 1 is no match, and
2, a file or a pattern file that could not be read, stops the script. The property
pattern matches a definition (`val` or `def`, the name, then `=`,
`(` or `:`), not a mention in a comment, and not a part whose name only starts with the
guarantee's; if the model defines a property another way, correct the pattern and record
it under Departures. The mutants are red tests inside `just verify-quint`: each patched
copy must violate the property it names, and the instance's pass run shows that property
holding on the unchanged model at the same bound, or the recipe fails. The green tests are
`just check-quint`, the recipe's exit status, and the greps over its log. The listener
that `python` starts is the positive control for the port probe after it. The
`wait ... || true` is on a process the script has just killed, and no claim rests on it.
`git diff "$base" -- quint/verify.txt` prints for a reader; the hand-back says whether it
is empty or raises a bound. The last two loops rest on claim G: if the recipe prints other
names, correct the two patterns and record the correction under Departures. Step 4's red
runs are quoted in the hand-back and are not repeated here, because they need the model as
it stood before the rule existed.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1

### Checkpoint 2

### The six rows of the lease's end

### Patches re-cut

### Properties not mutation-tested

### Departures

### Verification

## Open points

- **The order across the two shed queues.** One lease's end can create superseded
  entries and dead letters. The model keeps two lists, so `HeldRecordsOldestFirst` is
  checked inside each and not across them. The Allium's entities carry no field that
  orders a `SupersededRecord` against a `DeadLetter`. The maintainer says at step 3
  whether that reading of the guarantee is the intended one; if not, it is a question
  about the Allium (shared rule 2), not a change to the model.
- **Five invariants with no mutant.** Proposed above as reported, not mutation-tested.
  The maintainer rules at step 3 whether five more runs are worth it.
- **The decay and "the entries a step added".** `EveryDropIsVisible` says no step
  removes an entry, and the definition of "added" reads a list's length. QM09's decay
  breaks both in a tick that ends a lease and decays an entry. QM09 changes the one
  definition and the one clause, and its plan says so.
- **The first sentence of `HeldRecordsOldestFirst`.** The model has no action that
  reads what the `Consuming` surface exposes, so the listing's order is carried by one
  definition. QM10's replay can compare the Rust's listing with it. The maintainer says
  at step 3 whether that is enough for this ticket to own the guarantee.
- **The bound.** The `OldestFirst` path needs nine steps and the deepest mutant eight;
  QM03's ticket proposes 10. If the base has less than nine, step 6 raises it and every
  later run gets slower. The maintainer rules at step 6 if the timeout no longer fits.
- **What the recipe prints** (claim G). If `scripts/verify-quint.sh` names no witness and
  no mutant in its output, Verification cannot show that this ticket's were run. That is
  a finding for QM03's owner, handed back at step 2, not fixed here.
