---
id: T31
title: "Integration and hand-back"
status: todo
depends_on: [T28, T29, T30]
parallel_with: []
branch: ticket/t31-mvp-integration
estimated_size: S
---

# T31: Integration and hand-back

## Context

Nine tickets have merged since T19 specified the domain at `611a86e`: T22 recorded
decision 0019 and closed open question 8; T23 and T24 built the library and service
tracers; T25, T27 and T29 ran as parallel lanes on the wave-2 rule; T26 added expiry and
the tick lane; T28 and T30 produced the evidence. Each was green on its own branch and
each wrote some sentences to the handbook while another lane settled the fact. This
ticket is where the pieces are proved to fit, as T11 did for the bootstrap: the handbook
read against what landed, the pages that still describe the bootstrap or a prototype
corrected, every ownership claim the wave's tickets made checked against the tree, the
whole MVP diff reviewed under `review-change`, and the gate run from a fresh clone and in
CI. `tickets/T11-integration.md` is the procedure; its steps carry over with this wave's
names.

The ownership ledger the wave was written against, by ticket. A reviewer greps for each
name; the per-scenario table of 76 rows is in T28 and, once T28 landed, the "Scenario
catalog" in `docs/reference/testing.md`.

| Kind | T23 | T25 | T26 | T27 |
| --- | --- | --- | --- | --- |
| Rules (27) | `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`, `PublishNewKey`, `Supersede`, `DropStalePublish`, `PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord`, `PullFindsNoPocket` | `ReviveIdlePocket`, `AckSettlesRecord`, `AckDefersNewerVersion`, `NackSettlesRecord`, `NackDefersNewerVersion`, `Renew`, `ReleaseRequeuesPocket`, `ReleaseIdlesEmptyPocket`, `RequeueHeldRecord`, `RequeueDeferredRecord` | `ReviveParkedPocket`, `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `SupersededRecordDecays`, `DeadLetterDecays` | none |
| Invariants (25) | the five config checks `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative`, and `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`, `OneRecordPerKeyPerPocket`, `UniquePocketPosition`, `UniqueRecordPosition`, `PocketPositionsBelowNext`, `RecordPositionsBelowNext`, `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`, `GrantedBeforeExpiry`, `HeldVersionNotNewer`, `OutstandingOnlyWhileLeased`, `ShedBeforeExpiry` | `IdleHasNoRecords`, `EndedAfterGrant`, `NackedBeforeExpiry` | `ExpiriesWithinLimit` | none |
| Guarantees (9) | `EveryDropIsVisible`, `OldestSlotNewestPayload`, `ExclusivePocket`, `OldestFirst` | `ActionsNameTheirLease`, `OneVisitPerKeyPerLease`, `NothingLostWhileHeld` | `ExpiryRecoversWithinTtl` | `NeverRefusedForCapacity` |
| Scenarios (76) | 31 | 23 | 22 | none |

| Kind | T24 | T25 | T26 | T27 | T29 |
| --- | --- | --- | --- | --- | --- |
| Routes (12) | `POST /publishers/{publisher_id}`, `POST /consumers/{consumer_id}`, `PUT /pockets/{pocket_key}/records/{record_key}`, `POST /consumers/{consumer_id}/pull`, `GET /consumers/{consumer_id}/lease`, `GET /healthz` | `POST /leases/{lease_id}/ack`, `/nack`, `/renew`, `/release` | none | `POST /publishers/{publisher_id}/publish` | `GET /readyz` |
| Flags (13) | `--listen`, `--lease-ttl-ms`, `--max-consecutive-expiries`, `--max-pull-batch`, `--superseded-ttl-ms`, `--dead-letter-ttl-ms` | none | `--sweep-ms` | `--max-batch-rows`, `--max-payload-bytes`, `--max-body-bytes` | `--log-level`, `--request-timeout-ms`, `--shutdown-grace-ms` |
| Error codes (14) | `bad_request` 400, `publisher_exists` 409, `consumer_exists` 409, `unknown_publisher` 404, `unknown_consumer` 404, `equal_rank` 409 | `lease_not_active` 409, `lease_not_yours` 403, `record_not_held` 404 | none | `too_many_rows` 413, `payload_too_large` 413, `body_too_large` 413 | `not_ready` 503, `request_timeout` 408 |

The ten `[wire]` scenarios are exercised over HTTP by T24 (S019, S020, S028, S031, S033)
and T25 (S040, S047, S050, S053, S068); that is exercise, never ownership.
`just plan-spec pachinko` printed 190 obligations at `611a86e`, and T22 expected the same
count after closing open question 8; T28 reconciled every obligation id to a test.

The eight open questions that remain in `docs/specs/pachinko.allium`, in the module's
numbering, which does not compact (the `oq:9` scenario tags stay `oq:9`): 1, equal-rank
publishes; 2, who reads the superseded and dead-letter queues and what an entry must
carry; 3, capacity beyond shedding the decaying queues; 4, routing by worker pool or
priority; 5, an adaptive lease TTL; 6, a new pocket jumping the queue; 7, reviving a
parked pocket other than by a publish; 9, a key published again under the lease that
removed its record. Question 8, a late action before its lapse is processed, was closed by
T22 with branch A.

## Goal

The ticket branch at its final commit, cloned fresh and initialized, passes `just check`,
online and under `CONVENTIONS.md` §11's network denial; CI is green on the pull request;
every page in the handbook sweep says what landed and none describes the bootstrap or a
prototype; every rule, invariant, guarantee, scenario, route, flag and code in the ledger
is in the tree and tagged; `docs/how-to/work-with-the-specs.md` lists the eight open
questions as the module numbers them; decisions 0014 and 0019 cross-reference each other
and nothing else cites 0014 as current; `tickets/README.md` shows T22 to T31 `done`; and
the two `CONVENTIONS.md` sentences are handed back as a `main` pull request.

## Non-goals

- No new feature, route, flag, recipe, page or dependency. A gap in prose found here is
  a one-line fix to what a lane delivered. A missing MVP obligation, that is a rule,
  invariant, guarantee or scenario with no exercising test, or a route, flag or code
  absent from the tree, is never an Open point: it is handed back to the owning
  ticket's lane and blocks this ticket until the repair has merged and the sweep is
  rerun, whichever step finds it. What the steps below route to Open points stays
  there, and no step routes a missing MVP obligation there.
- No answer to any open question, and no edit to a decision record other than the 0014
  and 0019 cross-references.
- No edit to `tickets/CONVENTIONS.md` on this lane: §0 and §1 are frozen text that only a
  `main` pull request may change.
- No benchmark or load run: T30's numbers stand as quoted there.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `README.md` | shared, own lines | lines 10 to 11: the domain is specified and implemented; what the service does |
| `AGENTS.md` | shared, own lines | line 5, "Their domain is not defined yet", and line 13, "one placeholder module"; `just check-agents` |
| `docs/README.md` | shared, own lines | lines 15 to 24: the intro and the placeholder paragraph; the Look up and Run it lists complete |
| `docs/project/purpose-and-scope.md` | shared, own lines | "What it does", the runtime non-goal at lines 43 to 46 |
| `docs/project/repository-map.md` | shared, own lines | line 89, `docs/specs/` "declares no behaviour"; the crate rows |
| `docs/project/terminology.md` | shared, own lines | line 49, the Service row's "no runtime, transport or HTTP stack is chosen (decision 0014)" |
| `docs/tutorials/first-change.md` | shared, own lines | lines 60 to 62, "the domain is not defined yet" |
| `docs/explanation/architecture.md` | shared, own lines | "The service today" and "What is deferred" describe the MVP and cite 0019 |
| `docs/explanation/security-model.md` | shared, own lines | "What is deferred" and Related pages: the surface is decided and described |
| `docs/reference/api.md` | shared, own lines | the items named, if any, are the MVP's |
| `docs/reference/testing.md` | shared, own lines | lines 83 to 84, "a placeholder that states no behaviour"; the catalog present |
| `docs/reference/configuration.md` | shared, own lines | the intro's 0014 sentence; line 156, "declares nothing"; "Service flags" complete |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | the open-question list, eight entries in module numbering |
| `docs/decisions/0014-service-runtime-deferred.md`, `docs/decisions/0019-service-runtime-tokio-and-http.md` | shared, own lines | the cross-references, only if one is missing or wrong |
| `crates/libpachinko/src/lib.rs`, `crates/libpachinko/README.md`, `crates/pachinko/README.md` | shared, own lines | the crate docs and READMEs describe the MVP, if a lane left a bootstrap sentence |
| `CHANGELOG.md` | shared, own lines | both Unreleased sections read as one wave; the bootstrap line "The domain awaits its implementation" retired |
| `tickets/README.md` | shared, own lines | the Status column for T22 to T31 |
| `tickets/CONVENTIONS.md` | shared, own lines | §0 line 32 and §1 decision 1, through a `main` pull request; stop and ask |
| `tickets/T31-mvp-integration.md` | ticket | `status:` and hand-back notes |

Line numbers are as of `611a86e`; a lane that already corrected a line is a row with
nothing to do, recorded as such in the hand-back.

## Steps

1. Create the worktree from `main` after T28, T29 and T30 have merged (`README.md`, "How
   to pick up a ticket"), or, in a clean worktree already initialized on `main` whose
   branch carries nothing, create the branch there and skip `just initialize`. Run
   `just check` on the untouched worktree and quote its last line as the baseline.
2. **Reconcile the hand-backs.** Use the `plan-change` skill. Read the hand-back notes and
   Open points of T22 to T30. List every item handed back to "a later ticket", to T31 or
   to the maintainer, and every check a hand-back says it did not run, skipped or only
   dry-ran. Each gets a disposition here: fixed in this ticket (a one-line correction in a
   file above), closed by evidence another ticket quoted (cite the ticket and section),
   handed back to the owning ticket's lane as a blocking repair (a missing MVP
   obligation, Non-goals), or open with an owner in Open points, which a missing MVP
   obligation never is. A green gate is cited only for an item it runs.
3. **The ownership sweep.** For each row of the ledger in Context, show the name in the
   tree. Rules, invariants and guarantees: `rg -n '<Name>' crates/libpachinko` prints a
   test or an `// obligation:` tag that names it. Scenarios: the scenario lines of the
   Verification block, with their positive control, print nothing and exit 0, so the id
   set over `crates/libpachinko/tests` and `crates/libpachinko/src` (S005's tag is on
   T23's doctest under `src/`) is exactly S001 to S076 with no id tagged twice (the
   pattern takes the whole token, so an id outside the range fails the `diff` rather
   than escaping it), and
   `docs/reference/testing.md`'s catalog has 76 rows whose owning tests exist by file
   and name. Routes: `rg -n '\.route\(' crates/pachinko/src/http.rs` prints
   12. Flags: the `--name` arms in `crates/pachinko/src/config.rs` number 13 (the
   Verification regex assumes PR 39's `match` shape; adjust it to the shape T24 landed
   and quote the adjusted command). Codes:
   `code()` in `http.rs` has 14 arms and no wildcard. `just plan-spec pachinko` prints 190
   obligations, or the hand-back reconciles the difference against T22 and T28 rather
   than hiding it. A name missing from the tree is a blocking finding for the owning
   ticket's lane: that ticket reopens, lands the repair with its test on its own branch,
   and this ticket waits, then merges `origin/main`, reruns the sweep at the merge
   commit, and re-clones as step 9 says if step 7 has already run. This ticket adds no
   test and is not done until the sweep shows every name; the hand-back records each
   repair by ticket and commit.
4. **The handbook sweep.** Use the `review-docs` skill. Walk the Files touched rows in
   order. A search that must print nothing afterwards, with its positive control being the
   same search on `611a86e`, which prints every line in the table:

   ```sh
   rg -n -i 'not defined yet|is undefined|placeholder module|declares no behaviour|states no behaviour|declares nothing|awaits its implementation|no async runtime|no runtime, transport' README.md AGENTS.md CHANGELOG.md docs crates --glob '!docs/decisions/*'
   ```

   Decision records are not edited for these phrases: 0002, 0003 and 0012 describe the
   repository as it was when each was taken. Then the 0014 sweep: `rg -n '0014' docs`
   prints only 0014 itself with its "Superseded by decision 0019" line, the decisions
   index's row and its foundation sentence (line 54 at `611a86e`), 0019's own citation of
   what it supersedes, the manifest entry, and records
   0001, 0008 and 0015 citing it historically; a handbook page citing 0014 as the current
   runtime decision is corrected to 0019. `docs/how-to/work-with-the-specs.md` lists the
   eight questions in Context, in module numbering, and `docs/explanation/specifications.md`
   says eight. `AGENTS.md` lines 5 and 13 take `just check-agents`. Run `just check-docs`.
5. **`review-change` over the wave.** Use the `review-change` skill over
   `git diff 611a86e...HEAD`, the whole MVP diff, not this ticket's alone: the ten
   invariants in `AGENTS.md` (T23 added the tenth), the effects boundary with
   `just effects`, documentation ownership in `docs/manifest.yml`, scope. Each finding is
   fixed here if it is a one-line correction in a file above; a finding that a rule,
   invariant, guarantee or scenario has no exercising test, or that a route, flag or
   code is absent from the tree, is a missing MVP obligation and takes step 3's blocking
   repair whatever step found it; any other finding is recorded in Open points with the
   owning ticket.
6. **`tickets/README.md`.** Set the Status column for T22 to T30 to `done` where each
   ticket's `status:` says so, and T31's row to `done` in the same pull request as its
   `status:` (`CONVENTIONS.md` §11, definition of done). The "Review of the ticket set"
   bullet is the maintainer's; if it still reads "Pending", record that in Open points
   and leave it.
7. **Commit A, then the fresh clone.** `just check`, commit the sweep. Then clone the
   ticket branch at that commit from the local repository, which needs no network:

   ```sh
   git clone --branch ticket/t31-mvp-integration --single-branch "$(git rev-parse --git-common-dir)" ai_tmp/fresh
   git -C ai_tmp/fresh rev-parse HEAD
   ```

   The printed commit equals commit A. In `ai_tmp/fresh`: `just initialize` (network,
   authorised by this step; the clone is its own primary checkout, so it installs its own
   hook shim, which is deleted with it), then `just check`, quoting the whole output.
   Then `just check` again in the clone under `CONVENTIONS.md` §11's network denial,
   quoting it with both negative-control exit codes. If the gate needs the network, that
   is a finding to hand back, not a recipe to weaken. `ai_tmp/fresh` is deleted at the
   end of step 9; a merge of `origin/main` there means deleting it and re-cloning at the
   merge commit, so the requoted clone lines name the commit they ran at.
8. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff of this ticket, with the
   ledger in Context, the hand-back dispositions of step 2 and the sweep searches of
   steps 3 and 4 beside it. Ask: whether every rule, invariant, guarantee, scenario,
   route, flag and code in the ledger is in the tree and tagged, and whether a missing
   one was parked as an Open point rather than handed back and repaired; whether any
   page still describes the prototype branches, the bootstrap
   binary that printed one line, or a runtime that is not chosen; whether a page cites
   0014 as current; whether the open-question list matches the module's `open question`
   blocks one for one; whether a disposition in step 2 cites a gate that does not run the
   item; whether the fresh-clone quote came from the commit it names; whether a search
   that must print nothing was run without its positive control. Codex `gpt-6-astra`
   reviewers run read-only with fresh contexts, one lens each; separate triage subagents
   write verdicts; valid fixes are applied one writer at a time; two rounds at most. A
   fix after the clone makes step 7's quotes stale: rerun step 7. Then `just check`
   (`project-check` skill; a failing recipe goes to `fix-quality`), set `status: done`,
   and commit B with the hand-back notes, each quoted output headed by the commit it ran
   at.
9. **Fetch, push and the pull request.** `git fetch origin` (network, authorised by this
   step); if `origin/main` moved, merge it, re-clone as step 7 says, rerun the
   Verification block and `just check`, requote at the merge commit, commit. Delete
   `ai_tmp/fresh`. Then stop and ask: pushing and opening the pull
   request are separately authorised. When authorised, push, open the pull request, and
   read its CI run: the tested commit is the merge commit, not the branch head, and the
   run URL and both go in the hand-back. Merging to `main` runs `pages.yml`, which deploys
   the API reference (decision 0013): the merge is a deployment and is the maintainer's
   action.
10. **The `CONVENTIONS.md` hand-back.** Draft, in the hand-back notes, the two replacement
    sentences: §0 line 32, "The domain is undefined: no ticket here writes domain code,
    domain prose or a behavioural specification", becomes a sentence that the bootstrap
    tickets T00 to T18 wrote none and that T19 and the MVP wave did; §1 decision 1, "The
    domain is left undefined", gains a dated note that T19 defined it and T22 to T31
    implemented it. Then stop and ask: the edit lands through a separate `main` pull
    request, never on this lane (`CONVENTIONS.md` §11, the frozen list; `README.md` in
    `tickets/` records that only a `main` pull request edits those two sentences).

## Acceptance criteria

- The fresh clone's `just check` is green and quoted at commit A, and green again under
  `CONVENTIONS.md` §11's network denial with both negative-control probes failing.
- CI is green on the pull request, with the run URL and the tested merge commit quoted.
- Step 4's search prints nothing outside `docs/decisions/`, and its positive control at
  `611a86e` is quoted; `rg -n '0014' docs` prints only the survivors step 4 names.
- Every name in the ledger is shown in the tree; a name handed back during the sweep is
  shown again at the merge commit of its repair, and no missing name is carried as an
  Open point; the scenario id set is exactly S001 to S076 with each id tagged once, the
  routes 12, the flags 13, the codes 14, and `just plan-spec pachinko` prints 190 or the
  hand-back reconciles the count.
- `docs/how-to/work-with-the-specs.md` lists the eight open questions in module numbering
  and the module has eight `open question` blocks.
- Every item in step 2's list has a disposition, and no green gate is cited for an item
  it does not run.
- `tickets/README.md` shows T22 to T31 `done`.
- The two `CONVENTIONS.md` sentences are drafted in the hand-back and not edited on the
  lane.
- `just check-docs`, `just check-agents` and `just check` are green.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just plan-spec pachinko
set -o pipefail
rg -c 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | diff - <(printf 'scenario: S0%02d\n' $(seq 1 76)); echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort | uniq -d; echo "exit $?"
rg -n '\.route\(' crates/pachinko/src/http.rs
rg -c '^\s*"--[a-z-]+" =>' crates/pachinko/src/config.rs
rg -n -i 'not defined yet|is undefined|placeholder module|declares no behaviour|states no behaviour|declares nothing|awaits its implementation|no async runtime|no runtime, transport' README.md AGENTS.md CHANGELOG.md docs crates --glob '!docs/decisions/*'; echo "exit $?"
rg -n '0014' docs
rg -c 'open question' docs/specs/pachinko.allium
# the clone lines: at step 7, and again by re-cloning if step 9 merged origin/main
git -C ai_tmp/fresh rev-parse HEAD
# macOS; on Linux, unshare -rn in place of sandbox-exec (CONVENTIONS.md §11)
deny='(version 1)(allow default)(deny network*)'
(cd ai_tmp/fresh && sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org; echo "exit $?")
(cd ai_tmp/fresh && sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1; echo "exit $?")
(cd ai_tmp/fresh && sandbox-exec -p "$deny" just check; echo "exit $?")
just check-docs
just check-agents
just check
git status --short
```

## Hand-back notes

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

- The eight open questions in Context are the maintainer's to answer; the `research`
  task on the rank tie that T19 recorded is still the way to answer the first.
- The T19 readings the maintainer has not confirmed, listed in T22's Open points, stay
  listed there; this ticket adds none and closes none.
- Whether the maintainer sets the Flight reopen ratio against T30's batch figure (0019
  names the figure and leaves the ratio open).
- The suspend-aware clock 0019 lists as a follow-up has no ticket.
- `tickets/README.md`'s "Review of the ticket set" bullet, if still "Pending" when this
  ticket runs.
- The two `CONVENTIONS.md` sentences, until their `main` pull request merges.
