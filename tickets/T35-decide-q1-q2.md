---
id: T35
title: "Decide and land the held-version shed (Q1) and stale revival (Q2)"
status: todo
depends_on: [T32]
parallel_with: []
branch: ticket/t35-decide-q1-q2
estimated_size: M
---

# T35: Decide and land the held-version shed (Q1) and stale revival (Q2)

## Context

T31's `code-review` run 1 raised two questions about behaviour the specification and
the library implement today, and T31 parked both as proposed open questions for the
maintainer (`tickets/T31-mvp-integration.md`, "## Open points"):

- **Q1, the held-version shed.** "Proposed open question (`code-review` run 1, finding
  1): a supersede of a held record sheds the payload the consumer still holds, so a
  later nack dead-letters it too and it sits in both shed queues"
  (`tickets/T31-mvp-integration.md:923-925`).
- **Q2, the stale revival.** "Proposed open question (run 1, finding 2):
  `DropStalePublish` still touches the pocket, so a stale publish revives a parked
  pocket and resets its expiry count" (`tickets/T31-mvp-integration.md:926-928`).

T32 writes both into `docs/specs/pachinko.allium` as `open question` lines. This ticket
gets the maintainer's decision on each and lands it: the specification first, then the
scenarios, the obligations, the red tests, the code and the handbook. Q1 and Q2 are this
ticket's names. They are not the module's question 1 (equal-rank publishes) or question
2 (who reads the superseded and dead-letter queues); the module numbers T32's two
questions itself, and the executor reads those numbers from the module after T32 lands.

Every specification line below is as of `4daf9df`. T32 edits the module before this
ticket starts (it adds the two questions, and it resets `consecutive_expiries` on an
ack, settled or deferred (`AckSettlesRecord` and `AckDefersNewerVersion`), which
touches the ack rules), so the executor re-reads each cited rule at the merge of T32
and works from the lines it finds there. Code and test lines are also as of `4daf9df`.

### What the specification and the library do today

Q1, verified at `4daf9df`:

- `Supersede` always sheds the slot's loser as `SupersededRecord` with `reason:
  superseded`, held or not (`docs/specs/pachinko.allium:299-320`, the shed at
  `:308-316`). The library does the same: `crates/libpachinko/src/queue/pockets.rs:113-123`
  sheds whatever `supersede` returns, and `crates/libpachinko/src/records/memory.rs:96-106`
  swaps the slot's rank and payload and leaves `RecordStatus::Held { rank, payload }`
  alone, so the consumer still holds the version just shed.
- An ack of a superseded held version defers the newer one and sheds nothing:
  `AckDefersNewerVersion` (`docs/specs/pachinko.allium:456-472`) creates neither a
  `SupersededRecord` nor a `DeadLetter`; `Pockets::settle`
  (`crates/libpachinko/src/queue/pockets.rs:214-234`) defers, and `Queue::ack`
  (`crates/libpachinko/src/queue/settle.rs:58-66`) only returns the outcome.
- A nack of a superseded held version dead-letters the held version and defers the
  newer: `NackDefersNewerVersion` (`docs/specs/pachinko.allium:498-525`);
  `Queue::nack` (`crates/libpachinko/src/queue/settle.rs:95-117`) keeps a dead letter
  on both outcomes, settled and deferred.
- A release or a lapse returns a held record to queued and drops its held version:
  `RequeueHeldRecord` (`docs/specs/pachinko.allium:557-565`); `MemoryRecords::free_held`
  (`crates/libpachinko/src/records/memory.rs:143-153`), called from `Pockets::release`
  and `Pockets::lapse` (`crates/libpachinko/src/queue/pockets.rs:242-282`).

So only the nack path sheds the held version twice: `Supersede` puts it in the
superseded queue, then `NackDefersNewerVersion` puts the same rank and payload in the
dead-letter queue. No one test pins both halves. The nack half is pinned by
`a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer`
(`crates/libpachinko/tests/queue/settle.rs:261`), which asserts the dead letter but not
the superseded entry and carries three `// obligation:` tags (`:257-259`) and no
`// scenario:` tag. The supersede half is pinned by S045 (`:162`, asserting rank 1 shed
as superseded at `:168-174`) and by the walk model (`tests/queue/walk.rs`).
An ack sheds the held version once, at
supersede. `Supersede` sheds the slot's current loser, which is the held version only on
the first supersede during a hold; a second supersede sheds a version that was never
handed out (scenario C below, and S045,
`two_supersedes_while_held_shed_both_losers_and_leave_the_held_version`,
`crates/libpachinko/tests/queue/settle.rs:162`).

Q2, verified at `4daf9df`:

- `DropStalePublish` requires `rank < record.rank`, strictly, sheds the arrival as
  `reason: stale`, and emits `PocketTouched` (`docs/specs/pachinko.allium:322-339`);
  the module's comment says "Every publish into an existing pocket emits
  PocketTouched" (`:278-279`).
- `ReviveParkedPocket` answers it: the pocket is queued at a fresh position with
  `consecutive_expiries = 0` (`docs/specs/pachinko.allium:353-363`). `Pockets::publish`
  calls `touch` on every branch that does not return an error, the stale one included
  (`crates/libpachinko/src/queue/pockets.rs:125-146`, `touch` at `:64-79`).
- S024, `a_supersede_or_a_stale_publish_revives_a_parked_pocket`
  (`crates/libpachinko/tests/queue/expiry.rs:274`), pins it.
- An equal-rank publish is not stale: no rule covers it, the publish is rejected and
  touches nothing, so it does not revive (S025,
  `an_equal_rank_publish_does_not_revive_a_parked_pocket`,
  `crates/libpachinko/tests/queue/expiry.rs:299`; the module's question 1). A stale
  publish needs an existing record and an idle pocket has none (`IdleHasNoRecords`,
  `docs/specs/pachinko.allium:134-136`), so only a parked pocket is affected.

### The options

No option is chosen. The maintainer has said they lean to Q1 option 1, maybe together
with option 2; that is a leaning, recorded here so step 1 can confirm or replace it, and
nothing in this ticket builds on it before the answer.

Q1:

- **Today.** Keep both sheds and say so in the specification: a supersede sheds the
  held version, and a later nack dead-letters it as well.
- **Option 1, shed the held version at settle.** `Supersede` skips the shed when the
  loser is the held version (`record.status = held` and `record.rank =
  record.held_rank`); the held version is shed as `superseded` when the hold ends. The
  plain reading sheds a later supersede's loser at once; a stricter reading sheds
  nothing while the key is held and flushes every loser at settle, and differs only in
  scenario C. Option 1 is incomplete unless the shed also happens in
  `AckDefersNewerVersion` and in `RequeueHeldRecord` (release and lapse) when
  `held_rank < rank`: otherwise the held version is dropped with no record and
  `EveryDropIsVisible` (`docs/specs/pachinko.allium:793-796`) breaks.
  `NackDefersNewerVersion` keeps its dead letter and adds no shed.
- **Option 2, no dead letter for a superseded held version.** `NackDefersNewerVersion`
  drops its `DeadLetter.created`; supersede, ack and requeue are unchanged. The
  vocabulary's "Dead letter: a nacked payload" (`docs/specs/pachinko.allium:43`)
  narrows, and `nacked_by` is not recorded for this case, which touches the module's
  question 2 (what an entry must carry).
- **Options 1 and 2 together.** All of option 1, and `NackDefersNewerVersion` swaps its
  `DeadLetter.created` for a `SupersededRecord.created` of the held version: without
  that the held version lands in neither queue. Ack-defer and nack-defer then differ
  only in their trigger and in T32's `ensures: pocket.consecutive_expiries = 0`, which
  the ack carries and the nack does not; a merge into one rule must keep that
  difference.

Q2:

- **Today.** A stale publish revives a parked pocket; the specification says so.
- **No touch on stale.** `DropStalePublish` loses `ensures: PocketTouched`, so a stale
  resend is recorded as `stale` and changes nothing else, as an equal-rank publish
  already changes nothing. The comment at `docs/specs/pachinko.allium:278-279` changes;
  `ReviveParkedPocket`, `ReviveIdlePocket` and the module's question 7 (what else
  revives a parked pocket) do not.

### The scenarios

Q1: a consumer `c1` holds key `k` at rank 1 (`v1`); a publisher supersedes it while it
is held. Where the held version `v1` ends up, by option (option 1 in its plain reading):

| Scenario | Today | Option 1 | Option 2 | Options 1 and 2 |
| --- | --- | --- | --- | --- |
| A: publish `v1`, pull, publish `v3`, ack | superseded once, at supersede | superseded once, at the ack (a new `ensures`) | superseded once, at supersede | superseded once, at the ack |
| B: publish `v1`, pull, publish `v3`, nack | twice: superseded at supersede and a dead letter at the nack | once: a dead letter at the nack | once: superseded at supersede | nowhere, unless the nack sheds it as superseded |
| C: as B with `v5` published after `v3` | as B; `v3` superseded at the second supersede | as B; `v3` superseded at once | as B; `v3` as today | as B; `v3` superseded at once |
| D: publish `v1`, pull, publish `v3`, release | superseded once, at supersede; the release drops the held copy | nowhere, unless `RequeueHeldRecord` sheds it | superseded once, at supersede | nowhere, unless `RequeueHeldRecord` sheds it |

A lapse takes scenario D's path. Under option 1's stricter reading `v3` in scenario C
is held back too and shed at settle.

Q2: pocket `a` parked after three consecutive lapses, its record `a1` at rank 1. A
publish into it:

| Publish into parked `a` | Rule | Today | No touch on stale |
| --- | --- | --- | --- |
| A new key | `PublishNewKey` | revives | revives |
| A higher rank | `Supersede` | revives | revives |
| A lower rank (stale) | `DropStalePublish` | revives | stays parked |
| An equal rank | none (rejected) | stays parked | stays parked |

The `SupersededRecord` with `reason: stale` is created under both.

### The reference implementation

The maintainer's xzibitq repository holds `xzibitq/backend/src/xzibitq/qsketch.py`, a
reference implementation the maintainer pointed to. It is not in this repository and no
worktree here can read it; the line numbers below were read there on 2026-11-07, and the
executor asks the maintainer to confirm them rather than citing them as checked. On both
questions it behaves as the tree does today. That is a fact about the sketch, not a
recommendation for either option:

- Q1: `KeyedQueue.push` (lines 204-223) sheds the loser to `_superseded` at publish
  time (220) whether or not the key is held; `KeyedQueue.ack` (247-263) sheds nothing
  and moves the newer version to the back (261); `KeyedQueue.nack` (265-283) pushes the
  held version to `_deadletter` (281) whether or not it was superseded, the double
  shed; `KeyedQueue.release` (285-290) clears `_held` and sheds nothing.
- Q2: `MultiLeaseQueue.push` (533-545) takes the pocket out of `_q_in_waiting` (539)
  and back into `_qs` (543) before the rank check runs (545), so any publish revives a
  waiting pocket, a stale one included. Outside this ticket: a stale push is not
  recorded at all there (215-217), and revival does not reset the eviction count (only
  release does, 666), where the specification resets `consecutive_expiries`.

### The impact per option

Rust paths are under `crates/libpachinko/`. The obligation ids are inferred from the
grammar of the existing `// obligation:` tags (`rule-success.X`, `rule-failure.X.N`,
`rule-entity-creation.X.N`, `guarantee.X`); they are guesses until
`just plan-spec pachinko` runs on the edited module at step 4, and only its output is
cited in the tests and the hand-back.

| Option | Rules | Scenarios and tests | Obligations (inferred) | Code |
| --- | --- | --- | --- | --- |
| Q1 option 1 | `Supersede` sheds nothing when the loser is the held version (one rule cannot both shed and not, so likely two rules); `AckDefersNewerVersion` gains `SupersededRecord.created`; `RequeueHeldRecord` splits on `held_rank < rank` and sheds; `EveryDropIsVisible` and `NothingLostWhileHeld` text | S043 (`tests/queue/settle.rs:111`) also sheds; S045 (`:162`) sheds only `v3` at publish; S048 (`tests/queue/expiry.rs:368`); `tests/queue/settle.rs:261`; new: a superseded held version then a release, and then a lapse | changed: `rule-success.Supersede`, `guarantee.NothingLostWhileHeld`, `guarantee.EveryDropIsVisible`; new: success and failure ids for the split rules, `rule-entity-creation.AckDefersNewerVersion.1`, an entity-creation id on the requeue path | `src/queue/pockets.rs:113-123` reads the held status; `src/queue/pockets.rs:214-234` and `src/queue/settle.rs:58-66` shed on the deferred ack, stamped with `superseded_ttl`; `src/queue/pockets.rs:242-282` and `src/records/memory.rs:143-153` shed on release and lapse; the walk model in `tests/queue/walk.rs` (publish `:361-379`, settle `:571-611`, release `:636`, lapse `:686`) |
| Q1 option 2 | `NackDefersNewerVersion` drops `DeadLetter.created`; the vocabulary's dead letter narrows | `tests/queue/settle.rs:261` asserts no dead letter; S048 asserts none (it asserts nothing about dead letters today) | removed: `rule-entity-creation.NackDefersNewerVersion.1`; changed: `rule-success.NackDefersNewerVersion` | `src/queue/settle.rs:95-117` keeps a dead letter only on `SettleOutcome::Settled`; the walk model `tests/queue/walk.rs:571-611` |
| Q1 options 1 and 2 | option 1's, and `NackDefersNewerVersion` creates a `SupersededRecord` in place of its `DeadLetter` | option 1's, and `tests/queue/settle.rs:261` asserts a superseded entry and no dead letter | option 1's, and `rule-entity-creation.NackDefersNewerVersion.1` names a `SupersededRecord` | option 1's, and `src/queue/settle.rs:95-117` sheds rather than dead-letters on `Deferred` |
| Q2 no touch on stale | `DropStalePublish` loses `ensures: PocketTouched`; the comment at `:278-279` | S024 splits: a supersede revives, a stale publish does not; S026's wording; new: a stale resend leaves a parked pocket parked | changed: `rule-success.DropStalePublish`, `rule-failure.ReviveParkedPocket.1` (S024) | `src/queue/pockets.rs:125-146` returns before `touch` on the stale branch; the docs at `src/queue/publish.rs:1-2` and `:14-20`; the walk model `tests/queue/walk.rs:368-379` |

Two code paths option 1 needs that are not settled here. The release and lapse shed
needs the held versions before `free_held` drops them: either the pocket step reads
`PocketRecords::held` (`src/records.rs:310-323`) and `lookup` first, which changes no
trait, or `free_held` (`src/records.rs:280-295`) returns the versions it drops, which
changes the public `PocketRecords` trait and with it `src/records/memory.rs:143-153`,
the contract at `src/records/contract.rs:327`, the doc example's implementation at
`src/queue.rs:98` and the counting wrapper at `tests/queue/counting.rs`. The `supersede`
contract (`src/records.rs:206-226`, `src/records/memory.rs:96-106`, its contract test at
`src/records/contract.rs:167`) changes only if the plan moves the held check into the
store. Step 3 chooses, and a trait change is reported by `just semver`.

### Risks (`CONVENTIONS.md` §13)

- **The small denominator.** A new branch in `pockets.rs` or `settle.rs` with no test
  can breach the 90% floor; each branch the decision adds has its scenario test.
- **lychee and typos over `tickets/`.** The hand-back notes quote test names and paths
  as code spans, never as links.
- **The specification moves first.** A code change that lands before its rule, or a rule
  that names an obligation `just plan-spec pachinko` does not print, is a defect here
  (`AGENTS.md`, invariant 1).

## Goal

The maintainer's answers to Q1 and Q2 are recorded, quoted and dated in the hand-back.
The two `open question` lines T32 added are gone from `docs/specs/pachinko.allium`, each
replaced by the decision written into the rules it governs and a comment that says it
was decided; the rules, guarantees and vocabulary say what was decided; every scenario
the decision changes or adds is pinned by a tagged library test that was red before the
code changed (or, for an answer that keeps today's behaviour, a test shown to fail when
the code is changed to the rejected option); the library does what the rules say; and
the handbook's catalog, open-question lists and wire description agree.

This ticket owns:

| Kind | Names |
| --- | --- |
| Open questions (2) | the two T32 added: the held-version shed (Q1) and the stale revival (Q2) |
| Rules | those the decision changes, from `Supersede`, `DropStalePublish`, `AckDefersNewerVersion`, `NackDefersNewerVersion` and `RequeueHeldRecord`, and any rule a split creates |
| Guarantees | the text of `EveryDropIsVisible` and `NothingLostWhileHeld`, if the decision changes what they promise |
| Scenarios | the rows the decision changes among S024, S026, S043, S045 and S048; the scenario `tests/queue/settle.rs:261` pins, tagged with the next free id; and each new scenario, at the next free ids after T32's; and, whatever the answers, S024 and S045 lose the `oq:N` labels T32 gave them |
| Obligations | each id `just plan-spec pachinko` prints for the rules above after step 4, confirmed then, not from the inferred list in Context |

## Non-goals

- No answer to any other open question: not the module's question 1 (equal rank), 2
  (who reads the queues; option 2 touches it, and the hand-back says how, without
  answering it), 7 (what else revives a parked pocket) or 9.
- No change to T32's `consecutive_expiries` reset on an ack, settled or deferred
  (`AckSettlesRecord` and `AckDefersNewerVersion`), and no reset on a nack; this ticket
  builds on it.
- No new route, flag, error code or wire field: the publish outcomes (`stale`,
  `superseded`) and the ack and nack outcomes (`settled`, `deferred`) keep their names.
- No decision record unless the maintainer asks for one at step 1; if asked, it is
  handed back as its own `main` pull request, because a new record under
  `docs/decisions/` is outside Files touched.
- No edit to `crates/pachinko/` source: the service passes the library's behaviour
  through. A service test that fails is a finding, handed back.
- No implementation of option 1's stricter reading: if the maintainer chooses it,
  step 1 records the answer and stops.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The two `open question` lines removed with the decision recorded; the rules, guarantees, vocabulary and comments the decision changes |
| `crates/libpachinko/src/queue/pockets.rs` | shared, own lines | Q1 option 1: the held check in the supersede branch (`:113-123`), the shed on deferral (`:214-234`) and on release and lapse (`:242-282`); Q2: the stale branch before `touch` (`:125-146`); unit tests under `#[cfg(test)]` |
| `crates/libpachinko/src/queue/settle.rs` | shared, own lines | Q1: the ack's shed on `Deferred` (`:58-66`), the nack's dead letter or shed on `Deferred` (`:95-117`); the module and method docs |
| `crates/libpachinko/src/queue/publish.rs` | shared, own lines | The module doc (`:1-2`) under Q2's no touch; `Queue::publish`'s doc (`:14-20`) under Q1 option 1 (a held loser is not shed at supersede, "sheds the loser") or Q2's no touch ("revives an idle or parked one") |
| `crates/libpachinko/src/queue/types.rs` | shared, own lines | Q1 option 1: `PublishOutcome::Superseded`'s doc (`:70-72`) and `SupersededRecord`'s doc (`:248-250`) where they say the loser is shed at supersede; their doctests |
| `crates/libpachinko/src/queue/renew.rs`, `crates/libpachinko/src/queue/expiry.rs` | shared, own lines | Q1 option 1: the release and lapse callers stamp the shed with `now` and `superseded_ttl`, if the plan sheds there |
| `crates/libpachinko/src/queue/shed.rs` | shared, own lines | A keep path for a held version shed outside publish, if the plan needs one |
| `crates/libpachinko/src/records.rs`, `crates/libpachinko/src/records/memory.rs`, `crates/libpachinko/src/records/contract.rs`, `crates/libpachinko/src/queue.rs` | shared, own lines | Only if step 3 changes `free_held` or `supersede`: the trait, `MemoryRecords`, the contract and the doc example's implementation at `src/queue.rs:98` |
| `crates/libpachinko/tests/queue/settle.rs` | shared, own lines | S043, S045, the test at `:261` tagged and asserting the decision; the new Q1 scenarios on ack and nack |
| `crates/libpachinko/tests/queue/expiry.rs` | shared, own lines | S024 (split under Q2's no touch), S026's wording, S048, the release and lapse scenarios of option 1 |
| `crates/libpachinko/tests/queue/lease.rs` | shared, own lines | A release of a superseded held version, if option 1 is chosen and the test sits beside S053 |
| `crates/libpachinko/tests/queue/walk.rs` | shared, own lines | The model's publish (`:361-379`), settle (`:571-611`), release (`:636`) and lapse (`:686`) predict the decided behaviour |
| `crates/libpachinko/tests/queue/counting.rs` | shared, own lines | Only if `free_held`'s signature changes |
| `docs/reference/testing.md` | shared, own lines | The catalog rows for S024 to S026 (`:185-187`), S043 to S048 (`:204-209`) and each new id |
| `docs/reference/http-api.md` | shared, own lines | The nack sentence (`:62-63`) under option 2 or options 1 and 2; "until a publish into it" (`:74`) under Q2's no touch; the shed paragraph (`:224-227`) if what a dead letter holds changes |
| `docs/explanation/architecture.md` | shared, own lines | "Every publish into an existing pocket ends in the `PocketTouched` dispatch" (`:204-205`) under Q2's no touch; the dead letter sentence (`:190-191`) under option 2 |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | "Handle an open question" (`:61-85` at `4daf9df`; T32 makes the counts at `:64` and `:75` say ten and adds T32's two questions to the list at `:77-85`; re-read at T32's merge): the counts and the list lose the two |
| `docs/explanation/specifications.md` | shared, own lines | "Open questions are a feature" (`:56-66` at `4daf9df`; T32 makes it ten and names the two and their origin; re-read at T32's merge): the count, the list and the origin sentence lose the two |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Changed (the open question 8 entry at `:16-19` is the model); pachinko Unreleased, Changed, for what a client of the service observes |
| `tickets/README.md` | shared, own lines | T35's Status cell |
| `tickets/T35-decide-q1-q2.md` | ticket | `status:` and hand-back notes |

## Steps

1. **Authorisation required: the product decisions on Q1 and Q2.** These are the
   maintainer's decisions, and an `open question` is never resolved silently
   (`AGENTS.md`, "you are writing or changing a module under docs/specs/"). Ask the
   maintainer, citing the explainer page
   `https://claude.ai/artifact/EF8iWnTcdxrrQ2bLbcyYDL`, which steps through scenarios A to D and the Q2 table under each option, and the
   Context of this ticket:
   - Q1: today, option 1 (and in which reading, plain or stricter; the stricter reading
     is handed back, below), option 2, or options 1 and 2 together. The maintainer's
     stated leaning is option 1, maybe with option 2; ask for the decision, not a
     confirmation of the leaning.
   - Q2: today, or no touch on stale.
   - Under options 1 and 2 together: whether `AckDefersNewerVersion` and
     `NackDefersNewerVersion` merge into one rule, and that a merge keeps T32's
     `consecutive_expiries` reset on the deferred ack and adds none to the nack.
   - Under option 2: that the dead letter of this case, with its `nacked_by`, goes, and
     that the module's question 2 stays open.
   - Whether a decision record is wanted (Non-goals).
   - Whether the qsketch line numbers in Context are right, since this repository
     cannot read them.

   Then stop and do nothing further until the maintainer answers. Quote the answers in
   the hand-back, dated. An answer of "today" still resolves the question: its
   `open question` line goes, the decision is written as a comment on the rule, and no
   code changes. An answer that names something outside the options above is a new
   design: record it and stop again, rather than fit it to an option. Option 1's
   stricter reading is in that class: this ticket plans only the plain reading, and the
   stricter one holds every loser back until settle, a `PocketRecords` storage change
   that Files touched does not name. Record that answer, stop and hand it back for its
   own ticket. No review
   precedes this stop, because nothing has changed yet; T35-1 follows the first edit.
   This step needs no worktree; step 2 creates the worktree only after the answers.
2. Create the worktree from `main` after T32 has merged (`README.md`, "How to pick up a
   ticket") and run `just initialize` there (network, authorised by this step), or, in
   a clean worktree already initialized on `main` whose branch carries nothing, create
   the branch there and skip `just initialize`. Run `just check` on the untouched
   worktree and quote its last line as the baseline.
3. **The plan.** Use the `plan-change` skill. Read the whole module at T32's merge,
   including the two `open question` lines T32 added and their numbers, and re-locate
   every rule Context cites by its name. Run `just plan-spec pachinko` and keep its
   obligation list as the before list. Map the decision onto the impact table in
   Context; for option 1, choose between reading `PocketRecords::held` before
   `free_held` and changing `free_held`'s return, and say why (a public trait change is
   a breaking change to `libpachinko` and is reported by `just semver`). List each test
   that must flip, each new scenario with its next free id from
   `docs/reference/testing.md`'s catalog, and each handbook line in Files touched the
   decision reaches. Any file the plan needs outside Files touched is handed back.
4. **The specification.** Use the `spec-change` skill, which hands the Allium work to
   the `allium` skill. Remove the two `open question` lines T32 added, in the same
   commit as the decision is stated (`docs/how-to/work-with-the-specs.md`, "Handle an
   open question"), and extend the comment above the open questions (at `4daf9df`,
   `docs/specs/pachinko.allium:878-884`) so it says the two were closed by the
   maintainer's decision and the surviving numbers do not compact. Then, as decided:
   - Q1 option 1: guard the shed in `Supersede` on the loser not being the held version
     (a split into two rules if one rule cannot both shed and not), add
     `SupersededRecord.created` of the held version to `AckDefersNewerVersion`, split
     `RequeueHeldRecord` on `held_rank < rank` with the shed on the superseded side,
     and reword `EveryDropIsVisible` and `NothingLostWhileHeld` so a held loser becomes
     a `SupersededRecord` when the hold ends by an ack, a release or a lapse, and a
     `DeadLetter` when it ends by a nack.
   - Q1 option 2: remove `DeadLetter.created` from `NackDefersNewerVersion` and narrow
     the vocabulary's dead letter to a nacked latest version.
   - Q1 options 1 and 2: option 1, and `NackDefersNewerVersion` creates a
     `SupersededRecord` of the held version in place of its `DeadLetter`, and reword
     both guarantees so a held loser becomes a `SupersededRecord` however the hold
     ends.
   - Q2 no touch on stale: remove `ensures: PocketTouched` from `DropStalePublish` and
     reword the comment at `:278-279`.
   - An answer of today: no rule changes; a comment on the rule says the behaviour is
     decided and why (the double shed on a nack; the stale revival).

   Every SupersededRecord the rules create keeps `shed_at: now` and `expires_at: now +
   config.superseded_ttl`. Run `just check-specs`, `just analyse-specs` and
   `just plan-spec pachinko`, and diff the obligation list against step 3's before list:
   every id the hand-back and the tests name comes from this output, never from the
   inferred list in Context.
5. **Adversarial review** (`CONVENTIONS.md` §14), after the specification and before
   any test or code builds on it: the specification diff against the maintainer's
   quoted answers, Context's option definitions, scenarios A to D and the Q2 table.
   Ask: whether any `open question` line went without the decision stated in the rules;
   whether any path of scenarios A to D, or a lapse, drops the held version without a
   `SupersededRecord` or `DeadLetter`, so that `EveryDropIsVisible` fails; whether a
   held version can still land in both queues under an option that forbids it; whether
   the split rules' guards are disjoint and cover every case the old rule covered;
   whether a stale publish still reaches `PocketTouched` under no touch, or a supersede
   or a new key stops reaching it; whether any guarantee or vocabulary text still
   describes the old behaviour; whether `just check-specs` and `just analyse-specs`
   could pass with a rule that never fires; and whether the decision answers a
   question this ticket's Non-goals leave open. Two reviewer subagents with fresh
   contexts run in parallel, one lens each, `conformance` and `breakage`, each running
   the §14 harness the executing agent is not: from Claude Code, `codex exec -m
   gpt-6-astra -s read-only -C <worktree> -o ai_tmp/review/T35-1/findings-<lens>.md
   "<brief>" < /dev/null` (`-o` writes only the reviewer's last message, and a
   read-only reviewer writes no file of its own); from Codex, `claude -p --model
   claude-opus-5-5 --permission-mode plan "<brief>" >
   ai_tmp/review/T35-1/findings-<lens>.md`; separate triage subagents check every
   finding against the tree and write `ai_tmp/review/T35-1/verdicts.md`, each finding
   valid with its fix or invalid with the reason; one writer applies the valid fixes one
   at a time, only within Files touched, and hands back any fix outside it; if a
   high-severity finding was valid, a second round reviews the fix diff and writes
   beside the first with a `-2` suffix; two rounds at most. A finding that the decision
   itself is wrong goes back to the maintainer as a question, never into a fix. Rerun
   `just check-specs`, `just analyse-specs` and `just plan-spec pachinko` after the
   fixes.
6. **The red tests.** Use the `rust-change` skill. Derive the tests from the changed
   clauses before any code changes. Each new or changed test carries `// scenario:` with
   its id and `// obligation:` with the ids step 4's `just plan-spec pachinko` printed,
   in the shape the existing tags use (`rg -n '// obligation:' crates`). The designs:
   - Q1, every option: scenarios A, B, C and D, each asserting the superseded queue and
     the dead-letter queue entry by entry (rank, payload, reason or `nacked_by`), the
     record's status, rank and position, and that the held version appears exactly as
     many times as the decided column of Context's table says; under option 1, D runs
     once with a release and once with a lapse. The test at
     `tests/queue/settle.rs:261` takes a scenario id and the decided assertion, and
     asserts the superseded queue as well as the dead-letter queue.
   - Q1 option 1: S045 asserts only `v3` at publish, then `v1` at the ack; S043 asserts
     the superseded entry the ack creates, stamped `shed_at` at the ack.
   - Q1 option 2: S048 asserts that no dead letter exists after the deferred nack.
   - Q1 options 1 and 2: scenario A asserts `consecutive_expiries` is 0 after the
     deferred ack, and scenario B that the deferred nack leaves it unchanged.
   - Q2 no touch on stale: S024 splits into a supersede that revives and a stale
     publish that leaves the pocket parked, `consecutive_expiries` unchanged and the
     snapshot unchanged but for the new superseded entry; S026's label follows.
   - The walk model in `tests/queue/walk.rs` predicts the decided behaviour, so the
     invariant walk fails until the code follows.
   - Whatever the answers: S024's and S045's `// scenario:` comments and test names lose
     any open-question wording T32 gave them; a test name changes only if it names the
     question.

   Run `just test` and quote the failures, each test red for the reason the decision
   gives (shown to bite, as T26's hand-back does). A test whose scenario keeps today's
   behaviour under the decided option (for example scenarios A and D under Q1 option 2,
   or S024's reviving supersede under Q2's no touch on stale) is a pinning test, not a
   red test: on a scratch copy that is never committed, change the code to break that
   scenario and quote the test failing, then restore the code.
   For an answer of today on either
   question there is nothing to make red: instead, on a scratch copy never committed,
   change the code to the rejected option (for Q1, drop the dead letter on `Deferred` at
   `src/queue/settle.rs:107-115`; for Q2, return before `touch` on the stale branch)
   and quote the pinning tests failing, then restore it.
7. **The code.** Use the `rust-change` skill. Implement until step 6's tests pass, in
   the files Files touched names, along step 3's plan. A shed outside publish is
   stamped with the one `now` the operation read, as the nack's dead letter is
   (`src/queue/settle.rs:13-22`). Update the module and method docs that describe the
   old behaviour as the decision reaches them (`src/queue/publish.rs:1-2` and `:14-20`,
   `src/queue/types.rs:70-72` and `:248`, `src/queue/settle.rs:1-2` and `:68-72`,
   `src/queue/pockets.rs:59-63` and `:208-213`, `src/records.rs:280-282`).
   Run `just fmt-check`, `just clippy`, `just test`, `just doc`, `just coverage` and
   `just effects`; if `PocketRecords` changed, `just semver` too, quoting its report.
8. **The handbook.** Use the `review-docs` skill. `docs/reference/testing.md`: the
   catalog rows step 6 changed or added, each naming its test by file and name; whatever
   the answers, the rows for S024 and S045 drop the `oq:N` tag T32 gave them (leaving
   `edge`, or `decision:NNNN` if step 1 asked for a decision record), and the
   description's open-question suffix becomes the decided behaviour;
   `docs/reference/http-api.md`, `docs/explanation/architecture.md`,
   `docs/how-to/work-with-the-specs.md` and `docs/explanation/specifications.md`: the
   lines in Files touched the decision reaches, and the two open-question lists lose
   the two questions with their counts; `CHANGELOG.md`: libpachinko Unreleased, Changed,
   naming the decision and the rules, and pachinko Unreleased, Changed, for what a
   client observes (a nack's dead letter, a stale publish's revival). A line the
   decision does not reach is left alone and recorded as such. Run `just check-docs`.
9. **Adversarial review** (`CONVENTIONS.md` §14), the whole diff before the final
   commit, against the maintainer's quoted answers, the Goal table, Files touched and
   the specification at step 5's state. Ask: whether the code does what each changed
   rule says in every arm, including a held version superseded twice, a release, a
   lapse that parks, and a nack after a lapse-and-repull; whether any test passes with
   the code reverted to `4daf9df`'s behaviour; whether the walk model and the queue
   agree only because both were changed the same wrong way; whether any obligation tag
   names an id `just plan-spec pachinko` does not print; whether a scenario id is
   tagged twice or a catalog row names a test that does not exist; whether any page
   still describes the old behaviour or counts the two closed questions; whether a
   public item changed without its doctest; and whether any file outside Files touched
   changed. Two reviewer subagents with fresh contexts run in parallel, one lens each,
   `conformance` and `breakage`, each running the §14 harness the executing agent is
   not: from Claude Code, `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T35-2/findings-<lens>.md "<brief>" < /dev/null` (`-o` writes only the
   reviewer's last message); from Codex, `claude -p --model claude-opus-5-5
   --permission-mode plan "<brief>" > ai_tmp/review/T35-2/findings-<lens>.md`; separate
   triage subagents write `ai_tmp/review/T35-2/verdicts.md`; one writer applies the
   valid fixes one at a time, only within Files touched, and hands back any
   fix outside it; if a high-severity finding was valid, a second round reviews the fix
   diff and writes beside the first with a `-2` suffix; two rounds at most.
10. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
    `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
    `git fetch origin` (network, authorised by this step) and merge `origin/main`
    (`CONVENTIONS.md` §11); `Cargo.lock` is regenerated with `just lock` (network,
    authorised by this step) if the merge touched it, never merged by hand. Rerun the
    Verification block at the merge commit and requote it. Set `status: done` and T35's
    Status cell in `tickets/README.md` to `done`, and commit
    on the ticket branch with the hand-back notes, each quoted output headed by the
    commit it ran at. **Authorisation required: push and open a pull request.** Stop and
    ask. Merging to `main` runs `pages.yml`, which deploys the API reference (decision
    0013), so the merge is the maintainer's.

## Acceptance criteria

- The maintainer's answers to Q1 and Q2 are quoted in the hand-back with their date,
  and nothing before step 1's answer chose an option.
- `docs/specs/pachinko.allium` carries neither of the two `open question` lines T32
  added, `rg -c '^open question' docs/specs/pachinko.allium` prints T32's count less two,
  and each decision is stated in the rules it governs with a comment that says it was
  decided.
- Under the decided rules, scenarios A to D and the Q2 table in Context each hold as
  the decided column says, each pinned by a tagged library test; no path drops the held
  version without a `SupersededRecord` or a `DeadLetter`.
- Each new or changed test whose scenario the decision changes or adds was red before
  the code changed, and the hand-back quotes the failures. Each test whose scenario
  keeps today's behaviour, including every test for an answer of today, is quoted
  failing under a scratch-copy mutation that breaks it.
- Every `// obligation:` id the ticket adds is in `just plan-spec pachinko`'s output,
  and the hand-back lists them with the before and after obligation counts.
- The scenario ids over `crates/libpachinko/tests` and `crates/libpachinko/src` are
  each tagged once and run contiguously from S001 to the catalog's last row, and
  `docs/reference/testing.md`'s catalog has one row per id naming an existing test.
- No catalog row or test carries the `oq:N` label of either closed question.
- The walk model in `tests/queue/walk.rs` predicts the decided behaviour, and the
  invariant walk passes.
- `docs/how-to/work-with-the-specs.md` and `docs/explanation/specifications.md` list
  the module's remaining open questions one for one; `http-api.md`, `architecture.md`
  and both `CHANGELOG.md` sections say what landed.
- Every new or changed public item keeps invariant 3 and has a doctest; `just coverage`
  reports at or above 90% of lines over `crates/*/src/**`; `just effects` passes.
- Both adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding
  is fixed or handed back.
- `just check-specs`, `just analyse-specs`, `just check-docs` and `just check` are
  green at the commit that merges `origin/main`, and `git status --short` prints
  nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). The
scenario lines that must print nothing follow their positive control, the count line.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
just plan-spec pachinko; echo "exit $?"
rg -c '^open question' docs/specs/pachinko.allium; echo "exit $?"
rg -c 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort | uniq -d; echo "exit $?"
last=$(rg -o '^\| S([0-9]{3}) \|' -r '$1' docs/reference/testing.md | sort -n | tail -1)
echo "last S$last"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | diff - <(seq -f 'scenario: S%03g' 1 "$((10#$last))"); echo "exit $?"
rg -o '^\| (S[0-9]{3}) \|' -r '$1' docs/reference/testing.md \
  | sort | diff - <(seq -f 'S%03g' 1 "$((10#$last))"); echo "exit $?"
# each test a catalog row names exists; S005 names a doctest, checked by hand
rg -c '^\| S[0-9]{3} \|.*\| `[^`]+\.rs`, `[a-z0-9_]+`' docs/reference/testing.md; echo "exit $?"
rg -o '^\| S[0-9]{3} \|.*\| `([^`]+\.rs)`, `([a-z0-9_]+)`' -r '$1 $2' \
  docs/reference/testing.md \
  | ( bad=0; while read -r f n; do
        rg -q "fn $n\b" "$f" || { echo "MISSING $f $n"; bad=1; }
      done; exit "$bad" ); echo "exit $?"
# no oq:N label of the two closed questions; substitute T32's two numbers, read at step 3
# positive control first: the labels of the questions still open must match (exit 0)
rg -c 'oq:[0-9]+\b' docs/reference/testing.md crates/libpachinko/tests; echo "exit $?"
! rg -n 'oq:(10|11)\b' docs/reference/testing.md crates/libpachinko/tests; echo "exit $?"
rg -n '// obligation:' crates/libpachinko/tests/queue/settle.rs \
  crates/libpachinko/tests/queue/expiry.rs; echo "exit $?"
just fmt-check; echo "exit $?"
just clippy; echo "exit $?"
just test; echo "exit $?"
just doc; echo "exit $?"
just effects; echo "exit $?"
just coverage; echo "exit $?"
# only if PocketRecords changed (step 3)
just semver; echo "exit $?"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

### The decisions (step 1)

### Baseline

### The plan (step 3)

### The specification (step 4)

### T35-1: the specification (step 5)

### Shown to bite (step 6)

### The code (step 7)

### The handbook (step 8)

### T35-2: the whole diff (step 9)

### Step 10: review, merge and gate

## Open points

- The qsketch line numbers in Context were read outside this repository; step 1 asks
  the maintainer to confirm them.
- Under option 2, a nack of a superseded held version leaves no record of who nacked
  it; whether an entry must carry that is the module's question 2, which stays open.
- Under options 1 and 2, whether `AckDefersNewerVersion` and `NackDefersNewerVersion`
  merge, keeping T32's `consecutive_expiries` reset on the ack only, is asked at step 1;
  a merge renames obligation ids, which step 4's `just plan-spec pachinko` settles.
- Whether a decision record is wanted is asked at step 1; if so, it is a separate
  `main` pull request (Non-goals).
- qsketch does not record a stale push and does not reset the eviction count on
  revival, where the specification sheds the push as stale and resets
  `consecutive_expiries`; neither is this ticket's question. Owner: maintainer.
