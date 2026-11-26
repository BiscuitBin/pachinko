---
id: QM04
title: "Publish with nothing held"
status: todo
depends_on: [QM03]
parallel_with: []
branch: ticket/qm04-publish
estimated_size: M
---

# QM04: Publish with nothing held

## Context

After QM03 the repository holds a Quint model, `quint/pachinko.qnt`, with three rules
(`PullClaimsFrontPocket`, `PullFindsNoPocket` and `ReleaseRequeuesPocket`), six
invariants, the four ghosts, `FiredMatchesGuards`, `UnansweredStepChangesNothing`,
`Canary` and the instance `pachinko_walk`. `just check-quint` typechecks it inside
`just check`, and `just verify-quint` verifies it, runs a witness for each rule and
applies each mutant under `quint/mutants/`. All four are created by QM02 and QM03.

That model does not start where a deployment starts. QM02 seeded it: the three consumers
have already joined, and two pockets, `a` and `b`, each already hold one queued record.
The seed stood in for rules the model did not have. Nothing in it publishes, nobody
joins, and a record has only its pocket, its key and its status.

This ticket removes the seed. The model starts empty, and the two joins and four of the
five publish rules of `docs/specs/pachinko.allium` build every later state. Records
enter the model with their key, rank, payload, position and status, and the superseded
queue enters with its `reason`. No record is held yet: a pull's limit still ranges over
-1 and 0 until QM05, so a claim hands out nothing. The fifth publish rule,
`DeferBehindHeldVersion`, needs a held record and waits for QM07. No pocket is idle or
parked yet, so the `PocketTouched` that `PublishNewKey` and `Supersede` emit has no rule
to answer it: `ReviveIdlePocket` is QM06's and `ReviveParkedPocket` is QM08's.

Three tickets outside this series change what this ticket models. Each carries its own
step for the model (`.scratch/quint-model/README.md`, "The T tickets that touch the
model"). If one has merged into the series base before step 1, the Allium at the base
differs from the lines quoted here, and step 2 stops and asks.

- `tickets/T45-drop-joining.md` drops joining. It removes `PublisherJoins`,
  `ConsumerJoins`, `UniquePublisherId` and `UniqueConsumerId`, which are four of the
  twelve names this ticket owns. Its Context says of today's Allium: "Nothing in the
  module says what an id that never joined does." That gap is the first design point
  below.
- `tickets/T39-decide-q2-q6.md` lands `new_pocket_first`, which changes the position
  `PublishIntoNewPocket` gives a new pocket and adds a config value to the instance.
- `tickets/T47-start-rule-and-config-bounds.md` states the start of a deployment as a
  rule and settles the Queue's first position. It changes `init`.

Read these:

- `.scratch/quint-model/README.md`, whole. "Design fixed before QM01" is fixed for this
  ticket, both blocks.
- `docs/specs/pachinko.allium` at the base: the Vocabulary entries for Pocket, Record,
  Rank, Supersede, Superseded record and Position (lines 37 to 70); the `Queue`,
  `Publisher`, `Consumer`, `Record` and `SupersededRecord` entities with the comments
  above them (lines 100 to 120, 166 to 192 and 225 to 241); the Identity rules (lines
  306 to 316); the Publish section with all three of its comments (lines 318 to 454);
  `ReleaseRequeuesPocket` (lines 668 to 687); the six invariants this ticket owns (lines
  1006 to 1028, 1062 to 1066 and 1073 to 1076); and the `Publishing` surface (lines 1109
  to 1148).
- `crates/libpachinko/tests/queue/walk.rs`, for names and domains only: the constants
  at lines 36 to 43, `WalkStep` at lines 82 to 117, the domains in `step()` at lines 127
  to 145, and the payload the walk builds at line 307.
- `quint/pachinko.qnt`, `quint/verify.txt`, `scripts/verify-quint.sh` and every patch
  under `quint/mutants/`, as QM02 and QM03 left them at the base.
- `.scratch/quint-model/QM02-tracer-claim-and-release.md` and
  `.scratch/quint-model/QM03-verify-recipe-and-ci.md`: "Design proposed in this ticket",
  and in their hand-back notes "Design rulings". They settle how a lease is identified,
  how the pocket order is written, how a witness is run, what counts as a mutant
  caught, a patch's header, how the recipe reads the model's properties, the format of
  `quint/verify.txt` and the time budget. Where a ruling there differs from what this
  ticket assumes, the ruling wins and the plan says what changes.
- Decision 0022 under `docs/decisions/`, created by QM02.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

`quint/pachinko.qnt` starts from an empty deployment: nobody has joined, and there is no
pocket, no record, no lease and no superseded record. `publisherJoins(publisher)`,
`consumerJoins(consumer)` and `publish` are top-level actions in `step`, named after
`WalkStep`, and `publish` takes `publisher`, `pocket`, `key` and `rank`. Six rules have
their `guard_`, `effect_` and `witness_`, and `FiredMatchesGuards` covers them. Every
witness in the model, QM02's three included, is reached from the empty start. Six
invariants are checked properties, and three publish guarantees that QM07 owns are
checked as far as they can be with nothing held. `just verify-quint` is green at the
walk's domains, with the mutants below each caught by the property it names.

What a green run does not show, and the model's header says so: nothing is held, so no
version is deferred and no publish revives a pocket.

### Owns

| Kind | Names |
| --- | --- |
| Rules | `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`, `DropStalePublish` |
| Invariants | `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`, `UniqueRecordPosition`, `RecordPositionsBelowNext` |
| Guarantees | none |
| Partial or extended, not counted | `NeverRefusedForCapacity`, `EveryDropIsVisible` and `OldestSlotNewestPayload` with nothing held, as the three parts named under "Design proposed"; QM07 owns all three. `Supersede` lands with its queued case only; QM07 adds the held case. `guard_PullClaimsFrontPocket` and `guard_PullFindsNoPocket`, QM02's, read whether the consumer has joined once the seed is gone. `ReleaseRequeuesPocket`'s requirement reads real records. |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them after checkpoint 1 and
before step 4, and the hand-back records the ruling. After that the builder does not
reopen them. Points 1, 2 and 3 are this ticket's row in the README's "Design points a
ticket settles": what a step by a publisher or a consumer that has not joined does, the
counter's first value, and which invariants are true by representation. Later tickets
assume their recommended answers. Points 1 and 2 answer something the Allium does not
say, so they are also under Open points.

1. **A step by a party that has not joined.** The empty start makes this reachable: a
   publish by `p1` before `p1` joins, and a pull or a release by a consumer that never
   joined. The Allium's triggers take entities, `Publish(publisher, …)` at line 338 and
   `Pull(consumer, limit)` at lines 512, 528 and 570. The surfaces face those entities
   (`facing publisher: Publisher` at line 1110, `facing consumer: Consumer` at line
   1151). Only a join creates one (lines 306 to 316). No rule requires
   `exists publisher`, and nothing says what a publish naming an id that never joined
   does.
   - Recommended: the step is taken, no rule answers it, the state is unchanged and
     `fired` is empty, as the README's "Every step is total" has it. One definition per
     party says whether an id has joined. The guard of each publish rule holds only for
     a publisher that has joined, and `guard_PullClaimsFrontPocket` and
     `guard_PullFindsNoPocket` hold only for a consumer that has joined. That conjunct
     transcribes the trigger's entity argument; it is not a new requirement. The
     README's "Extended, not owned" lists the conjunct on QM02's two pull guards. T45
     then removes two definitions and two rules.
   - Alternative A: `step` does not offer the step at all. The model then says nothing
     of it, `step` is not total over the walk's domains, and QM10 could not replay a
     walk step that names an unjoined id.
   - Alternative B: no conjunct. A publish or a pull by an id that never joined is
     answered like any other, and the joins change nothing any rule reads. That is the
     behaviour T45 lands, ahead of the Allium.
2. **The counter's first value.** The Allium declares the Queue in a `given` block (lines
   91 to 94) and gives `next_position` no first value. No property in the model reads an
   absolute position; every one compares two.
   - Recommended: `init` reads the first value from one constant that `pachinko_walk`
     sets to 0, and decision 0022 says the Allium states no first value and names T47.
     If QM02's seed already introduced such a constant, keep QM02's.
   - Alternative: the literal 0 in `init`. It reads as if the Allium said so.
3. **What identifies a publisher, a consumer, a pocket and a record.** The README lets an
   invariant be true by representation "only where the Allium's own notion of identity
   makes it so". The Allium finds each of the four by key and never by anything else:
   `Publisher{id: publisher_id}` (line 308), `Consumer{id: consumer_id}` (line 314),
   `Pocket{key: pocket_key}` (line 339) and `Record{pocket: pocket, key: record_key}`
   (line 372). The Vocabulary says a pocket is "named by a pocket key" (line 37) and a
   record is "the one slot a record key has in a pocket" (line 38).
   - Recommended: the joined publishers and the joined consumers are sets of ids, the
     pockets are a map from a pocket key, and the records are a map from a pocket key and
     a record key. `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey` and
     `OneRecordPerKeyPerPocket` are then true by representation. Each is still written
     as a property under its Allium name, and decision 0022 gets one line for each with
     this reason. What keeps each true in the Allium is a guard (`not exists …`), and
     those guards are checked: an action that fires a rule when its guard does not hold
     fails `FiredMatchesGuards`, and one that overwrites an entry fails the rule's
     effect.
   - Alternative: lists of entities, so that two entries can share an id or a key and
     each invariant can fail. It costs a lookup that may find two entries, which the
     Allium's rules do not define, and one more mutant and one more run per invariant.
     The case is weakest for `Consumer`, which has leases of its own; if the maintainer
     wants one of the four falsifiable, that is the one.
   - `UniqueRecordPosition` and `RecordPositionsBelowNext` are not by representation: a
     position is a field, and each has a mutant below.
4. **What a record gains.** QM02's record has only a pocket, a key and a status. This
   ticket adds the rank, the payload and the position. No action makes a record held
   before QM05. `guard_Supersede` says `record.status = queued`, as the first disjunct
   of line 398 does, and QM07 adds the second disjunct without rewriting the first.
   `queued_records` (line 131) is one definition, which QM05's `oldest` reads.
   `ending_records` stays QM02's one definition, empty until QM06. A new pocket's
   position is written inside the section QM02 landed for the pocket order.
   `PublishIntoNewPocket`
   writes no `consecutive_expiries`: the field enters the state with QM08, which adds it
   to this rule. No `shed_at` or `expires_at` enters before QM09 (shared rule 7).
5. **The superseded queue is a list.** Fixed by the README. An entry holds the pocket's
   key, the record's key, a rank, a payload and a reason, `superseded` or `stale`. A
   rule appends at the end. Two identical stale publishes make two entries.
6. **A payload is the publish's own tuple**: `publisher`, `pocket`, `key` and `rank`, as
   line 307 of `walk.rs` builds it. Fixed by the README; it is not a free domain.
7. **What each effect's frame holds.** The README has every effect state its frame: what
   no rule fired in the step writes is equal in `prev` and the state. Two parts of it
   are the Allium's own sentences. `effect_Supersede` says the record's position is
   unchanged, from lines 167 and 168: "A supersede changes rank and payload and leaves
   position alone." `effect_DropStalePublish` says no record, no pocket and not the
   counter changes, from lines 362 and 363: "A stale publish changes no record and does
   not touch the pocket". Each of the four publish effects says the superseded queue
   after the step is the queue of `prev` with exactly the new entry appended, or is
   equal to it. No publish here fires a second rule, so no effect shares a field with
   another yet; QM06, whose revival also advances the counter, meets that case.
8. **Three parts of guarantees, named for what they say.** The README keeps a
   guarantee's Allium name for its owner, QM07, and has a part checked earlier be "a
   property of its own, named `<Guarantee>_<part>` or for what it says, and the owner
   keeps it". These three are named for what they say. Each is a `val` with no
   parameter whose name starts with an upper-case letter, is transcribed from the
   Allium apart from the actions, and reads no `fired`.

   | Property | Says | Reads | Fragment of |
   | --- | --- | --- | --- |
   | `PublishAnsweredByOneRule` | For every joined publisher, pocket key, record key and rank in the domains, the number of publish rules whose guard holds in the state is 0 when the pocket has a record of that key at that rank, and 1 otherwise | no ghost | `NeverRefusedForCapacity` |
   | `QueuedRecordKeepsPosition` | A record that is queued in `prev` and queued in the state has the same position in both | `prev` | `OldestSlotNewestPayload` |
   | `ShedVersionIsQueued` | A record queued in `prev` whose rank or payload differs in the state has its `prev` version among the entries the step appended, with reason `superseded`. If `last` is a publish by a joined publisher below the rank the record had in `prev`, the published version is among them with reason `stale` | `last`, `prev` | `EveryDropIsVisible` |

   `PublishAnsweredByOneRule` quantifies over every argument in every reachable state,
   so it does not depend on which step was taken. That the publish rules read no size is
   shown by reading the four guards, and the model's header says so; it is not a checked
   property. Each part is written to stay true when records are held: the two that
   speak of a record speak only of one that is queued before the step.

   The count reads guards, not `step`. A `step` that stopped offering `publish` once
   every pocket key had a pocket would break no property, because a step that is not
   offered is in no trace, and every witness is reached before that state. So two
   things stand beside the count. Checkpoint 2 reads `step`: it offers `publish` for
   every publisher, pocket key, record key and rank of the instance, whatever the state
   holds. And step 5 observes one publish answered in a deployment whose three pocket
   keys all have a pocket. Neither is exhaustive; the hand-back says so.

   The Allium says of a publish at the stored rank that it "is rejected and changes
   nothing, whatever its payload" (lines 319 to 322). QM02's
   `UnansweredStepChangesNothing` checks that publish, and one by a publisher that has
   not joined, because `FiredMatchesGuards` keeps `fired` empty for both. This ticket
   adds no property of its own for it, and aims a mutant at the label check.
9. **The bound follows the deepest trace.** With the seed gone, every witness and every
   mutant needs its setup inside the trace. The plan writes the shortest trace for each
   witness and each mutant in the model, QM02's and QM03's included, and counts its
   steps after `init`. QM03's ticket proposes a bound of 10 for `pachinko_walk` and a
   floor of the deepest witness's depth plus two. If the bound in `quint/verify.txt` is
   below the longest trace here plus two, this ticket raises it. It never lowers it.

The mutants. A mutant names one property, which may be any property the recipe checks
(shared rule 6): an Allium name, an `effect_<Rule>`, a part of a guarantee, or a label
check. It names it in the header QM03 landed (its ticket proposes three lines,
`# mutant:`, `# instance:` and `# property:`), and is cut as QM03's ticket says a patch
is cut. The unchanged model's pass on `pachinko_walk` is the evidence that each named
property holds at the same bound; no mutant has a run of its own for that.

| Patch | Mutation | Property that must catch it | Shortest trace, steps after `init` |
| --- | --- | --- | --- |
| `quint/mutants/qm04-supersede-moves-position.patch` | `Supersede` gives the record the counter's value as a new position and advances the counter | `QueuedRecordKeepsPosition` | join, publish, publish at a higher rank: 3 |
| `quint/mutants/qm04-stale-publish-dropped-silently.patch` | `DropStalePublish` appends nothing | `ShedVersionIsQueued` | join, publish, publish at a lower rank: 3 |
| `quint/mutants/qm04-identical-stale-publish-lost.patch` | `DropStalePublish` appends only an entry the queue does not already hold | `effect_DropStalePublish` | join, publish, the same stale publish twice: 4 |
| `quint/mutants/qm04-new-pocket-counter-short.patch` | `PublishIntoNewPocket` advances the counter by one, not two | `RecordPositionsBelowNext` | join, publish: 2 |
| `quint/mutants/qm04-new-key-reuses-position.patch` | `PublishNewKey` gives the new record the counter's value less one | `UniqueRecordPosition` | join, publish, publish a second key: 3 |
| `quint/mutants/qm04-equal-rank-publish-replaces-payload.patch` | A publish at the stored rank replaces the record's payload and leaves `fired` empty | `UnansweredStepChangesNothing` | two publishers join, one publishes, the other publishes the same key at the same rank: 4 |

The last trace needs the second publisher: a payload is the publish's own tuple, so the
same publisher at the same rank writes the payload the record already has.
`PublishAnsweredByOneRule` has no mutant; the hand-back says so.

## Claims this ticket checks

Row 12 is the README's. The others are claims this ticket rests on that nobody has run.
What a bound of n steps reaches is QM03's to measure, at its step 2; the plan takes it
from QM03's hand-back.

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `verify-quint` (every instance, witness and mutant, with setup) finishes inside the CI job's timeout after this ticket's six witnesses, six mutants and deeper bound are added | steps 1 and 6: the wall time before and after, against `timeout-minutes` of the `quint` job in `.github/workflows/ci.yml`. Not read, not run |
| A | `quint verify` accepts a state that holds a list of records, `append` on it, equality between two such lists, and a map whose key is a pair | step 4, the first `just check-quint`, and step 5, the first green `just verify-quint`. The scratch module the README says passed `typecheck` held none of these. Not run |
| B | A property that quantifies over 72 argument tuples and four guards in every state does not by itself push the run past the budget | step 5: the wall time with `PublishAnsweredByOneRule` named and without it. Not run |

An answer that differs is a design change: stop and bring it to the maintainer.

## Non-goals

- No held record. A pull's limit stays at -1 and 0; QM05 widens it.
- No `DeferBehindHeldVersion` and no second case of `Supersede`; QM07.
- No idle or parked pocket and neither revive rule; QM06 and QM08.
- No `shed_at`, no `expires_at`, no decay and no clock; QM09.
- No property under the names `NeverRefusedForCapacity`, `EveryDropIsVisible` or
  `OldestSlotNewestPayload`. QM07 states them in the form they keep.
- Nothing about the eviction order under `NeverRefusedForCapacity`. Decision 0022
  excludes it.
- No change to `scripts/verify-quint.sh`, to the CI job or to any page of the handbook
  other than decision 0022.
- No new instance module and no smaller domain.

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

`<id>` is `QM04`. Checkpoints:

- Checkpoint 1, the plan, `ai_tmp/qm04/plan.md`: step 3, with the `conformance` and
  `breakage` lenses, before any tracked file changes.
- Checkpoint 2, the diff, with a design lens: step 8, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | Created by QM02. The seed removed and `init` empty; the publishers, record keys and ranks in `pachinko_walk`; records and the superseded queue in the state; three variants of `Step` and six of `Rule`; the three actions in `step`; `guard_`, `effect_` and `witness_` for six rules; `FiredMatchesGuards` over them; six invariants; three parts of guarantees as properties; the header's "Not yet modelled" |
| `quint/mutants/qm04-supersede-moves-position.patch` | new | names `QueuedRecordKeepsPosition` |
| `quint/mutants/qm04-stale-publish-dropped-silently.patch` | new | names `ShedVersionIsQueued` |
| `quint/mutants/qm04-identical-stale-publish-lost.patch` | new | names `effect_DropStalePublish` |
| `quint/mutants/qm04-new-pocket-counter-short.patch` | new | names `RecordPositionsBelowNext` |
| `quint/mutants/qm04-new-key-reuses-position.patch` | new | names `UniqueRecordPosition` |
| `quint/mutants/qm04-equal-rank-publish-replaces-payload.patch` | new | names `UnansweredStepChangesNothing` |
| `quint/mutants/*.patch` | shared, own lines | Created by QM03. Each patch the model's change breaks, re-cut with its mutation and its named property kept (shared rule 11) |
| `quint/verify.txt` | shared, own lines | Created by QM03. `pachinko_walk`'s `--max-steps`, only if design point 9 raises it; whatever else the recipe needs to list a new witness or mutant, in QM03's format |
| `quint/traces/`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines | Created by QM10, and the last by QM12, so absent when this ticket runs in series order. If the tree holds them: the fixtures regenerated by `just quint-traces`, never by hand, before any gate runs, since `just check-quint` and `just verify-quint` both read them; the generator's witness list brought up to the rules this ticket adds; and each replay kept green, each new field mapped or added to that replay's list of what it does not compare, and a new step listed there only with the maintainer's answer on which ticket maps it (shared rule 11). A change under `crates/` follows `AGENTS.md`'s block for Rust |
| `docs/decisions/0022-*.md` | shared, own lines | Decision 0022, created by QM02; the file name is the one QM02 landed. One line for each invariant true by representation with its reason, one for the counter's first value, and one for the ruling on design point 1 |
| `.scratch/quint-model/QM04-publish.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM04's Status cell; the sentences a ruling on design point 1, 2 or 3 changes, if it differs from the recommended answer |
| `.scratch/quint-model/QM*.md` | shared, own lines | Only if such a ruling differs: the sentences it changes in the later tickets the README's index names. QM05 is written against the recommended answers |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm04-publish` from the
   series base the maintainer names, once QM03 has merged there, and record the base's
   full commit id as `base`. `git ls-files .scratch/quint-model/` must list the README
   and this ticket. Run `just initialize` (network, authorised). Run `just check` on the
   untouched worktree and quote its last line, headed by the commit. Then show that
   nothing listens on `localhost:8822`, with the port probe and its positive control
   from Verification below, and run `just verify-quint` on the untouched worktree.
   Quote its closing lines, its exit status and its wall time. That time is the
   baseline for claim 12.
2. **Read before writing, and the plan.** Read everything under "Read these". Compare
   each Allium line this ticket quotes with the Allium at `base`. If a rule, an
   invariant or a surface this ticket owns or cites is gone or reads differently,
   **stop** and ask: T45, T39 or T47 may have merged. Count the variants of
   `Rule` in the base's model: 3, as this ticket counts them. If the count differs,
   name the T ticket behind the difference (README, "The T tickets that touch the
   model"), and have the plan recompute each trace, each step count and the bound,
   which is still never lowered.
   Write `ai_tmp/qm04/plan.md` with:
   - for each of the six rules, the Allium's text beside the planned action, its
     `guard_` and its `effect_`;
   - each of the six invariants and three parts of guarantees, with the ghost it reads
     and, for each invariant, one of: true by representation, with the reason; aimed at
     by a named mutant; or checked and not mutation-tested;
   - each mutant, the one property that must catch it, and its shortest trace;
   - the shortest trace from the empty start for every witness in the model and every
     mutant under `quint/mutants/` at the base, its length, and the `--max-steps` that
     follows against the one in `quint/verify.txt`;
   - each "Not yet modelled" line added or removed: the publish line QM02 wrote goes;
     lines for `DeferBehindHeldVersion` with the held case of `Supersede` (QM07),
     `ReviveIdlePocket` (QM06) and `ReviveParkedPocket` (QM08) are there afterwards;
     the line QM02 wrote for the lapse, parking and `consecutive_expiries` (QM08)
     stays, and covers the count `PublishIntoNewPocket` does not write yet (design
     point 4). If the base's header has no such line, this ticket adds one for the
     count that names QM08;
   - each patch at the base that the change will break.
3. **Adversarial review** (checkpoint 1: `conformance` and `breakage`, over
   `ai_tmp/qm04/plan.md`, before any tracked file changes; `tickets/CONVENTIONS.md`
   §14). Questions:
   - Does each planned guard say what the Allium's `requires` lines say and nothing
     more, apart from the joined conjunct of design point 1? Are the four publish guards
     disjoint, and do they leave exactly the equal rank unanswered?
   - Is any effect written from the planned action, not from the Allium's `ensures`?
     Could an effect hold while a stale publish changed a record or a supersede moved
     one?
   - Could `PublishAnsweredByOneRule` pass because the set it quantifies over is empty,
     for example before any publisher has joined, and is anything reachable left
     outside it?
   - Is each trace in the plan the shortest, and is any witness or mutant, QM02's and
     QM03's included, deeper than the bound the plan proposes?
   - Does a patch name a property that a different mutation would also violate first,
     so that it could be reported as caught for the wrong reason?
4. **Red: the names before the behaviour.** **Stop** first for the maintainer's ruling on
   "Design proposed", with the checkpoint's findings in hand, and record it. Then, in
   `quint/pachinko.qnt`: remove the seed and make `init` empty; add the constants, the
   state, the `Step` and `Rule` variants, the six guards, effects and witnesses, the
   invariants and the three parts; and add the three actions to `step` in their
   unanswered form only, so that each leaves the state alone and leaves `fired` empty.
   No stub writes `last`: `step` writes it for each, from the arguments it chose, as
   the README's second design block has it and as QM02 landed it. A stub that wrote
   `last` itself would give the same red run, so the run does not show which was built.
   QM03's recipe reads the rules, witnesses and properties from the model
   itself, so nothing is listed by hand; if its ruling differs, list them where it reads
   them. `just check-quint` passes (claim A, first half). QM03's ticket proposes a
   recipe that applies every patch before it verifies anything; step 2 read in its
   hand-back whether that landed. If it did, a patch the empty start breaks stops the
   run at "does not apply": re-cut each such patch first (shared rule 11) and run
   again. Then,
   with port 8822 shown free, `just verify-quint` fails. Quote what it reports, with its
   exit status. It must fail on `FiredMatchesGuards` or on a witness that is not reached,
   and the hand-back says which it met first.
5. **Green: the actions.** Write the three actions in full. Re-cut each patch under
   `quint/mutants/` that no longer applies, keeping its mutation and its named property,
   and list each in the hand-back with those of step 4. Raise `--max-steps` in
   `quint/verify.txt` if the plan says so. With port 8822 shown free, `just verify-quint`
   passes: quote `Canary` violated on `pachinko_walk`, every property named, and all nine
   witnesses reached (claim A). Then, by hand through
   `just quint`, with the command QM03's recipe prints for its pass: quote the wall time
   of a `verify` that names every property and of one that leaves out
   `PublishAnsweredByOneRule` (claim B). Then one observation by hand, run the way
   QM03's recipe runs a witness, on a scratch copy under `ai_tmp/qm04/` that adds one
   definition, `publishIntoFullDeployment`: `fired` holds `PublishNewKey`, `Supersede`
   or `DropStalePublish`, and every pocket key of the instance has a pocket in `prev`.
   Five steps reach it: a join, a publish into each of `a`, `b` and `c`, and one more
   publish. Quote it observed. Each `verify` by hand, here and in every later step,
   follows a port probe of its own (shared rule 10).
6. **The mutants.** Add the six patches. With port 8822 shown free, `just verify-quint`
   passes and reports each of the six caught by the property it names, and each of
   QM03's still caught by its own. Quote those lines. Quote the run's wall time beside
   step 1's and beside the CI job's `timeout-minutes` (claim 12). The head-room QM03's
   ruling set is the whole job's (its ticket proposes that a job over 15 of its 20
   minutes stops and asks), and this run is the recipe alone, on another machine. So add
   the durations of the setup and the installs that QM03's hand-back quotes from CI, and
   say in the hand-back that the sum is an estimate until the job runs. If the sum does
   not leave that head-room, **stop** and ask; do not shrink a domain, lower the bound
   or drop a mutant.
7. **Words.** Bring the model's header to the "Not yet modelled" list of the plan, and
   say there that the publish rules read no size. Add this ticket's lines to decision
   0022. If a ruling on design point 1, 2 or 3 differs from the recommended answer, edit
   the sentences it changes in `.scratch/quint-model/README.md` and in the later tickets
   its index names, and list them in the hand-back. Run `just check-docs`.
8. **Adversarial review** (checkpoint 2: `conformance`, `breakage` and `design`, over
   `git diff "$base"...HEAD` plus the uncommitted changes, with the output of steps 4
   to 6, before the final commit; `tickets/CONVENTIONS.md` §14). Questions:
   - Does the model say what the Allium says of each of the six rules, and of a publish
     at the stored rank? Is anything the Allium allows with nothing held missing and
     not listed under "Not yet modelled"?
   - Could any of the six invariants or three parts hold for a reason other than the
     one claimed: an empty domain, a property that reads the action's own helper, or a
     ghost that a guard or an effect on the state reads?
   - Does `init` hold anything a join or a publish should have created? Is any witness
     reached only through a state the seed used to provide?
   - Does `step` offer `publish` for every publisher, pocket key, record key and rank of
     the instance in every state, or is a choice made only when the state allows it: a
     free pocket key, a count below a limit, a rank above the stored one?
   - Does any of the three new actions write `last`, or does `step` alone write it,
     from the arguments it chose?
   - Is the body of each `witness_<Rule>` this ticket adds `fired.contains(<Rule>)` and
     nothing else? The recipe reaches a witness and never reads its body.
   - Was a re-cut patch changed in what it mutates or in the property it names?
   - Could Verification pass with a mutant surviving, a witness not reached, or a server
     on port 8822 that the run did not start?
   - Is every domain of `pachinko_walk` the walk's, and is the bound at least the
     deepest trace?
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

- `init` holds no publisher, no consumer, no pocket, no record, no lease and no
  superseded record, and `git diff "$base" -- quint/pachinko.qnt` shows the seed
  removed. Both quoted.
- `just check-quint` is green, quoted.
- Step 4's red run is quoted: `just verify-quint` failed before the actions existed.
- `just verify-quint` is green, quoted: `Canary` violated on `pachinko_walk`; every
  property named; the six owned rules' witnesses and QM02's three reached from the empty
  start; each of the six mutants caught by the property it names; QM03's mutants still
  caught.
- The run's wall time is quoted against step 1's baseline and against the CI job's
  `timeout-minutes`, with the setup and install durations from QM03's hand-back added.
- `publishIntoFullDeployment` is observed, quoted, and checkpoint 2's design reviewer
  answered the question on what `step` offers.
- The model's "Not yet modelled" list has no line for publish or for the joins, and has
  lines for `DeferBehindHeldVersion` with the held case of `Supersede`,
  `ReviveIdlePocket`, `ReviveParkedPocket` and `consecutive_expiries`, each naming its
  ticket. Quoted.
- No domain is shrunk: `pachinko_walk` has publishers `p0` and `p1`, consumers `c0` to
  `c2`, pockets `a`, `b` and `c`, record keys `k0` to `k2`, ranks 0 to 3 and pull limits
  -1 and 0, and `--max-steps` is not below the base's. Quoted.
- Decision 0022 has a line for each invariant true by representation, for the counter's
  first value and for the ruling on design point 1.
- The hand-back lists each patch re-cut, and each owned invariant that no mutant aims
  at, with `PublishAnsweredByOneRule`, as checked and not mutation-tested.
- Each of claims 12, A and B has its outcome recorded, and each design point its
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
mkdir -p ai_tmp/qm04

# Every owned rule has its three definitions. grep -c exits 1 on a count of 0.
for rule in PublisherJoins ConsumerJoins PublishIntoNewPocket PublishNewKey Supersede DropStalePublish; do
  for prefix in guard_ effect_ witness_; do
    grep -c "${prefix}${rule}" quint/pachinko.qnt
  done
done
ls quint/mutants/qm04-supersede-moves-position.patch \
  quint/mutants/qm04-stale-publish-dropped-silently.patch \
  quint/mutants/qm04-identical-stale-publish-lost.patch \
  quint/mutants/qm04-new-pocket-counter-short.patch \
  quint/mutants/qm04-new-key-reuses-position.patch \
  quint/mutants/qm04-equal-rank-publish-replaces-payload.patch

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
just verify-quint >| ai_tmp/qm04/verify.log 2>&1
end=$(date +%s)
echo "verify-quint: $((end - start)) seconds"
grep -n 'timeout-minutes' .github/workflows/ci.yml
for name in witness_PublisherJoins witness_ConsumerJoins witness_PublishIntoNewPocket \
  witness_PublishNewKey witness_Supersede witness_DropStalePublish \
  witness_PullClaimsFrontPocket witness_PullFindsNoPocket witness_ReleaseRequeuesPocket \
  qm04-supersede-moves-position qm04-stale-publish-dropped-silently \
  qm04-identical-stale-publish-lost qm04-new-pocket-counter-short \
  qm04-new-key-reuses-position qm04-equal-rank-publish-replaces-payload \
  Canary PublishAnsweredByOneRule UnansweredStepChangesNothing; do
  grep -c -F "$name" ai_tmp/qm04/verify.log
done

# One mutant by hand: red on the patched copy, green on the model, same command.
copy=ai_tmp/qm04/mutant.qnt
patch -o "$copy" -i quint/mutants/qm04-supersede-moves-position.patch quint/pachinko.qnt
just quint typecheck "$copy"
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
if FORCE_COLOR=0 just quint verify --main=pachinko_walk --invariant=QueuedRecordKeepsPosition --max-steps="$steps" "$copy" >| ai_tmp/qm04/red.log 2>&1; then exit 1; else test "$?" -eq 1; fi
grep -F '[violation] Found an issue' ai_tmp/qm04/red.log
grep -F 'found a counterexample' ai_tmp/qm04/red.log
if grep -F 'reached a deadlock' ai_tmp/qm04/red.log; then exit 1; else test "$?" -eq 1; fi
mark=$(printf '\342\235\214')
grep -Fx "  ${mark} QueuedRecordKeepsPosition" ai_tmp/qm04/red.log
test "$(grep -c -F "$mark" ai_tmp/qm04/red.log)" -eq 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
just quint verify --main=pachinko_walk --invariant=QueuedRecordKeepsPosition --max-steps="$steps" quint/pachinko.qnt >| ai_tmp/qm04/green.log 2>&1
grep -F '[ok] No violation found' ai_tmp/qm04/green.log
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

just check
status=$(git status --short)
test -z "$status"
```

The red test is the `verify` on the patched copy: it must exit non-zero and print the
violation line, after `typecheck` has shown the copy is well typed, so the failure is not
a type error. The violation line alone is not enough: the CLI prints it for a deadlock
too (`cliReporting.js`, line 246, with the two explanations at `apalache.js`, lines 144
and 146). So the log must also hold `found a counterexample` and must not hold
`reached a deadlock`. That the run names one invariant is not taken as proof of which
one failed: the log must hold the reporter's own line for it, two spaces, the cross mark
U+274C, a space and the property's name, and no other cross-mark line, and the run must
exit 1. That is the evidence QM03's ticket proposes for a mutant (its point 2); if the
reporter cannot evaluate the trace it prints a warning and no such line, and the check
fails. The search for the deadlock line passes by printing nothing, and
the search before it, over the same log, is its positive control. The green test is the
same command on the unchanged model. Together they
stand in for the canary on that one property: the same `--main`, the same bound, one
result each way. `just verify-quint` is the green test for everything else, and step 4
holds its red run. The recipe itself runs no unchanged-model run for a mutant: its pass on
the instance is that evidence (shared rule 6). The two result lines are the ones AP01's
Verification greps for with the pinned CLI. Each `grep -c` prints a count and fails on
zero, so none passes by printing nothing. The listener's `lsof` line is the positive
control for the four silent `lsof` probes, one before each `verify` and one after the
last: the same command, matching something.
`git status --short` is silent on a clean tree; `git rev-parse HEAD` and the commit of
step 9 show there was a tree to read. Three things here were written without being run.
The loop over names assumes the recipe's log prints each witness's and each mutant's name,
as QM03's ticket proposes. `patch -o` is not the recipe's way to apply a patch: QM03's
ticket proposes `git apply`, which takes no fuzz. The two explanation lines and the
cross-mark line were read in the installed source (`apalache.js`, lines 144 and 146;
`cliReporting.js`, line 171); QM03's step 2 runs them. If any does not hold at the base, correct
the line to what QM03 landed and record it under Departures.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Patches re-cut

### Invariants checked and not mutation-tested

### Departures

### Verification

## Open points

- **A party that has not joined.** The Allium does not say what a publish, a pull or a
  release naming an id that never joined does (design point 1; finding 4 of
  `double/SPEC-NOTES.md`, as `tickets/T45-drop-joining.md` cites it). The maintainer
  rules at step 4. The ruling stands until T45 lands and removes the question.
- **The counter's first value.** The Allium gives none (design point 2). The maintainer
  rules at step 4; T47 settles it in the Allium.
- **The three parts later.** QM07 owns the three guarantees and, as the README has it,
  keeps these three properties beside them. `PublishAnsweredByOneRule` gains a named
  exception in QM05, which QM07 removes.
- **By representation or falsifiable.** Design point 3. The maintainer rules at step 4.
- **How deep a pass reaches.** A green run says no property fails within `--max-steps`
  steps of the empty start. The bound this ticket sets is the deepest witness or mutant,
  no more. Whether to spend more of the CI budget on depth is the maintainer's, with
  step 6's wall time in hand.
