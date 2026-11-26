---
id: T46
title: "No claim without records: a pull with a limit below 1 by a consumer with no lease claims no pocket (finding 11, ruled)"
status: todo
depends_on: [T43]
parallel_with: []
branch: ticket/t46-no-claim-without-records
estimated_size: M
---

# T46: No claim without records: a pull with a limit below 1 by a consumer with no lease claims no pocket (finding 11, ruled)

## Context

`double/SPEC-NOTES.md` lists the questions the Python double of the library raised
about `docs/specs/pachinko.allium`. Its finding 11, with note N19, asks: "Is a claim
that hands nothing intended?" Today a pull whose limit is zero or negative, made by a
consumer that holds no lease, claims the front pocket, creates a lease on it and answers
an empty batch. The pocket leaves the pocket order for the lease's life with its records
still queued, so a consumer probing with `limit = 0` takes a pocket away from every
other consumer until it releases the lease or the lease lapses.

The maintainer ruled on finding 11 on 2026-11-24 and chose to land the ruling by ticket,
because it changes behaviour in the specification, the library, the service, the double
and two catalogued scenarios. This is that ticket.

Nothing in this ticket re-opens the ruling. A finding that the ruling is wrong goes to
the maintainer as a question, never into a fix.

### The ruling on finding 11: no claim without records (2026-11-24)

- **No claim.** A pull with a limit below 1 by a consumer with no lease claims no pocket
  and hands nothing: no lease is created, no pocket's status changes, no record is held.
- **Inside a held lease nothing changes.** A pull with a limit below 1 by a consumer
  that holds an active lease still extends the lease and answers an empty batch, as
  today. `PullWithinLease` is not touched.
- **The rule.** `PullClaimsFrontPocket` gains `requires: limit >= 1`.
- **What the refused case answers is not part of this ruling.** What a pull with
  `limit <= 0` and no lease answers is settled with findings 2 and 5 (what each action
  answers, and the specification's named refusals and their order), which
  `tickets/T43-caller-contract.md` explores and puts to the maintainer. That is why this
  ticket depends on T43: step 1 reads T43's merged decision and stops if it is absent.
- **A behaviour change.** The specification's rule, the `OldestFirst` guarantee's
  wording ("a limit that is not positive yields an empty batch, not an error"), the
  comment above `PullWithinLease`, the Rust library, the service, the Python double, and
  catalog rows S031 and S032 in `docs/reference/testing.md` all move.
- **The reasoning**, as finding 11 and N19 record it; the maintainer gave no further
  reason with the ruling: a claim that hands
  nothing is not intended. The comment above `PullWithinLease` explains why an empty
  batch extends a held lease; nothing explains why a limit that is not positive should
  claim a new one, and such a claim keeps the pocket's queued records from every other
  consumer.

### What the new `requires` does to the Pull rules

The three rules that answer `Pull(consumer, limit)` today have disjoint guards that
cover every state the invariants hold in:

| Rule | Holds when |
| --- | --- |
| `PullWithinLease` | the consumer has an active lease |
| `PullClaimsFrontPocket` | the consumer has no active lease, and a pocket is queued |
| `PullFindsNoPocket` | the consumer has no active lease, and no pocket is queued |

With `requires: limit >= 1` on `PullClaimsFrontPocket`, a pull by a consumer with no
lease, with a pocket queued and with `limit <= 0`, matches neither
`PullClaimsFrontPocket` nor `PullFindsNoPocket`, which requires that no pocket is
queued. The specification therefore needs either a third rule for that case or a named
refusal of it. Which, and its name, is exactly the answer T43 decides. Three things
follow, and this ticket carries each:

- Note N14's claim that "one of the three always holds", and so that the Pull table's
  refusal is unreachable through the queue, becomes false if T43 answers with a refusal:
  the gap is then reached by an ordinary call. Finding 6's ruling (2026-11-24) labelled
  `PullClaimsFrontPocket`'s `requires: pocket.status = queued` as a restatement of its
  selection that cannot fail; that label stays true. What does not stay true is the
  surrounding claim, in N14, in the `unreachable-answers` figure's text, and in the
  double's `pull.py` and `refusals.py` docstrings, that a Pull rule always holds.
- One case sits beside the gap and the ruling does not speak to it: a consumer with no
  lease, no pocket queued, and `limit <= 0`. Today `PullFindsNoPocket` answers
  `NoPocketAvailable` there, and under the ruling's one new `requires` it still does. If
  T43 answers the gap with a refusal, the same call is then refused or answered
  `NoPocketAvailable` according to whether a pocket happens to be queued. Whether that
  is intended is for T43's decision or the maintainer, not for this ticket (step 1).
- The double's `PullRefused` is today documented and tested as unreachable. Whatever
  T43 names, the double's handling of the Pull table's no-guard answer moves with it.

### What exists today

Re-read each at step 3; these were true when the ticket was written, on the branch
`build-python-double` with the first part of the rulings applied but not committed, so
every line number is approximate.

The specification, `docs/specs/pachinko.allium`:

- The Pull comment, lines 487 to 490: `oldest(records, r => r.position, n)` is a black
  box that returns "all of them when n exceeds their count and none when n is not
  positive".
- The comment above `PullWithinLease`, lines 500 to 510. It says an empty batch extends
  the lease and that this is intended, and ends "No lease is granted on an empty pocket:
  `PullClaimsFrontPocket` claims only a queued pocket, ...". It says nothing about a
  limit.
- `PullWithinLease`, lines 511 to 525. Its batch is
  `oldest(pocket.queued_records, r => r.position, min(limit, config.max_pull_batch))`,
  and it ensures `lease.expires_at = now + config.lease_ttl` whatever the batch holds.
- `PullClaimsFrontPocket`, lines 527 to 554. Its `requires` are
  `not exists Lease{consumer: consumer, status: active}` (529), `free.count > 0` (531)
  and `pocket.status = queued` (535), the last under the comment finding 6 added (533
  to 534): "Restates the selection: pocket is one of the queued pockets, so this cannot
  fail." It has no `requires` on `limit`.
- `PullFindsNoPocket`, lines 569 to 575: no active lease and `free.count = 0`; it
  ensures `NoPocketAvailable(consumer: consumer)`. It has no `requires` on `limit`.
- The comment above `invariant PullBatchPositive`, lines 976 to 978: "The model is
  defined for max_pull_batch >= 1: below that no pull can hand out a record, so no
  record ever leaves queued." N19 lists this comment among the texts it concerns; it
  bounds the config value, not a pull's limit.
- The `Consuming` surface's `@guarantee OldestFirst`, lines 1185 to 1189: "never more
  than max_pull_batch and never more than limit when limit is positive; a limit that is
  not positive yields an empty batch, not an error."

The library, `crates/libpachinko/`:

- `src/queue/pull.rs` lines 13 to 17: `batch_size` clamps the limit to
  `0..=max_pull_batch`, so a limit that is not positive is a size of 0.
- `src/queue/pull.rs` lines 62 to 69: `Queue::pull` reads the consumer's active lease
  (which refuses an unknown consumer first), then calls `pull_within_lease` or
  `claim_front_pocket` with that size. `claim_front_pocket` (lines 91 to 127) answers
  `NoPocketAvailable` when no pocket is queued, and otherwise draws a lease id, claims
  the front pocket and holds `size` records. No branch reads the limit before claiming.
- The doc comment of `Queue::pull` (lines 20 to 39) says a batch holds "none when
  `limit` is not positive", and its doctest (line 57) claims a pocket with
  `queue.pull(&c, 0)` and asserts an empty batch.
- `src/error.rs` lines 73 to 95: `PullError` is `#[non_exhaustive]` with two variants,
  `UnknownConsumer` and `LeaseId` (`LeaseIdError::Exhausted` or `StillActive`).
  `src/queue/types.rs` lines 87 to 105: `PullOutcome` is `#[non_exhaustive]` with
  `Batch { lease, pocket_key, expires_at, records }` and `NoPocketAvailable`.
  `src/lib.rs` line 25 tabulates both for `Queue::pull`.
- `tests/queue/pull.rs` lines 129 to 138, S031,
  `a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch`: a pull with limit 0
  by `c1` with no lease leaves the pocket `Leased`, the record `Queued` and one lease.
- `tests/queue/pull.rs` lines 140 to 149, S032, `a_negative_limit_behaves_like_zero`: a
  pull with limit -1 claims the pocket; a pull with `i64::MIN`, then inside the lease,
  hands nothing; a pull with 10 hands `r0` and `r1`.
- `tests/queue/publish.rs` line 362, the last line of S022's test: it learns the front
  pocket by `fixture.pull("c1", 0).pocket_key`, a claim with limit 0 by a consumer with
  no lease.
- The invariant walk draws limits from -1 to 3 (`tests/queue/walk.rs` line 134). Its
  model, `tests/queue/walk/model.rs` lines 345 to 375, claims the front pocket before it
  computes the size, and `walk.rs` lines 381 to 395 label what a pull reached ("pull
  claims", "empty batch", "pull NoPocketAvailable"); `tests/queue/invariants.rs` lists
  the labels the walk must reach (near line 500) and documents `check_batch`'s size as
  "none for a limit that is not positive" (line 339).

The service, `crates/pachinko/`:

- The HTTP API accepts a limit of zero or less. `src/http.rs` line 406: the pull body's
  `limit` is an `i64`, and line 562 passes it to the queue unchanged. A body with no
  `limit`, or one that is not an integer, is 400 `bad_request`
  (`tests/http.rs` lines 371 to 372). `src/http.rs` line 296 maps
  `PullError::UnknownConsumer` to the refusal answered 404 `unknown_consumer`;
  `src/actor.rs` lines 530 to 531 turn `PullError::LeaseId` into the owner stopping.
- `src/actor.rs` lines 84 to 85: the `Pull` command's doc comment says "a `limit` of
  zero or less claims a pocket and hands out none".
- `tests/http.rs` lines 280 to 298, S031 over the wire (driver `zero_limit`): a pull
  with `{"limit": 0}` answers 200 with `"outcome": "batch"` and `"records": []`, and the
  lease read then shows a lease on the pocket. The driver also carries
  `// obligation: rule-success.PullClaimsFrontPocket`.
- `docs/reference/http-api.md`: the route table's pull row (line 28) lists 200 with a
  batch, 200 `no_pocket`, 404 `unknown_consumer` and 400 `bad_request`. Lines 38 to 39:
  "A `limit` of zero or less claims a pocket and returns an empty batch". Lines 96 to
  98 describe the pull under a lease that returns no records "because `limit` is zero or
  less", which stays true. Lines 124 to 128 say a pull applied after a lapse "claims a
  pocket under a new lease, and is handed the records the lapse queued, up to its
  `limit` and none at zero or less". Lines 143 to 147 advise a client that lost a batch
  to "pull with `{"limit": 0}`, which holds nothing more but returns the lease id"; that
  pull is inside the held lease and keeps working, but once the lease has lapsed the
  same call no longer claims a pocket, and the page should say what it answers then.

The catalog, `docs/reference/testing.md`:

- Row S031 (line 225): "A zero-limit pull claims a pocket and returns an empty batch",
  `edge, wire`, with the library test and the wire test named above.
- Row S032 (line 226): "A negative limit behaves like zero", `edge`, library test only.
- The page holds 93 scenarios, S001 to S093, and describes the walk's domain as "limits
  -1 to 3" (line 129).

The Python double, `double/`:

- `pachinko_double/pull.py` lines 77 to 81: `oldest` returns `()` when `n <= 0`.
- `pachinko_double/pull.py` lines 149 to 156: `PullClaimsFrontPocket`'s guard, which
  reads no limit. Lines 221 to 225: `PullFindsNoPocket`'s guard. Lines 233 to 244: the
  Pull table, three rows, `NoGuardHolds.REFUSAL`, and the refusal `PullRefused`.
- The module docstring of `pull.py` (lines 14 to 17) and the docstring of `PullRefused`
  in `pachinko_double/refusals.py` (lines 51 to 61) both say one Pull rule always holds
  in a valid state, so `Queue` never raises `PullRefused`.
- `tests/test_pull.py` lines 255 to 265, S031, and lines 267 to 273, S032, mirror the
  Rust tests. Lines 192 to 206 hold an untagged test that a zero or negative limit
  inside a lease extends the lease and hands nothing; it stays true.
- `tests/test_pull.py` lines 436 to 454, `PullGuardsTest`: its docstring says one guard
  always holds, and line 449 claims a pocket with `batch(queue, "c1", 0)`. Lines 418 to
  433 hold the `impl-choice (spec note N14)` test that reaches `PullRefused` on a
  hand-built state.
- `tests/test_publish.py` line 254, the last line of S022's test, claims with limit 0 as
  the Rust's does.
- `double/SPEC-NOTES.md`: finding 11 and note N19 carry the ruling and name this
  ticket; note N14 says this ticket updates it; the figure
  `double/diagrams/unreachable-answers.svg`, embedded under N14, says in its text "so a
  Pull row always holds and PullRefused is met only on a hand-built state"; N15's
  flowchart draws `PullRefused` as "none: hand-built state only (N14)"; the rulings
  table has a row for finding 11 naming S031, S032 and N19.
- `double/` and `double/SPEC-NOTES.md` exist on the branch `build-python-double`. They
  were not on `main` when this ticket was written.

Not verified when the ticket was written; step 3 checks each against the tree:

- How `double/diagrams/unreachable-answers.svg` is drawn. `scripts/diagrams.py` says it
  draws the diagrams under `docs/`; whether it, another script or a hand edit owns the
  double's figures was not established.
- Whether any Rust test other than S032's second pull pins the zero-limit pull inside a
  lease. A search of the test names found none.
- Whether `fuzz/`, the benches or the load generator ever pull with a limit below 1
  without a lease. A search for literal limits found none.
- Whether any page beyond those in Files touched states the old behaviour. The searches
  under Verification are the check.

### Cross-ticket constraints

- **T43 (the caller contract)** decides what the `limit <= 0`, no-lease pull answers.
  This ticket lands that answer and does not choose it.
- **T39 (the operator listing and `new_pocket_first`)** requires every scenario from
  S001 to S093 to pass with its test unchanged when its switch is off. This ticket
  changes the tests of S031 and S032, and the last line of S022's test, which claims
  with limit 0 only to learn the front pocket. Whichever of T39 and T46 lands second
  re-reads the other's merged text: if T39 is second, its "unchanged" is measured
  against the tree with this ticket in it; if this ticket is second, its new claim guard
  is read against T39's changed `PullClaimsFrontPocket`, and both tickets take new
  scenario ids from the catalog's next free id at the time, never from S094 by
  assumption.
- **T45 (drop joining)** removes `ConsumerJoins` and the `UnknownConsumer` refusal. If
  it has merged, a consumer needs no join before it pulls, and the unknown-consumer
  check before the limit check is gone; if it has not, both remain. Step 3 re-reads the
  merged state and writes tests and pages for the state it finds.
- **Finding 6's labels.** The comment on `PullClaimsFrontPocket`'s
  `requires: pocket.status = queued` says it restates the selection and cannot fail.
  That stays true. What stops being true is the claim around it, in note N14 and
  finding 6, that some Pull rule always holds. This ticket updates both: N14 and
  finding 6's text in `double/SPEC-NOTES.md`, and the comment in the module, which must
  say that the selection is restated and that the new `requires: limit >= 1` beside it
  is a real guard. The new guard does not sit under the restatement comment.

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The rule, the guarantee
  and the comments land, and pass `just check-specs` and `just analyse-specs`, before
  any test or code.
- **A dependency that has not decided.** If T43's decision is absent, or does not name
  the answer, this ticket has nothing to land. It stops at step 1 and invents no
  refusal, rule or status code.
- **A gate that cannot fire.** The claim "no pocket is claimed" is shown by the whole
  snapshot being equal before and after the pull, and by the lease-id source not being
  drawn, not by an empty `records` alone: the old behaviour also answers no records.
- **Half a ruling.** The library, the service and the double land together. A library
  that refuses and a service that maps the refusal to a 500, or a double that still
  claims, is a defect.
- **A public surface changes.** `Queue::pull` behaves differently for an input it
  accepted, and `PullError` or `PullOutcome` may gain a variant; both enums are
  `#[non_exhaustive]`. `just semver` reports it, the change is recorded in both
  Unreleased sections of `CHANGELOG.md`, and each new public item keeps invariant 3 and
  has a doctest.
- **The walk's model moves with the code.** The invariant walk predicts every outcome,
  so a model left unchanged fails loudly; a model changed without the label list in
  `invariants.rs` lets a reached outcome go unrequired.
- **The small denominator.** The new branch has its test, or coverage falls below 90%.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links.

## Goal

A pull with a limit below 1 by a consumer with no active lease claims no pocket in the
specification, the library, the service and the double: it creates no lease, draws no
lease id, changes no pocket and holds no record, and it answers what T43's decision
says. A pull with a limit below 1 inside a held lease behaves exactly as before. The
specification's `PullClaimsFrontPocket` carries `requires: limit >= 1`, every text that
said a non-positive limit claims a pocket or that a Pull rule always holds says what is
now true, and catalog rows S031 and S032 pin the new behaviour.

## Non-goals

- No choice of what the refused case answers, of a refusal's name, or of its place in
  the order refusals are checked. Those are T43's.
- No change to `PullWithinLease`: an empty batch still extends a held lease, whatever
  the limit.
- No change to `oldest`, which still returns no record for a count that is not
  positive, and none to `PullBatchPositive`.
- No change to the labels on the Release rules' restating `requires`, and none to the
  expiry rules' guards, which wait on `tickets/T42-clock-model.md`.
- No upper bound on a limit beyond `max_pull_batch`, and no change to how a limit above
  it is capped.
- No edit to finished tickets that quote the old titles of S031 and S032
  (`tickets/T23-library-publish-pull.md`, `tickets/T24-service-http-tracer.md`,
  `tickets/T28-invariant-walk.md`, `tickets/T31-mvp-integration.md`); they record what
  was true when they ran.
- No removal of joining; that is `tickets/T45-drop-joining.md`.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | `requires: limit >= 1` in `PullClaimsFrontPocket`; the rule or refusal T43 decided for the gap; the `OldestFirst` wording; the comment above `PullWithinLease`; the Pull comment, if it must say which pulls reach `oldest` with a count that is not positive |
| `crates/libpachinko/src/queue/pull.rs`, `error.rs`, `queue/types.rs`, `lib.rs` | shared, own lines | The guard before the claim; the answer T43 decided; the doc comments, the doctest and the table in `lib.rs` |
| `crates/libpachinko/tests/queue/pull.rs`, `publish.rs`, `walk.rs`, `walk/model.rs`, `invariants.rs` | shared, own lines | S031 and S032; S022's last line; the walk's model, its reach labels and the required-label list |
| `crates/pachinko/src/http.rs`, `actor.rs`, `error.rs` | shared, own lines | The wire form of the answer T43 decided; the `Pull` command's doc comment |
| `crates/pachinko/tests/http.rs` | shared, own lines | S031 over the wire, and a new home for its `rule-success.PullClaimsFrontPocket` tag if the wire suite needs one |
| `docs/reference/testing.md` | shared, own lines | Rows S031 and S032, any new row, and the counts |
| `docs/reference/http-api.md`, `docs/reference/api.md` | shared, own lines | The pull row's answers; the sentences on a limit of zero or less; the lost-batch advice once the lease has lapsed |
| `crates/libpachinko/README.md`, `README.md`, `docs/project/terminology.md` | shared, own lines, only if step 3 finds a sentence made false | The same |
| `CHANGELOG.md` | shared, own lines | Both Unreleased sections |
| `double/pachinko_double/pull.py`, `refusals.py`, `queue.py`, `exceptions.py` | shared, own lines, if the base holds `double/` | The claim guard; the Pull table's answer for the gap; the docstrings that say a Pull rule always holds |
| `double/tests/test_pull.py`, `test_publish.py`, `test_interface.py` | shared, own lines, if the base holds `double/` | S031, S032, `PullGuardsTest`, the N14 hand-built test, S022's last line |
| `double/SPEC-NOTES.md`, `double/README.md`, `double/diagrams/unreachable-answers.svg` | shared, own lines, if the base holds `double/` | Finding 11, N19, N14, N15's flowchart, the rulings table; the guide's Pull table; the figure's text if made false |
| `quint/pachinko.qnt`, `quint/mutants/*.patch`, `quint/traces/*`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines, if the tree this ticket lands on holds `quint/pachinko.qnt`, at the base or after the closing merge or rebase, and each path only where that tree holds it | `guard_PullClaimsFrontPocket` with `limit >= 1`, the rule or refusal T43 decided for the gap, and the `OldestFirst` property's wording; each mutant patch the edit breaks, re-cut; the trace fixtures, regenerated if they exist, with their generator and the two replays that read them, where the gap's answer changes what a step hands back |
| `tickets/README.md` | shared, own lines | T46's Status cell |
| `tickets/T46-no-claim-without-records.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch `ticket/t46-no-claim-without-records`
   and its worktree from `main`, after T43 has merged (`CONVENTIONS.md` §11). Run
   `just initialize` (network, authorised by this step). Run `just check` before any
   edit and quote its last line as the baseline. A red baseline is a `main` defect, not
   this ticket's work.
1. **Authorisation required: T43's decision.** T43 explores and decides; it lands no
   specification text itself. What it merges is a decision record under
   `docs/decisions/` that carries the checked draft, the maintainer's ruling quoted in
   `tickets/T43-caller-contract.md`'s hand-back notes, and the follow-up tickets that
   land the caller contract. Find all three. If a follow-up ticket of T43 lands the
   way the specification states a named refusal, and this ticket's answer is a
   refusal, ask the maintainer whether this ticket waits for that follow-up or lands
   its one refusal in the form the decision record drafts. If
   `tickets/T43-caller-contract.md` is not `status: done` on `main`, or the decision
   does not say what a pull with `limit <= 0` and no lease answers, stop and tell the
   maintainer; do not proceed on a guess. Otherwise quote the decision, with its path
   and commit, and then put to the maintainer each of these that the decision leaves
   unsaid, quoting this ticket's Context:
   - Whether the answer is a named refusal or a third Pull rule with an answer of its
     own, and its name.
   - Whether a pull with `limit <= 0`, no lease and no pocket queued is answered the
     same way, or keeps `NoPocketAvailable` from `PullFindsNoPocket`.
   - Where the check sits among the others a pull meets, the unknown-consumer check
     included if joining still exists.
   - The wire form: the HTTP status and the error code or `outcome` value.
   - The new titles of S031 and S032. Suggested, not ruled: S031 "A zero-limit pull by a
     consumer with no lease claims no pocket", and S032 "A negative limit behaves like
     zero", with a pull inside a lease at a limit below 1 pinned by S032's later pulls
     or by a new row.
   - The base branch. If `main` does not hold `double/`, say so and ask whether the
     double moves here or in a follow-up.
   - The Quint model. Step 7's paragraph "The Quint model" names three cases, which
     step 0 checks beside the baseline; if one holds, put its question here.

   Stop until the maintainer answers, and quote the answers in the hand-back, dated. An
   answer that names something outside these points is a new design: record it and stop
   again.
2. **The merged state.** Read what merged since this ticket was written: T43's
   decision record and any of its follow-up tickets that has landed, and T39, T45 and
   T42 if any has landed. Note for each whether
   it changed a Pull rule, joining, the catalog's next free id, or a text this ticket
   edits.
3. **The plan.** Use the `plan-change` skill. Re-read everything "What exists today"
   names and correct each line number; settle the four unverified points listed there.
   Run `just plan-spec pachinko` and keep its output for comparison. Write down, before
   any test: the specification lines to change; the tests to change and add; the
   library, service and double changes; each page and note to change.
4. **The specification.** Use the `spec-change` skill. Add `requires: limit >= 1` to
   `PullClaimsFrontPocket`, apart from the restatement comment. Add the rule or refusal
   step 1 fixed for the gap, in the form T43's decision record drafts for refusals, or
   the form its follow-up landed if step 1 found one. Reword `OldestFirst`
   so it no longer promises an empty batch for every limit that is not positive: inside
   a lease such a limit yields an empty batch, and without a lease it yields the answer
   step 1 fixed. Extend the comment above `PullWithinLease` to say that only a pull
   inside a lease answers an empty batch for such a limit, and why no pocket is claimed
   without one. Run `just check-specs` and `just analyse-specs`, then
   `just plan-spec pachinko`, and list the obligations that appeared or changed id; a
   new `requires` renumbers `rule-failure.PullClaimsFrontPocket.N`, and the tags in
   `crates/` follow.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   ruling and step 1's answers. Does any pull with `limit <= 0` and no lease still reach
   a rule that creates a lease? Is every (lease, queued pocket, limit) combination
   answered by exactly one rule or one named refusal? Is `PullWithinLease` unchanged?
5. **The tests, red first.** Rewrite S031 and S032 in `crates/libpachinko/tests/queue/pull.rs`
   to the new behaviour: the whole snapshot is equal before and after the pull, the
   lease-id source is not drawn, the pocket stays `Queued`, a second consumer's pull
   then claims that pocket, and the answer is the one step 1 fixed. Keep a pinned pull
   inside a lease at limits 0, a negative number and `i64::MIN` that extends the lease
   and hands nothing. Change the last line of S022's test to learn the front pocket
   without a zero-limit claim. Update the walk's model and its labels. Rewrite S031 over
   the wire. Show each changed test failing against the unchanged code and quote the
   failures.
6. **The code.** Use the `rust-change` skill. The library first, then the service, in
   one commit or in commits that each leave `just check` green. The limit is checked
   before a lease id is drawn and before anything is written. Run `just fmt-check`,
   `just clippy`, `just test`, `just coverage`, `just doc`, `just effects` and
   `just semver` as the change needs them.
7. **The double**, if the base holds it: the claim guard, the Pull table's answer for
   the gap, S031, S032, S022's last line, `PullGuardsTest` and the docstrings, with
   `just python-double` green. Then `double/SPEC-NOTES.md`: finding 11 and N19 say the
   ruling has landed and what the pull answers; N14 says which Pull refusal is now
   reachable and which answer, if any, is still met only on a hand-built state; N15's
   flowchart and the `unreachable-answers` figure's text are corrected where they say a
   Pull row always holds. Cite notes by N-id and finding number.

   **The Quint model**, if the tree this ticket lands on holds `quint/pachinko.qnt`, at
   the base or after the closing merge or rebase onto `main`: that model is derived
   from this specification, so the same change updates it wherever it models what this
   ticket changes. Read the condition again after that merge: if it brought in the
   model, `quint/traces/` or a replay the base did not hold, or changed
   `quint/pachinko.qnt` or a patch under `quint/mutants/`, the rest of this step is
   done then, against the merged tree, before any gate runs, each stop in it
   included. The rule is the same-change
   rule in `AGENTS.md` and the decision record under `docs/decisions/` that introduced
   the model; both exist wherever the model does.
   Here that is the claim's guard, the answer T43 decided for the gap, and
   the `OldestFirst` property. In this order: edit the model; re-cut each mutant patch
   under `quint/mutants/` that the edit breaks, keeping its mutation and the property it
   names, and list each in the hand-back; if `quint/traces/` exists, run
   `just quint-traces` to regenerate it, changing `scripts/quint-traces.py` where a
   witness changes, and keep each replay that exists green, the Rust test under
   `crates/libpachinko/tests/queue/walk/` in `just test` and
   `double/tests/test_replay.py` in `just python-double`, where a field or a step the
   model gained is either mapped or added to that replay's own list of what it does not
   compare, with the ticket that will map it, and a step only with the maintainer's
   answer on which ticket that is; then run `just check-quint`, and
   `just verify-quint` if the recipe exists. The fixtures come before the two gates
   because both fail on a model whose hash the fixtures do not record. Where the
   model's header lists the behaviour under "Not yet modelled", change nothing there
   and say so in the hand-back.

   **Stop and ask, and decide it before this ticket's first edit, not at this step,**
   in any of three cases. One: the base holds `quint/pachinko.qnt` and the model has no
   pull at a positive limit yet, which its header says under "Not yet modelled",
   whatever the line's words, and which the pull limits of its instance show (-1 and 0
   only). Two: the base holds no `quint/pachinko.qnt` but holds
   `.scratch/quint-model/README.md`, and that file's index does not show its ticket
   QM05 as `done`. In both, the model's only claim, built or planned, is one at a limit
   that is not positive, which this ticket removes, and `PullClaimsFrontPocket` would be
   left with no witness there. Three: the base holds neither file. `main` cannot then
   say whether a Quint model of this specification is being built on another base,
   which that series allows, so the files' absence is not an answer: ask the
   maintainer, with step 1's questions, whether one is, and whether its pull at a
   positive limit has landed there, and go on only with the answer quoted in the
   hand-back. The maintainer decides whether this ticket waits or the series changes
   its plan. Run the checks in step 0, beside the baseline, and quote what they
   found.
8. **The handbook.** Use the `review-docs` skill for every page in Files touched. Rows
   S031 and S032 in `docs/reference/testing.md` carry their new titles and test names.
   `docs/reference/http-api.md` states the new answer in the route table and in each
   sentence on a limit of zero or less, and says what the lost-batch advice's
   `{"limit": 0}` pull answers once the lease has lapsed. Run `just check-docs`.
9. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes, each quoted output headed by its commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Can the
   gate pass while a zero-limit pull still claims, in the library, over HTTP or in the
   double? Does any page, doc comment or note still state the old behaviour? Then run
   `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- T43's decision and the maintainer's answers at step 1 are quoted in the hand-back with
  their dates, and nothing before them chose an option.
- `PullClaimsFrontPocket` in `docs/specs/pachinko.allium` carries `requires: limit >= 1`,
  and the comment on `requires: pocket.status = queued` is unchanged.
- Every combination of (active lease or none, a pocket queued or none, limit at least 1
  or below 1) is answered by exactly one Pull rule or by the one refusal step 1 fixed,
  and the hand-back lists the six combinations with the rule or refusal for each.
- In the library and in the double, a pull with limit 0, with -1 and with the smallest
  64-bit integer, by a consumer with no lease and a pocket queued, leaves the snapshot
  equal, draws no lease id, and answers what step 1 fixed. Each is pinned by a tagged
  test that was red before the code changed, and the red output is quoted.
- After such a pull by one consumer, another consumer's pull with a positive limit
  claims the pocket and is handed its oldest records.
- Inside a held lease, a pull with each of those three limits extends the lease to
  `now + lease_ttl`, hands nothing and answers an empty batch naming the same lease, in
  the library and in the double.
- Over HTTP, a pull with `{"limit": 0}` by a consumer with no lease answers the status
  and body step 1 fixed, and the lease read then answers 204.
- The invariant walk's model predicts the new answer, and the walk is required to reach
  it.
- Rows S031 and S032 of `docs/reference/testing.md` name tests that exist, each scenario
  id is tagged once per suite as before, and the catalog's counts match.
- No doc comment, page, docstring or note says that a limit of zero or less claims a
  pocket, or that one of the Pull rules always holds. N14, N19, finding 11, N15's
  flowchart and the `unreachable-answers` figure's text in `double/SPEC-NOTES.md` say
  what is now true.
- Every `// obligation:` id in `crates/` is in `just plan-spec pachinko`'s output or
  names a `@guarantee` of the module.
- Every new or changed public item keeps invariant 3 and has a doctest; `just coverage`
  reports at or above 90%; `just effects` passes; the `just semver` report is quoted;
  both Unreleased sections of `CHANGELOG.md` record the change.
- If T39 merged first, the hand-back states which of its texts were re-read and that
  S001 to S093 other than S022, S031 and S032 pass with their tests unchanged.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on `rg`'s status 1, never on an error, and
follows a positive control over the same files. At step 0, before any edit, the two
stale-text searches are run once and must match: that shows they can. Add `double` to
the trees searched if the base holds it. Finished tickets under `tickets/` are not
searched. The stale-text search matches the words "zero-limit pull claims" in any
spelling, so a new title or test name that keeps those three words in a row, such as
one ending "claims no pocket", fails it: word the new names otherwise, or narrow the
pattern and say so in the hand-back.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
# the new guard is in the module (exit 0)
rg -n 'requires: limit >= 1' docs/specs/pachinko.allium; echo "exit $?"
# positive control: the guarantee is found in the file the next search reads (exit 0)
rg -c '@guarantee OldestFirst' docs/specs/pachinko.allium; echo "exit $?"
# the old promise is gone from the module: rg must exit 1, so this prints "exit 0"
rg -n -U 'not positive\s+(--\s+)?yields an empty batch' docs/specs/pachinko.allium
[ "$?" -eq 1 ]; echo "exit $?"
# positive control: the trees are read and the word is found (exit 0)
rg -c -i 'limit' crates docs/reference README.md | head -3; echo "exit $?"
# no text says a non-positive limit claims a pocket: rg must exit 1
rg -n -U -i 'zero(\s|//[/!]?)+or(\s|//[/!]?)+less(\s|//[/!]?)+claims|zero.limit(\s|_)pull(\s|_)claims' \
  crates docs/reference docs/explanation docs/project README.md
[ "$?" -eq 1 ]; echo "exit $?"
# no text says a Pull rule always holds: rg must exit 1 (only if the base holds double)
rg -n -U -i 'one(\s|#)+of(\s|#)+them(\s|#)+always(\s|#)+holds|Pull row always holds' double
[ "$?" -eq 1 ]; echo "exit $?"
# S031 and S032 are each tagged in the library suite, and S031 on the wire (exit 0)
rg -n 'scenario: S03[12]' crates/libpachinko/tests/queue/pull.rs crates/pachinko/tests/http.rs
echo "exit $?"
just plan-spec pachinko; echo "exit $?"
just test; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just semver; echo "exit $?"
just python-double; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Open points

- What a pull with `limit <= 0` and no lease answers, its name, its place in the check
  order and its wire form: T43's decision, read at step 1. This ticket assumes none of
  them.
- Whether the same pull with no pocket queued is refused too or keeps
  `NoPocketAvailable`: the ruling does not say; step 1 asks if T43's decision does not.
- The new titles of S031 and S032, and whether the pull inside a lease at a limit below
  1 gets a row of its own: suggested above, confirmed at step 1.
- Whether the double moves in this ticket depends on the base step 1 names.
- Who owns `double/diagrams/unreachable-answers.svg` and how it is redrawn: step 3
  finds out. If no script draws it and the text must change, the hand-back says how it
  was edited.
- The order of T39 and T46 is not fixed. The second to land re-reads the first.

## Hand-back notes

### Baseline

### T43's decision and the maintainer's answers (step 1)

### The merged state (step 2)

### The specification

### Red tests

### Verification output

### Adversarial reviews

### Handed back
