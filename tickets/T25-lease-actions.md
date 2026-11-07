---
id: T25
title: "Ack, nack, renew, release and fencing"
status: todo
depends_on: [T24]
parallel_with: [T27, T29]
branch: ticket/t25-lease-actions
estimated_size: M
---

# T25: Ack, nack, renew, release and fencing

## Context

T23 landed the library tracer: the effects traits and their fakes, the `PocketRecords`
seam over `MemoryRecords`, `Queue` with join, publish and pull, and the `PocketTouched`
dispatch whose idle and parked arms were left for the tickets that can first reach those
states. T24 landed the service tracer: the owner task, the HTTP routes for join, a single
publish, pull and the lease view, and the exhaustive error map. A consumer can therefore
claim a pocket and hold records, and can do nothing with them: no ack, nack, renew or
release exists, no lease ever ends except by a lapse T26 has not yet built, and no pocket
can go idle or come back from idle.

This ticket lands the consumer's four actions in the library and on the wire, and the
fencing that makes them safe: every action names the lease it acts under, and a lease
that has ended refuses every action even when the same consumer holds the same pocket
again under a new lease and the same key at a newer rank (the module's comment above
`AckSettlesRecord`, guarantee `ActionsNameTheirLease`). The decisions it builds on are
the decision table in T22's Context and decision 0019: one owner task applies every
command, ended leases leave the active registry, and the executor's serialization is what
makes a late action and a lapse totally ordered (OQ8 closed with branch A, which T26
pins). The wire shape is PR 39's: the four lease routes, the `{error, message}` body and
the three lease error codes.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by this step) before `git show` can read them.
Never check a prototype branch out into the ticket's worktree; read with
`git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

What each prototype contributes: PR 38 `crates/libpachinko/src/queue/settle.rs` line 45
(`ack`) and line 78 (`nack`), `crates/libpachinko/src/queue/renew.rs` line 35 (`renew`),
line 66 (`release`) and line 80 (`authorise`, the guard order), and
`crates/libpachinko/src/queue/leases.rs` line 72 (`check`), line 91 (`extend`) and line
100 (`end`, the lease leaving `active` and `by_expiry`); PR 38 `crates/pachinko/src/actor.rs`
line 50 (the `Command` enum, with the four action variants below it) and line 329
(`apply`); PR 39
`crates/pachinko/src/http.rs` lines 43 to 52 (the routes), line 89 (`status`), line 104
(`code`), line 163 (`SettleBody`), line 169 (`LeaseBody`) and line 346 (`settle`, one
handler for ack and nack). PR 38's `free_all` is replaced by T23's `free_held`; PR 39's
wildcard arms in `status` and `code` are not carried (T24's map is exhaustive).

## Goal

`Queue` gains `ack`, `nack`, `renew` and `release`, each taking the consumer, the lease
and, for ack and nack, the record key, refusing in the order of the rules' `requires`
and otherwise doing exactly what the rule's `ensures` says; the lease view is complete;
a publish revives an idle pocket; the four routes answer on the wire with the three new
error codes in the exhaustive map; and every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (10) | `ReviveIdlePocket`, `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord`, `NackDefersNewerVersion`, `Renew`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `RequeueHeldRecord`, `RequeueDeferredRecord` |
| Invariants (3) | `IdleHasNoRecords`, `EndedAfterGrant`, `NackedBeforeExpiry` |
| Guarantees (3) | `ActionsNameTheirLease`, `OneVisitPerKeyPerLease`, `NothingLostWhileHeld` |
| Routes (4) | `POST /leases/{lease_id}/ack`, `POST /leases/{lease_id}/nack`, `POST /leases/{lease_id}/renew`, `POST /leases/{lease_id}/release` |
| Error codes (3) | `lease_not_active` (409), `lease_not_yours` (403), `record_not_held` (404) |

The scenarios this ticket owns, each pinned by a library test carrying `// scenario:`:

| Id | Scenario |
| --- | --- |
| S023 | A publish revives an idle pocket at the back |
| S035 | A consumer with a lease never gets `NoPocketAvailable` or another pocket |
| S040 | An ack settles a held record |
| S041 | An ack extends the lease |
| S042 | Rejected acks change nothing, not even the lease's expiry |
| S043 | A supersede while the key is held defers it on ack |
| S044 | A deferred record restarts at the back of its pocket |
| S045 | Two supersedes while held: both losers are shed and the held version is untouched |
| S046 | A stale check while held compares with the latest rank, not the held one |
| S047 | A nack dead-letters the held version on the first nack, without redelivery |
| S049 | Rejected nacks change nothing and create no dead letter |
| S050 | A renew extends the lease to now plus `lease_ttl` |
| S051 | Renewals do not stack |
| S052 | A renew of an ended lease is rejected and moves nothing |
| S053 | A release requeues the pocket at the back and its held records in place |
| S054 | A release with only deferred records requeues the pocket rather than idling it |
| S055 | A release idles an empty pocket |
| S057 | A second release is rejected; a release at the grant instant is legal |
| S068 | A consumer that re-claims the same pocket cannot use its old lease |
| S070 | A consumer cannot act on another consumer's lease |
| S071 | A key acked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) |
| S072 | A key nacked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) |
| S076 | The consumer view exposes only the active lease and its held versions |

It exercises over HTTP, without owning them, S040, S047, S050, S053 and S068.

## Non-goals

- No lapse, no `tick`, no decay and no parked arm of `PocketTouched`: T26. This ticket
  creates dead letters and leaves them in the shed queue; nothing removes them yet.
- No `expires_at` guard on any action. The actions guard on the lease registry alone, so
  a late pull, ack, nack or renew succeeds and extends the lease, a late release
  succeeds by ending it and freeing its records, and neither is refused for being late
  (branch A); T26's S066 and S067 pin that order.
- No batch publish, no request-shape limits (T27); no readiness, log levels, timeouts or
  shutdown grace (T29).
- No answer to open question 9: S071 and S072 pin the rules as they stand and say so.
- No change to the specification. A rule this ticket cannot implement as written is
  handed back, never patched in the module.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/libpachinko/src/queue/settle.rs` | new | `ack` and `nack` on `Queue`, the four settle rules, `SettleOutcome`; unit tests under `#[cfg(test)]` |
| `crates/libpachinko/src/queue/renew.rs` | new | `renew` and `release` on `Queue`, `PocketFate`, the guard chain in rule order; unit tests |
| `crates/libpachinko/src/queue.rs` | shared, own lines | The two `mod` lines; the idle arm of `PocketTouched` (`ReviveIdlePocket`); `lease_view` completed; re-exports of the new public types |
| `crates/libpachinko/src/queue/leases.rs` | shared, own lines | `check`, `extend` and `end`: an ended lease leaves `active` and `by_expiry` and is returned with `ended_at` |
| `crates/libpachinko/src/queue/shed.rs` | shared, own lines | `keep_dead_letter`: dead letters keyed `(expires_at, arrival)` beside the superseded records |
| `crates/libpachinko/src/error.rs` | shared, own lines | `LeaseActionError` (`UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours`, `RecordNotHeld`), a thiserror enum |
| `crates/libpachinko/src/lib.rs` | shared, own lines | The crate docs name the four actions |
| `crates/libpachinko/tests/queue/main.rs` | shared, own lines | The `settle` and `lease` module lines |
| `crates/libpachinko/tests/queue/fixture.rs` | shared, own lines | Helpers the new files need: a pulled lease, a snapshot comparison |
| `crates/libpachinko/tests/queue/settle.rs` | new | The ack and nack scenarios, S071 and S072 |
| `crates/libpachinko/tests/queue/lease.rs` | new | The renew, release, revive, fencing and view scenarios |
| `crates/pachinko/src/actor.rs` | shared, own lines | `Command::Ack`, `Nack`, `Renew`, `Release` with their replies; the four `apply` arms; the handle methods |
| `crates/pachinko/src/http.rs` | shared, own lines | The four routes, the request and reply bodies, the three arms of `status` and `code`, the lease-id parser |
| `crates/pachinko/tests/http.rs` | shared, own lines | The wire tests: five scenarios exercised, one `unknown_consumer` test per route, one bad-lease-id test |
| `docs/reference/http-api.md` | shared, own lines | The four route rows, the lease-action paragraph, the three error rows and the refusal order |
| `docs/explanation/architecture.md` | shared, own lines | One paragraph in "The library": the actions, the registry and fencing |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added; pachinko Unreleased, Added |
| `tickets/T25-lease-actions.md` | ticket | `status:` and hand-back notes |

Every path except the five marked `new` and the ticket is also edited by T27 or T29, or
by T26 after this ticket merges; this ticket edits only its own lines in each. If T23
placed `PullOutcome` in a `types` submodule rather than `queue.rs`, `SettleOutcome` and
`PocketFate` go beside it and that file joins the table as `shared, own lines`.

## Steps

1. Create the worktree from `main` after T24 has merged (`README.md`, "How to pick up a
   ticket") and run `just initialize` there (network, authorised by this step), or, in a
   clean worktree already initialized on `main` whose branch carries nothing, create the
   branch there and skip `just initialize`. Run `just check` on the untouched worktree
   and quote its last line as the baseline. Run `git fetch origin` (network, authorised
   by this step) so `git show cb77f07:<path>` and `git show cf1a6b2:<path>` can read the
   prototypes.
2. **The plan.** Use the `plan-change` skill. Read the ten rules, the three invariants
   and the three guarantees above in `docs/specs/pachinko.allium`, and run
   `just plan-spec pachinko` to list the obligation ids the tests will carry. Read T23's
   `Queue`, `PocketRecords` (`settle`, `defer`, `free_held`, `held`) and the lease
   registry, and T24's `Command`, `QueueHandle` and error map, before designing anything
   against them.
3. **The library actions.** Use the `rust-change` skill. In `settle.rs` and `renew.rs`:
   - Every action checks, in this order and before any effect: the consumer has joined
     (`UnknownConsumer`); the lease is in the active registry (`LeaseNotActive`, which
     covers an ended lease and one that never existed alike, because an ended lease
     leaves `active` and `by_expiry`); the lease's consumer is the caller
     (`LeaseNotYours`); for ack and nack, the record exists in the lease's pocket and is
     held (`RecordNotHeld`). A refused action changes nothing, `expires_at` included
     (S042, S049): the extension is applied after the last guard, never before.
   - No action compares `expires_at` with `now`. A lease the registry still holds is
     active whatever the clock says; that is branch A, and T26 pins it.
   - `ack`: `held_rank = rank` settles the record (`AckSettlesRecord`: the record is
     removed, `SettleOutcome::Settled`); `held_rank < rank` defers it
     (`AckDefersNewerVersion`: status deferred, a fresh position from `next_position`,
     the held fields cleared, `SettleOutcome::Deferred`). Both extend the lease to
     `now + lease_ttl`.
   - `nack`: the same split (`NackSettlesRecord`, `NackDefersNewerVersion`), and both
     create a `DeadLetter` from the held rank and payload, stamped `nacked_at = now` and
     `expires_at = now + dead_letter_ttl`, kept in the shed queue keyed
     `(expires_at, arrival)` beside the superseded records. `NackedBeforeExpiry` holds by
     construction; the test asserts it on the snapshot.
   - `renew`: extends to `now + lease_ttl` and returns the new `expires_at`; the
     `by_expiry` index moves with it. Renewals replace, never add (S051).
   - `release`: the pocket's held and deferred records are freed in one `free_held`
     call, O(held), each back to queued at its own position (`RequeueHeldRecord`,
     `RequeueDeferredRecord`); then, with records left, the pocket requeues at the back
     with a fresh position and count 0 (`ReleaseRequeuesPocket`, `PocketFate::Queued`),
     and with none it goes idle with count 0 (`ReleaseIdlesEmptyPocket`,
     `PocketFate::Idle`). `records.count` in the module counts deferred records, so a
     pocket holding only deferred records requeues (S054). The lease ends as released
     with `ended_at = now`, leaves both registry indexes, and is returned to the library
     caller as it ended, so a test can assert `EndedAfterGrant` and that a release at the
     grant instant has `ended_at = granted_at` (S057).
   - `ReviveIdlePocket`: the idle arm of T23's `PocketTouched` dispatch requeues the
     pocket at the back with count 0 (S023).
   - `lease_view` returns the pocket key, `expires_at` and the held records at their
     held rank and payload, never the latest rank after a supersede while held (S076).
   - New public types `LeaseActionError`, `SettleOutcome` and `PocketFate` meet invariant
     3 (`Send + Sync + 'static`, `Clone`, `Debug`, `#[non_exhaustive]`) and carry a
     doctest each; the four methods carry one each.
4. **The library tests.** Use the `rust-change` skill. `tests/queue/settle.rs` and
   `tests/queue/lease.rs` pin the 23 scenarios above, each test tagged
   `// scenario: S0NN` and `// obligation:` with the ids `just plan-spec pachinko`
   prints. The designs that need stating:
   - S035: a consumer holding a lease pulls while another pocket is queued, and again
     while none is; both return its own pocket's batch, never `NoPocketAvailable` or the
     other pocket; after a release the next pull claims the other pocket.
   - S043 to S046: publish rank 1, pull, supersede to rank 3 while held, ack: the record
     is deferred at a fresh position behind a key published in between (S044); a second
     supersede to rank 5 before the ack sheds rank 3 too and the held version stays rank
     1 (S045); a publish at rank 2 while rank 3 is stored is `stale`, and one at rank 3
     is `EqualRank`, the OQ1 placeholder, so the comparison reads the latest rank and
     never `held_rank` (S046).
   - S047: nack rank 1 while held; one dead letter at rank 1, the record gone, and a pull
     under the same lease hands nothing back for the key.
   - S041, S050 and S051: pull at 0 with `lease_ttl` 10 ms; an ack at 4 gives
     `expires_at` 14, a renew at 6 gives 16, and a second renew at 6 still gives 16; the
     `by_expiry` index holds one entry for the lease after each.
   - S053 and S055: pockets A then B; pull A, ack one of two records, release: A is queued
     behind B with count 0 and its remaining record at its original position, and the
     next pull claims B. Pull A again, ack its last record, release: A is idle, the
     snapshot shows it with no records (`IdleHasNoRecords`), and a pull finds only what
     else is queued.
   - S057: release at the grant instant succeeds with `ended_at = granted_at`
     (`EndedAfterGrant` with equality); a second release is `LeaseNotActive`.
   - S052 and S068: release the lease, pull the same pocket again under a new lease,
     then renew, ack and release with the old lease: each is `LeaseNotActive`, and a
     snapshot taken before equals the one after.
   - S070: two consumers; each action with the other's lease is `LeaseNotYours`, after
     `LeaseNotActive` would have been reported for an ended one, never before it.
   - S071 and S072: ack (or nack) the held version, publish the key again under the
     same lease, pull again: the new record is handed out, as the rules stand; the test
     is labelled `OQ9` in its name and comment.
   Run `just fmt-check`, `just clippy`, `just test`, `just doc` and `just coverage`
   (at or above 90% over `crates/*/src/**`), and `just effects` (nothing new reaches the
   clock except through the trait).
5. **The service.** Use the `rust-change` skill. In `actor.rs`, four `Command` variants
   with oneshot replies, four `QueueHandle` methods and four `apply` arms; each arm
   answers with the library result and nothing else. In `http.rs`, the four routes; the
   handler parses the path's `lease_id` as 32 lower-case hexadecimal digits and answers
   400 `bad_request` otherwise, before anything reaches the owner. The shapes:

   | Route | Request body | Reply on 200 |
   | --- | --- | --- |
   | `POST /leases/{lease_id}/ack` | `{"consumer_id": "<id>", "record_key": "<key>"}` | `{"outcome": "settled"}` or `{"outcome": "deferred"}` |
   | `POST /leases/{lease_id}/nack` | `{"consumer_id": "<id>", "record_key": "<key>"}` | `{"outcome": "settled"}` or `{"outcome": "deferred"}` |
   | `POST /leases/{lease_id}/renew` | `{"consumer_id": "<id>"}` | `{"expires_at": <milliseconds since the Unix epoch>}` |
   | `POST /leases/{lease_id}/release` | `{"consumer_id": "<id>"}` | `{"pocket": "queued"}` or `{"pocket": "idle"}` |

   Refusals carry `{"error": "<code>", "message": "<text>"}` in this order, which is the
   order of the rules' `requires` and the only order the owner evaluates: 404
   `unknown_consumer` (the consumer never joined, checked before any lease guard), then
   409 `lease_not_active` (ended or never existed), then 403 `lease_not_yours`, then,
   for ack and nack, 404 `record_not_held`. The three new `LeaseActionError` variants
   extend `status()` and `code()`; the map keeps no wildcard arm, so a variant left
   unmapped fails the build. One error-map unit test per new code asserts status and
   body, as T24's do.
6. **The wire tests.** In `crates/pachinko/tests/http.rs`, against the in-process server
   T24 built: S040, S047, S050, S053 and S068 exercised over HTTP, each tagged
   `// scenario:` with the words "exercised over HTTP" (ownership stays with the library
   test); one test per route that an unjoined consumer gets 404 `unknown_consumer` even
   when the lease id is well formed and unknown; one test that a malformed lease id gets
   400 `bad_request`. Run `just test`.
7. **The handbook.** Use the `review-docs` skill. `docs/reference/http-api.md`: the four
   route rows with the bodies above, a paragraph on the lease id and fencing, the three
   error rows and the refusal order; `docs/explanation/architecture.md`, "The library":
   one paragraph on the actions, the registry and why an ended lease is unknown rather
   than remembered; `CHANGELOG.md`: both Unreleased sections. Run `just check-docs`.
8. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against the Goal table,
   the scenario table, Files touched, the ten rules in `docs/specs/pachinko.allium` and
   the refusal order in Step 5. Ask: whether an ack can settle a record the lease does
   not hold, or one held under another lease on the same key; whether a release can leave
   a held or deferred record held; whether `lease_not_yours` can be returned before
   `lease_not_active`, or `record_not_held` before either; whether any scenario test
   passes with its rule's guard removed; whether a rejected action moves `expires_at`;
   whether any action reads `expires_at` against `now`, so that T26's branch-A order
   could fail; whether a wire test passes against the library alone; and whether any
   file outside Files touched changed. Codex `gpt-6-astra` reviewers run read-only with
   fresh contexts, one lens each; separate triage subagents write verdicts; valid fixes
   are applied one writer at a time; two rounds at most.
9. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
   `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
   `git fetch origin` (network, authorised by this step) and merge `origin/main`: T27
   and T29 are parallel lanes and merge one at a time (`CONVENTIONS.md` §11, the wave-2
   rule), so a conflict in a shared file is resolved to keep both lanes' lines, and
   `Cargo.lock` is regenerated with `just lock` (network, authorised by this step) rather
   than merged by hand. Rerun the Verification block at the merge commit and requote it.
   Commit on the ticket branch with the hand-back notes, each quoted output headed by the
   commit it ran at. Pushing and opening the pull request are separately authorised:
   stop and ask.

## Acceptance criteria

- `Queue::ack`, `Queue::nack`, `Queue::renew` and `Queue::release` exist with the
  signatures in Step 3, refuse in the order `UnknownConsumer`, `LeaseNotActive`,
  `LeaseNotYours`, `RecordNotHeld`, and a refusal leaves the queue's snapshot unchanged.
- No action reads `expires_at` against `now`; a grep of the two new modules for `now`
  finds only the extension, the dead letter's stamps and release's `ended_at`.
- An ended lease is absent from both registry indexes, and `release` returns it with
  `ended_at >= granted_at`.
- `LeaseActionError`, `SettleOutcome` and `PocketFate` are `Send + Sync + 'static`,
  `Clone`, `Debug` and `#[non_exhaustive]`, and every new public item has a doctest.
- The 23 scenarios in the Goal table each have one library test tagged with their id,
  and S071 and S072 are labelled OQ9; `just plan-spec pachinko` prints the obligation ids
  the tests' `// obligation:` tags carry, and the hand-back lists the ids this ticket's
  tests cover.
- The four routes answer the bodies in Step 5; the error map has arms for the three new
  codes and no wildcard; S040, S047, S050, S053 and S068 are exercised over HTTP; each
  route has an `unknown_consumer` test and the lease-id parser has a `bad_request` test.
- `just coverage` reports at or above 90% of lines over `crates/*/src/**`, and
  `just effects` passes.
- `just check-docs` and `just check` are green at the commit that merges `origin/main`.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just plan-spec pachinko
just fmt-check
just clippy
just test
just doc
just effects
just coverage
just check-docs
just check
git status --short
```

## Hand-back notes

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

- Open question 9 stays open: S071 and S072 pin the rules as they stand, a key acked or
  nacked and published again under the same lease being a new queued record pullable in
  that lease. If the maintainer chooses one visit per key, the `Visit` design in T19's
  Open points is the ready answer and these two tests flip.
- T19's reading that a release ends the lease and leaves `expires_at` alone is now code:
  an ended lease leaves the registry, so nothing can read its `expires_at`. The maintainer
  confirms, or asks for the dead extension to be written for literal conformance.
- T19's deferral reading is now code: a deferred record takes its fresh position at the
  ack or nack, so a key published between that ack and the release queues behind it
  (S044). The alternative assigns the position when the lease ends.
- Dead letters accumulate until T26 lands decay; superseded records already did after T23.
  Nothing bounds either queue; that is open question 3, which T27 restates.
- The lease view is read through `GET /consumers/{consumer_id}/lease` (T24); whether a
  route should also expose a lease by its id is a product decision nobody has asked for.
- A lapse that T26 lands uses the same `free_held` and the same registry `end`; if T26
  finds the lapse path needs a different return shape, it extends these rather than
  adding a second path.
