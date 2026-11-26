---
id: T43
title: "Decide the caller contract: what each action answers, the lease's id, and the named refusals and their order"
status: todo
depends_on: []
parallel_with: []
branch: ticket/t43-caller-contract
estimated_size: L
---

# T43: Decide the caller contract: what each action answers, the lease's id, and the named refusals and their order

## Context

`docs/specs/pachinko.allium` says what each action does to the state and little about
what a caller gets back. A pull answers `Batch` or `NoPocketAvailable`; every other
trigger answers nothing. A `Lease` has no identity, though every ack, nack, renew and
release names its lease. A trigger no rule accepts is "rejected", and no rejection has a
name or a place in an order. The two implementations each filled those gaps their own
way: the Rust library and service (`crates/`) and the Python double (`double/`).

`double/SPEC-NOTES.md` records the gaps as findings 2, 3 and 5, resting on notes N05,
N11, N13, N15, N20, N21, N23 (its identity half), N24 and N32, and as what finding 1
left. The maintainer ruled on them on 2026-11-24. Some rulings are complete; others
leave points open. This ticket works out the whole caller contract: it drafts the
specification text, writes a decision record, stops for the maintainer's ruling on each
open point, and then creates the tickets that update the specification, the Rust, the
service, the Python double and the scenario catalog.

**This ticket changes no behaviour.** It does not edit
`docs/specs/pachinko.allium`, `crates/` or `double/` in any commit. Its deliverables
are a draft, a decision record, the maintainer's answers and follow-up ticket files.

Nothing in this ticket re-opens a ruling. A finding that a ruling is wrong goes to the
maintainer as a question, never into a draft that assumes otherwise.

### The rulings (all 2026-11-24, decided)

1. **Actions under a lease answer its `expires_at`** (finding 2; N13, N24). Every
   action that works under a lease and extends it answers the lease's `expires_at`:
   pull, renew, ack and nack. The maintainer's reason for ack and nack: they operate
   under the lease and implicitly renew it.
2. **A lease has an id in the specification** (finding 3; N05, N15, N23). The id is
   unique for the deployment's life, never reused, and exposed in the `Batch` and in
   the `Consuming` surface's lease view. **The id is a UUIDv7.**
3. **An ended lease may be forgotten** (what finding 1 left; N24). An implementation
   may forget an ended lease, since an id that matches no active lease is refused
   either way. This completes the ruling of 2026-11-20 that an ended lease is an
   implementation detail: the specification's letter still keeps every ended lease, and
   it is to say that one may be forgotten.
4. **The specification names its refusals and fixes the order they are checked**
   (finding 5; N11, N20, N21).

Three rulings of the same day belong to other tickets and bound this one:

- **Joining is dropped**, for publishers and for consumers
  (`tickets/T45-drop-joining.md`). A caller-supplied consumer id stays on pull, renew,
  ack, nack and release, accepted on first use. So `UnknownPublisher`,
  `UnknownConsumer` and the two already-joined refusals leave the contract; this ticket
  lists them as today's and leaves them out of the intended contract.
- **No claim without records** (`tickets/T46-no-claim-without-records.md`). A pull with
  a limit below 1 by a consumer with no lease claims no pocket and hands nothing;
  inside a held lease it still extends the lease with an empty batch. What that pull
  answers is decided here (open point 7) and landed by T46.
- **Starting a deployment is a rule** whose `requires` are the five config bounds
  (`tickets/T47-start-rule-and-config-bounds.md`). The name of its refusal is decided
  here (open point 8) and landed by T47.

What a tick reports (N33) is not this ticket's: `tickets/T42-clock-model.md` owns it.

### The open points

Each is an "Authorisation required" stop at step 5. The executor puts it to the
maintainer with the examples and options below and assumes no answer. The options are
suggestions for the question, not rulings.

1. **What a publish answers.** The specification: nothing. The double: nothing, by the
   maintainer's ruling of 2026-11-19, which stands until this point is ruled (N32). The
   Rust: one of four `PublishOutcome` values, one per rule or pair of rules, and the
   service sends them as `created`, `new_key`, `superseded`, `stale`. Examples to put,
   with pocket `pk` and key `k`:

   | The state before | The publish | The Rust answers | Rule |
   | --- | --- | --- | --- |
   | no pocket `pk` | `k` at rank 1 | `Created` | `PublishIntoNewPocket` |
   | `pk` exists, no `k` | `k` at rank 1 | `NewKey` | `PublishNewKey` |
   | `k` queued at rank 3 | rank 4 | `Superseded`; rank 3 is shed at once | `Supersede` |
   | `k` held at rank 3, nothing deferred | rank 4 | `Superseded`; nothing is shed | `DeferBehindHeldVersion` |
   | `k` stored at rank 3 | rank 1 | `Stale` | `DropStalePublish` |
   | `k` stored at rank 3 | rank 3 | refused, `EqualRank` | none |

   An ordinary supersede and a deferral share one answer: decision 0020's publish table
   records that "the maintainer chose `superseded` for it at step 1" (of T35), and a
   publisher cannot tell the two apart. Which outcome a publish that revives an idle or
   a parked pocket answers was not verified when this ticket was written; step 2 reads
   it in `crates/libpachinko/src/queue/publish.rs` and the store. Options: nothing, as
   the specification has it; the Rust's four; a deferral told apart from a supersede.
2. **What a release answers.** The specification and the double: nothing. The Rust:
   `(PocketFate, Lease)`, the pocket's fate (`Queued` under `ReleaseRequeuesPocket`,
   `Idle` under `ReleaseIdlesEmptyPocket`) and the lease as it ended; the service sends
   `{"pocket": "queued"}` or `{"pocket": "idle"}`. Examples: a lease holding `a`, acked,
   and `b`, unsettled, is released, `b` outlasts it, and the answer is `Queued`; with
   both acked and nothing deferred the pocket empties and the answer is `Idle`. Under
   ruling 3 the release's answer is the last a caller can be sure to see of the ended
   lease. Options: nothing; the pocket's fate; the fate and `ended_at`.
3. **Whether an ack or a nack tells a first mark from a repeat.** Ruling 1 gives both
   the lease's `expires_at`. The Rust's `SettleOutcome` is `Acked` or `Nacked`, "for
   the first mark and a repeat alike"; the service sends `{"outcome": "acked"}` and
   carries no `expires_at`. Example: `ack(k)` twice under one lease runs
   `AckMarksRecord`, then `AckRepeats`; both succeed, and only the first resets the
   pocket's `consecutive_expiries`. Options: `expires_at` alone; `expires_at` and the
   mark; `expires_at` and whether this was the first mark. The question for the
   maintainer is whether a consumer that retries after a lost answer needs to know.
4. **One lease refusal or two.** The double refuses with one, `LeaseFenced`, for a
   lease id that is absent, ended or another consumer's, "because the specification
   draws no line between the two `requires`" (N21). The Rust has two:
   `LeaseNotActive`, for a lease that has ended and one that never existed alike, and
   `LeaseNotYours`, for an active lease of another consumer; the service sends 409
   `lease_not_active` and 403 `lease_not_yours`. Ruling 3 bears on this: once an ended
   lease may be forgotten, no implementation can be required to tell an ended lease
   from an id never issued. Whether it also rules out telling "ended" from "another
   consumer's" is the executor's to work out and show: the second is asked only of an
   active lease, which is never forgotten. Example for the question: after `c1`
   releases lease `L`, `c2` renews `L`; before the release the Rust answers
   `LeaseNotYours`, after it `LeaseNotActive`, and the double answers `LeaseFenced`
   both times. A second refusal also tells a caller that an id it does not own is an
   active lease; say so.
5. **The check order.** Today, for an ack or a nack:
   - the double: the lease (`LeaseFenced`), then the record (`RecordNotHeld`), then the
     mark (`MarkConflict`); the unknown-consumer check runs before the outcome starts
     (N18, N22);
   - the Rust: `UnknownConsumer`, then `LeaseNotActive`, then `LeaseNotYours`, then
     `RecordNotHeld`, then `MarkConflict`;
   - the service, before any of those: the path's lease id parses (400 `bad_request`).

   Scenario S070 pins lease-before-record: an ack by a third consumer of a key the
   lease's pocket does not hold is a lease refusal, not "not held". Once joining is
   dropped the consumer check goes. The draft states an order for every trigger that
   can be refused, publish, pull and the start rule included, and the maintainer
   confirms it.
6. **How Allium states a named refusal.**
   `allium-skill-reference/allium/language-reference.md` says of `requires`: "If not
   met, the trigger is rejected", and, for an external stimulus trigger, "The action is
   rejected; caller receives an error". A search of that file when this ticket was
   written found no construct that names a rejection or orders the `requires` of rules
   that share a trigger. That is a search, not a reading: unverified until step 3 reads
   the reference in full. Candidates to weigh there, none of them chosen: a rule per
   refusal with a complementary guard that `ensures` a named outcome, as
   `PullFindsNoPocket` answers `NoPocketAvailable`; a `@guarantee` on the surface that
   names the refusals and their order in prose; a comment on each `requires`. A rule
   per refusal makes a refusal a rule that fires, which changes what
   `just plan-spec pachinko` obliges and must still let a refusal change nothing and
   read no clock (N28, scenario S085).
7. **What a pull with `limit <= 0` and no lease answers.** Today it claims a pocket and
   answers an empty batch (S031, S032; N19). Under the no-claim ruling it claims
   nothing. Its answer is open: `NoPocketAvailable`, which would be false when a pocket
   is queued; a new outcome; or a named refusal. `tickets/T46-no-claim-without-records.md`
   waits on this answer and its name. Note for the question:
   `docs/reference/http-api.md` tells a client that lost a batch to pull with
   `{"limit": 0}` to get its lease id back; that pull is inside a lease, which the
   ruling leaves as it is.
   The answer must also cover the neighbouring case, no lease, no pocket queued and
   `limit <= 0`: `PullFindsNoPocket` answers `NoPocketAvailable` there today, so if
   the first case becomes a refusal, the same call is refused or not depending on
   whether a pocket is queued, unless that rule changes too.
8. **The name of the start rule's refusal of a config outside its bounds.** Today the
   double raises `ConfigRefused`, naming every config invariant the config breaks, in
   the specification's order. The Rust's `ConfigError` has `MaxConsecutiveExpiriesZero`
   and `MaxPullBatchZero`, checked in that order and reporting the first; a negative
   TTL cannot be built at all (scenario S005 is a `compile_fail` doctest). To decide:
   one name or one per bound, and whether the refusal reports every broken bound or the
   first. `tickets/T47-start-rule-and-config-bounds.md` waits on this name.
9. **Where the UUIDv7's timestamp and random bits come from.** A UUIDv7 carries a
   millisecond timestamp and random bits (RFC 9562; the layout is stated here from
   memory and is unverified until step 3 reads the RFC). So an id draws on two effects,
   the clock and randomness. The points to put:
   - **The seam.** The library's `LeaseIds` trait is `fn next(&self) ->
     Result<LeaseId, LeaseIdError>`: it is given no time. The library has a `Clock`
     trait and no randomness trait. `AGENTS.md` names randomness among the effects a
     library reaches only through its own traits, and says a new kind of effect needs a
     decision record first. Either the id stays behind `LeaseIds`, and the source has to
     get the queue's `now` some way, or the library builds the id from `Clock` and a
     new randomness trait.
   - **The timestamp.** It should come from the specified `now`, the time the rule
     that grants the lease reads, not a second clock read. `tickets/T42-clock-model.md`
     decides what `now` is (it never decreases and may repeat); this ticket notes T42
     and does not wait on it. The draft says what a UUIDv7 needs from `now`, and what
     happens when two leases are granted at one `now`.
   - **"Never reused".** With random bits, uniqueness is a matter of probability unless
     something checks or counts. Today the Rust refuses a claim whose id is still
     active (`LeaseIdError::StillActive`) and a source with no id left
     (`LeaseIdError::Exhausted`); the draft says what becomes of both.
   - **Restart.** "Never reused" must survive a restart once a write-ahead log exists.
     No log exists today; the draft states what a UUIDv7 does and does not guarantee
     across a restart, so that the ticket that adds the log inherits a stated
     requirement.
   - **The text form.** Ruled on 2026-11-25, and no longer put at step 5: see
     "Maintainer call (2026-11-25)" below. Today a lease id is 128 bits sent as 32
     lower-case hexadecimal digits, and the lease routes refuse any other form with 400.
     The wire will carry the 36-character hyphenated form, lowercase out and any case
     in, under the "valid UUID" rule of that call, and the 32-digit form becomes
     refused.
   - **In the specification.** How the module types the id, where it says "UUIDv7",
     and how it states "never reused" when an ended lease may be forgotten, so that an
     invariant over the leases that exist cannot say it.
10. **Whether holding a lease id is ever authority.** Added by the maintainer call of
    2026-11-25 below, and not ruled. RFC 9562 §8 says UUIDs "MUST NOT be used as
    security capabilities" (unverified until step 3 reads §8). A lease id is a token
    its holder presents back; today an action names its lease and its consumer, and the
    lease refusals check that the lease is the consumer's (point 4). If possession of a
    lease id ever stands as proof of holding the lease, a UUIDv7's timestamp bits make
    it easier to guess than a version 4's random bits. Put it beside point 4, where a
    second refusal already tells a caller that an id it does not own is active, and
    beside this ticket's non-goal of authenticating a caller. The draft decides nothing
    here; it states what the specification and the service rely on a lease id for.

### Maintainer call (2026-11-25), recorded against this ticket as written

On 2026-11-25 the maintainer ruled the format of the publisher, consumer and lease ids,
answering item 9 of "What this double raises that no `N`-note holds" in
`gleam-double/SPEC-NOTES.md`. This ticket was written before that ruling. The call rules
one sub-bullet of open point 9 and adds open point 10; the rest of the ticket stands as
written. The consumer id, and the publisher id if T45 keeps one, are
`tickets/T49-consumer-id-uuid.md`'s.

| Call | Decision | Effect |
| --- | --- | --- |
| The lease id's text form (open point 9, "The text form") | A valid UUID: the RFC 9562 variant, bits `10`, so the first hex digit of octet 8 is `8`, `9`, `a` or `b`, with any version nibble, which refuses the nil and max UUIDs and leaves versions 9 to 15 open; exactly 36 characters in the 8-4-4-4-12 form, hex digits in any case on input; braces, `urn:uuid:`, the 32-digit form, whitespace and stray hyphens refused; parsed to the 16-byte value, emitted as lowercase canonical text, never compared as the text sent | The sub-bullet is ruled and is not put at step 5. Today's 32-digit form becomes refused: a breaking wire change for every client that holds or sends a lease id, which step 6's service follow-up states in `docs/reference/http-api.md` and both Unreleased sections of `CHANGELOG.md`. |
| A lease id a caller presents | No version check. The service always mints a UUIDv7, and an id it never minted is simply not found | The 36-character and variant checks still apply to a presented id; only the version check is waived. The status, code and place of the refusal of a malformed lease id stay with open point 5. |
| RFC 9562 §8, a UUID is not a capability | Not ruled | Open point 10, put at step 5. |

The RFC facts in this call are as RFC 9562 §4
(`https://www.rfc-editor.org/rfc/rfc9562.html#section-4`) and §8
(`https://www.rfc-editor.org/rfc/rfc9562.html#section-8`) were read on 2026-11-25, and
are unverified here until step 3 re-reads both. Lowercase on output is the maintainer's
ruling, not the RFC's: RFC 9562 says only that any case is valid.

### What exists today

Re-read each at step 2; these were true when the ticket was written.

What each trigger answers:

| Trigger | The specification | The double | The Rust library | The service |
| --- | --- | --- | --- | --- |
| publish | nothing; the surface names no outcome | nothing | `PublishOutcome`: `Created`, `NewKey`, `Superseded`, `Stale` | `{"outcome": ...}` with `created`, `new_key`, `superseded` or `stale` |
| pull | `Batch(consumer, lease, pocket, records)` or `NoPocketAvailable(consumer)` | `Batch` (consumer id, lease id, pocket key, records) or `NoPocketAvailable`; no expiry | `PullOutcome::Batch { lease, pocket_key, expires_at, records }` or `NoPocketAvailable` | a batch with `lease_id` and `expires_at`, or `{"outcome": "no_pocket"}` |
| renew | nothing; it ensures the new `expires_at` | nothing | the lease's `expires_at` | `{"expires_at": ...}` |
| ack, nack | nothing | nothing | `SettleOutcome`: `Acked` or `Nacked` | `{"outcome": "acked"}` or `"nacked"` |
| release | nothing | nothing | `(PocketFate, Lease)` | `{"pocket": "queued"}` or `"idle"` |
| the lease view | pocket key, `expires_at`, each held record's key, `held_rank`, `held_payload`, `mark`; no id | the same; no id | `LeaseView { pocket_key, expires_at, records }`; no id | the same three fields; no `lease_id` |

Today's refusals, by implementation. This is the inventory as built, apart from the
intended contract:

| Trigger | The double (`double/pachinko_double/refusals.py`) | The Rust (`crates/libpachinko/src/error.rs`) | The service (`docs/reference/http-api.md`) |
| --- | --- | --- | --- |
| making a queue | `ConfigRefused` | `ConfigError::MaxConsecutiveExpiriesZero`, `MaxPullBatchZero` | `ServeError::Config`; the service does not start |
| a join | `PublisherAlreadyJoined`, `ConsumerAlreadyJoined` | `JoinError::PublisherExists`, `ConsumerExists` | 409 `publisher_exists`, `consumer_exists` |
| publish | `UnknownPublisher`, `EqualRankRefused` | `PublishError::UnknownPublisher`, `EqualRank` | 404 `unknown_publisher`, 409 `equal_rank` |
| pull, lease view | `UnknownConsumer`; `PullRefused`, reached only from a hand-built state (N14) | `PullError::UnknownConsumer`; for a pull that claims, `PullError::LeaseId` over `LeaseIdError::Exhausted` or `StillActive` | 404 `unknown_consumer`; on a lease-id failure the queue's owner stops with `ServeError::LeaseIdsExhausted` |
| renew, release | `UnknownConsumer`, `LeaseFenced`; for release, `ReleaseRefused`, reached only from a hand-built state (N26) | `LeaseActionError::UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours` | 404 `unknown_consumer`, 409 `lease_not_active`, 403 `lease_not_yours` |
| ack, nack | `UnknownConsumer`, `LeaseFenced`, `RecordNotHeld`, `MarkConflict` | as renew, then `RecordNotHeld`, `MarkConflict` | as renew, then 404 `record_not_held`, 409 `mark_conflict` |

The service also refuses a request for its shape (`bad_request`, `too_many_rows`,
`payload_too_large`, `body_too_large`, `request_timeout`, `not_ready`). Those are the
transport's and the specification names none of them; the draft says where that line
is.

The intended contract, once T45 has dropped joining, has no `UnknownPublisher`, no
`UnknownConsumer` and no already-joined refusal. What is left to name: the equal-rank
publish; the lease refusal or refusals; the record not held; the mark conflict; the
start rule's refusal (open point 8); the answer of open point 7 if it is a refusal; and
whatever the lease-id source may still refuse (open point 9). `PullRefused` and
`ReleaseRefused` stay the double's: the maintainer ruled on 2026-11-24 that the
`requires` they stand behind are restatements that cannot fail in a valid state.

Other facts:

- `entity Lease` has `pocket`, `consumer`, `status`, `granted_at`, `expires_at` and
  `ended_at`, and no id. `PullClaimsFrontPocket` names the lease in its `Batch` by a
  lookup, `Lease{pocket: pocket, consumer: consumer, status: active}`.
- The `Consuming` surface's `ActionsNameTheirLease` guarantee says an action "naming an
  ended lease is rejected rather than landing on a successor lease over the same
  pocket". The Publish comment and the Ack and nack comment each say "rejected" and
  name nothing.
- The Rust's shared guard, `authorise` in `crates/libpachinko/src/queue/renew.rs`,
  checks that the consumer has joined, then calls `check` in
  `crates/libpachinko/src/queue/leases.rs`: active, then the consumer's. `mark_held` in
  `crates/libpachinko/src/queue/settle.rs` then checks the record, then the mark, all
  before it reads the clock. The test `an_unknown_consumer_is_refused_before_any_lease_guard`
  in `crates/libpachinko/tests/queue/lease.rs` pins the first of those.
- The double's order is `_mark_refusal` in `double/pachinko_double/settle.py`.
- `LeaseId` in `crates/libpachinko/src/ids.rs` wraps a `u128` and prints as 32
  hexadecimal digits. The service's source, `EpochSerialLeaseIds` in
  `crates/pachinko/src/lease_ids.rs`, puts an epoch drawn once from the operating
  system's random source in the high 64 bits and a serial from 1 in the low 64.
  The double's `LeaseIds` protocol in `double/pachinko_double/effects.py` is
  `next_id() -> str`, and its `SequentialLeaseIds` gives `lease-1`, `lease-2`.
- Decision 0019 (`docs/decisions/0019-service-runtime-tokio-and-http.md`) has a "Lease
  ids" paragraph that fixes the epoch-and-serial source and the 32-digit form. The
  UUIDv7 ruling supersedes that paragraph. T45 (the join routes) and T42 (the lapse
  intent) each revise the same decision.
- Decision 0020 (`docs/decisions/0020-held-set-until-lease-ends.md`) holds the publish
  table quoted under open point 1 and names `MarkConflict`.
- `docs/decisions/README.md` says "The next decision is 0021". T40 and T42 each write a
  decision record too, so take the next free number when step 4 runs.
- T40 to T47 are the last ticket files; the follow-ups take the next free ids.
- The Python double, `double/`, and `double/SPEC-NOTES.md` are on the branch
  `build-python-double`. They were not on `main` when this ticket was written.

### Risks (`CONVENTIONS.md` §13)

- **A draft that lands.** The specification is the source of truth (`AGENTS.md`,
  invariant 1). A draft committed into `docs/specs/pachinko.allium` before its code
  makes the module and both implementations disagree. The draft lives in the decision
  record until a follow-up ticket lands it with its code.
- **An assumed answer.** Each open point looks small, and several have an obvious
  favourite in the Rust. Choosing one without the maintainer is the failure this
  ticket exists to prevent.
- **A new kind of effect.** A randomness trait in the library needs a decision record
  first (`AGENTS.md`). This ticket's record can be that record only if the maintainer
  rules for it at step 5.
- **Tickets that wait.** T46 and T47 cannot start until open points 7 and 8 are
  answered. If the other points stall, hand those two answers back first.
- **Three tickets, one decision.** T42, T45 and this ticket's follow-up each revise
  decision 0019. Whichever lands second re-reads the other's merged text.
- **A wire change.** A new lease-id form, new response fields and renamed error codes
  change the HTTP API. The follow-up tickets carry that; this one states it.
- **lychee and typos over `tickets/`.** The follow-up tickets and the hand-back notes
  write paths and test names as code spans, never as links.

## Goal

The maintainer has ruled on every open point above, with the examples in front of them.
A decision record states the caller contract: what each trigger answers, the lease's
id and where its parts come from, each named refusal, and the order the refusals are
checked, with the specification text that says so. Follow-up tickets exist that land
it in the specification, the Rust, the service, the double and the catalog, and T46 and
T47 have the two answers they wait on. No behaviour has changed.

## Non-goals

- No edit to `docs/specs/pachinko.allium`, `crates/` or `double/` in any commit.
- No ruling re-opened: the four above, the dropping of joining, no claim without
  records, the start rule.
- What a tick reports, and what `now` is: T42's.
- Dropping joining (T45), the `limit >= 1` rule (T46) and the start rule (T47)
  themselves.
- A write-ahead log or recovery. Open point 9 states a requirement for it and builds
  nothing.
- Authentication of a caller. A consumer id stays caller-supplied.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/<nnnn>-caller-contract.md` | new | The decision record: context, the rulings, the drafted specification text, each open point with its options and the maintainer's answer, the consequences, what would reopen it |
| `docs/manifest.yml`, `docs/decisions/README.md` | shared, own lines | The record's manifest entry and its row; the "next decision" sentence |
| `tickets/T<nn>-*.md` | new | The follow-up tickets step 6 creates |
| `tickets/README.md` | shared, own lines | The follow-up rows and graph edges; T43's Status cell |
| `tickets/T43-caller-contract.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Ask the maintainer for the base branch: this
   ticket reads `double/`, which `main` may not hold. Create the branch and its
   worktree from that base (`CONVENTIONS.md` §11). Run `just initialize` (network,
   authorised by this step). Run `just check` before any edit and quote its last line
   as the baseline.
1. **The plan.** Use the `plan-change` skill. Read findings 1, 2, 3 and 5 of
   `double/SPEC-NOTES.md` and the nine notes named in Context, the whole of
   `docs/specs/pachinko.allium`, and decisions 0019 and 0020.
2. **Re-verify.** Re-read everything "What exists today" names and correct this
   ticket's tables in the hand-back where the code has moved. Settle the unverified
   fact of open point 1: which `PublishOutcome` a publish that revives an idle pocket
   and one that revives a parked pocket answers, from the code and a test that shows
   it.
3. **The sources.** Read `allium-skill-reference/allium/language-reference.md` in full
   for open point 6 and record, with the section read, whether Allium 3 has a
   construct for a named rejection or an order of checks. Read RFC 9562's UUID version
   7 sections (network, authorised by this step) and record the layout, what it says
   of ids made within one millisecond, and what it guarantees about uniqueness; read
   also §4 and §8, for the text form ruled on 2026-11-25 and for open point 10. Quote
   each source; a claim from memory stays marked unverified.
4. **The draft and the record.** Use the `spec-change` skill for the drafting and the
   `review-docs` skill for the page. Write the decision record at the next free
   number, with its `docs/manifest.yml` entry and `docs/decisions/README.md` row. It
   holds:
   - the four rulings, with dates and the maintainer's reasons as Context states them,
     and the maintainer call of 2026-11-25 on the lease id's text form;
   - the drafted specification text for the ruled parts, as fenced excerpts: the
     `Lease` id, the `Batch` and lease view that expose it, `expires_at` as the answer
     of pull, renew, ack and nack, and that an ended lease may be forgotten;
   - for each open point, the options, a draft excerpt for each option where it changes
     the module, the examples, and what each option moves in the Rust, the service, the
     double and the catalog;
   - the two inventories, today's and the intended contract;
   - the status "proposed" until step 5.

   To show the excerpts are valid Allium, apply them to the module in the working
   tree, uncommitted, run `just check-specs` and `just analyse-specs`, quote both, save
   the diff under `ai_tmp/`, and restore the file with
   `git restore docs/specs/pachinko.allium`; that restore discards only this step's own
   draft and is authorised by this step. Where options conflict, check each in turn.
   Confirm `git status --short docs/specs` prints nothing before committing. Run
   `just check-docs`.

   **Adversarial review** (`CONVENTIONS.md` §14): the record against the rulings and
   the code. Does any excerpt or sentence assume an answer to an open point? Does the
   inventory match `refusals.py`, `error.rs` and the routes? Is every fact about Allium
   and UUIDv7 quoted from its source? Is each example true of the code?
5. **Authorisation required: the open points.** Put the ten open points to the
   maintainer, in Context's order, each with its examples, its options and the record's
   excerpt. Ask also how the follow-up work is to be cut into tickets (step 6 proposes
   a cut). Stop until the maintainer answers. Quote each answer in the hand-back,
   dated. If the answers come in parts, record open points 7 and 8 as soon as they are
   given, since T46 and T47 wait on them. An answer that names something outside these
   points is a new design: record it and stop again.
6. **The record, ruled, and the follow-up tickets.** Rewrite the record to state the
   decision, with each answer, its date and the options that lost. Then write the
   follow-up tickets, to `CONVENTIONS.md` §11 and §14 and shaped like
   `tickets/T39-decide-q2-q6.md`: self-contained, each restating the rulings and the
   answers it lands. Between them they cover:
   - the specification first (`AGENTS.md`, invariant 1), then the Rust library, with
     the double and `double/SPEC-NOTES.md` moving with the Rust;
   - the service: the HTTP API's responses, error codes and lease-id form,
     `docs/reference/http-api.md`, and the revision of decision 0019's "Lease ids"
     paragraph, noting that T42 and T45 revise the same decision;
   - the scenario catalog, `docs/reference/testing.md`: the rows that read an answer
     or a refusal (S053 and S070 among them) and the new obligations
     `just plan-spec pachinko` reports;
   - the removal of joining's refusals as T45's, not theirs; whichever of T45 and a
     follow-up lands second re-reads the other.

   Add each follow-up's row and edges to `tickets/README.md`. Tell the maintainer
   that T46 and T47 can now be given their answers; editing those two tickets is their
   owners' work unless the maintainer asks for it here.
7. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Does the
   ruled record say only what the maintainer answered? Is each follow-up ticket
   self-contained, and does any of them re-open a ruling or leave a part of the
   contract with no ticket? Then run `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- The baseline `just check` line is quoted, headed by its commit.
- The hand-back records which facts of "What exists today" were re-verified and each
  correction, and settles the revive outcome of open point 1 with its evidence.
- The Allium and RFC 9562 findings are quoted from the sources, with the section read.
- A decision record exists, registered in `docs/manifest.yml` and
  `docs/decisions/README.md`, and `just check-docs` passes.
- Every drafted excerpt passed `just check-specs` and `just analyse-specs` when applied
  to the module, with the output quoted, and no commit on the branch changes
  `docs/specs/pachinko.allium`, `crates/` or `double/`.
- The maintainer's answer to each of the ten open points is quoted in the hand-back
  with its date, and nothing before step 5 chose an option.
- The record states, as decided: what each trigger answers; the lease's id, its
  sources, and its text form as the maintainer call of 2026-11-25 ruled it; each named
  refusal; the check order for every trigger that can be refused; and how the module
  states a refusal.
- The answers T46 and T47 wait on are stated in the record under their own headings.
- Follow-up tickets exist for the specification, the Rust, the service, the double and
  the catalog, each with its row in `tickets/README.md`, and one of them revises
  decision 0019's "Lease ids" paragraph.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed
  back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed").
`<base>` is the base branch step 0 named.

```sh
set -o pipefail
just check-docs; echo "exit $?"
# positive control: the diff against the base is read and lists this ticket's files (exit 0)
git diff --name-only <base>...HEAD | rg -c '^(docs/decisions|tickets)/'; echo "exit $?"
# no behaviour changed. The diff is read into a file first, so a failed git read
# cannot pass as "no match": both lines must print "exit 0"
mkdir -p ai_tmp && git diff --name-only <base>...HEAD >| ai_tmp/t43-names.txt; echo "git diff: exit $?"
rg -n '^(docs/specs|crates|double)/' ai_tmp/t43-names.txt; [ "$?" -eq 1 ]; echo "exit $?"
# the record is registered: both must match (exit 0)
rg -c 'caller-contract' docs/manifest.yml docs/decisions/README.md; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

None yet.

## Open points

- The ten open points in Context are step 5's to settle.
- The base branch is step 0's to ask.
- How the follow-up work is cut into tickets is proposed at step 5 and fixed at step 6.
- Whether this ticket's record can serve as the decision record a new randomness
  effect needs depends on the answer to open point 9.
