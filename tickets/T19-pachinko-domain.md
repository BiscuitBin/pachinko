---
id: T19
title: "Pachinko domain: the first behavioural module"
status: done
depends_on: [T18]
parallel_with: []
branch: ticket/t19-pachinko-domain
estimated_size: M
---

# T19: Pachinko domain: the first behavioural module

## Context

The bootstrap left `docs/specs/pachinko.allium` a placeholder: the repository had a full
gate, a handbook and an agent contract, and no domain. The maintainer brought a design
brief and a Python sketch for pachinko, a single-deployment queue of *pockets*, each
holding one *record* per key with a higher *rank* superseding in place, consumed under
exclusive, renewable *leases*. An `elicit` session on 2026-10-30 turned the brief into the
decisions below. This ticket writes them down as the first behavioural module, with its
vocabulary and the handbook ripple. Specifications win on *what* (decision 0002), so the
module lands before any code, and every product decision the maintainer declined to make
is an `open question`, never a guess.

The decisions the session confirmed. The module witnesses each by a rule, a guarantee or
an open question, and the hand-back notes say which:

| Topic | Decision |
| --- | --- |
| Supersede | A key keeps its oldest slot while unprocessed; a higher rank replaces payload and rank in place; the loser becomes a superseded record. A lower rank than stored is dropped into the superseded queue marked stale. Equal rank: open question. |
| Processing resets position | After an ack or nack of a held version that is no longer the latest, the newer version is *deferred*: it restarts at the back of the pocket, and the key is not pulled again in the same lease. The maintainer's words were broader, "any newer version restarts at the back of the pocket, and the key is not pulled again until the lease is released", shown only for a version arriving while the key was held; the approved read-back has the record gone on that ack or nack and a later publish of the key entering as a new record, so a key acked or nacked and then published afresh under the same lease is open question 9. |
| Pull outcomes | A batch, the pocket and zero or more records with the lease standing, or no pocket available, with no lease granted. Never an error, never a bare empty set. |
| Lease | Exactly one per consumer, one per pocket. Pull, ack, nack and renew (an explicit heartbeat) each extend `expires_at` by `lease_ttl`; release ends the lease, and a consumer must release before it can claim another pocket. The approved read-back's lease line listed release among the actions that move `expires_at`, carried over from the heartbeat question's premise; the module ends the lease instead, because `released` is terminal and nothing reads a released lease's `expires_at`, so the move would be unobservable; see Open points. |
| Expiry | Held and deferred records return to queued. The pocket stays at the front and its consecutive-expiry count rises; at `max_consecutive_expiries` it is *parked* until a publish revives it. A clean release resets the count to 0. An empty pocket at a lapse below the threshold goes idle, as on release; see Open points. |
| Release | Un-acked held records return to queued at their positions; the pocket re-enters at the back if it has records, else goes *idle*. |
| Revive | A publish into an idle or parked pocket re-queues it at the back with count 0. The maintainer confirmed the new-key case; the sketch's push revives on every push, so a supersede and a stale drop revive too, and the hand-back says so. |
| Rank | Publisher-supplied; pachinko only compares; per-key monotonicity is the publisher's job. |
| Capacity | Publish is never refused for capacity; under pressure the superseded and dead-letter queues shed before their TTL, live records never. Anything beyond that: open question. |
| Observers | Who reads superseded and dead-letter entries is an open question; the entities and their decay are in scope, with no surface yet. |
| Vocabulary | Pocket, Record, Rank, Supersede, Superseded record, Dead letter, Lease, Consumer, Publisher, Parked, Idle. Never holder, duel, message, primary queue or secondary queue. |
| Out of scope | Storage, snapshots, replication, pid lock, transport, autoscaling, disk spill, thread model, topics. |

The module's home is the root, `docs/specs/pachinko.allium`, as `docs/how-to/work-with-the-specs.md`
directs for behaviour that belongs in the root; this *is* the domain, and a split waits
until it grows.

## Goal

`docs/specs/pachinko.allium` states the domain above as entities, rules, invariants,
surfaces, config and open questions; `just check-specs` and `just analyse-specs` each
report one module with empty `diagnostics` and `findings`; `just plan-spec pachinko`
lists its test obligations; `docs/project/terminology.md` carries one row per domain
word; the handbook pages that said the domain was undefined say it is specified and not
implemented; `just check` is green.

## Non-goals

- No tests. `propagate` derives them in the implementation ticket, where they fail first.
- No implementation in either crate, and no effect trait. The clock arrives with the
  implementation (decisions 0003 and 0008).
- No runtime, transport or wire format. The `Publishing` and `Consuming` surfaces are the
  trigger decision 0014 names; the spike and the record that supersedes 0014 are
  follow-on work.
- No surface for reading the superseded and dead-letter queues: who reads them is an open
  question.
- No answer to any open question, the equal-rank publish included.
- No second module. The root is the domain's home until it outgrows its Scope.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | final | The module: the placeholder header replaced by Scope, Includes, Excludes, Dependencies and Vocabulary; seven entities; config; the rules for publish, supersede, revive, pull, ack, nack, renew, release, lease expiry and decay; the invariants; two surfaces; the open questions |
| `docs/project/terminology.md` | shared, own lines | The domain table above the repository table; the repository's `Consumer` row renamed `Caller`; the library row no longer says the domain is undefined |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | The intro, the module table row, the module section and the open-question section |
| `docs/explanation/specifications.md` | shared, own lines | "What exists today" and "Open questions are a feature" |
| `docs/explanation/layering.md` | shared, own lines | One paragraph: the module specifies the domain, and the link to the renamed how-to section |
| `docs/project/purpose-and-scope.md` | shared, own lines | "What it does", the first non-goal bullet, "Who it is for" |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added |
| `tickets/README.md` | shared, own lines | The index row, the bootstrap paragraph and one dependency-graph bullet |
| `tickets/T19-pachinko-domain.md` | ticket | `status:` and hand-back notes |

Pages that stay true until the implementation lands are not touched:
`docs/explanation/architecture.md` ("No trait exists yet") and decision 0014, which this
module's surfaces reopen as follow-on work.

## Steps

1. Create the worktree from `main` after T18 has merged (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. Run `just check` on the
   untouched worktree and quote its last line as the baseline.
2. **The module.** Use the `spec-change` skill, which hands the language to the `allium`
   skill. Replace the placeholder with the module, in the Vocabulary the maintainer chose.
   Run `just check-specs`, then `just analyse-specs`, and fix each diagnostic and finding
   at its root: a diagnostic may be waived only on the terms in
   `docs/how-to/work-with-the-specs.md`, and a finding cannot be waived. Run
   `just plan-spec pachinko`, quote its obligation count and strike none.
3. **The handbook ripple.** Use the `review-docs` skill. Terminology: one row per domain
   word, naming module `pachinko`, in a table above the repository's; the repository word
   for whatever calls the library can no longer be `Consumer`. The how-to, the
   explanation, the layering and the purpose pages: the domain is specified, not
   implemented. `CHANGELOG.md`: libpachinko Unreleased, Added. `tickets/README.md`: the
   index row and the paragraph that said the domain was undefined. Run `just check-docs`.
4. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against the decision
   table in Context, the Files touched list and the Vocabulary. Ask: whether every row of
   the table is witnessed by a rule, a guarantee or an open question, and whether any
   rule states a decision the table does not; whether a pull, ack, nack, renew, release or
   lapse can leave a record or a pocket in a state no rule leaves; whether a publish can
   be refused for anything but an equal rank; whether two rules on one trigger overlap in
   a way the analyser cannot see; and whether a page outside Files touched now says
   something false.
5. `just check`, then commit on the ticket branch, with the hand-back notes quoting each
   output headed by the commit it ran at.

## Acceptance criteria

- `just check-specs` and `just analyse-specs` each print one block, for
  `docs/specs/pachinko.allium`, with empty `diagnostics` and `findings`.
- `just plan-spec pachinko` exits 0 with a non-empty `obligations` array; the count is
  quoted.
- Every row of the decision table in Context is witnessed in the module by a rule, a
  guarantee or an open question, and the hand-back notes say which.
- `docs/project/terminology.md` has a domain table with a row for every word in the
  module's Vocabulary, and no word has two meanings on the page.
- `just check-docs` and `just check` are green.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check-specs
just analyse-specs
just plan-spec pachinko
just check-docs
just check
git status --short
```

## Hand-back notes

### Worktree and baseline: `a1435d6`

The worktree was a clean, initialized checkout of `main` at `a1435d6` whose branch
carried nothing, so the ticket branch was created there from `main` and no
`just initialize` ran; nothing reached the network. The pinned `allium` was 3.6.1.

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

### The module: `a1435d6` with uncommitted changes

The draft the elicitation produced met the pinned checker four times before the three
recipes went green, each fixed at the root with no waiver:

- `ByteArray` is not a type in Allium 3.6.1, although the reference's pattern 9 uses it.
  The payload is typed `String`, and the Vocabulary says why; the implementation stores
  bytes.
- A guard of the form `pocket.status in {idle, parked}` witnesses no transition edge: the
  checker reported `allium.status.noExit` for both states. One revive rule per source
  state replaced it.
- A status assigned inside a `for` loop over a black-box batch or over a projection is not
  observed as a transition: the checker reported `queued` and `deferred` without exits
  and `held` never assigned. The per-record transitions moved into chained rules:
  a pull emits `Handed` per record and `HoldRecord` moves it to held; release and expiry
  emit `Freed` and `RequeueHeldRecord` and `RequeueDeferredRecord` move it back.
- The analyser reports a `conflict` between a temporal rule and every other rule that
  leaves the same state to a different value, whatever their other guards, so the three
  expiry rules conflicted with each other and with both release rules. Probes in scratch
  space showed that a chained rule a temporal rule triggers is not treated so. The clock
  now emits `LeaseLapsed` from `LeaseLapses`, and three chained rules process it with
  disjoint guards; each also requires `expires_at <= now`, so a renewal processed first
  stops the lapse. Writing that exposed a race the elicitation did not cover, a consumer's
  action arriving after `expires_at` but before the lapse is processed, which is the
  module's eighth open question rather than a decision taken here.

The quotes below are from the rerun after both review rounds; the obligation count rose
from 172 at the first green run to 178 as the review rounds added invariants.

```text
just check-specs
allium check: 1 specifications, no diagnostics and no findings.
Exit status: 0

just analyse-specs
allium analyse: 1 specifications, no diagnostics and no findings.
Exit status: 0

just plan-spec pachinko
178 obligations, 0 diagnostics
Exit status: 0
```

The decision table, row by row:

- Supersede: rules `Supersede` and `DropStalePublish`; guarantees `OldestSlotNewestPayload`
  and `EveryDropIsVisible`; open question 1 for the equal rank.
- Processing resets position: `AckDefersNewerVersion`, `NackDefersNewerVersion` and
  `RequeueDeferredRecord`; guarantees `OneVisitPerKeyPerLease` and `NothingLostWhileHeld`;
  open question 9 for a key republished after its held version was acked or nacked and
  its record removed.
- Pull outcomes: `PullWithinLease`, `PullClaimsFrontPocket` and `PullFindsNoPocket`, which
  emit `Batch` or `NoPocketAvailable` and reject nothing; the pull has no `limit` guard,
  so a non-positive limit yields an empty batch rather than an error; invariant
  `PullBatchPositive` keeps `max_pull_batch` at 1 or more.
- Lease: invariants `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`,
  `LeasedHasOneActiveLease` and `UnleasedHasNoActiveLease`; every rule that continues a
  lease (`PullWithinLease`, the two ack rules, the two nack rules and `Renew`) sets
  `expires_at = now + config.lease_ttl`; the two release rules and the three lapse rules
  end it; `Renew` is the heartbeat.
- Expiry: `LeaseLapses` and the three `LeaseExpiry*` rules, with `RequeueHeldRecord` and
  `RequeueDeferredRecord`; guarantee `ExpiryRecoversWithinTtl`; invariants
  `ExpiriesWithinLimit`, `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
  `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative`; the last five are over
  config, which the pinned checker accepts, and the implementation reads them as checks at
  config load; the queued path leaves `enqueued_at` alone, so the pocket keeps the front;
  `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket` reset the count.
  `LeaseExpiryIdlesEmptyPocket` is the agent's reading for a pocket with no records,
  listed in Open points.
- Release: `ReleaseRequeuesPocket` and `ReleaseIdlesEmptyPocket`.
- Revive: `PublishNewKey`, `Supersede` and `DropStalePublish` all emit `PocketTouched`;
  `ReviveIdlePocket` and `ReviveParkedPocket` answer it. The supersede and stale-drop cases
  follow the sketch's push, as the table says.
- Rank: `Supersede` and `DropStalePublish` only compare; the Vocabulary; open question 1.
- Capacity: guarantee `NeverRefusedForCapacity`; open question 3.
- Observers: entities `SupersededRecord` and `DeadLetter`, rules `SupersededRecordDecays`
  and `DeadLetterDecays`, the Excludes line, open question 2.
- Vocabulary: the module's Vocabulary section and the domain table in
  `docs/project/terminology.md`.
- Out of scope: the Excludes.

### The handbook: `a1435d6` with uncommitted changes

`docs/explanation/layering.md` was not in the plan's list: it linked to the how-to
heading this ticket renamed and said the module declares no behaviour, so one paragraph
changed and the page joined Files touched.

```text
just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 44 pages and 45 canonical topics.
Exit status: 0
```

### Adversarial review, checkpoint 1: `a1435d6` with uncommitted changes

Harness and model: Codex (`codex-cli 0.165.0`) with `gpt-6-astra`, the other harness, for
both reviewers, read-only (`-s read-only`), each with a fresh context and one lens, run in
parallel on the diff of the uncommitted change against `a1435d6` and a copy of this ticket
with the hand-back notes withheld. Triage and the apply ran as fresh-context Claude
subagents in the executing agent's harness, one per lens for triage, one for the apply,
neither a reviewer nor the executing agent. The raw briefs, findings, verdicts, apply plan
and rerun logs are under `ai_tmp/review/T19-1/`, not committed.

Before the review snapshot was taken, the executing agent's own read of the module made
three edits the reviewers did not see: the `IdleHasNoRecords` invariant, a comment above
the config block, and the terminology Lease row no longer banning "claim". Triage checked
each finding against the current file.

Findings and verdicts (conformance lens, seven; breakage lens, five; two pairs overlap):

- CONFORMANCE-1 and BREAKAGE-5 (medium): the release rules do not extend `expires_at`,
  against the table's Lease row. Valid against the table; the table was the imprecise party.
  "Release renews the lease" was the agent's question premise, never the maintainer's words,
  and a released lease is terminal, so the extension would be unobservable. Applied: the
  table row, the hand-back's Lease line and the terminology Lease row now say release ends
  the lease; the module is unchanged.
- CONFORMANCE-2 (reviewer high, triage medium): `LeaseExpiryIdlesEmptyPocket` idles an
  empty pocket on a lapse, a case the table does not decide. Valid as a reading the
  maintainer was not shown; the rule follows the idle definition and the release row, and
  keeping an empty pocket at the front would be an equally undecided choice. Applied: a
  comment above the rule and an Open point; the rule stays.
- CONFORMANCE-3 and the parking half of BREAKAGE-2 (reviewer high and medium, triage
  medium): `ExpiryRecoversWithinTtl` said "renewing" while `Renew` is one of five extending
  actions, and promised pullable records on a lapse that parks the pocket. Valid. Applied:
  the guarantee reworded. The late-renewal half of BREAKAGE-2 was invalid: a late renew is
  itself a renewal, and the race is open question 8.
- CONFORMANCE-4 and BREAKAGE-1 (medium and high): `OneVisitPerKeyPerLease` is violated when
  a key whose record was removed by an ack or nack is published again and pulled under the
  same lease. Valid: the scenario holds rule by rule. The two triage reviews disagreed on
  the fix. The breakage triage read the maintainer's words as covering the case and drafted
  a `Visit` entity, checker-clean; the conformance triage found the case undecided. The
  deciding evidence, the read-back the maintainer approved before the plan, whose Record
  lifecycle has the record gone on that ack and a later publish entering as a new queued
  record, was not in the breakage triage's input when it started. Applied: the guarantee
  narrowed to the approved model, open question 9 added, the table row and both handbook
  counts updated, the dissent and the `Visit` design recorded in Open points. The executing
  agent's own `settled` draft for the same case was withdrawn on the same evidence.
- CONFORMANCE-5 (medium): "oldest first" in `NeverRefusedForCapacity` was the agent's
  addition. Valid. Applied: dropped, and the shedding order folded into open question 3.
- CONFORMANCE-6 (reviewer medium, triage low): the config defaults carry no provenance in
  the module. Valid in part: the parking preview and the scope options the maintainer
  selected carry 3 and 12 hours, so only `lease_ttl` and `max_pull_batch` have no maintainer
  touch. Applied: a provenance comment above the config block and a sharper Open point.
- CONFORMANCE-7 (medium): ten places outside Files touched still say the domain is
  undefined. Valid; handed back under the scope guard, listed in Open points.
- BREAKAGE-3 (medium): `max_consecutive_expiries = 0` lets `LeaseExpiryParksPocket` break
  `ExpiriesWithinLimit`, and a comment settles nothing machine-readable. Valid. Applied: the
  top-level invariant `ExpiryLimitPositive`, which the pinned checker accepts; the
  implementation reads it as a check at config load.
- BREAKAGE-4 (reviewer medium, triage low): `OldestFirst` said "never more than limit" while
  a non-positive limit yields an empty batch. Valid. Applied: the guarantee reworded and the
  reading added to Open points.

BREAKAGE-1 was valid at high, so a second round ran over the fix diff (below).

### Adversarial review, checkpoint 1, second round: `a1435d6` with uncommitted changes

The same two Codex reviewers, fresh contexts, over the fix diff from the first snapshot to
the applied state, with the approved read-back in their brief and one added question: does
any rule, guarantee or Vocabulary line contradict the read-back's lifecycle diagrams and
trace. Triage ran as the fresh-context Claude subagent that had triaged the breakage lens in
round one and had applied nothing; it also applied this round's fixes. Raw files under
`ai_tmp/review/T19-2/`.

Findings and verdicts, five, all medium, none handed back:

- conformance-2-1: the approved read-back's one-line lease summary lists release among the
  actions that move `expires_at`, and the first round dropped release from that list
  without recording the departure. Valid as a record gap, invalid as a rule change: every
  reader of `expires_at` is guarded to active leases and `released` is terminal, so the
  extension is unobservable, and a probe adding it left the plan unchanged. Applied: the
  departure recorded in the Lease row and in Open points; the module unchanged.
- conformance-2-2 and breakage-2-1: the reworded `ExpiryRecoversWithinTtl` promised a
  pullable pocket after any lapse that does not park it, while an empty pocket goes idle.
  Valid. Applied: the guarantee reworded around the records, so it holds under the
  current rule and under the Open point's alternative.
- breakage-2-2 and breakage-2-3: a negative `max_pull_batch` makes `OldestFirst`
  unsatisfiable and a negative TTL breaks `GrantedBeforeExpiry`, `ShedBeforeExpiry` or
  `NackedBeforeExpiry`. Valid. Applied: `PullBatchPositive` and three TTL invariants on
  config, each the model's domain of definition and not a figure; a positive TTL floor is
  a product decision and sits in the config Open point. Triage also found that the pinned
  checker does not type-check config-valued invariant expressions at all, which the
  checker-limits Open point now records.

No finding was high, so the two-round cap closes the checkpoint. The walk of the read-back's
trace through the rules found no step where the module and the read-back part other than
the unobservable lease extension above.

### Final gate: `a1435d6` with uncommitted changes

Run over the complete change, this section included, immediately before the commit; the
commit is the only change after it. `git status --short` after the commit is in the
hand-over.

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

### Landing: `633f05e` with uncommitted changes

On 2026-10-31 the user invoked `/show-me` followed by `/visual-pr` and asked for the pull
request to be opened with the diagrams in its description; that request is the
authorisation for `git fetch origin`, the push and the pull request. `git fetch origin`
found `origin/main` still at `a1435d6`, so no rebase or merge was needed and every quote
above stands. Pushed `ticket/t19-pachinko-domain` at `633f05e` and opened pull request 34,
`https://github.com/BiscuitBin/pachinko/pull/34`, whose description carries the pocket and
record lifecycles, the approved trace, the lapse chain and the file tree. The description
is saved at `ai_tmp/pr/pr-34/description.md`, under `ai_tmp/` rather than `.humanlayer/`
because nothing ignores the latter and an untracked file would dirty the worktree.

CI for `633f05e`, in progress when this was written: run
`https://github.com/BiscuitBin/pachinko/actions/runs/41185354096` (CI) and
`https://github.com/BiscuitBin/pachinko/actions/runs/41185353977` (Audit). Their
conclusions are read on the pull request; a pull request's tested commit is the merge
commit, not the branch head.

`status:` is set to `done` in this commit, in the same pull request, as the definition of
done requires. `just check` ran over this change, this section included, immediately
before the commit:

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

## Open points

- The module's nine open questions are the maintainer's to answer; seven come from the
  elicitation and the last two are gaps that writing and reviewing the rules exposed:
  equal-rank publishes; who reads the superseded and dead-letter queues and what an entry
  must carry; capacity beyond shedding the decaying queues; routing by worker pool or
  priority; an adaptive lease TTL; a new pocket jumping the queue; reviving a parked pocket
  other than by a publish; a consumer's action that arrives after its lease's `expires_at`
  but before the lapse is processed; and a key published again under the lease that
  removed its record. The maintainer asked for a `research` task on the rank tie, covering
  the literature, comparable queues and databases, user expectation and configurability,
  and a survey of comparable open-source queues.
- Config defaults: `max_pull_batch = 100` is the agent's placeholder; `lease_ttl =
  5.minutes`, `max_consecutive_expiries = 3`, `superseded_ttl = 12.hours` and
  `dead_letter_ttl = 12.hours` are the sketch's. The parking preview the maintainer
  selected parks on the third consecutive lapse, and the options selected named 12 hours
  as the sketch's decay default, so `lease_ttl` and `max_pull_batch` are the two with no
  maintainer touch in the record. The maintainer confirms or changes them. Four top-level
  invariants bound the config to the model's domain of definition, as `ExpiryLimitPositive`
  does: `max_pull_batch >= 1`, below which no record can ever be handed out, and the three
  TTLs `>= 0.seconds`, below which `GrantedBeforeExpiry`, `ShedBeforeExpiry` or
  `NackedBeforeExpiry` fails on the first firing. A zero TTL is admitted, with each
  `expires_at` equal to the stamp it is measured from and the three invariants holding with
  equality; whether any TTL needs a positive floor is a decision the maintainer has not
  taken.
- Readings the elicitation did not cover, each a one-line confirmation for the maintainer:
  - Deferral position: the module stamps the deferred record's `queued_at` when the held
    version is acked or nacked, following the maintainer's words that the newer version
    "restarts at the back of the pocket" and the approved read-back ("re-queued at the
    back" at ack). A key published into the pocket between that ack and the release
    therefore queues behind the deferred record; the alternative stamps it when the lease
    ends.
  - Release and `expires_at`: no elicitation question asked it, but the approved read-back
    says every pull, ack, nack, renew and release moves `expires_at` forward by the TTL.
    The two release rules end the lease and leave `expires_at` alone: `released` is
    terminal, `LeaseLapses`, the three lapse rules and the Consuming surface read only
    active leases, and `GrantedBeforeExpiry` holds either way, so the move has no
    observable effect. The alternative writes the dead `ensures` on both release rules for
    literal conformance.
  - Empty pocket at lapse: the expiry decision was shown for a pocket with records.
    `LeaseExpiryIdlesEmptyPocket` idles a leased pocket whose records were all acked or
    nacked when its lease lapses below the parking threshold, following the release rule
    and `IdleHasNoRecords`; at the threshold `LeaseExpiryParksPocket` parks it, since
    parking has no records guard. The alternative keeps an empty pocket queued at the
    front, which the next consumer claims for an empty batch and a lease it must then
    release.
  - A pull with a limit that is not positive yields an empty batch rather than an error,
    the agent's reading of "never an error"; the maintainer never discussed such a limit.
  - Open question 9, a key published again under the lease that removed its record: the
    two triage reviews of this ticket disagreed. One read the maintainer's words ("any
    newer version ... the key is not pulled again until the lease is released", "One visit
    per key per lease") as covering it and drafted a `Visit` entity, created when a record
    is handed out and removed when the lease ends, with a publish rule that creates such a
    key's new record as deferred; it passes the pinned checker and analyser. The other
    read the approved read-back, whose record lifecycle has the record gone on ack or nack
    and a later publish entering as a new queued record, as the model the maintainer
    approved. The module keeps the approved model and asks; if the maintainer chooses one
    visit per key, the `Visit` design is the ready answer.
- `SupersededRecord.reason` and `DeadLetter.nacked_by` are kept on the assumption in open
  question 2; nothing reads them yet, and the checker did not object.
- The word "consumer" in the generic sense of a library's caller remains in
  `docs/explanation/architecture.md`, `docs/reference/api.md`, `docs/reference/testing.md`,
  `docs/reference/configuration.md`, `docs/explanation/security-model.md`, `SECURITY.md`
  and several decision records, while `docs/project/terminology.md` now reserves it for
  the domain party and says "caller" for the other sense. Rewording those pages is handed
  back; decision records are not edited.
- Four limits of the pinned checker and analyser learned here belong in the
  "Diagnostics and waivers" section of `docs/how-to/work-with-the-specs.md`, which is
  carried verbatim from libpawdoku and which this ticket does not edit: a status assigned
  inside a `for` loop is not observed as a transition; a guard `status in {a, b}` witnesses
  no edge; a temporal rule that itself leaves a state conflicts with every other rule that
  leaves it; and a config-valued invariant is accepted structurally with its expression
  not type-checked, nothing being reported for a Duration against a bare integer, an
  Integer against a Duration literal or an Integer against a string. Handed back.
- Ten places outside Files touched still say the domain is undefined or the module declares
  nothing, and are now false: `README.md:10-11`, `AGENTS.md:5` and `:13`,
  `docs/README.md:20-22`, `docs/project/repository-map.md:89`,
  `docs/tutorials/first-change.md:60-62`, `docs/reference/configuration.md:155-156`,
  `docs/reference/testing.md:83-84`, `crates/libpachinko/src/lib.rs:4` and
  `crates/pachinko/README.md:8`. Handed back: `AGENTS.md` takes `just check-agents`,
  `lib.rs` is crate code, the rest is handbook ripple under `review-docs`.
- Follow-on work, not in this ticket: `propagate`, to derive the tests, which fail until
  implemented; `rust-change`, to implement in `libpachinko` with the clock as the first
  effect trait, its fake and the service implementation (decisions 0003 and 0008);
  decision 0014's spike, which the two surfaces trigger, and the record that supersedes
  0014; and the `research` task above.
