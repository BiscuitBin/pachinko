---
id: T27
title: "Batch publish with per-row outcomes and request-shape limits"
status: todo
depends_on: [T24]
parallel_with: [T25, T29]
branch: ticket/t27-batch-publish
estimated_size: M
---

# T27: Batch publish with per-row outcomes and request-shape limits

## Context

T24 left the service answering one publish per request over `PUT`, with axum's default
body limit of 2 MiB answered 400 `bad_request` and nothing else bounding a request. The
`Publishing` surface's guidance is throughput over latency, "batch and defer where it
raises the publish rate", and the evidence table in T22's Context shows why a batch route
exists: PR 38's batch publish ran at 1,565,592 rows/s against 157,715 single records/s.
The decision table in T22's Context fixes the shape this ticket builds: one batch route
with per-row outcomes; the equal-rank placeholder (open question 1) mapped per row, as
PR 38 `crates/pachinko/src/actor.rs` line 385 `publish_each` did, while the single `PUT`
keeps 409 `equal_rank`; open question 3 left open, with request-shape limits only, never
a capacity refusal; and the limits 1,000 rows per request, 1 MiB per payload and 16 MiB
per body, which sit among the comparables the table lists (Pub/Sub 1,000 messages and
10 MB; SQS 10 and 1 MiB; Kafka and NATS about 1 MiB per message; RabbitMQ 4.x 16 MiB).
The guarantee `NeverRefusedForCapacity` in `docs/specs/pachinko.allium` is this ticket's
to pin: a request is refused only for its shape, and a service under load waits, never
refuses.

This is a parallel lane: T25 and T29 edit the same service files at the same time, so
every shared file below carries only this ticket's lines, and `origin/main` is merged
before the push with the verification rerun and requoted (`CONVENTIONS.md` §11, the
wave-2 rule).

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by step 2) before `git show` can read them. Never
check a prototype branch out into the ticket's worktree; read with `git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

The files to read: PR 38 `crates/pachinko/src/actor.rs` (`Command::Publish` line 62,
`publish_each` line 385, the two unit tests from line 418); PR 39
`crates/pachinko/src/http.rs` (`body` line 120, `payload` line 130, `PublishBody` line
151) for how a rejection becomes `bad_request`. Neither prototype had a body, payload or
row limit; the three flags are new here.

## Goal

`POST /publishers/{publisher_id}/publish` publishes a batch in one owner command and
answers one outcome per row; the three limit flags refuse a request for its shape before
any row applies, and nothing refuses a request for want of room; `just check` is green.
This ticket owns:

| Owned | Items |
| --- | --- |
| Route (1) | `POST /publishers/{publisher_id}/publish` |
| Flags (3) | `--max-batch-rows` (default 1000), `--max-payload-bytes` (1048576), `--max-body-bytes` (16777216) |
| Error codes (3) | `too_many_rows` 413, `payload_too_large` 413, `body_too_large` 413 |
| Guarantee (1) | `NeverRefusedForCapacity` |

The single `PUT` of T24 shares `--max-payload-bytes` and `--max-body-bytes` once this
ticket lands.

## Non-goals

- No answer to open question 3: no bound on records, no shedding, no spill, no
  back-pressure signal to publishers beyond a full channel making the request wait.
- No answer to open question 1: an equal rank is still the library's error; this ticket
  only maps it per row.
- No atomic batch: rows apply in order and independently, and a refused row does not
  undo an earlier one. `http-api.md` says so.
- No change to the lease actions (T25), readiness, logging or timeouts (T29), and no
  library change: the owner applies T23's `publish` per row.
- No batch pull.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/pachinko/src/config.rs` | shared, own lines | The three flags, their defaults, the load check, three accessors |
| `crates/pachinko/src/actor.rs` | shared, own lines | `Command::PublishBatch`, `RowOutcome`, `publish_each`, `QueueHandle::publish_batch` |
| `crates/pachinko/src/http.rs` | shared, own lines | The route, its handler and bodies, three `Refusal` variants with their arms, the body-limit layer, the payload check on the single `PUT` |
| `crates/pachinko/tests/batch_publish.rs` | new | The wire tests for the route, the limits and the guarantee |
| `docs/reference/http-api.md` | shared, own lines | The route row, a "Batch publish" section, three error rows, the body-limit sentence T24 wrote |
| `docs/reference/configuration.md` | shared, own lines | Three rows in "Service flags" and the load check |
| `docs/explanation/security-model.md` | shared, own lines | A "Request-shape limits" paragraph |
| `CHANGELOG.md` | shared, own lines | pachinko Unreleased, Added |
| `tickets/README.md` | shared, own lines | The index row's status |
| `tickets/T27-batch-publish.md` | ticket | `status:` and hand-back notes |

`crates/pachinko/tests/http.rs` is T25's to edit on its lane; this ticket's tests live in
their own file with their own small `ureq` client, a deliberate duplication that keeps
the two lanes apart (see Open points).

## Steps

1. Create the worktree from `main` after T24 has merged (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. Run `just check` on the
   untouched worktree and quote its last line as the baseline.
2. **The plan.** Use the `plan-change` skill. Run `git fetch origin` (network, authorised
   by this step) so the prototype heads in Context resolve, and read the files listed
   there with `git show`. Read T24's `actor.rs`, `http.rs` and `config.rs` as merged:
   the `Command` enum, `QueueHandle`, `Refusal` with its exhaustive `status()` and
   `code()`, and `ServeConfig::parse`. Write the plan against the names they shipped.
3. **The flags and the load check.** Use the `rust-change` skill. `config.rs` gains
   `--max-batch-rows`, `--max-payload-bytes` and `--max-body-bytes`, read as unsigned
   integers like T24's flags (a negative, non-numeric or valueless flag is a usage error
   naming the flag), each at least 1, with the accessors `max_batch_rows()`,
   `max_payload_bytes()` and `max_body_bytes()`. The load check: `--max-body-bytes` must
   be at least the base64 size of one maximum payload plus the batch envelope, that is
   `ceil(max_payload_bytes / 3) * 4 + 88`, where 88 bytes is the shortest batch body
   around one row with empty keys and the longest rank literal,
   `{"records":[{"pocket_key":"","record_key":"","rank":-9223372036854775808,"payload":""}]}`;
   the single `PUT`'s envelope is 60 bytes, so the batch figure covers both. A smaller
   body cap is a usage error whose message names the minimum; with the defaults the
   minimum is 1,398,192 bytes and the check passes. Unit tests: the defaults, each flag,
   each usage error, a body cap one below the minimum refused and one at the minimum
   accepted.
4. **The command and the route.** Use the `rust-change` skill.
   - `actor.rs`: `Command::PublishBatch { publisher_id, rows: Vec<Publication>, reply:
     Reply<Result<Vec<RowOutcome>, PublishError>> }`, one command per request, so no
     other command runs between two rows; `RowOutcome` is `Applied(PublishOutcome)` or
     `EqualRank`; `publish_each` applies rows in order with T23's `Queue::publish`,
     maps `PublishError::EqualRank` to the row and returns any other error for the
     batch, as PR 38 line 385 does; `QueueHandle::publish_batch` sends it and waits.
     The rows are already bounded by `--max-batch-rows` when they reach the owner.
   - `http.rs`: the route `POST /publishers/{publisher_id}/publish`. Request
     `{"records": [{"pocket_key": "tenant-a", "record_key": "k1", "rank": 1, "payload":
     "aGVsbG8="}, ...]}`; reply 200 `{"outcomes": ["created", "new_key", "superseded",
     "stale", "equal_rank", ...]}` in row order, one string per row. The whole-request
     refusals, checked in this order and each before any row applies:

     | Check | Refusal |
     | --- | --- |
     | The body exceeds `--max-body-bytes`, by its `Content-Length` or as read | 413 `body_too_large` |
     | The body is not the JSON the route takes, or `records` is empty | 400 `bad_request` |
     | More rows than `--max-batch-rows` | 413 `too_many_rows` |
     | A row's payload is not base64, in row order | 400 `bad_request`, naming the row |
     | A row's decoded payload exceeds `--max-payload-bytes`, in row order | 413 `payload_too_large`, naming the row |
     | The publisher never joined (the owner, at the first row, before it lands) | 404 `unknown_publisher` |

     An empty `records` array is refused rather than answered `{"outcomes": []}`, so that
     the first row always exists and the owner's `unknown_publisher` check needs no
     extra library query. The body limit is `axum::extract::DefaultBodyLimit::max` over
     the router with `--max-body-bytes`, replacing axum's default on every route; the
     `Json` extractor's length-limit rejection is told apart from its other rejections
     and answered 413 `body_too_large`, the rest 400 `bad_request` as before. The single
     `PUT` applies the same payload check and answers 413 `payload_too_large` over the
     cap. `Refusal` gains `TooManyRows { rows, max }`, `PayloadTooLarge { row, bytes,
     max }` and `BodyTooLarge { max }`, each with its arm in `status()` and `code()`,
     which stay exhaustive with no wildcard. A handler whose owner has stopped answers
     503 `not_ready` as T24's do.

   **Adversarial review** (`CONVENTIONS.md` §14): the uncommitted diff of steps 1 to 4
   against this ticket's Goal, Files touched, the refusal table above, the decision table
   in T22's Context and the `Publishing` surface of `docs/specs/pachinko.allium`.
   `Command::PublishBatch`, `RowOutcome`, the refusal order and the three new `Refusal`
   variants are what step 5's tests and the other lanes' merges build on. Ask: whether
   any whole-request refusal can be decided after a row has applied; whether another
   command can run between two rows; whether `publish_each` returning an error for the
   batch can leave earlier rows applied, and whether the module and PR 38 line 385 say it
   may; whether the body limit replaces axum's default on every route, and whether one
   maximum payload still fits a single `PUT` and a one-row batch under the default
   `--max-body-bytes`; whether the length-limit rejection is told apart from the other
   `Json` rejections by its type rather than its message; whether `status()` or `code()`
   gained a wildcard arm; and whether a new public type misses invariant 3 or
   `#[non_exhaustive]`. Two reviewer subagents with fresh contexts run in parallel, one
   lens each, `conformance` and `breakage`, each running `codex exec -m gpt-6-astra -s
   read-only -C <worktree> -o ai_tmp/review/T27-1/findings-<lens>.md "<brief>" <
   /dev/null` (§14's command; `-o` writes only the reviewer's last message, and a
   read-only reviewer writes no file of its own); separate triage subagents check every
   finding against the tree and write `ai_tmp/review/T27-1/verdicts.md`, each finding
   valid with its fix or invalid with the reason; one writer applies the valid fixes one
   at a time, only within Files touched, and hands back any fix outside it; if a
   high-severity finding was valid, a second round reviews the fix diff and writes beside
   the first with a `-2` suffix; two rounds at most.
5. **The tests.** Decision 0009 for placement; every wait is bounded and fails on
   timeout, and no claim below is satisfied by a timeout.
   - Unit, `actor.rs`: `publish_each` over rows that yield `created`, `new_key`,
     `superseded`, `stale` and `equal_rank`, with a row after the `equal_rank` still
     applied; an unjoined publisher rejects at the first row with nothing landed. The
     guarantee's channel test: a `#[cfg(test)]` constructor returns a handle and its
     inbox with no owner running and a small capacity; the test `try_send`s filler
     commands until the channel reports `Full`, starts `publish_batch` as a future and
     shows it still pending after a short bounded wait (the wait proves pending, not
     success), then starts the owner on the inbox, and the future resolves to
     `Ok(Ok(...))` with every row `Applied` within a bounded wait; the fillers' replies
     arrive first, in order. The claim pinned: a full channel makes a publish wait and
     then succeed, never refuse.
   - Unit, `http.rs`: one test per new code asserting the status and the `{"error",
     "message"}` body; the length-limit rejection maps to `body_too_large` and a parse
     rejection to `bad_request`.
   - Wire, `crates/pachinko/tests/batch_publish.rs`, against `pachinko::serve` on
     `127.0.0.1:0` as T24's `tests/http.rs` does, with each test naming the obligations
     it exercises in an `// obligation:` comment by the ids `just plan-spec pachinko`
     prints, and the guarantee tests carrying `// guarantee: NeverRefusedForCapacity`:
     - Per-row outcomes: one request whose rows are `a/k1` rank 1, `a/k2` rank 1, `a/k1`
       rank 2, `a/k1` rank 1, `a/k1` rank 2 and `b/k1` rank 1 answers exactly
       `["created", "new_key", "superseded", "stale", "equal_rank", "created"]`, and a
       pull then hands out `k1` at rank 2: the rows after the refused one applied.
     - The guarantee, on a server started with `--max-batch-rows 64
       --max-payload-bytes 65536 --max-body-bytes 8388608`: a single `PUT` of a
       65,536-byte payload is 200; a batch of 64 rows of 65,536-byte payloads is 200
       with 64 outcomes of `created`. The maxima are the server's, not the defaults:
       with the defaults a full batch of maximum payloads is about 1.3 GiB of base64,
       over the body cap by design, so the body cap is the outer bound and
       `http-api.md` says which cap bites first.
     - Each limit with a positive control one below it, on a server started with
       `--max-batch-rows 4 --max-payload-bytes 16` and `--max-body-bytes` at the load
       minimum plus 256: five rows are 413 `too_many_rows` and four rows 200; a 17-byte
       payload is 413 `payload_too_large` naming the row and 16 bytes 200, on the batch
       route and on the single `PUT`; a body padded with JSON whitespace to one byte
       over the cap is 413 `body_too_large` and one padded to exactly the cap is 200.
       Each refusal asserts its code, so a request refused for another reason fails
       the test.
     - An unjoined publisher's batch is 404 `unknown_publisher` and a joined consumer's
       pull afterwards is `no_pocket`: no row landed; the same publisher's five-row
       batch is 413, the shape checks coming first.
     - A late-row refusal lands nothing: from a joined publisher, a request whose first
       row is valid and whose second row's payload is not base64 is 400 `bad_request`
       naming row 2, and another whose first row is valid and whose second row decodes
       to 17 bytes under `--max-payload-bytes 16` is 413 `payload_too_large` naming row
       2; after each, a joined consumer's pull is `no_pocket`, so the row checks ran
       over every row before any row applied.
     - Malformed JSON, an empty `records` array and a non-base64 payload are 400
       `bad_request`.
   - Run `just test`, `just coverage` (the floor is 90% of lines over
     `crates/*/src/**`) and `just doc`.
6. **The handbook.** Use the `review-docs` skill. `docs/reference/http-api.md`: the route
   row in the table; a "Batch publish" section with the request and reply shapes, the
   check order from step 4, the sentence that rows apply in order and independently
   and are never undone as a set, and the caps' interplay (the body cap is the outer
   bound; under the defaults a 1,000-row batch fits only when its encoded payloads
   average under about 16 KiB); three rows in the error table; the body-limit sentence
   T24 wrote now reads `--max-body-bytes`, 413 `body_too_large`. `docs/reference/configuration.md`:
   three rows in "Service flags" and the load check. `docs/explanation/security-model.md`:
   a "Request-shape limits" paragraph after the network surface: what each flag bounds,
   that a limit is a shape refusal and not a capacity one, and that nothing bounds the
   queue's memory (open question 3; the RSS figure in T22's evidence table is the
   reason to say so). `CHANGELOG.md`: pachinko Unreleased, Added. Run
   `just check-docs`. Then the manual smoke run, outside the Verification block because
   `serve` does not return: `cargo run -p pachinko --locked -- serve --listen 127.0.0.1:0
   --max-batch-rows 2` in one terminal; in another, join a publisher, send a two-row
   batch and read its outcomes, send a three-row batch and read `too_many_rows`, then
   ctrl-c; quote the output in the hand-back.
7. **The review.** Use the `review-change` skill over the whole diff against `main`.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, uncommitted, against
   this ticket's Goal, Files touched and Acceptance criteria, the decision table in T22's
   Context and the `Publishing` surface of `docs/specs/pachinko.allium`. Ask: whether a
   limit test passes because the body was rejected for another reason (a 413 asserted
   without its code, or a positive control missing); whether a batch test's rows are all
   `created`, so that order and independence go unexercised; whether the channel-full
   test can pass on a timeout, or passes when the owner never ran; whether a row can
   apply before a whole-request refusal is decided; whether a request can be refused for
   anything but its shape; whether `status()` or `code()` gained a wildcard arm; whether
   the load check can be satisfied by a body cap that still refuses one maximum payload;
   whether the single `PUT` and the batch route disagree on a cap; and whether a line in
   a shared file belongs to T25 or T29. Two reviewer subagents with fresh contexts run in
   parallel, one lens each, `conformance` and `breakage`, each running `codex exec -m
   gpt-6-astra -s read-only -C <worktree> -o ai_tmp/review/T27-2/findings-<lens>.md
   "<brief>" < /dev/null` (§14's command; `-o` writes only the reviewer's last message,
   and a read-only reviewer writes no file of its own); separate triage subagents check
   every finding against the tree and write `ai_tmp/review/T27-2/verdicts.md`, each
   finding valid with its fix or invalid with the reason; one writer applies the valid
   fixes one at a time, only within Files touched, and hands back any fix outside it; if
   a high-severity finding was valid, a second round reviews the fix diff and writes
   beside the first with a `-2` suffix; two rounds at most.
8. Use the `project-check` skill: `just check`, with `fix-quality` for any recipe that
   fails, then commit on the ticket branch with the hand-back notes quoting each output
   headed by the commit it ran at. Then the wave-2 rule (`CONVENTIONS.md` §11): run
   `git fetch origin` (network, authorised by this step) and merge `origin/main`, which
   holds whichever of T25 and T29 landed first; resolve conflicts in the shared files
   keeping both lanes' lines; if `Cargo.lock` conflicts, take `origin/main`'s and run
   `just lock-check`, and only if that fails run `just lock` (network, authorised by this
   step); rerun the Verification block at the merge commit and requote it in the
   hand-back. Pushing and opening a pull request are separately authorised: stop and
   ask.

## Acceptance criteria

- `POST /publishers/{publisher_id}/publish` answers `{"outcomes": [...]}` in row order
  with the five outcome strings, applies rows in order and independently, and reaches
  the owner as one `PublishBatch` command.
- The three flags parse with their defaults, each refuses 0, a negative, a non-numeric
  or a missing value as a usage error, and a `--max-body-bytes` below
  `ceil(max_payload_bytes / 3) * 4 + 88` is a usage error naming the minimum.
- Each of the three codes is produced by a wire test beside a positive control one
  below the limit, and asserted with its status by a unit test; `status()` and `code()`
  have no wildcard arm.
- Every whole-request refusal is decided before any row applies, in the order of the
  table in step 4, and the late-row tests show a valid first row unapplied after a
  refusal of a later row; the single `PUT` answers 413 `payload_too_large` and 413
  `body_too_large` over the same caps.
- `NeverRefusedForCapacity` is pinned by the three tests in step 5 (the maximum single
  publish, the full batch of maximum payloads under the server's caps, the full
  channel that waits and then succeeds), and the hand-back names the guarantee as
  owned here.
- `just coverage` reports at least 90% of lines over `crates/*/src/**`.
- `docs/reference/http-api.md`, `docs/reference/configuration.md` and
  `docs/explanation/security-model.md` describe the route, the flags and the limits as
  built, and no page still says a body over 2 MiB is 400.
- The verification quoted in the hand-back ran at the commit that merges `origin/main`,
  after the lanes that landed first.
- `just check-docs` and `just check` are green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just lock-check
just fmt-check
just clippy
just test
just coverage
just doc
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

- Open question 3 stays open: nothing bounds the number of records or the queue's
  memory, and the three flags bound a request's shape only. The RSS figure in T22's
  evidence table (8,191 MiB after ten seconds of batch publishing on the prototype) is
  the maintainer's reason to decide.
- The identical-retry sub-question of open question 1: a publisher that retries an
  identical publish, same key, rank and payload, after a lost reply gets `equal_rank`
  on the retry, per row on the batch route and 409 on the single `PUT`. Whether an
  identical retry should be idempotent is part of the rank-tie decision.
- With the defaults, a 1,000-row batch of 1 MiB payloads exceeds `--max-body-bytes`;
  the three defaults are the comparables' figures, not a set chosen to fit together.
  The maintainer may prefer a body cap that admits a full batch of maximum payloads,
  about 1.3 GiB, or a smaller payload cap.
- An empty `records` array is 400 `bad_request` here; answering `{"outcomes": []}`
  would need the owner to check the publisher without a row, a library query T23 does
  not have.
- `crates/pachinko/tests/batch_publish.rs` carries its own client helper beside the one
  in `tests/http.rs`; T31 may fold both into `tests/common/mod.rs` once the lanes have
  merged.
- A `Content-Length` over the cap is refused before the body is read, but a chunked body
  is read up to the cap before it is refused; a read timeout on the listener is T29's.
