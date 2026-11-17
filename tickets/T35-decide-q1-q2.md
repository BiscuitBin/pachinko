---
id: T35
title: "Land the held set that lasts until the lease ends (Q1), the nack reset and no touch on stale (Q2), all decided"
status: done
depends_on: [T32]
parallel_with: []
branch: ticket/t35-decide-q1-q2
estimated_size: L
---

# T35: Land the held set that lasts until the lease ends (Q1), the nack reset and no touch on stale (Q2), all decided

## Context

T31's `code-review` run 1 raised two questions about behaviour the specification and
the library implemented, and T32 wrote both into `docs/specs/pachinko.allium` as open
questions 10 and 11:

- **Q1, the held-version shed (open question 10).** A supersede of a held record shed
  the version the consumer still held, and a later nack dead-lettered it again.
- **Q2, the stale revival (open question 11).** `DropStalePublish` touched the pocket,
  so a stale publish revived a parked pocket and reset its expiry count.

Q1 and Q2 are this ticket's names; they are not the module's questions 1 and 2.

**Q1 is decided.** The maintainer reviewed the four options this ticket first offered
on the step-through page (`https://claude.ai/artifact/3X1JB236GK4MRvj3f69UE2`) and, in an
elicitation on 2026-11-13, chose a design that is none of them. It also closes the
module's open question 9, which this ticket first excluded. The specification already
says it: commit `a338355` on this branch. **Q2 and the nack reset are decided too.**
T32's close-out decided on 2026-11-13 that a nack resets `consecutive_expiries` and that
the change lands here, and on 2026-11-16 the maintainer chose "No touch on stale" for
Q2. Both are in the specification at `983b785` on this branch.

This ticket now carries the rest: the tests, the library, the service and the handbook
follow the specification at `a338355` and `983b785`.

### The decision on Q1 (maintainer, 2026-11-13)

The answers, quoted from the elicitation of 2026-11-13. Each question offered labelled
options; the label the maintainer chose is quoted, with their own notes.

- "After c1 acks or nacks a held record and its lease is still active, does that record
  stay in c1's held set (what the lease read shows) until the lease ends?" Answer:
  "Stays until the lease ends, in this change". The maintainer's note: "worst case the
  pocket will hold two versions of every record (the held version and the highest rank
  version that has arrived since the key entered the held set -- only if a version
  higher rank than the held version arrives). Non-held records can still be
  superseded."
- "Scenario A: c1 holds key k at v1, v3 is published, c1 acks v1. Where is v1 recorded
  afterwards?" Answer: "Nowhere: acked means consumed (Recommended)".
- "Scenario C: c1 holds key k at v1, then v3 and v5 are published. v3 was never handed
  to anyone. When is v3 recorded as superseded?" Answer: "At once, when v5 arrives
  (Recommended)".
- "Key k is in c1's held set at v3 and the lease is active. Should a publish of k be
  judged against the kept record the same way whether v3 is unsettled, acked or
  nacked?" Answer: "Yes, the kept record's rank decides (Recommended)".
- "c1 has acked (or nacked) a record and it is still in the held set. c1 sends another
  ack or nack for the same record under the same lease. What happens?" Answer: "A repeat
  succeeds, a change is refused".
- "Your notes suggest calling the lease's hold a transaction. [...] How should the
  specification name this?" Answer: "Keep 'lease'; state the properties (Recommended)".
- The maintainer's later note, "Total fairness": "if c1 ACKs or NACKs k.v1 when the
  lease ends k.v3 is inserted into the pockets record queue at a position that reflects
  the order in which it arrived relative to the other records remaining in the pocket.
  if c1 does not ACK/NACK k.v1 then when the lease ends k.v3 is inserted into the
  pockets record queue at the position that k.v1 occupied."

The decisions, verbatim from the plan the maintainer approved on 2026-11-13; "you" and
"your" there address the maintainer:

1. **Held until the lease ends.** A record handed out under a lease stays in the
   consumer's held set until that lease ends. An ack or nack marks it; no held version
   is removed, shed or dead-lettered while the lease is active. The lease's end, by
   release or lapse, reconciles every held record. A lapse keeps the marks.
2. **An acked version is recorded nowhere.** Acked means consumed. The superseded queue
   holds only versions no consumer acked or nacked.
3. **A nacked version becomes a dead letter only**, never a superseded record, created
   when the lease ends.
4. **A waiting version that was never handed out is shed at once** when a higher rank
   arrives (scenario C, plain reading). Only the held version is protected.
5. **Publish is judged against the kept record alike** whether it is unsettled, acked or
   nacked: higher waits, equal is rejected (open question 1), lower is stale. This
   closes open question 9: a key settled under a lease is not pullable again in that
   lease.
6. **A repeated ack or nack succeeds and changes nothing; a change is refused.** An ack
   after a nack, or a nack after an ack, is rejected.
7. **Positions ("total fairness", your later note).** A higher rank that arrives behind
   a held version takes its place in the pocket's order at arrival. When the lease ends:
   - held version acked or nacked: the newer version is queued at its arrival position;
   - held version unsettled: the newer version is queued at the held version's position.

   This replaces two things: the "queued at the back" line in the walk-through you
   approved, and today's rule, where a deferred version takes its position when the held
   version is acked or nacked.
8. **No "transaction" term.** The vocabulary keeps Lease; two guarantees state the
   properties.

Decision 6 is read with the approved assumption below that a repeated ack or nack
extends the lease, so "changes nothing" means nothing but the lease's expiry.

Reconciliation when the lease ends:

| Held version | Newer version waiting | Held version goes to | Record afterwards |
| --- | --- | --- | --- |
| unsettled | no | nowhere; it is queued again | queued, same position |
| unsettled | yes | superseded queue | newer version queued, held version's position |
| acked | no | nowhere (consumed) | removed |
| acked | yes | nowhere (consumed) | newer version queued, its arrival position |
| nacked | no | dead letters | removed |
| nacked | yes | dead letters | newer version queued, its arrival position |

Approved with the plan, as assumptions the maintainer did not correct:

- A consumer that posts both an ack and a nack: the first stands and the other is
  refused (decision 6).
- Scenario C: with v1 held and v3 then v5 arriving, the waiting version keeps v3's
  arrival position.
- A repeated ack or nack extends the lease, as every accepted action does.
- A dead letter's `nacked_at` is when the nack was processed; its `expires_at` is the
  lease's end plus `dead_letter_ttl`. A superseded record made at the lease's end has
  `shed_at` at the lease's end.
- "Deferred" stays the word for a newer version waiting behind a held one; `deferred`
  stops being a record status.
- The lease read shows each held record's mark.
- The ack resets `consecutive_expiries` when it is processed (T32), and so does the
  nack (T32's close-out, below).

### The decisions on the nack reset and Q2

**The nack reset (T32's close-out, 2026-11-13).** From `tickets/T32-spec-follow-up.md`,
"Close-out, 2026-11-13", where the questions are abridged and the answers verbatim:

| # | Question | Answer |
| --- | --- | --- |
| 4 | Should a nack reset a pocket's `consecutive_expiries`? | "Yes: a nack resets too" |
| 5 | Where does the nack reset land, since it is a specification change and cannot land here? | "T35" |

The maintainer's note on decision 4, in full:

> Because the consecutive_expiries feature is mean to capture pockets that keep killing
> workers (unrecoverable bugs), And cordon them off to avoid unnecessary churn and
> turbulence, in a situation where we may or may not be able to attribute the issue to a
> specific record. NACKs, while they are a negative for an individual record, they are
> still a part of the natural and expected workflow/lifecycle. Individual records can be
> cracked without affecting the other records in the pocket.

The same close-out's decision 6 gives the `QueuedHasRecords` invariant to a ticket after
T35.

**Q2 and this ticket's scope (maintainer, 2026-11-16).** The questions, verbatim, with
the label the maintainer chose; the "(Recommended)" label the dialog put on one option
is dropped:

- "Q2, the stale revival (the module's open question 11): a stale (lower-rank) publish
  into a parked pocket currently revives it and resets consecutive_expiries although no
  record changed. Which do you choose?" Answer: "No touch on stale".
- "T35 is taking on the nack reset (a nack resets the pocket's consecutive_expiries, as
  T32's close-out decided). Should this session also land the specification change now,
  as a338355 did for Q1, or only rewrite the T35 ticket so its own step 4 lands it?"
  Answer: "Land spec now + ticket".

The maintainer chose none of the other open items for that session: the step-1 wire
answers, the order of entries at a lease's end, the emptied-pocket pull test, the
decision record and the qsketch lines stay where they were.

### What landed at `a338355`

The specification, by rule name (the executor locates each by name, never by line):

| Area | At `a338355` |
| --- | --- |
| `Record` | `status: queued \| held`; while held: `held_rank`, `held_payload`, `mark` (`unsettled`, `acked`, `nacked`), `nacked_at`, `deferred_position`. Five new entity invariants pin the last two to the mark and to `held_rank < rank` |
| `Pocket` | `deferred_records` is gone; `ending_records` is the records a lease's end removes (held, acked or nacked, nothing deferred behind them) |
| Publish | `Supersede` sheds its loser only when no consumer holds that version (queued, or a version already deferred). `DeferBehindHeldVersion` is new: the loser is the held version, nothing is shed, the newer version is given an arrival position. `DropStalePublish` and the revive rules are unchanged |
| Pull | `HoldRecord` sets `mark = unsettled` |
| Ack and nack | `AckMarksRecord`, `AckRepeats`, `NackMarksRecord`, `NackRepeats` replace the four settle rules. No rule covers a changed mark |
| Release and lapse | The five rules that end a lease emit `Freed(lease, record_key, ended_at)` for every held record and choose queued or idle by `records.count` against `ending_records.count` |
| Lease end | Six rules answer `Freed`, one per row of the table above: `RequeueUnsettledRecord`, `SupersedeUnsettledHeldVersion`, `ConsumeAckedRecord`, `ConsumeAckedVersionQueueDeferred`, `DeadLetterNackedRecord`, `DeadLetterNackedVersionQueueDeferred` |
| Invariants | `OutstandingOnlyWhileLeased` names `held` only; `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` are new |
| Guarantees | `EveryDropIsVisible`, `OldestSlotNewestPayload`, `OneVisitPerKeyPerLease`, `NothingLostWhileHeld`, `ExpiryRecoversWithinTtl` reworded; `HeldSetStableUntilLeaseEnds` and `MarksStandHoweverLeaseEnds` are new; `exposes` gains `record.mark` |
| Open questions | 9 and 10 removed; eight remain, the eighth line being question 11 |

`just plan-spec pachinko` printed 190 obligations before the edit and 236 after: 35 ids
removed and 81 added. The removed ids are the four settle rules' and the two requeue
rules' success, failure and entity-creation ids, `projection.Pocket.deferred_records`
and the two `deferred` transition edges. The added ids are the eleven new rules'
success and failure ids, three entity-creation ids
(`SupersedeUnsettledHeldVersion.1`, `DeadLetterNackedRecord.1`,
`DeadLetterNackedVersionQueueDeferred.1`), `rule-failure.Supersede.4`,
`projection.Pocket.ending_records`, eight invariant ids, two `entity-optional` ids,
three `when-presence` ids and three `when-set.HoldRecord` ids. The executor reruns
`just plan-spec pachinko` and cites only its output.

The same commit drops questions 9 and 10 from the lists in
`docs/how-to/work-with-the-specs.md` and `docs/explanation/specifications.md`.

**The tree is inconsistent on purpose until this ticket's code step.** At `a338355` the
library, its tests, the service, loadgen and every behaviour page still follow the old
rules, 47 `// obligation:` tags name ids that no longer exist, and S045, S071 and S072
still carry `oq:10` and `oq:9`. `just check` passes there because no gate compares the
tags or the tests with the specification. Invariant 1 of `AGENTS.md` makes that
difference a defect of the code, and this ticket removes it.

### What landed at `983b785`

The specification, by rule name, in the commit
`983b78554dff6d6d6e2cb78b3fd7f4a9a6de9072`:

| Area | At `983b785` |
| --- | --- |
| Ack and nack | `NackMarksRecord` gains `ensures: pocket.consecutive_expiries = 0`, as `AckMarksRecord` has; `NackRepeats` gains nothing, as `AckRepeats` has nothing. The comment above `AckMarksRecord` says an ack or a nack is progress, gives the maintainer's reason in one clause, and says a renew and a pull leave the count |
| Lease expiry | The comment above `LeaseExpiryIdlesEmptyPocket` says every record of such a pocket was marked under the lease, so its count is 0 before the lapse and it parks only when `max_consecutive_expiries` is 1 |
| Publish | `DropStalePublish` no longer emits `PocketTouched`. The comment above `PublishNewKey` names the three rules that touch a pocket (`PublishNewKey`, `Supersede`, `DeferBehindHeldVersion`) and says a stale publish leaves a parked pocket parked, closing question 11. That also replaces "Every publish into an existing pocket emits PocketTouched", T32-1's handed-back point |
| Open questions | 11 removed; seven remain, questions 1 to 7, and the numbering comment says so |

`just plan-spec pachinko` printed 236 obligations before and after, with the same ids.
Its JSON differs only in `dependencies`: the five `DropStalePublish` obligations lose
`PocketTouched` from `trigger_emissions`, and the six `NackMarksRecord` obligations gain
`Pocket` in `entities_written`. The same commit drops question 11 from the lists in
`docs/how-to/work-with-the-specs.md` and `docs/explanation/specifications.md`.

As at `a338355`, the library, its tests, the service and the behaviour pages still
follow the old rules there: a nack leaves the count, and a stale publish revives a
parked pocket. No gate compares them with the specification, so `just check` passes.

### Q2: the decision

The maintainer chose "No touch on stale" on 2026-11-16 (above). The two options as they
were offered:

- **Before, at `a338355`.** A stale publish revives a parked pocket; the specification
  says so.
- **No touch on stale, chosen.** `DropStalePublish` loses `ensures: PocketTouched`, so a
  stale resend is recorded as `stale` and changes nothing else, as an equal-rank publish
  already changes nothing. The comment above `PublishNewKey` ("Every publish into an
  existing pocket emits PocketTouched") changes; `ReviveParkedPocket`,
  `ReviveIdlePocket` and the module's question 7 do not.

Pocket `a` parked after three consecutive lapses, its record `a1` at rank 1. A publish
into it:

| Publish into parked `a` | Rule | Before | No touch on stale (decided) |
| --- | --- | --- | --- |
| A new key | `PublishNewKey` | revives | revives |
| A higher rank | `Supersede` | revives | revives |
| A lower rank (stale) | `DropStalePublish` | revives | stays parked |
| An equal rank | none (rejected) | stays parked | stays parked |

The `SupersededRecord` with `reason: stale` is created under both. Under the Q1
decision and the nack reset a pocket can also park with no records, but only at
`max_consecutive_expiries` 1: every record was acked or nacked under the lease, so the
count is 0 before the lapse. A stale publish needs a stored record, so such a pocket is
revived only by `PublishNewKey`, under either option.

Verified at `62efe41` and unchanged at `a338355`: `DropStalePublish` requires
`rank < record.rank`, sheds the arrival as `reason: stale` and emits `PocketTouched`;
`ReviveParkedPocket` answers it; `Pockets::publish`
(`crates/libpachinko/src/queue/pockets.rs:82-148`) calls `touch` on every branch that
does not return an error. S024, `a_supersede_or_a_stale_publish_revives_a_parked_pocket`
(`crates/libpachinko/tests/queue/expiry.rs`), pins it and carries `oq:11`; S025 pins
that an equal rank does not revive. At `983b785` `DropStalePublish` no longer emits
`PocketTouched`; the code and S024 change at steps 5 and 6.

The maintainer's xzibitq repository holds `qsketch.py`, a reference sketch outside this
repository. As read there on 2026-11-07, `MultiLeaseQueue.push` (lines 533-545) takes a
waiting pocket back into circulation before the rank check runs, so any publish revives
it, a stale one included. That is a fact about the sketch, not a recommendation. At
step 1 the maintainer pointed at the file, and it was read again on 2026-11-16: `push`
pops the pocket from `_q_in_waiting` before `KeyedQueue.push` runs its rank check, so
the lines above are right. On Q1 the sketch behaves as the old rules did; the decision
above replaces it.

### The impact map

Surveyed read-only on 2026-11-13 against the specification's edit (the lines of
`docs/specs/pachinko.allium` and of the two handbook pages below were read just before
`a338355` and have moved a little; find rules by name). Code, test and handbook lines
are as of `62efe41`, which `a338355` does not change. The merge of T32's pull request 48
at `b5d4a83` later changed seven of the files cited (step 2's hand-back), so their lines
have moved too. Section 8's lines are as of `8f15563`, T32's head, where T32's ticket
records them. Every line is re-read at step 3 before it is relied on.

Abbreviations: `LQ/` is `crates/libpachinko/tests/queue/`, `LS/` is
`crates/libpachinko/src/`, `PT/` is `crates/pachinko/tests/`, `PS/` is
`crates/pachinko/src/`.

#### 1. Obligation tags

**1a. Tags naming REMOVED ids (47 tags over all 35 removed ids; every removed id has at least one tag).**

| Id | Count | Tags: `path:line` / test fn / scenario |
| --- | --- | --- |
| projection.Pocket.deferred_records | 1 | LQ/settle.rs:109 / `a_supersede_while_the_key_is_held_defers_it_on_ack` / S043 |
| rule-entity-creation.NackDefersNewerVersion.1 | 1 | LQ/settle.rs:260 / `a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer` / none |
| rule-entity-creation.NackSettlesRecord.1 | 1 | LQ/settle.rs:218 / `a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery` / S047 |
| rule-failure.AckDefersNewerVersion.1 to .4 | 1 each | LQ/settle.rs:69, :70, :71, :72 / `rejected_acks_change_nothing_not_even_the_leases_expiry` / S042 |
| rule-failure.AckDefersNewerVersion.5 (was `held_rank < rank`) | 1 | LQ/settle.rs:26 / `an_ack_settles_a_held_record` / S040 |
| rule-failure.AckSettlesRecord.1 | 2 | LQ/settle.rs:65 (S042); LQ/lease.rs:264 / `a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease` / S068 |
| rule-failure.AckSettlesRecord.2 | 2 | LQ/settle.rs:66 (S042); LQ/lease.rs:291 / `a_consumer_cannot_act_on_another_consumers_lease` / S070 |
| rule-failure.AckSettlesRecord.3 / .4 | 1 each | LQ/settle.rs:67 / :68 (S042) |
| rule-failure.AckSettlesRecord.5 (was `held_rank = rank`) | 1 | LQ/settle.rs:107 / `a_supersede_while_the_key_is_held_defers_it_on_ack` / S043 |
| rule-failure.NackDefersNewerVersion.1 to .4 | 1 each | LQ/settle.rs:295, :296, :297, :298 / `rejected_nacks_change_nothing_and_create_no_dead_letter` / S049 |
| rule-failure.NackDefersNewerVersion.5 | 1 | LQ/settle.rs:219 / S047 test |
| rule-failure.NackSettlesRecord.1 | 1 | LQ/settle.rs:291 (S049) |
| rule-failure.NackSettlesRecord.2 | 2 | LQ/settle.rs:292 (S049); LQ/lease.rs:292 (S070) |
| rule-failure.NackSettlesRecord.3 / .4 | 1 each | LQ/settle.rs:293 / :294 (S049) |
| rule-failure.NackSettlesRecord.5 | 1 | LQ/settle.rs:261 / nack-of-superseded test / none |
| rule-failure.RequeueDeferredRecord.1 / .2 | 1 each | LQ/invariants.rs:355 / :356 / `every_invariant_holds_after_every_step_of_any_walk` / none |
| rule-failure.RequeueHeldRecord.1 | 1 | LQ/invariants.rs:354 / walk test |
| rule-failure.RequeueHeldRecord.2 | 1 | LQ/lease.rs:196 / `a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it` / S054 |
| rule-success.AckDefersNewerVersion | 2 | LQ/settle.rs:106 (S043); LQ/expiry.rs:419 / `an_ack_resets_the_consecutive_expiry_count` / S077 |
| rule-success.AckSettlesRecord | 5 | LQ/settle.rs:25 (S040); LQ/settle.rs:42 `an_ack_extends_the_lease` (S041); LQ/expiry.rs:418 (S077); LQ/expiry.rs:591 `an_ack_after_expires_at_but_before_the_lapse_succeeds` (S067); PT/http.rs:392, driver `ack_settles`, test `an_ack_settles_a_held_record_over_http` (S040 wire) |
| rule-success.NackDefersNewerVersion | 1 | LQ/settle.rs:259 / nack-of-superseded test / none |
| rule-success.NackSettlesRecord | 2 | LQ/settle.rs:217 (S047); PT/http.rs:414, driver `nack_dead_letters`, test `a_nack_does_not_redeliver_over_http` (S047 wire) |
| rule-success.RequeueDeferredRecord | 3 | LQ/settle.rs:133 `a_deferred_record_restarts_at_the_back_of_its_pocket` (S044); LQ/lease.rs:195 (S054); LQ/expiry.rs:366 `a_supersede_while_held_is_deferred_on_nack_and_the_lapse_frees_it` (S048) |
| rule-success.RequeueHeldRecord | 2 | LQ/lease.rs:147 `a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place` (S053); LQ/expiry.rs:168 `a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records` (S060) |
| transition-edge.Record.deferred.queued | 1 | LQ/settle.rs:134 (S044) |
| transition-edge.Record.held.deferred | 1 | LQ/settle.rs:108 (S043) |

The prose in LQ/invariants.rs:17-36 (module doc) argues that `RequeueHeldRecord.1`, `RequeueDeferredRecord.1` and `RequeueDeferredRecord.2` are unreachable or unobservable. It has to be rewritten for the six `Freed` rules' `.1` (`exists record`) guards.

**1b. Tags on ids that still exist but whose rule or meaning changed.**

The numbered failure clauses were checked by span. `Supersede.1-3` keep their clauses, and `Supersede.4` is new: `record.status = queued or record.held_rank < record.rank`. `ReleaseRequeuesPocket.4` and `ReleaseIdlesEmptyPocket.4`, `LeaseExpiryKeepsPocketAtFront.5` and `LeaseExpiryIdlesEmptyPocket.5` changed from `records.count > 0` / `= 0` to a comparison against `ending_records.count`. `LeaseExpiryParksPocket.4` is unchanged. The new ack ids take the old numbering for `.1-.4` (active, consumer, exists record, status = held); only `.5` differs (`mark = unsettled` / `mark = acked`).

| Id | Count | Tags (path:line / fn / scenario) | What changed |
| --- | --- | --- | --- |
| rule-success.Supersede | 4 | LQ/publish.rs:94 S014; LQ/settle.rs:161 S045; PT/batch_publish.rs:224 `per_row_outcomes` (test `a_batch_answers_one_outcome_per_row_in_order_and_applies_rows_independently`); PT/http.rs:189 `large_ranks` S019 | A new guard: no longer taken for a held key whose held version is current. S045's tag is now wrong: the first publish onto the held key is `DeferBehindHeldVersion` |
| rule-entity-creation.Supersede.1 | 1 | LQ/publish.rs:95 S014 | Unchanged where the key is queued |
| rule-failure.Supersede.1 / .2 | 1 each | LQ/publish.rs:358, :368 (no scenario) | Clause unchanged |
| rule-failure.Supersede.3 | 2 | LQ/publish.rs:171 S016; LQ/expiry.rs:296 S025 | Clause unchanged |
| rule-failure.Supersede.4 | 0 | none | New; needs a tag |
| rule-success.ReleaseRequeuesPocket | 4 (lib 3 + wire 1) | LQ/lease.rs:146 S053; LQ/lease.rs:194 S054; LQ/expiry.rs:461 S056; PT/http.rs:458 `release_requeues` / `a_release_requeues_or_idles_the_pocket_over_http` | Guard now `records > ending_records`; emits `Freed(lease, record_key, ended_at)` for held records only |
| rule-success.ReleaseIdlesEmptyPocket | 2 | LQ/lease.rs:214 S055; PT/http.rs:459 `release_requeues` | Guard now `records = ending_records`; also emits Freed |
| rule-failure.ReleaseRequeuesPocket.1 | 2 | LQ/lease.rs:248 S057; LQ/lease.rs:266 S068 | Same clause |
| rule-failure.ReleaseRequeuesPocket.2 / .3 | 1 each | LQ/lease.rs:294 S070; LQ/invariants.rs:348 walk | Same |
| rule-failure.ReleaseRequeuesPocket.4 | 1 | LQ/lease.rs:215 `a_release_idles_an_empty_pocket` S055 | New clause; the test (ack all, then release) still makes it fail |
| rule-failure.ReleaseIdlesEmptyPocket.1/.2/.3 | 1 each | LQ/lease.rs:249 S057; :295 S070; LQ/invariants.rs:349 | Same |
| rule-failure.ReleaseIdlesEmptyPocket.4 | 1 | LQ/lease.rs:148 S053 | New clause: a1 acked plus a2 queued gives 2 > 1, so it still fails |
| rule-success.LeaseExpiryKeepsPocketAtFront | 1 | LQ/expiry.rs:165 S060 | Guard and Freed changed |
| rule-success.LeaseExpiryIdlesEmptyPocket | 1 | LQ/expiry.rs:204 S061 | Guard changed; S061 (ack both, then lapse) still idles |
| rule-success.LeaseExpiryParksPocket | 1 | LQ/expiry.rs:227 S062 | Freed now reconciles marks and dead-letters nacked records at parking |
| rule-failure.LeaseExpiryKeepsPocketAtFront.1-.5 | 1 each | .1 LQ/expiry.rs:131 S065; .2 :100 S059; .3 LQ/invariants.rs:351; .4 LQ/expiry.rs:228 S062; .5 LQ/expiry.rs:205 S061 | .5 is a new clause |
| rule-failure.LeaseExpiryIdlesEmptyPocket.1-.5 | 1 each | .1 LQ/expiry.rs:132; .2 :102; .3 LQ/invariants.rs:352; .4 LQ/expiry.rs:487 S063; .5 LQ/expiry.rs:167 S060 | .5 is a new clause |
| rule-failure.LeaseExpiryParksPocket.1-.4 | 1 each | .1 LQ/expiry.rs:130; .2 :101; .3 LQ/invariants.rs:350; .4 LQ/expiry.rs:166 | Clauses unchanged |
| when-set.{ReleaseRequeuesPocket,ReleaseIdlesEmptyPocket,LeaseExpiry*}.Lease.ended_at | 5 | LQ/lease.rs:152, :217; LQ/expiry.rs:104, :206, :230 | Ended-at is unchanged; the tests' fixtures use acks (see section 2) |
| guarantee.EveryDropIsVisible | 3 | LQ/publish.rs:99 S014, :135 S015, :172 S016 | Text rewritten: acked versions consumed, nacked versions dead-lettered at lease end, held versions shed at lease end. No test covers the new branches |
| guarantee.OldestSlotNewestPayload | 1 | LQ/publish.rs:134 S015 | Adds the arrival-position rule for a version deferred behind an acked or nacked held version |
| guarantee.OneVisitPerKeyPerLease | 4 | LQ/settle.rs:135 S044; LQ/settle.rs:324 S071; LQ/settle.rs:344 S072; LQ/invariants.rs:360 walk | Now holds for the key itself; S071/S072 assert the opposite |
| guarantee.NothingLostWhileHeld | 2 | LQ/settle.rs:110 S043; LQ/settle.rs:162 S045 | Reworded ("whatever the mark") |
| guarantee.ExpiryRecoversWithinTtl | 3 | LQ/expiry.rs:169 S060; LQ/expiry.rs:564 S066; PT/http.rs:618 `lapse_over_http` / `a_lease_nobody_renews_lapses_and_its_record_goes_to_another_consumer` | Now "unsettled records and deferred versions are queued" |
| guarantee.HeldSetStableUntilLeaseEnds, guarantee.MarksStandHoweverLeaseEnds | 0 | none | New guarantees; the checker gives them no ids (LQ/main.rs:11-12 convention) |
| invariant.OutstandingOnlyWhileLeased | 2 | LQ/invariants.rs:333; LQ/pull.rs:287 `held_records_exist_only_in_leased_pockets` | Now `status = held` only; `{held, deferred}` is gone |
| invariant.Record.HeldVersionNotNewer | 2 | LQ/invariants.rs:322; LQ/pull.rs:277 `a_held_version_is_never_newer_than_the_record` | Same text; the record shape changes |
| invariant.UniqueRecordPosition / RecordPositionsBelowNext | 3 each | LQ/invariants.rs:335/:337; LQ/expiry.rs:722/:724 S075; LQ/publish.rs:387/:388 | S075 breaks (section 2) |
| invariant.DeadLetter.NackedBeforeExpiry | 2 | LQ/settle.rs:221 S047; LQ/invariants.rs:324 | Still true, but the walk's witness changes (section 3) |
| invariant.DeadLetterTtlNonNegative | 3 | LQ/invariants.rs:342; LS/config.rs:36, :59 (S005 doc) | The walk's check, `expires_at == nacked_at + ttl` (LQ/invariants.rs:80-86), becomes false |
| entity-fields.Record | 1 | LQ/publish.rs:22 S011 | Gains `mark`, `nacked_at`, `deferred_position` |
| entity-fields.DeadLetter | 1 | LQ/settle.rs:220 S047 | Fields the same; `nacked_at` and `expires_at` mean other things now |
| rule-success.DeadLetterDecays / temporal.DeadLetterDecays | 2 / 1 | LQ/decay.rs:18 S007, :73 S074; :74 S074 | The dead letter exists only after the lease ends |
| rule-success.HoldRecord, when-set.HoldRecord.Record.{held_rank,held_payload}, when-presence.Record.{held_rank,held_payload}, transition-edge.Record.queued.held | 1 each | LQ/pull.rs:23, :27, :28, :29, :30, :26 (all `a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch` S027) | HoldRecord now also sets mark, nacked_at and deferred_position. The test's exact `RecordStatus::Held { rank, payload }` at LQ/pull.rs:50-56 changes shape |
| transition-edge.Record.held.queued | 1 | LQ/lease.rs:150 S053 | Now reached by four of the six Freed rules |
| transition-rejected.Record.status | 1 | LQ/invariants.rs:344 | RECORD_EDGES changes (section 3) |
| projection.Pocket.held_records | 2 | LQ/pull.rs:31 S027; LQ/lease.rs:333 S076 | Now includes acked and nacked records |
| projection.Pocket.queued_records / active_leases | 1 / 1 | LQ/publish.rs:49 S012 / LQ/pull.rs:35 S027 | Unaffected |
| surface-exposure.Consuming | 2 | LQ/lease.rs:332 S076; LQ/pull.rs:160 S036 | `exposes` gains `record.mark`; neither test checks it |
| surface-provides.Consuming | 2 | LQ/invariants.rs:357; PT/batch_publish.rs:489 `lease_body_limit` | Provides list unchanged |
| surface-actor.Consuming | 1 | LQ/identity.rs:148 | Unaffected |

The 81 ADDED ids have no tag in `crates/` (none exist yet). That includes `rule-failure.Supersede.4`, the two `entity-optional`, eight `invariant`, `projection.Pocket.ending_records`, the three `rule-entity-creation`, the eleven new rules' success and failure ids, three `when-presence` and three `when-set.HoldRecord.*`.

#### 2. Scenarios

The catalog is in `docs/reference/testing.md:169-257`. **The last id is S077 (`testing.md:257`), so the next free id is S078.** `testing.md:171` says "77 scenarios", while `CHANGELOG.md:54` says "76 scenarios"; that mismatch predates this ticket.

Each row below gives the catalog line, the test at `path:line` of its fn, what the test asserts today, and what changes under the decision.

| Id | Catalog | Test | Asserts today | Under the decision |
| --- | --- | --- | --- | --- |
| S040 core, wire | testing.md:220 | LQ/settle.rs:28; wire PT/http.rs:393 `ack_settles`, test :409 | `Ok(Settled)`; the pocket has only `other` left (:33); the lease view has 0 records (:35); no shed. Wire: `{"outcome":"settled"}` (:399), then lease `records == []` (:402), then a second ack gives 404 `record_not_held` (:403-405) | The record stays held and acked until the lease ends. The pocket has 2 records and the lease view has 1, marked acked. The outcome name is open (section 5). Over the wire the second ack becomes 200 (`AckRepeats`), not `record_not_held` |
| S041 | testing.md:221 | LQ/settle.rs:44 | The ack extends the lease | Unchanged except the outcome type |
| S042 edge | testing.md:222 | LQ/settle.rs:74 | Refusals; then ack k and a second ack, `RecordNotHeld` (:100-102) | The second ack succeeds and extends the lease. Missing: ack after nack refused (new refusal, section 5) |
| S043 edge | testing.md:223 | LQ/settle.rs:112 | Supersede while held gives `Superseded`; ack gives `Deferred`; status `Deferred`; fresh position == next (:118-127); lease view 0 records | Publish goes through `DeferBehindHeldVersion`, taking `deferred_position = next` at publish, not at ack. Ack only marks; status stays Held; lease view shows k at v1, acked. To rewrite whole |
| S044 edge | testing.md:224 | LQ/settle.rs:137 | k deferred behind held; `between` published after; after the ack `k.position > between` (:143); after release, pull order `["between","k"]` (:155) | Decision 7: k's arrival position is taken at the publish, before `between`, so after release (acked) k is queued at the earlier position. The order flips to `["k","between"]`, and :143 fails, since the position is unchanged until the lease ends. The "restarts at the back" title is wrong |
| S045 edge, oq:10 | testing.md:225 | LQ/settle.rs:164 | Two supersedes while held: shed `[(1,Superseded),(3,Superseded)]` (:168-177); held v1; ack gives `Deferred` | v1 is not shed; v3 is deferred and then shed at once by `Supersede` (held_rank < rank), so shed is `[(3,Superseded)]`. The ack marks only; at release (acked) v1 is consumed and is never shed. Retag: drop `oq:10` |
| S046 edge, oq:1 | testing.md:226 | LQ/settle.rs:190 | A stale or equal check against rank 3, not the held 1; status `Held{1,v1}` exact (:207-213) | Behaviour the same (`DeferBehindHeldVersion` sets rank 3); the exact `RecordStatus::Held` literal changes shape |
| S047 core, wire | testing.md:227 | LQ/settle.rs:223; wire PT/http.rs:415 `nack_dead_letters`, test :429 | `Ok(Settled)`; one dead letter at once with `nacked_at = now`, `expires_at = now + ttl` (:233-247); pocket has 0 records (:248); no redelivery | No dead letter until the lease ends; the pocket still has the record (held, nacked); no redelivery still holds. The dead letter appears at release or lapse with `nacked_at` = nack time and `expires_at` = lease end + ttl. Wire: outcome string |
| (untagged) | none | LQ/settle.rs:263 `a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer` | `Deferred`; dead letter at once; status Deferred at the fresh position | To rewrite: nack marks; at lease end `DeadLetterNackedVersionQueueDeferred` |
| S048 edge | testing.md:228 | LQ/expiry.rs:368 | Nack gives `Deferred`; status Deferred; the lapse requeues at the "deferred position" read after the nack (:374-381); pull `["a2","a1"]` | Status stays Held after the nack, and `position` is still a1's original. After the lapse a1 is queued at `deferred_position` (taken at the publish), so :377-381 fail. The pull order `["a2","a1"]` still holds. A dead letter is created at the lapse |
| S049 edge | testing.md:229 | LQ/settle.rs:300 | Nack refusals; no dead letter | Same; add nack-after-ack and nack-repeat cases |
| S053 core, wire | testing.md:233 | LQ/lease.rs:155; wire PT/http.rs:460 `release_requeues`, test :483 | ack a1, release: Queued at the back, a2 in place | Outcome the same (a1 acked counts in `ending_records`); a1 is now removed at release, not at ack. Wire unchanged (status codes and `pocket` only) |
| S054 edge | testing.md:234 | LQ/lease.rs:198 | Supersede while held, ack gives `Deferred`, release gives Queued, rank 2 | `ConsumeAckedVersionQueueDeferred`; same fate. The outcome type changes; "only deferred records" now means "only an acked record with a deferred version" |
| S055 core | testing.md:235 | LQ/lease.rs:220 | Ack both, release gives Idle | Same result through `ending_records` |
| S060 core | testing.md:240 | LQ/expiry.rs:171 | Lapse requeues unsettled records in place | Same (`RequeueUnsettledRecord`) |
| S061 edge | testing.md:241 | LQ/expiry.rs:208 | Ack both, lapse gives Idle | Same result; the records are removed at the lapse |
| S063 edge | testing.md:243 | LQ/expiry.rs:489 | `once.ack == Ok(Settled)` (:502); parked with 0 records; nacked pocket parked with 0 | The outcome type changes; nacked records become dead letters at the parking lapse (not asserted). The nack reset flips the nacked case (section 8) |
| S064 edge | testing.md:244 | LQ/expiry.rs:400 | `nack == Ok(Settled)` (:406) | The outcome type changes, and the nack reset inverts "after the nack" (section 8) |
| S067 decision:0019 | testing.md:247 | LQ/expiry.rs:593 | `second.ack == Ok(Settled)` (:596) | Outcome type only |
| S068 edge, wire | testing.md:248 | LQ/lease.rs:268 (:285-286 `Ok(Settled)`); wire PT/http.rs:489 `old_lease_fenced` (:509 `"settled"`), test :514 | Fencing | Outcome name only |
| S071 oq:9 | testing.md:251 | LQ/settle.rs:326 | Ack, then publish k@2 gives `NewKey`; record Queued; pulled again in the same lease | Inverted: `DeferBehindHeldVersion`; record Held (acked) with the version deferred; the pull returns nothing; after release k@2 is queued at its arrival position. Retag: drop `oq:9`; rename |
| S072 oq:9 | testing.md:252 | LQ/settle.rs:346 | Nack, then publish k@2 gives `NewKey`; pulled again; `dead_letters.len() == 1` (:357) | Inverted, as S071; the dead letter count is 0 until the lease ends |
| S073 core | testing.md:253 | LQ/decay.rs:47 | Superseded decay, no lease | Unaffected |
| S074 core | testing.md:254 | LQ/decay.rs:76 | Dead letters exist right after the nacks; decay at `nacked_at + ttl` (:85-100) | No dead letter until release or lapse; decay is at **lease end + ttl** (the specification's Lease end comment). The title "decay at nacked_at plus dead_letter_ttl" is wrong |
| S075 edge | testing.md:255 | LQ/expiry.rs:726 | Ack a1, then `publish("a","a1",1,"again").unwrap()` (:731-732); pull within the lease gives `["a2","a1"]` (:734) | **Breaks:** a1 is still held at rank 1, so the publish is `EqualRank` and the `unwrap()` panics. This is open-question-9 behaviour with no `oq` tag. Also, `publish a2@2` onto held a2 (:735) becomes a deferral |
| S076 core | testing.md:256 | LQ/lease.rs:335 | The view shows the held version v1, not v3 | Must also show `mark`; the publish of v3 no longer sheds v1 |
| S077 edge | testing.md:257 | LQ/expiry.rs:422 | `ack == Ok(Settled)` (:433); publish a2@2 gives `Superseded` (:440-443); ack gives `Deferred` resetting the count (:444-449) | Count behaviour the same (`AckMarksRecord` resets). The outcome types change; the publish outcome for `DeferBehindHeldVersion` is open |
| S007 edge | testing.md:187 | LQ/decay.rs:20 | `nack == Ok(Settled)`; `dead_letters[0].expires_at == T0` right after (:29-32) | The dead letter does not exist until the lease ends: `dead_letters[0]` panics. It needs a release or lapse first |
| S021 core | testing.md:201 | LQ/publish.rs:293 | Publish a@2 onto held a gives `Ok(Superseded)` (:303-306) | The publish outcome for `DeferBehindHeldVersion` is open (section 5) |
| S037 core | testing.md:217 | LQ/pull.rs:186 | Publish r0@2 onto held, not handed again; view shows rank 1 | Behaviour holds (defer); the `.unwrap()` publish is fine |
| S023, S034, S051, S056 | testing.md:203, :214, :231, :236 | LQ/lease.rs:41; LQ/expiry.rs:346; LQ/lease.rs:116; LQ/expiry.rs:463 | Use `ack(...).unwrap()` then release or renew | Behaviour unchanged |
| S027 | testing.md:207 | LQ/pull.rs:37 | Exact `RecordStatus::Held{rank,payload}` (:50-56) | Shape change (mark, nacked_at, deferred_position) |

Tests with no scenario tag that pin affected behaviour:

- LQ/expiry.rs:640 `a_backward_clock_step_never_lapses_a_lease_early_nor_stamps_backward`. It publishes a1@2 onto held a1, then reads `superseded[0].shed_at` (:648-649). Nothing is shed now, so the index panics. It needs a queued key, or the stamp read at the lease end.
- LQ/expiry.rs:659 `a_tick_examines_only_what_is_due`. It pins the store-step name `"free_held"` (:706-710) and expects 3 calls. Its 500 supersedes are on a queued pocket (:681-683), so they still shed.
- LQ/settle.rs:263, above.

**`oq:9` / `oq:10` tags and labels (all found):**

- `docs/reference/testing.md:225` (S045 `oq:10`), `:251` and `:252` (S071 and S072 `oq:9`).
- `LQ/settle.rs:2-3` (module doc "the two open-question-9 scenarios"), `:159-160` (S045 comment "open question 10…"), `:326` and `:346` (fn names `oq9_…`), `:327-329` and `:347` (comments "OQ9, as the rules stand").
- `LQ/invariants.rs:261-263` (check_batch doc "open question 9, as the rules stand").
- `docs/how-to/work-with-the-specs.md:77-78` ("The scenarios tagged `oq:9` and `oq:10` keep those tags until their tests follow the specification"; written at `a338355`, removed at step 7).

The tag legend is at `docs/reference/testing.md:173-176`. No other `oq:9` or `oq:10` exists in `crates/`, `loadgen/`, `docs/` (outside the spec), README or CHANGELOG.

#### 3. The walk model and the invariant tests

`LQ/walk.rs` (the model) and `LQ/invariants.rs` (the checks).

| What | Where | Today | Must change |
| --- | --- | --- | --- |
| Publish/supersede model | walk.rs:336-380 (`Model::publish`), :360-365 (always pushes the loser `record.rank/payload` as Superseded when `rank > record.rank`), :418-429 (`supersede`: rank and payload in place, held untouched) | Sheds the held version (OQ10 double shed) | Split into `Supersede` (queued, or `held_rank < rank`: shed the current or deferred loser, keep `deferred_position`) and `DeferBehindHeldVersion` (held with `held_rank = rank`: no shed, take `next_position` as `deferred_position`). Equal or stale against an acked or nacked record (now still present) |
| Settle model | walk.rs:568-613 (`settle`): guard `RecordStatus::Held` else `RecordNotHeld` (:577-588); extend; nack pushes ModelLetter at `now` (:591-600); ack resets the count (:601-603); `held_rank == rank` removes (:604-607); else takes a fresh position, status `Deferred` (:608-612) | Old four rules | Marks: unsettled to acked or nacked (`nacked_at = now`); a repeat succeeds with lease extend only; a conflicting mark is refused (variant TBD); no removal, no letter |
| Settle driver | walk.rs:925-972; `handed` removal on `Settled` (:962-968) | A key leaves `handed` when settled (OQ9) | Remove: a key never leaves `handed` during a lease. `turn.settled` (single `Option`, walk.rs:748/762) becomes the set of records a lease end removed |
| Release model | walk.rs:635-659 (`release`): `free(pocket)` then `records.is_empty()` gives Idle | Requeues everything | Six-way reconciliation per held record; the guard `records > ending_records`; creates SupersededRecord (shed_at = now) and DeadLetter (`nacked_at` = stored, `expires_at = now + dead_letter_ttl`, `nacked_by` = lease consumer); positions per decision 7 |
| Lapse model | walk.rs:685-707 (`lapse`): `at_threshold` from the pre-count, `free`, then Parked / Idle (empty) / Queued | Same | Same reconciliation, then the fate |
| `free` | walk.rs:709-716: all records become Queued, no position change | `RequeueHeldRecord`/`RequeueDeferredRecord` | Replace with the six Freed rules |
| Tick | walk.rs:661-683: lapses, then decay in the same `now` | Same | Dead letters and superseded records created by a lapse can decay in the same tick when the TTL is 0 (the model must predict it) |
| Model records | walk.rs:170-176 (`ModelPocket.records: BTreeMap<RecordKey, RecordView>`); :178-200 (`ModelShed`, `ModelLetter`); :221-222 (`Vec`s compared in snapshot order) | - | The order of several shed entries made at one lease end must match the implementation's (spec silent: section 7) |
| Status labels | walk.rs:1140-1155 `note_states` (`RecordStatus::Deferred => "Deferred"`, :1146) | - | Remove Deferred; consider labels per mark |
| Invariants checked by name | invariants.rs:93-115 `check_invariants` calls the 25 (20 fixture fns, plus 5 config in :62-88) | 25 | Add the 8 new: `NackedHasItsTime`, `OnlyNackedHasNackTime`, `DeferredHasArrivalPosition`, `OnlyDeferredHasArrivalPosition`, `DeferredArrivedAfterHeld`, `ArrivalPositionUnoccupied`, `UniqueArrivalPosition`, `ArrivalPositionsBelowNext` (the specification's Invariants section and the `Record` block). The count becomes 33 (plan-spec-after) |
| DeadLetterTtlNonNegative witness | invariants.rs:80-86: `letter.expires_at == letter.nacked_at + dead_letter_ttl` | - | **False under the new rules** (expires = lease end + ttl). Reformulate (for example `>=`, or predicted exactly by the model) |
| Fixture invariant fns | fixture.rs:255-538 (`invariants` mod): `held_version_not_newer` :400-406; `outstanding_only_while_leased` :408-424 (`status != Queued`); `all` :517-537 | - | Add the eight; these need the snapshot to carry mark, nacked_at and deferred_position |
| Transitions | invariants.rs:126-131 `RECORD_EDGES` includes `held->deferred` and `deferred->queued`; :135-141 `record_state` maps `Deferred` | - | Only queued↔held |
| Records across a step | invariants.rs:156-198 `check_records`: a record leaves only if it was held **and** `settled == Some(key)` (:166-173); held→held requires status equality (:181-186) | - | Records now leave only at release or lapse (acked or nacked, no deferral); held→held changes the mark (and `deferred_position` on publish), so compare held rank and payload only |
| Batch guarantees | invariants.rs:259-308 `check_batch` (`OldestFirst`, `ExclusivePocket`, `OneVisitPerKeyPerLease`); doc :259-263 cites OQ9 | - | Doc; with the `handed` removal gone the check gets stronger |
| WANTED labels | invariants.rs:373-437 | "ack Settled" :392, "ack Deferred" :393, "nack Settled" :399, "nack Deferred" :400, "record Deferred in Leased" :428, "publish Superseded" :380 | Replace with the new outcomes (mark, repeat, conflict refusal, defer behind held, the six reconciliation rows on release and on lapse, dead letter made at lapse) |
| Module doc | invariants.rs:1-39 ("all 25 invariants" :3; unreachable-guard notes :17-36) | - | Rewrite |
| Header doc | walk.rs:1-13 | Generic | Fine |

Other model or state-machine tests: none found beyond walk.rs and invariants.rs (`proptest` is used only there).

#### 4. Library code (`crates/libpachinko/src`)

Public-API changes are semver-relevant. `libpachinko` is 0.1.0 (`crates/libpachinko/Cargo.toml:3`); `just semver` is advisory before 1.0 (`docs/reference/commands.md:59`).

| Item | Where | `pub`? | Today | Decision changes |
| --- | --- | --- | --- | --- |
| `RecordStatus` | LS/records.rs:81-96 | pub, `#[non_exhaustive]` enum; the `Held { rank, payload }` variant is **not** non_exhaustive | `Queued`, `Held{rank,payload}`, `Deferred` (doc :93-95 "acked or nacked; pullable again once the lease ends") | Remove `Deferred` (breaking). `Held` needs `mark`, `nacked_at: Option<Timestamp>`, `deferred_position: Option<Position>`; new fields on that variant are breaking for literal patterns and constructions (doctest :78, contract, tests) |
| `RecordView` / `RecordView::new` | LS/records.rs:114-161 | pub | 5 params; doc :125 "or deferred"; doctest :136-143 uses `RecordStatus::Deferred` | Doctest breaks. `new` is already at rustqual's `max_parameters = 5` (rustqual.toml), so new fields cannot become new params |
| `PocketRecords` trait | LS/records.rs:183-337 | pub trait (store seam) | `lookup`, `insert_queued`, `supersede(key,rank,payload)->Option<(Rank,Payload)>` (:234-239, returns the loser even when held), `hold_oldest`, `settle(key)->Option<HeldRecord>` (:258-273, removes), `defer(key, position)->Option<HeldRecord>` (:275-291, held to Deferred at a fresh position), `free_held()->usize` (:293-308, "held and deferred to queued… O(outstanding)"), `record_count`, `held()` (:323-336) | Every implementor breaks. Needed: a mark step (with the nack time); a defer-behind-held step that stores `deferred_position` (or `supersede` taking a position); `supersede` must not shed the held version; `settle` and `defer` go; `free_held` becomes a lease-end step that reconciles per mark and returns what to shed and dead-letter (rank, payload, nacked_at) and keeps the O(held) cost; `held()` needs marks. Doc and doctests at :219-233, :241-256 ("held or deferred record is never taken"), :258-308 change |
| `MemoryRecords` | LS/records/memory.rs:22-171 | pub struct | `Entry{rank,payload,position,status}` (:31-37); `outstanding` index "held and deferred" (:8-10, :27); `held_version` matches `Deferred` (:52-59); `supersede` (:96-106); `settle` (:130-135); `defer` (:137-141); `free_held` (:143-153, sets every outstanding key Queued at its own position); `held` (:159-170 filters to Held) | Implement the new steps; the lease-end positions per decision 7 (keep the position, or take `deferred_position`); remove acked or nacked records without a deferral |
| MemoryRecords unit tests | memory.rs:199-239 | test | `the_outstanding_index_is_exactly_the_held_and_deferred_keys` (:213-227, uses `defer`, `settle`); `free_held_visits_only_the_outstanding` (:230-239) | Rewrite |
| Records contract | LS/records/contract.rs:65-75 (`check_contract`), :107-128 (`agree` matches `Deferred` :122), :158-201 (`a_second_insert_is_refused` uses `defer` :188-200), :203-260 (`supersede_keeps_position_and_status`: asserts supersede of a **held** record returns the loser `(2,"new")` = the held version :223-227, then defer, supersede of deferred :243-259), :262-287, :303-320 (`settle_removes_only_held`), :322-361 (`defer_takes_the_fresh_position`), :363-399 (`free_held_requeues_held_and_deferred`), :406-413 (`change` uses `settle`) | pub fn behind `testing` | Pins the old store steps | Rewrite for the new steps; the doc :49-53 on the O(outstanding) bound |
| `compile_fail` doctest | LS/queue.rs:85-105 | doc | Implements every trait method (incl. `settle`, `defer`, `free_held`) to prove `E0277` | With a changed trait it fails with another error (E0046/E0407), so the doctest no longer proves what it says. Update the method list |
| `Queue` docs | LS/queue.rs:1-8, :45-80 | - | Generic | Fine |
| `Queue::lease_view` | LS/queue.rs:225-262 | pub | `LeaseView::new(pocket, expires_at, self.pockets.held(..))` | Must expose `mark` (`exposes` gains `record.mark`) |
| `LeaseView` | LS/queue/types.rs:102-142 | pub, non_exhaustive | `records: Vec<HeldRecord>` | Carries the marks: a new type, or `HeldRecord` gains a field (section 5) |
| `HeldRecord` | LS/ids.rs:197-221 | pub, non_exhaustive | `key, rank, payload`; `new(key,rank,payload)` | Shared by batch, `LeaseView`, trait and `RecordReply`. Adding `mark` is additive; changing `new` is breaking |
| `PublishOutcome` | LS/queue/types.rs:55-74 | pub, non_exhaustive | `Superseded` doc :68-70 "the loser is a superseded record" | `DeferBehindHeldVersion` sheds nothing: a new variant (additive) or a reworded `Superseded` (open) |
| `SettleOutcome` | LS/queue/types.rs:311-327 | pub, non_exhaustive | `Settled` ("record is gone"), `Deferred` ("deferred at a fresh position"); doctest :313-316 | Both meanings gone; rename or replace (breaking either way if a variant is removed) |
| `PocketFate` | LS/queue/types.rs:329-344 | pub | Doc :338-342 "held, deferred or queued… no records" | Doc: fate by `records` vs `ending_records` |
| `SupersededRecord` / `ShedReason` | LS/queue/types.rs:237-309 | pub (testing re-export) | Doc :261-262 "the loser of a supersede, or a stale arrival" | Add "an unsettled held version at lease end"; `shed_at` = lease end there |
| `DeadLetter` | LS/queue/types.rs:346-394 | pub (testing re-export) | Doc :346 "A nacked payload"; `nacked_at` "When it was nacked" (:390); `expires_at` "**nacked_at plus dead_letter_ttl**" (:392); doctest :364-367 nack then `dead_letters[0]` | Doc :392 wrong (lease end + ttl); the doctest panics (no letter until the lease ends) |
| `Queue::ack` / `Queue::nack` / `settle_held` | LS/queue/settle.rs:1-157 | pub fns | Module doc :1-3; `Settled` struct :14-23; ack doc :26-39 + doctest :54-60 (`Ok(Deferred)` then a second ack `Err(RecordNotHeld)`); `ack` :62-71 (resets the count after settle); nack doc :73-83 + doctest :99-102 (`Ok(Settled)`, no redelivery); `nack` :104-126 (creates `DeadLetter` at `now` with `expires_at = now + dead_letter_ttl` :116-124); `settle_held` :131-156 (guards, extend, `pockets.settle`) | Mark-only, repeat = extend only, conflict refused; no dead letter here. Both doctests break (:59-60, :100) |
| `Pockets::settle` / `is_held` / `reset_expiries` | LS/queue/pockets.rs:199-243 | pub(super) | `settle` :208-234 (`held_rank == rank` gives `records.settle`, else `take_position` + `records.defer`) | Replace with a mark step; `reset_expiries` doc :236-238 names old rules |
| `Pockets::publish` | LS/queue/pockets.rs:82-148 | pub(super) | `rank > stored.rank` always calls `records.supersede` and sheds the loser (:113-124) | Branch: held and `held_rank = rank` means defer (take `next_position`, no shed); else supersede |
| `Pockets::release` | LS/queue/pockets.rs:245-263 | pub(super) | `free_held()`, reset the count, `record_count()==0` gives Idle; returns `Option<PocketFate>` only | Needs `now`, the consumer, both TTLs and a way to hand shed entries and dead letters to `Shed` (it has no clock, consumer, TTL or shed access today) |
| `Pockets::lapse` | LS/queue/pockets.rs:265-291 | pub(super) | Increments the count, `free_held()`, then park / requeue at its old position / idle; returns `()` | As release |
| `Queue::release` | LS/queue/renew.rs:62-112 | pub | Module doc :1-3 (old Freed rules); doc :62-67 "held and deferred records return to queued"; calls `pockets.release` | Pass `now`, the consumer and TTLs; keep the shed entries made |
| `Queue::tick` / `lapse` | LS/queue/expiry.rs:75-157 | pub / private | Doc :82-87 "held and deferred records return to queued"; `lapse` :146-156 calls `pockets.lapse(key, limit)` | Same plumbing. `TickReport` (:45-54) has no count of letters or shed entries **created**; spec silent (optional) |
| `Shed` | LS/queue/shed.rs:1-88 | pub(super) | Module doc :1-4 "the two nack rules [feed] the second"; `keep_dead_letter` :57-62 keyed `(expires_at, arrival)` | Doc only; keying still works |
| `Queue::publish` | LS/queue/publish.rs:13-65 | pub | Doc :14-20 "a higher rank supersedes in place and sheds the loser" | Doc and outcome for the held case |
| Crate docs | LS/lib.rs:1-3 ("settle"), :24, :27-28 (table rows `SettleOutcome: Settled, Deferred`), :56-73 (crate doctest `ack == Ok(SettleOutcome::Settled)` :72) | - | - | The crate doctest's outcome changes |
| Errors | LS/error.rs:78-104 `LeaseActionError` (non_exhaustive): `RecordNotHeld` "names a record key the lease's pocket does not hold" (:101-103) | pub | No variant for "ack after nack / nack after ack" | Needs a refusal (new variant additive, or reuse `RecordNotHeld`: open) |
| `testing` module | LS/testing.rs:1-23 | pub (feature) | Re-exports | Doc fine; contract changes |
| Inspect | LS/queue/inspect.rs:38-88 | pub (feature) | `PocketView.records: Vec<RecordView>` | Needs the new fields via `RecordView`/`RecordStatus` |
| Bench | `crates/libpachinko/benches/workloads.rs:21-24` (W3 doc "release finds the pocket empty and idles it"), :144-156 `consume_cycle` (16 acks, then release), :262 (W5 tick) | bench | - | Behaviour still idles (all acked count as ending). The cost moves from ack to release, so the figures in `docs/decisions/0019-service-runtime-tokio-and-http.md:174, :225` (W3) describe the old code; a decision record is history, flag only |
| Fuzz | `fuzz/fuzz_targets/smoke.rs` | - | Only `libpachinko::version()` | Unaffected |
| Test-side store | LQ/counting.rs:54-113 | test | Mirrors every trait method incl. `settle`, `defer`, `free_held` | Mirror the new trait |

#### 5. Service and wire (`crates/pachinko`)

There is no OpenAPI or schema file for the service. `git ls-files | grep -iE 'openapi|schema|swagger'` lists only the vendored `allium-skill-reference/allium/schemas/*.json`. There are no metrics: grep for `metric|counter|histogram` in PS/ finds only a test helper (`actor.rs:1047-1048`). The tick logs counts only (`PS/actor.rs:440-452`, `sweep` at :442: `lapsed`, `decayed_superseded`, `decayed_dead_letters`).

| What | Where | Today |
| --- | --- | --- |
| Ack and nack outcome strings | PS/http/leases.rs:45-52 `settle_outcome`: `Settled => "settled"`, `Deferred => "deferred"`, other gives `unmapped` (503); `SettleReply{outcome}` :26-30; handler docs :71 and :86 "200 with `settled` or `deferred`" | |
| Ack and nack commands | PS/actor.rs:97-110 (`Command::Ack/Nack` reply `Result<SettleOutcome, LeaseActionError>`), :307-337 (handle methods) | Type follows the library |
| `record_not_held` | PS/http.rs:163-166 (variant), :220 (404), :243 (code), :302 (map); unit test :873-875; map test :1011-1012; `unmapped` :256-260 answers 503 `not_ready` for an unknown library variant | A new library refusal not mapped here gives **503** |
| Publish outcomes | PS/http.rs:459-468 `publish_outcome` (`created`, `new_key`, `superseded`, `stale`); batch PS/http/batch.rs:36-41 | |
| Lease read | PS/http.rs:441-457 `LeaseReply{pocket_key, expires_at, records: Vec<RecordReply>}`; `RecordReply{record_key, rank, payload}` :408-425, **shared with the pull batch** :426-439 | |
| Loadgen (separate crate) | `loadgen/src/client.rs:17` `APPLIED = ["created","new_key","superseded","stale"]`; `:204-219` ack accepts only `"settled" \| "deferred"`, else an error (fail-closed run) | Wire strings hard-coded |
| HTTP tests | PT/http.rs:393-406 `ack_settles` (test :409): `"settled"` :399, lease `records == []` :402, second ack 404 `record_not_held` :403-405; :415-427 `nack_dead_letters` (test :429): `"settled"` :421; :460-480 `release_requeues` (test :483): status only; :489-511 `old_lease_fenced` (test :514): `"settled"` :509; :619-638 `lapse_over_http` (test :641): unaffected; :191-221 `large_ranks`: no held key, unaffected. PT/batch_publish.rs:226-252: fresh keys, unaffected. PS/http.rs:776-800 route probe: status only | |

What a client would see differently. These are open points, not decided here:

1. **Ack and nack reply.** `settled` and `deferred` both lose their meaning: an ack or nack always marks now. Open: the new outcome vocabulary (for example `acked`/`nacked`, or one `marked`), and whether a repeat reports itself differently from the first mark (`AckRepeats` is a distinct rule).
2. **A second ack (or nack) of the same record.** Today it is 404 `record_not_held` (pinned at PT/http.rs:403-405, LQ/settle.rs:100-102, doctest LS/queue/settle.rs:60). Under the decision it is 200 and extends the lease.
3. **Ack after nack, or nack after ack.** These are rejected under the decision. Open: the library variant and the HTTP status and code (`record_not_held` 404 reused? a new 409?). Until mapped, the service answers 503 `not_ready` through `unmapped`.
4. **Publish onto a held key** (`DeferBehindHeldVersion`). Today it is `superseded`, and the held version is shed into the superseded queue. Under the decision nothing is shed at publish. Open: keep `superseded`, or a new `deferred` publish outcome. A new string also breaks `loadgen/src/client.rs:17` for any workload that publishes onto held keys (the current workloads publish only fresh keys: `loadgen/src/workloads.rs:211`).
5. **Lease read** (`GET /consumers/{id}/lease`). `exposes` gains `record.mark`. Open: a `mark` field on lease records only, or on batch records too (`RecordReply` is shared; a batch record is always unsettled). Today acked or nacked records vanish from the lease read; under the decision they stay, with their mark.
6. **Release and lapse.** The replies are unchanged (`queued`/`idle`), but the pocket is idle only when every held record is acked or nacked with no deferral. Dead letters and superseded records now appear at the lease end; no route reads them (open question 2), so a client sees no difference beyond decay timing.
7. **Pull within a lease after an ack or nack plus a republish of that key.** Today the key is handed out again (OQ9). Under the decision it is not, until the lease ends.

#### 6. Handbook, READMEs, changelog (stale text)

`docs/specs/` is excluded. The two pages `a338355` changed are listed with the lines read just before it.

- **`docs/reference/http-api.md`**:
  - :26 the publish outcome list (if a new outcome is added).
  - :30 "200 `{"outcome": "settled"}` or `{"outcome": "deferred"}`".
  - :49-51 lease shape "its records the ones it holds, at the rank and payload": no mark.
  - :59-63 "An ack removes a held record whose held version is still the latest (`settled`), or, when a newer version arrived while it was held, defers that newer version to the back of the pocket until the lease ends (`deferred`). A nack does the same and keeps the held version as a dead letter."
  - :65-67 "A release ends the lease, returns its held and deferred records to queued in place".
  - :72-73 "(each is held or deferred, or none remains)".
  - :83-84 "returns its held and deferred records to queued in place".
  - :86-87 "a pocket whose records were all acked or nacked goes idle" (now: with no deferral).
  - :104-105 and :227 `record_not_held` meaning (now also the conflict case, if reused).
  - :237-238 "the dead letters nacks create are kept until they decay, at … after they were shed or nacked".
- **`docs/how-to/test-and-debug.md:98`**: the lease-read example JSON (no `mark`).
- **`docs/reference/api.md:86-88`**: "`ack` and `nack` … return a `SettleOutcome`, `Settled` or `Deferred`".
- **`docs/explanation/architecture.md`**:
  - :146 crate-root list `SettleOutcome`.
  - :183-186 store seam "settle, defer, and free every held and deferred record in O(outstanding)".
  - :197-198 "the shed queue of superseded records".
  - :216-218 "A nack's dead letter sits in the shed queue…" (created at the lease end now).
  - :223-225 tick "its held and deferred records return to queued".
- **`docs/project/terminology.md`**:
  - :30 "Superseded record | A payload that lost a supersede, or arrived with a lower rank…".
  - :31 "Dead letter | A nacked payload…".
  - :37 "Queued, held, deferred | … deferred means … restarts at the back with a fresh position…".
  - :39 Position "a deferral takes a fresh one for the record".
  - Missing rows: Held set, mark, Deferred (new sense).
- **`docs/reference/testing.md`**:
  - :128 and :438 "all 25 invariants" (now 33).
  - :171 "77 scenarios".
  - Rows :187, :201, :220-229, :234, :251-252, :254-255 (titles: S043 "defers it on ack", S044 "restarts at the back", S045 OQ10, S047 "dead-letters … on the first nack", S048 "deferred on nack", S054 "only deferred records", S071/S072 OQ9, S074 "decay at nacked_at plus").
  - :173-176 the tag legend.
- **`docs/how-to/work-with-the-specs.md`**: :20 module summary (fine); :60-87 open-question list (already reflects the decision); :77-78 "scenarios tagged `oq:9` and `oq:10` keep those tags until their tests follow".
- **`docs/explanation/specifications.md`**: :33-41 "What exists today" (no held-set mention; fine); :56-69 the open-question paragraph (already updated).
- **`docs/explanation/prior-art.md`**:
  - :229-230 "questions 10 and 11 came later … and have no section here".
  - :371-384 "A key re-published within the lease" (research on OQ9; now closed, so the section could note the decision).
  - :190, :194-195 "held and deferred states", "deferral" (still valid wording).
- **`docs/explanation/security-model.md:24-25`**: "dead-lettered ones until their expiry" (fine).
- **`docs/project/purpose-and-scope.md:21-25`**: fine.
- **`README.md`**:
  - :51 and :68 `settled` in the sequences.
  - :95-100 "PUT k1 at rank 3 → superseded … ack k1 → deferred" (the publish and ack replies change).
  - :13 and :16: fine.
- **`crates/libpachinko/README.md`**:
  - :31-32 signatures `SettleOutcome`.
  - :48 "`ack`, `nack` | `Settled`, `Deferred`".
  - :70-78 record state diagram (`Held --> [*]: ack or nack, Settled`, `Held --> Deferred …`, `Deferred --> Queued`).
  - :88-98 sequence ("Superseded, o-1 stays held at rank 2", "ack → Deferred").
  - :111 and :126 usage `SettleOutcome::Deferred`.
- **`crates/pachinko/README.md`**: nothing found.
- **`CHANGELOG.md`**:
  - :49-54 "checks all 25 invariants … 76 scenarios".
  - :60-66 "`SettleOutcome` for a settled or deferred record … `DeadLetter` for a nacked payload kept beside the superseded records".
  - :109-112 `record_not_held`.
  - Needs new `[Unreleased]` entries for both crates (libpachinko `[Unreleased]` at :12 (`### Changed` :14, `### Added` :44) and pachinko `[Unreleased]` at :92 (`### Changed` :94)).
- **`docs/decisions/0019-service-runtime-tokio-and-http.md:174, :225`**: W3 figures measured under the old ack. History; flag only.
- **Double shed or "shed twice"**: no handbook text found outside the spec.

#### 7. Surprises, mismatches and ambiguities

1. **S075 breaks outright** (LQ/expiry.rs:731-732): an untagged OQ9 behaviour, `ack` then an equal-rank re-publish `.unwrap()`, now gives `EqualRank`.
2. **The untagged backward-clock test** reads `superseded[0]` after a publish onto a held key (LQ/expiry.rs:648-649); it panics.
3. **The DeadLetterTtlNonNegative witness** in the walk (LQ/invariants.rs:80-86) is exact `nacked_at + ttl`, which is false now. The `DeadLetter` doc (LS/queue/types.rs:392) says the same.
4. **The `compile_fail` doctest** (LS/queue.rs:85-105) lists every trait method; a trait change silently turns it into a different compile error.
5. **The `PocketRecords` shape versus the new state.** The trait has no notion of mark, nack time or arrival position. `free_held` returns only a count (LS/records.rs:308), so the queue cannot learn what to shed or dead-letter at the lease end. `supersede` returns the held version as "loser" for a held record (contract.rs:223-227 pins it): that is the OQ10 double shed in the store contract. Per-record `nacked_at: Timestamp` must live in the store (the trait now takes no `Timestamp`) or in a queue-side map; implementation choice.
6. **Lease-end plumbing.** `Pockets::release` and `Pockets::lapse` (pockets.rs:251, :274) have no `now`, consumer id, TTLs or access to `Shed`; the reconciliation needs all four. rustqual limits (`max_parameters = 5`, `max_methods = 20`, `max_fields = 12` in `rustqual.toml`) may force a parameter struct.
7. **Ordering of several shed entries created at one lease end** is not specified (the spec's `for record in pocket.held_records`). `Shed` keys by `(expires_at, arrival)`, so the arrival order the implementation chooses (for example by position or key) is observable in `Snapshot` and must be pinned and documented ("What the specification leaves open is pinned", testing.md:154-157). The walk compares the `Vec`s in order (walk.rs:221-222).
8. **The tick order.** In one tick, lapses run before decay with the same `now` (LS/queue/expiry.rs:126-141), so with `dead_letter_ttl = 0` or `superseded_ttl = 0` an entry created by a lapse decays in that same tick. That is consistent with the spec; the model must predict it.
9. **`AckRepeats` does not reset `consecutive_expiries`.** The code's `ack` resets it unconditionally (LS/queue/settle.rs:69). The spec argues the count is already 0 under that lease (pachinko.allium Ack comment), so an unconditional reset is observationally equal. Say so in the code or test, or skip the reset. Since `983b785` the same holds for `NackRepeats`: `NackMarksRecord` resets and `NackRepeats` does not.
10. **Refusal for a conflicting mark.** `LeaseActionError` has no variant; the service maps unknown variants to 503 `not_ready` (PS/http.rs:256-260, :297-305), so the library and service must land together.
11. **`HeldRecord` is shared** by `PullOutcome::Batch`, `LeaseView`, `PocketRecords::held`/`Batch` and `RecordReply`. Putting `mark` on it puts an always-`unsettled` field on batches and the batch wire (open, section 5).
12. **`RecordView::new`** already takes 5 params, the rustqual limit. The new fields go into `RecordStatus::Held`, whose variant is not `#[non_exhaustive]`, so adding fields is breaking.
13. **File size risk.** `LQ/walk.rs` is 1,178 lines now; `testing.md:165-167` says rustqual holds test files to 1,000 lines. It evidently passes today (it counts code lines only), but the six new Freed rules will grow it; plan a split.
14. **Code already differing from the old specification: none found** in what was compared: the publish split (pockets.rs:105-140), settle (pockets.rs:214-234, settle.rs), release and lapse (pockets.rs:251-291) and the shed stamps (publish.rs:54-62, settle.rs:116-124) against the rules at `62efe41` (`Supersede`, the ack and nack rules, the release and lapse rules, the requeue rules); they matched.
15. **Points about the specification the survey could not settle from the code** (not decided here):
    - A dead letter's `nacked_by` is `lease.consumer` at the lease end. That equals the nacker, since only the lease holder can nack, so it is fine.
    - Decision 7 positions: `ConsumeAckedVersionQueueDeferred` and `DeadLetterNackedVersionQueueDeferred` set `record.position = deferred_position`, while `RequeueUnsettledRecord` and `SupersedeUnsettledHeldVersion` keep the held position. The store must therefore keep both positions while held. That fits the invariants `ArrivalPositionUnoccupied` and `UniqueArrivalPosition`, but the `MemoryRecords` queued index (memory.rs:26) must be keyed by the final position at the lease end, not at publish.
    - `DeferBehindHeldVersion` consumes a counter value on publish into a held key (a visible `next_position` change the walk must predict), and a later `Supersede` of the deferred version keeps that arrival position, shedding the deferred loser at once with `shed_at = now`.

#### 8. The nack reset

T32's "What T35 must flip" (`tickets/T32-spec-follow-up.md`, under Close-out) lists what
follows the reset, with lines as of `8f15563`. The specification's half landed at
`983b785`. The rest:

- `crates/libpachinko/tests/queue/expiry.rs`: S063's "nacked" case, which ends
  `(Parked, 3, 0)` at the default limit and, with the nacks resetting the count, idles
  with a count of 1; S064's "after the nack" assertion, now a reset; S064's title in
  the catalog.
- The walk's model, which must reset the count on a nack, and its reach label "nack
  keeps a counted pocket" (`crates/libpachinko/tests/queue/walk.rs:957`,
  `invariants.rs:405`).
- `a_nack_leaves_the_expiry_count_over_http` in `crates/pachinko/tests/http.rs`, with
  its driver `nack_keeps_count` and its S064 row in the catalog: red once the reset
  lands, so it is inverted and renamed.
- `Queue::nack` and the module doc in `crates/libpachinko/src/queue/settle.rs` ("a nack
  leaves it", line 3).
- `docs/reference/http-api.md:64`, "a nack leaves it".
- `CHANGELOG.md:40`, "A nack, renew or pull leaves the count", and `:137`, "a nack does
  not".
- Every sentence that lists the resets as "ack, release or revive":
  `crates/libpachinko/src/config.rs:69`, `crates/libpachinko/src/queue/types.rs:168`,
  `crates/libpachinko/src/queue/inspect.rs:47`, `docs/reference/configuration.md:39`,
  `docs/project/terminology.md:36`, `docs/reference/http-api.md:86`, `README.md:108`,
  `CHANGELOG.md:39` and `:59`, and the comment on `PARK_AT_TWO` in
  `crates/pachinko/tests/http.rs`.

### Risks (`CONVENTIONS.md` §13)

- **A public trait changes.** `PocketRecords` loses `settle` and `defer` and its
  `free_held` and `supersede` change meaning; `RecordStatus` loses a variant;
  `SettleOutcome` loses its meanings. Every implementor breaks, `just semver` reports
  it, and the `compile_fail` doctest at `LS/queue.rs:85-105` must still fail for the
  reason it states.
- **The library and the service land together.** An unmapped library refusal answers
  503 `not_ready` through `unmapped` (`PS/http.rs:256-260`), so a library-only commit
  changes the wire for the worse. `loadgen/src/client.rs:204-219` accepts only
  `settled` and `deferred` and fails closed on anything else.
- **The model and the queue agreeing for the wrong reason.** The walk model
  (`LQ/walk.rs`) is rewritten in the same ticket as the queue. The scenario tests are
  derived from the specification first and each is shown to bite before the code
  changes.
- **The small denominator.** A new branch with no test can breach the 90% floor; each
  of the six lease-end rows and each mark, repeat and refusal has its scenario test.
- **File size.** `LQ/walk.rs` is about 1,180 lines and rustqual holds test files to
  1,000 code lines (`docs/reference/testing.md`, "Conventions"); the six lease-end rules
  grow it. Plan a split at step 3.
- **lychee and typos over `tickets/`.** The hand-back notes quote test names and paths
  as code spans, never as links.
- **The specification moves first.** For any wire decision that needs a rule, the
  specification changes before the code (`AGENTS.md`, invariant 1), as it did for Q2 and
  the nack reset at `983b785`.

## Goal

The library, the service, loadgen, the tests and the handbook do what the specification
at `a338355` and `983b785` says: a record stays held until its lease ends, a nack resets
its pocket's expiry count, and a stale publish leaves a parked pocket parked. Every
scenario the decisions change or add is pinned by a tagged library test that was red
before the code changed; every `// obligation:` tag names an id
`just plan-spec pachinko` prints; no test, catalog row or page carries `oq:9`, `oq:10`
or `oq:11`; the walk model predicts the decided behaviour and checks all the module's
invariants; a client of the service sees marks on the lease read, a repeat ack succeed
and a changed mark refused, under names the maintainer chose at step 1.

This ticket owns:

| Kind | Names |
| --- | --- |
| Decisions | Q1, Q2 (the module's question 11) and the nack reset, all decided and recorded above; the wire names, the decision record and the eight data-model refinements of step 1, among them the module's question 1 closed as "reject" |
| Rules | The step-1 refinements: the nine guards that tell a deferred version by `deferred_position`, `Supersede`'s explicit guard, the invariants `HeldCopiesAgree` and `MarkedPocketHasNoExpiries`, and the equal-rank rejection stated as decided. Q2 and the nack reset landed at `983b785`; any defect found in the rules at `a338355` or `983b785` is fixed in the specification first |
| Scenarios | Every row of "Scenarios" in the impact map; S024 split and S026 rechecked under Q2; S063's nacked case, S064 and `a_nack_leaves_the_expiry_count_over_http` flipped by the nack reset; new scenarios from S078 for the six lease-end rows by release and by lapse, the publish table, repeated and changed marks, positions, the lease read's marks, and a parked pocket left with no records |
| Obligations | Every id `just plan-spec pachinko` prints for the rules changed at `a338355`, at `983b785` and at step 4 |

## Non-goals

- No answer to any other open question: not the module's question 2 (who reads the
  superseded and dead-letter queues, and whether an entry can be replayed), 3 (capacity)
  or 7 (what else revives a parked pocket). Question 1 (equal rank) is closed as
  "reject" at step 1, and the rejection does not change.
- No change to the Q1 decision. A finding that the decision is wrong goes to the
  maintainer as a question, never into a fix.
- No reset of `consecutive_expiries` on a renew or a pull; an ack and a nack reset it.
- No `QueuedHasRecords` invariant: T32's close-out (decision 6) gives it to a ticket
  after T35.
- No route that reads the superseded or dead-letter queues.
- No "transaction" vocabulary (decision 8).
- No decision record beyond decision 0020, which the maintainer asked for at step 1:
  terse, the why of the Q1 decision, pointing at the specification for the detail.
- No tombstones and no spilling of idle pockets to disk: the maintainer's notes leave
  both for later.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | Q2 and the nack reset, landed at `983b785` with `open question` 11 removed; the step-1 refinements, with `open question` 1 removed |
| `docs/decisions/0020-held-set-until-lease-ends.md`, `docs/decisions/README.md`, `docs/manifest.yml` | new, shared | Decision 0020, its index row and its manifest entry |
| `docs/operations/troubleshooting.md` | shared, own lines | The `equal_rank` sentence that called the refusal a placeholder |
| `docs/project/purpose-and-scope.md` | shared, own lines | The list of open questions, which still named equal-rank publishes |
| `crates/libpachinko/src/records.rs`, `records/memory.rs`, `records/contract.rs` | shared, own lines | `RecordStatus`, `RecordView`, the `PocketRecords` trait, `MemoryRecords` and the contract: marks, the nack time, the arrival position, the lease-end step |
| `crates/libpachinko/src/queue.rs`, `queue/pockets.rs`, `queue/settle.rs`, `queue/publish.rs`, `queue/renew.rs`, `queue/expiry.rs`, `queue/shed.rs`, `queue/types.rs`, `queue/inspect.rs` | shared, own lines | Publish's defer branch, the mark step, the lease-end reconciliation and its stamps, the outcome types, the lease view's marks, the docs and doctests |
| `crates/libpachinko/src/ids.rs`, `error.rs`, `lib.rs`, `config.rs` | shared, own lines | `Mark` and `MarkedRecord`, and the `HeldRecord` docs; the refusal for a changed mark; the `EqualRank` message, no longer a placeholder; the crate docs and doctest; the S005 doc if the dead-letter TTL wording changes; the `max_consecutive_expiries` doc (impact map, section 8) |
| `crates/libpachinko/tests/queue/*.rs`, `crates/libpachinko/tests/queue/walk/model.rs` | shared, own lines, one new | The scenarios, the walk model (split out of `walk.rs` into `walk/model.rs`), the invariant checks, the fixture and the counting store |
| `crates/libpachinko/benches/workloads.rs` | shared, own lines | W3's doc comment, if its wording is no longer true |
| `crates/libpachinko/README.md` | shared, own lines | The signatures, the record state diagram, the sequence and the usage |
| `crates/pachinko/src/http.rs`, `http/leases.rs`, `http/lease_read.rs`, `http/batch.rs`, `actor.rs` | shared, own lines, one new | The ack and nack outcome mapping, the refusal's status and code, the lease read's marks, the publish outcome if step 1 adds one |
| `crates/pachinko/tests/http.rs`, `tests/batch_publish.rs` | shared, own lines | The wire drivers for S040, S047, S068 and the new wire scenarios; `a_nack_leaves_the_expiry_count_over_http` and the `PARK_AT_TWO` comment |
| `loadgen/src/client.rs` | shared, own lines | The outcome strings it accepts |
| `docs/reference/testing.md` | shared, own lines | The catalog rows, the scenario and invariant counts, the tag legend |
| `docs/reference/http-api.md`, `docs/reference/api.md` | shared, own lines | The ack, nack, lease-read, release and error text, and the `equal_rank` text that called the refusal a placeholder |
| `docs/reference/configuration.md` | shared, own lines | The `--max-consecutive-expiries` sentence, CR5's, on T32's flip list |
| `docs/explanation/architecture.md`, `docs/project/terminology.md`, `docs/how-to/test-and-debug.md` | shared, own lines | The store seam, the shed queues, the terms (held set, mark, deferred), the lease-read example |
| `docs/how-to/work-with-the-specs.md`, `docs/explanation/specifications.md` | shared, own lines | Question 11 left both lists at `983b785` and question 1 leaves them at step 4; the sentence on the `oq:9`, `oq:10` and `oq:11` tags goes at step 7 |
| `docs/explanation/prior-art.md` | shared, own lines | A sentence on the section about a key re-published within the lease, saying question 9 is closed; the count of closed questions at :229-230 and :415; question 1 closed as "reject" at :11 and :281 |
| `README.md` | shared, own lines | The sequences' `settled`, `superseded` and `deferred` replies |
| `CHANGELOG.md` | shared, own lines | libpachinko and pachinko Unreleased, Changed |
| `tickets/README.md` | shared, own lines | T35's Status cell |
| `tickets/T35-decide-q1-q2.md` | ticket | `status:` and hand-back notes |

## Steps

1. **Authorisation required: the product decisions that remain.** Ask the maintainer,
   citing the step-through page `https://claude.ai/artifact/3X1JB236GK4MRvj3f69UE2` and this
   ticket's Context:
   - **The ack and nack reply.** `settled` and `deferred` have lost their meanings: an
     ack or nack now always marks. Which names does the library's outcome type and the
     wire use, and does a repeat answer differently from the first mark?
   - **The refusal of a changed mark** (an ack after a nack, a nack after an ack): the
     library's error variant and the service's status and code. `record_not_held` (404)
     no longer describes it: the record is held.
   - **A publish that is deferred behind a held version** (`DeferBehindHeldVersion`):
     does it still answer `superseded`, although nothing is shed, or a new outcome? A
     new string reaches `loadgen/src/client.rs:17`.
   - **Where the mark is shown.** The specification's `exposes` adds it to the lease
     read. `HeldRecord` and the wire's `RecordReply` are shared with the pull batch,
     where a record is always unsettled: a field on both, or on the lease read only?
   - Whether a decision record is wanted for the Q1 decision (Non-goals).
   - Whether the qsketch lines in Context are right, since this repository cannot read
     them.

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these options is a new design: record
   it and stop again.
2. **The worktree.** The branch `ticket/t35-decide-q1-q2` and its worktree exist,
   stacked on `ticket/t32-spec-follow-up` at `62efe41`, with the specification at
   `a338355`. T32 merged on 2026-11-16 as pull request 48 into `wave-2-tickets`, not
   `main`. As for T32 and T34, `wave-2-tickets` stands in for `main` as the merge target
   and the pull request base; `origin/wave-2-tickets` was merged into the branch at
   `b5d4a83` (Hand-back notes). Before any later merge, `git fetch origin` (network,
   authorised by this step) and merge `origin/wave-2-tickets` (`CONVENTIONS.md` §11), as
   T32 did. In a fresh worktree
   run `just initialize` (network, authorised by this step). Run `just check` before any
   edit and quote its last line as the baseline for the remaining steps.
3. **The plan.** Use the `plan-change` skill. Read the whole module and
   `just plan-spec pachinko`'s output, and re-read every line the impact map cites. Then
   decide and write down, before any test:
   - the store seam: how `PocketRecords` carries the mark, the nack time and the
     arrival position; what replaces `settle`, `defer` and `free_held`; what the
     lease-end step returns so the queue can stamp shed entries and dead letters, with
     its cost still bounded by the held set; and that `supersede` no longer returns
     the held version as a loser;
   - the lease-end plumbing: `Pockets::release` and `Pockets::lapse` need the end time,
     the lease's consumer, both TTLs and a path to `Shed`, within rustqual's limits;
   - the order of several entries created at one lease end, which the specification
     leaves open: choose one, pin it in a test and say so in
     `docs/reference/testing.md` ("What the specification leaves open is pinned");
   - whether `AckRepeats` and `NackRepeats` skip the count reset or perform it (it is 0
     either way);
   - the split of `LQ/walk.rs`;
   - each test that must flip, each new scenario with its id from S078, and each
     handbook line the decisions reach. A file the plan needs outside Files touched is
     handed back.
4. **The specification for the step-1 refinements.** Q2 and the nack reset landed at
   `983b785` (Hand-back notes). No step-1 wire answer needs a rule; the eight data-model
   refinements the maintainer chose at step 1 do (Hand-back notes, "The decisions at
   step 1"). Use the `spec-change` skill. Run `just check-specs`, `just analyse-specs`
   and `just plan-spec pachinko`, diff the obligation list against step 3's, and check
   that `rg -c '^open question' docs/specs/pachinko.allium` prints 6.
   **Adversarial review** (`CONVENTIONS.md` §14): checkpoint T35-3, the specification
   diff against the quoted answers. Ask whether each rule says what its answer says,
   whether the rebased guards are still disjoint, complete and equivalent to the guards
   at `983b785`, whether any text still describes the old behaviour, and whether any
   open question other than question 1 was answered. The commands are in §14; findings go to
   `ai_tmp/review/T35-3/`, with `-o` on a path the reviewer does not write itself.
   Checkpoints are numbered in the order they run: T35-1 and T35-2 reviewed the
   specification at `a338355` and `983b785`, T35-3 is this one and T35-4 is step 8's.
5. **The red tests.** Use the `rust-change` skill. Derive the tests from the
   specification's clauses before any code changes; each carries `// scenario:` with
   its id and `// obligation:` with ids from `just plan-spec pachinko`. The designs:
   - scenarios A to D of the Q1 decision (publish v1, pull, publish v3, then ack, nack,
     nack after a further v5, or neither), each ended once by release and once by
     lapse, asserting the superseded queue and the dead letters entry by entry (rank,
     payload, reason or `nacked_by`, `shed_at` or `nacked_at`, `expires_at`), the
     record's status, rank and position, and that the held version appears where the
     reconciliation table says and nowhere else;
   - the publish table of decision 5 against a held record that is unsettled, acked
     and nacked: higher defers, a second higher sheds the deferred version at once,
     equal is rejected, lower is stale;
   - marks: a repeat succeeds and extends the lease, a change is refused and changes
     nothing, and the lease read shows each mark;
   - the two fairness cases of decision 7 and scenario C's position, with a third key
     published in between so the order is observable;
   - the pocket at the lease's end: every record acked or nacked and none deferred
     (idle on release; idle or parked on lapse); every record acked with one deferred
     version (queued); only nacks with `max_consecutive_expiries = 1` (parked with no
     records, dead letters created), since at a higher limit the nacks reset the count
     and the lapse idles the pocket;
   - a dead letter created at the lease's end with `nacked_at` at the nack and
     `expires_at` at the end plus `dead_letter_ttl`, and its decay;
   - every row of "Scenarios" in the impact map rewritten to the decided behaviour,
     S071 and S072 inverted and renamed, S045 retagged, S075 and the backward-clock
     test repaired;
   - the walk model rewritten to the new rules with all 33 invariants, so the invariant
     walk fails until the code follows;
   - Q2 as decided: S024 splits into a supersede that revives and a stale publish that
     leaves the pocket parked with its count, and drops `oq:11`;
   - the nack reset (impact map, section 8): S063's nacked case idles with a count of 1,
     S064 is inverted so the nack resets the count, the walk's model resets on a nack
     and its label "nack keeps a counted pocket" changes, and
     `a_nack_leaves_the_expiry_count_over_http` and its driver are inverted and
     renamed.

   Run `just test` and quote the failures, each red for the reason the decision gives.
   A test whose scenario keeps today's behaviour is a pinning test: on a scratch copy
   that is never committed, break that behaviour and quote the test failing.
6. **The code.** Use the `rust-change` skill. Implement along step 3's plan until
   step 5's tests pass: the library, then the service and loadgen in the same commit
   series, so no commit leaves a library refusal unmapped. Every entry created at a
   lease's end is stamped from the one `now` the release or the tick read. `Queue::nack`
   resets the count, and a stale publish no longer touches its pocket. Update the docs
   and doctests the impact map lists, among them section 8's: "ack, release or revive"
   becomes "ack, nack, release or revive", and "a nack leaves it" goes. Run
   `just fmt-check`, `just clippy`, `just test`, `just doc`, `just coverage`,
   `just effects` and `just semver`, quoting the semver report.
7. **The handbook.** Use the `review-docs` skill. Every line under "Handbook, READMEs,
   changelog" in the impact map; the catalog rows, counts and legend in
   `docs/reference/testing.md`; the terms in `docs/project/terminology.md` (held set,
   mark, deferred in its new sense, superseded record, dead letter, position);
   `CHANGELOG.md` for both crates, naming what a client observes; the handbook lines of
   section 8, and its `CHANGELOG.md` lines by hand, since the Verification pattern does
   not read the changelog. Run `just check-docs`.
8. **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T35-4, the whole diff
   before the final commit, against the quoted decisions, the Goal table, Files touched
   and the specification. Ask: whether the code does what each rule says in every arm
   of the reconciliation table, by release and by lapse, including a lapse that parks;
   whether any path sheds or dead-letters a held version while its lease is active;
   whether a nack resets the count and a stale publish touches nothing, in the library,
   the model and the wire;
   whether any test passes with the code reverted to `62efe41`'s behaviour; whether the
   walk model and the queue agree only because both were changed the same wrong way;
   whether any obligation tag names an id `just plan-spec pachinko` does not print;
   whether a scenario id is tagged twice or a catalog row names a test that does not
   exist; whether any page still describes the old behaviour; whether a public item
   changed without its doctest; whether the service maps every library outcome and
   refusal; and whether any file outside Files touched changed. Two reviewers, one lens
   each, `conformance` and `breakage`, with the harness the executing agent is not;
   separate triage writes `ai_tmp/review/T35-4/verdicts.md`; one writer applies the
   valid fixes; two rounds at most.
9. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
   `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
   `git fetch origin` (network, authorised by this step) and merge
   `origin/wave-2-tickets` (step 2, `CONVENTIONS.md` §11); `Cargo.lock` is regenerated
   with `just lock` (network,
   authorised by this step) if the merge touched it. Rerun the Verification block at the
   merge commit and requote it. Set `status: done` and T35's Status cell in
   `tickets/README.md`, and commit with the hand-back notes, each quoted output headed
   by the commit it ran at. **Authorisation required: push and open a pull request.**
   Stop and ask. The pull request's base is `wave-2-tickets`, and its merge is the
   maintainer's; `pages.yml` deploys the API reference on a push to `main` or a manual
   run (decision 0013), and a push happens when `wave-2-tickets` itself merges there.

## Acceptance criteria

- The maintainer's answers at step 1 are quoted in the hand-back with their date, and
  nothing before them chose an option.
- `docs/specs/pachinko.allium` carries no `open question` for questions 1, 9, 10 or 11;
  `rg -c '^open question' docs/specs/pachinko.allium` prints 6, and no test, page or
  message calls the equal-rank refusal a placeholder.
- Decision 0020 records why the held set lasts until the lease ends, with its index row
  and manifest entry.
- A nack resets its pocket's `consecutive_expiries` when it is processed, in the library
  and over the wire (S064 and the renamed wire test), and a renew or a pull leaves it.
- A stale publish into a parked pocket leaves it parked with its count, and records the
  arrival as `stale` (S024's stale half).
- Each row of the reconciliation table holds by release and by lapse, each pinned by a
  tagged library test; no path sheds, dead-letters or removes a held version while its
  lease is active, and no version leaves a pocket without being acked, dead-lettered or
  recorded as superseded.
- Each test whose scenario the decisions change or add was red before the code changed,
  and the hand-back quotes the failures; each pinning test is quoted failing under a
  scratch-copy mutation.
- Every `// obligation:` id in `crates/` is in `just plan-spec pachinko`'s output, and
  the hand-back lists the before and after counts.
- The scenario ids are each tagged once and run contiguously from S001 to the
  catalog's last row, and the catalog has one row per id naming an existing test.
- No catalog row, test or page carries `oq:1`, `oq:9`, `oq:10` or `oq:11`.
- The walk model predicts the decided behaviour, checks every invariant the module
  declares, and the invariant walk passes.
- The service answers the ack, the nack, the repeat, the changed mark and the lease
  read as step 1 decided; no library outcome or refusal reaches `unmapped`; loadgen
  accepts the new replies.
- The handbook pages in Files touched say what landed, and both `CHANGELOG.md` sections
  name it.
- Every new or changed public item keeps invariant 3 and has a doctest; `just coverage`
  reports at or above 90% of lines over `crates/*/src/**`; `just effects` passes; the
  `just semver` report is quoted.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` is green at the commit that merges `origin/wave-2-tickets`, and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). The
lines that must print nothing follow their positive control.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
just plan-spec pachinko >| ai_tmp/T35/plan.json; echo "exit $?"
rg -c '^open question' docs/specs/pachinko.allium; echo "exit $?"
# every obligation tag names an id the planner prints, or a @guarantee of the module
{ jq -r '.obligations[].id' ai_tmp/T35/plan.json
  rg -o '@guarantee (\w+)' -r 'guarantee.$1' docs/specs/pachinko.allium
} | sort -u >| ai_tmp/T35/ids.txt; echo "exit $?"
wc -l < ai_tmp/T35/ids.txt; echo "exit $?"
rg -o --no-filename '// obligation: ([A-Za-z][A-Za-z0-9_.-]*)\s*$' -r '$1' crates \
  | sort -u | comm -23 - ai_tmp/T35/ids.txt; echo "exit $?"
rg -c 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort | uniq -d; echo "exit $?"
last=$(rg -o '^\| S([0-9]{3}) \|' -r '$1' docs/reference/testing.md | sort -n | tail -1)
echo "last S$last"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | diff - <(seq -f 'scenario: S%03g' 1 "$((10#$last))"); echo "exit $?"
rg -o '^\| (S[0-9]{3}) \|' -r '$1' docs/reference/testing.md \
  | sort | diff - <(seq -f 'S%03g' 1 "$((10#$last))"); echo "exit $?"
# each test, half and driver every catalog row names exists, in the file its cell
# names, library and wire alike; S005 names a doctest, checked by hand
rg '^\| S[0-9]{3} \|' docs/reference/testing.md \
  | rg -o '`[^`]+\.rs`, `[a-z0-9_]+`( \(driver `[a-z0-9_]+`|; [a-z]+ half: `[a-z0-9_]+`)?' \
  | sed -E 's/^`([^`]+)`, `([a-z0-9_]+)`[^`]*`?([a-z0-9_]*)`?$/\1 \2 \3/' \
  | ( bad=0; n=0; while read -r f a b; do
        for t in $a $b; do n=$((n + 1)); rg -q "fn $t\b" "$f" || { echo "MISSING $f $t"; bad=1; }; done
      done; echo "checked $n"; [ "$n" -gt 0 ] && exit "$bad"; exit 1 ); echo "exit $?"
# every test reference on a catalog row parsed above: only S005's doctest has no name
rows=$(rg '^\| S[0-9]{3} \|' docs/reference/testing.md)
refs=$(printf '%s\n' "$rows" | rg -o '`[^`]+\.rs`, |half: |\(driver ' | wc -l)
parsed=$(printf '%s\n' "$rows" \
  | rg -o '`[^`]+\.rs`, `[a-z0-9_]+`|half: `[a-z0-9_]+`|\(driver `[a-z0-9_]+`' | wc -l)
echo "references $refs parsed $parsed"
[ "$refs" -gt 0 ] && [ "$refs" -eq "$((parsed + 1))" ]; echo "exit $?"
# positive control first: the labels of the questions still open must match (exit 0)
rg -c 'oq:[0-9]+\b' docs/reference/testing.md crates/libpachinko/tests; echo "exit $?"
! rg -n 'oq:(1|9|10|11)\b|oq9_|open.question.(9|10|11)\b' docs/reference/testing.md \
  docs/how-to crates; echo "exit $?"
# the equal-rank refusal is no longer called a placeholder (question 1 closed)
! rg -n --pcre2 '(?i)first open question|placeholder for (the specification.s )?open question 1|\(open question 1\)|open_question_1|oq1 placeholder|open question 1 placeholder|equal.rank placeholder|equalrank. placeholder|refusal is a placeholder|placeholder, not a decision|"placeholder"|is_a_placeholder' \
  crates docs loadgen README.md --glob '!docs/decisions/**'; echo "exit $?"
# the old names are gone from code and pages (the changelog and tickets keep history)
! rg -n 'SettleOutcome::(Settled|Deferred)|RecordStatus::Deferred|free_held' \
  crates loadgen docs README.md --glob '!docs/decisions/**'; echo "exit $?"
# the nack reset's flip list is gone (impact map, section 8)
! rg -U -n '\back,[\s/]*release[\s/]+or[\s/]+(publish[\s/]+)?reviv|nack leaves (it|the count)|nack keeps a counted pocket' \
  crates docs README.md --glob '!docs/decisions/**'; echo "exit $?"
just fmt-check; echo "exit $?"
just clippy; echo "exit $?"
just test; echo "exit $?"
just doc; echo "exit $?"
just effects; echo "exit $?"
just coverage; echo "exit $?"
just semver; echo "exit $?"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

The `comm` line prints the tags that name neither a planner id nor a `@guarantee` of
the module, and must print nothing; run at `e812f5d` it prints the 35 removed ids, and
its pattern matches 313 of the 315 tag lines, the other two being the placeholders in
the module doc of `crates/libpachinko/tests/queue/main.rs`. The checker gives a
guarantee no id, so its tag is `guarantee.<Name>`. The two `! rg` lines both match at
`e812f5d`, so they can fire. The line that greps for the old names is adjusted at
step 3 if the plan keeps one of those names with a new meaning. The nack-reset line is
the third `! rg`: at `983b785` it matches 13 lines, the flip list of section 8 outside
`CHANGELOG.md`, so it can fire. It reads across lines (`-U`) because one doc comment
wraps the list, and `\back,` keeps it from matching the corrected "ack, nack, release or
revive". `CHANGELOG.md` keeps released history, so step 7 checks its lines by hand.
The placeholder line was widened at step 3 to what
`rg -n 'first open question|placeholder|open.question.1\b'` found over `crates`, `docs`
(decisions excluded) and `loadgen`; at `25cdbea` it matches 26 lines, so it can fire.
It leaves out the general uses of "placeholder" (the documentation contract, the
how-to's template names, the walk's placeholder id and the tag legend) and the dated
note that closes question 1. The catalog line selects the rows, then takes every
file and name pair from them, so a wire row's library test, each driver and the second
halves of S066 and S067 are checked too; its first form matched one pair a row, the
last, and the T35-4 review (T35-B2) showed it passing with S040's library test renamed.
With that rename in a copy, the line prints `MISSING` and exits 1.

## Hand-back notes

### The decisions on Q1 (2026-11-13)

Recorded in Context, quoted and dated. The maintainer's notes and both rounds of
answers were kept verbatim in `ai_tmp/T35/decisions.md` while the specification was
written; that file is scratch and is not committed, so Context carries the quotations.

### Baseline, at `62efe41`

`just initialize` ran in the new worktree (network, authorised by the plan's approval)
and exited 0. `just check` on the untouched worktree ended:

```text
All checks passed and the worktree is unchanged.
```

`just plan-spec pachinko` printed 190 obligations.

### The specification for Q1, at `a338355`

```text
$ just check-specs
allium check: 1 specifications, no diagnostics and no findings.
exit 0
$ just analyse-specs
allium analyse: 1 specifications, no diagnostics and no findings.
exit 0
$ just plan-spec pachinko   (obligations counted from its JSON)
236 obligations; diagnostics []
exit 0
$ rg -c '^open question ' docs/specs/pachinko.allium
8
exit 0
$ just check-docs
Validated 47 pages and 48 canonical topics.
exit 0
$ just check
All checks passed and the worktree is unchanged.
exit 0
```

Against the baseline's list, 35 obligation ids are gone and 81 are new; Context names
them by kind. Scenarios A to D (by release and by lapse), the publish table, repeated
and changed marks, S071 and S072, an ack before the higher rank, the pocket's state at
the lease's end and the positions of decision 7 were traced by hand through the rules;
each matched the reconciliation table.

The library, its tests, the service and the behaviour pages were not changed and still
follow the old rules.

### T35-1: the specification for Q1

Codex `codex-cli 0.167.2`, `-m gpt-6-astra -s read-only`, on 2026-11-13, over the
uncommitted edit that became `a338355`. Round 1 ran two reviewers, one lens each; triage
and fixes were the executing agent's, as the approved plan said. The raw briefs,
findings and verdicts stayed under `ai_tmp/review/T35-1/`, uncommitted.

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| `UniqueArrivalPosition` compared a present `deferred_position` with an absent one, which is false, so two held records with one deferred version broke it (both lenses) | high | valid | The invariant requires both to be present |
| Entries created at the lease's end were stamped with the answering rule's own `now` (both lenses) | medium | valid | `Freed` carries `ended_at`, and the three rules that create an entry stamp from it |
| `NothingLostWhileHeld` promised that every newer version is queued (both lenses) | medium | valid | The highest rank to arrive is queued; a deferred version it supersedes is shed at once |
| The Position vocabulary and the `Record` comment gave every deferred version the counter's value | low | valid | The first arrival position is inherited |
| The TTL comment claimed equality for all three expiry invariants at a zero TTL | low | valid | Reworded for the dead letter |
| `MarksStandHoweverLeaseEnds` said a repeat changes nothing | low | valid | It extends the lease and changes nothing else |

Round 2, one reviewer over the fix diff, confirmed the fixes and the six lease-end
rules disjoint and complete, and raised three low findings in comments, all valid and
fixed: the Deferred vocabulary still promised queuing for every deferred version; the
TTL comment still claimed equality for `GrantedBeforeExpiry`; the `Queue` comment said
every rule that assigns a position advances the counter. Those three fixes are comment
text and were not reviewed again; two rounds are the limit. Neither round raised a
concern about the decision itself.

### The merge of T32's pull request 48 (step 2)

`git fetch origin` exited 0 and moved `origin/wave-2-tickets` from
`ba070546a948417948f35c0d0e10d9acddea3c14` to
`7a7974307e60ca744af6468af23bfa5bd019384b`, the merge of pull request 48, whose parents
are the old head and T32's final head `8f155631f6b241f051a2097fb902a6ce48486dee`. The
old head is written in full because the spell checker reads its seven-character form
as a misspelt word. `origin/main` (`611a86e`) does not contain the merge. T32's commits
after `62efe41` touch none of the files this branch had changed. `git merge --no-ff
origin/wave-2-tickets` made the merge commit `b5d4a83` with no conflict; it brought 17
files and left `Cargo.lock` unchanged, so `just lock` did not run. Seven of those files
are in Files touched and later steps edit them: `CHANGELOG.md`,
`crates/libpachinko/src/config.rs`, `crates/libpachinko/src/queue/types.rs`,
`crates/pachinko/tests/http.rs`, `docs/project/terminology.md`,
`docs/reference/http-api.md` and `docs/reference/testing.md`. At `b5d4a83`:

```text
$ git merge-base --is-ancestor 7a79743 HEAD && echo "7a79743 is ancestor"
7a79743 is ancestor
$ just check; echo "exit=$?"   (last line of its log)
All checks passed and the worktree is unchanged.
exit=0
```

`git status --short` then printed nothing.

### The decisions on Q2 and the nack reset (2026-11-16)

Recorded in Context, quoted and dated: T32's close-out decisions 4 and 5 with the
maintainer's note, the answer "No touch on stale" to Q2, and "Land spec now + ticket".

### The specification for Q2 and the nack reset, at `983b785`

`just plan-spec pachinko` printed 236 obligations at `71c65ec`, before the edit. At
`983b78554dff6d6d6e2cb78b3fd7f4a9a6de9072`:

```text
$ just check-specs
allium check: 1 specifications, no diagnostics and no findings.
exit 0
$ just analyse-specs
allium analyse: 1 specifications, no diagnostics and no findings.
exit 0
$ just plan-spec pachinko   (obligations counted from its JSON)
exit 0
236 obligations; diagnostics []
$ rg -c '^open question' docs/specs/pachinko.allium
7
exit 0
$ just check-docs
Validated 47 pages and 48 canonical topics.
exit 0
```

The ids are the same before and after; Context says what changed in the JSON. The
library, its tests, the service and the behaviour pages were not changed and still
follow the old rules.

### T35-2: the specification for Q2 and the nack reset

Codex `codex-cli 0.167.2`, `-m gpt-6-astra -s read-only`, on 2026-11-16, over the
uncommitted edit that became `983b785`. Two reviewers ran in parallel, one lens each,
with six questions: whether a stale publish still reaches `PocketTouched`, whether a
supersede, a deferral or a new key stopped reaching it, whether any text still says a
nack leaves the count or describes the old Q2 behaviour, whether any other open
question was answered, whether the lapse comment's "only when
`max_consecutive_expiries` is 1" is true, and whether the numbering comment and the
handbook's counts agree with the seven `open question` lines left. A fresh Claude
subagent triaged the findings. The raw briefs, findings and verdicts stayed under
`ai_tmp/review/T35-2/`, uncommitted.

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| The ticket still forbade the nack reset in Non-goals and listed Q2 as undecided (conformance) | medium, triaged low | valid | Not in the specification commit: this ticket's realignment, the commit after `983b785`, carries it |

The breakage reviewer reported no findings, having traced every publish path, the nack
rules, the lapse comment and the counts. No high finding was valid, so no second round
ran. Neither reviewer could run `just check` in its read-only sandbox; the checks above
were run outside it.

### The decisions at step 1

Asked on 2026-11-16, in the planning session for steps 1 to 9. The questions are
abridged; each answer is the label the maintainer chose, or their own words, quoted
verbatim. A "(Recommended)" the dialog put on a label is dropped.

| Question | Answer | What it decides |
| --- | --- | --- |
| The ack and nack reply, and whether a repeat answers differently | "acked / nacked, repeat same" | `SettleOutcome::{Acked, Nacked}` replace `Settled` and `Deferred`; the wire answers `{"outcome":"acked"}` or `{"outcome":"nacked"}`; a repeat answers as the first mark did |
| The refusal of a changed mark | "409 mark_conflict" | `LeaseActionError::MarkConflict(RecordKey)`, checked after `RecordNotHeld`; the wire answers 409 `mark_conflict` |
| The publish deferred behind a held version | "I think option 2, because the publisher does not need to know that another (lower rank version) is currently being processed. It just needs to know that its record has been published and is on track to being processed. I think this is the case. I think this is true. Unless I'm missing something, let me know." | Option 2 was "Keep `superseded`". No new outcome: the `PublishOutcome::Superseded` doc names both rules, and loadgen's `APPLIED` is unchanged. Nothing was found missing: the deferral is visible to consumers, through the lease read, and not to publishers |
| Where the mark is shown | "I guess option 1. Double check to ensure that the datamodel captured by the allium specs makes sense and is well modeled." | Option 1 was "Lease read only". A new `MarkedRecord` goes in `LeaseView`, and the lease read's records gain `"mark"`; `HeldRecord`, the pull batch and its wire are unchanged. The data-model check is below |
| A decision record for Q1 | "Go ahead and draft a decision record as part of ticket 35, because this is a key decision. But keep it fairly terse, minimum but sufficient, The spec captures the nitty gritty detail, the decision mostly needs to capture why." | Decision 0020, written here with its index row and manifest entry; the Non-goal that handed a record back is replaced |
| Whether the qsketch lines in Context are right | "You can check here:" followed by the file's path in the maintainer's xzibitq repository | Read on 2026-11-16: `MultiLeaseQueue.push` (lines 533-545) pops the pocket from `_q_in_waiting` before `KeyedQueue.push` runs its `duel`, so any publish revives a waiting pocket. The Context lines were right and now say they were checked |

**The data-model check.** An independent review of the specification's data model found
it sound, with nothing reachable contradicting a decision, and raised refinements. Asked
to choose among them, the maintainer answered: "Please ask me about each of these
individually. Provide more context. and /show-me Where valuable, what they're about
through visuals and diagrams that can better communicate and build understanding and
alignment." Each was then asked on its own, with a diagram:

| # | Refinement | Answer |
| --- | --- | --- |
| 1 | Tell a version waiting behind a held one by `deferred_position != null`, not `held_rank < rank`, in `ending_records`, `Supersede`, `DeferBehindHeldVersion` and the six lease-end rules. `DeferredHasArrivalPosition` and `OnlyDeferredHasArrivalPosition` stay, since they tie the ranks to the position | "Rebase on position", with the note "we will continue to reject requal rank versions." (read as "equal") |
| 2 | A `transitions mark { unsettled -> acked; unsettled -> nacked }` block, with `nacked_at` made `when mark = nacked`, retiring `NackedHasItsTime` and `OnlyNackedHasNackTime`; dropped and recorded if the pinned checker rejects it | "Try it; drop if rejected" |
| 3 | An invariant `HeldCopiesAgree`: a held record with nothing deferred has `held_payload = payload` | "Add it" |
| 4 | An invariant `MarkedPocketHasNoExpiries`: a held, marked record's pocket has `consecutive_expiries = 0`, which the lapse comment relies on | "Add it" |
| 5 | Guard `Supersede` explicitly: `status = queued or (status = held and deferred_position != null)` | "Guard explicitly" |
| 6 | Say under Lease end that ending a lease and answering its `Freed` are one outcome, as a pull and its `HoldRecord` are | "State it" |
| 7 | Say in the Publish comment that publishing again at the held rank is rejected when nothing waits and stale when a newer version waits, with a library test pinning both | "Document it" |
| 8 | Whether the note on refinement 1 closes the module's open question 1 as "reject" | "Yes, close it: reject" |

Step 1 says an answer outside the offered options is a new design, to be recorded before
stopping again. Three answers went outside them: the decision record written in this
ticket, the refinements, and question 1 closed. The maintainer approved the plan that
carries all three on 2026-11-16, so the work goes on; the Goal, Non-goals, Files
touched, step 4, Acceptance criteria and Verification were realigned with them in the
commit after `25cdbea`. That leaves six `open question` lines, questions 2 to 7, since
the numbering never compacts.

### Baseline, at `25cdbea`

`just check` on the untouched worktree, before this realignment, ended:

```text
All checks passed and the worktree is unchanged.
```

### The plan (step 3)

Planned on 2026-11-16 and approved with the step-1 answers; a design subagent proposed the seams and the lead
adopted them. Decided before any test:

- **The store seam.** `PocketRecords` gains `defer_behind_held`, `mark` and `end_lease() -> Vec<EndedVersion>`;
  `settle`, `defer` and `free_held` go; `supersede` returns `None` for a held record with nothing deferred, so it
  never sheds a held version; `held()` returns `MarkedRecord`s. `RecordStatus::Held` carries `mark`, `nacked_at`
  and `deferred_position`, and `RecordView::new` keeps five parameters. `MemoryRecords` keys its held index by
  `(Position, RecordKey)`, so `end_lease` costs O(held × log n).
- **The lease-end plumbing.** No context goes down: `Pockets::release` and `Pockets::lapse` return the versions a
  lease's end removed, and `keep_ended`, beside `authorise` in `queue/renew.rs`, stamps them from the one `now` the
  release or the tick read.
- **The order at one lease end**, which the specification leaves open: smallest held position first, pinned in
  the store contract and by S091, and written in `docs/reference/testing.md`.
- **`AckRepeats` and `NackRepeats`** skip the count reset, as the rules do; the count is 0 under that lease
  either way (`MarkedPocketHasNoExpiries`). A repeat nack keeps the first `nacked_at` (S086).
- **Ack and nack** run every guard before reading the clock, so a refused mark reads no clock.
- **The walk's split.** The model moves to `crates/libpachinko/tests/queue/walk/model.rs`; `free` becomes
  `freed`, `Turn.settled` goes, and the model stops touching on stale.
- The tests to flip and add, the new scenarios from S078, and the handbook lines are those of steps 5 and 7.
  Files outside Files touched: decision 0020 and its index and manifest entries,
  `docs/operations/troubleshooting.md`, `walk/model.rs` and `http/lease_read.rs`, each added to Files touched at
  the step that needed it.

### The specification for the step-1 refinements, and T35-3 (step 4)

No step-1 wire answer needed a rule. The refinements did, and landed at `d84ba92e589f76a59d350d699209cf267fab2d58`:

- Refinement 1: `ending_records`, `Supersede`, `DeferBehindHeldVersion` and the six lease-end rules tell a
  deferred version by `deferred_position != null` (or `= null`); `DeferredHasArrivalPosition` and
  `OnlyDeferredHasArrivalPosition` are unchanged. Each rule keeps its clauses in their order, so the
  `rule-failure` ids keep their numbers.
- Refinements 3 and 4: `Record.HeldCopiesAgree` and `MarkedPocketHasNoExpiries`, cited in the lapse comment.
- Refinement 5: `Supersede` requires `record.status = queued or (record.status = held and
  record.deferred_position != null)`.
- Refinements 6 and 7: the Lease end comment says a lease's end and its `Freed` rules are one outcome; the
  Publish comment says a publish at the held rank is rejected when nothing is deferred and stale when a newer
  version is.
- Refinement 8: the Publish comment states the equal-rank rejection as decided on 2026-11-16, closing question 1;
  its `open question` line is gone, and the numbering comment says the six lines left are questions 2 to 7. Both
  handbook lists drop it.
- Refinement 2 was dropped whole. A `transitions mark { unsettled -> acked; unsettled -> nacked }` block, with
  or without `terminal: acked, nacked`, makes the pinned checker report Record's status edges as undeclared:

  ```text
  allium.status.undeclaredTransition 178 Rule produces transition 'queued' → 'held' on entity 'Record', but this edge is not in the declared transition graph.
  allium.status.undeclaredTransition 178 Rule produces transition 'held' → 'queued' on entity 'Record', but this edge is not in the declared transition graph.
  ```

  The language reference allows two graphs on one entity (its `Document` example), so this is the checker's
  limit, not the model's: allium 3.6.1 keeps one graph per entity. The other half, `nacked_at: Timestamp when
  mark = nacked`, passed the checker but breaks the reference's rule 7g without the graph (T35-3, below), so
  `nacked_at` stays `Timestamp? when status = held` with `NackedHasItsTime` and `OnlyNackedHasNackTime`.

At `d84ba92e589f76a59d350d699209cf267fab2d58`:

```text
$ just check-specs
allium check: 1 specifications, no diagnostics and no findings.
$ just analyse-specs
allium analyse: 1 specifications, no diagnostics and no findings.
$ just plan-spec pachinko   (obligations counted from its JSON)
238 obligations
$ rg -c '^open question' docs/specs/pachinko.allium
6
$ just check-docs
Validated 47 pages and 48 canonical topics.
```

Against `25cdbea`'s 236 ids, two are new and none is gone: `invariant.MarkedPocketHasNoExpiries` and
`invariant.Record.HeldCopiesAgree`. The module declares 35 invariants.

### T35-3: the specification for the step-1 refinements

Codex `codex-cli 0.167.2`, `-m gpt-6-astra -s read-only`, on 2026-11-16, over the uncommitted edit with
refinement 2's `nacked_at` half still in it. Two reviewers ran in parallel, one lens each, with the step-4
questions: each rule against its answer; the rebased guards disjoint, complete and equivalent to `983b785`'s;
the two new invariants and the `when mark = nacked` field in every reachable state; stale text and the question
counts; any other question answered. A fresh Claude subagent (`claude-opus-5-5`) triaged. The raw files stayed
under `ai_tmp/review/T35-3/`, uncommitted.

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| `nacked_at: Timestamp when mark = nacked` names a field with no `transitions` block, which the language reference's rule 7g forbids; the pinned checker accepts it, so the gate hid it (both lenses) | medium | valid | Refinement 2 reverted whole, as above, before the commit |

Both reviewers found the rebased guards equivalent to `983b785`'s under `HeldVersionNotNewer` and the two
arrival-position invariants, the six `Freed` rules disjoint and complete, `HeldCopiesAgree` and
`MarkedPocketHasNoExpiries` true in every reachable state reading a lease's end as one outcome, the text and
counts right, and no other question answered. No high finding was valid, so no second round ran. The triage also
asked for the Goal's Rules row, which still named the dropped graph, to be corrected, and a long line rewrapped;
both are in `d84ba92e589f76a59d350d699209cf267fab2d58`.

### Shown to bite (step 5)

The red tests were written against the skeleton at `11e5bb6`, which added the new types beside the old ones with
neutral bodies, so every red test compiled and failed on behaviour. Four agents wrote them, one after another
where they shared a test binary. Their full reports, with each first failure, stayed under `ai_tmp/T35/red/`,
uncommitted. `NEXTEST_PROFILE=ci just test` at `11e5bb6` plus the red tests ran 265 tests: 213 passed and 52
failed, each for the reason the decision gives:

- 38 library scenario tests, among them S040 (`Ok(Settled)` where `Ok(Acked)` is due), S043 and S082
  (`deferred_position: None` where the publish defers), S045 and S076 (the held v1 shed while held), S047, S087
  and S089 (a dead letter before the lease ends), S053 (the ack removed the record), S064 (the nack left the
  count at 1), S071 and S072 (`Ok(NewKey)` where the key waits), S074 and S007 (decay before the lease ends),
  S075 (the publish at the held rank was not `EqualRank`), S078 to S081 (scenarios A to D), S085
  (`RecordNotHeld` where `MarkConflict` is due), S091 (two entries shed while held) and S092 (a stale publish
  revived the parked pocket);
- the 2 walk tests: the shrunk case nacks a held record, and the queue answers `Ok(Settled)` where the model
  answers `Ok(Nacked)`;
- 8 wire tests: `"settled"` where `"acked"` or `"nacked"` is due (the S040, S047, S068 and S077 drivers), 404
  `record_not_held` where 409 `mark_conflict` is due, `no_pocket` where the nack reset keeps the pocket queued,
  and the `EqualRank` message calling itself a placeholder (the wire test and a service unit test);
- 4 store tests: the contract (and its run through the counting store) at "supersede of a held record with nothing
  deferred changes nothing", the held index in key order rather than position order, and the index not emptied by
  `end_lease`.

Pinning tests, each shown failing under a scratch mutation of one library file, restored with `cp`, and checked
with `git diff --quiet`:

| Test | Mutation | Failure |
| --- | --- | --- |
| S041 | the ack no longer extends the lease | `Timestamp(1000010)` vs `Timestamp(1000014)` |
| S049 | the lease extended before the `RecordNotHeld` guard | "a refused action changed the queue" |
| S017, S025 | `rank < stored.rank` became `<=` | `Ok(Stale)` vs `Err(EqualRank{..})` |
| S024 | `Supersede` returned before touching | `(Parked, 0, 3)` vs `(Queued, 5, 0)` |
| S060 | requeued records left out of the queued index | `[]` vs `["a1", "a2"]` |
| S070 | the lease check skipped the owner | `unwrap_err()` on an `Ok` |
| S027 | `hold_oldest` set `Acked`, then a `deferred_position` | `mark: Acked` vs `Unsettled`; `Some(Position(1))` vs `None` |
| the backward-clock test, repaired | `Queue::now` without its clamp | `Ok(1000010)` vs `Ok(1000110)` |
| S055 | the release never idled | `Queued` vs `Idle` |
| S061 | the lapse never idled | `(Queued, 1)` vs `(Idle, 1)` |
| S037 | the store's `supersede` requeued a held record | `["r0", "r1"]` vs `["r1"]` |

**A review of the red tests**, asked for by the maintainer on 2026-11-16 ("When you are done or at any key moment,
request an adversarial review from Codex (Astra)") and run before the code: Codex `codex-cli 0.167.2`,
`-m gpt-6-astra -s read-only`, one reviewer, on 2026-11-17, over `git diff -- crates` at `11e5bb6`. It found no
wrong test, and five gaps; the lead triaged them, since this review is not one of the ticket's checkpoints. The
raw files stayed under `ai_tmp/review/T35-red/`.

| Finding | Severity | Verdict | Fix, at `8be2162` |
| --- | --- | --- | --- |
| Same-tick decay after a lapse untested | medium | valid | `with_zero_ttls_a_lapses_entries_decay_in_the_tick_that_made_them`; with decay moved before the lapses it fails `(0, 0)` vs `(1, 1)` |
| Lease-end stamps from one `now` untested under an advancing clock | medium | valid | `a_lease_ends_entries_are_stamped_from_its_one_ended_at`, with a clock that moves on every read; with `keep_ended` reading the clock per entry it fails `(Timestamp(1006), Timestamp(6006))` vs `(Timestamp(1005), Timestamp(6005))` |
| The pull batch's wire shape unprotected | low | valid | the batch record asserted exactly, with no `mark` |
| S024 tagged `rule-failure.ReviveParkedPocket.1`, unreachable by public triggers (predates T35) | low | valid | moved to the walk's unreachable-guard list |
| S069 tagged `rule-failure.Renew.2` but exercises `Renew.1` (predates T35) | low | valid | retagged |

### The code (step 6)

`f927967` holds the code with the red tests, and `8be2162` the review's fixes and the `EqualRank` tests asserting
the decided wording. At `8be2162`:

```text
$ NEXTEST_PROFILE=ci just test
267 tests run: 267 passed; doctests 108, 4 and 27 passed
$ just fmt-check; just clippy; just doc; just effects; just metrics; just test-loadgen; just snapshots-check
each exit 0; effects: libpachinko names none of 8 banned crates and uses none of 121 banned std APIs
$ just coverage
99.13% of lines
```

The walk's generator weights changed (`1,1,8,6,4,3,2,2,5,1,3` to `1,1,10,7,5,4,1,3,6,1,4`) because two labels
were never reached under the old ones; every label is reached now, the rarest in 2 of 512 walks with the fixed
seed. The `compile_fail` doctest still fails only with E0277: a scratch copy with `Arc` compiles.

Obligation tags in `crates/`: at `25cdbea`, 318 tag lines over 198 distinct ids, 35 of them ids the planner no
longer prints; at `8be2162`, 450 lines over 248 ids, all of them planner ids or guarantees, and every planner id is
tagged. The scenarios run S001 to S092.

`just semver` against its default baseline, `611a86e` on `main`, checks nothing, since that library has only
`version()`. Against `25cdbea`, before this ticket's code (exit 100, advisory before 1.0):

```text
--- failure enum_variant_missing: pub enum variant removed or renamed ---
  variant RecordStatus::Deferred
  variant SettleOutcome::Settled
  variant SettleOutcome::Deferred
--- failure trait_method_added: pub trait method added ---
  trait method libpachinko::records::PocketRecords::defer_behind_held
  trait method libpachinko::records::PocketRecords::mark
  trait method libpachinko::records::PocketRecords::end_lease
--- failure trait_method_missing: pub trait method removed or renamed ---
  method settle of trait PocketRecords
  method defer of trait PocketRecords
  method free_held of trait PocketRecords
     Summary semver requires new major version: 4 major and 0 minor checks failed
```

### The handbook (step 7)

At `3d209d7`, with the `review-docs` skill: every page of impact-map sections 6 and 8, the question-1 lines, both
`[Unreleased]` sections of `CHANGELOG.md`, and decision 0020 with its index row and manifest entry. The catalog in
`docs/reference/testing.md` runs to S092 with 35 invariants; the `oq:1`, `oq:9`, `oq:10` and `oq:11` tags are gone
and S026's `oq:7` stays; the Q1 scenarios carry `decision:0020`; "What the specification leaves open is pinned"
states the order at one lease end. `docs/project/purpose-and-scope.md` still listed equal-rank publishes as open
and was corrected, so it joined Files touched.

```text
$ just check-docs
Validated 48 pages and 49 canonical topics.
```

The changelog's lines on section 8's list, checked by hand, since the Verification pattern does not read the
changelog: libpachinko's reset bullets (ack only, "a nack ... leaves the count"), the 25 invariants and 76
scenarios (now 35 and 92), "ack, release or revive", the settled and deferred outcomes and "a nacked payload";
pachinko's "a nack does not". `CHANGELOG.md` has no released section yet, so they were corrected in place.

### T35-4: the whole diff (step 8)

Codex `codex-cli 0.167.2`, `-m gpt-6-astra -s read-only`, on 2026-11-17, over `git diff 71c65ec..HEAD` at
`48cedc1`. Two reviewers ran in parallel, one lens each, with step 8's questions and three more: whether a refused
ack or nack reads the clock, whether the order at one lease end matches the pin in the store, the queue, the model
and the docs, and whether `end_lease`'s cost is bounded by the held set. A fresh Claude subagent
(`claude-opus-5-5`) triaged the findings and applied the valid fixes one at a time, at `6abd0a6aa1ae288ebf232663a926cf74e15ebefb`. The raw files
stayed under `ai_tmp/review/T35-4/`, uncommitted.

| Finding | Severity | Verdict | Fix |
| --- | --- | --- | --- |
| No test parks a pocket at `max_consecutive_expiries` 1 while a marked record has a version deferred behind it; the walk fixes the limit at 2 and a mark resets the count (breakage) | medium | valid | S093, for an ack and a nack: the deferred version is kept, queued at its arrival position, and the nacked one is dead-lettered. A mutation writing the held version back over the deferred one fails only S093 |
| The catalog check in Verification read only the last file and test of each row, so a missing library test on a row with a wire test passed (breakage; predates T35) | low | valid | The check selects the rows and then every pair, driver and half on each; it checks 120 names and fails closed on none |
| `README.md` said a pocket with no records left goes idle when its lease ends; at limit 1 the lapse parks it (conformance) | low | valid | Qualified |
| Decision 0020 said a publish onto a held key sheds nothing; a second higher rank sheds the deferred version (both lenses) | low | valid | Narrowed to the first higher rank with nothing deferred |
| `docs/explanation/prior-art.md` still spoke of a deferred record state (breakage) | low | valid | "the held record with its deferred successor" |

Both lenses found no defect in the code: every arm of the reconciliation table by release and by lapse, no held
version shed, dead-lettered or removed while its lease is active, the nack reset and no touch on stale in the
library, the model and the wire, every tag a planner id, every outcome and refusal mapped, no file outside Files
touched, guards before the clock, and the order and cost as pinned. No high finding was valid, so no second round
ran. Neither reviewer could run `just check` in its sandbox. At `6abd0a6aa1ae288ebf232663a926cf74e15ebefb`:

```text
$ NEXTEST_PROFILE=ci just test
268 tests run: 268 passed, 0 skipped
$ just clippy; just fmt-check; just check-docs
exit 0 each; Validated 48 pages and 49 canonical topics.
```

### Step 9: review, merge and gate

**`review-change`**, run by a fresh Claude subagent over `71c65ec..HEAD` at `6abd0a6aa1ae288ebf232663a926cf74e15ebefb`, step by step: no high or
medium finding. The spec matches the code rule by rule; invariant 3 holds for `Mark`, `MarkedRecord`,
`EndedVersion` and `RecordStatus::Held`; every new public item has an asserting doctest; `just effects` passes and
libpachinko's diff adds no direct effect; no scope creep. Two lows, both fixed at `4854fc1`: the architecture page
said the store decides nothing, and the `Queue::lease_view` doc and doctest did not show the mark.

**`code-review`**, at high effort over `71c65ec..HEAD`: no correctness bug. Of its eight findings, two were fixed at
`4854fc1` and six declined:

| Finding | Disposition |
| --- | --- |
| Whether a publish touches its pocket was decided by `outcome != Stale` | Fixed: an exhaustive match, so a new outcome must say whether it touches |
| `Pockets::publish` cloned the pocket key on every publish | Fixed: the publication is taken apart once |
| `RecordStatus::Held` keeps `mark` and `nacked_at` apart, so a store could hold `Nacked` with no time and drop its dead letter | Declined: the approved shape; `MemoryRecords` sets both together behind private state, the contract pins it, and the walk checks `NackedHasItsTime` after every step |
| The store, not the queue, picks the `Freed` row | Declined: the approved design (step 3) |
| `mark_of` and `raise` clone payloads through `lookup` | Declined here: the fix widens the public `PocketRecords` trait (Open points) |
| `LeaseRecordReply` repeats `RecordReply`'s encoding | Declined here: a refactor beyond the ticket; the wire tests assert both shapes exactly (Open points) |
| Three fallback arms in `MemoryRecords` are unreachable | Declined: the private held index holds only held keys, so the current API cannot reach them |
| The first-mark guard is checked in the queue and in the store | Declined: `PocketRecords::mark` promises a direct store call keeps the first mark, and the contract pins it |

**A final review**, asked for by the maintainer ("When you are done or at any key moment, request an adversarial
review from Codex (Astra)"): Codex `codex-cli 0.167.2`, `-m gpt-6-astra -s read-only`, on 2026-11-17, over
`48cedc1..HEAD` at `8020a17`, with the six declines to judge. It found no high or medium defect, judged all six
declines sound (the unreachable arms by reachability, not by the coverage figure first given), and raised three
lows, all fixed in the closing commit: the catalog check skipped a test reference it could not parse, now
cross-checked by counting every reference against those parsed (a renamed reference fails it, in bash and zsh);
the architecture sentence still opened with "the queue decides every rule"; and this section was empty. The raw
files stayed under `ai_tmp/review/T35-final/`, uncommitted.

**The merge.** `git fetch origin` exited 0 and left `origin/wave-2-tickets` at
`7a7974307e60ca744af6468af23bfa5bd019384b`, which `b5d4a83` already merged, so
`git merge-base --is-ancestor origin/wave-2-tickets HEAD` held and there was nothing to merge; `Cargo.lock` was
untouched and `just lock` did not run.

**The Verification block**, run whole at `8020a17` with `set -o pipefail` (each `echo "exit $?"` printed 0):

```text
just check-specs     allium check: 1 specifications, no diagnostics and no findings.
just analyse-specs   allium analyse: 1 specifications, no diagnostics and no findings.
just plan-spec       exit 0
open questions       6
ids.txt              249 lines (238 obligations and the module's guarantees)
comm (unknown tags)  nothing
scenario tags        no duplicates; last S093; tags and catalog rows both S001 to S093
catalog references   checked 120
oq: positive control docs/reference/testing.md:1 (S026's oq:7)
four ! rg lines      nothing printed
just fmt-check, just clippy, just doc, just effects   exit 0
just test            268 tests run: 268 passed, 0 skipped; doctests 108, 4 and 27
just coverage        lines 99.13%
just semver          202 checks: 202 pass, 58 skip; no semver update required (baseline 611a86e, see step 6)
just check-docs      Validated 48 pages and 49 canonical topics.
just check           All checks passed and the worktree is unchanged.
git status --short   nothing
```

The reference count the final review added, run on the closing commit's catalog, printed
`references 121 parsed 120` and exit 0. The closing commit changes only this ticket, `tickets/README.md`, the
architecture page's sentence and Verification's new lines. `just check` runs again on it before the hand-back,
and its last line goes to the maintainer with the hand-back, since a commit cannot quote a run on itself.

## Open points

- **Resolved: step 1's answers**, given on 2026-11-16 and quoted in the hand-back.
- The specification leaves the order of several entries created at one lease end open;
  step 3 pins one. Whether the specification should fix it is the maintainer's.
- A pocket can park with no records, now only at `max_consecutive_expiries` 1, since an
  ack or a nack under the lease resets the count. Parking it then is the old rule's
  behaviour for an emptied pocket and was not revisited; the module's question 7 asks
  what else revives a parked pocket.
- The pinned checker (allium 3.6.1) keeps one transition graph per entity, so the
  `mark` graph of refinement 2 could not land, and `nacked_at` stays `when status = held`
  with two invariants standing in for `when mark = nacked`. Whether to land it after a
  checker upgrade, or behind a waiver, is the maintainer's.
- Declined from step 9's `code-review`, for a later ticket if wanted: `Pockets::mark_of` and the publish path
  clone a record's payloads through `lookup` only to read its mark or rank (a narrower store query would widen
  the public `PocketRecords` trait), and the lease read's record reply repeats the batch reply's encoding.
- The W3 figures in decision 0019 were measured with the ack removing its record; the
  cost moves to the lease's end. A decision record is history and is not edited;
  whether to re-measure is the maintainer's.
- `CHANGELOG.md` says "76 scenarios" where `docs/reference/testing.md` says 77; the
  mismatch predates this ticket and step 7 corrects it with the new count.
- T32 merged on 2026-11-16 as pull request 48 into `wave-2-tickets`, and that merge is
  in this branch at `b5d4a83`. Its inherited items were settled first: the maintainer
  accepted its out-of-scope edits as recorded and had one §14 round run over `2681bd6`
  (T32's Close-out, decisions 1 and 2). `wave-2-tickets` is not yet in `main`; when it
  lands there is the maintainer's.
- `docs/explanation/prior-art.md` still counts the closed questions as at T31: :229-230
  say questions 10 and 11 came later, and :415 names "the one open question that is no
  longer in the module", stale since `a338355`. Step 7 corrects them.
- **Resolved: the qsketch lines**, read again on 2026-11-16 and right (hand-back, step 1).
- T32's Open points that this ticket inherits or leaves, as T32's ticket records them
  at its merge:
  - The comment above `PublishNewKey` was corrected at `983b785`.
  - **Resolved: the nack reset.** On 2026-11-13 the maintainer decided that a nack
    resets `consecutive_expiries` and that the change lands in T35 (T32's Close-out,
    decisions 4 and 5), while this ticket's Non-goals still excluded it. On 2026-11-16
    the maintainer had it landed: the Non-goal is gone, the reset is in the
    specification at `983b785`, and section 8 of the impact map carries T32's flip list
    for steps 5 to 7.
  - The four handed-back wording and test points (the "consecutive" wording, an HTTP
    test of the ack's reset, the `lease_not_active` sentence in
    `docs/reference/http-api.md`, the `spec-change` skill's "placeholder" wording) were
    fixed on T32 under its decision 7 and arrived with the merge.
  - T32's remaining Open points name their own owners and are not this ticket's.
