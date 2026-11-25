---
id: QM08
title: "The untimed lapse and parking"
status: todo
depends_on: [QM07]
parallel_with: [QM10, QM12]
branch: ticket/qm08-untimed-lapse-and-parking
estimated_size: M
---

# QM08: The untimed lapse and parking

## Context

After QM07 the model in `quint/pachinko.qnt` (created by QM02) holds 25 of the Allium's
32 rules. A lease ends in one way only, by a release, and a pocket is queued, leased or
idle. Nothing lapses, so no pocket parks, `consecutive_expiries` is not in the state, and
the reset an ack or a nack makes to it is a line under "Not yet modelled" in the model's
header comment (QM06 wrote it).

This ticket adds the other way a lease ends. Ruling 5 of
`.scratch/quint-model/README.md` fixes its form: an untimed lapse, "any active lease may
lapse at any moment". The clock, the timestamps, the TTLs and decay wait for QM09, which
waits for the clock model that `tickets/T42-clock-model.md` has ruled.

What the Allium says, at commit `cf1f3f3`. Read each at the ticket's base, not here
(shared rule 1):

- The Lease expiry comment (above `rule LeaseLapses`). The clock emits `LeaseLapsed` once
  per active lease whose `expires_at` has passed. "The three rules below process it with
  disjoint guards." "position is untouched on the queued path, so the pocket keeps its
  place at the front."
- `LeaseLapses`: `when: lease: Lease.expires_at <= now`, `requires: lease.status = active`,
  `ensures: LeaseLapsed(lease: lease)`.
- `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` and
  `LeaseExpiryIdlesEmptyPocket` answer `LeaseLapsed`. Each requires
  `lease.status = active`, `lease.expires_at <= now` and `pocket.status = leased`. They
  split on `pocket.consecutive_expiries + 1 >= config.max_consecutive_expiries`, and
  below the limit on `pocket.records.count` against `pocket.ending_records.count`. The
  parking rule has no records guard. Each ends the lease as `expired`, emits `Freed` for
  every held record, and adds 1 to the count.
- The comment above `LeaseExpiryIdlesEmptyPocket`: a pocket whose records were all acked
  or nacked "parks only when max_consecutive_expiries is 1".
- `ReviveParkedPocket` answers `PocketTouched` for a parked pocket: queued, a fresh
  position, the count back to 0. The comment above `ReviveIdlePocket` says only a publish
  revives a parked pocket. The comment above `PublishNewKey` says a stale publish and a
  publish at the stored rank leave a parked pocket parked, with its count.
- The comment under "Ack and nack": `AckMarksRecord` and `NackMarksRecord` reset the
  count to 0; `AckRepeats`, `NackRepeats`, a renew and a pull leave it.
- The invariants `ExpiriesWithinLimit` (on `Pocket`), `MarkedPocketHasNoExpiries` and
  `ExpiryLimitPositive`, and the `Consuming` guarantees `MarksStandHoweverLeaseEnds` and
  `ExpiryRecoversWithinTtl`.

Three things the surrounding work fixes for this ticket:

- **The six lease-end rules exist already.** QM05 to QM07 own the six rules that answer
  `Freed`, and the release runs them through one definition. The Lease end comment says
  "The rules read the mark and never how the lease ended". So the lapse calls that one
  definition and transcribes none of the six a second time.
- **QM10 may run in parallel, and so may QM12** once QM10 has merged (README, "Parallel
  pairs"). QM10 creates `replayStep`, the fixtures under
  `quint/traces/` and the command that regenerates them, `just quint-traces`. `lapse`
  has no `WalkStep` (the walk in `crates/libpachinko/tests/queue/walk.rs` has `Tick`,
  which drains every due lease), so it joins `step` only and never `replayStep`. QM11
  maps lapses to the Rust. Whichever of this ticket and QM10 or QM12 merges second
  rebases, and applies shared rule 11 to what the rebase changed: the fixtures are
  regenerated and both
  replays stay green, the Rust test in `just test` and, once QM12 has landed,
  `just python-double`. This ticket gives the model one field a replay must map,
  `consecutive_expiries`, and no step a replay sees.
- **T42's specification ticket.** It changes when a lapse is due. While the model has no
  clock that changes nothing here (shared rule 7), provided the untimed lapse still
  covers what the new text allows. One point T42 puts to the maintainer could break
  that: "The lapse intent" in `tickets/T42-clock-model.md`, under which an overdue lease
  would end only when another consumer takes its pocket, and the pocket would be
  claimable while still leased. That is a different behaviour, not a different time. If
  it is ruled, the ticket that lands it changes this model in the same change (shared
  rule 11). If it has landed at this ticket's base, **stop** at step 2 and ask.

Read these:

- `.scratch/quint-model/README.md`: rulings 4 and 5, both blocks of "Design fixed before
  QM01", "What each ticket owns", "Coverage a later ticket adds", and claim 12.
- `docs/specs/pachinko.allium`: everything listed above, the `Pocket` and `Lease`
  entities, the config block, and the Lease end comment with its table.
- `quint/pachinko.qnt` (created by QM02, last changed by QM07), `quint/verify.txt` and
  `scripts/verify-quint.sh` (created by QM03), every patch under `quint/mutants/`, and
  the decision record QM02 wrote as decision 0022 under `docs/decisions/`.
- `.scratch/quint-model/QM03-verify-recipe-and-ci.md`, "Design proposed", points 1, 2,
  3, 5 and 6, with its hand-back for what the maintainer ruled: how a witness and a
  mutant are run, a patch's header, the bound, and how the recipe reads the model.
- `.scratch/quint-model/QM05-pull-and-held-set.md`, for how a config bound is checked
  and for `ExclusivePocket`; `.scratch/quint-model/QM06-ack-nack-renew-lease-end.md`,
  for the lease-end definition, the parts of a guarantee and its "Not yet modelled"
  lines; `.scratch/quint-model/QM07-deferral.md`, for the mutants that cut inside the
  lease's end. Read each hand-back for the names that landed.
- `crates/libpachinko/tests/queue/walk.rs`, `config()` (lines 59 to 67 at `cf1f3f3`), for
  `max_consecutive_expiries = 2`. Read it for the value only.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

The model can lapse any active lease at any step. A lapse fires `LeaseLapses` and
exactly one of the three expiry rules, runs the lease's end through the definition the
release uses, and can park a pocket, which a publish revives. `consecutive_expiries` is
in the state with every write the Allium gives it. The three invariants and the
guarantee below are checked properties, each owned rule has its witness reached, and
the eight mutants are caught, all by `just verify-quint`.

The ticket also states, in the model's header comment and in decision 0022, why an
untimed lapse covers every timed behaviour, which properties that argument does not
carry to, and which properties cannot be stated until QM09.

### Owns

| Kind | Names |
| --- | --- |
| Rules | `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket` |
| Invariants | `ExpiriesWithinLimit`, `MarkedPocketHasNoExpiries`, `ExpiryLimitPositive` |
| Guarantees | `MarksStandHoweverLeaseEnds` |
| Partial or extended, not counted | The untimed part of `ExpiryRecoversWithinTtl`, which QM09 owns. The six lease-end rules, called from a lapse; QM05 to QM07 own them. The writes to `consecutive_expiries` added to `PublishIntoNewPocket`, `AckMarksRecord`, `NackMarksRecord`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket` and `ReviveIdlePocket`, with their effects; QM02, QM04 and QM06 own those rules |

Owning `MarksStandHoweverLeaseEnds` is not checking it in full: QM09 adds "a repeat
extends the lease" (README, "Coverage a later ticket adds").

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them at the **Stop** in step 4,
with checkpoint 1's findings in hand, and the hand-back records the ruling. After that
the builder does not reopen them. They are written against the recommended answers of
QM02, QM03, QM05 and QM06; where a ruling there went another way, this ticket follows
the ruling and the plan says what that changes.

### The transcription

- **State.** Each pocket gains `consecutive_expiries`, everywhere a pocket is made or
  written out: QM02's seed had two pockets and no count, so `init`, and `replayInit` if
  the tree holds it, gain the field wherever they still build a pocket, at 0. The pocket's
  status gains `parked` and the lease's status gains `expired`, where the base does not
  have them already. `max_consecutive_expiries` is a `const` of `pachinko`, and
  `pachinko_walk` sets it to 2, the walk's value.
- **One action, `lapse(lease)`, in `step` only.** Its argument ranges over every lease
  identity the base's representation has. Under QM02's recommended answer that is the
  indices of the list of leases and the one index past its end, so ended leases and one
  never granted are included. `last` records the step and its lease, in a new variant
  of `Step` spelt as the base spells the others. A lapse of a lease that is not active
  changes nothing and leaves `fired` empty (shared rule 4). A lapse of an active lease,
  in one step: ends the lease as `expired`; runs the base's lease-end definition over
  the pocket's held records, smallest position first, as the release does; adds 1 to
  the count; and sets the pocket's status by the expiry rule whose guard held. `fired`
  then holds `LeaseLapses`, that one expiry rule, and each lease-end rule that answered
  a `Freed`.
- **Guards.** `guard_LeaseLapses` is `lease.status = active`. The three expiry guards
  are the Allium's `requires` lines without `lease.expires_at <= now`.
  `pocket.status = leased` stays in each: the Allium does not label it a restatement
  (`tickets/T42-clock-model.md`, open point 5, is where that is ruled).
- **"Not yet modelled".** This ticket deletes every line of the header's list that names
  QM08 (shared rule 8). Earlier tickets' texts propose three: QM02's for the lapse,
  parking and `consecutive_expiries`, QM04's for `ReviveParkedPocket`, and QM06's, "the
  reset of consecutive_expiries by AckMarksRecord, NackMarksRecord,
  ReleaseIdlesEmptyPocket and ReviveIdlePocket (QM08)". Step 2 copies the lines as they
  landed. A line that names QM08 beside another ticket is edited so that it no longer
  names what this ticket adds. It adds two, each one line of the header comment holding
  this text. If the base's list already covers the lease's timestamps, the second is
  folded into that line and the plan says so.

  ```text
  the lapse's time condition, lease.expires_at <= now, in LeaseLapses and the three expiry rules (QM09)
  ended_at on an ended lease (QM09)
  ```

- **Disjoint and covering.** For an active lease whose pocket is leased, exactly one
  expiry guard holds. The first split is `count + 1 >= limit` against
  `count + 1 < limit`. Below the limit the second is `records.count` greater than
  `ending_records.count` against equal to it, and the ending records are among the
  records, so one of the two holds. `effect_LeaseLapses` carries this as a check,
  because it is what "emits `LeaseLapsed`" comes to when the chain is one action: if
  `fired` holds `LeaseLapses`, then `fired` holds exactly one of the three expiry rules,
  and the lease `last` names was active in `prev` and is `expired` in the state.
  Covering rests on `UnleasedHasNoActiveLease` (QM02).
- **"Keeps its place" means the position is unchanged.** No Allium invariant says where
  a lapsed pocket stands. `effect_LeaseExpiryKeepsPocketAtFront` therefore carries the
  frame the Lease expiry comment states: the pocket's `position` in the state is its
  `position` in `prev`, and `next_position` is unchanged. It does not say the pocket is
  the first queued pocket, because that is false: with `a` at position 0 leased to `c0`
  and `b` at position 2 leased to `c1`, a lapse of `c0`'s lease and then of `c1`'s leaves
  `b` queued behind `a`. `effect_LeaseExpiryParksPocket` and
  `effect_LeaseExpiryIdlesEmptyPocket` carry the same frame.
- **The count's other writes.** `PublishIntoNewPocket` creates the pocket with 0.
  `AckMarksRecord`, `NackMarksRecord`, both release rules and both revive rules write 0.
  `AckRepeats`, `NackRepeats`, `Renew`, both pull rules and every publish rule that does
  not revive leave it. A rule that writes the count gains the write in its action and
  the conjunct in its `effect_`: 0, or for the three expiry rules the count of `prev`
  plus 1. A rule that leaves it gains no write. Its `effect_` gains the frame's
  conjunct: the count is as in `prev`, unless a revival fired in the same step.
- **`ReviveParkedPocket`.** It fires in the same step as the publish rule that emitted
  `PocketTouched`, as `ReviveIdlePocket` does since QM06. In a parked pocket no record
  is held (`OutstandingOnlyWhileLeased`), so only `PublishNewKey` and `Supersede` can
  cause it. The two chains move the counter by different amounts. The revival allots
  one position, the pocket's. `PublishNewKey` allots one too, the record's, so that
  chain ends two higher. `Supersede` allots none, since the Allium's rule has no
  `ensures` on `next_position`, so that chain ends one higher. Both are read through
  the shared definition QM06 added for `ReviveIdlePocket` (README, second design block;
  QM06's ticket proposes `allotted(fired)`). This ticket adds `ReviveParkedPocket` to
  that definition, at one position, which is an edit to a definition QM06 owns and is
  listed in the hand-back. `effect_ReviveParkedPocket` reads the counter through it and
  says the pocket stands at `next_position - 1`. No earlier effect's text changes. A
  parked pocket has records, unlike an idle one, so `Supersede` with a revival is
  reachable here: eight steps, as `PublishNewKey` with a revival is, so the pass at the
  bound checks both, and the plan gives a trace for each.
- **Frames.** Each of the five effects states its frame, as the README asks: what
  neither the rule nor another rule fired in the same step writes is equal in `prev`
  and the state. In a lapse the expiry rule writes the lease, the pocket's status and
  the count, and the lease-end rules write the records and the shed queues, so the
  expiry rule shares no field with a lease-end rule. The lease-end rules share the
  records and the two shed queues among themselves, one firing for each held record,
  as they do in a release. Their effects already read those through the definitions
  QM06 and QM07 landed, and this ticket changes none of them. `FiredMatchesGuards`
  reads all of a lapse's guards in `prev`.
- **`ExpiryLimitPositive`** is a property that reads only the constant, the form QM05
  recommends for `PullBatchPositive`, and its control is a committed mutant that sets
  the limit to 0 in `pachinko_walk`, as QM05's `qm05-zero-pull-batch` does for the
  batch. If QM05's ruling was an `assume`, this follows that ruling instead. The
  property is a `val` of `pachinko`, not of the instance module, where QM03's helper
  refuses an upper-case `val`. And if `tickets/T47-start-rule-and-config-bounds.md` has
  landed and made the bounds the start rule's requirements, the patch leaves the model
  no start state: nothing is violated, and `ExpiryLimitPositive` cannot catch it. Step
  2 reads T47's hand-back, and `qm08-zero-expiry-limit` then takes the control T47's
  ruling named, as QM05's patch does. The lines here that name the patch change with
  it: its row in the mutants table and in Files touched, the count of eight, claim D,
  and in Verification the count and the header check. Each is brought to the ruled
  control before step 4 and recorded under Departures; none is dropped without its
  replacement.

### `MarksStandHoweverLeaseEnds`, in the form it keeps

It reads `prev`, `last`, `fired` and the state, and never how a lease ended.

1. **The first mark stands.** For every record held and acked or nacked in `prev` whose
   pocket's lease is still active in the state: the record is held, with the same mark,
   `held_rank` and `held_payload`.
2. **A repeat changes nothing else.** After an ack of an acked record, or a nack of a
   nacked one, under the consumer's active lease, `fired` is exactly `AckRepeats`, or
   `NackRepeats`, and the state equals `prev`. This clause carries a named exception
   for the gap "lease extension", which QM06 listed under "Not yet modelled": QM09
   replaces "equals `prev`" with "equals `prev` but for the lease's `expires_at`", and
   deletes the line.
3. **The other of the two is rejected.** After an ack of a nacked record, or a nack of
   an acked one, `fired` is empty and the state equals `prev`.
4. **A release and a lapse act alike.** For every lease active in `prev` and not active
   in the state, and every record of its pocket held in `prev`: an acked version added
   no entry to either shed queue in this step; a nacked version added exactly one dead
   letter, at its `held_rank` and `held_payload`, `nacked_by` the lease's consumer; and
   in both cases the record is gone when nothing was deferred, and queued when a version
   was.

QM06 states the release half as a part, `MarksStandHoweverLeaseEnds_release`, and its
mutant `qm06-nack-after-ack` names that part. The part stays in the model unchanged, as
QM06's rule for parts says. This ticket adds the whole guarantee beside it under the
Allium's name.

### The untimed part of `ExpiryRecoversWithinTtl`

QM09 owns the guarantee. This ticket adds one part, `ExpiryRecoversWithinTtl_untimed`,
named as QM06 names a part of a guarantee a later ticket owns. It stays in the model
when QM09 states the guarantee. Like QM06's parts it reads a lease's end from the
state, never from `last` or `fired`, so that it still holds when QM09's `tick` lapses
several leases in one step. For every lease active in `prev` and `expired` in the
state, with its pocket:

- every record of the pocket that was held and unsettled in `prev`, or held with a
  version deferred behind it, is a queued record of the pocket in the state;
- the pocket is queued, parked or idle in the state;
- if it is queued, it is at the position it had in `prev`. What the lapse queued is
  then pullable in that state, and the pocket order alone decides which consumer's
  claim reaches it and when. The property does not say the next pull gets it;
- if it is idle, it has no records.

Which of the three the pocket is, for which counts, is the three expiry rules'
`effect_`.

That covers two sentences of the guarantee: "Once a lapse is processed, its unsettled
records and the versions deferred behind its held ones are queued and, unless that lapse
parks the pocket, are pullable again at once", and "A pocket left with no records has
nothing to recover". The first two sentences of the guarantee, on `lease_ttl` and on a
late action, are QM09's.

### Why an untimed lapse covers every timed behaviour

The header comment and decision 0022 state this, in these terms.

- In the Allium a lapse needs `lease.expires_at <= now`. The model drops that conjunct
  and nothing else, so every guard of the model is the Allium's guard or weaker.
- No effect on a field the model holds reads a timestamp. The only time conditions are
  the lapse's, in four rules, and the triggers of the two decay rules, which the model
  does not have.
- So take any run of the Allium over the same domains and the same configuration as an
  instance of the model, as a sequence of rule firings. Leave out the decays and forget
  the timestamps. What remains is a run of that instance, with the same pockets,
  records, leases and counts after each firing. An invariant over those that holds on
  every run of the instance holds on every timed run over those domains.

What the argument does not carry:

- **Other domains, and depth.** The argument is about one setting of the model's
  constants. A run that uses a fourth pocket key is a run of another instance, not of
  `pachinko_walk`. And the argument is not the evidence: `just verify-quint` checks
  `pachinko_walk` up to its `--max-steps` and no further, so what it shows of the timed
  runs is what it shows of the model's, at those domains and within that bound. Decision
  0022 states the two apart.
- **The shed queues.** The model's queues only grow; a timed run's are these with the
  decayed entries removed. A property about what one step appends carries over. A
  property about an entry still being there does not, and QM09 states it
  (`EveryDropIsVisible`: only a decay removes an entry).
- **Witnesses.** The model has runs no clock allows, such as a lapse straight after a
  renew. A witness reached here may not be reachable under a clock. QM09 reaches each
  one again.
- **Anything that needs the lapse to wait.** Left for QM09, because they cannot be
  stated here: a lease does not lapse before its `expires_at`; a pull, ack, nack or
  renew moves `expires_at` to `now + lease_ttl`, so until QM09 `Renew`, `AckRepeats` and
  `NackRepeats` change nothing but `fired` and no mutant can catch a broken extension;
  every timestamp invariant; and the bound in the guarantee's first sentence, which is
  liveness and stays excluded (decision 0022).
- **A tick.** If the specification makes one tick lapse several leases as one outcome,
  the states between those lapses are states of this model that the specification never
  shows. An invariant that holds in them holds after the whole tick.

### Mutants

Each is a patch against `quint/pachinko.qnt` with the header QM03's format asks
(`# mutant:`, `# instance: pachinko_walk`, `# property:`). Each names one property, and
QM03's recipe checks that property alone, so a mutant that also breaks another
invariant on the way is still caught by the one it names. Each has a shortest trace
from the empty `init`, written here so that a mutant with no trace is not mistaken for
one that was caught. `L1` and `L2` are the first and second leases granted. Steps are
counted after `init`.

| Patch | Where it cuts | Property | Shortest trace | Steps |
| --- | --- | --- | --- | --- |
| `quint/mutants/qm08-ack-keeps-expiry-count.patch` | `AckMarksRecord` in `ack`: the count is not written | `MarkedPocketHasNoExpiries` | `p0` joins, `c0` joins, publish `a`/`k0` rank 0, pull `c0` limit 1, lapse `L1`, pull `c0` limit 1, ack `k0` under `L2` | 7 |
| `quint/mutants/qm08-parking-skipped.patch` | `lapse`: when the parking guard holds, the outcome of `LeaseExpiryKeepsPocketAtFront` is applied | `ExpiriesWithinLimit` | two joins, one publish, then pull and lapse three times | 9 |
| `quint/mutants/qm08-lapse-requeues-acked.patch` | `lapse`: every held record's mark is set to unsettled before the lease-end definition runs | `MarksStandHoweverLeaseEnds` | two joins, publish, pull `c0` limit 1, ack `k0`, lapse `L1` | 6 |
| `quint/mutants/qm08-expiry-moves-pocket-back.patch` | `lapse`, queued outcome: the pocket takes `next_position` and the counter advances, as in a release | `effect_LeaseExpiryKeepsPocketAtFront` | two joins, publish, pull `c0`, lapse `L1` | 5 |
| `quint/mutants/qm08-lapse-leaves-lease-active.patch` | `lapse`: the lease's status is not written; the rest runs | `ExclusivePocket` | `p0`, `c0` and `c1` join, publish, pull `c0` limit 1, lapse `L1`, pull `c1` limit 1 | 7 |
| `quint/mutants/qm08-expired-lease-accepted.patch` | the test of `lease.status = active` that `ack` uses: an `expired` lease passes, a `released` one does not | `ActionsNameTheirLease` | two joins, publish, pull `c0` limit 1, lapse `L1`, pull `c0` limit 1, ack `k0` naming `L1` | 7 |
| `quint/mutants/qm08-lapse-frees-other-pockets.patch` | `lapse`: the lease-end definition also runs over the held records of every other leased pocket | `HeldSetStableUntilLeaseEnds` | `p0`, `c0` and `c1` join, publish into `a` and into `b`, pull `c0` limit 1, pull `c1` limit 1, lapse `L1` | 8 |
| `quint/mutants/qm08-zero-expiry-limit.patch` | `pachinko_walk` sets `max_consecutive_expiries` to 0 | `ExpiryLimitPositive` | none: the first state | 0 |

The second mutant needs a third lapse: at a limit of 2 a count of 2 still satisfies
`ExpiriesWithinLimit`. It is the deepest check this ticket adds. The sixth cuts outside
`lapse`, and is still on the new path: only a lease ended by a lapse is `expired`, so a
lease ended by a release is refused as before.

### Coverage this ticket adds to earlier properties

README, "Coverage a later ticket adds", names QM08 in four rows, seven properties.

| Property | Owner | Answer |
| --- | --- | --- |
| `ExclusivePocket` | QM05 | the mutant `qm08-lapse-leaves-lease-active` |
| `ActionsNameTheirLease` | QM06 | the mutant `qm08-expired-lease-accepted` |
| `HeldSetStableUntilLeaseEnds` | QM05 | the mutant `qm08-lapse-frees-other-pockets`: the lapse of one lease is not the end of another |
| `EveryDropIsVisible`, `OldestSlotNewestPayload`, `HeldRecordsOldestFirst`, `NothingLostWhileHeld` | QM07 | the shared definition: a lapse ends a lease only through the definition the release uses. The plan names it as the base names it. The control below shows it |

**The release-disabled control** shows that the lapse does run that definition. For each
of the four properties in the last row, take a mutation that cuts inside the lease's
end and that the property catches. QM07's ticket lists three:
`qm07-acked-row-keeps-held-position` for `OldestSlotNewestPayload`,
`qm07-lease-end-newest-first` for `HeldRecordsOldestFirst` and
`qm07-deferred-version-lost` for `NothingLostWhileHeld`. Its mutant for
`EveryDropIsVisible` cuts in `Supersede`, not in the lease's end, so for that property
the control uses a cut of its own, kept in scratch: `SupersedeUnsettledHeldVersion`
adds no superseded entry. The control takes two scratch copies for each property, both
with `release` deleted from `step`, so that a lapse is the only way a lease ends. On
the first, with no cut, `Canary` is violated and the property passes at the walk's
bound: deleting `release` breaks nothing by itself. On the second, with the cut
applied, the same command reports the property violated. Its log holds
`found a counterexample` and not `reached a deadlock`, and the last step of the
counterexample, read from `last`, is a `lapse`. The cut is then the one difference
between the pass and the violation, and the violation is on the lapse's path. If a
patch named here did not land, the plan picks another and says so. The control is run
by hand at step 7 and adds no committed mutant.

### `--max-steps`, and what it costs

- **Proposed.** `pachinko_walk`'s bound in `quint/verify.txt` is at least 10.
  `witness_ReviveParkedPocket` needs 8 steps (two joins, a publish, a claim and a lapse
  twice, a publish), QM03's rule for the bound is the deepest witness's depth plus two,
  and `qm08-parking-skipped` needs 9. QM03 recommends 10, so nothing changes if that
  landed. If the file holds less, this ticket raises it to 10. It never lowers it
  (shared rule 5).
- **The cost** is not known. This ticket adds an action whose argument is every lease,
  five witnesses and eight mutants, one run each. Every step that no rule answers is still
  a step, so a 10-step trace has many shapes. Step 2 measures the recipe before the change
  and step 6 after it. The budget is the whole job's, and these runs are the recipe
  alone, on another machine: each figure is quoted with the durations of the setup and
  the installs that QM03's hand-back quotes from CI added, against the head-room QM03's
  ruling set (its ticket proposes 15 of the job's 20 minutes), and the hand-back says
  the sum is an estimate until the job runs. If it does not fit, **stop** and ask
  (ruling 4).
- **The alternative** to put to the maintainer at that stop: a bound per mutant, so that
  only the mutant that needs 9 runs that deep. That is a change to QM03's recipe and
  patch header, outside this ticket's files.

### The path the walk's limit cannot reach

- **What `pachinko_walk` cannot reach.** At a limit of 2 the parking rule fires only
  when the count is 1, and `MarkedPocketHasNoExpiries` then says no held record is
  marked. So on `pachinko_walk` a parking lapse never frees an acked or a nacked
  record, and a parked pocket always has a record. The Allium names the other case: at
  the threshold a pocket with every record marked "parks instead, since parking has no
  records guard, and its nacked records still become dead letters".
- **Recommended: once, by hand, in scratch.** Step 7 copies the model to
  `ai_tmp/qm08/limit1.qnt` and adds an instance `pachinko_limit1`, equal to
  `pachinko_walk` but with the limit at 1, and a definition `parksWithMarkedRecord`,
  true when `fired` holds both `LeaseExpiryParksPocket` and `DeadLetterNackedRecord`.
  On `pachinko_limit1`, `Canary` is violated, every property passes at the walk's
  bound, and `parksWithMarkedRecord` is observed; it needs 6 steps (two joins, a
  publish, a pull of limit 1, a nack, a lapse). On `pachinko_walk` it is not observed.
  Decision 0022 gets the line: the path is modelled, was checked at this ticket's
  commit by hand, and is reached by no instance the gate runs.
- **Alternative: a committed instance.** `LeaseExpiryKeepsPocketAtFront` and
  `LeaseExpiryIdlesEmptyPocket` are unreachable at a limit of 1, and QM03's recipe
  reaches every rule's witness on every instance. A committed `pachinko_limit1` needs
  the recipe to let an instance name the rules it cannot reach, which is a change to
  QM03's files and a ticket of its own. It also costs a second pass of every property.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `verify-quint` (every instance, witness and mutant, with setup) finishes inside the CI job's timeout. Each run being fast is not enough | steps 2, 5 and 6: the wall time of `just verify-quint` before the change, with the model and with the mutants, each with the setup and install durations from QM03's hand-back added, quoted beside the `quint` job's `timeout-minutes` and the head-room QM03's ruling set |
| A | `--max-steps=N` counts steps after `init`: a state first reached by step N is checked and one first reached by step N+1 is not. Read in `tools/quint/node_modules/@informalsystems/quint/dist/src/apalache.js` line 108 (`length: args.maxSteps`) and `verify.js` line 239 ("No trace of up to ... steps reached"), not run | step 5: a witness run for `ReviveParkedPocket` is observed at `--max-steps` equal to its depth and not at one less. The depth is the plan's, recomputed against the base as the README's "The T tickets that touch the model" asks: 8 today, 6 if T45 has removed the joins |
| B | The step counts in the mutants table are the shortest. Counted by hand from the Allium, not run | step 6: each mutant's counterexample has the length the table gives. A shorter one is recorded; a longer one, or none inside the bound, is a finding |
| C | On `pachinko_walk` a parking lapse never frees a marked record. Derived from `MarkedPocketHasNoExpiries`, not run | step 7: `parksWithMarkedRecord` is not observed on `pachinko_walk` and is observed on `pachinko_limit1` |
| D | A property that reads only a constant is violated in the first state of an instance that breaks it. QM05 checks the form for `PullBatchPositive`; this author ran nothing | step 6: `qm08-zero-expiry-limit` is caught |

## Non-goals

- No `now`, no timestamp field, no TTL and no decay (shared rule 7). `ended_at` waits
  for QM09 with the rest.
- No `tick`, no `advance`, and no order among several lapses. QM09 adds them.
- No `lapse` in `replayStep` and no `WalkStep` for it. QM11 maps lapses. Under
  `crates/` and `double/` only the conditional rows of Files touched change, and only
  to read `consecutive_expiries`.
- No second transcription of the six lease-end rules.
- No change to `scripts/verify-quint.sh`, to the format of `quint/verify.txt` or of a
  patch's header, or to the `quint` job in `.github/workflows/ci.yml`. A need there is
  handed back.
- No answer to either `open question` in the Allium, and none to any point T42 puts to
  the maintainer.

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

`<id>` is `QM08`. Checkpoints:

1. The plan, `ai_tmp/qm08/plan.md`, with the conformance and breakage lenses, in step 3,
   before any tracked file changes.
2. The diff, with the conformance, breakage and design lenses, in step 9, before the
   final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | the state, the constant, `lapse`, five variants of `Rule` and one of `Step`, the five owned rules with `guard_`, `effect_` and `witness_`, `FiredMatchesGuards` over them, the count's writes and their effects, QM06's shared definition for the counter, which gains `ReviveParkedPocket`, the four properties and `ExpiryRecoversWithinTtl_untimed`, and the "Not yet modelled" lines |
| `quint/mutants/qm08-ack-keeps-expiry-count.patch` | new | the mutant of that name |
| `quint/mutants/qm08-parking-skipped.patch` | new | the mutant of that name |
| `quint/mutants/qm08-lapse-requeues-acked.patch` | new | the mutant of that name |
| `quint/mutants/qm08-expiry-moves-pocket-back.patch` | new | the mutant of that name |
| `quint/mutants/qm08-lapse-leaves-lease-active.patch` | new | the mutant of that name |
| `quint/mutants/qm08-expired-lease-accepted.patch` | new | the mutant of that name |
| `quint/mutants/qm08-lapse-frees-other-pockets.patch` | new | the mutant of that name |
| `quint/mutants/qm08-zero-expiry-limit.patch` | new | the mutant of that name |
| `quint/mutants/` | shared, own lines | each earlier patch the model change breaks, re-cut with its mutation and its property kept (shared rule 11) |
| `quint/verify.txt` | shared, own lines | only if `pachinko_walk`'s bound is below 10: the bound |
| `quint/traces/` | shared, own lines | only if the tree this ticket lands on holds `quint/traces/`, at the base or after the closing merge or rebase: the fixtures regenerated by `just quint-traces`, never by hand, before any gate runs (shared rule 11) |
| `scripts/quint-traces.py` | shared, own lines | the fixtures' generator, created by QM10; on the same condition, and only if a rule this ticket adds is one `replayStep` reaches, which none of the five is: its witness list. No edit is expected; one that is made is recorded under Departures |
| `crates/libpachinko/tests/queue/walk/replay.rs` | shared, own lines | on the same condition: the row for `consecutive_expiries` leaves the list of what is not read from the fixture. Created by QM10 |
| `crates/libpachinko/tests/queue/walk/model/trace.rs` | shared, own lines | on the same condition: `consecutive_expiries` read from the fixture's state. Created by QM10 |
| `double/tests/test_replay.py` | shared, own lines | only if the tree this ticket lands on holds `double/tests/test_replay.py`, at the base or after the closing merge or rebase: `consecutive_expiries` mapped in the projection and compared. Created by QM12 |
| `docs/decisions/` | shared, own lines | in the record QM02 wrote as decision 0022 only: the superset argument and what it does not carry; `ExpiryLimitPositive` as a check on the constant; the path no gated instance reaches |
| `.scratch/quint-model/QM08-untimed-lapse-and-parking.md` | ticket | `status:` and the hand-back notes; and, only if T47's ruling replaced the config-bound control, the lines of "Design proposed", Files touched, acceptance and Verification that name `qm08-zero-expiry-limit`, each recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM08's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm08-untimed-lapse-and-parking`
   from the series base the maintainer names, once QM07 has merged there, and record the
   base's full commit id as `base`. `git ls-files .scratch/quint-model/` must list the
   README and this ticket. Run `just initialize` (network, authorised). Run `just check`
   on the untouched worktree and quote its last line, headed by the commit.
2. **Read before writing.** Read the Allium items under Context at `base`. Read the
   model, and record: how a lease is identified and what statuses it has; the name of
   the lease-end definition and where the release calls it; how `Step`'s variants are
   spelt; the "Not yet modelled" lines, with every line that names QM08 copied exactly
   into the plan; whether `replayStep`, `quint/traces/` and
   `double/tests/test_replay.py` exist.
   Read `quint/verify.txt` for `pachinko_walk`'s bound, and the hand-backs of QM03, QM05
   and QM06 for what was ruled. Check the Allium for T42's lapse intent, and **stop** if
   it has landed. Show that nothing listens on `localhost:8822`, run
   `just verify-quint`, and quote its wall time as the cost before this ticket. Here and
   in every later step, each `verify`, by hand or through the recipe, follows a port
   probe of its own, the one Verification writes out (shared rule 10). Then
   write `ai_tmp/qm08/plan.md`:
   - each of the five owned rules, the Allium's text beside the planned action,
     `guard_` and `effect_`;
   - each rule whose effect gains the count, with the conjunct;
   - each property with the ghosts it reads;
   - each mutant, as a diff against the base's model, with its property and its trace;
   - the coverage table, with the definition and the earlier mutants named;
   - each "Not yet modelled" line added or removed;
   - the answer to each proposal under "Design proposed", as this base allows it.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `ai_tmp/qm08/plan.md`, before any tracked file changes). Questions:
   - Does any planned guard or effect say more, or less, than the Allium's rule beside
     it? Is `pocket.status = leased` kept?
   - Can a lapse reach the six lease-end rules by any path other than the definition
     the release uses?
   - Does the plan move the counter by one for `Supersede` with a revival of a parked
     pocket and by two for `PublishNewKey` with one, both through the shared definition?
   - For each mutant: does the trace in the plan violate the named property, and only
     because of the cut? Could the unchanged model violate it?
   - Does `MarksStandHoweverLeaseEnds` read how the lease ended anywhere? Does
     `ExpiryRecoversWithinTtl_untimed` read `last` or `fired`?
   - Is anything the Allium allows after this ticket missing from the model and from
     "Not yet modelled"?
4. **Red.** **Stop** first for the maintainer's ruling on "Design proposed", with the
   checkpoint's findings in hand, and record it. Then add the state, the constant, the
   five variants of `Rule`, the guards, the effects, the witnesses and the properties,
   with `lapse` not yet in `step` and the revival not yet in the publish. Before either
   gate, bring what reads the model up to it (shared rule 11), here and after every
   later edit of the model. QM03's ticket proposes a recipe that applies every patch
   before it verifies anything, and the new field moves lines that earlier patches cut
   on: re-cut each earlier patch that no longer applies, or the run stops at "does not
   apply" and shows no witness. And if the tree holds `quint/traces/`, run
   `just quint-traces`, before any gate (shared rule 11): QM10 makes `just check-quint`
   refuse a model whose hash is not
   the manifest's, and ends `just verify-quint` on a diff of the regenerated fixtures.
   Bring each replay that exists up here too, before the next gate: `just test`, and
   `just python-double` where `double/tests/test_replay.py` exists, pass on the
   regenerated fixtures, with a field the model gained mapped or listed as shared rule
   11 says. Step 7 then holds their controls.
   `just check-quint` passes. `just verify-quint` fails, and its output shows a witness
   of this ticket unreached. Quote it. Then run the witness run for each of the five by
   hand, with the command the recipe prints, and quote all five unreached. A recipe
   that passes here is a finding against the recipe: stop.
5. **Green.** Add `lapse` to `step`, the count's writes, and `ReviveParkedPocket` to the
   publish action. Re-cut each earlier patch that no longer applies, and regenerate the
   fixtures if they exist, as at step 4. Show the port free,
   run `just verify-quint`, and quote it: `Canary` violated on every instance, every
   property passing, the five witnesses observed, every earlier mutant caught. Quote the
   wall time beside the `quint` job's `timeout-minutes` (claim 12), as "`--max-steps`,
   and what it costs" says to count it. By hand, show the
   witness run for `ReviveParkedPocket` observed at `--max-steps` equal to its depth and
   not at one less (claim A). The depth is the plan's, recomputed against the base: 8
   today, 6 if `tickets/T45-drop-joining.md` has removed the two joins. If the recipe
   does not fit the head-room, **stop** and ask.
6. **Mutants.** Add the eight patches. For each, quote from `just verify-quint`: the
   patch applied, the mutant typechecked and the named property violated. The unchanged
   model's pass on `pachinko_walk`, quoted at step 5, is what shows each named property
   holds at the same bound (shared rule 6). Record the length of each counterexample
   (claim B). Quote the wall time again.
7. **Controls.** Each by hand, through `just quint`, with the port shown free first and
   the output and exit status quoted:
   - **The release-disabled control**, as "Design proposed" describes it, once for each
     of the four properties.
   - **The limit at 1**, as "Design proposed" describes it, or as ruled (claim C).
   - **The replays** (shared rule 11). If the tree holds `quint/traces/`: `lapse` is not
     added to `replayStep`. Run `just quint-traces` (created by QM10), then
     `just quint-traces diff`, which QM10's ticket proposes as the read-only comparison
     of a fresh generation with the fixtures in the tree: it must exit 0. The control
     names no earlier run, so it holds when the fixtures first arrive in the closing
     merge and after the header's edit at step 8. Show that no step of any fixture is
     a lapse. Make the Rust replay read `consecutive_expiries`
     from the fixture: remove its row from the list in `replay.rs` and read the field
     in `model/trace.rs`. This is a change to Rust under `crates/`, so `AGENTS.md`'s
     block for it applies: use the `rust-change` skill, and run `just fmt-check`,
     `just clippy` and `just test`. If the tree holds `double/tests/test_replay.py`: map
     the field in it and run `just python-double`. No lapse is replayed,
     so every count compared is 0. Any other change a replay needs is handed back.
8. **Words.** Add the lines to decision 0022 and to the model's header comment. The
   header is part of the model's file, so this is an edit of the model: if the tree
   holds `quint/traces/`, run `just quint-traces` after it and before the next gate,
   and re-cut any patch it moved. Run `just check-docs`.
9. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14: conformance,
   breakage and design, over `git diff "$base"...HEAD` plus the uncommitted changes,
   before the final commit). Questions:
   - Does the diff transcribe any of the six lease-end rules a second time?
   - Is `lapse` reachable from `replayStep`, or is any timestamp, TTL or `now` in the
     diff?
   - Could `ExpiriesWithinLimit` or `MarkedPocketHasNoExpiries` hold because a write to
     the count is missing, not because the rules keep it?
   - Does each re-cut patch still make the mutation its name says, against the property
     it named before?
   - Does the superset argument in decision 0022 claim anything about the shed queues,
     about witnesses or about a tick that it does not carry?
   - Does `effect_LeaseExpiryKeepsPocketAtFront` say the pocket is first, which is
     false, or that its position is unchanged?
10. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed
    by the full commit id. Set `status: done` here and in
    `.scratch/quint-model/README.md`, write the hand-back notes, with each patch re-cut
    listed, and commit the record. Then run `just check` once more, on the record's
    commit, and show its last line and an empty `git status --short`. If it fails, fix
    the notes, commit again and run it again; never amend the work's commit. Before
    asking to push, fetch (network, authorised),
    merge or rebase onto the series base, and apply shared rule 11 to whatever that
    brought in (QM10 or QM12 may have merged meanwhile). Read again, against the merged
    tree, every condition here that turns on the tree holding `quint/traces/` or
    `double/tests/test_replay.py`: if the merge brought one in, do step 7's replay work
    now, regenerating the fixtures before any gate. Then rerun `just check` and
    Verification,
    and requote both (`tickets/CONVENTIONS.md` §11, "Green"; shared rule 13).
    If the base had moved, the quotes already committed are stale
    (§11, "Quoted evidence names its commit"): put the fresh ones in the hand-back,
    headed by the merged commit, commit that record, and run `just check` on it once
    more, with an empty `git status --short`, as after the first record.
    If the
    merge or rebase changed a file under Files touched, checkpoint 2 did not see that
    change: show its diff at the stop and ask whether it is reviewed before the push.
    **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just check-quint` passes, quoted.
- `just verify-quint` passes, quoted: `Canary` violated on every instance, every
  property named and passing, and `witness_LeaseLapses`, `witness_LeaseExpiryParksPocket`,
  `witness_LeaseExpiryKeepsPocketAtFront`, `witness_LeaseExpiryIdlesEmptyPocket` and
  `witness_ReviveParkedPocket` observed.
- Each of the eight mutants typechecks and is caught by the property the mutants table
  names, quoted, and each earlier mutant is still caught. The hand-back lists every
  patch re-cut.
- Step 4's red run is quoted: the same recipe failed, and all five witnesses were
  unreached, before `lapse` existed.
- The wall time of `just verify-quint` before and after this ticket is quoted beside the
  `quint` job's `timeout-minutes`, with the setup and install durations from QM03's
  hand-back added, and `pachinko_walk`'s bound is at least 10 and not lower than at
  `base`.
- No domain of `pachinko_walk` is smaller than at `base`, shown by the diff of the
  instance.
- The model's code lines hold no `now`, no timestamp field and no TTL, and `lapse` is
  not offered by `replayStep` if that exists.
- The "Not yet modelled" list is correct: no line names QM08, QM06's line for the reset
  among them, the two lines of "Design proposed" are there, and every line QM09 removes
  is still there.
- The release-disabled control is quoted for each of the four properties: the pass on
  the copy with no cut, and the counterexample on the copy with the cut, ending in a
  lapse.
- The hand-back lists the edit to QM06's shared definition for the counter, and gives
  the eight-step trace of a parked pocket's revival behind `Supersede` and the one
  behind `PublishNewKey`, both inside the bound of the quoted pass.
- Each of the seven properties under "Coverage this ticket adds" has its mutant caught
  or its definition named in the hand-back.
- The path the walk's limit cannot reach is shown as ruled: by hand on
  `pachinko_limit1`, quoted, with its line in decision 0022.
- Decision 0022 states the superset argument with what it does not carry.
- Claims 12 and A to D each have an outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with the review deltas.
- If the tree this ticket lands on holds `quint/traces/`, at the base or after the
  closing merge or rebase, the fixtures were regenerated by `just quint-traces`,
  no fixture holds a lapse, the Rust replay reads `consecutive_expiries` from the
  fixture and no longer lists it, and `just test` passes, quoted.
- If that tree holds `double/tests/test_replay.py`, the double's replay compares
  `consecutive_expiries` and `just python-double` passes, quoted.
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
mkdir -p ai_tmp/qm08

for rule in LeaseLapses LeaseExpiryParksPocket LeaseExpiryKeepsPocketAtFront \
  LeaseExpiryIdlesEmptyPocket ReviveParkedPocket; do
  grep -qw "guard_${rule}" "$model"
  grep -qw "effect_${rule}" "$model"
  grep -qw "witness_${rule}" "$model"
done
for name in ExpiriesWithinLimit MarkedPocketHasNoExpiries ExpiryLimitPositive \
  MarksStandHoweverLeaseEnds ExpiryRecoversWithinTtl_untimed; do
  grep -qw "$name" "$model"
done

# No clock: the code lines hold the count, and hold no time.
grep -v '^ *//' "$model" >| ai_tmp/qm08/code.txt
grep -qw 'consecutive_expiries' ai_tmp/qm08/code.txt
words='now|granted_at|expires_at|ended_at|nacked_at|shed_at|lease_ttl|superseded_ttl|dead_letter_ttl'
if grep -nwE "$words" ai_tmp/qm08/code.txt; then exit 1; else test "$?" -eq 1; fi

# The header: QM06's line for the reset is gone and the lapse's gap is listed.
grep -F 'Not yet modelled' "$model"
grep -F "the lapse's time condition, lease.expires_at <= now, in LeaseLapses" "$model"
if grep -F 'the reset of consecutive_expiries by' "$model"; then exit 1; else test "$?" -eq 1; fi
if grep -nF 'QM08' "$model"; then exit 1; else test "$?" -eq 1; fi

set -- quint/mutants/qm08-*.patch
test "$#" -eq 8
for patch in "$@"; do
  grep -q '^# instance: pachinko_walk$' "$patch"
  grep -q '^# property: ' "$patch"
done
grep -Fx '# property: ExpiriesWithinLimit' quint/mutants/qm08-parking-skipped.patch
grep -Fx '# property: MarkedPocketHasNoExpiries' quint/mutants/qm08-ack-keeps-expiry-count.patch
grep -Fx '# property: MarksStandHoweverLeaseEnds' quint/mutants/qm08-lapse-requeues-acked.patch
grep -Fx '# property: effect_LeaseExpiryKeepsPocketAtFront' quint/mutants/qm08-expiry-moves-pocket-back.patch
grep -Fx '# property: ExclusivePocket' quint/mutants/qm08-lapse-leaves-lease-active.patch
grep -Fx '# property: ActionsNameTheirLease' quint/mutants/qm08-expired-lease-accepted.patch
grep -Fx '# property: HeldSetStableUntilLeaseEnds' quint/mutants/qm08-lapse-frees-other-pockets.patch
grep -Fx '# property: ExpiryLimitPositive' quint/mutants/qm08-zero-expiry-limit.patch

bound=$(sed -n 's/^pachinko_walk max-steps=//p' quint/verify.txt)
test "$bound" -ge 10

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
just verify-quint >| ai_tmp/qm08/verify.log 2>&1
echo "verify-quint: $(( $(date +%s) - started )) seconds"
grep -n 'timeout-minutes' .github/workflows/ci.yml
for name in witness_LeaseLapses witness_LeaseExpiryParksPocket \
  witness_LeaseExpiryKeepsPocketAtFront witness_LeaseExpiryIdlesEmptyPocket \
  witness_ReviveParkedPocket ExpiriesWithinLimit MarkedPocketHasNoExpiries \
  ExpiryLimitPositive MarksStandHoweverLeaseEnds ExpiryRecoversWithinTtl_untimed \
  Canary; do
  grep -qw "$name" ai_tmp/qm08/verify.log
done
for patch in "$@"; do grep -qF "$(basename "$patch" .patch)" ai_tmp/qm08/verify.log; done

if test -d quint/traces; then
  grep -lw 'Pull' quint/traces/*.json
  if grep -lwi 'lapse' quint/traces/*.json; then exit 1; else test "$?" -eq 1; fi
  grep -c 'consecutive_expiries' crates/libpachinko/tests/queue/walk/model/trace.rs
fi
if test -f double/tests/test_replay.py; then
  grep -c 'consecutive_expiries' double/tests/test_replay.py
  just python-double
fi

just check
status=$(git status --short)
test -z "$status"
```

The red tests are the eight mutants, which `just verify-quint` passes only if each is
caught by the property its header names, and the header checks before it pin each patch
to the property this ticket gives it, so a patch re-aimed at an easier property fails
here. The green tests are `just check-quint`, `just verify-quint`, which also holds the
five witnesses and the canary, `just python-double` where the double's replay exists,
and `just check`, which runs the Rust replay in `just test`. Five probes pass by
printing nothing, and each follows a positive control over the same input: the search
for a time word follows the search for `consecutive_expiries` in the same file; the
searches for QM06's line and for any line that still names QM08 follow two searches of
the header that match; the port probe
follows the listener the script starts and then kills; and the search for a lapse in the
fixtures follows the search for a pull there.

The script assumes what earlier tickets fix, and a difference is recorded under
Departures with the corrected script kept in `ai_tmp/qm08/`: that the model's header
comment uses `//` lines; that `quint/verify.txt` and a patch's header have the form
QM03 recommends; that the log of `just verify-quint` names each property, witness and
mutant it handles, a mutant by its patch's name without `.patch`, as QM03's ticket
proposes; that the model names QM08 nowhere but in the header's list; and that a
fixture names each step by its `Step` variant. The script times the recipe alone;
steps 5 and 6 hold the sum with the job's setup. Step 4's
red run, step 5's depth probe and step 7's controls are quoted in the hand-back and are
not rerun here.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Departures

### Verification

## Open points

- **A page that lists what the model holds.** QM02 wrote handbook pages. If one lists
  the modelled rules, it needs this ticket's five rules and is not in Files touched.
  The executing agent finds it at step 2 and hands it back; the maintainer says whether
  to add it.
- **T42's lapse intent.** If the maintainer rules that an overdue lease ends only when
  another consumer takes the pocket, the untimed lapse no longer covers the
  specification. The ticket that lands that ruling settles it, under shared rule 11.
- **A gated instance with the limit at 1.** Recommended against here only because
  QM03's recipe asks every instance to reach every rule. If the maintainer wants the
  path under the gate, a ticket of its own changes the recipe first.
- **A bound per mutant.** Raised only as the alternative if a bound of 10 for every run
  does not fit the timeout. The maintainer settles it at the stop in step 5 or 6.
