---
id: T22
title: "Decision 0019 and the settled questions"
status: todo
depends_on: [T19]
parallel_with: []
branch: ticket/t22-decision-0019
estimated_size: S
---

# T22: Decision 0019 and the settled questions

## Context

T19 wrote the domain as `docs/specs/pachinko.allium`: 27 rules, 25 invariants, 9
guarantees and 9 open questions, 190 obligations from `just plan-spec pachinko` at
`611a86e`. Its two surfaces are the trigger decision 0014 names, so three prototypes then
built the domain three ways, each on an unmerged branch with its own ticket: an Arrow
Flight service over gRPC with a library-owned store trait, an owner task and batch
publish (`tickets/T20-pachinko-prototype.md` on PR 38); an HTTP/JSON service behind a
global mutex with an exhaustive error map, signal handling and a guard-predicting
proptest walk (`tickets/T20-prototype-and-rest-api.md` on PR 39); a worker-pool spike
partitioned by pocket key (`tickets/T20-worker-pool-spike.md` on PR 36); and a log-backed
store stacked on PR 38 (`tickets/T21-pocket-log-records.md` on T21). A comparison of the
three concluded: merge neither, build a new service that grafts PR 38's seams (the
`PocketRecords` trait and its `check_contract`, the expiry-indexed tick, the owner task,
the batch semantics) onto PR 39's HTTP/JSON wire, error map, signal handling and
guard-predicting walk. This ticket writes that down as decision 0019, which supersedes
0014, and carries the one decision that changes the specification, the close of open
question 8, into the module. Every later ticket of the MVP wave, T23 to T31, cites "the
decision table in T22's Context" rather than restating it.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by this step) before `git show` can read them.
Never check a prototype branch out into the ticket's worktree; read with
`git show <head>:<path>`. T21's branch is not on `origin`: 0019 names it as a reopen
option, and the maintainer's checkout is the only place its head can be read.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |
| PR 36, the worker-pool spike | `tickets/t20-worker-pool-spike-complete` | `0fa614e` | `git show 0fa614e:<path>` |
| T21, the log store on PR 38 | `ticket/t21-pocket-log-records` (local, not pushed) | `94fc9b8` | `git show 94fc9b8:<path>` in a checkout that has the branch |

### The decisions

The decision table. 0019 records each row; a later ticket that cannot honour a row hands
it back rather than reading around it.

| Question | Decision |
| --- | --- |
| Wire | axum HTTP/1.1 JSON with PR 39's routes, plus a batch publish route with per-row outcomes |
| Executor | one tokio owner task; bounded mpsc commands with oneshot replies; no `Mutex`; the sweep's `Tick` is a command the same owner applies, serialized with every other command, sent on a separate lane polled first by a `tokio::select!` with `biased;` |
| Store | `MemoryRecords` behind the library-owned `PocketRecords` trait with one `check_contract`; never `ArrowRecords`; `LogRecords` (T21) stays a reopen option |
| OQ8 | closed with branch A: a late action (after `expires_at`, before the lapse is processed) succeeds because the owner serializes the lapse and the action: a pull, ack, nack or renew extends the lease, and a release ends it and frees its records; recovery after a dead consumer takes `lease_ttl`, plus up to one sweep, plus the application of the tick in progress and of the command the owner is applying (a tick samples `now` once, so a lease expiring during a long tick waits for the next); pinned by tests in both orders |
| OQ3 | left open; only request-shape limits (rows, bytes, timeouts), never a capacity refusal |
| Batch limits (flags) | 1,000 rows per request, 1 MiB per payload, 16 MiB per body (Pub/Sub: 1,000 messages and 10 MB; SQS: 10 and 1 MiB; Kafka and NATS: about 1 MiB per message; RabbitMQ 4.x: 16 MiB per message) |
| Lease ids | a `LeaseIds` trait (PR 39's shape) with a counting fake; the real source is an epoch drawn once at start from the operating system's RNG and a serial advanced by `checked_add`, sent as an opaque 32-hex-digit string; the service refuses to start if the RNG fails; ended leases leave the active registry (PR 38) |
| Clock | a `Clock` trait with `FakeClock`; the real clock is `SystemTime` behind the queue's monotonic clamp (both prototypes), the trade-off recorded in 0019; a suspend-aware clock is a listed follow-up |
| OQ1, OQ9 | labelled placeholders, pinned by tests; OQ2 and OQ4 to OQ7 untouched |
| Ticket ids | T22 onward (T20 and T21 belong to the prototype branches); `status: todo` until done; sizes S, M, L |
| Decision record | one fresh 0019 superseding 0014, citing PR 38 at `cb77f07` and PR 39 at `cf1a6b2` |
| Tracer bullet | split: a library tracer (T23), then a service tracer (T24) |
| Parallel lanes | T25, T27 and T29 run in parallel after T24 and merge one at a time: shared files listed `shared, own lines`, `origin/main` merged before the push, verification rerun and requoted (`CONVENTIONS.md` §11, the wave-2 rule) |
| Evidence tickets | T28 (the walk and the catalog) and T30 (benches and the load generator) stay separate |

Config defaults are the specification's: `lease_ttl` 5 minutes, `max_consecutive_expiries`
3, `max_pull_batch` 100, `superseded_ttl` 12 hours, `dead_letter_ttl` 12 hours. The
maintainer confirmed them as the MVP's defaults; this ticket records that in 0019. The
T19 readings the maintainer has not confirmed stay listed in Open points.

### The evidence

Machine and harness, named once in 0019 and copied here so T30 can measure against it: one
Apple silicon Mac (macOS, Darwin 25.4.0, 18 CPUs), rustc 1.99.1, measured 2026-10-31 and
2026-11-03; end-to-end runs over loopback with closed-loop clients; medians of 5 runs;
64-byte payloads; P = 1,000 pockets. The in-process numbers are single-thread over the
prototype's own `Queue` with its fake clock.

| Measurement | Value |
| --- | --- |
| Single-record publish, C=64, PR 39 as shipped (INFO log) | 99,161 records/s |
| Single-record publish, C=64, PR 39 with the log capped at WARN | 157,715 records/s |
| Single-record publish, C=64, PR 38 as shipped (`ArrowRecords`) | 76,058 records/s |
| Single-record publish, C=64, PR 38 with `MemoryRecords` | 77,920 records/s |
| Batch publish, C=8, 256 rows per batch, PR 38 as shipped | 121,394 rows/s |
| Batch publish, C=8, 256 rows per batch, PR 38 with `MemoryRecords` | 1,565,592 rows/s |
| Batch publish, C=64, 1,024 rows per batch, PR 38 with `MemoryRecords` | 2,100,471 rows/s (139,374 as shipped) |
| In process, W1 publish at P=1,000: `ArrowRecords` vs `MemoryRecords` | 287,895 vs 4,246,025 ops/s (14.7x) |
| In process, W2 supersede | 781,874 vs 6,118,040 ops/s (7.8x) |
| In process, W3 pull 16, ack, release | 460,431 vs 2,550,028 ops/s (5.5x) |
| In process, W4 publish with every fifth op a consume step | 301,528 vs 2,831,593 ops/s (9.4x) |
| In process, W5 pull then lapse on tick | 257,835 vs 801,351 ops/s (3.1x) |
| Owner task vs mutex-with-yield vs bare mutex, W1 at C=64, in process, 18 tokio workers, PR 39's `Queue` | 1,375,780 vs 448,107 vs 818,772 ops/s; p50/p99 35.8/60.6 vs 4.96/337.7 vs 2.58/236.4 µs |
| Mutex at C=1 | 3,410,279 vs owner task 1,533,736 ops/s |
| Lapse lag p50 under batch load, PR 38 as shipped (Tick in the client FIFO) | 2,038 ms (max 3,257 ms), re-run 2,411 ms; PR 38 with `MemoryRecords` 439 ms; T21 560 ms; 315 to 689 ms in the other probe runs |
| RSS, PR 38 with `MemoryRecords`, 10 s of batch publishing | 8,191 MiB (nothing bounds it; `NeverRefusedForCapacity` forbids a capacity refusal) |
| RSS per record, consume at C=64 | about 0.93 KiB (`ArrowRecords`) vs 0.45 KiB (`MemoryRecords`) |
| T21 `LogRecords` in process | within ±2.5% of `MemoryRecords` on single-record calls; W1 at P=1,000 3,719,520 vs 4,885,113 ops/s (0.76x); p99 117 vs 44 ms at b=1,024, C=64 |

Caveats 0019 states beside the numbers: one machine, nothing on Linux; closed-loop
clients understate tail latency; the `MemoryRecords` and WARN-log variants are scratch
one-change variants neither prototype ships; HTTP batch publish was never measured, which
is why T30 exists.

Rejected options and their reopen triggers, each listed in 0019:

- Arrow Flight over gRPC (PR 38's wire): reopened if columnar producers become a
  requirement, or if T30's JSON batch rate falls below a ratio of the 1,565,592 rows/s
  Flight-plus-`MemoryRecords` figure that the maintainer sets. 0019 names the figure and
  leaves the ratio to the maintainer; it is not a gate.
- A global mutex (PR 39's executor): reopened only if the owner task's p50 at low
  concurrency becomes the limiting cost. Lock poisoning and the lapse-versus-action
  serialization argument are why it lost.
- Partitioning by pocket key (PR 36): reopened if one owner task's throughput is the
  bottleneck. PR 36 showed that positions and `PullClaimsFrontPocket` need a linearizable
  order first.
- `ArrowRecords`: never. `LogRecords` (T21): reopened if durability or an Arrow consumer
  becomes a requirement; it passes the same `check_contract`.

## Goal

`docs/decisions/0019-service-runtime-tokio-and-http.md` exists, registered and reachable,
and records every row of the decision table, the evidence table with its harness and
caveats, and the rejected options with their reopen triggers; decision 0014 says it is
superseded; `docs/specs/pachinko.allium` states branch A for the lapse race, carries
eight open questions, and still yields 190 obligations; every handbook page that said the
runtime was deferred says it is decided and names the tickets that build it; `just check`
is green.

## Non-goals

- No code in either crate, no dependency, no effect trait. T23 starts the library and
  T24 the service, each on this record.
- No answer to open questions 1, 2, 4, 5, 6, 7 or 9. Only question 8 closes here, and
  question 3 gains one sentence without closing.
- No rule changes in the module. The lapse section's comment, the config comment, one
  guarantee, the Excludes line and two open-question texts change; the 27 rules, the
  entities, the config values and the surfaces do not, which the unchanged obligation
  count shows.
- No pages for the HTTP API, the flags or the service's architecture: T24 to T29 write
  them. This ticket only stops the handbook saying the runtime is undecided.
- No edit to `tickets/README.md` or `CONVENTIONS.md`: the index already lists the wave,
  and the two `CONVENTIONS.md` sentences the wave departs from are T31's hand-back.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/0019-service-runtime-tokio-and-http.md` | new | The record: context, decision, consequences, rejected options with reopen triggers, evidence, related pages |
| `docs/decisions/0014-service-runtime-deferred.md` | shared, own lines | A sentence under the heading saying decision 0019 supersedes it, in the paragraph form 0004 uses for 0011 |
| `docs/manifest.yml` | shared, own lines | One row for 0019, `kind` `decision`, `canonical_for: ["decision_service_runtime"]` |
| `docs/decisions/README.md` | shared, own lines | The 0019 row; the superseded sentence below the table; "The next decision is 0020" |
| `docs/README.md` | shared, own lines | One bullet for 0019 in the Decisions list |
| `docs/explanation/architecture.md` | shared, own lines | "What is deferred": what 0019 decided and that T23 to T31 build it |
| `docs/explanation/security-model.md` | shared, own lines | "What is deferred" and Related pages: the network surface is decided; T24 and T27 describe it |
| `docs/project/purpose-and-scope.md` | shared, own lines | Line 46's bullet: the runtime is decided, not deferred |
| `docs/project/terminology.md` | shared, own lines | Line 49, the Service row: "because no runtime, transport or HTTP stack is chosen (decision 0014)" becomes a clause citing 0019 as the decision; the names the row forbids stay forbidden |
| `docs/reference/configuration.md` | shared, own lines | Lines 10 to 15: the runtime is decided; the flags arrive with T24 |
| `docs/specs/pachinko.allium` | shared, own lines | The Excludes line; the config comment; the lapse section's comment; `ExpiryRecoversWithinTtl`; open question 8 removed; open question 3 extended; the open-question numbering note |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | "Handle an open question": nine to eight |
| `docs/explanation/specifications.md` | shared, own lines | The open-question paragraph: nine to eight, the late-action item removed, the counts of origin corrected |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Changed: open question 8 closed by decision 0019 |
| `tickets/T22-decision-0019.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree from `main` at `611a86e` or later (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. T19 is merged, so this
   ticket waits for nothing. Run `just check` on the untouched worktree and quote its last
   line as the baseline. Run `git fetch origin` (network, authorised by this step) if
   `git show cb77f07:tickets/T20-pachinko-prototype.md` cannot read the PR 38 head.
2. **The record.** Use the `review-docs` skill. Write
   `docs/decisions/0019-service-runtime-tokio-and-http.md` in the shape of 0014, with
   the frontmatter the manifest row repeats, under the title "Decision 0019: The service
   runtime: tokio, one owner task and HTTP/JSON". Its sections:
   - Context: 0014's deferral, the two surfaces T19 named, the three prototypes and T21
     by branch and head as the sources table above lists them, and the comparison's
     conclusion.
   - Decision, one paragraph per row of the decision table. The executor paragraph
     carries the serialization argument: every command and the tick are applied by one
     task, so no owner borrow crosses an `.await`, a lapse and an action are totally
     ordered, and branch A of open question 8 follows. The clock paragraph records the
     trade-off: `SystemTime` behind the queue's monotonic clamp (PR 38
     `crates/libpachinko/src/queue.rs` line 199), so a backward step of the wall clock
     stretches every active lease by the step and nothing lapses early; a suspend-aware
     clock is the listed follow-up. The limits paragraph states the request-shape limits
     and the open question 3 stance in the table's words.
   - Consequences: the dependency tree T23 and T24 add (the crates named in the T23 and
     T24 tickets, each with its decision-0007 note); the network surface (loopback by
     default, no authentication, the limits T27 adds); the effects boundary (the clock
     and the lease ids are traits libpachinko defines with fakes beside them, and
     `pachinko` supplies `SystemTime` and the RNG, decisions 0003 and 0008); the
     unbounded memory the RSS row shows, which `NeverRefusedForCapacity` and open question
     3 leave unbounded.
   - Rejected options: the four above, each with its reopen trigger, the Flight figure
     named and the ratio left to the maintainer.
   - Evidence: the harness sentence, the table and the caveats, inline, with the head
     SHAs; nothing cited from a scratch directory.
   - What would reopen this: the triggers, gathered.
   - Related pages: architecture, security model, purpose and scope, configuration,
     decisions 0003, 0008 and 0014.
   Add the manifest row, the `docs/decisions/README.md` row and the `docs/README.md`
   bullet. Under 0014's heading, add the supersession in the form 0004 uses for 0011
   (`docs/decisions/0004-hook-runner-and-checkers.md`, the paragraph beginning "Decision
   0011 supersedes this record"): 0019 replaces the deferral, and 0014's consequences on
   the effects boundary stand. In `docs/decisions/README.md`, extend the sentence below
   the table that names 0004 and 0011, and change "The next decision is 0019" to 0020.
3. **The specification.** Use the `spec-change` skill, which hands the language to the
   `allium` skill. Five edits, none to a rule:
   - The Excludes line that reads "Transport, wire format and client authentication: the
     spike decision 0014 names, triggered by the surfaces below" now says decision 0019
     records them, triggered by the surfaces below.
   - The config comment that calls `max_pull_batch` "a placeholder nobody has chosen"
     now says the five defaults are the maintainer's, confirmed for the MVP by decision
     0019, and that a deployment sets its own; the values do not change.
   - The lapse section's comment no longer ends on "the eighth open question". It states
     branch A: a pull, ack, nack, renew or release that arrives after `expires_at` but
     before the lapse is processed succeeds, a release ending the lease and a pull, ack,
     nack or renew moving `expires_at`, as the comment already says, because the
     implementation applies the lapse and every action from one serialized stream
     (decision 0019), so the guards above decide in either order.
   - `ExpiryRecoversWithinTtl` states branch A in the guarantee: the lease is lost when
     the lapse is processed, `lease_ttl` after the last action plus the processing delay;
     a late pull, ack, nack or renew processed first extends the lease instead, and the
     records recover once a lapse is processed.
   - The eighth `open question` block, the late-action race, is removed. The third gains
     the sentence: request-shape limits (rows, bytes, timeouts) are not capacity
     refusals. The surviving questions keep the numbers T19 gave them, because the wave's
     scenario tags (`oq:1`, `oq:7`, `oq:9`) and `OneVisitPerKeyPerLease`'s "see open
     question 9" name them so; a comment above the Open Questions section says that the
     eighth was closed by decision 0019 and that the ninth keeps its number.
   Run `just check-specs`, then `just analyse-specs`, and fix each diagnostic and finding
   at its root. Run `just plan-spec pachinko`: 190 obligations are expected, since no
   rule, invariant, transition or surface clause changes. A different count is reconciled
   in the hand-back notes, clause by clause, never hidden.

   **Adversarial review** (`CONVENTIONS.md` §14): the uncommitted diff of
   `docs/specs/pachinko.allium` against the five edits above, the decision table in
   Context and the module's rules, before the handbook repeats it. Ask: whether any edit
   changes a rule, invariant, entity, config value, surface or guarantee clause rather
   than a comment or the guarantee's prose; whether branch A as written contradicts a
   rule's `requires`, so that a late action the comment says succeeds would be refused by
   its guard; whether removing the eighth block or extending the third silently resolves
   another `open question`, or answers the third rather than adding one sentence; whether
   `oq:1`, `oq:7`, `oq:9` or "see open question 9" now names the wrong question; whether
   `just plan-spec pachinko` could print 190 while an obligation id changed, since a
   count is not a diff of the ids; and whether T23 to T31 can be written against the
   module as it now reads. Two reviewer subagents with fresh contexts run in parallel,
   one lens each, `conformance` and `breakage`, each running `codex exec -m gpt-6-astra
   -s read-only -C <worktree> -o ai_tmp/review/T22-1/findings-<lens>.md "<brief>" <
   /dev/null` (§14's command; `-o` writes only the reviewer's last message, and a
   read-only reviewer writes no file of its own); separate triage subagents check every
   finding against the tree and write `ai_tmp/review/T22-1/verdicts.md`, each finding
   valid with its fix or invalid with the reason; one writer applies the valid fixes one
   at a time, only within Files touched, and hands back any fix outside it; if a
   high-severity finding was valid, a second round reviews the fix diff and writes beside
   the first with a `-2` suffix; two rounds at most.
4. **The handbook ripple.** Use the `review-docs` skill. `docs/explanation/architecture.md`
   "What is deferred": the runtime is decided by 0019 and built by T23 to T31; the
   section may be renamed if the rename keeps every link green.
   `docs/explanation/security-model.md` "What is deferred": the network surface is
   decided; T24 and T27 describe it; Related pages gains 0019.
   `docs/project/purpose-and-scope.md` line 46 and `docs/reference/configuration.md`
   lines 10 to 15: the runtime is decided and the flags arrive with T24.
   `docs/project/terminology.md` line 49, the Service row: the clause "because no
   runtime, transport or HTTP stack is chosen (decision 0014)" is replaced by one that
   cites 0019 as the decision, and the names the row forbids stay forbidden.
   `docs/how-to/work-with-the-specs.md` and `docs/explanation/specifications.md`: eight
   open questions where they say nine (`grep -n nine` over both), the late-action item
   dropped from the list, and "seven ... and the last two" corrected. `CHANGELOG.md`:
   libpachinko Unreleased, Changed. Run `just check-docs`.
5. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against the decision
   table and the evidence table in this ticket's Context, the Files touched list and the
   module's rules. Ask: whether every row of the decision table is stated in 0019 and
   whether 0019 states a decision the table does not; whether any number in 0019 is
   absent from the evidence table or differs from it; whether a stated decision is one a
   later ticket cannot honour against the rules as they stand, the lease-id scheme, the
   clamp and branch A included; whether the module's text now contradicts branch A
   anywhere, the guarantee, the comment and the Vocabulary included; whether a page
   outside Files touched now says something false, the ones that cite 0014 first; and
   whether `just plan-spec pachinko` could print 190 while a clause changed. Two reviewer
   subagents with fresh contexts run in parallel, one lens each, `conformance` and
   `breakage`, each running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T22-2/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T22-2/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most.
6. Use the `review-change` skill over the diff, then `project-check` for the full gate:
   `just check`, and commit on the ticket branch with the hand-back notes quoting each
   output headed by the commit it ran at. Pushing and opening a pull request are
   separately authorised: stop and ask.

## Acceptance criteria

- `docs/decisions/0019-service-runtime-tokio-and-http.md` states every row of the
  decision table, the harness sentence, every row of the evidence table, the four
  caveats and the four rejected options with their reopen triggers, and names
  `cb77f07`, `cf1a6b2`, `0fa614e` and `94fc9b8`.
- `docs/decisions/0014-service-runtime-deferred.md` says decision 0019 supersedes it;
  `docs/decisions/README.md` lists 0019, says 0014 is superseded, and says the next
  decision is 0020; `docs/README.md` lists 0019; `docs/manifest.yml` registers it under
  `decision_service_runtime`.
- `docs/specs/pachinko.allium` has eight `open question` blocks, none on the late-action
  race; its lapse comment and `ExpiryRecoversWithinTtl` state branch A; its Excludes line
  cites 0019; its config comment no longer calls `max_pull_batch` a placeholder; no rule
  or default changed.
- `just check-specs` and `just analyse-specs` each print one block, for
  `docs/specs/pachinko.allium`, with empty `diagnostics` and `findings`;
  `just plan-spec pachinko` prints 190 obligations, or the hand-back reconciles the count.
- `grep -rln 0014 docs/` is read page by page, and no page says the runtime is deferred
  or undecided, other than 0014 itself; 0019, the decisions index and the records 0001,
  0008 and 0015 cite 0014 historically and are not edited; `docs/project/terminology.md`
  line 49 no longer says that no runtime, transport or HTTP stack is chosen.
- `just check-docs` and `just check` are green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
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

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

- Config defaults: the maintainer confirmed the specification's five values as the MVP's
  defaults, and 0019 records that; `max_pull_batch = 100` was T19's placeholder and is
  now a confirmed figure.
- The T19 readings the maintainer has not confirmed, each still one line to confirm, and
  each pinned by a T23, T25 or T26 test as the rules stand: deferral position (a deferred
  record takes a fresh position at the ack or nack, not at the lease's end); release and
  `expires_at` (release ends the lease and leaves `expires_at` alone); an empty pocket at
  a lapse below the threshold goes idle rather than staying at the front; a non-positive
  pull limit yields an empty batch rather than an error; open question 9, a key
  republished under the lease that removed its record, where T19's two triage reviews
  disagreed and the `Visit` design is the ready alternative; actions name their lease,
  with the lease entity as the identity; and ordering by position from the one `Queue`
  counter, including whether its start value should be stated with a `default`.
- Whether the maintainer sets the Flight reopen ratio against the 1,565,592 rows/s
  figure, and what T30's measurement is compared with until then.
- A suspend-aware clock: the monotonic clamp stretches leases by a backward step and does
  nothing for a machine that sleeps; 0019 lists the follow-up and no ticket owns it.
- T21's branch `ticket/t21-pocket-log-records` is local to the maintainer's checkout and
  not on `origin`; 0019 cites its head `94fc9b8` as a reopen option that a fresh clone
  cannot read until the branch is pushed.
- The surviving open questions keep T19's numbers with a gap at eight; renumbering them
  is a `spec-change` for a later ticket once T25 and T26 have landed their `oq:` tags,
  or never, if the maintainer prefers stable numbers.
